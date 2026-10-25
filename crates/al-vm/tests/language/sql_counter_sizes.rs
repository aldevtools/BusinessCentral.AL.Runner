//! Exact sizes of the change of `SessionInformation.SqlStatementsExecuted()` and `SqlRowsRead()` (al-runner issue 5761).
//! The corpus pins them for nine shapes of record methods on a two-field table, each measured right after a Commit that
//! left the table empty or with N rows, for N = 1, 2, 5, 10, 20 (session/lt-bl-sql-counters/SqlCounters.al:204-395).
//! Any other shape, N, table or precondition keeps only the sign of the change, or refuses.

use al_builtins::native::Signal;
use al_testkit::{run, text};

const TABLE: &str = "table 50100 Row { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } } keys { key(PK; Id) { Clustered = true; } } }";

/// The corpus's RoReset and RoMeasure (SqlCounters.al:206-276), on table `Row`.
fn src(extra: &str) -> String {
    format!(
        "{TABLE}
table 50102 Wide {{ fields {{ field(1; Id; Integer) {{ }} field(2; Name; Text[30]) {{ }} field(3; Extra; Integer) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}
codeunit 50101 C {{
    var StmtBefore: BigInteger; RowsBefore: BigInteger;
    local procedure Mark() begin StmtBefore := SessionInformation.SqlStatementsExecuted(); RowsBefore := SessionInformation.SqlRowsRead(); end;
    local procedure RoReset(Rows: Integer)
    var Row: Record Row; I: Integer;
    begin
        Row.DeleteAll();
        for I := 1 to Rows do begin Row.Id := I; Row.Name := 'row ' + Format(I); Row.Insert(); end;
        Commit();
    end;
    local procedure Measured(): Text
    var Stmt: BigInteger; Rows: BigInteger;
    begin
        Stmt := SessionInformation.SqlStatementsExecuted();
        Rows := SessionInformation.SqlRowsRead();
        exit(Format(Stmt - StmtBefore) + '/' + Format(Rows - RowsBefore));
    end;
    procedure RoMeasure(Kind: Integer; N: Integer): Text
    var Row: Record Row; I: Integer; Cnt: Integer;
    begin
        if Kind in [1, 2] then RoReset(0) else RoReset(N);
        Mark();
        case Kind of
            1, 2:
                begin
                    for I := 1 to N do begin Row.Init(); Row.Id := 100 + I; Row.Name := 'Row'; Row.Insert(); end;
                    if Kind = 2 then Cnt := Row.Count();
                end;
            3: for I := 1 to N do Row.Get(I);
            4: for I := 1 to N do Row.Get(1);
            5: if Row.FindSet() then repeat until Row.Next() = 0;
            6: for I := 1 to N do begin Row.Get(I); Row.Name := 'changed'; Row.Modify(); end;
            7: for I := 1 to N do begin Row.Get(I); Row.Delete(); end;
            8: Row.DeleteAll();
            9: Cnt := Row.Count();
        end;
        exit(Measured());
    end;
    local procedure Twice(Kind: Integer; N: Integer): Text begin exit(RoMeasure(Kind, N) + ',' + RoMeasure(Kind, N)); end;
    procedure Series(Kind: Integer): Text
    begin
        exit(Twice(Kind, 1) + ' ' + Twice(Kind, 2) + ' ' + Twice(Kind, 5) + ' ' + Twice(Kind, 10) + ' ' + Twice(Kind, 20));
    end;
    {extra}
}}"
    )
}

fn refused(r: Result<al_builtins::value::Value, Signal>) {
    match r {
        Err(Signal::Unsupported(why)) => assert!(why.contains("SQL counter"), "{why}"),
        other => panic!("expected a SQL counter refusal, got {other:?}"),
    }
}

fn series(kind: u32) -> String {
    text(run(
        &src(&format!("procedure F(): Text begin exit(Series({kind})); end;")),
        "F",
    ))
}

fn twice(values: [&str; 5]) -> String {
    values.map(|v| format!("{v},{v}")).join(" ")
}

/// SqlCounters.al:323-334: inserts are buffered; a Count after them flushes.
#[test]
fn inserts_and_inserts_with_count_give_the_measured_sizes() {
    assert_eq!(series(1), twice(["0/0", "0/0", "1/0", "2/0", "4/0"]));
    assert_eq!(series(2), twice(["2/1", "3/1", "2/1", "3/1", "5/1"]));
}

/// SqlCounters.al:345-359: Get of distinct rows, Get of the same row, a FindSet loop.
#[test]
fn reads_give_the_measured_sizes() {
    assert_eq!(series(3), twice(["1/1", "2/2", "5/5", "10/10", "20/20"]));
    assert_eq!(series(4), twice(["1/1"; 5]));
    assert_eq!(series(5), twice(["1/2", "1/3", "1/6", "1/11", "1/21"]));
}

/// SqlCounters.al:371-390: Get and Modify, Get and Delete, DeleteAll, Count.
#[test]
fn writes_and_count_give_the_measured_sizes() {
    assert_eq!(series(6), twice(["2/2", "4/4", "10/10", "20/20", "40/40"]));
    assert_eq!(series(7), twice(["2/1", "4/2", "10/5", "20/10", "40/20"]));
    assert_eq!(series(8), twice(["1/0"; 5]));
    assert_eq!(series(9), twice(["1/1"; 5]));
}

/// The reporter's shape (SqlCounters.al:282-305): 10 inserts and a Count right after a Commit of the emptied table.
#[test]
fn the_statement_budget_repro_passes() {
    let s = src("procedure F(): Text var Row: Record Row; Before: BigInteger; After: BigInteger; Executed: BigInteger; I: Integer;
        begin
            Row.DeleteAll();
            Commit();
            Before := SessionInformation.SqlStatementsExecuted();
            for I := 1 to 10 do begin Row.Init(); Row.Id := I; Row.Name := 'Row'; Row.Insert(); end;
            if Row.Count() <> 10 then Error('Expected 10 rows.');
            After := SessionInformation.SqlStatementsExecuted();
            Executed := After - Before;
            exit(Format((Executed > 0) and (Executed <= 100)) + ' ' + Format(Executed));
        end;");
    assert_eq!(text(run(&s, "F")), "Yes 3");
}

/// An N the corpus does not measure (3, 0, 21) keeps the size unknown.
#[test]
fn an_unmeasured_n_is_refused() {
    for (kind, n) in [(1, 3), (2, 3), (3, 3), (5, 4), (8, 3), (9, 0), (6, 21)] {
        refused(run(
            &src(&format!("procedure F(): Text begin exit(RoMeasure({kind}, {n})); end;")),
            "F",
        ));
    }
}

/// The measurement must start right after the Commit: work between the Commit and the first read, or no Commit at all.
#[test]
fn a_measurement_not_right_after_a_commit_is_refused() {
    refused(run(
        &src("procedure F(): Text var Row: Record Row; I: Integer;
            begin RoReset(0); Row.Id := 1; Row.Insert(); Mark();
            for I := 2 to 3 do begin Row.Init(); Row.Id := I; Row.Insert(); end; exit(Measured()); end;"),
        "F",
    ));
    refused(run(
        &src("procedure F(): Text var Row: Record Row; I: Integer;
            begin Row.DeleteAll(); Mark(); for I := 1 to 2 do begin Row.Init(); Row.Id := I; Row.Insert(); end; exit(Measured()); end;"),
        "F",
    ));
}

/// The table must hold what the measurement started from: N rows for the reads and writes, none for the inserts.
#[test]
fn a_table_in_another_state_is_refused() {
    // Inserts into a table that already holds 5 rows.
    refused(run(
        &src("procedure F(): Text var Row: Record Row; I: Integer;
            begin RoReset(5); Mark(); for I := 1 to 2 do begin Row.Init(); Row.Id := 100 + I; Row.Insert(); end; exit(Measured()); end;"),
        "F",
    ));
    // Two Gets in a table of 5 rows.
    refused(run(
        &src(
            "procedure F(): Text var Row: Record Row; begin RoReset(5); Mark(); Row.Get(1); Row.Get(2); exit(Measured()); end;",
        ),
        "F",
    ));
}

/// Other shapes: a filter, inserts without Init, a FindSet loop stopped early, a mix of kinds, another table layout.
#[test]
fn an_unmeasured_shape_is_refused() {
    for body in [
        "RoReset(5); Mark(); Row.SetRange(Id, 1, 5); Cnt := Row.Count();",
        "RoReset(0); Mark(); Row.Id := 1; Row.Insert(); Row.Id := 2; Row.Insert();",
        "RoReset(2); Mark(); Row.FindSet(); Row.Next();",
        "RoReset(2); Mark(); Row.Get(1); Cnt := Row.Count();",
        "RoReset(1); Mark(); Row.Get(1); Row.Name := 'x'; Row.Modify(true);",
        "Wide.DeleteAll(); Commit(); Mark(); Wide.Init(); Wide.Id := 1; Wide.Insert(); Wide.Init(); Wide.Id := 2; Wide.Insert(); Cnt := Wide.Count();",
    ] {
        let s = src(&format!(
            "procedure F(): Text var Row: Record Row; Wide: Record Wide; Cnt: Integer; begin {body} exit(Measured()); end;"
        ));
        refused(run(&s, "F"));
    }
}
/// Nothing between two reads: the change is 0 (SqlCounters.al:105-119), a plain number.
#[test]
fn nothing_between_two_reads_is_a_change_of_zero() {
    let s = src("procedure F(): Text var X: Integer; begin Mark(); X := 1 + 2; exit(Measured()); end;");
    assert_eq!(text(run(&s, "F")), "0/0");
}
