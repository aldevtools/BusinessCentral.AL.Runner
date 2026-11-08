//! The memory budget of a test run (owner decision 2026-11-08): the run fits in the memory it may use by running fewer
//! test codeunits at once, down to one at a time, instead of being killed for running out of it.
//!
//! The front end works out the limit (free memory, the cgroup's `memory.max`, `--max-memory`) and hands it in with a
//! way to read the process's resident memory ([`MemoryBudget`]); this crate reads no OS state for it. The run then:
//!
//! 1. reads the resident memory once the apps are compiled, and stops ([`MemoryBudget::stop`]) when not even one test
//!    thread fits beside it;
//! 2. reads it again once the install baseline is loaded: that is the fixed part. The thread count is what fits in
//!    nine tenths of the limit beside it at [`PER_WORKER_START`] each, at least one, at most the threads asked for
//!    ([`workers_for`]);
//! 3. before a thread starts the next test codeunit, checks that the resident memory leaves room for one more
//!    ([`Gate`]), at the cost a running codeunit has been seen to hold (what the process grew beyond the fixed part,
//!    per running codeunit). When it does not, the thread waits for a running one to finish; one codeunit always runs,
//!    so the run always moves on. A tenth of the limit stays free: a codeunit can grow past the cost seen so far.
//!
//! Only how many codeunits run at once changes: each still runs on a session of its own from the same baseline, so the
//! outcomes are those of any other thread count (ADR-009: scheduling is internal; ADR-066's parallel run is unchanged).

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

/// The least memory a test thread needs on top of the fixed part, in bytes: below it the run stops at start.
pub const WORKER_FLOOR: u64 = 64 << 20;

/// The cost of a test thread assumed until one is seen, in bytes. Measured (BC 29, release, 12 cores, 2026-11-08): each
/// thread from 2 to 12 added 150 to 200 MB to the peak of Tests-ERM-Purchase (`docs/adr/066` amendment 7).
pub const PER_WORKER_START: u64 = 200 << 20;

/// The part of the limit the gate fills: a tenth stays free for a codeunit that grows past the cost seen so far.
fn usable(limit: u64) -> u64 {
    limit / 10 * 9
}

/// Reads the process's resident memory in bytes.
pub type ReadResident = Arc<dyn Fn() -> Option<u64> + Send + Sync>;
/// Stops the run with a message.
pub type StopRun = Arc<dyn Fn(&str) + Send + Sync>;

/// The limit of a run and how to read the process's resident memory, from the front end.
#[derive(Clone)]
pub struct MemoryBudget {
    /// The most bytes the process may hold.
    pub limit: u64,
    /// Where the limit comes from, for `--timings` and the stop message (`--max-memory`, `memory.max`, free memory).
    pub source: String,
    /// The process's resident memory now, in bytes; `None` where it cannot be read (the run is then not gated).
    pub rss: ReadResident,
    /// Called with the message when not even one test thread fits; the command line prints it and exits. Without it
    /// (or when it returns) the run goes on with one thread.
    pub stop: Option<StopRun>,
}

impl std::fmt::Debug for MemoryBudget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoryBudget")
            .field("limit", &self.limit)
            .field("source", &self.source)
            .finish_non_exhaustive()
    }
}

/// What the budget did in a run (`--timings`).
#[derive(Debug, Clone, Default)]
pub struct MemoryReport {
    /// The limit in bytes and where it comes from; `None`: no budget.
    pub limit: Option<(u64, String)>,
    /// Resident bytes once the apps were compiled.
    pub after_compile: Option<u64>,
    /// Resident bytes once the install baseline was loaded (the fixed part).
    pub after_baseline: Option<u64>,
    /// The estimated cost of one test thread, in bytes.
    pub per_worker: u64,
    /// The threads asked for (cores or `AL_TEST_THREADS`) and the threads chosen.
    pub asked: usize,
    pub workers: usize,
    /// How many times a thread waited for memory before it started a codeunit.
    pub waits: usize,
    /// How many times optional caches were dropped because memory was tight.
    pub caches_dropped: usize,
}

