// Issue 9260 neighbor: the install reads NavApp.GetCallerCallstackModuleInfos.
table 52810 "FP Readonly TB"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[250]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 52811 "FP Readonly B Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Row: Record "FP Readonly TB";
    begin
        Row.Code := 'B';
        Row.Value := CopyStr(Format(Callers()), 1, 250);
        Row.Insert();
    end;

    local procedure Callers(): Integer
    begin
        exit(NavApp.GetCallerCallstackModuleInfos().Count());
    end;
}
