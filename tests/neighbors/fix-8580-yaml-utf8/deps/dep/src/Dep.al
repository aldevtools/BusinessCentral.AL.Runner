// The dependency app: an install codeunit of its own, so that the install baseline runs, as in the reporter's suite.
table 60936 "YW Dep Row"
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

codeunit 60936 "YW Dep Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Row: Record "YW Dep Row";
    begin
        Row.Code := 'DEP';
        Row.Insert();
    end;
}
