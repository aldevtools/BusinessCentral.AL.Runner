//! The memory limit of a test run (ADR-066 amendments 9 and 10): the run holds the limit by running more of its work
//! one after the other, and stops when even one test thread cannot fit.
//!
//! The front end works out the limit (free memory, the cgroup's `memory.max`, `--max-memory`) and hands it in with a
//! way to read the process's resident memory ([`MemoryBudget`]); this crate reads no OS state for it. The run then:
//!
//! 1. reads the resident memory once the apps are compiled and once the install baseline is loaded (the fixed part),
//!    and stops when not even one test thread fits beside it ([`below_minimum`]);
//! 2. starts with one test thread. Another starts only when there is work it can use (the caller's part) and the
//!    resident memory plus the cost of one codeunit stays under nine tenths of the limit ([`Pool::try_add`]). The cost
//!    is learned in the run: the largest growth above the fixed part per running codeunit, seen from the first codeunit
//!    on, at least [`WORKER_FLOOR`]; until the first codeunit ends only one thread runs. Nothing is kept between runs;
//! 3. a thread other than the first that finishes a codeunit when there is no room ends ([`Pool::keep`]) and gives
//!    back its session, baseline copy and runs ahead; it never waits holding them. One thread always runs;
//! 4. under a hard limit ([`MemoryBudget::hard`]: `al-runner test`), stops the run ([`Halt`]) before the resident
//!    memory passes the limit: a watcher samples it while the tests run. The running tests are stopped, the tests not
//!    reached are reported as not run with the reason [`Halt::reason`], and the front end exits with code 2.
//!
//! Only which thread runs a codeunit, and when, changes: each codeunit still runs on a session of its own from the same
//! baseline, so the outcomes are those of any other thread count (ADR-009: scheduling is internal).

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// The least memory a test thread needs on top of the fixed part, in bytes: below it the run stops at start, and the
/// learned cost of a codeunit is never taken as less.
pub const WORKER_FLOOR: u64 = 64 << 20;

/// How the reason of a test not reached starts when the run stopped at its hard limit ([`Halt::reason`]).
pub const REASON: &str = "memory limit (";

/// How often the watcher reads the resident memory while the tests run.
const SAMPLE: Duration = Duration::from_millis(10);

/// The part of the limit new threads may fill: a tenth stays free for a codeunit that grows past the cost seen so far.
fn usable(limit: u64) -> u64 {
    limit / 10 * 9
}

/// Where a hard limit stops the run with one test thread left: an eighth of the limit (at least 64 MB) under it. The
/// process still grows after the stop until the running test has ended: on Tests-SCM by up to about 165 MB after a
/// stop at 2880 MB (it grew by about 450 MB a second there).
fn stop_line(limit: u64) -> u64 {
    limit.saturating_sub((limit / 8).max(64 << 20))
}

/// Where a hard limit stops the run while more threads run: closer to the limit (a thirty-second, at least 32 MB),
/// since the threads other than the first end after their codeunit when there is no room (`Pool::keep`), and so free
/// memory without a stop.
fn last_line(limit: u64) -> u64 {
    limit.saturating_sub((limit / 32).max(32 << 20))
}

/// Reads the process's resident memory in bytes.
pub type ReadResident = Arc<dyn Fn() -> Option<u64> + Send + Sync>;

/// The limit of a run and how to read the process's resident memory, from the front end.
#[derive(Clone)]
pub struct MemoryBudget {
    /// The most bytes the process may hold.
    pub limit: u64,
    /// Where the limit comes from, for `--timings` and the stop message (`--max-memory`, `memory.max`, free memory).
    pub source: String,
    /// The process's resident memory now, in bytes; `None` where it cannot be read (the run is then not limited).
    pub rss: ReadResident,
    /// A hard limit (ADR-066 amendment 10, decision 6; `al-runner test`): a run that cannot fit stops instead of
    /// going over. `false` (`serve`, `debug`): the run goes on with one thread.
    pub hard: bool,
}

