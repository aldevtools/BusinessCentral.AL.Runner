//! ITestHandler callbacks (BC 29, runtime 18.0): the handlers of the enum extensions of DefaultTestHandler run for every test
//! codeunit, then the handlers its `TestHandlers` property names, around the codeunit, each procedure and each case of a
//! data-driven procedure.
//!
//! Pins: corpus al-language-testlibraries/test-framework/data-driven-tests-and-handlers/BiTestHandlerDefault.al (header items
//! 1-6 at :12-20, codeunits 81624, 81644 and 81645), BiTestHandler.al:94-127 (Skip in OnBeforeTestProcedureRun) and
//! BiTestHandlerBehavior.al:382-420 (an error in a callback of a TestHandlers handler, two handlers).

use al_test_cli::report::Outcome;
use al_testkit::outcomes_in;

/// What System.app declares on BC 29 (the inline app has no System.app), the data source of BiDataDriven.al:72-121 and a log.
const SYSTEM: &str = r#"
interface ITestContext
{
}

interface ITestDataSource
{
    procedure ListTestCases(DataSetIdentifier: Text; context: DataSourceContext): List of [Text];
    procedure GetTestCase(DataSetIdentifier: Text; TestCaseIndex: Integer; TestCaseIdentifier: Text; context: DataSourceContext): Interface ITestContext;
}

interface ITestHandler
{
    procedure OnBeforeTestCodeunitRun(Context: TestHandlerContext)
    begin
    end;

    procedure OnAfterTestCodeunitRun(Context: TestHandlerContext)
    begin
    end;

    procedure OnBeforeTestProcedureRun(Context: TestHandlerContext)
    begin
    end;

    procedure OnAfterTestProcedureRun(Context: TestHandlerContext)
    begin
    end;

    procedure OnBeforeTestCaseRun(Context: TestHandlerContext)
    begin
    end;

    procedure OnAfterTestCaseRun(Context: TestHandlerContext)
    begin
    end;
}

enum 2000000023 TestHandler implements ITestHandler
{
    Extensible = true;
}

enum 2000000024 DefaultTestHandler implements ITestHandler
{
    Extensible = true;
}

interface "TH Case Data"
{
    procedure CaseText(): Text;
}

codeunit 50100 "TH Log"
{
    SingleInstance = true;
    var
        Entries: Text;
        Marker: Text;
    procedure Add(Entry: Text) begin Entries += Entry + ';'; end;
    procedure Take(): Text begin exit(Entries); end;
    procedure SetMarker(NewMarker: Text) begin Marker := NewMarker; end;
    procedure GetMarker(): Text begin exit(Marker); end;
}

codeunit 50101 "TH Case" implements ITestContext, "TH Case Data"
{
    var
        Value: Text;
    procedure SetText(NewValue: Text) begin Value := NewValue; end;
    procedure CaseText(): Text begin exit(Value); end;
}

codeunit 50102 "TH Source" implements ITestDataSource
{
    procedure ListTestCases(DataSetIdentifier: Text; context: DataSourceContext): List of [Text]
    var
        Cases: List of [Text];
    begin
        Cases.Add('alpha');
        Cases.Add('beta');
        exit(Cases);
    end;

    procedure GetTestCase(DataSetIdentifier: Text; TestCaseIndex: Integer; TestCaseIdentifier: Text; context: DataSourceContext): Interface ITestContext
    var
        TestCase: Codeunit "TH Case";
    begin
        TestCase.SetText(TestCaseIdentifier);
        exit(TestCase);
    end;
}
"#;

/// A handler as BiTestHandlerDefault.al:111-157 writes it: every callback logs `<prefix>.<callback>(proc|case|n=<count>)`, the
/// After callbacks with Success; OnBeforeTestProcedureRun sets the marker.
fn handler(id: u32, name: &str, prefix: &str) -> String {
    let mut s = format!("codeunit {id} \"{name}\" implements ITestHandler\n{{\n    var\n        Counter: Integer;\n");
    for (cb, short, after) in [
        ("OnBeforeTestCodeunitRun", "bCU", false),
        ("OnAfterTestCodeunitRun", "aCU", true),
        ("OnBeforeTestProcedureRun", "bP", false),
        ("OnAfterTestProcedureRun", "aP", true),
        ("OnBeforeTestCaseRun", "bC", false),
        ("OnAfterTestCaseRun", "aC", true),
    ] {
        s += &format!(
            "    procedure {cb}(Context: TestHandlerContext)\n    begin\n        Handle('{short}', Context, {after});\n    end;\n"
        );
    }
    s += &format!(
        r#"
    local procedure Handle(Callback: Text; Context: TestHandlerContext; WithSuccess: Boolean)
    var
        Log: Codeunit "TH Log";
        Entry: Text;
    begin
        Counter += 1;
        Entry := '{prefix}.' + Callback + '(' + Context.ProcedureName() + '|' + Context.TestCaseName() + '|n=' + Format(Counter);
        if WithSuccess then
            Entry += '|' + Format(Context.Success());
        Log.Add(Entry + ')');
        if Callback = 'bP' then
            Log.SetMarker('set by {prefix} in bP of ' + Context.ProcedureName());
    end;
}}
"#
    );
    s
}

