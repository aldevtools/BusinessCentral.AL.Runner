// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/page/page-update-method
// Scope: in-scope
// local object: the control's OnValidate that changes a tree page's view, and the page triggers that log, are what is under test
// Fixtures used: shared Assert (60021); own tables 97670, 97677, logs 97671, 97681, pages 97672-97675, 97678-97679, tests 97676, 97680
// Probe (al-runner issue 8882): the tree pages of TreeValidateFilterRefresh.al, line for line (a tree with OnAfterGetRecord only,
//   with both row triggers, with OnAfterGetCurrRecord only, and with TreeInitialState = CollapseAll), and its page 97714 (no row
//   trigger) with CollapseAll and with OnAfterGetCurrRecord, over rows and with calls its cells do not hold. The Name control's
//   OnValidate runs SetRange on G1 and CurrPage.Update(false). Cells: on the tree with OnAfterGetRecord, rows indented 0, 1, 1 and
//   0, 1, 2 with the first two in G1 (Last stands on the first row), 0, 0, 2, a fourth row, Expand(false) after Last, and
//   Expand(true) on the first row, each then the SetValue on Name, First, the walk and Close; on each of the six trees the SetValue
//   on Name from the first row right after the open, where the filter leaves the row out of the view. Readings: the page triggers
//   each step runs, in order; the error of the SetValue; the row the page stands on; the rows the walk reaches; the stored rows.
// BC versions: 27, 28, 29

table 97670 "LT TVS Line"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; "Group Code"; Code[10]) { }
        field(2; "Entry No."; Integer) { }
        field(3; Name; Text[30]) { }
        field(4; Value; Text[30]) { }
        field(5; Indentation; Integer) { }
    }

    keys
    {
        key(PK; "Group Code", "Entry No.") { Clustered = true; }
    }
}

codeunit 97671 "LT TVS Log"
{
    SingleInstance = true;

    var
        Buffer: Text;
        Filtering: Integer;
        Updating: Integer;

    procedure Add(Entry: Text)
    begin
        Buffer += Entry + ';';
    end;

    procedure Take(): Text
    var
        Result: Text;
    begin
        Result := Buffer;
        Buffer := '';
        exit(Result);
    end;

    // What the configurable pages' ChangeView does: Filter 0 nothing, 1 SetRange G1, 2 SetFilter G1, 3 Reset, 4 SetRange G0;
    // Update 0 no call, 1 CurrPage.Update(false), 2 CurrPage.Update(true).
    procedure Setup(NewFilter: Integer; NewUpdate: Integer)
    begin
        Buffer := '';
        Filtering := NewFilter;
        Updating := NewUpdate;
    end;

    procedure FilterStep(): Integer
    begin
        exit(Filtering);
    end;

    procedure UpdateStep(): Integer
    begin
        exit(Updating);
    end;
}

page 97672 "LT TVS Tree"
{
    PageType = List;
    SourceTable = "LT TVS Line";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                IndentationColumn = Rec.Indentation;
                IndentationControls = Name;
                ShowAsTree = true;
                TreeInitialState = ExpandAll;

                field(Name; Rec.Name)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    begin
                        Log.Add('P.V(' + Rec.Name + ')');
                        ChangeView();
                    end;
                }
                field(Value; Rec.Value)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    begin
                        CurrPage.Update(false);
                    end;
                }
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(ChangeViewAction)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    Log.Add('P.A(' + Rec.Name + ')');
                    ChangeView();
                end;
            }
        }
    }

    trigger OnAfterGetRecord()
    begin
        Log.Add('P.AGR(' + Rec.Name + ')');
    end;

    var
        Log: Codeunit "LT TVS Log";

    local procedure ChangeView()
    begin
        case Log.FilterStep() of
            1:
                Rec.SetRange("Group Code", 'G1');
            2:
                Rec.SetFilter("Group Code", 'G1');
            3:
                Rec.Reset();
            4:
                Rec.SetRange("Group Code", 'G0');
        end;
        case Log.UpdateStep() of
            1:
                CurrPage.Update(false);
            2:
                CurrPage.Update(true);
        end;
    end;
}

