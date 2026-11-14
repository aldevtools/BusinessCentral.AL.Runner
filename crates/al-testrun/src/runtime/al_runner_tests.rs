// The scans of production code stop at the first `#[cfg(test)]` of a file; a test module in a file of its own starts with one.
#[cfg(test)]
use super::*;
use al_sema::workspace::AppSource;

/// The outcomes of test codeunit `test` run through test runner codeunit `runner` (ADR-066 decision 1), or through
/// al-runner's own runner when `runner` is `None`.
fn outcomes_through(src: &str, test: u32, runner: Option<u32>) -> Vec<(String, Outcome)> {
    outcomes_through_on(src, test, runner, None)
}

/// [`outcomes_through`] on BC `bc` (a version such as "28.5"); `None` is the newest behavior.
fn outcomes_through_on(src: &str, test: u32, runner: Option<u32>, bc: Option<&str>) -> Vec<(String, Outcome)> {
    outcomes_through_isolated(src, test, runner, bc, TestIsolation::Codeunit)
}

/// [`outcomes_through_on`] under the runner's TestIsolation `isolation`.
fn outcomes_through_isolated(
    src: &str,
    test: u32,
    runner: Option<u32>,
    bc: Option<&str>,
    isolation: TestIsolation,
) -> Vec<(String, Outcome)> {
    let (ws, l) = al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), src.into())],
        }],
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let target = bc.map(|v| al_version::Target::for_bc(al_version::BcVersion::parse(v).unwrap()));
    let engine = Engine::with_target(l.program.clone(), Vec::new().into_iter().collect(), target);
    let index = |id: u32| {
        l.program
            .objects
            .iter()
            .position(|o| o.id == Some(id) && o.kind == al_ir::ObjKind::Codeunit)
            .unwrap()
    };
    let mut scratch = Scratch {
        runner: runner.map(index),
        ..Default::default()
    };
    run::run_codeunit_core(
        &engine,
        &l,
        index(test),
        None,
        isolation,
        None,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut Vec::new(),
        &mut scratch,
        None,
        false,
    )
}

const LOGGING_RUNNER: &str = "
table 60300 \"Runner Log\"
{
    fields
    {
        field(1; No; Integer) { }
        field(2; FunctionName; Text[100]) { }
        field(3; IsSuccess; Boolean) { }
        field(4; Before; Boolean) { }
    }
    keys { key(PK; No) { } }
}

codeunit 60301 \"Logging Runner\"
{
    Subtype = TestRunner;

    trigger OnRun()
    begin
        Codeunit.Run(60302);
    end;

    trigger OnBeforeTestRun(CodeunitId: Integer; CodeunitName: Text; FunctionName: Text; FunctionTestPermissions: TestPermissions): Boolean
    begin
        Log(FunctionName, false, true);
        exit(FunctionName <> 'C_Skipped');
    end;

    trigger OnAfterTestRun(CodeunitId: Integer; CodeunitName: Text; FunctionName: Text; FunctionTestPermissions: TestPermissions; IsSuccess: Boolean)
    begin
        Log(FunctionName, IsSuccess, false);
    end;

    local procedure Log(FunctionName: Text; IsSuccess: Boolean; Before: Boolean)
    var
        L: Record \"Runner Log\";
        N: Integer;
    begin
        if L.FindLast() then
            N := L.No;
        L.Init();
        L.No := N + 1;
        L.FunctionName := FunctionName;
        L.IsSuccess := IsSuccess;
        L.Before := Before;
        L.Insert();
    end;
}

codeunit 60302 \"Logged Tests\"
{
    Subtype = Test;

    trigger OnRun()
    begin
    end;

    [Test]
    procedure A_Passes()
    begin
    end;

    [Test]
    procedure B_Fails()
    begin
        Error('B fails');
    end;

    [Test]
    procedure C_Skipped()
    begin
        Error('C ran');
    end;

    [Test]
    procedure D_SeesTheLog()
    var
        L: Record \"Runner Log\";
        Got: Text;
    begin
        if L.FindSet() then
            repeat
                if L.Before then
                    Got += '+' + L.FunctionName + ' '
                else
                    Got += '-' + L.FunctionName + ':' + Format(L.IsSuccess) + ' ';
            until L.Next() = 0;
        if Got <> '+ +OnRun -OnRun:Yes +A_Passes -A_Passes:Yes +B_Fails -B_Fails:No +C_Skipped +D_SeesTheLog ' then
            Error('log %1', Got);
    end;
}
";

