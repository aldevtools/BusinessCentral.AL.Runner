// A copy of the corpus probe BgTaskCacheEdges.al of corpus pull request 1558 at c8c3bc81 (corpus issue 1555), run here until a
// bump pins it. Citations of BgTaskCacheEdges.al name the corpus file's lines (this copy has two lines more).
// BC Documentation:
//   https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/testpage/testpagetestpage-runpagebackgroundtask-method
//   https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/record/record-calcfields-method
//   https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/database/database-selectlatestversion-method
// Scope: in-scope
// local object: what a page background task's worker's repeated reads answer after the test session changed the rows they
//   read without a commit, in shapes past the cells of BgTaskQueryCache.al and BgTaskCalcCache.al (issue 1555)
// Fixtures used: shared Assert (60021); own tables 97706 to 97708, page 97706, enum 97706, worker codeunit 97708, test
//   codeunit 97709
// Probe (issue 1555, al-runner PR 8872 and 8885 reviews): each cell commits a parent of table 97706 (description 'first')
//   with line 1 of table 97707 (Kind A, Amount 5, Qty 1) and opens page 97706; the worker reads; the test session changes
//   the rows without a commit and reads the table it wrote; the worker reads again: 'first / again'. The changes: lines 0
//   (Kind K, Amount 2) and 3 (Kind B, Amount 11), a note of table 97708, the parent's description, or another parent
//   committed and then the two lines. In the TestFirst cells the test session reads first, then inserts the lines, and the
//   worker reads once, in the other sort direction.

table 97706 "BCE Parent"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Description; Text[50]) { }
        field(3; "Kind Text"; Code[100]) { }
        field(4; "Line Count"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("BCE Line" where(Parent = field("No.")));
        }
        field(5; "Note Count"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("BCE Note" where(Parent = field("No.")));
        }
        field(6; "Kind Text Count"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("BCE Line" where(Parent = field("No."), Kind = field(filter("Kind Text"))));
        }
        field(7; "Lines At Kind Text Count"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("BCE Line" where(Parent = field("No."), "Line No." = field("Kind Text Count")));
        }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

table 97707 "BCE Line"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; Parent; Code[20]) { }
        field(2; "Line No."; Integer) { }
        field(3; Kind; Code[10]) { }
        field(4; Amount; Decimal) { }
        field(5; Qty; Decimal) { }
    }

    keys
    {
        key(PK; Parent, "Line No.") { Clustered = true; }
    }
}

table 97708 "BCE Note"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; Parent; Code[20]) { }
        field(2; "Line No."; Integer) { }
    }

    keys
    {
        key(PK; Parent, "Line No.") { Clustered = true; }
    }
}

// The id of table 97706, for SelectLatestVersion with an Enum.
enum 97706 "BCE Table"
{
    Extensible = false;

    value(0; None) { }
    value(97706; Parent) { }
}

page 97706 "BCE Lines"
{
    PageType = List;
    SourceTable = "BCE Line";
    ApplicationArea = All;
    UsageCategory = Lists;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field(Parent; Rec.Parent) { }
                field("Line No."; Rec."Line No.") { }
            }
        }
    }
}

codeunit 97708 "BCE Reader"
{
    trigger OnRun()
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
    begin
        Parameters := Page.GetBackgroundParameters();
        Results.Add('Seen', Read(Parameters.Get('Mode'), Parameters.Get('Group')));
        Page.SetBackgroundTaskResult(Results);
    end;

    local procedure D(Value: Decimal): Text
    begin
        exit(Format(Value, 0, 9));
    end;

    local procedure Read(Mode: Text; Group: Text): Text
    var
        Parent: Record "BCE Parent";
        Temp: Record "BCE Parent" temporary;
        Line: Record "BCE Line";
        Ref: RecordRef;
        FRef: FieldRef;
        Which: Enum "BCE Table";
    begin
        case Mode of
            'linecount', 'descendingcalc':
                begin
                    if Mode = 'descendingcalc' then
                        Parent.SetAscending("No.", false);
                    Parent.Get(Group);
                    Parent.CalcFields("Line Count");
                    exit(Format(Parent."Line Count"));
                end;
            'sums', 'descendingsums':
                begin
                    if Mode = 'descendingsums' then
                        Line.SetAscending("Line No.", false);
                    Line.SetRange(Parent, Group);
                    Line.CalcSums(Amount);
                    exit(D(Line.Amount));
                end;
            'count', 'countdescending':
                begin
                    Line.SetRange(Parent, Group);
                    if Mode = 'countdescending' then
                        Line.Ascending(false);
                    exit(Format(Line.Count()));
                end;
            'innercount':
                begin
                    Parent.Get(Group);
                    Parent."Kind Text" := 'A|K';
                    Parent.CalcFields("Lines At Kind Text Count");
                    exit(Format(Parent."Lines At Kind Text Count"));
                end;
            'sumsotherfield', 'sumstwofilters':
                begin
                    Line.SetRange(Parent, Group);
                    Line.SetFilter(Qty, '>0');
                    if Mode = 'sumstwofilters' then
                        Line.SetFilter("Line No.", '>0');
                    Line.CalcSums(Amount);
                    exit(D(Line.Amount));
                end;
            'temporaryrecordref':
                begin
                    Ref.Open(Database::"BCE Parent", true);
                    Ref.Field(1).Value := Group;
                    Ref.Insert();
                    FRef := Ref.Field(4);
                    FRef.CalcField();
                    exit(Format(FRef.Value()));
                end;
            'temporary':
                begin
                    Parent.Get(Group);
                    Temp := Parent;
                    Temp.Insert();
                    Temp.CalcFields("Line Count");
                    exit(Format(Temp."Line Count"));
                end;
            'description', 'enumselect':
                begin
                    if Mode = 'enumselect' then begin
                        Which := Which::Parent;
                        // An Enum where the Integer table number goes (warning AL0603 in the compiler).
                        Database.SelectLatestVersion(Which);
                    end;
                    Parent.Get(Group);
                    exit(Parent.Description);
                end;
            'linesnotes':
                begin
                    Parent.SetRange("No.", Group);
                    Parent.SetFilter("Line Count", '>0');
                    Parent.SetRange("Note Count", 0);
                    exit(Format(Parent.FindFirst()));
                end;
            'noteslines':
                begin
                    Parent.SetRange("No.", Group);
                    Parent.SetRange("Note Count", 0);
                    Parent.SetFilter("Line Count", '>0');
                    exit(Format(Parent.FindFirst()));
                end;
        end;
        Error('Mode %1', Mode);
    end;
}

