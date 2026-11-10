//! `GetFilters` of a query with a static ColumnFilter on a plain (not aggregated) column (al-runner issue 5199). No
//! corpus test reads the getters on such a column (queries/TestQueryColumnFilter.al:40-100 pins the rows only), so
//! `GetFilter` of the column is refused, and `GetFilters` is refused the same way: it used to leave the column out
//! and answer as if the query had no such filter. Runtime filters alone stay pinned
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

const REFUSED: &str = "refused:GetFilters of a query with a static ColumnFilter on a plain column (not pinned)";

#[test]
fn getfilters_refuses_a_static_filter_on_a_plain_column_as_getfilter_does() {
    assert_eq!(
        run("GetFilterOfTheColumn"),
        "refused:GetFilter of a column with a static ColumnFilter (not pinned)"
    );
    assert_eq!(run("StaticOnly"), REFUSED);
    assert_eq!(run("StaticAndRuntimeOnAnotherColumn"), REFUSED);
}

#[test]
fn getfilters_without_a_static_filter_answers_as_before() {
    assert_eq!(run("RuntimeOnly"), "[Id: 1..5]");
    assert_eq!(run("NoFilter"), "[]");
}
