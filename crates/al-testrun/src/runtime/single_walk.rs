//! SingleInstance state across the test codeunits of a run (probe P7 of ADR-066, al-runner issue 6924).
//!
//! BC runs the test codeunits of a run in one session, so the globals of a SingleInstance codeunit that one codeunit
//! left are what the next one starts with (al-language/codeunits/TestCuP7BReader.al:24-37, against
//! TestCuP7ASetter.al:24-58; the same on BC 27.5, 28.5 and 29.0). al-runner runs the codeunits in parallel, each on a
//! session of its own that starts with no single instances (or, run again here, with given ones). Every session notes
//! the globals it read before writing them and what it left (`al_vm::single_carry`).
//!
//! After all codeunits ran, [`walk`] goes through them in run order, per session of BC (a suite, and on the
//! TestRunnerHub its two legs), keeping the state the serial run would have at each point. A codeunit's run counts
//! when every global it read first held, at its start, the value the serial state holds; it then started from what
//! the serial run would have given it as far as it could see, and its outcome is the serial one. Otherwise it runs
//! again, here, from the serial state. The state then takes what the codeunit left.
//!
//! A test that was refused does not show what it would have left: the globals its reach can write (all of them, when
//! part of its reach is not known) are not known from there on, and a later codeunit that reads one of them first
//! does not trust its failures, as within a codeunit (`run.rs`, `written.rs` `changes_read_by`).

use super::chain_walk::Carry as ChainCarry;
use super::corpus::refuse_all;
use super::written::{ReachIndex, reach_facts};
use super::*;
use al_vm::single_carry::GlobalKey;

std::thread_local! {
    /// Whether the last run again kept the Test Runner's rows unseen (`suite_rows::take_unseen`).
    static RERUN_UNSEEN: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// What a codeunit run again here is run with.
pub(super) struct SingleWalk<'a> {
    pub engine: &'a Arc<Engine>,
    pub lowered: &'a Lowered,
    pub baseline: Option<&'a al_vm::Baseline>,
    pub units: &'a [Unit],
    /// What the carry chain left (`chain_walk.rs`).
    pub carried: &'a [ChainCarry],
    /// Whether a codeunit sees what the chain member (suite, id, leg) left (`corpus.rs`).
    pub sees: &'a dyn Fn(usize, u32, u8, &Unit) -> bool,
    /// The place of a codeunit in its suite's run order (`corpus.rs`).
    pub place: &'a dyn Fn(usize, u32) -> u32,
    pub limit: Option<std::time::Duration>,
}

/// The serial state of one session of BC.
#[derive(Default)]
struct Stream {
    state: SingleState,
    /// Globals a refused test may have changed: why.
    unknown: std::collections::BTreeMap<GlobalKey, String>,
    /// Any global may have changed: why.
    unknown_all: Option<String>,
    /// A run again of a chain member left other rows than its first run: why the later codeunits are not run.
    broken: Option<String>,
}

