//! The writes of workspace layer entries that run off the critical path (ADR-022 amendment 2026-11-15, decision 6):
//! a compile defers its write ([`defer_write`]); `al-runner test` runs the writes after the results are printed
//! ([`wait_for_writes`]), serve starts them on a thread of their own after the response of a request
//! ([`start_writes`]). A write that does not finish leaves no file (temporary file, then rename).

/// A write of an entry that waits for the end of the run (decision 6).
type Job = Box<dyn FnOnce() + Send>;

/// The writes this process deferred, in the order the compiles made them.
static DEFERRED: std::sync::Mutex<Vec<Job>> = std::sync::Mutex::new(Vec::new());
/// The writer threads [`start_writes`] started and nothing joined yet.
static WRITERS: std::sync::Mutex<Vec<std::thread::JoinHandle<()>>> = std::sync::Mutex::new(Vec::new());

/// A test of serve's shutdown (issue 9232) sets this to a number of milliseconds: each batch of writes waits that long
/// before it starts, so a write in flight can be seen without an app large enough to make the encode slow.
const TEST_DELAY: &str = "ALRUNNER_TEST_SLOW_CACHE_WRITE_MS";

/// Runs `jobs` in order, after the delay a test asked for ([`TEST_DELAY`]).
fn run(jobs: Vec<Job>) {
    if jobs.is_empty() {
        return;
    }
    if let Some(ms) = std::env::var(TEST_DELAY).ok().and_then(|v| v.parse::<u64>().ok()) {
        std::thread::sleep(std::time::Duration::from_millis(ms));
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
    let (done, running): (Vec<_>, Vec<_>) = std::mem::take(&mut *writers).into_iter().partition(|h| h.is_finished());
    for h in done {
        let _ = h.join();
    }
    *writers = running;
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

/// Runs the deferred writes and waits for every write of this process (`al-runner test` before it exits, after the
/// results are printed; serve when it shuts down). A write that did not finish leaves no file (temporary file, rename).
pub fn wait_for_writes() {
    let jobs: Vec<Job> = std::mem::take(&mut *DEFERRED.lock().unwrap_or_else(|e| e.into_inner()));
    run(jobs);
    let writers: Vec<_> = std::mem::take(&mut *WRITERS.lock().unwrap_or_else(|e| e.into_inner()));
    for h in writers {
        let _ = h.join();
    }
}
