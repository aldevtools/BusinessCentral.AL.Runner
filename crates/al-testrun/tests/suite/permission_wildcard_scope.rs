//! ADR-062: wildcard grants (`codeunit * = X`, `tabledata * = RIMD`) under a lowered session. Values: corpus
//! `tests/al-language/permissions/wildcard-scope/Y93WildScope.al` (writer Y93, branch al-runtime-rs-staging-y93, Windows BC
//! 27.5 and 28.5 and Linux 28.5 and 29.0 agree):
//! - a wildcard applies to every object of the installation, whatever app declares the set (Y93WildScope.al:165-244, `xa cu|W All`,
//!   `xa td insert|W Data`);
//! - a page whose OnOpenPage calls a codeunit opens when a wildcard grants the codeunit (Y93WildScope.al:576-599, q1 `Set`, q4 an exact
//!   set of wildcards only, q5 `D365 BASIC` alone);
//! - an explicit lower grant next to a wildcard does not narrow it (Y93WildScope.al:114-140, `mixdata|narrow insert`, `mixcu|M3 explicit x`);
//! - `ExcludedPermissionSets` takes one object's permission out of a wildcard grant and leaves the others (Y93WildScope.al:142-162).

use super::permission_writes::run_main;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, RunOptions};

const MAIN: &str = r#"
dotnet
{
    assembly("Microsoft.Dynamics.Nav.NavUserAccount")
    {
        type("Microsoft.Dynamics.Nav.NavUserAccount.NavUserAccountHelper"; "NavUserAccountHelper") { }
    }
}
table 51600 "WS Row"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}
table 51601 "WS Narrow"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}
codeunit 51600 "WS Feature" { procedure IsEnabled(): Boolean begin exit(true); end; }
codeunit 51601 "WS Excluded" { procedure Ping(): Text begin exit('ping'); end; }
codeunit 51602 "WS Other" { procedure Ping(): Text begin exit('ping'); end; }
codeunit 51603 "WS Narrow Cu" { procedure Ping(): Text begin exit('ping'); end; }
page 51600 "WS Card"
{
    PageType = Card; UsageCategory = None;
    layout { area(Content) { field(Shown; Shown) { ApplicationArea = All; } } }
    var Shown: Text;
    trigger OnOpenPage()
    var Feature: Codeunit "WS Feature";
    begin
        if not Feature.IsEnabled() then Error('Feature is off');
    end;
}
page 51601 "WS Card Other App"
{
    PageType = Card; UsageCategory = None;
    layout { area(Content) { field(Shown; Shown) { ApplicationArea = All; } } }
    var Shown: Text;
    trigger OnOpenPage()
    var Writer: Codeunit "PB Writer";
    begin
        Writer.InsertRow(1);
    end;
}
permissionset 51600 "WS Basic" { Assignable = true; Permissions = tabledata "WS Row" = R; }
permissionset 51601 "WS All"
{
    Assignable = true;
    Permissions = codeunit * = X, page * = X, table * = X, tabledata * = RIMD, report * = X, query * = X, xmlport * = X, system * = X;
}
permissionset 51602 "WS Exec" { Assignable = true; Permissions = codeunit * = X, page * = X, table * = X, report * = X, query * = X, xmlport * = X, system * = X; }
permissionset 51603 "WS Ex Cu" { Assignable = true; Permissions = codeunit "WS Excluded" = X; }
permissionset 51604 "WS Ex Data" { Assignable = true; Permissions = tabledata "WS Row" = I; }
permissionset 51605 "WS Excl"
{
    Assignable = true;
    IncludedPermissionSets = "WS All";
    ExcludedPermissionSets = "WS Ex Cu", "WS Ex Data";
}
permissionset 51606 "WS Mix" { Assignable = true; Permissions = tabledata * = RIMD, tabledata "WS Narrow" = R, codeunit * = X, codeunit "WS Narrow Cu" = x; }
permissionset 51607 "WS Ind Cu" { Assignable = true; Permissions = page * = X, codeunit * = x; }

