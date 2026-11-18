//! Cells of the GlobalLanguage probe (corpus PR 1428, measured on BC 27.5, 28.5 and 29.0) that main answered wrongly or
//! refused: the closing-date letter of the readers, the localized Date filter error, CalcDate's letters. Each measured cell
//! runs through its assumption entry (a flagged pass); each neighbor keeps main's answer or refuses.

use super::global_language_reported::{FILTER_OBJECTS, get, refused, reports, run, tests};
use al_builtins::assumptions::Assumption;

/// `Evaluate(D, text)` with the result used under `lang`: ok flag, the normal date, whether it is a closing date.
fn evaluate(lang: i32, text: &str) -> String {
    format!(
        "var D: Date; Ok: Boolean; begin D := DMY2Date(1, 1, 2000); GlobalLanguage({lang}); Ok := Evaluate(D, '{text}'); \
         Error('V:[%1|%2|%3]', Format(Ok, 0, 9), Format(NormalDate(D), 0, 9), Format(D <> NormalDate(D), 0, 9)); end;"
    )
}

// GlobalLanguageClosingDate.al:145-249: Evaluate takes only the letter of the language, in either case; another letter returns
// false and leaves the variable unchanged.
#[test]
fn evaluate_reads_only_the_closing_letter_of_the_language() {
    let cells = [
        ("DanishC", evaluate(1030, "C12/31/23"), "false|2000-01-01|false"),
        ("DanishLowerC", evaluate(1030, "c12/31/23"), "false|2000-01-01|false"),
        ("DanishU", evaluate(1030, "U12/31/23"), "true|2023-12-31|true"),
        ("DanishLowerU", evaluate(1030, "u12/31/23"), "true|2023-12-31|true"),
        ("GermanC", evaluate(1031, "C01/05/24"), "false|2000-01-01|false"),
        ("GermanU", evaluate(1031, "U01/05/24"), "true|2024-01-05|true"),
        ("FrenchC", evaluate(1036, "C12/31/23"), "true|2023-12-31|true"),
        ("FrenchU", evaluate(1036, "U12/31/23"), "false|2000-01-01|false"),
        ("NorwegianS", evaluate(1044, "S12/31/23"), "false|2000-01-01|false"),
        ("SwedishS", evaluate(1053, "S12/31/23"), "true|2023-12-31|true"),
        ("SwedishU", evaluate(1053, "U12/31/23"), "false|2000-01-01|false"),
    ];
    let bodies: Vec<(&str, &str)> = cells.iter().map(|(n, b, _)| (*n, b.as_str())).collect();
    let o = tests(&bodies);
    for (name, _, value) in &cells {
        reports(&o, name, value, Some(Assumption::ClosingDateLetterByLanguage));
    }
}

#[test]
fn evaluate_closing_neighbors_keep_mains_answer_or_refuse() {
    let o = tests(&[
        // English is pinned (types/date-time/P143Date_FieldRef.al:89-113): no entry.
        ("EnglishC", &evaluate(1033, "C12/31/23")),
        // A language the probe did not measure keeps main's reading.
        ("DutchC", &evaluate(1043, "C12/31/23")),
        // The statement form: its error text under 1030 is not measured.
        (
            "DanishStatement",
            "var D: Date; begin GlobalLanguage(1030); Evaluate(D, 'C12/31/23'); end;",
        ),
        // A format number: not measured.
        (
            "DanishNumber",
            "var D: Date; Ok: Boolean; begin GlobalLanguage(1030); Ok := Evaluate(D, 'U12/31/23', 1); end;",
        ),
        // Lower-case s under 1053: not measured.
        ("SwedishLowerS", &evaluate(1053, "s12/31/23")),
        // The letter of the language before a text that is no date: not measured.
        ("DanishNoDate", &evaluate(1030, "U99/99/99")),
    ]);
    reports(&o, "EnglishC", "true|2023-12-31|true", None);
    reports(&o, "DutchC", "true|2023-12-31|true", None);
    refused(&o, "DanishStatement");
    refused(&o, "DanishNumber");
    refused(&o, "SwedishLowerS");
    refused(&o, "DanishNoDate");
}

/// A test that sets `filter` on the Date field under `lang` and reports the rows it selects or the error and its code.
fn date_filter(name: &str, lang: i32, filter: &str) -> String {
    format!(
        r#"
    [Test]
    procedure {name}()
    begin
        Probe({lang}, '{filter}');
    end;
"#
    )
}

