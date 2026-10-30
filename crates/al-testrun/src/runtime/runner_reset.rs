//! What BC's AL test runner does around every test method, as Microsoft's Test Runner app (BC 27.5,
//! `TestFramework/TestRunner/Test Runner.Source.zip`) does it. The platform calls the runner codeunit's
//! OnBeforeTestRun and OnAfterTestRun around each test method; for the OnRun trigger of a test codeunit nothing
//! below happens (TestRunnerMgt.Codeunit.al:92-96, ALTestRunnerResetEnvironment.Codeunit.al:28-29).
//!
//! Before the method, in this order:
//! 1. `Test Runner - Mgt`.PlatformBeforeTestRun runs codeunit 131006 "Permissions Mock" when it exists, which starts
//!    the mock, or stops it when it is started (TestRunnerMgt.Codeunit.al:104-105, :198-208).
//! 2. Its OnBeforeTestMethodRun event reaches "ALTestRunner Reset Environment".BeforeTestMethod
//!    (ALTestRunnerResetEnvironment.Codeunit.al:23-33): ClearLastError and ApplicationArea('') (:26-27), then
//!    ClearLegacyLibraries (:42-63): codeunit 130301 "Reset State Before Test Run" when it exists, then, unless the
//!    method's TestPermissions is Disabled, codeunit 130302 "Set State Test Run" when it exists; then
//!    BindStopSystemTableChanges (:65-75): codeunit 132553 "Block Changes to System Tables" when it exists.
//!
//! After the method, PlatformAfterTestRun runs codeunit 131006 again (TestRunnerMgt.Codeunit.al:127).
//!
//! Each codeunit is Microsoft's own code from the compiled apps (Tests-TestLibraries, Permissions Mock, Prevent
//! Metadata Updates), run by id with `Codeunit.Run` as the runner does. "Exists" is the runner's `AllObj` check: a
//! codeunit with that id in the loaded apps. When none of them is loaded (the corpus apps load only Permissions Mock)
//! only what is loaded runs. The steps of the Test Runner app itself are done here: our runner replaces it.

use super::*;

/// The codeunits the runner runs, found once per program.
#[derive(Debug, Default, Clone)]
pub(crate) struct RunnerReset {
    /// 131006 "Permissions Mock".
    mock: Option<(u32, String)>,
    /// 130301 "Reset State Before Test Run".
    reset_state: Option<(u32, String)>,
    /// 130302 "Set State Test Run".
    set_state: Option<(u32, String)>,
    /// 132553 "Block Changes to System Tables".
    block_changes: Option<(u32, String)>,
}

impl RunnerReset {
    /// Found in the engine's index of codeunits by id: this runs for every test codeunit, and a scan of the objects
    /// of a program with Microsoft's apps for each of the four ids was a measurable part of a run.
    pub(crate) fn of(engine: &Engine) -> RunnerReset {
        let [mock, reset_state, set_state, block_changes] = al_sema::check::demand::TEST_RUNNER_CODEUNITS.map(|id| {
            engine
                .codeunit_object(Some(id))
                .map(|o| (id, engine.program.objects[o].name.clone()))
        });
        RunnerReset {
            mock,
            reset_state,
            set_state,
            block_changes,
        }
    }

    /// Steps 1 and 2 before test method `routine` of test codeunit `obj`. The error is why the test is not run.
    pub(crate) fn before(
        &self,
        session: &mut Session,
        obj: &al_ir::Object,
        routine: &al_ir::Routine,
    ) -> Result<(), String> {
        run(session, &self.mock, "TestRunnerMgt.Codeunit.al:105")?;
        // ClearLastError and ApplicationArea('') (ALTestRunnerResetEnvironment.Codeunit.al:26-27).
        session.last_error = Default::default();
        session.start_test_method();
        run(
            session,
            &self.reset_state,
            "ALTestRunnerResetEnvironment.Codeunit.al:48-52",
        )?;
        if !test_permissions(obj, routine).eq_ignore_ascii_case("Disabled") {
            // 130302 assigns the test's permission sets through Permissions Mock (Set State Test Run.OnRun ->
            // Permission Test Catalog.InitializePermissionSetForTest -> Library - Lower Permissions
            // .StartLoggingNAVPermissions('D365 Full Access')); they are not enforced (assumption
            // `runner-permission-sets-grant-all`, server_classes.rs `permission_helper_add_set`).
            session.runner_permission_step = true;
            let r = run(
                session,
                &self.set_state,
                "ALTestRunnerResetEnvironment.Codeunit.al:54-62",
            );
            session.runner_permission_step = false;
            r?;
        }
        run(
            session,
            &self.block_changes,
            "ALTestRunnerResetEnvironment.Codeunit.al:70-74",
        )
    }

