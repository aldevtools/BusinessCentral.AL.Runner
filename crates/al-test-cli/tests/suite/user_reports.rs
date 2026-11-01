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

/// Issues 293 and 5580: `SessionInformation.SqlStatementsExecuted()` answers whether it is negative (no: session/TestMiscTypes.al:96-114,
/// session/lt-bl-sql-counters/SqlCounters.al:74-84); an exact count is not pinned and refuses the test.
#[test]
fn issue_293_sql_statements_executed_answers_only_the_pinned_comparisons() {
    let main = "
codeunit 69522 T { Subtype = Test;
  [Test] procedure P() begin
    if SessionInformation.SqlStatementsExecuted() < 0 then Error('The SQL statement count must not be negative.');
  end;
  [Test] procedure Q() begin
    if SessionInformation.SqlStatementsExecuted() <> 3 then Error('Three statements were expected.');
  end; }";
    let results = run_with_deps(&[], main);
    assert!(
        matches!(&results[..], [(_, Outcome::Pass), (_, Outcome::NotRun(why))] if why.contains("SQL counter")),
        "{results:?}"
    );
}

/// al-runner issue 5761: the reporter's statement budget, 10 inserts and a Count right after a Commit, within (0, 100].
/// Pin: session/lt-bl-sql-counters/SqlCounters.al:282-305 (RO_Counters_StatementBudget_Repro, 3 statements); 11 inserts
/// are not measured and refuse.
#[test]
fn issue_5761_statement_budget_of_inserts_and_count() {
    let test = |n: u32| {
        format!(
            "
table 50124 \"Repro Row 50124\" {{ fields {{ field(1; Id; Integer) {{ }} field(2; Name; Text[30]) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}
codeunit 50125 T {{ Subtype = Test; TestPermissions = Disabled;
  [Test] procedure P() var Row: Record \"Repro Row 50124\"; Before: BigInteger; After: BigInteger; Executed: BigInteger; i: Integer;
  begin
    Row.DeleteAll();
    Commit();
    Before := SessionInformation.SqlStatementsExecuted();
    for i := 1 to {n} do begin Row.Init(); Row.Id := i; Row.Name := 'Row'; Row.Insert(); end;
    if Row.Count() <> {n} then Error('Expected {n} rows.');
    After := SessionInformation.SqlStatementsExecuted();
    Executed := After - Before;
    if not ((Executed > 0) and (Executed <= 100)) then Error('The statements executed are outside the budget of 1 to 100.');
  end; }}"
        )
    };
    all_pass(run_with_deps(&[], &test(10)));
    let results = run_with_deps(&[], &test(11));
    assert!(
        matches!(&results[..], [(_, Outcome::NotRun(why))] if why.contains("SQL counter")),
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

/// Issue 6043: a page extension of another app declares a `local procedure` with the name and parameter types of a
/// `protected procedure` of its base page (Customer Card's ActivateFields). alc compiles it: only an extension procedure
/// that code outside the extension can reach clashes (AL0440, must-not-compile 0003 and 0042, public against
/// protected). The reporter's repro: the extension compiles and its test runs.
#[test]
fn issue_6043_extension_local_procedure_named_like_a_protected_base_procedure_compiles() {
    let base = "table 50200 \"Repro Rec\" { fields { field(1; \"No.\"; Code[10]) { } } keys { key(PK; \"No.\") { Clustered = true; } } }
page 50201 \"Repro Card\" { PageType = Card; SourceTable = \"Repro Rec\";
    layout { area(Content) { field(\"No.\"; Rec.\"No.\") { } } }
    protected procedure ActivateFields() begin end; }";
    let ext =
        "pageextension 50300 \"Repro Card Ext\" extends \"Repro Card\" { local procedure ActivateFields() begin end; }
codeunit 50301 T { Subtype = Test; [Test] procedure Compiles() begin end; }";
    all_pass(run_with_deps(&[base], ext));
}

/// Issue 6043: which body runs. In the extension's code a bare call runs the extension's own local procedure, in the
/// base page's code the base page's protected one; a call from outside on a page variable runs the base page's public
/// procedure, as the extension's local one is not visible there. alc binds the calls that way (a probe with alc: the extension's
/// local `F(): Text` next to the base's protected `F(): Integer` makes `I := F()` in the extension a Text-to-Integer
/// error); corpus pageext/lt-ext-local-same-name/ExtLocalSameNameTests.al:48-72 pins it on real BC.
#[test]
fn issue_6043_a_bare_call_runs_the_procedure_of_the_calling_object() {
    let base = "table 50200 \"Repro Rec\" { fields { field(1; \"No.\"; Code[10]) { } } keys { key(PK; \"No.\") { Clustered = true; } } }
codeunit 50202 \"Repro Log\" { SingleInstance = true; var L: Text;
    procedure Add(T: Text) begin L += T + ';'; end; procedure Take(): Text begin exit(L); end; }
page 50201 \"Repro Card\" { PageType = Card; SourceTable = \"Repro Rec\";
    protected procedure ActivateFields() var Log: Codeunit \"Repro Log\"; begin Log.Add('base'); end;
    procedure FromBase() begin ActivateFields(); end;
    procedure Shared() var Log: Codeunit \"Repro Log\"; begin Log.Add('base-shared'); end; }";
    let ext = "pageextension 50300 \"Repro Card Ext\" extends \"Repro Card\" {
    local procedure ActivateFields() var Log: Codeunit \"Repro Log\"; begin Log.Add('ext'); end;
    local procedure Shared() var Log: Codeunit \"Repro Log\"; begin Log.Add('ext-shared'); end;
    procedure FromExt() begin ActivateFields(); Shared(); end; }
codeunit 50301 T { Subtype = Test;
    [Test] procedure Calls() var P: Page \"Repro Card\"; Log: Codeunit \"Repro Log\"; begin
        P.FromBase(); P.FromExt(); P.Shared();
        if Log.Take() <> 'base;ext;ext-shared;base-shared;' then Error('got %1', Log.Take());
    end; }";
    all_pass(run_with_deps(&[base], ext));
}

/// Compiles like `run_with_deps` and returns the AL codes of the must-not-compile checks (`alcodes`) with the outcomes
/// (none when there is a code: alc does not compile the app).
fn codes_and_outcomes(apps: &[&str], objects: &str) -> (Vec<&'static str>, Vec<(String, Outcome)>) {
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
    let codes: Vec<&'static str> = al_sema::alcodes::check_workspace(&ws).iter().map(|d| d.code).collect();
    if !codes.is_empty() {
        return (codes, Vec::new());
    }
    let oi = l.program.objects.iter().position(|o| o.name == "T").unwrap();
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let engine = Engine::new(l.program.clone(), blocked);
    (codes, run_codeunit(&engine, &l, oi))
}

const TWIN_TABLE: &str = "table 50200 \"Repro Rec\" { fields { field(1; \"No.\"; Code[10]) { } } keys { key(PK; \"No.\") { Clustered = true; } } }
codeunit 50202 \"Repro Log\" { SingleInstance = true; var L: Text;
    procedure Add(T: Text) begin L += T + ';'; end; procedure Take(): Text begin exit(L); end; }";

/// A page (and a report) with procedure `ActivateFields` of access `base`, and the code that calls it from the base.
fn twin_base(base: &str) -> String {
    format!(
        "{TWIN_TABLE}
page 50201 \"Repro Card\" {{ PageType = Card; SourceTable = \"Repro Rec\";
    {base} procedure ActivateFields() var Log: Codeunit \"Repro Log\"; begin Log.Add('base'); end;
    procedure FromBase() begin ActivateFields(); end; }}
report 50203 \"Repro Rep\" {{ ProcessingOnly = true; dataset {{ dataitem(R; \"Repro Rec\") {{ }} }}
    {base} procedure ActivateFields() begin end; }}"
    )
}

/// Issue 6043 (the part PR 6052 left): a page extension procedure with the name and parameter types of a procedure of
/// the page it extends. alc, every access pair (`public` is no keyword), page and report extension:
/// - another app, both public or protected: the page extension compiles with warning AL0523; the report extension is
///   error AL0440;
/// - another app, `internal` on either side: both compile;
/// - the same app, both public or protected: AL0440, page and report;
/// - the same app, `internal` on either side: the declarations compile, a bare call in the extension is AL0196.
///
/// Which body a call runs is pinned only for an internal extension procedure beside a public or protected page procedure
/// of another app (pageext/lt-wd/WdPageExtInternalTwin.al); in the other shapes that compile every call of the name
/// refuses: a bare call in the base and in the extension, and a call on a page variable from outside.
#[test]
fn issue_6043_extension_procedure_named_like_a_base_procedure_compiles_as_alc_does_and_its_calls_refuse() {
    let refused = |r: &[(String, Outcome)], name: &str| {
        let o = &r
            .iter()
            .find(|(n, _)| n.ends_with(name))
            .unwrap_or_else(|| panic!("{name}: {r:?}"))
            .1;
        assert!(
            matches!(o, Outcome::NotRun(why) if why.contains("ActivateFields") && why.contains("not pinned")),
            "{name}: {o:?}"
        );
    };
    let passes = |r: &[(String, Outcome)], name: &str| {
        let o = &r
            .iter()
            .find(|(n, _)| n.ends_with(name))
            .unwrap_or_else(|| panic!("{name}: {r:?}"))
            .1;
        assert_eq!(o, &Outcome::Pass, "{name}");
    };
    // Another app.
    for (ext, base, outside) in [
        ("", "", true),
        ("", "protected", true),
        ("protected", "", true),
        ("protected", "protected", false),
        ("", "internal", true),
        ("protected", "internal", false),
        ("internal", "", true),
        ("internal", "protected", true),
        ("internal", "internal", true),
    ] {
        let both_visible = !ext.contains("internal") && !base.contains("internal");
        let page_ext = format!(
            "pageextension 50300 \"Repro Card Ext\" extends \"Repro Card\" {{
    {ext} procedure ActivateFields() var Log: Codeunit \"Repro Log\"; begin Log.Add('ext'); end;
    procedure FromExt() begin ActivateFields(); end; }}"
        );
        let outside_test = if outside {
            "[Test] procedure Outside() var P: Page \"Repro Card\"; begin P.ActivateFields(); end;"
        } else {
            ""
        };
        let tests = format!(
            "codeunit 50301 T {{ Subtype = Test;
    [Test] procedure Base() var P: Page \"Repro Card\"; begin P.FromBase(); end;
    [Test] procedure Ext() var P: Page \"Repro Card\"; begin P.FromExt(); end;
    {outside_test}
    [Test] procedure Other() begin end; }}"
        );
        let label = format!("another app, ext `{ext}`, base `{base}`");
        let (codes, r) = codes_and_outcomes(&[&twin_base(base)], &format!("{page_ext}\n{tests}"));
        assert!(codes.is_empty(), "{label}: {codes:?}");
        // An internal extension procedure beside a public or protected page procedure: the base's own call runs the base's,
        // the extension's own call its own (pageext/lt-wd/WdPageExtInternalTwin.al, al-runner issue 6294). A Page variable
        // call from the extension's own app is not pinned.
        if ext == "internal" && base != "internal" {
            passes(&r, "Base");
            passes(&r, "Ext");
        } else {
            refused(&r, "Base");
            refused(&r, "Ext");
        }
        if outside {
            refused(&r, "Outside");
        }
        passes(&r, "Other");
        // The report extension of another app: AL0440 where both are visible, else it compiles.
        let rep_ext = format!(
            "reportextension 50302 \"Repro Rep Ext\" extends \"Repro Rep\" {{ {ext} procedure ActivateFields() begin end; }}"
        );
        let (codes, _) = codes_and_outcomes(&[&twin_base(base)], &format!("{rep_ext}\n{tests}"));
        let want: Vec<&str> = if both_visible { vec!["AL0440"] } else { vec![] };
        assert_eq!(codes, want, "report, {label}");
    }
    // The same app.
    for (ext, base) in [
        ("", ""),
        ("", "protected"),
        ("protected", ""),
        ("protected", "protected"),
        ("", "internal"),
        ("protected", "internal"),
        ("internal", ""),
        ("internal", "protected"),
        ("internal", "internal"),
    ] {
        let both_visible = !ext.contains("internal") && !base.contains("internal");
        let src = format!(
            "{}
pageextension 50300 \"Repro Card Ext\" extends \"Repro Card\" {{
    {ext} procedure ActivateFields() var Log: Codeunit \"Repro Log\"; begin Log.Add('ext'); end; }}
reportextension 50302 \"Repro Rep Ext\" extends \"Repro Rep\" {{ {ext} procedure ActivateFields() begin end; }}
codeunit 50301 T {{ Subtype = Test;
    [Test] procedure Base() var P: Page \"Repro Card\"; begin P.FromBase(); end;
    [Test] procedure Other() begin end; }}",
            twin_base(base)
        );
        let label = format!("same app, ext `{ext}`, base `{base}`");
        let (codes, r) = codes_and_outcomes(&[], &src);
        if both_visible {
            assert_eq!(codes, vec!["AL0440", "AL0440"], "{label}");
        } else {
            assert!(codes.is_empty(), "{label}: {codes:?}");
            refused(&r, "Base");
            passes(&r, "Other");
        }
    }
}

