//! 0DT - 0DT is a 0 Duration, also for the system fields of a record never inserted; one undefined side raises DateInvalid
//! (types/lt-y62-undefined-date/Y62UndefinedDate.al: DateTimeSub_BothUndefined, DateTimeSub_OneUndefined_AndDefinedPair;
//! al-runner issue 6678).

use al_testkit::{error_message, text};

const SRC: &str = r#"
table 69790 R
{
    fields { field(1; "K"; Integer) { } }
    keys { key(PK; "K") { Clustered = true; } }
}
codeunit 69790 C
{
    procedure Both(): Text
    var A: DateTime; B: DateTime; D: Duration; Row: Record R; OneHour: Duration;
    begin
        A := 0DT; B := 0DT;
        D := B - A;
        OneHour := 3600000;
        exit(Format(D) + '|' + Format(D, 0, 9) + '|' + Format(D = 0) + '|' + Format(Row.SystemModifiedAt - Row.SystemCreatedAt, 0, 9)
            + '|' + Format((A - B) < OneHour) + '|' + Format((A - B) < 0));
    end;

    procedure DefinedMinusUndefined(): Duration
    var A: DateTime;
    begin
        A := CreateDateTime(DMY2Date(1, 1, 2026), 0T);
        exit(A - 0DT);
    end;

    procedure UndefinedMinusDefined(): Duration
    var A: DateTime;
    begin
        A := CreateDateTime(DMY2Date(1, 1, 2026), 0T);
        exit(0DT - A);
    end;
}
"#;

#[test]
fn undefined_minus_undefined_datetime_is_a_zero_duration() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "Both")),
        "|P0DT0H0M0.0S|Yes|P0DT0H0M0.0S|Yes|No"
    );
}

#[test]
fn one_undefined_datetime_side_raises_date_invalid() {
    assert_eq!(
        error_message(al_testkit::run_in(SRC, "C", "DefinedMinusUndefined")),
        "The date is not valid."
    );
    assert_eq!(
        error_message(al_testkit::run_in(SRC, "C", "UndefinedMinusDefined")),
        "The date is not valid."
    );
}
