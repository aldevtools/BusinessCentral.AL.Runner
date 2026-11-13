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

/// Runs test codeunit `codeunit` of the last app in `apps` (name, source). Each app depends on the ones before it and is
/// a workspace app, as the corpus apps are (a dependency app's handler codeunit is compiled on demand only).
fn run_named_apps(apps: &[(&str, String)], codeunit: &str) -> Vec<(String, Outcome)> {
    use al_sema::workspace::{AppKind, AppSource};
    let mut sources = Vec::new();
    let mut kinds = Vec::new();
    let mut ids: Vec<String> = Vec::new();
    for (i, (name, src)) in apps.iter().enumerate() {
        let id = format!("app{i}");
        sources.push(AppSource {
            id: id.clone(),
            name: name.to_string(),
            dependencies: ids.clone(),
            files: vec![(format!("{id}.al"), src.clone())],
        });
        kinds.push(AppKind::Workspace);
        ids.push(id);
    }
    let (ws, l) = al_sema::lower::compile_apps(sources, kinds, &Default::default());
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let oi = l.program.objects.iter().position(|o| o.name == codeunit).unwrap();
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().map(|r| r.to_string())).collect())
        .collect();
    let engine = al_vm::Engine::new(l.program.clone(), blocked);
    al_test_cli::runtime::run_codeunit(&engine, &l, oi)
}

fn default_ext(ext_id: u32, handler_name: &str) -> String {
    format!(
        "enumextension {ext_id} \"{handler_name} Ext\" extends DefaultTestHandler\n{{\n    value({ext_id}; \"{handler_name} Ext\") {{ Implementation = ITestHandler = \"{handler_name}\"; }}\n}}\n"
    )
}

const READER: &str = r#"
codeunit 50140 T
{
    Subtype = Test;

    [Test]
    procedure A_Plain()
    var
        Log: Codeunit "TH Log";
    begin
        Log.Add('body');
    end;

    [Test]
    procedure Z_Observe()
    var
        Log: Codeunit "TH Log";
    begin
        if Log.Take() <> EXPECTED then
            Error('log: %1', Log.Take());
    end;
}
"#;

// BiTestHandlerCrossApp.al:9-14 (codeunits 81646, 81647, al-runner 7181): the default handlers of every installed app run for
// a test codeunit of any app, in every callback. Order seen there: the apps that hold no tests and are no dependency
// (XH Handler D, then C), then the dependency's (A), then the test app's own default handler. All of that order also
// follows app name descending and enum value id descending, so a test of one of the two alone is not pinned (below).
#[test]
fn default_handlers_of_other_apps_run_in_the_pinned_order() {
    let lib = format!(
        "{SYSTEM}\n{}",
        handler(50112, "TH Lib Handler", "HA") + &default_ext(50112, "TH Lib Handler")
    );
    let c = handler(50113, "TH C Handler", "HC") + &default_ext(50113, "TH C Handler");
    let d = handler(50114, "TH D Handler", "HD") + &default_ext(50114, "TH D Handler");
    // The test app: its own default handler has the lowest value id and the lowest app name.
    let own = handler(50111, "TH Default Handler", "D") + &default_ext(50111, "TH Default Handler");
    let expected = "'HD.bCU(||n=1);HC.bCU(||n=1);HA.bCU(||n=1);D.bCU(||n=1);HD.bP(A_Plain||n=2);HC.bP(A_Plain||n=2);HA.bP(A_Plain||n=2);D.bP(A_Plain||n=2);body;HD.aP(A_Plain||n=3|Yes);HC.aP(A_Plain||n=3|Yes);HA.aP(A_Plain||n=3|Yes);D.aP(A_Plain||n=3|Yes);HD.bP(Z_Observe||n=4);HC.bP(Z_Observe||n=4);HA.bP(Z_Observe||n=4);D.bP(Z_Observe||n=4);'";
    let got = run_named_apps(
        &[
            ("X Lib", lib),
            ("Y Handler C", c),
            ("Z Handler D", d),
            ("A Tests", format!("{own}{}", READER.replace("EXPECTED", expected))),
        ],
        "T",
    );
    assert_all_pass(&got, &["A_Plain", "Z_Observe"]);
}

// The two rules that fit the pinned order (app name descending, value id descending) disagree here, so the order is not pinned
// and the test codeunit is refused, not run in a guessed order.
#[test]
fn default_handlers_of_two_apps_whose_order_rules_disagree_are_refused() {
    let lib = format!(
        "{SYSTEM}\n{}",
        handler(50112, "TH Lib Handler", "HA") + &default_ext(50112, "TH Lib Handler")
    );
    let other = handler(50113, "TH B Handler", "HB") + &default_ext(50100, "TH B Handler");
    let got = run_named_apps(
        &[
            ("A Lib", lib),
            ("B Other", other),
            ("Tests", READER.replace("EXPECTED", "''")),
        ],
        "T",
    );
    assert!(
        matches!(got.first(), Some((_, Outcome::NotRun(w))) if w.contains("order of the default test handlers is not pinned")),
        "{got:?}"
    );
}

