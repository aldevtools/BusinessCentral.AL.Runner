// Issue 9260: the install reads all three.
table 52730 "FP Readonly TD"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[250]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 52731 "FP Readonly D Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Row: Record "FP Readonly TD";
    begin
        Row.Code := 'D';
        Row.Value := CopyStr(CompanyProperty.DisplayName() + Format(CompanyProperty.ID()) + Format(Callers()), 1, 250);
        Row.Insert();
    end;

    local procedure Callers(): Integer
    begin
        exit(NavApp.GetCallerCallstackModuleInfos().Count());
    end;
}
