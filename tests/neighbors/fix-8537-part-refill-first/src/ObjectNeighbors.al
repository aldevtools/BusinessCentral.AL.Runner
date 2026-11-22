// Objects no cell had: a part over a stored table, and a refill that leaves two rows in the filtered view.
table 97153 "LT PR NB Stored Line"
{
    DataClassification = SystemMetadata;

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

page 97154 "LT PR NB Stored Part"
{
    PageType = ListPart;
    SourceTable = "LT PR NB Stored Line";
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

page 97155 "LT PR NB Stored Host"
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
            part(Load; "LT PR NB Stored Part") { ApplicationArea = All; }
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

page 97156 "LT PR NB TwoRows Part"
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
            Rec."Period Type" := Rec."Period Type"::Week;
            Rec.Insert();
        end;
        LinesLoaded := true;
        if Rec.FindFirst() then;
        CurrPage.Update(false);
    end;
}

page 97157 "LT PR NB TwoRows Host"
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
            part(Load; "LT PR NB TwoRows Part") { ApplicationArea = All; }
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

codeunit 97158 "LT PR NB Object Neighbors"
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
    procedure StoredTablePart_FilterFirst()
    var
        Op: Record "LT PR Op";
        Host: TestPage "LT PR NB Stored Host";
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
    procedure TwoRowsLeftByTheFilter_First()
    var
        Op: Record "LT PR Op";
        Host: TestPage "LT PR NB TwoRows Host";
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