impl SingleWalk<'_> {
    /// Checks every codeunit's run against the serial SingleInstance state and runs again those that saw another one.
    /// Returns the codeunits run again.
    pub fn walk(&self, slots: &mut [Option<UnitResult>]) -> Vec<usize> {
        let mut order: Vec<usize> = (0..self.units.len())
            .filter(|&k| !self.units[k].execute && slots[k].is_some())
            .collect();
        order.sort_by_key(|&k| {
            let u = &self.units[k];
            (u.si, u.hub_leg, (self.place)(u.si, u.id), k)
        });
        let mut streams: std::collections::HashMap<(usize, bool), Stream> = Default::default();
        let mut again = Vec::new();
        for k in order {
            let u = &self.units[k];
            let s = streams.entry((u.si, u.hub_leg)).or_default();
            if let Some(why) = &s.broken {
                if let Some(Ok(done)) = slots[k].as_mut().map(|r| &mut r.result) {
                    refuse_all(&mut done.0, why);
                }
                continue;
            }
            let matches = match slots[k].as_ref().map(|r| &r.result) {
                Some(Ok(done)) => done.5.singles_out.as_ref().is_none_or(|out| {
                    out.observed
                        .iter()
                        .all(|(key, seen)| s.state.get(key).is_none_or(|v| v == seen))
                }),
                _ => true,
            };
            if !matches {
                let r = self.run_again(k, Some(&s.state));
                if let Some(why) = self.chain_left_other(k, &r) {
                    s.broken = Some(why);
                }
                slots[k] = Some(r);
                again.push(k);
            }
            let Some(r) = slots[k].as_mut() else { continue };
            self.distrust(s, r);
            self.take(s, r);
        }
        again
    }

    /// Runs codeunit `k` again from the serial state `state`, on the database state the serial run gives it: what the
    /// last chain member before it left (non-chain codeunits leave no rows).
    fn run_again(&self, k: usize, state: Option<&SingleState>) -> UnitResult {
        let mut u = self.units[k].clone();
        u.singletons = state.map(|s| Arc::new(s.clone()));
        let before = self
            .carried
            .iter()
            .rev()
            .find(|(si, id, leg, _)| (si, id) != (&u.si, &u.id) && (self.sees)(*si, *id, *leg, &u));
        let (base, unfinished) = match before {
            Some((_, _, _, c)) if c.unfinished.is_none() && c.storage_why.is_none() => {
                (c.baseline.as_ref().or(self.baseline), None)
            }
            // What it left is not known: a read of it is refused, as in the run after the parallel part.
            Some((_, _, _, c)) => {
                u.leak = Some(Arc::new(c.tables.clone()));
                (self.baseline, c.unfinished.clone().or(c.storage_why.clone()))
            }
            None => (self.baseline, None),
        };
        // Whether the run again saw what the Test Runner's tables held at its start (`chain_left_other`).
        super::suite_rows::take_unseen();
        let mut r = run_unit(self.engine, self.lowered, base, &u, None, self.limit);
        RERUN_UNSEEN.with(|c| c.set(super::suite_rows::take_unseen()));
        if let (Some(why), Ok(done)) = (unfinished, &mut r.result) {
            if done.5.leak_hit {
                refuse_all(&mut done.0, &why);
            }
        }
        r
    }

    /// The serial check (ADR-066 amendment 1, owner 2026-11-06): runs the codeunits of each BC session in which a
    /// codeunit read SingleInstance state an earlier codeunit left once more, one after the other on this thread, each
    /// starting from the very single instances (and temporary tables) the codeunit before it left, moved from session
    /// to session, nothing copied (`run.rs` `SERIAL_SINGLES`). Every test the run above ran (passed or failed) must
    /// come out the same; the differences are returned. The database start of each codeunit is the one the run above
    /// gave it (`run_again`). Returns the codeunits run and the differences.
    pub fn serial_check(&self, slots: &[Option<UnitResult>]) -> (usize, Vec<String>) {
        let mut order: Vec<usize> = (0..self.units.len())
            .filter(|&k| !self.units[k].execute && slots[k].is_some())
            .collect();
        order.sort_by_key(|&k| {
            let u = &self.units[k];
            (u.si, u.hub_leg, (self.place)(u.si, u.id), k)
        });
        let mut streams: Vec<((usize, bool), Vec<usize>)> = Vec::new();
        for k in order {
            let key = (self.units[k].si, self.units[k].hub_leg);
            match streams.last_mut() {
                Some((s, ks)) if *s == key => ks.push(k),
                _ => streams.push((key, vec![k])),
            }
        }
        let out_of = |k: usize| match slots[k].as_ref().map(|r| &r.result) {
            Some(Ok(done)) => done.5.singles_out.as_ref(),
            _ => None,
        };
        let (mut ran, mut diffs) = (0, Vec::new());
        for (_, ks) in streams {
            // Only a session in which some codeunit read a global an earlier one wrote depends on the carrying.
            let mut written = std::collections::BTreeSet::new();
            let mut depends = false;
            for &k in &ks {
                if let Some(out) = out_of(k) {
                    depends |= out.observed.keys().any(|key| written.contains(key));
                    written.extend(out.wrote.keys().copied());
                }
            }
            if !depends {
                continue;
            }
            super::run::SERIAL_SINGLES.with(|c| *c.borrow_mut() = Some(None));
            for &k in &ks {
                let r = self.run_again(k, None);
                super::run::CARRIED.with(|c| c.borrow_mut().take());
                ran += 1;
                let (Some(Ok(first)), Ok(serial)) = (slots[k].as_ref().map(|r| &r.result), &r.result) else {
                    continue;
                };
                for (test, o) in &first.0 {
                    if !matches!(o, Outcome::Pass | Outcome::Fail(_)) {
                        continue;
                    }
                    let s = serial.0.iter().find(|(t, _)| t == test).map(|(_, o)| o);
                    if s != Some(o) {
                        diffs.push(format!(
                            "codeunit {} test {test}: {o:?} with SingleInstance state carried, {s:?} in one session",
                            self.units[k].id
                        ));
                    }
                }
            }
            super::run::SERIAL_SINGLES.with(|c| *c.borrow_mut() = None);
        }
        (ran, diffs)
    }

    /// For a chain member run again: why the later codeunits cannot use what the chain walk handed on, when the run
    /// again left other rows than the run the walk took.
    fn chain_left_other(&self, k: usize, _r: &UnitResult) -> Option<String> {
        let u = &self.units[k];
        let left = super::run::CARRIED.with(|c| c.borrow_mut().take());
        if u.carry.is_none() && u.isolation != TestIsolation::Disabled {
            return None;
        }
        let first = self
            .carried
            .iter()
            .find(|(si, id, _, _)| (*si, *id) == (u.si, u.id))
            .map(|c| &c.3);
        // The Test Runner's rows ("Test Method Line", "AL Test Suite") hold the runner's clock columns, which differ from
        // run to run: they are compared apart (`Baseline::fingerprint_apart`, as the chain walk does). `suite_rows::write`
        // empties them before each codeunit's tests; when nothing of the run again saw what they held (no subscriber,
        // no install or login code touching them: `suite_rows::take_unseen`), the same holds for the later codeunits,
        // which start the same way, so they cannot see the difference either.
        let apart: Vec<u32> = super::suite_rows::tables(&self.engine.program).into_iter().collect();
        let print = |c: Option<&super::run::Carried>| {
            c.and_then(|c| c.baseline.as_ref())
                .and_then(|b| b.fingerprint_apart(&apart))
        };
        let unseen = RERUN_UNSEEN.with(std::cell::Cell::take);
        let same = match (first, left.as_ref()) {
            (Some(a), Some(b)) => {
                a.tables == b.tables
                    && match (print(Some(a)), print(Some(b))) {
                        (Some(x), Some(y)) => x.main == y.main && (x.rows == y.rows || unseen),
                        (None, None) => a.baseline.is_none() && b.baseline.is_none(),
                        _ => false,
                    }
            }
            (None, None) => true,
            _ => false,
        };
        (!same).then(|| {
            format!(
                "test codeunit {} read SingleInstance state an earlier test codeunit left, and with it left other rows \
                 for the later codeunits than al-runner's first run of it: carrying both is not supported",
                u.id
            )
        })
    }

    /// Marks as not trusted the failures of a codeunit that read a global a refused test before it may have changed.
    fn distrust(&self, s: &Stream, r: &mut UnitResult) {
        let Ok(done) = &mut r.result else { return };
        let Some(out) = done.5.singles_out.as_ref() else { return };
        let why = out.observed.keys().find_map(|key| s.unknown.get(key).map(|why| (key, why))).map(|((o, g), why)| {
            let obj = &self.lowered.program.objects[*o];
            format!(
                "an earlier test codeunit of the run was refused ({why}): it can change variable {} of SingleInstance \
                 codeunit {}, which this codeunit reads, so the state it would have left is not BC's",
                obj.globals.get(*g).map_or("?", |s| s.name.as_str()),
                obj.name
            )
        });
        let why = why.or_else(|| {
            (!out.observed.is_empty()).then_some(()).and(s.unknown_all.as_ref()).map(|why| {
                format!(
                    "an earlier test codeunit of the run was refused ({why}): part of its reach is not known, so it may \
                     change the SingleInstance state this codeunit reads, and the state it would have left is not BC's"
                )
            })
        });
        let Some(why) = why else { return };
        for (_, o) in done.0.iter_mut() {
            if let Outcome::Fail(m) = o {
                *o = Outcome::NotRun(format!("{why}; this failure is not trusted: {m}"));
            }
        }
    }

    /// The serial state after codeunit `r`: what it left, and what its refused tests may have changed.
    fn take(&self, s: &mut Stream, r: &UnitResult) {
        let done = match &r.result {
            Ok(done) => done,
            Err(panic) => {
                s.unknown_all = Some(format!("the runtime stopped in test codeunit {}: {panic}", r.id));
                return;
            }
        };
        if let Some(out) = &done.5.singles_out {
            for (key, v) in &out.wrote {
                s.state.insert(*key, v.clone());
                if !out.observed.contains_key(key) {
                    s.unknown.remove(key);
                }
            }
            if let Some(&o) = out.bound.first() {
                // Whether BC keeps a manual binding of a single instance for the next test codeunit is not pinned
                // (docs/probes-needed.md).
                s.unknown_all.get_or_insert_with(|| {
                    format!(
                        "SingleInstance codeunit {} was still bound to its manual subscribers when test codeunit {} \
                         ended, and whether that binding reaches the next test codeunit is not pinned",
                        self.lowered.program.objects[o].name, r.id
                    )
                });
            }
        }
        let refused: Vec<&str> = done
            .0
            .iter()
            .filter(|(_, o)| matches!(o, Outcome::NotRun(_)))
            .map(|(n, _)| n.split('[').next().unwrap_or(n))
            .collect();
        if refused.is_empty() {
            return;
        }
        let obj = &self.lowered.program.objects[r.oi];
        let index = self
            .engine
            .derived(|| ReachIndex::new(&self.engine.events, self.lowered));
        let mut routines: Vec<usize> = (0..obj.routines.len())
            .filter(|&i| refused.iter().any(|n| obj.routines[i].name.eq_ignore_ascii_case(n)))
            .collect();
        // A codeunit none of whose tests ran: its OnRun did not run either.
        if refused.len() == done.0.len() {
            routines.extend(
                (0..obj.routines.len())
                    .filter(|&i| obj.routines[i].is_trigger && obj.routines[i].name.eq_ignore_ascii_case("OnRun")),
            );
        }
        for routine in routines {
            let facts = reach_facts(&self.engine.events, self.lowered, &index, r.oi, routine);
            let what = format!("{}.{}", obj.name, obj.routines[routine].name);
            if let Some(why) = &facts.unknown {
                s.unknown_all.get_or_insert_with(|| format!("{what}: {why}"));
            }
            for &(o, g) in &facts.globals_written {
                if single_instance(self.lowered, o) {
                    s.unknown.entry((o, g)).or_insert_with(|| what.clone());
                }
            }
        }
    }
}

/// Whether object `o` is a codeunit with `SingleInstance = true`.
fn single_instance(lowered: &Lowered, o: usize) -> bool {
    lowered.program.objects[o]
        .properties
        .iter()
        .any(|(k, v)| k.eq_ignore_ascii_case("SingleInstance") && v.trim().eq_ignore_ascii_case("true"))
}
