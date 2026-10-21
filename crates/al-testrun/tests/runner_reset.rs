//! What BC's AL test runner does around every test method, in its order (Test Runner app, BC 27.5 source):
//!
//! 1. `Test Runner - Mgt`.PlatformBeforeTestRun starts or stops the permissions mock: `Codeunit.Run(131006)` when
//!    codeunit 131006 exists (TestRunnerMgt.Codeunit.al:105, :198-208).
//! 2. Its OnBeforeTestMethodRun event reaches "ALTestRunner Reset Environment".BeforeTestMethod
//!    (ALTestRunnerResetEnvironment.Codeunit.al:23-33): ClearLastError, ApplicationArea(''), then for a test method
//!    (not OnRun) ClearLegacyLibraries (:42-63): `Codeunit.Run(130301)` when it exists, and `Codeunit.Run(130302)`
//!    when it exists and the method's TestPermissions is not Disabled; then BindStopSystemTableChanges (:65-75):
//!    `Codeunit.Run(132553)` when it exists.
//! 3. The test method.
//! 4. PlatformAfterTestRun starts or stops the mock again (TestRunnerMgt.Codeunit.al:127).
//!
//! The codeunits live in Microsoft's apps (Tests-TestLibraries, Permissions Mock, Prevent Metadata Updates); here a
//! dependency app holds stand-ins with the same ids that write what ran into a log.

use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, Manifest, RunOptions, SuiteRun};

const BASE: &str = r#"
table 50900 "Reset Row" { fields { field(1; "No."; Integer) { } } keys { key(PK; "No.") { } } }

codeunit 50900 "Reset Log"
{
    SingleInstance = true;
    var
        Log: Text;
        Refuse: Boolean;
    procedure Add(What: Text) begin Log += What + ';'; end;
    procedure Get(): Text begin exit(Log); end;
    procedure Clear() begin Log := ''; end;
    procedure RefuseNextReset() begin Refuse := true; end;
    procedure TakeRefuse(): Boolean var R: Boolean; begin R := Refuse; Refuse := false; exit(R); end;
}

codeunit 131006 "Permissions Mock"
{
    SingleInstance = true;
    var
        Started: Boolean;
        L: Codeunit "Reset Log";
    trigger OnRun()
    begin
        if Started then L.Add('stop') else L.Add('start');
        Started := not Started;
    end;
}

codeunit 130301 "Reset State Before Test Run"
{
    SingleInstance = true;
    trigger OnRun()
    var
        L: Codeunit "Reset Log";
        R: Record "Reset Row";
    begin
        Randomize(1);
        L.Add('130301');
        if L.TakeRefuse() then
            R.LockTable(true, true);
    end;
}

codeunit 130302 "Set State Test Run"
{
    SingleInstance = true;
    trigger OnRun()
    var
        L: Codeunit "Reset Log";
    begin
        L.Add('130302');
    end;
}

codeunit 132553 "Block Changes to System Tables"
{
    SingleInstance = true;
    trigger OnRun()
    var
        L: Codeunit "Reset Log";
    begin
        L.Add('132553');
    end;
}
"#;

const MAIN: &str = r#"
codeunit 50910 "Reset Disabled"
{
    Subtype = Test;
    TestPermissions = Disabled;
    var
        L: Codeunit "Reset Log";
        AtOnRun: Text;
        FirstDraw: Integer;

    trigger OnRun()
    begin
        AtOnRun := L.Get();
    end;

    [Test]
    procedure A_First()
    begin
        if AtOnRun <> '' then
            Error('the runner reset before OnRun: %1', AtOnRun);
        if L.Get() <> 'start;130301;132553;' then
            Error('order before the first test: %1', L.Get());
        FirstDraw := Random(1000000);
    end;

    [Test]
    procedure B_Second()
    begin
        if L.Get() <> 'start;130301;132553;stop;start;130301;132553;' then
            Error('order before the second test: %1', L.Get());
        if Random(1000000) <> FirstDraw then
            Error('the second test drew another number than the first');
    end;
}

codeunit 50911 "Reset Default"
{
    Subtype = Test;
    var
        L: Codeunit "Reset Log";

    [Test]
    procedure WithoutTestPermissions()
    begin
        if L.Get() <> 'start;130301;130302;132553;' then
            Error('order: %1', L.Get());
    end;
}

