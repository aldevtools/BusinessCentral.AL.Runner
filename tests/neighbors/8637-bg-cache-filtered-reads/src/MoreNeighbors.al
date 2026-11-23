// Neighbors from the reviews of al-runner PRs 8656 and 8682 that no cell of BgTaskQueryCache.al measures; each asserts BC's
// answer (a local BC 29.1 container, 3 runs) as 'first / again': a worker's Count of a group of table 97500 ascending, then
// descending (BC answers the first read, so the entry is shared across the sort direction) and ascending twice; a count and
// an exist FlowField whose field() is a count FlowField of the row; a CalcSums with no filter (the second sum less the first:
// 0); and a CalcFields and a CalcSums after SetAscending(false). BC answers each from the cache.
table 97583 "BQ More Parent"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Description; Text[50]) { }
        field(4; "Line Count"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("BQ More Line" where(Parent = field("No.")));
        }
        field(5; "Note Count"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("BQ More Note" where(Parent = field("No.")));
        }
        field(6; "Count At Note Count"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("BQ More Line" where(Parent = field("No."), "Line No." = field("Note Count")));
        }
        field(7; "Exist At Note Count"; Boolean)
        {
            FieldClass = FlowField;
            CalcFormula = exist("BQ More Line" where(Parent = field("No."), "Line No." = field("Note Count")));
        }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

table 97584 "BQ More Line"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; Parent; Code[20]) { }
        field(2; "Line No."; Integer) { }
        field(3; Amount; Decimal) { }
    }

    keys
    {
        key(PK; Parent, "Line No.") { Clustered = true; }
    }
}

table 97585 "BQ More Note"
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

codeunit 97583 "BQ More Reader"
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
        Parent: Record "BQ More Parent";
        Line: Record "BQ More Line";
        Row: Record "BQ Row";
        Kind: Option First,Second;
        C: Char;
        B: Byte;
        E: Enum "BQ More Kind";
    begin
        case Mode of
            'countasc', 'countdesc':
                begin
                    Row.SetRange("Group", Group);
                    if Mode = 'countdesc' then
                        Row.Ascending(false);
                    exit(Format(Row.Count()));
                end;
            'outercount':
                begin
                    Parent.Get(Group);
                    Parent.CalcFields("Count At Note Count");
                    exit(Format(Parent."Count At Note Count"));
                end;
            'outerexist':
                begin
                    Parent.Get(Group);
                    Parent.CalcFields("Exist At Note Count");
                    exit(Format(Parent."Exist At Note Count"));
                end;
            'calcsumsall':
                begin
                    Line.CalcSums(Amount);
                    exit(D(Line.Amount));
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
            'description', 'optionselect', 'charselect', 'byteselect', 'enumselect':
                begin
                    // Each value is 0, which names no table; the compiler takes each for the Integer (an Enum with warning
                    // AL0603).
                    case Mode of
                        'optionselect':
                            Database.SelectLatestVersion(Kind);
                        'charselect':
                            Database.SelectLatestVersion(C);
                        'byteselect':
                            Database.SelectLatestVersion(B);
                        'enumselect':
                            Database.SelectLatestVersion(E);
                    end;
                    Parent.Get(Group);
                    exit(Parent.Description);
                end;
        end;
        Error('Mode %1', Mode);
    end;
}

codeunit 97584 "BQ More Tests"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;

    local procedure NewGroup(): Code[20]
    begin
        exit(CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 18));
    end;

    local procedure Run(var Rows: TestPage "BQ Plain Rows"; Mode: Text; Group: Code[20]): Text
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
    begin
        Parameters.Add('Group', Group);
        Parameters.Add('Mode', Mode);
        Results := Rows.RunPageBackgroundTask(Codeunit::"BQ More Reader", Parameters, false);
        Results.Get('Seen', Seen);
        exit(Seen);
    end;

    local procedure AddLine(Parent: Code[20]; LineNo: Integer; Amount: Decimal)
    var
        Line: Record "BQ More Line";
    begin
        Line.Parent := Parent;
        Line."Line No." := LineNo;
        Line.Amount := Amount;
        Line.Insert();
        Line.SetRange(Parent, Parent);
        if Line.FindFirst() then;
    end;

    local procedure AddRow(Group: Code[20]; LineNo: Integer)
    var
        Row: Record "BQ Row";
    begin
        Row."Group" := Group;
        Row."Line No." := LineNo;
        Row.Description := 'r';
        Row.Insert();
        Row.SetRange("Group", Group);
        if Row.FindFirst() then;
    end;

    // A parent with line 1 (Amount 5) and a row of table 97500 (group G, line 1), committed; the worker (First); the change without a commit; the worker
    // (Again): 'first / again'.
    local procedure Reads(First: Text; Again: Text; Change: Text): Text
    var
        Rows: TestPage "BQ Plain Rows";
        Parent: Record "BQ More Parent";
        Note: Record "BQ More Note";
        G: Code[20];
        Seen: Text;
    begin
        G := NewGroup();
        Parent."No." := G;
        Parent.Insert();
        AddLine(G, 1, 5);
        AddRow(G, 1);
        Commit();
        Rows.OpenView();
        Seen := Run(Rows, First, G);
        case Change of
            'lines':
                begin
                    AddLine(G, 0, 2);
                    AddLine(G, 2, 11);
                end;
            'rows':
                AddRow(G, 2);
            'note':
                begin
                    Note.Parent := G;
                    Note."Line No." := 1;
                    Note.Insert();
                    Note.SetRange(Parent, G);
                    if Note.FindFirst() then;
                end;
        end;
        Seen += ' / ' + Run(Rows, Again, G);
        Rows.Close();
        exit(Seen);
    end;

    [Test]
    procedure CountAscendingThenDescending_RowAfterFirstRead()
    begin
        Assert.AreEqual('1 / 1', Reads('countasc', 'countdesc', 'rows'), 'countasc/desc');
    end;

    [Test]
    procedure CountAscendingTwice_RowAfterFirstRead()
    begin
        Assert.AreEqual('1 / 1', Reads('countasc', 'countasc', 'rows'), 'countasc/asc');
    end;

    [Test]
    procedure CountAtACountOfTheRow_NoteAfterFirstRead()
    begin
        Assert.AreEqual('0 / 0', Reads('outercount', 'outercount', 'note'), 'outercount');
    end;

    [Test]
    procedure ExistAtACountOfTheRow_NoteAfterFirstRead()
    begin
        Assert.AreEqual('No / No', Reads('outerexist', 'outerexist', 'note'), 'outerexist');
    end;

    [Test]
    procedure CalcSumsWithoutAFilter_LinesAfterFirstRead()
    var
        Seen: Text;
        A: Decimal;
        B: Decimal;
    begin
        Seen := Reads('calcsumsall', 'calcsumsall', 'lines');
        Evaluate(A, SelectStr(1, ConvertStr(Seen, '/', ',')));
        Evaluate(B, SelectStr(2, ConvertStr(Seen, '/', ',')));
        Assert.AreEqual('0', Format(B - A, 0, 9), 'calcsums of the whole table, again less first');
    end;

    [Test]
    procedure CalcFieldsAfterSetAscendingFalse_LinesAfterFirstRead()
    begin
        Assert.AreEqual('1 / 1', Reads('descendingcalc', 'descendingcalc', 'lines'), 'descending calc');
    end;

    [Test]
    procedure CalcSumsAfterSetAscendingFalse_LinesAfterFirstRead()
    begin
        Assert.AreEqual('5 / 5', Reads('descendingsums', 'descendingsums', 'lines'), 'descending sums');
    end;
}

