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
/// No corpus test pins which body a call runs in the shapes that compile, so every call of the name refuses: a bare
/// call in the base and in the extension, and a call on a page variable from outside.
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
        refused(&r, "Base");
        refused(&r, "Ext");
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
    let src = format!("{}\n{}", file, include_str!("fixtures/outside-fb/LibraryAssert.al"));
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
        include_str!("fixtures/outside-fb/Issue6139.al"),
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
        include_str!("fixtures/outside-fb/Issue6141.al"),
        "GoTo Other Row Tests",
    ));
}

/// Issue 6116: the reporter's repro (fixtures/outside-fb/Issue6116.al, with a stand-in for Library Assert) and a second test
/// codeunit that checks the exact differences BC gave the reporter. Pins: session/lt-bl-sql-counters/SkSqlBudget.al, writer TW
/// (TW_*_Counts: 1/1, 1/11, 10/10, 11/5 for the reporter's shapes) and writer TX (TX_*: the cost of each record method alone).
fn issue_6116_outcomes(exact: &str) -> Vec<(String, Outcome)> {
    let src = format!(
        "{}\n{}\n{}",
        include_str!("fixtures/outside-fb/Issue6116.al"),
        include_str!("fixtures/outside-fb/LibraryAssert.al"),
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

/// Issue 6129: only the reading the corpus branch pins but the pinned corpus does not yet hold (a procedure of a
/// TableNo codeunit) marks the assumption; the field readings are the existing behavior and stay unflagged.
#[test]
fn issue_6129_only_the_tableno_procedure_reading_is_flagged() {
    let (diags, marked) = issue_6129_compile(&issue_6129_objects(""), "LT UB Table No");
    assert!(diags.is_empty(), "{diags:?}");
    assert!(marked);
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
/// 1dc037a8; assumption `subpagelink-const-qualified-enum-value` until the bump).
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
/// UJ_PageExt_BgTask_Completed (corpus branch al-runtime-rs-staging-uj 1dc037a8; assumption `pageext-background-task-triggers`).
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
/// UJ_Temp_List_Modal_FilterAfterRebuild (corpus branch al-runtime-rs-staging-uj 1dc037a8; assumption `list-action-rebuilds-rows`).
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
