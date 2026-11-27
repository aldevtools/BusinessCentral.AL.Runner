// Issue 9260, the reporter's shape: the install reads CompanyProperty.DisplayName.
table 52700 "FP Readonly TA"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[250]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 52701 "FP Readonly A Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Row: Record "FP Readonly TA";
    begin
        Row.Code := 'A';
        Row.Value := CopyStr(CompanyProperty.DisplayName(), 1, 250);
        Row.Insert();
    end;
}
