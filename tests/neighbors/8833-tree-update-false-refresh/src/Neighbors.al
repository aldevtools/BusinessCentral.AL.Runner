// Neighbors of al-runner issue 8833 just past the cells of TreeValidateFilterRefresh.al (corpus pull request 1539): a second
// SetValue on Name, a SetValue on Name from the first row, and on page 97720 the rows indented otherwise than the cells' 0, 0, 1,
// Expand(false) after Last, Expand(true) on the first row, and a table of four rows. Each test asserts what Business
// Central 29.1 answered (3 runs). Each codeunit's tests share its global Seen, so a refused test stands alone.

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

// The SetValue on Name from the first row, in a codeunit of its own: the page it leaves behind after its error stays open.
codeunit 97726 "LT TVN First Row Tests"
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

    // The SetValue on Name from the first row A, right after the open: SetRange on G1 drops it.
    [Test]
    procedure Tree_FirstRowSetRangeDropsUpdateFalse()
    var
        Lines: TestPage "LT TVF Tree";
    begin
        Initialize(1, 1);
        Lines.OpenEdit();
        Log.Take();
        ClearLastError();
        if TrySetName(Lines, 'A2') then
            Note('error', 'none')
        else
            Note('error', GetLastErrorText());
        Note('setname', Log.Take());
        Note('stored', StoredLines());
        Expect('error', 'Validation error for Field: ,  Message = ''The view is filtered, and the entry is outside the filter. Some actions may not work.''');
        Expect('setname', 'P.V(A2);P.AGR(B);P.AGR(C);P.AGR(B);');
        Expect('stored', 'A2/,B/,C/,');
    end;
}

