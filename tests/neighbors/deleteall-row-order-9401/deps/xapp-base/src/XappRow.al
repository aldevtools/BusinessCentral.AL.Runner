// A table of another app, for the table extension of the neighbor app (an extension from another app than its table).
table 94041 "DRN Xapp Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}
