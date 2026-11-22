table 50110 "N8462 Entry"
{
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; Description; Text[100]) { }
        field(10; KeyField1; Code[50]) { }
        field(11; KeyField2; Code[50]) { }
    }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}

table 50111 "N8462 Temporary"
{
    TableType = Temporary;
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; Description; Text[100]) { }
        field(10; KeyField1; Code[50]) { }
    }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}
