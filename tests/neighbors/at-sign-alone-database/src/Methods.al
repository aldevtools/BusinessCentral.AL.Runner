// A copy of the corpus probe record/filters/AtSignAloneMethods.al of corpus pull request 1495 at 60048e66 (its lines are the probe's
// lines plus two).
// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/properties/devenv-calcformula-property
// Scope: in-scope
// Fixtures used: Assert (60021); own tables 97506 and 97507, report 97503, page 97503 and test codeunit 97501
// Probe (al-runner issue 8637, review of al-runner PR 8816): the readers of `<>@` alone that record/filters/AtSignAloneReads.al
//   leaves open (DeleteAll, ModifyAll, CalcSums, FindLast, a report's data item, a page), and a CalcFormula's quoted filter('@')
//   and filter('<>@') for every method and for source fields that are not Code or Text. Every value asserted below is what a local
//   Business Central 29.1 OnPrem W1 container produced, 3 runs.
// Facts: the rows of a new group are 1 (K, Amount 10) and 2 (empty, Amount 5). `<>@` alone on a Code or a Text field keeps the K
//   row for every reader of the database and no row of a temporary record. A CalcFormula's filter('@') keeps the empty row and
//   filter('<>@') the K row for count, exist, lookup, min, max and average as for sum, for a host in the database and a temporary
//   host. Over a Date source field filter('@') keeps no row and filter('<>@') every row; over an Integer, Decimal, Option or
//   Boolean source field CalcFields raises an error.
// BC versions: 27, 28, 29

table 97506 "AM Kind Row"
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
        field(6; Num; Integer) { }
        field(7; Opt; Option) { OptionMembers = A,B; }
        field(8; Day; Date) { }
        field(9; Flag; Boolean) { }
    }

    keys
    {
        key(PK; "Group", Id) { Clustered = true; }
    }
}

table 97507 "AM Formula Host"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(10; "Count Code At"; Integer) { FieldClass = FlowField; CalcFormula = count("AM Kind Row" where("Group" = field("No."), "Code Kind" = filter('@'))); }
        field(11; "Count Code Not At"; Integer) { FieldClass = FlowField; CalcFormula = count("AM Kind Row" where("Group" = field("No."), "Code Kind" = filter('<>@'))); }
        field(12; "Count Text At"; Integer) { FieldClass = FlowField; CalcFormula = count("AM Kind Row" where("Group" = field("No."), "Text Kind" = filter('@'))); }
        field(13; "Count Text Not At"; Integer) { FieldClass = FlowField; CalcFormula = count("AM Kind Row" where("Group" = field("No."), "Text Kind" = filter('<>@'))); }
        field(20; "Exist Code At"; Boolean) { FieldClass = FlowField; CalcFormula = exist("AM Kind Row" where("Group" = field("No."), "Code Kind" = filter('@'))); }
        field(21; "Exist Code Not At"; Boolean) { FieldClass = FlowField; CalcFormula = exist("AM Kind Row" where("Group" = field("No."), "Code Kind" = filter('<>@'))); }
        field(22; "Exist Text At"; Boolean) { FieldClass = FlowField; CalcFormula = exist("AM Kind Row" where("Group" = field("No."), "Text Kind" = filter('@'))); }
        field(23; "Exist Text Not At"; Boolean) { FieldClass = FlowField; CalcFormula = exist("AM Kind Row" where("Group" = field("No."), "Text Kind" = filter('<>@'))); }
        field(30; "Lookup Code At"; Decimal) { FieldClass = FlowField; CalcFormula = lookup("AM Kind Row".Amount where("Group" = field("No."), "Code Kind" = filter('@'))); }
        field(31; "Lookup Code Not At"; Decimal) { FieldClass = FlowField; CalcFormula = lookup("AM Kind Row".Amount where("Group" = field("No."), "Code Kind" = filter('<>@'))); }
        field(32; "Lookup Text At"; Decimal) { FieldClass = FlowField; CalcFormula = lookup("AM Kind Row".Amount where("Group" = field("No."), "Text Kind" = filter('@'))); }
        field(33; "Lookup Text Not At"; Decimal) { FieldClass = FlowField; CalcFormula = lookup("AM Kind Row".Amount where("Group" = field("No."), "Text Kind" = filter('<>@'))); }
        field(40; "Min Code At"; Decimal) { FieldClass = FlowField; CalcFormula = min("AM Kind Row".Amount where("Group" = field("No."), "Code Kind" = filter('@'))); }
        field(41; "Min Code Not At"; Decimal) { FieldClass = FlowField; CalcFormula = min("AM Kind Row".Amount where("Group" = field("No."), "Code Kind" = filter('<>@'))); }
        field(42; "Min Text At"; Decimal) { FieldClass = FlowField; CalcFormula = min("AM Kind Row".Amount where("Group" = field("No."), "Text Kind" = filter('@'))); }
        field(43; "Min Text Not At"; Decimal) { FieldClass = FlowField; CalcFormula = min("AM Kind Row".Amount where("Group" = field("No."), "Text Kind" = filter('<>@'))); }
        field(50; "Max Code At"; Decimal) { FieldClass = FlowField; CalcFormula = max("AM Kind Row".Amount where("Group" = field("No."), "Code Kind" = filter('@'))); }
        field(51; "Max Code Not At"; Decimal) { FieldClass = FlowField; CalcFormula = max("AM Kind Row".Amount where("Group" = field("No."), "Code Kind" = filter('<>@'))); }
        field(52; "Max Text At"; Decimal) { FieldClass = FlowField; CalcFormula = max("AM Kind Row".Amount where("Group" = field("No."), "Text Kind" = filter('@'))); }
        field(53; "Max Text Not At"; Decimal) { FieldClass = FlowField; CalcFormula = max("AM Kind Row".Amount where("Group" = field("No."), "Text Kind" = filter('<>@'))); }
        field(60; "Average Code At"; Decimal) { FieldClass = FlowField; CalcFormula = average("AM Kind Row".Amount where("Group" = field("No."), "Code Kind" = filter('@'))); }
        field(61; "Average Code Not At"; Decimal) { FieldClass = FlowField; CalcFormula = average("AM Kind Row".Amount where("Group" = field("No."), "Code Kind" = filter('<>@'))); }
        field(62; "Average Text At"; Decimal) { FieldClass = FlowField; CalcFormula = average("AM Kind Row".Amount where("Group" = field("No."), "Text Kind" = filter('@'))); }
        field(63; "Average Text Not At"; Decimal) { FieldClass = FlowField; CalcFormula = average("AM Kind Row".Amount where("Group" = field("No."), "Text Kind" = filter('<>@'))); }
        field(70; "Num At"; Decimal) { FieldClass = FlowField; CalcFormula = sum("AM Kind Row".Amount where("Group" = field("No."), Num = filter('@'))); }
        field(71; "Num Not At"; Decimal) { FieldClass = FlowField; CalcFormula = sum("AM Kind Row".Amount where("Group" = field("No."), Num = filter('<>@'))); }
        field(72; "Amount At"; Decimal) { FieldClass = FlowField; CalcFormula = sum("AM Kind Row".Amount where("Group" = field("No."), Amount = filter('@'))); }
        field(73; "Amount Not At"; Decimal) { FieldClass = FlowField; CalcFormula = sum("AM Kind Row".Amount where("Group" = field("No."), Amount = filter('<>@'))); }
        field(74; "Opt At"; Decimal) { FieldClass = FlowField; CalcFormula = sum("AM Kind Row".Amount where("Group" = field("No."), Opt = filter('@'))); }
        field(75; "Opt Not At"; Decimal) { FieldClass = FlowField; CalcFormula = sum("AM Kind Row".Amount where("Group" = field("No."), Opt = filter('<>@'))); }
        field(76; "Day At"; Decimal) { FieldClass = FlowField; CalcFormula = sum("AM Kind Row".Amount where("Group" = field("No."), Day = filter('@'))); }
        field(77; "Day Not At"; Decimal) { FieldClass = FlowField; CalcFormula = sum("AM Kind Row".Amount where("Group" = field("No."), Day = filter('<>@'))); }
        field(78; "Flag At"; Decimal) { FieldClass = FlowField; CalcFormula = sum("AM Kind Row".Amount where("Group" = field("No."), Flag = filter('@'))); }
        field(79; "Flag Not At"; Decimal) { FieldClass = FlowField; CalcFormula = sum("AM Kind Row".Amount where("Group" = field("No."), Flag = filter('<>@'))); }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

