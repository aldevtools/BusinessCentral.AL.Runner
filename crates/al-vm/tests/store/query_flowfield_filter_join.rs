//! A query `filter()` element over a FlowField of the header data item when the query joins a line data item or groups:
//! the FlowField is calculated per header row, before the join and the grouping. Every literal is what real BC answered in the
//! corpus: query/lt-y62-flowfield-filter-join/Y62QueryFlowFieldJoin.al (codeunit 93261, writer Y62).

use al_testkit::text;

const SRC: &str = r#"
table 50193 H
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; "Posting Date"; Date) { }
        field(3; Cancelled; Boolean) { FieldClass = FlowField; CalcFormula = exist(X where("Doc No." = field("No."))); }
        field(4; Amount; Decimal) { FieldClass = FlowField; CalcFormula = sum(L.Amount where("Doc No." = field("No."))); }
    }
    keys { key(PK; "No.") { Clustered = true; } }
}

table 50194 L
{
    fields
    {
        field(1; "Doc No."; Code[20]) { }
        field(2; "Line No."; Integer) { }
        field(3; ItemNo; Code[20]) { }
        field(4; Qty; Decimal) { }
        field(5; Amount; Decimal) { }
    }
    keys { key(PK; "Doc No.", "Line No.") { Clustered = true; } }
}

table 50195 X
{
    fields { field(1; "Doc No."; Code[20]) { } }
    keys { key(PK; "Doc No.") { Clustered = true; } }
}

query 50193 JoinMethod
{
    OrderBy = descending(SumQty);
    elements
    {
        dataitem(Header; H)
        {
            filter(Canc; Cancelled) { ColumnFilter = Canc = const(false); }
            filter(Amt; Amount) { ColumnFilter = Amt = filter('>0'); }
            dataitem(Line; L)
            {
                DataItemLink = "Doc No." = Header."No.";
                SqlJoinType = InnerJoin;
                column(ItemNo; ItemNo) { }
                column(SumQty; Qty) { Method = Sum; }
            }
        }
    }
}

query 50194 OuterMethod
{
    OrderBy = ascending(No, ItemNo);
    elements
    {
        dataitem(Header; H)
        {
            column(No; "No.") { }
            filter(Canc; Cancelled) { ColumnFilter = Canc = const(false); }
            filter(Amt; Amount) { ColumnFilter = Amt = filter('>0'); }
            dataitem(Line; L)
            {
                DataItemLink = "Doc No." = Header."No.";
                SqlJoinType = LeftOuterJoin;
                column(ItemNo; ItemNo) { }
                column(SumQty; Qty) { Method = Sum; }
            }
        }
    }
}

codeunit 50193 C
{
    local procedure Seed()
    var
        Hd: Record H;
        Ln: Record L;
        Cn: Record X;
    begin
        Hd."No." := 'H1'; Hd.Insert();
        Hd."No." := 'H2'; Hd.Insert();
        Hd."No." := 'H3'; Hd.Insert();
        Hd."No." := 'H4'; Hd.Insert();
        Hd."No." := 'H5'; Hd.Insert();
        AddLine('H1', 10000, 'A', 3, 60);
        AddLine('H1', 20000, 'B', 1, 40);
        AddLine('H2', 10000, 'A', 5, 5);
        AddLine('H2', 20000, 'A', 1, -5);
        AddLine('H4', 10000, 'A', 2, 50);
        AddLine('H5', 10000, 'B', 7, 70);
        Cn."Doc No." := 'H4'; Cn.Insert();
    end;

    local procedure AddLine(DocNo: Code[20]; LineNo: Integer; ItemNo: Code[20]; Qty: Decimal; Amount: Decimal)
    var
        Ln: Record L;
    begin
        Ln."Doc No." := DocNo;
        Ln."Line No." := LineNo;
        Ln.ItemNo := ItemNo;
        Ln.Qty := Qty;
        Ln.Amount := Amount;
        Ln.Insert();
    end;

    procedure Rows(): Text
    var
        Q: Query JoinMethod;
        Q2: Query JoinMethod;
        O: Query OuterMethod;
        Txt: Text;
    begin
        Seed();
        Q.Open();
        while Q.Read() do
            Txt += Q.ItemNo + '|' + Format(Q.SumQty, 0, 9) + ';';
        Q2.SetRange(Canc, true);
        Q2.Open();
        Txt += '/';
        while Q2.Read() do
            Txt += Q2.ItemNo + '|' + Format(Q2.SumQty, 0, 9) + ';';
        O.SetFilter(Amt, '>=0');
        O.Open();
        Txt += '/';
        while O.Read() do
            Txt += O.No + '|' + O.ItemNo + '|' + Format(O.SumQty, 0, 9) + ';';
        exit(Txt);
    end;
}
"#;

/// FlowFieldFilter_Join_Method `mode1`, `mode3` and FlowFieldFilter_OuterJoin_Method `mode2`: each header is kept or dropped by
/// its own FlowField value before the lines are joined and summed; a run-time filter replaces the static one; a header
/// without lines is kept by a left outer join with a blank item and a sum of 0.
#[test]
fn a_flowfield_filter_element_filters_the_header_rows_before_the_join_and_the_grouping() {
    assert_eq!(
        text(al_testkit::run(SRC, "Rows")),
        "B|8;A|3;/A|2;/H1|A|3;H1|B|1;H2|A|6;H3||0;H5|B|7;"
    );
}
