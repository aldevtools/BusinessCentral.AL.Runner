//! A static ColumnFilter on a Sum, Average, Min or Max column of an outer-joined data item filters the groups after the
//! join (HAVING), and a parent without a child meets it with an aggregate of 0; GetFilter and GetFilters read the static
//! filter back (queries/flowfields-filters/Y78QueryOuterAggFilter.al:1353-1394 for a LeftOuterJoin, :1397 RightOuterJoin,
//! :1441 FullOuterJoin). Issue 6675: Microsoft's query "Sales Reserv. From Item Ledger" (page "O365 Activities").

const SRC: &str = r#"
table 68961 "Probe OAgg Par"
{
    fields { field(1; Code; Code[10]) { } }
    keys { key(PK; Code) { Clustered = true; } }
}

table 68962 "Probe OAgg Chi"
{
    fields { field(1; Entry; Integer) { } field(2; ParCode; Code[10]) { } field(3; Qty; Decimal) { } }
    keys { key(PK; Entry) { Clustered = true; } }
}

query 89961 QSum
{
    QueryType = Normal;
    OrderBy = ascending(PCode);
    elements
    {
        dataitem(P; "Probe OAgg Par")
        {
            column(PCode; Code) { }
            dataitem(C; "Probe OAgg Chi")
            {
                DataItemLink = ParCode = P.Code;
                SqlJoinType = LeftOuterJoin;
                column(SumQ; Qty) { Method = Sum; ColumnFilter = SumQ = filter(> 0); }
                column(CntQ) { Method = Count; }
            }
        }
    }
}

query 89962 QMin
{
    QueryType = Normal;
    OrderBy = ascending(PCode);
    elements
    {
        dataitem(P; "Probe OAgg Par")
        {
            column(PCode; Code) { }
            dataitem(C; "Probe OAgg Chi")
            {
                DataItemLink = ParCode = P.Code;
                SqlJoinType = LeftOuterJoin;
                column(MinQ; Qty) { Method = Min; ColumnFilter = MinQ = filter(< 10); }
                column(CntQ) { Method = Count; }
            }
        }
    }
}

codeunit 50100 C
{
    local procedure Seed()
    var
        Par: Record "Probe OAgg Par";
        Chi: Record "Probe OAgg Chi";
    begin
        Par.Code := 'P1'; Par.Insert();
        Par.Code := 'P2'; Par.Insert();
        Par.Code := 'P3'; Par.Insert();
        Par.Code := 'P4'; Par.Insert();
        Par.Code := 'P5'; Par.Insert();
        AddChi(1, 'P1', 10);
        AddChi(2, 'P1', 20);
        AddChi(3, 'P2', 30);
        AddChi(4, 'P4', 0);
        AddChi(5, 'P5', 5);
        AddChi(6, 'P5', -5);
        AddChi(7, 'PX', 7);
    end;

    local procedure AddChi(No: Integer; Par: Code[10]; Qty: Decimal)
    var
        Chi: Record "Probe OAgg Chi";
    begin
        Chi.Entry := No; Chi.ParCode := Par; Chi.Qty := Qty; Chi.Insert();
    end;

    procedure SumRows(): Text
    var
        Q: Query QSum;
        R: Text;
    begin
        Seed();
        Q.Open();
        while Q.Read() do
            R += '[' + Q.PCode + '/' + Format(Q.SumQ) + '/' + Format(Q.CntQ) + ']';
        exit(R);
    end;

    procedure MinRows(): Text
    var
        Q: Query QMin;
        R: Text;
    begin
        Seed();
        Q.Open();
        while Q.Read() do
            R += '[' + Q.PCode + '/' + Format(Q.MinQ) + '/' + Format(Q.CntQ) + ']';
        exit(R);
    end;

    procedure Filters(): Text
    var
        S: Query QSum;
        M: Query QMin;
    begin
        exit(S.GetFilters() + '|' + S.GetFilter(SumQ) + '|' + M.GetFilters() + '|' + M.GetFilter(MinQ));
    end;
}
"#;

#[test]
fn a_sum_column_filter_drops_the_groups_after_a_left_outer_join() {
    // Y78QueryOuterAggFilter.al:1380.
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "SumRows")),
        "[P1/30/2][P2/30/1]"
    );
}

#[test]
fn a_parent_without_a_child_meets_a_min_column_filter_with_0() {
    // Y78QueryOuterAggFilter.al:1386.
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "MinRows")),
        "[P3/0/0][P4/0/1][P5/-5/2]"
    );
}

#[test]
fn get_filter_and_get_filters_read_the_static_filter_of_an_aggregate_column() {
    // Y78QueryOuterAggFilter.al:1378-1379, :1384-1385.
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "Filters")),
        "SumQ: >0|>0|MinQ: <10|<10"
    );
}
