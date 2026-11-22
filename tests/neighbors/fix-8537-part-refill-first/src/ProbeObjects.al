// The objects and tests of corpus PR 1474 (pages/parts/PartRefillTestFilter.al, merged) and its two-field key cell
// (corpus PR 1475), both merged; the suite test part_left_view_test_filter.rs reads them from here.
table 97100 "LT PR Op"
{
    DataClassification = SystemMetadata;

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

table 97101 "LT PR Booking"
{
    DataClassification = SystemMetadata;

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

table 97102 "LT PR Load Line"
{
    DataClassification = SystemMetadata;
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

codeunit 97103 "LT PR Load Calc"
{
    procedure Calculate(ResourceNo: Code[20]) Results: Dictionary of [Text, Text]
    var
        Booking: Record "LT PR Booking";
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

page 97104 "LT PR Load Part"
{
    PageType = ListPart;
    SourceTable = "LT PR Load Line";
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
                field("Period Start"; Rec."Period Start") { ApplicationArea = All; }
                field("Allocated Qty."; Rec."Allocated Qty.") { ApplicationArea = All; }
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

page 97105 "LT PR Load Host"
{
    PageType = Worksheet;
    SourceTable = "LT PR Op";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Ops)
            {
                field("No."; Rec."No.") { ApplicationArea = All; }
                field("Resource No."; Rec."Resource No.") { ApplicationArea = All; }
            }
            part(Load; "LT PR Load Part") { ApplicationArea = All; }
        }
    }

    actions
    {
        area(Processing)
        {
            action(Refresh)
            {
                ApplicationArea = All;

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
        LoadCalc: Codeunit "LT PR Load Calc";
    begin
        if CurrPage.Load.Page.IsLoadedFor(Rec."Resource No.") then
            exit;
        CurrPage.Load.Page.ClearLines(Rec."Resource No.");
        CurrPage.Load.Page.SetLoadLines(LoadCalc.Calculate(Rec."Resource No."));
    end;
}

page 97106 "LT PR Load List Host"
{
    PageType = List;
    SourceTable = "LT PR Op";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Ops)
            {
                field("No."; Rec."No.") { ApplicationArea = All; }
                field("Resource No."; Rec."Resource No.") { ApplicationArea = All; }
            }
            part(Load; "LT PR Load Part") { ApplicationArea = All; }
        }
    }

    actions
    {
        area(Processing)
        {
            action(Refresh)
            {
                ApplicationArea = All;

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
        LoadCalc: Codeunit "LT PR Load Calc";
    begin
        if CurrPage.Load.Page.IsLoadedFor(Rec."Resource No.") then
            exit;
        CurrPage.Load.Page.ClearLines(Rec."Resource No.");
        CurrPage.Load.Page.SetLoadLines(LoadCalc.Calculate(Rec."Resource No."));
    end;
}

page 97107 "LT PR Load Bg Host"
{
    PageType = Worksheet;
    SourceTable = "LT PR Op";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Ops)
            {
                field("No."; Rec."No.") { ApplicationArea = All; }
                field("Resource No."; Rec."Resource No.") { ApplicationArea = All; }
            }
            part(Load; "LT PR Load Part") { ApplicationArea = All; }
        }
    }

    actions
    {
        area(Processing)
        {
            action(Refresh)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    CurrPage.Load.Page.Invalidate();
                    CurrPage.Update(false);
                end;
            }
        }
    }

    var
        LoadTaskId: Integer;

    trigger OnAfterGetCurrRecord()
    var
        Params: Dictionary of [Text, Text];
    begin
        if CurrPage.Load.Page.IsLoadedFor(Rec."Resource No.") then
            exit;
        if LoadTaskId <> 0 then
            CurrPage.CancelBackgroundTask(LoadTaskId);
        LoadTaskId := 0;
        CurrPage.Load.Page.ClearLines(Rec."Resource No.");
        Params.Add('Resource', Rec."Resource No.");
        CurrPage.EnqueueBackgroundTask(LoadTaskId, Codeunit::"LT PR Load Task", Params, 120000, PageBackgroundTaskErrorLevel::Warning);
    end;

    trigger OnPageBackgroundTaskCompleted(TaskId: Integer; Results: Dictionary of [Text, Text])
    begin
        if TaskId <> LoadTaskId then
            exit;
        LoadTaskId := 0;
        CurrPage.Load.Page.SetLoadLines(Results);
    end;
}

codeunit 97108 "LT PR Load Task"
{
    trigger OnRun()
    var
        LoadCalc: Codeunit "LT PR Load Calc";
        Params: Dictionary of [Text, Text];
    begin
        Params := Page.GetBackgroundParameters();
        Page.SetBackgroundTaskResult(LoadCalc.Calculate(CopyStr(Params.Get('Resource'), 1, 20)));
    end;
}

page 97110 "LT PR Load Trig Part"
{
    PageType = ListPart;
    SourceTable = "LT PR Load Line";
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
                field("Period Start"; Rec."Period Start") { ApplicationArea = All; }
                field("Allocated Qty."; Rec."Allocated Qty.") { ApplicationArea = All; StyleExpr = QtyStyle; }
                field(QtyText; QtyText) { ApplicationArea = All; StyleExpr = QtyStyle; }
                field(Scope; ScopeText) { ApplicationArea = All; }
            }
        }
    }

    trigger OnOpenPage()
    begin
        ScopeText := 'open';
    end;

    trigger OnAfterGetRecord()
    begin
        QtyText := 'agr' + Format(Rec."Allocated Qty.", 0, 9);
        QtyStyle := 'Standard';
        if Rec."Allocated Qty." > 20 then
            QtyStyle := 'Unfavorable';
    end;

    var
        ResourceNo: Code[20];
        LinesLoaded: Boolean;
        QtyText: Text;
        QtyStyle: Text;
        ScopeText: Text;

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

