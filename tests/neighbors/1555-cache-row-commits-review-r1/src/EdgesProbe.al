// Copy of BgTaskCacheEdges.al at open corpus PR 1581, commit 852fb2b9 (38 cells); PR 1558 merged the first 33 cells.
// Citations name the corpus file's lines; this copy has two extra header lines.
// BC Documentation:
//   https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/testpage/testpagetestpage-runpagebackgroundtask-method
//   https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/record/record-calcfields-method
//   https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/database/database-selectlatestversion-method
// Scope: in-scope
// local object: what a page background task's worker's repeated reads answer after the test session changed the rows they
//   read without a commit, in shapes past the cells of BgTaskQueryCache.al and BgTaskCalcCache.al (issue 1555)
// Fixtures used: shared Assert (60021); own tables 97706 to 97712, page 97706, enums 97706 and 97707, and codeunits
//   97708 to 97715
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

// From the review of al-runner PR 8897: what ends a worker's CalcFields entry among commits of its row's table, and
// SelectLatestVersion with a value 0 of each type the compiler takes for the table number. Each cell commits a parent of table
// 97706 (description 'first') with line 1 and opens page 97706; the worker reads; the test session commits a ModifyAll, a
// DeleteAll or a Delete of another row of table 97706, then inserts lines 0 and 3 without a commit; the worker reads again.
codeunit 97710 "BCE Commit Reader"
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

    local procedure Read(Mode: Text; Group: Text): Text
    var
        Parent: Record "BCE Parent";
        Ref: RecordRef;
        FRef: FieldRef;
        Kind: Option First,Second;
        C: Char;
        B: Byte;
        Which: Enum "BCE Table";
    begin
        case Mode of
            'linecount':
                begin
                    Parent.Get(Group);
                    Parent.CalcFields("Line Count");
                    exit(Format(Parent."Line Count"));
                end;
            'recordref':
                begin
                    Ref.Open(Database::"BCE Parent");
                    Ref.Field(1).SetRange(Group);
                    Ref.FindFirst();
                    FRef := Ref.Field(4);
                    FRef.CalcField();
                    exit(Format(FRef.Value()));
                end;
            'description', 'optionselect', 'charselect', 'byteselect', 'enumselect':
                begin
                    // Each value is 0, which names no table (the Enum with warning AL0603).
                    case Mode of
                        'optionselect':
                            Database.SelectLatestVersion(Kind);
                        'charselect':
                            Database.SelectLatestVersion(C);
                        'byteselect':
                            Database.SelectLatestVersion(B);
                        'enumselect':
                            begin
                                Which := Which::None;
                                Database.SelectLatestVersion(Which);
                            end;
                    end;
                    Parent.Get(Group);
                    exit(Parent.Description);
                end;
        end;
        Error('Mode %1', Mode);
    end;
}

