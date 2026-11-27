// Issue 9260 neighbor: CompanyProperty.UrlName is not kept, so this install still touches everything.
table 52820 "FP Readonly TC"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[250]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 52821 "FP Readonly C Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Row: Record "FP Readonly TC";
    begin
        Row.Code := 'C';
        Row.Value := CopyStr(CompanyProperty.UrlName(), 1, 250);
        Row.Insert();
    end;
}
