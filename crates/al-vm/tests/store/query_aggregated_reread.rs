//! Read on an open aggregated query after a write to its table: the pinned shapes read the groups again and go on after the
//! last group returned (corpus queries/read-lifecycle/QueryAggregatedWriteInLoop.al, QueryLiveWrite.al); the shapes the corpus
//! has not measured keep main's refusal.

use al_builtins::native::Signal;
use al_testkit::text;

const SRC: &str = r#"
table 50290 T
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Grp; Code[10]) { }
        field(3; Val; Decimal) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

query 50290 CountSum
{
    OrderBy = ascending(Grp);
    elements
    {
        dataitem(S; T)
        {
            column(Grp; Grp) { }
            column(Cnt) { Method = Count; }
            column(Total; Val) { Method = Sum; }
        }
    }
}

query 50291 Avg
{
    elements
    {
        dataitem(S; T)
        {
            column(Grp; Grp) { }
            column(Mean; Val) { Method = Average; }
        }
    }
}

query 50292 OrderByCount
{
    OrderBy = descending(Cnt);
    elements
    {
        dataitem(S; T)
        {
            column(Grp; Grp) { }
            column(Cnt) { Method = Count; }
        }
    }
}

codeunit 50290 C
{
    local procedure Ins(IdNo: Integer; G: Code[10]; V: Decimal)
    var
        Row: Record T;
    begin
        Row.Id := IdNo;
        Row.Grp := G;
        Row.Val := V;
        Row.Insert();
    end;

    local procedure Seed()
    begin
        Ins(1, 'A', 1);
        Ins(2, 'A', 2);
        Ins(3, 'B', 10);
    end;

    procedure CountSum(): Text
    var
        Q: Query CountSum;
        Txt: Text;
    begin
        Seed();
        Q.Open();
        Q.Read();
        Ins(4, 'A', 100);
        Ins(5, 'C', 7);
        while Q.Read() do
            Txt += Q.Grp + '=' + Format(Q.Cnt) + '/' + Format(Q.Total) + ' ';
        exit(Txt);
    end;

    procedure Average(): Text
    var
        Q: Query Avg;
    begin
        Seed();
        Q.Open();
        Q.Read();
        Ins(4, 'C', 100);
        Q.Read();
        exit(Q.Grp);
    end;

    procedure OrderByCount(): Text
    var
        Q: Query OrderByCount;
    begin
        Seed();
        Q.Open();
        Q.Read();
        Ins(4, 'C', 100);
        Q.Read();
        exit(Q.Grp);
    end;
}
"#;