// Two default handlers of one app: their order is not pinned either (BiTestHandlerCrossApp.al has one per app).
#[test]
fn two_default_handlers_in_one_app_are_refused() {
    let both = format!(
        "{SYSTEM}\n{}{}{}{}",
        handler(50112, "TH One", "H1"),
        default_ext(50112, "TH One"),
        handler(50113, "TH Two", "H2"),
        default_ext(50113, "TH Two")
    );
    let got = run_named_apps(&[("Lib", both), ("Tests", READER.replace("EXPECTED", "''"))], "T");
    assert!(
        matches!(got.first(), Some((_, Outcome::NotRun(w))) if w.contains("order of the default test handlers is not pinned")),
        "{got:?}"
    );
}

/// The tests of `codeunit` and the assumptions each one hit.
fn hits_of(objects: &str, codeunit: &str) -> Vec<(String, Vec<al_testrun::assumptions::Assumption>)> {
    let l = al_testkit::compile(&format!("{SYSTEM}\n{objects}"));
    let engine = al_vm::Engine::new(l.program.clone(), al_testkit::blocked(&l));
    let oi = l.program.objects.iter().position(|o| o.name == codeunit).unwrap();
    let (mut stacks, mut hits) = (Vec::new(), Vec::new());
    let out = al_testrun::runtime::run_codeunit_full(
        &engine,
        &l,
        oi,
        None,
        al_testrun::isolation::TestIsolation::Codeunit,
        None,
        &mut stacks,
        &mut Vec::new(),
        &mut hits,
        None,
    );
    assert!(out.iter().all(|(_, o)| *o == Outcome::Pass), "{out:?}");
    out.into_iter()
        .map(|(n, _)| {
            let h = hits
                .iter()
                .find(|(t, _)| *t == n)
                .map(|(_, h)| h.clone())
                .unwrap_or_default();
            (n, h)
        })
        .collect()
}

/// A default handler that reads Success in OnBeforeTestCodeunitRun, or in OnBeforeTestProcedureRun of the procedure `T2` only.
fn success_reader(in_codeunit_callback: bool) -> String {
    let (cu, proc_) = if in_codeunit_callback {
        ("Log.Add(Format(Context.Success()));", "")
    } else {
        (
            "",
            "if Context.ProcedureName() = 'T2' then Log.Add(Format(Context.Success()));",
        )
    };
    format!(
        r#"
codeunit 50150 "TH Reader" implements ITestHandler
{{
    procedure OnBeforeTestCodeunitRun(Context: TestHandlerContext) var Log: Codeunit "TH Log"; begin {cu} end;
    procedure OnAfterTestCodeunitRun(Context: TestHandlerContext) begin end;
    procedure OnBeforeTestProcedureRun(Context: TestHandlerContext) var Log: Codeunit "TH Log"; begin {proc_} end;
    procedure OnAfterTestProcedureRun(Context: TestHandlerContext) begin end;
    procedure OnBeforeTestCaseRun(Context: TestHandlerContext) begin end;
    procedure OnAfterTestCaseRun(Context: TestHandlerContext) begin end;
}}
{}
codeunit 50151 T
{{
    Subtype = Test;
    [Test] procedure T1() begin end;
    [Test] procedure T2() begin end;
    [Test] procedure T3() begin end;
}}
"#,
        default_ext(50152, "TH Reader")
    )
}

// Review of PR 7840, MED 3: the read of Success in a Before callback of a default handler rests on assumption
// test-handler-default-before-success. The handler instance lives for the whole codeunit, so the value can reach every later
// test: every test from the read on is flagged, a read in OnBeforeTestCodeunitRun flags them all.
#[test]
fn a_default_handler_reading_success_in_a_before_callback_flags_every_test_it_can_reach() {
    use al_testrun::assumptions::Assumption::TestHandlerDefaultBeforeSuccess as A;
    let flagged = |hits: &[(String, Vec<al_testrun::assumptions::Assumption>)]| -> Vec<String> {
        hits.iter()
            .filter(|(_, h)| h.contains(&A))
            .map(|(n, _)| n.clone())
            .collect()
    };
    assert_eq!(flagged(&hits_of(&success_reader(true), "T")), ["T1", "T2", "T3"]);
    assert_eq!(flagged(&hits_of(&success_reader(false), "T")), ["T2", "T3"]);
}