page 97111 "LT PR Load Trig Host"
{
    PageType = Worksheet;
    SourceTable = "LT PR Op";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Ops)
            {
                field("No."; Rec."No.") { ApplicationArea = All; }
                field("Resource No."; Rec."Resource No.") { ApplicationArea = All; }
            }
            part(Load; "LT PR Load Trig Part") { ApplicationArea = All; }
        }
    }

    actions
    {
        area(Processing)
        {
            action(Refresh)
            {
                ApplicationArea = All;

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
        LoadCalc: Codeunit "LT PR Load Calc";
    begin
        if CurrPage.Load.Page.IsLoadedFor(Rec."Resource No.") then
            exit;
        CurrPage.Load.Page.ClearLines(Rec."Resource No.");
        CurrPage.Load.Page.SetLoadLines(LoadCalc.Calculate(Rec."Resource No."));
    end;
}

table 97112 "LT PR Load Line Typed"
{
    DataClassification = SystemMetadata;
    TableType = Temporary;

    fields
    {
        field(1; "Period Type"; Option) { OptionMembers = Day,Week; }
        field(2; "Period Start"; Date) { }
        field(3; "Allocated Qty."; Decimal) { }
    }

    keys
    {
        key(PK; "Period Type", "Period Start") { Clustered = true; }
    }
}

page 97113 "LT PR Typed Trig Part"
{
    PageType = ListPart;
    SourceTable = "LT PR Load Line Typed";
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
                field("Period Start"; Rec."Period Start") { ApplicationArea = All; }
                field("Allocated Qty."; Rec."Allocated Qty.") { ApplicationArea = All; StyleExpr = QtyStyle; }
                field(QtyText; QtyText) { ApplicationArea = All; StyleExpr = QtyStyle; }
                field(Scope; ScopeText) { ApplicationArea = All; }
            }
        }
    }

    trigger OnOpenPage()
    begin
        ScopeText := 'open';
    end;

    trigger OnAfterGetRecord()
    begin
        QtyText := 'agr' + Format(Rec."Allocated Qty.", 0, 9);
        QtyStyle := 'Standard';
        if Rec."Allocated Qty." > 20 then
            QtyStyle := 'Unfavorable';
    end;

    var
        ResourceNo: Code[20];
        LinesLoaded: Boolean;
        QtyText: Text;
        QtyStyle: Text;
        ScopeText: Text;

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
            Rec."Period Type" := Rec."Period Type"::Day;
            Evaluate(Rec."Period Start", DayKey, 9);
            Evaluate(Rec."Allocated Qty.", Results.Get(DayKey), 9);
            Rec.Insert();
        end;
        LinesLoaded := true;
        if Rec.FindFirst() then;
        CurrPage.Update(false);
    end;
}

page 97114 "LT PR Typed Trig Host"
{
    PageType = Worksheet;
    SourceTable = "LT PR Op";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Ops)
            {
                field("No."; Rec."No.") { ApplicationArea = All; }
                field("Resource No."; Rec."Resource No.") { ApplicationArea = All; }
            }
            part(Load; "LT PR Typed Trig Part") { ApplicationArea = All; }
        }
    }

    actions
    {
        area(Processing)
        {
            action(Refresh)
            {
                ApplicationArea = All;

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
        LoadCalc: Codeunit "LT PR Load Calc";
    begin
        if CurrPage.Load.Page.IsLoadedFor(Rec."Resource No.") then
            exit;
        CurrPage.Load.Page.ClearLines(Rec."Resource No.");
        CurrPage.Load.Page.SetLoadLines(LoadCalc.Calculate(Rec."Resource No."));
    end;
}
