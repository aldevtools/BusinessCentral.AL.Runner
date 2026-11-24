// The objects and tests of corpus PR 1474 (pages/parts/PartRefillTestFilter.al, merged) its two-field key cell
// (corpus PR 1475), both merged, and the styled part (corpus PR 1534, open); the suite test part_left_view_test_filter.rs reads them from here.
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

    [Test]
    procedure PartRefill_StyledPartActionThenFilterFirst_FindsRefilledRow()
    var
        Op: Record "LT PR Op";
        Host: TestPage "LT PR Styled Host";
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
        Assert.AreEqual('before=T:0:10;after=T:0:30;', Seen, 'Worksheet, the plain part with StyleExpr on Allocated Qty. set in its OnAfterGetRecord: Refresh, then Filter.SetFilter and First on the part');
    end;
}
