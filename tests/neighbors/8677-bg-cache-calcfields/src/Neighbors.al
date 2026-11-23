// Neighbors of al-runner issue 8677 that no cell of the corpus probe BgTaskQueryCache.al measures (BQ Tests 10, open corpus
// PR 1495, :5833-:6290): a page background task's CalcFields and CalcSums, run twice around lines the test session inserts
// without a commit. Each test asserts BC's answer (a local BC 29.1 container) as 'first / again'. They were past what the
// cells of BQ Tests 10 measure; open corpus PR 1533 (BgTaskCalcCache.al, al-runner issue 8803) now measures each shape, and
// CalcProbe.al is its excerpt: a negated sum, a CalcFields through a RecordRef's FieldRef, a FlowFilter range, a filter on a field that is
// no FlowFilter, a CalcSums with a > filter, a count whose field() is a sum FlowField of the row, a temporary record, and a sum
// whose field(filter()) reads a stored field of the row, set to A and then to B in one worker run. From the review of al-runner
// PR 8682: SetLoadFields, Get and CalcFields (the cell BgCalcFields_AfterSetLoadFields_FirstAnswer, :7255, measures it on
// table 97597; BC answers the first read). The exist over Tenant Media is in Platform.al. From the review of al-runner PR 8872,
// one test per fact the cache took past the first cells: a | of values on a field that is no FlowFilter, a CalcFields and a
// CalcSums ascending first and descending again, a count at a sum whose field(filter()) reads a stored field of the row (the
// cells of BgTaskCalcCache.al measure each), and a FieldRef.CalcField through RecordRef.GetTable of a temporary record a
// Variant got from another Variant, whose table the runner marks "rows not measured" (BC answers the first answer in each, a
// local BC 29.1 container, 3 runs).

table 50120 "BQN Parent"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Description; Text[50]) { }
        field(3; "Kind Filter"; Code[10]) { FieldClass = FlowFilter; }
        field(4; "Line Count"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("BQN Line" where(Parent = field("No.")));
        }
        field(5; "Negative Sum"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = - sum("BQN Line".Amount where(Parent = field("No.")));
        }
        field(6; "Kind Sum"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = sum("BQN Line".Amount where(Parent = field("No."), Kind = field("Kind Filter")));
        }
        field(7; "Amount Total"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = sum("BQN Line".Amount where(Parent = field("No.")));
        }
        field(8; "Lines At Total"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("BQN Line" where(Parent = field("No."), Amount = field("Amount Total")));
        }
        field(9; "Kind Text"; Code[100]) { }
        field(10; "Kind Text Sum"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = sum("BQN Line".Amount where(Parent = field("No."), Kind = field(filter("Kind Text"))));
        }
        field(11; "Lines At Kind Text Sum"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("BQN Line" where(Parent = field("No."), Amount = field("Kind Text Sum")));
        }
        field(12; "Same Description Count"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("BQN Parent" where(Description = field(Description)));
        }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