const DATE_FILTER_PROBE: &str = r#"
    [TryFunction]
    local procedure TrySetFilter(var R: Record "Ref Row"; F: Text)
    begin
        R.SetFilter("Doc Date", F);
    end;

    local procedure Probe(Lang: Integer; F: Text)
    var
        R: Record "Ref Row";
        Codes: Text;
    begin
        R.DeleteAll();
        R."Code" := 'B001';
        R."Doc Date" := DMY2Date(31, 12, 2023);
        R.Insert();
        R."Code" := 'B002';
        R."Doc Date" := ClosingDate(DMY2Date(31, 12, 2023));
        R.Insert();
        GlobalLanguage(Lang);
        if not TrySetFilter(R, F) then
            Error('V:[%1|%2]', GetLastErrorText(), GetLastErrorCode());
        if R.FindSet() then
            repeat
                Codes += R."Code" + ',';
            until R.Next() = 0;
        Error('V:[ok|%1]', Codes);
    end;
"#;

fn date_filters(cells: &[(&str, i32, &str)], extra: &str) -> Vec<(String, String, Vec<Assumption>)> {
    let mut src = format!(
        "{FILTER_OBJECTS}codeunit 58321 T\n{{\n    Subtype = Test;\n    TestPermissions = Disabled;\n{DATE_FILTER_PROBE}"
    );
    for (name, lang, filter) in cells {
        src += &date_filter(name, *lang, filter);
    }
    src += extra;
    src += "}\n";
    run(&src)
}

// FilterGlobalLanguageDateBool.al:229-370: a closing-date filter needs the letter of the language; another letter raises the
// localized date error (code Filter:CSide).
#[test]
fn date_filter_reads_only_the_closing_letter_of_the_language() {
    let o = date_filters(
        &[
            ("DanishC", 1030, "C12/31/23"),
            ("DanishU", 1030, "U12/31/23"),
            ("GermanC", 1031, "C12/31/23"),
            ("GermanU", 1031, "U12/31/23"),
            ("FrenchU", 1036, "U12/31/23"),
            ("FrenchC", 1036, "C12/31/23"),
            ("NorwegianS", 1044, "S12/31/23"),
            ("SwedishC", 1053, "C12/31/23"),
            ("SwedishS", 1053, "S12/31/23"),
        ],
        "",
    );
    let flag = Some(Assumption::ClosingDateLetterByLanguage);
    reports(
        &o,
        "DanishC",
        "Filteret \"C12/31/23\" er ikke gyldigt for feltet Doc Date i tabellen Ref Row. Formatet for Date 'C12/31/23' stemmer ikke overens med enhedens datoindstillinger.\\\\Kontroller, at dagen, måneden og året er i den rigtige rækkefølge, og brug den korrekte separator (f.eks. - eller /). Du kan også altid bruge formatet 'åååå-mm-dd'.|Filter:CSide",
        flag,
    );
    reports(&o, "DanishU", "ok|B002,", flag);
    reports(
        &o,
        "GermanC",
        "Der Filter 'C12/31/23' ist für das Feld 'Doc Date' der Tabelle 'Ref Row' nicht gültig. Das Format Date \u{201e}C12/31/23\u{201c} stimmt nicht mit den Datumseinstellungen Ihres Geräts überein.\\\\Stellen Sie sicher, dass Tag, Monat und Jahr die richtige Reihenfolge aufweisen und das richtige Trennzeichen verwendet wird (z. b. - oder /). Alternativ können Sie das Format \u{201e}jjjj-mm-tt\u{201c} verwenden.|Filter:CSide",
        flag,
    );
    reports(&o, "GermanU", "ok|B002,", flag);
    reports(
        &o,
        "FrenchU",
        "Le filtre \u{ab}\u{a0}U12/31/23\u{a0}\u{bb} n'est pas valide pour le champ Doc Date de la table Ref Row. Le format de Date \u{ab}\u{a0}U12/31/23\u{a0}\u{bb} ne correspond pas aux paramètres de date de votre appareil.\\\\Assurez-vous que le jour, le mois et l'année sont dans le bon ordre et utilisez le séparateur correct (par exemple, - ou /). Sinon, vous pouvez également utiliser le format \u{ab}\u{a0}aaaa-mm-jj\u{a0}\u{bb}.|Filter:CSide",
        flag,
    );
    reports(&o, "FrenchC", "ok|B002,", flag);
    reports(
        &o,
        "NorwegianS",
        "Filteret S12/31/23 er ikke gyldig for feltet Doc Date i tabellen Ref Row. Formatet på Date S12/31/23 samsvarer ikke med enhetens datoinnstillinger.\\\\Kontroller at dagen, måneden og året er i riktig rekkefølge, og at du bruker riktig skilletegn (for eksempel - eller /). Alternativt kan du alltids bruke formatet åååå-mm-dd.|Filter:CSide",
        flag,
    );
    reports(
        &o,
        "SwedishC",
        "Filtret C12/31/23 är inte giltigt för fältet Doc Date i tabellen Ref Row. Formatet på Date \"C12/31/23\" stämmer inte överens med enhetens datuminställningar.\\\\Kontrollera att dag, månad och år är i rätt ordning och har rätt avgränsare (som \u{2013} eller /). Alternativt kan du alltid använda formatet \"åååå\u{2013}mm\u{2013}dd\".|Filter:CSide",
        flag,
    );
    reports(&o, "SwedishS", "ok|B002,", flag);
}

