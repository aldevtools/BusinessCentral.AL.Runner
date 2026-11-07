//! ADR-062 stage 3: the AccessByPermission shapes `permission_abp.rs` does not cover. Values: corpus
//! `tests/al-language/permissions/permission-sets/Y84PermAbp.al` (BC 29.0 Linux CI): a guard with several letters is met by any
//! one of them held at level Yes; an Indirect level does not meet it; a part and a control over a table field take a guard too; the
//! page object's own property changes nothing; a query, an xmlport and a report can be guarded; a FlowField control over an
//! unreadable table is off the page whether the table has rows or not.

use super::permission_writes::run_main;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, RunOptions};

const MAIN: &str = r#"
table 51560 "AS Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Val; Integer) { }
        field(3; TfGuarded; Integer) { AccessByPermission = tabledata "AS Guard" = R; }
        field(4; Cnt; Integer) { FieldClass = FlowField; CalcFormula = count("AS Guard"); }
    }
    keys { key(PK; Id) { Clustered = true; } }
}
table 51561 "AS Guard" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }
query 51560 "AS Query" { elements { dataitem(G; "AS Guard") { column(Id; Id) { } } } }

page 51560 "AS Part"
{
    PageType = ListPart; SourceTable = "AS Row"; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(IdCol; Rec.Id) { ApplicationArea = All; } } } }
}
page 51561 "AS Card"
{
    PageType = Card; SourceTable = "AS Row"; UsageCategory = None;
    AccessByPermission = tabledata "AS Guard" = R;
    layout
    {
        area(Content)
        {
            field(RimdCtl; Rec.Val) { ApplicationArea = All; AccessByPermission = tabledata "AS Guard" = RIMD; }
            field(LowerCtl; Rec.Val) { ApplicationArea = All; AccessByPermission = tabledata "AS Guard" = r; }
            field(TfCtl; Rec.TfGuarded) { ApplicationArea = All; }
            field(FlowCtl; Rec.Cnt) { ApplicationArea = All; }
            field(QryCtl; Rec.Id) { ApplicationArea = All; AccessByPermission = query "AS Query" = X; }
            part(GuardPart; "AS Part") { ApplicationArea = All; AccessByPermission = tabledata "AS Guard" = R; }
        }
    }
}

permissionset 51560 "AS Row Only" { Assignable = true; Permissions = tabledata "AS Row" = RIMD; }
permissionset 51561 "AS Guard R" { Assignable = true; Permissions = tabledata "AS Row" = RIMD, tabledata "AS Guard" = R; }
permissionset 51562 "AS Guard Ind R" { Assignable = true; Permissions = tabledata "AS Row" = RIMD, tabledata "AS Guard" = r; }
permissionset 51563 "AS Exact" { Assignable = true; Permissions = tabledata "AS Row" = RIMD, tabledata "AS Guard" = R, page "AS Card" = X, page "AS Part" = X, query "AS Query" = X; }

codeunit 51562 "AS Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Mock: Codeunit "Permissions Mock";

    local procedure Expect(Got: Text; Want: Text)
    begin
        if Got <> Want then
            Error('want %1, got %2', Want, Got);
    end;

    local procedure Seed()
    var Row: Record "AS Row";
    begin
        Row.DeleteAll();
        Row.Id := 1; Row.Val := 10; Row.Insert();
    end;

    local procedure Vis(var P: TestPage "AS Card"; What: Text): Text
    var
        R: Boolean;
    begin
        if TryVis(P, What, R) then
            exit(Format(R));
        ClearLastError();
        exit('off');
    end;

    [TryFunction]
    local procedure TryVis(var P: TestPage "AS Card"; What: Text; var R: Boolean)
    begin
        case What of
            'rimd': R := P.RimdCtl.Visible();
            'lower': R := P.LowerCtl.Visible();
            'tf': R := P.TfCtl.Visible();
            'flow': R := P.FlowCtl.Visible();
            'qry': R := P.QryCtl.Visible();
            'part': R := P.GuardPart.IdCol.Visible();
        end;
    end;

    local procedure Facts(): Text
    var
        P: TestPage "AS Card";
        Res: Text;
    begin
        P.OpenView();
        P.First();
        Res := Vis(P, 'rimd') + ' ' + Vis(P, 'lower') + ' ' + Vis(P, 'tf') + ' ' + Vis(P, 'flow') + ' ' + Vis(P, 'qry') + ' ' + Vis(P, 'part');
        P.Close();
        exit(Res);
    end;

    [Test]
    procedure AnyLetterOfTheGuardMeetsIt()
    var Got: Text;
    begin
        Mock.Start();
        Seed();
        Mock.Set('AS Guard R');
        Got := Facts();
        Mock.ClearAssignments();
        Mock.Stop();
        Expect(Got, 'Yes Yes Yes Yes Yes Yes');
    end;

    [Test]
    procedure NoGrantLeavesTheGuardedControlsOff()
    var Got: Text;
    begin
        Mock.Start();
        Seed();
        Mock.Set('AS Row Only');
        Got := Facts();
        Mock.ClearAssignments();
        Mock.Stop();
        // The query guard is held through All Objects (Set); the FlowField control is off with rows in the source table or none.
        Expect(Got, 'off off off off Yes off');
    end;

    [Test]
    procedure AnIndirectLevelDoesNotMeetTheGuard()
    var Got: Text;
    begin
        Mock.Start();
        Seed();
        Mock.Set('AS Guard Ind R');
        Got := Facts();
        Mock.ClearAssignments();
        Mock.Stop();
        // An indirect read still lets the FlowField control show (Y84 `indr`: FLOW=Y).
        Expect(Got, 'off off off Yes Yes off');
    end;

    [Test]
    procedure TheExactSetShowsTheGuardedPartThroughItsPage()
    var Got: Text;
    begin
        Mock.Start();
        Seed();
        Mock.SetExactPermissionSet('AS Exact');
        Got := Facts();
        Mock.ClearAssignments();
        Mock.Stop();
        Expect(Got, 'Yes Yes Yes Yes Yes Yes');
    end;

    [Test]
    procedure ThePageObjectsOwnGuardChangesNothing()
    var P: TestPage "AS Card";
    begin
        Mock.Start();
        Seed();
        Mock.Set('AS Row Only');
        P.OpenView();
        P.Close();
        Mock.ClearAssignments();
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
fn any_letter_of_the_guard_meets_it() {
    passes("AnyLetterOfTheGuardMeetsIt");
}
#[test]
fn no_grant_leaves_the_guarded_controls_off() {
    passes("NoGrantLeavesTheGuardedControlsOff");
}
#[test]
fn an_indirect_level_does_not_meet_the_guard() {
    passes("AnIndirectLevelDoesNotMeetTheGuard");
}
#[test]
fn the_exact_set_shows_the_guarded_part_through_its_page() {
    passes("TheExactSetShowsTheGuardedPartThroughItsPage");
}
#[test]
fn the_page_objects_own_guard_changes_nothing() {
    passes("ThePageObjectsOwnGuardChangesNothing");
}