codeunit 51604 "WS Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Mock: Codeunit "Permissions Mock";

    local procedure Open(Spec: Text; Which: Integer)
    var
        Card: TestPage "WS Card";
        Other: TestPage "WS Card Other App";
    begin
        Mock.Start();
        case CopyStr(Spec, 1, 1) of
            'S': Mock.Set(CopyStr(Spec, 3, 20));
            'E': Mock.SetExactPermissionSet(CopyStr(Spec, 3, 20));
        end;
        case Which of
            1: begin Card.OpenView(); Card.Close(); end;
            2: begin Other.OpenView(); Other.Close(); end;
        end;
        Mock.ClearAssignments();
        Mock.Stop();
    end;

    [Test]
    procedure OpensUnderSetWithWildcardOnly()
    begin
        Open('S:WS Basic', 1);
    end;

    [Test]
    procedure OpensUnderExactWildcardSet()
    begin
        Open('E:WS Exec', 1);
    end;

    [Test]
    procedure OpensCallingAnotherAppsCodeunitUnderWildcards()
    begin
        Open('E:WS All', 2);
    end;

    local procedure Observe(Which: Integer) T: Text
    begin
        Mock.Start();
        if Which < 10 then
            Mock.SetExactPermissionSet('WS Excl')
        else
            Mock.SetExactPermissionSet('WS Mix');
        asserterror Op(Which);
        T := GetLastErrorText();
        ClearLastError();
        Mock.ClearAssignments();
        Mock.Stop();
    end;

    // The variables are made after the sets are assigned: an object loaded before is not checked again.
    local procedure Op(Which: Integer)
    var
        Row: Record "WS Row";
        Narrow: Record "WS Narrow";
        Excluded: Codeunit "WS Excluded";
        Other: Codeunit "WS Other";
        NarrowCu: Codeunit "WS Narrow Cu";
    begin
        case Which of
            1: Error('ok %1', Excluded.Ping());
            2: Error('ok %1', Other.Ping());
            3: begin Row.Id := 1; Row.Insert(); Error('ok inserted'); end;
            4: Error('ok %1', Row.IsEmpty());
            11: begin Narrow.Id := 1; Narrow.Insert(); Error('ok inserted'); end;
            12: Error('ok %1', NarrowCu.Ping());
        end;
    end;

    local procedure Expect(Which: Integer; Want: Text)
    var
        Got: Text;
    begin
        Got := Observe(Which);
        if Got <> Want then Error('%1: got %2', Which, Got);
    end;

    [Test]
    procedure ExcludedSetTakesOneObjectOutOfTheWildcard()
    begin
        Expect(1, 'Sorry, the current permissions prevented the action. (CodeUnit 51601 WS Excluded Execute: main)');
        Expect(2, 'ok ping');
        Expect(3, 'Sorry, the current permissions prevented the action. (TableData 51600 WS Row Insert: main)');
        Expect(4, 'ok Yes');
    end;

    local procedure SystemRights(Role: Text) T: Text
    var
        U: DotNet NavUserAccountHelper;
    begin
        if Role <> '' then begin
            Mock.Start();
            Mock.SetExactPermissionSet(CopyStr(Role, 1, 20));
        end;
        T := U.GetEffectivePermissionForObject(UserSecurityId(), CompanyName(), 10, 6110);
        if Role <> '' then begin
            Mock.ClearAssignments();
            Mock.Stop();
        end;
    end;

    [Test]
    procedure EffectivePermissionOnASystemObject()
    begin
        if SystemRights('WS Exec') <> '0,0,0,0,1' then Error('wildcard: %1', SystemRights('WS Exec'));
        if SystemRights('WS Basic') <> '0,0,0,0,0' then Error('basic: %1', SystemRights('WS Basic'));
        if SystemRights('') <> '0,0,0,0,1' then Error('no set: %1', SystemRights(''));
    end;

    [Test]
    procedure ExplicitLowerGrantDoesNotNarrowTheWildcard()
    begin
        Expect(11, 'ok inserted');
        Expect(12, 'ok ping');
    end;
}

// A refused test taints the later tests of its codeunit, so the unpinned case has a codeunit of its own.
codeunit 51605 "WS Tests Refused"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Mock: Codeunit "Permissions Mock";

    [Test]
    procedure IndirectCodeunitWildcardStaysRefused()
    var
        Card: TestPage "WS Card";
    begin
        Mock.Start();
        Mock.SetExactPermissionSet('WS Ind Cu');
        Card.OpenView();
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

/// Y93WildScope.al:588-589 (q1 `Set('D365 BASIC')`), Y90PermOpenPage.al `set_not_exact`.
#[test]
fn a_page_trigger_call_opens_when_the_set_wildcard_grants_the_callee() {
    assert_eq!(outcome("OpensUnderSetWithWildcardOnly"), Outcome::Pass);
}
/// Y93WildScope.al:594-597 (q4 an exact set of wildcards, q5 `D365 BASIC` alone, which holds wildcard Execute on every kind).
#[test]
fn a_page_trigger_call_opens_under_an_exact_set_of_wildcards() {
    assert_eq!(outcome("OpensUnderExactWildcardSet"), Outcome::Pass);
}
/// Y93WildScope.al:178-181 (`xa cu|W All`), :207-208 (`xa td insert|W Data`): a wildcard reaches the objects of another app.
#[test]
fn a_wildcard_reaches_another_apps_codeunit_from_a_page_trigger() {
    assert_eq!(outcome("OpensCallingAnotherAppsCodeunitUnderWildcards"), Outcome::Pass);
}
/// Y93WildScope.al:153-160: the excluded Execute and Insert grants are gone, the rest of the wildcard stays.
#[test]
fn an_excluded_set_takes_one_object_out_of_a_wildcard() {
    assert_eq!(outcome("ExcludedSetTakesOneObjectOutOfTheWildcard"), Outcome::Pass);
}
/// Y93WildScope.al:129-138: grants add up, an explicit `R` or `x` next to a wildcard keeps the wildcard's level.
#[test]
fn an_explicit_lower_grant_does_not_narrow_a_wildcard() {
    assert_eq!(outcome("ExplicitLowerGrantDoesNotNarrowTheWildcard"), Outcome::Pass);
}
/// Y93WildEffective.al:97-100, 118-121 (`system * = X` gives X on system objects 6110, 5810, 9026; a set without it gives none),
/// Y90PermSystem.al:234-241 (`wild exact`, `noset`: X on every system object asked).
#[test]
fn the_effective_permission_on_a_system_object_follows_the_system_wildcard() {
    assert_eq!(outcome("EffectivePermissionOnASystemObject"), Outcome::Pass);
}
/// An indirect wildcard on codeunits (`codeunit * = x`) under a page trigger call: Y93 does not measure it, so it stays refused.
#[test]
fn an_indirect_codeunit_wildcard_on_a_page_trigger_call_stays_refused() {
    match outcome("IndirectCodeunitWildcardStaysRefused") {
        Outcome::NotRun(why) => assert!(why.contains("wildcard"), "{why}"),
        o => panic!("{o:?}"),
    }
}
