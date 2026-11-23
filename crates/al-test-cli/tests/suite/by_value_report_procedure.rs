//! A record handed by value to a procedure of a report (al-runner issue 8386): the copy gets the view of a fresh record,
//! flagged under `by-value-report-procedure` (corpus PR 1450, record/filters/ByValueRecordView.al, the
//! `ByValue_ReportProcedure_*` and `ByValue_FlowFieldKeyLoadFields_*` tests of codeunit 78731, :1252-1703). Measured callers: the report itself (a data item trigger, a local procedure), a
//! codeunit and a page. Every other caller (also with no filter on the record, when it has a secondary key or a SetLoadFields
//! before a read) and a report extension procedure refuse a read of the copy's view. A data item that still holds its
//! DataItemTableView filters, handed from its OnAfterGetRecord to a codeunit's or its report's procedure, gets the fresh view
//! (`_DataItemView` :1675, `_DataItemViewToCodeunit` :1690; al-runner issue 8838, corpus PR 1544).

use crate::record_copy_reports::{PASS, VIEW_REFUSED, check, run};

/// al-runner issue 8386, the reporter's repro as written (report 99001026 "Replan Production Order", DeleteProdOrders).
const ISSUE_8386: &str = r##"
table 50100 "Repro Order"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; Status; Option)
        {
            OptionMembers = Planned,Released,Finished;
        }
        field(2; "No."; Code[20]) { }
        field(3; "Ref. No."; Code[20]) { }
        field(4; Level; Integer) { }
    }

    keys
    {
        key(PK; Status, "No.") { Clustered = true; }
        key(ByLevel; Level) { }
    }
}

report 50100 "Repro Replan Orders"
{
    ProcessingOnly = true;
    UsageCategory = None;

    dataset
    {
        dataitem(Order; "Repro Order")
        {
            DataItemTableView = sorting(Status, "No.") where(Status = filter(.. Released));

            trigger OnAfterGetRecord()
            begin
                if First then begin
                    Reset();
                    SetRange("Ref. No.", "Ref. No.");
                    First := false;
                    CollectLowerLevel(Order, Level);
                end;
            end;
        }
    }

    var
        First: Boolean;
        Result: Text;

    trigger OnPreReport()
    begin
        First := true;
    end;

    procedure GetResult(): Text
    begin
        exit(Result);
    end;

    local procedure CollectLowerLevel(ReproOrder: Record "Repro Order"; CurrentLevel: Integer)
    begin
        ReproOrder.SetCurrentKey(Level);
        ReproOrder.SetRange("Ref. No.", ReproOrder."Ref. No.");
        ReproOrder.SetRange(Level, CurrentLevel + 1);
        Result := 'filters=[' + ReproOrder.GetFilters() + '] rows=';
        if ReproOrder.FindSet() then
            repeat
                Result += ReproOrder."No." + ',';
            until ReproOrder.Next() = 0;
    end;
}

codeunit 50120 "Repro ByValue Key Tests"
{
    Subtype = Test;

    [Test]
    procedure ReportProcedureSetCurrentKeyOnDataItemCopy()
    var
        ReproOrder: Record "Repro Order";
        ReplanOrders: Report "Repro Replan Orders";
    begin
        ReproOrder.DeleteAll();
        Insert(ReproOrder.Status::Released, 'O1', 'REF', 0);
        Insert(ReproOrder.Status::Planned, 'O2', 'REF', 1);
        Insert(ReproOrder.Status::Released, 'O3', 'REF', 1);
        Insert(ReproOrder.Status::Finished, 'O4', 'REF', 1);
        Insert(ReproOrder.Status::Released, 'O5', 'OTHER', 1);
        Commit();

        ReproOrder.SetRange(Status, ReproOrder.Status::Released);
        ReproOrder.SetRange("No.", 'O1');
        ReplanOrders.SetTableView(ReproOrder);
        ReplanOrders.UseRequestPage(false);
        ReplanOrders.RunModal();

        if ReplanOrders.GetResult() <> 'filters=[Ref. No.: REF, Level: 1] rows=O2,O3,O4,' then
            Error('Expected the by-value copy to keep none of the data item''s filters (filters=[Ref. No.: REF, Level: 1] rows=O2,O3,O4,), found %1.', ReplanOrders.GetResult());
    end;

    local procedure Insert(Status: Option; No: Code[20]; RefNo: Code[20]; Level: Integer)
    var
        ReproOrder: Record "Repro Order";
    begin
        ReproOrder.Status := Status;
        ReproOrder."No." := No;
        ReproOrder."Ref. No." := RefNo;
        ReproOrder.Level := Level;
        ReproOrder.Insert();
    end;
}
"##;

