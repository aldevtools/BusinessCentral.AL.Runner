// Neighbors of al-runner issue 8838: a report's data item that still holds its DataItemTableView, handed by value from its
// OnAfterGetRecord. Rows 1 (C, Open, 30), 2 (B, Posted, 20), 3 (A, Open, 0), 4 (D, Open, 10).
table 50800 "NB DIV Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[20]) { }
        field(3; Status; Option) { OptionMembers = Open,Posted; }
        field(4; Amount; Decimal) { }
    }
    keys
    {
        key(PK; Id) { Clustered = true; }
        key(ByName; Name) { }
    }

    procedure NeedToReplenish(): Text
    var
        Before: Text;
    begin
        Before := Format(Rec.ReadIsolation());
        if Rec.ReadIsolation() <> IsolationLevel::UpdLock then
            Rec.ReadIsolation(IsolationLevel::ReadCommitted);
        exit(' iso before=' + Before + ' after=' + Format(Rec.ReadIsolation()));
    end;
}

table 50801 "NB DIV Other"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Data; Blob) { }
    }
    keys
    {
        key(PK; Id) { Clustered = true; }
    }

    procedure DescribeRow(R: Record "NB DIV Row"): Text
    begin
        exit('f={' + R.GetFilters() + '} n=' + Format(R.Count()) + ' key=' + R.CurrentKey());
    end;
}

codeunit 50802 "NB DIV Callee"
{
    procedure Describe(R: Record "NB DIV Row"): Text
    begin
        exit('f={' + R.GetFilters() + '} n=' + Format(R.Count()) + ' key=' + R.CurrentKey() + ' asc=' + Format(R.Ascending()));
    end;

    procedure Replenish(R: Record "NB DIV Row"): Text
    begin
        exit(Describe(R) + R.NeedToReplenish());
    end;
}

codeunit 50803 "NB DIV Result"
{
    SingleInstance = true;

    var
        Outcome: Text;

    procedure Set(NewOutcome: Text)
    begin
        Outcome := NewOutcome;
    end;

    procedure Get(): Text
    begin
        exit(Outcome);
    end;
}

report 50804 "NB DIV Desc"
{
    ProcessingOnly = true;
    UseRequestPage = false;

    dataset
    {
        dataitem(Row; "NB DIV Row")
        {
            DataItemTableView = sorting(Id) order(descending) where(Status = filter(Open));

            trigger OnPreDataItem()
            var
                Callee: Codeunit "NB DIV Callee";
            begin
                if Mode = 9 then
                    Outcome := Callee.Describe(Row);
            end;

            trigger OnAfterGetRecord()
            var
                Callee: Codeunit "NB DIV Callee";
                Other: Record "NB DIV Other";
                Copy: Record "NB DIV Row";
                OtherRep: Report "NB DIV Asc";
            begin
                if Done or (Mode = 9) then
                    exit;
                Done := true;
                case Mode of
                    1:
                        Outcome := Callee.Replenish(Row);
                    2:
                        Outcome := Describe(Row);
                    3:
                        begin
                            FilterGroup(5);
                            SetFilter(Amount, '>%1', 0);
                            FilterGroup(0);
                            Outcome := Callee.Describe(Row);
                        end;
                    4:
                        begin
                            Copy.Copy(Row);
                            Outcome := Callee.Describe(Copy);
                        end;
                    5:
                        Outcome := Other.DescribeRow(Row);
                    6:
                        Outcome := OtherRep.Describe(Row);
                    7:
                        begin
                            Reset();
                            Outcome := Callee.Describe(Row);
                        end;
                    8:
                        Outcome := HandOn(Row);
                end;
            end;
        }
    }

    var
        Mode: Integer;
        Done: Boolean;
        Outcome: Text;

    procedure SetMode(NewMode: Integer)
    begin
        Mode := NewMode;
    end;

    procedure GetResult(): Text
    begin
        exit(Outcome);
    end;

    procedure Describe(R: Record "NB DIV Row"): Text
    begin
        exit('f={' + R.GetFilters() + '} n=' + Format(R.Count()) + ' key=' + R.CurrentKey() + ' asc=' + Format(R.Ascending()));
    end;

    local procedure HandOn(R: Record "NB DIV Row"): Text
    var
        Callee: Codeunit "NB DIV Callee";
    begin
        exit(Callee.Describe(R));
    end;

    procedure DescribeItemAfterRun(): Text
    var
        Callee: Codeunit "NB DIV Callee";
    begin
        exit(Callee.Describe(Row));
    end;
}

