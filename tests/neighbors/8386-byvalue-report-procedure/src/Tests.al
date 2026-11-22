// Neighbors of al-runner issue 8386: a record handed by value to a report's procedure. Rows 1 (C, Open, 30),
// 2 (B, Posted, 20), 3 (A, Open, 10).
table 50100 "NB BR Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[20]) { }
        field(3; Status; Option) { OptionMembers = Open,Posted; }
        field(4; Amount; Decimal) { }
        field(5; Total; Decimal) { FieldClass = FlowField; CalcFormula = sum("NB BR Line".Amt where(RowId = field(Id))); }
    }
    keys
    {
        key(PK; Id) { Clustered = true; }
        key(ByName; Name) { }
    }

    procedure KeyOf(Copy: Record "NB BR Row"): Text
    begin
        exit(Copy.CurrentKey());
    end;

    procedure DescribeThroughReport(): Text
    var
        Copy: Record "NB BR Row";
        Rep: Report "NB BR Report";
    begin
        Copy.SetRange(Status, Copy.Status::Open);
        exit(Rep.Describe(Copy));
    end;

    procedure HandToReport(Mode: Integer): Text
    var
        Copy: Record "NB BR Row";
        Rep: Report "NB BR Report";
    begin
        case Mode of
            1:
                begin
                    Copy.SetLoadFields(Name);
                    Copy.SetCurrentKey(Total);
                end;
            2:
                Copy.SetCurrentKey(Name);
            3:
                Copy.SetLoadFields(Name);
        end;
        exit(Rep.KeyAndLoad(Copy));
    end;
}

table 50101 "NB BR Line"
{
    fields { field(1; Id; Integer) { } field(2; RowId; Integer) { } field(3; Amt; Decimal) { } }
    keys { key(PK; Id) { Clustered = true; } }
}

report 50102 "NB BR Report"
{
    ProcessingOnly = true;
    UseRequestPage = false;

    dataset
    {
        dataitem(Row; "NB BR Row")
        {
            DataItemTableView = sorting(Name) where(Status = filter(Open));

            dataitem(Line; "NB BR Line")
            {
                DataItemLink = RowId = field(Id);

                trigger OnAfterGetRecord()
                begin
                    if (Mode <> 3) or LineDone then
                        exit;
                    LineDone := true;
                    Outcome := DescribeLine(Line);
                end;
            }

            trigger OnAfterGetRecord()
            begin
                if Done or (Mode = 3) then
                    exit;
                if Mode = 4 then begin
                    Done := true;
                    Outcome := KeyLoadFromTrigger();
                    exit;
                end;
                Done := true;
                case Mode of
                    1:
                        Outcome := Describe(Row);
                    2:
                        begin
                            Reset();
                            SetRange(Status, Status);
                            Outcome := Recurse(Row, 1);
                        end;
                end;
            end;
        }
    }

    var
        Mode: Integer;
        Done: Boolean;
        LineDone: Boolean;
        Outcome: Text;

    procedure SetMode(NewMode: Integer)
    begin
        Mode := NewMode;
    end;

    procedure GetResult(): Text
    begin
        exit(Outcome);
    end;

    local procedure Recurse(R: Record "NB BR Row"; Level: Integer): Text
    var
        Inner: Text;
    begin
        if Level = 2 then
            exit('id=' + Format(R.Id) + ' ' + Describe(R));
        R.SetCurrentKey(Name);
        R.SetRange(Status, R.Status::Open);
        R.SetFilter(Amount, '<%1', 30);
        if R.FindFirst() then
            Inner := Recurse(R, Level + 1);
        exit(Inner + ' outer f={' + R.GetFilters() + '} key=' + R.CurrentKey());
    end;

    local procedure DescribeLine(L: Record "NB BR Line"): Text
    begin
        exit('f={' + L.GetFilters() + '} n=' + Format(L.Count()));
    end;

    procedure Describe(R: Record "NB BR Row"): Text
    begin
        exit('f={' + R.GetFilters() + '} n=' + Format(R.Count()) + ' key=' + R.CurrentKey());
    end;

    procedure KeyAndLoad(R: Record "NB BR Row"): Text
    begin
        R.FindFirst();
        exit('key=' + R.CurrentKey() + ' amount loaded=' + Format(R.AreFieldsLoaded(R.Amount)));
    end;

    local procedure KeyLoadFromTrigger(): Text
    var
        Local2: Record "NB BR Row";
    begin
        Local2.SetLoadFields(Name);
        Local2.SetCurrentKey(Total);
        exit(KeyAndLoad(Local2));
    end;
}

