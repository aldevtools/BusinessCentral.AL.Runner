// Neighbors of al-runner issue 8538 (corpus pull requests 1473, merged, and 1485): a record with SetAutoCalcFields handed by
// value to a procedure. BC's copy drops the caller's SetAutoCalcFields (record/filters/ByValueAutoCalcFields.al). The tests
// vary each fact the rule `measured_auto_calc` reads (the fields, the filters, the record state), the caller and callee
// kinds around it, and each reader of the mark it leaves on the copy. Heads H1 (Kind A, lines 4 and 6), H2 (A, line 20) and
// H3 (B, line 30). Every expected value of a shape the probe did not measure is the one the measured cells give.
table 50100 "NB AC Head"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Kind; Code[10]) { }
        field(3; "Line Total"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = sum("NB AC Detail".Amount where("Head No." = field("No.")));
            Editable = false;
        }
        field(4; "Line Count"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("NB AC Detail" where("Head No." = field("No.")));
            Editable = false;
        }
        field(5; "Min Amount"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = min("NB AC Detail".Amount where("Head No." = field("No.")));
            Editable = false;
        }
        field(6; "Max Amount"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = max("NB AC Detail".Amount where("Head No." = field("No.")));
            Editable = false;
        }
        field(7; "Has Lines"; Boolean)
        {
            FieldClass = FlowField;
            CalcFormula = exist("NB AC Detail" where("Head No." = field("No.")));
            Editable = false;
        }
        field(8; "Line Filter"; Integer) { FieldClass = FlowFilter; }
        field(9; Note; Text[30]) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
        key(ByKind; Kind) { }
    }

    procedure OwnProc(Copy: Record "NB AC Head"): Text
    var
        Callee: Codeunit "NB AC Callee";
    begin
        exit(Callee.DescribeVar(Copy));
    end;

    procedure DescribeThroughCodeunit(): Text
    var
        Head: Record "NB AC Head";
        Callee: Codeunit "NB AC Callee";
    begin
        Head.SetRange(Kind, 'A');
        Head.SetAutoCalcFields("Line Total");
        Head.FindFirst();
        exit(Callee.Describe(Head));
    end;
}

table 50101 "NB AC Detail"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Head No."; Code[20]) { }
        field(2; "Line No."; Integer) { }
        field(3; Amount; Decimal) { }
    }

    keys { key(PK; "Head No.", "Line No.") { Clustered = true; } }
}

table 50102 "NB AC Buffer"
{
    DataClassification = SystemMetadata;
    TableType = Temporary;

    fields
    {
        field(1; "Line No."; Integer) { }
    }

    keys { key(PK; "Line No.") { Clustered = true; } }

    procedure DescribeHead(H: Record "NB AC Head"): Text
    var
        Callee: Codeunit "NB AC Callee";
    begin
        exit(Callee.DescribeVar(H));
    end;
}

// A third table: its OnInsert, OnModify and a procedure hand a filtered record with SetAutoCalcFields to Buffer.DescribeHead.
table 50103 "NB AC Holder"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Line No."; Integer) { }
        field(2; Info; Text[250]) { }
    }

    keys { key(PK; "Line No.") { Clustered = true; } }

    trigger OnInsert()
    var
        Head: Record "NB AC Head";
        Buffer: Record "NB AC Buffer";
    begin
        Head.SetRange(Kind, 'A');
        Head.SetAutoCalcFields("Line Total");
        Head.FindFirst();
        Info := CopyStr(Buffer.DescribeHead(Head), 1, MaxStrLen(Info));
    end;

    trigger OnModify()
    var
        Head: Record "NB AC Head";
        Buffer: Record "NB AC Buffer";
    begin
        Head.SetRange(Kind, 'A');
        Head.SetAutoCalcFields("Line Total");
        Head.FindFirst();
        Info := CopyStr(Buffer.DescribeHead(Head), 1, MaxStrLen(Info));
    end;

    procedure FromProcedure(): Text
    var
        Head: Record "NB AC Head";
        Buffer: Record "NB AC Buffer";
    begin
        Head.SetRange(Kind, 'A');
        Head.SetAutoCalcFields("Line Total");
        Head.FindFirst();
        exit(Buffer.DescribeHead(Head));
    end;
}

