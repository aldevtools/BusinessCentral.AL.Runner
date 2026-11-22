// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/testfilter/testfilter-setfilter-method
// Scope: in-scope
// local object: the part's own refill (Reset, DeleteAll and CurrPage.Update(false), then Insert, FindFirst and CurrPage.Update(false))
//   and the host's ways to start it are what is under test
// Fixtures used: shared Assert (60021); own tables 97100-97102 and 97112, codeunits 97103 and 97108, pages 97104-97107, 97110,
//   97111, 97113 and 97114, test codeunit 97109
// Probe (al-runner issue 8537): a ListPart on a temporary table whose rows the host's OnAfterGetCurrRecord deletes and inserts again
//   through two part procedures, each ending with CurrPage.Update(false). A host action marks the part stale and calls
//   CurrPage.Update(false), so the part is cleared and refilled with new quantities. The test sets a filter on the part with
//   Filter.SetFilter before and after the action. Readings: the row First(), a read without a move, Next() and Previous() after
//   First(), and Last() find after the action; the same after a host Next() to an operation of another resource instead of the
//   action; on a List host; on a Worksheet host whose OnAfterGetCurrRecord fills the part from a page background task (the rows are
//   committed so the task's session reads them); with a part that declares OnOpenPage and OnAfterGetRecord (a variable control shows
//   what OnAfterGetRecord computed for the row), also over a two-field key (Period Type, Period Start, as Base Application's Load
//   Buffer) filtered on Period Start; and First() after the action without a new filter (Rec.Reset() in the part dropped the test's
//   filter). Results are `name=Found:Period Start - TheDay:Allocated Qty.`.
//   Measured on a local BC 29.1 OnPrem w1 container (10/10 green); BC 27.5, 28.5 and 29.0 are left to CI.
// BC versions: 27, 28, 29
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
codeunit 97109 "LT PR Tests"
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
    procedure PartRefill_WorksheetActionThenFilterFirst_FindsRefilledRow()
    var
        Op: Record "LT PR Op";
        Host: TestPage "LT PR Load Host";
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
    procedure PartRefill_WorksheetActionThenFilterRead_ShowsRefilledRow()
    var
        Op: Record "LT PR Op";
        Host: TestPage "LT PR Load Host";
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
        Seen += Fact('read', true, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        Host.Close();
        Assert.AreEqual('before=T:0:10;read=T:0:30;', Seen, 'Worksheet: Refresh, then Filter.SetFilter and a read of the part without a move');
    end;

    [Test]
    procedure PartRefill_WorksheetActionThenFilterFirstNextPrevious_StayOnRefilledRow()
    var
        Op: Record "LT PR Op";
        Host: TestPage "LT PR Load Host";
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
        Seen += Fact('first', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        Found := Host.Load.Next();
        Seen += Fact('next', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        Found := Host.Load.Previous();
        Seen += Fact('previous', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        Host.Close();
        Assert.AreEqual('before=T:0:10;first=T:0:30;next=F:0:30;previous=F:0:30;', Seen, 'Worksheet: Refresh, then Filter.SetFilter, First, Next and Previous on the part');
    end;

    [Test]
    procedure PartRefill_WorksheetActionThenFilterLast_FindsRefilledRow()
    var
        Op: Record "LT PR Op";
        Host: TestPage "LT PR Load Host";
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
        Found := Host.Load.Last();
        Seen += Fact('last', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        Host.Close();
        Assert.AreEqual('before=T:0:10;last=T:0:30;', Seen, 'Worksheet: Refresh, then Filter.SetFilter and Last on the part');
    end;

    [Test]
    procedure PartRefill_WorksheetHostNextThenFilterFirst_FindsRowOfNextResource()
    var
        Op: Record "LT PR Op";
        Host: TestPage "LT PR Load Host";
        Found: Boolean;
        Seen: Text;
    begin
        Initialize();
        Seed();
        AddOp('OP2', 'R2');
        AddBooking(4, 'R2', TheDay(), 7);
        AddBooking(5, 'R2', TheDay() + 1, 3);
        Op.Get('OP1');
        Host.OpenView();
        Host.GoToRecord(Op);
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen := Fact('op1', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        Found := Host.Next();
        Seen += 'host=' + Host."No.".Value() + ';';
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen += Fact('op2', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        Host.Close();
        Assert.AreEqual('op1=T:0:10;host=OP2;op2=T:0:7;', Seen, 'Worksheet: host Next to another resource, then Filter.SetFilter and First on the part');
    end;

    [Test]
    procedure PartRefill_WorksheetActionThenFirstWithoutFilter_ResetDropsTheFilter()
    var
        Op: Record "LT PR Op";
        Host: TestPage "LT PR Load Host";
        Found: Boolean;
        Seen: Text;
    begin
        Initialize();
        Seed();
        Op.Get('OP1');
        Host.OpenView();
        Host.GoToRecord(Op);
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay() + 1));
        Found := Host.Load.First();
        Seen := Fact('before', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        AddBooking(3, 'R1', TheDay() + 1, 20);
        Host.Refresh.Invoke();
        if Host.Load.Filter.GetFilter("Period Start") = '' then
            Seen += 'filter=none;'
        else
            Seen += 'filter=kept;';
        Found := Host.Load.First();
        Seen += Fact('after', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        Host.Close();
        Assert.AreEqual('before=T:1:5;filter=none;after=T:0:10;', Seen, 'Worksheet: Refresh, then First on the part without a new filter');
    end;

    [Test]
    procedure PartRefill_ListActionThenFilterFirst_FindsRefilledRow()
    var
        Op: Record "LT PR Op";
        Host: TestPage "LT PR Load List Host";
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
        Assert.AreEqual('before=T:0:10;after=T:0:30;', Seen, 'List: Refresh, then Filter.SetFilter and First on the part');
    end;

    [Test]
    procedure PartRefill_BackgroundTaskActionThenFilterFirst_FindsRefilledRow()
    var
        Op: Record "LT PR Op";
        Host: TestPage "LT PR Load Bg Host";
        Found: Boolean;
        Seen: Text;
    begin
        Initialize();
        Seed();
        Op.Get('OP1');
        Commit();
        Host.OpenView();
        Host.GoToRecord(Op);
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen := Fact('before', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        AddBooking(3, 'R1', TheDay(), 20);
        Commit();
        Host.Refresh.Invoke();
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Found := Host.Load.First();
        Seen += Fact('after', Found, Host.Load."Period Start".AsDate(), Host.Load."Allocated Qty.".AsDecimal());
        Host.Close();
        Assert.AreEqual('before=T:0:10;after=T:0:30;', Seen, 'Worksheet with a page background task: Refresh, then Filter.SetFilter and First on the part');
    end;

    [Test]
    procedure PartRefill_TriggerPartActionThenFilterFirst_FindsRefilledRow()
    var
        Op: Record "LT PR Op";
        Host: TestPage "LT PR Load Trig Host";
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
        Assert.AreEqual('before=T:0:10;text=agr10;scope=open;after=T:0:30;text=agr30;scope=open;', Seen, 'Worksheet, a part with OnOpenPage and OnAfterGetRecord: Refresh, then Filter.SetFilter and First on the part');
    end;

    [Test]
    procedure PartRefill_TypedKeyTriggerPartActionThenFilterFirst_FindsRefilledRow()
    var
        Op: Record "LT PR Op";
        Host: TestPage "LT PR Typed Trig Host";
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
