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
use super::written::{ReachIndex, union_reach_written};
use super::*;
use al_vm::single_carry::{BINDINGS_KEY, Carry, GlobalKey};

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
    /// Runs again that [`SingleWalk::predict`] made, by codeunit: [`SingleWalk::walk`] takes one when it fits the serial
    /// state at the codeunit, instead of running it again.
    pub predicted: &'a std::cell::RefCell<std::collections::HashMap<usize, UnitResult>>,
}

/// The serial state of one session of BC.
#[derive(Default)]
struct Stream {
    state: SingleState,
    /// `state` as one shared value for the runs again, made when one needs it and dropped when the state changes.
    shared: Option<Arc<SingleState>>,
    /// Globals a refused test may have changed: why.
    unknown: std::collections::BTreeMap<GlobalKey, String>,
    /// Any global may have changed: why.
    unknown_all: Option<String>,
    /// A run again of a chain member left other rows than its first run: why the later codeunits are not run.
    broken: Option<String>,
    /// The last chain member run again whose run left something else than its first run, with what it left (suite,
    /// id, leg, state): the later chain members and the codeunits that read what the chain left run again from it.
    relay: Option<(usize, u32, u8, super::run::Carried)>,
    /// What `take` noted since the last [`SingleWalk::resolve`], in order: the walk of a refused test's reach is only
    /// made when a later codeunit that read SingleInstance state has a failure to trust or not (`distrust`).
    pending: Vec<Pend>,
}

