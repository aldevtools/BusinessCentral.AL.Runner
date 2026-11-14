//! A second Open on a query that was read from while rows were left, with no filter change between (al-runner issue 7827).
//! Pinned: the Open as a statement raises nothing, and a SetRange and an Open after it read with the new filter (corpus
//! queries/read-lifecycle/QueryClosedAfterLastRow.al:162-182, Query_RowsLeft_OpenAgainAndSetRange). What a Read, a column
//! read or another Open right after that second Open does, and an Open whose return value is used, are not pinned: they refuse.

use al_testkit::outcomes_in;
use al_testrun::Outcome;

fn objects(on_before_open: bool) -> String {
    let trigger = if on_before_open {
        "trigger OnBeforeOpen() begin end;"
    } else {
        ""
    };
    format!(
        r#"
table 62810 "QR Row"
{{
    fields {{ field(1; Id; Integer) {{ }} field(2; Grp; Code[10]) {{ }} }}
    keys {{ key(PK; Id) {{ Clustered = true; }} }}
}}
query 62811 "QR Rows"
{{
    QueryType = Normal;
    elements {{ dataitem(Row; "QR Row") {{ column(Id; Id) {{ }} column(Grp; Grp) {{ }} }} }}
    {trigger}
}}
"#
    )
}

/// The outcome of a test whose body runs `steps` after seeding three rows, opening the query and reading one row.
fn outcome(on_before_open: bool, steps: &str) -> Outcome {
    let src = format!(
        r#"{}
codeunit 62812 T
{{
    Subtype = Test;
    var Q: Query "QR Rows"; Rows: Text;
    [Test] procedure Run()
    var R: Record "QR Row";
    begin
        R.DeleteAll();
        R.Id := 1; R.Grp := 'A'; R.Insert();
        R.Id := 2; R.Grp := 'A'; R.Insert();
        R.Id := 3; R.Grp := 'B'; R.Insert();
        Q.Open();
        if not Q.Read() then Error('read 1');
        {steps}
    end;
}}"#,
        objects(on_before_open)
    );
    let o = outcomes_in(&src, "T");
    o[0].1.clone()
}

const PINNED_END: &str = "Q.SetRange(Grp, 'B'); Q.Open(); while Q.Read() do Rows += Format(Q.Id) + ','; if Rows <> '3,' then Error('rows %1', Rows);";

#[test]
fn open_as_a_statement_with_rows_left_then_setrange_and_open_reads_the_new_filter() {
    assert_eq!(outcome(false, &format!("Q.Open(); {PINNED_END}")), Outcome::Pass);
}

#[test]
fn a_read_a_column_read_or_another_open_right_after_the_second_open_refuses() {
    for next in [
        "if Q.Read() then;",
        "Rows := Format(Q.Id);",
        "Q.Open();",
        "Q.SetFilter(Grp, 'B');",
        "Q.Close();",
    ] {
        let o = outcome(false, &format!("Q.Open(); {next}"));
        assert!(
            matches!(&o, Outcome::NotRun(why) if why.contains("opened again while rows were left")),
            "{next}: {o:?}"
        );
    }
}

// Main's refusal is kept for an Open whose return value is used (the issue's own repro), a query with OnBeforeOpen, a second
// Read before the Open, and a write to the table before the Open.
#[test]
fn an_open_whose_result_is_used_or_a_query_with_on_before_open_keeps_the_refusal() {
    let pinned = format!("Q.Open(); {PINNED_END}");
    let read_twice = format!("if not Q.Read() then Error('read 2'); {pinned}");
    let written = format!("R.Id := 4; R.Grp := 'B'; R.Insert(); {pinned}");
    for (trigger, steps) in [
        (
            false,
            "if not Q.Open() then Error('open 2'); if not Q.Read() then Error('read 2');",
        ),
        (true, pinned.as_str()),
        (false, read_twice.as_str()),
        (false, written.as_str()),
    ] {
        let o = outcome(trigger, steps);
        assert!(
            matches!(&o, Outcome::NotRun(why) if why.contains("already open and was read from")),
            "{steps}: {o:?}"
        );
    }
}
