// An excerpt of the corpus probe BgTaskCalcCache.al of corpus pull request 1533 at fe7a324c (al-runner issue 8803): the file
// without its two Tenant Media cells (:486, :492) and the Media Id fields they read, because the runner's unit tests compile
// this file without the platform tables. Line numbers below are the corpus file's.
// BC Documentation:
//   https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/testpage/testpagetestpage-runpagebackgroundtask-method
//   https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/record/record-calcfields-method
//   https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/record/record-calcsums-method
// Scope: in-scope
// local object: what a page background task's worker's repeated CalcFields and CalcSums answer after the test session changed
//   the rows they read without a commit
// Fixtures used: shared Assert (60021); own tables 97485 to 97487, page 97485, worker codeunit 97489, test codeunit 97490
// Probe (al-runner issue 8803): each cell commits a parent of table 97485 with line 1 of table 97486 (Kind A, Amount 5) and opens
//   page 97485; the worker reads; the test session inserts line 0 (Kind K, Amount 2) and line 2 (Kind B, Amount 11), a note of
//   table 97487, or a Tenant Media row, without a commit, and reads the table it wrote (so no insert is held back); the worker
//   reads again: 'first / again'. Measured on a local BC 29.1 OnPrem w1 container, 3 runs. BC answers the first answer in these
//   shapes: a negated sum, FieldRef.CalcField of a RecordRef, a FlowFilter range, a filter on a field that is no FlowFilter, a
//   CalcSums with a > filter or with no filter, a count whose field() is a sum FlowField of the row, a temporary record, a sum whose
//   field(filter()) reads a stored field of the row (keyed by its text), an exist over Tenant Media, a count, min, exist or lookup
//   whose field() is a count FlowField of the row, and a CalcFields or CalcSums after SetAscending(false). The exist over Tenant
//   Media answers No both times, but a Get of the Tenant Media row the worker never read does not find the test session's
//   Insert(false) either: the worker does not see that row, so that cell tells nothing of a cache.
// From the review of al-runner PR 8872: a | of values on a field that is no FlowFilter, a CalcFields and a CalcSums ascending
//   first and after SetAscending(field, false) again, and a count whose field() is a sum FlowField of the row that reads a stored
//   field of the row by field(filter()) (Kind Text A|K, so the inner sum takes line 0): BC answers the first answer in each.

table 97485 "BCC Parent"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Description; Text[50]) { }
        field(3; "Kind Filter"; Code[10]) { FieldClass = FlowFilter; }
        field(4; "Line Count"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("BCC Line" where(Parent = field("No.")));
        }
        field(5; "Negative Sum"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = - sum("BCC Line".Amount where(Parent = field("No.")));
        }
        field(6; "Kind Sum"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = sum("BCC Line".Amount where(Parent = field("No."), Kind = field("Kind Filter")));
        }
        field(7; "Amount Total"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = sum("BCC Line".Amount where(Parent = field("No.")));
        }
        field(8; "Lines At Total"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("BCC Line" where(Parent = field("No."), Amount = field("Amount Total")));
        }
        field(9; "Kind Text"; Code[100]) { }
        field(10; "Kind Text Sum"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = sum("BCC Line".Amount where(Parent = field("No."), Kind = field(filter("Kind Text"))));
        }
        field(13; "Note Count"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("BCC Note" where(Parent = field("No.")));
        }
        field(14; "Count At Note Count"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("BCC Line" where(Parent = field("No."), "Line No." = field("Note Count")));
        }
        field(15; "Min At Note Count"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = min("BCC Line".Amount where(Parent = field("No."), "Line No." = field("Note Count")));
        }
        field(16; "Exist At Note Count"; Boolean)
        {
            FieldClass = FlowField;
            CalcFormula = exist("BCC Line" where(Parent = field("No."), "Line No." = field("Note Count")));
        }
        field(17; "Kind At Note Count"; Code[10])
        {
            FieldClass = FlowField;
            CalcFormula = lookup("BCC Line".Kind where(Parent = field("No."), "Line No." = field("Note Count")));
        }
        field(18; "Lines At Kind Text Sum"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("BCC Line" where(Parent = field("No."), Amount = field("Kind Text Sum")));
        }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

table 97486 "BCC Line"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; Parent; Code[20]) { }
        field(2; "Line No."; Integer) { }
        field(3; Kind; Code[10]) { }
        field(4; Amount; Decimal) { }
    }

    keys
    {
        key(PK; Parent, "Line No.") { Clustered = true; }
    }
}

table 97487 "BCC Note"
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

