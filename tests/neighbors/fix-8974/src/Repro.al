tableextension 50410 "Repro XRec Line Ext" extends "Repro XRec Line"
{
    procedure ExtProc()
    begin
        Rec."Proc Ran" := true;
        Rec."Seen xRec Profile" := xRec."Profile Code";
    end;
}

codeunit 50411 "Repro XRec Subscriber"
{
    [EventSubscriber(ObjectType::Table, Database::"Repro XRec Line", 'OnAfterValidateEvent', 'No.', false, false)]
    local procedure OnAfterValidateNo(var Rec: Record "Repro XRec Line"; var xRec: Record "Repro XRec Line"; CurrFieldNo: Integer)
    begin
        case Rec.Route of
            1:
                Rec.OwnProc();
            2:
                Helper(Rec, xRec);
            3:
                Rec.ExtProc();
            4:
                Helper(Rec, xRec);
        end;
    end;

    local procedure Helper(var Line: Record "Repro XRec Line"; var xLine: Record "Repro XRec Line")
    begin
        if Line.Route = 2 then
            Line.OwnProc()
        else
            Line.ExtProc();
    end;
}

codeunit 50412 "Repro XRec Route Tests"
{
    Subtype = Test;

    [Test]
    procedure R1_SubscriberCallsTableProc()
    begin
        RunRoute(1);
    end;

    [Test]
    procedure R2_SubscriberHelperCallsTableProc()
    begin
        RunRoute(2);
    end;

    [Test]
    procedure R3_SubscriberCallsExtProc()
    begin
        RunRoute(3);
    end;

    [Test]
    procedure R4_SubscriberHelperCallsExtProc()
    begin
        RunRoute(4);
    end;

    local procedure RunRoute(NewRoute: Integer)
    var
        Line: Record "Repro XRec Line";
    begin
        Line.DeleteAll();
        Line.Init();
        Line."Entry No." := 1;
        Line."No." := 'X';
        Line."Profile Code" := 'OLD';
        Line.Route := NewRoute;
        Line.Insert();

        // The field trigger sets "Profile Code" to NEW; the subscriber of OnAfterValidateEvent then calls the procedure.
        Line.Validate("No.", 'Y');

        if not Line."Proc Ran" then
            Error('The procedure did not run.');
        if Line."Profile Code" <> 'NEW' then
            Error('Profile Code after the Validate: %1', Line."Profile Code");
        if Line."Seen xRec Profile" <> 'OLD' then
            Error('xRec."Profile Code" inside the procedure: ''%1'' (expected OLD, the before-image of the running Validate)', Line."Seen xRec Profile");
    end;
}