impl std::fmt::Debug for MemoryBudget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoryBudget")
            .field("limit", &self.limit)
            .field("source", &self.source)
            .field("hard", &self.hard)
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
    /// The learned cost of one running codeunit, in bytes (0: none ended).
    pub per_worker: u64,
    /// The threads asked for (cores or `AL_TEST_THREADS`).
    pub asked: usize,
    /// The most test threads that ran at once, and how many ran at the end.
    pub most: usize,
    pub at_end: usize,
    /// How many threads ended because there was no room for another codeunit.
    pub ended: usize,
    /// Why the run stopped for its hard limit (the message the front end prints before it exits with code 2).
    pub halted: Option<String>,
}

impl MemoryReport {
    /// The `--timings` line; `None` without a budget.
    pub fn line(&self) -> Option<String> {
        let (limit, source) = self.limit.as_ref()?;
        let mb = |b: u64| b >> 20;
        let at = |b: Option<u64>| b.map_or("?".to_string(), |b| format!("{} MB", mb(b)));
        Some(format!(
            "memory: budget {} MB ({source}); resident after compile {}, after install baseline {}; about {} MB per \
             test thread; test threads at most {} of {}, {} at the end, {} ended for memory{}",
            mb(*limit),
            at(self.after_compile),
            at(self.after_baseline),
            mb(self.per_worker),
            self.most,
            self.asked,
            self.at_end,
            self.ended,
            if self.halted.is_some() {
                "; stopped at the limit"
            } else {
                ""
            }
        ))
    }
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

/// A run stopped for its hard memory limit: the flag the running tests' watchdogs read (`run.rs`), the reason the
/// tests not reached get, and the message the front end prints.
#[derive(Debug)]
pub struct Halt {
    flag: Arc<AtomicBool>,
    reason: String,
    message: Mutex<Option<String>>,
}

impl Halt {
    fn new(reason: String) -> Halt {
        Halt {
            flag: Arc::default(),
            reason,
            message: Mutex::new(None),
        }
    }

    /// Stops the run with `message`; the first stop wins.
    fn set(&self, message: String) {
        let mut m = self.message.lock().unwrap_or_else(|e| e.into_inner());
        if m.is_none() {
            *m = Some(message);
            self.flag.store(true, Ordering::SeqCst);
        }
    }

    pub fn is_set(&self) -> bool {
        self.flag.load(Ordering::Relaxed)
    }

    /// The reason of a test not reached: "memory limit (--max-memory 2048 MB)".
    pub fn reason(&self) -> &str {
        &self.reason
    }

    /// The flag set when the run stops: the watchdog of a running test stops it.
    pub fn flag(&self) -> Arc<AtomicBool> {
        self.flag.clone()
    }

