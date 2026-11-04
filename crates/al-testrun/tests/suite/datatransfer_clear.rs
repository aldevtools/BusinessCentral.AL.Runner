//! Clear of a DataTransfer variable inside install code, and SetTables twice on one variable without Clear
//! (install-upgrade/AsDtInstaller.al C1-C3, read back by AsDtInstallRead.al Install_Clear_BetweenSetTables_CopyRows,
//! Install_SetTablesTwice_NoClear_CopyRows, Install_Clear_ThenCopyRowsWithoutSetTables).

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const SRC: &str = r#"
table 50590 "DTC Src" { fields { field(1; "Code"; Code[20]) { } field(2; Amount; Integer) { } field(3; Name; Text[30]) { } } keys { key(PK; "Code") { Clustered = true; } } }
table 50591 "DTC First" { fields { field(1; "Code"; Code[20]) { } field(2; Amount; Integer) { } field(3; Name; Text[30]) { } field(4; Note; Text[30]) { } } keys { key(PK; "Code") { Clustered = true; } } }
table 50592 "DTC Second" { fields { field(1; "Code"; Code[20]) { } field(2; Amount; Integer) { } field(3; Name; Text[30]) { } field(4; Note; Text[30]) { } } keys { key(PK; "Code") { Clustered = true; } } }
table 50593 "DTC Twice First" { fields { field(1; "Code"; Code[20]) { } field(2; Amount; Integer) { } field(3; Name; Text[30]) { } field(4; Note; Text[30]) { } } keys { key(PK; "Code") { Clustered = true; } } }
table 50594 "DTC Twice Second" { fields { field(1; "Code"; Code[20]) { } field(2; Amount; Integer) { } field(3; Name; Text[30]) { } field(4; Note; Text[30]) { } } keys { key(PK; "Code") { Clustered = true; } } }
table 50595 "DTC Only" { fields { field(1; "Code"; Code[20]) { } field(2; Amount; Integer) { } field(3; Name; Text[30]) { } field(4; Note; Text[30]) { } } keys { key(PK; "Code") { Clustered = true; } } }
table 50596 "DTC Obs" { fields { field(1; "Code"; Code[20]) { } field(2; Value; Text[250]) { } } keys { key(PK; "Code") { Clustered = true; } } }
codeunit 50590 "DTC Install"
{
    Subtype = Install;
    trigger OnInstallAppPerCompany()
    var S: Record "DTC Src";
    begin
        S."Code" := 'A'; S.Amount := 5; S.Name := 'alpha'; S.Insert();
        S."Code" := 'B'; S.Amount := 15; S.Name := 'beta'; S.Insert();
        Save('C1', Outcome(TryClearBetween()));
        Save('C2', Outcome(TryTwice()));
        Save('C3', Outcome(TryClearThenCopy()));
    end;
    local procedure Outcome(Ok: Boolean): Text
    begin
        if Ok then exit('ok');
        exit(GetLastErrorText() + '|' + GetLastErrorCode());
    end;
    local procedure Save(C: Code[20]; V: Text)
    var O: Record "DTC Obs";
    begin
        O."Code" := C; O.Value := V; O.Insert();
    end;
    [TryFunction]
    local procedure TryClearBetween()
    var S: Record "DTC Src"; D: Record "DTC Second"; DT: DataTransfer;
    begin
        DT.SetTables(Database::"DTC Src", Database::"DTC First");
        DT.AddSourceFilter(S.FieldNo(Amount), '>%1', 10);
        DT.AddFieldValue(S.FieldNo("Code"), D.FieldNo("Code"));
        DT.AddFieldValue(S.FieldNo(Amount), D.FieldNo(Amount));
        DT.AddConstantValue('first', D.FieldNo(Note));
        Clear(DT);
        DT.SetTables(Database::"DTC Src", Database::"DTC Second");
        DT.AddFieldValue(S.FieldNo("Code"), D.FieldNo("Code"));
        DT.AddFieldValue(S.FieldNo(Name), D.FieldNo(Name));
        DT.CopyRows();
    end;
    [TryFunction]
    local procedure TryTwice()
    var S: Record "DTC Src"; D: Record "DTC Twice Second"; DT: DataTransfer;
    begin
        DT.SetTables(Database::"DTC Src", Database::"DTC Twice First");
        DT.AddSourceFilter(S.FieldNo(Amount), '>%1', 10);
        DT.AddFieldValue(S.FieldNo("Code"), D.FieldNo("Code"));
        DT.AddFieldValue(S.FieldNo(Amount), D.FieldNo(Amount));
        DT.AddConstantValue('first', D.FieldNo(Note));
        DT.SetTables(Database::"DTC Src", Database::"DTC Twice Second");
        DT.AddFieldValue(S.FieldNo("Code"), D.FieldNo("Code"));
        DT.AddFieldValue(S.FieldNo(Name), D.FieldNo(Name));
        DT.CopyRows();
    end;
    [TryFunction]
    local procedure TryClearThenCopy()
    var S: Record "DTC Src"; D: Record "DTC Only"; DT: DataTransfer;
    begin
        DT.SetTables(Database::"DTC Src", Database::"DTC Only");
        DT.AddFieldValue(S.FieldNo("Code"), D.FieldNo("Code"));
        Clear(DT);
        DT.CopyRows();
    end;
}
codeunit 50597 "DTC Test"
{
    Subtype = Test;
    [Test]
    procedure TheInstallRan()
    var O: Record "DTC Obs"; F: Record "DTC First"; D: Record "DTC Second"; T1: Record "DTC Twice First"; T2: Record "DTC Twice Second"; Q: Record "DTC Only"; R: Text;
    begin
        if O.FindSet() then repeat R += O."Code" + '=' + O.Value + ';'; until O.Next() = 0;
        if D.FindSet() then repeat R += D."Code" + '|' + Format(D.Amount) + '|' + D.Name + '|' + D.Note + ';'; until D.Next() = 0;
        R += Format(F.Count()) + Format(T1.Count()) + Format(T2.Count()) + Format(Q.Count());
        if R <> 'C1=ok;C2=Not possible to add the same field as destionation field twice.|DB:Ok;C3=SetTables must first be called before calling other methods on DataTransfer.|DB:Ok;A|0|alpha|;B|0|beta|;0000' then Error('%1', R);
    end;
}
"#;

// AsDtInstallRead.al Install_Clear_BetweenSetTables_CopyRows (C1): Clear drops the filter, the mappings and the constant
// staged before it. Install_SetTablesTwice_NoClear_CopyRows (C2): a second SetTables keeps the field mappings, so mapping
// Code again raises. Install_Clear_ThenCopyRowsWithoutSetTables (C3): after Clear, CopyRows asks for SetTables.
#[test]
fn clear_of_a_datatransfer_in_install_code() {
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
        codeunit: 50597,
        name: "TheInstallRan".into(),
    });
    assert_eq!(o, Outcome::Pass, "{o:?}");
}
