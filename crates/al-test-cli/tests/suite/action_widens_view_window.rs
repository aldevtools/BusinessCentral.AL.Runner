//! A List with OnAfterGetRecord whose OnOpenPage filters Room > 0 stands on A02, the last of two rows in that view. Its action
//! removes the filter and calls `CurrPage.Update(false)`: the new view holds more rows than a window from A02 on. First then
//! goes to A01 and Next walks every row (al-runner issue 6469, a regression of the commit for issue 6381; the reporter's repro
//! without Library Assert, with 25 and 21 rows; real BC: the test passes). The refresh came from an action, not from a
//! control's OnValidate.

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
    procedure Rows25() begin Walk(25); end;
    [Test]
    procedure Rows21() begin Walk(21); end;
    [Test]
    procedure Rows20() begin Walk(20); end;
    [Test]
    procedure NextAfterTheAction()
    var
        Bins: TestPage "Repro Bins";
    begin
        InsertBins(21);
        Bins.OpenView();
        Bins.Last();
        Bins.ShowAll.Invoke();
        if not Bins.Next() or (Bins."Code".Value() <> 'A03') then
            Error('next after the action: %1', Bins."Code".Value());
    end;

    local procedure Walk(Rows: Integer)
    var
        Bins: TestPage "Repro Bins";
        Count: Integer;
    begin
        InsertBins(Rows);
        Bins.OpenView();
        if not Bins.Last() or (Bins."Code".Value() <> 'A02') then
            Error('last row with Room > 0: %1', Bins."Code".Value());
        Bins.ShowAll.Invoke();
        if not Bins.First() then
            Error('the list shows a row after the action');
        if Bins."Code".Value() <> 'A01' then
            Error('first row after the action: %1', Bins."Code".Value());
        repeat
            Count += 1;
        until not Bins.Next();
        if Count <> Rows then
            Error('rows after Show All: %1', Count);
        Bins.Close();
    end;

    local procedure InsertBins(Rows: Integer)
    var
        Bin: Record "Repro Bin";
        i: Integer;
    begin
        Bin.DeleteAll();
        for i := 1 to Rows do begin
            Bin."Code" := 'A' + Format(i, 2, '<Integer,2><Filler Character,0>');
            Bin."Room" := 0;
            if i in [1, 2] then
                Bin."Room" := 6;
            Bin.Insert();
        end;
    end;
}
"##;

#[test]
fn moves_after_an_action_widened_the_view_past_a_window() {
    let o = outcomes_in(SRC, "Repro Widen View Test");
    for t in ["Rows25", "Rows21", "Rows20", "NextAfterTheAction"] {
        let got = format!("{:?}", o.iter().find(|(n, _)| n == t).map(|(_, x)| x));
        assert_eq!(got, "Some(Pass)", "{t}");
    }
}
