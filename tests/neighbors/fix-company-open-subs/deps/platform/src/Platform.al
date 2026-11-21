// A stand-in for System Application's login code and its login memos (publisher Microsoft, invented code):
// "System Initialization Impl." fills the memo through the facade, then raises "System Initialization".OnAfterLogin
// from Company Triggers OnCompanyOpenCompleted. The memos keep a Boolean filled on first use.
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
        EnvInfo: Codeunit "Environment Information";
        SysInit: Codeunit "System Initialization";
    begin
        if EnvInfo.IsSaaS() then;
        SysInit.Raise();
    end;
}

codeunit 457 "Environment Information"
{
    SingleInstance = true;

    var
        Impl: Codeunit "Environment Information Impl.";

    procedure IsSaaS(): Boolean
    begin
        exit(Impl.IsSaaS());
    end;

    procedure IsSandbox(): Boolean
    begin
        exit(Impl.IsSandbox());
    end;

    procedure IsOnPrem(): Boolean
    begin
        exit(not Impl.IsSaaS());
    end;
}

codeunit 3702 "Environment Information Impl."
{
    SingleInstance = true;

    var
        Known: Boolean;
        SaaS: Boolean;

    procedure IsSaaS(): Boolean
    begin
        if not Known then begin
            SaaS := false;
            Known := true;
        end;
        exit(SaaS);
    end;

    procedure IsSandbox(): Boolean
    begin
        exit(IsSaaS());
    end;
}

codeunit 6723 "Server Setting"
{
    SingleInstance = true;

    var
        Impl: Codeunit "Server Setting Impl.";

    procedure GetEnableMembershipEntitlement(): Boolean
    begin
        exit(Impl.GetEnableMembershipEntitlement());
    end;
}

codeunit 3703 "Server Setting Impl."
{
    SingleInstance = true;

    var
        Known: Boolean;
        Entitled: Boolean;

    procedure GetEnableMembershipEntitlement(): Boolean
    begin
        if not Known then begin
            Entitled := false;
            Known := true;
        end;
        exit(Entitled);
    end;
}
