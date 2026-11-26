// An excerpt of the corpus probe BgTaskQueryCache.al of corpus pull request 1495 at 37bffb8a: BQ Tests 9b (:5637-:5831) and
// BQ Tests 9c (:6291-:6433), the cells of "@ alone" that need no page background task.

// BQ Tests 9b (al-runner issue 8637, @ alone in the database and in a temporary record; measured on a local BC 29.1 OnPrem
// w1 container). The source rows are K (Amount 10) and empty (Amount 5), in a Code and a Text field. @ alone keeps the rows
// whose value is empty when the database reads them, whatever the field's type: a FlowFilter's @ that CalcFields applies to
// a source field of either type, the same or the other (a Text FlowFilter over a Code field, a Code FlowFilter over a Text
// field), @ set on the source's Code and Text fields, and @ on a lookup FlowField (Text or Code) of a host in the database.
// It keeps every row of a temporary record: its Code and Text fields, and a lookup FlowField of a temporary host.
// Fixtures (the header's Fixtures used lines have no room): tables 97548 and 97549, test codeunit 97550.

table 97548 "BQ Kind Source"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; "Group"; Code[20]) { }
        field(2; Id; Integer) { }
        field(3; "Code Kind"; Code[20]) { }
        field(4; "Text Kind"; Text[50]) { }
        field(5; Amount; Decimal) { }
    }

    keys
    {
        key(PK; "Group", Id) { Clustered = true; }
    }
}

table 97549 "BQ Kind Host"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; "Text Filter"; Text[50]) { FieldClass = FlowFilter; }
        field(3; "Code Filter"; Code[20]) { FieldClass = FlowFilter; }
        field(4; "Code By Text"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = sum("BQ Kind Source".Amount where("Group" = field("No."), "Code Kind" = field("Text Filter")));
        }
        field(5; "Text By Text"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = sum("BQ Kind Source".Amount where("Group" = field("No."), "Text Kind" = field("Text Filter")));
        }
        field(6; "Text By Code"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = sum("BQ Kind Source".Amount where("Group" = field("No."), "Text Kind" = field("Code Filter")));
        }
        field(7; "Code By Code"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = sum("BQ Kind Source".Amount where("Group" = field("No."), "Code Kind" = field("Code Filter")));
        }
        field(8; "Count Code By Text"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("BQ Kind Source" where("Group" = field("No."), "Code Kind" = field("Text Filter")));
        }
        field(9; "Count Text By Code"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("BQ Kind Source" where("Group" = field("No."), "Text Kind" = field("Code Filter")));
        }
        field(10; "Kind One Text"; Text[50])
        {
            FieldClass = FlowField;
            CalcFormula = lookup("BQ Kind Source"."Text Kind" where("Group" = field("No."), Id = const(1)));
        }
        field(11; "Kind One Code"; Code[20])
        {
            FieldClass = FlowField;
            CalcFormula = lookup("BQ Kind Source"."Code Kind" where("Group" = field("No."), Id = const(1)));
        }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

codeunit 97550 "BQ Tests 9b"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;

    // A host and its source rows: K (Amount 10) and empty (Amount 5), the same in both kind fields.
    local procedure NewHost(var Host: Record "BQ Kind Host")
    var
        Source: Record "BQ Kind Source";
    begin
        Host."No." := CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 18);
        Host.Insert();
        Source."Group" := Host."No.";
        Source.Id := 1;
        Source."Code Kind" := 'K';
        Source."Text Kind" := 'K';
        Source.Amount := 10;
        Source.Insert();
        Source.Id := 2;
        Source."Code Kind" := '';
        Source."Text Kind" := '';
        Source.Amount := 5;
        Source.Insert();
        Host.SetRange("No.", Host."No.");
    end;

    // A Text FlowFilter @ alone: over a Code field (sum and count) and over a Text field.
    [Test]
    procedure AtSignAlone_TextFlowFilter_OverCodeAndText()
    var
        Host: Record "BQ Kind Host";
    begin
        NewHost(Host);
        Host.SetFilter("Text Filter", '@');
        Host.FindFirst();
        Host.CalcFields("Code By Text", "Text By Text", "Count Code By Text");
        Assert.AreEqual('5|5|1', Format(Host."Code By Text") + '|' + Format(Host."Text By Text") + '|' +
            Format(Host."Count Code By Text"), 'code, text, count');
    end;

    // A Code FlowFilter @ alone: over a Text field (sum and count) and over a Code field.
    [Test]
    procedure AtSignAlone_CodeFlowFilter_OverTextAndCode()
    var
        Host: Record "BQ Kind Host";
    begin
        NewHost(Host);
        Host.SetFilter("Code Filter", '@');
        Host.FindFirst();
        Host.CalcFields("Text By Code", "Code By Code", "Count Text By Code");
        Assert.AreEqual('5|5|1', Format(Host."Text By Code") + '|' + Format(Host."Code By Code") + '|' +
            Format(Host."Count Text By Code"), 'text, code, count');
    end;
    // @ alone set on the kind fields of the source rows themselves, in the database and in a temporary record.
    [Test]
    procedure AtSignAlone_SourceFields_DatabaseAndTemporary()
    var
        Host: Record "BQ Kind Host";
        Source: Record "BQ Kind Source";
        Temp: Record "BQ Kind Source" temporary;
        Seen: Text;
    begin
        NewHost(Host);
        Source.SetRange("Group", Host."No.");
        if Source.FindSet() then
            repeat
                Temp := Source;
                Temp.Insert();
            until Source.Next() = 0;
        Source.SetFilter("Code Kind", '@');
        Seen := Format(Source.Count());
        Source.SetRange("Code Kind");
        Source.SetFilter("Text Kind", '@');
        Seen += '|' + Format(Source.Count());
        Temp.SetFilter("Code Kind", '@');
        Seen += '|' + Format(Temp.Count());
        Temp.SetRange("Code Kind");
        Temp.SetFilter("Text Kind", '@');
        Seen += '|' + Format(Temp.Count());
        Assert.AreEqual('1|1|2|2', Seen, 'database code, database text, temporary code, temporary text');
    end;
    // @ alone on a lookup FlowField whose value is K (a Text and a Code one), of the host in the database and in a temporary
    // record.
    [Test]
    procedure AtSignAlone_LookupFlowFieldFilters_DatabaseAndTemporary()
    var
        Host: Record "BQ Kind Host";
        Temp: Record "BQ Kind Host" temporary;
        Seen: Text;
    begin
        NewHost(Host);
        Temp := Host;
        Temp.Insert();
        Temp.SetRange("No.", Host."No.");
        Host.SetFilter("Kind One Text", '@');
        Seen := Format(Host.Count());
        Host.SetRange("Kind One Text");
        Host.SetFilter("Kind One Code", '@');
        Seen += '|' + Format(Host.Count());
        Temp.SetFilter("Kind One Text", '@');
        Seen += '|' + Format(Temp.Count());
        Temp.SetRange("Kind One Text");
        Temp.SetFilter("Kind One Code", '@');
        Seen += '|' + Format(Temp.Count());
        Assert.AreEqual('0|0|1|1', Seen, 'database text, database code, temporary text, temporary code');
    end;
}


