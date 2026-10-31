//! A List without row triggers stands on its last row (row 3) when an action narrows the view so that the row leaves it and no
//! row of the new view follows it (`Rec.SetRange(Included, true)` with `CurrPage.Update(false)`). First, Next and Next then walk
//! the new view, rows 1 and 2 (al-runner issue 6468, the reporter's repro; corpus writer Y15, pages/lt-y15/
//! Y15ActionNarrowsBelowLast.al). The case where a row of the new view follows is pinned by pages/lt-y5/
//! Y5GoToKeyAfterActionFilter.al:158-174 (al-runner issue 6381).

use al_testkit::outcomes_in;

const SRC: &str = r##"
table 50100 "Filter Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Included; Boolean) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

page 50100 "Filter List"
{
    PageType = List;
    SourceTable = "Filter Row";
    InsertAllowed = false;
    DeleteAllowed = false;
    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field(Id; Rec.Id) { ApplicationArea = All; }
                field(Included; Rec.Included) { ApplicationArea = All; }
            }
        }
    }
    actions
    {
        area(Processing)
        {
            action(ShowAll)
            {
                ApplicationArea = All;
                trigger OnAction()
                begin
                    Rec.SetRange(Included);
                    CurrPage.Update(false);
                end;
            }
            action(ShowIncluded)
            {
                ApplicationArea = All;
                trigger OnAction()
                begin
                    Rec.SetRange(Included, true);
                    CurrPage.Update(false);
                end;
            }
        }
    }
    trigger OnOpenPage()
    begin
        Rec.SetRange(Included, true);
    end;
}

codeunit 50101 "Filter Action Tests"
{
    Subtype = Test;

    [Test]
    procedure FilterDropsLastRowButKeepsEarlierRows()
    var
        Row: Record "Filter Row";
        List: TestPage "Filter List";
    begin
        Row.DeleteAll();
        AddRow(1, true);
        AddRow(2, true);
        AddRow(3, false);
        List.OpenView();
        if not List.First() or (List.Id.AsInteger() <> 1) then
            Error('The initial view must start at row 1.');
        List.ShowAll.Invoke();
        if not List.Last() or (List.Id.AsInteger() <> 3) then
            Error('The full view must end at row 3.');
        List.ShowIncluded.Invoke();
        if not List.First() or (List.Id.AsInteger() <> 1) then
            Error('The restored view must start at row 1.');
        if not List.Next() or (List.Id.AsInteger() <> 2) then
            Error('The restored view must contain row 2.');
        if List.Next() then
            Error('The restored view must contain only two rows.');
        List.Close();
    end;

    [Test]
    procedure ReadRightAfterTheFilterDropsLastRow()
    var
        Row: Record "Filter Row";
        List: TestPage "Filter List";
    begin
        Row.DeleteAll();
        AddRow(1, true);
        AddRow(2, true);
        AddRow(3, false);
        List.OpenView();
        List.ShowAll.Invoke();
        List.Last();
        List.ShowIncluded.Invoke();
        if List.Id.Value() <> '2' then
            Error('shows %1', List.Id.Value());
        if List.Next() then
            Error('next after the action');
        if not List.Previous() or (List.Id.AsInteger() <> 1) then
            Error('previous after the action');
    end;

    [Test]
    procedure ActionLeavesNoRow()
    var
        Row: Record "Filter Row";
        List: TestPage "Filter List";
    begin
        Row.DeleteAll();
        AddRow(1, false);
        AddRow(2, false);
        List.OpenView();
        List.ShowAll.Invoke();
        List.Last();
        List.ShowIncluded.Invoke();
        if List.Id.Value() <> '' then
            Error('shows [%1]', List.Id.Value());
        if List.First() or List.Last() then
            Error('a move in the empty view');
    end;

    local procedure AddRow(Id: Integer; Included: Boolean)
    var
        Row: Record "Filter Row";
    begin
        Row.Id := Id;
        Row.Included := Included;
        Row.Insert();
    end;
}
"##;

#[test]
fn first_after_an_action_drops_the_last_row_walks_the_new_view() {
    let o = outcomes_in(SRC, "Filter Action Tests");
    let get = |n: &str| format!("{:?}", o.iter().find(|(name, _)| name == n).map(|(_, x)| x));
    assert_eq!(get("FilterDropsLastRowButKeepsEarlierRows"), "Some(Pass)");
    // Right after the action the page stands on the nearest row before (Y15ActionNarrowsBelowLast.al:237, :269); a view left
    // without rows shows a blank row (:345).
    assert_eq!(get("ReadRightAfterTheFilterDropsLastRow"), "Some(Pass)");
    assert_eq!(get("ActionLeavesNoRow"), "Some(Pass)");
}
