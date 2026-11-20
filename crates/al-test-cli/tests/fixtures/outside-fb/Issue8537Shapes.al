// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/testfilter/testfilter-setfilter-method
// Scope: in-scope
// local object: the hosts and parts each vary one fact of the refill measured in PartRefillTestFilter.al, which is what is under test
// Fixtures used: shared Assert (60021); from PartRefillTestFilter.al the tables "LT PR Op" (97100), "LT PR Booking" (97101),
//   "LT PR Load Line" (97102), codeunits "LT PR Load Calc" (97103) and "LT PR Load Task" (97108), page "LT PR Load Part" (97104);
//   Base Application table "Load Buffer" (933, temporary); own enums 90440-90441, pages 90442-90459, 90462, 90464-90466 and 90471-90476,
//   codeunit 90463, test codeunit 90461
// Probe (al-runner issue 8537, follow-up of PartRefillTestFilter.al): the measured Worksheet cell (a host's OnAfterGetCurrRecord makes
//   a ListPart on a temporary table delete and insert its rows through two part procedures, each ending with CurrPage.Update(false);
//   the Refresh action marks the part stale and calls CurrPage.Update(false); the test filters the part with Filter.SetFilter and
//   calls First before and after Refresh) with one fact varied per cell, toward a partner page that has all of them:
//   other parts on the host (a linked ListPart with UpdatePropagation = Both, two linked FactBoxes); Visible = a page variable on the
//   part control; more host code in OnAfterGetCurrRecord; a page background task with OnPageBackgroundTaskError; host view filters
//   (filter group 2) applied in Refresh; the part over a temporary Load Buffer (key Period Type, Period Start); a group with an
//   enum variable control (OnValidate) and a non-editable control above an Editable = false repeater; fields with OnDrillDown and
//   StyleExpr, OnOpenPage and OnAfterGetRecord on the part; part procedures with an enum parameter; the part handing Rec by var to a
//   codeunit that inserts the rows (and clearing them in a local procedure); OnOpenPage and OnAfterGetRecord on the host; other parts
//   with code (a linked ListPart with record triggers, a ListPart FactBox with OnAfterGetRecord, a CardPart FactBox with its own page
//   background task); a part page without Editable = false (Extensible and LinksAllowed false) whose repeater is
//   Editable = false; and all of them together, with First, First-Next-Previous and Last. Results are `name=Found:Period Start - TheDay:Allocated Qty.`; every cell finds the
//   refilled row (30). Measured on a local BC 29.1 OnPrem w1 container (16/16 green); BC 27.5, 28.5 and 29.0 are left to CI.
// BC versions: 27, 28, 29
enum 90440 "LT PS Load Level"
{
    Extensible = false;
    value(0; Resource) { }
    value(1; Group) { }
}

enum 90441 "LT PS Res Type"
{
    Extensible = false;
    value(0; Work) { }
    value(1; Machine) { }
}

codeunit 90463 "LT PS Filler"
{
    procedure FillLines(Results: Dictionary of [Text, Text]; var Lines: Record "LT PR Load Line" temporary)
    var
        DayKey: Text;
    begin
        foreach DayKey in Results.Keys() do begin
            Lines.Init();
            Evaluate(Lines."Period Start", DayKey, 9);
            Evaluate(Lines."Allocated Qty.", Results.Get(DayKey), 9);
            Lines.Insert();
        end;
    end;

    procedure FillBuffer(Results: Dictionary of [Text, Text]; var Lines: Record "Load Buffer" temporary)
    var
        DayKey: Text;
    begin
        foreach DayKey in Results.Keys() do begin
            Lines.Init();
            Lines."Period Type" := Lines."Period Type"::Day;
            Evaluate(Lines."Period Start", DayKey, 9);
            Evaluate(Lines."Allocated Qty.", Results.Get(DayKey), 9);
            Lines.Insert();
        end;
    end;
}

page 90442 "LT PS Op Lines"
{
    PageType = ListPart;
    SourceTable = "LT PR Op";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("No."; Rec."No.") { ApplicationArea = All; }
            }
        }
    }

    var
        Touched: Integer;

    procedure Touch()
    begin
        Touched += 1;
    end;
}

page 90443 "LT PS Op FB A"
{
    PageType = CardPart;
    SourceTable = "LT PR Op";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            field("Resource No."; Rec."Resource No.") { ApplicationArea = All; }
        }
    }
}

page 90444 "LT PS Op FB B"
{
    PageType = CardPart;
    SourceTable = "LT PR Op";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { ApplicationArea = All; }
        }
    }
}
page 90471 "LT PS Op Lines Code"
{
    PageType = ListPart;
    SourceTable = "LT PR Op";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("No."; Rec."No.") { ApplicationArea = All; }
            }
        }
    }

    trigger OnAfterGetRecord()
    begin
        Reads += 1;
    end;

    trigger OnInsertRecord(BelowxRec: Boolean): Boolean
    begin
        exit(true);
    end;

    trigger OnModifyRecord(): Boolean
    begin
        exit(true);
    end;

    trigger OnDeleteRecord(): Boolean
    begin
        exit(true);
    end;

    var
        Reads: Integer;
        Touched: Integer;

    procedure Touch()
    begin
        Touched += 1;
    end;
}

