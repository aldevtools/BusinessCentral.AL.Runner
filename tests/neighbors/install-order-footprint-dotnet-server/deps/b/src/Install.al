dotnet
{
    assembly("Microsoft.Dynamics.Nav.Types")
    {
        Culture = 'neutral';
        PublicKeyToken = '31bf3856ad364e35';
        type("Microsoft.Dynamics.Nav.Types.ALConfigSettings"; "FP Config B") { }
    }
    assembly("Microsoft.Dynamics.Nav.NavUserAccount")
    {
        Culture = 'neutral';
        PublicKeyToken = '31bf3856ad364e35';
        type("Microsoft.Dynamics.Nav.NavUserAccount.NavTenantSettingsHelper"; "FP Tenant B") { }
    }
}

table 52410 "FP Dotnet Server TB"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[100]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 52411 "FP Dotnet Server B Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Config: DotNet "FP Config B";
        Tenant: DotNet "FP Tenant B";
        TenantId: Text;
        Row: Record "FP Dotnet Server TB";
    begin
        Config := Config.Instance();
        Tenant.TryGetStringTenantSetting('TENANTID', TenantId);
        Row.Code := 'B';
        Row.Value := Format(Config.ApiServicesEnabled()) + Format(Config.ApiSubscriptionsEnabled()) +
            Format(Tenant.IsProduction()) + Format(TenantId <> '');
        Row.Insert();
    end;
}
