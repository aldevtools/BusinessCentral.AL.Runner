// The scans of production code stop at the first `#[cfg(test)]` of a file; a test module in a file of its own starts with one.
#[cfg(test)]
use super::*;
use al_sema::workspace::AppSource;

/// The outcomes of test codeunit `test` run through test runner codeunit `runner` (ADR-066 decision 1), or through
/// al-runner's own runner when `runner` is `None`.
fn outcomes_through(src: &str, test: u32, runner: Option<u32>) -> Vec<(String, Outcome)> {
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
    let engine = Engine::new(l.program.clone(), Vec::new().into_iter().collect());
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
        TestIsolation::Codeunit,
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
        if Got <> '+OnRun -OnRun:Yes +A_Passes -A_Passes:Yes +B_Fails -B_Fails:No +C_Skipped +D_SeesTheLog ' then
            Error('log %1', Got);
    end;
}
";

// ADR-066 decision 1: the runner's OnBeforeTestRun and OnAfterTestRun run around the OnRun trigger and each method; an
// answer of false skips the method (no row), and OnAfterTestRun gets IsSuccess (assumption `test-runner-trigger-calls`).
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
