dotnet
{
    assembly("Microsoft.Dynamics.Nav.Types")
    {
        Culture = 'neutral';
        PublicKeyToken = '31bf3856ad364e35';
        type("Microsoft.Dynamics.Nav.Types.ALConfigSettings"; "FP Config D") { }
    }
    assembly("Microsoft.Dynamics.Nav.NavUserAccount")
    {
        Culture = 'neutral';
        PublicKeyToken = '31bf3856ad364e35';
        type("Microsoft.Dynamics.Nav.NavUserAccount.NavTenantSettingsHelper"; "FP Tenant D") { }
    }
}

table 52430 "FP Dotnet Server TD"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[100]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 52431 "FP Dotnet Server D Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Config: DotNet "FP Config D";
        Tenant: DotNet "FP Tenant D";
        TenantId: Text;
        Row: Record "FP Dotnet Server TD";
    begin
        Config := Config.Instance();
        Tenant.TryGetStringTenantSetting('TENANTID', TenantId);
        Row.Code := 'D';
        Row.Value := Format(Config.ApiServicesEnabled()) + Format(Config.ApiSubscriptionsEnabled()) +
            Format(Tenant.IsProduction()) + Format(TenantId <> '');
        Row.Insert();
    end;
}
