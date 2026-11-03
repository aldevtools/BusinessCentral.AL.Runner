//! The shadow run of ADR-062 stage 4: the checks are evaluated under the test runner's permission sets, every access is
//! granted anyway, and what would have been denied is noted here. A diagnostic only; nothing in a run reads it back.
//!
//! A session notes a line with [`record`] on its thread; the runner moves the lines of a finished test to the shared list with
//! [`drain`], labelled with the test; the command line writes the list ([`take_all`]).

use std::cell::RefCell;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

static ENABLED: AtomicBool = AtomicBool::new(false);
static ALL: Mutex<Vec<String>> = Mutex::new(Vec::new());

thread_local! {
    static LINES: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

/// Turns the shadow run on for this process (`al-test-cli --permission-shadow FILE`).
pub fn enable() {
    ENABLED.store(true, Ordering::Relaxed);
}

pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// Notes one "would deny" (or "unknown") line of the test that runs on this thread.
pub fn record(line: String) {
    LINES.with(|l| l.borrow_mut().push(line));
}

/// Moves this thread's lines to the shared list, each labelled `label` and a tab.
pub fn drain(label: &str) {
    let lines = LINES.with(|l| std::mem::take(&mut *l.borrow_mut()));
    if lines.is_empty() {
        return;
    }
    let mut all = ALL.lock().unwrap_or_else(|e| e.into_inner());
    all.extend(lines.into_iter().map(|l| format!("{label}\t{l}")));
}

/// Every line drained so far.
pub fn take_all() -> Vec<String> {
    std::mem::take(&mut *ALL.lock().unwrap_or_else(|e| e.into_inner()))
}
