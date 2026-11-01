//! AllObj and AllObjWithCaption hold a TableData row next to the Table row of every table, with the Table row's name, caption,
//! subtype, namespace and packages (corpus metadata/lt-all-obj-namespace/AllObjNamespace.al:143-218, writer PW: a table of the
//! app under test, a temporary one, Base Application's Customer, the platform's Company and the virtual Integer and AllObj).
//! al-runner issue 6643: an install code's OnValidate reads `AllObjWithCaption.Get(TableData, Database::Contact)`.
//!
//! Needs the `corpus` submodule, including its `.alpackages` (`System.app`).

use al_deps::package::read_package;
use al_test_cli::report::Outcome;
use al_test_cli::runtime::run_codeunit;
use al_vm::Engine;
use std::path::Path;

fn outcomes(main: &str) -> Vec<(String, Outcome)> {
    use al_sema::workspace::{AppKind, AppSource};
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus/tests/al-language/.alpackages");
    let bytes = std::fs::read(dir.join("System.app")).expect("corpus submodule not checked out");
    let system = read_package(&bytes).expect("System.app reads").1;
    let app = |id: &str, name: &str, files: Vec<(String, String)>| AppSource {
        id: id.into(),
        name: name.into(),
        dependencies: vec![],
        files,
    };
    let apps = vec![
        app("main", "main", vec![("main.al".into(), main.into())]),
        app(al_sema::workspace::PLATFORM_APP_ID, "System", system),
    ];
    let (_, l) = al_sema::lower::compile_apps_with(
        apps,
        vec![AppKind::Workspace, AppKind::Dependency],
        &Default::default(),
        None,
        Default::default(),
    );
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let engine = Engine::new(l.program.clone(), blocked);
    let oi = l
        .program
        .objects
        .iter()
        .position(|o| o.name == "T")
        .expect("test codeunit T");
    run_codeunit(&engine, &l, oi)
}

const SRC: &str = r#"
table 69995 "ATD Row" { Caption = 'ATD Caption'; fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
table 69996 "ATD Temp" { TableType = Temporary; fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
codeunit 69997 T
{
    Subtype = Test;

    local procedure Row(TableId: Integer): Text
    var A: Record AllObj; Tb: Record AllObj; W: Record AllObjWithCaption; R: Text;
    begin
        if not A.Get(A."Object Type"::TableData, TableId) then
            R := 'allobj=<no row>'
        else begin
            Tb.Get(Tb."Object Type"::Table, TableId);
            R := 'allobj=' + A."Object Name" + ' ns=[' + A."AL Namespace" + '] same=' + Format(A."App Package ID" = Tb."App Package ID") + '/' + Format(A."App Runtime Package ID" = Tb."App Runtime Package ID");
        end;
        if not W.Get(W."Object Type"::TableData, TableId) then
            R += ' caption=<no row>'
        else
            R += ' caption=' + W."Object Caption" + ' subtype=[' + W."Object Subtype" + ']';
        exit(R + ';');
    end;

    [Test]
    procedure Rows()
    var Got: Text; A: Record AllObj; W: Record AllObjWithCaption;
    begin
        Got := Row(Database::"ATD Row") + Row(Database::"ATD Temp") + Row(Database::Company) + Row(Database::Integer) + Row(69999);
        if Got <> 'allobj=ATD Row ns=[] same=Yes/Yes caption=ATD Caption subtype=[Normal];allobj=ATD Temp ns=[] same=Yes/Yes caption=ATD Temp subtype=[Temporary];allobj=Company ns=[System.Environment] same=Yes/Yes caption=Company subtype=[Normal];allobj=Integer ns=[System.Utilities] same=Yes/Yes caption=Integer subtype=[Normal];allobj=<no row> caption=<no row>;' then
            Error('rows: %1', Got);
        A.SetRange("Object ID", Database::"ATD Row");
        W.SetRange("Object ID", Database::"ATD Row");
        if (A.Count() <> 2) or (W.Count() <> 2) then
            Error('counts %1 %2', A.Count(), W.Count());
    end;
}
"#;

#[test]
fn every_table_has_a_tabledata_row_like_its_table_row() {
    let o = outcomes(SRC);
    assert_eq!(o, vec![("Rows".to_string(), Outcome::Pass)]);
}
