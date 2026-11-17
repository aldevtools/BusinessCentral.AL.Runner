//! Round-3 review neighbors of the GlobalLanguage entries: each entry answers only the caller and the whole text it names
//! (Record.SetFilter in AL code, no placeholder argument, a record that is not temporary, filter group 0), and each other shape
//! keeps main's answer.

use super::global_language_reported::{FILTER_OBJECTS, refused, reports, run, tests};
use al_builtins::assumptions::Assumption;

/// Codeunit T over the filter objects: rows B001 (2023-12-31, false), B002 (closing 2023-12-31, true), A001 (2024-03-15, true),
/// then each body as a test.
fn filter_tests(bodies: &[(&str, &str)]) -> Vec<(String, String, Vec<Assumption>)> {
    let mut src = format!(
        "{FILTER_OBJECTS}codeunit 58321 T\n{{\n    Subtype = Test;\n    TestPermissions = Disabled;\n\
         local procedure Seed(var R: Record \"Ref Row\") begin R.DeleteAll(); \
         R.\"Code\" := 'B001'; R.\"Doc Date\" := DMY2Date(31, 12, 2023); R.Active := false; R.Insert(); \
         R.\"Code\" := 'B002'; R.\"Doc Date\" := ClosingDate(DMY2Date(31, 12, 2023)); R.Active := true; R.Insert(); \
         R.\"Code\" := 'A001'; R.\"Doc Date\" := DMY2Date(15, 3, 2024); R.Active := true; R.Insert(); end;\n"
    );
    for (name, body) in bodies {
        src += &format!("    [Test]\n    procedure {name}()\n    {body}\n");
    }
    src += "}\n";
    run(&src)
}

/// Seeds the rows, sets GlobalLanguage `lang`, runs `stmt` (a filter on R or on RR) and reports the count or the error.
fn count(lang: i32, stmt: &str) -> String {
    format!(
        "var R: Record \"Ref Row\"; TempR: Record \"Ref Row\" temporary; RR: RecordRef; FR: FieldRef; N: Integer; begin Seed(R); \
         Seed(TempR); RR.Open(Database::\"Ref Row\"); GlobalLanguage({lang}); {stmt} GlobalLanguage(1033); Error('V:[%1]', N); end;"
    )
}

/// The same with the error text and code of a failing statement.
fn count_or_error(lang: i32, stmt: &str) -> String {
    format!(
        "var R: Record \"Ref Row\"; RR: RecordRef; FR: FieldRef; N: Integer; T: Text; begin Seed(R); RR.Open(Database::\"Ref Row\"); \
         GlobalLanguage({lang}); asserterror begin {stmt} Error('NOERR:%1', N); end; T := GetLastErrorText() + '|' + GetLastErrorCode(); \
         GlobalLanguage(1033); Error('V:[%1]', T); end;"
    )
}

// Review round 2, HIGH 1: a Boolean filter under 1030 that is not the whole text Ja, Nej or Yes keeps main's answer (main reads a
// typed placeholder as the value).
#[test]
fn danish_boolean_filter_with_a_placeholder_keeps_mains_answer() {
    let o = filter_tests(&[
        (
            "PlaceholderTrue",
            &count(1030, "R.SetFilter(Active, '%1', true); N := R.Count();"),
        ),
        (
            "PlaceholderFalse",
            &count(1030, "R.SetFilter(Active, '%1', false); N := R.Count();"),
        ),
        (
            "TwoPlaceholders",
            &count(1030, "R.SetFilter(Active, '%1|%2', true, false); N := R.Count();"),
        ),
        (
            "FieldRefPlaceholder",
            &count(1030, "FR := RR.Field(5); FR.SetFilter('%1', true); N := RR.Count();"),
        ),
        // The measured text on a temporary record: not measured, main's refusal.
        (
            "TemporaryJa",
            &count(1030, "TempR.SetFilter(Active, 'Ja'); N := TempR.Count();"),
        ),
        // The measured text in filter group 2: not measured, main's refusal.
        (
            "FilterGroupJa",
            &count(1030, "R.FilterGroup(2); R.SetFilter(Active, 'Ja'); N := R.Count();"),
        ),
    ]);
    reports(&o, "PlaceholderTrue", "2", None);
    reports(&o, "PlaceholderFalse", "1", None);
    reports(&o, "TwoPlaceholders", "3", None);
    reports(&o, "FieldRefPlaceholder", "2", None);
    refused(&o, "TemporaryJa");
    refused(&o, "FilterGroupJa");
}

