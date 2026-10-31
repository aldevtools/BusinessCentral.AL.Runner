//! A List with OnAfterGetRecord whose OnOpenPage filters Room > 0 stands on A20 (the first of two rows in that view). Its
//! action removes the filter and calls `CurrPage.Update(false)`: 19 rows of the new view lie before A20. First then goes to
//! the first row of the new view, A01, and Next walks all 25 rows (al-runner issue 6470, the reporter's repro without
//! Library Assert; real BC: the test passes).

use al_testkit::outcomes_in;

const SRC: &str = r##"
table 50100 "Repro Bin"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Room"; Integer) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

page 50100 "Repro Bins"
{
    PageType = List;
    SourceTable = "Repro Bin";
    Editable = false;
    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("Code"; Rec."Code") { }
                field(RoomShown; RoomShown) { }
            }
        }
    }
    actions
    {
        area(Processing)
        {
            action(ShowAll)
            {
                trigger OnAction()
                begin
                    Rec.SetRange("Room");
                    CurrPage.Update(false);
                end;
            }
        }
    }

    trigger OnOpenPage()
    begin
        Rec.SetFilter("Room", '>0');
    end;

    trigger OnAfterGetRecord()
    begin
        RoomShown := Format(Rec."Room");
    end;

    var
        RoomShown: Text[10];
}

codeunit 50101 "Repro Widen View Test"
{
    Subtype = Test;

    [Test]
    procedure FirstAfterAnActionWidensTheViewToMoreThan20Rows()
    var
        Bins: TestPage "Repro Bins";
        Count: Integer;
    begin
        InsertBins();
        Bins.OpenView();
        Bins.ShowAll.Invoke();
        if not Bins.First() then
            Error('the list shows a row after the action');
        if Bins."Code".Value() <> 'A01' then
            Error('first row after the action: %1', Bins."Code".Value());
        repeat
            Count += 1;
        until not Bins.Next();
        if Count <> 25 then
            Error('rows after Show All: %1', Count);
        Bins.Close();
    end;

    local procedure InsertBins()
    var
        Bin: Record "Repro Bin";
        i: Integer;
    begin
        Bin.DeleteAll();
        for i := 1 to 25 do begin
            Bin."Code" := 'A' + Format(i, 2, '<Integer,2><Filler Character,0>');
            Bin."Room" := 0;
            if i in [20, 24] then
                Bin."Room" := 6;
            Bin.Insert();
        end;
    end;
}
"##;

#[test]
fn first_after_an_action_widened_the_view_goes_to_its_first_row() {
    let o = outcomes_in(SRC, "Repro Widen View Test");
    let got = format!(
        "{:?}",
        o.iter()
            .find(|(n, _)| n == "FirstAfterAnActionWidensTheViewToMoreThan20Rows")
            .map(|(_, x)| x)
    );
    assert_eq!(got, "Some(Pass)");
}
