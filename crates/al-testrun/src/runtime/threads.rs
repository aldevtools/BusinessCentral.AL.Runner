//! The worker threads of the parallel part of a run (ADR-066 amendments 4 and 10): the codeunits outside the carry
//! chain run on a session each, from the install baseline, so any thread may take any of them.
//!
//! The calling thread is the first worker. It starts another only while a codeunit is still waiting and the memory
//! pool has room for one more ([`Pool::try_add`]), and checks again each time it finished a codeunit. A worker other
//! than the first that finds no room after a codeunit ends ([`Pool::keep`]) and drops its baseline copy; the first one
//! always goes on, so the run always moves on.

use super::TEST_STACK;
use super::memory::Pool;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Runs `run(baseline, k)` for every `k` below `n`, on the calling thread with `first` and on worker threads that
/// each make their own baseline with `own`; the results in the order of `k`.
pub(super) fn run_parallel<R: Send>(
    pool: &Pool,
    n: usize,
    first: Option<&al_vm::Baseline>,
    own: &(dyn Fn() -> Option<al_vm::Baseline> + Sync),
    run: &(dyn Fn(Option<&al_vm::Baseline>, usize) -> R + Sync),
) -> Vec<R> {
    let next = AtomicUsize::new(0);
    let mut done: Vec<(usize, R)> = std::thread::scope(|scope| {
        let mut handles = Vec::new();
        let mut done = Vec::new();
        loop {
            // Another worker for each waiting codeunit that fits beside what runs now (the first one counted).
            let mut added = 0;
            while next.load(Ordering::Relaxed) + added + 1 < n {
                let Some(member) = pool.try_add(added) else { break };
                let next = &next;
                let spawned = std::thread::Builder::new()
                    .stack_size(TEST_STACK)
                    .spawn_scoped(scope, move || {
                        // Declared first, so it drops last: what this worker's AL code made is released at its end.
                        let _run_end = al_builtins::instance_cycles::RunEnd::new();
                        let _member = member;
                        let own = own();
                        let mut done = Vec::new();
                        loop {
                            let k = next.fetch_add(1, Ordering::Relaxed);
                            if k >= n {
                                return done;
                            }
                            let _running = pool.enter();
                            done.push((k, run(own.as_ref(), k)));
                            drop(_running);
                            if !pool.keep() {
                                return done;
                            }
                        }
                    });
                match spawned {
                    Ok(h) => handles.push(h),
                    // No room for another thread (an address space limit): fewer will do.
                    Err(_) => break,
                }
                added += 1;
            }
            let k = next.fetch_add(1, Ordering::Relaxed);
            if k >= n {
                break;
            }
            let _running = pool.enter();
            done.push((k, run(first, k)));
        }
        for h in handles {
            done.extend(h.join().expect("a test worker panicked"));
        }
        done
    });
    // Results in the order of the codeunits, whichever worker finished first.
    done.sort_by_key(|(k, _)| *k);
    done.into_iter().map(|(_, r)| r).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::memory::MemoryBudget;
    use std::sync::Arc;
    use std::sync::atomic::AtomicU64;

    /// Every codeunit runs once and the results keep their order, whatever the threads did.
    #[test]
    fn every_codeunit_runs_once_in_order_without_a_budget() {
        let pool = Pool::open(4);
        let out = run_parallel(&pool, 50, None, &|| None, &|_, k| k * 2);
        assert_eq!(out, (0..50).map(|k| k * 2).collect::<Vec<_>>());
        assert!(pool.report_into(Default::default()).most <= 4);
    }

    /// ADR-066 amendment 10, decisions 1 and 3: with no room after the first codeunit the run stays on one thread, and
    /// a worker that finds no room ends while the first thread finishes the work.
    #[test]
    fn with_no_room_the_first_thread_runs_everything() {
        let rss = Arc::new(AtomicU64::new(100 << 20));
        let seen = rss.clone();
        let budget = MemoryBudget {
            limit: 1000 << 20,
            source: "--max-memory".into(),
            rss: Arc::new(move || Some(seen.load(Ordering::SeqCst))),
            hard: false,
        };
        let pool = Pool::new(Some(budget), 100 << 20, 8);
        let first = std::thread::current().id();
        let out = run_parallel(&pool, 20, None, &|| None, &|_, k| {
            // The first codeunit grows the process to 950 MB: nine tenths of the limit leave no room.
            rss.store(950 << 20, Ordering::SeqCst);
            (k, std::thread::current().id() == first)
        });
        assert_eq!(out.len(), 20);
        assert!(out.iter().all(|(_, on_first)| *on_first), "{out:?}");
        assert_eq!(pool.report_into(Default::default()).most, 1);
    }
}