codeunit 50104 "NB AC Callee"
{
    procedure Describe(H: Record "NB AC Head"): Text
    begin
        exit(DescribeVar(H));
    end;

    procedure DescribeVar(var H: Record "NB AC Head"): Text
    var
        Result: Text;
    begin
        Result := StrSubstNo('entry=%1 n=%2', Format(H."Line Total", 0, 9), H.Count());
        if H.FindLast() then
            Result += StrSubstNo(' last=%1 lastTotal=%2', H."No.", Format(H."Line Total", 0, 9));
        exit(Result);
    end;

    procedure DescribeVariant(Value: Variant): Text
    var
        RecRef: RecordRef;
        Rows: Integer;
    begin
        RecRef.GetTable(Value);
        Rows := RecRef.Count();
        RecRef.FindLast();
        exit(StrSubstNo('n=%1 last=%2 lastTotal=%3', Rows, RecRef.Field(1).Value, Format(RecRef.Field(3).Value, 0, 9)));
    end;

    procedure DescribeCopyAsVariant(H: Record "NB AC Head"): Text
    begin
        exit(DescribeVariant(H));
    end;

    procedure Stats(H: Record "NB AC Head"): Text
    begin
        H.FindLast();
        exit(StrSubstNo('last=%1 min=%2 max=%3 has=%4', H."No.", Format(H."Min Amount", 0, 9), Format(H."Max Amount", 0, 9), H."Has Lines"));
    end;

    procedure GetOfCopy(H: Record "NB AC Head"): Text
    begin
        H.Get('H3');
        exit(StrSubstNo('get=%1 total=%2', H."No.", Format(H."Line Total", 0, 9)));
    end;

    procedure CopyOfCopy(H: Record "NB AC Head"): Text
    var
        Other: Record "NB AC Head";
    begin
        Other.Copy(H);
        Other.FindLast();
        exit(StrSubstNo('other=%1 total=%2', Other."No.", Format(Other."Line Total", 0, 9)));
    end;

    procedure NextOfCopy(H: Record "NB AC Head"): Text
    begin
        H.FindFirst();
        H.Next();
        exit(StrSubstNo('next=%1 total=%2', H."No.", Format(H."Line Total", 0, 9)));
    end;

    procedure AddAutoCalcOnCopy(H: Record "NB AC Head"): Text
    begin
        H.SetAutoCalcFields("Line Count");
        H.FindLast();
        exit(StrSubstNo('last=%1 total=%2 count=%3', H."No.", Format(H."Line Total", 0, 9), H."Line Count"));
    end;

    procedure ClearAutoCalcOnCopy(H: Record "NB AC Head"): Text
    begin
        H.SetAutoCalcFields();
        H.FindLast();
        exit(StrSubstNo('last=%1 total=%2', H."No.", Format(H."Line Total", 0, 9)));
    end;

    procedure WriteThenFind(H: Record "NB AC Head"): Text
    begin
        H.Note := 'written';
        H.FindLast();
        exit(StrSubstNo('last=%1 total=%2', H."No.", Format(H."Line Total", 0, 9)));
    end;

    procedure CalcFieldsOfCopy(H: Record "NB AC Head"): Text
    begin
        H.FindLast();
        H.CalcFields("Line Total");
        exit(StrSubstNo('last=%1 total=%2', H."No.", Format(H."Line Total", 0, 9)));
    end;

    procedure FieldsOnly(H: Record "NB AC Head"): Text
    begin
        exit(StrSubstNo('no=%1 total=%2', H."No.", Format(H."Line Total", 0, 9)));
    end;
}

interface "NB AC Describer"
{
    procedure Describe(H: Record "NB AC Head"): Text;
}

codeunit 50105 "NB AC Describer Impl" implements "NB AC Describer"
{
    procedure Describe(H: Record "NB AC Head"): Text
    var
        Callee: Codeunit "NB AC Callee";
    begin
        exit(Callee.DescribeVar(H));
    end;
}

codeunit 50106 "NB AC Events"
{
    [IntegrationEvent(false, false)]
    procedure OnDescribe(H: Record "NB AC Head"; var Result: Text)
    begin
    end;
}

