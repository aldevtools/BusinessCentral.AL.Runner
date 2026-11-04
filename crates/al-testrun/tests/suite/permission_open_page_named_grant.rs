//! ADR-062 stage 3, runner issue 6867: a codeunit call from OnOpenPage under a session with wildcard Execute on codeunits (`Permissions
//! Mock.Set` adds All Objects). Values: corpus `tests/al-language/permissions/permission-sets/Y90PermOpenPage.al:91-118`
//! (Y90_Open_D365BasicPlusAppSet, branch al-runtime-rs-staging-y90): `Set` plus an assigned set that grants the callee codeunit by name
//! opens the page. The reporter's repro (OpensUnderBasicPlusAppSet, OpensUnderFullAccess, OpensWithoutMock) is the test codeunit below.
//! The abort of a codeunit call from OnInit, OnAfterGetRecord and `Codeunit.Run` from OnOpenPage under an exact set: Y90PermOpenPage.al:72-88.

use super::permission_writes::run_main;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, RunOptions};

const MAIN: &str = r#"
table 51570 "OP Setup"
{
    fields { field(1; Code; Code[10]) { } field(2; Enabled; Boolean) { } }
    keys { key(PK; Code) { Clustered = true; } }
}
codeunit 51570 "OP Feature" { procedure IsEnabled(): Boolean begin exit(true); end; }
codeunit 51571 "OP Feature Sys" { procedure IsEnabled(): Boolean begin exit(true); end; }
page 51570 "OP Setup Card"
{
    PageType = Card; SourceTable = "OP Setup"; UsageCategory = None;
    layout { area(Content) { field(Enabled; Rec.Enabled) { ApplicationArea = All; } } }
    trigger OnOpenPage()
    var Feature: Codeunit "OP Feature";
    begin
        if not Feature.IsEnabled() then Error('Feature is off');
        if not Rec.Get() then Rec.Insert();
    end;
}
page 51571 "OP Setup Card Sys"
{
    PageType = Card; SourceTable = "OP Setup"; UsageCategory = None;
    layout { area(Content) { field(Enabled; Rec.Enabled) { ApplicationArea = All; } } }
    trigger OnOpenPage()
    var Feature: Codeunit "OP Feature Sys";
    begin
        if not Feature.IsEnabled() then Error('Feature is off');
    end;
}
codeunit 51573 "OP Run" { trigger OnRun() begin end; }
page 51572 "OP Init"
{
    PageType = Card; SourceTable = "OP Setup"; UsageCategory = None;
    layout { area(Content) { field(Enabled; Rec.Enabled) { ApplicationArea = All; } } }
    trigger OnInit()
    var Feature: Codeunit "OP Feature";
    begin
        if Feature.IsEnabled() then;
    end;
}
page 51573 "OP AfterGet"
{
    PageType = Card; SourceTable = "OP Setup"; UsageCategory = None;
    layout { area(Content) { field(Enabled; Rec.Enabled) { ApplicationArea = All; } } }
    trigger OnAfterGetRecord()
    var Feature: Codeunit "OP Feature";
    begin
        if Feature.IsEnabled() then;
    end;
}
page 51574 "OP Run Page"
{
    PageType = Card; SourceTable = "OP Setup"; UsageCategory = None;
    layout { area(Content) { field(Enabled; Rec.Enabled) { ApplicationArea = All; } } }
    trigger OnOpenPage()
    begin
        Codeunit.Run(Codeunit::"OP Run");
    end;
}
permissionset 51573 "OP Pages" { Assignable = true; Permissions = tabledata "OP Setup" = RIMD, page "OP Init" = X, page "OP AfterGet" = X, page "OP Run Page" = X; }
permissionset 51570 "OP App" { Assignable = true; Permissions = tabledata "OP Setup" = RIMD, table "OP Setup" = X, codeunit "OP Feature" = X, page "OP Setup Card" = X; }
permissionset 51571 "OP Basic" { Assignable = true; Permissions = tabledata "OP Setup" = R; }
permissionset 51572 "OP Sys" { Assignable = true; Permissions = tabledata "OP Setup" = RIMD, page "OP Setup Card Sys" = X, codeunit "OP Feature Sys" = X, system * = X; }

