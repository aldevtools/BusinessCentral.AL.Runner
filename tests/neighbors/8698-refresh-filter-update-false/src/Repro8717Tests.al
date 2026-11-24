codeunit 50128 "Repro Edit Card Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Log: Codeunit "Repro Edit Log";

    [Test]
    procedure UpdateFalseInOnValidate_ReadsTheStoredRowAgain()
    var
        Row: Record "Repro Edit Row";
        Card: TestPage "Repro Edit Card";
    begin
        Row.DeleteAll();
        Row."No." := 'R1';
        Row.Name := 'Alpha';
        Row.Insert();
        Commit();
        Card.OpenEdit();
        Card.GoToKey('R1');
        Log.Take();

        // The Name control's OnValidate calls CurrPage.Update(false): the page reads the row again from the table, so the
        // typed name is gone.
        Card.Name.SetValue('Beta');
        Check('SetValue log', 'AGR(Alpha);', Log.Take());
        Check('shown after SetValue', 'Alpha', Card.Name.Value());

        Card.Close();
        Row.Get('R1');
        Check('stored after Close', 'Alpha', Row.Name);
    end;

    local procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;
}