codeunit 97711 "BCE Commit Tests"
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
        Results := Lines.RunPageBackgroundTask(Codeunit::"BCE Commit Reader", Parameters, false);
        Results.Get('Seen', Seen);
        exit(Seen);
    end;

    local procedure AddLine(Parent: Code[20]; LineNo: Integer; Amount: Decimal)
    var
        Line: Record "BCE Line";
    begin
        Line.Parent := Parent;
        Line."Line No." := LineNo;
        Line.Kind := 'K';
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

    // What: a commit of table 97706 ('modifyall', 'deleteall' and 'delete' of another parent), then
    // lines 0 and 3 without a commit; or 'description', the parent's description changed without a commit.
    local procedure Change(G: Code[20]; What: Text)
    var
        Parent: Record "BCE Parent";
        Other: Code[20];
    begin
        if What = 'description' then begin
            Parent.Get(G);
            Parent.Description := 'second';
            Parent.Modify();
            exit;
        end;
        case What of
            'modifyall':
                begin
                    Other := NewParent();
                    Commit();
                    Parent.SetRange("No.", Other);
                    Parent.ModifyAll(Description, 'other');
                end;
            'deleteall':
                begin
                    Other := NewParent();
                    Commit();
                    Parent.SetRange("No.", Other);
                    Parent.DeleteAll();
                end;
            'delete':
                begin
                    Other := NewParent();
                    Commit();
                    Parent.Get(Other);
                    Parent.Delete();
                end;
        end;
        Commit();
        AddLine(G, 0, 2);
        AddLine(G, 3, 11);
    end;

    local procedure Reads(First: Text; Again: Text; What: Text): Text
    var
        Lines: TestPage "BCE Lines";
        G: Code[20];
        Seen: Text;
    begin
        G := NewParent();
        AddLine(G, 1, 5);
        Commit();
        Lines.OpenView();
        Seen := Run(Lines, First, G);
        Change(G, What);
        Seen += ' / ' + Run(Lines, Again, G);
        Lines.Close();
        exit(Seen);
    end;

    [Test]
    procedure BgCacheEdges_CalcFieldsAfterACommitOfAModifyAll_FirstAnswer()
    begin
        Assert.AreEqual('1 / 1', Reads('linecount', 'linecount', 'modifyall'), 'CalcFields after a commit of a ModifyAll of table 97706');
    end;

    [Test]
    procedure BgCacheEdges_CalcFieldsAfterACommitOfADeleteAll_FirstAnswer()
    begin
        Assert.AreEqual('1 / 1', Reads('linecount', 'linecount', 'deleteall'), 'CalcFields after a commit of a DeleteAll of table 97706');
    end;

    [Test]
    procedure BgCacheEdges_CalcFieldsAfterACommitOfADelete_FirstAnswer()
    begin
        Assert.AreEqual('1 / 1', Reads('linecount', 'linecount', 'delete'), 'CalcFields after a commit of a Delete of another row');
    end;

    [Test]
    procedure BgCacheEdges_TestCalcFieldsFirstThenACommit_TestsAnswer()
    var
        Lines: TestPage "BCE Lines";
        Parent: Record "BCE Parent";
        G: Code[20];
        Seen: Text;
    begin
        G := NewParent();
        AddLine(G, 1, 5);
        Commit();
        Lines.OpenView();
        Parent.Get(G);
        Parent.CalcFields("Line Count");
        Seen := Format(Parent."Line Count");
        NewParent();
        Commit();
        AddLine(G, 0, 2);
        AddLine(G, 3, 11);
        Seen += ' / ' + Run(Lines, 'linecount', G);
        Lines.Close();
        Assert.AreEqual('1 / 1', Seen, 'the test session''s CalcFields, a commit of table 97706, then the worker''s');
    end;

    [Test]
    procedure BgCacheEdges_RecordRefCalcFieldAfterACommitOfItsTable_FirstAnswer()
    begin
        Assert.AreEqual('1 / 1', Reads('recordref', 'recordref', 'delete'), 'FieldRef.CalcField of a database RecordRef after a commit of table 97706');
    end;

    [Test]
    procedure BgCacheEdges_SelectLatestVersionOfAnOptionZero_FirstAnswer()
    begin
        Assert.AreEqual('first / first', Reads('description', 'optionselect', 'description'), 'SelectLatestVersion(Option of 0)');
    end;

    [Test]
    procedure BgCacheEdges_SelectLatestVersionOfACharZero_FirstAnswer()
    begin
        Assert.AreEqual('first / first', Reads('description', 'charselect', 'description'), 'SelectLatestVersion(Char of 0)');
    end;

    [Test]
    procedure BgCacheEdges_SelectLatestVersionOfAByteZero_FirstAnswer()
    begin
        Assert.AreEqual('first / first', Reads('description', 'byteselect', 'description'), 'SelectLatestVersion(Byte of 0)');
    end;

    [Test]
    procedure BgCacheEdges_SelectLatestVersionOfAnEnumZero_FirstAnswer()
    begin
        Assert.AreEqual('first / first', Reads('description', 'enumselect', 'description'), 'SelectLatestVersion(Enum of 0)');
    end;
}

// From the review of al-runner PR 8897, the shapes it left without a cell: a commit of a ModifyAll, a DeleteAll or a Delete
// that reaches the row of a worker's CalcFields; a commit of a Truncate of the row's table (table 97709, which no other cell
// uses); and SelectLatestVersion with an Option, a Char, a Byte or an Enum of value 2, which names no table. Each cell
// commits a parent with line 1 and opens page 97706; the worker reads; the test session commits the change, then inserts
// lines 0 and 3 without a commit (or changes the parent's description without a commit); the worker reads again.
table 97709 "BCE Truncated Parent"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(4; "Line Count"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("BCE Line" where(Parent = field("No.")));
        }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

// Value 2, which names no table, for SelectLatestVersion with an Enum.
enum 97707 "BCE No Table"
{
    Extensible = false;

    value(2; Two) { }
}

