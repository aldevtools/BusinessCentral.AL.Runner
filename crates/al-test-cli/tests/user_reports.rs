//! Repros from outside users' issues, kept as regression tests. Each test names the issue and the corpus pin it relies on.

use al_sema::workspace::{AppKind, AppSource};
use al_test_cli::report::Outcome;
use al_test_cli::runtime::run_codeunit;
use al_vm::Engine;

/// Runs the test codeunit `T` of the workspace app `objects`. `apps` are dependency apps in order, each one
/// depending on the ones before it; the workspace app depends on all of them.
fn run_with_deps(apps: &[&str], objects: &str) -> Vec<(String, Outcome)> {
    let mut sources = Vec::new();
    let mut kinds = Vec::new();
    let mut ids: Vec<String> = Vec::new();
    for (i, src) in apps.iter().chain([&objects]).enumerate() {
        let id = format!("app{i}");
        sources.push(AppSource {
            id: id.clone(),
            name: id.clone(),
            dependencies: ids.clone(),
            files: vec![(format!("{id}.al"), src.to_string())],
        });
        kinds.push(if i < apps.len() {
            AppKind::Dependency
        } else {
            AppKind::Workspace
        });
        ids.push(id);
    }
    let (ws, l) = al_sema::lower::compile_apps(sources, kinds, &Default::default());
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let oi = l.program.objects.iter().position(|o| o.name == "T").unwrap();
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let engine = Engine::new(l.program.clone(), blocked);
    run_codeunit(&engine, &l, oi)
}

fn all_pass(results: Vec<(String, Outcome)>) {
    assert!(!results.is_empty());
    for (name, outcome) in results {
        assert_eq!(outcome, Outcome::Pass, "{name}");
    }
}

/// Issue 250: the enum field that a table extension of another app adds,
/// read from a TEMPORARY record after Insert and Get, is an enum value in a Variant, also before any value is
/// assigned (that default read as no type at all before PR 260), so `IsOption` is true (the
/// type test Library Assert's AreEqual makes) and it equals the enum constant. Pins: types/TestVariantExtended.al:261
/// (an enum in a Variant reports IsOption).
#[test]
fn issue_250_enum_field_of_extension_on_temporary_record_is_an_option() {
    let base = "table 69500 \"UR Base\" { fields { field(1; \"Document No.\"; Code[20]) { } field(2; \"Line No.\"; Integer) { } } keys { key(PK; \"Document No.\", \"Line No.\") { Clustered = true; } } }";
    {
        let fixture = "
enum 69501 \"UR Kind\" { Extensible = false; value(0; Normal) { } value(1; Other) { } }
tableextension 69501 \"UR Ext\" extends \"UR Base\" { fields { field(69501; \"UR Kind\"; Enum \"UR Kind\") { } } }";
        let main = "
codeunit 69502 T { Subtype = Test;
  [Test] procedure P() var Line: Record \"UR Base\" temporary; Kind: Enum \"UR Kind\"; V: Variant; begin
    Line.\"Document No.\" := 'REPRO'; Line.\"Line No.\" := 10000; Line.Insert();
    Line.Get('REPRO', 10000);
    V := Line.\"UR Kind\"; if not V.IsOption() then Error('default field value is not an option');
    Line.\"UR Kind\" := Kind::Other; Line.Modify(); Line.Get('REPRO', 10000);
    V := Line.\"UR Kind\"; if not V.IsOption() then Error('field is not an option');
    V := Kind::Normal; if not V.IsOption() then Error('constant is not an option');
    if Line.\"UR Kind\" <> Kind::Other then Error('not equal');
  end; }";
        all_pass(run_with_deps(&[base, fixture], main));
    }
}

/// Issue 255: `Info.SystemId(Value)` in method form sets the SystemId and `Info.SystemId()` reads it back.
/// Pin: errorinfo/lt-errnotif/EnTErrorInfoProps.al:106-115 and :259-262 (a SystemId round-trips).
#[test]
fn issue_255_error_info_system_id_method_form_round_trips() {
    let main = "
codeunit 69511 T { Subtype = Test;
  local procedure RoundTrip(Value: Guid): Guid var Info: ErrorInfo; begin
    Info := ErrorInfo.Create('Repro'); Info.SystemId(Value); exit(Info.SystemId());
  end;
  [Test] procedure P() var Value: Guid; begin
    Value := CreateGuid();
    if RoundTrip(Value) <> Value then Error('The error SystemId did not round-trip.');
  end; }";
    all_pass(run_with_deps(&[], main));
}

