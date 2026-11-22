//! Report extensions of different apps that change the run, when a dependency path orders the apps (al-runner issue 6653): the
//! base report's triggers, then the extensions' in dependency order (corpus writer Y64, branch al-runtime-rs-staging-y64 85d86e7b,
//! reports/extensions/Y64XappRunOrder.al with the fixtures in al-language-xapp-a, -b and -c: apps A <- B <- C <- the
//! test app, extension ids rising along the chain). Pinned only where the dependency order and the object id order agree. Two
//! apps with no dependency path between them: by app id where the app name sorts the same way (al-runner issue 7358,
//! reports/extensions/IndependentAppsReportExtensions.al), a flagged pass (assumption `report-ext-independent-app-order`); otherwise, and with ids against the dependency order, refused.

use al_builtins::assumptions::Assumption;
use al_sema::workspace::{AppKind, AppSource};
use al_test_cli::report::Outcome;
use al_test_cli::runtime::run_codeunit_full;
use al_vm::Engine;

/// Runs codeunit `T` of the last app; each app depends on the ones listed by index in `deps`.
fn run(apps: &[(&str, &[usize])]) -> Vec<(String, Outcome)> {
    let names: Vec<String> = (0..apps.len()).map(|i| format!("app{i}")).collect();
    run_named(apps, &names)
}

/// [`run`] with the app names `names` (the app ids stay `app0`, `app1`, ...).
fn run_named(apps: &[(&str, &[usize])], names: &[String]) -> Vec<(String, Outcome)> {
    run_marked(apps, names).0
}

/// The unprobed assumptions each test passed through, by test name.
pub(crate) type Hits = Vec<(String, Vec<Assumption>)>;

/// [`run_named`] that also returns the unprobed assumptions each test passed through.
pub(crate) fn run_marked(apps: &[(&str, &[usize])], names: &[String]) -> (Vec<(String, Outcome)>, Hits) {
    let ids: Vec<String> = (0..apps.len()).map(|i| format!("app{i}")).collect();
    let sources = apps
        .iter()
        .enumerate()
        .map(|(i, (src, deps))| AppSource {
            id: ids[i].clone(),
            name: names[i].clone(),
            dependencies: deps.iter().map(|&d| ids[d].clone()).collect(),
            files: vec![(format!("{}.al", ids[i]), src.to_string())],
        })
        .collect();
    let mut kinds = vec![AppKind::Dependency; apps.len()];
    *kinds.last_mut().unwrap() = AppKind::Workspace;
    let (ws, l) = al_sema::lower::compile_apps(sources, kinds, &Default::default());
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let oi = l.program.objects.iter().position(|o| o.name == "T").unwrap();
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().map(|r| r.to_string())).collect())
        .collect();
    let mut hits = Vec::new();
    let outcomes = run_codeunit_full(
        &Engine::new(l.program.clone(), blocked),
        &l,
        oi,
        None,
        al_test_cli::isolation::TestIsolation::Codeunit,
        None,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut hits,
        None,
    );
    (outcomes, hits)
}

const BASE: &str = r#"
table 62900 "XO Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
codeunit 62901 "XO Log" { SingleInstance = true; var L: Text; procedure Add(E: Text) begin L += E + ';'; end; procedure Take(): Text begin exit(L); end; }
report 62902 "XO Report"
{
    ProcessingOnly = true;
    dataset
    {
        dataitem(Row; "XO Row")
        {
            trigger OnPreDataItem() var Log: Codeunit "XO Log"; begin Log.Add('base:predataitem'); end;
            trigger OnAfterGetRecord() var Log: Codeunit "XO Log"; begin Log.Add('base:aftergetrecord'); end;
        }
    }
}
"#;

fn ext(id: u32, tag: &str) -> String {
    format!(
        r#"
reportextension {id} "XO Ext {tag}" extends "XO Report"
{{
    dataset
    {{
        modify(Row)
        {{
            trigger OnBeforePreDataItem() var Log: Codeunit "XO Log"; begin Log.Add('{tag}:beforepredataitem'); end;
            trigger OnAfterAfterGetRecord() var Log: Codeunit "XO Log"; begin Log.Add('{tag}:afteraftergetrecord'); end;
        }}
    }}
}}"#
    )
}

