//! Evaluate into a DateTime without a number and with number 0 (al-runner issue 6655), pinned by corpus
//! types/lt-evaluate-format-lasterror-setview/P86Eval.al:932-1110 (`t4` to `t8`): `yyyy-mm-dd hh:mm:ss` reads as that time in
//! UTC, `yyyy-mm-dd` as its midnight, `10:20` as 20 October of the current year at midnight, and `15.01.2024 10:20` is rejected.
//! Number 9 rejects the form with a space (:1191-1240).

use al_testkit::text;

const SRC: &str = r#"
codeunit 69798 C
{
    local procedure R(Txt: Text; Fmt: Integer): Text
    var V: DateTime; Ok: Boolean;
    begin
        if Fmt < 0 then Ok := Evaluate(V, Txt) else Ok := Evaluate(V, Txt, Fmt);
        exit(Format(Ok) + '|' + Format(V, 0, 9) + ';');
    end;

    procedure Forms(): Text
    var T: Text; Fmt: Integer;
    begin
        for Fmt := -1 to 1 do
            T += R('2024-01-15 10:20:30', Fmt * 9) + R('2024-01-15', Fmt * 9) + R('15.01.2024 10:20', Fmt * 9) + R('2022-08-22 05:00:00', Fmt * 9);
        exit(T);
    end;

    procedure MonthDay(): Text
    var V: DateTime;
    begin
        Evaluate(V, '10:20');
        exit(Format(V = CreateDateTime(DMY2Date(20, 10, Date2DMY(Today(), 3)), 0T)));
    end;
}
"#;

#[test]
fn iso_with_a_space_date_only_and_the_dotted_form() {
    // -9 is no number BC knows; it reads as the default here as in `R` (Fmt < 0).
    let def = "Yes|2024-01-15T10:20:30Z;Yes|2024-01-15T00:00:00Z;No|;Yes|2022-08-22T05:00:00Z;";
    let nine = "No|;Yes|2024-01-15T00:00:00Z;No|;No|;";
    assert_eq!(text(al_testkit::run(SRC, "Forms")), format!("{def}{def}{nine}"));
}

#[test]
fn hours_and_minutes_alone_read_as_month_and_day_of_this_year() {
    assert_eq!(text(al_testkit::run(SRC, "MonthDay")), "Yes");
}
