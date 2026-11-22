// The dependency app of the neighbor: a table and an install codeunit that fills it, so that the install baseline runs.
table 60950 "YD Dep Row"
{
    fields
    {
        field(1; Code; Code[20]) { }
    }
    keys
    {
        key(PK; Code) { Clustered = true; }
    }
}

codeunit 60951 "YD Dep Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Row: Record "YD Dep Row";
    begin
        Row.Code := 'DEP';
        Row.Insert();
    end;
}