codeunit 97712 "BCE Row Commit Reader"
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

    local procedure Read(Mode: Text; Group: Text): Text
    var
        Parent: Record "BCE Parent";
        Truncated: Record "BCE Truncated Parent";
        Kind: Option A,B,C;
        C: Char;
        B: Byte;
        Which: Enum "BCE No Table";
    begin
        case Mode of
            'linecount':
                begin
                    Parent.Get(Group);
                    Parent.CalcFields("Line Count");
                    exit(Format(Parent."Line Count"));
                end;
            // The key set without a Get, so the read works after the row is gone.
            'linecountkey':
                begin
                    Parent."No." := CopyStr(Group, 1, MaxStrLen(Parent."No."));
                    Parent.CalcFields("Line Count");
                    exit(Format(Parent."Line Count"));
                end;
            'truncatedkey':
                begin
                    Truncated."No." := CopyStr(Group, 1, MaxStrLen(Truncated."No."));
                    Truncated.CalcFields("Line Count");
                    exit(Format(Truncated."Line Count"));
                end;
            'description', 'optionselect', 'charselect', 'byteselect', 'enumselect':
                begin
                    // Each value is 2, which names no table (the Enum with warning AL0603).
                    case Mode of
                        'optionselect':
                            begin
                                Kind := Kind::C;
                                Database.SelectLatestVersion(Kind);
                            end;
                        'charselect':
                            begin
                                C := 2;
                                Database.SelectLatestVersion(C);
                            end;
                        'byteselect':
                            begin
                                B := 2;
                                Database.SelectLatestVersion(B);
                            end;
                        'enumselect':
                            begin
                                Which := Which::Two;
                                Database.SelectLatestVersion(Which);
                            end;
                    end;
                    Parent.Get(Group);
                    exit(Parent.Description);
                end;
        end;
        Error('Mode %1', Mode);
    end;
}

codeunit 97713 "BCE Row Commit Tests"
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
        Results := Lines.RunPageBackgroundTask(Codeunit::"BCE Row Commit Reader", Parameters, false);
        Results.Get('Seen', Seen);
        exit(Seen);
    end;

    local procedure AddLine(Parent: Code[20]; LineNo: Integer; Amount: Decimal)
    var
        Line: Record "BCE Line";
    begin
        Line.Parent := Parent;
        Line."Line No." := LineNo;
        Line.Kind := 'K';
        Line.Amount := Amount;
        Line.Qty := 1;
        Line.Insert();
        Line.SetRange(Parent, Parent);
        if Line.FindFirst() then;
    end;

    // What: 'modifyall', 'deleteall' or 'delete' of the parent itself, or 'truncate' of table 97709, committed, then lines 0 and
    // 3 without a commit; or 'description', the parent's description changed without a commit.
    local procedure Change(G: Code[20]; What: Text)
    var
        Parent: Record "BCE Parent";
        Truncated: Record "BCE Truncated Parent";
    begin
        if What = 'description' then begin
            Parent.Get(G);
            Parent.Description := 'second';
            Parent.Modify();
            exit;
        end;
        case What of
            'modifyall':
                begin
                    Parent.SetRange("No.", G);
                    Parent.ModifyAll(Description, 'changed');
                end;
            'deleteall':
                begin
                    Parent.SetRange("No.", G);
                    Parent.DeleteAll();
                end;
            'delete':
                begin
                    Parent.Get(G);
                    Parent.Delete();
                end;
            'truncate':
                Truncated.Truncate();
        end;
        Commit();
        AddLine(G, 0, 2);
        AddLine(G, 3, 11);
    end;

    local procedure Reads(First: Text; Again: Text; What: Text): Text
    var
        Lines: TestPage "BCE Lines";
        Parent: Record "BCE Parent";
        Truncated: Record "BCE Truncated Parent";
        G: Code[20];
        Seen: Text;
    begin
        G := CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 18);
        Parent."No." := G;
        Parent.Description := 'first';
        Parent.Insert();
        Truncated."No." := G;
        Truncated.Insert();
        AddLine(G, 1, 5);
        Commit();
        Lines.OpenView();
        Seen := Run(Lines, First, G);
        Change(G, What);
        Seen += ' / ' + Run(Lines, Again, G);
        Lines.Close();
        exit(Seen);
    end;

    [Test]
    procedure BgCacheEdges_CalcFieldsAfterACommitOfAModifyAllOfItsRow_FirstAnswer()
    begin
        Assert.AreEqual('1 / 1', Reads('linecount', 'linecount', 'modifyall'), 'CalcFields after a commit of a ModifyAll of its row');
    end;

    [Test]
    procedure BgCacheEdges_CalcFieldsAfterACommitOfADeleteAllOfItsRow_FirstAnswer()
    begin
        Assert.AreEqual('1 / 1', Reads('linecountkey', 'linecountkey', 'deleteall'), 'CalcFields after a commit of a DeleteAll of its row');
    end;

    [Test]
    procedure BgCacheEdges_CalcFieldsAfterACommitOfADeleteOfItsRow_FirstAnswer()
    begin
        Assert.AreEqual('1 / 1', Reads('linecountkey', 'linecountkey', 'delete'), 'CalcFields after a commit of a Delete of its row');
    end;

    [Test]
    procedure BgCacheEdges_SelectLatestVersionOfAnOptionTwo_FirstAnswer()
    begin
        Assert.AreEqual('first / first', Reads('description', 'optionselect', 'description'), 'SelectLatestVersion(Option of 2)');
    end;

    [Test]
    procedure BgCacheEdges_SelectLatestVersionOfACharTwo_FirstAnswer()
    begin
        Assert.AreEqual('first / first', Reads('description', 'charselect', 'description'), 'SelectLatestVersion(Char of 2)');
    end;

    [Test]
    procedure BgCacheEdges_SelectLatestVersionOfAByteTwo_FirstAnswer()
    begin
        Assert.AreEqual('first / first', Reads('description', 'byteselect', 'description'), 'SelectLatestVersion(Byte of 2)');
    end;

    [Test]
    procedure BgCacheEdges_SelectLatestVersionOfAnEnumTwo_FirstAnswer()
    begin
        Assert.AreEqual('first / first', Reads('description', 'enumselect', 'description'), 'SelectLatestVersion(Enum of 2)');
    end;

    // Last in this codeunit: a Truncate of table 97709, which no other cell reads.
    [Test]
    procedure BgCacheEdges_CalcFieldsAfterACommitOfATruncateOfItsTable_FirstAnswer()
    begin
        Assert.AreEqual('1 / 1', Reads('truncatedkey', 'truncatedkey', 'truncate'), 'CalcFields after a commit of a Truncate of table 97709');
    end;
}

