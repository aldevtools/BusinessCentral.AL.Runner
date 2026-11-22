// The calls no cell made after the test's filter, on the probe's objects (al-runner PR 8542).
codeunit 97150 "LT PR Neighbors"
{
    Subtype = Test;

    local procedure Refilled(var Host: TestPage "LT PR Load Host")
    var
        Op: Record "LT PR Op";
        Booking: Record "LT PR Booking";
    begin
        Op.DeleteAll();
        Booking.DeleteAll();
        Op."No." := 'OP1';
        Op."Resource No." := 'R1';
        Op.Insert();
        Add(1, TheDay(), 10);
        Add(2, TheDay() + 1, 5);
        Host.OpenView();
        Host.GoToRecord(Op);
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Host.Load.First();
        Add(3, TheDay(), 20);
        Host.Refresh.Invoke();
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
    end;

    local procedure Add(EntryNo: Integer; Day: Date; Qty: Decimal)
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

    [Test]
    procedure NextRightAfterTheFilter()
    var
        Host: TestPage "LT PR Load Host";
    begin
        Refilled(Host);
        Host.Load.Next();
    end;

    [Test]
    procedure PreviousRightAfterFirst()
    var
        Host: TestPage "LT PR Load Host";
    begin
        Refilled(Host);
        Host.Load.First();
        Host.Load.Previous();
    end;

    [Test]
    procedure NextAfterLast()
    var
        Host: TestPage "LT PR Load Host";
    begin
        Refilled(Host);
        Host.Load.Last();
        Host.Load.Next();
    end;

    [Test]
    procedure GoToKeyAfterTheFilter()
    var
        Host: TestPage "LT PR Load Host";
    begin
        Refilled(Host);
        Host.Load.GoToKey(TheDay());
    end;

    [Test]
    procedure AssertEqualsAfterTheFilter()
    var
        Host: TestPage "LT PR Load Host";
    begin
        Refilled(Host);
        Host.Load."Allocated Qty.".AssertEquals(30);
    end;

    [Test]
    procedure AscendingAfterTheFilter()
    var
        Host: TestPage "LT PR Load Host";
    begin
        Refilled(Host);
        Host.Load.Filter.Ascending(false);
        Host.Load.First();
    end;

    [Test]
    procedure RangeFilter()
    var
        Op: Record "LT PR Op";
        Booking: Record "LT PR Booking";
        Host: TestPage "LT PR Load Host";
    begin
        Op.DeleteAll();
        Booking.DeleteAll();
        Op."No." := 'OP1';
        Op."Resource No." := 'R1';
        Op.Insert();
        Add(1, TheDay(), 10);
        Add(2, TheDay() + 1, 5);
        Host.OpenView();
        Host.GoToRecord(Op);
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()) + '..' + Format(TheDay() + 1));
        Host.Load.First();
        Add(3, TheDay(), 20);
        Host.Refresh.Invoke();
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()) + '..' + Format(TheDay() + 1));
        Host.Load.First();
    end;

    [Test]
    procedure HostReadBetweenFilterAndFirst()
    var
        Host: TestPage "LT PR Load Host";
    begin
        Refilled(Host);
        Host."Resource No.".Value();
        Host.Load.First();
    end;

    [Test]
    procedure OtherPageBetweenFilterAndFirst()
    var
        Host: TestPage "LT PR Load Host";
        Other: TestPage "LT PR Load List Host";
    begin
        Refilled(Host);
        Other.OpenView();
        Other.Close();
        Host.Load.First();
    end;

    [Test]
    procedure HostReadBetweenFirstAndARead()
    var
        Host: TestPage "LT PR Load Host";
    begin
        Refilled(Host);
        Host.Load.First();
        Host."No.".Value();
        Host.Load."Allocated Qty.".AsDecimal();
    end;

    [Test]
    procedure FirstAfterReopen()
    var
        Host: TestPage "LT PR Load Host";
    begin
        Refilled(Host);
        Host.Close();
        Host.OpenView();
        Host.Load.First();
    end;

    // The moves and reads one cell measured, made on another cell (al-runner PR 8542 review, LOW 1 and 2).

    local procedure Seeded(var Op: Record "LT PR Op")
    var
        Booking: Record "LT PR Booking";
    begin
        Op.DeleteAll();
        Booking.DeleteAll();
        Op."No." := 'OP1';
        Op."Resource No." := 'R1';
        Op.Insert();
        Add(1, TheDay(), 10);
        Add(2, TheDay() + 1, 5);
    end;

    [Test]
    procedure LastOnTheListHost()
    var
        Op: Record "LT PR Op";
        Host: TestPage "LT PR Load List Host";
    begin
        Seeded(Op);
        Host.OpenView();
        Host.GoToRecord(Op);
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Host.Load.First();
        Add(3, TheDay(), 20);
        Host.Refresh.Invoke();
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Host.Load.Last();
    end;

    [Test]
    procedure NextOnTheTriggerPart()
    var
        Op: Record "LT PR Op";
        Host: TestPage "LT PR Load Trig Host";
    begin
        Seeded(Op);
        Host.OpenView();
        Host.GoToRecord(Op);
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Host.Load.First();
        Add(3, TheDay(), 20);
        Host.Refresh.Invoke();
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Host.Load.First();
        Host.Load.Next();
    end;

    [Test]
    procedure ValueReadOfAPlainPartField()
    var
        Host: TestPage "LT PR Load Host";
    begin
        Refilled(Host);
        Host.Load.First();
        Host.Load."Allocated Qty.".Value();
    end;

    [Test]
    procedure ValueReadBeforeFirst()
    var
        Op: Record "LT PR Op";
        Host: TestPage "LT PR Load Trig Host";
    begin
        Seeded(Op);
        Host.OpenView();
        Host.GoToRecord(Op);
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Host.Load.First();
        Add(3, TheDay(), 20);
        Host.Refresh.Invoke();
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Host.Load.QtyText.Value();
    end;

    // The AsDate and AsDecimal reads one cell made, made on another cell or of another control (al-runner PR 8700 review).

    [Test]
    procedure ReadWithoutMoveOnTheListHost()
    var
        Op: Record "LT PR Op";
        Host: TestPage "LT PR Load List Host";
    begin
        Seeded(Op);
        Host.OpenView();
        Host.GoToRecord(Op);
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Host.Load.First();
        Add(3, TheDay(), 20);
        Host.Refresh.Invoke();
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Host.Load."Allocated Qty.".AsDecimal();
    end;

    [Test]
    procedure ReadWithoutMoveOnTheTriggerPart()
    var
        Op: Record "LT PR Op";
        Host: TestPage "LT PR Load Trig Host";
    begin
        Seeded(Op);
        Host.OpenView();
        Host.GoToRecord(Op);
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Host.Load.First();
        Add(3, TheDay(), 20);
        Host.Refresh.Invoke();
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Host.Load."Allocated Qty.".AsDecimal();
    end;

    [Test]
    procedure QtyTextAsDecimalAfterFirst()
    var
        Op: Record "LT PR Op";
        Host: TestPage "LT PR Load Trig Host";
    begin
        Seeded(Op);
        Host.OpenView();
        Host.GoToRecord(Op);
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Host.Load.First();
        Add(3, TheDay(), 20);
        Host.Refresh.Invoke();
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Host.Load.First();
        Host.Load.QtyText.AsDecimal();
    end;
}
