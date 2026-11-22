// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/page/page-update-method
// Scope: in-scope
// local object: the control's OnValidate and the action that change the page's view, and the page triggers that log, are what is under test
// Fixtures used: shared Assert (60021); own table 97581, log codeunit 97582, pages 97583 to 97588, test codeunit 97589
// Probe (al-runner issue 8698): a page stands on the last of three rows A (group G0), B and C (group G1), opened with OpenEdit. SetValue
//   on Name runs the table field's OnValidate, then the control's OnValidate, which sets a filter on Rec and calls CurrPage.Update(false).
//   A SetValue on Value, whose OnValidate calls CurrPage.Update(false) too, follows; then First() and a walk with Next(), Close, and the
//   rows the table holds. Readings: the page and table triggers each step runs, in order; the Name and Value the page shows; the page's
//   filter on Group Code (TestPage.Filter); the row First lands on and the rows the walk reaches. Cells: Update(false), Update(true) and
//   no Update; a filter that keeps the row (SetRange or SetFilter on G1), one that drops it (SetRange on G0), Reset after a filter the
//   test set, and no filter change; a List page with OnAfterGetRecord only (the issue's page, page 97583 keeps its code line for line),
//   with OnAfterGetRecord and OnAfterGetCurrRecord, with OnAfterGetCurrRecord only and without row triggers; a Card page with
//   OnAfterGetRecord only; the same change of view from an action; on page 97584 also the middle and the first row, Previous() or a
//   third SetValue after the steps, then Close, a SetValue that fails first, and on page 97583 First() right after the first SetValue.
// BC versions: 27, 28, 29

table 97581 "LT RFU Line"
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
                Log: Codeunit "LT RFU Log";
            begin
                Log.Add('T.V(' + Name + ')');
            end;
        }
        field(4; Value; Text[30]) { }
    }

    keys
    {
        key(PK; "Group Code", "Entry No.") { Clustered = true; }
    }
}

codeunit 97582 "LT RFU Log"
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

// The issue's page, line for line: the Name control's OnValidate sets a filter on Rec and calls CurrPage.Update(false); the Value
// control's OnValidate calls CurrPage.Update(false). OnAfterGetRecord, no OnAfterGetCurrRecord.
page 97583 "LT RFU Issue Lines"
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

// A List page with OnAfterGetRecord only; the Name control's OnValidate and the action ChangeView do what the log codeunit's Setup says.
page 97584 "LT RFU Lines"
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
        Log: Codeunit "LT RFU Log";

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
page 97585 "LT RFU Lines Both"
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
        Log: Codeunit "LT RFU Log";

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
page 97586 "LT RFU Lines Curr"
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
        Log: Codeunit "LT RFU Log";

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

// The same without row triggers.
page 97587 "LT RFU Lines Plain"
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

    var
        Log: Codeunit "LT RFU Log";

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