// local objects: each Truncate cell needs its own table; a Truncate can leave its table locked for later cells.
// Review of al-runner PR 8932: cross the Truncate commit with a temporary Record, the test session's CalcFields first,
// and FieldRef.CalcField of a database RecordRef. Each calculation sets the primary key without Get, so it can calculate
// again after Truncate removed the row. The line table is separate and changes without a commit after the Truncate commit.
// Observed on local BC 29.0: all three answer '1 / 1'; SelectLatestVersion(Integer 0 or 2) answers 'first / first'.
table 97710 "BCE Truncate Temp"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;
    fields
    {
        field(1; "No."; Code[20]) { }
        field(4; "Line Count"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("BCE Line" where(Parent = field("No.")));
        }
    }
    keys { key(PK; "No.") { Clustered = true; } }
}

table 97713 "BCE Truncate Test First"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;
    fields
    {
        field(1; "No."; Code[20]) { }
        field(4; "Line Count"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("BCE Line" where(Parent = field("No.")));
        }
    }
    keys { key(PK; "No.") { Clustered = true; } }
}

table 97712 "BCE Truncate FieldRef"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;
    fields
    {
        field(1; "No."; Code[20]) { }
        field(4; "Line Count"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("BCE Line" where(Parent = field("No.")));
        }
    }
    keys { key(PK; "No.") { Clustered = true; } }
}

codeunit 97714 "BCE Truncate Reader"
{
    trigger OnRun()
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Temp: Record "BCE Truncate Temp" temporary;
        TestFirst: Record "BCE Truncate Test First";
        Parent: Record "BCE Parent";
        Ref: RecordRef;
        KeyField: FieldRef;
        CountField: FieldRef;
        Mode: Text;
        G: Code[20];
        Seen: Text;
    begin
        Parameters := Page.GetBackgroundParameters();
        Mode := Parameters.Get('Mode');
        G := CopyStr(Parameters.Get('Group'), 1, MaxStrLen(G));
        case Mode of
            'temp':
                begin
                    Temp."No." := G;
                    Temp.CalcFields("Line Count");
                    Seen := Format(Temp."Line Count");
                end;
            'testfirst':
                begin
                    TestFirst."No." := G;
                    TestFirst.CalcFields("Line Count");
                    Seen := Format(TestFirst."Line Count");
                end;
            'fieldref':
                begin
                    Ref.Open(Database::"BCE Truncate FieldRef");
                    KeyField := Ref.Field(1);
                    KeyField.Value := G;
                    CountField := Ref.Field(4);
                    CountField.CalcField();
                    Seen := Format(CountField.Value);
                    Ref.Close();
                end;
            'description', 'integerzero', 'integertwo':
                begin
                    case Mode of
                        'integerzero': Database.SelectLatestVersion(0);
                        'integertwo': Database.SelectLatestVersion(2);
                    end;
                    Parent.Get(G);
                    Seen := Parent.Description;
                end;
        end;
        Results.Add('Seen', Seen);
        Page.SetBackgroundTaskResult(Results);
    end;
}