impl MemoryReport {
    /// The `--timings` line; `None` without a budget.
    pub fn line(&self) -> Option<String> {
        let (limit, source) = self.limit.as_ref()?;
        let mb = |b: u64| b >> 20;
        let at = |b: Option<u64>| b.map_or("?".to_string(), |b| format!("{} MB", mb(b)));
        Some(format!(
            "memory: budget {} MB ({source}); resident after compile {}, after install baseline {}; about {} MB per \
             test thread; test threads {} of {}; waited for memory {} times, dropped caches {} times",
            mb(*limit),
            at(self.after_compile),
            at(self.after_baseline),
            mb(self.per_worker),
            self.workers,
            self.asked,
            self.waits,
            self.caches_dropped
        ))
    }
}

/// How many test threads fit: `(limit - fixed) / per_worker`, at least one, at most `asked`.
pub fn workers_for(limit: u64, fixed: u64, per_worker: u64, asked: usize) -> usize {
    let room = limit.saturating_sub(fixed) / per_worker.max(1);
    (room.min(asked as u64) as usize).max(1)
}

/// The message when not even one test thread fits beside `resident` bytes; `None` when one does.
pub fn below_minimum(budget: &MemoryBudget, resident: u64, when: &str) -> Option<String> {
    let need = resident + WORKER_FLOOR;
    (need > budget.limit).then(|| {
        format!(
            "not enough memory to run the tests: {when} the process holds {} MB, and one test thread needs at least \
             {} MB more, about {} MB in all, but the limit is {} MB ({}). Free memory, or raise the limit with \
             --max-memory.",
            resident >> 20,
            WORKER_FLOOR >> 20,
            need >> 20,
            budget.limit >> 20,
            budget.source
        )
    })
}

/// The budget of one run phase: the thread count, worked out after the compile and again after the install baseline.
pub(crate) struct Plan {
    budget: Option<MemoryBudget>,
    report: MemoryReport,
    gate: Gate,
}

impl Plan {
    /// Once the apps are compiled: one thread for a traced or debugged run (ADR-048 decision 3), else `asked`, fewer when
    /// the budget cannot hold that many test threads of [`WORKER_FLOOR`] beside what the process holds now.
    pub(crate) fn after_compile(opts: &super::RunOptions, asked: usize) -> Plan {
        let asked = if opts.trace || opts.hooks.debugger.is_some() {
            1
        } else {
            asked
        };
        let mut report = MemoryReport {
            asked,
            workers: asked,
            per_worker: PER_WORKER_START,
            ..Default::default()
        };
        let budget = opts.memory.clone();
        if let Some(b) = &budget {
            report.limit = Some((b.limit, b.source.clone()));
            report.after_compile = (b.rss)();
            if let Some(now) = report.after_compile {
                stop_when_below(b, now, "once the apps are compiled");
                report.workers = workers_for(usable(b.limit), now, PER_WORKER_START, asked);
            }
        }
        let gate = Gate::new(budget.clone(), report.after_compile.unwrap_or(0), PER_WORKER_START);
        Plan { budget, report, gate }
    }

    pub(crate) fn workers(&self) -> usize {
        self.report.workers
    }

    /// Once the install baseline is loaded: what the process holds now is the fixed part; as many threads as fit beside
    /// it at [`PER_WORKER_START`] each. Returns the thread count.
    pub(crate) fn after_baseline(&mut self) -> usize {
        let Some(b) = &self.budget else {
            return self.report.workers;
        };
        self.report.after_baseline = (b.rss)();
        if let Some(now) = self.report.after_baseline {
            stop_when_below(b, now, "once the install baseline is loaded");
            self.report.workers = workers_for(usable(b.limit), now, PER_WORKER_START, self.report.workers);
            self.gate = Gate::new(self.budget.clone(), now, PER_WORKER_START);
        }
        self.report.workers
    }

    /// The gate the test threads enter before each codeunit.
    pub(crate) fn gate(&self) -> &Gate {
        &self.gate
    }

    /// What the budget did.
    pub(crate) fn report(&self) -> MemoryReport {
        MemoryReport {
            waits: self.gate.waits(),
            per_worker: self.gate.per_worker(),
            caches_dropped: self.gate.dropped.load(Ordering::Relaxed),
            ..self.report.clone()
        }
    }
}

/// Stops the run ([`MemoryBudget::stop`]) when not even one test thread fits beside `resident` bytes. Without a stop, or
/// when it returns, the message goes to stderr and the run goes on with one thread.
fn stop_when_below(b: &MemoryBudget, resident: u64, when: &str) {
    if let Some(message) = below_minimum(b, resident, when) {
        match &b.stop {
            Some(stop) => stop(&message),
            None => eprintln!("al-runner: {message}"),
        }
    }
}

