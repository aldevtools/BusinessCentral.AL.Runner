//! `SessionInformation.SqlStatementsExecuted()` and `SqlRowsRead()` (issue 5580). The corpus pins only properties of the
//! counters, never a count (session/lt-bl-sql-counters/SqlCounters.al:74-202, session/TestMiscTypes.al:97-114): the value
//! is not negative, it is a BigInteger, and the sign of the change between two reads around pinned operations. The values
//! the runtime gives are a model (one per pinned increase), so a value may only reach a comparison whose answer the pins
//! decide; any other use refuses the test.

use al_builtins::native::Signal;
use al_testkit::{boolean, run, text};

const TABLE: &str = "table 50100 Row { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } } keys { key(PK; Id) { Clustered = true; } } }";

fn src(body: &str) -> String {
    format!(
        "{TABLE}
codeunit 50101 C {{
    var StmtBefore: BigInteger; RowsBefore: BigInteger;
    local procedure Mark() begin StmtBefore := SessionInformation.SqlStatementsExecuted(); RowsBefore := SessionInformation.SqlRowsRead(); end;
    local procedure Sign(Delta: BigInteger): Text begin if Delta > 0 then exit('increased'); if Delta < 0 then exit('decreased'); exit('same'); end;
    local procedure Note(): Text
    var Stmt: BigInteger; Rows: BigInteger;
    begin
        Stmt := SessionInformation.SqlStatementsExecuted();
        Rows := SessionInformation.SqlRowsRead();
        exit('statements ' + Sign(Stmt - StmtBefore) + ', rows ' + Sign(Rows - RowsBefore));
    end;
    local procedure Seed()
    var Row: Record Row; I: Integer;
    begin
        Row.DeleteAll();
        for I := 1 to 3 do begin Row.Id := I; Row.Name := 'row'; Row.Insert(); end;
        Commit();
    end;
    {body}
}}"
    )
}

fn refused(r: Result<al_builtins::value::Value, Signal>, needle: &str) {
    match r {
        Err(Signal::Unsupported(why)) => assert!(why.contains(needle), "{why}"),
        other => panic!("expected a refusal containing `{needle}`, got {other:?}"),
    }
}

/// SqlCounters.al:74-84 and TestMiscTypes.al:97-114: both counters are not negative.
#[test]
fn a_counter_is_not_negative() {
    let s = src(
        "procedure F(): Boolean begin exit((SessionInformation.SqlStatementsExecuted() >= 0) and (SessionInformation.SqlRowsRead() >= 0)); end;",
    );
    assert!(boolean(run(&s, "F")));
}

/// SqlCounters.al:87-103: the value is a BigInteger, also through a Variant.
#[test]
fn a_counter_is_a_biginteger_in_a_variant() {
    let s = src(
        "procedure F(): Text var V: Variant; begin V := SessionInformation.SqlStatementsExecuted(); exit(Format(V.IsBigInteger()) + Format(V.IsInteger())); end;",
    );
    assert_eq!(text(run(&s, "F")), "YesNo");
}

/// SqlCounters.al:105-119: two reads with nothing, or only arithmetic, between them give the same value.
#[test]
fn nothing_between_two_reads_changes_nothing() {
    let s = src("procedure F(): Text var X: Integer; begin Mark(); X := 1 + 2; exit(Note()); end;");
    assert_eq!(text(run(&s, "F")), "statements same, rows same");
}