page 90472 "LT PS Op FB List"
{
    PageType = ListPart;
    SourceTable = "LT PR Op";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("Resource No."; Rec."Resource No.") { ApplicationArea = All; }
            }
        }
    }

    trigger OnAfterGetRecord()
    begin
        Reads += 1;
    end;

    var
        Reads: Integer;
}

page 90473 "LT PS Op FB Bg"
{
    PageType = CardPart;
    SourceTable = "LT PR Op";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { ApplicationArea = All; }
            field(Info; Info) { ApplicationArea = All; }
        }
    }

    trigger OnAfterGetCurrRecord()
    var
        Params: Dictionary of [Text, Text];
    begin
        if TaskId <> 0 then
            CurrPage.CancelBackgroundTask(TaskId);
        Params.Add('Resource', Rec."Resource No.");
        CurrPage.EnqueueBackgroundTask(TaskId, Codeunit::"LT PR Load Task", Params, 120000, PageBackgroundTaskErrorLevel::Warning);
    end;

    trigger OnPageBackgroundTaskCompleted(CompletedTaskId: Integer; Results: Dictionary of [Text, Text])
    begin
        if CompletedTaskId <> TaskId then
            exit;
        TaskId := 0;
        Info := Format(Results.Count());
    end;

    trigger OnPageBackgroundTaskError(FailedTaskId: Integer; ErrorCode: Text; ErrorText: Text; ErrorCallStack: Text; var IsHandled: Boolean)
    begin
        IsHandled := true;
        TaskId := 0;
    end;

    var
        TaskId: Integer;
        Info: Text;
}

page 90445 "LT PS Parts Host"
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
            part(Lines; "LT PS Op Lines")
            {
                ApplicationArea = All;
                SubPageLink = "No." = field("No.");
                UpdatePropagation = Both;
            }
            part(Load; "LT PR Load Part")
            {
                ApplicationArea = All;
            }
        }
        area(FactBoxes)
        {
            part(InfoA; "LT PS Op FB A")
            {
                ApplicationArea = All;
                SubPageLink = "No." = field("No.");
            }
            part(InfoB; "LT PS Op FB B")
            {
                ApplicationArea = All;
                SubPageLink = "No." = field("No.");
            }
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
    begin
        LoadNow();
    end;

    local procedure LoadNow()
    var
        LoadCalc: Codeunit "LT PR Load Calc";
    begin
        if CurrPage.Load.Page.IsLoadedFor(Rec."Resource No.") then
            exit;
        CurrPage.Load.Page.ClearLines(Rec."Resource No.");
        CurrPage.Load.Page.SetLoadLines(LoadCalc.Calculate(Rec."Resource No."));
    end;
}

page 90446 "LT PS Visible Host"
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
            part(Load; "LT PR Load Part")
            {
                ApplicationArea = All;
                Visible = ShowLoad;
            }
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

    trigger OnOpenPage()
    begin
        ShowLoad := true;
    end;

    trigger OnAfterGetCurrRecord()
    begin
        if ShowLoad then
            LoadNow();
    end;

    local procedure LoadNow()
    var
        LoadCalc: Codeunit "LT PR Load Calc";
    begin
        if CurrPage.Load.Page.IsLoadedFor(Rec."Resource No.") then
            exit;
        CurrPage.Load.Page.ClearLines(Rec."Resource No.");
        CurrPage.Load.Page.SetLoadLines(LoadCalc.Calculate(Rec."Resource No."));
    end;
}

page 90447 "LT PS Code Host"
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
            part(Load; "LT PR Load Part")
            {
                ApplicationArea = All;
            }
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
        IsMachine: Boolean;
        CurrentCount: Integer;

    trigger OnAfterGetCurrRecord()
    begin
        IsMachine := Rec."Resource No." <> '';
        CurrentCount += 1;
        LoadNow();
    end;

    local procedure LoadNow()
    var
        LoadCalc: Codeunit "LT PR Load Calc";
    begin
        if CurrPage.Load.Page.IsLoadedFor(Rec."Resource No.") then
            exit;
        CurrPage.Load.Page.ClearLines(Rec."Resource No.");
        CurrPage.Load.Page.SetLoadLines(LoadCalc.Calculate(Rec."Resource No."));
    end;
}

page 90448 "LT PS BgErr Host"
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
            part(Load; "LT PS BgErr Part")
            {
                ApplicationArea = All;
            }
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
    begin
        StartLoad();
    end;

    trigger OnPageBackgroundTaskCompleted(TaskId: Integer; Results: Dictionary of [Text, Text])
    begin
        if TaskId <> LoadTaskId then
            exit;
        LoadTaskId := 0;
        CurrPage.Load.Page.SetLoadLines(Results);
    end;

    trigger OnPageBackgroundTaskError(TaskId: Integer; ErrorCode: Text; ErrorText: Text; ErrorCallStack: Text; var IsHandled: Boolean)
    begin
        if TaskId <> LoadTaskId then
            exit;
        LoadTaskId := 0;
        IsHandled := true;
        CurrPage.Load.Page.LoadFailed();
    end;

    local procedure StartLoad()
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
}

