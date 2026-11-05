//! The platform's run of a test codeunit for a test runner codeunit (ADR-066 decision 1).
//!
//! On BC, `Codeunit.Run` of a `Subtype = Test` codeunit from the code of a `Subtype = TestRunner` codeunit is the
//! platform's test codeunit run: the test codeunit's OnRun and each `[Test]` method, with the runner's
//! `OnBeforeTestRun` and `OnAfterTestRun` triggers around each (Microsoft's Test Runner app,
//! `TestRunnerMgt.Codeunit.al:25-58`, `RunTests` calls `CODEUNIT.Run(TestMethodLine."Test Codeunit")`). That run
//! belongs to the test runner of `al-testrun`, which owns the refusals, the isolation, the handlers and the results.
//! The VM only hands the call over: [`Session::with_test_codeunit_host`] installs the host for the length of one call
//! of the runner's code, and `Codeunit.Run` of a test codeunit under a running runner calls
//! [`TestCodeunitHost::run_test_codeunit`].

use std::cell::RefCell;
use std::rc::Rc;

use al_builtins::value::{Instance, Value};

use crate::{Session, Signal};

/// The test codeunit run of the test runner (al-testrun). ADR-066 decision 1.
pub trait TestCodeunitHost {
    /// Runs test codeunit `object` for the runner `runner` (its object and the instance its OnRun runs on, which
    /// also receives the runner's triggers) on `session`. An error or refusal is the `Codeunit.Run`'s own.
    fn run_test_codeunit(
        &mut self,
        session: &mut Session,
        object: usize,
        runner: (usize, Rc<RefCell<Instance>>),
    ) -> Result<(), Signal>;
}

/// The host installed by [`Session::with_test_codeunit_host`] and the runner codeunits running under it.
#[derive(Default)]
pub(crate) struct RunnerHost {
    /// The installed host. The lifetime is erased; [`Session::with_test_codeunit_host`] clears it before the borrow
    /// it was made from ends, a panic included.
    host: Option<*mut (dyn TestCodeunitHost + 'static)>,
    /// The `Subtype = TestRunner` codeunits whose OnRun is running under the host, innermost last: object and the
    /// instance the run made.
    runners: Vec<(usize, Rc<RefCell<Instance>>)>,
}

impl Session {
    /// Runs `f` with `host` installed: while `f` runs, `Codeunit.Run` of a test codeunit from a test runner
    /// codeunit's OnRun goes to `host` (ADR-066 decision 1).
    pub fn with_test_codeunit_host<R>(
        &mut self,
        host: &mut dyn TestCodeunitHost,
        f: impl FnOnce(&mut Session) -> R,
    ) -> R {
        let ptr: *mut (dyn TestCodeunitHost + '_) = host;
        // SAFETY: only the lifetime is changed. The pointer is used only while `f` runs (`run_for_runner`), during
        // which `host` stays mutably borrowed by this call, and it is removed below before this function returns or
        // unwinds.
        let ptr: *mut (dyn TestCodeunitHost + 'static) = unsafe { std::mem::transmute(ptr) };
        let before = self.runner_host.host.replace(ptr);
        let runners = std::mem::take(&mut self.runner_host.runners);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(self)));
        self.runner_host.host = before;
        self.runner_host.runners = runners;
        match result {
            Ok(r) => r,
            Err(p) => std::panic::resume_unwind(p),
        }
    }

    /// Whether a host is installed: a `Subtype = TestRunner` codeunit's OnRun is then noted
    /// ([`Session::enter_test_runner`]).
    pub(crate) fn has_test_codeunit_host(&self) -> bool {
        self.runner_host.host.is_some()
    }

    pub(crate) fn enter_test_runner(&mut self, object: usize, inst: Rc<RefCell<Instance>>) {
        self.runner_host.runners.push((object, inst));
    }

    pub(crate) fn leave_test_runner(&mut self) {
        self.runner_host.runners.pop();
    }

    /// `Codeunit.Run` of test codeunit `object`: `None` when no runner's OnRun is running under a host, else the
    /// host's run. The host is taken out while it runs: a second test codeunit run from inside it raises the
    /// nesting error before it gets here (`transactions.rs`).
    pub(crate) fn run_for_runner(&mut self, object: usize, guarded: bool) -> Option<Result<Value, Signal>> {
        let runner = self.runner_host.runners.last().cloned()?;
        let ptr = self.runner_host.host.take()?;
        let result = if guarded {
            // What the guarded form returns when a test fails is not probed (ADR-066 probe P5); 130454 uses the
            // statement form (`TestRunnerMgt.Codeunit.al:48`).
            Err(Signal::Unsupported(
                "Codeunit.Run of a test codeunit from a test runner, with its result used: what it returns when a test \
                 fails is not probed (ADR-066 P5)"
                    .into(),
            ))
        } else {
            // SAFETY: see `with_test_codeunit_host`; the pointer is live while the host is installed.
            unsafe { (*ptr).run_test_codeunit(self, object, runner) }.map(|()| Value::Null)
        };
        self.runner_host.host = Some(ptr);
        Some(result)
    }
}
