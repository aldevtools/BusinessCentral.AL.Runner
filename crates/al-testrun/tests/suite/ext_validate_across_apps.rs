//! OnBeforeValidate and OnAfterValidate of table extensions in several apps run app by app (al-runner issue 6486). The order
//! is pinned by tableextension/lt-bw-cross-app/Y20CrossAppValidateOrder.al:44 (corpus pin 92031e20): an app before the apps
//! that depend on it, and two apps that do not depend on each other in the order BC visits the apps (B before D: the lower app
//! id, as for event subscribers in `dispatch.rs`).

use al_sema::workspace::AppSource;
use al_testrun::runtime::CorpusRuntime;
use al_testrun::{CaseId, Outcome};

const A_ID: &str = "11111111-1111-1111-1111-111111111111";
const B_ID: &str = "22222222-2222-2222-2222-222222222222";
const D_ID: &str = "33333333-3333-3333-3333-333333333333";
const MAIN_ID: &str = "44444444-4444-4444-4444-444444444444";

const A: &str = r#"
table 50540 "EVA T"
{
    fields { field(1; K; Code[10]) { } field(2; F; Code[10]) { trigger OnValidate() begin Log += 'table;'; end; } field(3; Log; Text[250]) { } }
    keys { key(PK; K) { Clustered = true; } }
}
tableextension 50541 "EVA A" extends "EVA T"
{
    fields { modify(F) { trigger OnBeforeValidate() begin Log += 'a-before;'; end; trigger OnAfterValidate() begin Log += 'a-after;'; end; } }
}
"#;

fn ext(id: u32, tag: &str) -> String {
    format!(
        r#"tableextension {id} "EVA {tag}" extends "EVA T"
{{
    fields {{ modify(F) {{ trigger OnBeforeValidate() begin Log += '{tag}-before;'; end; trigger OnAfterValidate() begin Log += '{tag}-after;'; end; }} }}
}}"#
    )
}

const TEST: &str = r#"
codeunit 64001 T
{
    Subtype = Test;
    [Test]
    procedure Order()
    var R: Record "EVA T";
    begin
        R.Validate(F, 'X');
        if R.Log <> 'a-before;b-before;d-before;main-before;table;a-after;b-after;d-after;main-after;' then Error('order %1', R.Log);
    end;
}
"#;

fn app(id: &str, deps: &[&str], src: String) -> AppSource {
    AppSource {
        id: id.into(),
        name: format!("app {}", &id[..1]),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![(format!("{}.al", &id[..1]), src)],
    }
}

fn outcome(apps: Vec<AppSource>) -> Outcome {
    let kinds = vec![al_sema::AppKind::Workspace; apps.len()];
    let rt = CorpusRuntime::run_for_apps(apps, kinds, &[], &Default::default(), None, Default::default());
    rt.first_outcome(&CaseId {
        codeunit: 64001,
        name: "Order".into(),
    })
}

/// B and D both depend on A only; the main app depends on all three. B (lower app id) runs before D, listed in either order.
#[test]
fn apps_that_do_not_depend_on_each_other_run_in_app_id_order() {
    for swap in [false, true] {
        let mut apps = vec![
            app(A_ID, &[], A.into()),
            app(B_ID, &[A_ID], ext(50542, "b")),
            app(D_ID, &[A_ID], ext(50543, "d")),
            app(MAIN_ID, &[A_ID, B_ID, D_ID], format!("{}\n{TEST}", ext(50544, "main"))),
        ];
        if swap {
            apps.swap(1, 2);
        }
        assert_eq!(outcome(apps), Outcome::Pass, "swap {swap}");
    }
}

/// The reporter's shape (al-runner issue 6486): the table and one extension in one app, a second extension in an app that
/// depends on it. The extension of the app depended on runs first.
#[test]
fn an_app_runs_before_the_app_that_depends_on_it() {
    const MID: &str = r#"
table 50540 "EVA T"
{
    fields { field(1; K; Code[10]) { } field(2; F; Code[10]) { } field(3; Log; Text[100]) { } }
    keys { key(PK; K) { Clustered = true; } }
}
tableextension 50541 "EVA Mid" extends "EVA T"
{
    fields { modify(F) { trigger OnAfterValidate() begin Log += 'mid;'; end; } }
}
"#;
    const TOP: &str = r#"
tableextension 50542 "EVA Top" extends "EVA T"
{
    fields { modify(F) { trigger OnAfterValidate() begin Log += 'top;'; end; } }
}
codeunit 64001 T
{
    Subtype = Test;
    [Test]
    procedure Order()
    var R: Record "EVA T";
    begin
        R.Validate(F, 'X');
        if R.Log <> 'mid;top;' then Error('order %1', R.Log);
    end;
}
"#;
    let apps = vec![app(MAIN_ID, &[A_ID], TOP.into()), app(A_ID, &[], MID.into())];
    assert_eq!(outcome(apps), Outcome::Pass);
}
