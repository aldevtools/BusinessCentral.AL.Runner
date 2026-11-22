// al-runner issue 8537, the reporter's repro (app/Repro.al and test/Tests.Codeunit.al) as filed.
table 50100 "Repro Op"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; "Resource No."; Code[20]) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

table 50101 "Repro Booking"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; "Resource No."; Code[20]) { }
        field(3; Day; Date) { }
        field(4; Qty; Decimal) { }
    }

    keys
    {
        key(PK; "Entry No.") { Clustered = true; }
    }
}

table 50102 "Repro Load Line"
{
    DataClassification = CustomerContent;
    TableType = Temporary;

    fields
    {
        field(1; "Period Start"; Date) { }
        field(2; "Allocated Qty."; Decimal) { }
    }

    keys
    {
        key(PK; "Period Start") { Clustered = true; }
    }
}

codeunit 50100 "Repro Load Calc"
{
    procedure Calculate(ResourceNo: Code[20]) Results: Dictionary of [Text, Text]
    var
        Booking: Record "Repro Booking";
        DayKey: Text;
        Qty: Decimal;
    begin
        Booking.SetRange("Resource No.", ResourceNo);
        if Booking.FindSet() then
            repeat
                DayKey := Format(Booking.Day, 0, 9);
                Qty := 0;
                if Results.ContainsKey(DayKey) then
                    Evaluate(Qty, Results.Get(DayKey), 9);
                Qty += Booking.Qty;
                Results.Set(DayKey, Format(Qty, 0, 9));
            until Booking.Next() = 0;
    end;
}

page 50101 "Repro Load Part"
{
    PageType = ListPart;
    SourceTable = "Repro Load Line";
    SourceTableTemporary = true;
    InsertAllowed = false;
    DeleteAllowed = false;
    Editable = false;
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field("Period Start"; Rec."Period Start") { }
                field("Allocated Qty."; Rec."Allocated Qty.") { }
            }
        }
    }

    var
        ResourceNo: Code[20];
        LinesLoaded: Boolean;

    procedure IsLoadedFor(NewResourceNo: Code[20]): Boolean
    begin
        exit(LinesLoaded and (NewResourceNo = ResourceNo));
    end;

    procedure Invalidate()
    begin
        LinesLoaded := false;
    end;

    procedure ClearLines(NewResourceNo: Code[20])
    begin
        ResourceNo := NewResourceNo;
        LinesLoaded := false;
        Rec.Reset();
        Rec.DeleteAll(false);
        CurrPage.Update(false);
    end;

    procedure SetLoadLines(Results: Dictionary of [Text, Text])
    var
        DayKey: Text;
    begin
        foreach DayKey in Results.Keys() do begin
            Rec.Init();
            Evaluate(Rec."Period Start", DayKey, 9);
            Evaluate(Rec."Allocated Qty.", Results.Get(DayKey), 9);
            Rec.Insert();
        end;
        LinesLoaded := true;
        if Rec.FindFirst() then;
        CurrPage.Update(false);
    end;
}

page 50100 "Repro Load Host"
{
    PageType = Worksheet;
    SourceTable = "Repro Op";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Ops)
            {
                field("No."; Rec."No.") { }
                field("Resource No."; Rec."Resource No.") { }
            }
            part(Load; "Repro Load Part") { }
        }
    }

    actions
    {
        area(Processing)
        {
            action(Refresh)
            {
                trigger OnAction()
                begin
                    CurrPage.Load.Page.Invalidate();
                    CurrPage.Update(false);
                end;
            }
        }
    }

    trigger OnAfterGetCurrRecord()
    var
        LoadCalc: Codeunit "Repro Load Calc";
    begin
        if CurrPage.Load.Page.IsLoadedFor(Rec."Resource No.") then
            exit;
        CurrPage.Load.Page.ClearLines(Rec."Resource No.");
        CurrPage.Load.Page.SetLoadLines(LoadCalc.Calculate(Rec."Resource No."));
    end;
}



codeunit 50120 "Repro Part Refill Tests"
{
    Subtype = Test;

    [Test]
    procedure RefreshRefillsPartThenFirstFindsFilteredRow()
    var
        Op: Record "Repro Op";
        Booking: Record "Repro Booking";
        Host: TestPage "Repro Load Host";
        Day: Date;
    begin
        Op.DeleteAll();
        Op."No." := 'OP1';
        Op."Resource No." := 'R1';
        Op.Insert();
        Day := DMY2Date(15, 6, 2026);
        Booking.DeleteAll();
        Add(1, 10, Day);
        Add(2, 5, Day + 1);

        Host.OpenView();
        Host.GoToRecord(Op);
        Host.Load.Filter.SetFilter("Period Start", Format(Day));
        if not Host.Load.First() then
            Error('Expected First() to find the load line of the day before Refresh.');
        if Host.Load."Allocated Qty.".AsDecimal() <> 10 then
            Error('Expected Allocated Qty. 10 before Refresh, found %1.', Host.Load."Allocated Qty.".AsDecimal());

        Add(3, 20, Day);
        Host.Refresh.Invoke();
        Host.Load.Filter.SetFilter("Period Start", Format(Day));
        if not Host.Load.First() then
            Error('Expected First() to find the load line of the day after Refresh.');
        if Host.Load."Allocated Qty.".AsDecimal() <> 30 then
            Error('Expected Allocated Qty. 30 after Refresh, found %1.', Host.Load."Allocated Qty.".AsDecimal());
        Host.Close();
    end;

    local procedure Add(EntryNo: Integer; Qty: Decimal; Day: Date)
    var
        Booking: Record "Repro Booking";
    begin
        Booking."Entry No." := EntryNo;
        Booking."Resource No." := 'R1';
        Booking.Day := Day;
        Booking.Qty := Qty;
        Booking.Insert();
    end;
}
