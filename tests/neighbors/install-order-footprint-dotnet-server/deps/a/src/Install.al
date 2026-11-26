dotnet
{
    assembly("Microsoft.Dynamics.Nav.Types")
    {
        Culture = 'neutral';
        PublicKeyToken = '31bf3856ad364e35';
        type("Microsoft.Dynamics.Nav.Types.ALConfigSettings"; "FP Config A") { }
    }
    assembly("Microsoft.Dynamics.Nav.NavUserAccount")
    {
        Culture = 'neutral';
        PublicKeyToken = '31bf3856ad364e35';
        type("Microsoft.Dynamics.Nav.NavUserAccount.NavTenantSettingsHelper"; "FP Tenant A") { }
    }
}

table 52400 "FP Dotnet Server TA"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[100]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 52401 "FP Dotnet Server A Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Config: DotNet "FP Config A";
        Tenant: DotNet "FP Tenant A";
        TenantId: Text;
        Row: Record "FP Dotnet Server TA";
    begin
        Config := Config.Instance();
        Tenant.TryGetStringTenantSetting('TENANTID', TenantId);
        Row.Code := 'A';
        Row.Value := Format(Config.ApiServicesEnabled()) + Format(Config.ApiSubscriptionsEnabled()) +
            Format(Tenant.IsProduction()) + Format(TenantId <> '');
        Row.Insert();
    end;
}