report 97503 "AM Kind Rows"
{
    ProcessingOnly = true;
    UseRequestPage = false;

    dataset
    {
        dataitem(Row; "AM Kind Row")
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

page 97503 "AM Kind List"
{
    PageType = List;
    SourceTable = "AM Kind Row";
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

codeunit 97501 "AM Kind Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;

    // A new group: row 1 (K, Amount 10, Num 1, Opt B, a date, Flag true) and row 2 (empty, Amount 5, Num 0, Opt A, 0D, false).
    local procedure NewGroup(): Code[20]
    var
        Row: Record "AM Kind Row";
    begin
        Row."Group" := CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 18);
        Row.Id := 1;
        Row."Code Kind" := 'K';
        Row."Text Kind" := 'K';
        Row.Amount := 10;
        Row.Num := 1;
        Row.Opt := Row.Opt::B;
        Row.Day := 20260101D;
        Row.Flag := true;
        Row.Insert();
        Row.Id := 2;
        Row."Code Kind" := '';
        Row."Text Kind" := '';
        Row.Amount := 5;
        Row.Num := 0;
        Row.Opt := Row.Opt::A;
        Row.Day := 0D;
        Row.Flag := false;
        Row.Insert();
        exit(Row."Group");
    end;

    local procedure CopyGroup(G: Code[20]; var Temp: Record "AM Kind Row" temporary)
    var
        Row: Record "AM Kind Row";
    begin
        Row.SetRange("Group", G);
        if Row.FindSet() then
            repeat
                Temp := Row;
                Temp.Insert();
            until Row.Next() = 0;
    end;

    local procedure Ids(var R: Record "AM Kind Row"): Text
    var
        Seen: Text;
    begin
        if R.FindSet() then
            repeat
                Seen += Format(R.Id) + ',';
            until R.Next() = 0;
        exit(Seen);
    end;

    local procedure SetNotAt(var R: Record "AM Kind Row"; FieldNo: Integer)
    begin
        if FieldNo = R.FieldNo("Code Kind") then
            R.SetFilter("Code Kind", '<>@')
        else
            R.SetFilter("Text Kind", '<>@');
    end;

    local procedure ClearKinds(var R: Record "AM Kind Row")
    begin
        R.SetRange("Code Kind");
        R.SetRange("Text Kind");
    end;

    // DeleteAll with <>@ alone, then the rows left in the group.
    local procedure DeleteNotAt(var R: Record "AM Kind Row"; FieldNo: Integer): Text
    begin
        SetNotAt(R, FieldNo);
        R.DeleteAll();
        ClearKinds(R);
        exit(Ids(R));
    end;

    [Test]
    procedure AtSignAloneNegated_DeleteAll()
    var
        R: Record "AM Kind Row";
        TempCode: Record "AM Kind Row" temporary;
        TempText: Record "AM Kind Row" temporary;
        Seen: Text;
    begin
        R.SetRange("Group", NewGroup());
        Seen := 'code=' + DeleteNotAt(R, R.FieldNo("Code Kind"));
        R.SetRange("Group", NewGroup());
        Seen += '|text=' + DeleteNotAt(R, R.FieldNo("Text Kind"));
        CopyGroup(NewGroup(), TempCode);
        Seen += '|temp code=' + DeleteNotAt(TempCode, TempCode.FieldNo("Code Kind"));
        CopyGroup(NewGroup(), TempText);
        Seen += '|temp text=' + DeleteNotAt(TempText, TempText.FieldNo("Text Kind"));
        Assert.AreEqual('code=2,|text=2,|temp code=1,2,|temp text=1,2,', Seen, 'the rows left');
    end;

    // ModifyAll of Amount to 1 with <>@ alone, then the sum of the group.
    local procedure ModifyNotAt(var R: Record "AM Kind Row"; FieldNo: Integer): Text
    begin
        SetNotAt(R, FieldNo);
        R.ModifyAll(Amount, 1);
        ClearKinds(R);
        R.CalcSums(Amount);
        exit(Format(R.Amount));
    end;

    [Test]
    procedure AtSignAloneNegated_ModifyAll()
    var
        R: Record "AM Kind Row";
        TempCode: Record "AM Kind Row" temporary;
        TempText: Record "AM Kind Row" temporary;
        Seen: Text;
    begin
        R.SetRange("Group", NewGroup());
        Seen := 'code=' + ModifyNotAt(R, R.FieldNo("Code Kind"));
        R.SetRange("Group", NewGroup());
        Seen += '|text=' + ModifyNotAt(R, R.FieldNo("Text Kind"));
        CopyGroup(NewGroup(), TempCode);
        Seen += '|temp code=' + ModifyNotAt(TempCode, TempCode.FieldNo("Code Kind"));
        CopyGroup(NewGroup(), TempText);
        Seen += '|temp text=' + ModifyNotAt(TempText, TempText.FieldNo("Text Kind"));
        Assert.AreEqual('code=6|text=6|temp code=15|temp text=15', Seen, 'the sum after ModifyAll');
    end;

    // CalcSums and FindLast with <>@ alone.
    local procedure SumLastNotAt(var R: Record "AM Kind Row"; FieldNo: Integer): Text
    var
        Seen: Text;
    begin
        SetNotAt(R, FieldNo);
        R.CalcSums(Amount);
        Seen := Format(R.Amount);
        if R.FindLast() then
            Seen += ',last=' + Format(R.Id)
        else
            Seen += ',last=none';
        ClearKinds(R);
        exit(Seen);
    end;

    [Test]
    procedure AtSignAloneNegated_CalcSumsFindLast()
    var
        R: Record "AM Kind Row";
        Temp: Record "AM Kind Row" temporary;
        G: Code[20];
        Seen: Text;
    begin
        G := NewGroup();
        CopyGroup(G, Temp);
        R.SetRange("Group", G);
        Seen := SumLastNotAt(R, R.FieldNo("Code Kind"));
        Seen += '|' + SumLastNotAt(R, R.FieldNo("Text Kind"));
        Seen += '|' + SumLastNotAt(Temp, Temp.FieldNo("Code Kind"));
        Seen += '|' + SumLastNotAt(Temp, Temp.FieldNo("Text Kind"));
        Assert.AreEqual('10,last=1|10,last=1|0,last=none|0,last=none', Seen,
            'database code, database text, temporary code, temporary text');
    end;

    // A report's data item reads the rows of a view set by SetTableView with <>@ alone.
    local procedure ReportIds(G: Code[20]; FieldNo: Integer): Text
    var
        R: Record "AM Kind Row";
        Rows: Report "AM Kind Rows";
    begin
        R.SetRange("Group", G);
        SetNotAt(R, FieldNo);
        Rows.SetTableView(R);
        Rows.UseRequestPage(false);
        Rows.RunModal();
        exit(Rows.SeenIds());
    end;

    [Test]
    procedure AtSignAloneNegated_ReportDataItem()
    var
        R: Record "AM Kind Row";
        G: Code[20];
        Seen: Text;
    begin
        G := NewGroup();
        Commit();
        Seen := ReportIds(G, R.FieldNo("Code Kind"));
        Seen += '|' + ReportIds(G, R.FieldNo("Text Kind"));
        Assert.AreEqual('1,|1,', Seen, 'code, text');
    end;

    // A page opened by Page.Run on a record with <>@ alone: the rows the test page goes through.
    local procedure PageIds(G: Code[20]; FieldNo: Integer): Text
    var
        R: Record "AM Kind Row";
        List: TestPage "AM Kind List";
        Seen: Text;
    begin
        R.SetRange("Group", G);
        SetNotAt(R, FieldNo);
        List.Trap();
        Page.Run(Page::"AM Kind List", R);
        if List.First() then
            repeat
                Seen += List.Id.Value() + ',';
            until not List.Next();
        List.Close();
        exit(Seen);
    end;

    [Test]
    procedure AtSignAloneNegated_PageRows()
    var
        R: Record "AM Kind Row";
        G: Code[20];
        Seen: Text;
    begin
        G := NewGroup();
        Seen := PageIds(G, R.FieldNo("Code Kind"));
        Seen += '|' + PageIds(G, R.FieldNo("Text Kind"));
        Assert.AreEqual('1,|1,', Seen, 'code, text');
    end;

    // count, exist, lookup, min, max and average with filter('@') and filter('<>@') over the Code and the Text field.
    local procedure Methods(var Host: Record "AM Formula Host"): Text
    begin
        Host.CalcFields("Count Code At", "Count Code Not At", "Count Text At", "Count Text Not At");
        Host.CalcFields("Exist Code At", "Exist Code Not At", "Exist Text At", "Exist Text Not At");
        Host.CalcFields("Lookup Code At", "Lookup Code Not At", "Lookup Text At", "Lookup Text Not At");
        Host.CalcFields("Min Code At", "Min Code Not At", "Min Text At", "Min Text Not At");
        Host.CalcFields("Max Code At", "Max Code Not At", "Max Text At", "Max Text Not At");
        Host.CalcFields("Average Code At", "Average Code Not At", "Average Text At", "Average Text Not At");
        exit(
            StrSubstNo('count=%1,%2,%3,%4', Host."Count Code At", Host."Count Code Not At", Host."Count Text At", Host."Count Text Not At") +
            StrSubstNo('|exist=%1,%2,%3,%4', Host."Exist Code At", Host."Exist Code Not At", Host."Exist Text At", Host."Exist Text Not At") +
            StrSubstNo('|lookup=%1,%2,%3,%4', Host."Lookup Code At", Host."Lookup Code Not At", Host."Lookup Text At", Host."Lookup Text Not At") +
            StrSubstNo('|min=%1,%2,%3,%4', Host."Min Code At", Host."Min Code Not At", Host."Min Text At", Host."Min Text Not At") +
            StrSubstNo('|max=%1,%2,%3,%4', Host."Max Code At", Host."Max Code Not At", Host."Max Text At", Host."Max Text Not At") +
            StrSubstNo('|average=%1,%2,%3,%4', Host."Average Code At", Host."Average Code Not At", Host."Average Text At", Host."Average Text Not At"));
    end;

    [Test]
    procedure AtSignAlone_CalcFormulaMethods()
    var
        Host: Record "AM Formula Host";
        TempHost: Record "AM Formula Host" temporary;
        Seen: Text;
    begin
        Host."No." := NewGroup();
        Host.Insert();
        TempHost := Host;
        TempHost.Insert();
        Seen := Methods(Host) + ' / ' + Methods(TempHost);
        Assert.AreEqual(
            'count=1,1,1,1|exist=Yes,Yes,Yes,Yes|lookup=5,10,5,10|min=5,10,5,10|max=5,10,5,10|average=5,10,5,10 / ' +
            'count=1,1,1,1|exist=Yes,Yes,Yes,Yes|lookup=5,10,5,10|min=5,10,5,10|max=5,10,5,10|average=5,10,5,10',
            Seen, 'database host / temporary host; code @, code <>@, text @, text <>@');
    end;

    // filter('@') and filter('<>@') over a Date source field: no row, every row.
    [Test]
    procedure AtSignAlone_CalcFormulaDateSource()
    var
        Host: Record "AM Formula Host";
    begin
        Host."No." := NewGroup();
        Host.Insert();
        Host.CalcFields("Day At", "Day Not At");
        Assert.AreEqual('0,15', Format(Host."Day At") + ',' + Format(Host."Day Not At"), 'date @, date <>@');
    end;

    // filter('@') and filter('<>@') over an Integer, a Decimal, an Option and a Boolean source field: CalcFields raises an error.
    local procedure CalcError(var Host: Record "AM Formula Host"; FieldNo: Integer): Text
    begin
        case FieldNo of
            Host.FieldNo("Num At"):
                asserterror Host.CalcFields("Num At");
            Host.FieldNo("Num Not At"):
                asserterror Host.CalcFields("Num Not At");
            Host.FieldNo("Amount At"):
                asserterror Host.CalcFields("Amount At");
            Host.FieldNo("Amount Not At"):
                asserterror Host.CalcFields("Amount Not At");
            Host.FieldNo("Opt At"):
                asserterror Host.CalcFields("Opt At");
            Host.FieldNo("Opt Not At"):
                asserterror Host.CalcFields("Opt Not At");
            Host.FieldNo("Flag At"):
                asserterror Host.CalcFields("Flag At");
            Host.FieldNo("Flag Not At"):
                asserterror Host.CalcFields("Flag Not At");
        end;
        exit(GetLastErrorText());
    end;

    [Test]
    procedure AtSignAlone_CalcFormulaOtherSources()
    var
        Host: Record "AM Formula Host";
    begin
        Host."No." := NewGroup();
        Host.Insert();
        Assert.AreEqual('The value "@" can''t be evaluated into type Integer.', CalcError(Host, Host.FieldNo("Num At")), 'integer @');
        Assert.AreEqual('The value "@" can''t be evaluated into type Integer.', CalcError(Host, Host.FieldNo("Num Not At")), 'integer <>@');
        Assert.AreEqual('The value "@" can''t be evaluated into type Decimal.', CalcError(Host, Host.FieldNo("Amount At")), 'decimal @');
        Assert.AreEqual('The value "@" can''t be evaluated into type Decimal.', CalcError(Host, Host.FieldNo("Amount Not At")), 'decimal <>@');
        Assert.AreEqual('''@'' is not an option. The existing options are: A,B', CalcError(Host, Host.FieldNo("Opt At")), 'option @');
        Assert.AreEqual('''@'' is not an option. The existing options are: A,B', CalcError(Host, Host.FieldNo("Opt Not At")), 'option <>@');
        Assert.AreEqual('The value "@" can''t be evaluated into type Boolean.', CalcError(Host, Host.FieldNo("Flag At")), 'boolean @');
        Assert.AreEqual('The value "@" can''t be evaluated into type Boolean.', CalcError(Host, Host.FieldNo("Flag Not At")), 'boolean <>@');
    end;

    // The error code GetLastErrorCode returns after each of those errors.
    local procedure CalcErrorCode(var Host: Record "AM Formula Host"; FieldNo: Integer): Text
    begin
        CalcError(Host, FieldNo);
        exit(GetLastErrorCode());
    end;

    [Test]
    procedure AtSignAlone_CalcFormulaOtherSources_ErrorCode()
    var
        Host: Record "AM Formula Host";
        Seen: Text;
    begin
        Host."No." := NewGroup();
        Host.Insert();
        Seen := CalcErrorCode(Host, Host.FieldNo("Num At")) + ',' + CalcErrorCode(Host, Host.FieldNo("Num Not At"));
        Seen += '|' + CalcErrorCode(Host, Host.FieldNo("Amount At")) + ',' + CalcErrorCode(Host, Host.FieldNo("Amount Not At"));
        Seen += '|' + CalcErrorCode(Host, Host.FieldNo("Opt At")) + ',' + CalcErrorCode(Host, Host.FieldNo("Opt Not At"));
        Seen += '|' + CalcErrorCode(Host, Host.FieldNo("Flag At")) + ',' + CalcErrorCode(Host, Host.FieldNo("Flag Not At"));
        Assert.AreEqual(
            'InvalidFilterExpression:Evaluate,InvalidFilterExpression:Evaluate|InvalidFilterExpression:Evaluate,InvalidFilterExpression:Evaluate|' +
            'InvalidFilterExpression:InvalidOptionString,InvalidFilterExpression:InvalidOptionString|' +
            'InvalidFilterExpression:Evaluate,InvalidFilterExpression:Evaluate',
            Seen, 'integer, decimal, option, boolean; @ and <>@');
    end;

    // <>@ alone set in filter group Group, read in filter group 0: Count and the rows FindSet goes through. Filter group 1 is
    // reset at the end, as it is the session's.
    local procedure GroupNotAt(var R: Record "AM Kind Row"; Group: Integer; FieldNo: Integer): Text
    var
        Seen: Text;
    begin
        R.FilterGroup(Group);
        SetNotAt(R, FieldNo);
        R.FilterGroup(0);
        Seen := Format(R.Count()) + ':' + Ids(R);
        R.FilterGroup(Group);
        ClearKinds(R);
        R.FilterGroup(0);
        exit(Seen);
    end;

    [Test]
    procedure AtSignAloneNegated_FilterGroupsOneAndTwo()
    var
        R: Record "AM Kind Row";
        Temp: Record "AM Kind Row" temporary;
        G: Code[20];
        Seen: Text;
    begin
        G := NewGroup();
        CopyGroup(G, Temp);
        R.SetRange("Group", G);
        Seen := GroupNotAt(R, 1, R.FieldNo("Code Kind")) + '|' + GroupNotAt(R, 1, R.FieldNo("Text Kind"));
        Seen += '|' + GroupNotAt(R, 2, R.FieldNo("Code Kind")) + '|' + GroupNotAt(R, 2, R.FieldNo("Text Kind"));
        Seen += '|' + GroupNotAt(Temp, 2, Temp.FieldNo("Code Kind")) + '|' + GroupNotAt(Temp, 2, Temp.FieldNo("Text Kind"));
        Assert.AreEqual('1:1,|1:1,|1:1,|1:1,|0:|0:', Seen, 'database group 1 code, text, group 2 code, text; temporary group 2 code, text');
    end;

    // The same in filter group 1 of a temporary record.
    [Test]
    procedure AtSignAloneNegated_FilterGroupOne_Temporary()
    var
        Temp: Record "AM Kind Row" temporary;
        Seen: Text;
    begin
        CopyGroup(NewGroup(), Temp);
        Seen := GroupNotAt(Temp, 1, Temp.FieldNo("Code Kind")) + '|' + GroupNotAt(Temp, 1, Temp.FieldNo("Text Kind"));
        Assert.AreEqual('0:|0:', Seen, 'temporary group 1 code, text');
    end;

    // <>@ alone and @ alone set on a min and on a max FlowField of Code and of Text type (table 97511): Count, IsEmpty and
    // FindFirst, of a host in the database and of a temporary host. The min of the group is the empty value, the max is K.
    local procedure MinMaxReads(var Host: Record "AM Min Max Host"; FieldNo: Integer; Filter: Text): Text
    begin
        Host.SetRange("Min Code");
        Host.SetRange("Max Code");
        Host.SetRange("Min Text");
        Host.SetRange("Max Text");
        case FieldNo of
            Host.FieldNo("Min Code"):
                Host.SetFilter("Min Code", Filter);
            Host.FieldNo("Max Code"):
                Host.SetFilter("Max Code", Filter);
            Host.FieldNo("Min Text"):
                Host.SetFilter("Min Text", Filter);
            Host.FieldNo("Max Text"):
                Host.SetFilter("Max Text", Filter);
        end;
        exit(Format(Host.Count()) + ',' + Format(Host.IsEmpty()) + ',' + Format(Host.FindFirst()));
    end;

    local procedure MinMaxAll(var Host: Record "AM Min Max Host"): Text
    var
        Seen: Text;
        No: Integer;
    begin
        for No := 2 to 5 do begin
            Seen += MinMaxReads(Host, No, '<>@') + ';' + MinMaxReads(Host, No, '@');
            if No < 5 then
                Seen += '|';
        end;
        exit(Seen);
    end;

    [Test]
    procedure AtSignAloneNegated_MinMaxFlowFieldFilter()
    var
        Host: Record "AM Min Max Host";
        TempHost: Record "AM Min Max Host" temporary;
        Seen: Text;
    begin
        Host."No." := NewGroup();
        Host.Insert();
        TempHost := Host;
        TempHost.Insert();
        Host.SetRange("No.", Host."No.");
        TempHost.SetRange("No.", Host."No.");
        Seen := MinMaxAll(Host) + ' / ' + MinMaxAll(TempHost);
        Assert.AreEqual(
            '0,Yes,No;1,No,Yes|1,No,Yes;0,Yes,Yes|0,Yes,No;1,No,Yes|1,No,Yes;0,Yes,Yes / ' +
            '0,Yes,No;1,No,Yes|0,Yes,No;1,No,Yes|0,Yes,No;1,No,Yes|0,Yes,No;1,No,Yes',
            Seen, 'min code, max code, min text, max text: <>@;@; database / temporary');
    end;

    // The other readers of a sum with filter('@') or filter('<>@') over a Date, an Integer and an Option source field (table 97507):
    // SetAutoCalcFields and FindFirst, SetRange on the FlowField and Count, a query column (queries 97503 and 97504), CalcFields of a
    // temporary host. Each answers as CalcFields of a host in the database does (:395, :430, :454).
    local procedure ErrorOf(Ok: Boolean; Value: Text): Text
    begin
        if Ok then
            exit(Value);
        exit(GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    [TryFunction]
    local procedure TryAutoCalc(G: Code[20]; FieldNo: Integer; var Value: Text)
    var
        Host: Record "AM Formula Host";
    begin
        Host.SetRange("No.", G);
        case FieldNo of
            Host.FieldNo("Day At"):
                Host.SetAutoCalcFields("Day At", "Day Not At");
            Host.FieldNo("Num At"):
                Host.SetAutoCalcFields("Num At");
            Host.FieldNo("Opt At"):
                Host.SetAutoCalcFields("Opt At");
        end;
        Host.FindFirst();
        Value := Format(Host."Day At") + ',' + Format(Host."Day Not At");
    end;

    [TryFunction]
    local procedure TryRangeCount(G: Code[20]; FieldNo: Integer; var Value: Text)
    var
        Host: Record "AM Formula Host";
    begin
        Host.SetRange("No.", G);
        case FieldNo of
            Host.FieldNo("Day At"):
                Host.SetRange("Day At", 0);
            Host.FieldNo("Day Not At"):
                Host.SetRange("Day Not At", 15);
            Host.FieldNo("Num At"):
                Host.SetRange("Num At", 0);
            Host.FieldNo("Opt At"):
                Host.SetRange("Opt At", 0);
        end;
        Value := Format(Host.Count());
    end;

    [TryFunction]
    local procedure TryDayQuery(G: Code[20]; var Value: Text)
    var
        Days: Query "AM Day Columns";
    begin
        Days.SetRange(HostNo, G);
        Days.Open();
        while Days.Read() do
            Value += Format(Days.DayAt) + ',' + Format(Days.DayNotAt) + ';';
        Days.Close();
    end;

    [TryFunction]
    local procedure TryNumQuery(G: Code[20]; var Value: Text)
    var
        Nums: Query "AM Num Columns";
    begin
        Nums.SetRange(HostNo, G);
        Nums.Open();
        while Nums.Read() do
            Value += Format(Nums.NumAt) + ';';
        Nums.Close();
    end;

    [TryFunction]
    local procedure TryTempCalc(var TempHost: Record "AM Formula Host" temporary; FieldNo: Integer; var Value: Text)
    begin
        case FieldNo of
            TempHost.FieldNo("Day At"):
                TempHost.CalcFields("Day At", "Day Not At");
            TempHost.FieldNo("Num At"):
                TempHost.CalcFields("Num At");
            TempHost.FieldNo("Opt At"):
                TempHost.CalcFields("Opt At");
        end;
        Value := Format(TempHost."Day At") + ',' + Format(TempHost."Day Not At");
    end;

    [Test]
    procedure AtSignAlone_CalcFormulaOtherSources_Readers()
    var
        Host: Record "AM Formula Host";
        TempHost: Record "AM Formula Host" temporary;
        G: Code[20];
        V: Text;
        Seen: Text;
    begin
        G := NewGroup();
        Host."No." := G;
        Host.Insert();
        V := '';
        Seen := 'auto day=' + ErrorOf(TryAutoCalc(G, Host.FieldNo("Day At"), V), V);
        V := '';
        Seen += ' num=' + ErrorOf(TryAutoCalc(G, Host.FieldNo("Num At"), V), V);
        V := '';
        Seen += ' opt=' + ErrorOf(TryAutoCalc(G, Host.FieldNo("Opt At"), V), V);
        V := '';
        Seen += ' | range day=' + ErrorOf(TryRangeCount(G, Host.FieldNo("Day At"), V), V);
        V := '';
        Seen += ' notday=' + ErrorOf(TryRangeCount(G, Host.FieldNo("Day Not At"), V), V);
        V := '';
        Seen += ' num=' + ErrorOf(TryRangeCount(G, Host.FieldNo("Num At"), V), V);
        V := '';
        Seen += ' opt=' + ErrorOf(TryRangeCount(G, Host.FieldNo("Opt At"), V), V);
        V := '';
        Seen += ' | query day=' + ErrorOf(TryDayQuery(G, V), V);
        V := '';
        Seen += ' num=' + ErrorOf(TryNumQuery(G, V), V);
        TempHost := Host;
        TempHost.Insert();
        V := '';
        Seen += ' | temp day=' + ErrorOf(TryTempCalc(TempHost, Host.FieldNo("Day At"), V), V);
        V := '';
        Seen += ' num=' + ErrorOf(TryTempCalc(TempHost, Host.FieldNo("Num At"), V), V);
        V := '';
        Seen += ' opt=' + ErrorOf(TryTempCalc(TempHost, Host.FieldNo("Opt At"), V), V);
        Assert.AreEqual(
            'auto day=0,15 num=The value "@" can''t be evaluated into type Integer.|InvalidFilterExpression:Evaluate ' +
            'opt=''@'' is not an option. The existing options are: A,B|InvalidFilterExpression:InvalidOptionString | ' +
            'range day=1 notday=1 num=The value "@" can''t be evaluated into type Integer.|InvalidFilterExpression:Evaluate ' +
            'opt=''@'' is not an option. The existing options are: A,B|InvalidFilterExpression:InvalidOptionString | ' +
            'query day=0,15; num=The value "@" can''t be evaluated into type Integer.|InvalidFilterExpression:Evaluate | ' +
            'temp day=0,15 num=The value "@" can''t be evaluated into type Integer.|InvalidFilterExpression:Evaluate ' +
            'opt=''@'' is not an option. The existing options are: A,B|InvalidFilterExpression:InvalidOptionString',
            Seen, 'SetAutoCalcFields, SetRange and Count, a query column, a temporary host');
    end;

    // filter('@') and filter('<>@') over an Option source field with a blank member (tables 97513 and 97515), of a host in the
    // database and of a temporary host: the error lists the member names as they are, the blank one included.
    [TryFunction]
    local procedure TryBlankOpt(var Host: Record "AM Blank Opt Host"; Negated: Boolean)
    begin
        if Negated then
            Host.CalcFields("Blank Opt Not At")
        else
            Host.CalcFields("Blank Opt At");
    end;

    [Test]
    procedure AtSignAlone_CalcFormulaBlankOptionMember()
    var
        Row: Record "AM Blank Opt Row";
        Host: Record "AM Blank Opt Host";
        TempHost: Record "AM Blank Opt Host" temporary;
        Seen: Text;
    begin
        Row."Group" := CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 18);
        Row.Id := 1;
        Row.Opt := Row.Opt::B;
        Row.Amount := 10;
        Row.Insert();
        Row.Id := 2;
        Row.Opt := Row.Opt::" ";
        Row.Amount := 5;
        Row.Insert();
        Host."No." := Row."Group";
        Host.Insert();
        TempHost := Host;
        TempHost.Insert();
        Seen := ErrorOf(TryBlankOpt(Host, false), '') + ' # ' + ErrorOf(TryBlankOpt(Host, true), '');
        Seen += ' # ' + ErrorOf(TryBlankOpt(TempHost, false), '') + ' # ' + ErrorOf(TryBlankOpt(TempHost, true), '');
        Assert.AreEqual(
            '''@'' is not an option. The existing options are:  ,A,B|InvalidFilterExpression:InvalidOptionString # ' +
            '''@'' is not an option. The existing options are:  ,A,B|InvalidFilterExpression:InvalidOptionString # ' +
            '''@'' is not an option. The existing options are:  ,A,B|InvalidFilterExpression:InvalidOptionString # ' +
            '''@'' is not an option. The existing options are:  ,A,B|InvalidFilterExpression:InvalidOptionString',
            Seen, 'database @, database <>@, temporary @, temporary <>@');
    end;

    // A Code FlowFilter set to <>@ alone, applied by CalcFields through field(filter()) and through field() with every method, over
    // a Code and over a Text source field (table 97512), of a host in the database and of a temporary host: the K row.
    local procedure FlowMethods(var Host: Record "AM Flow Host"): Text
    var
        Ref: RecordRef;
        F: FieldRef;
        No: Integer;
        Seen: Text;
    begin
        Host.SetFilter("Code Filter", '<>@');
        Ref.GetTable(Host);
        for No := 10 to 35 do begin
            F := Ref.Field(No);
            F.CalcField();
            Seen += Format(F.Value) + ',';
        end;
        exit(Seen);
    end;

    [Test]
    procedure AtSignAloneNegated_CodeFlowFilterMethods()
    var
        Host: Record "AM Flow Host";
        TempHost: Record "AM Flow Host" temporary;
        Seen: Text;
    begin
        Host."No." := NewGroup();
        Host.Insert();
        TempHost := Host;
        TempHost.Insert();
        Seen := FlowMethods(Host) + ' / ' + FlowMethods(TempHost);
        Assert.AreEqual(
            '1,Yes,10,10,10,10,1,Yes,10,10,10,10,10,1,Yes,10,10,10,10,1,Yes,10,10,10,10,10, / ' +
            '1,Yes,10,10,10,10,1,Yes,10,10,10,10,10,1,Yes,10,10,10,10,1,Yes,10,10,10,10,10,',
            Seen, 'fields 10-35 (Code over Code and Code over Text, field(filter()) then field()); database / temporary');
    end;

    // The other Find methods with <>@ alone and @ alone set on a min and on a max FlowField (table 97511): Find('='), Find('-'),
    // Find('+'), FindLast, and FindSet with the rows Next goes through, of a host in the database and of a temporary host.
    local procedure MinMaxFinds(var Host: Record "AM Min Max Host"; FieldNo: Integer; Filter: Text): Text
    var
        Seen: Text;
        Rows: Integer;
    begin
        Host.SetRange("Min Code");
        Host.SetRange("Max Code");
        Host.SetRange("Min Text");
        Host.SetRange("Max Text");
        case FieldNo of
            Host.FieldNo("Min Code"):
                Host.SetFilter("Min Code", Filter);
            Host.FieldNo("Max Code"):
                Host.SetFilter("Max Code", Filter);
            Host.FieldNo("Min Text"):
                Host.SetFilter("Min Text", Filter);
            Host.FieldNo("Max Text"):
                Host.SetFilter("Max Text", Filter);
        end;
        Seen := Format(Host.Find('=')) + ',' + Format(Host.Find('-')) + ',' + Format(Host.Find('+')) + ',' + Format(Host.FindLast());
        if Host.FindSet() then
            repeat
                Rows += 1;
            until Host.Next() = 0;
        exit(Seen + ',' + Format(Rows));
    end;

    local procedure MinMaxFindsAll(var Host: Record "AM Min Max Host"): Text
    var
        Seen: Text;
        No: Integer;
    begin
        for No := 2 to 5 do begin
            Seen += MinMaxFinds(Host, No, '<>@') + ';' + MinMaxFinds(Host, No, '@');
            if No < 5 then
                Seen += '|';
        end;
        exit(Seen);
    end;

    [Test]
    procedure AtSignAloneNegated_MinMaxFlowFieldFilter_FindMethods()
    var
        Host: Record "AM Min Max Host";
        TempHost: Record "AM Min Max Host" temporary;
        Seen: Text;
    begin
        Host."No." := NewGroup();
        Host.Insert();
        TempHost := Host;
        TempHost.Insert();
        Host.SetRange("No.", Host."No.");
        TempHost.SetRange("No.", Host."No.");
        Seen := MinMaxFindsAll(Host) + ' / ' + MinMaxFindsAll(TempHost);
        Assert.AreEqual(
            'No,No,No,No,0;Yes,Yes,Yes,Yes,1|Yes,Yes,Yes,Yes,1;Yes,Yes,Yes,Yes,1|No,No,No,No,0;Yes,Yes,Yes,Yes,1|Yes,Yes,Yes,Yes,1;Yes,Yes,Yes,Yes,1 / ' +
            'No,No,No,No,0;Yes,Yes,Yes,Yes,1|No,No,No,No,0;Yes,Yes,Yes,Yes,1|No,No,No,No,0;Yes,Yes,Yes,Yes,1|No,No,No,No,0;Yes,Yes,Yes,Yes,1',
            Seen, 'min code, max code, min text, max text: <>@;@; database / temporary');
    end;
}

// The FlowFields of AtSignAloneNegated_MinMaxFlowFieldFilter: the min and the max of the Code and of the Text field of a group.
table 97511 "AM Min Max Host"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; "Min Code"; Code[20]) { FieldClass = FlowField; CalcFormula = min("AM Kind Row"."Code Kind" where("Group" = field("No."))); }
        field(3; "Max Code"; Code[20]) { FieldClass = FlowField; CalcFormula = max("AM Kind Row"."Code Kind" where("Group" = field("No."))); }
        field(4; "Min Text"; Text[50]) { FieldClass = FlowField; CalcFormula = min("AM Kind Row"."Text Kind" where("Group" = field("No."))); }
        field(5; "Max Text"; Text[50]) { FieldClass = FlowField; CalcFormula = max("AM Kind Row"."Text Kind" where("Group" = field("No."))); }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

// The queries of AtSignAlone_CalcFormulaOtherSources_Readers: a column over a FlowField of table 97507.
query 97503 "AM Day Columns"
{
    QueryType = Normal;

    elements
    {
        dataitem(Host; "AM Formula Host")
        {
            column(HostNo; "No.") { }
            column(DayAt; "Day At") { }
            column(DayNotAt; "Day Not At") { }
        }
    }
}

query 97504 "AM Num Columns"
{
    QueryType = Normal;

    elements
    {
        dataitem(Host; "AM Formula Host")
        {
            column(HostNo; "No.") { }
            column(NumAt; "Num At") { }
        }
    }
}

// The rows and the FlowFields of AtSignAlone_CalcFormulaBlankOptionMember: an Option source field with a blank member.
table 97513 "AM Blank Opt Row"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; "Group"; Code[20]) { }
        field(2; Id; Integer) { }
        field(3; Opt; Option) { OptionMembers = " ",A,B; }
        field(4; Amount; Decimal) { }
    }

    keys
    {
        key(PK; "Group", Id) { Clustered = true; }
    }
}

