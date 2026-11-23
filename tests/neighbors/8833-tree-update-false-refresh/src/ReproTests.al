codeunit 50110 "Repro Tree Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Log: Codeunit "Repro Tree Log";

    local procedure Initialize()
    var
        Row: Record "Repro Tree Row";
    begin
        Row.DeleteAll();
        AddRow('G0', 1, 'A', 0);
        AddRow('G1', 2, 'B', 0);
        AddRow('G1', 3, 'C', 1);
        Commit();
        Log.Take();
    end;

    local procedure AddRow(GroupCode: Code[10]; EntryNo: Integer; Name: Text[30]; Indentation: Integer)
    var
        Row: Record "Repro Tree Row";
    begin
        Row."Group Code" := GroupCode;
        Row."Entry No." := EntryNo;
        Row.Name := Name;
        Row.Indentation := Indentation;
        Row.Insert();
    end;

    local procedure StoredRows(): Text
    var
        Row: Record "Repro Tree Row";
        Result: Text;
    begin
        if Row.FindSet() then
            repeat
                Result += Row.Name + ',';
            until Row.Next() = 0;
        exit(Result);
    end;

    local procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Actual <> Expected then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;

    // A tree page (ShowAsTree, C indented under B) stands on its last row B. The Name control's OnValidate sets SetRange on
    // Group Code G1 and calls CurrPage.Update(false).
    [Test]
    procedure UpdateFalseOnTree_SavesTheRowAndShowsTheNewView()
    var
        Rows: TestPage "Repro Tree List";
        Walk: Text;
    begin
        Initialize();
        Rows.OpenEdit();
        Rows.Last();
        Check('open', 'B:AGR(A);AGR(B);AGR(C);AGR(A);', Rows.Name.Value() + ':' + Log.Take());
        Rows.Name.SetValue('C2');
        Check('SetValue log', 'AGR(B);AGR(C2);AGR(C);AGR(C2);', Log.Take());
        Check('shown after the SetValue', 'C2', Rows.Name.Value());
        Check('stored after the SetValue', 'A,C2,C,', StoredRows());
        Rows.First();
        repeat
            Walk += Rows.Name.Value() + ',';
        until not Rows.Next();
        Check('walk', 'C2,', Walk);
        Rows.Close();
    end;
}