#[test]
fn issue_8386_reporter_repro_report_procedure_copy_is_flagged() {
    let o = run(ISSUE_8386, "Repro ByValue Key Tests");
    check(&o, &[("ReportProcedureSetCurrentKeyOnDataItemCopy", PASS, true)]);
}

/// al-runner issue 8838, the reporter's repro as written, with Library Assert's AreEqual, IsFalse and IsTrue as `Error` calls
/// (report 7300 "Calculate Bin Replenishment" into codeunit 7308 "Replenishment".ReplenishBin and table 7302 "Bin Content"
/// .NeedToReplenish). The reporter ran it green on BC 29.0.
const ISSUE_8838: &str = r##"
table 50901 "Repro Isolation Row"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; "No."; Code[20]) { DataClassification = CustomerContent; }
        field(2; "Sorting Sequence No."; Integer) { DataClassification = CustomerContent; }
        field(3; Quantity; Decimal) { DataClassification = CustomerContent; }
        field(4; Fixed; Boolean) { DataClassification = CustomerContent; }
    }

    keys
    {
        key(PK; "No.", "Sorting Sequence No.") { Clustered = true; }
    }

    procedure NeedToReplenish(ExcludeQty: Decimal): Boolean
    begin
        if Rec.ReadIsolation() <> IsolationLevel::UpdLock then
            Rec.ReadIsolation(IsolationLevel::ReadCommitted);

        exit(Rec.Quantity > ExcludeQty);
    end;
}

table 50902 "Repro Isolation Result"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; "Entry No."; Integer) { DataClassification = CustomerContent; }
        field(2; "Source No."; Code[20]) { DataClassification = CustomerContent; }
        field(3; "Source Sorting Sequence No."; Integer) { DataClassification = CustomerContent; }
        field(4; Replenish; Boolean) { DataClassification = CustomerContent; }
    }

    keys
    {
        key(PK; "Entry No.") { Clustered = true; }
    }
}

codeunit 50903 "Repro Isolation Caller"
{
    procedure ReplenishBin(ToReproIsolationRow: Record "Repro Isolation Row"): Boolean
    begin
        exit(ToReproIsolationRow.NeedToReplenish(0));
    end;
}

report 50904 "Repro Isolation Descending"
{
    ProcessingOnly = true;
    UsageCategory = None;
    ApplicationArea = All;

    dataset
    {
        dataitem(ReproIsolationRow; "Repro Isolation Row")
        {
            DataItemTableView = sorting("No.", "Sorting Sequence No.") order(descending) where(Fixed = filter(true));

            trigger OnAfterGetRecord()
            begin
                NextEntryNo += 1;
                ReproIsolationResult.Init();
                ReproIsolationResult."Entry No." := NextEntryNo;
                ReproIsolationResult."Source No." := ReproIsolationRow."No.";
                ReproIsolationResult."Source Sorting Sequence No." := ReproIsolationRow."Sorting Sequence No.";
                ReproIsolationResult.Replenish := ReproIsolationCaller.ReplenishBin(ReproIsolationRow);
                ReproIsolationResult.Insert();
            end;
        }
    }

    var
        ReproIsolationResult: Record "Repro Isolation Result";
        ReproIsolationCaller: Codeunit "Repro Isolation Caller";
        NextEntryNo: Integer;
}