const TEST: &str = r#"
codeunit 62990 T
{
    Subtype = Test;
    [Test] procedure Run()
    var R: Record "XO Row"; Log: Codeunit "XO Log"; Rep: Report "XO Report"; Got: Text;
    begin
        R.Id := 1; R.Insert();
        Log.Take();
        Rep.UseRequestPage(false);
        Rep.RunModal();
        Got := Log.Take();
        if Got <> 'A:beforepredataitem;B:beforepredataitem;M:beforepredataitem;base:predataitem;base:aftergetrecord;A:afteraftergetrecord;B:afteraftergetrecord;M:afteraftergetrecord;' then
            Error('order: %1', Got);
    end;
}"#;

#[test]
fn extensions_along_a_dependency_chain_run_in_dependency_order() {
    let a = format!("{BASE}{}", ext(62910, "A"));
    let b = ext(62920, "B");
    let m = format!("{}{TEST}", ext(62930, "M"));
    let o = run(&[(&a, &[]), (&b, &[0]), (&m, &[0, 1])]);
    assert_eq!(o, vec![("Run".to_string(), Outcome::Pass)]);
}

const TEST_TWO: &str = r#"
codeunit 62990 T
{
    Subtype = Test;
    [Test] procedure Run()
    var R: Record "XO Row"; Log: Codeunit "XO Log"; Rep: Report "XO Report"; Got: Text;
    begin
        R.Id := 1; R.Insert();
        Log.Take();
        Rep.UseRequestPage(false);
        Rep.RunModal();
        Got := Log.Take();
        if Got <> 'B:beforepredataitem;M:beforepredataitem;base:predataitem;base:aftergetrecord;B:afteraftergetrecord;M:afteraftergetrecord;' then
            Error('order: %1', Got);
    end;
}"#;

#[test]
fn two_apps_without_a_path_run_in_app_id_order_when_the_names_agree() {
    let a = BASE.to_string();
    // B (app1) and M (app2) have no dependency path between them; B's extension has the higher object id.
    let b = ext(62940, "B");
    let m = format!("{}{TEST_TWO}", ext(62930, "M"));
    let names: Vec<String> = (0..3).map(|i| format!("app{i}")).collect();
    let (o, hits) = run_marked(&[(&a, &[]), (&b, &[0]), (&m, &[0])], &names);
    assert_eq!(o, vec![("Run".to_string(), Outcome::Pass)]);
    // One measured pair does not tell the rule: the pass is flagged until the corpus probe lands.
    assert_eq!(
        hits,
        vec![("Run".to_string(), vec![Assumption::ReportExtIndependentAppOrder])]
    );
}

#[test]
fn apps_without_a_path_or_ids_against_it_stay_refused() {
    let a = BASE.to_string();
    // B and M have no dependency path between them, and their app names sort against their app ids.
    let b = ext(62920, "B");
    let m = format!("{}{TEST}", ext(62930, "M"));
    let names = ["base".to_string(), "zulu".to_string(), "alpha".to_string()];
    let o = run_named(&[(&a, &[]), (&b, &[0]), (&m, &[0])], &names);
    assert!(
        matches!(&o[..], [(_, Outcome::NotRun(why))] if why.contains("not pinned")),
        "{o:?}"
    );
    // The dependent app's extension has the lower id.
    let b = ext(62940, "B");
    let m = format!("{}{TEST}", ext(62930, "M"));
    let o = run(&[(&a, &[]), (&b, &[0]), (&m, &[0, 1])]);
    assert!(
        matches!(&o[..], [(_, Outcome::NotRun(why))] if why.contains("not pinned")),
        "{o:?}"
    );
}