/// Outcomes of the test codeunit `codeunit` of a reporter's repro in fixtures/outside-fb, with the stand-in Library Assert.
fn repro_outcomes(file: &str, codeunit: &str) -> Vec<(String, Outcome)> {
    let src = format!("{}\n{}", file, include_str!("../fixtures/outside-fb/LibraryAssert.al"));
    let o = al_testkit::outcomes_in(&src, codeunit);
    assert!(!o.is_empty());
    o
}

/// Issue 6139: on a worksheet in its primary key with a ListPart and the row triggers, a second marking (from the OnDrillDown
/// or from an action, both with CurrPage.Update(false)) after row 2 lost its flag: GoToRecord of row 2 answers false and the
/// page shows row 1, the row the marking left Rec on. Measured by the reporter on BC 29; pinned by corpus writer UD
/// (pages/lt-qr-onopenpage-moves-rec/SuRefillKey.al:722, :753, :784 on branch al-runtime-rs-staging-ud).
#[test]
fn issue_6139_second_marking_after_the_row_lost_its_flag() {
    all_pass(repro_outcomes(
        include_str!("../fixtures/outside-fb/Issue6139.al"),
        "Reapply Probe Tests",
    ));
}

/// Issue 6141: on a worksheet in the key ByName with a ListPart and the row triggers, GoToRecord of an unmarked row after a
/// marking OnDrillDown answers false and the page stays on row 1, the row the drilldown left Rec on, after a walk over the
/// marked rows (corpus pages/lt-qr-onopenpage-moves-rec/SuRefillKey.al:541 TW_NameKeyWithPart_GoToRecordOfUnmarkedRow on
/// corpus master). The control, GoToRecord of Rec's row, answers true.
#[test]
fn issue_6141_goto_record_of_an_unmarked_row_with_a_part() {
    all_pass(repro_outcomes(
        include_str!("../fixtures/outside-fb/Issue6141.al"),
        "GoTo Other Row Tests",
    ));
}

/// Issue 6116: the reporter's repro (fixtures/outside-fb/Issue6116.al, with a stand-in for Library Assert) and a second test
/// codeunit that checks the exact differences BC gave the reporter. Pins: session/lt-bl-sql-counters/SkSqlBudget.al, writer TW
/// (TW_*_Counts: 1/1, 1/11, 10/10, 11/5 for the reporter's shapes) and writer TX (TX_*: the cost of each record method alone).
fn issue_6116_outcomes(exact: &str) -> Vec<(String, Outcome)> {
    let src = format!(
        "{}\n{}\n{}",
        include_str!("../fixtures/outside-fb/Issue6116.al"),
        include_str!("../fixtures/outside-fb/LibraryAssert.al"),
        ISSUE_6116_EXACT
    );
    al_testkit::outcomes_in(&src, exact)
}

const ISSUE_6116_EXACT: &str = "
// Stands in for Base Application's setup subscriber (Global Triggers asked on the first write to a table, enabling nothing for it).
codeunit 50111 \"Repro Trigger Setup\"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Global Triggers\", 'GetDatabaseTableTriggerSetup', '', false, false)]
    local procedure Setup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean)
    begin
    end;
}

codeunit 50110 \"Repro Exact Counts\"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        StmtsBefore: BigInteger;
        RowsBefore: BigInteger;

    local procedure Mark()
    begin
        StmtsBefore := SessionInformation.SqlStatementsExecuted();
        RowsBefore := SessionInformation.SqlRowsRead();
    end;

    local procedure Check(Stmts: BigInteger; Rows: BigInteger)
    var
        StmtsDelta: BigInteger;
        RowsDelta: BigInteger;
    begin
        StmtsDelta := SessionInformation.SqlStatementsExecuted() - StmtsBefore;
        RowsDelta := SessionInformation.SqlRowsRead() - RowsBefore;
        if (StmtsDelta <> Stmts) or (RowsDelta <> Rows) then
            Error('%1/%2, expected %3/%4', StmtsDelta, RowsDelta, Stmts, Rows);
    end;

    local procedure InsertLines(GroupCode: Code[20]; LineCount: Integer)
    var
        RuleLine: Record \"Repro Rule Line\";
        i: Integer;
    begin
        for i := 1 to LineCount do begin
            RuleLine.Init();
            RuleLine.\"Group Code\" := GroupCode;
            RuleLine.\"Line No.\" := i;
            RuleLine.Value := 'Value';
            RuleLine.Weight := i;
            RuleLine.Insert();
        end;
    end;

    [Test]
    procedure GetOfOneRow()
    var
        RuleStore: Codeunit \"Repro Rule Store\";
    begin
        InsertLines('READ1', 10);
        Mark();
        if RuleStore.ReadWeight('READ1', 5) <> 5 then Error('weight');
        Check(1, 1);
    end;

    [Test]
    procedure FilteredFindSet()
    var
        RuleStore: Codeunit \"Repro Rule Store\";
    begin
        InsertLines('READ2', 10);
        Mark();
        if RuleStore.SumWeights('READ2') <> 55 then Error('sum');
        Check(1, 11);
    end;

    [Test]
    procedure GetAndModify()
    var
        RuleStore: Codeunit \"Repro Rule Store\";
    begin
        InsertLines('WRITE1', 10);
        Mark();
        RuleStore.RaiseWeights('WRITE1', 5);
        Check(10, 10);
    end;

    [Test]
    procedure FindFirstOrInsert()
    var
        RuleStore: Codeunit \"Repro Rule Store\";
    begin
        Mark();
        if RuleStore.BuildPath('TREE1', 5) = 0 then Error('node');
        Check(11, 5);
    end;

    // Not pinned (SkSqlBudget.al TX notes): a FindSet that finds rows right after the same query found none costs one statement more
    // the first time on BC 29 SaaS. Stays refused.
    [Test]
    procedure FindSetAfterTheSameQueryFoundNothing()
    var
        RuleStore: Codeunit \"Repro Rule Store\";
    begin
        InsertLines('READ3', 10);
        if RuleStore.SumWeights('NONE') <> 0 then Error('sum');
        Mark();
        if RuleStore.SumWeights('READ3') <> 55 then Error('sum');
        Check(1, 11);
    end;
}
";

fn issue_6116_outcome(name: &str) -> Outcome {
    let o = issue_6116_outcomes("Repro Exact Counts");
    o.into_iter()
        .find(|(n, _)| n == name)
        .unwrap_or_else(|| panic!("no test {name}"))
        .1
}

#[test]
fn issue_6116_reporter_repro_passes() {
    let o = issue_6116_outcomes("Repro Sql Budget Tests");
    assert_eq!(o.len(), 4);
    for (name, outcome) in o {
        assert_eq!(outcome, Outcome::Pass, "{name}");
    }
}

#[test]
fn issue_6116_get_with_load_fields_counts() {
    assert_eq!(issue_6116_outcome("GetOfOneRow"), Outcome::Pass);
}

#[test]
fn issue_6116_filtered_find_set_counts() {
    assert_eq!(issue_6116_outcome("FilteredFindSet"), Outcome::Pass);
}

#[test]
fn issue_6116_get_and_modify_loop_counts() {
    assert_eq!(issue_6116_outcome("GetAndModify"), Outcome::Pass);
}

#[test]
fn issue_6116_find_first_or_insert_counts() {
    assert_eq!(issue_6116_outcome("FindFirstOrInsert"), Outcome::Pass);
}

#[test]
fn issue_6116_find_set_after_the_same_query_found_nothing_stays_refused() {
    let o = issue_6116_outcome("FindSetAfterTheSameQueryFoundNothing");
    assert!(
        matches!(&o, Outcome::NotRun(why) if why.contains("SQL counter")),
        "{o:?}"
    );
}

