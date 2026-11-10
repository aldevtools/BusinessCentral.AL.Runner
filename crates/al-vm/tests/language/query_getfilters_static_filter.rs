//! `GetFilters` of a query with a static ColumnFilter on a plain (not aggregated) column (al-runner issue 5199). No
//! corpus test reads the getters on such a column (queries/TestQueryColumnFilter.al:40-100 pins the rows only), so
//! both getters refuse, in every state of the query (also after a SetRange on that column). `GetFilters` used to leave
//! the column out and answer as if the query had no such filter. Runtime filters alone stay pinned
//! (queries/TestQueryRuntimeContracts.al:203-229).

use al_builtins::native::Signal;
use al_builtins::value::Value;

fn out(r: Result<Value, Signal>) -> String {
    match r {
        Ok(Value::Text(t)) => format!("[{t}]"),
        Err(Signal::Error(e)) => format!("error:{}|{}", e.message, e.code),
        Err(Signal::Unsupported(why)) => format!("refused:{why}"),
        other => format!("{other:?}"),
    }
}

const SRC: &str = r#"
table 50100 "QF Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Kind; Integer) { }
        field(3; Amount; Decimal) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

query 50100 "QF Static"
{
    elements
    {
        dataitem(Row; "QF Row")
        {
            column(Id; Id) { }
            column(Kind; Kind) { ColumnFilter = Kind = const(2); }
        }
    }
}

query 50102 "QF Element"
{
    elements
    {
        dataitem(Row; "QF Row")
        {
            column(Id; Id) { }
            filter(KindFilter; Kind) { ColumnFilter = KindFilter = const(2); }
        }
    }
}

query 50101 "QF Plain"
{
    elements
    {
        dataitem(Row; "QF Row")
        {
            column(Id; Id) { }
            column(Kind; Kind) { }
        }
    }
}

codeunit 50100 C
{
    procedure StaticOnly(): Text
    var
        Q: Query "QF Static";
    begin
        exit(Q.GetFilters());
    end;

    procedure StaticAndRuntimeOnAnotherColumn(): Text
    var
        Q: Query "QF Static";
    begin
        Q.SetRange(Id, 1, 5);
        exit(Q.GetFilters());
    end;

    procedure GetFilterOfTheColumn(): Text
    var
        Q: Query "QF Static";
    begin
        exit(Q.GetFilter(Kind));
    end;

    procedure RuntimeOnTheSameColumn(): Text
    var
        Q: Query "QF Static";
    begin
        Q.SetRange(Kind, 3);
        exit(Q.GetFilters());
    end;

    procedure GetFilterAfterRuntimeOnTheSameColumn(): Text
    var
        Q: Query "QF Static";
    begin
        Q.SetRange(Kind, 3);
        exit(Q.GetFilter(Kind));
    end;

    procedure GetFilterOfAnotherColumn(): Text
    var
        Q: Query "QF Static";
    begin
        Q.SetRange(Id, 1, 5);
        exit(Q.GetFilter(Id));
    end;

    procedure FilterElement(): Text
    var
        Q: Query "QF Element";
    begin
        exit(Q.GetFilters());
    end;

    procedure RuntimeOnly(): Text
    var
        Q: Query "QF Plain";
    begin
        Q.SetRange(Id, 1, 5);
        exit(Q.GetFilters());
    end;

    procedure NoFilter(): Text
    var
        Q: Query "QF Plain";
    begin
        exit(Q.GetFilters());
    end;
}
"#;

fn run(routine: &str) -> String {
    out(al_testkit::run_in(SRC, "C", routine))
}

const GET_FILTERS: &str = "refused:GetFilters on a query column or filter element that is not aggregated and has a static ColumnFilter (not pinned)";
const GET_FILTER: &str = "refused:GetFilter on a query column or filter element that is not aggregated and has a static ColumnFilter (not pinned)";

#[test]
fn both_getters_refuse_a_static_filter_that_is_not_aggregated() {
    assert_eq!(run("GetFilterOfTheColumn"), GET_FILTER);
    assert_eq!(run("StaticOnly"), GET_FILTERS);
    assert_eq!(run("StaticAndRuntimeOnAnotherColumn"), GET_FILTERS);
    assert_eq!(run("FilterElement"), GET_FILTERS);
}

#[test]
fn a_runtime_filter_on_that_column_does_not_lift_the_refusal() {
    assert_eq!(run("RuntimeOnTheSameColumn"), GET_FILTERS);
    assert_eq!(run("GetFilterAfterRuntimeOnTheSameColumn"), GET_FILTER);
}

#[test]
fn getfilters_without_a_static_filter_answers_as_before() {
    assert_eq!(run("RuntimeOnly"), "[Id: 1..5]");
    assert_eq!(run("NoFilter"), "[]");
    // GetFilter of another column of the query with the static filter: a runtime filter reads back as on any query.
    assert_eq!(run("GetFilterOfAnotherColumn"), "[1..5]");
}