page 90462 "LT PS BgErr Part"
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

    procedure LoadFailed()
    begin
        LinesLoaded := true;
        CurrPage.Update(false);
    end;
}

page 90449 "LT PS Filter Host"
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
            part(Load; "LT PR Load Part")
            {
                ApplicationArea = All;
            }
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
                    ApplyViewFilters();
                    CurrPage.Update(false);
                end;
            }
        }
    }

    trigger OnAfterGetCurrRecord()
    begin
        LoadNow();
    end;

    local procedure LoadNow()
    var
        LoadCalc: Codeunit "LT PR Load Calc";
    begin
        if CurrPage.Load.Page.IsLoadedFor(Rec."Resource No.") then
            exit;
        CurrPage.Load.Page.ClearLines(Rec."Resource No.");
        CurrPage.Load.Page.SetLoadLines(LoadCalc.Calculate(Rec."Resource No."));
    end;

    local procedure ApplyViewFilters()
    begin
        Rec.FilterGroup(2);
        Rec.SetFilter("Resource No.", '<>%1', 'ZZ');
        Rec.FilterGroup(0);
    end;
}

page 90450 "LT PS Buffer Part"
{
    PageType = ListPart;
    SourceTable = "Load Buffer";
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

page 90451 "LT PS Buffer Host"
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
            part(Load; "LT PS Buffer Part")
            {
                ApplicationArea = All;
            }
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
    begin
        LoadNow();
    end;

    local procedure LoadNow()
    var
        LoadCalc: Codeunit "LT PR Load Calc";
    begin
        if CurrPage.Load.Page.IsLoadedFor(Rec."Resource No.") then
            exit;
        CurrPage.Load.Page.ClearLines(Rec."Resource No.");
        CurrPage.Load.Page.SetLoadLines(LoadCalc.Calculate(Rec."Resource No."));
    end;
}

page 90452 "LT PS Group Part"
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
            group(LevelSelector)
            {
                ShowCaption = false;

                field(ShowLoadFor; LoadLevel)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    begin
                        LinesLoaded := false;
                    end;
                }
                field(Scope; ScopeText)
                {
                    ApplicationArea = All;
                    Editable = false;
                }
            }
            repeater(Lines)
            {
                Editable = false;

                field("Period Start"; Rec."Period Start") { ApplicationArea = All; }
                field("Allocated Qty."; Rec."Allocated Qty.") { ApplicationArea = All; }
            }
        }
    }

    trigger OnOpenPage()
    begin
        ScopeText := 'open';
    end;

    trigger OnAfterGetRecord()
    begin
        LoadText := 'agr' + Format(Rec."Allocated Qty.", 0, 9);
        LoadStyle := 'Standard';
        if Rec."Allocated Qty." > 20 then
            LoadStyle := 'Unfavorable';
    end;

    var
        ResourceNo: Code[20];
        LinesLoaded: Boolean;
        LoadLevel: Enum "LT PS Load Level";
        ScopeText: Text;
        LoadText: Text;
        LoadStyle: Text;

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

page 90453 "LT PS Group Host"
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
            part(Load; "LT PS Group Part")
            {
                ApplicationArea = All;
            }
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
    begin
        LoadNow();
    end;

    local procedure LoadNow()
    var
        LoadCalc: Codeunit "LT PR Load Calc";
    begin
        if CurrPage.Load.Page.IsLoadedFor(Rec."Resource No.") then
            exit;
        CurrPage.Load.Page.ClearLines(Rec."Resource No.");
        CurrPage.Load.Page.SetLoadLines(LoadCalc.Calculate(Rec."Resource No."));
    end;
}

page 90454 "LT PS Drill Part"
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
                field("Allocated Qty."; Rec."Allocated Qty.")
                {
                    ApplicationArea = All;
                    StyleExpr = LoadStyle;

                    trigger OnDrillDown()
                    begin
                        DrillCount += 1;
                    end;
                }
                field(LoadText; LoadText) { ApplicationArea = All; StyleExpr = LoadStyle; }
            }
        }
    }

    trigger OnOpenPage()
    begin
        ScopeText := 'open';
    end;

    trigger OnAfterGetRecord()
    begin
        LoadText := 'agr' + Format(Rec."Allocated Qty.", 0, 9);
        LoadStyle := 'Standard';
        if Rec."Allocated Qty." > 20 then
            LoadStyle := 'Unfavorable';
    end;

    var
        ResourceNo: Code[20];
        LinesLoaded: Boolean;
        ScopeText: Text;
        LoadText: Text;
        LoadStyle: Text;
        DrillCount: Integer;

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

