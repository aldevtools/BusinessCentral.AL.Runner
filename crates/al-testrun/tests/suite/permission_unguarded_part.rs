//! ADR-062 stage 3: a part with no AccessByPermission under an exact permission set. Values: corpus
//! `tests/al-language/permissions/permission-sets/Y90PermParts.al:20-78` (Y90_Part_ExactA..C, Y90_Part_Set; branch al-runtime-rs-staging-y90):
//! the part shows only when the session executes the part page and its source table.

use super::permission_writes::run_main;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, RunOptions};

const MAIN: &str = r#"
table 51580 "UP Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }
page 51580 "UP Part"
{
    PageType = ListPart; SourceTable = "UP Row"; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(IdCol; Rec.Id) { ApplicationArea = All; } } } }
}
page 51581 "UP Card"
{
    PageType = Card; SourceTable = "UP Row"; UsageCategory = None;
    layout { area(Content) { field(IdCtl; Rec.Id) { ApplicationArea = All; } part(PartCtl; "UP Part") { ApplicationArea = All; } } }
}
permissionset 51580 "UP Pages" { Assignable = true; Permissions = tabledata "UP Row" = RIMD, page "UP Card" = X, page "UP Part" = X; }
permissionset 51581 "UP Pages Table" { Assignable = true; Permissions = tabledata "UP Row" = RIMD, page "UP Card" = X, page "UP Part" = X, table "UP Row" = X; }
permissionset 51582 "UP Page Star" { Assignable = true; Permissions = tabledata "UP Row" = RIMD, page * = X, codeunit * = X; }

codeunit 51580 "UP Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Mock: Codeunit "Permissions Mock";

    local procedure Visible(SetName: Code[20]; Exact: Boolean): Boolean
    var
        Row: Record "UP Row";
        Card: TestPage "UP Card";
        Seen: Boolean;
    begin
        Row.Id := 1;
        if Row.Insert() then;
        Mock.Start();
        if Exact then Mock.SetExactPermissionSet(SetName) else Mock.Set(SetName);
        Card.OpenView();
        asserterror begin
            if Card.PartCtl.IdCol.Visible() then;
            Error('seen');
        end;
        Seen := GetLastErrorText() = 'seen';
        ClearLastError();
        Card.Close();
        Mock.ClearAssignments();
        Mock.Stop();
        exit(Seen);
    end;

    [Test]
    procedure ExactPagesOnlyLeavesThePartOff()
    begin
        if Visible('UP Pages', true) then Error('part is visible');
    end;

    [Test]
    procedure ExactPageStarAndCodeunitStarLeavesThePartOff()
    begin
        if Visible('UP Page Star', true) then Error('part is visible');
    end;

    [Test]
    procedure ExactPagesAndTableExecuteShowsThePart()
    begin
        if not Visible('UP Pages Table', true) then Error('part is not visible');
    end;

    [Test]
    procedure SetShowsThePart()
    begin
        if not Visible('UP Pages', false) then Error('part is not visible');
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
fn an_exact_set_with_the_pages_only_leaves_an_unguarded_part_off() {
    passes("ExactPagesOnlyLeavesThePartOff");
}
#[test]
fn an_exact_set_with_every_page_and_codeunit_leaves_an_unguarded_part_off() {
    passes("ExactPageStarAndCodeunitStarLeavesThePartOff");
}
#[test]
fn an_exact_set_that_also_executes_the_source_table_shows_the_part() {
    passes("ExactPagesAndTableExecuteShowsThePart");
}
#[test]
fn a_set_shows_the_part() {
    passes("SetShowsThePart");
}
