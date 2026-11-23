// Neighbors of the cells of ListValidateFilterRefresh.al (al-runner issue 8698): each test takes the steps of a cell and
// changes one fact. Measured on a local BC 29.1 W1 container (3 runs); every reading is BC's.

// The issue's page with Update(true) in the Name control's OnValidate: not the probe's code.
page 97590 "LT RFU Other Lines"
{
    PageType = List;
    SourceTable = "LT RFU Line";
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
                        CurrPage.Update(true);
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

    trigger OnAfterGetRecord()
    begin
        Log.Add('P.AGR(' + Rec.Name + ')');
    end;

    var
        Log: Codeunit "LT RFU Log";
}

codeunit 97591 "LT RFU Neighbor Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        Log: Codeunit "LT RFU Log";
        Seen: Dictionary of [Text, Text];
        RvrLog: Codeunit "LT RVR Log";
        ReadResult: Text;

    local procedure Initialize(FilterStep: Integer; UpdateStep: Integer; Rows: Integer)
    var
        Line: Record "LT RFU Line";
    begin
        Line.DeleteAll();
        AddRow('G0', 1, 'A');
        AddRow('G1', 2, 'B');
        if Rows >= 3 then
            AddRow('G1', 3, 'C');
        if Rows >= 4 then
            AddRow('G1', 4, 'D');
        Commit();
        Log.Setup(FilterStep, UpdateStep);
        Clear(Seen);
    end;

    local procedure AddRow(GroupCode: Code[10]; EntryNo: Integer; Name: Text[30])
    var
        Line: Record "LT RFU Line";
    begin
        Line.Init();
        Line."Group Code" := GroupCode;
        Line."Entry No." := EntryNo;
        Line.Name := Name;
        Line.Insert();
    end;

    local procedure StoredRows(): Text
    var
        Line: Record "LT RFU Line";
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

    // The cell's steps on page 97584 (Last, SetValue on Name, SetValue on Value); `Then` is the call after them: 0 Next, 1 the
    // action, 2 First and the walk, 3 AssertEquals and Editable() on the Name control, then First and the walk.
    local procedure RunLines(After: Integer)
    var
        Lines: TestPage "LT RFU Lines";
        Walk: Text;
    begin
        Lines.OpenEdit();
        Lines.Last();
        Note('open', Log.Take());
        Lines.Name.SetValue(Lines.Name.Value() + '2');
        Note('setname', Log.Take());
        Note('shown', Lines.Name.Value() + '/' + Lines.Value.Value());
        Lines.Value.SetValue('V');
        Note('setvalue', Log.Take());
        Note('shown2', Lines.Name.Value() + '/' + Lines.Value.Value());
        case After of
            0:
                begin
                    Note('then', Format(Lines.Next()) + ':' + Lines.Name.Value() + ':' + Log.Take());
                end;
            1:
                begin
                    Lines.ChangeViewAction.Invoke();
                    Note('then', Lines.Name.Value() + '/' + Lines.Value.Value() + ':' + Log.Take());
                end;
            3:
                begin
                    Lines.Name.AssertEquals('C');
                    Note('reads', Format(Lines.Name.Editable()) + ':' + Log.Take());
                    Lines.First();
                    Note('then', Lines.Name.Value() + ':' + Log.Take());
                    repeat
                        Walk += Lines.Name.Value() + ',';
                    until not Lines.Next();
                    Note('walk', Walk);
                end;
            2:
                begin
                    Lines.First();
                    Note('then', Lines.Name.Value() + ':' + Log.Take());
                    repeat
                        Walk += Lines.Name.Value() + ',';
                    until not Lines.Next();
                    Note('walk', Walk);
                end;
        end;
        Log.Take();
        Lines.Close();
        Note('stored', StoredRows());
    end;

    // The cell :832 over four rows (D in G1 after C): Last stands on D.
    [Test]
    procedure Lines_FourRows()
    begin
        Initialize(1, 1, 4);
        RunLines(2);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(D);P.AGR(A);');
        Expect('setname', 'P.AGR(D);T.V(D2);P.V(D2);P.AGR(D);P.AGR(C);P.AGR(B);');
        Expect('shown', 'D/');
        Expect('setvalue', 'P.AGR(D);P.AGR(D);P.AGR(C);P.AGR(B);');
        Expect('shown2', 'D/');
        Expect('then', 'B:');
        Expect('walk', 'B,C,D,,');
        Expect('stored', 'A/,B/,C/,D/,');
    end;

    // The cell :832 over two rows (no C): Last stands on B.
    [Test]
    procedure Lines_TwoRows()
    begin
        Initialize(1, 1, 2);
        RunLines(2);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(A);');
        Expect('setname', 'P.AGR(B);T.V(B2);P.V(B2);P.AGR(B);');
        Expect('shown', 'B/');
        Expect('setvalue', 'P.AGR(B);');
        Expect('shown2', 'B/');
        Expect('then', 'B:');
        Expect('walk', 'B,,');
        Expect('stored', 'A/,B/,');
    end;

    // The issue's steps on a page whose Name control calls CurrPage.Update(true) after the filter.
    [Test]
    procedure OtherCode_UpdateTrue()
    var
        Lines: TestPage "LT RFU Other Lines";
        Walk: Text;
    begin
        Initialize(0, 0, 3);
        Lines.OpenEdit();
        Lines.Last();
        Note('open', Log.Take());
        Lines.Name.SetValue('C2');
        Note('setname', Log.Take());
        Note('shown', Lines.Name.Value() + '/' + Lines.Value.Value());
        Lines.Value.SetValue('V');
        Note('setvalue', Log.Take());
        Note('shown2', Lines.Name.Value() + '/' + Lines.Value.Value());
        Lines.First();
        Note('then', Lines.Name.Value() + ':' + Log.Take());
        repeat
            Walk += Lines.Name.Value() + ',';
        until not Lines.Next();
        Note('walk', Walk);
        Log.Take();
        Lines.Close();
        Note('stored', StoredRows());
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setname', 'P.AGR(C);T.V(C2);P.AGR(C2);P.AGR(B);');
        Expect('shown', 'C2/');
        Expect('setvalue', 'P.AGR(C2);P.AGR(C2);P.AGR(B);');
        Expect('shown2', 'C2/');
        Expect('then', 'B:');
        Expect('walk', 'B,C2,,');
        Expect('stored', 'A/,B/,C2/,');
    end;

    // The steps of ListValidateFilterRefresh.al:1258 on the Card over four rows (D in G1 after C).
    [Test]
    procedure Card_FourRows()
    var
        Card: TestPage "LT RFU Card";
    begin
        Initialize(1, 1, 4);
        Card.OpenEdit();
        Card.Last();
        Log.Take();
        Card.Name.SetValue('D2');
        Note('setname', Log.Take());
        Note('shown', Card.Name.Value() + '/' + Card.Value.Value());
        Card.Value.SetValue('V');
        Note('setvalue', Log.Take());
        Note('shown2', Card.Name.Value() + '/' + Card.Value.Value());
        Card.Close();
        Note('stored', StoredRows());
        Expect('setname', 'T.V(D2);P.V(D2);P.AGR(D);');
        Expect('shown', 'D/');
        Expect('setvalue', 'P.AGR(D);');
        Expect('shown2', 'D/');
        Expect('stored', 'A/,B/,C/,D/,');
    end;

    // The steps of ListValidateFilterRefresh.al:832, then Next() instead of First().
    [Test]
    procedure Lines_NextAfterSetValues()
    begin
        Initialize(1, 1, 3);
        RunLines(0);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setname', 'P.AGR(C);T.V(C2);P.V(C2);P.AGR(C);P.AGR(B);');
        Expect('shown', 'C/');
        Expect('setvalue', 'P.AGR(C);P.AGR(C);P.AGR(B);');
        Expect('shown2', 'C/');
        Expect('then', 'Yes::');
        Expect('stored', 'A/,B/,C/,');
    end;

    // The steps of ListValidateFilterRefresh.al:832, then the action (SetRange on G1 and Update(false) again).
    [Test]
    procedure Lines_ActionAfterSetValues()
    begin
        Initialize(1, 1, 3);
        RunLines(1);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setname', 'P.AGR(C);T.V(C2);P.V(C2);P.AGR(C);P.AGR(B);');
        Expect('shown', 'C/');
        Expect('setvalue', 'P.AGR(C);P.AGR(C);P.AGR(B);');
        Expect('shown2', 'C/');
        Expect('then', 'C/:P.AGR(C);P.AGR(C);P.A(C);P.AGR(C);P.AGR(C);P.AGR(B);');
        Expect('stored', 'A/,B/,C/,');
    end;

    // The steps of ListValidateFilterRefresh.al:832 with AssertEquals and Editable() on the Name control between the SetValues
    // and First: the reads read no row, and First and the walk read as the cell's.
    [Test]
    procedure Lines_ReadOnlyCallsAfterSetValues()
    begin
        Initialize(1, 1, 3);
        RunLines(3);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setname', 'P.AGR(C);T.V(C2);P.V(C2);P.AGR(C);P.AGR(B);');
        Expect('shown', 'C/');
        Expect('setvalue', 'P.AGR(C);P.AGR(C);P.AGR(B);');
        Expect('shown2', 'C/');
        Expect('reads', 'Yes:');
        Expect('then', 'B:');
        Expect('walk', 'B,C,,');
        Expect('stored', 'A/,B/,C/,');
    end;

    // The steps of ListValidateFilterRefresh.al:1105 on the List with both row triggers, then Next() instead of First().
    [Test]
    procedure Both_NextAfterSetValues()
    var
        Lines: TestPage "LT RFU Lines Both";
    begin
        Initialize(1, 1, 3);
        Lines.OpenEdit();
        Lines.Last();
        Log.Take();
        Lines.Name.SetValue('C2');
        Note('setname', Log.Take());
        Lines.Value.SetValue('V');
        Note('setvalue', Log.Take());
        Note('then', Format(Lines.Next()) + ':' + Lines.Name.Value() + ':' + Log.Take());
        Lines.Close();
        Note('stored', StoredRows());
        Expect('setname', 'T.V(C2);P.V(C2);P.AGR(C);P.AGR(B);P.AGR(C);P.AGR(C);P.AGCR(C);');
        Expect('setvalue', 'P.AGR(C);P.AGR(B);P.AGR(C);P.AGR(C);P.AGCR(C);');
        Expect('then', 'Yes::P.AGCR();');
        Expect('stored', 'A/,B/,C/,');
    end;

    // The steps of ListValidateFilterRefresh.al:1632 on the Card of issue 8717, with a second SetValue on Name before Close.
    [Test]
    procedure EditCard_SecondSetValue()
    var
        Row: Record "LT RFU Edit Row";
        Card: TestPage "LT RFU Edit Card";
        EditLog: Codeunit "LT RFU Edit Log";
    begin
        Initialize(0, 0, 3);
        Row.DeleteAll();
        Row."No." := 'R1';
        Row.Name := 'Alpha';
        Row.Insert();
        Commit();
        Card.OpenEdit();
        Card.GoToKey('R1');
        EditLog.Take();
        Card.Name.SetValue('Beta');
        Note('setname', EditLog.Take() + Card.Name.Value());
        Card.Name.SetValue('Gamma');
        Note('setname2', EditLog.Take() + Card.Name.Value());
        Card.Close();
        Row.Get('R1');
        Note('stored', EditLog.Take() + Row.Name);
        Expect('setname', 'AGR(Alpha);Alpha');
        Expect('setname2', 'AGR(Alpha);Alpha');
        Expect('stored', 'Alpha');
    end;

    // Page 97498 of ProbeReads.al (ListValidateViewReads.al): the issue's cell with an As* read that does not fit its control
    // between the two SetValues: 1 AsInteger on Name (Text), 2 AsBoolean on Qty (Decimal). BC raises an error the test can
    // catch and the cell goes on as without the call. GetOption on Qty and GetOption(4) on Kind (three options) raise a CLR
    // exception no test can catch (measured on BC 29.1, 3 runs; the Rust test read_only_calls_pass_only_where_measured).
    local procedure RunWideMismatch(Which: Integer)
    var
        Lines: TestPage "LT RVR Lines";
        Walk: Text;
    begin
        InitializeWide();
        Lines.OpenEdit();
        Lines.Last();
        Note('open', RvrLog.Take());
        Lines.Name.SetValue('C2');
        Note('setname', RvrLog.Take());
        ClearLastError();
        ReadResult := '';
        if TryWideRead(Lines, Which) then
            Note('read', ReadResult + ':' + RvrLog.Take())
        else
            Note('read', 'error ' + GetLastErrorText() + ':' + RvrLog.Take());
        Note('shown', Lines.Name.Value() + '/' + Lines.Value.Value());
        Lines.Value.SetValue('V');
        Note('setvalue', RvrLog.Take());
        Lines.First();
        Note('first', Lines.Name.Value() + ':' + RvrLog.Take());
        repeat
            Walk += Lines.Name.Value() + ',';
        until not Lines.Next();
        Note('walk', Walk);
        RvrLog.Take();
        Lines.Close();
        Note('close', RvrLog.Take());
        Note('stored', WideStoredRows());
    end;

    [TryFunction]
    local procedure TryWideRead(var Lines: TestPage "LT RVR Lines"; Which: Integer)
    begin
        case Which of
            1:
                ReadResult := Format(Lines.Name.AsInteger());
            2:
                ReadResult := Format(Lines.Qty.AsBoolean());
        end;
    end;

    local procedure InitializeWide()
    var
        Row: Record "LT RVR Row";
    begin
        Row.DeleteAll();
        AddWideRow('G0', 1, 'A', Row.Kind::Alpha, 0);
        AddWideRow('G1', 2, 'B', Row.Kind::Beta, 0);
        AddWideRow('G1', 3, 'C', Row.Kind::Gamma, 1);
        Commit();
        RvrLog.Take();
        Clear(Seen);
    end;

    local procedure AddWideRow(GroupCode: Code[10]; EntryNo: Integer; Name: Text[30]; Kind: Option; Indentation: Integer)
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

    local procedure WideStoredRows(): Text
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

    [Test]
    procedure Wide_AsIntegerOnText_BetweenSteps()
    begin
        RunWideMismatch(1);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setname', 'P.AGR(C);T.V(C2);P.AGR(C);P.AGR(B);');
        Expect('read', 'error Invalid Conversion. Source: String, Destination: NavInteger.:');
        Expect('shown', 'C/');
        Expect('setvalue', 'P.AGR(C);P.AGR(C);P.AGR(B);');
        Expect('first', 'B:');
        Expect('walk', 'B,C,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    [Test]
    procedure Wide_AsBooleanOnDecimal_BetweenSteps()
    begin
        RunWideMismatch(2);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setname', 'P.AGR(C);T.V(C2);P.AGR(C);P.AGR(B);');
        Expect('read', 'error Invalid Conversion. Source: Decimal, Destination: NavBoolean.:');
        Expect('shown', 'C/');
        Expect('setvalue', 'P.AGR(C);P.AGR(C);P.AGR(B);');
        Expect('first', 'B:');
        Expect('walk', 'B,C,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;
}