codeunit 97715 "BCE Truncate Tests"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;

    local procedure Run(var Lines: TestPage "BCE Lines"; Mode: Text; G: Code[20]): Text
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
    begin
        Parameters.Add('Group', G);
        Parameters.Add('Mode', Mode);
        Results := Lines.RunPageBackgroundTask(Codeunit::"BCE Truncate Reader", Parameters, false);
        exit(Results.Get('Seen'));
    end;

    local procedure AddLine(G: Code[20]; LineNo: Integer)
    var
        Line: Record "BCE Line";
    begin
        Line.Parent := G;
        Line."Line No." := LineNo;
        Line.Kind := 'K';
        Line.Amount := 5;
        Line.Qty := 1;
        Line.Insert();
        Line.SetRange(Parent, G);
        if Line.FindFirst() then;
    end;

    local procedure Reads(Mode: Text): Text
    var
        Lines: TestPage "BCE Lines";
        TempTable: Record "BCE Truncate Temp";
        TestFirst: Record "BCE Truncate Test First";
        FieldRefTable: Record "BCE Truncate FieldRef";
        G: Code[20];
        Seen: Text;
    begin
        G := CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 18);
        case Mode of
            'temp':
                begin
                    TempTable."No." := G;
                    TempTable.Insert();
                end;
            'testfirst':
                begin
                    TestFirst."No." := G;
                    TestFirst.Insert();
                end;
            'fieldref':
                begin
                    FieldRefTable."No." := G;
                    FieldRefTable.Insert();
                end;
        end;
        AddLine(G, 1);
        Commit();
        Lines.OpenView();
        if Mode = 'testfirst' then begin
            TestFirst.CalcFields("Line Count");
            Seen := Format(TestFirst."Line Count");
        end else
            Seen := Run(Lines, Mode, G);
        case Mode of
            'temp': TempTable.Truncate();
            'testfirst': TestFirst.Truncate();
            'fieldref': FieldRefTable.Truncate();
        end;
        Commit();
        AddLine(G, 0);
        AddLine(G, 3);
        Seen += ' / ' + Run(Lines, Mode, G);
        Lines.Close();
        exit(Seen);
    end;

    local procedure Selects(Mode: Text): Text
    var
        Lines: TestPage "BCE Lines";
        Parent: Record "BCE Parent";
        G: Code[20];
        Seen: Text;
    begin
        G := CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 18);
        Parent."No." := G;
        Parent.Description := 'first';
        Parent.Insert();
        AddLine(G, 1);
        Commit();
        Lines.OpenView();
        Seen := Run(Lines, 'description', G);
        Parent.Description := 'second';
        Parent.Modify();
        Parent.Get(G);
        Seen += ' / ' + Run(Lines, Mode, G);
        Lines.Close();
        exit(Seen);
    end;

    [Test]
    procedure BgCacheEdges_Truncate_TemporaryCalcFields()
    begin
        Assert.AreEqual('1 / 1', Reads('temp'), 'Temporary CalcFields after a Truncate commit');
    end;

    [Test]
    procedure BgCacheEdges_Truncate_TestCalcFieldsFirst()
    begin
        Assert.AreEqual('1 / 1', Reads('testfirst'), 'Test CalcFields first, then a Truncate commit');
    end;

    [Test]
    procedure BgCacheEdges_Truncate_FieldRefCalcField()
    begin
        Assert.AreEqual('1 / 1', Reads('fieldref'), 'FieldRef.CalcField after a Truncate commit');
    end;

    [Test]
    procedure BgCacheEdges_SelectLatestVersion_IntegerZero()
    begin
        Assert.AreEqual('first / first', Selects('integerzero'), 'SelectLatestVersion(Integer 0), naming no table');
    end;

    [Test]
    procedure BgCacheEdges_SelectLatestVersion_IntegerTwo()
    begin
        Assert.AreEqual('first / first', Selects('integertwo'), 'SelectLatestVersion(Integer 2), naming no table');
    end;
}