// From the review of al-runner PR 8808 (al-runner issue 8879), on a committed parent with line 1 and description 'first'. A
// worker's FindFirst of the parent filtered on its key and on two FlowFields of different source tables (Line Count > 0, Note
// Count = 0); the test session inserts a note without a commit and reads it; the worker's FindFirst again, with the filters set
// in the same order and in the other order: BC answers the first answer in both, so the order the filters were set in is not
// part of the entry. A worker's Get of the parent; the test session changes its description without a commit; the worker calls
// SelectLatestVersion with an Option, a Char, a Byte or an Enum whose value 0 names no table, and gets the parent again: BC
// answers 'first' for each (a local BC 29.1 container, 3 runs).
codeunit 97510 "BQ More Follow-up Tests"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;

    local procedure Run(var Rows: TestPage "BQ Plain Rows"; Mode: Text; Group: Code[20]): Text
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
    begin
        Parameters.Add('Group', Group);
        Parameters.Add('Mode', Mode);
        Results := Rows.RunPageBackgroundTask(Codeunit::"BQ More Reader", Parameters, false);
        Results.Get('Seen', Seen);
        exit(Seen);
    end;

    local procedure Reads(First: Text; Again: Text; Change: Text): Text
    var
        Rows: TestPage "BQ Plain Rows";
        Parent: Record "BQ More Parent";
        Line: Record "BQ More Line";
        Note: Record "BQ More Note";
        G: Code[20];
        Seen: Text;
    begin
        G := CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 18);
        Parent."No." := G;
        Parent.Description := 'first';
        Parent.Insert();
        Line.Parent := G;
        Line."Line No." := 1;
        Line.Amount := 5;
        Line.Insert();
        Commit();
        Rows.OpenView();
        Seen := Run(Rows, First, G);
        case Change of
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
        end;
        Seen += ' / ' + Run(Rows, Again, G);
        Rows.Close();
        exit(Seen);
    end;

    [Test]
    procedure TwoFlowFieldFiltersSameOrder_NoteAfterFirstRead()
    begin
        Assert.AreEqual('Yes / Yes', Reads('linesnotes', 'linesnotes', 'note'), 'Line Count, Note Count twice');
    end;

    [Test]
    procedure TwoFlowFieldFiltersSwapped_NoteAfterFirstRead()
    begin
        Assert.AreEqual('Yes / Yes', Reads('linesnotes', 'noteslines', 'note'), 'Line Count, Note Count, then the other order');
    end;

    [Test]
    procedure SelectLatestVersionOfAnOption_DescriptionAfterFirstRead()
    begin
        Assert.AreEqual('first / first', Reads('description', 'optionselect', 'description'), 'SelectLatestVersion(Option)');
    end;

    [Test]
    procedure SelectLatestVersionOfAChar_DescriptionAfterFirstRead()
    begin
        Assert.AreEqual('first / first', Reads('description', 'charselect', 'description'), 'SelectLatestVersion(Char)');
    end;

    [Test]
    procedure SelectLatestVersionOfAByte_DescriptionAfterFirstRead()
    begin
        Assert.AreEqual('first / first', Reads('description', 'byteselect', 'description'), 'SelectLatestVersion(Byte)');
    end;

    [Test]
    procedure SelectLatestVersionOfAnEnum_DescriptionAfterFirstRead()
    begin
        Assert.AreEqual('first / first', Reads('description', 'enumselect', 'description'), 'SelectLatestVersion(Enum)');
    end;
}

enum 97510 "BQ More Kind"
{
    Extensible = false;

    value(0; None) { }
}
