// Neighbors of al-runner issue 8882 just past the cells of TreeValidateFilterShapes.al (corpus pull request 1554), on its
// pages: rows whose groups or number the cells do not hold, Expand(true) on the first row and a cell's rows on the page with
// both row triggers, and a filter that leaves the row out from the last of four top-level rows, from a first row whose
// children stay in the view, and with Update(true). Each test asserts what Business Central 29.1 answered (3 runs). Each
// codeunit's tests share its global Seen.

codeunit 97682 "LT TVS Neighbor Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        Log: Codeunit "LT TVS Log";
        Seen: Dictionary of [Text, Text];

    local procedure Initialize(UpdateKind: Integer)
    var
        Line: Record "LT TVS Line";
    begin
        Line.DeleteAll();
        Commit();
        Log.Setup(1, UpdateKind);
        Clear(Seen);
    end;

    local procedure AddLine(GroupCode: Code[10]; EntryNo: Integer; Name: Text[30]; Indentation: Integer)
    var
        Line: Record "LT TVS Line";
    begin
        Line."Group Code" := GroupCode;
        Line."Entry No." := EntryNo;
        Line.Name := Name;
        Line.Indentation := Indentation;
        Line.Insert();
    end;

    local procedure StoredLines(): Text
    var
        Line: Record "LT TVS Line";
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
    local procedure TrySetName(var Lines: TestPage "LT TVS Tree"; NewName: Text)
    begin
        Lines.Name.SetValue(NewName);
    end;

    // From the row the page stands on: the SetValue on Name, its error or none, the log, the rows stored, then (without an
    // error) First, the walk and Close.
    local procedure SetAndWalk(var Lines: TestPage "LT TVS Tree")
    var
        Walk: Text;
    begin
        Note('at', Lines.Name.Value() + ':' + Log.Take());
        ClearLastError();
        if TrySetName(Lines, 'X2') then
            Note('error', 'none')
        else
            Note('error', GetLastErrorCode() + ': ' + GetLastErrorText());
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

    [TryFunction]
    local procedure TrySetNameBoth(var Lines: TestPage "LT TVS Tree Both"; NewName: Text)
    begin
        Lines.Name.SetValue(NewName);
    end;

    // From the row the page stands on: the SetValue on Name, its error or none, the log, the rows stored, then (without an
    // error) First, the walk and Close.
    local procedure SetAndWalkBoth(var Lines: TestPage "LT TVS Tree Both")
    var
        Walk: Text;
    begin
        Note('at', Lines.Name.Value() + ':' + Log.Take());
        ClearLastError();
        if TrySetNameBoth(Lines, 'X2') then
            Note('error', 'none')
        else
            Note('error', GetLastErrorCode() + ': ' + GetLastErrorText());
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

    // A at the top, B and C under it, all in G1: SetRange on G1 keeps every row.
    [Test]
    procedure AllInOneGroup_UnderFirst()
    var
        Lines: TestPage "LT TVS Tree";
    begin
        Initialize(1);
        AddLine('G1', 1, 'A', 0);
        AddLine('G1', 2, 'B', 1);
        AddLine('G1', 3, 'C', 1);
        Commit();
        Lines.OpenEdit();
        Lines.Last();
        SetAndWalk(Lines);
        Expect('at', 'A:P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('error', 'none');
        Expect('setname', 'P.V(X2);P.AGR(X2);P.AGR(B);P.AGR(C);P.AGR(X2);');
        Expect('stored1', 'X2/,B/,C/,');
        Expect('walk', 'X2,');
        Expect('close', '');
        Expect('stored', 'X2/,B/,C/,');
    end;

    // The cells' rows and two more, D and E (G1) at the top: Last stands on E.
    [Test]
    procedure FiveRows_LastRow()
    var
        Lines: TestPage "LT TVS Tree";
    begin
        Initialize(1);
        AddLine('G0', 1, 'A', 0);
        AddLine('G1', 2, 'B', 0);
        AddLine('G1', 3, 'C', 1);
        AddLine('G1', 4, 'D', 0);
        AddLine('G1', 5, 'E', 0);
        Commit();
        Lines.OpenEdit();
        Lines.Last();
        SetAndWalk(Lines);
        Expect('at', 'E:P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(D);P.AGR(E);P.AGR(A);');
        Expect('error', 'none');
        Expect('setname', 'P.AGR(E);P.V(X2);P.AGR(X2);P.AGR(D);P.AGR(C);P.AGR(B);');
        Expect('stored1', 'A/,B/,C/,D/,X2/,');
        Expect('walk', 'B,D,X2,');
        Expect('close', 'P.AGR(X2);');
        Expect('stored', 'A/,B/,C/,D/,X2/,');
    end;

    // The rows of Shape011 (A and B in G1, C in G2, indented 0, 1, 1) on the page with both row triggers.
    [Test]
    procedure Both_Shape011()
    var
        Lines: TestPage "LT TVS Tree Both";
    begin
        Initialize(1);
        AddLine('G1', 1, 'A', 0);
        AddLine('G1', 2, 'B', 1);
        AddLine('G2', 3, 'C', 1);
        Commit();
        Lines.OpenEdit();
        Lines.Last();
        SetAndWalkBoth(Lines);
        Expect('at', 'A:P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);P.AGR(A);P.AGCR(A);');
        Expect('error', 'none');
        Expect('setname', 'P.V(X2);P.AGR(X2);P.AGR(B);P.AGR(X2);P.AGR(X2);P.AGCR(X2);');
        Expect('stored1', 'X2/,B/,C/,');
        Expect('walk', 'X2,');
        Expect('close', '');
        Expect('stored', 'X2/,B/,C/,');
    end;

    // Expand(true) on the first row A right after the open, then Last, on the page with both row triggers.
    [Test]
    procedure Both_ExpandTrueOnFirstRow()
    var
        Lines: TestPage "LT TVS Tree Both";
    begin
        Initialize(1);
        AddLine('G0', 1, 'A', 0);
        AddLine('G1', 2, 'B', 0);
        AddLine('G1', 3, 'C', 1);
        Commit();
        Lines.OpenEdit();
        Lines.Expand(true);
        Lines.Last();
        SetAndWalkBoth(Lines);
        Expect('at', 'B:P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);P.AGR(A);P.AGCR(A);P.AGR(B);P.AGR(B);P.AGCR(B);');
        Expect('error', 'none');
        Expect('setname', 'P.V(X2);P.AGR(X2);P.AGR(C);P.AGR(X2);P.AGR(X2);P.AGCR(X2);');
        Expect('stored1', 'A/,X2/,C/,');
        Expect('walk', 'X2,');
        Expect('close', '');
        Expect('stored', 'A/,X2/,C/,');
    end;

}

