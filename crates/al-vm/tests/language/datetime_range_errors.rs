//! DateTime +/- a number outside the DateTime range raises: past 9999-12-31 23:59:59.999 and before 0001-01-01 raise
//! 'The date is not valid.' (DateInvalid), an amount that overflows 64 bits raises the overflow error (types/lt-datetime/
//! Arithmetic.al:DateTime_Arithmetic_Errors, :294-313).

use al_testkit::text;

const SRC: &str = r#"
codeunit 69712 C
{
    procedure PastMax(): Text
    var DT: DateTime;
    begin
        asserterror DT := CreateDateTime(99991231D, 235959.999T) + 1;
        exit(GetLastErrorText() + '|' + GetLastErrorCode());
    end;
    procedure BeforeMin(): Text
    var DT: DateTime;
    begin
        asserterror DT := CreateDateTime(00010101D, 000000T) - 1;
        exit(GetLastErrorText() + '|' + GetLastErrorCode());
    end;
    procedure BigMax(): Text
    var DT: DateTime;
    begin
        asserterror DT := CreateDateTime(20240101D, 0T) + 9223372036854775807L;
        exit(GetLastErrorText() + '|' + GetLastErrorCode());
    end;
    procedure BigMin(): Text
    var DT: DateTime;
    begin
        asserterror DT := CreateDateTime(20240101D, 0T) - 9223372036854775807L;
        exit(GetLastErrorText() + '|' + GetLastErrorCode());
    end;
}
"#;

#[test]
fn past_the_maximum_raises_date_invalid() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "PastMax")), "The date is not valid.|DateInvalid");
}

#[test]
fn before_the_minimum_raises_date_invalid() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "BeforeMin")), "The date is not valid.|DateInvalid");
}

#[test]
fn a_huge_amount_overflows() {
    let want = "Arithmetic operation resulted in an overflow.|AL:Overflow";
    assert_eq!(text(al_testkit::run_in(SRC, "C", "BigMax")), want);
    assert_eq!(text(al_testkit::run_in(SRC, "C", "BigMin")), want);
}