table 50121 "BQN Line"
{
    DataClassification = SystemMetadata;

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

page 50122 "BQN Lines"
{
    PageType = List;
    SourceTable = "BQN Line";
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

codeunit 50123 "BQN Reader"
{
    trigger OnRun()
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Group: Text;
        Mode: Text;
    begin
        Parameters := Page.GetBackgroundParameters();
        Parameters.Get('Group', Group);
        Parameters.Get('Mode', Mode);
        Results.Add('Seen', Calc(Mode, Group));
        Page.SetBackgroundTaskResult(Results);
    end;

    local procedure D(Value: Decimal): Text
    begin
        exit(Format(Value, 0, 9));
    end;

    local procedure Calc(Mode: Text; Group: Text): Text
    var
        Parent: Record "BQN Parent";
        Temp: Record "BQN Parent" temporary;
        Line: Record "BQN Line";
        Ref: RecordRef;
        FRef: FieldRef;
        V1: Variant;
        V2: Variant;
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
                    Ref.Open(Database::"BQN Parent");
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
            'atsum':
                begin
                    Parent.Get(Group);
                    Parent.CalcFields("Lines At Total");
                    exit(Format(Parent."Lines At Total"));
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
            'loadfields':
                begin
                    Parent.SetLoadFields(Description);
                    Parent.Get(Group);
                    Parent.CalcFields("Line Count");
                    exit(Format(Parent."Line Count"));
                end;
            'temporary':
                begin
                    Parent.Get(Group);
                    Temp := Parent;
                    Temp.Insert();
                    Temp.CalcFields("Line Count");
                    exit(Format(Temp."Line Count"));
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
            'samedescription':
                begin
                    Parent.Get(Group);
                    Parent.CalcFields("Same Description Count");
                    exit(Format(Parent."Same Description Count"));
                end;
            'descendingcalc':
                begin
                    Parent.SetAscending("No.", false);
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
            'descendingsums':
                begin
                    Line.SetAscending("Line No.", false);
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
            'variantcopy':
                begin
                    Parent.Get(Group);
                    Temp := Parent;
                    Temp.Insert();
                    V1 := Temp;
                    V2 := V1;
                    Ref.GetTable(V2);
                    FRef := Ref.Field(4);
                    FRef.CalcField();
                    exit(Format(FRef.Value()));
                end;
        end;
        Error('Mode %1', Mode);
    end;
}

codeunit 50124 "BQN Tests"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;

    local procedure Run(var Lines: TestPage "BQN Lines"; Mode: Text; Group: Code[20]): Text
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
    begin
        Parameters.Add('Group', Group);
        Parameters.Add('Mode', Mode);
        Results := Lines.RunPageBackgroundTask(Codeunit::"BQN Reader", Parameters, false);
        Results.Get('Seen', Seen);
        exit(Seen);
    end;

    local procedure AddLine(Parent: Code[20]; LineNo: Integer; Kind: Code[10]; Amount: Decimal)
    var
        Line: Record "BQN Line";
    begin
        Line.Parent := Parent;
        Line."Line No." := LineNo;
        Line.Kind := Kind;
        Line.Amount := Amount;
        Line.Insert();
        Line.SetRange(Parent, Parent);
        if Line.FindFirst() then;
    end;

    local procedure Reads(Mode: Text): Text
    begin
        exit(ReadsAcross(Mode, Mode));
    end;

    // A parent with line 1 (Kind A, Amount 5), committed; the worker (First); lines 0 (Kind K, Amount 2) and 2 (Kind B,
    // Amount 11) without a commit; the worker again (Again).
    local procedure ReadsAcross(First: Text; Again: Text): Text
    var
        Lines: TestPage "BQN Lines";
        Parent: Record "BQN Parent";
        G: Code[20];
        Seen: Text;
    begin
        G := CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 18);
        Parent."No." := G;
        Parent.Insert();
        AddLine(G, 1, 'A', 5);
        Commit();
        Lines.OpenView();
        Seen := Run(Lines, First, G);
        AddLine(G, 0, 'K', 2);
        AddLine(G, 2, 'B', 11);
        Seen += ' / ' + Run(Lines, Again, G);
        Lines.Close();
        exit(Seen);
    end;

    [Test]
    procedure NegatedSum_LinesAfterFirstRead()
    begin
        Assert.AreEqual('-5 / -5', Reads('negative'), 'a negated sum');
    end;

    [Test]
    procedure RecordRefCalcField_LinesAfterFirstRead()
    begin
        Assert.AreEqual('1 / 1', Reads('recordref'), 'FieldRef.CalcField');
    end;

    [Test]
    procedure FlowFilterRange_LinesAfterFirstRead()
    begin
        Assert.AreEqual('5 / 5', Reads('flowfilterrange'), 'a FlowFilter range');
    end;

    [Test]
    procedure FilterOnAStoredField_LinesAfterFirstRead()
    begin
        Assert.AreEqual('1 / 1', Reads('otherfilter'), 'a filter on Description');
    end;

    [Test]
    procedure CalcSumsGreaterThan_LinesAfterFirstRead()
    begin
        Assert.AreEqual('5 / 5', Reads('calcsumsgt'), 'CalcSums with a > filter');
    end;

    [Test]
    procedure CountAtASumOfTheRow_LinesAfterFirstRead()
    begin
        Assert.AreEqual('1 / 1', Reads('atsum'), 'field() of a sum FlowField of the row');
    end;

    [Test]
    procedure TemporaryRecord_LinesAfterFirstRead()
    begin
        Assert.AreEqual('1 / 1', Reads('temporary'), 'a temporary record');
    end;

    [Test]
    procedure FieldFilterOfAStoredField_LinesAfterFirstRead()
    begin
        Assert.AreEqual('5,0 / 5,0', Reads('fieldfilter'), 'field(filter()) of Kind Text, A then B');
    end;

    [Test]
    procedure LoadFieldsGetCalcFields_LinesAfterFirstRead()
    begin
        Assert.AreEqual('1 / 1', Reads('loadfields'), 'SetLoadFields, Get, CalcFields');
    end;

    [Test]
    procedure OrOfValuesOnAStoredField_LinesAfterFirstRead()
    begin
        Assert.AreEqual('1 / 1', Reads('otherfilteror'), 'a | of values on Description');
    end;

    [Test]
    procedure CalcFieldsAscendingThenDescending_LinesAfterFirstRead()
    begin
        Assert.AreEqual('1 / 1', ReadsAcross('linecount', 'descendingcalc'), 'CalcFields, then after SetAscending(false)');
    end;

    [Test]
    procedure CalcSumsAscendingThenDescending_LinesAfterFirstRead()
    begin
        Assert.AreEqual('5 / 5', ReadsAcross('sums', 'descendingsums'), 'CalcSums, then after SetAscending(false)');
    end;

    [Test]
    procedure CountAtASumWithAFieldFilterOfTheRow_LinesAfterFirstRead()
    begin
        Assert.AreEqual('1 / 1', Reads('innerfieldfilter'), 'field() of a sum whose field(filter()) reads Kind Text');
    end;

    [Test]
    procedure VariantCopiedTemporaryRecord_LinesAfterFirstRead()
    begin
        Assert.AreEqual('1 / 1', Reads('variantcopy'), 'a temporary record a Variant got from another Variant');
    end;
}

