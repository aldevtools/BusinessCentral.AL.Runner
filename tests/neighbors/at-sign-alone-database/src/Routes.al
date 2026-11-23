// A copy of the corpus probe record/filters/AtSignAloneReads.al of corpus pull request 1495 at ca3a9942 (its lines are the probe's
// lines plus two).
// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/record/record-setfilter-method
// Scope: in-scope
// Fixtures used: Assert (60021); own tables 97502, 97503 and 97505, report 97500, page 97500 and test codeunit 97500
// Probe (al-runner issue 8637, review of al-runner PR 8716): `@` alone and `<>@` alone on a Code and a Text field, read from the
//   database and from a temporary record. pages/background-tasks/BgTaskQueryCache.al (BQ Tests 9b, 9c) measures SetFilter read by
//   Count, a FlowFilter that CalcFields applies, a lookup FlowField's filter read by Count and a query's filters; this file
//   measures the other read methods and the other places a filter comes from. Every value asserted below is what a local
//   Business Central 29.1 OnPrem W1 container produced, 3 runs.
// Facts: the rows of a new group are 1 (K, Amount 10) and 2 (empty, Amount 5), the same in the Code and in the Text field.
//   A read of the database keeps the empty rows for `@` and the others for `<>@`: FindSet and Next, FindFirst, FindLast,
//   Find('-'), Find('+'), Find('<'), Count, IsEmpty, CalcSums, a descending FindSet, DeleteAll, ModifyAll, a report's data item
//   after SetTableView, a page opened on the record. Find('=') of a row of the database keeps every row for `@` and the others
//   for `<>@`; so do Find, FindFirst, FindLast, FindSet and Next with `@` or `<>@` on a lookup FlowField, where Count and IsEmpty
//   keep the empty rows for `@`. A temporary record keeps every row for `@` and none for `<>@`. Filter places: Record.SetFilter
//   in filter groups 0, 1 and 2, SetView, FieldRef.SetFilter, RecordRef.SetView, a CalcFormula's field(filter()) over a normal
//   field that holds `@` and over a FlowFilter set to `@`, a CalcFormula's filter('@') and filter('<>@'); GetFilter, GetFilters, GetView.
// BC versions: 27, 28, 29

table 97502 "AS Kind Row"
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

table 97503 "AS Kind Host"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Pattern; Text[30]) { }
        field(3; "Code Filter"; Code[20]) { FieldClass = FlowFilter; }
        field(4; "Text Filter"; Text[50]) { FieldClass = FlowFilter; }
        field(5; "Code By Pattern"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = sum("AS Kind Row".Amount where("Group" = field("No."), "Code Kind" = field(filter(Pattern))));
        }
        field(6; "Text By Pattern"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = sum("AS Kind Row".Amount where("Group" = field("No."), "Text Kind" = field(filter(Pattern))));
        }
        field(7; "Count Code By Pattern"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("AS Kind Row" where("Group" = field("No."), "Code Kind" = field(filter(Pattern))));
        }
        field(8; "Code At Const"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = sum("AS Kind Row".Amount where("Group" = field("No."), "Code Kind" = filter('@')));
        }
        field(9; "Text At Const"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = sum("AS Kind Row".Amount where("Group" = field("No."), "Text Kind" = filter('@')));
        }
        field(10; "Code By Code Filter"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = sum("AS Kind Row".Amount where("Group" = field("No."), "Code Kind" = field(filter("Code Filter"))));
        }
        field(11; "Text By Text Filter"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = sum("AS Kind Row".Amount where("Group" = field("No."), "Text Kind" = field(filter("Text Filter"))));
        }
        field(12; "Kind One Code"; Code[20])
        {
            FieldClass = FlowField;
            CalcFormula = lookup("AS Kind Row"."Code Kind" where("Group" = field("No."), Id = const(1)));
        }
        field(13; "Kind One Text"; Text[50])
        {
            FieldClass = FlowField;
            CalcFormula = lookup("AS Kind Row"."Text Kind" where("Group" = field("No."), Id = const(1)));
        }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

report 97500 "AS Kind Rows"
{
    ProcessingOnly = true;
    UseRequestPage = false;

    dataset
    {
        dataitem(Row; "AS Kind Row")
        {
            trigger OnAfterGetRecord()
            begin
                Seen += Format(Row.Id) + ',';
            end;
        }
    }

    var
        Seen: Text;

    procedure SeenIds(): Text
    begin
        exit(Seen);
    end;
}

