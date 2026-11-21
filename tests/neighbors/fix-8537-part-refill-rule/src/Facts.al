// Each fact the rule of part_refill_rule.rs requires, changed on the probe's Worksheet cell or its two-field key cell.
page 97160 "LT PR NR PartAction Part"
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

    actions { area(Processing) { action(Poke) { ApplicationArea = All; trigger OnAction() begin end; } } }
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

page 97161 "LT PR NR PartAction Host"
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
            part(Load; "LT PR NR PartAction Part") { ApplicationArea = All; }
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

page 97162 "LT PR NR RecFieldValidate Part"
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
                field("Period Start"; Rec."Period Start") { ApplicationArea = All; trigger OnValidate() begin end; }
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

page 97163 "LT PR NR RecFieldValidate Host"
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
            part(Load; "LT PR NR RecFieldValidate Part") { ApplicationArea = All; }
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

page 97164 "LT PR NR InsertAllowed Part"
{
    PageType = ListPart;
    SourceTable = "LT PR Load Line";
    SourceTableTemporary = true;
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

page 97165 "LT PR NR InsertAllowed Host"
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
            part(Load; "LT PR NR InsertAllowed Part") { ApplicationArea = All; }
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

page 97166 "LT PR NR UpdProp Part"
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

page 97167 "LT PR NR UpdProp Host"
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
            part(Load; "LT PR NR UpdProp Part") { ApplicationArea = All; UpdatePropagation = Both; }
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

page 97168 "LT PR NR HostNewRecord Part"
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

page 97169 "LT PR NR HostNewRecord Host"
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
            part(Load; "LT PR NR HostNewRecord Part") { ApplicationArea = All; }
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

    trigger OnNewRecord(BelowxRec: Boolean)
    begin
    end;

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

page 97170 "LT PR NR ListPlusHost Part"
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

page 97171 "LT PR NR ListPlusHost Host"
{
    PageType = ListPlus;
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
            part(Load; "LT PR NR ListPlusHost Part") { ApplicationArea = All; }
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

page 97172 "LT PR NR HiddenPart Part"
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

page 97173 "LT PR NR HiddenPart Host"
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
            part(Load; "LT PR NR HiddenPart Part") { ApplicationArea = All; Visible = ShowLoad; }
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
        ShowLoad: Boolean;

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

table 97174 "LT PR NR ThreeKeyFields Line"
{
    DataClassification = SystemMetadata;
    TableType = Temporary;

    fields
    {
        field(1; "Period Type"; Option) { OptionMembers = Day,Week; }
        field(2; "Period Start"; Date) { }
        field(3; "Allocated Qty."; Decimal) { }
        field(4; "Period End"; Date) { }
    }

    keys
    {
        key(PK; "Period Type", "Period Start", "Period End") { Clustered = true; }
    }
}

page 97175 "LT PR NR ThreeKeyFields Part"
{
    PageType = ListPart;
    SourceTable = "LT PR NR ThreeKeyFields Line";
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

page 97176 "LT PR NR ThreeKeyFields Host"
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
            part(Load; "LT PR NR ThreeKeyFields Part") { ApplicationArea = All; }
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

table 97177 "LT PR NR FilterFirstKey Line"
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
        key(PK; "Period Start", "Period Type") { Clustered = true; }
    }
}

page 97178 "LT PR NR FilterFirstKey Part"
{
    PageType = ListPart;
    SourceTable = "LT PR NR FilterFirstKey Line";
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

page 97179 "LT PR NR FilterFirstKey Host"
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
            part(Load; "LT PR NR FilterFirstKey Part") { ApplicationArea = All; }
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

codeunit 97180 "LT PR NR Facts"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;

    local procedure Initialize()
    var
        Op: Record "LT PR Op";
        Booking: Record "LT PR Booking";
    begin
        Op.DeleteAll();
        Booking.DeleteAll();
    end;

    local procedure AddOp(No: Code[20]; ResourceNo: Code[20])
    var
        Op: Record "LT PR Op";
    begin
        Op."No." := No;
        Op."Resource No." := ResourceNo;
        Op.Insert();
    end;

    local procedure AddBooking(EntryNo: Integer; ResourceNo: Code[20]; Day: Date; Qty: Decimal)
    var
        Booking: Record "LT PR Booking";
    begin
        Booking."Entry No." := EntryNo;
        Booking."Resource No." := ResourceNo;
        Booking.Day := Day;
        Booking.Qty := Qty;
        Booking.Insert();
    end;

    local procedure TheDay(): Date
    begin
        exit(DMY2Date(15, 6, 2026));
    end;

    local procedure Seed()
    begin
        AddOp('OP1', 'R1');
        AddBooking(1, 'R1', TheDay(), 10);
        AddBooking(2, 'R1', TheDay() + 1, 5);
    end;

    local procedure Fact(Name: Text; Found: Boolean; PeriodStart: Date; Qty: Decimal): Text
    var
        FoundTxt: Text;
    begin
        FoundTxt := 'F';
        if Found then
            FoundTxt := 'T';
        if PeriodStart = 0D then
            exit(Name + '=' + FoundTxt + ':blank:' + Format(Qty, 0, 9) + ';');
        exit(Name + '=' + FoundTxt + ':' + Format(PeriodStart - TheDay()) + ':' + Format(Qty, 0, 9) + ';');
    end;


    [Test]
    procedure Fact_PartAction_First()
    var
        Op: Record "LT PR Op";
        Host: TestPage "LT PR NR PartAction Host";
        Found: Boolean;
        Seen: Text;
    begin
        Initialize();
        Seed();
        Op.Get('OP1');
        Host.OpenView();
        Host.GoToRecord(Op);
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen := Fact('before', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        AddBooking(3, 'R1', TheDay(), 20);
        Host.Refresh.Invoke();
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen += Fact('after', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        Host.Close();
        Assert.AreEqual('before=T:0:10;after=T:0:30;', Seen, 'Worksheet: Refresh, then Filter.SetFilter and First on the part');
    end;

    [Test]
    procedure Fact_RecFieldValidate_First()
    var
        Op: Record "LT PR Op";
        Host: TestPage "LT PR NR RecFieldValidate Host";
        Found: Boolean;
        Seen: Text;
    begin
        Initialize();
        Seed();
        Op.Get('OP1');
        Host.OpenView();
        Host.GoToRecord(Op);
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen := Fact('before', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        AddBooking(3, 'R1', TheDay(), 20);
        Host.Refresh.Invoke();
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen += Fact('after', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        Host.Close();
        Assert.AreEqual('before=T:0:10;after=T:0:30;', Seen, 'Worksheet: Refresh, then Filter.SetFilter and First on the part');
    end;

    [Test]
    procedure Fact_InsertAllowed_First()
    var
        Op: Record "LT PR Op";
        Host: TestPage "LT PR NR InsertAllowed Host";
        Found: Boolean;
        Seen: Text;
    begin
        Initialize();
        Seed();
        Op.Get('OP1');
        Host.OpenView();
        Host.GoToRecord(Op);
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen := Fact('before', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        AddBooking(3, 'R1', TheDay(), 20);
        Host.Refresh.Invoke();
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen += Fact('after', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        Host.Close();
        Assert.AreEqual('before=T:0:10;after=T:0:30;', Seen, 'Worksheet: Refresh, then Filter.SetFilter and First on the part');
    end;

    [Test]
    procedure Fact_UpdProp_First()
    var
        Op: Record "LT PR Op";
        Host: TestPage "LT PR NR UpdProp Host";
        Found: Boolean;
        Seen: Text;
    begin
        Initialize();
        Seed();
        Op.Get('OP1');
        Host.OpenView();
        Host.GoToRecord(Op);
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen := Fact('before', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        AddBooking(3, 'R1', TheDay(), 20);
        Host.Refresh.Invoke();
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen += Fact('after', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        Host.Close();
        Assert.AreEqual('before=T:0:10;after=T:0:30;', Seen, 'Worksheet: Refresh, then Filter.SetFilter and First on the part');
    end;

    [Test]
    procedure Fact_HostNewRecord_First()
    var
        Op: Record "LT PR Op";
        Host: TestPage "LT PR NR HostNewRecord Host";
        Found: Boolean;
        Seen: Text;
    begin
        Initialize();
        Seed();
        Op.Get('OP1');
        Host.OpenView();
        Host.GoToRecord(Op);
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen := Fact('before', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        AddBooking(3, 'R1', TheDay(), 20);
        Host.Refresh.Invoke();
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen += Fact('after', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        Host.Close();
        Assert.AreEqual('before=T:0:10;after=T:0:30;', Seen, 'Worksheet: Refresh, then Filter.SetFilter and First on the part');
    end;

    [Test]
    procedure Fact_ListPlusHost_First()
    var
        Op: Record "LT PR Op";
        Host: TestPage "LT PR NR ListPlusHost Host";
        Found: Boolean;
        Seen: Text;
    begin
        Initialize();
        Seed();
        Op.Get('OP1');
        Host.OpenView();
        Host.GoToRecord(Op);
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen := Fact('before', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        AddBooking(3, 'R1', TheDay(), 20);
        Host.Refresh.Invoke();
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen += Fact('after', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        Host.Close();
        Assert.AreEqual('before=T:0:10;after=T:0:30;', Seen, 'Worksheet: Refresh, then Filter.SetFilter and First on the part');
    end;

    [Test]
    procedure Fact_HiddenPart_First()
    var
        Op: Record "LT PR Op";
        Host: TestPage "LT PR NR HiddenPart Host";
        Found: Boolean;
        Seen: Text;
    begin
        Initialize();
        Seed();
        Op.Get('OP1');
        Host.OpenView();
        Host.GoToRecord(Op);
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen := Fact('before', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        AddBooking(3, 'R1', TheDay(), 20);
        Host.Refresh.Invoke();
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen += Fact('after', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        Host.Close();
        Assert.AreEqual('before=T:0:10;after=T:0:30;', Seen, 'Worksheet: Refresh, then Filter.SetFilter and First on the part');
    end;

    [Test]
    procedure Fact_ThreeKeyFields_First()
    var
        Op: Record "LT PR Op";
        Host: TestPage "LT PR NR ThreeKeyFields Host";
        Found: Boolean;
        Seen: Text;
    begin
        Initialize();
        Seed();
        Op.Get('OP1');
        Host.OpenView();
        Host.GoToRecord(Op);
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen := Fact('before', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        Seen += 'text=' + Host.Load.QtyText.Value() + ';scope=' + Host.Load.Scope.Value() + ';';
        AddBooking(3, 'R1', TheDay(), 20);
        Host.Refresh.Invoke();
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen += Fact('after', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        Seen += 'text=' + Host.Load.QtyText.Value() + ';scope=' + Host.Load.Scope.Value() + ';';
        Host.Close();
        Assert.AreEqual('before=T:0:10;text=agr10;scope=open;after=T:0:30;text=agr30;scope=open;', Seen, 'Worksheet, a part with OnOpenPage and OnAfterGetRecord over a two-field key (Period Type, Period Start): Refresh, then Filter.SetFilter on Period Start and First on the part');
    end;

    [Test]
    procedure Fact_FilterFirstKey_First()
    var
        Op: Record "LT PR Op";
        Host: TestPage "LT PR NR FilterFirstKey Host";
        Found: Boolean;
        Seen: Text;
    begin
        Initialize();
        Seed();
        Op.Get('OP1');
        Host.OpenView();
        Host.GoToRecord(Op);
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen := Fact('before', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        Seen += 'text=' + Host.Load.QtyText.Value() + ';scope=' + Host.Load.Scope.Value() + ';';
        AddBooking(3, 'R1', TheDay(), 20);
        Host.Refresh.Invoke();
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen += Fact('after', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        Seen += 'text=' + Host.Load.QtyText.Value() + ';scope=' + Host.Load.Scope.Value() + ';';
        Host.Close();
        Assert.AreEqual('before=T:0:10;text=agr10;scope=open;after=T:0:30;text=agr30;scope=open;', Seen, 'Worksheet, a part with OnOpenPage and OnAfterGetRecord over a two-field key (Period Type, Period Start): Refresh, then Filter.SetFilter on Period Start and First on the part');
    end;
}