/// SqlCounters.al:122-159: the pinned sequence on a table after a Commit.
#[test]
fn the_pinned_table_operations_change_the_counters_as_measured() {
    let s = src("procedure F(): Text var Row: Record Row; N: Integer; R: Text;
        begin
            Mark();
            Seed();
            Mark(); Row.Get(2); R := Note();
            Mark(); Row.FindSet(); repeat until Row.Next() = 0; R += '|' + Note();
            Mark(); N := Row.Count(); R += '|' + Note();
            Mark(); Row.Id := 10; Row.Name := 'ten'; Row.Insert(); R += '|' + Note();
            Mark(); Row.Name := 'TEN'; Row.Modify(); R += '|' + Note();
            Mark(); Row.Delete(); R += '|' + Note();
            exit(R);
        end;");
    assert_eq!(
        text(run(&s, "F")),
        "statements increased, rows increased|statements increased, rows increased|statements same, rows same|\
         statements same, rows same|statements increased, rows increased|statements increased, rows same"
    );
}

/// SqlCounters.al:161-201: operations on a temporary record change neither counter.
#[test]
fn temporary_record_operations_change_nothing() {
    let s = src("procedure F(): Text var Row: Record Row temporary; N: Integer; R: Text;
        begin
            Row.Id := 1; Row.Insert(); Row.Id := 2; Row.Insert();
            Mark(); Row.Get(2); R := Note();
            Mark(); Row.FindSet(); repeat until Row.Next() = 0; R += '|' + Note();
            Mark(); N := Row.Count(); R += '|' + Note();
            Mark(); Row.Id := 10; Row.Insert(); R += '|' + Note();
            Mark(); Row.Name := 'TEN'; Row.Modify(); R += '|' + Note();
            Mark(); Row.Delete(); R += '|' + Note();
            exit(R);
        end;");
    assert_eq!(text(run(&s, "F")), ["statements same, rows same"; 6].join("|"));
}

/// An exact count is not pinned: comparing with a number other than 0 refuses.
#[test]
fn an_exact_count_is_refused() {
    let s = src(
        "procedure F(): Boolean begin Mark(); exit(SessionInformation.SqlStatementsExecuted() - StmtBefore = 1); end;",
    );
    refused(run(&s, "F"), "SQL counter");
    let s = src(
        "procedure F(): Boolean begin Mark(); exit(SessionInformation.SqlStatementsExecuted() - StmtBefore <= 5); end;",
    );
    refused(run(&s, "F"), "SQL counter");
}

/// Whether the count since the session began is above 0 is not pinned (only that it is not negative).
#[test]
fn a_single_read_compared_with_zero_other_than_not_negative_is_refused() {
    let s = src("procedure F(): Boolean begin exit(SessionInformation.SqlStatementsExecuted() > 0); end;");
    refused(run(&s, "F"), "SQL counter");
}

/// Format, Message text, a record field and a text concatenation would show the model's number: refused.
#[test]
fn other_uses_of_a_counter_are_refused() {
    for body in [
        "procedure F(): Text begin exit(Format(SessionInformation.SqlStatementsExecuted())); end;",
        "procedure F(): Text begin Message('%1', SessionInformation.SqlRowsRead()); end;",
        "procedure F(): Text var Row: Record Row; begin Row.Id := SessionInformation.SqlRowsRead(); end;",
        "procedure F(): Text var D: Decimal; begin D := SessionInformation.SqlRowsRead(); end;",
        "procedure F(): Text var B: BigInteger; begin Mark(); B := (SessionInformation.SqlRowsRead() - RowsBefore) * 2; end;",
        "procedure F(): Text var L: List of [BigInteger]; begin L.Add(SessionInformation.SqlRowsRead()); end;",
    ] {
        refused(run(&src(body), "F"), "SQL counter");
    }
}

/// An operation no corpus test pins between two reads (FindFirst) leaves the sign of the change unknown: refused.
#[test]
fn an_unpinned_operation_between_reads_is_refused() {
    let s = src("procedure F(): Text var Row: Record Row; begin Seed(); Mark(); Row.FindFirst(); exit(Note()); end;");
    refused(run(&s, "F"), "SQL counter");
}

/// The statement counter and the rows counter are not comparable with each other.
#[test]
fn the_two_counters_do_not_compare() {
    let s = src(
        "procedure F(): Boolean begin exit(SessionInformation.SqlRowsRead() >= SessionInformation.SqlStatementsExecuted()); end;",
    );
    refused(run(&s, "F"), "SQL counter");
}

/// A difference keeps its taint through a global, a Variant and a parameter.
#[test]
fn a_difference_through_a_variant_keeps_its_sign_only() {
    let body = |check: &str| {
        src(&format!(
            "var G: Variant;
            local procedure Check(B: BigInteger): Boolean begin exit({check}); end;
            procedure F(): Boolean begin Mark(); G := SessionInformation.SqlRowsRead() - RowsBefore; exit(Check(G)); end;"
        ))
    };
    assert!(boolean(run(&body("B = 0"), "F")));
    refused(run(&body("B = 7"), "F"), "SQL counter");
}

/// The measured sequence counts only from a Commit seen after the first read: a context from before it is not known.
#[test]
fn the_table_sequence_needs_its_commit_seen() {
    let s = src("procedure F(): Text var Row: Record Row; begin Seed(); Mark(); Row.Get(2); exit(Note()); end;");
    refused(run(&s, "F"), "SQL counter");
}

/// Two steps between two reads, or the steps in another order, are not what was measured.
#[test]
fn the_table_sequence_is_matched_step_by_step() {
    let s = src(
        "procedure F(): Text var Row: Record Row; begin Mark(); Seed(); Mark(); Row.Get(2); Row.Get(3); exit(Note()); end;",
    );
    refused(run(&s, "F"), "SQL counter");
    let s = src(
        "procedure F(): Text var Row: Record Row; N: Integer; begin Mark(); Seed(); Mark(); N := Row.Count(); exit(Note()); end;",
    );
    refused(run(&s, "F"), "SQL counter");
}