// Review round 2, T1: the closing-date letter of a Date filter is read only for the whole text letter + mm/dd/yy of
// Record.SetFilter; every other caller and text keeps main's answer.
#[test]
fn closing_letter_filter_neighbors_keep_mains_answer() {
    let o = filter_tests(&[
        (
            "DanishFieldRefU",
            &count(1030, "FR := RR.Field(4); FR.SetFilter('U12/31/23'); N := RR.Count();"),
        ),
        (
            "DanishSetViewU",
            &count(
                1030,
                "R.SetView('WHERE(\"Doc Date\"=FILTER(U12/31/23))'); N := R.Count();",
            ),
        ),
        (
            "DanishAlternativeU",
            &count(
                1030,
                "R.SetFilter(\"Doc Date\", 'U12/31/23|U01/05/24'); N := R.Count();",
            ),
        ),
        (
            "DanishRangeU",
            &count(1030, "R.SetFilter(\"Doc Date\", '..U12/31/23'); N := R.Count();"),
        ),
        (
            "DanishBlankU",
            &count(1030, "R.SetFilter(\"Doc Date\", ' U12/31/23'); N := R.Count();"),
        ),
        (
            "DanishPlaceholderU",
            &count(1030, "R.SetFilter(\"Doc Date\", '%1', 'U12/31/23'); N := R.Count();"),
        ),
        (
            "DanishTemporaryU",
            &count(1030, "TempR.SetFilter(\"Doc Date\", 'U12/31/23'); N := TempR.Count();"),
        ),
        (
            "DanishFilterGroupU",
            &count(
                1030,
                "R.FilterGroup(2); R.SetFilter(\"Doc Date\", 'U12/31/23'); N := R.Count();",
            ),
        ),
        (
            "DanishBlankC",
            &count(1030, "R.SetFilter(\"Doc Date\", ' C12/31/23'); N := R.Count();"),
        ),
        (
            "DanishTrailingBlankC",
            &count(1030, "R.SetFilter(\"Doc Date\", 'C12/31/23 '); N := R.Count();"),
        ),
        (
            "GermanFieldRefC",
            &count(1031, "FR := RR.Field(4); FR.SetFilter('C12/31/23'); N := RR.Count();"),
        ),
        (
            "GermanAlternativeC",
            &count(1031, "R.SetFilter(\"Doc Date\", 'C12/31/23|12/31/23'); N := R.Count();"),
        ),
        (
            "GermanAlternativeU",
            &count_or_error(1031, "R.SetFilter(\"Doc Date\", 'U12/31/23|12/31/23'); N := R.Count();"),
        ),
    ]);
    for name in [
        "DanishFieldRefU",
        "DanishSetViewU",
        "DanishAlternativeU",
        "DanishRangeU",
        "DanishBlankU",
        "DanishPlaceholderU",
        "DanishTemporaryU",
        "DanishFilterGroupU",
    ] {
        refused(&o, name);
    }
    reports(&o, "DanishBlankC", "1", None);
    reports(&o, "DanishTrailingBlankC", "1", None);
    reports(&o, "GermanFieldRefC", "1", None);
    reports(&o, "GermanAlternativeC", "2", None);
    reports(
        &o,
        "GermanAlternativeU",
        "Der Filter 'U12/31/23|12/31/23' ist für das Feld 'Doc Date' der Tabelle 'Ref Row' nicht gültig. Das Format Date \u{201e}U12/31/23\u{201c} stimmt nicht mit den Datumseinstellungen Ihres Geräts überein.\\\\Stellen Sie sicher, dass Tag, Monat und Jahr die richtige Reihenfolge aufweisen und das richtige Trennzeichen verwendet wird (z. b. - oder /). Alternativ können Sie das Format \u{201e}jjjj-mm-tt\u{201c} verwenden.|Filter:CSide",
        None,
    );
}

/// `Evaluate(D, text[, number])` with the result used under `lang`: ok flag, the normal date, whether it is a closing date.
fn evaluate(lang: i32, args: &str) -> String {
    format!(
        "var D: Date; Ok: Boolean; begin D := DMY2Date(1, 1, 2000); GlobalLanguage({lang}); Ok := Evaluate(D, {args}); \
         GlobalLanguage(1033); Error('V:[%1|%2|%3]', Format(Ok, 0, 9), Format(NormalDate(D), 0, 9), Format(D <> NormalDate(D), 0, 9)); end;"
    )
}

// Review round 2, follow-up list: Evaluate reads the letter only before mm/dd/yy and without a format number 9.
#[test]
fn closing_letter_evaluate_neighbors_keep_mains_answer() {
    let o = tests(&[
        ("TrailingBlankC", &evaluate(1030, "'C12/31/23 '")),
        ("NumberNineU", &evaluate(1030, "'U12/31/23', 9")),
        ("NumberNineC", &evaluate(1030, "'C12/31/23', 9")),
    ]);
    reports(&o, "TrailingBlankC", "true|2023-12-31|true", None);
    reports(&o, "NumberNineU", "false|2000-01-01|false", None);
    reports(&o, "NumberNineC", "false|2000-01-01|false", None);
}

// Review round 2, T2: the statement form of the Norwegian and Swedish Evaluate cells is not measured. (The DateFormula texts with
// format number 9 are measured in the statement form, by the reporter of al-runner issue 8200.)
#[test]
fn evaluate_statement_forms_refuse_or_keep_mains_answer() {
    let boolean = |l: i32, t: &str| {
        format!(
            "var B: Boolean; begin GlobalLanguage({l}); Evaluate(B, '{t}'); GlobalLanguage(1033); Error('V:[%1]', Format(B, 0, 9)); end;"
        )
    };
    let o = tests(&[
        ("NorwegianJa", &boolean(1044, "Ja")),
        ("SwedishNej", &boolean(1053, "Nej")),
        // English words keep main's reading in the statement form.
        ("NorwegianTrue", &boolean(1044, "true")),
        // The text the measured form rejects.
        ("SwedishYes", &boolean(1053, "Yes")),
    ]);
    refused(&o, "NorwegianJa");
    refused(&o, "SwedishNej");
    reports(&o, "NorwegianTrue", "true", None);
    refused(&o, "SwedishYes");
}
