//! ADR-062 stage 3: Execute on objects under an exact permission set. Values: corpus
//! `tests/al-language/permissions/permission-sets/Y24PermExecute.al` Y24_P20_* (BC 27.5, 28.5, 29.0): without an Execute grant
//! `Codeunit.Run`, a call on a codeunit variable and a report run fail with `(<Kind> <id> <name> Execute: <app>)` and code
//! `DB:ExecuteObjDenied`; the test codeunit's own procedures and objects made before the set was assigned are not checked.

use super::permission_writes::run_main;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, RunOptions};

const MAIN: &str = r#"
table 51510 "EX Row"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}

codeunit 51510 "EX Target"
{
    trigger OnRun() begin end;
    procedure Ping(): Text begin exit('pong'); end;
    procedure Again(): Text begin exit(Ping()); end;
}

codeunit 51511 "EX Granter"
{
    procedure CallsTarget(): Text
    var Target: Codeunit "EX Target";
    begin
        exit(Target.Ping());
    end;
}

report 51510 "EX Report"
{
    ProcessingOnly = true;
    UseRequestPage = false;
    trigger OnPreReport() begin end;
}

permissionset 51510 "EX TD" { Assignable = true; Permissions = tabledata "EX Row" = RIMD; }
permissionset 51511 "EX Target X" { Assignable = true; Permissions = tabledata "EX Row" = RIMD, codeunit "EX Target" = X; }
permissionset 51512 "EX Granter X" { Assignable = true; Permissions = tabledata "EX Row" = RIMD, codeunit "EX Granter" = X; }

codeunit 51512 "EX Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Mock: Codeunit "Permissions Mock";
        GlobalTarget: Codeunit "EX Target";

    local procedure Own(): Text
    begin
        exit('own');
    end;

    local procedure Expect(Got: Text; Want: Text)
    begin
        if Got <> Want then
            Error('want %1, got %2', Want, Got);
    end;

    local procedure RunTarget()
    begin
        Codeunit.Run(Codeunit::"EX Target");
    end;

    local procedure VarTarget(): Text
    var
        Target: Codeunit "EX Target";
    begin
        exit(Target.Ping());
    end;

    local procedure RunReport()
    var
        Rep: Report "EX Report";
    begin
        Rep.UseRequestPage(false);
        Rep.RunModal();
    end;

    local procedure ViaGranter(): Text
    var
        Granter: Codeunit "EX Granter";
    begin
        exit(Granter.CallsTarget());
    end;

    [Test]
    procedure RunDeniedWithoutExecute()
    var
        Text1: Text;
        Code1: Text;
    begin
        Mock.Start();
        Mock.SetExactPermissionSet('EX TD');
        asserterror RunTarget();
        Text1 := GetLastErrorText();
        Code1 := GetLastErrorCode();
        Mock.ClearAssignments();
        ClearLastError();
        Expect(Text1, 'Sorry, the current permissions prevented the action. (CodeUnit 51510 EX Target Execute: ' + 'main' + ')');
        Expect(Code1, 'DB:ExecuteObjDenied');
        Mock.Stop();
    end;

    [Test]
    procedure VariableCallDeniedWithoutExecute()
    var
        Code1: Text;
    begin
        Mock.Start();
        Mock.SetExactPermissionSet('EX TD');
        asserterror Expect(VarTarget(), 'pong');
        Code1 := GetLastErrorCode();
        Mock.ClearAssignments();
        ClearLastError();
        Expect(Code1, 'DB:ExecuteObjDenied');
        Mock.Stop();
    end;

    [Test]
    procedure ExecuteGrantAllowsTheCall()
    begin
        Mock.Start();
        Mock.SetExactPermissionSet('EX Target X');
        Expect(VarTarget(), 'pong');
        Codeunit.Run(Codeunit::"EX Target");
        Mock.ClearAssignments();
        Mock.Stop();
    end;

    [Test]
    procedure TheTestCodeunitsOwnProceduresAndEarlierObjectsAreFree()
    begin
        Mock.Start();
        Mock.SetExactPermissionSet('EX TD');
        Expect(Own(), 'own');
        Mock.ClearAssignments();
        Mock.SetExactPermissionSet('EX TD');
        Expect(GlobalTarget.Ping(), 'pong');
        Mock.ClearAssignments();
        Mock.Stop();
    end;

    [Test]
    procedure ExecuteOnTheCallerDoesNotCoverWhatItRuns()
    var
        Code1: Text;
    begin
        Mock.Start();
        Mock.SetExactPermissionSet('EX Granter X');
        asserterror Expect(ViaGranter(), 'pong');
        Code1 := GetLastErrorCode();
        Mock.ClearAssignments();
        ClearLastError();
        Expect(Code1, 'DB:ExecuteObjDenied');
        Mock.Stop();
    end;

    [Test]
    procedure SetAddsAllObjects()
    begin
        Mock.Start();
        Mock.Set('EX TD');
        Expect(VarTarget(), 'pong');
        RunReport();
        Mock.ClearAssignments();
        Mock.Stop();
    end;

    [Test]
    procedure ReportRunDeniedWithoutExecute()
    var
        Text1: Text;
    begin
        Mock.Start();
        Mock.SetExactPermissionSet('EX TD');
        asserterror RunReport();
        Text1 := GetLastErrorText();
        Mock.ClearAssignments();
        ClearLastError();
        if not Text1.Contains('(Report 51510 EX Report Execute: ') then
            Error('%1', Text1);
        Mock.Stop();
    end;
}
"#;

fn results() -> &'static CorpusRuntime {
    static RT: std::sync::OnceLock<CorpusRuntime> = std::sync::OnceLock::new();
    RT.get_or_init(|| run_main(MAIN, &RunOptions::default()))
}

fn passes(test: &str) {
    let o = results()
        .results()
        .iter()
        .find(|r| r.test == test)
        .unwrap_or_else(|| panic!("{test} did not run"))
        .outcome
        .clone();
    assert_eq!(o, Outcome::Pass, "{test}");
}

#[test]
fn run_is_denied_without_execute() {
    passes("RunDeniedWithoutExecute");
}
#[test]
fn a_variable_call_is_denied_without_execute() {
    passes("VariableCallDeniedWithoutExecute");
}
#[test]
fn an_execute_grant_allows_the_call() {
    passes("ExecuteGrantAllowsTheCall");
}
#[test]
fn own_procedures_and_earlier_objects_are_free() {
    passes("TheTestCodeunitsOwnProceduresAndEarlierObjectsAreFree");
}
#[test]
fn execute_on_the_caller_does_not_cover_what_it_runs() {
    passes("ExecuteOnTheCallerDoesNotCoverWhatItRuns");
}
#[test]
fn set_adds_all_objects() {
    passes("SetAddsAllObjects");
}
#[test]
fn a_report_run_is_denied_without_execute() {
    passes("ReportRunDeniedWithoutExecute");
}
