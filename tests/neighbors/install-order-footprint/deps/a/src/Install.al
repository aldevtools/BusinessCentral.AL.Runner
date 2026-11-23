table 50910 "Order Probe Feature"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 50911 "Order Probe A Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Events: Codeunit "Order Probe Events";
        Feature: Record "Order Probe Feature";
    begin
        Events.InitializeMappings();
        Feature."Code" := 'A-FEATURE';
        if Feature.Insert() then;
    end;
}
