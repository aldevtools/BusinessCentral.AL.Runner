// The scans of production code stop at the first `#[cfg(test)]` of a file; a test module in a file of its own starts with one.
#[cfg(test)]
use super::*;
use al_builtins::value::Value;
use al_sema::workspace::AppSource;

/// The Test Runner app's shape, cut down: its two tables, `Test Suite Mgt.`.RunNextTest and a runner codeunit that reads
/// its codeunit from the "Test Method Line" rows (as 130450 does, `TestRunnerMgt.Codeunit.al:25-58`) and stamps the
/// method lines with the clock around each test (`:83-144`), so their rows differ from run to run.
const RUNNER_APP: &str = r#"
table 130450 "Test Method Line"
{
    fields
    {
        field(1; "Test Suite"; Code[10]) { }
        field(2; "Line No."; Integer) { AutoIncrement = true; }
        field(3; "Line Type"; Option) { OptionMembers = "Codeunit","Function"; }
        field(4; "Test Codeunit"; Integer) { }
        field(5; Name; Text[128]) { }
        field(6; "Function"; Text[128]) { }
        field(7; Run; Boolean) { }
        field(8; Result; Option) { OptionMembers = " ",Failure,Success,Skipped; }
        field(10; "Start Time"; DateTime) { }
        field(11; "Finish Time"; DateTime) { }
        field(12; Level; Integer) { }
    }
    keys { key(PK; "Test Suite", "Line No.") { } }
}

table 130451 "AL Test Suite"
{
    fields
    {
        field(1; Name; Code[10]) { }
        field(8; "Test Runner Id"; Integer) { }
    }
    keys { key(PK; Name) { } }
}

codeunit 130456 "Test Suite Mgt."
{
    procedure RunNextTest(var TestMethodLine: Record "Test Method Line"): Boolean
    var
        Suite: Record "AL Test Suite";
    begin
        Suite.Get(TestMethodLine."Test Suite");
        TestMethodLine.Reset();
        TestMethodLine.SetRange("Test Suite", Suite.Name);
        TestMethodLine.SetRange("Line Type", TestMethodLine."Line Type"::Codeunit);
        if not TestMethodLine.FindFirst() then
            exit(false);
        Codeunit.Run(Suite."Test Runner Id", TestMethodLine);
        exit(true);
    end;
}

codeunit 130460 "Rows Runner"
{
    Subtype = TestRunner;
    TableNo = "Test Method Line";
    TestIsolation = Codeunit;

    trigger OnRun()
    begin
        Codeunit.Run(Rec."Test Codeunit");
    end;

    trigger OnBeforeTestRun(CodeunitId: Integer; CodeunitName: Text; FunctionName: Text; FunctionTestPermissions: TestPermissions): Boolean
    var
        L: Record "Test Method Line";
    begin
        if (FunctionName = '') or (FunctionName = 'OnRun') then
            exit(true);
        L.SetRange("Test Codeunit", CodeunitId);
        L.SetRange("Function", FunctionName);
        if L.FindFirst() then begin
            L."Start Time" := CurrentDateTime();
            L.Modify();
        end;
        exit(true);
    end;

    trigger OnAfterTestRun(CodeunitId: Integer; CodeunitName: Text; FunctionName: Text; FunctionTestPermissions: TestPermissions; IsSuccess: Boolean)
    var
        L: Record "Test Method Line";
    begin
        if (FunctionName = '') or (FunctionName = 'OnRun') then
            exit;
        L.SetRange("Test Codeunit", CodeunitId);
        L.SetRange("Function", FunctionName);
        if L.FindFirst() then begin
            L."Finish Time" := CurrentDateTime();
            if IsSuccess then
                L.Result := L.Result::Success
            else
                L.Result := L.Result::Failure;
            L.Modify();
        end;
        Commit();
    end;
}

table 60470 "Spec Row"
{
    fields { field(1; No; Integer) { } }
    keys { key(PK; No) { } }
}
"#;

/// A test codeunit with one test whose body is `body`.
fn test_codeunit(id: u32, body: &str) -> String {
    format!(
        r#"
codeunit {id} "Spec Tests {id}"
{{
    Subtype = Test;

    [Test]
    procedure TheTest()
    var
        L: Record "Test Method Line";
        Row: Record "Spec Row";
        A: BigInteger;
        B: BigInteger;
    begin
        {body}
    end;
}}
"#
    )
}

