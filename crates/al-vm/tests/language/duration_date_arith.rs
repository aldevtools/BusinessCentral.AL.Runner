//! A whole number and a Duration add and subtract as milliseconds into a BigInteger, and a Date converts into a DateTime at
//! midnight (Tests-SCM-Manufacturing; the assumption date-to-datetime-is-midnight; types/duration pins the BigInteger).

use al_testkit::text;

const SRC: &str = r#"
codeunit 69751 C
{
    procedure F(): Text
    var D: Duration; I: Integer; DT: DateTime; Day: Date; Blank: DateTime;
    begin
        D := 3000; I := 2000;
        Day := DMY2Date(5, 3, 2024);
        DT := Day;
        Blank := 0D;
        exit(Format(D + I) + ';' + Format(I + D) + ';' + Format(D - I) + ';' + Format(DT, 0, 9) + ';' + Format(Blank = 0DT));
    end;
}
"#;

#[test]
fn duration_arithmetic_with_a_number_and_a_date_into_a_datetime() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "F")),
        "5000;5000;1000;2024-03-05T00:00:00Z;Yes"
    );
}
