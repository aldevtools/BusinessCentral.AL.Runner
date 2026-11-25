// A manual subscriber of the batch event contract whose binding a single instance holds from one test codeunit to the
// next (--strict-order runs both in one session, as BC does). No cell of corpus PR 1569 binds that way.
table 62520 "OH Row"
{
    fields
    {
        field(1; Id; Integer) { }
    }

    procedure Confirm(var Choice: Option " ",Skip,Update): Text
    var
        Seen: Text;
    begin
        Seen := Format(Choice) + '|' + Format(Choice, 0, 2);
        Choice := Choice::Update;
        exit(Seen);
    end;
}

codeunit 62521 "OH Events"
{
    procedure Raise(var V: Option; var Row: Record "OH Row")
    begin
        OnWithSender(V, Row);
    end;

    [IntegrationEvent(true, false)]
    local procedure OnWithSender(var V: Option; var Row: Record "OH Row")
    begin
    end;
}

codeunit 62522 "OH Manual Sub"
{
    EventSubscriberInstance = Manual;

    var
        RunLog: Text;

    procedure GetRunLog(): Text
    begin
        exit(RunLog);
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"OH Events", 'OnWithSender', '', false, false)]
    local procedure HandleWithSender(var V: Option; var Row: Record "OH Row")
    begin
        Pass(V, Row);
    end;

    local procedure Pass(var V: Option; var Row: Record "OH Row")
    begin
        Row.Confirm(V);
    end;
}

codeunit 62523 "OH Holder"
{
    SingleInstance = true;

    var
        Sub: Codeunit "OH Manual Sub";

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

codeunit 62524 "OH Batch"
{
    procedure Run(): Text
    var
        Choice: Option " ",Skip,Update;
        Events: Codeunit "OH Events";
        Row: Record "OH Row";
    begin
        Choice := Choice::Skip;
        Events.Raise(Choice, Row);
        exit(Format(Choice));
    end;
}

codeunit 62525 "OH Bind Tests"
{
    Subtype = Test;

    [Test]
    procedure Held_Bind()
    var
        Holder: Codeunit "OH Holder";
    begin
        Holder.BindHeld();
    end;
}

codeunit 62526 "OH Raise Tests"
{
    Subtype = Test;

    // The binding the codeunit before left, moved to this codeunit's session: no cell.
    [Test]
    procedure Held_RaiseInNextCodeunit()
    var
        Holder: Codeunit "OH Holder";
        Batch: Codeunit "OH Batch";
    begin
        if Holder.HeldLog() <> '' then
            Error('expected an empty log');
        if Batch.Run() <> 'Update' then
            Error('expected Update');
        Holder.UnbindHeld();
    end;
}