/// The compiled apps of `src` with the runner app.
fn compiled(src: &str) -> (Arc<Engine>, Lowered) {
    let (ws, l) = al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("runner.al".into(), RUNNER_APP.into()), ("f.al".into(), src.into())],
        }],
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let engine = Engine::new(l.program.clone(), Vec::new().into_iter().collect());
    (engine, l)
}

fn codeunit(l: &Lowered, id: u32) -> usize {
    l.program
        .objects
        .iter()
        .position(|o| o.id == Some(id) && o.kind == al_ir::ObjKind::Codeunit)
        .unwrap()
}

/// The outcomes of test codeunit `test` run through the runner from `baseline`, and whether nothing in its session saw
/// what the Test Runner's tables held before the runner's rows were written (`suite_rows::take_unseen`).
fn run_from(
    engine: &Arc<Engine>,
    l: &Lowered,
    test: u32,
    baseline: Option<&al_vm::Baseline>,
) -> (Vec<(String, Outcome)>, bool) {
    super::suite_rows::take_unseen();
    let mut scratch = Scratch {
        runner: Some(codeunit(l, 130460)),
        ..Default::default()
    };
    let out = run::run_codeunit_core(
        engine,
        l,
        codeunit(l, test),
        baseline,
        TestIsolation::Codeunit,
        None,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut Vec::new(),
        &mut scratch,
        None,
        false,
    );
    (out, super::suite_rows::take_unseen())
}

/// A baseline whose "Test Method Line" holds `n` lines of an earlier codeunit, as a carry chain hands them on.
fn baseline_with_lines(engine: &Arc<Engine>, l: &Lowered, n: i32) -> al_vm::Baseline {
    let table = l
        .program
        .objects
        .iter()
        .position(|o| o.id == Some(130450) && o.kind == al_ir::ObjKind::Table)
        .unwrap();
    let mut s = Session::new(engine.clone());
    for i in 1..=n {
        s.insert_row(
            table,
            &[
                (1, Value::Text("DEFAULT".to_string().into())),
                (2, Value::Integer(i * 10_000)),
                (4, Value::Integer(60999)),
                (6, Value::Text(format!("Earlier{i}").into())),
            ],
        )
        .expect("a line");
    }
    s.commit_transaction();
    s.capture_baseline()
}

fn pass() -> Vec<(String, Outcome)> {
    vec![("TheTest".to_string(), Outcome::Pass)]
}

// A run whose code does not touch the Test Runner's tables before the runner writes its rows starts the same way
// whatever they held: the rows of an earlier codeunit are deleted first, unseen.
#[test]
fn a_unit_that_does_not_touch_the_runner_rows_before_the_reset_leaves_them_unseen() {
    let (engine, l) = compiled(&test_codeunit(60471, "Row.No := 1; Row.Insert();"));
    assert_eq!(run_from(&engine, &l, 60471, None), (pass(), true));
    let earlier = baseline_with_lines(&engine, &l, 30);
    assert_eq!(run_from(&engine, &l, 60471, Some(&earlier)), (pass(), true));
}

