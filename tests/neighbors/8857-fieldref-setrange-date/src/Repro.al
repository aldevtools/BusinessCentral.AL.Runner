table 91800 "IDS Row"
{
    fields
    {
        field(1; "No."; Integer) { }
        field(2; "Posted"; Date) { }
    }
    keys { key(PK; "No.") { Clustered = true; } }
}

codeunit 91801 "IDS Test"
{
    Subtype = Test;

    [Test]
    procedure SetRangeIntegerOnDate()
    var
        Row: Record "IDS Row";
        RecRef: RecordRef;
        FRef: FieldRef;
    begin
        RecRef.GetTable(Row);
        FRef := RecRef.Field(2);
        asserterror FRef.SetRange(20240101);
        Error(GetLastErrorText());   // BC 29: the date-format error above; al-runner: not run (refused)
    end;
}