codeunit 97709 "BCE Tests"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;

    local procedure Run(var Lines: TestPage "BCE Lines"; Mode: Text; Group: Code[20]): Text
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
    begin
        Parameters.Add('Group', Group);
        Parameters.Add('Mode', Mode);
        Results := Lines.RunPageBackgroundTask(Codeunit::"BCE Reader", Parameters, false);
        Results.Get('Seen', Seen);
        exit(Seen);
    end;

    // Inserts a line, then reads the parent's lines.
    local procedure AddLine(Parent: Code[20]; LineNo: Integer; Kind: Code[10]; Amount: Decimal)
    var
        Line: Record "BCE Line";
    begin
        Line.Parent := Parent;
        Line."Line No." := LineNo;
        Line.Kind := Kind;
        Line.Amount := Amount;
        Line.Qty := 1;
        Line.Insert();
        Line.SetRange(Parent, Parent);
        if Line.FindFirst() then;
    end;

    local procedure NewParent(): Code[20]
    var
        Parent: Record "BCE Parent";
    begin
        Parent."No." := CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 18);
        Parent.Description := 'first';
        Parent.Insert();
        exit(Parent."No.");
    end;

    local procedure Change(G: Code[20]; What: Text)
    var
        Parent: Record "BCE Parent";
        Note: Record "BCE Note";
    begin
        case What of
            'lines':
                begin
                    AddLine(G, 0, 'K', 2);
                    AddLine(G, 3, 'B', 11);
                end;
            'note':
                begin
                    Note.Parent := G;
                    Note."Line No." := 1;
                    Note.Insert();
                    Note.SetRange(Parent, G);
                    if Note.FindFirst() then;
                end;
            'description':
                begin
                    Parent.Get(G);
                    Parent.Description := 'second';
                    Parent.Modify();
                end;
            'parentcommit':
                begin
                    NewParent();
                    Commit();
                    AddLine(G, 0, 'K', 2);
                    AddLine(G, 3, 'B', 11);
                end;
            'rowcommit':
                begin
                    Parent.Get(G);
                    Parent.Description := 'second';
                    Parent.Modify();
                    Commit();
                    AddLine(G, 0, 'K', 2);
                    AddLine(G, 3, 'B', 11);
                end;
        end;
    end;

    // The committed parent with line 1; the worker (First); the change without a commit; the worker again (Again).
    local procedure Reads(First: Text; Again: Text; What: Text): Text
    var
        Lines: TestPage "BCE Lines";
        G: Code[20];
        Seen: Text;
    begin
        G := NewParent();
        AddLine(G, 1, 'A', 5);
        Commit();
        Lines.OpenView();
        Seen := Run(Lines, First, G);
        Change(G, What);
        Seen += ' / ' + Run(Lines, Again, G);
        Lines.Close();
        exit(Seen);
    end;

    // The committed parent with line 1; the test session's read ('count': Count of the lines; 'calcfields': CalcFields of the
    // parent's Line Count), ascending; lines 0 and 3 without a commit; the worker's read (Worker), descending.
    local procedure TestFirst(TestRead: Text; Worker: Text): Text
    var
        Lines: TestPage "BCE Lines";
        Parent: Record "BCE Parent";
        Line: Record "BCE Line";
        G: Code[20];
        Seen: Text;
    begin
        G := NewParent();
        AddLine(G, 1, 'A', 5);
        Commit();
        Lines.OpenView();
        case TestRead of
            'count':
                begin
                    Line.SetRange(Parent, G);
                    Seen := Format(Line.Count());
                end;
            'calcfields':
                begin
                    Parent.Get(G);
                    Parent.CalcFields("Line Count");
                    Seen := Format(Parent."Line Count");
                end;
        end;
        Change(G, 'lines');
        Seen += ' / ' + Run(Lines, Worker, G);
        Lines.Close();
        exit(Seen);
    end;

    [Test]
    procedure BgCacheEdges_CalcFieldsDescendingThenAscending_FirstAnswer()
    begin
        Assert.AreEqual('1 / 1', Reads('descendingcalc', 'linecount', 'lines'), 'CalcFields descending, then ascending');
    end;

    [Test]
    procedure BgCacheEdges_CalcSumsDescendingThenAscending_FirstAnswer()
    begin
        Assert.AreEqual('5 / 5', Reads('descendingsums', 'sums', 'lines'), 'CalcSums descending, then ascending');
    end;

    [Test]
    procedure BgCacheEdges_CountDescendingThenAscending_FirstAnswer()
    begin
        Assert.AreEqual('1 / 1', Reads('countdescending', 'count', 'lines'), 'Count descending, then ascending');
    end;

    [Test]
    procedure BgCacheEdges_TestCountFirst_WorkerDescending_TestsAnswer()
    begin
        Assert.AreEqual('1 / 1', TestFirst('count', 'countdescending'), 'the test session''s Count, then the worker''s descending');
    end;

    [Test]
    procedure BgCacheEdges_TestCalcFieldsFirst_WorkerDescending_TestsAnswer()
    begin
        Assert.AreEqual('1 / 1', TestFirst('calcfields', 'descendingcalc'), 'the test session''s CalcFields, then the worker''s descending');
    end;

    [Test]
    procedure BgCacheEdges_CountAtAnInnerCountWithAFieldFilter_FirstAnswer()
    begin
        Assert.AreEqual('1 / 1', Reads('innercount', 'innercount', 'lines'), 'field() of a count whose field(filter()) reads Kind Text');
    end;

    [Test]
    procedure BgCacheEdges_CalcSumsGreaterThanOnAnotherField_FirstAnswer()
    begin
        Assert.AreEqual('5 / 5', Reads('sumsotherfield', 'sumsotherfield', 'lines'), 'CalcSums of Amount with Qty > 0');
    end;

    [Test]
    procedure BgCacheEdges_CalcSumsTwoGreaterThanFilters_FirstAnswer()
    begin
        Assert.AreEqual('5 / 5', Reads('sumstwofilters', 'sumstwofilters', 'lines'), 'CalcSums of Amount with Qty > 0 and Line No. > 0');
    end;

    [Test]
    procedure BgCacheEdges_TemporaryRecordRefCalcField_FirstAnswer()
    begin
        Assert.AreEqual('1 / 1', Reads('temporaryrecordref', 'temporaryrecordref', 'lines'), 'FieldRef.CalcField of a temporary RecordRef');
    end;

    [Test]
    procedure BgCacheEdges_TemporaryRecordAfterACommitOfItsTable_FirstAnswer()
    begin
        Assert.AreEqual('1 / 1', Reads('temporary', 'temporary', 'parentcommit'), 'a temporary record after a commit of table 97706');
    end;

    [Test]
    procedure BgCacheEdges_CalcFieldsAfterACommitOfItsTable_FirstAnswer()
    begin
        Assert.AreEqual('1 / 1', Reads('linecount', 'linecount', 'parentcommit'), 'CalcFields after a commit of another row of table 97706');
    end;

    [Test]
    procedure BgCacheEdges_CalcFieldsAfterACommitOfItsRow_FirstAnswer()
    begin
        Assert.AreEqual('1 / 1', Reads('linecount', 'linecount', 'rowcommit'), 'CalcFields after a commit of its row');
    end;

    [Test]
    procedure BgCacheEdges_TemporaryRecordAfterACommitOfItsRow_FirstAnswer()
    begin
        Assert.AreEqual('1 / 1', Reads('temporary', 'temporary', 'rowcommit'), 'a temporary record after a commit of the row of table 97706');
    end;

    [Test]
    procedure BgCacheEdges_SelectLatestVersionOfAnEnumNamingTheTable_ReadsTheTable()
    begin
        Assert.AreEqual('first / second', Reads('description', 'enumselect', 'description'), 'SelectLatestVersion(Enum of 97706)');
    end;

    [Test]
    procedure BgCacheEdges_TwoFlowFieldFiltersSameOrder_FirstAnswer()
    begin
        Assert.AreEqual('Yes / Yes', Reads('linesnotes', 'linesnotes', 'note'), 'Line Count, Note Count twice');
    end;

    [Test]
    procedure BgCacheEdges_TwoFlowFieldFiltersSwapped_FirstAnswer()
    begin
        Assert.AreEqual('Yes / Yes', Reads('linesnotes', 'noteslines', 'note'), 'Line Count, Note Count, then the other order');
    end;
}
