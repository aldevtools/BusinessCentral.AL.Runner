//! ADR-062 stage 3: AccessByPermission on page controls under a restricted session. Values: corpus
//! `tests/al-language/permissions/lt-y19-perm-probes/Y24PermExecute.al` Y24_P21_AccessByPermissionPage (BC 27.5, 28.5, 29.0): a field
//! guarded by `tabledata T = R` or `= I` and an action guarded by `codeunit C = X` are left off the page when the session does not
//! grant the permission (`The field with ID = n is not found on the page.`), and are there when it does.

use super::permission_writes::run_main;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, RunOptions};

const MAIN: &str = r#"
table 51520 "AB Row"
{
    fields { field(1; Id; Integer) { } field(2; Val; Integer) { } field(3; Nm; Text[20]) { } }
    keys { key(PK; Id) { Clustered = true; } }
}
table 51521 "AB Child"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}
codeunit 51520 "AB Target" { procedure Ping() begin end; }

page 51520 "AB Card"
{
    PageType = Card;
    SourceTable = "AB Row";
    UsageCategory = None;
    layout
    {
        area(Content)
        {
            field(PlainCtl; Rec.Id) { ApplicationArea = All; }
            field(GuardRCtl; Rec.Val) { ApplicationArea = All; AccessByPermission = tabledata "AB Child" = R; }
            field(GuardICtl; Rec.Nm) { ApplicationArea = All; AccessByPermission = tabledata "AB Child" = I; }
        }
    }
    actions
    {
        area(Processing)
        {
            action(PlainAct) { ApplicationArea = All; trigger OnAction() begin end; }
            action(GuardXAct)
            {
                ApplicationArea = All;
                AccessByPermission = codeunit "AB Target" = X;
                trigger OnAction() begin end;
            }
        }
    }
}

page 51521 "AB Odd Card"
{
    PageType = Card;
    SourceTable = "AB Row";
    UsageCategory = None;
    layout
    {
        area(Content)
        {
            field(PlainCtl; Rec.Id) { ApplicationArea = All; }
            field(GuardMCtl; Rec.Val) { ApplicationArea = All; AccessByPermission = tabledata "AB Child" = M; }
        }
    }
}

permissionset 51520 "AB Row Only" { Assignable = true; Permissions = tabledata "AB Row" = RIMD; }
permissionset 51521 "AB Grant" { Assignable = true; Permissions = tabledata "AB Row" = RIMD, tabledata "AB Child" = RI; }
permissionset 51522 "AB Exact NoX" { Assignable = true; Permissions = tabledata "AB Row" = RIMD, page "AB Card" = X, page "AB Odd Card" = X; }
permissionset 51523 "AB Exact X" { Assignable = true; Permissions = tabledata "AB Row" = RIMD, page "AB Card" = X, codeunit "AB Target" = X; }

