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
//!    is learned in the run: the ninetieth percentile of what each codeunit grew the process by over its own run, per live test thread, shared parts left out, seen from the first codeunit
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

/// How many codeunit costs the learning keeps: the newest ones.
const KEEP: usize = 512;

/// The cost of one codeunit from its window: the resident memory grew from `rss_in` to `now` while it ran. What the
/// shared parts grew in that time (`fixed_in` to `fixed_now`: shared rows, the frozen baseline, state held once) is not
/// a thread's cost. The other `threads` grew in the same window, so each counts an equal part. At least [`WORKER_FLOOR`].
fn window_cost(rss_in: u64, fixed_in: u64, now: u64, fixed_now: u64, threads: usize) -> u64 {
    let grew = now
        .saturating_sub(rss_in)
        .saturating_sub(fixed_now.saturating_sub(fixed_in));
    let threads = threads.max(1) as u64;
    // A window sees only growth: memory the allocator reuses never shows in it. With several threads alive, what the
    // process holds above the fixed part, shared by them, is what a thread really costs (never less than the window).
    let held = if threads > 1 {
        now.saturating_sub(fixed_now) / threads
    } else {
        0
    };
    (grew / threads).max(held).max(WORKER_FLOOR)
}

/// The statistic of the learned cost: the ninetieth percentile of the codeunits seen (`sorted` ascending, not empty
/// unless nothing ended). One large codeunit or a codeunit that ran beside a growing neighbour does not set the cost
/// of every thread; the hard stop on the resident memory still catches a codeunit that outgrows it.
fn percentile(sorted: &[u64]) -> u64 {
    match sorted.len() {
        0 => 0,
        n => sorted[(n * 9).div_ceil(10) - 1],
    }
}

/// Reads the process's resident memory in bytes.
pub type ReadResident = Arc<dyn Fn() -> Option<u64> + Send + Sync>;

/// Reads how many bytes the run's shared rows hold now ([`Pool::share`]).
pub type SharedBytes = Arc<dyn Fn() -> u64 + Send + Sync>;

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
    /// About how many bytes the threads share and the fixed part counts once: the frozen install baseline and the test
    /// data rows built once (ADR-069 decision 10).
    pub shared: u64,
}

