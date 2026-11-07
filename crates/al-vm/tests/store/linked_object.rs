//! A `LinkedObject = true` table has no SQL table behind it in a test database: Insert and Count raise the SQL error BC raises
//! (record/table-properties/XTablePropsTests.al:720-737). The other uses are not pinned and are refused.

use al_builtins::native::Signal;
use al_testkit::error_message;

const SRC: &str = "table 50100 L { LinkedObject = true; fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } } \
    codeunit 50100 C { \
    procedure Ins(): Text var R: Record L; begin R.Id := 1; R.Insert(); end; \
    procedure Cnt(): Integer var R: Record L; begin exit(R.Count()); end; \
    procedure InsUsed(): Boolean var R: Record L; begin R.Id := 1; exit(R.Insert()); end; \
    procedure Fnd(): Boolean var R: Record L; begin exit(R.FindFirst()); end; }";

const TEXT: &str = "An unexpected error occurred after a database command was cancelled.";

#[test]
fn insert_and_count_raise_the_sql_error() {
    assert_eq!(error_message(al_testkit::run_in(SRC, "C", "Ins")), TEXT);
    assert_eq!(error_message(al_testkit::run_in(SRC, "C", "Cnt")), TEXT);
    match al_testkit::run_in(SRC, "C", "Ins") {
        Err(Signal::Error(e)) => assert_eq!(e.code, "Sql:Sql"),
        other => panic!("expected the SQL error, got {other:?}"),
    }
}

#[test]
fn another_use_is_refused() {
    for routine in ["Fnd", "InsUsed"] {
        match al_testkit::run_in(SRC, "C", routine) {
            Err(Signal::Unsupported(why)) => assert!(why.contains("LinkedObject"), "{why}"),
            other => panic!("expected a refusal, got {other:?}"),
        }
    }
}