// BQ Tests 9c (al-runner issue 8637, @ alone in a query; measured on a local BC 29.1 OnPrem w1 container): a query over table
// 97548 reads the database, and @ alone keeps the rows whose value is empty there too, set on a Code or a Text column by
// SetFilter or in the DataItemTableFilter. The source rows are K and empty.
// Fixtures (the header's Fixtures used lines have no room): queries 97559, 97560 and 97561, test codeunit 97559.

query 97559 "BQ Kind Columns"
{
    QueryType = Normal;

    elements
    {
        dataitem(Source; "BQ Kind Source")
        {
            column(KindGroup; "Group") { }
            column(Id; Id) { }
            column(CodeKind; "Code Kind") { }
            column(TextKind; "Text Kind") { }
        }
    }
}

query 97560 "BQ Kind Code At"
{
    QueryType = Normal;

    elements
    {
        dataitem(Source; "BQ Kind Source")
        {
            DataItemTableFilter = "Code Kind" = filter('@');
            column(KindGroup; "Group") { }
            column(Id; Id) { }
        }
    }
}

query 97561 "BQ Kind Text At"
{
    QueryType = Normal;

    elements
    {
        dataitem(Source; "BQ Kind Source")
        {
            DataItemTableFilter = "Text Kind" = filter('@');
            column(KindGroup; "Group") { }
            column(Id; Id) { }
        }
    }
}

codeunit 97559 "BQ Tests 9c"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;

    // Rows 1 (K) and 2 (empty) of a new group, in the Code and the Text kind field.
    local procedure NewGroup(): Code[20]
    var
        Source: Record "BQ Kind Source";
    begin
        Source."Group" := CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 18);
        Source.Id := 1;
        Source."Code Kind" := 'K';
        Source."Text Kind" := 'K';
        Source.Amount := 10;
        Source.Insert();
        Source.Id := 2;
        Source."Code Kind" := '';
        Source."Text Kind" := '';
        Source.Amount := 5;
        Source.Insert();
        exit(Source."Group");
    end;

    [Test]
    procedure QueryAtSignAlone_CodeColumn_KeepsTheEmptyValues()
    var
        Columns: Query "BQ Kind Columns";
        N: Integer;
    begin
        Columns.SetRange(KindGroup, NewGroup());
        Columns.SetFilter(CodeKind, '@');
        Columns.Open();
        while Columns.Read() do begin
            N += 1;
            Assert.AreEqual(2, Columns.Id, 'the row @ keeps');
        end;
        Columns.Close();
        Assert.AreEqual(1, N, 'the rows @ keeps');
    end;

    [Test]
    procedure QueryAtSignAlone_TextColumn_KeepsTheEmptyValues()
    var
        Columns: Query "BQ Kind Columns";
        N: Integer;
    begin
        Columns.SetRange(KindGroup, NewGroup());
        Columns.SetFilter(TextKind, '@');
        Columns.Open();
        while Columns.Read() do begin
            N += 1;
            Assert.AreEqual(2, Columns.Id, 'the row @ keeps');
        end;
        Columns.Close();
        Assert.AreEqual(1, N, 'the rows @ keeps');
    end;

    [Test]
    procedure QueryAtSignAlone_CodeDataItemTableFilter_KeepsTheEmptyValues()
    var
        CodeAt: Query "BQ Kind Code At";
        N: Integer;
    begin
        CodeAt.SetRange(KindGroup, NewGroup());
        CodeAt.Open();
        while CodeAt.Read() do begin
            N += 1;
            Assert.AreEqual(2, CodeAt.Id, 'the row @ keeps');
        end;
        CodeAt.Close();
        Assert.AreEqual(1, N, 'the rows @ keeps');
    end;

    [Test]
    procedure QueryAtSignAlone_TextDataItemTableFilter_KeepsTheEmptyValues()
    var
        TextAt: Query "BQ Kind Text At";
        N: Integer;
    begin
        TextAt.SetRange(KindGroup, NewGroup());
        TextAt.Open();
        while TextAt.Read() do begin
            N += 1;
            Assert.AreEqual(2, TextAt.Id, 'the row @ keeps');
        end;
        TextAt.Close();
        Assert.AreEqual(1, N, 'the rows @ keeps');
    end;
}
