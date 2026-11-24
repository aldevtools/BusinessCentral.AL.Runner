// Neighbors of al-runner issue 8833 just past the cells of TreeValidateFilterRefresh.al (corpus pull request 1539): a second
// SetValue on Name. Business Central 29.1 answered as the test asserts (3 runs). The neighbors that stood here for a SetValue
// on Name from the first row and for rows indented otherwise than the cells' are cells of TreeValidateFilterShapes.al now
// (corpus pull request 1554), in tests/neighbors/8882-tree-shapes.

codeunit 97725 "LT TVN Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        Log: Codeunit "LT TVF Log";
        Seen: Dictionary of [Text, Text];

    local procedure Initialize(FilterKind: Integer; UpdateKind: Integer)
    var
        Line: Record "LT TVF Line";
    begin
        Line.DeleteAll();
        AddLine('G0', 1, 'A', 0);
        AddLine('G1', 2, 'B', 0);
        AddLine('G1', 3, 'C', 1);
        Commit();
        Log.Setup(FilterKind, UpdateKind);
        Clear(Seen);
    end;

    local procedure AddLine(GroupCode: Code[10]; EntryNo: Integer; Name: Text[30]; Indentation: Integer)
    var
        Line: Record "LT TVF Line";
    begin
        Line."Group Code" := GroupCode;
        Line."Entry No." := EntryNo;
        Line.Name := Name;
        Line.Indentation := Indentation;
        Line.Insert();
    end;

    local procedure StoredLines(): Text
    var
        Line: Record "LT TVF Line";
        Result: Text;
    begin
        if Line.FindSet() then
            repeat
                Result += Line.Name + '/' + Line.Value + ',';
            until Line.Next() = 0;
        exit(Result);
    end;

    local procedure Note(What: Text; Actual: Text)
    begin
        Seen.Set(What, Actual);
    end;

    local procedure Expect(What: Text; Expected: Text)
    begin
        Assert.AreEqual(Expected, Seen.Get(What), What);
    end;

    local procedure Walk(var Lines: TestPage "LT TVF Tree"): Text
    var
        Result: Text;
    begin
        Lines.First();
        repeat
            Result += Lines.Name.Value() + ',';
        until not Lines.Next();
        exit(Result);
    end;

    [TryFunction]
    local procedure TrySetName(var Lines: TestPage "LT TVF Tree"; NewName: Text)
    begin
        Lines.Name.SetValue(NewName);
    end;

    // Two SetValues on Name, the second with no change of view: no cell makes the second.
    [Test]
    procedure Tree_NameTwiceUpdateFalse()
    var
        Lines: TestPage "LT TVF Tree";
    begin
        Initialize(1, 1);
        Lines.OpenEdit();
        Lines.Last();
        Log.Take();
        Lines.Name.SetValue('C2');
        Log.Take();
        Lines.Name.SetValue('C3');
        Note('setname2', Log.Take());
        Note('stored', StoredLines());
        Note('walk', Walk(Lines));
        Lines.Close();
        Expect('setname2', 'P.V(C3);P.AGR(C3);P.AGR(C);P.AGR(C3);');
        Expect('stored', 'A/,C3/,C/,');
        Expect('walk', 'C3,');
    end;
}
