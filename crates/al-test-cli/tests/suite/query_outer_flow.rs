//! FlowFields and FlowFilters of a left-outer-joined query data item (al-runner issues 6988, 6989, 6990 and 6991, follow-ups
//! to 6174). The tables, the rows and every expected text are those of the corpus probes
//! queries/flowfields-filters/QueryOuterFlowKinds.al and QueryOuterFlowFilters.al (corpus branch al-runtime-rs-staging-r6988).

use super::user_reports::{all_pass, run_with_deps};

/// Tables 95710-95713 and the seed of codeunit 95718 "QOF Seed" of QueryOuterFlowKinds.al.
const TABLES: &str = r#"
table 95710 "QOF Parent" { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } } keys { key(PK; Id) { Clustered = true; } } }
table 95711 "QOF Child"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; ParentId; Integer) { }
        field(3; RefCode; Code[10]) { }
        field(4; Kind; Code[10]) { }
        field(10; Total; Decimal) { FieldClass = FlowField; CalcFormula = sum("QOF Detail".Amount where(ChildId = field(Id), Day = field("Day Filter"))); }
        field(11; NegTotal; Decimal) { FieldClass = FlowField; CalcFormula = - sum("QOF Detail".Amount where(ChildId = field(Id))); }
        field(12; Cnt; Integer) { FieldClass = FlowField; CalcFormula = count("QOF Detail" where(ChildId = field(Id), Day = field("Day Filter"))); }
        field(13; HasDetail; Boolean) { FieldClass = FlowField; CalcFormula = exist("QOF Detail" where(ChildId = field(Id))); }
        field(14; MinAmt; Decimal) { FieldClass = FlowField; CalcFormula = min("QOF Detail".Amount where(ChildId = field(Id))); }
        field(15; MaxAmt; Decimal) { FieldClass = FlowField; CalcFormula = max("QOF Detail".Amount where(ChildId = field(Id))); }
        field(16; AvgAmt; Decimal) { FieldClass = FlowField; CalcFormula = average("QOF Detail".Amount where(ChildId = field(Id))); }
        field(17; ParentName; Text[30]) { FieldClass = FlowField; CalcFormula = lookup("QOF Parent".Name where(Id = field(ParentId))); }
        field(18; RefName; Text[30]) { FieldClass = FlowField; CalcFormula = lookup("QOF Ref".Name where(Code = field(RefCode))); }
        field(20; "Day Filter"; Integer) { FieldClass = FlowFilter; }
    }
    keys { key(PK; Id) { Clustered = true; } key(ByParent; ParentId) { } }
}
table 95712 "QOF Detail" { fields { field(1; Id; Integer) { } field(2; ChildId; Integer) { } field(3; Amount; Decimal) { } field(4; Day; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }
table 95713 "QOF Ref" { fields { field(1; Code; Code[10]) { } field(2; Name; Text[30]) { } } keys { key(PK; Code) { Clustered = true; } } }
codeunit 95718 "QOF Seed"
{
    procedure Seed()
    var
        Parent: Record "QOF Parent";
        Child: Record "QOF Child";
        Detail: Record "QOF Detail";
        Ref: Record "QOF Ref";
    begin
        Parent.DeleteAll();
        Child.DeleteAll();
        Detail.DeleteAll();
        Ref.DeleteAll();
        AddParent(1, 'P-One');
        AddParent(2, 'P-Two');
        AddParent(3, 'P-Three');
        AddParent(4, 'P-Four');
        Ref.Code := 'R1';
        Ref.Name := 'Ref One';
        Ref.Insert();
        Ref.Code := 'R2';
        Ref.Name := 'Ref Two';
        Ref.Insert();
        AddChild(10, 1, 'R1', 'A');
        AddChild(11, 1, '', 'B');
        AddChild(30, 3, 'R2', 'A');
        AddChild(40, 4, 'RX', 'A');
        AddChild(50, 99, 'R1', 'A');
        AddDetail(100, 10, 5, 1);
        AddDetail(101, 10, 7.25, 2);
        AddDetail(102, 11, 1, 1);
        AddDetail(103, 40, -3, 2);
        AddDetail(104, 40, 4, 1);
    end;

    local procedure AddParent(Id: Integer; Name: Text[30])
    var
        Parent: Record "QOF Parent";
    begin
        Parent.Id := Id;
        Parent.Name := Name;
        Parent.Insert();
    end;

    local procedure AddChild(Id: Integer; ParentId: Integer; RefCode: Code[10]; Kind: Code[10])
    var
        Child: Record "QOF Child";
    begin
        Child.Id := Id;
        Child.ParentId := ParentId;
        Child.RefCode := RefCode;
        Child.Kind := Kind;
        Child.Insert();
    end;

    local procedure AddDetail(Id: Integer; ChildId: Integer; Amount: Decimal; Day: Integer)
    var
        Detail: Record "QOF Detail";
    begin
        Detail.Id := Id;
        Detail.ChildId := ChildId;
        Detail.Amount := Amount;
        Detail.Day := Day;
        Detail.Insert();
    end;
}
"#;

fn run(objects: &str) {
    all_pass(run_with_deps(&[], &format!("{TABLES}{objects}")));
}

/// Issue 6991 (Base Application query 1319 "Sales by Cust. Grp. Chart Mgt."): a filter() element over the FlowFilter field of a
/// child data item without SqlJoinType (a left outer join) sets the flow filter of the FlowFields of that item; it is no column
/// read and drops no row. A Method = Sum column over such a FlowField gives 0 for a parent without a child.
/// Pins: QueryOuterFlowFilters.al OuterFlowFilter_SumColumns_Rows and OuterFlowFilter_PlainColumns_Rows, InnerFlowFilter_SumColumns_Rows.
#[test]
fn issue_6991_flowfilter_filter_element_of_outer_joined_item() {
    run(r#"
query 95721 "QOF Outer FlowFilter"
{
    OrderBy = ascending(PId), ascending(CId);
    elements
    {
        dataitem(Parent; "QOF Parent")
        {
            column(PId; Id) { }
            dataitem(Child; "QOF Child")
            {
                DataItemLink = ParentId = Parent.Id;
                SqlJoinType = LeftOuterJoin;
                filter(DayF; "Day Filter") { }
                column(CId; Id) { }
                column(Total; Total) { }
                column(Cnt; Cnt) { }
            }
        }
    }
}
query 95722 "QOF Outer FlowFilter Sum"
{
    OrderBy = ascending(PId);
    elements
    {
        dataitem(Parent; "QOF Parent")
        {
            column(PId; Id) { }
            dataitem(Child; "QOF Child")
            {
                DataItemLink = ParentId = Parent.Id;
                filter(DayF; "Day Filter") { }
                column(SumTotal; Total) { Method = Sum; }
                column(SumCnt; Cnt) { Method = Sum; }
            }
        }
    }
}
query 95723 "QOF Inner FlowFilter Sum"
{
    OrderBy = ascending(PId);
    elements
    {
        dataitem(Parent; "QOF Parent")
        {
            column(PId; Id) { }
            dataitem(Child; "QOF Child")
            {
                DataItemLink = ParentId = Parent.Id;
                SqlJoinType = InnerJoin;
                filter(DayF; "Day Filter") { }
                column(SumTotal; Total) { Method = Sum; }
                column(SumCnt; Cnt) { Method = Sum; }
            }
        }
    }
}
codeunit 95725 T
{
    Subtype = Test;

    local procedure Plain(var Q: Query "QOF Outer FlowFilter") Rows: Text
    begin
        Q.Open();
        while Q.Read() do
            Rows += Format(Q.PId) + ':' + Format(Q.CId) + '=' + Format(Q.Total, 0, 9) + '/' + Format(Q.Cnt) + ' ';
        Q.Close();
    end;

    local procedure Outer(var Q: Query "QOF Outer FlowFilter Sum") Rows: Text
    begin
        Q.Open();
        while Q.Read() do
            Rows += Format(Q.PId) + '=' + Format(Q.SumTotal, 0, 9) + '/' + Format(Q.SumCnt) + ' ';
        Q.Close();
    end;

    local procedure Inner(var Q: Query "QOF Inner FlowFilter Sum") Rows: Text
    begin
        Q.Open();
        while Q.Read() do
            Rows += Format(Q.PId) + '=' + Format(Q.SumTotal, 0, 9) + '/' + Format(Q.SumCnt) + ' ';
        Q.Close();
    end;

    local procedure Check(Name: Text; Got: Text; Expected: Text)
    begin
        if Got <> Expected then
            Error('%1: got [%2], expected [%3]', Name, Got, Expected);
    end;

    [Test]
    procedure PlainColumns()
    var
        Seeder: Codeunit "QOF Seed";
        Q: Query "QOF Outer FlowFilter";
    begin
        Seeder.Seed();
        Check('no filter', Plain(Q), '1:10=12.25/2 1:11=1/1 2:0=0/0 3:30=0/0 4:40=1/2 ');
        Clear(Q);
        Q.SetRange(DayF, 1);
        Check('day 1', Plain(Q), '1:10=5/1 1:11=1/1 2:0=0/0 3:30=0/0 4:40=4/1 ');
        Clear(Q);
        Q.SetRange(DayF, 2, 3);
        Check('day 2..3', Plain(Q), '1:10=7.25/1 1:11=0/0 2:0=0/0 3:30=0/0 4:40=-3/1 ');
        Clear(Q);
        Q.SetRange(DayF, 9);
        Check('day 9', Plain(Q), '1:10=0/0 1:11=0/0 2:0=0/0 3:30=0/0 4:40=0/0 ');
    end;

    [Test]
    procedure SumColumns()
    var
        Seeder: Codeunit "QOF Seed";
        Q: Query "QOF Outer FlowFilter Sum";
    begin
        Seeder.Seed();
        Check('no filter', Outer(Q), '1=13.25/3 2=0/0 3=0/0 4=1/2 ');
        Clear(Q);
        Q.SetRange(DayF, 1);
        Check('day 1', Outer(Q), '1=6/2 2=0/0 3=0/0 4=4/1 ');
        Clear(Q);
        Q.SetRange(DayF, 2, 3);
        Check('day 2..3', Outer(Q), '1=7.25/1 2=0/0 3=0/0 4=-3/1 ');
        Clear(Q);
        Q.SetRange(DayF, 9);
        Check('day 9', Outer(Q), '1=0/0 2=0/0 3=0/0 4=0/0 ');
    end;

    [Test]
    procedure InnerSumColumns()
    var
        Seeder: Codeunit "QOF Seed";
        Q: Query "QOF Inner FlowFilter Sum";
    begin
        Seeder.Seed();
        Check('no filter', Inner(Q), '1=13.25/3 3=0/0 4=1/2 ');
        Clear(Q);
        Q.SetRange(DayF, 1);
        Check('day 1', Inner(Q), '1=6/2 3=0/0 4=4/1 ');
        Clear(Q);
        Q.SetRange(DayF, 9);
        Check('day 9', Inner(Q), '1=0/0 3=0/0 4=0/0 ');
    end;
}
"#);
}
