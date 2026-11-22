// Neighbors of al-runner issue 8677 that no cell of the corpus probe BgTaskQueryCache.al measures (BQ Tests 10, open corpus
// PR 1495, :5833-:6290): a page background task's CalcFields and CalcSums, run twice around lines the test session inserts
// without a commit. Each test asserts BC's answer (a local BC 29.1 container) as 'first / again'. They are past what the
// cells measure: a negated sum, a CalcFields through a RecordRef's FieldRef, a FlowFilter range, a filter on a field that is
// no FlowFilter, a CalcSums with a range, a count whose field() is a sum FlowField of the row, a temporary record, and a sum
// whose field(filter()) reads a stored field of the row, set to A and then to B in one worker run.

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
            'calcsumsrange':
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
            'temporary':
                begin
                    Parent.Get(Group);
                    Temp := Parent;
                    Temp.Insert();
                    Temp.CalcFields("Line Count");
                    exit(Format(Temp."Line Count"));
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

    // A parent with line 1 (Kind A, Amount 5), committed; the worker; lines 0 (Kind K, Amount 2) and 2 (Kind B, Amount 11)
    // without a commit; the worker again.
    local procedure Reads(Mode: Text): Text
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
        Seen := Run(Lines, Mode, G);
        AddLine(G, 0, 'K', 2);
        AddLine(G, 2, 'B', 11);
        Seen += ' / ' + Run(Lines, Mode, G);
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
    procedure CalcSumsRange_LinesAfterFirstRead()
    begin
        Assert.AreEqual('5 / 5', Reads('calcsumsrange'), 'CalcSums with a range');
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
}
