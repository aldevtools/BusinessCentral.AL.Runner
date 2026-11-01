//! DataTransfer inside an install codeunit (al-vm `datatransfer`): the reporter's same-table CopyFields with a source filter
//! (al-runner issue 6586) and a CopyRows between two tables, as record/lt-as-records/AsDtInstaller.al S1 and S3 pin, and both
//! with table extensions of the same app (Y47DtInstall.al S4-S7).

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const SRC: &str = r#"
table 50580 "DTI Entry" { fields { field(1; "Entry No."; Integer) { } field(2; Name; Text[50]) { } field(3; "Display Name"; Text[50]) { } } keys { key(PK; "Entry No.") { Clustered = true; } } }
table 50581 "DTI Copy" { fields { field(1; "Entry No."; Integer) { } field(2; Name; Text[50]) { } field(3; Note; Text[30]) { } } keys { key(PK; "Entry No.") { Clustered = true; } } }
codeunit 50580 "DTI Install"
{
    Subtype = Install;
    trigger OnInstallAppPerCompany()
    var E: Record "DTI Entry"; C: Record "DTI Copy"; DT: DataTransfer; DT2: DataTransfer;
    begin
        E."Entry No." := 1; E.Name := 'alpha'; E.Insert();
        E."Entry No." := 2; E.Name := 'beta'; E."Display Name" := 'keep'; E.Insert();
        E."Entry No." := 3; E.Name := ''; E."Display Name" := ''; E.Insert();
        DT.SetTables(Database::"DTI Entry", Database::"DTI Entry");
        DT.AddSourceFilter(E.FieldNo("Display Name"), '=%1', '');
        DT.AddFieldValue(E.FieldNo(Name), E.FieldNo("Display Name"));
        DT.CopyFields();
        DT2.SetTables(Database::"DTI Entry", Database::"DTI Copy");
        DT2.AddSourceFilter(E.FieldNo("Entry No."), '>%1', 1);
        DT2.AddFieldValue(E.FieldNo("Entry No."), C.FieldNo("Entry No."));
        DT2.AddFieldValue(E.FieldNo(Name), C.FieldNo(Name));
        DT2.AddConstantValue('copied', C.FieldNo(Note));
        DT2.CopyRows();
    end;
}
codeunit 50582 "DTI Test"
{
    Subtype = Test;
    [Test]
    procedure TheInstallCopiedTheRows()
    var E: Record "DTI Entry"; C: Record "DTI Copy"; S: Text;
    begin
        if E.FindSet() then repeat S += Format(E."Entry No.") + ':' + E."Display Name" + ';'; until E.Next() = 0;
        if C.FindSet() then repeat S += Format(C."Entry No.") + '=' + C.Name + '/' + C.Note + ';'; until C.Next() = 0;
        if S <> '1:alpha;2:keep;3:;2=beta/copied;3=/copied;' then Error('%1', S);
    end;
}
"#;

#[test]
fn datatransfer_runs_inside_the_install_codeunit() {
    let rt = CorpusRuntime::run_suites_with(
        vec![AppSource {
            id: "main".into(),
            name: "main".into(),
            dependencies: vec![],
            files: vec![("main.al".into(), SRC.into())],
        }],
        vec![al_sema::AppKind::Workspace],
        &[],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions::default(),
    );
    let o = rt.first_outcome(&CaseId {
        codeunit: 50582,
        name: "TheInstallCopiedTheRows".into(),
    });
    assert_eq!(o, Outcome::Pass, "{o:?}");
}

const EXT_SRC: &str = r#"
table 50583 "DTI Src" { fields { field(1; "Code"; Code[20]) { } field(2; Amount; Integer) { } field(3; Label; Text[30]) { } } keys { key(PK; "Code") { Clustered = true; } }
    trigger OnModify() begin Error('src modify'); end; }
tableextension 50583 "DTI Src Ext" extends "DTI Src" { fields { field(50; "Ext Qty"; Integer) { } } trigger OnAfterModify() begin Error('src ext modify'); end; }
table 50584 "DTI Dest" { fields { field(1; "Code"; Code[20]) { } field(2; Amount; Integer) { trigger OnValidate() begin Error('validate'); end; } field(3; Note; Text[30]) { } } keys { key(PK; "Code") { Clustered = true; } }
    trigger OnInsert() begin Error('dest insert'); end; trigger OnModify() begin Error('dest modify'); end; }
