// A stand-in for System Application's login code and its login memos (publisher Microsoft, invented code). The memos
// ask the service tier through the DotNet server classes the runtime serves: NavTenantSettingsHelper.IsSandbox(),
// ALConfigSettings.Instance() and its IsSaaS(). "System Initialization Impl." fills the memo through the facade, raises
// "System Initialization".OnAfterLogin from Company Triggers OnCompanyOpenCompleted, then calls code the runner refuses
// (Debugger), as the corpus run refuses System Application's InitCompany at Assisted Company Setup.CompanyActive
// (IsEmpty on an unseeded G/L Entry, ADR-047).
dotnet
{
    assembly("Microsoft.Dynamics.Nav.NavUserAccount")
    {
        type("Microsoft.Dynamics.Nav.NavUserAccount.NavTenantSettingsHelper"; "NavTenantSettingsHelper") { }
    }
    assembly("Microsoft.Dynamics.Nav.Types")
    {
        type("Microsoft.Dynamics.Nav.Types.ALConfigSettings"; "ALConfigSettings") { }
    }
}

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
        Halt: Codeunit "DRL Halt";
    begin
        if EnvInfo.IsSaaS() then;
        SysInit.Raise();
        Halt.Halt();
    end;
}

codeunit 152 "DRL Halt"
{
    procedure Halt()
    begin
        Debugger.Deactivate();
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
}

codeunit 3702 "Environment Information Impl."
{
    SingleInstance = true;

    var
        Helper: DotNet NavTenantSettingsHelper;
        SaaSKnown: Boolean;
        SaaS: Boolean;
        SandboxKnown: Boolean;
        Sandbox: Boolean;

    procedure IsSaaS(): Boolean
    var
        ServerSetting: Codeunit "Server Setting";
    begin
        if not SaaSKnown then begin
            SaaS := IsSandbox() or ServerSetting.GetEnableMembershipEntitlement();
            SaaSKnown := true;
        end;
        exit(SaaS);
    end;

    procedure IsSandbox(): Boolean
    begin
        if not SandboxKnown then begin
            Sandbox := Helper.IsSandbox();
            SandboxKnown := true;
        end;
        exit(Sandbox);
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
        Config: DotNet ALConfigSettings;
        Known: Boolean;

    procedure GetEnableMembershipEntitlement(): Boolean
    begin
        if not Known then begin
            Config := Config.Instance();
            Known := true;
        end;
        exit(Config.IsSaaS());
    end;
}
