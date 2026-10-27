//! `FieldRef.Value := 'abc'` on an Integer field raises `The value "abc" can't be evaluated into type Integer.`
//! (al-runner issue 5942, the reporter's repro on real BC; registered assumption `fieldref-text-into-integer-error`).
//! Its error code is not pinned: reading it is refused.

use al_builtins::assumptions::{Assumption, clear_hits, take_hits};
use al_builtins::native::Signal;

const TABLE: &str =
    "table 50711 Row { fields { field(1; Code; Code[20]) { } field(4; Qty; Integer) { } } keys { key(PK; Code) { } } }";

#[test]
fn letters_into_an_integer_fieldref_raise_the_evaluate_text() {
    let src = format!(
        "{TABLE} codeunit 50764 C {{ procedure F(): Text var RecRef: RecordRef; FldRef: FieldRef; begin
            RecRef.Open(Database::Row); FldRef := RecRef.Field(4); asserterror FldRef.Value := 'abc';
            exit(GetLastErrorText()); end;
          procedure G(): Text var RecRef: RecordRef; FldRef: FieldRef; begin
            RecRef.Open(Database::Row); FldRef := RecRef.Field(4); asserterror FldRef.Value := 'abc';
            exit(GetLastErrorCode()); end; }}"
    );
    clear_hits();
    assert_eq!(
        al_testkit::text(al_testkit::run_in(&src, "C", "F")),
        "The value \"abc\" can't be evaluated into type Integer."
    );
    assert!(take_hits().contains(&Assumption::FieldRefTextIntoIntegerError));
    assert!(matches!(
        al_testkit::run_in(&src, "C", "G"),
        Err(Signal::Unsupported(_))
    ));
}
