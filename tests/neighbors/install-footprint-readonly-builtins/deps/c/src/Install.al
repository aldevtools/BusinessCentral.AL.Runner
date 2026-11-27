// Issue 9260: the install reads NavApp.GetCallerCallstackModuleInfos through a procedure of its own.
table 52720 "FP Readonly TC"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[250]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 52721 "FP Readonly C Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Row: Record "FP Readonly TC";
    begin
        Row.Code := 'C';
        Row.Value := CopyStr(Format(Callers()), 1, 250);
        Row.Insert();
    end;

    local procedure Callers(): Integer
    begin
        exit(NavApp.GetCallerCallstackModuleInfos().Count());
    end;
}
