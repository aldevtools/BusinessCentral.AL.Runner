//! The test runner codeunit's own AL (ADR-066 decision 1).
//!
//! A unit (one test codeunit) can be run through a `Subtype = TestRunner` codeunit: the runner's OnRun runs on the
//! unit's session, and its `Codeunit.Run` of the test codeunit comes back here through the VM's
//! [`al_vm::TestCodeunitHost`] hook (`drive`). The test codeunit run itself stays the one of `run.rs`; around the OnRun
//! trigger and each test method it calls the runner's `OnBeforeTestRun` and `OnAfterTestRun` triggers on the
//! runner's instance ([`Steps::Al`]) in place of the four steps `runner_reset.rs` replays by hand ([`Steps::Builtin`]).
//!
//! What the platform does with the triggers is not probed yet (ADR-066 probes P1-P5); it runs under the assumptions
//! `test-runner-trigger-calls` and `test-runner-trigger-transactions` (hard rule 3 exception: Microsoft's CI runs
//! every Microsoft test through runner 130450, `TestRunnerIsolCodeunit.Codeunit.al:16-40`).

use super::runner_reset::{RunnerReset, test_permissions};
use super::*;
use al_builtins::value::{Instance, Value};
use std::cell::RefCell;
use std::rc::Rc;

/// A test runner codeunit (object index) and the instance its OnRun runs on.
pub(crate) type RunnerRef = (usize, Rc<RefCell<Instance>>);

/// The outcome of each test of a codeunit, by name.
pub(crate) type Outcomes = Vec<(String, Outcome)>;

/// What runs around the OnRun trigger and each test method of a test codeunit.
pub(crate) enum Steps {
    /// Al-runner's own runner: the steps of Microsoft's Test Runner app, replayed (`runner_reset.rs`).
    Builtin(RunnerReset),
    /// A test runner codeunit's triggers, on the instance its OnRun runs on.
    Al {
        runner: usize,
        instance: Rc<RefCell<Instance>>,
    },
}

impl Steps {
    /// The steps of a unit: with `al` the runner whose OnRun runs the test codeunit; a scratch run (`execute`) has no
    /// runner.
    pub(crate) fn new(engine: &Engine, execute: bool, al: Option<(usize, Rc<RefCell<Instance>>)>) -> Steps {
        match al {
            Some((runner, instance)) => Steps::Al { runner, instance },
            None if execute => Steps::Builtin(RunnerReset::default()),
            None => Steps::Builtin(RunnerReset::of(engine)),
        }
    }

    pub(crate) fn is_al(&self) -> bool {
        matches!(self, Steps::Al { .. })
    }

    /// Before test method `routine`: `Ok(false)` when the runner skips it. The error is why the test is not run.
    pub(crate) fn before(
        &self,
        session: &mut Session,
        obj: &al_ir::Object,
        routine: &al_ir::Routine,
    ) -> Result<bool, String> {
        match self {
            Steps::Builtin(reset) => reset.before(session, obj, routine).map(|()| true),
            Steps::Al { runner, instance } => {
                let permissions = test_permissions(obj, routine);
                before_test_run(session, *runner, instance, obj, &routine.name, &permissions)
            }
        }
    }

    /// [`Steps::before`], with a skip as an error and `skipped` set.
    pub(crate) fn before_or_skip(
        &self,
        session: &mut Session,
        obj: &al_ir::Object,
        routine: &al_ir::Routine,
        skipped: &mut bool,
    ) -> Result<(), String> {
        match self.before(session, obj, routine)? {
            true => Ok(()),
            false => {
                *skipped = true;
                Err("skipped by the test runner".into())
            }
        }
    }

    /// After test method `routine`, which passed when `success`; nothing for a method the runner `skipped`.
    pub(crate) fn after(
        &self,
        session: &mut Session,
        obj: &al_ir::Object,
        routine: &al_ir::Routine,
        success: bool,
        skipped: bool,
    ) -> Result<(), String> {
        if skipped {
            return Ok(());
        }
        match self {
            Steps::Builtin(reset) => reset.after(session),
            Steps::Al { runner, instance } => {
                session.clear_stop();
                let permissions = test_permissions(obj, routine);
                after_test_run(session, *runner, instance, obj, &routine.name, &permissions, success)
            }
        }
    }

