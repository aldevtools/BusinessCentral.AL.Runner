//! DataTransfer inside an install codeunit (al-vm `datatransfer`): the reporter's same-table CopyFields with a source filter
//! (al-runner issue 6586) and a CopyRows between two tables, as record/lt-as-records/AsDtInstaller.al S1 and S3 pin.

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
table 50583 "DTI Ext" { fields { field(1; "Entry No."; Integer) { } field(2; Name; Text[50]) { } } keys { key(PK; "Entry No.") { Clustered = true; } } }
table 50585 "DTI Ext Src" { fields { field(1; "Entry No."; Integer) { } } keys { key(PK; "Entry No.") { Clustered = true; } } }
tableextension 50583 "DTI Ext Ext" extends "DTI Ext" { fields { field(50; Extra; Text[30]) { InitValue = 'x'; } } }
codeunit 50583 "DTI Ext Install"
{
    Subtype = Install;
    trigger OnInstallAppPerCompany()
    var E: Record "DTI Ext Src"; X: Record "DTI Ext"; DT: DataTransfer;
    begin
        E."Entry No." := 1; E.Insert();
        DT.SetTables(Database::"DTI Ext Src", Database::"DTI Ext");
        DT.AddFieldValue(E.FieldNo("Entry No."), X.FieldNo("Entry No."));
        DT.CopyRows();
    end;
}
codeunit 50584 "DTI Ext Test"
{
    Subtype = Test;
    [Test]
    procedure TheInstallRan()
    var E: Record "DTI Ext";
    begin
        E.FindFirst();
    end;
}
"#;

// What a CopyRows does with the code of a table extension of the destination is not pinned (record/lt-as-records/Y47DtInstall.al
// has no table extension): the copy is refused. CopyFields: datatransfer_other_app_ext.rs.
#[test]
fn datatransfer_into_a_table_with_an_extension_is_refused() {
    let rt = CorpusRuntime::run_suites_with(
        vec![AppSource {
            id: "main".into(),
            name: "main".into(),
            dependencies: vec![],
            files: vec![("main.al".into(), EXT_SRC.into())],
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
        codeunit: 50584,
        name: "TheInstallRan".into(),
    });
    assert!(
        matches!(&o, Outcome::NotRun(r) if r.contains("table extension")),
        "{o:?}"
    );
}