/// Issue 6129: `X::Value` where the implicit Rec has a field `X` and an enum type `X` exists too. The objects of the
/// corpus pin tests/al-language/types/lt-enum-type-before-rec-field/EnumTypeBeforeRecField.al (corpus branch
/// al-runtime-rs-staging-ub, bump PR 793; not in the pinned corpus yet): `body` goes into the test procedure.
fn issue_6129_objects(body: &str) -> String {
    format!(
        "enum 90250 \"LT UB Field Enum\" {{ value(0; Zero) {{ }} value(1; Both) {{ }} value(10; OnlyA) {{ }} }}
enum 90251 \"LT UB X\" {{ value(0; Zero) {{ }} value(2; Both) {{ }} value(20; OnlyX) {{ }} }}
table 90252 \"LT UB Rec\" {{
    fields {{ field(1; Id; Integer) {{ }} field(2; \"LT UB X\"; Enum \"LT UB Field Enum\") {{ }} }}
    keys {{ key(PK; Id) {{ Clustered = true; }} }}
    procedure OnlyA(): Integer begin exit(\"LT UB X\"::OnlyA.AsInteger()); end;
    procedure OnlyAText(): Text begin exit(Format(\"LT UB X\"::OnlyA)); end;
    procedure Both(): Integer begin exit(\"LT UB X\"::Both.AsInteger()); end;
    trigger OnInsert() begin Id := \"LT UB X\"::Both.AsInteger(); end;
}}
page 90254 \"LT UB Page\" {{ SourceTable = \"LT UB Rec\"; layout {{ area(Content) {{ field(Id; Rec.Id) {{ }} }} }}
    procedure PgOnlyA(): Integer begin exit(\"LT UB X\"::OnlyA.AsInteger()); end;
    procedure PgBoth(): Integer begin exit(\"LT UB X\"::Both.AsInteger()); end;
}}
codeunit 90253 \"LT UB Table No\" {{ TableNo = \"LT UB Rec\";
    procedure OnlyX(): Integer begin exit(\"LT UB X\"::OnlyX.AsInteger()); end;
    procedure OnlyXText(): Text begin exit(Format(\"LT UB X\"::OnlyX)); end;
    procedure Both(): Integer begin exit(\"LT UB X\"::Both.AsInteger()); end;
    trigger OnRun() begin Rec.Id := \"LT UB X\"::Both.AsInteger(); end;
}}
codeunit 90256 \"LT UB Global Var\" {{ TableNo = \"LT UB Rec\";
    var \"LT UB X\": Enum \"LT UB Field Enum\";
    procedure OnlyA(): Integer begin exit(\"LT UB X\"::OnlyA.AsInteger()); end;
    procedure Both(): Integer begin exit(\"LT UB X\"::Both.AsInteger()); end;
    procedure LocalOnlyA(): Integer var \"LT UB X\": Enum \"LT UB Field Enum\"; begin exit(\"LT UB X\"::OnlyA.AsInteger()); end;
    procedure LocalBoth(): Integer var \"LT UB X\": Enum \"LT UB Field Enum\"; begin exit(\"LT UB X\"::Both.AsInteger()); end;
}}
codeunit 50503 T {{ Subtype = Test;
    local procedure Eq(Expected: Variant; Actual: Variant; What: Text) begin if Format(Expected) <> Format(Actual) then Error('%1: expected %2, got %3', What, Expected, Actual); end;
    [Test] procedure Run()
    var TableNoCu: Codeunit \"LT UB Table No\"; GlobalCu: Codeunit \"LT UB Global Var\"; R: Record \"LT UB Rec\"; Pg: Page \"LT UB Page\";
    begin {body} end; }}"
    )
}

/// Issue 6129, every context of the corpus pin (EnumTypeBeforeRecField.al:141-205): in a procedure of a TableNo
/// codeunit `X` is the enum type (:142-149); in its OnRun trigger (:152-158), a table procedure (:161-168) and
/// trigger (:171-177), a page procedure (:180-187), and with a global (:190-196) or local (:199-205) variable `X`,
/// `X` is the field or variable.
#[test]
fn issue_6129_enum_type_only_in_a_procedure_of_a_tableno_codeunit() {
    let body = "Eq(20, TableNoCu.OnlyX(), 'tableno only x'); Eq('OnlyX', TableNoCu.OnlyXText(), 'tableno text');
        Eq(2, TableNoCu.Both(), 'tableno both');
        Codeunit.Run(Codeunit::\"LT UB Table No\", R); Eq(1, R.Id, 'onrun');
        Eq(10, R.OnlyA(), 'table only a'); Eq('OnlyA', R.OnlyAText(), 'table text'); Eq(1, R.Both(), 'table both');
        R.Insert(true); Eq(1, R.Id, 'oninsert');
        Eq(10, Pg.PgOnlyA(), 'page only a'); Eq(1, Pg.PgBoth(), 'page both');
        Eq(10, GlobalCu.OnlyA(), 'global only a'); Eq(1, GlobalCu.Both(), 'global both');
        Eq(10, GlobalCu.LocalOnlyA(), 'local only a'); Eq(1, GlobalCu.LocalBoth(), 'local both');";
    all_pass(run_with_deps(&[], &issue_6129_objects(body)));
}

/// Compiles `src` as one workspace app: its diagnostics, and whether object `obj` marks the assumption.
fn issue_6129_compile(src: &str, obj: &str) -> (Vec<String>, bool) {
    let sources = vec![AppSource {
        id: "app".into(),
        name: "app".into(),
        dependencies: vec![],
        files: vec![("app.al".into(), src.into())],
    }];
    let (ws, l) = al_sema::lower::compile_apps(sources, vec![AppKind::Workspace], &Default::default());
    let diags = ws.diagnostics.iter().map(|d| format!("{d:?}")).collect();
    let marked = l
        .program
        .objects
        .iter()
        .find(|o| o.name == obj)
        .is_some_and(|o| format!("{o:?}").contains("EnumTypeBeforeRecField"));
    (diags, marked)
}

/// Issue 6129, the must-not-compile companions of the pin (corpus must-not-compile 2262 to 2267, branch
/// al-runtime-rs-staging-ub), run through the must-not-compile harness: a member only the other enum has is AL0132 in
/// each context.
#[test]
fn issue_6129_a_member_only_the_other_enum_has_does_not_compile() {
    let head = "enum 50000 \"MNC UB Field Enum\" { value(0; Zero) { } value(1; Both) { } value(10; OnlyA) { } }
enum 50001 \"MNC UB X\" { value(0; Zero) { } value(2; Both) { } value(20; OnlyX) { } }
table 50002 \"MNC UB Rec\" { fields { field(1; Id; Integer) { } field(2; \"MNC UB X\"; Enum \"MNC UB Field Enum\") { } }
    keys { key(PK; Id) { Clustered = true; } } TABLEPROC }
";
    let cases = [
        // 2262: TableNo codeunit procedure, a member only the field's enum has.
        (
            "OnlyA",
            "codeunit 50003 C { TableNo = \"MNC UB Rec\"; procedure P(): Integer begin exit(\"MNC UB X\"::OnlyA.AsInteger()); end; }",
        ),
        // 2263: the table's own procedure, a member only the enum type has.
        (
            "OnlyX",
            "TABLE procedure P(): Integer begin exit(\"MNC UB X\"::OnlyX.AsInteger()); end;",
        ),
        // 2264: a page on the table.
        (
            "OnlyX",
            "page 50004 C { SourceTable = \"MNC UB Rec\"; layout { area(Content) { field(Id; Rec.Id) { } } } procedure P(): Integer begin exit(\"MNC UB X\"::OnlyX.AsInteger()); end; }",
        ),
        // 2265: a global variable X.
        (
            "OnlyX",
            "codeunit 50003 C { TableNo = \"MNC UB Rec\"; var \"MNC UB X\": Enum \"MNC UB Field Enum\"; procedure P(): Integer begin exit(\"MNC UB X\"::OnlyX.AsInteger()); end; }",
        ),
        // 2266: a local variable X.
        (
            "OnlyX",
            "codeunit 50003 C { TableNo = \"MNC UB Rec\"; procedure P(): Integer var \"MNC UB X\": Enum \"MNC UB Field Enum\"; begin exit(\"MNC UB X\"::OnlyX.AsInteger()); end; }",
        ),
        // 2267: the OnRun trigger of a TableNo codeunit.
        (
            "OnlyX",
            "codeunit 50003 C { TableNo = \"MNC UB Rec\"; trigger OnRun() begin Rec.Id := \"MNC UB X\"::OnlyX.AsInteger(); end; }",
        ),
    ];
    let mut wrong = Vec::new();
    for (member, case) in cases {
        let body = match case.strip_prefix("TABLE ") {
            Some(p) => head.replace("TABLEPROC", p),
            None => format!("{}{case}", head.replace("TABLEPROC", "")),
        };
        let text = format!("// @expect AL0132 | '\"MNC UB X\"' does not contain a definition for '{member}'\n{body}");
        let parsed = al_test_cli::mnc::parse_case("issue-6129.al", &text).unwrap();
        let out = al_test_cli::mnc::run_case(&parsed, 29);
        if out.verdict != al_test_cli::mnc::Verdict::Match {
            wrong.push(format!("{case}: {:?} {:?}", out.verdict, out.got));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// Issue 6129: corpus master now pins the reading in a procedure of a TableNo codeunit
/// (types/lt-enum-type-before-rec-field/EnumTypeBeforeRecField.al:142-149), so the assumption is retired and no
/// reading marks it any more; the field readings stay unflagged as before.
#[test]
fn issue_6129_no_reading_is_flagged_once_pinned() {
    let (diags, marked) = issue_6129_compile(&issue_6129_objects(""), "LT UB Table No");
    assert!(diags.is_empty(), "{diags:?}");
    assert!(!marked);
    for obj in ["LT UB Rec", "LT UB Page", "LT UB Global Var"] {
        assert!(!issue_6129_compile(&issue_6129_objects(""), obj).1, "{obj}");
    }
}

/// Issue 6168: `FilterPageBuilder.RunModal` reaches the bound `[FilterPageHandler]`, which gets a RecordRef on the
/// builder's table; the filters it sets come back through `GetView`, and its answer is the result of `RunModal`.
/// Pins: filterpage/W3FilterPageBuilder.al:159-212 (FieldRef.SetFilter in the handler, answer true and false),
/// platform/lt-platform-members/PlTFilterPageBuilder.al:293-311 (Record.SetView of the builder's view).
#[test]
fn issue_6168_filter_page_handler_answers_run_modal() {
    let objects = "
table 69620 \"UR Fp Row\" { fields { field(1; \"No.\"; Code[20]) { } field(2; Name; Text[30]) { } } keys { key(PK; \"No.\") { Clustered = true; } } }
codeunit 69621 \"UR Fp App\" {
    procedure SetFilterViaFilterPage(var MyRec: Record \"UR Fp Row\"): Boolean
    var
        FilterPageBuilder: FilterPageBuilder;
    begin
        FilterPageBuilder.AddRecord(MyRec.TableCaption(), MyRec);
        FilterPageBuilder.AddField(MyRec.TableCaption(), MyRec.\"No.\");
        if not FilterPageBuilder.RunModal() then
            exit(false);
        MyRec.SetView(FilterPageBuilder.GetView(MyRec.TableCaption()));
        exit(true);
    end;
}
codeunit 69622 T {
    Subtype = Test;
    var Answer: Boolean;
    [Test]
    [HandlerFunctions('MyFilterPageHandler')]
    procedure FilterIsApplied()
    var
        Row: Record \"UR Fp Row\";
        App: Codeunit \"UR Fp App\";
    begin
        Answer := true;
        if not App.SetFilterViaFilterPage(Row) then
            Error('RunModal answered false');
        if Row.GetFilter(\"No.\") <> 'A*' then
            Error('filter is %1', Row.GetFilter(\"No.\"));
    end;
    [Test]
    [HandlerFunctions('MyFilterPageHandler')]
    procedure CancelLeavesTheRecord()
    var
        Row: Record \"UR Fp Row\";
        App: Codeunit \"UR Fp App\";
    begin
        Answer := false;
        if App.SetFilterViaFilterPage(Row) then
            Error('RunModal answered true');
        if Row.GetFilter(\"No.\") <> '' then
            Error('filter is %1', Row.GetFilter(\"No.\"));
    end;
    [FilterPageHandler]
    procedure MyFilterPageHandler(var RecRef: RecordRef): Boolean
    begin
        RecRef.Field(1).SetFilter('A*');
        exit(Answer);
    end;
}";
    all_pass(run_with_deps(&[], objects));
}

/// al-runner issue 6179: a generic field-by-field copy compares `FieldRef.Length` of every field, a Media and a MediaSet field
/// included. Pin: record/lt-media-length-find-empty/MediaLengthFindEmpty.al:56-75 (Media 16, MediaSet 16).
#[test]
fn issue_6179_fieldref_length_of_media_and_mediaset() {
    let main = "
table 69540 \"UR Media Row\" { fields { field(1; Id; Integer) { } field(2; Pic; Media) { } field(3; Pics; MediaSet) { } }
  keys { key(PK; Id) { Clustered = true; } } }
codeunit 69541 T { Subtype = Test;
  [Test] procedure P() var RR: RecordRef; i: Integer; Total: Integer; begin
    RR.Open(Database::\"UR Media Row\");
    for i := 1 to RR.FieldCount() do Total += RR.FieldIndex(i).Length();
    if RR.Field(2).Length() <> 16 then Error('Media length');
    if RR.Field(3).Length() <> 16 then Error('MediaSet length');
  end; }";
    all_pass(run_with_deps(&[], main));
}

/// al-runner issue 6176: robust JSON parsing reads a property of the wrong type with `AsBoolean` inside a TryFunction; the
/// conversion error is caught and the TryFunction returns false. Pin: json/JsonValueGetters.al:172 (`text_asboolean`, a text of
/// letters raises "Unable to convert ... System.Boolean."); the reporter's text with blanks is the assumption
/// `json-other-text-as-boolean-conversion-error`.
#[test]
fn issue_6176_json_text_as_boolean_in_a_try_function() {
    let main = "
codeunit 69542 T { Subtype = Test;
  [TryFunction] local procedure TryBool(V: JsonValue; var B: Boolean) begin B := V.AsBoolean(); end;
  [Test] procedure P() var O: JsonObject; T: JsonToken; B: Boolean; begin
    O.ReadFrom('{\"successfully\":\"not a boolean\",\"other\":\"hello\",\"yes\":\"true\"}');
    O.Get('successfully', T);
    if TryBool(T.AsValue(), B) then Error('converted');
    if GetLastErrorText() <> 'Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavJsonValue to System.Boolean.' then Error(GetLastErrorText());
    O.Get('other', T);
    if TryBool(T.AsValue(), B) then Error('converted letters');
    O.Get('yes', T);
    if not TryBool(T.AsValue(), B) then Error('true text');
    if not B then Error('not true');
  end; }";
    all_pass(run_with_deps(&[], main));
}

/// al-runner issue 6178: Microsoft's retention policy code calls `Dialog.HideSubsequentDialogs(true)` before it opens a progress
/// dialog (only under GuiAllowed); the call changes nothing a test sees. Pin: errorinfo/lt-errnotif/EnTDialogSession.al
/// UJ_Dialog_HideSubsequent_OpenUpdateClose (corpus branch al-runtime-rs-staging-uj 1dc037a8; assumption
/// `dialog-hide-subsequent-dialogs-no-headless-effect` until the bump).
#[test]
fn issue_6178_hide_subsequent_dialogs_then_open_update_close() {
    let main = "
codeunit 69543 T { Subtype = Test;
  [Test] procedure P() var D: Dialog; begin
    D.HideSubsequentDialogs(true);
    D.Open('Applying #1######');
    D.Update(1, 'x');
    D.Close();
    if not D.HideSubsequentDialogs(true) then Error('not true');
  end; }";
    all_pass(run_with_deps(&[], main));
}

/// al-runner issue 6181: a part's `SubPageLink` with `const("Enum"::Value)` compiles and filters the part on that value.
/// Pin: pages/lt-uj-outside-leads/UjSubPageLink.al UJ_SubPageLink_ConstEnum_Rows (corpus branch al-runtime-rs-staging-uj
/// 1dc037a8; former assumption `subpagelink-const-qualified-enum-value` (pinned on corpus master since bump 1030) until the bump).
#[test]
fn issue_6181_subpagelink_const_with_a_qualified_enum_value() {
    let main = "
enum 69544 \"UR Src\" { Extensible = true; value(0; Resource) { } value(1; Header) { } }
table 69545 \"UR Setup\" { fields { field(1; Code; Code[10]) { } } keys { key(PK; Code) { Clustered = true; } } }
table 69546 \"UR Line\" { fields { field(1; \"Setup Code\"; Code[10]) { } field(2; \"UR Src\"; Enum \"UR Src\") { } field(3; N; Integer) { } }
  keys { key(PK; \"Setup Code\", \"UR Src\", N) { Clustered = true; } } }
page 69547 \"UR Part\" { PageType = ListPart; SourceTable = \"UR Line\";
  layout { area(Content) { repeater(R) { field(NCtl; Rec.N) { ApplicationArea = All; } } } } }
page 69548 \"UR Card\" { PageType = Card; SourceTable = \"UR Setup\";
  layout { area(Content) { field(CodeCtl; Rec.Code) { ApplicationArea = All; }
    part(Lines; \"UR Part\") { ApplicationArea = All; SubPageLink = \"Setup Code\" = field(Code), \"UR Src\" = const(\"UR Src\"::Header); } } } }
codeunit 69549 T { Subtype = Test;
  [Test] procedure P() var S: Record \"UR Setup\"; L: Record \"UR Line\"; Card: TestPage \"UR Card\"; begin
    S.Code := 'S1'; S.Insert();
    L.\"Setup Code\" := 'S1'; L.\"UR Src\" := L.\"UR Src\"::Resource; L.N := 1; L.Insert();
    L.\"UR Src\" := L.\"UR Src\"::Header; L.N := 2; L.Insert();
    Card.OpenView();
    if not Card.Lines.First() then Error('no row');
    if Card.Lines.NCtl.Value() <> '2' then Error('row %1', Card.Lines.NCtl.Value());
    if Card.Lines.Next() then Error('second row');
    Card.Close();
  end; }";
    all_pass(run_with_deps(&[], main));
}

/// al-runner issue 6182: a page extension declares OnPageBackgroundTaskCompleted (Microsoft's User Card Perf. Factboxes); the
/// TestPage opens and the page's trigger runs before the extension's. Pin: pages/lt-uj-outside-leads/UjPageExtBgTask.al
/// UJ_PageExt_BgTask_Completed (corpus branch al-runtime-rs-staging-uj 1dc037a8; former assumption `pageext-background-task-triggers` (pinned on corpus master since bump 1030)).
#[test]
fn issue_6182_page_extension_background_task_completed() {
    let main = "
table 69550 \"UR Bg Row\" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }
codeunit 69551 \"UR Bg Log\" { SingleInstance = true; var L: Text;
  procedure Add(T: Text) begin L += T + ';'; end; procedure Take(): Text begin exit(L); end; }
codeunit 69552 \"UR Bg Worker\" { trigger OnRun() var R: Dictionary of [Text, Text]; begin R.Add('who', 'w'); Page.SetBackgroundTaskResult(R); end; }
page 69553 \"UR Bg Card\" { PageType = Card; SourceTable = \"UR Bg Row\";
  layout { area(Content) { field(IdCtl; Rec.Id) { ApplicationArea = All; } } }
  trigger OnPageBackgroundTaskCompleted(TaskId: Integer; Results: Dictionary of [Text, Text]) var Log: Codeunit \"UR Bg Log\"; begin Log.Add('base'); end; }
pageextension 69554 \"UR Bg Ext\" extends \"UR Bg Card\" {
  trigger OnAfterGetCurrRecord() var Args: Dictionary of [Text, Text]; TaskId: Integer; begin
    CurrPage.EnqueueBackgroundTask(TaskId, Codeunit::\"UR Bg Worker\", Args, 1000); end;
  trigger OnPageBackgroundTaskCompleted(TaskId: Integer; Results: Dictionary of [Text, Text]) var Log: Codeunit \"UR Bg Log\"; begin Log.Add('ext'); end; }
codeunit 69555 T { Subtype = Test;
  [Test] procedure P() var Row: Record \"UR Bg Row\"; Log: Codeunit \"UR Bg Log\"; Card: TestPage \"UR Bg Card\"; begin
    Row.Id := 1; Row.Insert();
    Card.OpenView();
    Card.Close();
    if Log.Take() <> 'base;ext;' then Error(Log.Take());
  end; }";
    all_pass(run_with_deps(&[], main));
}

/// al-runner issue 6180: a history list over a temporary table without the new-row line, run modally; its action rebuilds the rows,
/// then the handler filters, goes to the first row and reads it. Pin: pages/lt-uj-outside-leads/UjTempList.al
/// UJ_Temp_List_Modal_FilterAfterRebuild (corpus branch al-runtime-rs-staging-uj 1dc037a8; former assumption `list-action-rebuilds-rows` (pinned on corpus master since bump 1030)).
#[test]
fn issue_6180_history_list_action_rebuilds_temporary_rows() {
    let main = "
table 69556 \"UR Hist\" { fields { field(1; \"Entry No.\"; Integer) { } field(2; \"Field No.\"; Integer) { } field(3; \"Old Value\"; Text[30]) { } }
  keys { key(PK; \"Entry No.\") { Clustered = true; } } }
page 69557 \"UR Hist List\" { PageType = List; SourceTable = \"UR Hist\"; SourceTableTemporary = true;
  InsertAllowed = false; ModifyAllowed = false; DeleteAllowed = false;
  layout { area(Content) { repeater(Rows) { field(FieldNoCtl; Rec.\"Field No.\") { ApplicationArea = All; }
    field(OldValueCtl; Rec.\"Old Value\") { ApplicationArea = All; } } } }
  actions { area(Processing) { action(ThisMonth) { ApplicationArea = All; trigger OnAction() begin
    Rec.Reset(); Rec.DeleteAll(); Add(21, 7, 'a'); Add(22, 8, 'b'); Add(23, 7, 'c'); Rec.FindFirst(); CurrPage.Update(false); end; } } }
  trigger OnOpenPage() begin Add(1, 7, 'old'); Rec.FindFirst(); end;
  trigger OnAfterGetRecord() begin end;
  trigger OnAfterGetCurrRecord() begin end;
  local procedure Add(No: Integer; F: Integer; V: Text[30]) begin
    Rec.Init(); Rec.\"Entry No.\" := No; Rec.\"Field No.\" := F; Rec.\"Old Value\" := V; Rec.Insert(); end; }
codeunit 69558 T { Subtype = Test; var Seen: Text;
  [ModalPageHandler] procedure Handler(var H: TestPage \"UR Hist List\") begin
    H.ThisMonth.Invoke();
    H.Filter.SetFilter(\"Field No.\", '8');
    H.First();
    Seen := H.OldValueCtl.Value();
    H.OK().Invoke();
  end;
  [Test] [HandlerFunctions('Handler')] procedure P() var H: Page \"UR Hist List\"; begin
    H.RunModal();
    if Seen <> 'b' then Error('seen %1', Seen);
  end; }";
    all_pass(run_with_deps(&[], main));
}

/// Issue 6173: reading a query `filter()` element (not a column) before a row is read raises the error a column
/// read without a row raises, before Open and after Open without a Read. After a Read BC returns the value of
/// another column or crashes, so that form stays refused. Pins: corpus branch al-runtime-rs-staging-uh (commit
/// ab6c6eb2, corpus PR 799) errors/lt-ms-refusals-131/P131QueryOuterFilter.al UH_Query_ReadFilter_BeforeOpen,
/// UH_Query_ReadFilter_OpenNoRead (assumption query-filter-element-read-without-row until that is pinned).
const ISSUE_6173_OBJECTS: &str = "
table 69630 \"UR Qf Addr\" { fields { field(1; \"No.\"; Code[20]) { } field(2; Country; Code[10]) { } field(3; \"Post Box\"; Text[30]) { } } keys { key(PK; \"No.\") { Clustered = true; } } }
query 69631 \"UR Qf Query\" {
    elements {
        dataitem(Addr; \"UR Qf Addr\") {
            column(No; \"No.\") { }
            column(CountryCode; Country) { }
            filter(PostBox; \"Post Box\") { }
        }
    }
}
codeunit 69632 T {
    Subtype = Test;
    local procedure Seed()
    var
        Addr: Record \"UR Qf Addr\";
    begin
        Addr.DeleteAll();
        Addr.\"No.\" := 'A1';
        Addr.Country := 'DE';
        Addr.\"Post Box\" := 'PB 7';
        Addr.Insert();
    end;
    local procedure GetFieldValue(MyQuery: Query \"UR Qf Query\"; FieldNo: Integer): Text
    begin
        case FieldNo of
            1:
                exit(MyQuery.No);
            2:
                exit(MyQuery.CountryCode);
            3:
                exit(Format(MyQuery.PostBox));
        end;
    end;
    [Test]
    procedure BeforeOpen()
    var
        Q: Query \"UR Qf Query\";
    begin
        Seed();
        asserterror GetFieldValue(Q, 3);
        if GetLastErrorText() <> 'You have not yet retrieved a data row for this query. The value for column ''PostBox'' can only be accessed after a row is retrieved with a call to the READ function.' then
            Error('got %1', GetLastErrorText());
    end;
    [Test]
    procedure OpenNoRead()
    var
        Q: Query \"UR Qf Query\";
        Txt: Text;
    begin
        Seed();
        Q.SetRange(PostBox, 'PB 7');
        Q.Open();
        asserterror Txt := Format(Q.PostBox);
        if GetLastErrorText() <> 'You have not yet retrieved a data row for this query. The value for column ''PostBox'' can only be accessed after a row is retrieved with a call to the READ function.' then
            Error('got %1', GetLastErrorText());
    end;
    [Test]
    procedure AfterRead()
    var
        Q: Query \"UR Qf Query\";
        Txt: Text;
    begin
        Seed();
        Q.Open();
        Q.Read();
        Txt := GetFieldValue(Q, 1) + GetFieldValue(Q, 3);
    end;
}";

#[test]
fn issue_6173_filter_element_read_without_a_row_raises() {
    let results = run_with_deps(&[], ISSUE_6173_OBJECTS);
    let get = |n: &str| {
        results
            .iter()
            .find(|(name, _)| name == n)
            .map(|(_, o)| o.clone())
            .unwrap()
    };
    assert_eq!(get("BeforeOpen"), Outcome::Pass);
    assert_eq!(get("OpenNoRead"), Outcome::Pass);
    match get("AfterRead") {
        Outcome::NotRun(why) => assert!(why.contains("filter element"), "{why}"),
        other => panic!("AfterRead: {other:?}"),
    }
}

/// Issue 6174: a query column over a Sum or Count FlowField of a LeftOuterJoin dataitem is calculated per joined
/// row, and is 0 for a parent row without a matching child. Pins:
/// errors/lt-ms-refusals-130/P130QueryOuterFlow.al:163-184 (P130_Query_FlowFieldOfOuterJoinedItem) and :210-229.
#[test]
fn issue_6174_flowfield_column_of_outer_joined_item() {
    let objects = "
table 69640 \"UR Of Header\" { fields { field(1; \"No.\"; Code[20]) { } } keys { key(PK; \"No.\") { Clustered = true; } } }
table 69641 \"UR Of Line\" {
    fields {
        field(1; \"Document No.\"; Code[20]) { }
        field(2; \"Line No.\"; Integer) { }
        field(3; \"Reserved Qty. (Base)\"; Decimal) { FieldClass = FlowField; CalcFormula = sum(\"UR Of Entry\".Qty where(\"Line No.\" = field(\"Line No.\"))); }
        field(4; Entries; Integer) { FieldClass = FlowField; CalcFormula = count(\"UR Of Entry\" where(\"Line No.\" = field(\"Line No.\"))); }
    }
    keys { key(PK; \"Document No.\", \"Line No.\") { Clustered = true; } }
}
table 69642 \"UR Of Entry\" { fields { field(1; Id; Integer) { } field(2; \"Line No.\"; Integer) { } field(3; Qty; Decimal) { } } keys { key(PK; Id) { Clustered = true; } } }
query 69643 \"UR Of Query\" {
    OrderBy = ascending(No);
    elements {
        dataitem(Header; \"UR Of Header\") {
            column(No; \"No.\") { }
            dataitem(Line; \"UR Of Line\") {
                DataItemLink = \"Document No.\" = Header.\"No.\";
                SqlJoinType = LeftOuterJoin;
                column(Reserved_Qty_Base; \"Reserved Qty. (Base)\") { }
                column(Entries; Entries) { }
            }
        }
    }
}
codeunit 69644 T {
    Subtype = Test;
    [Test]
    procedure Availability()
    var
        Header: Record \"UR Of Header\";
        Line: Record \"UR Of Line\";
        Entry: Record \"UR Of Entry\";
        Q: Query \"UR Of Query\";
        Res: Text;
    begin
        Header.\"No.\" := 'A';
        Header.Insert();
        Header.\"No.\" := 'B';
        Header.Insert();
        Line.\"Document No.\" := 'A';
        Line.\"Line No.\" := 10;
        Line.Insert();
        Entry.Id := 1;
        Entry.\"Line No.\" := 10;
        Entry.Qty := 2.5;
        Entry.Insert();
        Entry.Id := 2;
        Entry.\"Line No.\" := 10;
        Entry.Qty := 4;
        Entry.Insert();
        Q.Open();
        while Q.Read() do
            Res += '[' + Q.No + '|' + Format(Q.Reserved_Qty_Base) + '|' + Format(Q.Entries) + ']';
        if Res <> '[A|6.5|2][B|0|0]' then
            Error('got %1', Res);
    end;
}";
    all_pass(run_with_deps(&[], objects));
}

/// Issue 6177: `RecordRef.Field` reaches the user-name system FlowFields (SystemCreatedByUserName 2000000005,
/// SystemCreatedByFullName 2000000006, SystemModifiedByUserName 2000000007, SystemModifiedByFullName 2000000008):
/// class FlowField, and after `CalcField` the user name fields hold `UserId()` and the full name fields the User row's
/// Full Name (not blank, not the user name) on a normal record; all four are blank on a temporary record. Pin:
/// system-fields/W6SystemFieldStamps.al:216-248 (UserNameSystemFields_NormalAndTemporaryRecord).
const ISSUE_6177_OBJECTS: &str = "
table 69650 \"UR Un Row\" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }
codeunit 69651 T {
    Subtype = Test;

    local procedure Collect(var RecRef: RecordRef): Text
    var
        FldRef: FieldRef;
        No: Integer;
        Res: Text;
    begin
        for No := 2000000005 to 2000000008 do begin
            FldRef := RecRef.Field(No);
            if FldRef.Class() = FieldClass::FlowField then
                FldRef.CalcField();
            Res += '[' + Format(FldRef.Number()) + '|' + Format(FldRef.Class()) + '|' + Format(FldRef.Type()) + '|' + Format(FldRef.Length())
                + '|user=' + Format(Format(FldRef.Value()) = UserId()) + '|blank=' + Format(Format(FldRef.Value()) = '') + ']';
        end;
        exit(Res);
    end;

    [Test]
    procedure NormalRecord()
    var
        Row: Record \"UR Un Row\";
        RecRef: RecordRef;
        Res: Text;
    begin
        Row.Id := 1;
        Row.Insert();
        Row.Get(1);
        RecRef.GetTable(Row);
        Res := Collect(RecRef);
        if Res <> '[2000000005|FlowField|Text|50|user=Yes|blank=No][2000000006|FlowField|Text|80|user=No|blank=No][2000000007|FlowField|Text|50|user=Yes|blank=No][2000000008|FlowField|Text|80|user=No|blank=No]' then
            Error('got %1', Res);
    end;

    [Test]
    procedure TemporaryRecord()
    var
        Row: Record \"UR Un Row\" temporary;
        RecRef: RecordRef;
        Res: Text;
    begin
        Row.Id := 1;
        Row.Insert();
        Row.Get(1);
        RecRef.GetTable(Row);
        Res := Collect(RecRef);
        if Res <> '[2000000005|FlowField|Text|50|user=No|blank=Yes][2000000006|FlowField|Text|80|user=No|blank=Yes][2000000007|FlowField|Text|50|user=No|blank=Yes][2000000008|FlowField|Text|80|user=No|blank=Yes]' then
            Error('got %1', Res);
    end;
}";

#[test]
fn issue_6177_user_name_system_flow_fields_through_record_ref() {
    all_pass(run_with_deps(&[], ISSUE_6177_OBJECTS));
}

/// Issue 6220: a bound `[HyperlinkHandler]` receives the URL `Hyperlink` opens; without one the call raises
/// `Unhandled UI: Hyperlink <url>`. Pins: handlers/W3Hyperlink.al:24-38 (Hyperlink_WithHandler), :40-63 (empty and mailto
/// URLs), :65-83 (Hyperlink_WithoutHandler).
#[test]
fn issue_6220_hyperlink_handler_gets_the_url() {
    let objects = "
codeunit 69660 \"UR HL Opener\" {
    procedure OpenHelp(Topic: Text)
    begin
        Hyperlink('https://example.invalid/help/' + Topic);
    end;
}
codeunit 69661 T {
    Subtype = Test;
    var
        LastUrl: Text;
        Calls: Integer;

    [Test]
    [HandlerFunctions('HyperlinkHandler')]
    procedure OpenHelp_CallsHyperlinkWithTopicUrl()
    var
        Opener: Codeunit \"UR HL Opener\";
    begin
        Opener.OpenHelp('setup');
        if LastUrl <> 'https://example.invalid/help/setup' then
            Error('got %1', LastUrl);
        Hyperlink('');
        if (LastUrl <> '') or (Calls <> 2) then
            Error('empty: got %1 after %2 calls', LastUrl, Calls);
    end;

    [Test]
    procedure WithoutHandler()
    begin
        asserterror Hyperlink('https://example.invalid/x');
        if GetLastErrorText() <> 'Unhandled UI: Hyperlink https://example.invalid/x' then
            Error('text %1', GetLastErrorText());
        if GetLastErrorCode() <> 'MissingUIHandler' then
            Error('code %1', GetLastErrorCode());
    end;

    [HyperlinkHandler]
    procedure HyperlinkHandler(Message: Text[1024])
    begin
        LastUrl := Message;
        Calls += 1;
    end;
}";
    all_pass(run_with_deps(&[], objects));
}

/// Issue 6216: a page extension's `OnBeforeAction` in `modify(<action>)` runs before the action's `OnAction`, its
/// `OnAfterAction` after; an error in `OnBeforeAction` runs neither and reaches `Invoke` as `TestWrapped:Dialog`; opening
/// the page runs none. Pins (corpus branch al-runtime-rs-staging-uq at d651ec93, bump PR 809):
/// pages/lt-pageext-action-after/PageExtensionActionOnAfterAction.al UQ_OpenWithoutInvoke_NoBeforeOrAfter,
/// UQ_BeforeAndAfterInOneModify_Order, UQ_BeforeActionRaises_OnActionAndAfterSkipped.
#[test]
fn issue_6216_pageext_on_before_action_runs_before_on_action() {
    let objects = "
table 69662 \"UR BA Entry\" { fields { field(1; \"No.\"; Code[20]) { } } keys { key(PK; \"No.\") { Clustered = true; } } }
codeunit 69663 \"UR BA Trace\" {
    SingleInstance = true;
    var
        Trace: Text;
    procedure Add(Step: Text)
    begin
        Trace += Step + ';';
    end;
    procedure Get(): Text
    begin
        exit(Trace);
    end;
    procedure Reset()
    begin
        Trace := '';
    end;
}
page 69664 \"UR BA List\" {
    PageType = List;
    SourceTable = \"UR BA Entry\";
    layout { area(Content) { repeater(Lines) { field(\"No.\"; Rec.\"No.\") { } } } }
    actions {
        area(Processing) {
            action(DoIt) {
                trigger OnAction()
                var
                    BATrace: Codeunit \"UR BA Trace\";
                begin
                    BATrace.Add('action');
                end;
            }
            action(Fails) {
                trigger OnAction()
                var
                    BATrace: Codeunit \"UR BA Trace\";
                begin
                    BATrace.Add('fails action');
                end;
            }
        }
    }
}
pageextension 69665 \"UR BA List Ext\" extends \"UR BA List\" {
    actions {
        modify(DoIt) {
            trigger OnBeforeAction()
            var
                BATrace: Codeunit \"UR BA Trace\";
            begin
                BATrace.Add('before');
            end;
            trigger OnAfterAction()
            var
                BATrace: Codeunit \"UR BA Trace\";
            begin
                BATrace.Add('after');
            end;
        }
        modify(Fails) {
            trigger OnBeforeAction()
            var
                BATrace: Codeunit \"UR BA Trace\";
            begin
                BATrace.Add('fails before');
                Error('UR before failure');
            end;
            trigger OnAfterAction()
            var
                BATrace: Codeunit \"UR BA Trace\";
            begin
                BATrace.Add('fails after');
            end;
        }
    }
}
codeunit 69666 T {
    Subtype = Test;

    [Test]
    procedure ModifiedActionRunsOnBeforeAction()
    var
        BATrace: Codeunit \"UR BA Trace\";
        BAList: TestPage \"UR BA List\";
    begin
        BATrace.Reset();
        BAList.OpenView();
        if BATrace.Get() <> '' then
            Error('open %1', BATrace.Get());
        BAList.DoIt.Invoke();
        BAList.Close();
        if BATrace.Get() <> 'before;action;after;' then
            Error('trace %1', BATrace.Get());
    end;

    [Test]
    procedure ErrorInOnBeforeActionSkipsTheRest()
    var
        BATrace: Codeunit \"UR BA Trace\";
        BAList: TestPage \"UR BA List\";
    begin
        BATrace.Reset();
        BAList.OpenView();
        asserterror BAList.Fails.Invoke();
        if GetLastErrorText() <> 'UR before failure' then
            Error('text %1', GetLastErrorText());
        if GetLastErrorCode() <> 'TestWrapped:Dialog' then
            Error('code %1', GetLastErrorCode());
        BAList.Close();
        if BATrace.Get() <> 'fails before;' then
            Error('trace %1', BATrace.Get());
    end;
}";
    all_pass(run_with_deps(&[], objects));
}

/// Issue 6221: a SaveValues page that a page extension extends with a control bound to the extension's own variable opens;
/// nothing is stored before the first close, and the next open restores the extension's variable as it does the page's.
/// Pin: misc/lt-probes-needed-6/PnF02SaveValuesExt.al:157-205 (SaveValues_Ext_OkThenReopen: first open all blank, after OK
/// `second_A` and `second_B`, `second_E` restored) and :207-234 (a Cancel stores nothing).
#[test]
fn issue_6221_savevalues_page_with_extension_variable() {
    let objects = "
table 69670 \"UR SV Row\" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }
page 69671 \"UR SV Card\" {
    PageType = Card;
    SourceTable = \"UR SV Row\";
    SaveValues = true;
    layout { area(Content) { field(A; Var1) { } } }
    var
        Var1: Text[30];
}
pageextension 69672 \"UR SV Ext\" extends \"UR SV Card\" {
    layout { addlast(Content) { field(B; ExtVar) { } } }
    var
        ExtVar: Boolean;
}
codeunit 69673 T {
    Subtype = Test;
    var
        Mode: Text;
        SeenA: Text;
        SeenB: Text;

    [ModalPageHandler]
    procedure CardHandler(var P: TestPage \"UR SV Card\")
    begin
        case Mode of
            'set':
                begin
                    P.A.SetValue('a1');
                    P.B.SetValue(true);
                    P.OK().Invoke();
                end;
            'read':
                begin
                    SeenA := P.A.Value();
                    SeenB := P.B.Value();
                    P.OK().Invoke();
                end;
        end;
    end;

    local procedure Open(NewMode: Text)
    var
        Row: Record \"UR SV Row\";
    begin
        Mode := NewMode;
        if not Row.Get(1) then begin
            Row.Id := 1;
            Row.Insert();
        end;
        Page.RunModal(Page::\"UR SV Card\", Row);
    end;

    [Test]
    [HandlerFunctions('CardHandler')]
    procedure FirstOpenThenReopen()
    begin
        Open('read');
        if (SeenA <> '') or (SeenB <> 'No') then
            Error('first %1 %2', SeenA, SeenB);
        Open('set');
        Open('read');
        if (SeenA <> 'a1') or (SeenB <> 'Yes') then
            Error('second %1 %2', SeenA, SeenB);
    end;
}";
    all_pass(run_with_deps(&[], objects));
}

/// al-runner issue 6211: inside a table procedure, `this.<Field>` is the field of the current record, the same as the
/// unqualified name: it can be read, assigned and have a method called on it. The reporter's alc compiles every form
/// below without a diagnostic. At run time each form does what the bare name does, so no assumption is registered; a
/// corpus writer pins the shapes on real BC.
const ISSUE_6211_OBJECTS: &str = "
table 50100 \"TF Entry\"
{
    fields
    {
        field(1; \"No.\"; Code[20]) { }
        field(2; ETag; Text[50]) { }
        field(3; \"Set Codes\"; Text[250]) { }
    }
    keys { key(PK; \"No.\") { Clustered = true; } }

    var
        Counter: Integer;

    procedure SetETag(NewETag: Text[50])
    begin
        if this.ETag = NewETag then
            exit;
        this.ETag := NewETag;
    end;

    procedure GetSetCodeList(): List of [Text]
    begin
        exit(this.\"Set Codes\".Split('|'));
    end;

    procedure Bump(): Integer
    begin
        this.Counter += 1;
        exit(this.Counter);
    end;

    procedure BumpTwice(): Integer
    begin
        this.Bump();
        exit(this.Bump());
    end;

    procedure ValidateThroughThis(NewETag: Text[50])
    begin
        this.Validate(ETag, NewETag);
        this.TestField(ETag);
    end;
}

tableextension 50102 \"TF Entry Ext\" extends \"TF Entry\"
{
    fields
    {
        field(50100; Extra; Text[30]) { }
    }

    procedure SetExtraFromETag()
    begin
        this.Extra := this.ETag;
        this.SetETag(this.Extra + '!');
    end;
}

codeunit 50101 T
{
    Subtype = Test;

    [Test]
    procedure AssignFieldThroughThis()
    var
        Entry: Record \"TF Entry\";
    begin
        Entry.SetETag('abc');
        if Entry.ETag <> 'abc' then
            Error('ETag is %1', Entry.ETag);
    end;

    [Test]
    procedure CallMethodOnFieldThroughThis()
    var
        Entry: Record \"TF Entry\";
    begin
        Entry.\"Set Codes\" := 'A|B';
        if Entry.GetSetCodeList().Count() <> 2 then
            Error('expected 2 codes');
    end;

    [Test]
    procedure GlobalAndProcedureThroughThis()
    var
        Entry: Record \"TF Entry\";
    begin
        if Entry.BumpTwice() <> 2 then
            Error('expected 2');
    end;

    [Test]
    procedure ValidateAndTestFieldThroughThis()
    var
        Entry: Record \"TF Entry\";
    begin
        Entry.ValidateThroughThis('v');
        if Entry.ETag <> 'v' then
            Error('ETag is %1', Entry.ETag);
    end;

    [Test]
    procedure TableExtensionThroughThis()
    var
        Entry: Record \"TF Entry\";
    begin
        Entry.ETag := 'x';
        Entry.SetExtraFromETag();
        if (Entry.Extra <> 'x') or (Entry.ETag <> 'x!') then
            Error('Extra %1 ETag %2', Entry.Extra, Entry.ETag);
    end;
}";

#[test]
fn issue_6211_table_field_through_this_reads_assigns_and_calls_as_the_unqualified_name() {
    all_pass(run_with_deps(&[], ISSUE_6211_OBJECTS));
}

/// A bare `this` in a table, passed on by value, hands over the current record (misc/lt-probes-needed-6/
/// PnF18ThisInTable.al:64-67). Passed to a `var` parameter it is refused: no corpus test pins it.
#[test]
fn issue_6211_bare_this_in_a_table_by_value_runs_and_by_var_is_refused() {
    let objects = "
table 50110 \"TB Row\"
{
    fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } }
    keys { key(PK; Id) { Clustered = true; } }

    procedure PassThis(var Other: Record \"TB Row\")
    begin
        Other.CopyFrom(this);
    end;

    procedure CopyFrom(Source: Record \"TB Row\")
    begin
        Rec := Source;
    end;

    procedure PassThisByVar(var Other: Record \"TB Row\")
    begin
        Other.Take(this);
    end;

    procedure Take(var Source: Record \"TB Row\")
    begin
        Rec := Source;
    end;
}

codeunit 50111 T
{
    Subtype = Test;

    [Test]
    procedure ByValue()
    var
        Row: Record \"TB Row\";
        Other: Record \"TB Row\";
    begin
        Row.Id := 9;
        Row.Name := 'nine';
        Row.PassThis(Other);
        if (Other.Id <> 9) or (Other.Name <> 'nine') then
            Error('got %1 %2', Other.Id, Other.Name);
    end;

    [Test]
    procedure ByVar()
    var
        Row: Record \"TB Row\";
        Other: Record \"TB Row\";
    begin
        Row.PassThisByVar(Other);
    end;
}";
    let results = run_with_deps(&[], objects);
    let get = |n: &str| results.iter().find(|(t, _)| t == n).map(|(_, o)| o.clone()).unwrap();
    assert_eq!(get("ByValue"), Outcome::Pass);
    assert!(
        matches!(get("ByVar"), Outcome::NotRun(ref why) if why.contains("passed to a `var` parameter (not pinned)")),
        "{:?}",
        get("ByVar")
    );
}

/// Runs the test codeunit `T` of a workspace app with id `main_id` (its manifest declares the logo `logo`, `None` when no
/// manifest was read) over the platform's System app and an optional dependency app `dep` (id `DEP_ID`).
fn run_over_system(
    main_id: &str,
    logo: Option<al_ir::AppLogo>,
    dep: Option<&str>,
    main: &str,
) -> Vec<(String, Outcome)> {
    use al_deps::package::read_package;
    let packages = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus/tests/al-language/.alpackages");
    let system = read_package(&std::fs::read(packages.join("System.app")).expect("corpus submodule checked out"))
        .expect("System.app reads")
        .1;
    let app = |id: &str, deps: Vec<String>, files: Vec<(String, String)>| AppSource {
        id: id.into(),
        name: id.into(),
        dependencies: deps,
        files,
    };
    let mut sources = vec![
        app(
            main_id,
            dep.map(|_| vec![DEP_ID.to_string()]).unwrap_or_default(),
            vec![("main.al".into(), main.into())],
        ),
        app(al_sema::workspace::PLATFORM_APP_ID, Vec::new(), system),
    ];
    let mut kinds = vec![AppKind::Workspace, AppKind::Dependency];
    if let Some(dep) = dep {
        sources.push(app(DEP_ID, Vec::new(), vec![("dep.al".into(), dep.into())]));
        kinds.push(AppKind::Dependency);
    }
    let symbols = ["BC27PLUS".to_string()].into();
    let (_, l) = al_sema::lower::compile_apps_with(sources, kinds, &symbols, None, Default::default());
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let mut program = (*l.program).clone();
    for a in program.apps.iter_mut() {
        a.version = "1.0.0.0".into();
        if a.id == main_id {
            a.logo = logo.clone();
        }
    }
    let engine = Engine::new(std::sync::Arc::new(program), blocked);
    let oi = l.program.objects.iter().position(|o| o.name == "T").unwrap();
    run_codeunit(&engine, &l, oi)
}

const DEP_ID: &str = "437dbf0e-84ff-417a-965d-ed2bb9650972";
const MAIN_ID: &str = "5d86850b-0d76-4eca-bd7b-951ad998e997";

/// al-runner issue 6148, cause 3: install code asks AllObj for the TableData row of a Base Application table (Contact)
/// before it seeds setup rows. The row exists with the Table row's name and package ids; a table that does not exist
/// has none. Pin: metadata/lt-all-obj-namespace/AllObjNamespace.al UG_AllObj_TableDataRow_OfContact_AllColumns and
/// UG_AllObj_TableDataRow_OfMissingTable_EveryAccessForm (corpus branch al-runtime-rs-staging-ug eff7eb2a; assumption
/// `allobj-tabledata-row-of-dependency-table` until the bump).
#[test]
fn issue_6148_allobj_table_data_row_of_a_dependency_table() {
    let dep = r#"table 5050 Contact { fields { field(1; "No."; Code[20]) { } } keys { key(PK; "No.") { Clustered = true; } } }"#;
    let main = r#"
codeunit 50000 T { Subtype = Test;
  [Test] procedure ContactExists() var A: Record AllObj; Tbl: Record AllObj; begin
    if not A.Get(A."Object Type"::TableData, Database::Contact) then Error('no TableData row');
    if A."Object Name" <> 'Contact' then Error('name %1', A."Object Name");
    Tbl.Get(Tbl."Object Type"::Table, 5050);
    if A."App Package ID" <> Tbl."App Package ID" then Error('package');
    if A."App Runtime Package ID" <> Tbl."App Runtime Package ID" then Error('runtime package');
    A.Reset(); A.SetRange("Object Type", A."Object Type"::TableData); A.SetRange("Object ID", 5050);
    if A.IsEmpty() then Error('empty');
    if A.Count() <> 1 then Error('count %1', A.Count()); end;
  [Test] procedure MissingTable() var A: Record AllObj; begin
    if A.Get(A."Object Type"::TableData, 73199) then Error('row');
    A.SetRange("Object Type", A."Object Type"::TableData); A.SetRange("Object ID", 73199);
    if not A.IsEmpty() then Error('not empty'); end; }"#;
    all_pass(run_over_system(MAIN_ID, Some(al_ir::AppLogo::None), Some(dep), main));
}

/// al-runner issue 6148, cause 2: Company-Initialize reaches Guided Experience Impl.GetExtensionLogo for an app that
/// registers assisted setup: the Published Application row's Logo, then the Media row by its MediaId. An app whose
/// app.json declares no logo has a blank Logo and no Media row; an app whose manifest was not read stays refused.
/// Pin: onprem session/PZPublishedApplicationColumns.al Logo_GuidedExperienceSequence_OfThisAppAndMicrosoftApps
/// (corpus branch al-runtime-rs-staging-ug eff7eb2a).
#[test]
fn issue_6148_extension_logo_of_an_app_without_a_logo() {
    let main = format!(
        r#"
codeunit 50000 T {{ Subtype = Test;
  [Test] procedure GetExtensionLogo() var App: Record "Published Application"; Media: Record Media; begin
    App.SetRange(ID, '{MAIN_ID}');
    App.SetRange("Tenant Visible", true);
    if not App.FindFirst() then Error('no row');
    if App.Logo.HasValue() then Error('logo');
    if not IsNullGuid(App.Logo.MediaId()) then Error('media id');
    Media.SetRange(ID, App.Logo.MediaId());
    if Media.FindFirst() then Error('media row'); end; }}"#
    );
    all_pass(run_over_system(MAIN_ID, Some(al_ir::AppLogo::None), None, &main));
    let o = run_over_system(MAIN_ID, None, None, &main);
    assert!(matches!(&o[0].1, Outcome::NotRun(why) if why.contains("Logo")), "{o:?}");
}

/// The 67-byte 1 x 1 gray PNG of cross-app fixture D (onprem session/UsAppLogo.al:15).
const TINY_PNG: [u8; 67] = [
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00,
    0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x00, 0x00, 0x00, 0x00, 0x3a, 0x7e, 0x9b, 0x55, 0x00, 0x00, 0x00, 0x0a, 0x49,
    0x44, 0x41, 0x54, 0x78, 0xda, 0x63, 0x68, 0x00, 0x00, 0x00, 0x82, 0x00, 0x81, 0xda, 0x45, 0x08, 0x3b, 0x00, 0x00,
    0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
];

/// al-runner issue 6148, the rest of cause 2: the Published Application Logo of an app whose manifest declares a logo is a
/// media with a value, the same id on every read, and a Media row that holds the logo file's bytes unchanged: image/png, the
/// picture's height and width, every other column empty, Prohibit Cache No. Pin: onprem session/UsAppLogo.al:79-117
/// UsLogo_TinyPng_AppD and :120-148 UsLogo_BigPng_AppC (corpus PR 816; assumption `published-application-logo-is-the-logo-file`
/// until the pin includes it). A logo that is no PNG stays refused.
#[test]
fn issue_6148_extension_logo_of_an_app_with_a_logo() {
    let main = format!(
        r#"
codeunit 50000 T {{ Subtype = Test;
  [Test] procedure GetExtensionLogo() var App: Record "Published Application"; Media: Record Media; First: Guid; InS: InStream; B: Byte; N: Integer; begin
    App.SetRange(ID, '{MAIN_ID}');
    App.SetRange("Tenant Visible", true);
    if not App.FindFirst() then Error('no row');
    if not App.Logo.HasValue() then Error('no logo');
    if IsNullGuid(App.Logo.MediaId()) then Error('null media id');
    First := App.Logo.MediaId();
    App.FindFirst();
    if App.Logo.MediaId() <> First then Error('other media id');
    Media.SetRange(ID, App.Logo.MediaId());
    if not Media.FindFirst() then Error('no media row');
    if Media.Count() <> 1 then Error('count %1', Media.Count());
    if Media.Description <> '' then Error('description');
    if Media."Mime Type" <> 'image/png' then Error('mime %1', Media."Mime Type");
    if (Media.Height <> 1) or (Media.Width <> 1) then Error('size %1 x %2', Media.Height, Media.Width);
    if Media."Company Name" <> '' then Error('company');
    if Media."Expiration Date" <> 0DT then Error('expiration');
    if Media."Prohibit Cache" then Error('prohibit cache');
    if Media."File Name" <> '' then Error('file name');
    if Media."Security Token" <> '' then Error('token');
    if Media."Creating User" <> '' then Error('user');
    if not Media.Content.HasValue() then Error('content');
    Media.CalcFields(Content);
    Media.Content.CreateInStream(InS);
    while not InS.EOS() do begin InS.Read(B); N += 1; if N = 2 then if B <> 80 then Error('byte 2 %1', B); end;
    if N <> 67 then Error('length %1', N);
    if not Media.Get(App.Logo.MediaId()) then Error('get'); end; }}"#
    );
    all_pass(run_over_system(
        MAIN_ID,
        Some(al_ir::AppLogo::File(TINY_PNG.to_vec())),
        None,
        &main,
    ));
    // A logo that is no PNG: its MIME type and size are not pinned.
    let o = run_over_system(MAIN_ID, Some(al_ir::AppLogo::File(b"GIF89a".to_vec())), None, &main);
    assert!(matches!(&o[0].1, Outcome::NotRun(why) if why.contains("Logo")), "{o:?}");
}

/// al-runner issue 6148: Guided Experience Impl.InsertGuidedExperienceItem imports the app's logo with the extension id (a
/// Guid) as the description: `Icon.ImportStream(IconInStream, ExtensionId)`. The Tenant Media row's Description is the
/// Guid's text form in braces and upper case, MIME type image/png, the picture's size, no file name. Pin: onprem
/// session/UsAppLogo.al:170-186 UsLogo_GuidedExperienceIcon_CallerD (corpus PR 816). Its bytes (re-encoded) stay refused.
#[test]
fn issue_6148_media_import_stream_with_a_guid_description() {
    let main = format!(
        r#"
table 50100 "UR Icon" {{ fields {{ field(1; Code; Code[10]) {{ }} field(2; Icon; Media) {{ }} }} keys {{ key(PK; Code) {{ Clustered = true; }} }} }}
codeunit 50000 T {{ Subtype = Test;
  [Test] procedure ImportWithGuid() var App: Record "Published Application"; Media: Record Media; R: Record "UR Icon"; TM: Record "Tenant Media"; InS: InStream; Id: Guid; begin
    Id := '{MAIN_ID}';
    App.SetRange(ID, Id);
    App.FindFirst();
    Media.SetRange(ID, App.Logo.MediaId());
    Media.FindFirst();
    Media.CalcFields(Content);
    Media.Content.CreateInStream(InS);
    R.Code := 'A';
    R.Icon.ImportStream(InS, Id);
    R.Insert();
    TM.Get(R.Icon.MediaId());
    if TM.Description <> '{{5D86850B-0D76-4ECA-BD7B-951AD998E997}}' then Error('description %1', TM.Description);
    if TM."Mime Type" <> 'image/png' then Error('mime %1', TM."Mime Type");
    if (TM.Width <> 1) or (TM.Height <> 1) then Error('size'); end; }}"#
    );
    all_pass(run_over_system(
        MAIN_ID,
        Some(al_ir::AppLogo::File(TINY_PNG.to_vec())),
        None,
        &main,
    ));
    // BC re-encodes the picture it imports (UsAppLogo.al:7; types/W7Media.al:141-145): a look at the Tenant Media bytes is refused.
    let look = main
        .replace(
            "if (TM.Width <> 1) or (TM.Height <> 1) then Error('size'); end;",
            "TM.CalcFields(Content); TM.Content.CreateInStream(InS); InS.Read(B); end;",
        )
        .replace("InS: InStream;", "InS: InStream; B: Byte;");
    let o = run_over_system(MAIN_ID, Some(al_ir::AppLogo::File(TINY_PNG.to_vec())), None, &look);
    assert!(matches!(&o[0].1, Outcome::NotRun(_)), "{o:?}");
}

/// Media rows other than an app's logo and the null id are not pinned: a read of another id is refused.
#[test]
fn media_row_of_another_id_stays_refused() {
    let main = r#"
codeunit 50000 T { Subtype = Test;
  [Test] procedure Other() var Media: Record Media; begin
    Media.SetRange(ID, '{11111111-2222-3333-4444-555555555555}');
    if Media.FindFirst() then; end; }"#;
    let o = run_over_system(MAIN_ID, Some(al_ir::AppLogo::File(TINY_PNG.to_vec())), None, main);
    assert!(matches!(&o[0].1, Outcome::NotRun(_)), "{o:?}");
}

/// al-runner issue 6191: ForNAV's GetBuiltInLayout reads "User Defined" (column 14) of the Report Layout List rows of a
/// report while an app installs. A built-in RDLC or Word layout is not user defined; a report without layouts has no
/// rows. Pin: platform/lt-ug-report-layout-list/UgReportLayoutList.al:112, :146.
#[test]
fn issue_6191_built_in_layout_is_not_user_defined() {
    let main = r#"
report 50100 "UR Layouts" { DefaultRenderingLayout = Rdlc;
  dataset { dataitem(I; Integer) { column(N; Number) { } } }
  rendering { layout(Rdlc) { Type = RDLC; LayoutFile = 'a.rdl'; } layout(Word) { Type = Word; LayoutFile = 'a.docx'; } } }
report 50101 "UR No Layout" { dataset { dataitem(I; Integer) { column(N; Number) { } } } }
codeunit 50000 T { Subtype = Test;
  [Test] procedure GetBuiltInLayout() var L: Record "Report Layout List"; begin
    L.SetRange("Report ID", Report::"UR Layouts");
    L.SetRange("User Defined", false);
    if L.Count() <> 2 then Error('count %1', L.Count());
    L.FindFirst();
    if L."User Defined" then Error('user defined'); end;
  [Test] procedure NoLayout() var L: Record "Report Layout List"; begin
    L.SetRange("Report ID", Report::"UR No Layout");
    if not L.IsEmpty() then Error('rows'); end; }"#;
    all_pass(run_over_system(MAIN_ID, Some(al_ir::AppLogo::None), None, main));
}

/// Issue 6213: a conditional expression with a Text[N] true branch and an unbounded Text false branch (a Text variable or a
/// StrSubstNo result). A false value that fits is returned unchanged; a longer one raises StringLengthExceeded with the value
/// in the message cut to its first 200 characters plus `...`. Pins: types/lt-up-ternary/UpTernary.al (corpus PR 808, branch
/// commit 910da83e) UP_Ternary_Text250TrueUnboundedTextFalse and _Length; types/TestTernaryContracts.al PYC_Ternary_MoreBranchPairs
/// (Text[10] against a 12-character Text).
const ISSUE_6213_OBJECTS: &str = "
codeunit 50100 \"TT Url Builder\"
{
    var
        BaseUrl: Text[250];
        AccountName: Text;
        Short: Text[30];

    procedure SetAccountName(NewAccountName: Text)
    begin
        AccountName := NewAccountName;
    end;

    procedure SetBaseUrl(NewBaseUrl: Text[250])
    begin
        BaseUrl := NewBaseUrl;
    end;

    procedure GetBaseUrl(): Text[250]
    begin
        exit(BaseUrl);
    end;

    procedure GetUrl(): Text
    var
        BaseUrlTok: Label 'https://%1.blob.core.windows.net', Locked = true;
    begin
        exit(BaseUrl <> '' ? GetBaseUrl() : StrSubstNo(BaseUrlTok, AccountName));
    end;

    procedure GetUrlFromText(): Text
    var
        Fallback: Text;
    begin
        Fallback := 'https://' + AccountName;
        exit(BaseUrl <> '' ? BaseUrl : Fallback);
    end;

    procedure GetIntoText(): Text
    var
        Result: Text;
    begin
        Result := BaseUrl <> '' ? BaseUrl : AccountName;
        exit(Result);
    end;

    procedure GetShort(): Text
    begin
        exit(Short <> '' ? Short : AccountName);
    end;
}

codeunit 50101 T
{
    Subtype = Test;

    [Test]
    procedure FalseValueThatFits()
    var
        Builder: Codeunit \"TT Url Builder\";
    begin
        Builder.SetAccountName('acct');
        if Builder.GetUrl() <> 'https://acct.blob.core.windows.net' then
            Error('got %1', Builder.GetUrl());
        if Builder.GetUrlFromText() <> 'https://acct' then
            Error('got %1', Builder.GetUrlFromText());
        if Builder.GetIntoText() <> 'acct' then
            Error('got %1', Builder.GetIntoText());
        Builder.SetBaseUrl('https://custom.example');
        if Builder.GetUrl() <> 'https://custom.example' then
            Error('got %1', Builder.GetUrl());
    end;

    [Test]
    procedure FalseValueTooLong()
    var
        Builder: Codeunit \"TT Url Builder\";
    begin
        Builder.SetAccountName(PadStr('', 300, 'a'));
        asserterror Builder.GetIntoText();
        if GetLastErrorCode() <> 'StringLengthExceeded' then
            Error('code %1', GetLastErrorCode());
        if GetLastErrorText() <> 'The length of the string is 300, but it must be less than or equal to 250 characters. Value: ' + PadStr('', 200, 'a') + '...' then
            Error('text %1', GetLastErrorText());
    end;

    [Test]
    procedure OtherLength()
    var
        Builder: Codeunit \"TT Url Builder\";
    begin
        Builder.SetAccountName('acct');
        if Builder.GetShort() <> 'acct' then
            Error('got %1', Builder.GetShort());
    end;
}";

#[test]
fn issue_6213_text_n_true_branch_and_unbounded_text_false_branch() {
    // Another length than 10 and 250 (here Text[30]) runs too: the false branch converts to the true branch's Text[N] for
    // every length (types/lt-y31-conversion-matrix/Y31ConvCd.al:3015 Text10_Text).
    for (name, outcome) in run_with_deps(&[], ISSUE_6213_OBJECTS) {
        assert_eq!(outcome, Outcome::Pass, "{name}");
    }
}

/// Issue 6213 (reopened): a Text[50] true branch and a built-in function returning Text as false branch (CompanyName(),
/// UserId(), Format(x)) compiles; a false value that fits is returned unchanged, and the result assigned to a shorter Text
/// fails as that assignment does. Pins: lt-ss1030/SsTernaryBuiltinText.al (corpus branch al-runtime-rs-staging-ss1030 at
/// c36d14ea, not yet on master): Ternary_TextNTrue_BuiltinFalse_ValuesAndLengths (:47-69) and _IntoShorterTarget (:71-98).
/// A false value longer than 50 raises StringLengthExceeded when the branch is taken, as a false branch longer than the true
/// branch's Text[N] does (types/lt-y31-conversion-matrix/Y31ConvCd.al:3015 Text10_Text, `eleven` and `guidlow`).
const ISSUE_6213B_OBJECTS: &str = "
codeunit 50100 \"TB Holder\"
{
    var
        Named: Text[50];

    procedure SetNamed(NewName: Text[50])
    begin
        Named := NewName;
    end;

    procedure Company(): Text
    var
        Result: Text;
    begin
        Result := Named <> '' ? Named : CompanyName();
        exit(Result);
    end;

    procedure User(): Text
    begin
        exit(Named <> '' ? Named : UserId());
    end;

    procedure Formatted(Value: Integer): Text
    begin
        exit(Named <> '' ? Named : Format(Value));
    end;

    procedure CompanyIntoText10(var Target: Text[10])
    begin
        Target := Named <> '' ? Named : CompanyName();
    end;

    procedure Long(): Text
    begin
        exit(Named <> '' ? Named : PadStr('', 60, 'x'));
    end;
}

codeunit 50101 T
{
    Subtype = Test;

    [Test]
    procedure ValuesAndLengths()
    var
        Holder: Codeunit \"TB Holder\";
    begin
        if Holder.Company() <> CompanyName() then
            Error('company %1', Holder.Company());
        if Holder.User() <> UserId() then
            Error('user %1', Holder.User());
        if Holder.Formatted(12345) <> '12345' then
            Error('format %1', Holder.Formatted(12345));
        Holder.SetNamed('Named Company');
        if (Holder.Company() <> 'Named Company') or (Holder.User() <> 'Named Company') or (Holder.Formatted(12345) <> 'Named Company') then
            Error('set');
        if StrLen(Holder.Company()) <> 13 then
            Error('len');
    end;

    [Test]
    procedure IntoShorterTarget()
    var
        Holder: Codeunit \"TB Holder\";
        Target: Text[10];
    begin
        Holder.SetNamed('Short');
        Holder.CompanyIntoText10(Target);
        if Target <> 'Short' then
            Error('fits %1', Target);
        Holder.SetNamed('');
        asserterror Holder.CompanyIntoText10(Target);
        if GetLastErrorText() <> StrSubstNo('The length of the string is %1, but it must be less than or equal to 10 characters. Value: %2', StrLen(CompanyName()), CompanyName()) then
            Error('text %1', GetLastErrorText());
        if (Target <> 'Short') or (MaxStrLen(Target) <> 10) then
            Error('target %1', Target);
    end;

    [Test]
    procedure LongerThanTheTrueBranch()
    var
        Holder: Codeunit \"TB Holder\";
    begin
        Holder.Long();
    end;
}";

#[test]
fn issue_6213_text_50_true_branch_and_builtin_text_false_branch() {
    for (name, outcome) in run_with_deps(&[], ISSUE_6213B_OBJECTS) {
        if name.ends_with("LongerThanTheTrueBranch") {
            assert!(
                matches!(&outcome, Outcome::Fail(why) if why.starts_with("The length of the string is 60, but it must be less than or equal to 50 characters.")),
                "{outcome:?}"
            );
        } else {
            assert_eq!(outcome, Outcome::Pass, "{name}");
        }
    }
}

/// Issue 6215: an interface true branch and a codeunit false branch that implements it compiles and the false branch calls
/// the codeunit's implementation. Pin: types/lt-up-ternary/UpTernary.al (corpus PR 808, branch commit 910da83e)
/// UP_Ternary_InterfaceTrueCodeunitFalse.
const ISSUE_6215_OBJECTS: &str = "
interface \"TI Preview\" { procedure GetPreview(): Text; }
interface \"TI Selector\" { procedure GetName(): Text; }
codeunit 50100 \"TI Special\" implements \"TI Preview\", \"TI Selector\"
{
    procedure GetPreview(): Text begin exit('special'); end;
    procedure GetName(): Text begin exit('special'); end;
}
codeunit 50101 \"TI Default\" implements \"TI Preview\"
{
    procedure GetPreview(): Text begin exit('default'); end;
}
codeunit 50102 \"TI Factory\"
{
    procedure GetSelector(): Interface \"TI Selector\"
    var
        Special: Codeunit \"TI Special\";
    begin
        exit(Special);
    end;

    procedure ViaCast(UseSpecial: Boolean): Interface \"TI Preview\"
    var
        Default: Codeunit \"TI Default\";
    begin
        exit(UseSpecial ? (GetSelector() as \"TI Preview\") : Default);
    end;

    procedure Assigned(UseSpecial: Boolean): Text
    var
        Default: Codeunit \"TI Default\";
        Special: Codeunit \"TI Special\";
        Preview: Interface \"TI Preview\";
        Chosen: Interface \"TI Preview\";
    begin
        Preview := Special;
        Chosen := UseSpecial ? Preview : Default;
        exit(Chosen.GetPreview());
    end;
}
codeunit 50103 T
{
    Subtype = Test;

    [Test]
    procedure InterfaceTrueCodeunitFalse()
    var
        Factory: Codeunit \"TI Factory\";
    begin
        if Factory.ViaCast(false).GetPreview() <> 'default' then
            Error('cast false %1', Factory.ViaCast(false).GetPreview());
        if Factory.ViaCast(true).GetPreview() <> 'special' then
            Error('cast true %1', Factory.ViaCast(true).GetPreview());
        if Factory.Assigned(false) <> 'default' then
            Error('assigned false');
        if Factory.Assigned(true) <> 'special' then
            Error('assigned true');
    end;
}";

#[test]
fn issue_6215_interface_true_branch_and_implementing_codeunit_false_branch() {
    all_pass(run_with_deps(&[], ISSUE_6215_OBJECTS));
}

/// Issue 6217: addafter/addlast of a promoted category group the page does not declare (Category_Category7,
/// Category_Category12, Category_New) compiles and the extension's action and actionref run from a TestPage. Pin:
/// pages/lt-up-page-ext/UpPromotedCategories.al (corpus PR 808, branch commit 910da83e).
fn issue_6217_objects(anchor: &str) -> String {
    format!(
        "
table 50100 \"MA Entry\" {{ fields {{ field(1; \"No.\"; Code[20]) {{ }} }} keys {{ key(PK; \"No.\") {{ Clustered = true; }} }} }}
codeunit 50104 \"MA Log\" {{ SingleInstance = true; var L: Text; procedure Add(E: Text) begin L += E; end; procedure Take(): Text begin exit(L); end; }}
page 50101 \"MA List\"
{{
    PageType = List;
    SourceTable = \"MA Entry\";
    layout {{ area(Content) {{ repeater(Lines) {{ field(\"No.\"; Rec.\"No.\") {{ }} }} }} }}
    actions
    {{
        area(Processing)
        {{
            action(Post) {{ Caption = 'Post'; trigger OnAction() begin end; }}
        }}
        area(Promoted)
        {{
            group(Category_Process) {{ Caption = 'Process'; actionref(Post_Promoted; Post) {{ }} }}
            group(Category_Category4) {{ Caption = 'Page'; }}
        }}
    }}
}}
pageextension 50102 \"MA List Ext\" extends \"MA List\"
{{
    actions
    {{
        addlast(Processing)
        {{
            action(OpenSomething)
            {{
                Caption = 'Open Something';
                ApplicationArea = All;
                trigger OnAction() var Log: Codeunit \"MA Log\"; begin Log.Add('x'); end;
            }}
        }}
        {anchor}
        {{
            group(MyCategory)
            {{
                Caption = 'My Category';
                actionref(OpenSomething_Promoted; OpenSomething) {{ }}
            }}
        }}
    }}
}}
codeunit 50103 T
{{
    Subtype = Test;

    [Test]
    procedure P()
    var
        MAList: TestPage \"MA List\";
        Log: Codeunit \"MA Log\";
        R: Record \"MA Entry\";
    begin
        R.\"No.\" := 'A';
        R.Insert();
        MAList.OpenView();
        MAList.OpenSomething.Invoke();
        MAList.OpenSomething_Promoted.Invoke();
        MAList.Close();
        if Log.Take() <> 'xx' then
            Error('log %1', Log.Take());
    end;
}}"
    )
}

#[test]
fn issue_6217_extension_adds_after_an_undeclared_promoted_category() {
    for anchor in [
        "addafter(Category_Category7)",
        "addlast(Category_Category7)",
        "addafter(Category_Category12)",
        "addafter(Category_New)",
        "addlast(Category_New)",
    ] {
        let results = run_with_deps(&[], &issue_6217_objects(anchor));
        assert!(!results.is_empty(), "{anchor}");
        for (name, outcome) in results {
            assert_eq!(outcome, Outcome::Pass, "{anchor}: {name}");
        }
    }
}

/// Issue 6217: the forms corpus PR 808 does not pin (addbefore/addfirst) stay refused as not pinned. Another numbered group from
/// 4 to 20 is the same implicit group (al-runner issue 6646, tests/suite/pageext_implicit_promoted.rs).
#[test]
fn issue_6217_unpinned_promoted_category_forms_stay_refused() {
    for anchor in ["addbefore(Category_Category7)", "addfirst(Category_Category7)"] {
        let results = run_with_deps(&[], &issue_6217_objects(anchor));
        assert!(
            matches!(&results[..], [(_, Outcome::NotRun(why))] if why.contains("not pinned")),
            "{anchor}: {results:?}"
        );
    }
}

/// Issue 6219: a page extension reads its base page's protected var through `CurrPage.` and assigns the plain name. Pin:
/// pages/lt-up-page-ext/UpPageExt.al (corpus PR 808, branch commit 910da83e) UP_PageExt_ReadsProtectedVarThroughCurrPage.
/// Assigning through `CurrPage.<var>` is not pinned (real BC failed to open the page in the writer's probe) and stays refused.
fn issue_6219_objects(trigger_body: &str) -> String {
    format!(
        "
table 50100 \"CP Entry\" {{ fields {{ field(1; \"No.\"; Code[20]) {{ }} }} keys {{ key(PK; \"No.\") {{ Clustered = true; }} }} }}
page 50101 \"CP List\"
{{
    PageType = List;
    SourceTable = \"CP Entry\";
    layout {{ area(Content) {{ repeater(Lines) {{ field(\"No.\"; Rec.\"No.\") {{ }} }} }} }}
    actions
    {{
        area(Processing)
        {{
            action(Apply) {{ Caption = 'Apply'; Enabled = ApplyEnabled; trigger OnAction() begin end; }}
        }}
    }}
    trigger OnOpenPage()
    begin
        ApplyEnabled := false;
    end;

    protected var
        ApplyEnabled: Boolean;
}}
pageextension 50102 \"CP List Ext\" extends \"CP List\"
{{
    trigger OnOpenPage()
    begin
        {trigger_body}
    end;
}}
codeunit 50103 T
{{
    Subtype = Test;

    [Test]
    procedure P()
    var
        CPList: TestPage \"CP List\";
    begin
        CPList.OpenView();
        if not CPList.Apply.Enabled() then
            Error('not enabled');
        CPList.Close();
    end;
}}"
    )
}

#[test]
fn issue_6219_extension_reads_protected_var_through_currpage() {
    all_pass(run_with_deps(
        &[],
        &issue_6219_objects("ApplyEnabled := not CurrPage.ApplyEnabled;"),
    ));
}

#[test]
fn issue_6219_assigning_through_currpage_stays_refused() {
    let results = run_with_deps(&[], &issue_6219_objects("CurrPage.ApplyEnabled := true;"));
    assert!(
        matches!(&results[..], [(_, Outcome::NotRun(why))] if why.contains("CurrPage.ApplyEnabled") && why.contains("not pinned")),
        "{results:?}"
    );
}

/// al-runner #6412: MaxIteration on a data item over a record table stops it after that many rows, a skipped row counting
/// toward them, and the record stands on the next row in OnPostDataItem (report/lt-maxiteration-zero/
/// SgMaxIterationTable.al with MaxIterationZero.al:153-199; corpus branch al-runtime-rs-staging-y11
/// Y11MaxIterationRecord.al).
#[test]
fn issue_6412_max_iteration_caps_a_record_data_item() {
    let main = "
table 69640 \"UR MI Row\" { fields { field(1; No; Integer) { } } keys { key(PK; No) { Clustered = true; } } }
codeunit 69641 \"UR MI Log\" { SingleInstance = true; var T: Text;
  procedure Add(S: Text) begin T += S + ';'; end;
  procedure Take(): Text var R: Text; begin R := T; T := ''; exit(R); end; }
report 69642 \"UR MI Two\" { ProcessingOnly = true; UseRequestPage = false;
  dataset { dataitem(R; \"UR MI Row\") { MaxIteration = 2;
    trigger OnAfterGetRecord() begin L.Add('row ' + Format(R.No)); if R.No = 1 then CurrReport.Skip(); end;
    trigger OnPostDataItem() begin L.Add('post at ' + Format(R.No)); end; } }
  var L: Codeunit \"UR MI Log\"; }
report 69643 \"UR MI Ten\" { ProcessingOnly = true; UseRequestPage = false;
  dataset { dataitem(R; \"UR MI Row\") { MaxIteration = 10;
    trigger OnAfterGetRecord() begin L.Add('row ' + Format(R.No)); end; } }
  var L: Codeunit \"UR MI Log\"; }
codeunit 69644 T { Subtype = Test;
  [Test] procedure P() var Row: Record \"UR MI Row\"; L: Codeunit \"UR MI Log\"; I: Integer; begin
    for I := 1 to 5 do begin Row.No := I; Row.Insert(); end;
    Report.Run(Report::\"UR MI Two\");
    if L.Take() <> 'row 1;row 2;post at 3;' then Error('two');
    Report.Run(Report::\"UR MI Ten\");
    if L.Take() <> 'row 1;row 2;row 3;row 4;row 5;' then Error('ten');
  end; }";
    all_pass(run_with_deps(&[], main));
}

/// Gap cycle G8: OnPrem platform tables that a fresh tenant leaves empty and that take every write
/// (al-language-onprem/platform/PtOnPremTables.al:57-78, :80-101, :149-170, :192-213), and Device, whose Insert raises the
/// license error and stores nothing (:172-190).
#[test]
fn empty_writable_onprem_platform_tables() {
    let main = r#"
codeunit 50000 T { Subtype = Test;
  [Test] procedure ConfigurationPackageFile() var R: Record "Configuration Package File"; begin
    if not R.IsEmpty() then Error('rows');
    R.Code := 'LTPT'; R.Name := 'LT PT Package'; if not R.Insert(true) then Error('insert');
    if R.Insert(true) then Error('duplicate');
    R.Name := 'Renamed'; R.Modify(true); R.Get('LTPT'); if R.Name <> 'Renamed' then Error('modify');
    R.Delete(true); if R.Count() <> 0 then Error('delete');
    R.Insert(false); R.DeleteAll(false); if R.Count() <> 0 then Error('delete all'); end;
  [Test] procedure DocumentServiceScenario() var R: Record "Document Service Scenario"; begin
    if R.FindFirst() then Error('rows');
    R."Document Service" := 'LTPT'; R."Use for Application" := true; R.Insert(true);
    R.FindFirst(); if R."Service Integration" <> R."Service Integration"::OneDrive then Error('option'); end;
  [Test] procedure ReportSettingsOverride() var R: Record "Report Settings Override"; begin
    if not R.IsEmpty() then Error('rows');
    R."Object ID" := 99999; R."Company Name" := 'LTPT'; R.Timeout := 30; R.Insert(true);
    R.Timeout := 60; R.Modify(true); R.Get(99999, 'LTPT'); if R.Timeout <> 60 then Error('modify'); end;
  [Test] procedure TenantProfile() var R: Record "Tenant Profile"; begin
    if not R.IsEmpty() then Error('rows');
    R."Profile ID" := 'LTPTPROFILE'; R.Description := 'LT PT Profile'; R.Insert(true);
    R.FindFirst(); if R."Customization Status" <> R."Customization Status"::Updated then Error('status'); end;
  [Test] procedure Device() var R: Record Device; begin
    if not R.IsEmpty() then Error('rows');
    R."MAC Address" := 'LTPT0001'; R.Name := 'LT PT Device';
    asserterror R.Insert(true);
    if GetLastErrorText() <> 'The new devices do not meet the terms of the current license. The current license allows 0 full devices, 0 limited devices, 0 ISV devices, 0 ISV functional devices and 1 devices.' then Error('text %1', GetLastErrorText());
    if R.Count() <> 0 then Error('stored'); end; }"#;
    all_pass(run_over_system(MAIN_ID, None, None, main));
}

/// Gap cycle G8: a Device Insert without triggers is not pinned (PtOnPremTables.al:180 inserts with triggers).
#[test]
fn device_insert_without_triggers_is_refused() {
    let main = r#"
codeunit 50000 T { Subtype = Test;
  [Test] procedure NoTriggers() var R: Record Device; begin
    R."MAC Address" := 'LTPT0001'; if R.Insert(false) then; end; }"#;
    let o = run_over_system(MAIN_ID, None, None, main);
    assert!(matches!(&o[0].1, Outcome::NotRun(m) if m.contains("Device")), "{o:?}");
}

/// Gap cycle G8: a guarded Codeunit.Run does not trap the license error of a Device Insert: it reaches the asserterror
/// around the call (al-language-onprem/platform/PtOnPremTables.al:177-185).
#[test]
fn device_license_error_passes_a_guarded_codeunit_run() {
    let main = r#"
codeunit 50001 Ins { trigger OnRun() var R: Record Device; begin R."MAC Address" := 'LTPT0001'; R.Insert(true); end; }
codeunit 50000 T { Subtype = Test;
  [Test] procedure Guarded() var R: Record Device; begin
    asserterror if Codeunit.Run(Codeunit::Ins) then Error('ran');
    if GetLastErrorText() <> 'The new devices do not meet the terms of the current license. The current license allows 0 full devices, 0 limited devices, 0 ISV devices, 0 ISV functional devices and 1 devices.' then Error('text %1', GetLastErrorText());
    if R.Count() <> 0 then Error('stored'); end; }"#;
    all_pass(run_over_system(MAIN_ID, None, None, main));
}

/// Gap cycle G8: Power BI Blob takes every write and keeps its Blob File (al-language-onprem/platform/PtOnPremTables.al
/// Y17_PowerBIBlob_InsertModifyDelete, Y17_PowerBIBlob_BlobRoundTrip, corpus master 97c29f86). A fresh tenant holds demo rows
/// nothing pins: a read is refused until the test's own DeleteAll without filters has emptied the table, as Microsoft's Power
/// BI tests do before they insert (Tests-Integration PowerBIEmbeddedReportTests, PowerBISynchronizerTests SetupSystemBlob).
#[test]
fn power_bi_blob_is_answered_once_the_test_emptied_it() {
    let main = r#"
codeunit 50000 T { Subtype = Test;
  [Test] procedure Fresh() var B: Record "Power BI Blob"; begin if B.Count() = 0 then; end;
  [Test] procedure FilteredDeleteAll() var B: Record "Power BI Blob"; begin B.SetRange(Version, 2); B.DeleteAll(); B.Reset(); if B.IsEmpty() then; end;
  [Test] procedure ClearedThenRolledBack() var B: Record "Power BI Blob"; begin asserterror begin B.DeleteAll(); Error('x'); end; if B.IsEmpty() then; end;
  [Test] procedure Cleared() var B: Record "Power BI Blob"; R: Record "Power BI Blob"; OutS: OutStream; InS: InStream; Id: Guid; Line: Text; begin
    B.DeleteAll(); Evaluate(Id, '{77777777-7777-7777-7777-777777777777}');
    B.Init(); B.Id := Id; B.Name := 'LT'; B.Version := 3; B."Blob File".CreateOutStream(OutS); OutS.WriteText('This is not really relevant');
    if not B.Insert() then Error('insert'); if B.Insert() then Error('duplicate');
    if not R.Get(Id) then Error('get'); if not R."Blob File".HasValue() then Error('has value');
    Clear(R); R.SetAutoCalcFields("Blob File"); R.Get(Id); R."Blob File".CreateInStream(InS); InS.ReadText(Line);
    if Line <> 'This is not really relevant' then Error('text %1', Line);
    Clear(R); R.SetRange("GP Enabled", false); if not R.FindFirst() then Error('find'); R.SetRange("GP Enabled", true); if not R.IsEmpty() then Error('empty');
    B.Name := 'Renamed'; B.Modify(); B.Delete(); if B.Count() <> 0 then Error('count'); end; }"#;
    let o = run_over_system(MAIN_ID, None, None, main);
    let get = |n: &str| o.iter().find(|(t, _)| t.ends_with(n)).map(|(_, o)| o.clone()).expect(n);
    assert_eq!(get("Cleared"), Outcome::Pass);
    for t in ["Fresh", "FilteredDeleteAll", "ClearedThenRolledBack"] {
        assert!(
            matches!(get(t), Outcome::NotRun(m) if m.contains("Power BI Blob")),
            "{t}: {:?}",
            get(t)
        );
    }
}