// ADR-066 decision 1: the runner's OnBeforeTestRun and OnAfterTestRun run for the codeunit itself (FunctionName ''), then
// around the OnRun trigger and each method; an answer of false skips the method (no row, no OnAfterTestRun), and
// OnAfterTestRun gets IsSuccess. Pinned on BC 27.5, 28.5 and 29.0 by the isolation-probe reader
// (corpus tests/al-language-isolation-probe/src/RpReader.Codeunit.al:21-62).
#[test]
fn a_runner_codeunit_gets_its_triggers_around_each_test_and_can_skip_one() {
    let got = outcomes_through(LOGGING_RUNNER, 60302, Some(60301));
    assert_eq!(
        got,
        vec![
            ("A_Passes".to_string(), Outcome::Pass),
            ("B_Fails".to_string(), Outcome::Fail("B fails".into())),
            ("D_SeesTheLog".to_string(), Outcome::Pass),
        ]
    );
}

const STUB_RUNNER: &str = "
codeunit 60310 \"Stub Runner\"
{
    Subtype = TestRunner;

    trigger OnRun()
    var
        Events: Codeunit \"Stub Events\";
    begin
        Events.OnBeforeCodeunitRun();
        Codeunit.Run(60313);
    end;
}

codeunit 60311 \"Stub Events\"
{
    [IntegrationEvent(false, false)]
    procedure OnBeforeCodeunitRun()
    begin
    end;
}

codeunit 60312 \"Stub Switch\"
{
    SingleInstance = true;
    var
        Enabled: Boolean;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Stub Events\", 'OnBeforeCodeunitRun', '', false, false)]
    local procedure EnableStub()
    begin
        Enabled := true;
    end;

    procedure IsEnabled(): Boolean
    begin
        exit(Enabled);
    end;
}

codeunit 60313 \"Stub Tests\"
{
    Subtype = Test;

    [Test]
    procedure TheStubIsOn()
    var
        Switch: Codeunit \"Stub Switch\";
    begin
        if not Switch.IsEnabled() then
            Error('the stub is off');
    end;
}
";

// Al-runner issue 6924's shape: a subscriber to an event the runner raises before it runs the test codeunit switches a
// stub on. Through the runner codeunit the test sees it; al-runner's own runner raises no such event.
#[test]
fn a_subscriber_to_an_event_the_runner_raises_before_the_codeunit_reaches_its_tests() {
    assert_eq!(
        outcomes_through(STUB_RUNNER, 60313, Some(60310)),
        vec![("TheStubIsOn".to_string(), Outcome::Pass)]
    );
    assert_eq!(
        outcomes_through(STUB_RUNNER, 60313, None),
        vec![("TheStubIsOn".to_string(), Outcome::Fail("the stub is off".into()))]
    );
}

// A runner whose OnRun does not run the unit's test codeunit: its tests are not run, with the reason.
#[test]
fn a_runner_that_does_not_run_the_codeunit_leaves_its_tests_not_run() {
    let src = STUB_RUNNER.replace("Codeunit.Run(60313);", "");
    let got = outcomes_through(&src, 60313, Some(60310));
    assert!(
        matches!(&got[..], [(n, Outcome::NotRun(why))] if n == "TheStubIsOn" && why.contains("did not run this test codeunit")),
        "{got:?}"
    );
}

// ADR-066 decision 5: `--test-runner ID` takes a runner that reads its tests from "Test Method Line" rows; one that picks
// its own codeunits in its OnRun is refused with a reason, and a codeunit that is no runner too.
#[test]
fn a_chosen_runner_must_read_its_tests_from_rows() {
    let (ws, l) = al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), STUB_RUNNER.into())],
        }],
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let engine = Engine::new(l.program.clone(), Vec::new().into_iter().collect());
    let choose = |r| al_runner::choose(r, &engine, TestIsolation::Codeunit, false);
    let own = choose(TestRunner::Codeunit(60310)).unwrap_err();
    assert!(own.contains("picks its own test codeunits"), "{own}");
    let not_runner = choose(TestRunner::Codeunit(60311)).unwrap_err();
    assert!(not_runner.contains("is not a test runner"), "{not_runner}");
    assert!(
        choose(TestRunner::Codeunit(1))
            .unwrap_err()
            .contains("not in the compiled apps")
    );
    // Without the Test Runner app Microsoft's runner falls back to al-runner's own, as on the TestRunnerHub path.
    assert_eq!(choose(TestRunner::Microsoft), Ok(None));
    assert_eq!(
        al_runner::choose(TestRunner::Codeunit(60310), &engine, TestIsolation::Codeunit, true),
        Ok(None)
    );
}