/// Admission of test codeunits by memory: a thread [`Gate::enter`]s before it runs a codeunit and leaves when the
/// returned [`Pass`] is dropped. Without a budget every thread enters at once.
pub struct Gate {
    budget: Option<MemoryBudget>,
    /// Resident bytes with no codeunit running (the fixed part), and the cost of one running codeunit: at least the
    /// start value, else what the process grew beyond `fixed` per running codeunit when last seen.
    fixed: u64,
    start: u64,
    per_worker: AtomicU64,
    running: Mutex<usize>,
    done: Condvar,
    waits: AtomicUsize,
    /// Set when a thread had to wait; the owner of an optional cache drops it ([`Gate::take_tight`]).
    tight: AtomicBool,
    /// How many times an optional cache was dropped for it ([`Gate::dropped_cache`]).
    dropped: AtomicUsize,
}

impl Gate {
    /// A gate that lets every thread in.
    pub fn open() -> Gate {
        Gate::new(None, 0, 0)
    }

    pub fn new(budget: Option<MemoryBudget>, fixed: u64, per_worker: u64) -> Gate {
        Gate {
            budget,
            fixed,
            start: per_worker,
            per_worker: AtomicU64::new(per_worker),
            running: Mutex::new(0),
            done: Condvar::new(),
            waits: AtomicUsize::new(0),
            tight: AtomicBool::new(false),
            dropped: AtomicUsize::new(0),
        }
    }

    /// Waits until memory leaves room for one more codeunit, or until none runs.
    pub fn enter(&self) -> Pass<'_> {
        let mut running = self.running.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(b) = &self.budget {
            let mut waited = false;
            while *running > 0 && !self.fits(b, *running) {
                if !waited {
                    waited = true;
                    self.waits.fetch_add(1, Ordering::Relaxed);
                }
                self.tight.store(true, Ordering::Relaxed);
                running = self
                    .done
                    .wait_timeout(running, Duration::from_millis(20))
                    .unwrap_or_else(|e| e.into_inner())
                    .0;
            }
        }
        *running += 1;
        Pass { gate: self }
    }

    /// Whether one more codeunit fits beside `running` ones in [`usable`] memory: the larger of the resident memory and
    /// the estimate of what the running ones hold (one just let in has not grown yet), plus one more.
    fn fits(&self, b: &MemoryBudget, running: usize) -> bool {
        let Some(now) = (b.rss)() else { return true };
        let seen = now.saturating_sub(self.fixed) / running.max(1) as u64;
        let per = seen.max(self.start);
        self.per_worker.store(per, Ordering::Relaxed);
        now.max(self.fixed + per * running as u64) + per <= usable(b.limit)
    }

    /// Whether a thread waited for memory since the last call.
    pub fn take_tight(&self) -> bool {
        self.tight.swap(false, Ordering::Relaxed)
    }

    /// Notes that an optional cache was dropped because a thread waited for memory.
    pub fn dropped_cache(&self) {
        self.dropped.fetch_add(1, Ordering::Relaxed);
    }

    /// The cost of one running codeunit the gate assumes now, in bytes.
    pub fn per_worker(&self) -> u64 {
        self.per_worker.load(Ordering::Relaxed)
    }

    pub fn waits(&self) -> usize {
        self.waits.load(Ordering::Relaxed)
    }
}

/// A codeunit let in by a [`Gate`]; dropping it lets the next one in.
pub struct Pass<'a> {
    gate: &'a Gate,
}

