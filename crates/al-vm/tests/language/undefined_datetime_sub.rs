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

// types/lt-y65-undefined-arith/Y65UndefinedArith.al (writer Y65): 0DT moved by a number or a Duration raises DateInvalid; 0T moved
// either way raises "Adding to an undefined (0T) time."; 0DT - 0DT goes into the numbers as 0; CreateDateTime(0D, 0T) is 0DT,
// CreateDateTime(0D, T) raises DateInvalid; DT2Date/DT2Time of 0DT are 0D/0T.
const MORE: &str = r#"
codeunit 69791 M
{
    procedure PlusN(): DateTime begin exit(0DT + 1000); end;
    procedure PlusZero(): DateTime begin exit(0DT + 0); end;
    procedure MinusN(): DateTime begin exit(0DT - 1000); end;
    procedure PlusDuration(): DateTime var D: Duration; begin D := 0; exit(0DT + D); end;
    procedure TimeMinus(): Time begin exit(0T - 1000); end;
    procedure Create(): DateTime begin exit(CreateDateTime(0D, 120000T)); end;
    procedure Into(): Text
    var A: DateTime; I: Integer; B: BigInteger; D: Decimal;
    begin
        A := 0DT;
        I := A - A; B := A - A; D := A - A;
        exit(Format(I) + '|' + Format(B) + '|' + Format(D) + '|' + Format(CreateDateTime(0D, 0T) = 0DT) + '|' + Format(DT2Date(0DT) = 0D)
            + '|' + Format(DT2Time(0DT) = 0T) + '|' + Format(0DT < CurrentDateTime()));
    end;
}
"#;

#[test]
fn moving_an_undefined_datetime_raises_date_invalid() {
    for p in ["PlusN", "PlusZero", "MinusN", "PlusDuration", "Create"] {
        assert_eq!(
            error_message(al_testkit::run_in(MORE, "M", p)),
            "The date is not valid.",
            "{p}"
        );
    }
    assert_eq!(
        error_message(al_testkit::run_in(MORE, "M", "TimeMinus")),
        "Adding to an undefined (0T) time."
    );
}

#[test]
fn undefined_datetime_difference_into_numbers_and_conversions() {
    assert_eq!(text(al_testkit::run_in(MORE, "M", "Into")), "0|0|0|Yes|Yes|Yes|Yes");
}