const PERF_RUNNER: &str = "
codeunit 60320 \"Perf Runner\"
{
    Subtype = TestRunner;

    trigger OnRun()
    var
        Events: Codeunit \"Perf Events\";
    begin
        Events.OnBeforeCodeunitRun();
        Codeunit.Run(60323);
        Events.OnAfterCodeunitRun();
    end;
}

codeunit 60321 \"Perf Events\"
{
    [IntegrationEvent(false, false)]
    procedure OnBeforeCodeunitRun()
    begin
    end;

    [IntegrationEvent(false, false)]
    procedure OnAfterCodeunitRun()
    begin
    end;
}

codeunit 60322 \"Perf Subscriber\"
{
    SingleInstance = true;

    var
        StartTime: Dictionary of [Text, DateTime];
        StartSql: Dictionary of [Text, BigInteger];
        MsgTok: Label 'Test perf signal: %1 ms, %2 SQL executes', Locked = true;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Perf Events\", 'OnBeforeCodeunitRun', '', false, false)]
    local procedure OnBeforeCodeunitRun()
    begin
        StartTime.Set('CU', CurrentDateTime());
        StartSql.Set('CU', SessionInformation.SqlStatementsExecuted());
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Perf Events\", 'OnAfterCodeunitRun', '', false, false)]
    local procedure OnAfterCodeunitRun()
    var
        DurationMs: Integer;
        SqlExecutes: BigInteger;
    begin
        if not StartTime.ContainsKey('CU') then
            exit;
        DurationMs := CurrentDateTime() - StartTime.Get('CU');
        SqlExecutes := SessionInformation.SqlStatementsExecuted() - StartSql.Get('CU');
        Session.LogMessage('PX0001', StrSubstNo(MsgTok, DurationMs, SqlExecutes),
            Verbosity::Normal, DataClassification::SystemMetadata, TelemetryScope::ExtensionPublisher, 'Category', 'PX');
    end;
}

codeunit 60323 \"Perf Tests\"
{
    Subtype = Test;

    [Test]
    procedure OnePlusOne()
    begin
        if 1 + 1 <> 2 then
            Error('math');
    end;
}
";

// Al-runner issue 7129: a per-codeunit performance telemetry subscriber to the runner's events logs the change of a SQL
// counter through StrSubstNo. The text only reaches Session.LogMessage, so the runner and the test run on.
#[test]
fn a_runner_subscriber_that_logs_a_sql_counter_change_through_strsubstno_runs() {
    assert_eq!(
        outcomes_through(PERF_RUNNER, 60323, Some(60320)),
        vec![("OnePlusOne".to_string(), Outcome::Pass)]
    );
}

/// A runner that logs every trigger call and what its `Ok := Codeunit.Run(...)` returned, then fails its own OnRun with
/// the log when the log is not `EXPECTED`: a wrong log turns every test into "not run" with the log in the reason. The
/// shape of the probe runners 61330 and 61331 (corpus tests/al-language-isolation-probe/src/RpRunnerLogic.Codeunit.al:24-33),
/// with the ClearLastError of Microsoft's bookkeeping after a method.
const CHECKING_RUNNER: &str = "
table 60330 \"Check Log\"
{
    fields
    {
        field(1; No; Integer) { }
        field(2; Entry; Text[250]) { }
    }
    keys { key(PK; No) { } }
}

codeunit 60331 \"Checking Runner\"
{
    Subtype = TestRunner;

    trigger OnRun()
    var
        L: Record \"Check Log\";
        Ok: Boolean;
        Got: Text;
    begin
        Commit();
        ClearLastError();
        Ok := Codeunit.Run(TESTCODEUNIT);
        Log('RUN:' + Format(Ok) + ' err=[' + GetLastErrorText() + ']');
        if L.FindSet() then
            repeat
                Got += L.Entry + ' ';
            until L.Next() = 0;
        if Got <> 'EXPECTED ' then
            Error('log %1', Got);
    end;

    trigger OnBeforeTestRun(CodeunitId: Integer; CodeunitName: Text; FunctionName: Text; FunctionTestPermissions: TestPermissions): Boolean
    begin
        Log('+' + FunctionName);
        exit(true);
    end;

    trigger OnAfterTestRun(CodeunitId: Integer; CodeunitName: Text; FunctionName: Text; FunctionTestPermissions: TestPermissions; IsSuccess: Boolean)
    begin
        Log('-' + FunctionName + ':' + Format(IsSuccess));
        // What Microsoft's PlatformAfterTestRun does for a method, and not for '' or OnRun (TestRunnerMgt.Codeunit.al).
        if (FunctionName <> '') and (FunctionName <> 'OnRun') then
            ClearLastError();
    end;

    local procedure Log(Entry: Text)
    var
        L: Record \"Check Log\";
        N: Integer;
    begin
        if L.FindLast() then
            N := L.No;
        L.Init();
        L.No := N + 1;
        L.Entry := CopyStr(Entry, 1, 250);
        L.Insert();
        Commit();
    end;
}

