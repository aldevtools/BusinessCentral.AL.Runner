// A stand-in for System Application's login code (publisher Microsoft, invented code): "System Initialization Impl." raises
// "System Initialization".OnAfterLogin from Company Triggers OnCompanyOpenCompleted. "UT Ping" is an event the partner's
// login subscriber raises and the stand-in test library subscribes to, as a row write reaches a test library's global
// trigger subscriber.
codeunit 150 "System Initialization"
{
    procedure Raise()
    begin
        OnAfterLogin();
    end;

    [IntegrationEvent(false, false, true)]
    procedure OnAfterLogin()
    begin
    end;
}

codeunit 151 "System Initialization Impl."
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Company Triggers", 'OnCompanyOpenCompleted', '', false, false)]
    local procedure InitCompany()
    var
        S: Codeunit "System Initialization";
    begin
        S.Raise();
    end;
}

codeunit 152 "UT Ping"
{
    procedure Raise()
    begin
        OnPing();
    end;

    [IntegrationEvent(false, false)]
    procedure OnPing()
    begin
    end;
}