page 97500 "AS Kind List"
{
    PageType = List;
    SourceTable = "AS Kind Row";
    UsageCategory = None;
    Editable = false;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field(Id; Rec.Id) { ApplicationArea = All; }
            }
        }
    }
}

codeunit 97500 "AS Kind Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;

    // A new group: row 1 (K, Amount 10) and, with WithEmpty, row 2 (empty, Amount 5), in the Code and the Text field.
    local procedure NewGroup(WithEmpty: Boolean): Code[20]
    var
        Row: Record "AS Kind Row";
    begin
        Row."Group" := CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 18);
        Row.Id := 1;
        Row."Code Kind" := 'K';
        Row."Text Kind" := 'K';
        Row.Amount := 10;
        Row.Insert();
        if WithEmpty then begin
            Row.Id := 2;
            Row."Code Kind" := '';
            Row."Text Kind" := '';
            Row.Amount := 5;
            Row.Insert();
        end;
        exit(Row."Group");
    end;

    // The rows of group G copied into the temporary record Temp.
    local procedure CopyGroup(G: Code[20]; var Temp: Record "AS Kind Row" temporary)
    var
        Row: Record "AS Kind Row";
    begin
        Row.SetRange("Group", G);
        if Row.FindSet() then
            repeat
                Temp := Row;
                Temp.Insert();
            until Row.Next() = 0;
    end;

    local procedure Ids(var R: Record "AS Kind Row"): Text
    var
        Seen: Text;
    begin
        if R.FindSet() then
            repeat
                Seen += Format(R.Id) + ',';
            until R.Next() = 0;
        exit(Seen);
    end;

    local procedure Found(Ok: Boolean; var R: Record "AS Kind Row"): Text
    begin
        if Ok then
            exit(Format(R.Id));
        exit('none');
    end;

    // What the read methods answer on R, whose filters are set.
    local procedure Reads(var R: Record "AS Kind Row"): Text
    var
        Seen: Text;
    begin
        Seen := 'set=' + Ids(R);
        Seen += '|first=' + Found(R.FindFirst(), R);
        Seen += '|last=' + Found(R.FindLast(), R);
        Seen += '|-=' + Found(R.Find('-'), R);
        Seen += '|+=' + Found(R.Find('+'), R);
        Seen += '|count=' + Format(R.Count());
        Seen += '|empty=' + Format(R.IsEmpty());
        R.CalcSums(Amount);
        Seen += '|sum=' + Format(R.Amount);
        R.Ascending(false);
        Seen += '|desc=' + Ids(R);
        R.Ascending(true);
        exit(Seen);
    end;

    // The read methods with @ alone on the Code and on the Text field of a record in the database: the empty row only.
    [Test]
    procedure AtSignAlone_ReadMethods_Database()
    var
        R: Record "AS Kind Row";
        Seen: Text;
    begin
        R.SetRange("Group", NewGroup(true));
        R.SetFilter("Code Kind", '@');
        Seen := Reads(R);
        R.SetRange("Code Kind");
        R.SetFilter("Text Kind", '@');
        Seen += ' / ' + Reads(R);
        Assert.AreEqual(
            'set=2,|first=2|last=2|-=2|+=2|count=1|empty=No|sum=5|desc=2, / set=2,|first=2|last=2|-=2|+=2|count=1|empty=No|sum=5|desc=2,',
            Seen, 'code / text');
    end;

    // The same reads of a temporary record: every row.
    [Test]
    procedure AtSignAlone_ReadMethods_Temporary()
    var
        Temp: Record "AS Kind Row" temporary;
        Seen: Text;
    begin
        CopyGroup(NewGroup(true), Temp);
        Temp.SetFilter("Code Kind", '@');
        Seen := Reads(Temp);
        Temp.SetRange("Code Kind");
        Temp.SetFilter("Text Kind", '@');
        Seen += ' / ' + Reads(Temp);
        Assert.AreEqual(
            'set=1,2,|first=1|last=2|-=1|+=2|count=2|empty=No|sum=15|desc=2,1, / set=1,2,|first=1|last=2|-=1|+=2|count=2|empty=No|sum=15|desc=2,1,',
            Seen, 'code / text');
    end;

    // IsEmpty of a group without an empty value: the database has no row for @, a temporary record has one.
    [Test]
    procedure AtSignAlone_IsEmpty_NoEmptyValue()
    var
        R: Record "AS Kind Row";
        Temp: Record "AS Kind Row" temporary;
        G: Code[20];
        Seen: Text;
    begin
        G := NewGroup(false);
        CopyGroup(G, Temp);
        R.SetRange("Group", G);
        R.SetFilter("Code Kind", '@');
        Seen := Format(R.IsEmpty());
        R.SetRange("Code Kind");
        R.SetFilter("Text Kind", '@');
        Seen += '|' + Format(R.IsEmpty());
        Temp.SetFilter("Code Kind", '@');
        Seen += '|' + Format(Temp.IsEmpty());
        Temp.SetRange("Code Kind");
        Temp.SetFilter("Text Kind", '@');
        Seen += '|' + Format(Temp.IsEmpty());
        Assert.AreEqual('Yes|Yes|No|No', Seen, 'database code, database text, temporary code, temporary text');
    end;

    // Find('=') on row 1 and on row 2, and Find('<') from row 2, with @ alone set after Get.
    local procedure FindAgain(var R: Record "AS Kind Row"; G: Code[20]; FieldNo: Integer): Text
    var
        Seen: Text;
    begin
        R.Get(G, 1);
        SetAt(R, FieldNo);
        Seen := 'eq1=' + Format(R.Find('='));
        R.Get(G, 2);
        Seen += '|eq2=' + Format(R.Find('='));
        R.Get(G, 2);
        Seen += '|lt2=' + Found(R.Find('<'), R);
        exit(Seen);
    end;

    local procedure SetAt(var R: Record "AS Kind Row"; FieldNo: Integer)
    begin
        R.SetRange("Code Kind");
        R.SetRange("Text Kind");
        if FieldNo = R.FieldNo("Code Kind") then
            R.SetFilter("Code Kind", '@')
        else
            R.SetFilter("Text Kind", '@');
    end;

    [Test]
    procedure AtSignAlone_FindEqualsAndPrevious()
    var
        R: Record "AS Kind Row";
        Temp: Record "AS Kind Row" temporary;
        G: Code[20];
        Seen: Text;
    begin
        G := NewGroup(true);
        CopyGroup(G, Temp);
        R.SetRange("Group", G);
        Seen := FindAgain(R, G, R.FieldNo("Code Kind"));
        Seen += ' / ' + FindAgain(R, G, R.FieldNo("Text Kind"));
        Seen += ' / ' + FindAgain(Temp, G, Temp.FieldNo("Code Kind"));
        Seen += ' / ' + FindAgain(Temp, G, Temp.FieldNo("Text Kind"));
        Assert.AreEqual(
            'eq1=Yes|eq2=Yes|lt2=none / eq1=Yes|eq2=Yes|lt2=none / eq1=Yes|eq2=Yes|lt2=1 / eq1=Yes|eq2=Yes|lt2=1',
            Seen, 'database code, database text, temporary code, temporary text');
    end;

    // A host whose lookup FlowFields read K, with @ alone on the lookup FlowField: Find('=') after Get, Find('-'), Find('+'),
    // FindFirst, FindLast, the rows FindSet and Next go through, Count and IsEmpty, of a host in the database and of a temporary
    // host.
    local procedure FindHost(var Host: Record "AS Kind Host"; FieldNo: Integer): Text
    var
        No: Code[20];
        N: Integer;
        Seen: Text;
    begin
        No := Host."No.";
        Host.SetRange("Kind One Code");
        Host.SetRange("Kind One Text");
        Host.Get(No);
        if FieldNo = Host.FieldNo("Kind One Code") then
            Host.SetFilter("Kind One Code", '@')
        else
            Host.SetFilter("Kind One Text", '@');
        Seen := 'eq=' + Format(Host.Find('='));
        Seen += '|-=' + Format(Host.Find('-'));
        Seen += '|+=' + Format(Host.Find('+'));
        Seen += '|first=' + Format(Host.FindFirst());
        Seen += '|last=' + Format(Host.FindLast());
        if Host.FindSet() then
            repeat
                N += 1;
            until Host.Next() = 0;
        Seen += '|set=' + Format(N);
        Seen += '|count=' + Format(Host.Count());
        Seen += '|empty=' + Format(Host.IsEmpty());
        exit(Seen);
    end;

    [Test]
    procedure AtSignAlone_FindEquals_LookupFlowFieldFilter()
    var
        Host: Record "AS Kind Host";
        TempHost: Record "AS Kind Host" temporary;
        Seen: Text;
    begin
        Host."No." := NewGroup(true);
        Host.Insert();
        TempHost := Host;
        TempHost.Insert();
        Host.SetRange("No.", Host."No.");
        TempHost.SetRange("No.", Host."No.");
        Seen := FindHost(Host, Host.FieldNo("Kind One Code"));
        Seen += ' / ' + FindHost(Host, Host.FieldNo("Kind One Text"));
        Seen += ' / ' + FindHost(TempHost, TempHost.FieldNo("Kind One Code"));
        Seen += ' / ' + FindHost(TempHost, TempHost.FieldNo("Kind One Text"));
        Assert.AreEqual(
            'eq=Yes|-=Yes|+=Yes|first=Yes|last=Yes|set=1|count=0|empty=Yes / eq=Yes|-=Yes|+=Yes|first=Yes|last=Yes|set=1|count=0|empty=Yes / ' +
            'eq=Yes|-=Yes|+=Yes|first=Yes|last=Yes|set=1|count=1|empty=No / eq=Yes|-=Yes|+=Yes|first=Yes|last=Yes|set=1|count=1|empty=No',
            Seen, 'database code, database text, temporary code, temporary text');
    end;

    // DeleteAll with @ alone, then the rows left in the group.
    local procedure DeleteAt(var R: Record "AS Kind Row"; FieldNo: Integer): Text
    begin
        SetAt(R, FieldNo);
        R.DeleteAll();
        R.SetRange("Code Kind");
        R.SetRange("Text Kind");
        exit(Ids(R));
    end;

    [Test]
    procedure AtSignAlone_DeleteAll()
    var
        R: Record "AS Kind Row";
        TempCode: Record "AS Kind Row" temporary;
        TempText: Record "AS Kind Row" temporary;
        Seen: Text;
    begin
        R.SetRange("Group", NewGroup(true));
        Seen := 'code=' + DeleteAt(R, R.FieldNo("Code Kind"));
        R.SetRange("Group", NewGroup(true));
        Seen += '|text=' + DeleteAt(R, R.FieldNo("Text Kind"));
        CopyGroup(NewGroup(true), TempCode);
        Seen += '|temp code=' + DeleteAt(TempCode, TempCode.FieldNo("Code Kind"));
        CopyGroup(NewGroup(true), TempText);
        Seen += '|temp text=' + DeleteAt(TempText, TempText.FieldNo("Text Kind"));
        Assert.AreEqual('code=1,|text=1,|temp code=|temp text=', Seen, 'the rows left');
    end;

    // ModifyAll of Amount to 1 with @ alone, then the sum of the group.
    local procedure ModifyAt(var R: Record "AS Kind Row"; FieldNo: Integer): Text
    begin
        SetAt(R, FieldNo);
        R.ModifyAll(Amount, 1);
        R.SetRange("Code Kind");
        R.SetRange("Text Kind");
        R.CalcSums(Amount);
        exit(Format(R.Amount));
    end;

    [Test]
    procedure AtSignAlone_ModifyAll()
    var
        R: Record "AS Kind Row";
        TempCode: Record "AS Kind Row" temporary;
        TempText: Record "AS Kind Row" temporary;
        Seen: Text;
    begin
        R.SetRange("Group", NewGroup(true));
        Seen := 'code=' + ModifyAt(R, R.FieldNo("Code Kind"));
        R.SetRange("Group", NewGroup(true));
        Seen += '|text=' + ModifyAt(R, R.FieldNo("Text Kind"));
        CopyGroup(NewGroup(true), TempCode);
        Seen += '|temp code=' + ModifyAt(TempCode, TempCode.FieldNo("Code Kind"));
        CopyGroup(NewGroup(true), TempText);
        Seen += '|temp text=' + ModifyAt(TempText, TempText.FieldNo("Text Kind"));
        Assert.AreEqual('code=11|text=11|temp code=2|temp text=2', Seen, 'the sum after ModifyAll');
    end;

    // @ alone set in filter group Group, read in filter group 0.
    local procedure GroupCount(var R: Record "AS Kind Row"; Group: Integer; FieldNo: Integer): Text
    begin
        R.FilterGroup(Group);
        SetAt(R, FieldNo);
        R.FilterGroup(0);
        exit(Format(R.Count()));
    end;

    [Test]
    procedure AtSignAlone_FilterGroupTwo()
    var
        R: Record "AS Kind Row";
        Temp: Record "AS Kind Row" temporary;
        G: Code[20];
        Seen: Text;
    begin
        G := NewGroup(true);
        CopyGroup(G, Temp);
        R.SetRange("Group", G);
        Seen := GroupCount(R, 2, R.FieldNo("Code Kind"));
        Seen += '|' + GroupCount(R, 2, R.FieldNo("Text Kind"));
        Seen += '|' + GroupCount(Temp, 2, Temp.FieldNo("Code Kind"));
        Seen += '|' + GroupCount(Temp, 2, Temp.FieldNo("Text Kind"));
        Assert.AreEqual('1|1|2|2', Seen, 'database code, database text, temporary code, temporary text');
    end;

    [Test]
    procedure AtSignAlone_FilterGroupOne()
    var
        R: Record "AS Kind Row";
        Seen: Text;
    begin
        R.SetRange("Group", NewGroup(true));
        Seen := GroupCount(R, 1, R.FieldNo("Code Kind"));
        Seen += '|' + GroupCount(R, 1, R.FieldNo("Text Kind"));
        R.FilterGroup(1);
        R.SetRange("Text Kind");
        R.FilterGroup(0);
        Assert.AreEqual('1|1', Seen, 'database code, database text');
    end;

    // SetView with FILTER(@) on the Code and on the Text field: Count and GetFilter.
    local procedure ViewAt(var R: Record "AS Kind Row"; G: Code[20]; FieldName: Text): Text
    begin
        R.SetView(StrSubstNo('WHERE(Group=CONST(%1),%2=FILTER(@))', G, FieldName));
        exit(Format(R.Count()) + ',' + R.GetFilter("Code Kind") + ',' + R.GetFilter("Text Kind"));
    end;

    [Test]
    procedure AtSignAlone_SetView()
    var
        R: Record "AS Kind Row";
        Temp: Record "AS Kind Row" temporary;
        G: Code[20];
        Seen: Text;
    begin
        G := NewGroup(true);
        CopyGroup(G, Temp);
        Seen := ViewAt(R, G, 'Code Kind');
        Seen += '|' + ViewAt(R, G, 'Text Kind');
        Seen += '|' + ViewAt(Temp, G, 'Code Kind');
        Seen += '|' + ViewAt(Temp, G, 'Text Kind');
        Assert.AreEqual('1,@,|1,,@|2,@,|2,,@', Seen, 'database code, database text, temporary code, temporary text');
    end;

    // FieldRef.SetFilter of @ alone on the Code and on the Text field, then RecordRef.SetView with FILTER(@): Count and GetFilter.
    local procedure RefReads(var Ref: RecordRef; G: Code[20]): Text
    var
        GroupRef: FieldRef;
        CodeRef: FieldRef;
        TextRef: FieldRef;
        Seen: Text;
    begin
        GroupRef := Ref.Field(1);
        CodeRef := Ref.Field(3);
        TextRef := Ref.Field(4);
        GroupRef.SetRange(G);
        CodeRef.SetFilter('@');
        Seen := Format(Ref.Count()) + ',' + CodeRef.GetFilter();
        CodeRef.SetRange();
        TextRef.SetFilter('@');
        Seen += ',' + Format(Ref.Count()) + ',' + TextRef.GetFilter();
        Ref.SetView(StrSubstNo('WHERE(Group=CONST(%1),Code Kind=FILTER(@))', G));
        Seen += ',' + Format(Ref.Count());
        Ref.SetView(StrSubstNo('WHERE(Group=CONST(%1),Text Kind=FILTER(@))', G));
        Seen += ',' + Format(Ref.Count());
        exit(Seen);
    end;

    [Test]
    procedure AtSignAlone_FieldRefAndRecordRefSetView()
    var
        Temp: Record "AS Kind Row" temporary;
        Ref: RecordRef;
        TempRef: RecordRef;
        G: Code[20];
        Seen: Text;
    begin
        G := NewGroup(true);
        CopyGroup(G, Temp);
        Ref.Open(Database::"AS Kind Row");
        Seen := RefReads(Ref, G);
        TempRef.GetTable(Temp);
        Seen += '|' + RefReads(TempRef, G);
        Assert.AreEqual('1,@,1,@,1,1|2,@,2,@,2,2', Seen, 'database, temporary');
    end;

    // A host whose Pattern holds @: field(filter(Pattern)) over the Code and the Text field (sum and count), of a host in the
    // database and of a temporary host; the source rows are in the database.
    [Test]
    procedure AtSignAlone_CalcFormulaFieldFilterOfPattern()
    var
        Host: Record "AS Kind Host";
        TempHost: Record "AS Kind Host" temporary;
        Seen: Text;
    begin
        Host."No." := NewGroup(true);
        Host.Pattern := '@';
        Host.Insert();
        TempHost := Host;
        TempHost.Insert();
        Host.CalcFields("Code By Pattern", "Text By Pattern", "Count Code By Pattern");
        Seen := Format(Host."Code By Pattern") + ',' + Format(Host."Text By Pattern") + ',' + Format(Host."Count Code By Pattern");
        TempHost.CalcFields("Code By Pattern", "Text By Pattern", "Count Code By Pattern");
        Seen += '|' + Format(TempHost."Code By Pattern") + ',' + Format(TempHost."Text By Pattern") + ',' +
            Format(TempHost."Count Code By Pattern");
        Assert.AreEqual('5,5,1|5,5,1', Seen, 'database host, temporary host');
    end;

    // filter('@') in a CalcFormula over the Code and the Text field, of a host in the database and of a temporary host.
    [Test]
    procedure AtSignAlone_CalcFormulaConstFilter()
    var
        Host: Record "AS Kind Host";
        TempHost: Record "AS Kind Host" temporary;
        Seen: Text;
    begin
        Host."No." := NewGroup(true);
        Host.Insert();
        TempHost := Host;
        TempHost.Insert();
        Host.CalcFields("Code At Const", "Text At Const");
        Seen := Format(Host."Code At Const") + ',' + Format(Host."Text At Const");
        TempHost.CalcFields("Code At Const", "Text At Const");
        Seen += '|' + Format(TempHost."Code At Const") + ',' + Format(TempHost."Text At Const");
        Assert.AreEqual('5,5|5,5', Seen, 'database host, temporary host');
    end;

    // field(filter()) over a Code and a Text FlowFilter set to @ alone.
    [Test]
    procedure AtSignAlone_CalcFormulaFieldFilterOfFlowFilter()
    var
        Host: Record "AS Kind Host";
    begin
        Host."No." := NewGroup(true);
        Host.Insert();
        Host.SetFilter("Code Filter", '@');
        Host.SetFilter("Text Filter", '@');
        Host.CalcFields("Code By Code Filter", "Text By Text Filter");
        Assert.AreEqual('5,5', Format(Host."Code By Code Filter") + ',' + Format(Host."Text By Text Filter"), 'code, text');
    end;

    // A report's data item reads the rows of a view set by SetTableView with @ alone on the Code and on the Text field.
    local procedure ReportIds(G: Code[20]; FieldNo: Integer): Text
    var
        R: Record "AS Kind Row";
        Rows: Report "AS Kind Rows";
    begin
        R.SetRange("Group", G);
        SetAt(R, FieldNo);
        Rows.SetTableView(R);
        Rows.UseRequestPage(false);
        Rows.RunModal();
        exit(Rows.SeenIds());
    end;

    [Test]
    procedure AtSignAlone_ReportDataItem()
    var
        R: Record "AS Kind Row";
        G: Code[20];
        Seen: Text;
    begin
        G := NewGroup(true);
        Commit();
        Seen := ReportIds(G, R.FieldNo("Code Kind"));
        Seen += '|' + ReportIds(G, R.FieldNo("Text Kind"));
        Assert.AreEqual('2,|2,', Seen, 'code, text');
    end;

    // A page opened by Page.Run on a record with @ alone on the Code and on the Text field: the rows the test page goes through.
    local procedure PageIds(G: Code[20]; FieldNo: Integer): Text
    var
        R: Record "AS Kind Row";
        List: TestPage "AS Kind List";
        Seen: Text;
    begin
        R.SetRange("Group", G);
        SetAt(R, FieldNo);
        List.Trap();
        Page.Run(Page::"AS Kind List", R);
        if List.First() then
            repeat
                Seen += List.Id.Value() + ',';
            until not List.Next();
        List.Close();
        exit(Seen);
    end;

    [Test]
    procedure AtSignAlone_PageRows()
    var
        R: Record "AS Kind Row";
        G: Code[20];
        Seen: Text;
    begin
        G := NewGroup(true);
        Seen := PageIds(G, R.FieldNo("Code Kind"));
        Seen += '|' + PageIds(G, R.FieldNo("Text Kind"));
        Assert.AreEqual('2,|2,', Seen, 'code, text');
    end;

    // <>@ alone on the Code and on the Text field: the rows FindSet goes through and Count, then Find('=') on row 1 and on row 2
    // with <>@ set after Get, in the database and in a temporary record.
    local procedure SetNotAt(var R: Record "AS Kind Row"; FieldNo: Integer)
    begin
        R.SetRange("Code Kind");
        R.SetRange("Text Kind");
        if FieldNo = R.FieldNo("Code Kind") then
            R.SetFilter("Code Kind", '<>@')
        else
            R.SetFilter("Text Kind", '<>@');
    end;

    local procedure NotAt(var R: Record "AS Kind Row"; G: Code[20]; FieldNo: Integer): Text
    var
        Seen: Text;
    begin
        SetNotAt(R, FieldNo);
        Seen := Ids(R) + Format(R.Count());
        R.Get(G, 1);
        SetNotAt(R, FieldNo);
        Seen += ',eq1=' + Format(R.Find('='));
        R.Get(G, 2);
        SetNotAt(R, FieldNo);
        Seen += ',eq2=' + Format(R.Find('='));
        exit(Seen);
    end;

    [Test]
    procedure AtSignAloneNegated_DatabaseAndTemporary()
    var
        R: Record "AS Kind Row";
        Temp: Record "AS Kind Row" temporary;
        G: Code[20];
        Seen: Text;
    begin
        G := NewGroup(true);
        CopyGroup(G, Temp);
        R.SetRange("Group", G);
        Seen := NotAt(R, G, R.FieldNo("Code Kind"));
        Seen += '|' + NotAt(R, G, R.FieldNo("Text Kind"));
        Seen += '|' + NotAt(Temp, G, Temp.FieldNo("Code Kind"));
        Seen += '|' + NotAt(Temp, G, Temp.FieldNo("Text Kind"));
        Assert.AreEqual('1,1,eq1=Yes,eq2=No|1,1,eq1=Yes,eq2=No|0,eq1=No,eq2=No|0,eq1=No,eq2=No', Seen,
            'database code, database text, temporary code, temporary text');
    end;

    // <>@ alone on a lookup FlowField whose value is K: Count, IsEmpty and FindFirst, of a host in the database and of a temporary
    // host.
    local procedure NotAtHost(var Host: Record "AS Kind Host"; FieldNo: Integer): Text
    begin
        Host.SetRange("Kind One Code");
        Host.SetRange("Kind One Text");
        if FieldNo = Host.FieldNo("Kind One Code") then
            Host.SetFilter("Kind One Code", '<>@')
        else
            Host.SetFilter("Kind One Text", '<>@');
        exit(Format(Host.Count()) + ',' + Format(Host.IsEmpty()) + ',' + Format(Host.FindFirst()));
    end;

    [Test]
    procedure AtSignAloneNegated_LookupFlowFieldFilter()
    var
        Host: Record "AS Kind Host";
        TempHost: Record "AS Kind Host" temporary;
        Seen: Text;
    begin
        Host."No." := NewGroup(true);
        Host.Insert();
        TempHost := Host;
        TempHost.Insert();
        Host.SetRange("No.", Host."No.");
        TempHost.SetRange("No.", Host."No.");
        Seen := NotAtHost(Host, Host.FieldNo("Kind One Code"));
        Seen += '|' + NotAtHost(Host, Host.FieldNo("Kind One Text"));
        Seen += '|' + NotAtHost(TempHost, TempHost.FieldNo("Kind One Code"));
        Seen += '|' + NotAtHost(TempHost, TempHost.FieldNo("Kind One Text"));
        Assert.AreEqual('1,No,Yes|1,No,Yes|0,Yes,No|0,Yes,No', Seen, 'database code, database text, temporary code, temporary text');
    end;

    // A CalcFormula's quoted filter('<>@') over the Code and the Text field (table 97505), of a host in the database and of a
    // temporary host. An unquoted filter(@) does not compile (AL0870).
    [Test]
    procedure AtSignAloneNegated_CalcFormulaQuotedFilter()
    var
        Host: Record "AS Kind Filter Host";
        TempHost: Record "AS Kind Filter Host" temporary;
        Seen: Text;
    begin
        Host."No." := NewGroup(true);
        Host.Insert();
        TempHost := Host;
        TempHost.Insert();
        Host.CalcFields("Code Not At", "Text Not At");
        Seen := Format(Host."Code Not At") + ',' + Format(Host."Text Not At");
        TempHost.CalcFields("Code Not At", "Text Not At");
        Seen += '|' + Format(TempHost."Code Not At") + ',' + Format(TempHost."Text Not At");
        Assert.AreEqual('10,10|10,10', Seen, 'database host, temporary host');
    end;

    // <>@ alone by FieldRef.SetFilter, by SetView with FILTER(<>@) (Count and GetFilter) and by RecordRef.SetView, on the Code and
    // on the Text field.
    local procedure NotAtRefs(var R: Record "AS Kind Row"; G: Code[20]; FieldName: Text; FieldNo: Integer): Text
    var
        Ref: RecordRef;
        GroupRef: FieldRef;
        KindRef: FieldRef;
        Seen: Text;
    begin
        Ref.GetTable(R);
        GroupRef := Ref.Field(1);
        KindRef := Ref.Field(FieldNo);
        GroupRef.SetRange(G);
        KindRef.SetFilter('<>@');
        Seen := Format(Ref.Count()) + ',' + KindRef.GetFilter();
        Ref.SetView(StrSubstNo('WHERE(Group=CONST(%1),%2=FILTER(<>@))', G, FieldName));
        Seen += ',' + Format(Ref.Count());
        R.SetView(StrSubstNo('WHERE(Group=CONST(%1),%2=FILTER(<>@))', G, FieldName));
        Seen += ',' + Format(R.Count()) + ',' + R.GetFilter("Code Kind") + ',' + R.GetFilter("Text Kind");
        exit(Seen);
    end;

    [Test]
    procedure AtSignAloneNegated_FieldRefAndSetView()
    var
        R: Record "AS Kind Row";
        Temp: Record "AS Kind Row" temporary;
        G: Code[20];
        Seen: Text;
    begin
        G := NewGroup(true);
        CopyGroup(G, Temp);
        Seen := NotAtRefs(R, G, 'Code Kind', R.FieldNo("Code Kind"));
        Seen += '|' + NotAtRefs(R, G, 'Text Kind', R.FieldNo("Text Kind"));
        Seen += '|' + NotAtRefs(Temp, G, 'Code Kind', Temp.FieldNo("Code Kind"));
        Seen += '|' + NotAtRefs(Temp, G, 'Text Kind', Temp.FieldNo("Text Kind"));
        Assert.AreEqual('1,<>@,1,1,<>@,|1,<>@,1,1,,<>@|0,<>@,0,0,<>@,|0,<>@,0,0,,<>@', Seen,
            'database code, database text, temporary code, temporary text');
    end;

    // GetFilter, GetFilters and GetView of @ alone and of <>@ alone on the Code and on the Text field.
    local procedure ReadBack(Filter: Text; FieldNo: Integer): Text
    var
        R: Record "AS Kind Row";
    begin
        if FieldNo = R.FieldNo("Code Kind") then begin
            R.SetFilter("Code Kind", Filter);
            exit(R.GetFilter("Code Kind") + ' # ' + R.GetFilters() + ' # ' + R.GetView());
        end;
        R.SetFilter("Text Kind", Filter);
        exit(R.GetFilter("Text Kind") + ' # ' + R.GetFilters() + ' # ' + R.GetView());
    end;

    [Test]
    procedure AtSignAlone_GetFilterGetFiltersGetView()
    var
        R: Record "AS Kind Row";
        Seen: Text;
    begin
        Seen := ReadBack('@', R.FieldNo("Code Kind"));
        Seen += ' | ' + ReadBack('<>@', R.FieldNo("Code Kind"));
        Seen += ' | ' + ReadBack('@', R.FieldNo("Text Kind"));
        Seen += ' | ' + ReadBack('<>@', R.FieldNo("Text Kind"));
        Assert.AreEqual(
            '@ # Code Kind: @ # VERSION(1) SORTING(Group,Id) WHERE(Code Kind=FILTER(@)) | ' +
            '<>@ # Code Kind: <>@ # VERSION(1) SORTING(Group,Id) WHERE(Code Kind=FILTER(<>@)) | ' +
            '@ # Text Kind: @ # VERSION(1) SORTING(Group,Id) WHERE(Text Kind=FILTER(@)) | ' +
            '<>@ # Text Kind: <>@ # VERSION(1) SORTING(Group,Id) WHERE(Text Kind=FILTER(<>@))',
            Seen, 'code @, code <>@, text @, text <>@');
    end;
}

// The CalcFormulas of AtSignAloneNegated_CalcFormulaQuotedFilter: a quoted filter('<>@').
table 97505 "AS Kind Filter Host"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; "Code Not At"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = sum("AS Kind Row".Amount where("Group" = field("No."), "Code Kind" = filter('<>@')));
        }
        field(3; "Text Not At"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = sum("AS Kind Row".Amount where("Group" = field("No."), "Text Kind" = filter('<>@')));
        }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}