codeunit 60332 \"Failing Tests\"
{
    Subtype = Test;

    [Test]
    procedure A_Passes()
    begin
    end;

    [Test]
    procedure B_Fails()
    begin
        Error('B fails');
    end;
}

codeunit 60333 \"OnRun Raises\"
{
    Subtype = Test;

    trigger OnRun()
    begin
        Error('OnRun fails');
    end;

    [Test]
    procedure NeverRuns()
    begin
    end;
}

codeunit 60334 \"Passing Tests\"
{
    Subtype = Test;

    [Test]
    procedure A_Passes()
    begin
    end;
}
";

fn checking_runner(test: u32, expected: &str) -> String {
    CHECKING_RUNNER
        .replace("TESTCODEUNIT", &test.to_string())
        .replace("EXPECTED", expected)
}

// The codeunit's own '' calls come before its first method and after its last, the one after with IsSuccess false when a
// method failed; `Ok := Codeunit.Run(...)` returns false and raises nothing (RpReader.Codeunit.al:111-116, P5 under the
// probe runners on BC 27.5, 28.5 and 29.0; the empty last error comes from the ClearLastError of Microsoft's bookkeeping,
// which this runner copies).
#[test]
fn a_failing_method_makes_codeunit_run_return_false_after_the_codeunit_level_trigger_calls() {
    let src = checking_runner(
        60332,
        "+ +A_Passes -A_Passes:Yes +B_Fails -B_Fails:No -:No RUN:No err=[]",
    );
    let got = outcomes_through(&src, 60332, Some(60331));
    assert_eq!(
        got,
        vec![
            ("A_Passes".to_string(), Outcome::Pass),
            ("B_Fails".to_string(), Outcome::Fail("B fails".into())),
        ]
    );
}

// A failing OnRun: the OnRun calls with IsSuccess false, no method runs, and `Ok := Codeunit.Run(...)` returns false with
// the OnRun error as the last error. BC 27.5 and 28.5 call OnAfterTestRun for the codeunit itself ('') after it, BC 29.0
// does not (RpReader.Codeunit.al:119-125 and :195-202, the `#if BC29PLUS` guard).
#[test]
fn a_failing_on_run_makes_codeunit_run_return_false_and_bc_29_skips_the_codeunit_level_after_call() {
    // The runner did not fail its own OnRun (the log was right): the test is not run for its OnRun error alone.
    let on_run_failed = |got: Vec<(String, Outcome)>| {
        assert!(
            matches!(&got[..], [(n, Outcome::NotRun(why))] if n == "NeverRuns" && why.starts_with("OnRun failed")),
            "{got:?}"
        );
    };
    let before_29 = checking_runner(60333, "+ +OnRun -OnRun:No -:No RUN:No err=[OnRun fails]");
    on_run_failed(outcomes_through_on(&before_29, 60333, Some(60331), Some("28.5")));
    on_run_failed(outcomes_through_on(&before_29, 60333, Some(60331), Some("27.5")));
    let from_29 = checking_runner(60333, "+ +OnRun -OnRun:No RUN:No err=[OnRun fails]");
    on_run_failed(outcomes_through_on(&from_29, 60333, Some(60331), Some("29.0")));
}

// What `Ok := Codeunit.Run(...)` returns when every method passes is not probed (P5 measures only a failing method and a
// failing OnRun): the runner is refused, so its tests are not run.
#[test]
fn codeunit_run_of_a_passing_test_codeunit_with_its_result_used_is_refused() {
    let src = checking_runner(60334, "+ +A_Passes -A_Passes:Yes -:Yes RUN:Yes err=[]");
    let got = outcomes_through(&src, 60334, Some(60331));
    assert!(
        matches!(&got[..], [(n, Outcome::NotRun(why))] if n == "A_Passes" && why.contains("every test passed")),
        "{got:?}"
    );
}

