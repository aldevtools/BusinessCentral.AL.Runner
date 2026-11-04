//! al-runner issue 6924, item 3.9: a subscriber of OnAfterValidateEvent of a field validates the same field of the same
//! Rec again, guarded by `if Rec.F = xRec.F then exit`. The runtime handed that inner Validate the outer before-image as
//! xRec, so the guard never held and the run ended in "Excessive recursion" where real BC ends. On BC 27.5 the inner
//! Validate sees the record as it is when it starts (events/table-events/ValidateInValidateSubscriber.al:212
//! AfterValidateSubscriber_XRecGuard_ValidatesAgain_NoValue `B(rec=BLUE,x=BLUE)`,
//! `guard exit`), and so does one that a codeunit called from OnValidate nests (OnValidate_CodeunitValidatesSameFieldAgain, :236).

use al_test_cli::report::Outcome;

const OBJECTS: &str = "
table 63120 Row { fields { field(1; Id; Integer) { } field(2; Loc; Code[10]) { } } keys { key(PK; Id) { } } }
codeunit 63121 Subs
{
    SingleInstance = true;
    var Log: Text;
    procedure Take(): Text begin exit(Log); end;
    [EventSubscriber(ObjectType::Table, Database::Row, 'OnAfterValidateEvent', 'Loc', false, false)]
    local procedure OnAfterValidateEvent_Loc(var Rec: Record Row; var xRec: Record Row; CurrFieldNo: Integer)
    begin
        Log += 'A(rec=' + Rec.Loc + ',x=' + xRec.Loc + ');';
        if Rec.Loc = xRec.Loc then exit;
        Rec.Validate(Loc);
    end;
}
codeunit 63123 Helper
{
    procedure Revalidate(var R: Record Row) begin R.Validate(Loc, 'INNER'); end;
}";

fn outcomes(tests: &str) -> Vec<(String, Outcome)> {
    al_testkit::outcomes_in(
        &format!("{OBJECTS}\ncodeunit 63122 T {{ Subtype = Test; {tests} }}"),
        "T",
    )
}

#[test]
fn a_validate_nested_by_a_validate_subscriber_sees_the_record_as_it_is() {
    let o = outcomes(
        "[Test] procedure Guarded() var R: Record Row; S: Codeunit Subs; begin
            R.Id := 1; R.Loc := 'RED'; R.Insert(); R.Validate(Loc, 'BLUE');
            if S.Take() <> 'A(rec=BLUE,x=RED);A(rec=BLUE,x=BLUE);' then Error('log %1', S.Take()); end;",
    );
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
}