codeunit 50107 "NB AC Subscriber"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"NB AC Events", 'OnDescribe', '', false, false)]
    local procedure DescribeOnEvent(H: Record "NB AC Head"; var Result: Text)
    var
        Callee: Codeunit "NB AC Callee";
    begin
        Result := Callee.DescribeVar(H);
    end;
}

codeunit 50108 "NB AC Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Callee: Codeunit "NB AC Callee";
        Dropped: Label 'entry=10 n=3 last=H3 lastTotal=0', Locked = true;

    local procedure Initialize()
    var
        Head: Record "NB AC Head";
        Detail: Record "NB AC Detail";
        Holder: Record "NB AC Holder";
    begin
        Head.DeleteAll();
        Detail.DeleteAll();
        Holder.DeleteAll();
        AddHead('H1', 'A');
        AddHead('H2', 'A');
        AddHead('H3', 'B');
        AddDetail('H1', 1, 4);
        AddDetail('H1', 2, 6);
        AddDetail('H2', 1, 20);
        AddDetail('H3', 1, 30);
    end;

    local procedure AddHead(No: Code[20]; Kind: Code[10])
    var
        Head: Record "NB AC Head";
    begin
        Head."No." := No;
        Head.Kind := Kind;
        Head.Insert();
    end;

    local procedure AddDetail(HeadNo: Code[20]; LineNo: Integer; Amount: Decimal)
    var
        Detail: Record "NB AC Detail";
    begin
        Detail."Head No." := HeadNo;
        Detail."Line No." := LineNo;
        Detail.Amount := Amount;
        Detail.Insert();
    end;

    // The issue's shape: a filter in group 0, SetAutoCalcFields, FindFirst.
    local procedure FilteredHead(var Head: Record "NB AC Head")
    begin
        Initialize();
        Head.SetRange(Kind, 'A');
        Head.SetAutoCalcFields("Line Total");
        Head.FindFirst();
    end;

    local procedure Check(Actual: Text; Expected: Text)
    begin
        if Actual <> Expected then
            Error('Expected "%1", got "%2"', Expected, Actual);
    end;

    // The shapes the rule admits: the copy drops the caller's SetAutoCalcFields.

    [Test]
    procedure Codeunit_Group0Filter()
    var
        Head: Record "NB AC Head";
    begin
        FilteredHead(Head);
        Check(Callee.Describe(Head), Dropped);
    end;

    [Test]
    procedure Codeunit_NoFilter_NoRead()
    var
        Head: Record "NB AC Head";
    begin
        Initialize();
        Head.SetAutoCalcFields("Line Total");
        Check(Callee.Describe(Head), 'entry=0 n=3 last=H3 lastTotal=0');
    end;

    [Test]
    procedure Codeunit_ThreeMethods_MinMaxExist()
    var
        Head: Record "NB AC Head";
    begin
        Initialize();
        Head.SetRange(Kind, 'A');
        Head.SetAutoCalcFields("Min Amount", "Max Amount", "Has Lines");
        Head.FindFirst();
        Check(Callee.Stats(Head), 'last=H3 min=0 max=0 has=No');
    end;

    [Test]
    procedure Codeunit_FilterInGroup2()
    var
        Head: Record "NB AC Head";
    begin
        Initialize();
        Head.SetRange(Kind, 'A');
        Head.FilterGroup(2);
        Head.SetFilter("No.", '<>%1', 'H9');
        Head.FilterGroup(0);
        Head.SetAutoCalcFields("Line Total");
        Head.FindFirst();
        Check(Callee.Describe(Head), Dropped);
    end;

    [Test]
    procedure Codeunit_SecondaryKey_Descending_Marked()
    var
        Head: Record "NB AC Head";
    begin
        Initialize();
        Head.SetCurrentKey(Kind);
        Head.Ascending(false);
        Head.SetRange(Kind, 'A');
        Head.SetAutoCalcFields("Line Total");
        Head.FindFirst();
        Head.Mark(true);
        Check(Callee.Describe(Head), 'entry=20 n=3 last=H3 lastTotal=0');
    end;

    [Test]
    procedure Codeunit_VariantGetTable()
    var
        Head: Record "NB AC Head";
    begin
        FilteredHead(Head);
        Check(Callee.DescribeCopyAsVariant(Head), 'n=3 last=H3 lastTotal=0');
    end;

    [Test]
    procedure RowProcRow_FromCodeunit()
    var
        Head: Record "NB AC Head";
    begin
        FilteredHead(Head);
        Check(Head.OwnProc(Head), Dropped);
    end;

    [Test]
    procedure BufferProc_FromCodeunit()
    var
        Head: Record "NB AC Head";
        Buffer: Record "NB AC Buffer";
    begin
        FilteredHead(Head);
        Check(Buffer.DescribeHead(Head), Dropped);
    end;

    [Test]
    procedure BufferProc_FromThirdTableOnInsert()
    var
        Holder: Record "NB AC Holder";
    begin
        Initialize();
        Holder."Line No." := 1;
        Holder.Insert(true);
        Check(Holder.Info, Dropped);
    end;

    // Each reader of the copy's mark.

    [Test]
    procedure Reader_Get()
    var
        Head: Record "NB AC Head";
    begin
        FilteredHead(Head);
        Check(Callee.GetOfCopy(Head), 'get=H3 total=0');
    end;

    [Test]
    procedure Reader_CopyOfTheCopy()
    var
        Head: Record "NB AC Head";
    begin
        FilteredHead(Head);
        Check(Callee.CopyOfCopy(Head), 'other=H3 total=0');
    end;

    [Test]
    procedure Reader_Next()
    var
        Head: Record "NB AC Head";
    begin
        FilteredHead(Head);
        Check(Callee.NextOfCopy(Head), 'next=H2 total=0');
    end;

    [Test]
    procedure Reader_OwnSetAutoCalcFieldsOfAnotherField()
    var
        Head: Record "NB AC Head";
    begin
        FilteredHead(Head);
        Check(Callee.AddAutoCalcOnCopy(Head), 'last=H3 total=0 count=1');
    end;

    [Test]
    procedure Reader_OwnSetAutoCalcFieldsEmpty()
    var
        Head: Record "NB AC Head";
    begin
        FilteredHead(Head);
        Check(Callee.ClearAutoCalcOnCopy(Head), 'last=H3 total=0');
    end;

    [Test]
    procedure Reader_AfterAFieldWrite()
    var
        Head: Record "NB AC Head";
    begin
        FilteredHead(Head);
        Check(Callee.WriteThenFind(Head), 'last=H3 total=0');
    end;

    [Test]
    procedure Reader_CalcFieldsAfterFind()
    var
        Head: Record "NB AC Head";
    begin
        FilteredHead(Head);
        Check(Callee.CalcFieldsOfCopy(Head), 'last=H3 total=30');
    end;

    [Test]
    procedure Reader_FieldValuesOnly()
    var
        Head: Record "NB AC Head";
    begin
        FilteredHead(Head);
        Check(Callee.FieldsOnly(Head), 'no=H1 total=10');
    end;

    // Record states the rule does not admit.

    [Test]
    procedure State_FourFlowFields()
    var
        Head: Record "NB AC Head";
    begin
        Initialize();
        Head.SetRange(Kind, 'A');
        Head.SetAutoCalcFields("Line Total", "Line Count", "Min Amount", "Max Amount");
        Head.FindFirst();
        Check(Callee.Describe(Head), Dropped);
    end;

    [Test]
    procedure State_FilterOnFlowFilter()
    var
        Head: Record "NB AC Head";
    begin
        Initialize();
        Head.SetRange(Kind, 'A');
        Head.SetRange("Line Filter", 1);
        Head.SetAutoCalcFields("Line Total");
        Head.FindFirst();
        Check(Callee.Describe(Head), Dropped);
    end;

    [Test]
    procedure State_FilterInGroup4()
    var
        Head: Record "NB AC Head";
    begin
        Initialize();
        Head.SetRange(Kind, 'A');
        Head.FilterGroup(4);
        Head.SetFilter("No.", '<>%1', 'H9');
        Head.FilterGroup(0);
        Head.SetAutoCalcFields("Line Total");
        Head.FindFirst();
        Check(Callee.Describe(Head), Dropped);
    end;

    [Test]
    procedure State_CallerStandsInGroup2()
    var
        Head: Record "NB AC Head";
    begin
        Initialize();
        Head.FilterGroup(2);
        Head.SetRange(Kind, 'A');
        Head.SetAutoCalcFields("Line Total");
        Head.FindFirst();
        Check(Callee.Describe(Head), Dropped);
    end;

    [Test]
    procedure State_AdHocKey()
    var
        Head: Record "NB AC Head";
    begin
        Initialize();
        Head.SetCurrentKey(Note);
        Head.SetRange(Kind, 'A');
        Head.SetAutoCalcFields("Line Total");
        Head.FindFirst();
        Check(Callee.Describe(Head), Dropped);
    end;

    [Test]
    procedure State_FieldSortedDescending()
    var
        Head: Record "NB AC Head";
    begin
        Initialize();
        Head.SetCurrentKey(Kind);
        Head.SetAscending(Kind, false);
        Head.SetRange(Kind, 'A');
        Head.SetAutoCalcFields("Line Total");
        Head.FindFirst();
        Check(Callee.Describe(Head), 'entry=20 n=3 last=H3 lastTotal=0');
    end;

    [Test]
    procedure State_MarkedOnly()
    var
        Head: Record "NB AC Head";
    begin
        FilteredHead(Head);
        Head.Mark(true);
        Head.MarkedOnly(true);
        Check(Callee.Describe(Head), Dropped);
    end;

    [Test]
    procedure State_SetLoadFields()
    var
        Head: Record "NB AC Head";
    begin
        Initialize();
        Head.SetLoadFields(Kind);
        Head.SetRange(Kind, 'A');
        Head.SetAutoCalcFields("Line Total");
        Head.FindFirst();
        Check(Callee.Describe(Head), Dropped);
    end;

    [Test]
    procedure State_ReadIsolation()
    var
        Head: Record "NB AC Head";
    begin
        Initialize();
        Head.ReadIsolation := IsolationLevel::ReadCommitted;
        Head.SetRange(Kind, 'A');
        Head.SetAutoCalcFields("Line Total");
        Head.FindFirst();
        Check(Callee.Describe(Head), Dropped);
    end;

    [Test]
    procedure State_TemporarySource()
    var
        Head: Record "NB AC Head";
        TempHead: Record "NB AC Head" temporary;
    begin
        Initialize();
        Head.FindSet();
        repeat
            TempHead := Head;
            TempHead.Insert();
        until Head.Next() = 0;
        TempHead.SetRange(Kind, 'A');
        TempHead.SetAutoCalcFields("Line Total");
        TempHead.FindFirst();
        Check(Callee.Describe(TempHead), Dropped);
    end;

    // Callers and callees the rule does not admit.

    [Test]
    procedure Caller_ThirdTableProcedure()
    var
        Holder: Record "NB AC Holder";
    begin
        Initialize();
        Check(Holder.FromProcedure(), Dropped);
    end;

    [Test]
    procedure Caller_ThirdTableOnModify()
    var
        Holder: Record "NB AC Holder";
    begin
        Initialize();
        Holder."Line No." := 1;
        Holder.Insert(false);
        Holder.Modify(true);
        Check(Holder.Info, Dropped);
    end;

    [Test]
    procedure Caller_TableCodeCallsCodeunit()
    var
        Head: Record "NB AC Head";
    begin
        Initialize();
        Check(Head.DescribeThroughCodeunit(), Dropped);
    end;

    [Test]
    procedure Callee_Interface()
    var
        Head: Record "NB AC Head";
        Impl: Codeunit "NB AC Describer Impl";
        Describer: Interface "NB AC Describer";
    begin
        FilteredHead(Head);
        Describer := Impl;
        Check(Describer.Describe(Head), Dropped);
    end;

    [Test]
    procedure Callee_EventSubscriber()
    var
        Head: Record "NB AC Head";
        Events: Codeunit "NB AC Events";
        Result: Text;
    begin
        FilteredHead(Head);
        Events.OnDescribe(Head, Result);
        Check(Result, Dropped);
    end;
}
