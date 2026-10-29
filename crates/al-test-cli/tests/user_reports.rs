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
