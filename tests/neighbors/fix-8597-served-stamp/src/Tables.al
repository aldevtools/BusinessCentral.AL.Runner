// A per-company table with a secondary key, a Text field for case-insensitive filters, and a FlowField over Amount.
table 50170 "N97 Item"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; No; Code[20]) { }
        field(2; Group; Integer) { }
        field(3; Name; Text[30]) { }
        field(4; Amount; Decimal) { }
    }

    keys
    {
        key(PK; No) { Clustered = true; }
        key(ByGroup; Group, No) { }
    }
}

table 50171 "N97 Group"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; No; Integer) { }
        field(2; Total; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = sum("N97 Item".Amount where(Group = field(No)));
        }
    }

    keys
    {
        key(PK; No) { Clustered = true; }
    }
}
