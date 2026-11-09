//! Evaluate into a Date of a date text with a trailing D (al-runner issue 7363), of filter texts with an operator (issue 7364),
//! and into a DateTime of a date-only text (issue 7384): corpus master format-evaluate/Y71EvaluateDashedDates.al (corpus PR 1247),
//! Evaluate_Date_TrailingD_En, _De, Evaluate_Date_FilterOperatorTexts, _De, Evaluate_DateTime_DateOnlyText_En, _De and
//! Evaluate_DateTime_DateAndTimeText_En.

use al_builtins::value::Value;
use al_testkit::{Built, text};

const SRC: &str = r#"
codeunit 50990 Y
{
    procedure Ev(Value: Text; Lang: Integer): Text
    var
        D: Date;
        Ok: Boolean;
        R: Text;
    begin
        GlobalLanguage(Lang);
        Ok := Evaluate(D, Value);
        R := StrSubstNo('%1|%2', Ok, Format(D, 0, 9));
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure EvDT(Value: Text; Lang: Integer): Text
    var
        DT: DateTime;
        Ok: Boolean;
        R: Text;
    begin
        GlobalLanguage(Lang);
        DT := CreateDateTime(DMY2Date(5, 5, 2005), 123456T);
        Ok := Evaluate(DT, Value);
        R := StrSubstNo('%1|%2', Ok, Format(DT, 0, 9));
        GlobalLanguage(1033);
        exit(R);
    end;
}
"#;

fn check(routine: &str, cases: &[(&str, i32, &str)]) {
    let mut b = Built::from_source(SRC);
    let (oi, ri) = b.routine_in("Y", routine);
    let mut wrong = Vec::new();
    for (t, lang, want) in cases {
        let got = text(
            b.call(oi, ri, vec![Value::Text(t.to_string().into()), Value::Integer(*lang)])
                .map(|(v, _)| v),
        );
        if got != *want {
            wrong.push(format!("{t:?} ({lang}): expected {want:?}, got {got:?}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_date_with_a_trailing_d_reads_as_the_date() {
    check(
        "Ev",
        &[
            ("12/09/26D", 1033, "Yes|2026-12-09"),
            ("12/09/2026D", 1033, "Yes|2026-12-09"),
            ("12/09/26d", 1033, "Yes|2026-12-09"),
            ("12/09/26 D", 1033, "Yes|2026-12-09"),
            ("01/01/26D", 1033, "Yes|2026-01-01"),
            ("12-09-26D", 1033, "Yes|2026-12-09"),
            ("12.09.26D", 1033, "Yes|2026-12-09"),
            ("120926D", 1033, "Yes|2026-12-09"),
            ("12092026D", 1033, "Yes|2026-12-09"),
            ("13/09/26D", 1033, "No|"),
            ("0D", 1033, "No|"),
            ("12/09/26DD", 1033, "Yes|2026-12-09"),
            ("12/09/26D ", 1033, "Yes|2026-12-09"),
            ("12/09/26", 1033, "Yes|2026-12-09"),
            ("12.09.26D", 1031, "Ja|2026-12-09"),
            ("12.09.2026D", 1031, "Ja|2026-12-09"),
            ("12/09/26D", 1031, "Ja|2026-12-09"),
            ("12.09.26", 1031, "Ja|2026-12-09"),
            ("0D", 1031, "Nein|"),
        ],
    );
}

#[test]
fn a_filter_text_with_an_operator_is_no_date() {
    check(
        "Ev",
        &[
            ("<=11/09/26", 1033, "No|"),
            (">=11/09/26", 1033, "No|"),
            ("<11/09/26", 1033, "No|"),
            (">11/09/26", 1033, "No|"),
            ("<>11/09/26", 1033, "No|"),
            ("=11/09/26", 1033, "No|"),
            ("<= 11/09/26", 1033, "No|"),
            ("<=11/09/2026", 1033, "No|"),
            ("11/09/26..12/09/26", 1033, "Yes|2026-11-09"),
            ("..11/09/26", 1033, "No|"),
            ("11/09/26..", 1033, "Yes|2026-11-09"),
            ("<=", 1033, "No|"),
            ("<=11-09-26", 1033, "No|"),
            ("<=11.09.26", 1031, "Nein|"),
            ("<=11.09.2026", 1031, "Nein|"),
            (">=11.09.26", 1031, "Nein|"),
            ("<=11/09/26", 1031, "Nein|"),
        ],
    );
}

#[test]
fn a_date_only_text_reads_into_a_datetime_at_midnight() {
    check(
        "EvDT",
        &[
            ("01/01/26", 1033, "Yes|2026-01-01T00:00:00Z"),
            ("01/01/23", 1033, "Yes|2023-01-01T00:00:00Z"),
            ("11/01/26", 1033, "Yes|2026-11-01T00:00:00Z"),
            ("01/01/2026", 1033, "Yes|2026-01-01T00:00:00Z"),
            ("13/01/26", 1033, "No|2005-05-05T12:34:56Z"),
            ("01-01-26", 1033, "Yes|2026-01-01T00:00:00Z"),
            ("01.01.26", 1033, "Yes|2026-01-01T00:00:00Z"),
            ("010126", 1033, "Yes|2026-01-01T00:00:00Z"),
            ("2026-01-01", 1033, "Yes|2026-01-01T00:00:00Z"),
            ("2026-01-01T00:00:00", 1033, "No|2005-05-05T12:34:56Z"),
            ("01/01/26 12:00:00", 1033, "Yes|2026-01-01T12:00:00Z"),
            ("01/01/26 12:00", 1033, "Yes|2026-01-01T12:00:00Z"),
            ("01/01/26 ", 1033, "Yes|2026-01-01T00:00:00Z"),
            ("", 1033, "Yes|"),
            ("01.01.26", 1031, "Ja|2026-01-01T00:00:00Z"),
            ("01.01.2026", 1031, "Ja|2026-01-01T00:00:00Z"),
            ("01/01/26", 1031, "Ja|2026-01-01T00:00:00Z"),
            ("01.01.26 12:00:00", 1031, "Ja|2026-01-01T12:00:00Z"),
        ],
    );
}
