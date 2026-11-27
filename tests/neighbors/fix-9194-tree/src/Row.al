table 86270 "CKK Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "No."; Integer) { }
        field(2; Txt; Text[30]) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}
