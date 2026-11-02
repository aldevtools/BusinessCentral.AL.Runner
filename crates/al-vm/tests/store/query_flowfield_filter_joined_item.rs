//! A query `filter()` element over a FlowField of the JOINED data item of an inner join, with a Sum column on the root item: the
//! FlowField is calculated per joined row before the grouping. Every literal is what real BC answered in the corpus:
//! query/lt-y67-query-flowfield-filter/Y67QueryFlowFieldFilter.al (codeunit 93644, writer Y67).

use al_testkit::text;

const SRC: &str = r#"
table 50196 QH
{
    fields
    {
        field(1; "No."; Code[10]) { }
        field(3; LineCount; Integer) { FieldClass = FlowField; CalcFormula = count(QL where("Hdr No." = field("No."))); }
        field(4; LineSum; Decimal) { FieldClass = FlowField; CalcFormula = sum(QL.Amount where("Hdr No." = field("No."))); }
    }
    keys { key(PK; "No.") { Clustered = true; } }
}

table 50197 QL
{
    fields
    {
        field(1; "Hdr No."; Code[10]) { }
        field(2; "Line No."; Integer) { }
        field(3; Amount; Decimal) { }
    }
    keys { key(PK; "Hdr No.", "Line No.") { Clustered = true; } }
}

query 50196 JoinedFilter
{
    elements
    {
        dataitem(L; QL)
        {
            column(HdrNo; "Hdr No.") { }
            column(Total; Amount) { Method = Sum; }
            dataitem(H; QH)
            {
                DataItemLink = "No." = L."Hdr No.";
                SqlJoinType = InnerJoin;
                filter(LineSum; LineSum) { ColumnFilter = LineSum = filter(<> 0); }
            }
        }
    }
}

query 50197 JoinedOpen
{
    elements
    {
        dataitem(L; QL)
        {
            column(HdrNo; "Hdr No.") { }
            column(Total; Amount) { Method = Sum; }
            dataitem(H; QH)
            {
                DataItemLink = "No." = L."Hdr No.";
                SqlJoinType = InnerJoin;
                filter(LineSum; LineSum) { }
                column(Cnt; LineCount) { }
            }
        }
    }
}

query 50198 OuterJoined
{
    elements
    {
        dataitem(L; QL)
        {
            column(HdrNo; "Hdr No.") { }
            column(Total; Amount) { Method = Sum; }
            dataitem(H; QH)
            {
                DataItemLink = "No." = L."Hdr No.";
                SqlJoinType = LeftOuterJoin;
                filter(LineSum; LineSum) { ColumnFilter = LineSum = filter(<> 0); }
            }
        }
    }
}

codeunit 50196 C
{
    local procedure Seed()
    begin
        AddHdr('H1'); AddHdr('H2'); AddHdr('H3'); AddHdr('H4');
        AddLine('H1', 1, 10); AddLine('H1', 2, 20); AddLine('H2', 1, 5); AddLine('H4', 1, 7); AddLine('H4', 2, -7);
    end;

    local procedure AddHdr(No: Code[10])
    var
        Hd: Record QH;
    begin
        Hd."No." := No;
        Hd.Insert();
    end;

    local procedure AddLine(HdrNo: Code[10]; LineNo: Integer; Amount: Decimal)
    var
        Ln: Record QL;
    begin
        Ln."Hdr No." := HdrNo;
        Ln."Line No." := LineNo;
        Ln.Amount := Amount;
        Ln.Insert();
    end;

    procedure Joined(): Text
    var
        Q: Query JoinedFilter;
        Q2: Query JoinedFilter;
        Txt: Text;
    begin
        Seed();
        Q.Open();
        while Q.Read() do
            Txt += Q.HdrNo + '=' + Format(Q.Total, 0, 9) + ' ';
        Txt += '/';
        Q2.SetFilter(LineSum, '>15');
        Q2.Open();
        while Q2.Read() do
            Txt += Q2.HdrNo + '=' + Format(Q2.Total, 0, 9) + ' ';
        exit(Txt);
    end;

    procedure Open(): Text
    var
        Q: Query JoinedOpen;
        Q2: Query JoinedOpen;
        Txt: Text;
    begin
        Seed();
        Q.Open();
        while Q.Read() do
            Txt += Q.HdrNo + '=' + Format(Q.Total, 0, 9) + '/' + Format(Q.Cnt) + ' ';
        Txt += '|';
        Q2.SetRange(LineSum, 0);
        Q2.Open();
        while Q2.Read() do
            Txt += Q2.HdrNo + '=' + Format(Q2.Total, 0, 9) + '/' + Format(Q2.Cnt) + ' ';
        exit(Txt);
    end;

    procedure Outer(): Text
    var
        Q: Query OuterJoined;
    begin
        Seed();
        Q.Open();
        exit('opened');
    end;
}
"#;

/// FlowFieldFilter_Joined_RootSum (Y67QueryFlowFieldFilter.al:350-371): the static filter and a run-time filter over it.
#[test]
fn a_flowfield_filter_of_the_inner_joined_item_filters_before_the_grouping() {
    assert_eq!(text(al_testkit::run(SRC, "Joined")), "H1=30 H2=5 /H1=30 ");
}

/// FlowFieldFilter_Joined_RunTimeOnly (Y67QueryFlowFieldFilter.al:373-401): no filter, then SetRange 0, with a FlowField
/// column of the joined item.
#[test]
fn a_run_time_filter_on_the_joined_flowfield_element() {
    assert_eq!(text(al_testkit::run(SRC, "Open")), "H1=30/2 H2=5/1 H4=0/2 |H4=0/2 ");
}

/// The FlowField filter of a LEFT OUTER joined item is not pinned (a missing row would meet it with 0): refused.
#[test]
fn a_flowfield_filter_of_a_left_outer_joined_item_stays_refused() {
    let r = al_testkit::run(SRC, "Outer");
    assert!(matches!(&r, Err(al_builtins::native::Signal::Unsupported(_))), "{r:?}");
}
