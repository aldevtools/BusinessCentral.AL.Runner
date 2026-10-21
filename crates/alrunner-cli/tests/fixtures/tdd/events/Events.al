// A stub that runs in an event subscriber flags the test that raised the event.
codeunit 50150 "Tdd Ping Target"
{
}

codeunit 50151 "Tdd Publisher"
{
    procedure Ping()
    begin
        OnPing();
    end;

    [IntegrationEvent(false, false)]
    local procedure OnPing()
    begin
    end;
}

codeunit 50152 "Tdd Subscriber"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Tdd Publisher", 'OnPing', '', false, false)]
    local procedure HandlePing()
    var
        Target: Codeunit "Tdd Ping Target";
    begin
        Target.Pinged();
    end;
}

codeunit 50160 "Tdd Event Tests"
{
    Subtype = Test;

    [Test]
    procedure RaisesTheEvent()
    var
        Publisher: Codeunit "Tdd Publisher";
    begin
        Publisher.Ping();
    end;

    [Test]
    procedure DoesNotRaiseIt()
    begin
    end;
}
