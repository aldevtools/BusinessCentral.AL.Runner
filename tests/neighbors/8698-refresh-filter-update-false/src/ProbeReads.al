// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/testfield/testfield-data-type
// Scope: in-scope
// local object: the pages' controls of every type, the tree and the control's OnValidate that changes the view are what is under test
// Fixtures used: shared Assert (60021); own table 97496, log codeunit 97497, pages 97498 and 97499, tests 97574
// Probe (al-runner issue 8698, read-only calls): the issue's page of ListValidateFilterRefresh.al (page 97583) with more controls, as a
//   List (page 97498) and as a tree (page 97499, ShowAsTree, C indented under B). The page stands on the last of three rows A (G0), B
//   and C (G1); SetValue on Name sets SetRange(G1) and calls CurrPage.Update(false); SetValue on Value calls CurrPage.Update(false);
//   then First(), a walk with Next() and Close. The test's read-only calls at three points: before the SetValue on Name, between the
//   two SetValues, after them before First. Block 1: the As* reads of the Integer, Decimal, Date, Time, DateTime and Boolean controls;
//   block 2: OptionCount and GetOption of the Option control, IsExpanded on the row; block 3: GetField by control id, then its Value.
//   And a SetValue that the control cannot take, then ValidationErrorCount and GetValidationError. Readings: the triggers each step
//   runs, the answers of the calls, the shown row, First and the walk, the stored rows.
// BC versions: 27, 28, 29

table 97496 "LT RVR Row"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; "Group Code"; Code[10]) { }
        field(2; "Entry No."; Integer) { }
        field(3; Name; Text[30])
        {
            trigger OnValidate()
            var
                Log: Codeunit "LT RVR Log";
            begin
                Log.Add('T.V(' + Name + ')');
            end;
        }
        field(4; Value; Text[30]) { }
        field(5; Kind; Option)
        {
            OptionMembers = Alpha,Beta,Gamma;
        }
        field(6; Qty; Decimal) { }
        field(7; Day; Date) { }
        field(8; At; Time) { }
        field(9; Stamp; DateTime) { }
        field(10; Flag; Boolean) { }
        field(11; Indentation; Integer) { }
    }

    keys
    {
        key(PK; "Group Code", "Entry No.") { Clustered = true; }
    }
}

codeunit 97497 "LT RVR Log"
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

page 97498 "LT RVR Lines"
{
    PageType = List;
    SourceTable = "LT RVR Row";
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
                        Rec.SetRange("Group Code", 'G1');
                        CurrPage.Update(false);
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
                field("Entry No."; Rec."Entry No.")
                {
                    ApplicationArea = All;
                }
                field(Kind; Rec.Kind)
                {
                    ApplicationArea = All;
                }
                field(Qty; Rec.Qty)
                {
                    ApplicationArea = All;
                }
                field(Day; Rec.Day)
                {
                    ApplicationArea = All;
                }
                field(At; Rec.At)
                {
                    ApplicationArea = All;
                }
                field(Stamp; Rec.Stamp)
                {
                    ApplicationArea = All;
                }
                field(Flag; Rec.Flag)
                {
                    ApplicationArea = All;
                }
            }
        }
    }

    trigger OnAfterGetRecord()
    begin
        Log.Add('P.AGR(' + Rec.Name + ')');
    end;

    var
        Log: Codeunit "LT RVR Log";
}

page 97499 "LT RVR Tree"
{
    PageType = List;
    SourceTable = "LT RVR Row";
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
                        Rec.SetRange("Group Code", 'G1');
                        CurrPage.Update(false);
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
                field("Entry No."; Rec."Entry No.")
                {
                    ApplicationArea = All;
                }
                field(Kind; Rec.Kind)
                {
                    ApplicationArea = All;
                }
                field(Qty; Rec.Qty)
                {
                    ApplicationArea = All;
                }
                field(Day; Rec.Day)
                {
                    ApplicationArea = All;
                }
                field(At; Rec.At)
                {
                    ApplicationArea = All;
                }
                field(Stamp; Rec.Stamp)
                {
                    ApplicationArea = All;
                }
                field(Flag; Rec.Flag)
                {
                    ApplicationArea = All;
                }
            }
        }
    }

    trigger OnAfterGetRecord()
    begin
        Log.Add('P.AGR(' + Rec.Name + ')');
    end;

    var
        Log: Codeunit "LT RVR Log";
}

