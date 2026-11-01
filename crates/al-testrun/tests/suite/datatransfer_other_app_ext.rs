//! DataTransfer.CopyFields into a table that has a table extension of another app (al-runner issue 6668): the reporter's
//! shape, a filter on and a copy into the extension's field, and a constant into it filtered on a base field
//! (record/lt-as-records/Y47DtInstallRead.al Y65_DtItemAttribute_Outcomes on writer Y65's corpus branch; assumption
//! datatransfer-other-app-extension). CopyRows into such a table is not pinned and stays refused.

use al_sema::workspace::AppSource;
use al_testrun::runtime::CorpusRuntime;
use al_testrun::{CaseId, Outcome};

const BASE_ID: &str = "11111111-1111-1111-1111-111111116668";
const MAIN_ID: &str = "22222222-2222-2222-2222-222222226668";

const BASE: &str = r#"
table 50590 "DTX Attr"
{
    fields { field(1; ID; Integer) { } field(2; Name; Text[250]) { } field(3; Blocked; Boolean) { } }
    keys { key(PK; ID) { Clustered = true; } }
}
"#;

const MAIN: &str = r#"
tableextension 50590 "DTX Attr Ext" extends "DTX Attr"
{
    fields { field(50590; "Display Name"; Text[250]) { } field(50591; Tag; Text[10]) { } }
}
codeunit 50590 "DTX Install"
{
    Subtype = Install;
    trigger OnInstallAppPerCompany()
    var A: Record "DTX Attr"; DT: DataTransfer; DT2: DataTransfer;
    begin
        A.ID := 1; A.Name := 'Colour'; A.Insert();
        A.ID := 2; A.Name := 'Size'; A."Display Name" := 'keep'; A.Insert();
        A.ID := 3; A.Name := 'Old'; A.Blocked := true; A.Insert();
        DT.SetTables(Database::"DTX Attr", Database::"DTX Attr");
        DT.AddSourceFilter(A.FieldNo("Display Name"), '=%1', '');
        DT.AddFieldValue(A.FieldNo(Name), A.FieldNo("Display Name"));
        DT.CopyFields();
        DT2.SetTables(Database::"DTX Attr", Database::"DTX Attr");
        DT2.AddSourceFilter(A.FieldNo(Blocked), '=%1', false);
        DT2.AddConstantValue('t', A.FieldNo(Tag));
        DT2.CopyFields();
    end;
}
codeunit 50591 "DTX Test"
{
    Subtype = Test;
    [Test]
    procedure Copied()
    var A: Record "DTX Attr"; S: Text;
    begin
        if A.FindSet() then repeat S += Format(A.ID) + ':' + A."Display Name" + '/' + A.Tag + ';'; until A.Next() = 0;
        if S <> '1:Colour/t;2:keep/t;3:keep/;' then Error('%1', S);
    end;
}
"#;

const MAIN_ROWS: &str = r#"
tableextension 50590 "DTX Attr Ext" extends "DTX Attr"
{
    fields { field(50590; "Display Name"; Text[250]) { } }
}
table 50592 "DTX Src"
{
    fields { field(1; ID; Integer) { } field(2; Name; Text[50]) { } }
    keys { key(PK; ID) { Clustered = true; } }
}
codeunit 50590 "DTX Install"
{
    Subtype = Install;
    trigger OnInstallAppPerCompany()
    var S: Record "DTX Src"; A: Record "DTX Attr"; DT: DataTransfer;
    begin
        S.ID := 1; S.Name := 'x'; S.Insert();
        DT.SetTables(Database::"DTX Src", Database::"DTX Attr");
        DT.AddFieldValue(S.FieldNo(ID), A.FieldNo(ID));
        DT.CopyRows();
    end;
}
codeunit 50591 "DTX Test"
{
    Subtype = Test;
    [Test]
    procedure Copied()
    var A: Record "DTX Attr";
    begin
        A.FindFirst();
    end;
}
"#;

fn outcome(main: &str) -> Outcome {
    let apps = vec![
        AppSource {
            id: BASE_ID.into(),
            name: "base".into(),
            dependencies: vec![],
            files: vec![("base.al".into(), BASE.into())],
        },
        AppSource {
            id: MAIN_ID.into(),
            name: "main".into(),
            dependencies: vec![BASE_ID.into()],
            files: vec![("main.al".into(), main.into())],
        },
    ];
    let kinds = vec![al_sema::AppKind::Workspace; apps.len()];
    let rt = CorpusRuntime::run_for_apps(apps, kinds, &[], &Default::default(), None, Default::default());
    rt.first_outcome(&CaseId {
        codeunit: 50591,
        name: "Copied".into(),
    })
}

#[test]
fn copy_fields_into_a_table_with_an_extension_of_another_app_runs() {
    let o = outcome(MAIN);
    assert_eq!(o, Outcome::Pass, "{o:?}");
}

#[test]
fn copy_rows_into_a_table_with_an_extension_of_another_app_stays_refused() {
    let o = outcome(MAIN_ROWS);
    assert!(matches!(o, Outcome::NotRun(_)), "{o:?}");
}
