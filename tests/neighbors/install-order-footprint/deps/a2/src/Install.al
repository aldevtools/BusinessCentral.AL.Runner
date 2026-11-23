table 50942 "Order Probe Feature 2"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 50941 "Order Probe A2 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Events: Codeunit "Order Probe Events";
        Feature: Record "Order Probe Feature 2";
    begin
        Events.InitializeMappings();
        Feature."Code" := 'A2-FEATURE';
        if Feature.Insert() then;
    end;
}