report 50805 "NB DIV Name"
{
    ProcessingOnly = true;
    UseRequestPage = false;

    dataset
    {
        dataitem(Row; "NB DIV Row")
        {
            DataItemTableView = sorting(Name) order(descending) where(Status = filter(Open), Amount = filter(> 0));

            trigger OnAfterGetRecord()
            var
                Callee: Codeunit "NB DIV Callee";
            begin
                if Done then
                    exit;
                Done := true;
                Outcome := Callee.Replenish(Row);
            end;
        }
    }

    var
        Done: Boolean;
        Outcome: Text;

    procedure GetResult(): Text
    begin
        exit(Outcome);
    end;
}

report 50806 "NB DIV Asc"
{
    ProcessingOnly = true;
    UseRequestPage = false;

    dataset
    {
        dataitem(Row; "NB DIV Row")
        {
            DataItemTableView = sorting(Name) where(Status = filter(Open));

            trigger OnAfterGetRecord()
            var
                Callee: Codeunit "NB DIV Callee";
            begin
                if Done then
                    exit;
                Done := true;
                Outcome := Callee.Describe(Row);
            end;
        }
    }

    var
        Done: Boolean;
        Outcome: Text;

    procedure GetResult(): Text
    begin
        exit(Outcome);
    end;

    procedure Describe(R: Record "NB DIV Row"): Text
    begin
        exit('f={' + R.GetFilters() + '} n=' + Format(R.Count()) + ' key=' + R.CurrentKey());
    end;
}

report 50809 "NB DIV Extended"
{
    ProcessingOnly = true;
    UseRequestPage = false;

    dataset
    {
        dataitem(Row; "NB DIV Row")
        {
            DataItemTableView = sorting(Id) order(descending) where(Status = filter(Open));
        }
    }
}

reportextension 50807 "NB DIV Extended Ext" extends "NB DIV Extended"
{
    dataset
    {
        modify(Row)
        {
            trigger OnAfterAfterGetRecord()
            var
                Callee: Codeunit "NB DIV Callee";
                Result: Codeunit "NB DIV Result";
            begin
                if Result.Get() = '' then
                    Result.Set(Callee.Describe(Row));
            end;
        }
    }
}

xmlport 50808 "NB DIV Xml"
{
    Direction = Export;

    schema
    {
        textelement(Root)
        {
            tableelement(Row; "NB DIV Row")
            {
                SourceTableView = sorting(Id) order(descending) where(Status = filter(Open));

                fieldattribute(Id; Row.Id) { }

                trigger OnAfterGetRecord()
                var
                    Callee: Codeunit "NB DIV Callee";
                begin
                    if Outcome = '' then
                        Outcome := Callee.Describe(Row);
                end;
            }
        }
    }

    var
        Outcome: Text;

    procedure GetResult(): Text
    begin
        exit(Outcome);
    end;
}

