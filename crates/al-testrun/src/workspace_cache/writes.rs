//! The writes of workspace layer entries that run off the critical path (ADR-022 amendment 2026-11-15, decision 6):
//! a compile defers its write ([`defer_write`]); `al-runner test` runs the writes after the results are printed
//! ([`wait_for_writes`]), serve starts them on a thread of their own after the response of a request
//! ([`start_writes`]). A write that does not finish leaves no file (temporary file, then rename).

use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// A write of an entry that waits for the end of the run (decision 6).
type Job = Box<dyn FnOnce() + Send>;

/// The writes this process deferred, in the order the compiles made them.
static DEFERRED: std::sync::Mutex<Vec<Job>> = std::sync::Mutex::new(Vec::new());
/// The writer threads [`start_writes`] started and nothing joined yet.
static WRITERS: std::sync::Mutex<Vec<JoinHandle<()>>> = std::sync::Mutex::new(Vec::new());

/// A test of serve's shutdown (issue 9232) sets this to a number of milliseconds: each batch of writes waits that long
/// before it starts, so a write in flight can be seen without an app large enough to make the encode slow.
const TEST_DELAY: &str = "ALRUNNER_TEST_SLOW_CACHE_WRITE_MS";

/// Runs `jobs` in order, after the delay a test asked for ([`TEST_DELAY`]).
fn run(jobs: Vec<Job>) {
    if jobs.is_empty() {
        return;
    }
    if let Some(ms) = std::env::var(TEST_DELAY).ok().and_then(|v| v.parse::<u64>().ok()) {
        std::thread::sleep(Duration::from_millis(ms));
    }
    jobs.into_iter().for_each(|job| job());
}

/// Defers `write` to the end of the run (decision 6): the encode and the write wait until the results are printed
/// (`al-runner test`, [`wait_for_writes`]) or the response is sent (serve, [`start_writes`]), so they do not share the
/// cores with the tests.
pub(crate) fn defer_write(write: impl FnOnce() + Send + 'static) {
    DEFERRED.lock().unwrap_or_else(|e| e.into_inner()).push(Box::new(write));
}

/// How many writes wait for the end of the run ([`defer_write`]).
pub fn pending_writes() -> usize {
    DEFERRED.lock().unwrap_or_else(|e| e.into_inner()).len()
}

/// Starts the deferred writes on a thread of their own, in order (serve, after the response of a request is sent), and
/// joins the writer threads that finished (review LOW 6: the list does not grow with the requests).
pub fn start_writes() {
    let jobs: Vec<Job> = std::mem::take(&mut *DEFERRED.lock().unwrap_or_else(|e| e.into_inner()));
    let mut writers = WRITERS.lock().unwrap_or_else(|e| e.into_inner());
    join_finished(&mut writers);
    if jobs.is_empty() {
        return;
    }
    let handle = std::thread::Builder::new()
        .name("workspace-cache-write".into())
        .spawn(move || run(jobs));
    if let Ok(h) = handle {
        writers.push(h);
    }
}

/// Joins the writer threads of `writers` that finished and keeps the others.
fn join_finished(writers: &mut Vec<JoinHandle<()>>) {
    let (done, running): (Vec<_>, Vec<_>) = std::mem::take(writers).into_iter().partition(|h| h.is_finished());
    for h in done {
        let _ = h.join();
    }
    *writers = running;
}

/// How often a wait for the writes looks whether they finished.
const POLL: Duration = Duration::from_millis(10);

/// Starts the deferred writes and waits until every write of this process finished, or until `deadline`: true when
/// they finished. A writer thread stays listed until it finished, so a second wait (a stop signal while serve's
/// `shutdown` waits, issue 9232) sees it too.
fn wait_until(deadline: Option<Instant>) -> bool {
    start_writes();
    loop {
        let mut writers = WRITERS.lock().unwrap_or_else(|e| e.into_inner());
        join_finished(&mut writers);
        if writers.is_empty() {
            return true;
        }
        drop(writers);
        let left = deadline.map_or(POLL, |d| d.saturating_duration_since(Instant::now()));
        if left.is_zero() {
            return false;
        }
        std::thread::sleep(left.min(POLL));
    }
}

/// Runs the deferred writes and waits for every write of this process (`al-runner test` before it exits, after the
/// results are printed; serve when it ends). A write that did not finish leaves no file (temporary file, rename).
///
/// The deferred writes run here, on the caller: a one-shot run starts no thread, waits for nothing, and a write that
/// panics panics the caller. Only serve has writer threads ([`start_writes`]); they are waited for as
/// [`wait_for_writes_within`] does, so that they stay listed and a stop signal during serve's end-of-input wait still
/// waits for them (issue 9232).
pub fn wait_for_writes() {
    let jobs: Vec<Job> = std::mem::take(&mut *DEFERRED.lock().unwrap_or_else(|e| e.into_inner()));
    run(jobs);
    wait_until(None);
}

/// [`wait_for_writes`] for at most `bound` (serve's `shutdown` reply and a stop signal, issue 9232): true when every
/// write finished, false when the bound passed first. The writes go on; one the process does not live to finish
/// leaves no file.
pub fn wait_for_writes_within(bound: Duration) -> bool {
    wait_until(Some(Instant::now() + bound))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The unbounded wait runs the deferred writes on the caller, as before issue 9232: no thread, so no poll and no
    /// spawn that can fail, and a write that panics panics the caller. One test, as the list of writes is the
    /// process's.
    #[test]
    fn the_unbounded_wait_runs_the_deferred_writes_on_the_caller() {
        let ran_on = std::sync::Arc::new(std::sync::Mutex::new(None));
        let seen = ran_on.clone();
        defer_write(move || *seen.lock().unwrap() = Some(std::thread::current().id()));
        wait_for_writes();
        assert_eq!(*ran_on.lock().unwrap(), Some(std::thread::current().id()));
        defer_write(|| panic!("a write that fails"));
        assert!(
            std::panic::catch_unwind(wait_for_writes).is_err(),
            "the panic of a write reaches the caller"
        );
        assert_eq!(pending_writes(), 0);
    }
}
