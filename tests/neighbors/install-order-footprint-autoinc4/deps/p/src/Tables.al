table 51400 "FP AutoInc Log"
{
    fields
    {
        field(1; "Entry No."; Integer) { AutoIncrement = true; }
        field(2; "Message"; Text[30]) { }
    }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}