// A SetValue whose filter leaves the row out, and Update(true): in a codeunit of its own, as the runner refuses some of them.
codeunit 97683 "LT TVS Neighbor Drop Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        Log: Codeunit "LT TVS Log";
        Seen: Dictionary of [Text, Text];

    local procedure Initialize(UpdateKind: Integer)
    var
        Line: Record "LT TVS Line";
    begin
        Line.DeleteAll();
        Commit();
        Log.Setup(1, UpdateKind);
        Clear(Seen);
    end;

    local procedure AddLine(GroupCode: Code[10]; EntryNo: Integer; Name: Text[30]; Indentation: Integer)
    var
        Line: Record "LT TVS Line";
    begin
        Line."Group Code" := GroupCode;
        Line."Entry No." := EntryNo;
        Line.Name := Name;
        Line.Indentation := Indentation;
        Line.Insert();
    end;

    local procedure StoredLines(): Text
    var
        Line: Record "LT TVS Line";
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
    local procedure TrySetName(var Lines: TestPage "LT TVS Tree"; NewName: Text)
    begin
        Lines.Name.SetValue(NewName);
    end;

    // From the row the page stands on: the SetValue on Name, its error or none, the log, the rows stored, then (without an
    // error) First, the walk and Close.
    local procedure SetAndWalk(var Lines: TestPage "LT TVS Tree")
    var
        Walk: Text;
    begin
        Note('at', Lines.Name.Value() + ':' + Log.Take());
        ClearLastError();
        if TrySetName(Lines, 'X2') then
            Note('error', 'none')
        else
            Note('error', GetLastErrorCode() + ': ' + GetLastErrorText());
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

    // The cells' rows and D (G2) at the top: Last stands on D, which SetRange on G1 leaves out.
    [Test]
    procedure FourRows_LastRowDrops()
    var
        Lines: TestPage "LT TVS Tree";
    begin
        Initialize(1);
        AddLine('G0', 1, 'A', 0);
        AddLine('G1', 2, 'B', 0);
        AddLine('G1', 3, 'C', 1);
        AddLine('G2', 4, 'D', 0);
        Commit();
        Lines.OpenEdit();
        Lines.Last();
        SetAndWalk(Lines);
        Expect('at', 'D:P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(D);P.AGR(A);');
        Expect('error', 'TestValidation: Validation error for Field: ,  Message = ''The view is filtered, and the entry is outside the filter. Some actions may not work.''');
        Expect('setname', 'P.AGR(D);P.V(X2);P.AGR(C);P.AGR(B);');
        Expect('stored1', 'A/,B/,C/,X2/,');
        Expect('walk', '');
        Expect('close', '-');
        Expect('stored', 'A/,B/,C/,X2/,');
    end;

    // A (G0) at the top, B and C (G1) under it: from A, SetRange on G1 leaves A out and keeps its children.
    [Test]
    procedure FirstRowDrops_ChildrenStay()
    var
        Lines: TestPage "LT TVS Tree";
    begin
        Initialize(1);
        AddLine('G0', 1, 'A', 0);
        AddLine('G1', 2, 'B', 1);
        AddLine('G1', 3, 'C', 1);
        Commit();
        Lines.OpenEdit();
        SetAndWalk(Lines);
        Expect('at', 'A:P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('error', 'TestValidation: Validation error for Field: ,  Message = ''The view is filtered, and the entry is outside the filter. Some actions may not work.''');
        Expect('setname', 'P.V(X2);P.AGR(B);P.AGR(C);P.AGR(B);');
        Expect('stored1', 'X2/,B/,C/,');
        Expect('walk', '');
        Expect('close', '-');
        Expect('stored', 'X2/,B/,C/,');
    end;

    // The cells' rows: from A, SetRange on G1 leaves A out, with Update(true).
    [Test]
    procedure FirstRowDrops_UpdateTrue()
    var
        Lines: TestPage "LT TVS Tree";
    begin
        Initialize(2);
        AddLine('G0', 1, 'A', 0);
        AddLine('G1', 2, 'B', 0);
        AddLine('G1', 3, 'C', 1);
        Commit();
        Lines.OpenEdit();
        SetAndWalk(Lines);
        Expect('at', 'A:P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('error', 'none');
        Expect('setname', 'P.V(X2);P.AGR(B);P.AGR(C);P.AGR(B);');
        Expect('stored1', 'X2/,B/,C/,');
        Expect('walk', 'B,');
        Expect('close', '');
        Expect('stored', 'X2/,B/,C/,');
    end;
}
