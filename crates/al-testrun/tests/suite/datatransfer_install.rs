//! DataTransfer inside an install codeunit (al-vm `datatransfer`): the reporter's same-table CopyFields with a source filter
//! (al-runner issue 6586) and a CopyRows between two tables, as install-upgrade/AsDtInstaller.al S1 and S3 pin, and both
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

// al-runner issue 7644: a join of a Guid field against a Code field that holds no Guid counted the failed comparison as "no
// match" and changed nothing. A join of two fields of different types is refused before CopyFields writes a row; a join of
// two fields of the same type runs (the test after this one).
const JOIN_SRC: &str = r#"
table 50586 "DTJ Src" { fields { field(1; Id; Guid) { } field(2; Name; Text[30]) { } field(3; Txt; Text[20]) { } } keys { key(PK; Id) { } } }
table 50587 "DTJ Dest" { fields { field(1; "Code"; Code[20]) { } field(2; Name; Text[30]) { } field(3; Txt; Text[20]) { } } keys { key(PK; "Code") { } } }
codeunit 50586 "DTJ Install"
{
    Subtype = Install;
    trigger OnInstallAppPerCompany()
    var S: Record "DTJ Src"; D: Record "DTJ Dest"; DT: DataTransfer;
    begin
        S.Id := CreateGuid(); S.Name := 'from source'; S.Txt := 'k'; S.Insert();
        D."Code" := 'X'; D.Name := 'kept'; D.Txt := 'K'; D.Insert();
        DT.SetTables(Database::"DTJ Src", Database::"DTJ Dest");
        DT.AddJoin(S.FieldNo(JOINFIELD), D.FieldNo(JOINDEST));
        DT.AddFieldValue(S.FieldNo(Name), D.FieldNo(Name));
        DT.CopyFields();
    end;
}
codeunit 50588 "DTJ Test"
{
    Subtype = Test;
    [Test]
    procedure TheJoinRan()
    var D: Record "DTJ Dest";
    begin
        D.Get('X');
        if D.Name <> 'kept' then Error('%1', D.Name);
    end;
}
"#;

fn join_outcome(source_field: &str, dest_field: &str) -> Outcome {
    let src = JOIN_SRC
        .replace("JOINFIELD", source_field)
        .replace("JOINDEST", dest_field);
    let rt = CorpusRuntime::run_suites_with(
        vec![AppSource {
            id: "main".into(),
            name: "main".into(),
            dependencies: vec![],
            files: vec![("main.al".into(), src)],
        }],
        vec![al_sema::AppKind::Workspace],
        &[],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions::default(),
    );
    rt.first_outcome(&CaseId {
        codeunit: 50588,
        name: "TheJoinRan".into(),
    })
}

#[test]
fn a_datatransfer_join_of_two_fields_of_different_types_is_refused() {
    for (s, d) in [("Id", "\"Code\""), ("Txt", "\"Code\"")] {
        let o = join_outcome(s, d);
        assert!(
            matches!(&o, Outcome::NotRun(why) if why.contains("AddJoin")),
            "{s} against {d}: {o:?}"
        );
    }
}

// A join of two fields of the same type copies as before the refusal of issue 7644 (orchestrator decision on the review of
// that change): Integer, Guid and Code[10] joins each copy the source name onto the matching destination row. The database
// match for these types is the plain value equality the Code[20] cases pin (install-upgrade/AsDtInstaller.al:163); the
// matrix of other types is queued in docs/probes-needed.md.
const SAME_TYPE_SRC: &str = r#"
table 50590 "DTS Src" { fields { field(1; "Entry No."; Integer) { } field(2; Name; Text[50]) { } field(3; Id; Guid) { } field(4; Loc; Code[10]) { } } keys { key(PK; "Entry No.") { Clustered = true; } } }
table 50591 "DTS Dst" { fields { field(1; "Entry No."; Integer) { } field(2; Name; Text[50]) { } field(3; Id; Guid) { } field(4; Loc; Code[10]) { } } keys { key(PK; "Entry No.") { Clustered = true; } } }
table 50592 "DTS Out" { fields { field(1; K; Code[20]) { } field(2; V; Text[250]) { } } keys { key(PK; K) { Clustered = true; } } }
codeunit 50590 "DTS Install"
{
    Subtype = Install;
    trigger OnInstallAppPerCompany()
    var S: Record "DTS Src"; D: Record "DTS Dst"; i: Integer;
    begin
        for i := 1 to 3 do begin
            S."Entry No." := i; S.Name := 'src' + Format(i); S.Id := CreateGuid(); S.Loc := 'L' + Format(i); S.Insert();
            D."Entry No." := i; D.Name := ''; D.Id := S.Id; D.Loc := 'L' + Format(i); D.Insert();
        end;
        Join('INT', S.FieldNo("Entry No."));
        Join('GUID', S.FieldNo(Id));
        Join('CODE10', S.FieldNo(Loc));
    end;

    local procedure Join(K: Code[20]; FieldNo: Integer)
    var S: Record "DTS Src"; D: Record "DTS Dst"; O: Record "DTS Out"; DT: DataTransfer;
    begin
        DT.SetTables(Database::"DTS Src", Database::"DTS Dst");
        DT.AddJoin(FieldNo, FieldNo);
        DT.AddFieldValue(S.FieldNo(Name), D.FieldNo(Name));
        DT.CopyFields();
        O.K := K;
        if D.FindSet() then repeat O.V += D.Name + ','; until D.Next() = 0;
        O.Insert();
        D.ModifyAll(Name, '');
    end;
}
codeunit 50593 "DTS Test"
{
    Subtype = Test;
    [Test]
    procedure TheSameTypeJoinsCopied()
    var O: Record "DTS Out"; R: Text;
    begin
        if O.FindSet() then repeat R += O.K + '=' + O.V + ' '; until O.Next() = 0;
        if R <> 'CODE10=src1,src2,src3, GUID=src1,src2,src3, INT=src1,src2,src3, ' then Error('%1', R);
    end;
}
"#;

#[test]
fn a_datatransfer_join_of_two_fields_of_the_same_type_copies() {
    let rt = CorpusRuntime::run_suites_with(
        vec![AppSource {
            id: "main".into(),
            name: "main".into(),
            dependencies: vec![],
            files: vec![("main.al".into(), SAME_TYPE_SRC.into())],
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
        codeunit: 50593,
        name: "TheSameTypeJoinsCopied".into(),
    });
    assert_eq!(o, Outcome::Pass, "{o:?}");
}