codeunit 50810 "NB DIV Test"
{
    Subtype = Test;

    local procedure Seed()
    var
        Row: Record "NB DIV Row";
        Result: Codeunit "NB DIV Result";
    begin
        Row.DeleteAll();
        AddRow(1, 'C', Row.Status::Open, 30);
        AddRow(2, 'B', Row.Status::Posted, 20);
        AddRow(3, 'A', Row.Status::Open, 0);
        AddRow(4, 'D', Row.Status::Open, 10);
        Result.Set('');
    end;

    local procedure AddRow(Id: Integer; Name: Text[20]; Status: Option; Amount: Decimal)
    var
        Row: Record "NB DIV Row";
    begin
        Row.Id := Id;
        Row.Name := Name;
        Row.Status := Status;
        Row.Amount := Amount;
        Row.Insert();
    end;

    local procedure RunDesc(Mode: Integer): Text
    var
        Rep: Report "NB DIV Desc";
    begin
        Rep.SetMode(Mode);
        Rep.RunModal();
        exit(Rep.GetResult());
    end;

    local procedure Expect(Got: Text; Want: Text)
    begin
        if Got <> Want then
            Error('%1', Got);
    end;

    // The issue's shape (corpus PR 1544, ByValueDataItemView.al:220).
    [Test]
    procedure DescendingToCodeunitTableProcedure()
    begin
        Seed();
        Expect(RunDesc(1), 'f={} n=4 key=Id asc=Yes iso before=Default after=ReadCommitted');
    end;

    // ByValueDataItemView.al:234.
    [Test]
    procedure DescendingToOwnReportProcedure()
    begin
        Seed();
        Expect(RunDesc(2), 'f={} n=4 key=Id asc=Yes');
    end;

    // ByValueDataItemView.al:248.
    [Test]
    procedure SecondaryKeyDescendingWithSetTableView()
    var
        Row: Record "NB DIV Row";
        Rep: Report "NB DIV Name";
    begin
        Seed();
        Row.SetFilter(Amount, '<%1', 30);
        Rep.SetTableView(Row);
        Rep.RunModal();
        Expect(Rep.GetResult(), 'f={} n=4 key=Id asc=Yes iso before=Default after=ReadCommitted');
    end;

    // ByValueRecordView.al:1690 of corpus PR 1450: ascending, to a codeunit.
    [Test]
    procedure AscendingToCodeunit()
    var
        Rep: Report "NB DIV Asc";
    begin
        Seed();
        Rep.RunModal();
        Expect(Rep.GetResult(), 'f={} n=4 key=Id asc=Yes');
    end;

    // A filter code set in filter group 5 on top of the view: not the view the run set.
    [Test]
    procedure FilterGroup5SetByCode()
    begin
        Seed();
        Expect(RunDesc(3), 'f={} n=4 key=Id asc=Yes');
    end;

    // A copy of the data item holds its filters but is not the data item.
    [Test]
    procedure CopyOfTheDataItem()
    begin
        Seed();
        Expect(RunDesc(4), 'f={} n=4 key=Id asc=Yes');
    end;

    // A table procedure whose parameter is the data item (Buffer.Proc(Entry) from report code).
    [Test]
    procedure ToTableProcedure()
    begin
        Seed();
        Expect(RunDesc(5), 'f={} n=4 key=Id');
    end;

    // Another report's procedure.
    [Test]
    procedure ToOtherReportProcedure()
    begin
        Seed();
        Expect(RunDesc(6), 'f={} n=4 key=Id');
    end;

    // After Reset the data item holds no filter: the codeunit cells pin that view.
    [Test]
    procedure AfterReset()
    begin
        Seed();
        Expect(RunDesc(7), 'f={} n=4 key=Id asc=Yes');
    end;

    // A local procedure of the report hands its copy on: the copy no longer holds the view.
    [Test]
    procedure ThroughOwnProcedureToCodeunit()
    begin
        Seed();
        Expect(RunDesc(8), 'f={} n=4 key=Id asc=Yes');
    end;

    // OnPreDataItem: another trigger of the data item.
    [Test]
    procedure FromOnPreDataItem()
    begin
        Seed();
        Expect(RunDesc(9), 'f={} n=4 key=Id asc=Yes');
    end;

    // A report extension's trigger on the data item.
    [Test]
    procedure FromReportExtensionTrigger()
    var
        Result: Codeunit "NB DIV Result";
    begin
        Seed();
        Report.RunModal(Report::"NB DIV Extended");
        Expect(Result.Get(), 'f={} n=4 key=Id asc=Yes');
    end;

    // A procedure of the report that hands its data item after the run ended.
    [Test]
    procedure AfterTheRun()
    var
        Rep: Report "NB DIV Desc";
    begin
        Seed();
        Rep.SetMode(0);
        Rep.RunModal();
        Expect(Rep.DescribeItemAfterRun(), 'f={} n=4 key=Id asc=Yes');
    end;

    // An XmlPort's SourceTableView is held in the same filter group number.
    [Test]
    procedure XmlPortSourceTableView()
    var
        Other: Record "NB DIV Other";
        Xml: XmlPort "NB DIV Xml";
        OutS: OutStream;
    begin
        Seed();
        Other.Data.CreateOutStream(OutS);
        Xml.SetDestination(OutS);
        Xml.Export();
        Expect(Xml.GetResult(), 'f={} n=4 key=Id asc=Yes');
    end;
}
