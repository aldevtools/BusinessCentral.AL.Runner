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

// The review of #8905 at 01:31Z, on page 97672 (page 97720 of TreeValidateFilterRefresh.al): over rows A (G0), B (G1) and C
// (G1) indented 0, 0, 2, Last then a SetValue on Value first, which no cell over those rows makes (main's path); and three
// shapes the runner cannot tell from a cell before the OnValidate runs, which it refuses: over the same rows the SetValue on
// Name with Update(true), Expand(false) after Last then the SetValue on Name with Update(true), and after a cell that keeps
// the row (A and B in G1, C in G2, indented 0, 1, 1) a SetValue on Value without First.
codeunit 97684 "LT TVS Neighbor Path Tests"
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

    [TryFunction]
    local procedure TrySetValue(var Lines: TestPage "LT TVS Tree"; NewValue: Text)
    begin
        Lines.Value.SetValue(NewValue);
    end;

    local procedure Outcome(Ok: Boolean): Text
    begin
        if Ok then
            exit('none');
        exit(GetLastErrorCode() + ': ' + GetLastErrorText());
    end;

    // From the row the page stands on: the SetValue (on Name, or on Value), its error or none, the log, the rows stored, then
    // (without an error) First, the walk and Close.
    local procedure SetAndWalk(var Lines: TestPage "LT TVS Tree"; OnValue: Boolean)
    var
        Walk: Text;
    begin
        Note('at', Lines.Name.Value() + ':' + Log.Take());
        ClearLastError();
        if OnValue then
            Note('error', Outcome(TrySetValue(Lines, 'V2')))
        else
            Note('error', Outcome(TrySetName(Lines, 'X2')));
        Note('set', Log.Take());
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

    local procedure AddRows002()
    begin
        AddLine('G0', 1, 'A', 0);
        AddLine('G1', 2, 'B', 0);
        AddLine('G1', 3, 'C', 2);
        Commit();
    end;

    // Rows 0, 0, 2: Last, then the SetValue on Value first (its OnValidate calls Update(false)).
    [Test]
    procedure Rows002_LastThenValueFirst()
    var
        Lines: TestPage "LT TVS Tree";
    begin
        Initialize(1);
        AddRows002();
        Lines.OpenEdit();
        Lines.Last();
        SetAndWalk(Lines, true);
        Expect('at', 'B:P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('error', 'none');
        Expect('set', 'P.AGR(B);P.AGR(B);P.AGR(C);P.AGR(B);P.AGR(A);');
        Expect('stored1', 'A/,B/V2,C/,');
        Expect('walk', 'A,B,');
        Expect('close', 'P.AGR(B);');
        Expect('stored', 'A/,B/V2,C/,');
    end;

    // Rows 0, 0, 2: Last, then the SetValue on Name, SetRange on G1 with Update(true).
    [Test]
    procedure Rows002_LastNameUpdateTrue()
    var
        Lines: TestPage "LT TVS Tree";
    begin
        Initialize(2);
        AddRows002();
        Lines.OpenEdit();
        Lines.Last();
        SetAndWalk(Lines, false);
        Expect('at', 'B:P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('error', 'none');
        Expect('set', 'P.AGR(B);P.V(X2);P.AGR(X2);P.AGR(C);P.AGR(X2);');
        Expect('stored1', 'A/,X2/,C/,');
        Expect('walk', 'X2,');
        Expect('close', '');
        Expect('stored', 'A/,X2/,C/,');
    end;

    // The tree's rows: Last, Expand(false), then the SetValue on Name, SetRange on G1 with Update(true).
    [Test]
    procedure ExpandFalse_NameUpdateTrue()
    var
        Lines: TestPage "LT TVS Tree";
    begin
        Initialize(2);
        AddLine('G0', 1, 'A', 0);
        AddLine('G1', 2, 'B', 0);
        AddLine('G1', 3, 'C', 1);
        Commit();
        Lines.OpenEdit();
        Lines.Last();
        Lines.Expand(false);
        SetAndWalk(Lines, false);
        Expect('at', 'B:P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('error', 'none');
        Expect('set', 'P.AGR(B);P.V(X2);P.AGR(X2);P.AGR(C);P.AGR(X2);');
        Expect('stored1', 'A/,X2/,C/,');
        Expect('walk', 'X2,');
        Expect('close', '');
        Expect('stored', 'A/,X2/,C/,');
    end;

    // Rows 0, 1, 1 (A and B in G1, C in G2): Last, the SetValue on Name that keeps the row (Update(false)), then a SetValue on
    // Value without First, and Close.
    [Test]
    procedure KeepCell_ThenValueWithoutFirst()
    var
        Lines: TestPage "LT TVS Tree";
    begin
        Initialize(1);
        AddLine('G1', 1, 'A', 0);
        AddLine('G1', 2, 'B', 1);
        AddLine('G2', 3, 'C', 1);
        Commit();
        Lines.OpenEdit();
        Lines.Last();
        Note('at', Lines.Name.Value() + ':' + Log.Take());
        Lines.Name.SetValue('X2');
        Note('setname', Log.Take());
        Lines.Value.SetValue('V2');
        Note('setvalue', Lines.Name.Value() + ':' + Log.Take());
        Lines.Close();
        Note('close', Log.Take());
        Note('stored', StoredLines());
        Expect('at', 'A:P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setname', 'P.V(X2);P.AGR(X2);P.AGR(B);P.AGR(X2);');
        Expect('setvalue', 'X2:P.AGR(X2);P.AGR(B);P.AGR(X2);');
        Expect('close', '');
        Expect('stored', 'X2/V2,B/,C/,');
    end;
}

// The review of #8905 at 04:05Z, on page 97672 (page 97720): rows a cell of corpus PR 1539 holds, then Last and a call no cell
// over them makes next. The rule starts at the call, as on main, and refuses once the OnValidate or the action ran: the flat rows
// G0, G1, G2 with the SetValue on Value and with the action, the child in G0 before two top-level rows in G1 with the SetValue on
// Value, and the rows G0, G1, G5 with the SetValue on Name. Each test asserts what Business Central 29.1 answered (3 runs).
codeunit 97685 "LT TVS Neighbor Row Tests"
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
    local procedure TryCall(var Lines: TestPage "LT TVS Tree"; Call: Integer)
    begin
        case Call of
            0:
                Lines.Name.SetValue('X2');
            1:
                Lines.Value.SetValue('V2');
            2:
                Lines.ChangeViewAction.Invoke();
        end;
    end;

    // Last, then the call (0 the SetValue on Name, 1 the SetValue on Value, 2 the action), its error or none, the log, the rows
    // stored, then (without an error) First, the walk and Close.
    local procedure LastThen(var Lines: TestPage "LT TVS Tree"; Call: Integer)
    var
        Walk: Text;
        Error: Text;
    begin
        Lines.Last();
        Note('at', Lines.Name.Value() + ':' + Log.Take());
        ClearLastError();
        if TryCall(Lines, Call) then
            Error := 'none'
        else
            Error := GetLastErrorCode() + ': ' + GetLastErrorText();
        Note('error', Error);
        Note('call', Log.Take());
        Note('stored1', StoredLines());
        if Error = 'none' then begin
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

    local procedure AddFlat()
    begin
        AddLine('G0', 1, 'A', 0);
        AddLine('G1', 2, 'B', 0);
        AddLine('G2', 3, 'C', 0);
        Commit();
    end;

    [Test]
    procedure FlatRows_LastThenValue()
    var
        Lines: TestPage "LT TVS Tree";
    begin
        Initialize(1);
        AddFlat();
        Lines.OpenEdit();
        LastThen(Lines, 1);
        Expect('at', 'C:P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('error', 'none');
        Expect('call', 'P.AGR(C);P.AGR(C);P.AGR(B);P.AGR(A);');
        Expect('stored1', 'A/,B/,C/V2,');
        Expect('walk', 'A,B,C,');
        Expect('close', 'P.AGR(C);');
        Expect('stored', 'A/,B/,C/V2,');
    end;

    [Test]
    procedure FlatRows_LastThenAction()
    var
        Lines: TestPage "LT TVS Tree";
    begin
        Initialize(1);
        AddFlat();
        Lines.OpenEdit();
        LastThen(Lines, 2);
        Expect('at', 'C:P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('error', 'none');
        Expect('call', 'P.AGR(C);P.AGR(C);P.A(C);P.AGR(B);P.AGR(B);');
        Expect('stored1', 'A/,B/,C/,');
        Expect('walk', 'B,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    [Test]
    procedure ChildInG0_LastThenValue()
    var
        Lines: TestPage "LT TVS Tree";
    begin
        Initialize(1);
        AddLine('G0', 1, 'A', 1);
        AddLine('G1', 2, 'B', 0);
        AddLine('G1', 3, 'C', 0);
        Commit();
        Lines.OpenEdit();
        LastThen(Lines, 1);
        Expect('at', 'C:P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('error', 'none');
        Expect('call', 'P.AGR(C);P.AGR(C);P.AGR(B);P.AGR(A);');
        Expect('stored1', 'A/,B/,C/V2,');
        Expect('walk', 'A,B,C,');
        Expect('close', 'P.AGR(C);');
        Expect('stored', 'A/,B/,C/V2,');
    end;

    [Test]
    procedure GroupsG0G1G5_LastThenName()
    var
        Lines: TestPage "LT TVS Tree";
    begin
        Initialize(1);
        AddLine('G0', 1, 'A', 0);
        AddLine('G1', 2, 'B', 0);
        AddLine('G5', 3, 'C', 0);
        Commit();
        Lines.OpenEdit();
        LastThen(Lines, 0);
        Expect('at', 'C:P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('error', 'TestValidation: Validation error for Field: ,  Message = ''The view is filtered, and the entry is outside the filter. Some actions may not work.''');
        Expect('call', 'P.AGR(C);P.V(X2);P.AGR(B);');
        Expect('stored1', 'A/,B/,X2/,');
        Expect('walk', '');
        Expect('close', '-');
        Expect('stored', 'A/,B/,X2/,');
    end;
}