codeunit 51521 "AB Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Mock: Codeunit "Permissions Mock";

    local procedure Seed()
    var
        Row: Record "AB Row";
    begin
        Row.DeleteAll();
        Row.Id := 1; Row.Val := 10; Row.Nm := 'a'; Row.Insert();
    end;

    local procedure Outcome1(var P: TestPage "AB Card"; What: Text): Text
    var
        R: Text;
    begin
        if TryControl(P, What, R) then
            exit(R);
        R := 'error: ' + Norm(GetLastErrorText());
        ClearLastError();
        exit(R);
    end;

    [TryFunction]
    local procedure TryControl(var P: TestPage "AB Card"; What: Text; var R: Text)
    begin
        case What of
            'plain': R := P.PlainCtl.Value();
            'r value': R := P.GuardRCtl.Value();
            'r visible': R := Format(P.GuardRCtl.Visible());
            'i visible': R := Format(P.GuardICtl.Visible());
            'x visible': R := Format(P.GuardXAct.Visible());
            'x enabled': R := Format(P.GuardXAct.Enabled());
        end;
    end;

    local procedure Norm(T: Text): Text
    var
        Out: Text;
        I: Integer;
        C: Text[1];
        InDigits: Boolean;
    begin
        for I := 1 to StrLen(T) do begin
            C := CopyStr(T, I, 1);
            if (C >= '0') and (C <= '9') then begin
                if not InDigits then
                    Out += 'n';
                InDigits := true;
            end else begin
                InDigits := false;
                Out += C;
            end;
        end;
        exit(Out);
    end;

    local procedure Facts(): Text
    var
        P: TestPage "AB Card";
        Res: Text;
    begin
        P.OpenView();
        P.First();
        Res := Outcome1(P, 'plain') + '|' + Outcome1(P, 'r value') + '|' + Outcome1(P, 'r visible') + '|' + Outcome1(P, 'i visible') + '|' +
            Outcome1(P, 'x visible') + '|' + Outcome1(P, 'x enabled');
        P.Close();
        exit(Res);
    end;

    local procedure Expect(Got: Text; Want: Text)
    begin
        if Got <> Want then
            Error('want %1, got %2', Want, Got);
    end;

    [Test]
    procedure GrantShowsTheControls()
    var
        Got: Text;
    begin
        Mock.Start();
        Seed();
        Mock.Set('AB Grant');
        Got := Facts();
        Mock.ClearAssignments();
        Mock.Stop();
        Expect(Got, '1|10|Yes|Yes|Yes|Yes');
    end;

    [Test]
    procedure NoGrantLeavesTheFieldsOff()
    var
        Got: Text;
    begin
        Mock.Start();
        Seed();
        Mock.Set('AB Row Only');
        Got := Facts();
        Mock.ClearAssignments();
        Mock.Stop();
        // Set adds All Objects: the action guarded by Execute on a codeunit stays.
        Expect(Got, '1|error: The field with ID = n is not found on the page.|error: The field with ID = n is not found on the page.|error: The field with ID = n is not found on the page.|Yes|Yes');
    end;

    [Test]
    procedure ExactSetWithoutExecuteLeavesTheActionOff()
    var
        Got: Text;
    begin
        Mock.Start();
        Seed();
        Mock.SetExactPermissionSet('AB Exact NoX');
        Got := Facts();
        Mock.ClearAssignments();
        Mock.Stop();
        Expect(Got, '1|error: The field with ID = n is not found on the page.|error: The field with ID = n is not found on the page.|error: The field with ID = n is not found on the page.|error: The action with ID = n is not found on the page.|error: The action with ID = n is not found on the page.');
    end;

    [Test]
    procedure ExactSetWithExecuteShowsTheAction()
    var
        Got: Text;
    begin
        Mock.Start();
        Seed();
        Mock.SetExactPermissionSet('AB Exact X');
        Got := Facts();
        Mock.ClearAssignments();
        Mock.Stop();
        Expect(Got, '1|error: The field with ID = n is not found on the page.|error: The field with ID = n is not found on the page.|error: The field with ID = n is not found on the page.|Yes|Yes');
    end;

    [Test]
    procedure NoSetShowsEverything()
    var
        Got: Text;
    begin
        Seed();
        Got := Facts();
        Expect(Got, '1|10|Yes|Yes|Yes|Yes');
    end;

    // Every letter is pinned (Y84PermAbp.al Y84_Abp_LettersA): a guard on Modify leaves the field off under a set without it.
    [Test]
    procedure AModifyGuardLeavesTheFieldOff()
    var
        P: TestPage "AB Odd Card";
        Got: Text;
    begin
        Mock.Start();
        Seed();
        Mock.Set('AB Row Only');
        P.OpenView();
        P.First();
        asserterror Got := P.GuardMCtl.Value();
        Got := GetLastErrorText();
        P.Close();
        Mock.ClearAssignments();
        Mock.Stop();
        Expect(Norm(Got), 'The field with ID = n is not found on the page.');
    end;
}
"#;

fn results() -> &'static CorpusRuntime {
    static RT: std::sync::OnceLock<CorpusRuntime> = std::sync::OnceLock::new();
    RT.get_or_init(|| run_main(MAIN, &RunOptions::default()))
}

fn outcome(test: &str) -> Outcome {
    results()
        .results()
        .iter()
        .find(|r| r.test == test)
        .unwrap_or_else(|| panic!("{test} did not run"))
        .outcome
        .clone()
}

#[test]
fn a_grant_shows_the_guarded_controls() {
    assert_eq!(outcome("GrantShowsTheControls"), Outcome::Pass);
}
#[test]
fn no_grant_leaves_the_guarded_fields_off() {
    assert_eq!(outcome("NoGrantLeavesTheFieldsOff"), Outcome::Pass);
}
#[test]
fn an_exact_set_without_execute_leaves_the_action_off() {
    assert_eq!(outcome("ExactSetWithoutExecuteLeavesTheActionOff"), Outcome::Pass);
}
#[test]
fn an_exact_set_with_execute_shows_the_action() {
    assert_eq!(outcome("ExactSetWithExecuteShowsTheAction"), Outcome::Pass);
}
#[test]
fn without_a_set_everything_is_shown() {
    assert_eq!(outcome("NoSetShowsEverything"), Outcome::Pass);
}
#[test]
fn a_modify_guard_leaves_the_field_off() {
    assert_eq!(outcome("AModifyGuardLeavesTheFieldOff"), Outcome::Pass);
}
