//! OnBeforeValidate and OnAfterValidate of table extensions in several apps for one field (al-runner issue 6486). Measured on
//! BC by corpus writer Y20 (tableextension/lt-bw-cross-app/Y20CrossAppValidateOrder.al:36, fixtures xapp-a to xapp-d): the
//! extension of an app runs before that of an app that depends on it, before and after the field's OnValidate. Two apps that
//! do not depend on each other run in ascending app id (Y20CrossAppValidateOrder.al:44-45: B before D).

use al_builtins::native::Signal;
use al_sema::workspace::AppSource;

fn app(id: &str, deps: &[&str], src: &str) -> AppSource {
    AppSource {
        id: id.into(),
        name: format!("App {id}"),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![(format!("{id}.al"), src.into())],
    }
}

const BASE: &str = r#"
table 50540 "EVA T"
{
    fields
    {
        field(1; K; Code[10]) { }
        field(2; F; Code[10]) { trigger OnValidate() begin Log += 'table;'; end; }
        field(3; Log; Text[250]) { }
    }
    keys { key(PK; K) { Clustered = true; } }
}
tableextension 50541 "EVA A" extends "EVA T"
{
    fields { modify(F) { trigger OnBeforeValidate() begin Log += 'a-before;'; end; trigger OnAfterValidate() begin Log += 'a-after;'; end; } }
}
"#;

fn ext(id: u32, name: &str) -> String {
    format!(
        "tableextension {id} \"EVA {name}\" extends \"EVA T\" {{ fields {{ modify(F) {{ \
         trigger OnBeforeValidate() begin Log += '{name}-before;'; end; \
         trigger OnAfterValidate() begin Log += '{name}-after;'; end; }} }} }}"
    )
}

const CALLER: &str = r#"
codeunit 50543 T
{
    procedure Run(): Text
    var
        R: Record "EVA T";
    begin
        R.Validate(F, 'X');
        exit(R.Log);
    end;
}
"#;

fn run(apps: Vec<AppSource>) -> Result<String, Signal> {
    let mut b = al_testkit::Built::new(al_testkit::compile_apps(apps));
    b.run_in("T", "Run").map(|v| al_testkit::text(Ok(v)))
}

#[test]
fn extensions_of_apps_that_depend_on_each_other_run_in_dependency_order() {
    // Listed in an order that is neither the dependency order nor the id order.
    let apps = vec![
        app("m", &["a", "b", "c"], &format!("{}{CALLER}", ext(50549, "main"))),
        app("c", &["a", "b"], &ext(50548, "c")),
        app("b", &["a"], &ext(50547, "b")),
        app("a", &[], BASE),
    ];
    assert_eq!(
        run(apps).unwrap(),
        "a-before;b-before;c-before;main-before;table;a-after;b-after;c-after;main-after;"
    );
}

#[test]
fn extensions_of_apps_that_do_not_depend_on_each_other_run_in_app_id_order() {
    let apps = vec![
        app("a", &[], BASE),
        app("d", &["a"], &ext(50546, "d")),
        app("b", &["a"], &ext(50547, "b")),
        app("m", &["a", "b", "d"], CALLER),
    ];
    assert_eq!(
        run(apps).unwrap(),
        "a-before;b-before;d-before;table;a-after;b-after;d-after;"
    );
}
