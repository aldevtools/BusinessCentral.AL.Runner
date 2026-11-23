// Page 97720 of TreeValidateFilterRefresh.al (corpus pull request 1539) line for line, with its table and its log codeunit,
// under other names: the data shapes of the review of the 8846 pull request, where the filter of the Name control's OnValidate
// leaves the row out and keeps one row whose place the drop cells did not measure at first (cells of 1539 since :1424-:1460), and
// Close after the failed SetValue. Each test asserts what Business Central 29.1 answered (3 runs).

table 97675 "LT TDN Line"
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

codeunit 97676 "LT TDN Shape Log"
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

page 97677 "LT TDN Tree"
{
    PageType = List;
    SourceTable = "LT TDN Line";
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
        Log: Codeunit "LT TDN Shape Log";

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

codeunit 97678 "LT TDN Shape Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        Log: Codeunit "LT TDN Shape Log";
        Seen: Dictionary of [Text, Text];

    // Rows A, B and C (entries 1, 2 and 3) in the groups and with the Indentation given.
    local procedure Initialize(GroupA: Code[10]; GroupB: Code[10]; GroupC: Code[10]; IndentA: Integer; IndentB: Integer; IndentC: Integer)
    var
        Line: Record "LT TDN Line";
    begin
        Line.DeleteAll();
        AddLine(GroupA, 1, 'A', IndentA);
        AddLine(GroupB, 2, 'B', IndentB);
        AddLine(GroupC, 3, 'C', IndentC);
        Commit();
        Log.Setup(0, 0);
        Clear(Seen);
    end;

    local procedure AddLine(GroupCode: Code[10]; EntryNo: Integer; Name: Text[30]; Indentation: Integer)
    var
        Line: Record "LT TDN Line";
    begin
        Line."Group Code" := GroupCode;
        Line."Entry No." := EntryNo;
        Line.Name := Name;
        Line.Indentation := Indentation;
        Line.Insert();
    end;

    local procedure StoredLines(): Text
    var
        Line: Record "LT TDN Line";
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
    local procedure TrySetName(var Lines: TestPage "LT TDN Tree"; NewName: Text)
    begin
        Lines.Name.SetValue(NewName);
    end;

    // From the first row (At 0, right after the open), the middle row (At 1, Last then Previous) or the last row (At 2, Last),
    // the SetValue on Name with Log.Setup(FilterKind, 1): its error and the rows the table holds.
    local procedure RunTreeDrop(At: Integer; FilterKind: Integer)
    var
        Lines: TestPage "LT TDN Tree";
    begin
        Log.Setup(FilterKind, 1);
        Lines.OpenEdit();
        if At > 0 then
            Lines.Last();
        if At = 1 then
            Lines.Previous();
        Note('open', Lines.Name.Value() + ':' + Log.Take());
        ClearLastError();
        if TrySetName(Lines, 'X2') then
            Note('error', 'none')
        else
            Note('error', GetLastErrorCode() + ': ' + GetLastErrorText());
        Note('setname', Log.Take());
        Note('stored', StoredLines());
    end;

    // A in G5, B in G1, C in G0, none indented (in the key C, B, A): from the middle row B, SetRange on G0 keeps only C.
    [Test]
    procedure GroupsG5G1G0_MiddleRowDrops()
    begin
        Initialize('G5', 'G1', 'G0', 0, 0, 0);
        RunTreeDrop(1, 4);
        Expect('open', 'B:P.AGR(C);P.AGR(B);P.AGR(A);P.AGR(C);');
        Expect('error', 'TestValidation: Validation error for Field: ,  Message = ''The view is filtered, and the entry is outside the filter. Some actions may not work.''');
        Expect('setname', 'P.AGR(B);P.V(X2);P.AGR(C);');
        Expect('stored', 'C/,X2/,A/,');
    end;

    // A in G0, B in G2, C in G1, none indented (in the key A, C, B): from the first row A, SetRange on G1 keeps only C.
    [Test]
    procedure GroupsG0G2G1_FirstRowDrops()
    begin
        Initialize('G0', 'G2', 'G1', 0, 0, 0);
        RunTreeDrop(0, 1);
        Expect('open', 'A:P.AGR(A);P.AGR(C);P.AGR(B);P.AGR(A);');
        Expect('error', 'TestValidation: Validation error for Field: ,  Message = ''The view is filtered, and the entry is outside the filter. Some actions may not work.''');
        Expect('setname', 'P.V(X2);P.AGR(C);');
        Expect('stored', 'X2/,C/,B/,');
    end;

    // A in G1, B in G0, C in G2, none indented (in the key B, A, C): from the last row C, SetRange on G1 keeps only A.
    [Test]
    procedure GroupsG1G0G2_LastRowDrops()
    begin
        Initialize('G1', 'G0', 'G2', 0, 0, 0);
        RunTreeDrop(2, 1);
        Expect('open', 'C:P.AGR(B);P.AGR(A);P.AGR(C);P.AGR(B);');
        Expect('error', 'TestValidation: Validation error for Field: ,  Message = ''The view is filtered, and the entry is outside the filter. Some actions may not work.''');
        Expect('setname', 'P.AGR(C);P.V(X2);P.AGR(A);');
        Expect('stored', 'B/,A/,X2/,');
    end;

    // A and B in G1, C in G0 indented 1 (in the key C, A, B): from Last, SetRange on G0 keeps only C.
    [Test]
    procedure ChildInG0_LastRowDrops()
    begin
        Initialize('G1', 'G1', 'G0', 0, 0, 1);
        RunTreeDrop(2, 4);
        Expect('open', 'B:P.AGR(C);P.AGR(A);P.AGR(B);P.AGR(C);');
        Expect('error', 'TestValidation: Validation error for Field: ,  Message = ''The view is filtered, and the entry is outside the filter. Some actions may not work.''');
        Expect('setname', 'P.AGR(B);P.V(X2);P.AGR(C);');
        Expect('stored', 'C/,A/,X2/,');
    end;
}

