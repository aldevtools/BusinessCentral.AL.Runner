//! A SQL counter value given to a record filter or kept in a collection (al-runner issue 7859). The corpus pins only that a
//! counter is not negative and the sign of its change (session-system/session/SqlCounters.al): a filter matcher or a
//! collection that compares the value with its own code would read the model's number, so every such use refuses. A List
//! value holds it unread (SqlCounters.al P6924_Counters_DictionaryOfGuidInteger_AddGetAndLaterDifference) and Get gives it
//! back.

use al_builtins::native::Signal;
use al_testkit::{boolean, run};

const TABLE: &str =
    "table 50110 Row { fields { field(1; Id; BigInteger) { } } keys { key(PK; Id) { Clustered = true; } } }";

fn src(body: &str) -> String {
    format!("{TABLE}\ncodeunit 50111 C {{\n{body}\n}}")
}

fn refused(body: &str) {
    match run(&src(body), "F") {
        Err(Signal::Unsupported(why)) => assert!(why.contains("SQL counter value"), "{body}: {why}"),
        other => panic!("{body}: expected a refusal, got {other:?}"),
    }
}

#[test]
fn a_counter_value_as_a_record_filter_value_refuses() {
    refused(
        "procedure F() var Row: Record Row; N: BigInteger; begin N := SessionInformation.SqlStatementsExecuted(); Row.SetRange(Id, N); end;",
    );
    refused(
        "procedure F() var Row: Record Row; N: BigInteger; begin N := SessionInformation.SqlStatementsExecuted(); Row.SetRange(Id, 0, N); end;",
    );
    refused(
        "procedure F() var Row: Record Row; N: BigInteger; begin N := SessionInformation.SqlStatementsExecuted(); Row.SetFilter(Id, '>%1', N); end;",
    );
    refused(
        "procedure F() var R: RecordRef; N: BigInteger; begin N := SessionInformation.SqlStatementsExecuted(); R.Open(50110); R.Field(1).SetRange(N); end;",
    );
    refused(
        "procedure F() var R: RecordRef; N: BigInteger; begin N := SessionInformation.SqlStatementsExecuted(); R.Open(50110); R.Field(1).SetFilter('<%1', N); end;",
    );
    refused(
        "procedure F() var Row: Record Row; V: Variant; begin V := SessionInformation.SqlStatementsExecuted(); Row.SetRange(Id, V); end;",
    );
}

#[test]
fn a_counter_value_as_a_collection_key_or_search_value_refuses() {
    refused(
        "procedure F() var L: List of [BigInteger]; N: BigInteger; begin N := SessionInformation.SqlStatementsExecuted(); L.Add(1); if L.Contains(N) then; end;",
    );
    refused(
        "procedure F() var L: List of [BigInteger]; N: BigInteger; begin N := SessionInformation.SqlStatementsExecuted(); L.Add(1); if L.IndexOf(N) = 0 then; end;",
    );
    refused(
        "procedure F() var D: Dictionary of [BigInteger, Integer]; begin D.Add(SessionInformation.SqlStatementsExecuted(), 1); end;",
    );
    refused(
        "procedure F() var D: Dictionary of [BigInteger, Integer]; N: BigInteger; begin N := SessionInformation.SqlStatementsExecuted(); if D.ContainsKey(N) then; end;",
    );
}

#[test]
fn a_collection_that_holds_a_counter_value_refuses_a_search() {
    refused(
        "procedure F() var L: List of [BigInteger]; begin L.Add(SessionInformation.SqlStatementsExecuted()); if L.Contains(5) then; end;",
    );
    refused(
        "procedure F() var L: List of [BigInteger]; begin L.Add(SessionInformation.SqlStatementsExecuted()); if L.IndexOf(5) = 0 then; end;",
    );
    refused(
        "procedure F() var L: List of [BigInteger]; begin L.Add(SessionInformation.SqlStatementsExecuted()); if L.LastIndexOf(5) = 0 then; end;",
    );
    refused(
        "procedure F() var L: List of [BigInteger]; begin L.Add(SessionInformation.SqlStatementsExecuted()); if L.Remove(5) then; end;",
    );
    refused(
        "procedure F() var D: Dictionary of [Integer, BigInteger]; begin D.Add(1, SessionInformation.SqlStatementsExecuted()); if D.Values.Contains(5) then; end;",
    );
}

#[test]
fn a_list_value_holds_a_counter_value_unread() {
    let body = "procedure F(): Boolean var L: List of [BigInteger]; begin L.Add(SessionInformation.SqlStatementsExecuted()); exit((L.Count = 1) and (L.Get(1) >= 0)); end;";
    assert!(boolean(run(&src(body), "F")));
}