    /// The step after the test method.
    pub(crate) fn after(&self, session: &mut Session) -> Result<(), String> {
        session.clear_stop();
        run(session, &self.mock, "TestRunnerMgt.Codeunit.al:127")
    }
}

/// Codeunit 130440 "Library - Random" of Microsoft's test libraries.
pub(crate) const LIBRARY_RANDOM: u32 = 130440;

/// `Library - Random`.SetSeed(`seed`) on the session's single instance, as codeunit 130301 calls it with 1 (a
/// `random.librarySeed` variant of a stability run, ADR-058). Without codeunit 130440 in the program the step does
/// nothing (the command line says so). The error is why the test is not run.
pub(crate) fn library_seed(session: &mut Session, seed: i32) -> Result<(), String> {
    let engine = session.engine.clone();
    let Some(object) = engine.codeunit_object(Some(LIBRARY_RANDOM)) else {
        return Ok(());
    };
    let obj = &engine.program.objects[object];
    let Some(routine) = obj
        .routines
        .iter()
        .position(|r| !r.is_trigger && r.name.eq_ignore_ascii_case("SetSeed"))
    else {
        return Err(format!(
            "the stability variant sets the seed of codeunit {LIBRARY_RANDOM} \"{}\", which has no SetSeed procedure in this compile",
            obj.name
        ));
    };
    let instance = session.codeunit_instance(object);
    match session.call(
        object,
        routine,
        instance,
        vec![al_builtins::value::Value::Integer(seed)],
    ) {
        Ok(_) => Ok(()),
        Err(Signal::Unsupported(r)) => Err(format!(
            "the stability variant calls {}.SetSeed({seed}), and it was refused: {r}",
            obj.name
        )),
        Err(Signal::Error(e)) => Err(format!(
            "the stability variant calls {}.SetSeed({seed}), and it failed: {}",
            obj.name, e.message
        )),
        Err(other) => Err(format!(
            "the stability variant calls {}.SetSeed({seed}), and it ended with {}",
            obj.name,
            other.describe()
        )),
    }
}

fn run(session: &mut Session, codeunit: &Option<(u32, String)>, at: &str) -> Result<(), String> {
    let Some((id, name)) = codeunit else {
        return Ok(());
    };
    match session.run_codeunit_by_id(*id) {
        Ok(()) => Ok(()),
        Err(Signal::Unsupported(r)) => Err(format!(
            "BC's test runner runs codeunit {id} \"{name}\" around each test method (Test Runner app, {at}), and it was refused: {r}"
        )),
        Err(Signal::Error(e)) => Err(format!(
            "BC's test runner runs codeunit {id} \"{name}\" around each test method (Test Runner app, {at}), and it failed (what the runner does then is not pinned): {}",
            e.message
        )),
        Err(other) => Err(format!(
            "BC's test runner runs codeunit {id} \"{name}\" around each test method (Test Runner app, {at}), and it ended with {}",
            other.describe()
        )),
    }
}

/// The TestPermissions the platform passes for a test method: the method's `[TestPermissions]` attribute unless it is
/// InheritFromTestCodeunit, else the codeunit's property. With neither, BC passes Restrictive (AL Runner v2,
/// AlRunner/TestRunnerMgtEvents.cs:164-169, read from the platform's NavTestExecution.BeforeTestRunAsync; not pinned by
/// the corpus). Only "Disabled or not" matters here.
pub(crate) fn test_permissions(obj: &al_ir::Object, routine: &al_ir::Routine) -> String {
    routine
        .attributes
        .iter()
        .filter(|a| a.name.eq_ignore_ascii_case("TestPermissions"))
        .filter_map(|a| {
            a.text
                .split_once("::")
                .map(|(_, v)| v.trim_end_matches([')', ']', ' ']).trim().to_string())
        })
        .find(|v| !v.eq_ignore_ascii_case("InheritFromTestCodeunit"))
        .or_else(|| {
            obj.properties
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("TestPermissions"))
                .map(|(_, v)| v.trim().to_string())
        })
        .unwrap_or_else(|| "Restrictive".into())
}