const DEFAULT_EXT: &str = r#"
enumextension 50110 "TH Default" extends DefaultTestHandler
{
    value(50110; "TH Default") { Implementation = ITestHandler = "TH Default Handler"; }
}
"#;

const OWN_EXT: &str = r#"
enumextension 50111 "TH Own" extends TestHandler
{
    value(50111; "TH Own") { Implementation = ITestHandler = "TH Own Handler"; }
}
"#;

fn outcomes(objects: &str, codeunit: &str) -> Vec<(String, Outcome)> {
    outcomes_in(&format!("{SYSTEM}\n{objects}"), codeunit)
}

fn assert_all_pass(got: &[(String, Outcome)], names: &[&str]) {
    let got_names: Vec<&str> = got.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(got_names, names, "{got:?}");
    for (n, o) in got {
        assert_eq!(*o, Outcome::Pass, "{n}: {got:?}");
    }
}

// Issue 7179, BiTestHandlerDefault.al:12-13, :264 and :274: `Codeunit::"Name"` and the numeric id name the same data source.
// Issue 7180, :257-291: a test codeunit with only data-driven procedures runs, in alphabetical order.
#[test]
fn a_codeunit_with_only_data_driven_procedures_runs_both_attribute_forms() {
    let got = outcomes(
        r#"
codeunit 50120 "TH DD Only"
{
    Subtype = Test;

    [TestDataSource(50102, 'Set1')]
    procedure Rows2(Context: Interface ITestContext)
    begin
    end;

    [TestDataSource(Codeunit::"TH Source", 'Set1')]
    procedure Rows1(Context: Interface ITestContext)
    begin
    end;
}
"#,
        "TH DD Only",
    );
    assert_all_pass(&got, &["Rows1[alpha]", "Rows1[beta]", "Rows2[alpha]", "Rows2[beta]"]);
}

// BiTestHandlerDefault.al:259-291 (codeunit 81624): the default handler reaches a codeunit with only data-driven procedures;
// order bCU, then per procedure bP, (bC, body, aC) per case, aP; one handler instance for the codeunit (n grows); Success Yes;
// the marker the default handler set in bP is what the body sees.
#[test]
fn the_default_handler_runs_around_a_data_driven_only_codeunit() {
    let got = outcomes(
        &format!(
            "{}{DEFAULT_EXT}{}",
            handler(50112, "TH Default Handler", "D"),
            r#"
codeunit 50121 "TH DD Default"
{
    Subtype = Test;

    [TestDataSource(Codeunit::"TH Source", 'Set1')]
    procedure Rows1(Context: Interface ITestContext)
    var
        Log: Codeunit "TH Log";
        CaseData: Interface "TH Case Data";
    begin
        CaseData := Context as "TH Case Data";
        Log.Add('run1(' + CaseData.CaseText() + ')');
    end;

    [TestDataSource(50102, 'Set1')]
    procedure Rows2(Context: Interface ITestContext)
    var
        Log: Codeunit "TH Log";
        CaseData: Interface "TH Case Data";
        Expected: Text;
    begin
        CaseData := Context as "TH Case Data";
        Log.Add('run2(' + CaseData.CaseText() + ')');
        if CaseData.CaseText() = 'beta' then begin
            Expected := 'D.bCU(||n=1);D.bP(Rows1||n=2);D.bC(Rows1|alpha|n=3);run1(alpha);D.aC(Rows1|alpha|n=4|Yes);D.bC(Rows1|beta|n=5);run1(beta);D.aC(Rows1|beta|n=6|Yes);D.aP(Rows1||n=7|Yes);D.bP(Rows2||n=8);D.bC(Rows2|alpha|n=9);run2(alpha);D.aC(Rows2|alpha|n=10|Yes);D.bC(Rows2|beta|n=11);run2(beta);';
            if Log.Take() <> Expected then
                Error('log: %1', Log.Take());
            if Log.GetMarker() <> 'set by D in bP of Rows2' then
                Error('marker: %1', Log.GetMarker());
        end;
    end;
}
"#
        ),
        "TH DD Default",
    );
    assert_all_pass(&got, &["Rows1[alpha]", "Rows1[beta]", "Rows2[alpha]", "Rows2[beta]"]);
}