codeunit 50911 "Repro Isolation Report Test"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure ADescendingReportHandsItsRowToACodeunitWhoseTableProcedureReadsTheIsolationLevel()
    var
        ReproIsolationResult: Record "Repro Isolation Result";
    begin
        DeleteRows();
        InsertRow('R1', 10000, 5);
        InsertRow('R1', 20000, 0);

        Report.Run(Report::"Repro Isolation Descending", false, false);

        if ReproIsolationResult.Count() <> 2 then
            Error('The rows the report processed: %1', ReproIsolationResult.Count());

        ReproIsolationResult.Get(1);
        if ReproIsolationResult."Source Sorting Sequence No." <> 20000 then
            Error('The sorting sequence number of the row the report read first, reading descending: %1', ReproIsolationResult."Source Sorting Sequence No.");
        if ReproIsolationResult.Replenish then
            Error('Whether the row the report read first, the one with no quantity, needs replenishment');

        ReproIsolationResult.Get(2);
        if not ReproIsolationResult.Replenish then
            Error('Whether the row the report read second, the one with a quantity, needs replenishment');
    end;

    local procedure DeleteRows()
    var
        ReproIsolationRow: Record "Repro Isolation Row";
        ReproIsolationResult: Record "Repro Isolation Result";
    begin
        ReproIsolationRow.DeleteAll(false);
        ReproIsolationResult.DeleteAll(false);
    end;

    local procedure InsertRow(DocumentNo: Code[20]; SortingSequenceNo: Integer; Quantity: Decimal)
    var
        ReproIsolationRow: Record "Repro Isolation Row";
    begin
        ReproIsolationRow.Init();
        ReproIsolationRow."No." := DocumentNo;
        ReproIsolationRow."Sorting Sequence No." := SortingSequenceNo;
        ReproIsolationRow.Quantity := Quantity;
        ReproIsolationRow.Fixed := true;
        ReproIsolationRow.Insert(false);
    end;
}
"##;

#[test]
fn issue_8838_reporter_repro_descending_data_item_to_codeunit_is_flagged() {
    let o = run(ISSUE_8838, "Repro Isolation Report Test");
    check(
        &o,
        &[(
            "ADescendingReportHandsItsRowToACodeunitWhoseTableProcedureReadsTheIsolationLevel",
            PASS,
            true,
        )],
    );
}

/// The neighbors of the probe's cells, each with the value BC gave in the cell (record/filters/ByValueRecordView.al of corpus
/// PR 1450) or a refusal. Rows 1 (C, Open, 30), 2 (B, Posted, 20), 3 (A, Open, 10).
const NEIGHBORS: &str = r##"
table 50180 "BR Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[20]) { }
        field(3; Status; Option) { OptionMembers = Open,Posted; }
        field(4; Amount; Decimal) { }
        field(5; Total; Decimal) { FieldClass = FlowField; CalcFormula = sum("BR Line".Amt where(RowId = field(Id))); }
    }
    keys
    {
        key(PK; Id) { Clustered = true; }
        key(ByName; Name) { }
    }

    procedure KeyOf(Copy: Record "BR Row"): Text
    begin
        exit(Copy.CurrentKey());
    end;

    procedure DescribeThroughReport(): Text
    var
        Copy: Record "BR Row";
        Rep: Report "BR Report";
    begin
        Copy.SetRange(Status, Copy.Status::Open);
        exit(Rep.Describe(Copy));
    end;

    procedure HandToReport(Mode: Integer): Text
    begin
        exit(HandOn(Mode));
    end;

    local procedure HandOn(Mode: Integer): Text
    var
        Copy: Record "BR Row";
        Rep: Report "BR Report";
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

table 50181 "BR Line"
{
    fields { field(1; Id; Integer) { } field(2; RowId; Integer) { } field(3; Amt; Decimal) { } }
    keys { key(PK; Id) { Clustered = true; } }
}