    /// Before the test codeunit's declared OnRun trigger: the runner gets `OnBeforeTestRun` with FunctionName
    /// `OnRun` (assumption `test-runner-trigger-calls`; `TestRunnerMgt.Codeunit.al:92-96` answers true for it). An
    /// answer of false is not probed and refuses the codeunit.
    pub(crate) fn before_on_run(&self, session: &mut Session, obj: &al_ir::Object) -> Result<(), String> {
        let Steps::Al { runner, instance } = self else {
            return Ok(());
        };
        let permissions = codeunit_permissions(obj);
        match before_test_run(session, *runner, instance, obj, "OnRun", &permissions)? {
            true => Ok(()),
            false => Err(format!(
                "the test runner codeunit {} answered false from OnBeforeTestRun for the OnRun trigger: what the platform does then is not probed (ADR-066 P3)",
                runner_name(session, *runner)
            )),
        }
    }

    /// After the test codeunit's declared OnRun trigger, which ended without an error when `success`.
    pub(crate) fn after_on_run(&self, session: &mut Session, obj: &al_ir::Object, success: bool) -> Result<(), String> {
        let Steps::Al { runner, instance } = self else {
            return Ok(());
        };
        let permissions = codeunit_permissions(obj);
        after_test_run(session, *runner, instance, obj, "OnRun", &permissions, success)
    }
}

/// The codeunit's TestPermissions (for its OnRun trigger), else Restrictive (`runner_reset::test_permissions`).
fn codeunit_permissions(obj: &al_ir::Object) -> String {
    obj.properties
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("TestPermissions"))
        .map_or_else(|| "Restrictive".into(), |(_, v)| v.trim().to_string())
}

fn runner_name(session: &Session, runner: usize) -> String {
    let o = &session.engine.program.objects[runner];
    format!("{} \"{}\"", o.id.unwrap_or(0), o.name)
}

/// The arguments the triggers start with: CodeunitID, CodeunitName, FunctionName, FunctionTestPermissions.
fn trigger_args(obj: &al_ir::Object, function: &str, permissions: &str) -> Vec<Value> {
    let members = al_builtins::sysenum::members("testpermissions").unwrap_or(&[]);
    let ordinal = members
        .iter()
        .position(|m| m.eq_ignore_ascii_case(permissions))
        .unwrap_or(0);
    let members: Arc<[String]> = members.iter().map(|m| m.to_string()).collect();
    vec![
        Value::Integer(obj.id.unwrap_or(0) as i32),
        Value::Text(obj.name.clone().into()),
        Value::Text(function.to_string().into()),
        Value::Option(i32::try_from(ordinal).unwrap_or(0), members),
    ]
}

fn trigger(session: &Session, runner: usize, name: &str) -> Option<usize> {
    session.engine.program.objects[runner]
        .routines
        .iter()
        .position(|r| r.is_trigger && r.name.eq_ignore_ascii_case(name))
}

fn before_test_run(
    session: &mut Session,
    runner: usize,
    instance: &Rc<RefCell<Instance>>,
    obj: &al_ir::Object,
    function: &str,
    permissions: &str,
) -> Result<bool, String> {
    al_builtins::assumptions::assume(Assumption::TestRunnerTriggerCalls);
    al_builtins::assumptions::assume(Assumption::TestRunnerTriggerTransactions);
    // A runner without the trigger runs every method (the platform calls no code).
    let Some(r) = trigger(session, runner, "OnBeforeTestRun") else {
        return Ok(true);
    };
    // The permission sets codeunit 130302 assigns from Microsoft's runner (ALTestRunnerResetEnvironment.Codeunit.al:54-62,
    // reached from this trigger through OnBeforeTestMethodRun) are the runner's, as in `runner_reset.rs`.
    session.runner_permission_step = true;
    let ran = session.call(runner, r, instance.clone(), trigger_args(obj, function, permissions));
    session.runner_permission_step = false;
    let answer = match ran {
        Ok((Value::Boolean(b), _)) => b,
        // A trigger that does not set its return value answers false, as any Boolean procedure does.
        Ok(_) => false,
        Err(e) => return Err(trigger_failed(session, runner, "OnBeforeTestRun", e)),
    };
    // NavTestExecution.BeforeTestRunAsync commits after the trigger (AL Runner v2 TestExecutor.cs; assumption
    // `test-runner-trigger-transactions`).
    session.commit_transaction();
    Ok(answer)
}