// A Card page with OnAfterGetRecord only.
page 97588 "LT RFU Card"
{
    PageType = Card;
    SourceTable = "LT RFU Line";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            group(General)
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
        Log: Codeunit "LT RFU Log";

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

codeunit 97589 "LT RFU Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        Log: Codeunit "LT RFU Log";
        Seen: Dictionary of [Text, Text];

    local procedure Initialize(FilterStep: Integer; UpdateStep: Integer)
    var
        Line: Record "LT RFU Line";
    begin
        Line.DeleteAll();
        AddRow('G0', 1, 'A');
        AddRow('G1', 2, 'B');
        AddRow('G1', 3, 'C');
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

    // The readings of a List page: the open, the SetValue on Name (or the action), the SetValue on Value, First, the walk, Close.
    local procedure RunIssueLines()
    var
        Lines: TestPage "LT RFU Issue Lines";
        Walk: Text;
    begin
        Lines.OpenEdit();
        Lines.Last();
        Note('open', Log.Take());
        Lines.Name.SetValue('C2');
        Note('setname', Log.Take());
        Note('shown', Lines.Name.Value() + '/' + Lines.Value.Value());
        Note('filter', Lines.Filter.GetFilter("Group Code"));
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
        Note('stored', StoredRows());
    end;

    // Start: 0 Last(); 1 Last() then Previous(); 2 the row the page opens on; 3 the test's filter on G1, then Last(). The SetValue
    // types the row's name and '2'.
    local procedure RunLines(ByAction: Boolean; Start: Integer)
    var
        Lines: TestPage "LT RFU Lines";
        Walk: Text;
    begin
        Lines.OpenEdit();
        if Start = 3 then
            Lines.Filter.SetFilter("Group Code", 'G1');
        if Start <> 2 then
            Lines.Last();
        if Start = 1 then
            Lines.Previous();
        Note('open', Log.Take());
        if ByAction then
            Lines.ChangeViewAction.Invoke()
        else
            Lines.Name.SetValue(Lines.Name.Value() + '2');
        Note('setname', Log.Take());
        Note('shown', Lines.Name.Value() + '/' + Lines.Value.Value());
        Note('filter', Lines.Filter.GetFilter("Group Code"));
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
        Note('stored', StoredRows());
    end;

    // The Name control's OnValidate sets SetRange(G1) and calls Update(false); then a second SetValue on Name runs Reset and
    // Update(false).
    local procedure RunLinesFilterThenReset()
    var
        Lines: TestPage "LT RFU Lines";
        Walk: Text;
    begin
        Lines.OpenEdit();
        Lines.Last();
        Note('open', Log.Take());
        Lines.Name.SetValue('C2');
        Note('setname', Log.Take());
        Note('shown', Lines.Name.Value() + '/' + Lines.Value.Value());
        Note('filter', Lines.Filter.GetFilter("Group Code"));
        Log.Setup(3, 1);
        Lines.Name.SetValue('C3');
        Note('setreset', Log.Take());
        Note('shown2', Lines.Name.Value() + '/' + Lines.Value.Value());
        Note('filter2', Lines.Filter.GetFilter("Group Code"));
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

    local procedure RunBoth(ByAction: Boolean)
    var
        Lines: TestPage "LT RFU Lines Both";
        Walk: Text;
    begin
        Lines.OpenEdit();
        Lines.Last();
        Note('open', Log.Take());
        if ByAction then
            Lines.ChangeViewAction.Invoke()
        else
            Lines.Name.SetValue('C2');
        Note('setname', Log.Take());
        Note('shown', Lines.Name.Value() + '/' + Lines.Value.Value());
        Note('filter', Lines.Filter.GetFilter("Group Code"));
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
        Note('stored', StoredRows());
    end;

    local procedure RunCurr()
    var
        Lines: TestPage "LT RFU Lines Curr";
        Walk: Text;
    begin
        Lines.OpenEdit();
        Lines.Last();
        Note('open', Log.Take());
        Lines.Name.SetValue('C2');
        Note('setname', Log.Take());
        Note('shown', Lines.Name.Value() + '/' + Lines.Value.Value());
        Note('filter', Lines.Filter.GetFilter("Group Code"));
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
        Note('stored', StoredRows());
    end;

    local procedure RunPlain()
    var
        Lines: TestPage "LT RFU Lines Plain";
        Walk: Text;
    begin
        Lines.OpenEdit();
        Lines.Last();
        Note('open', Log.Take());
        Lines.Name.SetValue('C2');
        Note('setname', Log.Take());
        Note('shown', Lines.Name.Value() + '/' + Lines.Value.Value());
        Note('filter', Lines.Filter.GetFilter("Group Code"));
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
        Note('stored', StoredRows());
    end;

    local procedure RunCard(ByAction: Boolean)
    var
        Card: TestPage "LT RFU Card";
        Walk: Text;
    begin
        Card.OpenEdit();
        Card.Last();
        Note('open', Log.Take());
        if ByAction then
            Card.ChangeViewAction.Invoke()
        else
            Card.Name.SetValue('C2');
        Note('setname', Log.Take());
        Note('shown', Card.Name.Value() + '/' + Card.Value.Value());
        Note('filter', Card.Filter.GetFilter("Group Code"));
        Card.Value.SetValue('V');
        Note('setvalue', Log.Take());
        Note('shown2', Card.Name.Value() + '/' + Card.Value.Value());
        Card.First();
        Note('first', Card.Name.Value() + ':' + Log.Take());
        repeat
            Walk += Card.Name.Value() + ',';
        until not Card.Next();
        Note('walk', Walk);
        Log.Take();
        Card.Close();
        Note('close', Log.Take());
        Note('stored', StoredRows());
    end;

    [Test]
    procedure IssueList_SetRangeUpdateFalse_ReadsStoredRowAndAppliesFilter()
    begin
        Initialize(0, 0);
        RunIssueLines();
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setname', 'P.AGR(C);T.V(C2);P.AGR(C);P.AGR(B);');
        Expect('shown', 'C/');
        Expect('filter', 'G1');
        Expect('setvalue', 'P.AGR(C);P.AGR(C);P.AGR(B);');
        Expect('shown2', 'C/');
        Expect('first', 'B:');
        Expect('walk', 'B,C,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    [Test]
    procedure List_SetRangeKeepsRowUpdateFalse()
    begin
        Initialize(1, 1);
        RunLines(false, 0);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setname', 'P.AGR(C);T.V(C2);P.V(C2);P.AGR(C);P.AGR(B);');
        Expect('shown', 'C/');
        Expect('filter', 'G1');
        Expect('setvalue', 'P.AGR(C);P.AGR(C);P.AGR(B);');
        Expect('shown2', 'C/');
        Expect('first', 'B:');
        Expect('walk', 'B,C,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    [Test]
    procedure List_SetRangeKeepsRowUpdateTrue()
    begin
        Initialize(1, 2);
        RunLines(false, 0);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setname', 'P.AGR(C);T.V(C2);P.V(C2);P.AGR(C2);P.AGR(B);');
        Expect('shown', 'C2/');
        Expect('filter', 'G1');
        Expect('setvalue', 'P.AGR(C2);P.AGR(C2);P.AGR(B);');
        Expect('shown2', 'C2/');
        Expect('first', 'B:');
        Expect('walk', 'B,C2,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C2/,');
    end;

    [Test]
    procedure List_SetRangeKeepsRowNoUpdate()
    begin
        Initialize(1, 0);
        RunLines(false, 0);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setname', 'P.AGR(C);T.V(C2);P.V(C2);');
        Expect('shown', 'C2/');
        Expect('filter', 'G1');
        Expect('setvalue', 'P.AGR(C);P.AGR(B);');
        Expect('shown2', 'C/');
        Expect('first', 'B:');
        Expect('walk', 'B,C,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    [Test]
    procedure List_SetFilterKeepsRowUpdateFalse()
    begin
        Initialize(2, 1);
        RunLines(false, 0);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setname', 'P.AGR(C);T.V(C2);P.V(C2);P.AGR(C);P.AGR(B);');
        Expect('shown', 'C/');
        Expect('filter', 'G1');
        Expect('setvalue', 'P.AGR(C);P.AGR(C);P.AGR(B);');
        Expect('shown2', 'C/');
        Expect('first', 'B:');
        Expect('walk', 'B,C,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    [Test]
    procedure List_SetRangeDropsRowUpdateFalse()
    begin
        Initialize(4, 1);
        RunLines(false, 0);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setname', 'P.AGR(C);T.V(C2);P.V(C2);P.AGR(A);');
        Expect('shown', 'A/');
        Expect('filter', 'G0');
        Expect('setvalue', 'P.AGR(A);');
        Expect('shown2', 'A/');
        Expect('first', 'A:');
        Expect('walk', 'A,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    [Test]
    procedure List_ResetAfterTestFilterUpdateFalse()
    begin
        Initialize(3, 1);
        RunLines(false, 3);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);P.AGR(B);P.AGR(C);');
        Expect('setname', 'P.AGR(C);T.V(C2);P.V(C2);P.AGR(C);P.AGR(B);P.AGR(A);');
        Expect('shown', 'C/');
        Expect('filter', '');
        Expect('setvalue', 'P.AGR(C);P.AGR(C);P.AGR(B);P.AGR(A);');
        Expect('shown2', 'C/');
        Expect('first', 'A:');
        Expect('walk', 'A,B,C,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    [Test]
    procedure List_NoFilterChangeUpdateFalse()
    begin
        Initialize(0, 1);
        RunLines(false, 0);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setname', 'P.AGR(C);T.V(C2);P.V(C2);P.AGR(C);P.AGR(B);P.AGR(A);');
        Expect('shown', 'C/');
        Expect('filter', '');
        Expect('setvalue', 'P.AGR(C);P.AGR(C);P.AGR(B);P.AGR(A);');
        Expect('shown2', 'C/');
        Expect('first', 'A:');
        Expect('walk', 'A,B,C,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    [Test]
    procedure List_NoFilterChangeUpdateTrue()
    begin
        Initialize(0, 2);
        RunLines(false, 0);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setname', 'P.AGR(C);T.V(C2);P.V(C2);P.AGR(C2);P.AGR(B);P.AGR(A);');
        Expect('shown', 'C2/');
        Expect('filter', '');
        Expect('setvalue', 'P.AGR(C2);P.AGR(C2);P.AGR(B);P.AGR(A);');
        Expect('shown2', 'C2/');
        Expect('first', 'A:');
        Expect('walk', 'A,B,C2,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C2/,');
    end;

    [Test]
    procedure List_SetRangeThenReset_UpdateFalse()
    begin
        Initialize(1, 1);
        RunLinesFilterThenReset();
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setname', 'P.AGR(C);T.V(C2);P.V(C2);P.AGR(C);P.AGR(B);');
        Expect('shown', 'C/');
        Expect('filter', 'G1');
        Expect('setreset', 'P.AGR(C);T.V(C3);P.V(C3);P.AGR(C);P.AGR(B);P.AGR(A);');
        Expect('shown2', 'C/');
        Expect('filter2', '');
        Expect('first', 'A:');
        Expect('walk', 'A,B,C,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    [Test]
    procedure List_MiddleRow_SetRangeKeepsRowUpdateFalse()
    begin
        Initialize(1, 1);
        RunLines(false, 1);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setname', 'P.AGR(B);T.V(B2);P.V(B2);P.AGR(B);P.AGR(C);P.AGR(B);');
        Expect('shown', 'B/');
        Expect('filter', 'G1');
        Expect('setvalue', 'P.AGR(B);P.AGR(C);P.AGR(B);');
        Expect('shown2', 'B/');
        Expect('first', 'B:');
        Expect('walk', 'B,C,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    [Test]
    procedure List_FirstRow_SetRangeDropsRowUpdateFalse()
    begin
        Initialize(1, 1);
        RunLines(false, 2);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setname', 'T.V(A2);P.V(A2);P.AGR(B);P.AGR(C);P.AGR(B);');
        Expect('shown', 'B/');
        Expect('filter', 'G1');
        Expect('setvalue', 'P.AGR(B);P.AGR(C);P.AGR(B);');
        Expect('shown2', 'B/');
        Expect('first', 'B:');
        Expect('walk', 'B,C,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    [Test]
    procedure List_FirstRow_NoFilterChangeUpdateFalse()
    begin
        Initialize(0, 1);
        RunLines(false, 2);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setname', 'T.V(A2);P.V(A2);P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('shown', 'A/');
        Expect('filter', '');
        Expect('setvalue', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('shown2', 'A/');
        Expect('first', 'A:');
        Expect('walk', 'A,B,C,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    [Test]
    procedure List_Action_SetRangeKeepsRowUpdateFalse()
    begin
        Initialize(1, 1);
        RunLines(true, 0);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setname', 'P.AGR(C);P.AGR(C);P.A(C);P.AGR(C);P.AGR(C);P.AGR(B);');
        Expect('shown', 'C/');
        Expect('filter', 'G1');
        Expect('setvalue', 'P.AGR(C);P.AGR(C);P.AGR(B);');
        Expect('shown2', 'C/');
        Expect('first', 'B:');
        Expect('walk', 'B,C,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    [Test]
    procedure List_Action_SetRangeDropsRowUpdateFalse()
    begin
        Initialize(4, 1);
        RunLines(true, 0);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setname', 'P.AGR(C);P.AGR(C);P.A(C);P.AGR(A);P.AGR(A);');
        Expect('shown', 'A/');
        Expect('filter', 'G0');
        Expect('setvalue', 'P.AGR(A);');
        Expect('shown2', 'A/');
        Expect('first', 'A:');
        Expect('walk', 'A,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    [Test]
    procedure List_Action_SetRangeKeepsRowNoUpdate()
    begin
        Initialize(1, 0);
        RunLines(true, 0);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setname', 'P.AGR(C);P.AGR(C);P.A(C);P.AGR(C);P.AGR(C);P.AGR(B);');
        Expect('shown', 'C/');
        Expect('filter', 'G1');
        Expect('setvalue', 'P.AGR(C);P.AGR(C);P.AGR(B);');
        Expect('shown2', 'C/');
        Expect('first', 'B:');
        Expect('walk', 'B,C,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    [Test]
    procedure ListBoth_SetRangeKeepsRowUpdateFalse()
    begin
        Initialize(1, 1);
        RunBoth(false);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);P.AGR(A);P.AGCR(A);P.AGCR();P.AGR(C);P.AGR(C);P.AGCR(C);');
        Expect('setname', 'T.V(C2);P.V(C2);P.AGR(C);P.AGR(B);P.AGR(C);P.AGR(C);P.AGCR(C);');
        Expect('shown', 'C/');
        Expect('filter', 'G1');
        Expect('setvalue', 'P.AGR(C);P.AGR(B);P.AGR(C);P.AGR(C);P.AGCR(C);');
        Expect('shown2', 'C/');
        Expect('first', 'B:P.AGR(B);P.AGR(B);P.AGCR(B);');
        Expect('walk', 'B,C,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    [Test]
    procedure ListBoth_SetRangeKeepsRowUpdateTrue()
    begin
        Initialize(1, 2);
        RunBoth(false);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);P.AGR(A);P.AGCR(A);P.AGCR();P.AGR(C);P.AGR(C);P.AGCR(C);');
        Expect('setname', 'T.V(C2);P.V(C2);P.AGR(C2);P.AGR(B);P.AGR(C2);P.AGR(C2);P.AGCR(C2);');
        Expect('shown', 'C2/');
        Expect('filter', 'G1');
        Expect('setvalue', 'P.AGR(C2);P.AGR(B);P.AGR(C2);P.AGR(C2);P.AGCR(C2);');
        Expect('shown2', 'C2/');
        Expect('first', 'B:P.AGR(B);P.AGR(B);P.AGCR(B);');
        Expect('walk', 'B,C2,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C2/,');
    end;

    [Test]
    procedure ListBoth_SetRangeDropsRowUpdateFalse()
    begin
        Initialize(4, 1);
        RunBoth(false);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);P.AGR(A);P.AGCR(A);P.AGCR();P.AGR(C);P.AGR(C);P.AGCR(C);');
        Expect('setname', 'T.V(C2);P.V(C2);P.AGR(A);P.AGR(A);P.AGCR(A);');
        Expect('shown', 'A/');
        Expect('filter', 'G0');
        Expect('setvalue', 'P.AGR(A);P.AGR(A);P.AGCR(A);');
        Expect('shown2', 'A/');
        Expect('first', 'A:');
        Expect('walk', 'A,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    [Test]
    procedure ListBoth_NoFilterChangeUpdateFalse()
    begin
        Initialize(0, 1);
        RunBoth(false);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);P.AGR(A);P.AGCR(A);P.AGCR();P.AGR(C);P.AGR(C);P.AGCR(C);');
        Expect('setname', 'T.V(C2);P.V(C2);P.AGR(C);P.AGR(B);P.AGR(A);P.AGR(C);P.AGR(C);P.AGCR(C);');
        Expect('shown', 'C/');
        Expect('filter', '');
        Expect('setvalue', 'P.AGR(C);P.AGR(B);P.AGR(A);P.AGR(C);P.AGR(C);P.AGCR(C);');
        Expect('shown2', 'C/');
        Expect('first', 'A:P.AGR(A);P.AGR(A);P.AGCR(A);');
        Expect('walk', 'A,B,C,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    [Test]
    procedure ListBoth_Action_SetRangeKeepsRowUpdateFalse()
    begin
        Initialize(1, 1);
        RunBoth(true);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);P.AGR(A);P.AGCR(A);P.AGCR();P.AGR(C);P.AGR(C);P.AGCR(C);');
        Expect('setname', 'P.AGR(C);P.A(C);P.AGR(C);P.AGR(C);P.AGR(B);P.AGR(C);P.AGR(C);P.AGCR(C);');
        Expect('shown', 'C/');
        Expect('filter', 'G1');
        Expect('setvalue', 'P.AGR(C);P.AGR(B);P.AGR(C);P.AGR(C);P.AGCR(C);');
        Expect('shown2', 'C/');
        Expect('first', 'B:P.AGR(B);P.AGR(B);P.AGCR(B);');
        Expect('walk', 'B,C,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    [Test]
    procedure ListCurr_SetRangeKeepsRowUpdateFalse()
    begin
        Initialize(1, 1);
        RunCurr();
        Expect('open', 'P.AGCR(A);P.AGCR();P.AGCR(C);');
        Expect('setname', 'T.V(C2);P.V(C2);P.AGCR(C);');
        Expect('shown', 'C/');
        Expect('filter', 'G1');
        Expect('setvalue', 'P.AGCR(C);');
        Expect('shown2', 'C/');
        Expect('first', 'B:P.AGCR(B);');
        Expect('walk', 'B,C,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    [Test]
    procedure ListCurr_NoFilterChangeUpdateFalse()
    begin
        Initialize(0, 1);
        RunCurr();
        Expect('open', 'P.AGCR(A);P.AGCR();P.AGCR(C);');
        Expect('setname', 'T.V(C2);P.V(C2);P.AGCR(C);');
        Expect('shown', 'C/');
        Expect('filter', '');
        Expect('setvalue', 'P.AGCR(C);');
        Expect('shown2', 'C/');
        Expect('first', 'A:P.AGCR(A);');
        Expect('walk', 'A,B,C,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    [Test]
    procedure ListPlain_SetRangeKeepsRowUpdateFalse()
    begin
        Initialize(1, 1);
        RunPlain();
        Expect('open', '');
        Expect('setname', 'T.V(C2);P.V(C2);');
        Expect('shown', 'C/');
        Expect('filter', 'G1');
        Expect('setvalue', '');
        Expect('shown2', 'C/');
        Expect('first', 'B:');
        Expect('walk', 'B,C,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    [Test]
    procedure ListPlain_NoFilterChangeUpdateFalse()
    begin
        Initialize(0, 1);
        RunPlain();
        Expect('open', '');
        Expect('setname', 'T.V(C2);P.V(C2);');
        Expect('shown', 'C/');
        Expect('filter', '');
        Expect('setvalue', '');
        Expect('shown2', 'C/');
        Expect('first', 'A:');
        Expect('walk', 'A,B,C,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    [Test]
    procedure Card_SetRangeKeepsRowUpdateFalse()
    begin
        Initialize(1, 1);
        RunCard(false);
        Expect('open', 'P.AGR(A);P.AGR(C);');
        Expect('setname', 'T.V(C2);P.V(C2);P.AGR(C);');
        Expect('shown', 'C/');
        Expect('filter', 'G1');
        Expect('setvalue', 'P.AGR(C);');
        Expect('shown2', 'C/');
        Expect('first', 'B:P.AGR(B);');
        Expect('walk', 'B,C,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    [Test]
    procedure Card_SetRangeKeepsRowUpdateTrue()
    begin
        Initialize(1, 2);
        RunCard(false);
        Expect('open', 'P.AGR(A);P.AGR(C);');
        Expect('setname', 'T.V(C2);P.V(C2);P.AGR(C2);');
        Expect('shown', 'C2/');
        Expect('filter', 'G1');
        Expect('setvalue', 'P.AGR(C2);');
        Expect('shown2', 'C2/');
        Expect('first', 'B:P.AGR(B);');
        Expect('walk', 'B,C2,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C2/,');
    end;

    [Test]
    procedure Card_NoFilterChangeUpdateFalse()
    begin
        Initialize(0, 1);
        RunCard(false);
        Expect('open', 'P.AGR(A);P.AGR(C);');
        Expect('setname', 'T.V(C2);P.V(C2);P.AGR(C);');
        Expect('shown', 'C/');
        Expect('filter', '');
        Expect('setvalue', 'P.AGR(C);');
        Expect('shown2', 'C/');
        Expect('first', 'A:P.AGR(A);');
        Expect('walk', 'A,B,C,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    [Test]
    procedure Card_Action_SetRangeKeepsRowUpdateFalse()
    begin
        Initialize(1, 1);
        RunCard(true);
        Expect('open', 'P.AGR(A);P.AGR(C);');
        Expect('setname', 'P.AGR(C);P.A(C);P.AGR(C);P.AGR(C);');
        Expect('shown', 'C/');
        Expect('filter', 'G1');
        Expect('setvalue', 'P.AGR(C);');
        Expect('shown2', 'C/');
        Expect('first', 'B:P.AGR(B);');
        Expect('walk', 'B,C,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    // SetFilter on G1 with Update(true).
    [Test]
    procedure List_SetFilterKeepsRowUpdateTrue()
    begin
        Initialize(2, 2);
        RunLines(false, 0);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setname', 'P.AGR(C);T.V(C2);P.V(C2);P.AGR(C2);P.AGR(B);');
        Expect('shown', 'C2/');
        Expect('filter', 'G1');
        Expect('setvalue', 'P.AGR(C2);P.AGR(C2);P.AGR(B);');
        Expect('shown2', 'C2/');
        Expect('first', 'B:');
        Expect('walk', 'B,C2,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C2/,');
    end;

    // Reset while the page holds no filter: the view stays as it is.
    [Test]
    procedure List_ResetWithoutFilterUpdateFalse()
    begin
        Initialize(3, 1);
        RunLines(false, 0);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setname', 'P.AGR(C);T.V(C2);P.V(C2);P.AGR(C);P.AGR(B);P.AGR(A);');
        Expect('shown', 'C/');
        Expect('filter', '');
        Expect('setvalue', 'P.AGR(C);P.AGR(C);P.AGR(B);P.AGR(A);');
        Expect('shown2', 'C/');
        Expect('first', 'A:');
        Expect('walk', 'A,B,C,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    // SetRange on G0, which drops C, without CurrPage.Update; the SetValue on Value then calls Update(false).
    [Test]
    procedure List_SetRangeDropsRowNoUpdate()
    begin
        Initialize(4, 0);
        RunLines(false, 0);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setname', 'P.AGR(C);T.V(C2);P.V(C2);');
        Expect('shown', 'C2/');
        Expect('filter', 'G0');
        Expect('setvalue', 'P.AGR(A);');
        Expect('shown2', 'A/');
        Expect('first', 'A:');
        Expect('walk', 'A,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    // After the steps of List_SetRangeKeepsRowUpdateFalse, Previous() instead of First().
    [Test]
    procedure List_PreviousAfterSetValues()
    begin
        Initialize(1, 1);
        RunLinesThen(0);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setname', 'P.AGR(C);T.V(C2);P.V(C2);P.AGR(C);P.AGR(B);');
        Expect('shown', 'C/');
        Expect('setvalue', 'P.AGR(C);P.AGR(C);P.AGR(B);');
        Expect('shown2', 'C/');
        Expect('then', 'Yes:B:');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    // After the steps of List_SetRangeKeepsRowUpdateFalse, a third SetValue, on Value again.
    [Test]
    procedure List_ThirdSetValue()
    begin
        Initialize(1, 1);
        RunLinesThen(1);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setname', 'P.AGR(C);T.V(C2);P.V(C2);P.AGR(C);P.AGR(B);');
        Expect('shown', 'C/');
        Expect('setvalue', 'P.AGR(C);P.AGR(C);P.AGR(B);');
        Expect('shown2', 'C/');
        Expect('then', 'C/:P.AGR(C);P.AGR(C);P.AGR(B);');
        Expect('close', 'P.AGR(C);');
        Expect('stored', 'A/,B/,C/,');
    end;

    // A SetValue on Name that fails (longer than Text[30]), then the steps of List_SetRangeKeepsRowUpdateFalse.
    [Test]
    procedure List_TooLongName_ThenSetRangeKeepsRowUpdateFalse()
    var
        Lines: TestPage "LT RFU Lines";
    begin
        Initialize(1, 1);
        Lines.OpenEdit();
        Lines.Last();
        Note('open', Log.Take());
        asserterror Lines.Name.SetValue('C234567890123456789012345678901');
        Note('error', Log.Take());
        Lines.Name.SetValue('C2');
        Note('setname', Log.Take());
        Note('shown', Lines.Name.Value() + '/' + Lines.Value.Value());
        Lines.Value.SetValue('V');
        Note('setvalue', Log.Take());
        Note('shown2', Lines.Name.Value() + '/' + Lines.Value.Value());
        Lines.Close();
        Note('close', Log.Take());
        Note('stored', StoredRows());
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('error', 'P.AGR(C);');
        Expect('setname', 'T.V(C2);P.V(C2);P.AGR(C);P.AGR(B);');
        Expect('shown', 'C/');
        Expect('setvalue', 'P.AGR(C);P.AGR(C);P.AGR(B);');
        Expect('shown2', 'C/');
        Expect('close', 'P.AGR(C);');
        Expect('stored', 'A/,B/,C/,');
    end;

    // The issue's page: First() right after the SetValue on Name, without the SetValue on Value.
    [Test]
    procedure IssueList_FirstAfterSetName()
    var
        Lines: TestPage "LT RFU Issue Lines";
        Walk: Text;
    begin
        Initialize(0, 0);
        Lines.OpenEdit();
        Lines.Last();
        Note('open', Log.Take());
        Lines.Name.SetValue('C2');
        Note('setname', Log.Take());
        Note('shown', Lines.Name.Value() + '/' + Lines.Value.Value());
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
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setname', 'P.AGR(C);T.V(C2);P.AGR(C);P.AGR(B);');
        Expect('shown', 'C/');
        Expect('first', 'B:');
        Expect('walk', 'B,C,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    // The issue's page with the reporter's steps: First() and then Close(), without a walk.
    [Test]
    procedure IssueList_FirstThenClose()
    var
        Lines: TestPage "LT RFU Issue Lines";
    begin
        Initialize(0, 0);
        Lines.OpenEdit();
        Lines.Last();
        Note('open', Log.Take());
        Lines.Name.SetValue('C2');
        Note('setname', Log.Take());
        Lines.Value.SetValue('V');
        Note('setvalue', Log.Take());
        Note('shown2', Lines.Name.Value() + '/' + Lines.Value.Value());
        Lines.First();
        Note('first', Lines.Name.Value() + ':' + Log.Take());
        Lines.Close();
        Note('close', Log.Take());
        Note('stored', StoredRows());
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setname', 'P.AGR(C);T.V(C2);P.AGR(C);P.AGR(B);');
        Expect('setvalue', 'P.AGR(C);P.AGR(C);P.AGR(B);');
        Expect('shown2', 'C/');
        Expect('first', 'B:');
        Expect('close', '');
        Expect('stored', 'A/,B/,C/,');
    end;

    // The steps of List_SetRangeKeepsRowUpdateFalse, then After: 0 Previous(), 1 a third SetValue on Value; then Close.
    local procedure RunLinesThen(After: Integer)
    var
        Lines: TestPage "LT RFU Lines";
    begin
        Lines.OpenEdit();
        Lines.Last();
        Note('open', Log.Take());
        Lines.Name.SetValue('C2');
        Note('setname', Log.Take());
        Note('shown', Lines.Name.Value() + '/' + Lines.Value.Value());
        Lines.Value.SetValue('V');
        Note('setvalue', Log.Take());
        Note('shown2', Lines.Name.Value() + '/' + Lines.Value.Value());
        if After = 0 then
            Note('then', Format(Lines.Previous()) + ':' + Lines.Name.Value() + ':' + Log.Take())
        else begin
            Lines.Value.SetValue('W');
            Note('then', Lines.Name.Value() + '/' + Lines.Value.Value() + ':' + Log.Take());
        end;
        Lines.Close();
        Note('close', Log.Take());
        Note('stored', StoredRows());
    end;

    // Last in this copy of the probe (the corpus file has it after List_SetRangeDropsRowUpdateFalse): main refuses it, and a
    // refusal makes main distrust every later failure of the codeunit (tests/neighbors/README.md).
    [Test]
    procedure List_SetRangeDropsRowUpdateTrue()
    begin
        Initialize(4, 2);
        RunLines(false, 0);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('setname', 'P.AGR(C);T.V(C2);P.V(C2);P.AGR(A);');
        Expect('shown', 'A/');
        Expect('filter', 'G0');
        Expect('setvalue', 'P.AGR(A);');
        Expect('shown2', 'A/');
        Expect('first', 'A:');
        Expect('walk', 'A,,');
        Expect('close', '');
        Expect('stored', 'A/,B/,C2/,');
    end;
}