// OnAfterTestRun of a failed method reads the failure as the last error; of a passing one an empty text
// (RpReader.Codeunit.al:44-53 P2, on BC 27.5, 28.5 and 29.0).
#[test]
fn on_after_test_run_of_a_failed_method_reads_the_failure_as_the_last_error() {
    let src = checking_runner(
        60332,
        "+ +A_Passes -A_Passes:Yes[] +B_Fails -B_Fails:No[B fails] -:No[] RUN:No err=[]",
    )
    .replace(
        "Log('-' + FunctionName + ':' + Format(IsSuccess));",
        "Log('-' + FunctionName + ':' + Format(IsSuccess) + '[' + GetLastErrorText() + ']');",
    );
    let got = outcomes_through(&src, 60332, Some(60331));
    assert_eq!(
        got,
        vec![
            ("A_Passes".to_string(), Outcome::Pass),
            ("B_Fails".to_string(), Outcome::Fail("B fails".into())),
        ]
    );
}

// A runner trigger that reaches an isolated storage key an earlier test changed under TestIsolation = Function is refused at
// that call, as a test is (al-runner issue 8002): OnBeforeTestRun leaves its test not run; a refused OnAfterTestRun keeps the
// result of its test, and a later failure is not trusted.
#[test]
fn a_runner_trigger_that_reaches_a_changed_key_is_refused() {
    let src = r#"
codeunit 60340 "Key Runner"
{
    Subtype = TestRunner;
    TestIsolation = Function;
    trigger OnRun() begin Codeunit.Run(60341); end;
    trigger OnBeforeTestRun(CodeunitId: Integer; CodeunitName: Text; FunctionName: Text; FunctionTestPermissions: TestPermissions): Boolean
    var V: Text;
    begin
        if FunctionName = 'B_BeforeReads' then
            if IsolatedStorage.Get('HK', DataScope::Module, V) then;
        exit(true);
    end;
    trigger OnAfterTestRun(CodeunitId: Integer; CodeunitName: Text; FunctionName: Text; FunctionTestPermissions: TestPermissions; IsSuccess: Boolean)
    var V: Text;
    begin
        if FunctionName = 'C_AfterReads' then
            if IsolatedStorage.Get('HK', DataScope::Module, V) then;
    end;
}
codeunit 60341 "Key Tests"
{
    Subtype = Test;
    trigger OnRun() begin IsolatedStorage.Set('HK', 'before', DataScope::Module); end;
    [Test] procedure A_Changes() begin IsolatedStorage.Set('HK', 'changed', DataScope::Module); end;
    [Test] procedure B_BeforeReads() begin end;
    [Test] procedure C_AfterReads() begin end;
    [Test] procedure D_Fails() begin Error('D fails'); end;
}
"#;
    let got = outcomes_through_isolated(src, 60341, Some(60340), None, TestIsolation::Function);
    let of = |t: &str| {
        &got.iter()
            .find(|(n, _)| n == t)
            .unwrap_or_else(|| panic!("{t}: {got:?}"))
            .1
    };
    assert_eq!(of("A_Changes"), &Outcome::Pass, "{got:?}");
    assert!(
        matches!(of("B_BeforeReads"), Outcome::NotRun(m) if m.contains("OnBeforeTestRun") && m.contains("isolated storage key 'HK'")),
        "{got:?}"
    );
    // Without the OnBeforeTestRun read, the refused OnAfterTestRun of C_AfterReads is what D_Fails' failure is not trusted on.
    let src = src.replace(
        "if FunctionName = 'B_BeforeReads' then",
        "if FunctionName = 'nothing' then",
    );
    let got = outcomes_through_isolated(&src, 60341, Some(60340), None, TestIsolation::Function);
    let of = |t: &str| {
        &got.iter()
            .find(|(n, _)| n == t)
            .unwrap_or_else(|| panic!("{t}: {got:?}"))
            .1
    };
    assert_eq!(of("B_BeforeReads"), &Outcome::Pass, "{got:?}");
    assert_eq!(of("C_AfterReads"), &Outcome::Pass, "{got:?}");
    assert!(
        matches!(of("D_Fails"), Outcome::NotRun(m) if m.contains("C_AfterReads") && m.contains("OnAfterTestRun") && m.contains("this failure is not trusted")),
        "{got:?}"
    );
}