// BiTestHandlerDefault.al:293-332 (codeunit 81644): the default handler reaches a codeunit without TestHandlers; OnRun runs
// after bCU and before the first bP.
#[test]
fn the_default_handler_runs_for_a_codeunit_without_test_handlers_and_onrun_follows_bcu() {
    let got = outcomes(
        &format!(
            "{}{DEFAULT_EXT}{}",
            handler(50112, "TH Default Handler", "D"),
            r#"
codeunit 50122 "TH Default Only"
{
    Subtype = Test;

    trigger OnRun()
    var
        Log: Codeunit "TH Log";
    begin
        Log.Add('OnRun');
    end;

    [Test]
    procedure A_Plain()
    var
        Log: Codeunit "TH Log";
    begin
        Log.Add('body A marker=' + Log.GetMarker());
    end;

    [Test]
    procedure Z_Observe()
    var
        Log: Codeunit "TH Log";
    begin
        if Log.Take() <> 'D.bCU(||n=1);OnRun;D.bP(A_Plain||n=2);body A marker=set by D in bP of A_Plain;D.aP(A_Plain||n=3|Yes);D.bP(Z_Observe||n=4);' then
            Error('log: %1', Log.Take());
    end;
}
"#
        ),
        "TH Default Only",
    );
    assert_all_pass(&got, &["A_Plain", "Z_Observe"]);
}

// BiTestHandlerDefault.al:334-371 (codeunit 81645): default handlers run before the TestHandlers handlers in every callback,
// After callbacks too; each handler counts its own calls; the later handler's marker wins.
#[test]
fn default_handlers_run_before_own_handlers_in_every_callback() {
    let got = outcomes(
        &format!(
            "{}{}{DEFAULT_EXT}{OWN_EXT}{}",
            handler(50112, "TH Default Handler", "D"),
            handler(50113, "TH Own Handler", "O"),
            r#"
codeunit 50123 "TH Combo"
{
    Subtype = Test;
    TestHandlers = "TH Own";

    [Test]
    procedure A_Plain()
    var
        Log: Codeunit "TH Log";
    begin
        Log.Add('body A marker=' + Log.GetMarker());
    end;

    [TestDataSource(50102, 'Set1')]
    procedure ZRows(Context: Interface ITestContext)
    var
        Log: Codeunit "TH Log";
        CaseData: Interface "TH Case Data";
    begin
        CaseData := Context as "TH Case Data";
        Log.Add('body ' + CaseData.CaseText() + ' marker=' + Log.GetMarker());
        if CaseData.CaseText() = 'beta' then
            if Log.Take() <> 'D.bCU(||n=1);O.bCU(||n=1);D.bP(A_Plain||n=2);O.bP(A_Plain||n=2);body A marker=set by O in bP of A_Plain;D.aP(A_Plain||n=3|Yes);O.aP(A_Plain||n=3|Yes);D.bP(ZRows||n=4);O.bP(ZRows||n=4);D.bC(ZRows|alpha|n=5);O.bC(ZRows|alpha|n=5);body alpha marker=set by O in bP of ZRows;D.aC(ZRows|alpha|n=6|Yes);O.aC(ZRows|alpha|n=6|Yes);D.bC(ZRows|beta|n=7);O.bC(ZRows|beta|n=7);body beta marker=set by O in bP of ZRows;' then
                Error('log: %1', Log.Take());
    end;
}
"#
        ),
        "TH Combo",
    );
    assert_all_pass(&got, &["A_Plain", "ZRows[alpha]", "ZRows[beta]"]);
}

const SKIP_HANDLER: &str = r#"
codeunit 50114 "TH Skip Handler" implements ITestHandler
{
    procedure OnBeforeTestProcedureRun(Context: TestHandlerContext)
    var
        Log: Codeunit "TH Log";
    begin
        Log.Add('bP(' + Context.ProcedureName() + ',' + Context.CodeunitName() + ',' + Format(Context.CodeunitId()) + ')');
        if Context.ProcedureName() = 'B_Skip' then
            Context.Skip('skip reason');
        if Context.ProcedureName() = 'A_Fail' then
            Error('handler error');
    end;

    procedure OnAfterTestProcedureRun(Context: TestHandlerContext)
    var
        Log: Codeunit "TH Log";
    begin
        Log.Add('aP(' + Context.ProcedureName() + ')');
    end;
}