/// A table that declares a global variable of the report, as Base Application's General Ledger Setup declares one of
/// "Adjust Add. Reporting Currency".
const HOLDER: &str = r#"
table 62903 "XO Holder"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { } }
    var Rep: Report "XO Report";
    procedure Answer(): Integer begin exit(42); end;
    procedure RunIt() begin Rep.UseRequestPage(false); Rep.RunModal(); end;
}"#;

/// A codeunit that declares a local variable of the report in each procedure, and one that declares a global of it.
const USERS: &str = r#"
codeunit 62904 "XO Local User"
{
    procedure Answer(): Integer
    var Rep: Report "XO Report";
    begin
        exit(42);
    end;
    procedure RunIt()
    var Rep: Report "XO Report";
    begin
        Rep.UseRequestPage(false); Rep.RunModal();
    end;
}
codeunit 62905 "XO Global User"
{
    var Rep: Report "XO Report";
    procedure Answer(): Integer begin exit(42); end;
    procedure RunIt() begin Rep.UseRequestPage(false); Rep.RunModal(); end;
}"#;

/// Test codeunit `T` for `user`, the type of an object that declares a variable of the report: `OtherCodeRuns` calls its
/// `Answer`, `RunningTheReport` its `RunIt`.
fn user_tests(user: &str) -> String {
    format!(
        r#"
codeunit 62990 T
{{
    Subtype = Test;
    [Test] procedure OtherCodeRuns()
    var U: {user};
    begin
        if U.Answer() <> 42 then Error('answer');
    end;
    [Test] procedure RunningTheReport()
    var U: {user}; R: Record "XO Row";
    begin
        R.Id := 1; R.Insert();
        U.RunIt();
    end;
}}"#
    )
}

/// The reason `ext_order.rs` refuses the order of the extensions for.
const ORDER_REASON: &str = "their order across apps is not pinned";

/// Runs [`user_tests`] for `user` against "XO Report" with the extensions of [`a_refused_order_refuses_running_the_report_not_declaring_it`]
/// and asserts that only running the report is refused, for the order.
fn assert_only_the_run_is_refused(user: &str) {
    let a = format!("{BASE}{HOLDER}{USERS}{}", ext(62950, "A"));
    let b = ext(62940, "B");
    let o = run(&[(&a, &[]), (&b, &[0]), (&user_tests(user), &[0, 1])]);
    assert!(
        matches!(&o[..], [(first, Outcome::Pass), (second, Outcome::NotRun(why))]
            if first == "OtherCodeRuns" && second == "RunningTheReport" && why.contains(ORDER_REASON)),
        "{o:?}"
    );
}