// The value the page shows and Close after the SetValue whose filter left the row out: in a codeunit of its own, as the runner
// refuses the read.
codeunit 97679 "LT TDN Close Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        Log: Codeunit "LT TDN Shape Log";
        Seen: Dictionary of [Text, Text];

    // Rows A, B and C (entries 1, 2 and 3) in the groups and with the Indentation given.
    local procedure Initialize(GroupA: Code[10]; GroupB: Code[10]; GroupC: Code[10]; IndentA: Integer; IndentB: Integer; IndentC: Integer)
    var
        Line: Record "LT TDN Line";
    begin
        Line.DeleteAll();
        AddLine(GroupA, 1, 'A', IndentA);
        AddLine(GroupB, 2, 'B', IndentB);
        AddLine(GroupC, 3, 'C', IndentC);
        Commit();
        Log.Setup(0, 0);
        Clear(Seen);
    end;

    local procedure AddLine(GroupCode: Code[10]; EntryNo: Integer; Name: Text[30]; Indentation: Integer)
    var
        Line: Record "LT TDN Line";
    begin
        Line."Group Code" := GroupCode;
        Line."Entry No." := EntryNo;
        Line.Name := Name;
        Line.Indentation := Indentation;
        Line.Insert();
    end;

    local procedure StoredLines(): Text
    var
        Line: Record "LT TDN Line";
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
    local procedure TrySetName(var Lines: TestPage "LT TDN Tree"; NewName: Text)
    begin
        Lines.Name.SetValue(NewName);
    end;

    // The cells' flat rows A (G0), B (G1) and C (G2): from the first row A, SetRange on G1 leaves A out; then the value the page
    // shows, and Close.
    [Test]
    procedure FirstRowDrops_ThenValueAndClose()
    var
        Lines: TestPage "LT TDN Tree";
    begin
        Initialize('G0', 'G1', 'G2', 0, 0, 0);
        Log.Setup(1, 1);
        Lines.OpenEdit();
        Note('open', Lines.Name.Value() + ':' + Log.Take());
        ClearLastError();
        if TrySetName(Lines, 'X2') then
            Note('error', 'none')
        else
            Note('error', GetLastErrorCode() + ': ' + GetLastErrorText());
        Note('setname', Log.Take());
        Note('shown', Lines.Name.Value() + ':' + Log.Take());
        Lines.Close();
        Note('close', Log.Take());
        Note('stored', StoredLines());
        Expect('open', 'A:P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('error', 'TestValidation: Validation error for Field: ,  Message = ''The view is filtered, and the entry is outside the filter. Some actions may not work.''');
        Expect('setname', 'P.V(X2);P.AGR(B);');
        Expect('shown', 'B:');
        Expect('close', '');
        Expect('stored', 'X2/,B/,C/,');
    end;
}
