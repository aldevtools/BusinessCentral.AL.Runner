// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/page/page-update-method
// Scope: in-scope
// local object: the control's OnValidate and the action that change a tree page's view, and the page triggers that log, are what is under test
// Fixtures used: shared Assert (60021); own tables 97716, 97719, log codeunit 97717, pages 97714, 97718, 97720-97723, 97727, tests 97724
// Probe (al-runner issues 8833, 8846): a tree page (ShowAsTree over Indentation, C indented under B) stands on its last top-level
//   row B of three rows A (group G0), B and C (G1), opened with OpenEdit. SetValue on Name runs the control's OnValidate, which sets
//   a filter on Rec and calls CurrPage.Update; on the configurable pages a SetValue on Value (CurrPage.Update(false)) follows. Then
//   First(), a walk with Next(), Close, and the rows the table holds. Readings: the page triggers each step runs, in order; the
//   values the page shows; the rows First and the walk reach; the stored rows. Cells: the issue's page (97718, the repro's page),
//   also with B expanded first; on the configurable pages Update(false), Update(true) and no Update, a filter that keeps the row
//   (SetRange or SetFilter on G1), no filter change and Reset of no filter, the same from an action, B expanded first, and the
//   SetValue on Value alone, on a tree with OnAfterGetRecord only (97720), with both row triggers (97721), with
//   OnAfterGetCurrRecord only (97722), and with TreeInitialState = CollapseAll (97723); a filter that leaves the row out, from the
//   first, middle and last row, on 97720 over the tree's rows, over rows none indented and in other groups, on the List 97727
//   (97720 without ShowAsTree), on 97718 and on 97714 (97718 without OnAfterGetRecord, the page of issue 8846's repro). The cells
//   whose SetValue raises the filtered-view error leave their tree page behind, so they run after the other cells.
// BC versions: 27, 28, 29

table 97716 "LT TVF Row"
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

codeunit 97717 "LT TVF Log"
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

// The issue's page, line for line as the repro's: the Name control's OnValidate sets a filter on Rec and calls
// CurrPage.Update(false). OnAfterGetRecord, no OnAfterGetCurrRecord.
page 97718 "LT TVF Issue Tree"
{
    PageType = List;
    SourceTable = "LT TVF Row";
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

    trigger OnAfterGetRecord()
    begin
        Log.Add('AGR(' + Rec.Name + ')');
    end;

    var
        Log: Codeunit "LT TVF Log";
}

table 97719 "LT TVF Line"
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