    pub fn message(&self) -> Option<String> {
        self.message.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

/// The budget of one run phase: what the process holds after the compile and the install baseline, and the [`Pool`].
pub(crate) struct Plan {
    budget: Option<MemoryBudget>,
    report: MemoryReport,
    pool: Pool,
}

impl Plan {
    /// Once the apps are compiled: one thread for a traced or debugged run (ADR-048 decision 3), else at most `asked`.
    /// Stops the run when not even one thread fits beside what the process holds now.
    pub(crate) fn after_compile(opts: &super::RunOptions, asked: usize) -> Plan {
        let asked = if opts.trace || opts.hooks.debugger.is_some() {
            1
        } else {
            asked
        };
        let mut report = MemoryReport {
            asked,
            ..Default::default()
        };
        let budget = opts.memory.clone();
        let mut fixed = 0;
        if let Some(b) = &budget {
            report.limit = Some((b.limit, b.source.clone()));
            report.after_compile = (b.rss)();
            fixed = report.after_compile.unwrap_or(0);
        }
        let pool = Pool::new(budget.clone(), fixed, asked);
        if let (Some(b), Some(now)) = (&budget, report.after_compile) {
            pool.stop_when_below(b, now, "once the apps are compiled");
        }
        Plan { budget, report, pool }
    }

    /// The threads asked for: the most the run may use.
    pub(crate) fn workers(&self) -> usize {
        self.report.asked
    }

    /// Whether the run stopped for its hard limit.
    pub(crate) fn halted(&self) -> bool {
        self.pool.halted()
    }

    /// Once the install baseline is loaded: what the process holds now is the fixed part. Returns the most threads.
    pub(crate) fn after_baseline(&mut self) -> usize {
        if let Some(b) = &self.budget {
            self.report.after_baseline = (b.rss)();
            if let Some(now) = self.report.after_baseline {
                self.pool.s.fixed.store(now, Ordering::Relaxed);
                self.pool.stop_when_below(b, now, "once the install baseline is loaded");
            }
        }
        self.report.asked
    }

    /// The test threads of the run.
    pub(crate) fn pool(&self) -> &Pool {
        &self.pool
    }

    /// The hooks of the run's units: `hooks` with the run's [`Halt`], so that a stop reaches every running test.
    pub(crate) fn hooks(&self, hooks: &crate::events::Hooks) -> crate::events::Hooks {
        let mut h = hooks.clone();
        if self.budget.as_ref().is_some_and(|b| b.hard) {
            h.halt = Some(self.pool.halt());
        }
        h
    }

    /// What the budget did.
    pub(crate) fn report(&self) -> MemoryReport {
        self.pool.report_into(self.report.clone())
    }
}

/// The shared part of a [`Pool`], also read by its watcher thread.
struct Shared {
    budget: Option<MemoryBudget>,
    /// Resident bytes with no codeunit running (the fixed part).
    fixed: AtomicU64,
    /// The most threads the run may use.
    max: usize,
    /// The learned cost of one running codeunit; `learned` once a codeunit ended.
    cost: AtomicU64,
    learned: AtomicBool,
    /// Test threads alive (the first one included), the most at once, the ones ended for memory, codeunits running.
    live: AtomicUsize,
    most: AtomicUsize,
    ended: AtomicUsize,
    running: AtomicUsize,
    halt: Arc<Halt>,
}

/// The test threads of a run and the memory they may use (ADR-066 amendment 10). The thread that creates it is the
/// first one; [`Pool::try_add`] lets another start, [`Pool::keep`] tells one whether it goes on, [`Pool::enter`] counts a
/// running codeunit. Under a hard limit [`Pool::watch`] samples the resident memory while the tests run.
pub struct Pool {
    s: Arc<Shared>,
    watcher: Mutex<Option<(std::sync::mpsc::Sender<()>, std::thread::JoinHandle<()>)>>,
}

impl Pool {
    /// A pool without a budget for `max` threads.
    pub fn open(max: usize) -> Pool {
        Pool::new(None, 0, max)
    }

    pub fn new(budget: Option<MemoryBudget>, fixed: u64, max: usize) -> Pool {
        let reason = budget
            .as_ref()
            .map_or(String::new(), |b| format!("{REASON}{} {} MB)", b.source, b.limit >> 20));
        Pool {
            s: Arc::new(Shared {
                budget,
                fixed: AtomicU64::new(fixed),
                max: max.max(1),
                cost: AtomicU64::new(0),
                learned: AtomicBool::new(false),
                live: AtomicUsize::new(1),
                most: AtomicUsize::new(1),
                ended: AtomicUsize::new(0),
                running: AtomicUsize::new(0),
                halt: Arc::new(Halt::new(reason)),
            }),
            watcher: Mutex::new(None),
        }
    }

    /// The test threads alive.
    pub fn live(&self) -> usize {
        self.s.live.load(Ordering::SeqCst)
    }

    /// The most threads the run may use.
    pub fn max(&self) -> usize {
        self.s.max
    }

    /// The learned cost of one running codeunit, in bytes.
    pub fn cost(&self) -> u64 {
        self.s.cost.load(Ordering::Relaxed)
    }

    pub fn halted(&self) -> bool {
        self.s.halt.is_set()
    }

    pub fn halt(&self) -> Arc<Halt> {
        self.s.halt.clone()
    }

    /// The resident memory now, when the budget can read it.
    fn resident(&self) -> Option<(&MemoryBudget, u64)> {
        let b = self.s.budget.as_ref()?;
        Some((b, (b.rss)()?))
    }

    /// Whether `next` more codeunits fit in nine tenths of the limit beside the resident memory, and the live threads
    /// and `extra` more fit at the learned cost of a codeunit each beside the fixed part (a thread just started has
    /// not grown yet). Without a budget (or where the resident memory cannot be read) always; with one, only once a
    /// codeunit ended (the cost is known).
    fn room(&self, next: usize, extra: usize) -> bool {
        let Some((b, now)) = self.resident() else {
            return true;
        };
        if !self.s.learned.load(Ordering::SeqCst) || self.halted() {
            return false;
        }
        let (fixed, cost) = (self.s.fixed.load(Ordering::Relaxed), self.cost());
        let threads = (self.live() + extra) as u64;
        (now + cost * next as u64).max(fixed + cost * threads) <= usable(b.limit)
    }

    /// Lets one more test thread start when the run may use another and its codeunit fits beside the resident memory
    /// and the `extra` threads the caller just started. The thread is counted until the returned [`Member`] drops.
    pub fn try_add(&self, extra: usize) -> Option<Member<'_>> {
        if self.live() >= self.s.max || !self.room(extra + 1, extra + 1) {
            return None;
        }
        let live = self.s.live.fetch_add(1, Ordering::SeqCst) + 1;
        self.s.most.fetch_max(live, Ordering::SeqCst);
        Some(Member { pool: self })
    }

    /// Asked by a thread other than the first after each codeunit: whether it takes another. `false`: there is no
    /// room, and the thread ends (decision 3), giving back what it holds.
    pub fn keep(&self) -> bool {
        if self.room(1, 0) {
            return true;
        }
        self.s.ended.fetch_add(1, Ordering::SeqCst);
        false
    }

    /// Counts a running codeunit until the returned guard drops; the cost is learned when it ends.
    pub fn enter(&self) -> Running<'_> {
        self.s.running.fetch_add(1, Ordering::SeqCst);
        Running { pool: self }
    }