page 97485 "BCC Lines"
{
    PageType = List;
    SourceTable = "BCC Line";
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

codeunit 97489 "BCC Reader"
{
    trigger OnRun()
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
    begin
        Parameters := Page.GetBackgroundParameters();
        Results.Add('Seen', Calc(Parameters.Get('Mode'), Parameters.Get('Group'), Parameters.Get('Media')));
        Page.SetBackgroundTaskResult(Results);
    end;

    local procedure D(Value: Decimal): Text
    begin
        exit(Format(Value, 0, 9));
    end;

    local procedure Calc(Mode: Text; Group: Text; MediaText: Text): Text
    var
        Parent: Record "BCC Parent";
        Temp: Record "BCC Parent" temporary;
        Line: Record "BCC Line";
        Ref: RecordRef;
        FRef: FieldRef;
        Seen: Text;
    begin
        case Mode of
            'negative':
                begin
                    Parent.Get(Group);
                    Parent.CalcFields("Negative Sum");
                    exit(D(Parent."Negative Sum"));
                end;
            'recordref':
                begin
                    Ref.Open(Database::"BCC Parent");
                    Ref.Field(1).SetRange(Group);
                    Ref.FindFirst();
                    FRef := Ref.Field(4);
                    FRef.CalcField();
                    exit(Format(FRef.Value()));
                end;
            'flowfilterrange':
                begin
                    Parent.Get(Group);
                    Parent.SetFilter("Kind Filter", 'A..K');
                    Parent.CalcFields("Kind Sum");
                    exit(D(Parent."Kind Sum"));
                end;
            'otherfilter':
                begin
                    Parent.Get(Group);
                    Parent.SetRange(Description, '');
                    Parent.CalcFields("Line Count");
                    exit(Format(Parent."Line Count"));
                end;
            'calcsumsgt':
                begin
                    Line.SetRange(Parent, Group);
                    Line.SetFilter(Amount, '>0');
                    Line.CalcSums(Amount);
                    exit(D(Line.Amount));
                end;
            'calcsumsall':
                begin
                    Line.CalcSums(Amount);
                    exit(D(Line.Amount));
                end;
            'atsum':
                begin
                    Parent.Get(Group);
                    Parent.CalcFields("Lines At Total");
                    exit(Format(Parent."Lines At Total"));
                end;
            'temporary':
                begin
                    Parent.Get(Group);
                    Temp := Parent;
                    Temp.Insert();
                    Temp.CalcFields("Line Count");
                    exit(Format(Temp."Line Count"));
                end;
            'fieldfilter':
                begin
                    Parent.Get(Group);
                    Parent."Kind Text" := 'A';
                    Parent.CalcFields("Kind Text Sum");
                    Seen := D(Parent."Kind Text Sum");
                    Parent."Kind Text" := 'B';
                    Parent.CalcFields("Kind Text Sum");
                    exit(Seen + ',' + D(Parent."Kind Text Sum"));
                end;
            'outercount':
                begin
                    Parent.Get(Group);
                    Parent.CalcFields("Count At Note Count");
                    exit(Format(Parent."Count At Note Count"));
                end;
            'outermin':
                begin
                    Parent.Get(Group);
                    Parent.CalcFields("Min At Note Count");
                    exit(D(Parent."Min At Note Count"));
                end;
            'outerexist':
                begin
                    Parent.Get(Group);
                    Parent.CalcFields("Exist At Note Count");
                    exit(Format(Parent."Exist At Note Count"));
                end;
            'outerlookup':
                begin
                    Parent.Get(Group);
                    Parent.CalcFields("Kind At Note Count");
                    exit('[' + Parent."Kind At Note Count" + ']');
                end;
            'descendingcalc':
                begin
                    Parent.SetAscending("No.", false);
                    Parent.Get(Group);
                    Parent.CalcFields("Line Count");
                    exit(Format(Parent."Line Count"));
                end;
            'descendingsums':
                begin
                    Line.SetAscending("Line No.", false);
                    Line.SetRange(Parent, Group);
                    Line.CalcSums(Amount);
                    exit(D(Line.Amount));
                end;
            'otherfilteror':
                begin
                    Parent.Get(Group);
                    Parent.SetFilter(Description, '%1|%2', '', 'X');
                    Parent.CalcFields("Line Count");
                    exit(Format(Parent."Line Count"));
                end;
            'linecount':
                begin
                    Parent.Get(Group);
                    Parent.CalcFields("Line Count");
                    exit(Format(Parent."Line Count"));
                end;
            'sums':
                begin
                    Line.SetRange(Parent, Group);
                    Line.CalcSums(Amount);
                    exit(D(Line.Amount));
                end;
            'innerfieldfilter':
                begin
                    Parent.Get(Group);
                    Parent."Kind Text" := 'A|K';
                    Parent.CalcFields("Lines At Kind Text Sum");
                    exit(Format(Parent."Lines At Kind Text Sum"));
                end;
        end;
        Error('Mode %1', Mode);
    end;
}

codeunit 97490 "BCC Tests"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;

    local procedure Run(var Lines: TestPage "BCC Lines"; Mode: Text; Group: Code[20]; MediaId: Guid): Text
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
    begin
        Parameters.Add('Group', Group);
        Parameters.Add('Mode', Mode);
        Parameters.Add('Media', Format(MediaId));
        Results := Lines.RunPageBackgroundTask(Codeunit::"BCC Reader", Parameters, false);
        Results.Get('Seen', Seen);
        exit(Seen);
    end;

    // Inserts a line, then reads the parent's lines.
    local procedure AddLine(Parent: Code[20]; LineNo: Integer; Kind: Code[10]; Amount: Decimal)
    var
        Line: Record "BCC Line";
    begin
        Line.Parent := Parent;
        Line."Line No." := LineNo;
        Line.Kind := Kind;
        Line.Amount := Amount;
        Line.Insert();
        Line.SetRange(Parent, Parent);
        if Line.FindFirst() then;
    end;

    local procedure Reads(Mode: Text; Change: Text): Text
    begin
        exit(ReadsAcross(Mode, Mode, Change));
    end;

    // A parent with line 1 (Kind A, Amount 5) and a Media Id, committed; the worker (First); the change without a commit
    // ('lines': lines 0 and 2; 'note': a note; 'media': a Tenant Media row of the Media Id); the worker again (Again):
    // 'first / again'.
    local procedure ReadsAcross(First: Text; Again: Text; Change: Text): Text
    var
        Lines: TestPage "BCC Lines";
        Parent: Record "BCC Parent";
        Note: Record "BCC Note";
        G: Code[20];
        Seen: Text;
    begin
        G := CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 18);
        Parent."No." := G;
        Parent.Insert();
        AddLine(G, 1, 'A', 5);
        Commit();
        Lines.OpenView();
        Seen := Run(Lines, First, G, CreateGuid());
        case Change of
            'lines':
                begin
                    AddLine(G, 0, 'K', 2);
                    AddLine(G, 2, 'B', 11);
                end;
            'note':
                begin
                    Note.Parent := G;
                    Note."Line No." := 1;
                    Note.Insert();
                    Note.SetRange(Parent, G);
                    if Note.FindFirst() then;
                end;
        end;
        Seen += ' / ' + Run(Lines, Again, G, CreateGuid());
        Lines.Close();
        exit(Seen);
    end;

    [Test]
    procedure BgCalcCache_NegatedSum_FirstAnswer()
    begin
        Assert.AreEqual('-5 / -5', Reads('negative', 'lines'), 'a negated sum');
    end;

    [Test]
    procedure BgCalcCache_RecordRefCalcField_FirstAnswer()
    begin
        Assert.AreEqual('1 / 1', Reads('recordref', 'lines'), 'FieldRef.CalcField');
    end;

    [Test]
    procedure BgCalcCache_FlowFilterRange_FirstAnswer()
    begin
        Assert.AreEqual('5 / 5', Reads('flowfilterrange', 'lines'), 'a FlowFilter range');
    end;

    [Test]
    procedure BgCalcCache_FilterOnAStoredField_FirstAnswer()
    begin
        Assert.AreEqual('1 / 1', Reads('otherfilter', 'lines'), 'a filter on Description');
    end;

    [Test]
    procedure BgCalcCache_CalcSumsGreaterThan_FirstAnswer()
    begin
        Assert.AreEqual('5 / 5', Reads('calcsumsgt', 'lines'), 'CalcSums with a > filter');
    end;

    [Test]
    procedure BgCalcCache_CalcSumsWithoutAFilter_FirstAnswer()
    var
        Seen: Text;
        First: Decimal;
        Again: Decimal;
    begin
        Seen := Reads('calcsumsall', 'lines');
        Evaluate(First, SelectStr(1, ConvertStr(Seen, '/', ',')));
        Evaluate(Again, SelectStr(2, ConvertStr(Seen, '/', ',')));
        Assert.AreEqual('0', Format(Again - First, 0, 9), 'CalcSums of the whole table, again less first');
    end;

    [Test]
    procedure BgCalcCache_CountAtASumOfTheRow_FirstAnswer()
    begin
        Assert.AreEqual('1 / 1', Reads('atsum', 'lines'), 'field() of a sum FlowField of the row');
    end;

    [Test]
    procedure BgCalcCache_TemporaryRecord_FirstAnswer()
    begin
        Assert.AreEqual('1 / 1', Reads('temporary', 'lines'), 'a temporary record');
    end;

    [Test]
    procedure BgCalcCache_FieldFilterOfAStoredField_FirstAnswer()
    begin
        Assert.AreEqual('5,0 / 5,0', Reads('fieldfilter', 'lines'), 'field(filter()) of Kind Text, A then B');
    end;

    [Test]
    procedure BgCalcCache_CountAtACountOfTheRow_FirstAnswer()
    begin
        Assert.AreEqual('0 / 0', Reads('outercount', 'note'), 'a count at a count FlowField of the row');
    end;

    [Test]
    procedure BgCalcCache_MinAtACountOfTheRow_FirstAnswer()
    begin
        Assert.AreEqual('0 / 0', Reads('outermin', 'note'), 'a min at a count FlowField of the row');
    end;

    [Test]
    procedure BgCalcCache_ExistAtACountOfTheRow_FirstAnswer()
    begin
        Assert.AreEqual('No / No', Reads('outerexist', 'note'), 'an exist at a count FlowField of the row');
    end;

    [Test]
    procedure BgCalcCache_LookupAtACountOfTheRow_FirstAnswer()
    begin
        Assert.AreEqual('[] / []', Reads('outerlookup', 'note'), 'a lookup at a count FlowField of the row');
    end;

    [Test]
    procedure BgCalcCache_CalcFieldsAfterSetAscendingFalse_FirstAnswer()
    begin
        Assert.AreEqual('1 / 1', Reads('descendingcalc', 'lines'), 'CalcFields after SetAscending(false)');
    end;

    [Test]
    procedure BgCalcCache_CalcSumsAfterSetAscendingFalse_FirstAnswer()
    begin
        Assert.AreEqual('5 / 5', Reads('descendingsums', 'lines'), 'CalcSums after SetAscending(false)');
    end;

    [Test]
    procedure BgCalcCache_OrOfValuesOnAStoredField_FirstAnswer()
    begin
        Assert.AreEqual('1 / 1', Reads('otherfilteror', 'lines'), 'a | of values on Description');
    end;

    [Test]
    procedure BgCalcCache_CalcFieldsAscendingThenDescending_FirstAnswer()
    begin
        Assert.AreEqual('1 / 1', ReadsAcross('linecount', 'descendingcalc', 'lines'), 'CalcFields, then after SetAscending(false)');
    end;

    [Test]
    procedure BgCalcCache_CalcSumsAscendingThenDescending_FirstAnswer()
    begin
        Assert.AreEqual('5 / 5', ReadsAcross('sums', 'descendingsums', 'lines'), 'CalcSums, then after SetAscending(false)');
    end;

    [Test]
    procedure BgCalcCache_CountAtASumWithAFieldFilterOfTheRow_FirstAnswer()
    begin
        Assert.AreEqual('1 / 1', Reads('innerfieldfilter', 'lines'), 'field() of a sum whose field(filter()) reads Kind Text');
    end;
}

