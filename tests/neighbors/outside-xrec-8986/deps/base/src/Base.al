// The reporter's base table (al-runner issue 8986), plus a procedure that reads xRec.
table 50300 "NX Line"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; "Serial No."; Integer) { }
        field(3; "xRec Serial In Table OnInsert"; Integer) { }
    }

    keys
    {
        key(PK; "Entry No.") { Clustered = true; }
    }

    trigger OnInsert()
    begin
        "xRec Serial In Table OnInsert" := xRec."Serial No.";
    end;

    procedure NxBaseXSerial(): Integer
    begin
        exit(xRec."Serial No.");
    end;
}
