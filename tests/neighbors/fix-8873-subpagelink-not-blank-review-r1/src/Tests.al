codeunit 50140 "NB SPNBR Tests"
{
    Subtype = Test;

    local procedure Seed()
    var
        H: Record "NB SPNBR Header";
        L: Record "NB SPNBR Line";
    begin
        H.DeleteAll();
        L.DeleteAll();
        H."No." := 'H1';
        H.Insert();
        AddLine('H1', 1, '');
        AddLine('H1', 2, 'H1');
    end;

    local procedure AddLine(HeaderNo: Code[10]; LineNo: Integer; Arrival: Code[10])
    var
        L: Record "NB SPNBR Line";
    begin
        L."Header No." := HeaderNo;
        L."Line No." := LineNo;
        L."Arrival No." := Arrival;
        L.Insert();
    end;

    local procedure Check(Expected: Text; Actual: Text; What: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected [%2], got [%3]', What, Expected, Actual);
    end;

    [Test]
    procedure CardPart_KeepsMain()
    var
        H: Record "NB SPNBR Header";
        Host: TestPage "NB SPNBR CardPart Host";
    begin
        Seed();
        H.Get('H1');
        Host.OpenView();
        Host.GoToRecord(H);
        Check('<>''''', Host.CardPartLink.Filter.GetFilter("Arrival No."), 'filter');
        Host.Close();
    end;

    [Test]
    procedure FlowFilterField_KeepsMain()
    var
        H: Record "NB SPNBR Header";
        Host: TestPage "NB SPNBR FlowFilter Host";
    begin
        Seed();
        H.Get('H1');
        Host.OpenView();
        Host.GoToRecord(H);
        Check('<>''''', Host.FlowFilterLink.Filter.GetFilter("Arrival Filter"), 'filter');
        Host.Close();
    end;

    [Test]
    procedure FlowField_KeepsMain()
    var
        H: Record "NB SPNBR Header";
        Host: TestPage "NB SPNBR FlowField Host";
    begin
        Seed();
        H.Get('H1');
        Host.OpenView();
        Host.GoToRecord(H);
        Check('<>''''', Host.FlowFieldLink.Filter.GetFilter("Header Lookup"), 'filter');
        Host.Close();
    end;

    [Test]
    procedure RunPageLink_CardTarget_KeepsMain()
    var
        H: Record "NB SPNBR Header";
        Host: TestPage "NB SPNBR Run Host";
        Target: TestPage "NB SPNBR Target Card";
    begin
        Seed();
        H.Get('H1');
        Host.OpenView();
        Host.GoToRecord(H);
        Target.Trap();
        Host.RunCard.Invoke();
        Check('<>''''', Target.Filter.GetFilter("Arrival No."), 'filter');
        Target.Close();
        Host.Close();
    end;

    // A control that runs on main and on head, so the two runs are compared: the host page opens; only the action's
    // RunPageLink carries the term.
    [Test]
    procedure RunHost_Opens_Control()
    var
        H: Record "NB SPNBR Header";
        Host: TestPage "NB SPNBR Run Host";
    begin
        Seed();
        H.Get('H1');
        Host.OpenView();
        Host.GoToRecord(H);
        Check('H1', Host."No.".Value, 'no');
        Host.Close();
    end;
}