enumextension 50115 "TH Skip" extends TestHandler
{
    value(50115; "TH Skip") { Implementation = ITestHandler = "TH Skip Handler"; }
}
"#;

// BiTestHandler.al:52-57, :108-125: Skip in OnBeforeTestProcedureRun skips the body and OnAfterTestProcedureRun (BC reports the
// test as skipped). BiTestHandlerBehavior.al:206-213, :382-420: an error in OnBeforeTestProcedureRun of a TestHandlers handler
// does not stop the test, which passes. A handler that implements only some callbacks (System.app's ITestHandler declares
// empty bodies for all six).
#[test]
fn skip_in_before_procedure_skips_the_test_and_a_handler_error_does_not_stop_it() {
    let got = outcomes(
        &format!(
            "{SKIP_HANDLER}{}",
            r#"
codeunit 50124 "TH Skips"
{
    Subtype = Test;
    TestHandlers = "TH Skip";

    [Test]
    procedure A_Fail()
    var
        Log: Codeunit "TH Log";
    begin
        Log.Add('body A');
    end;

    [Test]
    procedure B_Skip()
    var
        Log: Codeunit "TH Log";
    begin
        Log.Add('body B');
    end;

    [Test]
    procedure C_Observe()
    var
        Log: Codeunit "TH Log";
    begin
        if Log.Take() <> 'bP(A_Fail,TH Skips,50124);body A;aP(A_Fail);bP(B_Skip,TH Skips,50124);bP(C_Observe,TH Skips,50124);' then
            Error('log: %1', Log.Take());
    end;
}
"#
        ),
        "TH Skips",
    );
    let names: Vec<&str> = got.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["A_Fail", "B_Skip", "C_Observe"], "{got:?}");
    assert_eq!(got[0].1, Outcome::Pass, "{got:?}");
    assert!(
        matches!(&got[1].1, Outcome::NotRun(w) if w.contains("skip reason")),
        "{got:?}"
    );
    assert_eq!(got[2].1, Outcome::Pass, "{got:?}");
}

// OnAfterTestCodeunitRun is not pinned for an error (BiTestHandler.al:11-12 only says its effects were not seen by a later
// codeunit): an error there makes the passing tests of the codeunit not run, never a silent pass.
#[test]
fn an_error_in_on_after_test_codeunit_run_is_refused_not_ignored() {
    let got = outcomes(
        r#"
codeunit 50130 "TH After Fails" implements ITestHandler
{
    procedure OnAfterTestCodeunitRun(Context: TestHandlerContext)
    begin
        Error('after codeunit failed');
    end;
}

codeunit 50131 "TH After"
{
    Subtype = Test;
    TestHandlers = "TH After Fails";

    [Test]
    procedure One()
    begin
    end;
}

enumextension 50132 "TH After Ext" extends TestHandler
{
    value(50132; "TH After Fails") { Implementation = ITestHandler = "TH After Fails"; }
}
"#,
        "TH After",
    );
    assert_eq!(got.len(), 1, "{got:?}");
    assert!(
        matches!(&got[0].1, Outcome::NotRun(w) if w.contains("OnAfterTestCodeunitRun")),
        "{got:?}"
    );
}

// A DefaultTestHandler value declared in a dependency app, with the test codeunit in the app that depends on it (al-runner
// 7181, the reporter's shape with the handler in another app). BiTestHandlerDefault.al pins handler and test codeunit in ONE
// app only, so this shape stays refused until a corpus probe pins it (docs/probes-needed.md, "DefaultTestHandler of another app").
#[test]
fn a_default_handler_of_a_dependency_app_is_refused_not_skipped() {
    let dep = format!("{SYSTEM}\n{}{DEFAULT_EXT}", handler(50112, "TH Default Handler", "D"));
    let got = crate::user_reports::run_with_deps(
        &[&dep],
        r#"
codeunit 50140 T
{
    Subtype = Test;

    [Test]
    procedure One()
    begin
    end;
}
"#,
    );
    assert_eq!(got.len(), 1, "{got:?}");
    assert!(
        matches!(&got[0].1, Outcome::NotRun(w) if w.contains("default test handler") && w.contains("another app")),
        "{got:?}"
    );
}
