//! FieldRef.GetFilter of the user-name system FlowFields (SystemCreatedByUserName 2000000005 to SystemModifiedByFullName 2000000008)
//! with no filter on them is the empty text (al-runner issue 6645; corpus writer Y64, branch al-runtime-rs-staging-y64 85d86e7b,
//! system-fields/lt-y64-username-filters/Y64UserNameFilters.al:85-108). A filter on them (SetFilter, SetRange, a view that names
//! them) is pinned there too but not supported yet: refused. SetView used to drop a condition on them as on a name of no field.

use al_sema::workspace::{AppKind, AppSource};
use al_test_cli::report::Outcome;
use al_test_cli::runtime::run_codeunit;
use al_vm::Engine;

fn run(objects: &str) -> Vec<(String, Outcome)> {
    let sources = vec![AppSource {
        id: "app0".into(),
        name: "app0".into(),
        dependencies: vec![],
        files: vec![("app0.al".into(), objects.to_string())],
    }];
    let (ws, l) = al_sema::lower::compile_apps(sources, vec![AppKind::Workspace], &Default::default());
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let oi = l.program.objects.iter().position(|o| o.name == "T").unwrap();
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    run_codeunit(&Engine::new(l.program.clone(), blocked), &l, oi)
}

const OBJECTS: &str = "
table 69670 \"UN GF Row\" { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } } keys { key(PK; Id) { Clustered = true; } } }
codeunit 69671 T {
    Subtype = Test;

    [Test]
    procedure NoFilter()
    var
        RecRef: RecordRef;
        FldRef: FieldRef;
        No: Integer;
        Res: Text;
    begin
        RecRef.Open(Database::\"UN GF Row\");
        RecRef.SetView('WHERE(Name=FILTER(a*))');
        for No := 2000000005 to 2000000008 do begin
            FldRef := RecRef.Field(No);
            Res += '[' + FldRef.GetFilter() + ']';
        end;
        if Res <> '[][][][]' then
            Error('got %1', Res);
        if RecRef.Field(2).GetFilter() <> 'a*' then
            Error('name filter %1', RecRef.Field(2).GetFilter());
    end;

    [Test]
    procedure ViewNamingTheField()
    var
        RecRef: RecordRef;
    begin
        RecRef.Open(Database::\"UN GF Row\");
        RecRef.SetView('WHERE(SystemCreatedByUserName=FILTER(<>''''))');
        Error('got %1', RecRef.Field(2000000005).GetFilter());
    end;
}";

#[test]
fn getfilter_of_a_user_name_flow_field_without_a_filter_is_empty() {
    let o = run(OBJECTS);
    assert_eq!(o[0], ("NoFilter".to_string(), Outcome::Pass), "{o:?}");
    assert!(
        matches!(&o[1], (n, Outcome::NotRun(why)) if n == "ViewNamingTheField" && why.contains("a view condition on the user-name system FlowField")),
        "{o:?}"
    );
}
