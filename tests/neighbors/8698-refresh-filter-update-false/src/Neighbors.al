// Neighbors of the cells of ListValidateFilterRefresh.al (al-runner issue 8698): each test takes the steps of a cell and
// changes one fact. Measured on a local BC 29.1 W1 container (3 runs); every reading is BC's.

// The issue's page with Update(true) in the Name control's OnValidate: not the probe's code.
page 97571 "LT RFU Other Lines"
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

codeunit 97570 "LT RFU Neighbor Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        Log: Codeunit "LT RFU Log";
        Seen: Dictionary of [Text, Text];

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

    // The cell's steps on page 97584 (Last, SetValue on Name, SetValue on Value); `Then` is the call after them: 0 Previous,
    // 1 a third SetValue on Value, 2 First and the walk.
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
}
