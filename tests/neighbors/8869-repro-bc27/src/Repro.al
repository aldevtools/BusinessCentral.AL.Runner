table 50100 "Repro Row"
{
    fields
    {
        field(1; "Key"; Integer) { }
        field(2; Int; Integer) { }
    }
    keys { key(PK; "Key") { Clustered = true; } }
}

codeunit 50101 "SetFilter Error Text Test"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure SetFilterWithTextOnIntegerFieldRaisesBcsText()
    var
        RecRef: RecordRef;
        FR: FieldRef;
        C: Code[20];
    begin
        RecRef.Open(Database::"Repro Row");
        FR := RecRef.Field(2);
        C := 'ABC';
        asserterror FR.SetFilter('%1', C);
        // BC 29 raises exactly this text
        if GetLastErrorText() <> 'The value "ABC" can''t be evaluated into type Integer.' then
            Error('got [%1], BC gives [The value "ABC" can''t be evaluated into type Integer.]', GetLastErrorText());
    end;
}