#[test]
fn date_filter_closing_neighbors_keep_mains_answer_or_refuse() {
    let extra = r#"
    [Test]
    procedure DanishFieldRef()
    var
        R: RecordRef;
    begin
        R.Open(Database::"Ref Row");
        GlobalLanguage(1030);
        R.Field(4).SetFilter('C12/31/23');
    end;

    [Test]
    procedure DanishPlaceholder()
    var
        R: Record "Ref Row";
    begin
        GlobalLanguage(1030);
        R.SetFilter("Doc Date", '%1', 'C12/31/23');
    end;
"#;
    let o = date_filters(
        &[
            // English is pinned: the closing date of C, no entry.
            ("EnglishC", 1033, "C12/31/23"),
            // Two values: not the measured shape, main's reading of the English letter.
            ("DanishAlternative", 1030, "C12/31/23|12/31/23"),
            // A language the probe did not measure keeps main's reading of the English letter.
            ("DutchC", 1043, "C12/31/23"),
            // A lower-case letter in a filter, and the letter of the language before a text that is no date: not measured.
            ("DanishLowerU", 1030, "u12/31/23"),
            ("DanishNoDate", 1030, "U99/99/99"),
        ],
        extra,
    );
    reports(&o, "EnglishC", "ok|B002,", None);
    reports(&o, "DutchC", "ok|B002,", None);
    reports(&o, "DanishAlternative", "ok|B001,B002,", None);
    refused(&o, "DanishLowerU");
    refused(&o, "DanishNoDate");
    // FieldRef.SetFilter and a placeholder are not the measured shape: main's reading of the English letter, no error.
    for n in ["DanishFieldRef", "DanishPlaceholder"] {
        let (out, hits) = get(&o, n);
        assert!(out == "Pass" && hits.is_empty(), "{n}: {out} {hits:?}");
    }
}

/// CalcDate of `formula` from 2024-03-15 under `lang`: the date, or the error and its code.
fn calc(lang: i32, formula: &str) -> String {
    format!("begin Calc({lang}, '{formula}'); end;")
}

/// Codeunit T with one test per body, and the CalcDate helpers [`calc`] calls.
fn calc_tests(bodies: &[(&str, &str)]) -> Vec<(String, String, Vec<Assumption>)> {
    let mut src = String::from(
        "codeunit 51700 T\n{\n    Subtype = Test;\n\
         [TryFunction] local procedure TryCalc(F: Text; var D: Date) begin D := CalcDate(F, DMY2Date(15, 3, 2024)); end;\n\
         local procedure Calc(L: Integer; F: Text) var D: Date; begin GlobalLanguage(L); if TryCalc(F, D) then \
         Error('V:[%1]', Format(D, 0, 9)); Error('V:[%1|%2]', GetLastErrorText(), GetLastErrorCode()); end;\n",
    );
    for (name, body) in bodies {
        src += &format!("    [Test]\n    procedure {name}()\n    {body}\n");
    }
    src += "}\n";
    run(&src)
}

