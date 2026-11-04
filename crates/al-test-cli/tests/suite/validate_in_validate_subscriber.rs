//! al-runner issue 6924, item 3.9: a subscriber of OnAfterValidateEvent of a field validates the same field of the same
//! Rec again, guarded by `if Rec.F = xRec.F then exit`. Which xRec that inner Validate sees is not pinned
//! (record/TestXRecContracts.al:133 pins only a Validate that the table's own OnValidate trigger nests). The runtime
//! handed it the outer before-image, so the guard never held and the run ended in "Excessive recursion" where real BC
//! ends. Until the probe (events/table-events/ValidateInValidateSubscriber.al, corpus branch
//! al-runtime-rs-staging-r6924b) pins it, the inner Validate is refused when the two candidate xRecs differ.

use al_test_cli::report::Outcome;

const OBJECTS: &str = "
table 63120 Row { fields { field(1; Id; Integer) { } field(2; Loc; Code[10]) { } } keys { key(PK; Id) { } } }
codeunit 63121 Subs
{
    [EventSubscriber(ObjectType::Table, Database::Row, 'OnAfterValidateEvent', 'Loc', false, false)]
    local procedure OnAfterValidateEvent_Loc(var Rec: Record Row; var xRec: Record Row; CurrFieldNo: Integer)
    begin
        if Rec.Loc = xRec.Loc then exit;
        if Rec.Id = 2 then exit;
        Rec.Validate(Loc);
    end;
}";

fn outcomes(tests: &str) -> Vec<(String, Outcome)> {
    al_testkit::outcomes_in(
        &format!("{OBJECTS}\ncodeunit 63122 T {{ Subtype = Test; {tests} }}"),
        "T",
    )
}

#[test]
fn a_validate_of_the_same_record_nested_by_a_validate_subscriber_is_refused() {
    let o = outcomes(
        "[Test] procedure Guarded() var R: Record Row; begin
            R.Id := 1; R.Loc := 'RED'; R.Insert(); R.Validate(Loc, 'BLUE'); end;",
    );
    assert!(
        matches!(&o[0].1, Outcome::NotRun(m) if m.contains("event subscriber") && m.contains("xRec")),
        "{o:?}"
    );
}

// The subscriber that validates nothing is unchanged.
#[test]
fn a_validate_subscriber_that_does_not_validate_again_runs() {
    let o = outcomes(
        "[Test] procedure Plain() var R: Record Row; begin
            R.Id := 2; R.Loc := 'RED'; R.Insert(); R.Validate(Loc, 'BLUE'); if R.Loc <> 'BLUE' then Error('x'); end;",
    );
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
}
