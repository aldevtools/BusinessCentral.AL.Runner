codeunit 50146 "Repro Tree Drop Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    local procedure AddRow(GroupCode: Code[10]; EntryNo: Integer; Name: Text[30]; Indentation: Integer)
    var
        Row: Record "Repro Tree Drop Row";
    begin
        Row."Group Code" := GroupCode;
        Row."Entry No." := EntryNo;
        Row.Name := Name;
        Row.Indentation := Indentation;
        Row.Insert();
    end;

    local procedure StoredRows(): Text
    var
        Row: Record "Repro Tree Drop Row";
        Result: Text;
    begin
        if Row.FindSet() then
            repeat
                Result += Row.Name + ',';
            until Row.Next() = 0;
        exit(Result);
    end;

    [TryFunction]
    local procedure TrySetName(var Rows: TestPage "Repro Tree Drop List"; NewName: Text)
    begin
        Rows.Name.SetValue(NewName);
    end;

    // A tree page (ShowAsTree, C indented under B) opens on its first row A (group G0). The Name control's OnValidate sets
    // SetRange on Group Code G1, which leaves A out of the view, and calls CurrPage.Update(false).
    [Test]
    procedure FirstRowSetRangeDropsRow_RaisesValidationError()
    var
        Row: Record "Repro Tree Drop Row";
        Rows: TestPage "Repro Tree Drop List";
        Failure: Text;
    begin
        Row.DeleteAll();
        AddRow('G0', 1, 'A', 0);
        AddRow('G1', 2, 'B', 0);
        AddRow('G1', 3, 'C', 1);
        Commit();

        Rows.OpenEdit();
        ClearLastError();
        if TrySetName(Rows, 'A2') then
            Failure := 'none'
        else
            Failure := GetLastErrorText();
        if Failure <> 'Validation error for Field: ,  Message = ''The view is filtered, and the entry is outside the filter. Some actions may not work.''' then
            Error('SetValue error: <%1>', Failure);
        if StoredRows() <> 'A2,B,C,' then
            Error('stored rows: <%1>', StoredRows());
    end;
}