report 50182 "BR Report"
{
    ProcessingOnly = true;
    UseRequestPage = false;

    dataset
    {
        dataitem(Row; "BR Row")
        {
            DataItemTableView = sorting(Name) where(Status = filter(Open));

            dataitem(Line; "BR Line")
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

    local procedure Recurse(R: Record "BR Row"; Level: Integer): Text
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

    local procedure DescribeLine(L: Record "BR Line"): Text
    begin
        exit('f={' + L.GetFilters() + '} n=' + Format(L.Count()));
    end;

    procedure Describe(R: Record "BR Row"): Text
    begin
        exit('f={' + R.GetFilters() + '} n=' + Format(R.Count()) + ' key=' + R.CurrentKey());
    end;

    procedure KeyAndLoad(R: Record "BR Row"): Text
    begin
        R.FindFirst();
        exit('key=' + R.CurrentKey() + ' amount loaded=' + Format(R.AreFieldsLoaded(R.Amount)));
    end;
}

reportextension 50183 "BR Report Ext" extends "BR Report"
{
    procedure DescribeExt(R: Record "BR Row"): Text
    begin
        exit('f={' + R.GetFilters() + '}');
    end;
}

report 50184 "BR Other Report"
{
    ProcessingOnly = true;
    UseRequestPage = false;

    dataset
    {
        dataitem(Row; "BR Row")
        {
            trigger OnAfterGetRecord()
            var
                Rep: Report "BR Report";
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
        Copy: Record "BR Row";
        Rep: Report "BR Report";
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

report 50187 "BR Request Report"
{
    ProcessingOnly = true;

    dataset
    {
        dataitem(Row; "BR Row") { }
    }

    requestpage
    {
        layout
        {
            area(Content)
            {
                field(ShownField; Shown) { ApplicationArea = All; }
            }
        }

        trigger OnOpenPage()
        var
            ReqRow: Record "BR Row";
        begin
            ReqRow.SetRange(Status, ReqRow.Status::Open);
            Outcome := Describe(ReqRow);
        end;
    }

    var
        Shown: Boolean;
        Outcome: Text;

    procedure GetResult(): Text
    begin
        exit(Outcome);
    end;

    local procedure Describe(R: Record "BR Row"): Text
    begin
        exit('f={' + R.GetFilters() + '} n=' + Format(R.Count()));
    end;
}

page 50185 "BR Page"
{
    PageType = List;
    SourceTable = "BR Row";
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
                    Row: Record "BR Row";
                    Rep: Report "BR Report";
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
                    Row: Record "BR Row";
                    Rep: Report "BR Report";
                begin
                    Row.SetLoadFields(Name);
                    Row.SetCurrentKey(Total);
                    Error('%1', Rep.KeyAndLoad(Row));
                end;
            }
        }
    }

    procedure KeyAndLoad(R: Record "BR Row"): Text
    begin
        R.FindFirst();
        exit('key=' + R.CurrentKey() + ' amount loaded=' + Format(R.AreFieldsLoaded(R.Amount)));
    end;
}

interface "BR IKeyAndLoad"
{
    procedure KeyAndLoad(R: Record "BR Row"): Text;
}

codeunit 50188 "BR Codeunit Callee" implements "BR IKeyAndLoad"
{
    procedure KeyAndLoad(R: Record "BR Row"): Text
    begin
        R.FindFirst();
        exit('key=' + R.CurrentKey() + ' amount loaded=' + Format(R.AreFieldsLoaded(R.Amount)));
    end;

    procedure Describe(R: Record "BR Row"): Text
    begin
        exit('f={' + R.GetFilters() + '} key=' + R.CurrentKey());
    end;
}

codeunit 50186 "BR Test"
{
    Subtype = Test;

    local procedure Seed()
    var
        Row: Record "BR Row";
        Line: Record "BR Line";
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

    local procedure AddRow(Id: Integer; Name: Text[20]; Status: Option; Amount: Decimal)
    var
        Row: Record "BR Row";
    begin
        Row.Id := Id;
        Row.Name := Name;
        Row.Status := Status;
        Row.Amount := Amount;
        Row.Insert();
    end;

    local procedure AddLine(Id: Integer; RowId: Integer)
    var
        Line: Record "BR Line";
    begin
        Line.Id := Id;
        Line.RowId := RowId;
        Line.Insert();
    end;

    local procedure RunReport(Mode: Integer): Text
    var
        Row: Record "BR Row";
        Rep: Report "BR Report";
    begin
        Row.SetFilter(Amount, '>=%1', 20);
        Rep.SetMode(Mode);
        Rep.SetTableView(Row);
        Rep.RunModal();
        exit(Rep.GetResult());
    end;

    local procedure Expect(Got: Text; Want: Text)
    begin
        if Got <> Want then
            Error('%1', Got);
    end;

    [Test]
    procedure FromCodeunit_SecondaryKeyAndFilter_Flagged()
    var
        Row: Record "BR Row";
        Rep: Report "BR Report";
    begin
        Seed();
        Row.SetCurrentKey(Name);
        Row.SetRange(Status, Row.Status::Open);
        Expect(Rep.Describe(Row), 'f={} n=3 key=Id');
    end;

    [Test]
    procedure FromCodeunit_SecondaryKeyNoFilter_Flagged()
    var
        Row: Record "BR Row";
        Rep: Report "BR Report";
    begin
        Seed();
        Row.SetCurrentKey(Name);
        Expect(Rep.Describe(Row), 'f={} n=3 key=Id');
    end;

    [Test]
    procedure FromCodeunit_SecondaryKeyNoFilter_AutoCalc_Refused()
    var
        Row: Record "BR Row";
        Rep: Report "BR Report";
    begin
        Seed();
        Row.SetCurrentKey(Name);
        Row.SetAutoCalcFields(Total);
        Expect(Rep.Describe(Row), '');
    end;

    [Test]
    procedure FromCodeunit_FlowFieldKeyAndLoadFields_Flagged()
    var
        Row: Record "BR Row";
        Rep: Report "BR Report";
    begin
        Seed();
        Row.SetLoadFields(Name);
        Row.SetCurrentKey(Total);
        Expect(Rep.Describe(Row), 'f={} n=3 key=Id');
    end;

    [Test]
    procedure FromCodeunit_FlowFieldKeyLoadFieldsReadIsolation_Refused()
    var
        Row: Record "BR Row";
        Rep: Report "BR Report";
    begin
        Seed();
        Row.SetLoadFields(Name);
        Row.ReadIsolation := IsolationLevel::ReadCommitted;
        Row.SetCurrentKey(Total);
        Expect(Rep.Describe(Row), '');
    end;

    [Test]
    procedure TableProcedure_FlowFieldKeyAndLoadFields_Refused()
    var
        Row: Record "BR Row";
    begin
        Seed();
        Row.SetLoadFields(Name);
        Row.SetCurrentKey(Total);
        Expect(Row.KeyOf(Row), '');
    end;

    [Test]
    [HandlerFunctions('RequestHandler')]
    procedure FromRequestPageTrigger_Flagged()
    var
        Rep: Report "BR Request Report";
    begin
        Seed();
        Commit();
        Rep.RunModal();
        Expect(Rep.GetResult(), 'f={} n=3');
    end;

    [RequestPageHandler]
    procedure RequestHandler(var RequestPage: TestRequestPage "BR Request Report")
    begin
        RequestPage.OK().Invoke();
    end;

    [Test]
    procedure FlowFieldKeyAndFilter_ToReport_Refused()
    var
        Row: Record "BR Row";
        Rep: Report "BR Report";
    begin
        Seed();
        Row.SetCurrentKey(Total);
        Row.SetRange(Status, Row.Status::Open);
        Expect(Rep.Describe(Row), '');
    end;

    [Test]
    procedure FlowFieldKeyDescending_ToReport_Refused()
    var
        Row: Record "BR Row";
        Rep: Report "BR Report";
    begin
        Seed();
        Row.SetCurrentKey(Total);
        Row.Ascending(false);
        Expect(Rep.Describe(Row), '');
    end;

    [Test]
    procedure FlowFieldKeyAndFilter_ToCodeunit_Refused()
    var
        Row: Record "BR Row";
        Callee: Codeunit "BR Codeunit Callee";
    begin
        Seed();
        Row.SetCurrentKey(Total);
        Row.SetRange(Status, Row.Status::Open);
        Expect(Callee.Describe(Row), '');
    end;

    [Test]
    procedure FlowFieldKeyLoadFields_FromPageAction_Refused()
    var
        Rows: TestPage "BR Page";
    begin
        Seed();
        Rows.OpenView();
        asserterror Rows.KeyLoadToReport.Invoke();
        Rows.Close();
        Expect(GetLastErrorText(), '');
    end;

    [Test]
    procedure FlowFieldKeyLoadFields_ToPageProcedure_Refused()
    var
        Row: Record "BR Row";
        RowsPage: Page "BR Page";
    begin
        Seed();
        Row.SetLoadFields(Name);
        Row.SetCurrentKey(Total);
        Expect(RowsPage.KeyAndLoad(Row), '');
    end;

    [Test]
    procedure FlowFieldKeyLoadFields_ThroughInterface_Refused()
    var
        Row: Record "BR Row";
        Callee: Codeunit "BR Codeunit Callee";
        KeyAndLoad: Interface "BR IKeyAndLoad";
    begin
        Seed();
        Row.SetLoadFields(Name);
        Row.SetCurrentKey(Total);
        KeyAndLoad := Callee;
        Expect(KeyAndLoad.KeyAndLoad(Row), '');
    end;

    [Test]
    procedure FromCodeunit_FilterGroup3_Refused()
    var
        Row: Record "BR Row";
        Rep: Report "BR Report";
    begin
        Seed();
        Row.FilterGroup(3);
        Row.SetRange(Status, Row.Status::Open);
        Row.FilterGroup(0);
        Expect(Rep.Describe(Row), '');
    end;

    [Test]
    procedure FromPageAction_Flagged()
    var
        Rows: TestPage "BR Page";
    begin
        Seed();
        Rows.OpenView();
        asserterror Rows.DescribeRows.Invoke();
        Rows.Close();
        Expect(GetLastErrorText(), 'f={} n=3 key=Id');
    end;

    [Test]
    procedure FromTableProcedure_Refused()
    var
        Row: Record "BR Row";
    begin
        Seed();
        Expect(Row.DescribeThroughReport(), '');
    end;

    [Test]
    procedure FromTable_FlowFieldKeyLoadFieldsNoFilter_Refused()
    var
        Row: Record "BR Row";
    begin
        Seed();
        Expect(Row.HandToReport(1), '');
    end;

    [Test]
    procedure FromAnotherReport_FlowFieldKeyLoadFieldsNoFilter_Refused()
    var
        Other: Report "BR Other Report";
    begin
        Seed();
        Expect(Other.HandToReport(1), '');
    end;

    [Test]
    procedure FromTable_SecondaryKeyNoFilter_Refused()
    var
        Row: Record "BR Row";
    begin
        Seed();
        Expect(Row.HandToReport(2), '');
    end;

    [Test]
    procedure FromAnotherReport_SecondaryKeyNoFilter_Refused()
    var
        Other: Report "BR Other Report";
    begin
        Seed();
        Expect(Other.HandToReport(2), '');
    end;

    [Test]
    procedure FromTable_LoadFieldsNoFilter_Refused()
    var
        Row: Record "BR Row";
    begin
        Seed();
        Expect(Row.HandToReport(3), '');
    end;

    [Test]
    procedure FromAnotherReport_Refused()
    var
        Row: Record "BR Row";
        Other: Report "BR Other Report";
    begin
        Seed();
        Row.SetRange(Status, Row.Status::Open);
        Other.SetTableView(Row);
        Other.RunModal();
        Expect(Other.GetResult(), '');
    end;

    [Test]
    procedure ReportExtensionProcedure_Refused()
    var
        Row: Record "BR Row";
        Rep: Report "BR Report";
    begin
        Seed();
        Row.SetRange(Status, Row.Status::Open);
        Expect(Rep.DescribeExt(Row), '');
    end;

    [Test]
    procedure DataItemWithItsView_Flagged()
    begin
        Seed();
        Expect(RunReport(1), 'f={} n=3 key=Id');
    end;

    [Test]
    procedure DataItemRecursion_Flagged()
    begin
        Seed();
        Expect(RunReport(2), 'id=3 f={} n=3 key=Id outer f={Status: Open, Amount: <30} key=Name');
    end;

    [Test]
    procedure ChildDataItemLink_Flagged()
    begin
        Seed();
        Expect(RunReport(3), 'f={} n=3');
    end;
}
"##;

#[test]
fn by_value_report_procedure_neighbors_are_flagged_where_measured_and_refused_elsewhere() {
    let o = run(NEIGHBORS, "BR Test");
    let want = [
        // ByValue_ReportProcedure_FromCodeunit_SecondaryKey and _SecondaryKeyNoFilter: `f={} n=3 key=Id`.
        ("FromCodeunit_SecondaryKeyAndFilter_Flagged", PASS, true),
        ("FromCodeunit_SecondaryKeyNoFilter_Flagged", PASS, true),
        // Main kept the secondary key here, which the cell above shows is not BC's view; SetAutoCalcFields is not measured.
        (
            "FromCodeunit_SecondaryKeyNoFilter_AutoCalc_Refused",
            VIEW_REFUSED,
            false,
        ),
        // A key on a FlowField with a SetLoadFields before any read: the copy has the primary key and every field loaded
        // (ByValue_FlowFieldKeyLoadFields_CodeunitProcedure and _ReportProcedure); with ReadIsolation it is not measured.
        ("FromCodeunit_FlowFieldKeyAndLoadFields_Flagged", PASS, true),
        (
            "FromCodeunit_FlowFieldKeyLoadFieldsReadIsolation_Refused",
            VIEW_REFUSED,
            false,
        ),
        // No cell has a FlowField key with a SetLoadFields handed to a table procedure (main answered `key=Total`, clean).
        ("TableProcedure_FlowFieldKeyAndLoadFields_Refused", VIEW_REFUSED, false),
        // ByValue_ReportProcedure_FromRequestPage.
        ("FromRequestPageTrigger_Flagged", PASS, true),
        // A FlowField key with a filter or another order: no cell (main kept the caller's view; BC hands a fresh one in
        // every cell). A FlowField key with a SetLoadFields from a page or to a page procedure or an interface: the cells
        // hand it from a codeunit to a codeunit or a report procedure only.
        ("FlowFieldKeyAndFilter_ToReport_Refused", VIEW_REFUSED, false),
        ("FlowFieldKeyDescending_ToReport_Refused", VIEW_REFUSED, false),
        ("FlowFieldKeyAndFilter_ToCodeunit_Refused", VIEW_REFUSED, false),
        ("FlowFieldKeyLoadFields_FromPageAction_Refused", VIEW_REFUSED, false),
        ("FlowFieldKeyLoadFields_ToPageProcedure_Refused", VIEW_REFUSED, false),
        ("FlowFieldKeyLoadFields_ThroughInterface_Refused", VIEW_REFUSED, false),
        ("FromCodeunit_FilterGroup3_Refused", VIEW_REFUSED, false),
        // ByValue_ReportProcedure_FromPageAction.
        ("FromPageAction_Flagged", PASS, true),
        ("FromTableProcedure_Refused", VIEW_REFUSED, false),
        ("FromAnotherReport_Refused", VIEW_REFUSED, false),
        // The same callers with no filter on the record: main kept their view with no flag (`key=Id amount loaded=No` for a
        // SetLoadFields with or without a FlowField key, `key=Name` for a secondary key), where the measured callers' cells show BC
        // drops both (record/filters/ByValueRecordView.al:1378, :1586, :1603).
        ("FromTable_FlowFieldKeyLoadFieldsNoFilter_Refused", VIEW_REFUSED, false),
        (
            "FromAnotherReport_FlowFieldKeyLoadFieldsNoFilter_Refused",
            VIEW_REFUSED,
            false,
        ),
        ("FromTable_SecondaryKeyNoFilter_Refused", VIEW_REFUSED, false),
        ("FromAnotherReport_SecondaryKeyNoFilter_Refused", VIEW_REFUSED, false),
        ("FromTable_LoadFieldsNoFilter_Refused", VIEW_REFUSED, false),
        ("ReportExtensionProcedure_Refused", VIEW_REFUSED, false),
        // BC hands no DataItemTableView filter (ByValue_ReportProcedure_DataItemView :1675, `callee f={} n=3 key=Id`): the
        // runtime admits the data item's own view in its filter group (al-runner issue 8838).
        ("DataItemWithItsView_Flagged", PASS, true),
        // ByValue_ReportProcedure_DataItemNested and _ChildDataItemLink.
        ("DataItemRecursion_Flagged", PASS, true),
        ("ChildDataItemLink_Flagged", PASS, true),
    ];
    check(&o, &want);
}