// A tree with OnAfterGetRecord only; the Name control's OnValidate and the action ChangeView do what the log codeunit's Setup says.
page 97720 "LT TVF Tree"
{
    PageType = List;
    SourceTable = "LT TVF Line";
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
        Log: Codeunit "LT TVF Log";

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

// The same with OnAfterGetRecord and OnAfterGetCurrRecord.
page 97721 "LT TVF Tree Both"
{
    PageType = List;
    SourceTable = "LT TVF Line";
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
        Log: Codeunit "LT TVF Log";

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

// The same with OnAfterGetCurrRecord only.
page 97722 "LT TVF Tree Curr"
{
    PageType = List;
    SourceTable = "LT TVF Line";
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
        Log: Codeunit "LT TVF Log";

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

// The tree with OnAfterGetRecord only, with TreeInitialState = CollapseAll.
page 97723 "LT TVF Tree Collapsed"
{
    PageType = List;
    SourceTable = "LT TVF Line";
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
        Log: Codeunit "LT TVF Log";

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

codeunit 97724 "LT TVF Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        Log: Codeunit "LT TVF Log";
        Seen: Dictionary of [Text, Text];

    local procedure Initialize()
    var
        Row: Record "LT TVF Row";
        Line: Record "LT TVF Line";
    begin
        Row.DeleteAll();
        Line.DeleteAll();
        AddRow('G0', 1, 'A', 0);
        AddRow('G1', 2, 'B', 0);
        AddRow('G1', 3, 'C', 1);
        AddLine('G0', 1, 'A', 0);
        AddLine('G1', 2, 'B', 0);
        AddLine('G1', 3, 'C', 1);
        Commit();
        Log.Setup(0, 0);
        Clear(Seen);
    end;

    local procedure AddRow(GroupCode: Code[10]; EntryNo: Integer; Name: Text[30]; Indentation: Integer)
    var
        Row: Record "LT TVF Row";
    begin
        Row."Group Code" := GroupCode;
        Row."Entry No." := EntryNo;
        Row.Name := Name;
        Row.Indentation := Indentation;
        Row.Insert();
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

    local procedure StoredRows(): Text
    var
        Row: Record "LT TVF Row";
        Result: Text;
    begin
        if Row.FindSet() then
            repeat
                Result += Row.Name + ',';
            until Row.Next() = 0;
        exit(Result);
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

    // The repro's steps on the issue's page: Last, the SetValue on Name, then First, the walk and Close; with Expanded, Expand(true)
    // on B after Last.
    local procedure RunIssue(Expanded: Boolean)
    var
        Rows: TestPage "LT TVF Issue Tree";
        Walk: Text;
    begin
        Rows.OpenEdit();
        Rows.Last();
        Note('open', Rows.Name.Value() + ':' + Log.Take());
        if Expanded then begin
            Rows.Expand(true);
            Note('expand', Log.Take());
        end;
        Rows.Name.SetValue('C2');
        Note('setname', Log.Take());
        Note('shown', Rows.Name.Value());
        Note('stored1', StoredRows());
        Rows.First();
        Note('first', Rows.Name.Value() + ':' + Log.Take());
        repeat
            Walk += Rows.Name.Value() + ',';
        until not Rows.Next();
        Note('walk', Walk);
        Log.Take();
        Rows.Close();
        Note('close', Log.Take());
        Note('stored', StoredRows());
    end;

    // Last, the SetValue on Name (or the action, with ViaAction) with Log.Setup(FilterKind, UpdateKind), the SetValue on Value, then First,
    // the walk and Close; with Expanded, Expand(true) on B after Last.
    local procedure RunTree(FilterKind: Integer; UpdateKind: Integer; Expanded: Boolean; ViaAction: Boolean)
    var
        Lines: TestPage "LT TVF Tree";
        Walk: Text;
    begin
        Log.Setup(FilterKind, UpdateKind);
        Lines.OpenEdit();
        Lines.Last();
        Note('open', Log.Take());
        if Expanded then begin
            Lines.Expand(true);
            Note('expand', Log.Take());
        end;
        if ViaAction then
            Lines.ChangeViewAction.Invoke()
        else
            Lines.Name.SetValue('C2');
        Note('setname', Log.Take());
        Note('shown', Lines.Name.Value() + '/' + Lines.Value.Value());
        Note('stored1', StoredLines());
        Lines.Value.SetValue('V');
        Note('setvalue', Log.Take());
        Note('shown2', Lines.Name.Value() + '/' + Lines.Value.Value());
        Lines.First();
        Note('first', Lines.Name.Value() + ':' + Log.Take());
        repeat
            Walk += Lines.Name.Value() + ',';
        until not Lines.Next();
        Note('walk', Walk);
        Log.Take();
        Lines.Close();
        Note('close', Log.Take());
        Note('stored', StoredLines());
    end;

    // Last, the SetValue on Name (or the action, with ViaAction) with Log.Setup(FilterKind, UpdateKind), the SetValue on Value, then First,
    // the walk and Close; with Expanded, Expand(true) on B after Last.
    local procedure RunBoth(FilterKind: Integer; UpdateKind: Integer; Expanded: Boolean; ViaAction: Boolean)
    var
        Lines: TestPage "LT TVF Tree Both";
        Walk: Text;
    begin
        Log.Setup(FilterKind, UpdateKind);
        Lines.OpenEdit();
        Lines.Last();
        Note('open', Log.Take());
        if Expanded then begin
            Lines.Expand(true);
            Note('expand', Log.Take());
        end;
        if ViaAction then
            Lines.ChangeViewAction.Invoke()
        else
            Lines.Name.SetValue('C2');
        Note('setname', Log.Take());
        Note('shown', Lines.Name.Value() + '/' + Lines.Value.Value());
        Note('stored1', StoredLines());
        Lines.Value.SetValue('V');
        Note('setvalue', Log.Take());
        Note('shown2', Lines.Name.Value() + '/' + Lines.Value.Value());
        Lines.First();
        Note('first', Lines.Name.Value() + ':' + Log.Take());
        repeat
            Walk += Lines.Name.Value() + ',';
        until not Lines.Next();
        Note('walk', Walk);
        Log.Take();
        Lines.Close();
        Note('close', Log.Take());
        Note('stored', StoredLines());
    end;

    // Last, the SetValue on Name (or the action, with ViaAction) with Log.Setup(FilterKind, UpdateKind), the SetValue on Value, then First,
    // the walk and Close; with Expanded, Expand(true) on B after Last.
    local procedure RunCurr(FilterKind: Integer; UpdateKind: Integer; Expanded: Boolean; ViaAction: Boolean)
    var
        Lines: TestPage "LT TVF Tree Curr";
        Walk: Text;
    begin
        Log.Setup(FilterKind, UpdateKind);
        Lines.OpenEdit();
        Lines.Last();
        Note('open', Log.Take());
        if Expanded then begin
            Lines.Expand(true);
            Note('expand', Log.Take());
        end;
        if ViaAction then
            Lines.ChangeViewAction.Invoke()
        else
            Lines.Name.SetValue('C2');
        Note('setname', Log.Take());
        Note('shown', Lines.Name.Value() + '/' + Lines.Value.Value());
        Note('stored1', StoredLines());
        Lines.Value.SetValue('V');
        Note('setvalue', Log.Take());
        Note('shown2', Lines.Name.Value() + '/' + Lines.Value.Value());
        Lines.First();
        Note('first', Lines.Name.Value() + ':' + Log.Take());
        repeat
            Walk += Lines.Name.Value() + ',';
        until not Lines.Next();
        Note('walk', Walk);
        Log.Take();
        Lines.Close();
        Note('close', Log.Take());
        Note('stored', StoredLines());
    end;

    // Last, the SetValue on Name (or the action, with ViaAction) with Log.Setup(FilterKind, UpdateKind), the SetValue on Value, then First,
    // the walk and Close; with Expanded, Expand(true) on B after Last.
    local procedure RunCollapsed(FilterKind: Integer; UpdateKind: Integer; Expanded: Boolean; ViaAction: Boolean)
    var
        Lines: TestPage "LT TVF Tree Collapsed";
        Walk: Text;
    begin
        Log.Setup(FilterKind, UpdateKind);
        Lines.OpenEdit();
        Lines.Last();
        Note('open', Log.Take());
        if Expanded then begin
            Lines.Expand(true);
            Note('expand', Log.Take());
        end;
        if ViaAction then
            Lines.ChangeViewAction.Invoke()
        else
            Lines.Name.SetValue('C2');
        Note('setname', Log.Take());
        Note('shown', Lines.Name.Value() + '/' + Lines.Value.Value());
        Note('stored1', StoredLines());
        Lines.Value.SetValue('V');
        Note('setvalue', Log.Take());
        Note('shown2', Lines.Name.Value() + '/' + Lines.Value.Value());
        Lines.First();
        Note('first', Lines.Name.Value() + ':' + Log.Take());
        repeat
            Walk += Lines.Name.Value() + ',';
        until not Lines.Next();
        Note('walk', Walk);
        Log.Take();
        Lines.Close();
        Note('close', Log.Take());
        Note('stored', StoredLines());
    end;

    // The issue's cell: the repro's steps.
    [Test]
    procedure Issue_SetRangeUpdateFalse_SavesRowAndAppliesFilter()
    begin
        Initialize();
        RunIssue(false);
        Expect('open', 'B:AGR(A);AGR(B);AGR(C);AGR(A);');
        Expect('setname', 'AGR(B);AGR(C2);AGR(C);AGR(C2);');
        Expect('shown', 'C2');
        Expect('stored1', 'A,C2,C,');
        Expect('first', 'C2:');
        Expect('walk', 'C2,');
        Expect('close', '');
        Expect('stored', 'A,C2,C,');
    end;

    // The same with B expanded first.
    [Test]
    procedure Issue_Expanded_SetRangeUpdateFalse()
    begin
        Initialize();
        RunIssue(true);
        Expect('open', 'B:AGR(A);AGR(B);AGR(C);AGR(A);');
        Expect('expand', '');
        Expect('setname', 'AGR(B);AGR(C2);AGR(C);AGR(C2);');
        Expect('shown', 'C2');
        Expect('stored1', 'A,C2,C,');
        Expect('first', 'C2:');
        Expect('walk', 'C2,C,');
        Expect('close', 'AGR(C);');
        Expect('stored', 'A,C2,C,');
    end;

    // The SetValue on Name with SetRangeKeeps and UpdateFalse.
    [Test]
    procedure Tree_SetRangeKeepsUpdateFalse()
    begin
        Initialize();
        RunTree(1, 1, false, false);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setname', 'P.AGR(B);P.V(C2);P.AGR(C2);P.AGR(C);P.AGR(C2);');
        Expect('shown', 'C2/');
        Expect('stored1', 'A/,C2/,C/,');
        Expect('setvalue', 'P.AGR(C2);P.AGR(C);P.AGR(C2);');
        Expect('shown2', 'C2/V');
        Expect('first', 'C2:');
        Expect('walk', 'C2,');
        Expect('close', '');
        Expect('stored', 'A/,C2/V,C/,');
    end;

    // The SetValue on Name with SetFilterKeeps and UpdateFalse.
    [Test]
    procedure Tree_SetFilterKeepsUpdateFalse()
    begin
        Initialize();
        RunTree(2, 1, false, false);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setname', 'P.AGR(B);P.V(C2);P.AGR(C2);P.AGR(C);P.AGR(C2);');
        Expect('shown', 'C2/');
        Expect('stored1', 'A/,C2/,C/,');
        Expect('setvalue', 'P.AGR(C2);P.AGR(C);P.AGR(C2);');
        Expect('shown2', 'C2/V');
        Expect('first', 'C2:');
        Expect('walk', 'C2,');
        Expect('close', '');
        Expect('stored', 'A/,C2/V,C/,');
    end;

    // The SetValue on Name with SetRangeKeeps and UpdateTrue.
    [Test]
    procedure Tree_SetRangeKeepsUpdateTrue()
    begin
        Initialize();
        RunTree(1, 2, false, false);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setname', 'P.AGR(B);P.V(C2);P.AGR(C2);P.AGR(C);P.AGR(C2);');
        Expect('shown', 'C2/');
        Expect('stored1', 'A/,C2/,C/,');
        Expect('setvalue', 'P.AGR(C2);P.AGR(C);P.AGR(C2);');
        Expect('shown2', 'C2/V');
        Expect('first', 'C2:');
        Expect('walk', 'C2,');
        Expect('close', '');
        Expect('stored', 'A/,C2/V,C/,');
    end;

    // The SetValue on Name with SetRangeKeeps and NoUpdate.
    [Test]
    procedure Tree_SetRangeKeepsNoUpdate()
    begin
        Initialize();
        RunTree(1, 0, false, false);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setname', 'P.AGR(B);P.V(C2);');
        Expect('shown', 'C2/');
        Expect('stored1', 'A/,B/,C/,');
        Expect('setvalue', 'P.AGR(C2);P.AGR(C);P.AGR(C2);');
        Expect('shown2', 'C2/V');
        Expect('first', 'C2:');
        Expect('walk', 'C2,');
        Expect('close', '');
        Expect('stored', 'A/,C2/V,C/,');
    end;

    // The SetValue on Name with NoChange and UpdateFalse.
    [Test]
    procedure Tree_NoChangeUpdateFalse()
    begin
        Initialize();
        RunTree(0, 1, false, false);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setname', 'P.AGR(B);P.V(C2);P.AGR(C2);P.AGR(C);P.AGR(C2);P.AGR(A);');
        Expect('shown', 'C2/');
        Expect('stored1', 'A/,C2/,C/,');
        Expect('setvalue', 'P.AGR(C2);P.AGR(C2);P.AGR(C);P.AGR(C2);P.AGR(A);');
        Expect('shown2', 'C2/V');
        Expect('first', 'A:');
        Expect('walk', 'A,C2,');
        Expect('close', 'P.AGR(C2);');
        Expect('stored', 'A/,C2/V,C/,');
    end;

    // The SetValue on Name with ResetNoFilter and UpdateFalse.
    [Test]
    procedure Tree_ResetNoFilterUpdateFalse()
    begin
        Initialize();
        RunTree(3, 1, false, false);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setname', 'P.AGR(B);P.V(C2);P.AGR(C2);P.AGR(C);P.AGR(C2);P.AGR(A);');
        Expect('shown', 'C2/');
        Expect('stored1', 'A/,C2/,C/,');
        Expect('setvalue', 'P.AGR(C2);P.AGR(C2);P.AGR(C);P.AGR(C2);P.AGR(A);');
        Expect('shown2', 'C2/V');
        Expect('first', 'A:');
        Expect('walk', 'A,C2,');
        Expect('close', 'P.AGR(C2);');
        Expect('stored', 'A/,C2/V,C/,');
    end;

    // The SetValue on Name with SetRangeKeeps and UpdateFalse, B expanded first.
    [Test]
    procedure Tree_SetRangeKeepsUpdateFalse_Expanded()
    begin
        Initialize();
        RunTree(1, 1, true, false);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('expand', '');
        Expect('setname', 'P.AGR(B);P.V(C2);P.AGR(C2);P.AGR(C);P.AGR(C2);');
        Expect('shown', 'C2/');
        Expect('stored1', 'A/,C2/,C/,');
        Expect('setvalue', 'P.AGR(C2);P.AGR(C);P.AGR(C2);');
        Expect('shown2', 'C2/V');
        Expect('first', 'C2:');
        Expect('walk', 'C2,C,');
        Expect('close', 'P.AGR(C);');
        Expect('stored', 'A/,C2/V,C/,');
    end;

    // The action with SetRangeKeeps and UpdateFalse.
    [Test]
    procedure Tree_ActionSetRangeKeepsUpdateFalse()
    begin
        Initialize();
        RunTree(1, 1, false, true);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setname', 'P.AGR(B);P.AGR(B);P.A(B);P.AGR(B);P.AGR(B);P.AGR(C);P.AGR(B);');
        Expect('shown', 'B/');
        Expect('stored1', 'A/,B/,C/,');
        Expect('setvalue', 'P.AGR(B);P.AGR(C);P.AGR(B);');
        Expect('shown2', 'B/V');
        Expect('first', 'B:');
        Expect('walk', 'B,');
        Expect('close', '');
        Expect('stored', 'A/,B/V,C/,');
    end;

    // The SetValue on Name with SetRangeKeeps and UpdateFalse.
    [Test]
    procedure Both_SetRangeKeepsUpdateFalse()
    begin
        Initialize();
        RunBoth(1, 1, false, false);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);P.AGR(A);P.AGCR(A);P.AGR(B);P.AGR(B);P.AGCR(B);');
        Expect('setname', 'P.V(C2);P.AGR(C2);P.AGR(C);P.AGR(C2);P.AGR(C2);P.AGCR(C2);');
        Expect('shown', 'C2/');
        Expect('stored1', 'A/,C2/,C/,');
        Expect('setvalue', 'P.AGR(C2);P.AGR(C);P.AGR(C2);P.AGR(C2);P.AGCR(C2);');
        Expect('shown2', 'C2/V');
        Expect('first', 'C2:');
        Expect('walk', 'C2,');
        Expect('close', '');
        Expect('stored', 'A/,C2/V,C/,');
    end;

    // The SetValue on Name with SetRangeKeeps and UpdateTrue.
    [Test]
    procedure Both_SetRangeKeepsUpdateTrue()
    begin
        Initialize();
        RunBoth(1, 2, false, false);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);P.AGR(A);P.AGCR(A);P.AGR(B);P.AGR(B);P.AGCR(B);');
        Expect('setname', 'P.V(C2);P.AGR(C2);P.AGR(C);P.AGR(C2);P.AGR(C2);P.AGCR(C2);');
        Expect('shown', 'C2/');
        Expect('stored1', 'A/,C2/,C/,');
        Expect('setvalue', 'P.AGR(C2);P.AGR(C);P.AGR(C2);P.AGR(C2);P.AGCR(C2);');
        Expect('shown2', 'C2/V');
        Expect('first', 'C2:');
        Expect('walk', 'C2,');
        Expect('close', '');
        Expect('stored', 'A/,C2/V,C/,');
    end;

    // The SetValue on Name with NoChange and UpdateFalse.
    [Test]
    procedure Both_NoChangeUpdateFalse()
    begin
        Initialize();
        RunBoth(0, 1, false, false);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);P.AGR(A);P.AGCR(A);P.AGR(B);P.AGR(B);P.AGCR(B);');
        Expect('setname', 'P.V(C2);P.AGR(C2);P.AGR(C);P.AGR(C2);P.AGR(A);P.AGR(C2);P.AGR(C2);P.AGCR(C2);');
        Expect('shown', 'C2/');
        Expect('stored1', 'A/,C2/,C/,');
        Expect('setvalue', 'P.AGR(C2);P.AGR(C);P.AGR(C2);P.AGR(A);P.AGR(C2);P.AGR(C2);P.AGCR(C2);');
        Expect('shown2', 'C2/V');
        Expect('first', 'A:P.AGR(A);P.AGR(A);P.AGCR(A);');
        Expect('walk', 'A,C2,');
        Expect('close', '');
        Expect('stored', 'A/,C2/V,C/,');
    end;

    // The SetValue on Name with SetRangeKeeps and UpdateFalse, B expanded first.
    [Test]
    procedure Both_SetRangeKeepsUpdateFalse_Expanded()
    begin
        Initialize();
        RunBoth(1, 1, true, false);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);P.AGR(A);P.AGCR(A);P.AGR(B);P.AGR(B);P.AGCR(B);');
        Expect('expand', '');
        Expect('setname', 'P.V(C2);P.AGR(C2);P.AGR(C);P.AGR(C2);P.AGR(C2);P.AGCR(C2);');
        Expect('shown', 'C2/');
        Expect('stored1', 'A/,C2/,C/,');
        Expect('setvalue', 'P.AGR(C2);P.AGR(C);P.AGR(C2);P.AGR(C2);P.AGCR(C2);');
        Expect('shown2', 'C2/V');
        Expect('first', 'C2:');
        Expect('walk', 'C2,C,');
        Expect('close', '');
        Expect('stored', 'A/,C2/V,C/,');
    end;

    // The SetValue on Name with SetRangeKeeps and UpdateFalse.
    [Test]
    procedure Curr_SetRangeKeepsUpdateFalse()
    begin
        Initialize();
        RunCurr(1, 1, false, false);
        Expect('open', 'P.AGCR(A);P.AGCR(B);');
        Expect('setname', 'P.V(C2);P.AGCR(C2);');
        Expect('shown', 'C2/');
        Expect('stored1', 'A/,C2/,C/,');
        Expect('setvalue', 'P.AGCR(C2);');
        Expect('shown2', 'C2/V');
        Expect('first', 'C2:');
        Expect('walk', 'C2,');
        Expect('close', '');
        Expect('stored', 'A/,C2/V,C/,');
    end;

    // The SetValue on Name with NoChange and UpdateFalse.
    [Test]
    procedure Curr_NoChangeUpdateFalse()
    begin
        Initialize();
        RunCurr(0, 1, false, false);
        Expect('open', 'P.AGCR(A);P.AGCR(B);');
        Expect('setname', 'P.V(C2);P.AGCR(C2);');
        Expect('shown', 'C2/');
        Expect('stored1', 'A/,C2/,C/,');
        Expect('setvalue', 'P.AGCR(C2);');
        Expect('shown2', 'C2/V');
        Expect('first', 'A:P.AGCR(A);');
        Expect('walk', 'A,C2,');
        Expect('close', '');
        Expect('stored', 'A/,C2/V,C/,');
    end;

    // The SetValue on Name with SetRangeKeeps and UpdateFalse.
    [Test]
    procedure Collapsed_SetRangeKeepsUpdateFalse()
    begin
        Initialize();
        RunCollapsed(1, 1, false, false);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setname', 'P.AGR(B);P.V(C2);P.AGR(C2);P.AGR(C);P.AGR(C2);');
        Expect('shown', 'C2/');
        Expect('stored1', 'A/,C2/,C/,');
        Expect('setvalue', 'P.AGR(C2);P.AGR(C);P.AGR(C2);');
        Expect('shown2', 'C2/V');
        Expect('first', 'C2:');
        Expect('walk', 'C2,');
        Expect('close', '');
        Expect('stored', 'A/,C2/V,C/,');
    end;

    // The SetValue on Name with NoChange and UpdateFalse.
    [Test]
    procedure Collapsed_NoChangeUpdateFalse()
    begin
        Initialize();
        RunCollapsed(0, 1, false, false);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setname', 'P.AGR(B);P.V(C2);P.AGR(C2);P.AGR(C);P.AGR(C2);P.AGR(A);');
        Expect('shown', 'C2/');
        Expect('stored1', 'A/,C2/,C/,');
        Expect('setvalue', 'P.AGR(C2);P.AGR(C2);P.AGR(C);P.AGR(C2);P.AGR(A);');
        Expect('shown2', 'C2/V');
        Expect('first', 'A:');
        Expect('walk', 'A,C2,');
        Expect('close', 'P.AGR(C2);');
        Expect('stored', 'A/,C2/V,C/,');
    end;

    // The SetValue on Name with SetRangeKeeps and UpdateFalse, B expanded first.
    [Test]
    procedure Collapsed_SetRangeKeepsUpdateFalse_Expanded()
    begin
        Initialize();
        RunCollapsed(1, 1, true, false);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('expand', '');
        Expect('setname', 'P.AGR(B);P.V(C2);P.AGR(C2);P.AGR(C);P.AGR(C2);');
        Expect('shown', 'C2/');
        Expect('stored1', 'A/,C2/,C/,');
        Expect('setvalue', 'P.AGR(C2);P.AGR(C);P.AGR(C2);');
        Expect('shown2', 'C2/V');
        Expect('first', 'C2:');
        Expect('walk', 'C2,C,');
        Expect('close', 'P.AGR(C);');
        Expect('stored', 'A/,C2/V,C/,');
    end;

    // Last, then only the SetValue on Value (Update(false), no change of view), First, the walk and Close.
    local procedure RunTreeValueOnly()
    var
        Lines: TestPage "LT TVF Tree";
        Walk: Text;
    begin
        Log.Setup(1, 1);
        Lines.OpenEdit();
        Lines.Last();
        Note('open', Log.Take());
        Lines.Value.SetValue('V');
        Note('setvalue', Log.Take());
        Note('shown2', Lines.Name.Value() + '/' + Lines.Value.Value());
        Note('stored1', StoredLines());
        Lines.First();
        Note('first', Lines.Name.Value() + ':' + Log.Take());
        repeat
            Walk += Lines.Name.Value() + ',';
        until not Lines.Next();
        Note('walk', Walk);
        Log.Take();
        Lines.Close();
        Note('close', Log.Take());
        Note('stored', StoredLines());
    end;

    // The SetValue on Value alone, Update(false) with no change of view.
    [Test]
    procedure Tree_ValueOnlyUpdateFalse()
    begin
        Initialize();
        RunTreeValueOnly();
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setvalue', 'P.AGR(B);P.AGR(B);P.AGR(C);P.AGR(B);P.AGR(A);');
        Expect('shown2', 'B/V');
        Expect('stored1', 'A/,B/V,C/,');
        Expect('first', 'A:');
        Expect('walk', 'A,B,');
        Expect('close', 'P.AGR(B);');
        Expect('stored', 'A/,B/V,C/,');
    end;

    // The SetValue on Name with SetRangeDrops and UpdateFalse: the row leaves the view. Last in the codeunit: the page that the
    // failed SetValue leaves behind would read the rows later tests save.
    [Test]
    procedure Tree_SetRangeDropsUpdateFalse_Fails()
    var
        Lines: TestPage "LT TVF Tree";
    begin
        Initialize();
        Log.Setup(4, 1);
        Lines.OpenEdit();
        Lines.Last();
        Note('open', Log.Take());
        asserterror Lines.Name.SetValue('C2');
        Note('error', GetLastErrorCode() + ': ' + GetLastErrorText());
        Note('setname', Log.Take());
        Note('stored', StoredLines());
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('error', 'TestValidation: Validation error for Field: ,  Message = ''The view is filtered, and the entry is outside the filter. Some actions may not work.''');
        Expect('setname', 'P.AGR(B);P.V(C2);P.AGR(A);');
        Expect('stored', 'A/,B/,C/,');
    end;

    // The cells where the filter leaves the row out, from the first, the middle and the last row, on the tree 97720 and on the List
    // 97727 (the same page without ShowAsTree). Their three rows: A (group G0), B (G1) and C (G2), none indented.
    local procedure InitializeFlat()
    var
        Row: Record "LT TVF Row";
        Line: Record "LT TVF Line";
    begin
        Row.DeleteAll();
        Line.DeleteAll();
        AddLine('G0', 1, 'A', 0);
        AddLine('G1', 2, 'B', 0);
        AddLine('G2', 3, 'C', 0);
        Commit();
        Log.Setup(0, 0);
        Clear(Seen);
    end;

    // The List 97727 over those rows: from the first row (At 0, right after the open), the middle row (At 1, Last then Previous)
    // or the last row (At 2, Last), the SetValue on Name with Log.Setup(FilterKind, 1), the SetValue on Value, First, the walk and
    // Close.
    local procedure RunListDrop(At: Integer; FilterKind: Integer)
    var
        Lines: TestPage "LT TVF Lines";
        Walk: Text;
    begin
        Log.Setup(FilterKind, 1);
        Lines.OpenEdit();
        if At > 0 then
            Lines.Last();
        if At = 1 then
            Lines.Previous();
        Note('open', Lines.Name.Value() + ':' + Log.Take());
        Lines.Name.SetValue('X2');
        Note('setname', Log.Take());
        Note('shown', Lines.Name.Value() + '/' + Lines.Value.Value());
        Note('stored1', StoredLines());
        Lines.Value.SetValue('V');
        Note('setvalue', Log.Take());
        Note('shown2', Lines.Name.Value() + '/' + Lines.Value.Value());
        Lines.First();
        Note('first', Lines.Name.Value() + ':' + Log.Take());
        repeat
            Walk += Lines.Name.Value() + ',';
        until not Lines.Next();
        Note('walk', Walk);
        Log.Take();
        Lines.Close();
        Note('close', Log.Take());
        Note('stored', StoredLines());
    end;

    [TryFunction]
    local procedure TrySetTreeName(var Lines: TestPage "LT TVF Tree"; NewName: Text)
    begin
        Lines.Name.SetValue(NewName);
    end;

    // The tree 97720 over those rows, from the first, the middle or the last row as in RunListDrop: the SetValue on Name, its
    // error and the rows the table holds.
    local procedure RunTreeDrop(At: Integer; FilterKind: Integer)
    var
        Lines: TestPage "LT TVF Tree";
    begin
        Log.Setup(FilterKind, 1);
        Lines.OpenEdit();
        if At > 0 then
            Lines.Last();
        if At = 1 then
            Lines.Previous();
        Note('open', Lines.Name.Value() + ':' + Log.Take());
        ClearLastError();
        if TrySetTreeName(Lines, 'X2') then
            Note('error', 'none')
        else
            Note('error', GetLastErrorCode() + ': ' + GetLastErrorText());
        Note('setname', Log.Take());
        Note('stored', StoredLines());
    end;

    // The List, the first row A: SetRange on G1 leaves it out.
    [Test]
    procedure List_FirstRowSetRangeDropsUpdateFalse()
    begin
        InitializeFlat();
        RunListDrop(0, 1);
        Expect('open', 'A:P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setname', 'P.V(X2);P.AGR(B);');
        Expect('shown', 'B/');
        Expect('stored1', 'A/,B/,C/,');
        Expect('setvalue', 'P.AGR(B);');
        Expect('shown2', 'B/');
        Expect('first', 'B:');
        Expect('walk', 'B,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    // The List, the middle row B: SetRange on G0 leaves it out.
    [Test]
    procedure List_MiddleRowSetRangeDropsUpdateFalse()
    begin
        InitializeFlat();
        RunListDrop(1, 4);
        Expect('open', 'B:P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setname', 'P.AGR(B);P.V(X2);P.AGR(A);');
        Expect('shown', 'A/');
        Expect('stored1', 'A/,B/,C/,');
        Expect('setvalue', 'P.AGR(A);');
        Expect('shown2', 'A/');
        Expect('first', 'A:');
        Expect('walk', 'A,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    // The List, the last row C: SetRange on G1 leaves it out.
    [Test]
    procedure List_LastRowSetRangeDropsUpdateFalse()
    begin
        InitializeFlat();
        RunListDrop(2, 1);
        Expect('open', 'C:P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setname', 'P.AGR(C);P.V(X2);P.AGR(B);');
        Expect('shown', 'B/');
        Expect('stored1', 'A/,B/,C/,');
        Expect('setvalue', 'P.AGR(B);');
        Expect('shown2', 'B/');
        Expect('first', 'B:');
        Expect('walk', 'B,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    // The tree, the first row A: SetRange on G1 leaves it out.
    [Test]
    procedure FlatTree_FirstRowSetRangeDropsUpdateFalse_Fails()
    begin
        InitializeFlat();
        RunTreeDrop(0, 1);
        Expect('open', 'A:P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('error', 'TestValidation: Validation error for Field: ,  Message = ''The view is filtered, and the entry is outside the filter. Some actions may not work.''');
        Expect('setname', 'P.V(X2);P.AGR(B);');
        Expect('stored', 'X2/,B/,C/,');
    end;

    // The tree, the middle row B: SetRange on G0 leaves it out.
    [Test]
    procedure FlatTree_MiddleRowSetRangeDropsUpdateFalse_Fails()
    begin
        InitializeFlat();
        RunTreeDrop(1, 4);
        Expect('open', 'B:P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('error', 'TestValidation: Validation error for Field: ,  Message = ''The view is filtered, and the entry is outside the filter. Some actions may not work.''');
        Expect('setname', 'P.AGR(B);P.V(X2);P.AGR(A);');
        Expect('stored', 'A/,X2/,C/,');
    end;

    // The tree, the last row C: SetRange on G1 leaves it out.
    [Test]
    procedure FlatTree_LastRowSetRangeDropsUpdateFalse_Fails()
    begin
        InitializeFlat();
        RunTreeDrop(2, 1);
        Expect('open', 'C:P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('error', 'TestValidation: Validation error for Field: ,  Message = ''The view is filtered, and the entry is outside the filter. Some actions may not work.''');
        Expect('setname', 'P.AGR(C);P.V(X2);P.AGR(B);');
        Expect('stored', 'A/,B/,X2/,');
    end;
    [TryFunction]
    local procedure TrySetIssueName(var Rows: TestPage "LT TVF Issue Tree"; NewName: Text)
    begin
        Rows.Name.SetValue(NewName);
    end;

    [TryFunction]
    local procedure TrySetDropName(var Rows: TestPage "LT TVF Drop Tree"; NewName: Text)
    begin
        Rows.Name.SetValue(NewName);
    end;

    // The issue's tree page 97718 on its first row A (group G0): the control's SetRange on G1 leaves A out of the view.
    [Test]
    procedure IssueTree_FirstRowSetRangeDropsUpdateFalse_Fails()
    var
        Rows: TestPage "LT TVF Issue Tree";
    begin
        Initialize();
        Rows.OpenEdit();
        Note('open', Rows.Name.Value() + ':' + Log.Take());
        ClearLastError();
        if TrySetIssueName(Rows, 'A2') then
            Note('error', 'none')
        else
            Note('error', GetLastErrorCode() + ': ' + GetLastErrorText());
        Note('setname', Log.Take());
        Note('stored', StoredRows());
        Expect('open', 'A:AGR(A);AGR(B);AGR(C);AGR(A);');
        Expect('error', 'TestValidation: Validation error for Field: ,  Message = ''The view is filtered, and the entry is outside the filter. Some actions may not work.''');
        Expect('setname', 'AGR(B);AGR(C);AGR(B);');
        Expect('stored', 'A2,B,C,');
    end;

    // Page 97714, the page of al-runner issue 8846's repro line for line (the issue's tree without OnAfterGetRecord), on its first
    // row A: the SetValue in a TryFunction.
    [Test]
    procedure DropTree_FirstRowSetRangeDropsUpdateFalse_Fails()
    var
        Rows: TestPage "LT TVF Drop Tree";
    begin
        Initialize();
        Rows.OpenEdit();
        Note('open', Rows.Name.Value());
        ClearLastError();
        if TrySetDropName(Rows, 'A2') then
            Note('error', 'none')
        else
            Note('error', GetLastErrorCode() + ': ' + GetLastErrorText());
        Note('stored', StoredRows());
        Expect('open', 'A');
        Expect('error', 'TestValidation: Validation error for Field: ,  Message = ''The view is filtered, and the entry is outside the filter. Some actions may not work.''');
        Expect('stored', 'A2,B,C,');
    end;

    // The same SetValue under asserterror.
    [Test]
    procedure DropTree_FirstRowSetRangeDropsUpdateFalse_AssertError()
    var
        Rows: TestPage "LT TVF Drop Tree";
    begin
        Initialize();
        Rows.OpenEdit();
        Note('open', Rows.Name.Value());
        asserterror Rows.Name.SetValue('A2');
        Note('error', GetLastErrorCode() + ': ' + GetLastErrorText());
        Note('stored', StoredRows());
        Expect('open', 'A');
        Expect('error', 'TestValidation: Validation error for Field: ,  Message = ''The view is filtered, and the entry is outside the filter. Some actions may not work.''');
        Expect('stored', 'A,B,C,');
    end;

    // The tree 97720 over rows A, B and C (entries 1, 2 and 3) in the groups and with the Indentation given: the rows the drop
    // cells hold in other data.
    local procedure InitializeShape(GroupA: Code[10]; GroupB: Code[10]; GroupC: Code[10]; IndentA: Integer; IndentB: Integer; IndentC: Integer)
    var
        Row: Record "LT TVF Row";
        Line: Record "LT TVF Line";
    begin
        Row.DeleteAll();
        Line.DeleteAll();
        AddLine(GroupA, 1, 'A', IndentA);
        AddLine(GroupB, 2, 'B', IndentB);
        AddLine(GroupC, 3, 'C', IndentC);
        Commit();
        Log.Setup(0, 0);
        Clear(Seen);
    end;

    // A in G5, B in G1, C in G0, none indented (in the key C, B, A): from the middle row B, SetRange on G0 keeps only C.
    [Test]
    procedure FlatTree_GroupsG5G1G0_MiddleRowSetRangeDropsUpdateFalse_Fails()
    begin
        InitializeShape('G5', 'G1', 'G0', 0, 0, 0);
        RunTreeDrop(1, 4);
        Expect('open', 'B:P.AGR(C);P.AGR(B);P.AGR(A);P.AGR(C);');
        Expect('error', 'TestValidation: Validation error for Field: ,  Message = ''The view is filtered, and the entry is outside the filter. Some actions may not work.''');
        Expect('setname', 'P.AGR(B);P.V(X2);P.AGR(C);');
        Expect('stored', 'C/,X2/,A/,');
    end;

    // A in G0, B in G2, C in G1, none indented (in the key A, C, B): from the first row A, SetRange on G1 keeps only C.
    [Test]
    procedure FlatTree_GroupsG0G2G1_FirstRowSetRangeDropsUpdateFalse_Fails()
    begin
        InitializeShape('G0', 'G2', 'G1', 0, 0, 0);
        RunTreeDrop(0, 1);
        Expect('open', 'A:P.AGR(A);P.AGR(C);P.AGR(B);P.AGR(A);');
        Expect('error', 'TestValidation: Validation error for Field: ,  Message = ''The view is filtered, and the entry is outside the filter. Some actions may not work.''');
        Expect('setname', 'P.V(X2);P.AGR(C);');
        Expect('stored', 'X2/,C/,B/,');
    end;

    // A in G1, B in G0, C in G2, none indented (in the key B, A, C): from the last row C, SetRange on G1 keeps only A.
    [Test]
    procedure FlatTree_GroupsG1G0G2_LastRowSetRangeDropsUpdateFalse_Fails()
    begin
        InitializeShape('G1', 'G0', 'G2', 0, 0, 0);
        RunTreeDrop(2, 1);
        Expect('open', 'C:P.AGR(B);P.AGR(A);P.AGR(C);P.AGR(B);');
        Expect('error', 'TestValidation: Validation error for Field: ,  Message = ''The view is filtered, and the entry is outside the filter. Some actions may not work.''');
        Expect('setname', 'P.AGR(C);P.V(X2);P.AGR(A);');
        Expect('stored', 'B/,A/,X2/,');
    end;

    // A and B in G1, C in G0 indented 1 (in the key C, A, B): from Last, SetRange on G0 keeps only C.
    [Test]
    procedure Tree_ChildInG0_LastRowSetRangeDropsUpdateFalse_Fails()
    begin
        InitializeShape('G1', 'G1', 'G0', 0, 0, 1);
        RunTreeDrop(2, 4);
        Expect('open', 'B:P.AGR(C);P.AGR(A);P.AGR(B);P.AGR(C);');
        Expect('error', 'TestValidation: Validation error for Field: ,  Message = ''The view is filtered, and the entry is outside the filter. Some actions may not work.''');
        Expect('setname', 'P.AGR(B);P.V(X2);P.AGR(C);');
        Expect('stored', 'C/,A/,X2/,');
    end;

    // The tree, the last row C: SetRange on G0 keeps only A, the first row, with B between.
    [Test]
    procedure FlatTree_LastRowSetRangeG0DropsUpdateFalse_Fails()
    begin
        InitializeFlat();
        RunTreeDrop(2, 4);
        Expect('open', 'C:P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('error', 'TestValidation: Validation error for Field: ,  Message = ''The view is filtered, and the entry is outside the filter. Some actions may not work.''');
        Expect('setname', 'P.AGR(C);P.V(X2);P.AGR(A);');
        Expect('stored', 'A/,B/,X2/,');
    end;

    // The List, the last row C: SetRange on G0 keeps only A, the first row, with B between.
    [Test]
    procedure List_LastRowSetRangeG0DropsUpdateFalse()
    begin
        InitializeFlat();
        RunListDrop(2, 4);
        Expect('open', 'C:P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setname', 'P.AGR(C);P.V(X2);P.AGR(A);');
        Expect('shown', 'A/');
        Expect('stored1', 'A/,B/,C/,');
        Expect('setvalue', 'P.AGR(A);');
        Expect('shown2', 'A/');
        Expect('first', 'A:');
        Expect('walk', 'A,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;
}

// A List over the tree's table, without ShowAsTree; otherwise page 97720.
page 97727 "LT TVF Lines"
{
    PageType = List;
    SourceTable = "LT TVF Line";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
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
        Log: Codeunit "LT TVF Log";

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

// The page of al-runner issue 8846's repro, line for line: the issue's tree page 97718 without OnAfterGetRecord.
page 97714 "LT TVF Drop Tree"
{
    PageType = List;
    SourceTable = "LT TVF Row";
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
}
