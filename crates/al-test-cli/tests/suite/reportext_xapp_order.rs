//! Report extensions of different apps that change the run, when a dependency path orders the apps (al-runner issue 6653): the
//! base report's triggers, then the extensions' in dependency order (corpus writer Y64, branch al-runtime-rs-staging-y64 85d86e7b,
//! reports/extensions/Y64XappRunOrder.al with the fixtures in al-language-xapp-a, -b and -c: apps A <- B <- C <- the
//! test app, extension ids rising along the chain). Pinned only where the dependency order and the object id order agree. Two
//! apps with no dependency path between them: by app id where the app name sorts the same way (al-runner issue 7358,
//! reports/extensions/IndependentAppsReportExtensions.al); otherwise, and with ids against the dependency order, refused.

use al_sema::workspace::{AppKind, AppSource};
use al_test_cli::report::Outcome;
use al_test_cli::runtime::run_codeunit;
use al_vm::Engine;

/// Runs codeunit `T` of the last app; each app depends on the ones listed by index in `deps`.
fn run(apps: &[(&str, &[usize])]) -> Vec<(String, Outcome)> {
    let names: Vec<String> = (0..apps.len()).map(|i| format!("app{i}")).collect();
    run_named(apps, &names)
}

/// [`run`] with the app names `names` (the app ids stay `app0`, `app1`, ...).
fn run_named(apps: &[(&str, &[usize])], names: &[String]) -> Vec<(String, Outcome)> {
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
    run_codeunit(&Engine::new(l.program.clone(), blocked), &l, oi)
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
    let o = run(&[(&a, &[]), (&b, &[0]), (&m, &[0])]);
    assert_eq!(o, vec![("Run".to_string(), Outcome::Pass)]);
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