impl Drop for Pass<'_> {
    fn drop(&mut self) {
        let mut running = self.gate.running.lock().unwrap_or_else(|e| e.into_inner());
        *running = running.saturating_sub(1);
        self.gate.done.notify_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MB: u64 = 1 << 20;

    fn budget(limit: u64, rss: Arc<AtomicUsize>) -> MemoryBudget {
        MemoryBudget {
            limit,
            source: "--max-memory".into(),
            rss: Arc::new(move || Some(rss.load(Ordering::SeqCst) as u64)),
            stop: None,
        }
    }

    #[test]
    fn workers_fit_the_room_left_by_the_fixed_part() {
        assert_eq!(workers_for(6000 * MB, 2000 * MB, 500 * MB, 12), 8);
        assert_eq!(
            workers_for(6000 * MB, 2000 * MB, 100 * MB, 12),
            12,
            "never more than asked"
        );
        assert_eq!(workers_for(2100 * MB, 2000 * MB, 500 * MB, 12), 1, "at least one");
        assert_eq!(
            workers_for(1000 * MB, 2000 * MB, 500 * MB, 12),
            1,
            "over the limit: one"
        );
        assert_eq!(workers_for(1000 * MB, 0, 0, 4), 4, "no cost known: as asked");
    }

    #[test]
    fn below_minimum_names_what_is_needed_and_the_limit() {
        let b = budget(1000 * MB, Arc::default());
        assert!(below_minimum(&b, 900 * MB, "after the compile").is_none());
        let m = below_minimum(&b, 990 * MB, "after the compile").expect("does not fit");
        assert!(
            m.contains("990 MB") && m.contains("1000 MB") && m.contains("--max-memory"),
            "{m}"
        );
    }

    #[test]
    fn a_thread_waits_while_memory_is_tight_and_one_always_runs() {
        let rss = Arc::new(AtomicUsize::new((980 * MB) as usize));
        let gate = Gate::new(Some(budget(1000 * MB, rss.clone())), 0, 50 * MB);
        // Nothing runs: the first codeunit is let in even over the limit.
        let first = gate.enter();
        assert_eq!(gate.waits(), 0);
        std::thread::scope(|s| {
            let second = s.spawn(|| {
                let _p = gate.enter();
            });
            std::thread::sleep(Duration::from_millis(100));
            assert!(!second.is_finished(), "waits while 980 + 50 MB exceed 1000 MB");
            drop(first);
            second.join().unwrap();
        });
        assert_eq!(gate.waits(), 1);
        assert!(gate.take_tight());
        assert!(!gate.take_tight());
    }

    #[test]
    fn a_thread_enters_at_once_when_memory_has_room() {
        let rss = Arc::new(AtomicUsize::new((100 * MB) as usize));
        let gate = Gate::new(Some(budget(1000 * MB, rss)), 100 * MB, 50 * MB);
        let _a = gate.enter();
        let _b = gate.enter();
        let _c = gate.enter();
        assert_eq!(gate.waits(), 0);
    }

    #[test]
    fn the_cost_of_a_thread_is_learned_from_what_the_running_ones_hold() {
        // One codeunit runs and the process grew from 100 MB to 1100 MB: a second would need about as much again.
        let rss = Arc::new(AtomicUsize::new((100 * MB) as usize));
        let gate = Gate::new(Some(budget(2000 * MB, rss.clone())), 100 * MB, 64 * MB);
        let first = gate.enter();
        rss.store((1100 * MB) as usize, Ordering::SeqCst);
        std::thread::scope(|s| {
            let second = s.spawn(|| drop(gate.enter()));
            std::thread::sleep(Duration::from_millis(100));
            assert!(!second.is_finished(), "1100 + 1000 MB exceed the 2000 MB limit");
            drop(first);
            second.join().unwrap();
        });
        assert_eq!(gate.per_worker() >> 20, 1000);
    }

    #[test]
    fn a_tenth_of_the_limit_is_kept_free() {
        let rss = Arc::new(AtomicUsize::new((850 * MB) as usize));
        let gate = Gate::new(Some(budget(1000 * MB, rss)), 850 * MB, 64 * MB);
        let _first = gate.enter();
        std::thread::scope(|s| {
            let second = s.spawn(|| drop(gate.enter()));
            std::thread::sleep(Duration::from_millis(100));
            assert!(!second.is_finished(), "850 + 64 MB pass 90 % of 1000 MB");
            drop(_first);
            second.join().unwrap();
        });
    }

    #[test]
    fn the_first_thread_count_assumes_the_measured_cost_of_a_thread() {
        let rss = Arc::new(AtomicUsize::new((1000 * MB) as usize));
        let opts = crate::runtime::RunOptions {
            memory: Some(budget(3000 * MB, rss)),
            ..Default::default()
        };
        let mut plan = Plan::after_compile(&opts, 12);
        // 90 % of 3000 MB less 1000 MB held, at PER_WORKER_START (200 MB) each.
        assert_eq!(plan.after_baseline(), 8);
        let plan = Plan::after_compile(&crate::runtime::RunOptions::default(), 12);
        assert_eq!(plan.workers(), 12, "no budget: as asked");
    }

    #[test]
    fn an_open_gate_never_waits() {
        let gate = Gate::open();
        let _a = gate.enter();
        let _b = gate.enter();
        assert_eq!(gate.waits(), 0);
    }
}
