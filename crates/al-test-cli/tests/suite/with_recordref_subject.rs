//! al-runner issue 8082: a RecordRef or a FieldRef as the subject of a `with` statement. The checker accepts it
//! (al-sema `check::tests::with_over_a_recordref_or_a_fieldref`); running such a `with` is not lowered (only a
//! record's is), so a test that reaches it is not run, and the others in the app run.

use al_test_cli::report::Outcome;

const OBJECTS: &str = "
    table 63130 \"Repro Entry\" { fields { field(1; \"Entry No.\"; Integer) { } field(2; Description; Text[50]) { } }
        keys { key(PK; \"Entry No.\") { } } }
    codeunit 63131 \"Repro With Members\" {
        procedure WithRecordRef(): Integer var RecRef: RecordRef; FRef: FieldRef; begin
            with RecRef do begin
                Open(Database::\"Repro Entry\");
                FRef := Field(1);
                if FindFirst() then exit(FRef.Value);
                Close();
            end;
            exit(0);
        end;
        procedure WithFieldRef(): Text var RecRef: RecordRef; FRef: FieldRef; begin
            RecRef.Open(Database::\"Repro Entry\"); FRef := RecRef.Field(2);
            with FRef do exit(Name());
        end;
        procedure Plain(): Integer begin exit(7); end;
    }
    codeunit 63132 T { Subtype = Test;
        [Test] procedure RecordRefSubject() var C: Codeunit \"Repro With Members\"; begin C.WithRecordRef(); end;
        [Test] procedure FieldRefSubject() var C: Codeunit \"Repro With Members\"; begin C.WithFieldRef(); end;
        [Test] procedure Other() var C: Codeunit \"Repro With Members\"; begin if C.Plain() <> 7 then Error('x'); end;
    }";

#[test]
fn a_with_over_a_recordref_or_a_fieldref_compiles_and_only_its_tests_are_not_run() {
    let o = al_testkit::outcomes_in(OBJECTS, "T");
    let get = |n: &str| o.iter().find(|(t, _)| t == n).map(|(_, r)| r.clone()).unwrap();
    assert!(
        matches!(get("RecordRefSubject"), Outcome::NotRun(why) if why.contains("with on a value of type")),
        "{o:?}"
    );
    assert!(
        matches!(get("FieldRefSubject"), Outcome::NotRun(why) if why.contains("with on a value of type")),
        "{o:?}"
    );
    assert_eq!(get("Other"), Outcome::Pass);
}