codeunit 97574 "LT RVR Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        Log: Codeunit "LT RVR Log";
        Seen: Dictionary of [Text, Text];

    local procedure Initialize()
    var
        Row: Record "LT RVR Row";
    begin
        Row.DeleteAll();
        AddRow('G0', 1, 'A', Row.Kind::Alpha, 0);
        AddRow('G1', 2, 'B', Row.Kind::Beta, 0);
        AddRow('G1', 3, 'C', Row.Kind::Gamma, 1);
        Commit();
        Log.Take();
        Clear(Seen);
    end;

    local procedure AddRow(GroupCode: Code[10]; EntryNo: Integer; Name: Text[30]; Kind: Option; Indentation: Integer)
    var
        Row: Record "LT RVR Row";
    begin
        Row.Init();
        Row."Group Code" := GroupCode;
        Row."Entry No." := EntryNo;
        Row.Name := Name;
        Row.Kind := Kind;
        Row.Qty := EntryNo + 0.25;
        Row.Day := DMY2Date(EntryNo, 3, 2026);
        Row.At := 103000T;
        Row.Stamp := CreateDateTime(Row.Day, Row.At);
        Row.Flag := EntryNo = 3;
        Row.Indentation := Indentation;
        Row.Insert();
    end;

    local procedure StoredRows(): Text
    var
        Row: Record "LT RVR Row";
        Result: Text;
    begin
        if Row.FindSet() then
            repeat
                Result += Row.Name + '/' + Row.Value + ',';
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

    // The control id the compiler gives the Name control of page 97498 (Tree = false) or 97499 (Tree = true), as the app's
    // SymbolReference.json lists it; TestPage.GetField takes a control id.
    local procedure NameControlId(Tree: Boolean): Integer
    begin
        if Tree then
            exit(136840208);
        exit(26152855);
    end;

    // Block 1: the As* reads; block 2: OptionCount, GetOption and IsExpanded; block 3: GetField by the Name control's id, then
    // its Value. The answers, then what the page logged meanwhile.
    local procedure ReadsLines(var Lines: TestPage "LT RVR Lines"; Block: Integer): Text
    var
        Result: Text;
    begin
        case Block of
            1:
                Result := Format(Lines."Entry No.".AsInteger()) + '/' + Format(Lines.Qty.AsDecimal(), 0, 9) + '/' +
                  Format(Lines.Day.AsDate(), 0, 9) + '/' + Format(Lines.At.AsTime(), 0, 9) + '/' +
                  Format(Lines.Stamp.AsDateTime(), 0, 9) + '/' + Format(Lines.Flag.AsBoolean());
            2:
                Result := Format(Lines.Kind.OptionCount()) + '/' + Lines.Kind.GetOption(1) + '/' + Lines.Kind.GetOption(3) + '/' +
                  Format(Lines.IsExpanded());
            3:
                Result := Lines.GetField(NameControlId(false)).Value();
        end;
        exit(Result + ':' + Log.Take());
    end;

    // Block 1: the As* reads; block 2: OptionCount, GetOption and IsExpanded; block 3: GetField by the Name control's id, then
    // its Value. The answers, then what the page logged meanwhile.
    local procedure ReadsTree(var Lines: TestPage "LT RVR Tree"; Block: Integer): Text
    var
        Result: Text;
    begin
        case Block of
            1:
                Result := Format(Lines."Entry No.".AsInteger()) + '/' + Format(Lines.Qty.AsDecimal(), 0, 9) + '/' +
                  Format(Lines.Day.AsDate(), 0, 9) + '/' + Format(Lines.At.AsTime(), 0, 9) + '/' +
                  Format(Lines.Stamp.AsDateTime(), 0, 9) + '/' + Format(Lines.Flag.AsBoolean());
            2:
                Result := Format(Lines.Kind.OptionCount()) + '/' + Lines.Kind.GetOption(1) + '/' + Lines.Kind.GetOption(3) + '/' +
                  Format(Lines.IsExpanded());
            3:
                Result := Lines.GetField(NameControlId(true)).Value();
        end;
        exit(Result + ':' + Log.Take());
    end;

    // The issue's cell (ListValidateFilterRefresh.al:815) on this page, with the read-only calls of Block at ReadAt: 1 before the
    // SetValue on Name, 2 between the SetValues, 3 after them before First; 0 none.
    local procedure RunLines(ReadAt: Integer; Block: Integer)
    var
        Lines: TestPage "LT RVR Lines";
        Walk: Text;
    begin
        Lines.OpenEdit();
        Lines.Last();
        Note('open', Log.Take());
        if ReadAt = 1 then
            Note('reads', ReadsLines(Lines, Block));
        Lines.Name.SetValue('C2');
        Note('setname', Log.Take());
        Note('shown', Lines.Name.Value() + '/' + Lines.Value.Value());
        if ReadAt = 2 then
            Note('reads', ReadsLines(Lines, Block));
        Lines.Value.SetValue('V');
        Note('setvalue', Log.Take());
        Note('shown2', Lines.Name.Value() + '/' + Lines.Value.Value());
        if ReadAt = 3 then
            Note('reads', ReadsLines(Lines, Block));
        Lines.First();
        Note('first', Lines.Name.Value() + ':' + Log.Take());
        repeat
            Walk += Lines.Name.Value() + ',';
        until not Lines.Next();
        Note('walk', Walk);
        Log.Take();
        Lines.Close();
        Note('close', Log.Take());
        Note('stored', StoredRows());
    end;

    // Last, then a SetValue the Qty control cannot take; the page's and the control's validation errors.
    local procedure RunLinesError()
    var
        Lines: TestPage "LT RVR Lines";
    begin
        Lines.OpenEdit();
        Lines.Last();
        Note('open', Log.Take());
        ClearLastError();
        if TryRunLinesQty(Lines) then
            Note('error', 'none')
        else
            Note('error', GetLastErrorText());
        Note('counts', Format(Lines.ValidationErrorCount()) + '/' + Format(Lines.Qty.ValidationErrorCount()) + ':' + Log.Take());
        if Lines.ValidationErrorCount() > 0 then
            Note('verror', Lines.GetValidationError(1));
        if Lines.Qty.ValidationErrorCount() > 0 then
            Note('cerror', Lines.Qty.GetValidationError(1));
        Note('shown', Lines.Name.Value() + '/' + Lines.Qty.Value());
        Lines.Close();
        Note('close', Log.Take());
        Note('stored', StoredRows());
    end;

    [TryFunction]
    local procedure TryRunLinesQty(var Lines: TestPage "LT RVR Lines")
    begin
        Lines.Qty.SetValue('X');
    end;

    // The issue's cell (ListValidateFilterRefresh.al:815) on this page, with the read-only calls of Block at ReadAt: 1 before the
    // SetValue on Name, 2 between the SetValues, 3 after them before First; 0 none.
    local procedure RunTree(ReadAt: Integer; Block: Integer)
    var
        Lines: TestPage "LT RVR Tree";
        Walk: Text;
    begin
        Lines.OpenEdit();
        Lines.Last();
        Note('open', Log.Take());
        if ReadAt = 1 then
            Note('reads', ReadsTree(Lines, Block));
        Lines.Name.SetValue('C2');
        Note('setname', Log.Take());
        Note('shown', Lines.Name.Value() + '/' + Lines.Value.Value());
        if ReadAt = 2 then
            Note('reads', ReadsTree(Lines, Block));
        Lines.Value.SetValue('V');
        Note('setvalue', Log.Take());
        Note('shown2', Lines.Name.Value() + '/' + Lines.Value.Value());
        if ReadAt = 3 then
            Note('reads', ReadsTree(Lines, Block));
        Lines.First();
        Note('first', Lines.Name.Value() + ':' + Log.Take());
        repeat
            Walk += Lines.Name.Value() + ',';
        until not Lines.Next();
        Note('walk', Walk);
        Log.Take();
        Lines.Close();
        Note('close', Log.Take());
        Note('stored', StoredRows());
    end;

    // Last, then a SetValue the Qty control cannot take; the page's and the control's validation errors.
    local procedure RunTreeError()
    var
        Lines: TestPage "LT RVR Tree";
    begin
        Lines.OpenEdit();
        Lines.Last();
        Note('open', Log.Take());
        ClearLastError();
        if TryRunTreeQty(Lines) then
            Note('error', 'none')
        else
            Note('error', GetLastErrorText());
        Note('counts', Format(Lines.ValidationErrorCount()) + '/' + Format(Lines.Qty.ValidationErrorCount()) + ':' + Log.Take());
        if Lines.ValidationErrorCount() > 0 then
            Note('verror', Lines.GetValidationError(1));
        if Lines.Qty.ValidationErrorCount() > 0 then
            Note('cerror', Lines.Qty.GetValidationError(1));
        Note('shown', Lines.Name.Value() + '/' + Lines.Qty.Value());
        Lines.Close();
        Note('close', Log.Take());
        Note('stored', StoredRows());
    end;

    [TryFunction]
    local procedure TryRunTreeQty(var Lines: TestPage "LT RVR Tree")
    begin
        Lines.Qty.SetValue('X');
    end;

    // The issue's cell on page 97498, no read-only calls.
    [Test]
    procedure Lines_SetRangeUpdateFalse_ReadsStoredRowAndAppliesFilter()
    begin
        Initialize();
        RunLines(0, 0);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setname', 'P.AGR(C);T.V(C2);P.AGR(C);P.AGR(B);');
        Expect('shown', 'C/');
        Expect('setvalue', 'P.AGR(C);P.AGR(C);P.AGR(B);');
        Expect('shown2', 'C/');
        Expect('first', 'B:');
        Expect('walk', 'B,C,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    [Test]
    procedure Lines_SetValueQtyNotANumber_ValidationError()
    begin
        Initialize();
        RunLinesError();
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('error', 'Validation error for Field: Qty,  Message = ''Your entry of ''X'' is not an acceptable value for ''Qty''. ''X'' is not a part of a valid decimal expression.''');
        Expect('counts', '1/1:');
        Expect('verror', 'Your entry of ''X'' is not an acceptable value for ''Qty''. ''X'' is not a part of a valid decimal expression.');
        Expect('cerror', 'Your entry of ''X'' is not an acceptable value for ''Qty''. ''X'' is not a part of a valid decimal expression.');
        Expect('shown', 'C/3.25');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    [Test]
    procedure Lines_AsReads_BeforeFirstStep()
    begin
        Initialize();
        RunLines(1, 1);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('reads', '3/3.25/2026-03-03/10:30:00/2026-03-03T10:30:00Z/Yes:');
        Expect('setname', 'P.AGR(C);T.V(C2);P.AGR(C);P.AGR(B);');
        Expect('shown', 'C/');
        Expect('setvalue', 'P.AGR(C);P.AGR(C);P.AGR(B);');
        Expect('shown2', 'C/');
        Expect('first', 'B:');
        Expect('walk', 'B,C,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    [Test]
    procedure Lines_AsReads_BetweenSteps()
    begin
        Initialize();
        RunLines(2, 1);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('reads', '3/3.25/2026-03-03/10:30:00/2026-03-03T10:30:00Z/Yes:');
        Expect('setname', 'P.AGR(C);T.V(C2);P.AGR(C);P.AGR(B);');
        Expect('shown', 'C/');
        Expect('setvalue', 'P.AGR(C);P.AGR(C);P.AGR(B);');
        Expect('shown2', 'C/');
        Expect('first', 'B:');
        Expect('walk', 'B,C,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    [Test]
    procedure Lines_AsReads_AfterSteps()
    begin
        Initialize();
        RunLines(3, 1);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('reads', '3/3.25/2026-03-03/10:30:00/2026-03-03T10:30:00Z/Yes:');
        Expect('setname', 'P.AGR(C);T.V(C2);P.AGR(C);P.AGR(B);');
        Expect('shown', 'C/');
        Expect('setvalue', 'P.AGR(C);P.AGR(C);P.AGR(B);');
        Expect('shown2', 'C/');
        Expect('first', 'B:');
        Expect('walk', 'B,C,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    [Test]
    procedure Lines_OptionsAndTree_BeforeFirstStep()
    begin
        Initialize();
        RunLines(1, 2);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('reads', '3/Alpha/Gamma/No:');
        Expect('setname', 'P.AGR(C);T.V(C2);P.AGR(C);P.AGR(B);');
        Expect('shown', 'C/');
        Expect('setvalue', 'P.AGR(C);P.AGR(C);P.AGR(B);');
        Expect('shown2', 'C/');
        Expect('first', 'B:');
        Expect('walk', 'B,C,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    [Test]
    procedure Lines_OptionsAndTree_BetweenSteps()
    begin
        Initialize();
        RunLines(2, 2);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('reads', '3/Alpha/Gamma/No:');
        Expect('setname', 'P.AGR(C);T.V(C2);P.AGR(C);P.AGR(B);');
        Expect('shown', 'C/');
        Expect('setvalue', 'P.AGR(C);P.AGR(C);P.AGR(B);');
        Expect('shown2', 'C/');
        Expect('first', 'B:');
        Expect('walk', 'B,C,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    [Test]
    procedure Lines_OptionsAndTree_AfterSteps()
    begin
        Initialize();
        RunLines(3, 2);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('reads', '3/Alpha/Gamma/No:');
        Expect('setname', 'P.AGR(C);T.V(C2);P.AGR(C);P.AGR(B);');
        Expect('shown', 'C/');
        Expect('setvalue', 'P.AGR(C);P.AGR(C);P.AGR(B);');
        Expect('shown2', 'C/');
        Expect('first', 'B:');
        Expect('walk', 'B,C,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    [Test]
    procedure Lines_GetFieldById_BeforeFirstStep()
    begin
        Initialize();
        RunLines(1, 3);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('reads', 'C:');
        Expect('setname', 'P.AGR(C);T.V(C2);P.AGR(C);P.AGR(B);');
        Expect('shown', 'C/');
        Expect('setvalue', 'P.AGR(C);P.AGR(C);P.AGR(B);');
        Expect('shown2', 'C/');
        Expect('first', 'B:');
        Expect('walk', 'B,C,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    // The issue's cell on page 97499, no read-only calls.
    [Test]
    procedure Tree_SetRangeUpdateFalse_ReadsStoredRowAndAppliesFilter()
    begin
        Initialize();
        RunTree(0, 0);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setname', 'P.AGR(B);T.V(C2);P.AGR(C2);P.AGR(C);P.AGR(C2);P.AGR(C2);P.AGR(C2);');
        Expect('shown', 'C2/');
        Expect('setvalue', 'P.AGR(C2);P.AGR(C);P.AGR(C2);P.AGR(C2);');
        Expect('shown2', 'C2/V');
        Expect('first', 'C2:');
        Expect('walk', 'C2,');
        Expect('close', '');
        Expect('stored', 'A/,C2/V,C/,');
    end;

    [Test]
    procedure Tree_SetValueQtyNotANumber_ValidationError()
    begin
        Initialize();
        RunTreeError();
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('error', 'Validation error for Field: Qty,  Message = ''Your entry of ''X'' is not an acceptable value for ''Qty''. ''X'' is not a part of a valid decimal expression.''');
        Expect('counts', '1/1:');
        Expect('verror', 'Your entry of ''X'' is not an acceptable value for ''Qty''. ''X'' is not a part of a valid decimal expression.');
        Expect('cerror', 'Your entry of ''X'' is not an acceptable value for ''Qty''. ''X'' is not a part of a valid decimal expression.');
        Expect('shown', 'B/2.25');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    [Test]
    procedure Tree_AsReads_BeforeFirstStep()
    begin
        Initialize();
        RunTree(1, 1);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('reads', '2/2.25/2026-03-02/10:30:00/2026-03-02T10:30:00Z/No:');
        Expect('setname', 'P.AGR(B);T.V(C2);P.AGR(C2);P.AGR(C);P.AGR(C2);P.AGR(C2);P.AGR(C2);P.AGR(C2);');
        Expect('shown', 'C2/');
        Expect('setvalue', 'P.AGR(C2);P.AGR(C);P.AGR(C2);P.AGR(C2);P.AGR(C2);');
        Expect('shown2', 'C2/V');
        Expect('first', 'C2:');
        Expect('walk', 'C2,');
        Expect('close', '');
        Expect('stored', 'A/,C2/V,C/,');
    end;

    [Test]
    procedure Tree_AsReads_BetweenSteps()
    begin
        Initialize();
        RunTree(2, 1);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('reads', '2/2.25/2026-03-02/10:30:00/2026-03-02T10:30:00Z/No:');
        Expect('setname', 'P.AGR(B);T.V(C2);P.AGR(C2);P.AGR(C);P.AGR(C2);P.AGR(C2);P.AGR(C2);');
        Expect('shown', 'C2/');
        Expect('setvalue', 'P.AGR(C2);P.AGR(C);P.AGR(C2);P.AGR(C2);P.AGR(C2);');
        Expect('shown2', 'C2/V');
        Expect('first', 'C2:');
        Expect('walk', 'C2,');
        Expect('close', '');
        Expect('stored', 'A/,C2/V,C/,');
    end;

    [Test]
    procedure Tree_AsReads_AfterSteps()
    begin
        Initialize();
        RunTree(3, 1);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('reads', '2/2.25/2026-03-02/10:30:00/2026-03-02T10:30:00Z/No:');
        Expect('setname', 'P.AGR(B);T.V(C2);P.AGR(C2);P.AGR(C);P.AGR(C2);P.AGR(C2);P.AGR(C2);');
        Expect('shown', 'C2/');
        Expect('setvalue', 'P.AGR(C2);P.AGR(C);P.AGR(C2);P.AGR(C2);P.AGR(C2);');
        Expect('shown2', 'C2/V');
        Expect('first', 'C2:');
        Expect('walk', 'C2,');
        Expect('close', '');
        Expect('stored', 'A/,C2/V,C/,');
    end;

    [Test]
    procedure Tree_OptionsAndTree_BeforeFirstStep()
    begin
        Initialize();
        RunTree(1, 2);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('reads', '3/Alpha/Gamma/No:');
        Expect('setname', 'P.AGR(B);T.V(C2);P.AGR(C2);P.AGR(C);P.AGR(C2);P.AGR(C2);P.AGR(C2);');
        Expect('shown', 'C2/');
        Expect('setvalue', 'P.AGR(C2);P.AGR(C);P.AGR(C2);P.AGR(C2);P.AGR(C2);');
        Expect('shown2', 'C2/V');
        Expect('first', 'C2:');
        Expect('walk', 'C2,');
        Expect('close', '');
        Expect('stored', 'A/,C2/V,C/,');
    end;

    [Test]
    procedure Tree_OptionsAndTree_BetweenSteps()
    begin
        Initialize();
        RunTree(2, 2);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('reads', '3/Alpha/Gamma/No:');
        Expect('setname', 'P.AGR(B);T.V(C2);P.AGR(C2);P.AGR(C);P.AGR(C2);P.AGR(C2);P.AGR(C2);');
        Expect('shown', 'C2/');
        Expect('setvalue', 'P.AGR(C2);P.AGR(C);P.AGR(C2);P.AGR(C2);P.AGR(C2);');
        Expect('shown2', 'C2/V');
        Expect('first', 'C2:');
        Expect('walk', 'C2,');
        Expect('close', '');
        Expect('stored', 'A/,C2/V,C/,');
    end;

    [Test]
    procedure Tree_OptionsAndTree_AfterSteps()
    begin
        Initialize();
        RunTree(3, 2);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('reads', '3/Alpha/Gamma/No:');
        Expect('setname', 'P.AGR(B);T.V(C2);P.AGR(C2);P.AGR(C);P.AGR(C2);P.AGR(C2);P.AGR(C2);');
        Expect('shown', 'C2/');
        Expect('setvalue', 'P.AGR(C2);P.AGR(C);P.AGR(C2);P.AGR(C2);P.AGR(C2);');
        Expect('shown2', 'C2/V');
        Expect('first', 'C2:');
        Expect('walk', 'C2,');
        Expect('close', '');
        Expect('stored', 'A/,C2/V,C/,');
    end;

    [Test]
    procedure Tree_GetFieldById_BeforeFirstStep()
    begin
        Initialize();
        RunTree(1, 3);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('reads', 'B:');
        Expect('setname', 'P.AGR(B);T.V(C2);P.AGR(C2);P.AGR(C);P.AGR(C2);P.AGR(C2);P.AGR(C2);');
        Expect('shown', 'C2/');
        Expect('setvalue', 'P.AGR(C2);P.AGR(C);P.AGR(C2);P.AGR(C2);P.AGR(C2);');
        Expect('shown2', 'C2/V');
        Expect('first', 'C2:');
        Expect('walk', 'C2,');
        Expect('close', '');
        Expect('stored', 'A/,C2/V,C/,');
    end;

    [Test]
    procedure Tree_GetFieldById_BetweenSteps()
    begin
        Initialize();
        RunTree(2, 3);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('reads', 'C2:');
        Expect('setname', 'P.AGR(B);T.V(C2);P.AGR(C2);P.AGR(C);P.AGR(C2);P.AGR(C2);P.AGR(C2);');
        Expect('shown', 'C2/');
        Expect('setvalue', 'P.AGR(C2);P.AGR(C);P.AGR(C2);P.AGR(C2);P.AGR(C2);');
        Expect('shown2', 'C2/V');
        Expect('first', 'C2:');
        Expect('walk', 'C2,');
        Expect('close', '');
        Expect('stored', 'A/,C2/V,C/,');
    end;

    [Test]
    procedure Tree_GetFieldById_AfterSteps()
    begin
        Initialize();
        RunTree(3, 3);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('reads', 'C2:');
        Expect('setname', 'P.AGR(B);T.V(C2);P.AGR(C2);P.AGR(C);P.AGR(C2);P.AGR(C2);P.AGR(C2);');
        Expect('shown', 'C2/');
        Expect('setvalue', 'P.AGR(C2);P.AGR(C);P.AGR(C2);P.AGR(C2);P.AGR(C2);');
        Expect('shown2', 'C2/V');
        Expect('first', 'C2:');
        Expect('walk', 'C2,');
        Expect('close', '');
        Expect('stored', 'A/,C2/V,C/,');
    end;

    // Last in this copy of the probe (the corpus file has them after Lines_GetFieldById_BeforeFirstStep), where a refusal of
    // theirs cannot make the runner distrust a later failure of the codeunit (tests/neighbors/README.md).
    [Test]
    procedure Lines_GetFieldById_BetweenSteps()
    begin
        Initialize();
        RunLines(2, 3);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('reads', 'C:');
        Expect('setname', 'P.AGR(C);T.V(C2);P.AGR(C);P.AGR(B);');
        Expect('shown', 'C/');
        Expect('setvalue', 'P.AGR(C);P.AGR(C);P.AGR(B);');
        Expect('shown2', 'C/');
        Expect('first', 'B:');
        Expect('walk', 'B,C,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    [Test]
    procedure Lines_GetFieldById_AfterSteps()
    begin
        Initialize();
        RunLines(3, 3);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('reads', 'C:');
        Expect('setname', 'P.AGR(C);T.V(C2);P.AGR(C);P.AGR(B);');
        Expect('shown', 'C/');
        Expect('setvalue', 'P.AGR(C);P.AGR(C);P.AGR(B);');
        Expect('shown2', 'C/');
        Expect('first', 'B:');
        Expect('walk', 'B,C,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;
}