page 90455 "LT PS Drill Host"
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
            part(Load; "LT PS Drill Part")
            {
                ApplicationArea = All;
            }
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
    begin
        LoadNow();
    end;

    local procedure LoadNow()
    var
        LoadCalc: Codeunit "LT PR Load Calc";
    begin
        if CurrPage.Load.Page.IsLoadedFor(Rec."Resource No.") then
            exit;
        CurrPage.Load.Page.ClearLines(Rec."Resource No.");
        CurrPage.Load.Page.SetLoadLines(LoadCalc.Calculate(Rec."Resource No."));
    end;
}

page 90456 "LT PS Enum Part"
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
        ResourceType: Enum "LT PS Res Type";

    procedure IsLoadedFor(NewType: Enum "LT PS Res Type"; NewResourceNo: Code[20]): Boolean
    begin
        exit(LinesLoaded and (NewType = ResourceType) and (NewResourceNo = ResourceNo));
    end;

    procedure Invalidate()
    begin
        LinesLoaded := false;
    end;

    procedure ClearLines(NewType: Enum "LT PS Res Type"; NewResourceNo: Code[20])
    begin
        ResourceType := NewType;
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

page 90457 "LT PS Enum Host"
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
            part(Load; "LT PS Enum Part")
            {
                ApplicationArea = All;
            }
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
    begin
        LoadNow();
    end;

    local procedure LoadNow()
    var
        LoadCalc: Codeunit "LT PR Load Calc";
    begin
        if CurrPage.Load.Page.IsLoadedFor("LT PS Res Type"::Machine, Rec."Resource No.") then
            exit;
        CurrPage.Load.Page.ClearLines("LT PS Res Type"::Machine, Rec."Resource No.");
        CurrPage.Load.Page.SetLoadLines(LoadCalc.Calculate(Rec."Resource No."));
    end;
}

page 90464 "LT PS Filler Part"
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
        ClearLoadLines();
        CurrPage.Update(false);
    end;

    procedure SetLoadLines(Results: Dictionary of [Text, Text])
    var
        Filler: Codeunit "LT PS Filler";
    begin
        Filler.FillLines(Results, Rec);
        LinesLoaded := true;
        if Rec.FindFirst() then;
        CurrPage.Update(false);
    end;

    local procedure ClearLoadLines()
    begin
        Rec.Reset();
        Rec.DeleteAll(false);
    end;
}

page 90465 "LT PS Filler Host"
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
            part(Load; "LT PS Filler Part")
            {
                ApplicationArea = All;
            }
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
    begin
        LoadNow();
    end;

    local procedure LoadNow()
    var
        LoadCalc: Codeunit "LT PR Load Calc";
    begin
        if CurrPage.Load.Page.IsLoadedFor(Rec."Resource No.") then
            exit;
        CurrPage.Load.Page.ClearLines(Rec."Resource No.");
        CurrPage.Load.Page.SetLoadLines(LoadCalc.Calculate(Rec."Resource No."));
    end;
}

page 90466 "LT PS Trig Host"
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
            part(Load; "LT PR Load Part")
            {
                ApplicationArea = All;
            }
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
        RowStyle: Text;
        Opened: Boolean;

    trigger OnOpenPage()
    begin
        Opened := true;
    end;

    trigger OnAfterGetCurrRecord()
    begin
        LoadNow();
    end;

    local procedure LoadNow()
    var
        LoadCalc: Codeunit "LT PR Load Calc";
    begin
        if CurrPage.Load.Page.IsLoadedFor(Rec."Resource No.") then
            exit;
        CurrPage.Load.Page.ClearLines(Rec."Resource No.");
        CurrPage.Load.Page.SetLoadLines(LoadCalc.Calculate(Rec."Resource No."));
    end;

    trigger OnAfterGetRecord()
    begin
        RowStyle := 'Standard';
        if Rec."Resource No." = '' then
            RowStyle := 'Unfavorable';
    end;
}

