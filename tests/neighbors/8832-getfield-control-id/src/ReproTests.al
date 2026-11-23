codeunit 50108 "Repro GetField Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure GetFieldByControlId_ReturnsTheControl()
    var
        Row: Record "Repro GetField Row";
        Rows: TestPage "Repro GetField List";
        Shown: Text;
    begin
        Row.DeleteAll();
        Row."No." := 'R1';
        Row.Name := 'Alpha';
        Row.Insert();

        Rows.OpenView();
        Rows.First();
        // NameControlId: the id the compiler gives the Name control of page 50107 (the compiled app's SymbolReference.json
        // lists it).
        Shown := Rows.GetField(NameControlId()).Value();
        Rows.Close();
        if Shown <> 'Alpha' then
            Error('GetField(%1) returned <%2>', NameControlId(), Shown);
    end;

    local procedure NameControlId(): Integer
    begin
        exit(356257777);
    end;
}