    /// Reads the resident memory: learns the cost of a running codeunit, and under a hard limit stops the run before
    /// the resident memory passes the limit (the watcher calls it while the tests run).
    pub fn sample(&self) {
        let Some((b, now)) = self.resident() else { return };
        let running = self.s.running.load(Ordering::SeqCst);
        if running > 0 {
            let fixed = self.s.fixed.load(Ordering::Relaxed);
            let per = (now.saturating_sub(fixed) / running as u64).max(WORKER_FLOOR);
            self.s.cost.fetch_max(per, Ordering::Relaxed);
        }
        let line = if self.live() > 1 {
            last_line(b.limit)
        } else {
            stop_line(b.limit)
        };
        if b.hard && now > line {
            self.stop(
                b,
                format!(
                    "the run held {} MB with {} test thread(s) and was still growing",
                    now >> 20,
                    self.live()
                ),
            );
        }
    }

    /// A codeunit ended: the cost is known now. Under a hard limit, stops the run when the fixed part and the cost of
    /// one codeunit do not fit (decision 6, after the first codeunit). Only with one thread left: with more, the cost
    /// seen per running codeunit also holds what the other threads set up, and they end first when there is no room.
    fn ended(&self) {
        self.sample();
        self.s.learned.store(true, Ordering::SeqCst);
        let Some(b) = self.s.budget.as_ref().filter(|b| b.hard && self.live() == 1) else {
            return;
        };
        let need = self.s.fixed.load(Ordering::Relaxed) + self.cost();
        if need > stop_line(b.limit) {
            self.stop(
                b,
                format!(
                    "one test thread needs about {} MB (the fixed part and its largest codeunit)",
                    need >> 20
                ),
            );
        }
    }

