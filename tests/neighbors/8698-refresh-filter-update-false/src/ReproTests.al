codeunit 50123 "Repro Filter Update Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Log: Codeunit "Repro Log";

    [Test]
    procedure SetFilterThenUpdateFalse_ReadsTheStoredRowAndAppliesTheFilter()
    var
        Line: Record "Repro Line";
        Lines: TestPage "Repro Lines Filter Update";
    begin
        Line.DeleteAll();
        AddRow('G0', 1, 'A');
        AddRow('G1', 2, 'B');
        AddRow('G1', 3, 'C');
        Commit();
        Lines.OpenEdit();
        Lines.Last();
        Log.Take();

        // The Name control's OnValidate sets a filter and calls CurrPage.Update(false): the page reads the row again from
        // the table, so the typed name is gone, then the row before it in the new view.
        Lines.Name.SetValue('C2');
        Check('SetValue Name log', 'P.AGR(C);T.V(C2);P.AGR(C);P.AGR(B);', Log.Take());
        Check('shown after SetValue Name', 'C/', Lines.Name.Value() + '/' + Lines.Value.Value());

        // The Value control's OnValidate calls CurrPage.Update(false): the typed value is gone too.
        Lines.Value.SetValue('V');
        Check('SetValue Value log', 'P.AGR(C);P.AGR(C);P.AGR(B);', Log.Take());
        Check('shown after SetValue Value', 'C/', Lines.Name.Value() + '/' + Lines.Value.Value());

        // The filter the OnValidate set is the page's view: First() lands on B, the first row of group G1.
        Lines.First();
        Check('First', 'B', Lines.Name.Value());
        Lines.Close();
        Line.Get('G1', 3);
        Check('stored row', 'C/', Line.Name + '/' + Line.Value);
    end;

    local procedure AddRow(GroupCode: Code[10]; EntryNo: Integer; Name: Text[30])
    var
        Line: Record "Repro Line";
    begin
        Line.Init();
        Line."Group Code" := GroupCode;
        Line."Entry No." := EntryNo;
        Line.Name := Name;
        Line.Insert();
    end;

    local procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;
}
