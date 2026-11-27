// Issue 9260: the install reads CompanyProperty.ID.
table 52710 "FP Readonly TB"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[250]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 52711 "FP Readonly B Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Row: Record "FP Readonly TB";
    begin
        Row.Code := 'B';
        Row.Value := CopyStr(Format(CompanyProperty.ID()), 1, 250);
        Row.Insert();
    end;
}
