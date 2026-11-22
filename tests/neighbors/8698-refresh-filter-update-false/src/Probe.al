// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/page/page-update-method
// Scope: in-scope
// local object: the control's OnValidate and the action that change the page's view, and the page triggers that log, are what is under test
// Fixtures used: shared Assert (60021); own tables 97581, 97570, log codeunits 97582, 97573, pages 97583-97588, 97571, tests 97589, 97572
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

    // The test's read-only calls at a point of a cell (ReadAt): 1 before the SetValue on Name (or the action), 2 between it
    // and the SetValue on Value, 3 after that, before First. Block 1: AssertEquals, Caption and Editable on both controls,
    // Caption and Editable on the page; block 2: Enabled, Visible, HideValue, ShowMandatory and ValidationErrorCount on both
    // controls, ValidationErrorCount on the page. The answers, then what the page logged meanwhile. Copies of RunIssueLines,
    // RunLines, RunBoth, RunCurr and RunCard with the calls at that point; two blocks keep each test at 32 calls or fewer.
    local procedure ReadsIssueLines(var Lines: TestPage "LT RFU Issue Lines"; Block: Integer): Text
    var
        Result: Text;
    begin
        if Block = 1 then begin
            Lines.Name.AssertEquals('C');
            Lines.Value.AssertEquals('');
            Result := Lines.Name.Caption() + '/' + Lines.Value.Caption() + '/' + Format(Lines.Name.Editable()) + '/' + Format(Lines.Value.Editable()) + '/' +
              Lines.Caption() + '/' + Format(Lines.Editable());
        end else
            Result := Format(Lines.Name.Enabled()) + '/' + Format(Lines.Value.Enabled()) + '/' + Format(Lines.Name.Visible()) + '/' + Format(Lines.Value.Visible()) + '/' +
              Format(Lines.Name.HideValue()) + '/' + Format(Lines.Value.HideValue()) + '/' + Format(Lines.Name.ShowMandatory()) + '/' +
              Format(Lines.Value.ShowMandatory()) + '/' + Format(Lines.Name.ValidationErrorCount()) + '/' + Format(Lines.Value.ValidationErrorCount()) + '/' +
              Format(Lines.ValidationErrorCount());
        exit(Result + ':' + Log.Take());
    end;

    local procedure RunReadsIssueLines(ReadAt: Integer; Block: Integer)
    var
        Lines: TestPage "LT RFU Issue Lines";
        Walk: Text;
    begin
        Lines.OpenEdit();
        Lines.Last();
        Note('open', Log.Take());
        if ReadAt = 1 then
            Note('reads', ReadsIssueLines(Lines, Block));
        Lines.Name.SetValue('C2');
        Note('setname', Log.Take());
        Note('shown', Lines.Name.Value() + '/' + Lines.Value.Value());
        Note('filter', Lines.Filter.GetFilter("Group Code"));
        if ReadAt = 2 then
            Note('reads', ReadsIssueLines(Lines, Block));
        Lines.Value.SetValue('V');
        Note('setvalue', Log.Take());
        Note('shown2', Lines.Name.Value() + '/' + Lines.Value.Value());
        if ReadAt = 3 then
            Note('reads', ReadsIssueLines(Lines, Block));
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

    local procedure ReadsLines(var Lines: TestPage "LT RFU Lines"; Block: Integer): Text
    var
        Result: Text;
    begin
        if Block = 1 then begin
            Lines.Name.AssertEquals('C');
            Lines.Value.AssertEquals('');
            Result := Lines.Name.Caption() + '/' + Lines.Value.Caption() + '/' + Format(Lines.Name.Editable()) + '/' + Format(Lines.Value.Editable()) + '/' +
              Lines.Caption() + '/' + Format(Lines.Editable());
        end else
            Result := Format(Lines.Name.Enabled()) + '/' + Format(Lines.Value.Enabled()) + '/' + Format(Lines.Name.Visible()) + '/' + Format(Lines.Value.Visible()) + '/' +
              Format(Lines.Name.HideValue()) + '/' + Format(Lines.Value.HideValue()) + '/' + Format(Lines.Name.ShowMandatory()) + '/' +
              Format(Lines.Value.ShowMandatory()) + '/' + Format(Lines.Name.ValidationErrorCount()) + '/' + Format(Lines.Value.ValidationErrorCount()) + '/' +
              Format(Lines.ValidationErrorCount());
        exit(Result + ':' + Log.Take());
    end;

    local procedure RunReadsLines(ByAction: Boolean; ReadAt: Integer; Block: Integer)
    var
        Lines: TestPage "LT RFU Lines";
        Walk: Text;
    begin
        Lines.OpenEdit();
        Lines.Last();
        Note('open', Log.Take());
        if ReadAt = 1 then
            Note('reads', ReadsLines(Lines, Block));
        if ByAction then
            Lines.ChangeViewAction.Invoke()
        else
            Lines.Name.SetValue(Lines.Name.Value() + '2');
        Note('setname', Log.Take());
        Note('shown', Lines.Name.Value() + '/' + Lines.Value.Value());
        Note('filter', Lines.Filter.GetFilter("Group Code"));
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

    local procedure ReadsBoth(var Lines: TestPage "LT RFU Lines Both"; Block: Integer): Text
    var
        Result: Text;
    begin
        if Block = 1 then begin
            Lines.Name.AssertEquals('C');
            Lines.Value.AssertEquals('');
            Result := Lines.Name.Caption() + '/' + Lines.Value.Caption() + '/' + Format(Lines.Name.Editable()) + '/' + Format(Lines.Value.Editable()) + '/' +
              Lines.Caption() + '/' + Format(Lines.Editable());
        end else
            Result := Format(Lines.Name.Enabled()) + '/' + Format(Lines.Value.Enabled()) + '/' + Format(Lines.Name.Visible()) + '/' + Format(Lines.Value.Visible()) + '/' +
              Format(Lines.Name.HideValue()) + '/' + Format(Lines.Value.HideValue()) + '/' + Format(Lines.Name.ShowMandatory()) + '/' +
              Format(Lines.Value.ShowMandatory()) + '/' + Format(Lines.Name.ValidationErrorCount()) + '/' + Format(Lines.Value.ValidationErrorCount()) + '/' +
              Format(Lines.ValidationErrorCount());
        exit(Result + ':' + Log.Take());
    end;

    local procedure RunReadsBoth(ByAction: Boolean; ReadAt: Integer; Block: Integer)
    var
        Lines: TestPage "LT RFU Lines Both";
        Walk: Text;
    begin
        Lines.OpenEdit();
        Lines.Last();
        Note('open', Log.Take());
        if ReadAt = 1 then
            Note('reads', ReadsBoth(Lines, Block));
        if ByAction then
            Lines.ChangeViewAction.Invoke()
        else
            Lines.Name.SetValue('C2');
        Note('setname', Log.Take());
        Note('shown', Lines.Name.Value() + '/' + Lines.Value.Value());
        Note('filter', Lines.Filter.GetFilter("Group Code"));
        if ReadAt = 2 then
            Note('reads', ReadsBoth(Lines, Block));
        Lines.Value.SetValue('V');
        Note('setvalue', Log.Take());
        Note('shown2', Lines.Name.Value() + '/' + Lines.Value.Value());
        if ReadAt = 3 then
            Note('reads', ReadsBoth(Lines, Block));
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

    local procedure ReadsCurr(var Lines: TestPage "LT RFU Lines Curr"; Block: Integer): Text
    var
        Result: Text;
    begin
        if Block = 1 then begin
            Lines.Name.AssertEquals('C');
            Lines.Value.AssertEquals('');
            Result := Lines.Name.Caption() + '/' + Lines.Value.Caption() + '/' + Format(Lines.Name.Editable()) + '/' + Format(Lines.Value.Editable()) + '/' +
              Lines.Caption() + '/' + Format(Lines.Editable());
        end else
            Result := Format(Lines.Name.Enabled()) + '/' + Format(Lines.Value.Enabled()) + '/' + Format(Lines.Name.Visible()) + '/' + Format(Lines.Value.Visible()) + '/' +
              Format(Lines.Name.HideValue()) + '/' + Format(Lines.Value.HideValue()) + '/' + Format(Lines.Name.ShowMandatory()) + '/' +
              Format(Lines.Value.ShowMandatory()) + '/' + Format(Lines.Name.ValidationErrorCount()) + '/' + Format(Lines.Value.ValidationErrorCount()) + '/' +
              Format(Lines.ValidationErrorCount());
        exit(Result + ':' + Log.Take());
    end;

    local procedure RunReadsCurr(ReadAt: Integer; Block: Integer)
    var
        Lines: TestPage "LT RFU Lines Curr";
        Walk: Text;
    begin
        Lines.OpenEdit();
        Lines.Last();
        Note('open', Log.Take());
        if ReadAt = 1 then
            Note('reads', ReadsCurr(Lines, Block));
        Lines.Name.SetValue('C2');
        Note('setname', Log.Take());
        Note('shown', Lines.Name.Value() + '/' + Lines.Value.Value());
        Note('filter', Lines.Filter.GetFilter("Group Code"));
        if ReadAt = 2 then
            Note('reads', ReadsCurr(Lines, Block));
        Lines.Value.SetValue('V');
        Note('setvalue', Log.Take());
        Note('shown2', Lines.Name.Value() + '/' + Lines.Value.Value());
        if ReadAt = 3 then
            Note('reads', ReadsCurr(Lines, Block));
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

    local procedure ReadsCard(var Card: TestPage "LT RFU Card"; Block: Integer): Text
    var
        Result: Text;
    begin
        if Block = 1 then begin
            Card.Name.AssertEquals('C');
            Card.Value.AssertEquals('');
            Result := Card.Name.Caption() + '/' + Card.Value.Caption() + '/' + Format(Card.Name.Editable()) + '/' + Format(Card.Value.Editable()) + '/' +
              Card.Caption() + '/' + Format(Card.Editable());
        end else
            Result := Format(Card.Name.Enabled()) + '/' + Format(Card.Value.Enabled()) + '/' + Format(Card.Name.Visible()) + '/' + Format(Card.Value.Visible()) + '/' +
              Format(Card.Name.HideValue()) + '/' + Format(Card.Value.HideValue()) + '/' + Format(Card.Name.ShowMandatory()) + '/' +
              Format(Card.Value.ShowMandatory()) + '/' + Format(Card.Name.ValidationErrorCount()) + '/' + Format(Card.Value.ValidationErrorCount()) + '/' +
              Format(Card.ValidationErrorCount());
        exit(Result + ':' + Log.Take());
    end;

    local procedure RunReadsCard(ByAction: Boolean; ReadAt: Integer; Block: Integer)
    var
        Card: TestPage "LT RFU Card";
        Walk: Text;
    begin
        Card.OpenEdit();
        Card.Last();
        Note('open', Log.Take());
        if ReadAt = 1 then
            Note('reads', ReadsCard(Card, Block));
        if ByAction then
            Card.ChangeViewAction.Invoke()
        else
            Card.Name.SetValue('C2');
        Note('setname', Log.Take());
        Note('shown', Card.Name.Value() + '/' + Card.Value.Value());
        Note('filter', Card.Filter.GetFilter("Group Code"));
        if ReadAt = 2 then
            Note('reads', ReadsCard(Card, Block));
        Card.Value.SetValue('V');
        Note('setvalue', Log.Take());
        Note('shown2', Card.Name.Value() + '/' + Card.Value.Value());
        if ReadAt = 3 then
            Note('reads', ReadsCard(Card, Block));
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

    // The cell :815 with AssertEquals, Caption and Editable before the SetValue on Name (or the action).
    [Test]
    procedure IssueList_ReadsCaptionsAndValues_BeforeFirstStep()
    begin
        Initialize(1, 1);
        RunReadsIssueLines(1, 1);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('reads', 'Name/Value/Yes/Yes/LT RFU Issue Lines/Yes:');
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

    // The cell :815 with Enabled, Visible, HideValue, ShowMandatory and ValidationErrorCount before the SetValue on Name (or the action).
    [Test]
    procedure IssueList_ReadsControlState_BeforeFirstStep()
    begin
        Initialize(1, 1);
        RunReadsIssueLines(1, 2);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('reads', 'Yes/Yes/Yes/Yes/No/No/No/No/0/0/0:');
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

    // The cell :815 with AssertEquals, Caption and Editable between it and the SetValue on Value.
    [Test]
    procedure IssueList_ReadsCaptionsAndValues_BetweenSteps()
    begin
        Initialize(1, 1);
        RunReadsIssueLines(2, 1);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('reads', 'Name/Value/Yes/Yes/LT RFU Issue Lines/Yes:');
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

    // The cell :815 with Enabled, Visible, HideValue, ShowMandatory and ValidationErrorCount between it and the SetValue on Value.
    [Test]
    procedure IssueList_ReadsControlState_BetweenSteps()
    begin
        Initialize(1, 1);
        RunReadsIssueLines(2, 2);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('reads', 'Yes/Yes/Yes/Yes/No/No/No/No/0/0/0:');
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

    // The cell :815 with AssertEquals, Caption and Editable after the SetValue on Value, before First.
    [Test]
    procedure IssueList_ReadsCaptionsAndValues_AfterSteps()
    begin
        Initialize(1, 1);
        RunReadsIssueLines(3, 1);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('reads', 'Name/Value/Yes/Yes/LT RFU Issue Lines/Yes:');
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

    // The cell :815 with Enabled, Visible, HideValue, ShowMandatory and ValidationErrorCount after the SetValue on Value, before First.
    [Test]
    procedure IssueList_ReadsControlState_AfterSteps()
    begin
        Initialize(1, 1);
        RunReadsIssueLines(3, 2);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('reads', 'Yes/Yes/Yes/Yes/No/No/No/No/0/0/0:');
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

    // The cell :832 with AssertEquals, Caption and Editable before the SetValue on Name (or the action).
    [Test]
    procedure List_ReadsCaptionsAndValues_BeforeFirstStep()
    begin
        Initialize(1, 1);
        RunReadsLines(false, 1, 1);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('reads', 'Name/Value/Yes/Yes/LT RFU Lines/Yes:');
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

    // The cell :832 with Enabled, Visible, HideValue, ShowMandatory and ValidationErrorCount before the SetValue on Name (or the action).
    [Test]
    procedure List_ReadsControlState_BeforeFirstStep()
    begin
        Initialize(1, 1);
        RunReadsLines(false, 1, 2);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('reads', 'Yes/Yes/Yes/Yes/No/No/No/No/0/0/0:');
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

    // The cell :832 with AssertEquals, Caption and Editable between it and the SetValue on Value.
    [Test]
    procedure List_ReadsCaptionsAndValues_BetweenSteps()
    begin
        Initialize(1, 1);
        RunReadsLines(false, 2, 1);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('reads', 'Name/Value/Yes/Yes/LT RFU Lines/Yes:');
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

    // The cell :832 with Enabled, Visible, HideValue, ShowMandatory and ValidationErrorCount between it and the SetValue on Value.
    [Test]
    procedure List_ReadsControlState_BetweenSteps()
    begin
        Initialize(1, 1);
        RunReadsLines(false, 2, 2);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('reads', 'Yes/Yes/Yes/Yes/No/No/No/No/0/0/0:');
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

    // The cell :832 with AssertEquals, Caption and Editable after the SetValue on Value, before First.
    [Test]
    procedure List_ReadsCaptionsAndValues_AfterSteps()
    begin
        Initialize(1, 1);
        RunReadsLines(false, 3, 1);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('reads', 'Name/Value/Yes/Yes/LT RFU Lines/Yes:');
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

    // The cell :832 with Enabled, Visible, HideValue, ShowMandatory and ValidationErrorCount after the SetValue on Value, before First.
    [Test]
    procedure List_ReadsControlState_AfterSteps()
    begin
        Initialize(1, 1);
        RunReadsLines(false, 3, 2);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('reads', 'Yes/Yes/Yes/Yes/No/No/No/No/0/0/0:');
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

    // The cell :1054 with AssertEquals, Caption and Editable before the SetValue on Name (or the action).
    [Test]
    procedure List_Action_ReadsCaptionsAndValues_BeforeFirstStep()
    begin
        Initialize(1, 1);
        RunReadsLines(true, 1, 1);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('reads', 'Name/Value/Yes/Yes/LT RFU Lines/Yes:');
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

    // The cell :1054 with Enabled, Visible, HideValue, ShowMandatory and ValidationErrorCount before the SetValue on Name (or the action).
    [Test]
    procedure List_Action_ReadsControlState_BeforeFirstStep()
    begin
        Initialize(1, 1);
        RunReadsLines(true, 1, 2);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('reads', 'Yes/Yes/Yes/Yes/No/No/No/No/0/0/0:');
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

    // The cell :1054 with AssertEquals, Caption and Editable between it and the SetValue on Value.
    [Test]
    procedure List_Action_ReadsCaptionsAndValues_BetweenSteps()
    begin
        Initialize(1, 1);
        RunReadsLines(true, 2, 1);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('reads', 'Name/Value/Yes/Yes/LT RFU Lines/Yes:');
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

    // The cell :1054 with Enabled, Visible, HideValue, ShowMandatory and ValidationErrorCount between it and the SetValue on Value.
    [Test]
    procedure List_Action_ReadsControlState_BetweenSteps()
    begin
        Initialize(1, 1);
        RunReadsLines(true, 2, 2);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('reads', 'Yes/Yes/Yes/Yes/No/No/No/No/0/0/0:');
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

    // The cell :1054 with AssertEquals, Caption and Editable after the SetValue on Value, before First.
    [Test]
    procedure List_Action_ReadsCaptionsAndValues_AfterSteps()
    begin
        Initialize(1, 1);
        RunReadsLines(true, 3, 1);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('reads', 'Name/Value/Yes/Yes/LT RFU Lines/Yes:');
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

    // The cell :1054 with Enabled, Visible, HideValue, ShowMandatory and ValidationErrorCount after the SetValue on Value, before First.
    [Test]
    procedure List_Action_ReadsControlState_AfterSteps()
    begin
        Initialize(1, 1);
        RunReadsLines(true, 3, 2);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);');
        Expect('reads', 'Yes/Yes/Yes/Yes/No/No/No/No/0/0/0:');
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

    // The cell :1105 with AssertEquals, Caption and Editable before the SetValue on Name (or the action).
    [Test]
    procedure ListBoth_ReadsCaptionsAndValues_BeforeFirstStep()
    begin
        Initialize(1, 1);
        RunReadsBoth(false, 1, 1);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);P.AGR(A);P.AGCR(A);P.AGCR();P.AGR(C);P.AGR(C);P.AGCR(C);');
        Expect('reads', 'Name/Value/Yes/Yes/LT RFU Lines Both/Yes:');
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

    // The cell :1105 with Enabled, Visible, HideValue, ShowMandatory and ValidationErrorCount before the SetValue on Name (or the action).
    [Test]
    procedure ListBoth_ReadsControlState_BeforeFirstStep()
    begin
        Initialize(1, 1);
        RunReadsBoth(false, 1, 2);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);P.AGR(A);P.AGCR(A);P.AGCR();P.AGR(C);P.AGR(C);P.AGCR(C);');
        Expect('reads', 'Yes/Yes/Yes/Yes/No/No/No/No/0/0/0:');
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

    // The cell :1105 with AssertEquals, Caption and Editable between it and the SetValue on Value.
    [Test]
    procedure ListBoth_ReadsCaptionsAndValues_BetweenSteps()
    begin
        Initialize(1, 1);
        RunReadsBoth(false, 2, 1);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);P.AGR(A);P.AGCR(A);P.AGCR();P.AGR(C);P.AGR(C);P.AGCR(C);');
        Expect('reads', 'Name/Value/Yes/Yes/LT RFU Lines Both/Yes:');
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

    // The cell :1105 with Enabled, Visible, HideValue, ShowMandatory and ValidationErrorCount between it and the SetValue on Value.
    [Test]
    procedure ListBoth_ReadsControlState_BetweenSteps()
    begin
        Initialize(1, 1);
        RunReadsBoth(false, 2, 2);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);P.AGR(A);P.AGCR(A);P.AGCR();P.AGR(C);P.AGR(C);P.AGCR(C);');
        Expect('reads', 'Yes/Yes/Yes/Yes/No/No/No/No/0/0/0:');
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

    // The cell :1105 with AssertEquals, Caption and Editable after the SetValue on Value, before First.
    [Test]
    procedure ListBoth_ReadsCaptionsAndValues_AfterSteps()
    begin
        Initialize(1, 1);
        RunReadsBoth(false, 3, 1);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);P.AGR(A);P.AGCR(A);P.AGCR();P.AGR(C);P.AGR(C);P.AGCR(C);');
        Expect('reads', 'Name/Value/Yes/Yes/LT RFU Lines Both/Yes:');
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

    // The cell :1105 with Enabled, Visible, HideValue, ShowMandatory and ValidationErrorCount after the SetValue on Value, before First.
    [Test]
    procedure ListBoth_ReadsControlState_AfterSteps()
    begin
        Initialize(1, 1);
        RunReadsBoth(false, 3, 2);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);P.AGR(A);P.AGCR(A);P.AGCR();P.AGR(C);P.AGR(C);P.AGCR(C);');
        Expect('reads', 'Yes/Yes/Yes/Yes/No/No/No/No/0/0/0:');
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

    // The cell :1173 with AssertEquals, Caption and Editable before the SetValue on Name (or the action).
    [Test]
    procedure ListBoth_Action_ReadsCaptionsAndValues_BeforeFirstStep()
    begin
        Initialize(1, 1);
        RunReadsBoth(true, 1, 1);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);P.AGR(A);P.AGCR(A);P.AGCR();P.AGR(C);P.AGR(C);P.AGCR(C);');
        Expect('reads', 'Name/Value/Yes/Yes/LT RFU Lines Both/Yes:');
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

    // The cell :1173 with Enabled, Visible, HideValue, ShowMandatory and ValidationErrorCount before the SetValue on Name (or the action).
    [Test]
    procedure ListBoth_Action_ReadsControlState_BeforeFirstStep()
    begin
        Initialize(1, 1);
        RunReadsBoth(true, 1, 2);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);P.AGR(A);P.AGCR(A);P.AGCR();P.AGR(C);P.AGR(C);P.AGCR(C);');
        Expect('reads', 'Yes/Yes/Yes/Yes/No/No/No/No/0/0/0:');
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

    // The cell :1173 with AssertEquals, Caption and Editable between it and the SetValue on Value.
    [Test]
    procedure ListBoth_Action_ReadsCaptionsAndValues_BetweenSteps()
    begin
        Initialize(1, 1);
        RunReadsBoth(true, 2, 1);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);P.AGR(A);P.AGCR(A);P.AGCR();P.AGR(C);P.AGR(C);P.AGCR(C);');
        Expect('reads', 'Name/Value/Yes/Yes/LT RFU Lines Both/Yes:');
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

    // The cell :1173 with Enabled, Visible, HideValue, ShowMandatory and ValidationErrorCount between it and the SetValue on Value.
    [Test]
    procedure ListBoth_Action_ReadsControlState_BetweenSteps()
    begin
        Initialize(1, 1);
        RunReadsBoth(true, 2, 2);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);P.AGR(A);P.AGCR(A);P.AGCR();P.AGR(C);P.AGR(C);P.AGCR(C);');
        Expect('reads', 'Yes/Yes/Yes/Yes/No/No/No/No/0/0/0:');
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

    // The cell :1173 with AssertEquals, Caption and Editable after the SetValue on Value, before First.
    [Test]
    procedure ListBoth_Action_ReadsCaptionsAndValues_AfterSteps()
    begin
        Initialize(1, 1);
        RunReadsBoth(true, 3, 1);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);P.AGR(A);P.AGCR(A);P.AGCR();P.AGR(C);P.AGR(C);P.AGCR(C);');
        Expect('reads', 'Name/Value/Yes/Yes/LT RFU Lines Both/Yes:');
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

    // The cell :1173 with Enabled, Visible, HideValue, ShowMandatory and ValidationErrorCount after the SetValue on Value, before First.
    [Test]
    procedure ListBoth_Action_ReadsControlState_AfterSteps()
    begin
        Initialize(1, 1);
        RunReadsBoth(true, 3, 2);
        Expect('open', 'P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);P.AGR(A);P.AGCR(A);P.AGCR();P.AGR(C);P.AGR(C);P.AGCR(C);');
        Expect('reads', 'Yes/Yes/Yes/Yes/No/No/No/No/0/0/0:');
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

    // The cell :1190 with AssertEquals, Caption and Editable before the SetValue on Name (or the action).
    [Test]
    procedure ListCurr_ReadsCaptionsAndValues_BeforeFirstStep()
    begin
        Initialize(1, 1);
        RunReadsCurr(1, 1);
        Expect('open', 'P.AGCR(A);P.AGCR();P.AGCR(C);');
        Expect('reads', 'Name/Value/Yes/Yes/LT RFU Lines Curr/Yes:');
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

    // The cell :1190 with Enabled, Visible, HideValue, ShowMandatory and ValidationErrorCount before the SetValue on Name (or the action).
    [Test]
    procedure ListCurr_ReadsControlState_BeforeFirstStep()
    begin
        Initialize(1, 1);
        RunReadsCurr(1, 2);
        Expect('open', 'P.AGCR(A);P.AGCR();P.AGCR(C);');
        Expect('reads', 'Yes/Yes/Yes/Yes/No/No/No/No/0/0/0:');
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

    // The cell :1190 with AssertEquals, Caption and Editable between it and the SetValue on Value.
    [Test]
    procedure ListCurr_ReadsCaptionsAndValues_BetweenSteps()
    begin
        Initialize(1, 1);
        RunReadsCurr(2, 1);
        Expect('open', 'P.AGCR(A);P.AGCR();P.AGCR(C);');
        Expect('reads', 'Name/Value/Yes/Yes/LT RFU Lines Curr/Yes:');
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

    // The cell :1190 with Enabled, Visible, HideValue, ShowMandatory and ValidationErrorCount between it and the SetValue on Value.
    [Test]
    procedure ListCurr_ReadsControlState_BetweenSteps()
    begin
        Initialize(1, 1);
        RunReadsCurr(2, 2);
        Expect('open', 'P.AGCR(A);P.AGCR();P.AGCR(C);');
        Expect('reads', 'Yes/Yes/Yes/Yes/No/No/No/No/0/0/0:');
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

    // The cell :1190 with AssertEquals, Caption and Editable after the SetValue on Value, before First.
    [Test]
    procedure ListCurr_ReadsCaptionsAndValues_AfterSteps()
    begin
        Initialize(1, 1);
        RunReadsCurr(3, 1);
        Expect('open', 'P.AGCR(A);P.AGCR();P.AGCR(C);');
        Expect('reads', 'Name/Value/Yes/Yes/LT RFU Lines Curr/Yes:');
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

    // The cell :1190 with Enabled, Visible, HideValue, ShowMandatory and ValidationErrorCount after the SetValue on Value, before First.
    [Test]
    procedure ListCurr_ReadsControlState_AfterSteps()
    begin
        Initialize(1, 1);
        RunReadsCurr(3, 2);
        Expect('open', 'P.AGCR(A);P.AGCR();P.AGCR(C);');
        Expect('reads', 'Yes/Yes/Yes/Yes/No/No/No/No/0/0/0:');
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

    // The cell :1258 with AssertEquals, Caption and Editable before the SetValue on Name (or the action).
    [Test]
    procedure Card_ReadsCaptionsAndValues_BeforeFirstStep()
    begin
        Initialize(1, 1);
        RunReadsCard(false, 1, 1);
        Expect('open', 'P.AGR(A);P.AGR(C);');
        Expect('reads', 'Name/Value/Yes/Yes/LT RFU Card - G1 ∙ 3/Yes:');
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

    // The cell :1258 with Enabled, Visible, HideValue, ShowMandatory and ValidationErrorCount before the SetValue on Name (or the action).
    [Test]
    procedure Card_ReadsControlState_BeforeFirstStep()
    begin
        Initialize(1, 1);
        RunReadsCard(false, 1, 2);
        Expect('open', 'P.AGR(A);P.AGR(C);');
        Expect('reads', 'Yes/Yes/Yes/Yes/No/No/No/No/0/0/0:');
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

    // The cell :1258 with AssertEquals, Caption and Editable between it and the SetValue on Value.
    [Test]
    procedure Card_ReadsCaptionsAndValues_BetweenSteps()
    begin
        Initialize(1, 1);
        RunReadsCard(false, 2, 1);
        Expect('open', 'P.AGR(A);P.AGR(C);');
        Expect('reads', 'Name/Value/Yes/Yes/LT RFU Card - G1 ∙ 3/Yes:');
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

    // The cell :1258 with Enabled, Visible, HideValue, ShowMandatory and ValidationErrorCount between it and the SetValue on Value.
    [Test]
    procedure Card_ReadsControlState_BetweenSteps()
    begin
        Initialize(1, 1);
        RunReadsCard(false, 2, 2);
        Expect('open', 'P.AGR(A);P.AGR(C);');
        Expect('reads', 'Yes/Yes/Yes/Yes/No/No/No/No/0/0/0:');
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

    // The cell :1258 with AssertEquals, Caption and Editable after the SetValue on Value, before First.
    [Test]
    procedure Card_ReadsCaptionsAndValues_AfterSteps()
    begin
        Initialize(1, 1);
        RunReadsCard(false, 3, 1);
        Expect('open', 'P.AGR(A);P.AGR(C);');
        Expect('reads', 'Name/Value/Yes/Yes/LT RFU Card - G1 ∙ 3/Yes:');
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

    // The cell :1258 with Enabled, Visible, HideValue, ShowMandatory and ValidationErrorCount after the SetValue on Value, before First.
    [Test]
    procedure Card_ReadsControlState_AfterSteps()
    begin
        Initialize(1, 1);
        RunReadsCard(false, 3, 2);
        Expect('open', 'P.AGR(A);P.AGR(C);');
        Expect('reads', 'Yes/Yes/Yes/Yes/No/No/No/No/0/0/0:');
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

    // The cell :1309 with AssertEquals, Caption and Editable before the SetValue on Name (or the action).
    [Test]
    procedure Card_Action_ReadsCaptionsAndValues_BeforeFirstStep()
    begin
        Initialize(1, 1);
        RunReadsCard(true, 1, 1);
        Expect('open', 'P.AGR(A);P.AGR(C);');
        Expect('reads', 'Name/Value/Yes/Yes/LT RFU Card - G1 ∙ 3/Yes:');
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

    // The cell :1309 with Enabled, Visible, HideValue, ShowMandatory and ValidationErrorCount before the SetValue on Name (or the action).
    [Test]
    procedure Card_Action_ReadsControlState_BeforeFirstStep()
    begin
        Initialize(1, 1);
        RunReadsCard(true, 1, 2);
        Expect('open', 'P.AGR(A);P.AGR(C);');
        Expect('reads', 'Yes/Yes/Yes/Yes/No/No/No/No/0/0/0:');
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

    // The cell :1309 with AssertEquals, Caption and Editable between it and the SetValue on Value.
    [Test]
    procedure Card_Action_ReadsCaptionsAndValues_BetweenSteps()
    begin
        Initialize(1, 1);
        RunReadsCard(true, 2, 1);
        Expect('open', 'P.AGR(A);P.AGR(C);');
        Expect('reads', 'Name/Value/Yes/Yes/LT RFU Card - G1 ∙ 3/Yes:');
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

    // The cell :1309 with Enabled, Visible, HideValue, ShowMandatory and ValidationErrorCount between it and the SetValue on Value.
    [Test]
    procedure Card_Action_ReadsControlState_BetweenSteps()
    begin
        Initialize(1, 1);
        RunReadsCard(true, 2, 2);
        Expect('open', 'P.AGR(A);P.AGR(C);');
        Expect('reads', 'Yes/Yes/Yes/Yes/No/No/No/No/0/0/0:');
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

    // The cell :1309 with AssertEquals, Caption and Editable after the SetValue on Value, before First.
    [Test]
    procedure Card_Action_ReadsCaptionsAndValues_AfterSteps()
    begin
        Initialize(1, 1);
        RunReadsCard(true, 3, 1);
        Expect('open', 'P.AGR(A);P.AGR(C);');
        Expect('reads', 'Name/Value/Yes/Yes/LT RFU Card - G1 ∙ 3/Yes:');
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

    // The cell :1309 with Enabled, Visible, HideValue, ShowMandatory and ValidationErrorCount after the SetValue on Value, before First.
    [Test]
    procedure Card_Action_ReadsControlState_AfterSteps()
    begin
        Initialize(1, 1);
        RunReadsCard(true, 3, 2);
        Expect('open', 'P.AGR(A);P.AGR(C);');
        Expect('reads', 'Yes/Yes/Yes/Yes/No/No/No/No/0/0/0:');
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