/// A note of `take`, applied in order by `SingleWalk::resolve`.
enum Pend {
    /// The runtime stopped in a test codeunit: any global may have changed.
    Stopped(String),
    /// A codeunit wrote this global without reading it first: what a refused test may have changed there is settled.
    Wrote(GlobalKey),
    /// Tests of codeunit `oi` (routines `routines`) were refused: `what` names them.
    Refused {
        oi: usize,
        routines: Vec<usize>,
        what: String,
    },
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
        let t_walk = std::time::Instant::now();
        let mut t_rerun = std::time::Duration::ZERO;
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
                // A run again made for the prediction of the batches (`Predictor::advance`) stands when it saw the state the serial
                // run holds here: a codeunit outside the chain, with no relayed rows, runs the same way (`run_again`).
                let cached = (s.relay.is_none() && !self.is_member(k))
                    .then(|| self.predicted.borrow_mut().remove(&k))
                    .flatten()
                    .filter(|c| fits(c, &s.state));
                let t_r = std::time::Instant::now();
                let r = match cached {
                    Some(c) => c,
                    None => {
                        let shared = s.shared.get_or_insert_with(|| Arc::new(s.state.clone())).clone();
                        self.run_again(k, Some(shared), s.relay.as_ref())
                    }
                };
                match self.chain_left_other(k) {
                    // Back on the first run's chain: the later codeunits keep what the first runs gave them.
                    Left::Same if relayed && self.is_member(k) => s.relay = None,
                    Left::Same => {}
                    Left::Relay(c) => s.relay = Some((u.si, u.id, self.leg_of(k), *c)),
                    Left::Broken(why) => s.broken = Some(why),
                }
                t_rerun += t_r.elapsed();
                slots[k] = Some(r);
                again.push(k);
            }
            let Some(r) = slots[k].as_mut() else { continue };
            self.distrust(s, r);
            self.take(s, r);
        }
        eprintln!("TIMES walk {:.2}s of which reruns {:.2}s; single_out {:.2}s restore {:.2}s enc {:.2}s grp {:.2}s bindcarry {:.2}s", t_walk.elapsed().as_secs_f64(), t_rerun.as_secs_f64(), al_vm::single_carry::TIME_OUT.load(std::sync::atomic::Ordering::Relaxed) as f64 / 1e9, al_vm::single_carry::TIME_IN.load(std::sync::atomic::Ordering::Relaxed) as f64 / 1e9, al_vm::single_carry::TIME_ENC.load(std::sync::atomic::Ordering::Relaxed) as f64 / 1e9, al_vm::single_carry::TIME_GRP.load(std::sync::atomic::Ordering::Relaxed) as f64 / 1e9, al_vm::single_carry::TIME_RK.load(std::sync::atomic::Ordering::Relaxed) as f64 / 1e9);
        again
    }

    /// Runs codeunit `k` again from the serial state `state`, on the database state the serial run gives it: what the
    /// last chain member before it left (non-chain codeunits leave no rows).
    fn run_again(
        &self,
        k: usize,
        state: Option<Arc<SingleState>>,
        relay: Option<&(usize, u32, u8, super::run::Carried)>,
    ) -> UnitResult {
        let mut u = self.units[k].clone();
        u.singletons = state;
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
    fn distrust(&self, s: &mut Stream, r: &mut UnitResult) {
        let Ok(done) = &mut r.result else { return };
        let Some(out) = done.5.singles_out.as_ref() else { return };
        // Only a failure is distrusted: what the refused tests before may have changed is worked out for one.
        if out.observed.is_empty() || !done.0.iter().any(|(_, o)| matches!(o, Outcome::Fail(_))) {
            return;
        }
        self.resolve(s);
        let why = out.observed.keys().find_map(|key| s.unknown.get(key).map(|why| (key, why))).map(|(&key, why)| {
            if key == BINDINGS_KEY {
                return format!(
                    "an earlier test codeunit of the run was refused ({why}): it can bind or unbind event subscribers, \
                     and this codeunit raises an event with a manual subscriber, so the state it would have left is not \
                     BC's"
                );
            }
            let (o, g) = key;
            let obj = &self.lowered.program.objects[o];
            format!(
                "an earlier test codeunit of the run was refused ({why}): it can change variable {} of SingleInstance \
                 codeunit {}, which this codeunit reads, so the state it would have left is not BC's",
                obj.globals.get(g).map_or("?", |s| s.name.as_str()),
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

    /// The serial state after codeunit `r`: what it left. What its refused tests may have changed is noted
    /// ([`Pend`]) and worked out when it is needed ([`SingleWalk::resolve`]).
    fn take(&self, s: &mut Stream, r: &UnitResult) {
        let done = match &r.result {
            Ok(done) => done,
            Err(panic) => {
                s.pending.push(Pend::Stopped(format!(
                    "the runtime stopped in test codeunit {}: {panic}",
                    r.id
                )));
                return;
            }
        };
        if let Some(out) = &done.5.singles_out {
            for (key, v) in &out.wrote {
                // A value the state holds already stays as it is (nothing to copy, and the shared copy stays valid).
                if s.state.get(key) != Some(v) {
                    s.state.insert(*key, v.clone());
                    s.shared = None;
                }
                if !out.observed.contains_key(key) {
                    s.pending.push(Pend::Wrote(*key));
                }
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
        let what = match routines.len() {
            1 => format!("{}.{}", obj.name, obj.routines[first].name),
            n => format!(
                "{}.{} and {} other refused routines",
                obj.name,
                obj.routines[first].name,
                n - 1
            ),
        };
        s.pending.push(Pend::Refused {
            oi: r.oi,
            routines,
            what,
        });
    }

    /// Works out what the refused tests noted since the last call may have changed, in order.
    fn resolve(&self, s: &mut Stream) {
        for p in std::mem::take(&mut s.pending) {
            match p {
                Pend::Stopped(why) => {
                    s.unknown_all = Some(why);
                }
                Pend::Wrote(key) => {
                    s.unknown.remove(&key);
                }
                Pend::Refused { oi, routines, what } => {
                    let index = self
                        .engine
                        .derived(|| ReachIndex::new(&self.engine.events, self.lowered));
                    // One walk of the refused routines' joined reach: they share most of it, and a global any of them
                    // can write is not known from here on, whichever wrote it.
                    let roots: Vec<(usize, usize)> = routines.iter().map(|&i| (oi, i)).collect();
                    let facts = union_reach_written(&self.engine.events, self.lowered, &index, &roots, &|o| {
                        single_instance(self.lowered, o)
                    });
                    if let Some(why) = &facts.unknown {
                        s.unknown_all.get_or_insert_with(|| format!("{what}: {why}"));
                    }
                    // A refused test that may bind or unbind: which bindings reach the next codeunit is not known.
                    if facts.state_writes & super::written::state::BINDING != 0 {
                        s.unknown.entry(BINDINGS_KEY).or_insert_with(|| what.clone());
                    }
                    for &(o, g) in &facts.globals_written {
                        if single_instance(self.lowered, o) {
                            s.unknown.entry((o, g)).or_insert_with(|| what.clone());
                        }
                    }
                }
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
        .all(|(key, seen)| key_fits(key, seen, start, serial))
}

/// Whether the global `key`, which a run saw holding `seen` at its first read and was given `start`'s value of, held that
/// in the state `serial` too (`fits`).
fn key_fits(key: &GlobalKey, seen: &Carry, start: Option<&SingleState>, serial: &SingleState) -> bool {
    match (serial.get(key), start.and_then(|s| s.get(key))) {
        // The run was given the very value `serial` holds: it restored it as the run from `serial` does
        // (`Session::restore_single_instances`), whatever its first read encodes it as (a record buffer of a
        // rolled-back row reads back without its row version), and a value that is not carried (the same
        // description) was refused where it read it, as there.
        // A value that carries (`Val`) that the run could not read back (it saw `Opaque`: its session could not restore it,
        // as when its committed row version is below the record's) does not count: the run again from the serial state
        // may restore it.
        (Some(v), Some(given)) if same_given(v, given) => {
            !(matches!(seen, Carry::Opaque(_)) && matches!(v, Carry::Val(_)))
        }
        (Some(v), _) => v == seen,
        (None, given) => given.is_none(),
    }
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
pub(super) fn after_run(
    before: Arc<SingleState>,
    r: &UnitResult,
    keep_wrote: bool,
) -> (Arc<SingleState>, Option<Arc<SingleState>>) {
    let out = match &r.result {
        Ok(done) => done.5.singles_out.as_ref(),
        Err(_) => None,
    };
    let mut now = before;
    // Nothing is copied when the codeunit wrote nothing, or when no run ahead holds the state it started from.
    if let Some(out) = out.filter(|o| !o.wrote.is_empty()) {
        Arc::make_mut(&mut now).extend(out.wrote.iter().map(|(k, v)| (*k, v.clone())));
    }
    (now, out.filter(|_| keep_wrote).map(|o| Arc::new(o.wrote.clone())))
}

/// The batches of the parallel part (`rounds_run.rs`): per BC session (suite, hub leg) in run order, so many codeunits
/// at a time. A batch starts from the state the batches before it left ([`Predictor`]): a codeunit that reads a global
/// a codeunit before it in the same batch changed saw the old value and has to run again. So the size of a session's
/// batch adapts: it doubles (up to `max`) while the batch before it was predicted right, and falls back to `min` when a
/// codeunit of it had to run again, because such a change (a cache a codeunit filled, a mock one started) makes every
/// later codeunit of its batch run twice. Only the time of the run depends on the sizes.
pub(super) struct Batches {
    /// Per session: its codeunits still to run, in run order, and the size of its next batch.
    queues: Vec<((usize, bool), std::collections::VecDeque<usize>, usize)>,
    /// The sessions (indexes of `queues`) the last batch took codeunits from.
    last: Vec<usize>,
    min: usize,
    max: usize,
}

impl Batches {
    /// The batches for the codeunits `rest` (indexes of `units`).
    pub(super) fn new(
        units: &[Unit],
        rest: &[usize],
        place: &dyn Fn(usize, u32) -> u32,
        min: usize,
        max: usize,
    ) -> Batches {
        let mut order: Vec<usize> = rest.iter().copied().filter(|&k| !units[k].execute).collect();
        order.sort_by_key(|&k| {
            let u = &units[k];
            (u.si, u.hub_leg, place(u.si, u.id), k)
        });
        let mut streams: Vec<((usize, bool), Vec<usize>)> = Vec::new();
        for k in order {
            let key = (units[k].si, units[k].hub_leg);
            match streams.last_mut() {
                Some((q, ks)) if *q == key => ks.push(k),
                _ => streams.push((key, vec![k])),
            }
        }
        let mut b = Batches::of(streams, min, max);
        // The codeunits that execute (they run with nothing before them) go in the first batch.
        let first: Vec<usize> = rest.iter().copied().filter(|&k| units[k].execute).collect();
        if !first.is_empty() {
            b.queues.insert(0, ((usize::MAX, false), first.into(), usize::MAX));
        }
        b
    }

    /// The batches of sessions `streams` (the session, its codeunits in run order).
    fn of(streams: Vec<((usize, bool), Vec<usize>)>, min: usize, max: usize) -> Batches {
        Batches {
            queues: streams.into_iter().map(|(key, ks)| (key, ks.into(), min)).collect(),
            last: Vec::new(),
            min,
            max,
        }
    }

    /// The next batch (indexes of `units`); empty when every codeunit ran.
    pub(super) fn next(&mut self) -> Vec<usize> {
        self.last.clear();
        let mut out = Vec::new();
        for (i, (_, ks, size)) in self.queues.iter_mut().enumerate() {
            let n = (*size).min(ks.len());
            if n > 0 {
                self.last.push(i);
                out.extend(ks.drain(..n));
            }
        }
        out
    }

    /// Whether every codeunit is in a batch already.
    pub(super) fn is_done(&self) -> bool {
        self.queues.iter().all(|(_, ks, _)| ks.is_empty())
    }

    /// Tells which sessions had a codeunit that ran again after the last batch (`Predictor::advance`).
    pub(super) fn feedback(&mut self, again: &std::collections::HashSet<(usize, bool)>) {
        for &i in &self.last {
            let (key, _, size) = &mut self.queues[i];
            if *size == usize::MAX {
                continue;
            }
            *size = if again.contains(key) {
                self.min
            } else {
                (*size * 2).min(self.max)
            };
        }
    }
}

/// A BC session's predicted state, with a shared copy of it for the runs again (dropped when the state changes).
type Predicted = (SingleState, Option<Arc<SingleState>>);

/// The predicted SingleInstance state of each BC session after the codeunits that ran so far, in run order.
pub(super) struct Predictor {
    /// Per session: its codeunits in run order, how many were taken into the state, and the state.
    streams: Vec<PredictedStream>,
}

type PredictedStream = ((usize, bool), Vec<usize>, usize, Predicted);

/// The state of each BC session, and the sessions where a codeunit ran again ([`SingleWalk::advance`]).
pub(super) type Advanced = (
    std::collections::HashMap<(usize, bool), Arc<SingleState>>,
    std::collections::HashSet<(usize, bool)>,
);

impl SingleWalk<'_> {
    /// A predictor over all codeunits of the run (chain members included).
    pub fn predictor(&self) -> Predictor {
        let units = self.units;
        let mut order: Vec<usize> = (0..units.len()).filter(|&k| !units[k].execute).collect();
        order.sort_by_key(|&k| (units[k].si, units[k].hub_leg, (self.place)(units[k].si, units[k].id), k));
        let mut streams: Vec<PredictedStream> = Vec::new();
        for k in order {
            let key = (units[k].si, units[k].hub_leg);
            match streams.last_mut() {
                Some((q, ks, _, _)) if *q == key => ks.push(k),
                _ => streams.push((key, vec![k], 0, Predicted::default())),
            }
        }
        Predictor { streams }
    }

    /// Takes the codeunits whose runs `result` has, in run order up to the first one it has not, into the predicted
    /// states: each run when it fits the state so far, and otherwise as run again from it (the run again is only for
    /// what it leaves: `walk` takes it when it fits there). Returns the state of each session (those with one) and the
    /// sessions where a codeunit ran again. Only a prediction: `walk` checks every run against the serial state.
    pub fn advance<'r>(&self, p: &mut Predictor, result: &dyn Fn(usize) -> Option<&'r UnitResult>) -> Advanced {
        let mut seeds = std::collections::HashMap::new();
        let mut again = std::collections::HashSet::new();
        for (key, ks, next, (st, shared)) in p.streams.iter_mut() {
            while let Some(&k) = ks.get(*next) {
                let Some(r) = result(k) else { break };
                *next += 1;
                let mut cache = self.predicted.borrow_mut();
                if !fits(r, st) && !cache.get(&k).is_some_and(|c| fits(c, st)) {
                    let given = shared.get_or_insert_with(|| Arc::new(st.clone())).clone();
                    let rerun = self.run_again(k, Some(given), None);
                    super::run::CARRIED.with(|c| c.borrow_mut().take());
                    cache.insert(k, rerun);
                    again.insert(*key);
                }
                let r = if fits(r, st) { r } else { &cache[&k] };
                if let Ok(done) = &r.result {
                    if let Some(out) = &done.5.singles_out {
                        for (key, v) in &out.wrote {
                            if st.get(key) != Some(v) {
                                st.insert(*key, v.clone());
                                *shared = None;
                            }
                        }
                    }
                }
            }
            if !st.is_empty() {
                seeds.insert(*key, shared.get_or_insert_with(|| Arc::new(st.clone())).clone());
            }
        }
        (seeds, again)
    }
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
        Carry::Val(Arc::new(CarryVal::Plain(vec![b])))
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

    /// A run that read a value its session could not restore (it saw a value that is not carried) is not the serial run
    /// when the serial state holds that value as a carried one, even when the run was given exactly that value: the run
    /// again from the serial state may restore it (a record whose row version is above the session's committed one).
    #[test]
    fn a_run_that_could_not_restore_the_given_value_does_not_fit() {
        let refused = || Carry::Opaque("a record buffer with a row version this session has not committed".into());
        assert!(!fits(&run(Some(state(val(1))), refused()), &state(val(1))));
        // The serial state holds a value that is not carried either, with the same description: the run was refused
        // there as the run from the serial state is.
        assert!(fits(&run(Some(state(refused())), refused()), &state(refused())));
    }

    /// A session's batches start at the smallest size, double while no codeunit of the batch before ran again, and fall
    /// back to the smallest when one did, so a change made in the middle of a batch costs the codeunits of that batch
    /// after it, not those of a batch twice as large.
    #[test]
    fn the_batches_of_a_session_double_and_fall_back_when_a_codeunit_ran_again() {
        let key = (0, false);
        let mut b = Batches::of(vec![(key, (0..100).collect())], 4, 16);
        let none = std::collections::HashSet::new();
        let again = std::collections::HashSet::from([key]);
        let sizes: Vec<usize> = [&none, &none, &again, &none, &none, &none]
            .into_iter()
            .map(|feedback| {
                let n = b.next().len();
                b.feedback(feedback);
                n
            })
            .collect();
        assert_eq!(sizes, vec![4, 8, 16, 4, 8, 16]);
        assert!(!b.is_done());
        let rest: usize = std::iter::from_fn(|| Some(b.next().len())).take_while(|n| *n > 0).sum();
        assert_eq!(rest, 100 - sizes.iter().sum::<usize>());
        assert!(b.is_done());
    }

    /// A session that did not take part in a batch keeps its size, and sessions go in batches side by side.
    #[test]
    fn the_batches_of_two_sessions_go_side_by_side() {
        let mut b = Batches::of(
            vec![((0, false), vec![0, 1, 2]), ((1, false), vec![10, 11, 12, 13])],
            2,
            8,
        );
        assert_eq!(b.next(), vec![0, 1, 10, 11]);
        b.feedback(&std::collections::HashSet::new());
        assert_eq!(b.next(), vec![2, 12, 13]);
    }
}