reportextension 50103 "NB BR Report Ext" extends "NB BR Report"
{
    procedure DescribeExt(R: Record "NB BR Row"): Text
    begin
        exit('f={' + R.GetFilters() + '}');
    end;
}

report 50104 "NB BR Other Report"
{
    ProcessingOnly = true;
    UseRequestPage = false;

    dataset
    {
        dataitem(Row; "NB BR Row")
        {
            trigger OnAfterGetRecord()
            var
                Rep: Report "NB BR Report";
            begin
                if Outcome <> '' then
                    exit;
                Outcome := Rep.Describe(Row);
            end;
        }
    }

    var
        Outcome: Text;

    procedure GetResult(): Text
    begin
        exit(Outcome);
    end;

    procedure HandToReport(Mode: Integer): Text
    var
        Copy: Record "NB BR Row";
        Rep: Report "NB BR Report";
    begin
        case Mode of
            1:
                begin
                    Copy.SetLoadFields(Name);
                    Copy.SetCurrentKey(Total);
                end;
            2:
                Copy.SetCurrentKey(Name);
        end;
        exit(Rep.KeyAndLoad(Copy));
    end;
}

page 50105 "NB BR Page"
{
    PageType = List;
    SourceTable = "NB BR Row";
    ApplicationArea = All;
    UsageCategory = None;
    Editable = false;

    layout
    {
        area(Content)
        {
            repeater(Rows) { field(Id; Rec.Id) { ApplicationArea = All; } }
        }
    }

    actions
    {
        area(Processing)
        {
            action(DescribeRows)
            {
                ApplicationArea = All;

                trigger OnAction()
                var
                    Row: Record "NB BR Row";
                    Rep: Report "NB BR Report";
                begin
                    Row.SetCurrentKey(Name);
                    Row.SetRange(Status, Row.Status::Open);
                    Error('%1', Rep.Describe(Row));
                end;
            }
            action(KeyLoadToReport)
            {
                ApplicationArea = All;

                trigger OnAction()
                var
                    Row: Record "NB BR Row";
                    Rep: Report "NB BR Report";
                begin
                    Row.SetLoadFields(Name);
                    Row.SetCurrentKey(Total);
                    Error('%1', Rep.KeyAndLoad(Row));
                end;
            }
        }
    }

    procedure KeyAndLoad(R: Record "NB BR Row"): Text
    begin
        R.FindFirst();
        exit('key=' + R.CurrentKey() + ' amount loaded=' + Format(R.AreFieldsLoaded(R.Amount)));
    end;
}

interface "NB BR IKeyAndLoad"
{
    procedure KeyAndLoad(R: Record "NB BR Row"): Text;
}

xmlport 50108 "NB BR XmlPort"
{
    schema
    {
        textelement(Root) { }
    }

    procedure KeyAndLoad(R: Record "NB BR Row"): Text
    begin
        R.FindFirst();
        exit('key=' + R.CurrentKey() + ' amount loaded=' + Format(R.AreFieldsLoaded(R.Amount)));
    end;
}

query 50109 "NB BR Query"
{
    elements
    {
        dataitem(Row; "NB BR Row")
        {
            column(Id; Id) { }
        }
    }

    procedure KeyAndLoad(R: Record "NB BR Row"): Text
    begin
        R.FindFirst();
        exit('key=' + R.CurrentKey() + ' amount loaded=' + Format(R.AreFieldsLoaded(R.Amount)));
    end;
}

codeunit 50106 "NB BR Codeunit Callee" implements "NB BR IKeyAndLoad"
{
    procedure KeyAndLoad(R: Record "NB BR Row"): Text
    begin
        R.FindFirst();
        exit('key=' + R.CurrentKey() + ' amount loaded=' + Format(R.AreFieldsLoaded(R.Amount)));
    end;

    procedure Describe(R: Record "NB BR Row"): Text
    begin
        exit('f={' + R.GetFilters() + '} key=' + R.CurrentKey());
    end;
}