// The Card page of al-runner issue 8717, line for line: its Name control's OnValidate calls CurrPage.Update(false) without a
// change of view, OnAfterGetRecord logs the name it reads; over a table of its own with one row.
table 97570 "LT RFU Edit Row"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Name; Text[30]) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

codeunit 97573 "LT RFU Edit Log"
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

page 97571 "LT RFU Edit Card"
{
    PageType = Card;
    SourceTable = "LT RFU Edit Row";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            group(General)
            {
                field("No."; Rec."No.")
                {
                    ApplicationArea = All;
                }
                field(Name; Rec.Name)
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
        Log.Add('AGR(' + Rec.Name + ')');
    end;

    var
        Log: Codeunit "LT RFU Edit Log";
}

codeunit 97572 "LT RFU Edit Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        Log: Codeunit "LT RFU Edit Log";

    local procedure Initialize()
    var
        Row: Record "LT RFU Edit Row";
    begin
        Row.DeleteAll();
        Row."No." := 'R1';
        Row.Name := 'Alpha';
        Row.Insert();
        Commit();
        Log.Take();
    end;

    // Opened on its only row: SetValue on Name, whose OnValidate calls CurrPage.Update(false), then Close.
    [Test]
    procedure EditCard_UpdateFalseInOnValidate_ReadsTheStoredRowAgain()
    var
        Row: Record "LT RFU Edit Row";
        Card: TestPage "LT RFU Edit Card";
    begin
        Initialize();
        Card.OpenEdit();
        Card.GoToKey('R1');
        Assert.AreEqual('AGR(Alpha);AGR(Alpha);', Log.Take(), 'open');
        Card.Name.SetValue('Beta');
        Assert.AreEqual('AGR(Alpha);', Log.Take(), 'SetValue on Name');
        Assert.AreEqual('Alpha', Card.Name.Value(), 'shown after the SetValue');
        Card.Close();
        Assert.AreEqual('', Log.Take(), 'close');
        Row.Get('R1');
        Assert.AreEqual('Alpha', Row.Name, 'stored after Close');
    end;

    // The test's read-only calls at a point of the cell :1632, in the two blocks of ReadsLines.
    local procedure ReadsEditCard(var Card: TestPage "LT RFU Edit Card"; Block: Integer): Text
    var
        Result: Text;
    begin
        if Block = 1 then begin
            Card."No.".AssertEquals('R1');
            Card.Name.AssertEquals('Alpha');
            Result := Card."No.".Caption() + '/' + Card.Name.Caption() + '/' + Format(Card."No.".Editable()) + '/' + Format(Card.Name.Editable()) + '/' +
              Card.Caption() + '/' + Format(Card.Editable());
        end else
            Result := Format(Card."No.".Enabled()) + '/' + Format(Card.Name.Enabled()) + '/' + Format(Card."No.".Visible()) + '/' + Format(Card.Name.Visible()) + '/' +
              Format(Card."No.".HideValue()) + '/' + Format(Card.Name.HideValue()) + '/' + Format(Card."No.".ShowMandatory()) + '/' +
              Format(Card.Name.ShowMandatory()) + '/' + Format(Card."No.".ValidationErrorCount()) + '/' + Format(Card.Name.ValidationErrorCount()) + '/' +
              Format(Card.ValidationErrorCount());
        exit(Result + ':' + Log.Take());
    end;

    // The cell :1632 with AssertEquals, Caption and Editable after GoToKey.
    [Test]
    procedure EditCard_ReadsCaptionsAndValues_AfterGoToKey()
    var
        Row: Record "LT RFU Edit Row";
        Card: TestPage "LT RFU Edit Card";
        Reads: Text;
    begin
        Initialize();
        Card.OpenEdit();
        Card.GoToKey('R1');
        Assert.AreEqual('AGR(Alpha);AGR(Alpha);', Log.Take(), 'open');
        Reads := ReadsEditCard(Card, 1);
        Card.Name.SetValue('Beta');
        Assert.AreEqual('AGR(Alpha);', Log.Take(), 'SetValue on Name');
        Assert.AreEqual('Alpha', Card.Name.Value(), 'shown after the SetValue');
        Card.Close();
        Assert.AreEqual('', Log.Take(), 'close');
        Row.Get('R1');
        Assert.AreEqual('Alpha', Row.Name, 'stored after Close');
        Assert.AreEqual('No./Name/Yes/Yes/LT RFU Edit Card - R1/Yes:', Reads, 'reads');
    end;

    // The cell :1632 with Enabled, Visible, HideValue, ShowMandatory and ValidationErrorCount after GoToKey.
    [Test]
    procedure EditCard_ReadsControlState_AfterGoToKey()
    var
        Row: Record "LT RFU Edit Row";
        Card: TestPage "LT RFU Edit Card";
        Reads: Text;
    begin
        Initialize();
        Card.OpenEdit();
        Card.GoToKey('R1');
        Assert.AreEqual('AGR(Alpha);AGR(Alpha);', Log.Take(), 'open');
        Reads := ReadsEditCard(Card, 2);
        Card.Name.SetValue('Beta');
        Assert.AreEqual('AGR(Alpha);', Log.Take(), 'SetValue on Name');
        Assert.AreEqual('Alpha', Card.Name.Value(), 'shown after the SetValue');
        Card.Close();
        Assert.AreEqual('', Log.Take(), 'close');
        Row.Get('R1');
        Assert.AreEqual('Alpha', Row.Name, 'stored after Close');
        Assert.AreEqual('Yes/Yes/Yes/Yes/No/No/No/No/0/0/0:', Reads, 'reads');
    end;

    // The cell :1632 with AssertEquals, Caption and Editable after the SetValue, before Close.
    [Test]
    procedure EditCard_ReadsCaptionsAndValues_AfterSetValue()
    var
        Row: Record "LT RFU Edit Row";
        Card: TestPage "LT RFU Edit Card";
        Reads: Text;
    begin
        Initialize();
        Card.OpenEdit();
        Card.GoToKey('R1');
        Assert.AreEqual('AGR(Alpha);AGR(Alpha);', Log.Take(), 'open');
        Card.Name.SetValue('Beta');
        Assert.AreEqual('AGR(Alpha);', Log.Take(), 'SetValue on Name');
        Assert.AreEqual('Alpha', Card.Name.Value(), 'shown after the SetValue');
        Reads := ReadsEditCard(Card, 1);
        Card.Close();
        Assert.AreEqual('', Log.Take(), 'close');
        Row.Get('R1');
        Assert.AreEqual('Alpha', Row.Name, 'stored after Close');
        Assert.AreEqual('No./Name/Yes/Yes/LT RFU Edit Card - R1/Yes:', Reads, 'reads');
    end;

    // The cell :1632 with Enabled, Visible, HideValue, ShowMandatory and ValidationErrorCount after the SetValue, before Close.
    [Test]
    procedure EditCard_ReadsControlState_AfterSetValue()
    var
        Row: Record "LT RFU Edit Row";
        Card: TestPage "LT RFU Edit Card";
        Reads: Text;
    begin
        Initialize();
        Card.OpenEdit();
        Card.GoToKey('R1');
        Assert.AreEqual('AGR(Alpha);AGR(Alpha);', Log.Take(), 'open');
        Card.Name.SetValue('Beta');
        Assert.AreEqual('AGR(Alpha);', Log.Take(), 'SetValue on Name');
        Assert.AreEqual('Alpha', Card.Name.Value(), 'shown after the SetValue');
        Reads := ReadsEditCard(Card, 2);
        Card.Close();
        Assert.AreEqual('', Log.Take(), 'close');
        Row.Get('R1');
        Assert.AreEqual('Alpha', Row.Name, 'stored after Close');
        Assert.AreEqual('Yes/Yes/Yes/Yes/No/No/No/No/0/0/0:', Reads, 'reads');
    end;
}