/// The order of the extensions matters only when the report runs: a variable of the report is no reason to refuse the
/// other code of the object that declares it. The shape of Base Application's own reportextension 99000786 "Mfg. Adjust Add.
/// Rep. Currency" and one of a dependent app with a lower object id: ids against the dependency order, refused.
#[test]
fn a_refused_order_refuses_running_the_report_not_declaring_it() {
    assert_only_the_run_is_refused(r#"Record "XO Holder""#);
}

/// A local variable of the report refuses neither the procedure that declares it nor the other procedures of its object.
#[test]
fn a_refused_order_does_not_refuse_a_procedure_with_a_local_variable_of_the_report() {
    assert_only_the_run_is_refused(r#"Codeunit "XO Local User""#);
}

/// A codeunit's global of the report does not refuse the codeunit's other procedures.
#[test]
fn a_refused_order_does_not_refuse_a_codeunit_with_a_global_of_the_report() {
    assert_only_the_run_is_refused(r#"Codeunit "XO Global User""#);
}

/// A report with a reason of its own besides the order: `MaxIteration = 0` (al-sema `reports.rs` admits a positive number).
const REFUSED_REPORT: &str = r#"
report 62906 "XO Refused Report"
{
    ProcessingOnly = true;
    dataset { dataitem(Row; "XO Row") { MaxIteration = 0; } }
}
codeunit 62907 "XO Refused User"
{
    var Rep: Report "XO Refused Report";
    procedure Answer(): Integer begin exit(42); end;
    procedure RunIt() begin Rep.UseRequestPage(false); Rep.RunModal(); end;
}"#;

/// [`ext`] of "XO Refused Report".
fn refused_ext(id: u32, tag: &str) -> String {
    ext(id, tag)
        .replace("\"XO Report\"", "\"XO Refused Report\"")
        .replace("XO Ext", "XO Refused Ext")
}

/// Leaving the order out of the reasons a variable is refused for keeps the report's other reasons: its variable still
/// refuses the object that declares it, for that reason, and so does a run.
#[test]
fn a_refused_order_and_a_reason_of_its_own_refuse_the_variable_for_that_reason() {
    let a = format!("{BASE}{REFUSED_REPORT}{}", refused_ext(62950, "A"));
    let b = refused_ext(62940, "B");
    let o = run(&[
        (&a, &[]),
        (&b, &[0]),
        (&user_tests(r#"Codeunit "XO Refused User""#), &[0, 1]),
    ]);
    assert!(
        matches!(&o[..], [(_, Outcome::NotRun(first)), (_, Outcome::NotRun(second))]
            if [first, second].iter().all(|w| w.contains("maxiteration") && !w.contains(ORDER_REASON))),
        "{o:?}"
    );
}

/// A report with a procedure and a request page OnInit of its own, which [`init_ext`] extends with one more.
const INIT_REPORT: &str = r#"
report 62908 "XO Init Report"
{
    ProcessingOnly = true;
    dataset { dataitem(Row; "XO Row") { } }
    requestpage { trigger OnInit() var Log: Codeunit "XO Log"; begin Log.Add('base:oninit'); end; }
    procedure Answer(): Integer begin exit(42); end;
}"#;

/// A reportextension of "XO Init Report" that changes the run and adds a request page OnInit.
fn init_ext(id: u32, tag: &str) -> String {
    format!(
        r#"
reportextension {id} "XO Init Ext {tag}" extends "XO Init Report"
{{
    dataset
    {{
        modify(Row)
        {{
            trigger OnBeforePreDataItem() var Log: Codeunit "XO Log"; begin Log.Add('{tag}:beforepredataitem'); end;
        }}
    }}
    requestpage
    {{
        trigger OnInit() var Log: Codeunit "XO Log"; begin Log.Add('{tag}:oninit'); end;
    }}
}}"#
    )
}

/// A procedure call on a variable of the report, which runs OnInitReport and the request page's OnInit first, with no run.
const TEST_INIT: &str = r#"
codeunit 62990 T
{
    Subtype = Test;
    [Test] procedure CallAProcedure()
    var Rep: Report "XO Init Report";
    begin
        if Rep.Answer() <> 42 then Error('answer');
    end;
}"#;

/// The request page's OnInit runs when a procedure first uses the report (reports/request-page/
/// RequestPageOnInitFirstUse.al:104-185). When and in which order an OnInit a reportextension adds runs there is not pinned,
/// so the call is refused, also where no run follows that would refuse it: here with a refused order of the extensions.
#[test]
fn a_procedure_call_refuses_the_request_page_oninit_extensions_add_in_a_refused_order() {
    let a = format!("{BASE}{INIT_REPORT}{}", init_ext(62950, "A"));
    let b = init_ext(62940, "B");
    let o = run(&[(&a, &[]), (&b, &[0]), (TEST_INIT, &[0, 1])]);
    assert!(
        matches!(&o[..], [(_, Outcome::NotRun(why))]
            if why.contains("OnInit a reportextension adds") && why.contains("first uses")),
        "{o:?}"
    );
}

/// The same with one extension, whose order is pinned.
#[test]
fn a_procedure_call_refuses_the_request_page_oninit_one_extension_adds() {
    let a = format!("{BASE}{INIT_REPORT}");
    let b = init_ext(62950, "B");
    let o = run(&[(&a, &[]), (&b, &[0]), (TEST_INIT, &[0, 1])]);
    assert!(
        matches!(&o[..], [(_, Outcome::NotRun(why))]
            if why.contains("OnInit a reportextension adds") && why.contains("first uses")),
        "{o:?}"
    );
}
