codeunit 50131 "Repro ExtField Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure GetFieldByExtensionControlId_ReturnsTheExtensionField()
    var
        Row: Record "Repro ExtField Row";
        Card: TestPage "Repro ExtField Card";
        Shown: Text;
        ByName: Text;
    begin
        Row.DeleteAll();
        Row."No." := 'R1';
        Row.Name := 'Alpha';
        Row.Note := 'Gamma';
        Row.Insert();

        Card.OpenView();
        ByName := Card.ExtNote.Value();
        // ExtNoteControlId: the id the compiler gives the ExtNote control of page extension 50117 (the compiled app's
        // SymbolReference.json lists it).
        Shown := Card.GetField(ExtNoteControlId()).Value();
        Card.Close();
        if (ByName <> 'Gamma') or (Shown <> 'Gamma') then
            Error('GetField(%1) returned <%2>; the control by its name shows <%3>', ExtNoteControlId(), Shown, ByName);
    end;

    local procedure ExtNoteControlId(): Integer
    begin
        exit(962137182);
    end;
}