// The Count of a group, read ascending and then descending (review of al-runner PR 8808): a worker's Count of the lines of a
// committed parent with line 1, ascending; the test session inserts lines 0 and 2 without a commit and reads them; the worker's
// Count of the same lines with Ascending(false): BC answers the first answer, one entry for both sort directions (a local
// BC 29.1 container, 3 runs).
codeunit 97472 "BCC Count Reader"
{
    trigger OnRun()
    var
        Line: Record "BCC Line";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
    begin
        Parameters := Page.GetBackgroundParameters();
        Line.SetRange(Parent, Parameters.Get('Group'));
        if Parameters.Get('Mode') = 'descending' then
            Line.Ascending(false);
        Results.Add('Seen', Format(Line.Count()));
        Page.SetBackgroundTaskResult(Results);
    end;
}

codeunit 97473 "BCC Count Tests"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;

    local procedure Run(var Lines: TestPage "BCC Lines"; Mode: Text; Group: Code[20]): Text
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
    begin
        Parameters.Add('Group', Group);
        Parameters.Add('Mode', Mode);
        Results := Lines.RunPageBackgroundTask(Codeunit::"BCC Count Reader", Parameters, false);
        Results.Get('Seen', Seen);
        exit(Seen);
    end;

    local procedure AddLine(Parent: Code[20]; LineNo: Integer)
    var
        Line: Record "BCC Line";
    begin
        Line.Parent := Parent;
        Line."Line No." := LineNo;
        Line.Kind := 'K';
        Line.Amount := 1;
        Line.Insert();
        Line.SetRange(Parent, Parent);
        if Line.FindFirst() then;
    end;

    [Test]
    procedure BgCalcCache_CountAscendingThenDescending_FirstAnswer()
    var
        Lines: TestPage "BCC Lines";
        Parent: Record "BCC Parent";
        G: Code[20];
        Seen: Text;
    begin
        G := CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 18);
        Parent."No." := G;
        Parent.Insert();
        AddLine(G, 1);
        Commit();
        Lines.OpenView();
        Seen := Run(Lines, 'ascending', G);
        AddLine(G, 0);
        AddLine(G, 2);
        Seen += ' / ' + Run(Lines, 'descending', G);
        Lines.Close();
        Assert.AreEqual('1 / 1', Seen, 'Count ascending, then descending');
    end;
}
