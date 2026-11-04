//! Round with a negative fractional precision and Date2DMY of the undefined date raise
//! (errors/PnBDateFormat.al:135-163, Date2DWY_PartsOutOfRange; :387-407, Round_Precision).

fn run(body: &str) -> String {
    let src = format!("codeunit 50100 C {{ procedure F(): Text {body} }}");
    al_testkit::text(al_testkit::run_with(&src, "C", "F", vec![]).map(|(v, _)| v))
}

#[test]
fn round_with_a_negative_fractional_precision_shows_its_size() {
    assert_eq!(
        run(
            "var D: Decimal; begin asserterror D := Round(5.55, -0.1, '='); exit(GetLastErrorText() + '|' + GetLastErrorCode()); end;"
        ),
        "The value of ROUND parameter 2 is outside of the permitted range. The current value is: 0.1. The permitted range is: from 1 to 2,147,483,647.|OutsidePermittedRange"
    );
}

#[test]
fn date2dmy_of_the_undefined_date_raises() {
    assert_eq!(
        run(
            "var I: Integer; begin asserterror I := Date2DMY(0D, 1); exit(GetLastErrorText() + '|' + GetLastErrorCode()); end;"
        ),
        "The date is not valid.|DateInvalid"
    );
}