// From the review of al-runner PR 8897: a worker's CalcFields of the parent's Line Count, then the test session commits a
// ModifyAll, a DeleteAll or a Delete of another parent, and inserts lines 0 and 2 without a
// commit, and the worker reads again (the cells of BgTaskCacheEdges.al, BCE Commit Tests, measure each); the test session's
// CalcFields first, then a commit of table 50120 and the lines; FieldRef.CalcField of a database RecordRef after a commit of
// table 50120; and a FlowField whose formula reads the row's own table (Same Description Count), with another parent of that
// description committed between the reads. BC's answers, a local BC 29.1 container, 3 runs.
codeunit 50128 "BQN Commit Tests"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;

    local procedure Run(var Lines: TestPage "BQN Lines"; Mode: Text; Group: Code[20]): Text
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
    begin
        Parameters.Add('Group', Group);
        Parameters.Add('Mode', Mode);
        Results := Lines.RunPageBackgroundTask(Codeunit::"BQN Reader", Parameters, false);
        Results.Get('Seen', Seen);
        exit(Seen);
    end;

    local procedure AddLine(Parent: Code[20]; LineNo: Integer; Amount: Decimal)
    var
        Line: Record "BQN Line";
    begin
        Line.Parent := Parent;
        Line."Line No." := LineNo;
        Line.Kind := 'K';
        Line.Amount := Amount;
        Line.Insert();
        Line.SetRange(Parent, Parent);
        if Line.FindFirst() then;
    end;

    local procedure NewParent(Description: Text[50]): Code[20]
    var
        Parent: Record "BQN Parent";
    begin
        Parent."No." := CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 18);
        Parent.Description := Description;
        Parent.Insert();
        exit(Parent."No.");
    end;

    local procedure Change(G: Code[20]; What: Text)
    var
        Parent: Record "BQN Parent";
        Other: Code[20];
    begin
        case What of
            'modifyall':
                begin
                    Other := NewParent('');
                    Commit();
                    Parent.SetRange("No.", Other);
                    Parent.ModifyAll(Description, 'other');
                end;
            'deleteall':
                begin
                    Other := NewParent('');
                    Commit();
                    Parent.SetRange("No.", Other);
                    Parent.DeleteAll();
                end;
            'delete':
                begin
                    Other := NewParent('');
                    Commit();
                    Parent.Get(Other);
                    Parent.Delete();
                end;
            'samedescription':
                NewParent(G);
        end;
        Commit();
        AddLine(G, 0, 2);
        AddLine(G, 2, 11);
    end;

    // A parent with description G and line 1, committed; the worker (First); the change; the worker again (Again).
    local procedure Reads(First: Text; Again: Text; What: Text): Text
    var
        Lines: TestPage "BQN Lines";
        G: Code[20];
        Seen: Text;
    begin
        G := CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 18);
        NewParentNo(G);
        AddLine(G, 1, 5);
        Commit();
        Lines.OpenView();
        Seen := Run(Lines, First, G);
        Change(G, What);
        Seen += ' / ' + Run(Lines, Again, G);
        Lines.Close();
        exit(Seen);
    end;

    local procedure NewParentNo(G: Code[20])
    var
        Parent: Record "BQN Parent";
    begin
        Parent."No." := G;
        Parent.Description := G;
        Parent.Insert();
    end;

    [Test]
    procedure ModifyAllCommitted_LinesAfterFirstRead()
    begin
        Assert.AreEqual('1 / 1', Reads('linecount', 'linecount', 'modifyall'), 'a ModifyAll of another parent committed');
    end;

    [Test]
    procedure DeleteAllCommitted_LinesAfterFirstRead()
    begin
        Assert.AreEqual('1 / 1', Reads('linecount', 'linecount', 'deleteall'), 'a DeleteAll of another parent committed');
    end;

    [Test]
    procedure DeleteCommitted_LinesAfterFirstRead()
    begin
        Assert.AreEqual('1 / 1', Reads('linecount', 'linecount', 'delete'), 'a Delete of another parent committed');
    end;

    [Test]
    procedure TestCalcFieldsFirstThenCommit_LinesAfterIt()
    var
        Lines: TestPage "BQN Lines";
        Parent: Record "BQN Parent";
        G: Code[20];
        Seen: Text;
    begin
        G := CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 18);
        NewParentNo(G);
        AddLine(G, 1, 5);
        Commit();
        Lines.OpenView();
        Parent.Get(G);
        Parent.CalcFields("Line Count");
        Seen := Format(Parent."Line Count");
        NewParent('');
        Commit();
        AddLine(G, 0, 2);
        AddLine(G, 2, 11);
        Seen += ' / ' + Run(Lines, 'linecount', G);
        Lines.Close();
        Assert.AreEqual('1 / 1', Seen, 'the test session''s CalcFields, a commit, then the worker''s');
    end;

    [Test]
    procedure RecordRefCalcFieldAfterACommit_LinesAfterFirstRead()
    begin
        Assert.AreEqual('1 / 1', Reads('recordref', 'recordref', 'delete'), 'FieldRef.CalcField after a Delete of another parent committed');
    end;

    [Test]
    procedure FlowFieldOverItsOwnTable_ParentCommitted()
    begin
        Assert.AreEqual('1 / 2', Reads('samedescription', 'samedescription', 'samedescription'), 'a count of parents of the same description');
    end;
}
