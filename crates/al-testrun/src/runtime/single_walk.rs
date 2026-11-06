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
use super::written::{ReachIndex, union_reach_facts};
use super::*;
use al_vm::single_carry::{Carry, GlobalKey};

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
    /// The codeunits the run reran from what a chain member left because they read it (`corpus.rs`).
    pub leak_reran: &'a std::collections::HashSet<usize>,
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
    /// The last chain member run again whose run left something else than its first run, with what it left (suite,
    /// id, leg, state): the later chain members and the codeunits that read what the chain left run again from it.
    relay: Option<(usize, u32, u8, super::run::Carried)>,
}

/// What a chain member run again left, against its first run (`SingleWalk::chain_left_other`).
enum Left {
    Same,
    Relay(Box<super::run::Carried>),
    Broken(String),
}

impl SingleWalk<'_> {
    /// Walks the runs ([`SingleWalk::walk`]), notes the codeunits run again in `report`, and runs the serial check of
    /// the carrying when asked (ADR-066 amendment 1: the same outcomes with the codeunits of each BC session run in one
    /// go, the single instances moved from session to session).
    pub fn walk_and_check(&self, slots: &mut [Option<UnitResult>], opts: &RunOptions, report: &mut super::ChainReport) {
        let again = self.walk(slots);
        if opts.single_serial_check || std::env::var_os("AL_SINGLE_SERIAL_CHECK").is_some() {
            let (ran, diffs) = self.serial_check(slots);
            report.serial_checked += ran;
            report.serial_mismatches.extend(diffs);
            report.serial_check = true;
        }
        for k in again {
            report.reruns.push(super::ChainEntry {
                id: self.units[k].id,
                name: self.lowered.program.objects[self.units[k].oi].name.clone(),
                why: super::RerunWhy::ReadSingleInstance,
                took: slots[k].as_ref().map_or(std::time::Duration::ZERO, |r| r.took),
                left: false,
                ahead: false,
            });
        }
    }

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
            let matches = slots[k].as_ref().is_none_or(|r| fits(r, &s.state));
            // A chain member before it ran again and left something else than its first run: the chain members after
            // it, and the codeunits that read what the chain left, start from that.
            let relayed = s
                .relay
                .as_ref()
                .is_some_and(|(si, id, leg, _)| (self.sees)(*si, *id, *leg, u))
                && self.reads_relay(k, s.relay.as_ref());
            if !matches || relayed {
                let r = self.run_again(k, Some(&s.state), s.relay.as_ref());
                match self.chain_left_other(k) {
                    // Back on the first run's chain: the later codeunits keep what the first runs gave them.
                    Left::Same if relayed && self.is_member(k) => s.relay = None,
                    Left::Same => {}
                    Left::Relay(c) => s.relay = Some((u.si, u.id, self.leg_of(k), *c)),
                    Left::Broken(why) => s.broken = Some(why),
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
    fn run_again(
        &self,
        k: usize,
        state: Option<&SingleState>,
        relay: Option<&(usize, u32, u8, super::run::Carried)>,
    ) -> UnitResult {
        let mut u = self.units[k].clone();
        u.singletons = state.map(|s| Arc::new(s.clone()));
        let before = relay
            .filter(|(si, id, leg, _)| (si, id) != (&u.si, &u.id) && (self.sees)(*si, *id, *leg, &u))
            .or_else(|| {
                self.carried
                    .iter()
                    .rev()
                    .find(|(si, id, leg, _)| (si, id) != (&u.si, &u.id) && (self.sees)(*si, *id, *leg, &u))
            });
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
        // A codeunit that left nothing new leaves the state it was given (`baseline: None`, `numbers_taken.rs`): that is
        // `base`, as the chain walk fills it in (`chain_walk.rs`).
        super::run::CARRIED.with(|c| {
            if let Some(left) = c.borrow_mut().as_mut().filter(|l| l.baseline.is_none()) {
                left.baseline = base.cloned();
            }
        });
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
            // What the last chain member of this check left: the later members, and the codeunits that read what the
            // chain left, start from it.
            let mut relay: Option<(usize, u32, u8, super::run::Carried)> = None;
            for &k in &ks {
                let from = relay.as_ref().filter(|_| self.reads_relay(k, relay.as_ref()));
                let r = self.run_again(k, None, from);
                let left = super::run::CARRIED.with(|c| c.borrow_mut().take());
                if self.is_member(k) {
                    if let Some(c) = left {
                        relay = Some((self.units[k].si, self.units[k].id, self.leg_of(k), c));
                    }
                }
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
                            "codeunit {} test {test}: {} with SingleInstance state carried, {} in one session",
                            self.units[k].id,
                            outcome_text(Some(o)),
                            outcome_text(s)
                        ));
                    }
                }
            }
            super::run::SERIAL_SINGLES.with(|c| *c.borrow_mut() = None);
        }
        (ran, diffs)
    }

    /// Whether codeunit `k` starts from what a relayed chain member left: a chain member does, and so does a codeunit
    /// the run reran from what the chain left (it read it) or that can reach session storage the member changed
    /// (`corpus.rs` `reaches_session_storage`).
    fn reads_relay(&self, k: usize, relay: Option<&(usize, u32, u8, super::run::Carried)>) -> bool {
        self.is_member(k)
            || self.leak_reran.contains(&k)
            || relay.is_some_and(|r| {
                r.3.everything && super::corpus::reaches_session_storage(self.engine, self.lowered, self.units[k].oi)
            })
    }

    /// The leg of chain member `k` in the chain walk's notes (`chain_walk::Carry`).
    fn leg_of(&self, k: usize) -> u8 {
        let u = &self.units[k];
        self.carried
            .iter()
            .find(|(si, id, _, _)| (*si, *id) == (u.si, u.id))
            .map_or(0, |c| c.2)
    }

    /// Whether codeunit `k` is a member of the carry chain (what it leaves reaches the later codeunits).
    fn is_member(&self, k: usize) -> bool {
        let u = &self.units[k];
        u.carry.is_some() || u.isolation == TestIsolation::Disabled
    }

    /// For a chain member run again: why the later codeunits cannot use what the chain walk handed on, when the run
    /// again left other rows than the run the walk took.
    fn chain_left_other(&self, k: usize) -> Left {
        let u = &self.units[k];
        let left = super::run::CARRIED.with(|c| c.borrow_mut().take());
        if !self.is_member(k) {
            return Left::Same;
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
        let broken = |what: &str| {
            Left::Broken(format!(
                "test codeunit {} read SingleInstance state an earlier test codeunit left, and with it left other rows \
                 for the later codeunits than al-runner's first run of it ({what}): carrying both is not supported",
                u.id
            ))
        };
        match (first, left) {
            (None, None) => Left::Same,
            (Some(a), Some(b)) => {
                // The same tables, the same session storage and the same Test Runner's rows (or unseen ones): the
                // codeunits after it that read none of them (and so ran from the install baseline) are not touched by
                // what differs. What differs otherwise (number sequences, counters, the rows of those tables) reaches
                // the chain members after it and the codeunits that read what it left: they run again from what this
                // run left (`walk`).
                // Fewer tables (a setup the serial state makes it skip): every later codeunit that reads one of
                // them read one of the first run's tables too, so it is run again from this run's rows as well. A
                // table only this run changed was read by codeunits the first run's tables do not name: refused.
                if !b.tables.is_subset(&a.tables) {
                    let only = |x: &super::run::Carried, y: &super::run::Carried| {
                        let v: Vec<String> = x.tables.difference(&y.tables).map(u32::to_string).collect();
                        v.join(", ")
                    };
                    return broken(&format!(
                        "it changed other tables: only the first run {}; only the run again {}",
                        only(a, &b),
                        only(&b, a)
                    ));
                }
                match (print(Some(a)), print(Some(&b))) {
                    (Some(x), Some(y)) if x.rows == y.rows || unseen => {
                        if x.main == y.main {
                            Left::Same
                        } else {
                            Left::Relay(Box::new(b))
                        }
                    }
                    (None, None) if a.baseline.is_none() && b.baseline.is_none() => Left::Same,
                    (Some(_), Some(_)) => broken("the Test Runner's rows differ, and the run saw them"),
                    _ => broken("what it left has no fingerprint"),
                }
            }
            _ => broken("one of the runs left nothing"),
        }
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
        let Some(&first) = routines.first() else { return };
        // One walk of the refused routines' joined reach: they share most of it, and a global any of them can write is
        // not known from here on, whichever wrote it.
        let roots: Vec<(usize, usize)> = routines.iter().map(|&i| (r.oi, i)).collect();
        let facts = union_reach_facts(&self.engine.events, self.lowered, &index, &roots);
        let what = match routines.len() {
            1 => format!("{}.{}", obj.name, obj.routines[first].name),
            n => format!(
                "{}.{} and {} other refused routines",
                obj.name,
                obj.routines[first].name,
                n - 1
            ),
        };
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

/// Whether run `r` saw, of every SingleInstance global it read before writing it, what the state `serial` gives it: the
/// value `serial` holds (or it was given that very value), or, for a global `serial` does not hold (no earlier codeunit wrote it), the value of a new instance, which the run saw when it did not start with that global given (`Unit::singletons`: none, the serial
/// state of `walk`, or a prediction of the carry chain's walk, `chain_walk.rs`). A run that did not track them fits.
pub(super) fn fits(r: &UnitResult, serial: &SingleState) -> bool {
    let Ok(done) = &r.result else { return true };
    let Some(out) = done.5.singles_out.as_ref() else {
        return true;
    };
    let start = done.5.singles_in.as_deref();
    out.observed
        .iter()
        .all(|(key, seen)| match (serial.get(key), start.and_then(|s| s.get(key))) {
            // The run was given the very value `serial` holds: it restored it as the run from `serial` does
            // (`Session::restore_single_instances`), whatever its first read encodes it as (a record buffer of a
            // rolled-back row reads back without its row version), and a value that is not carried (the same
            // description) was refused where it read it, as there.
            (Some(v), Some(given)) if same_given(v, given) => true,
            (Some(v), _) => v == seen,
            (None, given) => given.is_none(),
        })
}

/// Whether two carried values restore the same: equal carried values, or values that are not carried with the same
/// description.
fn same_given(a: &Carry, b: &Carry) -> bool {
    match (a, b) {
        (Carry::Opaque(x), Carry::Opaque(y)) => x == y,
        _ => a == b,
    }
}

/// The state `before` with what run `r` wrote applied (`walk`'s `take`, without what refused tests may have changed);
/// also what it wrote. Predictions only (`chain_walk.rs`): `fits` checks every run against the state of `walk`.
pub(super) fn after_run(before: &SingleState, r: &UnitResult) -> (SingleState, Option<Arc<SingleState>>) {
    let wrote = match &r.result {
        Ok(done) => done.5.singles_out.as_ref().map(|out| Arc::new(out.wrote.clone())),
        Err(_) => None,
    };
    let mut now = before.clone();
    if let Some(w) = &wrote {
        now.extend(w.iter().map(|(k, v)| (*k, v.clone())));
    }
    (now, wrote)
}

/// An outcome in words, for the serial check's lines.
fn outcome_text(o: Option<&Outcome>) -> String {
    match o {
        Some(Outcome::Pass) => "passed".to_string(),
        Some(Outcome::Fail(m)) => format!("failed ({m})"),
        Some(Outcome::NotRun(why)) => format!("not run ({why})"),
        None => "missing".to_string(),
    }
}

/// Whether object `o` is a codeunit with `SingleInstance = true`.
fn single_instance(lowered: &Lowered, o: usize) -> bool {
    lowered.program.objects[o]
        .properties
        .iter()
        .any(|(k, v)| k.eq_ignore_ascii_case("SingleInstance") && v.trim().eq_ignore_ascii_case("true"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use al_vm::single_codec::CarryVal;

    fn val(b: u8) -> Carry {
        Carry::Val(CarryVal::Plain(vec![b]))
    }

    /// A run that read global (1, 0) first and saw `seen` there, started from `start`.
    fn run(start: Option<SingleState>, seen: Carry) -> UnitResult {
        let mut scratch = Scratch {
            singles_in: start.map(Arc::new),
            ..Default::default()
        };
        scratch.singles_out = Some(al_vm::single_carry::SingleOut {
            observed: [((1, 0), seen)].into_iter().collect(),
            ..Default::default()
        });
        UnitResult {
            si: 0,
            oi: 0,
            id: 1,
            result: Ok((Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new(), scratch)),
            took: std::time::Duration::ZERO,
            sidecar_start: std::time::Duration::ZERO,
        }
    }

    fn state(c: Carry) -> SingleState {
        [((1, 0), c)].into_iter().collect()
    }

    /// A run given the very value the serial state holds fits, even when its first read encodes it otherwise (a
    /// record buffer of a rolled-back row reads back without its row version, Tests-Dimension 134485), and so does a
    /// value that is not carried with the same description.
    #[test]
    fn a_run_given_the_serial_value_fits_whatever_its_first_read_encodes() {
        assert!(fits(&run(Some(state(val(1))), val(2)), &state(val(1))));
        let opaque = || Carry::Opaque("a stream".into());
        assert!(fits(&run(Some(state(opaque())), opaque()), &state(opaque())));
        assert!(!fits(
            &run(Some(state(Carry::Opaque("other".into()))), opaque()),
            &state(opaque())
        ));
    }

    /// A run given another value, or none, fits only when it saw the serial value; a run given a value for a global
    /// the serial state does not hold never fits.
    #[test]
    fn a_run_given_another_value_fits_only_when_it_saw_the_serial_one() {
        assert!(fits(&run(Some(state(val(3))), val(1)), &state(val(1))));
        assert!(!fits(&run(Some(state(val(3))), val(3)), &state(val(1))));
        assert!(fits(&run(None, val(1)), &state(val(1))));
        assert!(!fits(&run(None, val(0)), &state(val(1))));
        assert!(fits(&run(None, val(0)), &SingleState::new()));
        assert!(!fits(&run(Some(state(val(0))), val(0)), &SingleState::new()));
    }
}