fn after_test_run(
    session: &mut Session,
    runner: usize,
    instance: &Rc<RefCell<Instance>>,
    obj: &al_ir::Object,
    function: &str,
    permissions: &str,
    success: bool,
) -> Result<(), String> {
    let Some(r) = trigger(session, runner, "OnAfterTestRun") else {
        return Ok(());
    };
    let mut args = trigger_args(obj, function, permissions);
    args.push(Value::Boolean(success));
    match session.call(runner, r, instance.clone(), args) {
        Ok(_) => Ok(()),
        Err(e) => Err(trigger_failed(session, runner, "OnAfterTestRun", e)),
    }
}

fn trigger_failed(session: &Session, runner: usize, which: &str, e: Signal) -> String {
    let name = runner_name(session, runner);
    match e {
        Signal::Unsupported(r) => format!("the test runner codeunit {name}'s {which} trigger was refused: {r}"),
        Signal::Error(e) => format!(
            "the test runner codeunit {name}'s {which} trigger failed (what the platform does then is not probed): {}",
            e.message
        ),
        other => format!(
            "the test runner codeunit {name}'s {which} trigger ended with {}",
            other.describe()
        ),
    }
}

/// The run of a unit's test codeunit, handed to the VM for the length of the runner's OnRun.
struct Host<'a, R> {
    /// The unit's test codeunit.
    object: usize,
    /// The test codeunit run of `run.rs`, with the runner and its instance.
    body: &'a mut dyn FnMut(&mut Session, RunnerRef) -> R,
    /// What `body` returned; `Some` once the runner ran the test codeunit.
    ran: Option<R>,
}

impl<R> al_vm::TestCodeunitHost for Host<'_, R> {
    fn run_test_codeunit(
        &mut self,
        session: &mut Session,
        object: usize,
        runner: (usize, Rc<RefCell<Instance>>),
    ) -> Result<(), Signal> {
        if object != self.object || self.ran.is_some() {
            let o = &session.engine.program.objects[object];
            return Err(Signal::Unsupported(format!(
                "the test runner runs test codeunit {} \"{}\" while al-runner runs another one or the same one again: a runner that picks its own test codeunits is not supported yet (ADR-066 decision 5)",
                o.id.unwrap_or(0),
                o.name
            )));
        }
        session.in_test_codeunit = true;
        let r = (self.body)(session, runner);
        session.in_test_codeunit = false;
        self.ran = Some(r);
        // The runner's Codeunit.Run returns and raises nothing, whatever the tests did (assumption
        // `test-runner-trigger-calls`, ADR-066 P5).
        Ok(())
    }
}

/// Runs the OnRun trigger of runner codeunit `runner` on `session`, with its `Codeunit.Run` of test codeunit `object`
/// going to `body`. The value is what `body` returned; the error is why the unit's tests are not run.
pub(crate) fn drive<R>(
    session: &mut Session,
    runner: usize,
    object: usize,
    body: &mut dyn FnMut(&mut Session, RunnerRef) -> R,
) -> Result<R, String> {
    let name = runner_name(session, runner);
    let Some(id) = session.engine.program.objects[runner].id else {
        return Err(format!("the test runner codeunit {name} has no id"));
    };
    let mut host = Host {
        object,
        body,
        ran: None,
    };
    // The runner's own code runs outside the test codeunit (the nesting check of `Codeunit.Run` holds only inside it).
    let in_test = std::mem::replace(&mut session.in_test_codeunit, false);
    let result = session.with_test_codeunit_host(&mut host, |s| s.run_codeunit_by_id(id));
    session.in_test_codeunit = in_test;
    match (result, host.ran) {
        (Ok(()), Some(r)) => Ok(r),
        (Ok(()), None) => Err(format!(
            "the test runner codeunit {name} did not run this test codeunit"
        )),
        (Err(Signal::Unsupported(r)), _) => Err(format!("the test runner codeunit {name} was refused: {r}")),
        (Err(Signal::Error(e)), _) => Err(format!(
            "the test runner codeunit {name} failed (what BC reports then is not pinned): {}",
            e.message
        )),
        (Err(other), _) => Err(format!(
            "the test runner codeunit {name} ended with {}",
            other.describe()
        )),
    }
}
