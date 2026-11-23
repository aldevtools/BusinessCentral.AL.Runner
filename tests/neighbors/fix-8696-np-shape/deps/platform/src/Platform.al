// A stand-in for the login code (publisher Microsoft, invented code): "System Initialization Impl." raises
// "System Initialization".OnAfterLogin from Company Triggers OnCompanyOpenCompleted. "XS Touch Hub" raises an event that
// apps the partner app does not depend on subscribe to (as the table trigger setup event reaches a test library).
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

codeunit 152 "XS Touch Hub"
{
    procedure Touch()
    begin
        OnTouched();
    end;

    [IntegrationEvent(false, false)]
    procedure OnTouched()
    begin
    end;
}