codeunit 50912 "Reset Method Disabled"
{
    Subtype = Test;
    TestPermissions = NonRestrictive;
    var
        L: Codeunit "Reset Log";

    [Test]
    [TestPermissions(TestPermissions::Disabled)]
    procedure MethodDisabled()
    begin
        if L.Get() <> 'start;130301;132553;' then
            Error('order: %1', L.Get());
    end;

    [Test]
    [TestPermissions(TestPermissions::InheritFromTestCodeunit)]
    procedure MethodInherits()
    begin
        if L.Get() <> 'start;130301;132553;stop;start;130301;130302;132553;' then
            Error('order: %1', L.Get());
    end;
}

codeunit 50914 "Reset Random Failure"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure A_FailsAfterDrawing()
    begin
        if Random(10) > 0 then
            Error('first');
    end;

    [Test]
    procedure B_FailsAfterDrawing()
    begin
        if Random(10) > 0 then
            Error('second');
    end;
}

codeunit 50913 "Reset Refused"
{
    Subtype = Test;
    TestPermissions = Disabled;
    var
        L: Codeunit "Reset Log";

    [Test]
    procedure A_AsksForARefusal()
    begin
        L.RefuseNextReset();
    end;

    [Test]
    procedure B_ResetRefused()
    begin
        Error('the test ran although the reset before it was refused');
    end;
}
"#;

fn app(id: &str, deps: &[&str], src: &str) -> AppSource {
    AppSource {
        id: id.into(),
        name: id.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![(format!("{id}.al"), src.into())],
    }
}

fn run() -> CorpusRuntime {
    let manifest = |p: &str| Manifest {
        publisher: p.into(),
        version: "1.0.0.0".into(),
        resources: vec![],
        include_source: false,
        target: String::new(),
        layout_files: Vec::new(),
        id_ranges: Vec::new(),
        symbols: vec![],
    };
    CorpusRuntime::try_run_suites_with(
        vec![app("main", &["base"], MAIN), app("base", &[], BASE)],
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        &[manifest("Me"), manifest("Microsoft")],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions::default(),
    )
    .expect("the run starts")
}

fn outcome(rt: &CorpusRuntime, test: &str) -> Outcome {
    rt.results()
        .iter()
        .find(|r| r.test == test)
        .unwrap_or_else(|| panic!("{test} did not run"))
        .outcome
        .clone()
}

#[test]
fn the_runner_resets_before_every_test_method_in_bcs_order() {
    let rt = run();
    assert_eq!(outcome(&rt, "A_First"), Outcome::Pass);
    assert_eq!(outcome(&rt, "B_Second"), Outcome::Pass);
}

#[test]
fn codeunit_130302_runs_only_when_the_test_permissions_are_not_disabled() {
    let rt = run();
    assert_eq!(outcome(&rt, "WithoutTestPermissions"), Outcome::Pass);
    assert_eq!(outcome(&rt, "MethodDisabled"), Outcome::Pass);
    assert_eq!(outcome(&rt, "MethodInherits"), Outcome::Pass);
}

#[test]
fn a_reset_that_is_refused_refuses_the_test_and_names_the_codeunit() {
    let rt = run();
    assert_eq!(outcome(&rt, "A_AsksForARefusal"), Outcome::Pass);
    match outcome(&rt, "B_ResetRefused") {
        Outcome::NotRun(why) => assert!(
            why.contains("130301") && why.contains("Reset State Before Test Run") && why.contains("LockTable"),
            "{why}"
        ),
        other => panic!("{other:?}"),
    }
}

/// 130301 reseeds the `Random` sequence before every test method (`LibraryRandom.SetSeed(1)`), so what a test draws
/// does not depend on what the tests before it drew: a failure after a failed test that drew is trusted.
#[test]
fn after_the_runners_reseed_a_failure_that_draws_does_not_depend_on_earlier_draws() {
    let rt = run();
    assert_eq!(outcome(&rt, "A_FailsAfterDrawing"), Outcome::Fail("first".into()));
    assert_eq!(outcome(&rt, "B_FailsAfterDrawing"), Outcome::Fail("second".into()));
}