codeunit 50107 "NB BR Lib"
{
    procedure Seed()
    var
        Row: Record "NB BR Row";
        Line: Record "NB BR Line";
    begin
        Row.DeleteAll();
        Line.DeleteAll();
        AddRow(1, 'C', Row.Status::Open, 30);
        AddRow(2, 'B', Row.Status::Posted, 20);
        AddRow(3, 'A', Row.Status::Open, 10);
        AddLine(1, 1);
        AddLine(2, 1);
        AddLine(3, 2);
    end;

    procedure AddRow(Id: Integer; Name: Text[20]; Status: Option; Amount: Decimal)
    var
        Row: Record "NB BR Row";
    begin
        Row.Id := Id;
        Row.Name := Name;
        Row.Status := Status;
        Row.Amount := Amount;
        Row.Insert();
    end;

    procedure AddLine(Id: Integer; RowId: Integer)
    var
        Line: Record "NB BR Line";
    begin
        Line.Id := Id;
        Line.RowId := RowId;
        Line.Insert();
    end;

    procedure RunReport(Mode: Integer): Text
    var
        Row: Record "NB BR Row";
        Rep: Report "NB BR Report";
    begin
        Row.SetFilter(Amount, '>=%1', 20);
        Rep.SetMode(Mode);
        Rep.SetTableView(Row);
        Rep.RunModal();
        exit(Rep.GetResult());
    end;

    procedure Expect(Got: Text; Want: Text)
    begin
        if Got <> Want then
            Error('%1', Got);
    end;
}

codeunit 50110 "NB BR Test 50110"
{
    Subtype = Test;

    [Test]
    procedure FromCodeunit_SecondaryKeyAndFilter()
    var
        Lib: Codeunit "NB BR Lib";
        Row: Record "NB BR Row";
        Rep: Report "NB BR Report";
    begin
        Lib.Seed();
        Row.SetCurrentKey(Name);
        Row.SetRange(Status, Row.Status::Open);
        Lib.Expect(Rep.Describe(Row), 'f={} n=3 key=Id');
    end;
}

codeunit 50111 "NB BR Test 50111"
{
    Subtype = Test;

    [Test]
    procedure FromCodeunit_SecondaryKeyNoFilter()
    var
        Lib: Codeunit "NB BR Lib";
        Row: Record "NB BR Row";
        Rep: Report "NB BR Report";
    begin
        Lib.Seed();
        Row.SetCurrentKey(Name);
        Lib.Expect(Rep.Describe(Row), 'f={} n=3 key=Id');
    end;
}

codeunit 50112 "NB BR Test 50112"
{
    Subtype = Test;

    [Test]
    procedure FromCodeunit_PrimaryKeyNoFilter()
    var
        Lib: Codeunit "NB BR Lib";
        Row: Record "NB BR Row";
        Rep: Report "NB BR Report";
    begin
        Lib.Seed();
        Lib.Expect(Rep.Describe(Row), 'f={} n=3 key=Id');
    end;
}

codeunit 50113 "NB BR Test 50113"
{
    Subtype = Test;

    [Test]
    procedure FromCodeunit_SecondaryKeyNoFilterAutoCalc()
    var
        Lib: Codeunit "NB BR Lib";
        Row: Record "NB BR Row";
        Rep: Report "NB BR Report";
    begin
        Lib.Seed();
        Row.SetCurrentKey(Name);
        Row.SetAutoCalcFields(Total);
        Lib.Expect(Rep.Describe(Row), 'f={} n=3 key=Id');
    end;
}

codeunit 50114 "NB BR Test 50114"
{
    Subtype = Test;

    [Test]
    procedure FromCodeunit_FilterGroup3()
    var
        Lib: Codeunit "NB BR Lib";
        Row: Record "NB BR Row";
        Rep: Report "NB BR Report";
    begin
        Lib.Seed();
        Row.FilterGroup(3);
        Row.SetRange(Status, Row.Status::Open);
        Row.FilterGroup(0);
        Lib.Expect(Rep.Describe(Row), 'f={} n=3 key=Id');
    end;
}