fn refused(r: Result<al_builtins::value::Value, Signal>) -> String {
    match r {
        Err(Signal::Unsupported(why)) => why.to_string(),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// The pinned shape: a Count and a Sum grouped by one Code column, OrderBy on it. The group already read is not handed out
/// again, the groups after it are read with the rows as they are now (QueryLiveWrite.al:108-111).
#[test]
fn a_pinned_aggregated_query_reads_the_groups_after_the_last_one_again() {
    assert_eq!(text(al_testkit::run(SRC, "CountSum")), "B=1/10 C=1/7 ");
}

/// An Average column is not in the measured shapes: main's refusal stays.
#[test]
fn an_average_column_keeps_the_refusal() {
    assert!(refused(al_testkit::run(SRC, "Average")).contains("open aggregated query"));
}

/// OrderBy on an aggregated column is not in the measured shapes: main's refusal stays.
#[test]
fn an_order_by_on_an_aggregated_column_keeps_the_refusal() {
    assert!(refused(al_testkit::run(SRC, "OrderByCount")).contains("open aggregated query"));
}

const KEYS: &str = r#"
table 50295 L
{
    fields
    {
        field(1; Header; Code[10]) { }
        field(2; LineNo; Integer) { }
        field(3; Contact; Code[10]) { }
        field(4; Kind; Integer) { }
        field(5; Grp; Code[10]) { }
    }
    keys { key(PK; Header, LineNo) { Clustered = true; } }
}

// The cell of QueryAggregatedWriteInLoop.al:41-54: every group column but the last two filtered by a SetRange to one value.
query 50295 Overlap
{
    elements
    {
        dataitem(D; L)
        {
            column(Qty) { Method = Count; }
            column(Contact; Contact) { }
            column(Header; Header) { }
            column(LineNo; LineNo) { }
            column(Grp; Grp) { }
        }
    }
}

// Two group columns, both free.
query 50296 TwoFree
{
    elements
    {
        dataitem(D; L)
        {
            column(Contact; Contact) { }
            column(LineNo; LineNo) { }
            column(Qty) { Method = Count; }
        }
    }
}

// OrderBy on a group column whose values tie.
query 50297 OrderTies
{
    OrderBy = ascending(Kind);
    elements
    {
        dataitem(D; L)
        {
            column(Contact; Contact) { }
            column(Kind; Kind) { }
            column(Qty) { Method = Count; }
        }
    }
}

// One group column, no filter: the cell of QueryAggregatedWriteInLoop.al:75-84.
query 50298 ByContact
{
    elements
    {
        dataitem(D; L)
        {
            column(Contact; Contact) { }
            column(Qty) { Method = Count; }
        }
    }
}

codeunit 50295 K
{
    local procedure Ins(H: Code[10]; N: Integer; C: Code[10]; K: Integer)
    var
        Row: Record L;
    begin
        Row.Header := H;
        Row.LineNo := N;
        Row.Contact := C;
        Row.Kind := K;
        Row.Grp := 'G';
        Row.Insert();
    end;

    local procedure Seed()
    var
        Row: Record L;
    begin
        Row.DeleteAll();
        Ins('H1', 10, 'C1', 1);
        Ins('H1', 20, 'C1', 2);
        Ins('H1', 30, 'C2', 1);
        Ins('H2', 10, 'C1', 1);
    end;

    procedure Pinned(): Text
    var
        Q: Query Overlap;
        Txt: Text;
    begin
        Seed();
        Q.SetRange(Contact, 'C1');
        Q.SetRange(Header, 'H1');
        Q.Open();
        Q.Read();
        Txt := Format(Q.LineNo) + ' ';
        Ins('H1', 15, 'C1', 1);
        Ins('H1', 25, 'C1', 1);
        while Q.Read() do
            Txt += Format(Q.LineNo) + ' ';
        exit(Txt);
    end;

    procedure TwoFreeColumns(): Text
    var
        Q: Query TwoFree;
    begin
        Seed();
        Q.Open();
        Q.Read();
        Ins('H3', 5, 'C0', 1);
        Q.Read();
        exit(Q.Contact);
    end;

    procedure SetFilterRange(): Text
    var
        Q: Query Overlap;
    begin
        Seed();
        Q.SetFilter(Contact, '<>C9');
        Q.SetRange(Header, 'H1');
        Q.Open();
        Q.Read();
        Ins('H1', 15, 'C1', 1);
        Q.Read();
        exit(Q.Contact);
    end;

    procedure SetFilterEqual(): Text
    var
        Q: Query Overlap;
    begin
        Seed();
        Q.SetFilter(Contact, 'C1');
        Q.SetRange(Header, 'H1');
        Q.Open();
        Q.Read();
        Ins('H1', 15, 'C1', 1);
        Q.Read();
        exit(Q.Contact);
    end;

    procedure FilterOnLineOnly(): Text
    var
        Q: Query Overlap;
    begin
        Seed();
        Q.SetRange(LineNo, 10);
        Q.Open();
        Q.Read();
        Ins('H4', 10, 'C4', 1);
        Q.Read();
        exit(Q.Contact);
    end;

    procedure OrderByWithTies(): Text
    var
        Q: Query OrderTies;
    begin
        Seed();
        Q.Open();
        Q.Read();
        Ins('H3', 5, 'C0', 1);
        Q.Read();
        exit(Q.Contact);
    end;

    procedure ReadAfterEnd(): Text
    var
        Q: Query ByContact;
    begin
        Seed();
        Q.Open();
        while Q.Read() do;
        Ins('H3', 5, 'C7', 1);
        exit(Format(Q.Read()));
    end;
}
"#;

/// The 75082 cell: the rows after the last line returned, with the inserted lines (QueryAggregatedWriteInLoop.al:254-433).
#[test]
fn the_filtered_cell_reads_the_lines_after_the_last_one_again() {
    assert_eq!(text(al_testkit::run(KEYS, "Pinned")), "10 15 20 25 ");
}

/// Shapes no case measured keep main's refusal: two free group columns, a SetFilter (a range or one value), a filter on another
/// column than the leading ones, an OrderBy whose values tie, a Read after Read returned false.
#[test]
fn group_key_shapes_no_case_measured_keep_the_refusal() {
    for name in [
        "TwoFreeColumns",
        "SetFilterRange",
        "SetFilterEqual",
        "FilterOnLineOnly",
        "OrderByWithTies",
        "ReadAfterEnd",
    ] {
        let why = refused(al_testkit::run(KEYS, name));
        assert!(why.contains("open aggregated query"), "{name}: {why}");
    }
}