page 97673 "LT TVS Tree Both"
{
    PageType = List;
    SourceTable = "LT TVS Line";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                IndentationColumn = Rec.Indentation;
                IndentationControls = Name;
                ShowAsTree = true;
                TreeInitialState = ExpandAll;

                field(Name; Rec.Name)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    begin
                        Log.Add('P.V(' + Rec.Name + ')');
                        ChangeView();
                    end;
                }
                field(Value; Rec.Value)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    begin
                        CurrPage.Update(false);
                    end;
                }
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(ChangeViewAction)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    Log.Add('P.A(' + Rec.Name + ')');
                    ChangeView();
                end;
            }
        }
    }

    trigger OnAfterGetRecord()
    begin
        Log.Add('P.AGR(' + Rec.Name + ')');
    end;

    trigger OnAfterGetCurrRecord()
    begin
        Log.Add('P.AGCR(' + Rec.Name + ')');
    end;

    var
        Log: Codeunit "LT TVS Log";

    local procedure ChangeView()
    begin
        case Log.FilterStep() of
            1:
                Rec.SetRange("Group Code", 'G1');
            2:
                Rec.SetFilter("Group Code", 'G1');
            3:
                Rec.Reset();
            4:
                Rec.SetRange("Group Code", 'G0');
        end;
        case Log.UpdateStep() of
            1:
                CurrPage.Update(false);
            2:
                CurrPage.Update(true);
        end;
    end;
}

page 97674 "LT TVS Tree Curr"
{
    PageType = List;
    SourceTable = "LT TVS Line";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                IndentationColumn = Rec.Indentation;
                IndentationControls = Name;
                ShowAsTree = true;
                TreeInitialState = ExpandAll;

                field(Name; Rec.Name)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    begin
                        Log.Add('P.V(' + Rec.Name + ')');
                        ChangeView();
                    end;
                }
                field(Value; Rec.Value)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    begin
                        CurrPage.Update(false);
                    end;
                }
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(ChangeViewAction)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    Log.Add('P.A(' + Rec.Name + ')');
                    ChangeView();
                end;
            }
        }
    }

    trigger OnAfterGetCurrRecord()
    begin
        Log.Add('P.AGCR(' + Rec.Name + ')');
    end;

    var
        Log: Codeunit "LT TVS Log";

    local procedure ChangeView()
    begin
        case Log.FilterStep() of
            1:
                Rec.SetRange("Group Code", 'G1');
            2:
                Rec.SetFilter("Group Code", 'G1');
            3:
                Rec.Reset();
            4:
                Rec.SetRange("Group Code", 'G0');
        end;
        case Log.UpdateStep() of
            1:
                CurrPage.Update(false);
            2:
                CurrPage.Update(true);
        end;
    end;
}

page 97675 "LT TVS Tree Collapsed"
{
    PageType = List;
    SourceTable = "LT TVS Line";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                IndentationColumn = Rec.Indentation;
                IndentationControls = Name;
                ShowAsTree = true;
                TreeInitialState = CollapseAll;

                field(Name; Rec.Name)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    begin
                        Log.Add('P.V(' + Rec.Name + ')');
                        ChangeView();
                    end;
                }
                field(Value; Rec.Value)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    begin
                        CurrPage.Update(false);
                    end;
                }
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(ChangeViewAction)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    Log.Add('P.A(' + Rec.Name + ')');
                    ChangeView();
                end;
            }
        }
    }

    trigger OnAfterGetRecord()
    begin
        Log.Add('P.AGR(' + Rec.Name + ')');
    end;

    var
        Log: Codeunit "LT TVS Log";

    local procedure ChangeView()
    begin
        case Log.FilterStep() of
            1:
                Rec.SetRange("Group Code", 'G1');
            2:
                Rec.SetFilter("Group Code", 'G1');
            3:
                Rec.Reset();
            4:
                Rec.SetRange("Group Code", 'G0');
        end;
        case Log.UpdateStep() of
            1:
                CurrPage.Update(false);
            2:
                CurrPage.Update(true);
        end;
    end;
}