codeunit 50115 "NB BR Test 50115"
{
    Subtype = Test;

    [Test]
    procedure FlowFieldKeyLoadFields_ToReport()
    var
        Lib: Codeunit "NB BR Lib";
        Row: Record "NB BR Row";
        Rep: Report "NB BR Report";
    begin
        Lib.Seed();
        Row.SetLoadFields(Name);
        Row.SetCurrentKey(Total);
        Lib.Expect(Rep.KeyAndLoad(Row), 'key=Id amount loaded=Yes');
    end;
}

codeunit 50116 "NB BR Test 50116"
{
    Subtype = Test;

    [Test]
    procedure FlowFieldKeyLoadFields_ToCodeunit()
    var
        Lib: Codeunit "NB BR Lib";
        Row: Record "NB BR Row";
        Callee: Codeunit "NB BR Codeunit Callee";
    begin
        Lib.Seed();
        Row.SetLoadFields(Name);
        Row.SetCurrentKey(Total);
        Lib.Expect(Callee.KeyAndLoad(Row), 'key=Id amount loaded=Yes');
    end;
}

codeunit 50117 "NB BR Test 50117"
{
    Subtype = Test;

    [Test]
    procedure FlowFieldKeyLoadFields_ToTableProcedure()
    var
        Lib: Codeunit "NB BR Lib";
        Row: Record "NB BR Row";
    begin
        Lib.Seed();
        Row.SetLoadFields(Name);
        Row.SetCurrentKey(Total);
        Lib.Expect(Row.KeyOf(Row), 'Id');
    end;
}

codeunit 50118 "NB BR Test 50118"
{
    Subtype = Test;

    [Test]
    procedure FlowFieldKeyLoadFieldsReadIsolation_ToReport()
    var
        Lib: Codeunit "NB BR Lib";
        Row: Record "NB BR Row";
        Rep: Report "NB BR Report";
    begin
        Lib.Seed();
        Row.SetLoadFields(Name);
        Row.ReadIsolation := IsolationLevel::ReadCommitted;
        Row.SetCurrentKey(Total);
        Lib.Expect(Rep.KeyAndLoad(Row), 'key=Id amount loaded=Yes');
    end;
}

codeunit 50119 "NB BR Test 50119"
{
    Subtype = Test;

    [Test]
    procedure FromPageAction()
    var
        Lib: Codeunit "NB BR Lib";
        Rows: TestPage "NB BR Page";
    begin
        Lib.Seed();
        Rows.OpenView();
        asserterror Rows.DescribeRows.Invoke();
        Rows.Close();
        Lib.Expect(GetLastErrorText(), 'f={} n=3 key=Id');
    end;
}

codeunit 50120 "NB BR Test 50120"
{
    Subtype = Test;

    [Test]
    procedure FromTableProcedure_Filtered()
    var
        Lib: Codeunit "NB BR Lib";
        Row: Record "NB BR Row";
    begin
        Lib.Seed();
        Lib.Expect(Row.DescribeThroughReport(), 'f={} n=3 key=Id');
    end;
}

codeunit 50121 "NB BR Test 50121"
{
    Subtype = Test;

    [Test]
    procedure FromTableProcedure_FlowFieldKeyLoadFields()
    var
        Lib: Codeunit "NB BR Lib";
        Row: Record "NB BR Row";
    begin
        Lib.Seed();
        Lib.Expect(Row.HandToReport(1), 'key=Id amount loaded=Yes');
    end;
}

codeunit 50122 "NB BR Test 50122"
{
    Subtype = Test;

    [Test]
    procedure FromTableProcedure_SecondaryKey()
    var
        Lib: Codeunit "NB BR Lib";
        Row: Record "NB BR Row";
    begin
        Lib.Seed();
        Lib.Expect(Row.HandToReport(2), 'key=Id amount loaded=Yes');
    end;
}

codeunit 50123 "NB BR Test 50123"
{
    Subtype = Test;

    [Test]
    procedure FromTableProcedure_LoadFields()
    var
        Lib: Codeunit "NB BR Lib";
        Row: Record "NB BR Row";
    begin
        Lib.Seed();
        Lib.Expect(Row.HandToReport(3), 'key=Id amount loaded=Yes');
    end;
}

