table 50100 "Repro Row"
{
    fields
    {
        field(1; "Key"; Integer) { }
        field(2; Bool; Boolean) { }
    }
    keys { key(PK; "Key") { Clustered = true; } }
}

codeunit 50101 "SetFilter Boolean Test"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure SetFilterWithNumericCodeOnBooleanField()
    var
        RecRef: RecordRef;
        FR: FieldRef;
        C: Code[20];
    begin
        RecRef.Open(Database::"Repro Row");
        FR := RecRef.Field(2);
        C := '123';
        FR.SetFilter('%1', C);
        // BC 29: the filter is 'Yes' (as FR.Value := C gives true, on al-runner too)
        if FR.GetFilter() <> 'Yes' then
            Error('got [%1], BC gives [Yes]', FR.GetFilter());
    end;
}