// ADR-066 decision 4: a test whose own code reads or writes the Test Runner's rows is not run (on BC the table holds
// the other codeunits' lines too); the runner's own triggers around it read and write them freely.
#[test]
fn a_test_that_reads_the_runner_rows_is_not_run() {
    let (engine, l) = compiled(&test_codeunit(60472, "if L.FindFirst() then;"));
    let (out, unseen) = run_from(&engine, &l, 60472, None);
    assert!(!unseen, "what the test did to the rows may depend on what they held");
    assert_eq!(out.len(), 1);
    match &out[0].1 {
        Outcome::NotRun(why) => assert!(why.contains("Test Method Line") && why.contains("ADR-066"), "{why}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_test_codeunit_whose_on_run_reads_the_runner_rows_runs_none_of_its_tests() {
    let src = test_codeunit(60473, "").replace(
        "Subtype = Test;",
        "Subtype = Test;\n    trigger OnRun() var S: Record \"AL Test Suite\"; begin if S.IsEmpty() then; end;",
    );
    let (engine, l) = compiled(&src);
    let (out, unseen) = run_from(&engine, &l, 60473, None);
    assert!(!unseen, "what OnRun did to the rows may depend on what they held");
    match &out[0].1 {
        Outcome::NotRun(why) => assert!(why.starts_with("OnRun:") && why.contains("AL Test Suite"), "{why}"),
        other => panic!("{other:?}"),
    }
}

// A subscriber to an event of either table could see the rows the reset deletes (a DeleteAll with delete events
// deletes row by row): the rows are not unseen for that run, so the carry chain compares them.
#[test]
fn a_subscriber_to_an_event_of_the_runner_tables_makes_their_rows_seen() {
    let src = format!(
        "{}\ncodeunit 60475 \"Line Watcher\"\n{{\n    [EventSubscriber(ObjectType::Table, Database::\"Test Method Line\", 'OnAfterDeleteEvent', '', false, false)]\n    local procedure Deleted(var Rec: Record \"Test Method Line\") begin end;\n}}\n",
        test_codeunit(60474, "")
    );
    let (engine, l) = compiled(&src);
    let earlier = baseline_with_lines(&engine, &l, 3);
    assert_eq!(run_from(&engine, &l, 60474, Some(&earlier)), (pass(), false));
}

/// A `Global Triggers` setup subscriber that turns on OnDatabaseDelete for table `table` (a Database:: name).
fn setup_subscriber(table: &str) -> String {
    format!(
        "codeunit 60477 \"Setup Subscriber\"\n{{\n    [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Global Triggers\", GetDatabaseTableTriggerSetup, '', false, false)]\n    local procedure Setup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean)\n    begin\n        if TableId = Database::\"{table}\" then\n            OnDatabaseDelete := true;\n    end;\n\n    [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Global Triggers\", OnDatabaseDelete, '', false, false)]\n    local procedure Deleted(RecRef: RecordRef) begin end;\n}}\n"
    )
}

// The Global Triggers setup decides per table: turned on for a runner table, the reset raises a database event per
// deleted row, so its rows are seen; turned on for another table only, they stay unseen.
#[test]
fn a_global_trigger_setup_on_a_runner_table_makes_its_rows_seen_and_one_on_another_table_does_not() {
    for (table, unseen) in [
        ("Test Method Line", false),
        ("AL Test Suite", false),
        ("Spec Row", true),
    ] {
        let src = format!("{}\n{}", test_codeunit(60476, ""), setup_subscriber(table));
        let (engine, l) = compiled(&src);
        let earlier = baseline_with_lines(&engine, &l, 3);
        assert_eq!(
            run_from(&engine, &l, 60476, Some(&earlier)),
            (pass(), unseen),
            "{table}"
        );
    }
}

// The SQL counters: the reset runs before any AL code of the unit can read a counter, and AL sees only differences of
// two reads (al-builtins `sqlcount.rs`), so a difference a test takes is the same whatever the reset deleted.
#[test]
fn a_counter_difference_in_a_test_is_the_same_whatever_rows_the_reset_deleted() {
    let body = "A := SessionInformation.SqlStatementsExecuted();
        B := SessionInformation.SqlStatementsExecuted();
        if B - A <> 0 then Error('the counter moved with no work between');
        Row.No := 1;
        Row.Insert();
        Commit();
        A := SessionInformation.SqlStatementsExecuted();
        Row.Get(1);
        B := SessionInformation.SqlStatementsExecuted();
        if not (B > A) then Error('a Get after a Commit sent no statement');";
    let (engine, l) = compiled(&test_codeunit(60478, body));
    let none = run_from(&engine, &l, 60478, None);
    assert_eq!(none, (pass(), true));
    for n in [1, 40] {
        let earlier = baseline_with_lines(&engine, &l, n);
        assert_eq!(run_from(&engine, &l, 60478, Some(&earlier)), none, "{n} earlier lines");
    }
}

// AL code the session runs before the runner writes its rows (here an install codeunit, which a session without an
// install baseline runs first) and that reads the Test Runner's tables makes the session's start depend on their rows.
#[test]
fn install_code_that_reads_the_runner_rows_before_the_reset_makes_them_seen() {
    let src = format!(
        "{}\ncodeunit 60479 \"Reads Lines\"\n{{\n    Subtype = Install;\n    trigger OnInstallAppPerCompany() var L: Record \"Test Method Line\"; begin if L.IsEmpty() then; end;\n}}\n",
        test_codeunit(60480, "")
    );
    let (engine, l) = compiled(&src);
    assert_eq!(run_from(&engine, &l, 60480, None), (pass(), false));
}