codeunit 51572 "OP Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Mock: Codeunit "Permissions Mock";

    [Test]
    procedure OpensUnderBasicPlusAppSet()
    var
        Card: TestPage "OP Setup Card";
    begin
        Mock.Start();
        Mock.Set('OP Basic');
        Mock.Assign('OP App');
        Card.OpenEdit();
        Card.Enabled.SetValue(true);
        Card.Close();
        Mock.ClearAssignments();
        Mock.Stop();
    end;

    local procedure AbortText(Which: Integer): Text
    var
        Row: Record "OP Setup";
        P1: TestPage "OP Init";
        P2: TestPage "OP AfterGet";
        P3: TestPage "OP Run Page";
        T: Text;
    begin
        if not Row.Get('A') then begin Row.Code := 'A'; Row.Insert(); end;
        Mock.Start();
        Mock.SetExactPermissionSet('OP Pages');
        case Which of
            1: asserterror P1.OpenView();
            2: asserterror P2.OpenView();
            3: asserterror P3.OpenView();
        end;
        T := GetLastErrorText() + ' | ' + GetLastErrorCode();
        ClearLastError();
        Mock.ClearAssignments();
        Mock.Stop();
        exit(T);
    end;

    [Test]
    procedure OnInitCallEndsTheOpen()
    begin
        if AbortText(1) <> 'Page OP Init has to close. | FormAbort:Permission' then Error(AbortText(1));
    end;

    [Test]
    procedure OnAfterGetRecordCallEndsTheOpen()
    begin
        if AbortText(2) <> 'Page OP AfterGet has to close. | FormAbort:Permission' then Error(AbortText(2));
    end;

    [Test]
    procedure CodeunitRunInOnOpenPageEndsTheOpen()
    begin
        if AbortText(3) <> 'Page OP Run Page has to close. | FormAbort:Permission' then Error(AbortText(3));
    end;

    [Test]
    procedure OpensWithoutMock()
    var
        Card: TestPage "OP Setup Card";
    begin
        Card.OpenEdit();
        Card.Close();
    end;

    [Test]
    procedure OpensUnderExactSetWithSystemWildcardAndCalleeByName()
    var
        Card: TestPage "OP Setup Card Sys";
    begin
        Mock.Start();
        Mock.SetExactPermissionSet('OP Sys');
        Card.OpenEdit();
        Card.Close();
        Mock.ClearAssignments();
        Mock.Stop();
    end;
}

// A refused test taints the later tests of its codeunit, so the wildcard-only case has a codeunit of its own.
codeunit 51574 "OP Tests Refused"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Mock: Codeunit "Permissions Mock";

    [Test]
    procedure OpensUnderFullAccessAlone()
    var
        Card: TestPage "OP Setup Card";
    begin
        Mock.Start();
        Mock.Set('OP Basic');
        Card.OpenEdit();
        Card.Close();
        Mock.ClearAssignments();
        Mock.Stop();
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
fn a_wildcard_session_plus_a_set_granting_the_callee_by_name_opens_the_page() {
    assert_eq!(outcome("OpensUnderBasicPlusAppSet"), Outcome::Pass);
}
#[test]
fn an_exact_set_with_system_wildcard_and_the_callee_by_name_opens_the_page() {
    assert_eq!(
        outcome("OpensUnderExactSetWithSystemWildcardAndCalleeByName"),
        Outcome::Pass
    );
}
#[test]
fn the_session_without_a_mock_opens_the_page() {
    assert_eq!(outcome("OpensWithoutMock"), Outcome::Pass);
}
/// The callee is granted by the wildcard only: a wildcard applies to every object of the installation (Y93WildScope.al:165-244) and
/// the call runs (Y93WildScope.al:588-589, q1 `Set('D365 BASIC')`). The page's own `Rec.Insert()` is then checked: `OP Basic` grants
/// only R on the table.
#[test]
fn a_wildcard_alone_granting_the_callee_runs_the_call() {
    assert_eq!(
        outcome("OpensUnderFullAccessAlone"),
        Outcome::Fail(
            "Sorry, the current permissions prevented the action. (TableData 51570 OP Setup Insert: main)".into()
        )
    );
}
#[test]
fn a_codeunit_call_from_on_init_ends_the_open_under_an_exact_set() {
    assert_eq!(outcome("OnInitCallEndsTheOpen"), Outcome::Pass);
}
#[test]
fn a_codeunit_call_from_on_after_get_record_ends_the_open_under_an_exact_set() {
    assert_eq!(outcome("OnAfterGetRecordCallEndsTheOpen"), Outcome::Pass);
}
#[test]
fn codeunit_run_from_on_open_page_ends_the_open_under_an_exact_set() {
    assert_eq!(outcome("CodeunitRunInOnOpenPageEndsTheOpen"), Outcome::Pass);
}