tableextension 50584 "DTI Dest Ext" extends "DTI Dest"
{
    fields { field(51; "Ext Amount"; Integer) { InitValue = 7; } field(52; "Ext Note"; Text[30]) { InitValue = 'ext init'; } }
    trigger OnAfterInsert() begin Error('dest ext insert'); end;
}
codeunit 50583 "DTI Ext Install"
{
    Subtype = Install;
    trigger OnInstallAppPerCompany()
    var S: Record "DTI Src"; D: Record "DTI Dest"; DT: DataTransfer; DT2: DataTransfer; DT3: DataTransfer; DT4: DataTransfer;
    begin
        S."Code" := 'A'; S.Amount := 10; S.Label := 'la'; S."Ext Qty" := 1; S.Insert();
        S."Code" := 'B'; S.Amount := 20; S.Label := 'lb'; S."Ext Qty" := 2; S.Insert();
        S."Code" := 'C'; S.Amount := 30; S.Label := 'lc'; S."Ext Qty" := 3; S.Insert();
        DT.SetTables(Database::"DTI Src", Database::"DTI Dest");
        DT.AddFieldValue(S.FieldNo("Code"), D.FieldNo("Code"));
        DT.AddFieldValue(S.FieldNo("Ext Qty"), D.FieldNo(Amount));
        DT.AddFieldValue(S.FieldNo(Amount), D.FieldNo("Ext Amount"));
        DT.AddFieldValue(S.FieldNo(Label), D.FieldNo(Note));
        DT.CopyRows();
        DT2.SetTables(Database::"DTI Dest", Database::"DTI Dest");
        DT2.AddSourceFilter(D.FieldNo("Ext Amount"), '>=%1', 20);
        DT2.AddFieldValue(D.FieldNo("Ext Amount"), D.FieldNo(Amount));
        DT2.AddConstantValue('k5', D.FieldNo("Ext Note"));
        DT2.CopyFields();
        DT3.SetTables(Database::"DTI Src", Database::"DTI Src");
        DT3.AddSourceFilter(S.FieldNo(Amount), '=%1', 10);
        DT3.AddConstantValue(99, S.FieldNo("Ext Qty"));
        DT3.CopyFields();
        DT4.SetTables(Database::"DTI Src", Database::"DTI Dest");
        DT4.AddJoin(S.FieldNo("Code"), D.FieldNo("Code"));
        DT4.AddSourceFilter(S.FieldNo("Ext Qty"), '>%1', 2);
        DT4.AddFieldValue(S.FieldNo("Ext Qty"), D.FieldNo("Ext Amount"));
        DT4.CopyFields();
    end;
}
codeunit 50585 "DTI Ext Test"
{
    Subtype = Test;
    [Test]
    procedure TheInstallRan()
    var S: Record "DTI Src"; D: Record "DTI Dest"; R: Text;
    begin
        if D.FindSet() then repeat R += D."Code" + '|' + Format(D.Amount) + '|' + D.Note + '|' + Format(D."Ext Amount") + '|' + D."Ext Note" + ';'; until D.Next() = 0;
        if S.FindSet() then repeat R += S."Code" + '|' + Format(S.Amount) + '|' + S.Label + '|' + Format(S."Ext Qty") + ';'; until S.Next() = 0;
        if R <> 'A|1|la|99|ext init;B|20|lb|20|k5;C|30|lc|3|k5;A|10|la|99;B|20|lb|2;C|30|lc|3;' then Error('%1', R);
    end;
}
"#;

fn ext_outcome(apps: Vec<AppSource>, kinds: Vec<al_sema::AppKind>) -> Outcome {
    let rt = CorpusRuntime::run_suites_with(
        apps,
        kinds,
        &[],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions::default(),
    );
    rt.first_outcome(&CaseId {
        codeunit: 50585,
        name: "TheInstallRan".into(),
    })
}

// Tables with a table extension of the same app: extension fields work as source, destination, source filter and join
// target, an unmapped extension field gets its InitValue on CopyRows, and no trigger runs (record/lt-as-records/
// Y47DtInstall.al S4-S7 with Y57DtExtTables.al, read by Y47DtInstallRead.al Y57_DtExt_Rows).
#[test]
fn datatransfer_with_table_extension_fields_of_the_same_app() {
    let o = ext_outcome(
        vec![AppSource {
            id: "main".into(),
            name: "main".into(),
            dependencies: vec![],
            files: vec![("main.al".into(), EXT_SRC.into())],
        }],
        vec![al_sema::AppKind::Workspace],
    );
    assert_eq!(o, Outcome::Pass, "{o:?}");
}

// A table extension from another app is not pinned: the copy is refused.
#[test]
fn datatransfer_into_a_table_with_an_extension_of_another_app_is_refused() {
    let base = r#"
table 50586 "DTI Other" { fields { field(1; "Entry No."; Integer) { } field(2; Name; Text[50]) { } } keys { key(PK; "Entry No.") { Clustered = true; } } }
"#;
    let ext = r#"
tableextension 50586 "DTI Other Ext" extends "DTI Other" { fields { field(50; Extra; Text[30]) { InitValue = 'x'; } } }
codeunit 50586 "DTI Other Install"
{
    Subtype = Install;
    trigger OnInstallAppPerCompany()
    var E: Record "DTI Other"; DT: DataTransfer;
    begin
        E."Entry No." := 1; E.Name := 'alpha'; E.Insert();
        DT.SetTables(Database::"DTI Other", Database::"DTI Other");
        DT.AddFieldValue(E.FieldNo(Name), E.FieldNo(Name));
        DT.CopyFields();
    end;
}
codeunit 50585 "DTI Ext Test"
{
    Subtype = Test;
    [Test]
    procedure TheInstallRan()
    var E: Record "DTI Other";
    begin
        E.FindFirst();
    end;
}
"#;
    let o = ext_outcome(
        vec![
            AppSource {
                id: "base".into(),
                name: "base".into(),
                dependencies: vec![],
                files: vec![("base.al".into(), base.into())],
            },
            AppSource {
                id: "main".into(),
                name: "main".into(),
                dependencies: vec!["base".into()],
                files: vec![("main.al".into(), ext.into())],
            },
        ],
        vec![al_sema::AppKind::Dependency, al_sema::AppKind::Workspace],
    );
    assert!(
        matches!(&o, Outcome::NotRun(r) if r.contains("table extension of another app")),
        "{o:?}"
    );
}