page 90458 "LT PS Full Part"
{
    PageType = ListPart;
    SourceTable = "Load Buffer";
    SourceTableTemporary = true;
    InsertAllowed = false;
    DeleteAllowed = false;
    Extensible = false;
    LinksAllowed = false;
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            group(LevelSelector)
            {
                ShowCaption = false;

                field(ShowLoadFor; LoadLevel)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    begin
                        LinesLoaded := false;
                    end;
                }
                field(Scope; ScopeText)
                {
                    ApplicationArea = All;
                    Editable = false;
                }
            }
            repeater(Lines)
            {
                Editable = false;

                field("Period Start"; Rec."Period Start") { ApplicationArea = All; }
                field("Allocated Qty."; Rec."Allocated Qty.")
                {
                    ApplicationArea = All;
                    StyleExpr = LoadStyle;

                    trigger OnDrillDown()
                    begin
                        DrillCount += 1;
                    end;
                }
                field(LoadText; LoadText) { ApplicationArea = All; StyleExpr = LoadStyle; }
            }
        }
    }

    trigger OnOpenPage()
    begin
        ScopeText := 'open';
    end;

    trigger OnAfterGetRecord()
    begin
        LoadText := 'agr' + Format(Rec."Allocated Qty.", 0, 9);
        LoadStyle := 'Standard';
        if Rec."Allocated Qty." > 20 then
            LoadStyle := 'Unfavorable';
    end;

    var
        ResourceNo: Code[20];
        LinesLoaded: Boolean;
        ResourceType: Enum "LT PS Res Type";
        LoadLevel: Enum "LT PS Load Level";
        ScopeText: Text;
        LoadText: Text;
        LoadStyle: Text;
        DrillCount: Integer;

    procedure IsLoadedFor(NewType: Enum "LT PS Res Type"; NewResourceNo: Code[20]): Boolean
    begin
        exit(LinesLoaded and (NewType = ResourceType) and (NewResourceNo = ResourceNo));
    end;

    procedure Invalidate()
    begin
        LinesLoaded := false;
    end;

    procedure ClearLines(NewType: Enum "LT PS Res Type"; NewResourceNo: Code[20])
    begin
        ResourceType := NewType;
        ResourceNo := NewResourceNo;
        LinesLoaded := false;
        ClearLoadLines();
        CurrPage.Update(false);
    end;

    procedure SetLoadLines(Results: Dictionary of [Text, Text])
    var
        Filler: Codeunit "LT PS Filler";
    begin
        Filler.FillBuffer(Results, Rec);
        LinesLoaded := true;
        if Rec.FindFirst() then;
        CurrPage.Update(false);
    end;

    procedure TaskParameters() Params: Dictionary of [Text, Text]
    begin
        Params.Add('Resource', ResourceNo);
    end;

    procedure LoadFailed()
    begin
        LinesLoaded := true;
        CurrPage.Update(false);
    end;

    local procedure ClearLoadLines()
    begin
        Rec.Reset();
        Rec.DeleteAll(false);
    end;
}

page 90475 "LT PS Editable Part"
{
    PageType = ListPart;
    SourceTable = "LT PR Load Line";
    SourceTableTemporary = true;
    InsertAllowed = false;
    DeleteAllowed = false;
    Extensible = false;
    LinksAllowed = false;
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                Editable = false;

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

page 90476 "LT PS Editable Host"
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
            part(Load; "LT PS Editable Part")
            {
                ApplicationArea = All;
            }
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
    begin
        LoadNow();
    end;

    local procedure LoadNow()
    var
        LoadCalc: Codeunit "LT PR Load Calc";
    begin
        if CurrPage.Load.Page.IsLoadedFor(Rec."Resource No.") then
            exit;
        CurrPage.Load.Page.ClearLines(Rec."Resource No.");
        CurrPage.Load.Page.SetLoadLines(LoadCalc.Calculate(Rec."Resource No."));
    end;
}

page 90474 "LT PS PartsCode Host"
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
            part(Lines; "LT PS Op Lines Code")
            {
                ApplicationArea = All;
                SubPageLink = "No." = field("No.");
                UpdatePropagation = Both;
            }
            part(Load; "LT PR Load Part")
            {
                ApplicationArea = All;
            }
        }
        area(FactBoxes)
        {
            part(InfoA; "LT PS Op FB List")
            {
                ApplicationArea = All;
                SubPageLink = "No." = field("No.");
            }
            part(InfoB; "LT PS Op FB Bg")
            {
                ApplicationArea = All;
                SubPageLink = "No." = field("No.");
            }
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
        IsMachine: Boolean;
        CurrentCount: Integer;

    trigger OnAfterGetCurrRecord()
    begin
        IsMachine := Rec."Resource No." <> '';
        CurrentCount += 1;
        CurrPage.Lines.Page.Touch();
        LoadNow();
    end;

    local procedure LoadNow()
    var
        LoadCalc: Codeunit "LT PR Load Calc";
    begin
        if CurrPage.Load.Page.IsLoadedFor(Rec."Resource No.") then
            exit;
        CurrPage.Load.Page.ClearLines(Rec."Resource No.");
        CurrPage.Load.Page.SetLoadLines(LoadCalc.Calculate(Rec."Resource No."));
    end;
}

