table 50300 "Raw Control Buffer"
{
    TableType = Temporary;

    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; Data; Blob) { }
    }

    keys
    {
        key(PK; "Entry No.") { Clustered = true; }
    }
}