impl MemoryReport {
    /// The `--timings` line; `None` without a budget.
    pub fn line(&self) -> Option<String> {
        let (limit, source) = self.limit.as_ref()?;
        let mb = |b: u64| b >> 20;
        let at = |b: Option<u64>| b.map_or("?".to_string(), |b| format!("{} MB", mb(b)));
        Some(format!(
            "memory: budget {} MB ({source}); resident after compile {}, after install baseline {}; shared by the \
             threads once {} MB; about {} MB per test thread; test threads at most {} of {}, {} at the end, {} ended \
             for memory{}",
            mb(*limit),
            at(self.after_compile),
            at(self.after_baseline),
            mb(self.shared),
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

/// The watcher of the compile (decision 6 for the whole run; owner, 2026-11-09): under a hard limit it reads the
/// resident memory from before the apps are loaded until they are compiled, and sets its flag before the resident
/// memory passes the stop line. The compile's threads are armed with the flag (`al_sema::halt`) and stop at their next
/// object or file; the caller reports every test as not run.
pub(crate) struct CompileWatch {
    flag: Arc<AtomicBool>,
    message: Arc<Mutex<Option<String>>>,
    reason: String,
    watcher: Option<(std::sync::mpsc::Sender<()>, std::thread::JoinHandle<()>)>,
}

impl CompileWatch {
    /// Starts the watcher under a hard limit whose resident memory can be read; `None` otherwise. A process that holds
    /// more than the stop line already has its flag set at once.
    pub(crate) fn start(budget: Option<&MemoryBudget>) -> Option<CompileWatch> {
        let b = budget.filter(|b| b.hard)?.clone();
        (b.rss)()?;
        let flag: Arc<AtomicBool> = Arc::default();
        let message: Arc<Mutex<Option<String>>> = Arc::default();
        let sample = {
            let (flag, message, b) = (flag.clone(), message.clone(), b.clone());
            move || {
                if let Some(m) = (b.rss)().and_then(|now| compile_stop(&b, now)) {
                    message.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert(m);
                    flag.store(true, Ordering::SeqCst);
                }
            }
        };
        sample();
        let (done, rx) = std::sync::mpsc::channel::<()>();
        let thread = std::thread::spawn(move || {
            while let Err(std::sync::mpsc::RecvTimeoutError::Timeout) = rx.recv_timeout(SAMPLE) {
                sample();
            }
        });
        Some(CompileWatch {
            flag,
            message,
            reason: format!("{REASON}{} {} MB)", b.source, b.limit >> 20),
            watcher: Some((done, thread)),
        })
    }

    /// The flag the compile's threads are armed with.
    pub(crate) fn flag(&self) -> Arc<AtomicBool> {
        self.flag.clone()
    }

    /// The reason of the tests not run once the compile stopped: "memory limit (--max-memory 2048 MB)".
    pub(crate) fn reason(&self) -> &str {
        &self.reason
    }

    /// Stops the watcher: the message when the compile was stopped.
    pub(crate) fn stop(&mut self) -> Option<String> {
        if let Some((done, thread)) = self.watcher.take() {
            let _ = done.send(());
            let _ = thread.join();
        }
        self.message.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

impl Drop for CompileWatch {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Sets a session's stop flag once the run stops at its hard limit (`halt`), while it lives: the install baseline's
/// code then stops at its next loop iteration, as a running test does through its watchdog.
pub(crate) struct StopBridge {
    done: Option<(std::sync::mpsc::Sender<()>, std::thread::JoinHandle<()>)>,
}

impl StopBridge {
    /// `None` without a `halt` flag (no hard limit).
    pub(crate) fn start(halt: Option<Arc<AtomicBool>>, stop: Arc<AtomicBool>) -> Option<StopBridge> {
        let halt = halt?;
        let (done, rx) = std::sync::mpsc::channel::<()>();
        let thread = std::thread::spawn(move || {
            while let Err(std::sync::mpsc::RecvTimeoutError::Timeout) = rx.recv_timeout(SAMPLE) {
                if halt.load(Ordering::SeqCst) {
                    stop.store(true, Ordering::SeqCst);
                    return;
                }
            }
        });
        Some(StopBridge {
            done: Some((done, thread)),
        })
    }
}

impl Drop for StopBridge {
    fn drop(&mut self) {
        if let Some((done, thread)) = self.done.take() {
            let _ = done.send(());
            let _ = thread.join();
        }
    }
}

/// The message when the compile must stop: the resident memory `now` passed the stop line of a hard limit. The
/// compile's threads stop at their next file or object, but the process still grows a little first: on Tests-SCM a
/// cold compile cache passed a line a thirty-second under the limit by 92 MB (3068 MB of 3072), so the compile keeps
/// the one-thread line ([`stop_line`]), an eighth under it.
fn compile_stop(b: &MemoryBudget, now: u64) -> Option<String> {
    (now > stop_line(b.limit)).then(|| {
        format!(
            "not enough memory to run the tests: the process held {} MB while the apps were compiled and was still \
             growing, but the limit is {} MB ({}). The compile was stopped and the tests are reported as not run. \
             Free memory, or raise the limit with --max-memory.",
            now >> 20,
            b.limit >> 20,
            b.source
        )
    })
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
        pool.watch(); // A hard limit stops the run before the resident memory passes it, baseline included.
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

    /// The flag of the run's stop under a hard limit; `None` without one.
    pub(crate) fn halt_flag(&self) -> Option<Arc<AtomicBool>> {
        self.budget.as_ref().filter(|b| b.hard).map(|_| self.pool.halt().flag())
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
    /// Resident bytes with no codeunit running (the fixed part, without what the threads share: [`Pool::fixed`]).
    fixed: AtomicU64,
    /// About how many bytes the test data rows that every thread shares hold now ([`Pool::share`]); they grow as the
    /// threads first touch tables, and count once, in the fixed part, never in the cost of a thread.
    shared: std::sync::OnceLock<(SharedBytes, u64)>,
    /// [`Pool::add_fixed`]: what the run made after the fixed part was read, shared by every thread.
    added: AtomicU64,
    /// The most threads the run may use.
    max: usize,
    /// The learned cost of one running codeunit; `learned` once a codeunit ended.
    cost: AtomicU64,
    learned: AtomicBool,
    /// The cost each ended codeunit showed over its own window ([`window_cost`]), the newest [`KEEP`] of them.
    samples: Mutex<std::collections::VecDeque<u64>>,
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
                shared: Default::default(),
                added: AtomicU64::new(0),
                max: max.max(1),
                cost: AtomicU64::new(0),
                learned: AtomicBool::new(false),
                samples: Mutex::default(),
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

    /// Counts `bytes` the run made after the fixed part was read, shared by every thread, in the fixed part (ADR-069
    /// decision 10).
    pub fn add_fixed(&self, bytes: u64) {
        self.s.added.fetch_add(bytes, Ordering::Relaxed);
    }

    /// [`Pool::add_fixed`] as a function that holds the pool's shared part, not the pool.
    pub fn fixed_adder(&self) -> impl Fn(u64) + 'static {
        let s = self.s.clone();
        move |bytes| {
            s.added.fetch_add(bytes, Ordering::Relaxed);
        }
    }

    /// Counts what `bytes` reads, the rows every thread shares, in the fixed part as they grow (decision 10): what it
    /// reads now is already in the fixed part. Only the first call counts.
    pub fn share(&self, bytes: SharedBytes) {
        let at = bytes();
        let _ = self.s.shared.set((bytes, at));
    }

    /// What the threads share and the fixed part counts once beyond what the process held when it was read.
    fn shared(&self) -> u64 {
        self.s.added.load(Ordering::Relaxed) + self.s.shared.get().map_or(0, |(b, at)| b().saturating_sub(*at))
    }

    /// The fixed part now: what the process held with no codeunit running, with the shared rows built since.
    fn fixed(&self) -> u64 {
        self.s.fixed.load(Ordering::Relaxed) + self.shared()
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
        let (fixed, cost) = (self.fixed(), self.cost());
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
        Running {
            pool: self,
            window: self.resident().map(|(_, now)| (now, self.fixed(), self.live())),
        }
    }

    /// Reads the resident memory: learns the cost of a running codeunit, and under a hard limit stops the run before
    /// the resident memory passes the limit (the watcher calls it while the tests run).
    pub fn sample(&self) {
        if let Some((b, now)) = self.resident() {
            self.sample_at(b, now);
        }
    }

    fn sample_at(&self, b: &MemoryBudget, now: u64) {
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
    fn ended(&self, window: Option<(u64, u64, usize)>) {
        let read = self.resident();
        if let Some((b, now)) = read {
            self.sample_at(b, now);
        }
        if let (Some((rss_in, fixed_in, live_in)), Some((_, now))) = (window, read) {
            let threads = self.live().max(live_in).max(self.s.running.load(Ordering::SeqCst));
            let per = window_cost(rss_in, fixed_in, now, self.fixed(), threads);
            let mut v = self.s.samples.lock().unwrap_or_else(|e| e.into_inner());
            if v.len() == KEEP {
                v.pop_front();
            }
            v.push_back(per);
            let mut sorted: Vec<u64> = v.iter().copied().collect();
            sorted.sort_unstable();
            self.s.cost.store(percentile(&sorted), Ordering::Relaxed);
        }
        self.s.learned.store(true, Ordering::SeqCst);
        let Some(b) = self.s.budget.as_ref().filter(|b| b.hard && self.live() == 1) else {
            return;
        };
        let need = self.fixed() + self.cost();
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
            shared: self.shared(),
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
    /// The resident memory, the fixed part and the live threads when the codeunit started.
    window: Option<(u64, u64, usize)>,
}

impl Drop for Running<'_> {
    fn drop(&mut self) {
        self.pool.ended(self.window);
        self.pool.s.running.fetch_sub(1, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {

    /// Decision 6 during the install baseline: once the run stops at its hard limit, the install code stops too (the
    /// bridge sets its session's stop flag), so the baseline does not grow past the limit.
    #[test]
    fn the_install_baseline_stops_when_the_run_stops() {
        let halt = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let bridge = StopBridge::start(Some(halt.clone()), stop.clone());
        std::thread::sleep(SAMPLE * 3);
        assert!(!stop.load(Ordering::SeqCst));
        halt.store(true, Ordering::SeqCst);
        let waited = std::time::Instant::now();
        while !stop.load(Ordering::SeqCst) && waited.elapsed() < Duration::from_secs(5) {
            std::thread::sleep(SAMPLE);
        }
        assert!(stop.load(Ordering::SeqCst), "the session's stop flag is set");
        drop(bridge);
        assert!(StopBridge::start(None, stop).is_none());
    }

    /// Decision 6 for the compile: the watcher sets its flag once the resident memory passes the stop line of a hard
    /// limit, at once when the process already holds more; a soft limit has no watcher.
    #[test]
    fn the_compile_watch_stops_past_the_stop_line_of_a_hard_limit_only() {
        let at = |mb: u64, hard: bool| MemoryBudget {
            limit: 1000 * MB,
            source: "--max-memory".into(),
            rss: Arc::new(move || Some(mb * MB)),
            hard,
        };
        // The compile's line is the one-thread line (`stop_line`): 875 MB of 1000.
        assert!(compile_stop(&at(870, true), 870 * MB).is_none());
        let m = compile_stop(&at(880, true), 880 * MB).expect("past the line");
        assert!(
            m.contains("880 MB while the apps were compiled") && m.contains("1000 MB (--max-memory)"),
            "{m}"
        );
        assert!(CompileWatch::start(Some(&at(990, false))).is_none());
        let mut w = CompileWatch::start(Some(&at(990, true))).expect("hard");
        assert!(w.flag().load(Ordering::SeqCst), "the first read already stops it");
        assert_eq!(w.reason(), "memory limit (--max-memory 1000 MB)");
        assert!(w.stop().is_some_and(|m| m.contains("990 MB")));
        let mut fits = CompileWatch::start(Some(&at(500, true))).expect("hard");
        assert!(!fits.flag().load(Ordering::SeqCst));
        assert_eq!(fits.stop(), None);
    }
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

    /// A thread that is alive but not inside a codeunit (it is building its own baseline copy and session, `own()` in
    /// `threads.rs`) already holds memory: it is part of the cost of its thread, never of the one running codeunit
    /// (issue 6924: 6328 MB learned per thread where the whole process peaked at 8.9 GB with 8 threads).
    #[test]
    fn memory_of_threads_not_yet_in_a_codeunit_is_not_charged_to_the_running_one() {
        let now = rss(500);
        let pool = Pool::new(Some(budget(100_000 * MB, now.clone(), true)), 500 * MB, 8);
        let first = pool.enter();
        now.store(700 * MB, Ordering::SeqCst);
        drop(first);
        let members: Vec<_> = (0..7).map(|k| pool.try_add(k).expect("room")).collect();
        assert_eq!(pool.live(), 8);
        // One codeunit runs; the other seven threads are setting up. The process grows by 2200 MB in its window.
        let running = pool.enter();
        now.store((500 + 8 * 300) * MB, Ordering::SeqCst);
        drop(running);
        assert_eq!(
            pool.cost() >> 20,
            300,
            "2400 MB held by 8 threads, not all of it for one codeunit"
        );
        let running = pool.enter();
        drop(running);
        drop(members);
    }

    /// Issue 6924, whole area: the first codeunit runs alone and the state a chain carries (held once, counted with the
    /// fixed part through `add_fixed`) is not the cost of a thread.
    #[test]
    fn state_a_chain_holds_once_is_not_the_cost_of_a_thread() {
        let now = rss(500);
        let pool = Pool::new(Some(budget(100_000 * MB, now.clone(), true)), 500 * MB, 8);
        let first = pool.enter();
        pool.add_fixed(1000 * MB);
        now.store(1900 * MB, Ordering::SeqCst); // 1000 MB carried state and 400 MB its own
        drop(first);
        assert_eq!(pool.cost() >> 20, 400);
    }

    /// One large codeunit among many small ones does not set the cost of every thread (ninetieth percentile).
    #[test]
    fn one_large_codeunit_does_not_set_the_cost_of_every_thread() {
        let now = rss(500);
        let pool = Pool::new(Some(budget(100_000 * MB, now.clone(), false)), 500 * MB, 8);
        let mut at = 500;
        for grew in std::iter::repeat_n(100, 10).chain([1000]) {
            let r = pool.enter();
            at += grew;
            now.store(at * MB, Ordering::SeqCst);
            drop(r);
        }
        assert_eq!(pool.cost() >> 20, 100);
    }

    /// The safety stays when the estimate is low: the cost is only used to admit threads; a run whose resident memory
    /// climbs past the stop line is stopped (exit code 2) whatever was learned.
    #[test]
    fn a_low_estimate_does_not_let_the_run_pass_a_hard_limit() {
        let now = rss(500);
        let pool = Pool::new(Some(budget(2048 * MB, now.clone(), true)), 500 * MB, 4);
        let first = pool.enter();
        now.store(564 * MB, Ordering::SeqCst);
        drop(first);
        assert_eq!(pool.cost() >> 20, 64, "a low estimate");
        let helper = pool.try_add(0).expect("room at the low estimate");
        let running = pool.enter();
        now.store(2040 * MB, Ordering::SeqCst); // the codeunits grow far past it
        pool.sample();
        assert!(pool.halted(), "stopped before the limit");
        assert!(!pool.keep(), "and no room to go on");
        drop((running, helper));
    }

    /// A window of a codeunit that reuses freed memory shows no growth; with several threads alive the memory the
    /// process holds above the fixed part, divided by them, is the cost (issue 6924: about 360 MB real, not the floor).
    #[test]
    fn a_flat_window_beside_other_threads_still_learns_what_a_thread_holds() {
        let now = rss(500);
        let pool = Pool::new(Some(budget(100_000 * MB, now.clone(), true)), 500 * MB, 8);
        drop(pool.enter());
        let members: Vec<_> = (0..7).map(|k| pool.try_add(k).expect("room")).collect();
        now.store((500 + 8 * 360) * MB, Ordering::SeqCst);
        let r = pool.enter(); // the process already holds it: no growth in the window
        drop(r);
        assert_eq!(pool.cost() >> 20, 360);
        drop(members);
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

    /// ADR-069 decision 10: what every thread shares (the frozen install baseline, the test data rows built once) is part
    /// of the fixed part, counted once, so the cost a thread learns holds only its own sessions.
    #[test]
    fn the_shared_rows_count_once_in_the_fixed_part_not_in_the_cost_of_a_thread() {
        let now = rss(1000);
        let shared = Arc::new(AtomicU64::new(0));
        let pool = Pool::new(Some(budget(4000 * MB, now.clone(), false)), 1000 * MB, 8);
        let reader = shared.clone();
        pool.share(Arc::new(move || reader.load(Ordering::SeqCst)));
        pool.add_fixed(100 * MB); // the frozen install baseline, made after the fixed part was read
        now.store(1100 * MB, Ordering::SeqCst); // the frozen baseline is in the resident memory by now
        let first = pool.enter();
        // The first codeunit built 1500 MB of shared test data rows and holds 200 MB of its own.
        shared.store(1500 * MB, Ordering::SeqCst);
        now.store(2800 * MB, Ordering::SeqCst);
        drop(first);
        assert_eq!(pool.cost(), 200 * MB, "the shared rows are not a thread's cost");
        // 2800 + 2 x 200 MB fit in nine tenths of 4000 MB; with the rows in the cost (1700 MB) not even one would.
        let a = pool.try_add(0).expect("a second thread");
        let b = pool.try_add(1).expect("a third thread");
        drop((a, b));
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
            shared: 150 * MB,
        };
        let line = r.line().expect("a budget");
        assert!(line.contains("shared by the threads once 150 MB"), "{line}");
        assert!(line.contains("test threads at most 2 of 12, 1 at the end"), "{line}");
        assert!(line.contains("1 ended for memory"), "{line}");
    }
}