    /// Under a hard limit: not even one test thread fits beside `resident` bytes, so the run stops before its tests.
    fn stop_when_below(&self, b: &MemoryBudget, resident: u64, when: &str) {
        if let Some(message) = below_minimum(b, resident, when) {
            if b.hard {
                self.s.halt.set(message);
            } else {
                eprintln!("al-runner: {message}");
            }
        }
    }

    fn stop(&self, b: &MemoryBudget, needed: String) {
        self.s.halt.set(format!(
            "not enough memory to run the tests: {needed}, but the limit is {} MB ({}). The tests not reached are \
             reported as not run. Free memory, or raise the limit with --max-memory.",
            b.limit >> 20,
            b.source
        ));
    }

    /// Starts the watcher that samples the resident memory ([`Pool::sample`]) until [`Pool::unwatch`] or the pool
    /// drops; only under a hard limit whose resident memory can be read.
    pub fn watch(&self) {
        if self.resident().is_none_or(|(b, _)| !b.hard) {
            return;
        }
        let mut w = self.watcher.lock().unwrap_or_else(|e| e.into_inner());
        if w.is_some() {
            return;
        }
        let (done, rx) = std::sync::mpsc::channel::<()>();
        let pool = Pool {
            s: self.s.clone(),
            watcher: Mutex::new(None),
        };
        let thread = std::thread::spawn(move || {
            while let Err(std::sync::mpsc::RecvTimeoutError::Timeout) = rx.recv_timeout(SAMPLE) {
                pool.sample();
            }
        });
        *w = Some((done, thread));
    }

    /// Stops the watcher.
    pub fn unwatch(&self) {
        if let Some((done, thread)) = self.watcher.lock().unwrap_or_else(|e| e.into_inner()).take() {
            let _ = done.send(());
            let _ = thread.join();
        }
    }

    /// `report` with what the pool saw.
    pub fn report_into(&self, report: MemoryReport) -> MemoryReport {
        MemoryReport {
            per_worker: self.cost(),
            most: self.s.most.load(Ordering::SeqCst),
            at_end: self.live(),
            ended: self.s.ended.load(Ordering::SeqCst),
            halted: self.s.halt.message(),
            ..report
        }
    }
}

impl Drop for Pool {
    fn drop(&mut self) {
        self.unwatch();
    }
}

/// A test thread other than the first, counted by its [`Pool`] while it lives.
pub struct Member<'a> {
    pool: &'a Pool,
}

impl Drop for Member<'_> {
    fn drop(&mut self) {
        self.pool.s.live.fetch_sub(1, Ordering::SeqCst);
    }
}

/// A running codeunit, counted by its [`Pool`] while it runs.
pub struct Running<'a> {
    pool: &'a Pool,
}