page 90459 "LT PS Full Host"
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
            part(Lines; "LT PS Op Lines Code")
            {
                ApplicationArea = All;
                SubPageLink = "No." = field("No.");
                UpdatePropagation = Both;
            }
            part(Load; "LT PS Full Part")
            {
                ApplicationArea = All;
                Visible = ShowLoad;
            }
        }
        area(FactBoxes)
        {
            part(InfoA; "LT PS Op FB List")
            {
                ApplicationArea = All;
                SubPageLink = "No." = field("No.");
            }
            part(InfoB; "LT PS Op FB Bg")
            {
                ApplicationArea = All;
                SubPageLink = "No." = field("No.");
            }
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
                    ApplyViewFilters();
                    CurrPage.Update(false);
                end;
            }
        }
    }

    var
        ShowLoad: Boolean;
        IsMachine: Boolean;
        CurrentCount: Integer;
        LoadTaskId: Integer;
        RowStyle: Text;
        Opened: Boolean;

    trigger OnOpenPage()
    begin
        ShowLoad := true;
        Opened := true;
    end;

    trigger OnAfterGetCurrRecord()
    begin
        IsMachine := Rec."Resource No." <> '';
        CurrentCount += 1;
        CurrPage.Lines.Page.Touch();
        if ShowLoad then
            StartLoad();
    end;

    trigger OnPageBackgroundTaskCompleted(TaskId: Integer; Results: Dictionary of [Text, Text])
    begin
        if TaskId <> LoadTaskId then
            exit;
        LoadTaskId := 0;
        CurrPage.Load.Page.SetLoadLines(Results);
    end;

    trigger OnPageBackgroundTaskError(TaskId: Integer; ErrorCode: Text; ErrorText: Text; ErrorCallStack: Text; var IsHandled: Boolean)
    begin
        if TaskId <> LoadTaskId then
            exit;
        LoadTaskId := 0;
        IsHandled := true;
        CurrPage.Load.Page.LoadFailed();
    end;

    local procedure StartLoad()
    var
        Params: Dictionary of [Text, Text];
    begin
        if CurrPage.Load.Page.IsLoadedFor("LT PS Res Type"::Machine, Rec."Resource No.") then
            exit;
        if LoadTaskId <> 0 then
            CurrPage.CancelBackgroundTask(LoadTaskId);
        LoadTaskId := 0;
        CurrPage.Load.Page.ClearLines("LT PS Res Type"::Machine, Rec."Resource No.");
        Params := CurrPage.Load.Page.TaskParameters();
        CurrPage.EnqueueBackgroundTask(LoadTaskId, Codeunit::"LT PR Load Task", Params, 120000, PageBackgroundTaskErrorLevel::Warning);
    end;

    trigger OnAfterGetRecord()
    begin
        RowStyle := 'Standard';
        if Rec."Resource No." = '' then
            RowStyle := 'Unfavorable';
    end;

    local procedure ApplyViewFilters()
    begin
        Rec.FilterGroup(2);
        Rec.SetFilter("Resource No.", '<>%1', 'ZZ');
        Rec.FilterGroup(0);
    end;
}