// Page 97720 over rows the cells do not hold, and Expand calls no cell makes; SetRange on G1 and Update(false) in the Name
// control's OnValidate, as in the cells.
codeunit 97728 "LT TVN Shape Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        Log: Codeunit "LT TVF Log";
        Seen: Dictionary of [Text, Text];

    local procedure Initialize()
    var
        Line: Record "LT TVF Line";
    begin
        Line.DeleteAll();
        Commit();
        Log.Setup(1, 1);
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

    [TryFunction]
    local procedure TrySetName(var Lines: TestPage "LT TVF Tree"; NewName: Text)
    begin
        Lines.Name.SetValue(NewName);
    end;

    // From the row the page stands on: the SetValue on Name, its error or none, the log, the rows stored, then (without an
    // error) First, the walk and Close.
    local procedure SetAndWalk(var Lines: TestPage "LT TVF Tree")
    var
        Walk: Text;
    begin
        Note('at', Lines.Name.Value() + ':' + Log.Take());
        ClearLastError();
        if TrySetName(Lines, 'X2') then
            Note('error', 'none')
        else
            Note('error', GetLastErrorText());
        Note('setname', Log.Take());
        Note('stored1', StoredLines());
        if Seen.Get('error') = 'none' then begin
            Lines.First();
            repeat
                Walk += Lines.Name.Value() + ',';
            until not Lines.Next();
            Log.Take();
            Lines.Close();
            Note('close', Log.Take());
        end else
            Note('close', '-');
        Note('walk', Walk);
        Note('stored', StoredLines());
    end;

    // Last, then the SetValue, over rows in groups G1, G1 and G2 indented 0, 1 and Third.
    local procedure RunUnderFirst(Third: Integer)
    var
        Lines: TestPage "LT TVF Tree";
    begin
        Initialize();
        AddLine('G1', 1, 'A', 0);
        AddLine('G1', 2, 'B', 1);
        AddLine('G2', 3, 'C', Third);
        Commit();
        Lines.OpenEdit();
        Lines.Last();
        SetAndWalk(Lines);
    end;

    [Test]
    procedure Shape011_SetRangeKeepsUpdateFalse()
    begin
        RunUnderFirst(1);
        Expect('at', 'A:P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('error', 'none');
        Expect('setname', 'P.V(X2);P.AGR(X2);P.AGR(B);P.AGR(X2);');
        Expect('stored1', 'X2/,B/,C/,');
        Expect('walk', 'X2,');
        Expect('close', '');
        Expect('stored', 'X2/,B/,C/,');
    end;

    [Test]
    procedure Shape012_SetRangeKeepsUpdateFalse()
    begin
        RunUnderFirst(2);
        Expect('at', 'A:P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('error', 'none');
        Expect('setname', 'P.V(X2);P.AGR(X2);P.AGR(B);P.AGR(X2);');
        Expect('stored1', 'X2/,B/,C/,');
        Expect('walk', 'X2,');
        Expect('close', '');
        Expect('stored', 'X2/,B/,C/,');
    end;

    // The cells' rows A (G0), B and C (G1), but C indented 2.
    [Test]
    procedure Shape002_SetRangeKeepsUpdateFalse()
    var
        Lines: TestPage "LT TVF Tree";
    begin
        Initialize();
        AddLine('G0', 1, 'A', 0);
        AddLine('G1', 2, 'B', 0);
        AddLine('G1', 3, 'C', 2);
        Commit();
        Lines.OpenEdit();
        Lines.Last();
        SetAndWalk(Lines);
        Expect('at', 'B:P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('error', 'none');
        Expect('setname', 'P.AGR(B);P.V(X2);P.AGR(X2);P.AGR(C);P.AGR(X2);');
        Expect('stored1', 'A/,X2/,C/,');
        Expect('walk', 'X2,');
        Expect('close', '');
        Expect('stored', 'A/,X2/,C/,');
    end;

    // The cells' rows and a fourth, D (G1) at the top.
    [Test]
    procedure FourRows_SetRangeKeepsUpdateFalse()
    var
        Lines: TestPage "LT TVF Tree";
    begin
        Initialize();
        AddLine('G0', 1, 'A', 0);
        AddLine('G1', 2, 'B', 0);
        AddLine('G1', 3, 'C', 1);
        AddLine('G1', 4, 'D', 0);
        Commit();
        Lines.OpenEdit();
        Lines.Last();
        SetAndWalk(Lines);
        Expect('at', 'D:P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(D);P.AGR(A);');
        Expect('error', 'none');
        Expect('setname', 'P.AGR(D);P.V(X2);P.AGR(X2);P.AGR(C);P.AGR(B);');
        Expect('stored1', 'A/,B/,C/,X2/,');
        Expect('walk', 'B,X2,');
        Expect('close', 'P.AGR(X2);');
        Expect('stored', 'A/,B/,C/,X2/,');
    end;

    // The cells' rows: Last, Expand(false) on B, then the SetValue.
    [Test]
    procedure ExpandFalseAfterLast_SetRangeKeepsUpdateFalse()
    var
        Lines: TestPage "LT TVF Tree";
    begin
        Initialize();
        AddLine('G0', 1, 'A', 0);
        AddLine('G1', 2, 'B', 0);
        AddLine('G1', 3, 'C', 1);
        Commit();
        Lines.OpenEdit();
        Lines.Last();
        Lines.Expand(false);
        SetAndWalk(Lines);
        Expect('at', 'B:P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('error', 'none');
        Expect('setname', 'P.AGR(B);P.V(X2);P.AGR(X2);P.AGR(C);P.AGR(X2);');
        Expect('stored1', 'A/,X2/,C/,');
        Expect('walk', 'X2,');
        Expect('close', '');
        Expect('stored', 'A/,X2/,C/,');
    end;

    // The cells' rows: Expand(true) on the first row A right after the open, Last, then the SetValue.
    [Test]
    procedure ExpandTrueOnFirstRow_SetRangeKeepsUpdateFalse()
    var
        Lines: TestPage "LT TVF Tree";
    begin
        Initialize();
        AddLine('G0', 1, 'A', 0);
        AddLine('G1', 2, 'B', 0);
        AddLine('G1', 3, 'C', 1);
        Commit();
        Lines.OpenEdit();
        Lines.Expand(true);
        Lines.Last();
        SetAndWalk(Lines);
        Expect('at', 'B:P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('error', 'none');
        Expect('setname', 'P.AGR(B);P.V(X2);P.AGR(X2);P.AGR(C);P.AGR(X2);');
        Expect('stored1', 'A/,X2/,C/,');
        Expect('walk', 'X2,');
        Expect('close', '');
        Expect('stored', 'A/,X2/,C/,');
    end;
}
