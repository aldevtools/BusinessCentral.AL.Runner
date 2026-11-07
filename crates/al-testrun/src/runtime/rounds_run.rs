//! The parallel part of a corpus run, in rounds (ADR-066 amendment 1, `single_walk.rs`).
//!
//! The test codeunits that are not in the carry chain run on worker threads, a session each. A codeunit that reads
//! SingleInstance state an earlier codeunit left (a cache Change Log Management filled, a binding of a SingleInstance
//! subscriber) saw none of it in a run of its own and has to run again from the serial state, which costs a second run.
//! So the codeunits go in rounds, the first few of each BC session first: a round starts from the state the rounds before
//! it left, as [`SingleWalk::predict`] gives it (a prediction: `walk` checks every run against the serial state and runs
//! again those that saw another one). The worker threads, each with its own copy of the install baseline, stay for all
//! rounds.

use super::chain_walk::Carry as ChainCarry;
use super::corpus::observed_detail;
use super::single_walk::{SingleWalk, rounds};
use super::*;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;

/// What the rounds run with.
pub(super) struct RoundRun<'a> {
    pub engine: &'a Arc<Engine>,
    pub lowered: &'a Lowered,
    pub baseline: Option<&'a al_vm::Baseline>,
    pub carried: &'a [ChainCarry],
    pub sees: &'a dyn Fn(usize, u32, u8, &Unit) -> bool,
    pub place: &'a dyn Fn(usize, u32) -> u32,
    pub limit: Option<std::time::Duration>,
    /// Worker threads including this one.
    pub workers: usize,
    /// Keep what `--explain` shows (runs on this thread only).
    pub trace: bool,
    /// The install baseline's snapshot for the workers' copies.
    pub snapshot: Option<&'a [u8]>,
    /// A worker thread's own copy of the install baseline, from the snapshot.
    pub thread_baseline: &'a (dyn Fn(Option<&[u8]>) -> Option<al_vm::Baseline> + Sync),
}

/// A round: the codeunits to run (clones, with the state they start from), and the next one a thread takes.
struct Job {
    units: Vec<Unit>,
    next: AtomicUsize,
}

/// Takes the codeunits of `job` in turn and runs them on a session each.
fn pull(
    engine: &Arc<Engine>,
    lowered: &Lowered,
    limit: Option<std::time::Duration>,
    job: &Job,
    baseline: Option<&al_vm::Baseline>,
) -> Vec<(usize, UnitResult)> {
    let mut done = Vec::new();
    loop {
        let k = job.next.fetch_add(1, Ordering::Relaxed);
        let Some(u) = job.units.get(k) else {
            return done;
        };
        done.push((k, run_unit(engine, lowered, baseline, u, None, limit)));
    }
}

impl RoundRun<'_> {
    /// Runs the codeunits `rest_all` (indexes of `units`) in rounds. Returns them in the order they ran with their
    /// results. `units` get the state each round starts from; `predicted` keeps the runs again the prediction made.
    pub fn run(
        &self,
        units: &mut [Unit],
        rest_all: &[usize],
        slots: &[Option<UnitResult>],
        predicted: &std::cell::RefCell<HashMap<usize, UnitResult>>,
        details: &mut HashMap<(usize, u32, String), explain::TestDetail>,
    ) -> (Vec<usize>, Vec<UnitResult>) {
        let batches = rounds(units, rest_all, self.place);
        let mut rest: Vec<usize> = Vec::new();
        let mut results: Vec<UnitResult> = Vec::new();
        let threads = if self.workers > 1 && rest_all.len() >= 2 {
            self.workers.min(rest_all.len())
        } else {
            1
        };
        std::thread::scope(|scope| {
            let (done_tx, done_rx) = mpsc::channel::<Vec<(usize, UnitResult)>>();
            let mut senders: Vec<mpsc::Sender<Arc<Job>>> = Vec::new();
            let (engine, lowered, limit) = (self.engine, self.lowered, self.limit);
            let (thread_baseline, snapshot) = (self.thread_baseline, self.snapshot);
            for _ in 1..threads {
                let (tx, rx) = mpsc::channel::<Arc<Job>>();
                let done_tx = done_tx.clone();
                let spawned = std::thread::Builder::new()
                    .stack_size(TEST_STACK)
                    .spawn_scoped(scope, move || {
                        let own = thread_baseline(snapshot);
                        for job in rx {
                            let done = pull(engine, lowered, limit, &job, own.as_ref());
                            if done_tx.send(done).is_err() {
                                break;
                            }
                        }
                    });
                match spawned {
                    Ok(_) => senders.push(tx),
                    // No room for another thread (an address space limit): fewer will do.
                    Err(_) => break,
                }
            }
            drop(done_tx);
            for (round, batch) in batches.into_iter().enumerate() {
                if round > 0 {
                    let seeds = self.seeds(units, &rest, &results, slots, predicted);
                    for &k in &batch {
                        if !units[k].execute {
                            units[k].singletons = seeds.get(&(units[k].si, units[k].hub_leg)).cloned();
                        }
                    }
                }
                let out: Vec<UnitResult> = if senders.is_empty() || batch.len() < 2 {
                    batch
                        .iter()
                        .map(|&k| &units[k])
                        .map(|u| {
                            let mut observe = |name: &str, _: &Outcome, seen: &Observed| {
                                details.insert((u.si, u.id, name.to_lowercase()), observed_detail(self.lowered, seen));
                            };
                            let trace: Option<&mut Observer> = self.trace.then_some(&mut observe);
                            run_unit(self.engine, self.lowered, self.baseline, u, trace, self.limit)
                        })
                        .collect()
                } else {
                    let job = Arc::new(Job {
                        units: batch.iter().map(|&k| units[k].clone()).collect(),
                        next: AtomicUsize::new(0),
                    });
                    let sent = senders.iter().filter(|tx| tx.send(job.clone()).is_ok()).count();
                    let mut done = pull(engine, lowered, limit, &job, self.baseline);
                    for _ in 0..sent {
                        done.extend(done_rx.recv().expect("a test worker panicked"));
                    }
                    // Results in the order of the units, whichever worker finished first.
                    done.sort_by_key(|(k, _)| *k);
                    done.into_iter().map(|(_, r)| r).collect()
                };
                rest.extend(batch);
                results.extend(out);
            }
            drop(senders);
        });
        (rest, results)
    }

    /// The single instances each BC session's codeunits of the rounds so far left, as the serial run has them.
    fn seeds(
        &self,
        units: &[Unit],
        rest: &[usize],
        results: &[UnitResult],
        slots: &[Option<UnitResult>],
        predicted: &std::cell::RefCell<HashMap<usize, UnitResult>>,
    ) -> HashMap<(usize, bool), Arc<SingleState>> {
        let none = HashSet::new();
        SingleWalk {
            engine: self.engine,
            lowered: self.lowered,
            baseline: self.baseline,
            units,
            carried: self.carried,
            sees: self.sees,
            place: self.place,
            limit: self.limit,
            leak_reran: &none,
            predicted,
        }
        .predict(rest, results, slots)
    }
}
