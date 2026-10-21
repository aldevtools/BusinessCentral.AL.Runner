table 50270 "Debug Outcome Row"
{
    fields
    {
        field(1; "No."; Integer) { }
        field(2; Name; Text[30]) { }
        field(3; Amount; Decimal) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }

    trigger OnInsert()
    begin
        if Name = '' then
            Name := 'Row ' + Format("No.");
    end;
}