// GlobalLanguageDateFormula.al:540-702: a formula in angle brackets reads English letters in every language, and its error
// lists the English units in the language's text; a text without brackets reads the language's letters.
#[test]
fn calcdate_letters_by_language() {
    let cells = [
        ("German1D", calc(1031, "1D")),
        ("French1D", calc(1036, "1D")),
        ("Danish1D", calc(1030, "1D")),
        ("DanishU", calc(1030, "<1U>")),
        ("GermanT", calc(1031, "<1T>")),
        ("FrenchK", calc(1036, "<1K>")),
        ("NorwegianZ", calc(1044, "<1Z>")),
        ("SwedishJ", calc(1053, "<1J>")),
    ];
    let bodies: Vec<(&str, &str)> = cells.iter().map(|(n, b)| (*n, b.as_str())).collect();
    let o = calc_tests(&bodies);
    let flag = Some(Assumption::CalcdateLettersByLanguage);
    reports(
        &o,
        "German1D",
        "Die Datumsformel \u{201e}1D\u{201c} muss eine Zeiteinheit enthalten.\\\\Mögliche Zeiteinheiten: T,WT,W,M,Q oder J.\\L, die laufende Zeiteinheit basierend auf dem Datum, kann als Präfix der Zeiteinheit angegeben werden.|Evaluate",
        flag,
    );
    reports(
        &o,
        "French1D",
        "La formule de date \u{ab}\u{a0}1D\u{a0}\u{bb} devrait inclure une unité de temps.\\\\Les unités de temps peuvent être\u{a0}: J,JS,S,M,T ou A.\\F, signifiant unité de temps actuelle basée sur la date, peut être spécifié comme préfixe de l\u{2019}unité de temps.|Evaluate",
        flag,
    );
    reports(&o, "Danish1D", "2024-03-16", flag);
    reports(
        &o,
        "DanishU",
        "Der skal indgå en tidsenhed i datoformlen '1U'.\\\\Tidsenheder kan være: D,WD,W,M,Q eller Y.\\C betyder den aktuelle tidsenhed baseret på dato, kan specificeres som et præfiks for tidsenheden.|Evaluate",
        flag,
    );
    reports(
        &o,
        "GermanT",
        "Die Datumsformel \u{201e}1T\u{201c} muss eine Zeiteinheit enthalten.\\\\Mögliche Zeiteinheiten: D,WD,W,M,Q oder Y.\\C, die laufende Zeiteinheit basierend auf dem Datum, kann als Präfix der Zeiteinheit angegeben werden.|Evaluate",
        flag,
    );
    reports(
        &o,
        "FrenchK",
        "La formule de date \u{ab}\u{a0}1K\u{a0}\u{bb} devrait inclure une unité de temps.\\\\Les unités de temps peuvent être\u{a0}: D,WD,W,M,Q ou Y.\\C, signifiant unité de temps actuelle basée sur la date, peut être spécifié comme préfixe de l\u{2019}unité de temps.|Evaluate",
        flag,
    );
    reports(
        &o,
        "NorwegianZ",
        "Datoformelen 1Z må inneholde en tidsenhet.\\\\Tidsenheter kan være: D,WD,W,M,Q eller Y.\\C, som betyr nåværende tidsenhet basert på dato, kan angis som et prefiks til tidsenheten.|Evaluate",
        flag,
    );
    reports(
        &o,
        "SwedishJ",
        "Datumformeln '1J' ska inkludera en tidsenhet.\\\\Tidsenheter kan vara: D,WD,W,M,Q eller Y.\\C, avser aktuell tidsenhet baserad på datum och kan anges som ett prefix till tidsenheten.|Evaluate",
        flag,
    );
}

#[test]
fn calcdate_neighbors_keep_mains_answer_or_refuse() {
    let o = calc_tests(&[
        // English letters in brackets: main's reading, now measured in every language; no entry.
        ("GermanBracketD", &calc(1031, "<1D>")),
        // A text without brackets other than 1D: not measured.
        ("GermanW", &calc(1031, "1W")),
        ("DanishM", &calc(1030, "1M")),
        // Two letters in brackets: the error is not measured.
        ("DanishUU", &calc(1030, "<1U+1K>")),
        // A language the probe did not measure keeps main's refusal of the error.
        ("DutchU", &calc(1043, "<1U>")),
    ]);
    reports(&o, "GermanBracketD", "2024-03-16", None);
    refused(&o, "GermanW");
    refused(&o, "DanishM");
    refused(&o, "DanishUU");
    refused(&o, "DutchU");
}

// GlobalLanguageBoolean.al:187: under 1036 Evaluate reads No as false and rejects Yes (pinned before this probe by
// format-evaluate/EvalLanguageBoolean.al, so no entry).
#[test]
fn french_boolean_reads_no_and_rejects_yes() {
    let body = |t: &str| {
        format!(
            "var B: Boolean; Ok: Boolean; begin B := true; GlobalLanguage(1036); Ok := Evaluate(B, '{t}'); \
             Error('V:[%1|%2]', Format(Ok, 0, 9), Format(B, 0, 9)); end;"
        )
    };
    let (no, yes) = (body("No"), body("Yes"));
    let o = tests(&[("No", &no), ("Yes", &yes)]);
    reports(&o, "No", "true|false", None);
    reports(&o, "Yes", "false|true", None);
}
