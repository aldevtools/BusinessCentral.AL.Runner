dotnet
{
    assembly("Microsoft.Dynamics.Nav.Types")
    {
        Culture = 'neutral';
        PublicKeyToken = '31bf3856ad364e35';
        type("Microsoft.Dynamics.Nav.Types.ALConfigSettings"; "FP Config C") { }
    }
    assembly("Microsoft.Dynamics.Nav.NavUserAccount")
    {
        Culture = 'neutral';
        PublicKeyToken = '31bf3856ad364e35';
        type("Microsoft.Dynamics.Nav.NavUserAccount.NavTenantSettingsHelper"; "FP Tenant C") { }
    }
}

table 52420 "FP Dotnet Server TC"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[100]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 52421 "FP Dotnet Server C Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Config: DotNet "FP Config C";
        Tenant: DotNet "FP Tenant C";
        TenantId: Text;
        Row: Record "FP Dotnet Server TC";
    begin
        Config := Config.Instance();
        Tenant.TryGetStringTenantSetting('TENANTID', TenantId);
        Row.Code := 'C';
        Row.Value := Format(Config.ApiServicesEnabled()) + Format(Config.ApiSubscriptionsEnabled()) +
            Format(Tenant.IsProduction()) + Format(TenantId <> '');
        Row.Insert();
    end;
}
