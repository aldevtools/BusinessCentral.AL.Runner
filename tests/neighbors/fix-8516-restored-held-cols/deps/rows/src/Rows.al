// The dependency app: tables its install code fills, so their rows reach the tests through the install baseline,
// the store made from exported rows (MemStore::from_exported) that a worker thread and the baseline cache read back.
table 61700 "HN Ae Key"
{
    fields
    {
        field(1; Name; Text[50]) { }
    }
    keys
    {
        key(PK; Name) { Clustered = true; }
    }
}

table 61701 "HN Accent Key"
{
    fields
    {
        field(1; Name; Text[50]) { }
    }
    keys
    {
        key(PK; Name) { Clustered = true; }
    }
}

table 61702 "HN Ascii Key"
{
    fields
    {
        field(1; Name; Text[50]) { }
    }
    keys
    {
        key(PK; Name) { Clustered = true; }
    }
}

table 61703 "HN Ae Description"
{
    fields
    {
        field(1; Code; Code[20]) { }
        field(2; Description; Text[50]) { }
    }
    keys
    {
        key(PK; Code) { Clustered = true; }
        key(ByDescription; Description) { }
    }
}

table 61705 "HN Ae Code"
{
    fields
    {
        field(1; "No."; Code[20]) { }
    }
    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

codeunit 61704 "HN Rows Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        AeKey: Record "HN Ae Key";
        AccentKey: Record "HN Accent Key";
        AsciiKey: Record "HN Ascii Key";
        AeDescription: Record "HN Ae Description";
        AeCode: Record "HN Ae Code";
    begin
        AeKey.Name := 'Alpha';
        AeKey.Insert();
        AeKey.Name := 'ærø';
        AeKey.Insert();
        AeKey.Name := 'Bravo';
        AeKey.Insert();

        AccentKey.Name := 'Alpha';
        AccentKey.Insert();
        AccentKey.Name := 'Émile';
        AccentKey.Insert();
        AccentKey.Name := 'Bravo';
        AccentKey.Insert();

        AsciiKey.Name := 'Alpha';
        AsciiKey.Insert();
        AsciiKey.Name := 'Bravo';
        AsciiKey.Insert();

        AeDescription.Code := 'A';
        AeDescription.Description := 'Alpha';
        AeDescription.Insert();
        AeDescription.Code := 'B';
        AeDescription.Description := 'ærø';
        AeDescription.Insert();

        AeCode."No." := 'ALPHA';
        AeCode.Insert();
        AeCode."No." := 'ÆBLE';
        AeCode.Insert();
        AeCode."No." := 'BRAVO';
        AeCode.Insert();
    end;
}
