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