table 97515 "AM Blank Opt Host"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; "Blank Opt At"; Decimal) { FieldClass = FlowField; CalcFormula = sum("AM Blank Opt Row".Amount where("Group" = field("No."), Opt = filter('@'))); }
        field(3; "Blank Opt Not At"; Decimal) { FieldClass = FlowField; CalcFormula = sum("AM Blank Opt Row".Amount where("Group" = field("No."), Opt = filter('<>@'))); }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

// The FlowFields of AtSignAloneNegated_CodeFlowFilterMethods: every method over the Code field (Code over Code) and over the Text
// field (Code over Text) of a group, filtered by the Code FlowFilter through field(filter()) (10-22) and through field() (23-35).
table 97512 "AM Flow Host"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; "Code Filter"; Code[20]) { FieldClass = FlowFilter; }
        field(10; "CC Count Filter"; Integer) { FieldClass = FlowField; CalcFormula = count("AM Kind Row" where("Group" = field("No."), "Code Kind" = field(filter("Code Filter")))); }
        field(11; "CC Exist Filter"; Boolean) { FieldClass = FlowField; CalcFormula = exist("AM Kind Row" where("Group" = field("No."), "Code Kind" = field(filter("Code Filter")))); }
        field(12; "CC Lookup Filter"; Decimal) { FieldClass = FlowField; CalcFormula = lookup("AM Kind Row".Amount where("Group" = field("No."), "Code Kind" = field(filter("Code Filter")))); }
        field(13; "CC Min Filter"; Decimal) { FieldClass = FlowField; CalcFormula = min("AM Kind Row".Amount where("Group" = field("No."), "Code Kind" = field(filter("Code Filter")))); }
        field(14; "CC Max Filter"; Decimal) { FieldClass = FlowField; CalcFormula = max("AM Kind Row".Amount where("Group" = field("No."), "Code Kind" = field(filter("Code Filter")))); }
        field(15; "CC Average Filter"; Decimal) { FieldClass = FlowField; CalcFormula = average("AM Kind Row".Amount where("Group" = field("No."), "Code Kind" = field(filter("Code Filter")))); }
        field(16; "CT Count Filter"; Integer) { FieldClass = FlowField; CalcFormula = count("AM Kind Row" where("Group" = field("No."), "Text Kind" = field(filter("Code Filter")))); }
        field(17; "CT Exist Filter"; Boolean) { FieldClass = FlowField; CalcFormula = exist("AM Kind Row" where("Group" = field("No."), "Text Kind" = field(filter("Code Filter")))); }
        field(18; "CT Lookup Filter"; Decimal) { FieldClass = FlowField; CalcFormula = lookup("AM Kind Row".Amount where("Group" = field("No."), "Text Kind" = field(filter("Code Filter")))); }
        field(19; "CT Min Filter"; Decimal) { FieldClass = FlowField; CalcFormula = min("AM Kind Row".Amount where("Group" = field("No."), "Text Kind" = field(filter("Code Filter")))); }
        field(20; "CT Max Filter"; Decimal) { FieldClass = FlowField; CalcFormula = max("AM Kind Row".Amount where("Group" = field("No."), "Text Kind" = field(filter("Code Filter")))); }
        field(21; "CT Average Filter"; Decimal) { FieldClass = FlowField; CalcFormula = average("AM Kind Row".Amount where("Group" = field("No."), "Text Kind" = field(filter("Code Filter")))); }
        field(22; "CT Sum Filter"; Decimal) { FieldClass = FlowField; CalcFormula = sum("AM Kind Row".Amount where("Group" = field("No."), "Text Kind" = field(filter("Code Filter")))); }
        field(23; "CC Count Field"; Integer) { FieldClass = FlowField; CalcFormula = count("AM Kind Row" where("Group" = field("No."), "Code Kind" = field("Code Filter"))); }
        field(24; "CC Exist Field"; Boolean) { FieldClass = FlowField; CalcFormula = exist("AM Kind Row" where("Group" = field("No."), "Code Kind" = field("Code Filter"))); }
        field(25; "CC Lookup Field"; Decimal) { FieldClass = FlowField; CalcFormula = lookup("AM Kind Row".Amount where("Group" = field("No."), "Code Kind" = field("Code Filter"))); }
        field(26; "CC Min Field"; Decimal) { FieldClass = FlowField; CalcFormula = min("AM Kind Row".Amount where("Group" = field("No."), "Code Kind" = field("Code Filter"))); }
        field(27; "CC Max Field"; Decimal) { FieldClass = FlowField; CalcFormula = max("AM Kind Row".Amount where("Group" = field("No."), "Code Kind" = field("Code Filter"))); }
        field(28; "CC Average Field"; Decimal) { FieldClass = FlowField; CalcFormula = average("AM Kind Row".Amount where("Group" = field("No."), "Code Kind" = field("Code Filter"))); }
        field(29; "CT Count Field"; Integer) { FieldClass = FlowField; CalcFormula = count("AM Kind Row" where("Group" = field("No."), "Text Kind" = field("Code Filter"))); }
        field(30; "CT Exist Field"; Boolean) { FieldClass = FlowField; CalcFormula = exist("AM Kind Row" where("Group" = field("No."), "Text Kind" = field("Code Filter"))); }
        field(31; "CT Lookup Field"; Decimal) { FieldClass = FlowField; CalcFormula = lookup("AM Kind Row".Amount where("Group" = field("No."), "Text Kind" = field("Code Filter"))); }
        field(32; "CT Min Field"; Decimal) { FieldClass = FlowField; CalcFormula = min("AM Kind Row".Amount where("Group" = field("No."), "Text Kind" = field("Code Filter"))); }
        field(33; "CT Max Field"; Decimal) { FieldClass = FlowField; CalcFormula = max("AM Kind Row".Amount where("Group" = field("No."), "Text Kind" = field("Code Filter"))); }
        field(34; "CT Average Field"; Decimal) { FieldClass = FlowField; CalcFormula = average("AM Kind Row".Amount where("Group" = field("No."), "Text Kind" = field("Code Filter"))); }
        field(35; "CT Sum Field"; Decimal) { FieldClass = FlowField; CalcFormula = sum("AM Kind Row".Amount where("Group" = field("No."), "Text Kind" = field("Code Filter"))); }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}