impl Drop for Running<'_> {
    fn drop(&mut self) {
        self.pool.ended();
        self.pool.s.running.fetch_sub(1, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MB: u64 = 1 << 20;

    fn budget(limit: u64, rss: Arc<AtomicU64>, hard: bool) -> MemoryBudget {
        MemoryBudget {
            limit,
            source: "--max-memory".into(),
            rss: Arc::new(move || Some(rss.load(Ordering::SeqCst))),
            hard,
        }
    }

    fn rss(mb: u64) -> Arc<AtomicU64> {
        Arc::new(AtomicU64::new(mb * MB))
    }

    #[test]
    fn below_minimum_names_what_is_needed_and_the_limit() {
        let b = budget(1000 * MB, rss(0), true);
        assert!(below_minimum(&b, 900 * MB, "after the compile").is_none());
        let m = below_minimum(&b, 990 * MB, "after the compile").expect("does not fit");
        assert!(
            m.contains("990 MB") && m.contains("1000 MB") && m.contains("--max-memory"),
            "{m}"
        );
    }

    /// ADR-066 amendment 10, decisions 1 and 2: the run starts with one thread; a second starts only once the first
    /// codeunit ended, and only when the cost learned from it fits.
    #[test]
    fn the_run_starts_one_thread_and_adds_one_only_after_the_first_codeunit_when_its_cost_fits() {
        let now = rss(500);
        let pool = Pool::new(Some(budget(10_000 * MB, now.clone(), true)), 500 * MB, 4);
        assert_eq!(pool.live(), 1);
        assert!(
            pool.try_add(0).is_none(),
            "no codeunit ended yet: the cost is not known"
        );
        let first = pool.enter();
        now.store(1500 * MB, Ordering::SeqCst);
        drop(first);
        assert_eq!(pool.cost() >> 20, 1000, "learned from the first codeunit");
        // 1500 MB held, 1000 MB per codeunit: 2500 MB fits in 9000 MB.
        let second = pool.try_add(0).expect("room for a second thread");
        assert_eq!(pool.live(), 2);
        // A third beside it, counted with the one just added: 1500 + 2 x 1000 MB.
        let third = pool.try_add(1).expect("room for a third thread");
        drop((second, third));
        assert_eq!(pool.live(), 1);
        assert_eq!(pool.report_into(MemoryReport::default()).most, 3);
    }

    #[test]
    fn a_thread_is_not_added_when_its_learned_cost_does_not_fit() {
        let now = rss(500);
        let pool = Pool::new(Some(budget(3000 * MB, now.clone(), true)), 500 * MB, 4);
        let first = pool.enter();
        now.store(2000 * MB, Ordering::SeqCst);
        drop(first);
        // 2000 + 1500 MB pass nine tenths of 3000 MB.
        assert!(pool.try_add(0).is_none());
        assert_eq!(pool.live(), 1);
    }

    /// Decision 3: a thread with no room ends instead of waiting; the run's first thread never ends this way.
    #[test]
    fn a_thread_ends_instead_of_waiting_when_there_is_no_room() {
        let now = rss(500);
        let pool = Pool::new(Some(budget(4000 * MB, now.clone(), false)), 500 * MB, 4);
        drop(pool.enter());
        now.store(1000 * MB, Ordering::SeqCst);
        let helper = pool.try_add(0).expect("room");
        assert!(pool.keep(), "room left: the thread goes on");
        // 3560 MB and the cost of one more codeunit (at least 64 MB) pass nine tenths of 4000 MB.
        now.store(3560 * MB, Ordering::SeqCst);
        assert!(!pool.keep(), "no room: the thread ends");
        drop(helper);
        assert_eq!(pool.live(), 1, "one thread always runs");
        assert_eq!(pool.report_into(MemoryReport::default()).ended, 1);
        assert!(!pool.halted(), "a soft limit never stops the run");
    }

    /// Without a budget every thread the work can use starts at once.
    #[test]
    fn without_a_budget_threads_start_without_a_learned_cost() {
        let pool = Pool::new(None, 0, 3);
        let a = pool.try_add(0).expect("a second");
        let b = pool.try_add(1).expect("a third");
        assert!(pool.try_add(2).is_none(), "never more than the threads asked for");
        assert!(pool.keep());
        drop((a, b));
    }

    /// Decision 6: a hard limit stops the run before the resident memory passes it, with the numbers and the reason
    /// the tests not reached get.
    #[test]
    fn a_hard_limit_stops_the_run_before_the_resident_memory_passes_it() {
        let now = rss(500);
        let pool = Pool::new(Some(budget(2048 * MB, now.clone(), true)), 500 * MB, 1);
        let running = pool.enter();
        now.store(1700 * MB, Ordering::SeqCst);
        pool.sample();
        assert!(!pool.halted(), "1700 MB is still under the stop line of 2048 MB");
        now.store(1850 * MB, Ordering::SeqCst);
        pool.sample();
        assert!(pool.halted());
        let halt = pool.halt();
        assert_eq!(halt.reason(), "memory limit (--max-memory 2048 MB)");
        let m = halt.message().expect("a message");
        assert!(m.contains("1850 MB") && m.contains("2048 MB (--max-memory)"), "{m}");
        assert!(halt.flag().load(Ordering::SeqCst), "the running test is stopped");
        drop(running);
    }

    /// With more than one thread the stop comes closer to the limit: the other threads end when there is no room.
    #[test]
    fn with_more_threads_the_run_stops_closer_to_the_limit() {
        let now = rss(500);
        let pool = Pool::new(Some(budget(2048 * MB, now.clone(), true)), 500 * MB, 2);
        drop(pool.enter());
        let helper = pool.try_add(0).expect("room");
        now.store(1950 * MB, Ordering::SeqCst);
        pool.sample();
        assert!(!pool.halted(), "1950 MB with two threads: the helper ends first");
        drop(helper);
        pool.sample();
        assert!(pool.halted(), "1950 MB with one thread: past the stop line");
    }

    #[test]
    fn a_soft_limit_never_stops_the_run() {
        let now = rss(500);
        let pool = Pool::new(Some(budget(1000 * MB, now.clone(), false)), 500 * MB, 1);
        let _running = pool.enter();
        now.store(5000 * MB, Ordering::SeqCst);
        pool.sample();
        assert!(!pool.halted());
    }

    /// Decision 6, after the first codeunit: the cost of one thread beside the fixed part does not fit.
    #[test]
    fn a_hard_limit_stops_the_run_when_one_threads_cost_cannot_fit() {
        let now = rss(500);
        let pool = Pool::new(Some(budget(2048 * MB, now.clone(), true)), 500 * MB, 4);
        let first = pool.enter();
        now.store(2040 * MB, Ordering::SeqCst);
        drop(first);
        assert!(pool.halted(), "500 + 1540 MB do not fit under the stop line");
    }

    #[test]
    fn a_hard_limit_stops_the_run_at_start_when_one_thread_does_not_fit() {
        let opts = crate::runtime::RunOptions {
            memory: Some(budget(100 * MB, rss(90), true)),
            ..Default::default()
        };
        let plan = Plan::after_compile(&opts, 12);
        assert!(plan.pool().halted());
        let m = plan.pool().halt().message().expect("a message");
        assert!(
            m.contains("not enough memory to run the tests") && m.contains("the limit is 100 MB"),
            "{m}"
        );
        let soft = crate::runtime::RunOptions {
            memory: Some(budget(100 * MB, rss(90), false)),
            ..Default::default()
        };
        assert!(
            !Plan::after_compile(&soft, 12).pool().halted(),
            "serve and debug go on with one thread"
        );
    }

    /// A test not reached before the stop keeps the limit as its reason when a later step refuses the whole codeunit
    /// (a chain member after one the stop left not run as a whole).
    #[test]
    fn a_refusal_of_the_whole_codeunit_keeps_the_memory_limit_as_the_reason() {
        use crate::Outcome;
        let mut done = vec![
            ("Ran".to_string(), Outcome::Pass),
            (
                "NotReached".to_string(),
                Outcome::NotRun("memory limit (--max-memory 2048 MB)".into()),
            ),
        ];
        crate::runtime::corpus::refuse_all(&mut done, "an earlier test codeunit was not run");
        assert_eq!(
            done[0].1,
            Outcome::NotRun("an earlier test codeunit was not run".into())
        );
        assert_eq!(done[1].1, Outcome::NotRun("memory limit (--max-memory 2048 MB)".into()));
    }

    #[test]
    fn the_report_line_names_the_most_threads_and_how_many_ended_for_memory() {
        let r = MemoryReport {
            limit: Some((3072 * MB, "--max-memory".into())),
            after_compile: Some(700 * MB),
            after_baseline: Some(710 * MB),
            per_worker: 1600 * MB,
            asked: 12,
            most: 2,
            at_end: 1,
            ended: 1,
            halted: None,
        };
        let line = r.line().expect("a budget");
        assert!(line.contains("test threads at most 2 of 12, 1 at the end"), "{line}");
        assert!(line.contains("1 ended for memory"), "{line}");
    }
}