codeunit 90461 "LT PS Tests"
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

    local procedure AddBooking(EntryNo: Integer; Day: Date; Qty: Decimal)
    var
        Booking: Record "LT PR Booking";
    begin
        Booking."Entry No." := EntryNo;
        Booking."Resource No." := 'R1';
        Booking.Day := Day;
        Booking.Qty := Qty;
        Booking.Insert();
    end;

    local procedure TheDay(): Date
    begin
        exit(DMY2Date(15, 6, 2026));
    end;

    local procedure Seed(var Op: Record "LT PR Op")
    begin
        Op."No." := 'OP1';
        Op."Resource No." := 'R1';
        Op.Insert();
        AddBooking(1, TheDay(), 10);
        AddBooking(2, TheDay() + 1, 5);
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
    procedure PartRefillShape_OtherPartsHost_FindsRefilledRow()
    var
        Op: Record "LT PR Op";
        Host: TestPage "LT PS Parts Host";
        Found: Boolean;
        Seen: Text;
    begin
        Initialize();
        Seed(Op);
        Host.OpenView();
        Host.GoToRecord(Op);
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen := Fact('before', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        AddBooking(3, TheDay(), 20);
        Host.Refresh.Invoke();
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen += Fact('after', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        Host.Close();
        Assert.AreEqual('before=T:0:10;after=T:0:30;', Seen, 'a host with a linked ListPart (UpdatePropagation = Both) and two linked FactBoxes: Refresh, then Filter.SetFilter and first on the part');
    end;

    [Test]
    procedure PartRefillShape_VisibleVariable_FindsRefilledRow()
    var
        Op: Record "LT PR Op";
        Host: TestPage "LT PS Visible Host";
        Found: Boolean;
        Seen: Text;
    begin
        Initialize();
        Seed(Op);
        Host.OpenView();
        Host.GoToRecord(Op);
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen := Fact('before', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        AddBooking(3, TheDay(), 20);
        Host.Refresh.Invoke();
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen += Fact('after', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        Host.Close();
        Assert.AreEqual('before=T:0:10;after=T:0:30;', Seen, 'the Load part control with Visible = a page variable OnOpenPage sets: Refresh, then Filter.SetFilter and first on the part');
    end;

    [Test]
    procedure PartRefillShape_MoreHostCode_FindsRefilledRow()
    var
        Op: Record "LT PR Op";
        Host: TestPage "LT PS Code Host";
        Found: Boolean;
        Seen: Text;
    begin
        Initialize();
        Seed(Op);
        Host.OpenView();
        Host.GoToRecord(Op);
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen := Fact('before', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        AddBooking(3, TheDay(), 20);
        Host.Refresh.Invoke();
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen += Fact('after', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        Host.Close();
        Assert.AreEqual('before=T:0:10;after=T:0:30;', Seen, 'more code in the host OnAfterGetCurrRecord before the load: Refresh, then Filter.SetFilter and first on the part');
    end;

    [Test]
    procedure PartRefillShape_BackgroundErrorTrigger_FindsRefilledRow()
    var
        Op: Record "LT PR Op";
        Host: TestPage "LT PS BgErr Host";
        Found: Boolean;
        Seen: Text;
    begin
        Initialize();
        Seed(Op);
        Commit();
        Host.OpenView();
        Host.GoToRecord(Op);
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen := Fact('before', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        AddBooking(3, TheDay(), 20);
        Commit();
        Host.Refresh.Invoke();
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen += Fact('after', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        Host.Close();
        Assert.AreEqual('before=T:0:10;after=T:0:30;', Seen, 'a page background task with OnPageBackgroundTaskError and a LoadFailed procedure: Refresh, then Filter.SetFilter and first on the part');
    end;

    [Test]
    procedure PartRefillShape_ViewFiltersInRefresh_FindsRefilledRow()
    var
        Op: Record "LT PR Op";
        Host: TestPage "LT PS Filter Host";
        Found: Boolean;
        Seen: Text;
    begin
        Initialize();
        Seed(Op);
        Host.OpenView();
        Host.GoToRecord(Op);
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen := Fact('before', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        AddBooking(3, TheDay(), 20);
        Host.Refresh.Invoke();
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen += Fact('after', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        Host.Close();
        Assert.AreEqual('before=T:0:10;after=T:0:30;', Seen, 'Refresh applies host view filters (filter group 2) before CurrPage.Update(false): Refresh, then Filter.SetFilter and first on the part');
    end;

    [Test]
    procedure PartRefillShape_LoadBufferPart_FindsRefilledRow()
    var
        Op: Record "LT PR Op";
        Host: TestPage "LT PS Buffer Host";
        Found: Boolean;
        Seen: Text;
    begin
        Initialize();
        Seed(Op);
        Host.OpenView();
        Host.GoToRecord(Op);
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen := Fact('before', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        AddBooking(3, TheDay(), 20);
        Host.Refresh.Invoke();
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen += Fact('after', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        Host.Close();
        Assert.AreEqual('before=T:0:10;after=T:0:30;', Seen, 'the part over a temporary Load Buffer (Base Application), key Period Type, Period Start: Refresh, then Filter.SetFilter and first on the part');
    end;

    [Test]
    procedure PartRefillShape_GroupAboveRepeater_FindsRefilledRow()
    var
        Op: Record "LT PR Op";
        Host: TestPage "LT PS Group Host";
        Found: Boolean;
        Seen: Text;
    begin
        Initialize();
        Seed(Op);
        Host.OpenView();
        Host.GoToRecord(Op);
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen := Fact('before', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        AddBooking(3, TheDay(), 20);
        Host.Refresh.Invoke();
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen += Fact('after', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        Host.Close();
        Assert.AreEqual('before=T:0:10;after=T:0:30;', Seen, 'a group with an enum variable control (OnValidate) and a non-editable control above an Editable = false repeater: Refresh, then Filter.SetFilter and first on the part');
    end;

    [Test]
    procedure PartRefillShape_DrillDownAndStyle_FindsRefilledRow()
    var
        Op: Record "LT PR Op";
        Host: TestPage "LT PS Drill Host";
        Found: Boolean;
        Seen: Text;
    begin
        Initialize();
        Seed(Op);
        Host.OpenView();
        Host.GoToRecord(Op);
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen := Fact('before', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        AddBooking(3, TheDay(), 20);
        Host.Refresh.Invoke();
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen += Fact('after', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        Host.Close();
        Assert.AreEqual('before=T:0:10;after=T:0:30;', Seen, 'fields with OnDrillDown and StyleExpr, OnOpenPage and OnAfterGetRecord on the part: Refresh, then Filter.SetFilter and first on the part');
    end;

    [Test]
    procedure PartRefillShape_EnumParameters_FindsRefilledRow()
    var
        Op: Record "LT PR Op";
        Host: TestPage "LT PS Enum Host";
        Found: Boolean;
        Seen: Text;
    begin
        Initialize();
        Seed(Op);
        Host.OpenView();
        Host.GoToRecord(Op);
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen := Fact('before', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        AddBooking(3, TheDay(), 20);
        Host.Refresh.Invoke();
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen += Fact('after', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        Host.Close();
        Assert.AreEqual('before=T:0:10;after=T:0:30;', Seen, 'part procedures with an enum and a Code parameter: Refresh, then Filter.SetFilter and first on the part');
    end;

    [Test]
    procedure PartRefillShape_RecHandedToCodeunit_FindsRefilledRow()
    var
        Op: Record "LT PR Op";
        Host: TestPage "LT PS Filler Host";
        Found: Boolean;
        Seen: Text;
    begin
        Initialize();
        Seed(Op);
        Host.OpenView();
        Host.GoToRecord(Op);
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen := Fact('before', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        AddBooking(3, TheDay(), 20);
        Host.Refresh.Invoke();
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen += Fact('after', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        Host.Close();
        Assert.AreEqual('before=T:0:10;after=T:0:30;', Seen, 'the part hands Rec by var to a codeunit that inserts the rows, and clears them in a local procedure: Refresh, then Filter.SetFilter and first on the part');
    end;

    [Test]
    procedure PartRefillShape_HostRowTriggers_FindsRefilledRow()
    var
        Op: Record "LT PR Op";
        Host: TestPage "LT PS Trig Host";
        Found: Boolean;
        Seen: Text;
    begin
        Initialize();
        Seed(Op);
        Host.OpenView();
        Host.GoToRecord(Op);
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen := Fact('before', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        AddBooking(3, TheDay(), 20);
        Host.Refresh.Invoke();
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen += Fact('after', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        Host.Close();
        Assert.AreEqual('before=T:0:10;after=T:0:30;', Seen, 'OnOpenPage and OnAfterGetRecord on the host: Refresh, then Filter.SetFilter and first on the part');
    end;

    [Test]
    procedure PartRefillShape_OtherPartsWithCode_FindsRefilledRow()
    var
        Op: Record "LT PR Op";
        Host: TestPage "LT PS PartsCode Host";
        Found: Boolean;
        Seen: Text;
    begin
        Initialize();
        Seed(Op);
        Commit();
        Host.OpenView();
        Host.GoToRecord(Op);
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen := Fact('before', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        AddBooking(3, TheDay(), 20);
        Commit();
        Host.Refresh.Invoke();
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen += Fact('after', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        Host.Close();
        Assert.AreEqual('before=T:0:10;after=T:0:30;', Seen, 'other parts with code: a linked ListPart with record triggers, a ListPart FactBox with OnAfterGetRecord, a CardPart FactBox with its own page background task; the host calls a procedure of the ListPart: Refresh, then Filter.SetFilter and first on the part');
    end;

    [Test]
    procedure PartRefillShape_EditablePagePart_FindsRefilledRow()
    var
        Op: Record "LT PR Op";
        Host: TestPage "LT PS Editable Host";
        Found: Boolean;
        Seen: Text;
    begin
        Initialize();
        Seed(Op);
        Host.OpenView();
        Host.GoToRecord(Op);
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen := Fact('before', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        AddBooking(3, TheDay(), 20);
        Host.Refresh.Invoke();
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen += Fact('after', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        Host.Close();
        Assert.AreEqual('before=T:0:10;after=T:0:30;', Seen, 'the part page without Editable = false (Extensible and LinksAllowed false), its repeater Editable = false: Refresh, then Filter.SetFilter and first on the part');
    end;

    [Test]
    procedure PartRefillShape_AllTogether_FindsRefilledRow()
    var
        Op: Record "LT PR Op";
        Host: TestPage "LT PS Full Host";
        Found: Boolean;
        Seen: Text;
    begin
        Initialize();
        Seed(Op);
        Commit();
        Host.OpenView();
        Host.GoToRecord(Op);
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen := Fact('before', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        AddBooking(3, TheDay(), 20);
        Commit();
        Host.Refresh.Invoke();
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen += Fact('after', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        Host.Close();
        Assert.AreEqual('before=T:0:10;after=T:0:30;', Seen, 'all of the above together: Refresh, then Filter.SetFilter and first on the part');
    end;

    [Test]
    procedure PartRefillShape_AllTogether_FirstNextPrevious()
    var
        Op: Record "LT PR Op";
        Host: TestPage "LT PS Full Host";
        Found: Boolean;
        Seen: Text;
    begin
        Initialize();
        Seed(Op);
        Commit();
        Host.OpenView();
        Host.GoToRecord(Op);
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen := Fact('before', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        AddBooking(3, TheDay(), 20);
        Commit();
        Host.Refresh.Invoke();
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen += Fact('first', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        Found := Host.Load.Next();
        Seen += Fact('next', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        Found := Host.Load.Previous();
        Seen += Fact('previous', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        Host.Close();
        Assert.AreEqual('before=T:0:10;first=T:0:30;next=F:0:30;previous=F:0:30;', Seen, 'all together: Refresh, then Filter.SetFilter and nextprev on the part');
    end;

    [Test]
    procedure PartRefillShape_AllTogether_Last()
    var
        Op: Record "LT PR Op";
        Host: TestPage "LT PS Full Host";
        Found: Boolean;
        Seen: Text;
    begin
        Initialize();
        Seed(Op);
        Commit();
        Host.OpenView();
        Host.GoToRecord(Op);
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen := Fact('before', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        AddBooking(3, TheDay(), 20);
        Commit();
        Host.Refresh.Invoke();
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.Last();
        Seen += Fact('last', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        Host.Close();
        Assert.AreEqual('before=T:0:10;last=T:0:30;', Seen, 'all together: Refresh, then Filter.SetFilter and last on the part');
    end;
}