/// Issue 292: a procedure of a Page variable that never ran calls `CurrPage.Update(false)`; the call does nothing and the
/// procedure goes on. Pin: pages/lt-closed-page-variable-currpage/ClosedPageVariableCurrPageUpdate.al:98-114 (Update_NeverRun).
#[test]
fn issue_292_currpage_update_false_on_a_page_variable_that_never_ran_returns() {
    let main = "
table 69526 \"UR Row\" { fields { field(1; Number; Integer) { } } keys { key(PK; Number) { Clustered = true; } } }
page 69520 \"UR Unopened Part\" { PageType = ListPart; SourceTable = \"UR Row\"; SourceTableTemporary = true;
  layout { area(Content) { repeater(Rows) { field(Number; Rec.Number) { ApplicationArea = All; } } } }
  procedure RefreshBeforeOpen(): Boolean begin CurrPage.Update(false); exit(true); end; }
codeunit 69521 T { Subtype = Test;
  [Test] procedure P() var Part: Page \"UR Unopened Part\"; begin
    if not Part.RefreshBeforeOpen() then Error('The page procedure did not return.');
  end; }";
    all_pass(run_with_deps(&[], main));
}

/// Issue 293: `SessionInformation.SqlStatementsExecuted()` stays refused. The corpus pins only that the value is not negative
/// (session/TestMiscTypes.al:96-114), not the value itself; corpus issue #673 item 2 probes it.
#[test]
fn issue_293_sql_statements_executed_stays_refused() {
    let main = "
codeunit 69522 T { Subtype = Test;
  [Test] procedure P() begin
    if SessionInformation.SqlStatementsExecuted() < 0 then Error('The SQL statement count must not be negative.');
  end; }";
    let results = run_with_deps(&[], main);
    assert!(
        matches!(&results[..], [(_, Outcome::NotRun(why))] if why.contains("SqlStatementsExecuted")),
        "{results:?}"
    );
}

/// Issue 297: a page extension adds to the `Creation` action area of a page that declares no actions, and a TestPage of
/// the page opens and closes. Pin: pages/W1ActionAreas.al:44-115 and :137-157 (ExtensionActions_InImplicitAreas_Run: the
/// extension adds to Navigation, Creation, Reporting, Processing and Prompting of a page with no actions).
#[test]
fn issue_297_extension_adds_to_the_implicit_creation_area() {
    let main = "
table 69527 \"UR Creation Row\" { fields { field(1; Number; Integer) { } } keys { key(PK; Number) { Clustered = true; } } }
page 69523 \"UR Creation List\" { PageType = List; SourceTable = \"UR Creation Row\"; SourceTableTemporary = true;
  layout { area(Content) { repeater(Rows) { field(Number; Rec.Number) { ApplicationArea = All; } } } } }
pageextension 69524 \"UR Creation Ext\" extends \"UR Creation List\" {
  actions { addlast(Creation) { action(ProbeAction) { ApplicationArea = All; Caption = 'Probe'; } } } }
codeunit 69525 T { Subtype = Test;
  [Test] procedure P() var List: TestPage \"UR Creation List\"; begin
    List.OpenView(); List.Close();
  end; }";
    all_pass(run_with_deps(&[], main));
}

/// Found next to issue 292: `CurrPage.Close()` in a procedure of a Page variable that never ran does nothing either.
/// Pin: pages/lt-closed-page-variable-currpage/ClosedPageVariableCurrPageClose.al:98-114 (Close_NeverRun).
#[test]
fn issue_292_currpage_close_on_a_page_variable_that_never_ran_returns() {
    let main = "
table 69529 \"UR Close Row\" { fields { field(1; Number; Integer) { } } keys { key(PK; Number) { Clustered = true; } } }
page 69530 \"UR Unopened Close\" { PageType = List; SourceTable = \"UR Close Row\";
  layout { area(Content) { repeater(Rows) { field(Number; Rec.Number) { ApplicationArea = All; } } } }
  procedure CloseBeforeOpen(): Boolean begin CurrPage.Close(); exit(true); end; }
codeunit 69531 T { Subtype = Test;
  [Test] procedure P() var Pg: Page \"UR Unopened Close\"; begin
    if not Pg.CloseBeforeOpen() then Error('The page procedure did not return.');
  end; }";
    all_pass(run_with_deps(&[], main));
}