codeunit 97676 "LT TVS Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        Log: Codeunit "LT TVS Log";
        Seen: Dictionary of [Text, Text];

    local procedure Initialize()
    var
        Line: Record "LT TVS Line";
    begin
        Line.DeleteAll();
        Commit();
        Log.Setup(1, 1);
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

    // After Initialize: Last, then the SetValue, over rows in groups G1, G1 and G2 indented 0, 1 and Third.
    local procedure RunUnderFirst(Third: Integer)
    var
        Lines: TestPage "LT TVS Tree";
    begin
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
        Initialize();
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
        Initialize();
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
        Lines: TestPage "LT TVS Tree";
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
        Lines: TestPage "LT TVS Tree";
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
        Lines: TestPage "LT TVS Tree";
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
        Lines: TestPage "LT TVS Tree";
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

    [TryFunction]
    local procedure TrySetNameTree(var Lines: TestPage "LT TVS Tree"; NewName: Text)
    begin
        Lines.Name.SetValue(NewName);
    end;

    // The cells' rows A (G0), B and C (G1), C under B: right after the open, on the first row A, the SetValue on Name, whose
    // SetRange on G1 leaves A out of the view.
    [Test]
    procedure Tree_FirstRowSetRangeDropsUpdateFalse()
    var
        Lines: TestPage "LT TVS Tree";
    begin
        Initialize();
        AddLine('G0', 1, 'A', 0);
        AddLine('G1', 2, 'B', 0);
        AddLine('G1', 3, 'C', 1);
        Commit();
        Lines.OpenEdit();
        Note('open', Lines.Name.Value() + ':' + Log.Take());
        ClearLastError();
        if TrySetNameTree(Lines, 'A2') then
            Note('error', 'none')
        else
            Note('error', GetLastErrorCode() + ': ' + GetLastErrorText());
        Note('setname', Log.Take());
        Note('stored', StoredLines());
        Expect('open', 'A:P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('error', 'TestValidation: Validation error for Field: ,  Message = ''The view is filtered, and the entry is outside the filter. Some actions may not work.''');
        Expect('setname', 'P.V(A2);P.AGR(B);P.AGR(C);P.AGR(B);');
        Expect('stored', 'A2/,B/,C/,');
    end;

    [TryFunction]
    local procedure TrySetNameBoth(var Lines: TestPage "LT TVS Tree Both"; NewName: Text)
    begin
        Lines.Name.SetValue(NewName);
    end;

    // The cells' rows A (G0), B and C (G1), C under B: right after the open, on the first row A, the SetValue on Name, whose
    // SetRange on G1 leaves A out of the view.
    [Test]
    procedure Both_FirstRowSetRangeDropsUpdateFalse()
    var
        Lines: TestPage "LT TVS Tree Both";
    begin
        Initialize();
        AddLine('G0', 1, 'A', 0);
        AddLine('G1', 2, 'B', 0);
        AddLine('G1', 3, 'C', 1);
        Commit();
        Lines.OpenEdit();
        Note('open', Lines.Name.Value() + ':' + Log.Take());
        ClearLastError();
        if TrySetNameBoth(Lines, 'A2') then
            Note('error', 'none')
        else
            Note('error', GetLastErrorCode() + ': ' + GetLastErrorText());
        Note('setname', Log.Take());
        Note('stored', StoredLines());
        Expect('open', 'A:P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);P.AGR(A);P.AGCR(A);');
        Expect('error', 'TestValidation: Validation error for Field: ,  Message = ''The view is filtered, and the entry is outside the filter. Some actions may not work.''');
        Expect('setname', 'P.V(A2);P.AGR(B);P.AGR(C);P.AGR(B);P.AGR(B);P.AGCR(B);');
        Expect('stored', 'A2/,B/,C/,');
    end;

    [TryFunction]
    local procedure TrySetNameCurr(var Lines: TestPage "LT TVS Tree Curr"; NewName: Text)
    begin
        Lines.Name.SetValue(NewName);
    end;

    // The cells' rows A (G0), B and C (G1), C under B: right after the open, on the first row A, the SetValue on Name, whose
    // SetRange on G1 leaves A out of the view.
    [Test]
    procedure Curr_FirstRowSetRangeDropsUpdateFalse()
    var
        Lines: TestPage "LT TVS Tree Curr";
    begin
        Initialize();
        AddLine('G0', 1, 'A', 0);
        AddLine('G1', 2, 'B', 0);
        AddLine('G1', 3, 'C', 1);
        Commit();
        Lines.OpenEdit();
        Note('open', Lines.Name.Value() + ':' + Log.Take());
        ClearLastError();
        if TrySetNameCurr(Lines, 'A2') then
            Note('error', 'none')
        else
            Note('error', GetLastErrorCode() + ': ' + GetLastErrorText());
        Note('setname', Log.Take());
        Note('stored', StoredLines());
        Expect('open', 'A:P.AGCR(A);');
        Expect('error', 'TestValidation: Validation error for Field: ,  Message = ''The view is filtered, and the entry is outside the filter. Some actions may not work.''');
        Expect('setname', 'P.V(A2);P.AGCR(B);');
        Expect('stored', 'A2/,B/,C/,');
    end;

    [TryFunction]
    local procedure TrySetNameCollapsed(var Lines: TestPage "LT TVS Tree Collapsed"; NewName: Text)
    begin
        Lines.Name.SetValue(NewName);
    end;

    // The cells' rows A (G0), B and C (G1), C under B: right after the open, on the first row A, the SetValue on Name, whose
    // SetRange on G1 leaves A out of the view.
    [Test]
    procedure Collapsed_FirstRowSetRangeDropsUpdateFalse()
    var
        Lines: TestPage "LT TVS Tree Collapsed";
    begin
        Initialize();
        AddLine('G0', 1, 'A', 0);
        AddLine('G1', 2, 'B', 0);
        AddLine('G1', 3, 'C', 1);
        Commit();
        Lines.OpenEdit();
        Note('open', Lines.Name.Value() + ':' + Log.Take());
        ClearLastError();
        if TrySetNameCollapsed(Lines, 'A2') then
            Note('error', 'none')
        else
            Note('error', GetLastErrorCode() + ': ' + GetLastErrorText());
        Note('setname', Log.Take());
        Note('stored', StoredLines());
        Expect('open', 'A:P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('error', 'TestValidation: Validation error for Field: ,  Message = ''The view is filtered, and the entry is outside the filter. Some actions may not work.''');
        Expect('setname', 'P.V(A2);P.AGR(B);P.AGR(C);P.AGR(B);');
        Expect('stored', 'A2/,B/,C/,');
    end;
}

// The issue's tree of TreeValidateFilterRefresh.al without OnAfterGetRecord (page 97714 there, the page of al-runner issue 8846's
// repro), with TreeInitialState = CollapseAll, and with OnAfterGetCurrRecord: the SetValue on Name from the first row A right
// after the open, whose SetRange on G1 leaves A out of the view of B and C (rows A (G0), B and C (G1), C under B).

// The log of page 97679.
codeunit 97681 "LT TVS Drop Log"
{
    SingleInstance = true;

    var
        Buffer: Text;

    procedure Add(Entry: Text)
    begin
        Buffer += Entry + ';';
    end;

    procedure Take(): Text
    var
        Result: Text;
    begin
        Result := Buffer;
        Buffer := '';
        exit(Result);
    end;
}

table 97677 "LT TVS Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Group Code"; Code[10]) { }
        field(2; "Entry No."; Integer) { }
        field(3; Name; Text[30]) { }
        field(4; Indentation; Integer) { }
    }

    keys
    {
        key(PK; "Group Code", "Entry No.") { Clustered = true; }
    }
}

// Page 97714 of TreeValidateFilterRefresh.al with TreeInitialState = CollapseAll.
page 97678 "LT TVS Drop Tree Collapsed"
{
    PageType = List;
    SourceTable = "LT TVS Row";
    ApplicationArea = All;
    UsageCategory = Lists;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                IndentationColumn = Rec.Indentation;
                IndentationControls = Name;
                ShowAsTree = true;
                TreeInitialState = CollapseAll;

                field(Name; Rec.Name)
                {
                    trigger OnValidate()
                    begin
                        Rec.SetRange("Group Code", 'G1');
                        CurrPage.Update(false);
                    end;
                }
            }
        }
    }
}

// Page 97714 of TreeValidateFilterRefresh.al with OnAfterGetCurrRecord.
page 97679 "LT TVS Drop Tree Curr"
{
    PageType = List;
    SourceTable = "LT TVS Row";
    ApplicationArea = All;
    UsageCategory = Lists;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                IndentationColumn = Rec.Indentation;
                IndentationControls = Name;
                ShowAsTree = true;
                TreeInitialState = ExpandAll;

                field(Name; Rec.Name)
                {
                    trigger OnValidate()
                    begin
                        Rec.SetRange("Group Code", 'G1');
                        CurrPage.Update(false);
                    end;
                }
            }
        }
    }

    trigger OnAfterGetCurrRecord()
    begin
        Log.Add('AGCR(' + Rec.Name + ')');
    end;

    var
        Log: Codeunit "LT TVS Drop Log";
}

codeunit 97680 "LT TVS Drop Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        Log: Codeunit "LT TVS Drop Log";
        Seen: Dictionary of [Text, Text];

    local procedure Initialize()
    var
        Row: Record "LT TVS Row";
    begin
        Row.DeleteAll();
        AddRow('G0', 1, 'A', 0);
        AddRow('G1', 2, 'B', 0);
        AddRow('G1', 3, 'C', 1);
        Commit();
        Log.Take();
        Clear(Seen);
    end;

    local procedure AddRow(GroupCode: Code[10]; EntryNo: Integer; Name: Text[30]; Indentation: Integer)
    var
        Row: Record "LT TVS Row";
    begin
        Row."Group Code" := GroupCode;
        Row."Entry No." := EntryNo;
        Row.Name := Name;
        Row.Indentation := Indentation;
        Row.Insert();
    end;

    local procedure StoredRows(): Text
    var
        Row: Record "LT TVS Row";
        Result: Text;
    begin
        if Row.FindSet() then
            repeat
                Result += Row.Name + ',';
            until Row.Next() = 0;
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
    local procedure TrySetNameCollapsed(var Rows: TestPage "LT TVS Drop Tree Collapsed"; NewName: Text)
    begin
        Rows.Name.SetValue(NewName);
    end;

    // CollapseAll, no row trigger.
    [Test]
    procedure DropCollapsed_FirstRowSetRangeDropsUpdateFalse()
    var
        Rows: TestPage "LT TVS Drop Tree Collapsed";
    begin
        Initialize();
        Rows.OpenEdit();
        Note('open', Rows.Name.Value() + ':' + Log.Take());
        ClearLastError();
        if TrySetNameCollapsed(Rows, 'A2') then
            Note('error', 'none')
        else
            Note('error', GetLastErrorCode() + ': ' + GetLastErrorText());
        Note('setname', Log.Take());
        Note('stored', StoredRows());
        Expect('open', 'A:');
        Expect('error', 'TestValidation: Validation error for Field: ,  Message = ''The view is filtered, and the entry is outside the filter. Some actions may not work.''');
        Expect('setname', '');
        Expect('stored', 'A2,B,C,');
    end;

    [TryFunction]
    local procedure TrySetNameCurr(var Rows: TestPage "LT TVS Drop Tree Curr"; NewName: Text)
    begin
        Rows.Name.SetValue(NewName);
    end;

    // OnAfterGetCurrRecord only.
    [Test]
    procedure DropCurr_FirstRowSetRangeDropsUpdateFalse()
    var
        Rows: TestPage "LT TVS Drop Tree Curr";
    begin
        Initialize();
        Rows.OpenEdit();
        Note('open', Rows.Name.Value() + ':' + Log.Take());
        ClearLastError();
        if TrySetNameCurr(Rows, 'A2') then
            Note('error', 'none')
        else
            Note('error', GetLastErrorCode() + ': ' + GetLastErrorText());
        Note('setname', Log.Take());
        Note('stored', StoredRows());
        Expect('open', 'A:AGCR(A);');
        Expect('error', 'TestValidation: Validation error for Field: ,  Message = ''The view is filtered, and the entry is outside the filter. Some actions may not work.''');
        Expect('setname', 'AGCR(B);');
        Expect('stored', 'A2,B,C,');
    end;
}