codeunit 50124 "NB BR Test 50124"
{
    Subtype = Test;

    [Test]
    procedure FromAnotherReport_Filtered()
    var
        Lib: Codeunit "NB BR Lib";
        Row: Record "NB BR Row";
        Other: Report "NB BR Other Report";
    begin
        Lib.Seed();
        Row.SetRange(Status, Row.Status::Open);
        Other.SetTableView(Row);
        Other.RunModal();
        Lib.Expect(Other.GetResult(), 'f={} n=3 key=Id');
    end;
}

codeunit 50125 "NB BR Test 50125"
{
    Subtype = Test;

    [Test]
    procedure FromAnotherReport_FlowFieldKeyLoadFields()
    var
        Lib: Codeunit "NB BR Lib";
        Other: Report "NB BR Other Report";
    begin
        Lib.Seed();
        Lib.Expect(Other.HandToReport(1), 'key=Id amount loaded=Yes');
    end;
}

codeunit 50126 "NB BR Test 50126"
{
    Subtype = Test;

    [Test]
    procedure FromAnotherReport_SecondaryKey()
    var
        Lib: Codeunit "NB BR Lib";
        Other: Report "NB BR Other Report";
    begin
        Lib.Seed();
        Lib.Expect(Other.HandToReport(2), 'key=Id amount loaded=Yes');
    end;
}

codeunit 50127 "NB BR Test 50127"
{
    Subtype = Test;

    [Test]
    procedure ReportExtensionProcedure()
    var
        Lib: Codeunit "NB BR Lib";
        Row: Record "NB BR Row";
        Rep: Report "NB BR Report";
    begin
        Lib.Seed();
        Row.SetRange(Status, Row.Status::Open);
        Lib.Expect(Rep.DescribeExt(Row), 'f={}');
    end;
}

codeunit 50128 "NB BR Test 50128"
{
    Subtype = Test;

    [Test]
    procedure DataItemWithItsView()
    var
        Lib: Codeunit "NB BR Lib";
    begin
        Lib.Seed();
        Lib.Expect(Lib.RunReport(1), 'f={} n=3 key=Id');
    end;
}

codeunit 50129 "NB BR Test 50129"
{
    Subtype = Test;

    [Test]
    procedure DataItemRecursion()
    var
        Lib: Codeunit "NB BR Lib";
    begin
        Lib.Seed();
        Lib.Expect(Lib.RunReport(2), 'id=3 f={} n=3 key=Id outer f={Status: Open, Amount: <30} key=Name');
    end;
}

codeunit 50130 "NB BR Test 50130"
{
    Subtype = Test;

    [Test]
    procedure ChildDataItemLink()
    var
        Lib: Codeunit "NB BR Lib";
    begin
        Lib.Seed();
        Lib.Expect(Lib.RunReport(3), 'f={} n=3');
    end;
}

codeunit 50131 "NB BR Test 50131"
{
    Subtype = Test;

    [Test]
    procedure FlowFieldKeyAndFilter_ToReport()
    var
        Lib: Codeunit "NB BR Lib";
        Row: Record "NB BR Row";
        Rep: Report "NB BR Report";
    begin
        Lib.Seed();
        Row.SetCurrentKey(Total);
        Row.SetRange(Status, Row.Status::Open);
        Lib.Expect(Rep.Describe(Row), 'f={} n=3 key=Id');
    end;
}

codeunit 50132 "NB BR Test 50132"
{
    Subtype = Test;

    [Test]
    procedure FlowFieldKeyDescending_ToReport()
    var
        Lib: Codeunit "NB BR Lib";
        Row: Record "NB BR Row";
        Rep: Report "NB BR Report";
    begin
        Lib.Seed();
        Row.SetCurrentKey(Total);
        Row.Ascending(false);
        Lib.Expect(Rep.Describe(Row), 'f={} n=3 key=Id');
    end;
}

