// A manual subscriber (EventSubscriberInstance = Manual) of the batch event contract, IntegrationEvent(true, false), that
// hands its memberless var parameter on to a var parameter with members (corpus PR 1569, OptionVarMembersBatchManual.al).
codeunit 62510 "OC Manual Events"
{
    procedure Raise(var V: Option; var Row: Record "OC Row")
    begin
        OnWithSender(V, Row);
    end;

    procedure Setup(var Bound: Codeunit "OC Manual Sub")
    begin
        OnSetup(Bound);
    end;

    [IntegrationEvent(true, false)]
    local procedure OnWithSender(var V: Option; var Row: Record "OC Row")
    begin
    end;

    [IntegrationEvent(false, false)]
    local procedure OnSetup(var Bound: Codeunit "OC Manual Sub")
    begin
    end;
}

codeunit 62511 "OC Manual Sub"
{
    EventSubscriberInstance = Manual;

    trigger OnRun()
    var
        Bound: Codeunit "OC Manual Sub";
        Batch: Codeunit "OC Manual Batch";
    begin
        BindSubscription(Bound);
        RunLog := Batch.Run();
        UnbindSubscription(Bound);
    end;

    var
        RunLog: Text;

    procedure GetRunLog(): Text
    begin
        exit(RunLog);
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"OC Manual Events", 'OnWithSender', '', false, false)]
    local procedure HandleWithSender(var V: Option; var Row: Record "OC Row")
    begin
        Pass(V, Row);
    end;

    local procedure Pass(var V: Option; var Row: Record "OC Row")
    begin
        Row.Confirm(V);
    end;
}

// Binds the instance it is handed from inside a subscriber.
codeunit 62512 "OC Manual Binder"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"OC Manual Events", 'OnSetup', '', false, false)]
    local procedure BindOnSetup(var Bound: Codeunit "OC Manual Sub")
    begin
        BindSubscription(Bound);
    end;
}

codeunit 62513 "OC Manual Batch"
{
    procedure Run(): Text
    var
        Choice: Option " ",Skip,Update;
        Events: Codeunit "OC Manual Events";
        Row: Record "OC Row";
    begin
        Choice := Choice::Skip;
        Events.Raise(Choice, Row);
        exit(Format(Choice));
    end;

    procedure RunBound(): Text
    var
        Sub: Codeunit "OC Manual Sub";
        Log: Text;
    begin
        BindSubscription(Sub);
        Log := Run();
        UnbindSubscription(Sub);
        exit(Log);
    end;
}

// Holds a bound instance across test codeunits.
codeunit 62514 "OC Manual Holder"
{
    SingleInstance = true;

    var
        Sub: Codeunit "OC Manual Sub";

    procedure BindHeld()
    begin
        BindSubscription(Sub);
    end;

    procedure UnbindHeld()
    begin
        UnbindSubscription(Sub);
    end;

    // A read of the held instance: the codeunit that calls it depends on the state an earlier test codeunit left.
    procedure HeldLog(): Text
    begin
        exit(Sub.GetRunLog());
    end;
}

codeunit 62515 "OC Manual Tests"
{
    Subtype = Test;

    local procedure Check(Actual: Text; Expected: Text)
    begin
        if Actual <> Expected then
            Error('expected %1, got %2', Expected, Actual);
    end;

    // Bound in the OnRun of Codeunit.Run (ManualRun_*).
    [Test]
    procedure Manual_BoundInOnRun()
    var
        Sub: Codeunit "OC Manual Sub";
    begin
        Sub.Run();
        Check(Sub.GetRunLog(), 'Update');
    end;

    // Bound in a procedure (ManualBound_*).
    [Test]
    procedure Manual_BoundInProcedure()
    var
        Batch: Codeunit "OC Manual Batch";
    begin
        Check(Batch.RunBound(), 'Update');
    end;

    // Bound by the test (ManualTestBound_*).
    [Test]
    procedure Manual_BoundByTest()
    var
        Sub: Codeunit "OC Manual Sub";
        Batch: Codeunit "OC Manual Batch";
    begin
        BindSubscription(Sub);
        Check(Batch.Run(), 'Update');
        UnbindSubscription(Sub);
    end;

    // Bound and unbound before the raise (ManualUnbound_*): no subscriber runs.
    [Test]
    procedure Manual_UnboundBeforeRaise()
    var
        Sub: Codeunit "OC Manual Sub";
        Batch: Codeunit "OC Manual Batch";
    begin
        BindSubscription(Sub);
        UnbindSubscription(Sub);
        Check(Batch.Run(), 'Skip');
    end;

    // Never bound (ManualNeverBound_*): no subscriber runs.
    [Test]
    procedure Manual_NeverBound()
    var
        Batch: Codeunit "OC Manual Batch";
    begin
        Check(Batch.Run(), 'Skip');
    end;

    // Two instances bound at once: no cell.
    [Test]
    procedure Manual_TwoInstancesBound()
    var
        Sub1: Codeunit "OC Manual Sub";
        Sub2: Codeunit "OC Manual Sub";
        Batch: Codeunit "OC Manual Batch";
    begin
        BindSubscription(Sub1);
        BindSubscription(Sub2);
        Check(Batch.Run(), 'Update');
    end;

    // Bound inside a subscriber: no cell.
    [Test]
    procedure Manual_BoundInsideSubscriber()
    var
        Sub: Codeunit "OC Manual Sub";
        Events: Codeunit "OC Manual Events";
        Batch: Codeunit "OC Manual Batch";
    begin
        Events.Setup(Sub);
        Check(Batch.Run(), 'Update');
    end;

    // Bound through a single instance, raised in this codeunit: the binding stays in the session (no cell).
    [Test]
    procedure Manual_HeldBySingleInstance()
    var
        Holder: Codeunit "OC Manual Holder";
        Batch: Codeunit "OC Manual Batch";
    begin
        Holder.BindHeld();
        Check(Batch.Run(), 'Update');
        Holder.UnbindHeld();
    end;
}
