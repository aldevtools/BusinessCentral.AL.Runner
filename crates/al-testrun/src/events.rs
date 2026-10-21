//! What a run tells while it runs: one event per test as it finishes, and a flag that stops the run after the current
//! test (`al-runner serve`, ADR-028). Installed per thread by the code that runs a test codeunit, so the signatures of
//! the run functions did not change.

use crate::Outcome;
use std::cell::RefCell;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Where a statement is: the position the capture names a statement by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loc {
    /// The app that holds the file, as the caller numbered the apps of the run.
    pub app: usize,
    /// The path of the file inside its app.
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub end_line: u32,
}

/// What a capture saw (`RunOptions::capture`), with where the statements it names are.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CapturedAt {
    pub captured: al_vm::Captured,
    /// By position id: every statement the capture names.
    pub locations: std::collections::BTreeMap<u32, Loc>,
}

/// A test that finished.
#[derive(Debug, Clone)]
pub struct TestEvent {
    pub codeunit: u32,
    pub codeunit_name: String,
    pub test: String,
    pub outcome: Outcome,
    pub call_stack: String,
    pub time: std::time::Duration,
    /// With `RunOptions::coverage`: the statements the test ran (position id, runs).
    pub coverage: Vec<(u32, u32)>,
    /// With `RunOptions::capture`: what the test's code did, statement by statement.
    pub captured: Option<CapturedAt>,
    /// `--tdd` (ADR-051 decision 5): the stubs that ran while the test ran, sorted (`TestResult::stubs`).
    pub stubs: Vec<String>,
}

type OnTest = Arc<dyn Fn(TestEvent) + Send + Sync>;
/// What is told when a test is about to run: codeunit id and test name.
pub type OnStart = Arc<dyn Fn(u32, &str) + Send + Sync>;

/// The callbacks of a run.
#[derive(Clone, Default)]
pub struct Hooks {
    /// Called on the thread that ran the test, as soon as its outcome is known.
    pub on_test: Option<OnTest>,
    /// Called on the worker thread just before a test starts to run (after the cancel check): codeunit id and test name.
    pub on_start: Option<OnStart>,
    /// Set from outside: no test starts after it is set; the running one finishes.
    pub cancel: Option<Arc<AtomicBool>>,
}

impl std::fmt::Debug for Hooks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Hooks")
            .field("on_test", &self.on_test.is_some())
            .field("on_start", &self.on_start.is_some())
            .field("cancel", &self.cancel.is_some())
            .finish()
    }
}

thread_local! {
    static CURRENT: RefCell<(Hooks, Vec<String>)> = RefCell::new((Hooks::default(), Vec::new()));
}

pub(crate) fn install(h: Hooks) {
    CURRENT.with(|c| *c.borrow_mut() = (h, Vec::new()));
}

/// Removes the hooks of this thread and returns the names of the tests already reported.
pub(crate) fn uninstall() -> Vec<String> {
    CURRENT.with(|c| std::mem::take(&mut *c.borrow_mut()).1)
}

pub(crate) fn reported() -> Vec<String> {
    CURRENT.with(|c| c.borrow().1.clone())
}

pub(crate) fn cancelled() -> bool {
    CURRENT.with(|c| c.borrow().0.cancel.as_ref().is_some_and(|f| f.load(Ordering::Relaxed)))
}

/// The flag that stops the run (for a scratch run, whose one routine is stopped while it runs).
pub(crate) fn cancel_flag() -> Option<Arc<AtomicBool>> {
    CURRENT.with(|c| c.borrow().0.cancel.clone())
}

/// Tells the run's hook that a test is about to run.
pub(crate) fn starting(codeunit: u32, test: &str) {
    crate::crash::running(codeunit, test);
    let hook = CURRENT.with(|c| c.borrow().0.on_start.clone());
    if let Some(h) = hook {
        h(codeunit, test);
    }
}

pub(crate) fn wants_events() -> bool {
    CURRENT.with(|c| c.borrow().0.on_test.is_some())
}

pub(crate) fn report(e: TestEvent) {
    let hook = CURRENT.with(|c| {
        let mut c = c.borrow_mut();
        c.1.push(e.test.clone());
        c.0.on_test.clone()
    });
    if let Some(h) = hook {
        h(e);
    }
}