codeunit 50133 "NB BR Test 50133"
{
    Subtype = Test;

    [Test]
    procedure FlowFieldKeyMarks_ToReport()
    var
        Lib: Codeunit "NB BR Lib";
        Row: Record "NB BR Row";
        Rep: Report "NB BR Report";
    begin
        Lib.Seed();
        Row.Get(2);
        Row.Mark(true);
        Row.SetCurrentKey(Total);
        Lib.Expect(Rep.Describe(Row), 'f={} n=3 key=Id');
    end;
}

codeunit 50134 "NB BR Test 50134"
{
    Subtype = Test;

    [Test]
    procedure FlowFieldKeyAndFilter_ToCodeunit()
    var
        Lib: Codeunit "NB BR Lib";
        Row: Record "NB BR Row";
        Callee: Codeunit "NB BR Codeunit Callee";
    begin
        Lib.Seed();
        Row.SetCurrentKey(Total);
        Row.SetRange(Status, Row.Status::Open);
        Lib.Expect(Callee.Describe(Row), 'f={} key=Id');
    end;
}

codeunit 50135 "NB BR Test 50135"
{
    Subtype = Test;

    [Test]
    procedure FlowFieldKeyLoadFields_FromPageAction()
    var
        Lib: Codeunit "NB BR Lib";
        Rows: TestPage "NB BR Page";
    begin
        Lib.Seed();
        Rows.OpenView();
        asserterror Rows.KeyLoadToReport.Invoke();
        Rows.Close();
        Lib.Expect(GetLastErrorText(), 'key=Id amount loaded=Yes');
    end;
}

codeunit 50136 "NB BR Test 50136"
{
    Subtype = Test;

    [Test]
    procedure FlowFieldKeyLoadFields_FromReportTrigger()
    var
        Lib: Codeunit "NB BR Lib";
    begin
        Lib.Seed();
        Lib.Expect(Lib.RunReport(4), 'key=Id amount loaded=Yes');
    end;
}

codeunit 50137 "NB BR Test 50137"
{
    Subtype = Test;

    [Test]
    procedure FlowFieldKeyLoadFields_ToPageProcedure()
    var
        Lib: Codeunit "NB BR Lib";
        Row: Record "NB BR Row";
        RowsPage: Page "NB BR Page";
    begin
        Lib.Seed();
        Row.SetLoadFields(Name);
        Row.SetCurrentKey(Total);
        Lib.Expect(RowsPage.KeyAndLoad(Row), 'key=Id amount loaded=Yes');
    end;
}

codeunit 50138 "NB BR Test 50138"
{
    Subtype = Test;

    [Test]
    procedure FlowFieldKeyLoadFields_ToXmlPortProcedure()
    var
        Lib: Codeunit "NB BR Lib";
        Row: Record "NB BR Row";
        Port: XmlPort "NB BR XmlPort";
    begin
        Lib.Seed();
        Row.SetLoadFields(Name);
        Row.SetCurrentKey(Total);
        Lib.Expect(Port.KeyAndLoad(Row), 'key=Id amount loaded=Yes');
    end;
}

codeunit 50139 "NB BR Test 50139"
{
    Subtype = Test;

    [Test]
    procedure FlowFieldKeyLoadFields_ToQueryProcedure()
    var
        Lib: Codeunit "NB BR Lib";
        Row: Record "NB BR Row";
        Q: Query "NB BR Query";
    begin
        Lib.Seed();
        Row.SetLoadFields(Name);
        Row.SetCurrentKey(Total);
        Lib.Expect(Q.KeyAndLoad(Row), 'key=Id amount loaded=Yes');
    end;
}

codeunit 50140 "NB BR Test 50140"
{
    Subtype = Test;

    [Test]
    procedure FlowFieldKeyLoadFields_ThroughInterface()
    var
        Lib: Codeunit "NB BR Lib";
        Row: Record "NB BR Row";
        Callee: Codeunit "NB BR Codeunit Callee";
        KeyAndLoad: Interface "NB BR IKeyAndLoad";
    begin
        Lib.Seed();
        Row.SetLoadFields(Name);
        Row.SetCurrentKey(Total);
        KeyAndLoad := Callee;
        Lib.Expect(KeyAndLoad.KeyAndLoad(Row), 'key=Id amount loaded=Yes');
    end;
}
