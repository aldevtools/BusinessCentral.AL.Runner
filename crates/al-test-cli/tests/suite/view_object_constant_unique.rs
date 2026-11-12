//! An object constant in a page's SourceTableView (`const(Database::"Name")`, crates/al-vm/src/views_cross_enum.rs
//! view_qualified_operands; pages/view-filters/ViewConstBlankCode.al:719) resolves at run time only when exactly one table
//! has the name. A name two tables share (two apps, two namespaces) is not resolved to either of them: it keeps the behavior
//! from before the resolver, the text goes on to the filter unchanged and is refused.

use al_testrun::Outcome;

const SHARED_A: &str = r#"
namespace VC.First;
table 69960 "VC Dup" { fields { field(1; K; Integer) { } } keys { key(PK; K) { } } }
"#;

const SRC: &str = r#"
namespace VC.Second;
table 69961 "VC Dup" { fields { field(1; K; Integer) { } } keys { key(PK; K) { } } }
table 69962 "VC Only" { fields { field(1; K; Integer) { } } keys { key(PK; K) { } } }
table 69963 "VC Obj" { fields { field(1; "Table ID"; Integer) { } } keys { key(PK; "Table ID") { } } }
page 69964 "VC Unique View"
{
    PageType = List; SourceTable = "VC Obj"; ApplicationArea = All; UsageCategory = None;
    SourceTableView = where("Table ID" = const(Database::"VC Only"));
    layout { area(Content) { repeater(R) { field(Id; Rec."Table ID") { } } } }
}
page 69965 "VC Shared View"
{
    PageType = List; SourceTable = "VC Obj"; ApplicationArea = All; UsageCategory = None;
    SourceTableView = where("Table ID" = const(Database::"VC Dup"));
    layout { area(Content) { repeater(R) { field(Id; Rec."Table ID") { } } } }
}
codeunit 69966 "VC Tests"
{
    Subtype = Test;
    local procedure Seed()
    var O: Record "VC Obj";
    begin
        O.DeleteAll();
        O."Table ID" := 69960; O.Insert(); O."Table ID" := 69961; O.Insert(); O."Table ID" := 69962; O.Insert();
    end;
    [Test] procedure UniqueName()
    var P: TestPage "VC Unique View";
    begin Seed(); P.OpenView(); Error('row=%1', P.Id.Value); end;
    [Test] procedure SharedName()
    var P: TestPage "VC Shared View";
    begin Seed(); P.OpenView(); Error('row=%1', P.Id.Value); end;
}
"#;

fn outcome(outcomes: &[(String, Outcome)], name: &str) -> Outcome {
    outcomes
        .iter()
        .find(|(n, _)| n == name)
        .unwrap_or_else(|| panic!("no test {name}"))
        .1
        .clone()
}

#[test]
fn a_view_object_constant_resolves_only_a_name_one_table_holds() {
    use al_sema::workspace::{AppKind, AppSource};
    let app = |id: &str, deps: Vec<String>, src: &str| AppSource {
        id: id.into(),
        name: id.into(),
        dependencies: deps,
        files: vec![(format!("{id}.al"), src.into())],
    };
    let (ws, l) = al_sema::lower::compile_apps_with(
        vec![app("first", vec![], SHARED_A), app("second", vec!["first".into()], SRC)],
        vec![AppKind::Dependency, AppKind::Workspace],
        &Default::default(),
        None,
        Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let engine = al_vm::Engine::new(l.program.clone(), al_testkit::blocked(&l));
    let oi = l
        .program
        .objects
        .iter()
        .position(|o| o.name.ends_with("VC Tests"))
        .expect("VC Tests");
    let o = al_testrun::runtime::run_codeunit(&engine, &l, oi);
    match outcome(&o, "UniqueName") {
        Outcome::Fail(why) => assert!(why.starts_with("row=69962"), "UniqueName: {why}"),
        other => panic!("UniqueName: {other:?}"),
    }
    // Not resolved to either table (a first match gave `row=69960`, the other namespace's table): the text reaches the filter
    // unchanged and is refused, as before the resolver.
    match outcome(&o, "SharedName") {
        Outcome::NotRun(why) => assert!(why.contains("is not an Integer"), "SharedName: {why}"),
        other => panic!("SharedName: {other:?}"),
    }
}
