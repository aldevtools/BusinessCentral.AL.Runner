//! Reporter repros of the GlobalLanguage area (al-runner issues 8178, 8182, 8183, 8199, 8200): the measured shape runs through
//! its assumption entry (a flagged pass), and each neighbor keeps main's answer or refuses.

use al_builtins::assumptions::Assumption;
use al_testkit::{blocked, compile};
use al_testrun::isolation::TestIsolation;

/// Each test of codeunit `T` in `src`: its outcome (debug form) and the assumptions it ran through.
fn run(src: &str) -> Vec<(String, String, Vec<Assumption>)> {
    let l = compile(src);
    let b = blocked(&l);
    let engine = al_vm::Engine::new(l.program.clone(), b);
    let oi = l
        .program
        .objects
        .iter()
        .position(|o| o.name == "T")
        .expect("codeunit T");
    let mut hits = Vec::new();
    let o = al_testrun::runtime::run_codeunit_full(
        &engine,
        &l,
        oi,
        None,
        TestIsolation::Codeunit,
        None,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut hits,
        None,
    );
    o.into_iter()
        .map(|(n, r)| {
            let h = hits
                .iter()
                .find(|(t, _)| *t == n)
                .map(|(_, h): &(String, Vec<Assumption>)| h.clone())
                .unwrap_or_default();
            (n, format!("{r:?}"), h)
        })
        .collect()
}

/// One test body per procedure, in a test codeunit named T.
fn tests(bodies: &[(&str, &str)]) -> Vec<(String, String, Vec<Assumption>)> {
    let mut src = String::from("codeunit 51700 T\n{\n    Subtype = Test;\n");
    for (name, body) in bodies {
        src += &format!("    [Test]\n    procedure {name}()\n    {body}\n");
    }
    src += "}\n";
    run(&src)
}

fn get<'a>(o: &'a [(String, String, Vec<Assumption>)], name: &str) -> (&'a str, &'a [Assumption]) {
    o.iter()
        .find(|(n, ..)| n == name)
        .map(|(_, o, h)| (o.as_str(), h.as_slice()))
        .unwrap_or_else(|| panic!("no test {name}"))
}

/// The test failed with exactly the reporter's `V:[...]` text and ran through `flag` (or through no entry).
fn reports(o: &[(String, String, Vec<Assumption>)], name: &str, value: &str, flag: Option<Assumption>) {
    let (out, hits) = get(o, name);
    assert!(
        out.starts_with("Fail") && out.contains(&format!("V:[{value}]")),
        "{name}: {out}"
    );
    match flag {
        Some(a) => assert!(hits.contains(&a), "{name}: {hits:?}"),
        None => assert!(hits.is_empty(), "{name}: {hits:?}"),
    }
}

fn refused(o: &[(String, String, Vec<Assumption>)], name: &str) {
    let (out, _) = get(o, name);
    assert!(out.starts_with("NotRun"), "{name}: {out}");
}

const CLOSING: &str = "Format(ClosingDate(DMY2Date(31, 12, 2023)))";

// al-runner issue 8178 and its comment.
#[test]
fn closing_date_letter_follows_the_measured_languages() {
    let body = |l: i32| format!("begin GlobalLanguage({l}); Error('V:[%1]', {CLOSING}); end;");
    let (b30, b31, b36, b44, b53) = (body(1030), body(1031), body(1036), body(1044), body(1053));
    let o = tests(&[
        ("English", &format!("begin Error('V:[%1]', {CLOSING}); end;")),
        ("Danish", &b30),
        ("German", &b31),
        ("French", &b36),
        ("Norwegian", &b44),
        ("Swedish", &b53),
    ]);
    let flag = Some(Assumption::ClosingDateLetterByLanguage);
    reports(&o, "English", "C12/31/23", None);
    reports(&o, "Danish", "U12/31/23", flag);
    reports(&o, "German", "U12/31/23", flag);
    reports(&o, "French", "C12/31/23", flag);
    reports(&o, "Norwegian", "U12/31/23", flag);
    reports(&o, "Swedish", "S12/31/23", flag);
}

#[test]
fn closing_date_neighbors_refuse_or_keep_mains_answer() {
    let o = tests(&[
        (
            "Dutch",
            &format!("begin GlobalLanguage(1043); Error('V:[%1]', {CLOSING}); end;"),
        ),
        (
            "DanishFormat1",
            "begin GlobalLanguage(1030); Error('V:[%1]', Format(ClosingDate(DMY2Date(31, 12, 2023)), 0, 1)); end;",
        ),
        (
            "DanishCode",
            "begin GlobalLanguage(1030); Error('V:[%1]', Format(ClosingDate(DMY2Date(31, 12, 2023)), 0, '<Closing><Day>')); end;",
        ),
        (
            "EnglishFormat2",
            "begin Error('V:[%1]', Format(ClosingDate(DMY2Date(31, 12, 2023)), 0, 2)); end;",
        ),
        (
            "DanishFormat9",
            "begin GlobalLanguage(1030); Error('V:[%1]', Format(ClosingDate(DMY2Date(31, 12, 2023)), 0, 9)); end;",
        ),
    ]);
    refused(&o, "Dutch");
    refused(&o, "DanishFormat1");
    refused(&o, "DanishCode");
    reports(&o, "EnglishFormat2", "123123CD", None);
    reports(&o, "DanishFormat9", "2023-12-31", None);
}

// al-runner issue 8182.
#[test]
fn boolean_words_under_norwegian_and_swedish() {
    let eval = |l: i32, t: &str| {
        format!(
            "var B: Boolean; Ok: Boolean; begin GlobalLanguage({l}); B := false; Ok := Evaluate(B, '{t}'); Error('V:[%1|%2]', Format(Ok, 0, 9), Format(B, 0, 9)); end;"
        )
    };
    let (e1, e2, e3, e4, e5, e6) = (
        eval(1044, "Ja"),
        eval(1053, "Ja"),
        eval(1044, "Nei"),
        eval(1053, "Nej"),
        eval(1044, "Yes"),
        eval(1053, "1"),
    );
    let (e7, e8) = (eval(1044, "Nej"), eval(1053, "0"));
    let o = tests(&[
        (
            "FormatTrue1044",
            "begin GlobalLanguage(1044); Error('V:[%1]', Format(true)); end;",
        ),
        (
            "FormatTrue1053",
            "begin GlobalLanguage(1053); Error('V:[%1]', Format(true, 6)); end;",
        ),
        (
            "FormatFalse1044",
            "begin GlobalLanguage(1044); Error('V:[%1]', Format(false)); end;",
        ),
        ("EvaluateJa1044", &e1),
        ("EvaluateJa1053", &e2),
        ("EvaluateNei1044", &e3),
        ("EvaluateNej1053", &e4),
        ("EvaluateYes1044", &e5),
        ("EvaluateOne1053", &e6),
        ("EvaluateNej1044", &e7),
        ("EvaluateZero1053", &e8),
    ]);
    let flag = Some(Assumption::BooleanWordsNorwegianSwedish);
    reports(&o, "FormatTrue1044", "Ja", flag);
    reports(&o, "FormatTrue1053", "Ja    ", flag);
    refused(&o, "FormatFalse1044");
    reports(&o, "EvaluateJa1044", "true|true", flag);
    reports(&o, "EvaluateJa1053", "true|true", flag);
    reports(&o, "EvaluateNei1044", "true|false", flag);
    reports(&o, "EvaluateNej1053", "true|false", flag);
    reports(&o, "EvaluateYes1044", "false|false", flag);
    reports(&o, "EvaluateOne1053", "true|true", flag);
    refused(&o, "EvaluateNej1044");
    refused(&o, "EvaluateZero1053");
}

// al-runner issue 8199 (the reporter's first test, unchanged).
#[test]
fn closing_date_minus_a_date_raises() {
    let o = run(r#"
codeunit 58340 T
{
    Subtype = Test;

    [Test]
    procedure SubtractFromClosingDateRaises()
    var
        D: Date;
        N: Integer;
    begin
        D := ClosingDate(DMY2Date(31, 12, 2023));
        asserterror N := D - DMY2Date(31, 12, 2023);
        if GetLastErrorText() <> 'The operation cannot be performed on a closing date value.' then
            Error('BC raises on a closing date; got %1 (%2)', N, GetLastErrorText());
    end;

    [Test]
    procedure DateMinusClosing()
    var
        N: Integer;
    begin
        N := DMY2Date(31, 12, 2023) - ClosingDate(DMY2Date(31, 12, 2023));
    end;

    [Test]
    procedure PlainDates()
    begin
        Error('V:[%1]', DMY2Date(31, 12, 2023) - DMY2Date(30, 12, 2023));
    end;
}
"#);
    let (out, hits) = get(&o, "SubtractFromClosingDateRaises");
    assert_eq!(out, "Pass");
    assert_eq!(hits, [Assumption::ClosingDateMinusDateRaises]);
    refused(&o, "DateMinusClosing");
    reports(&o, "PlainDates", "1", None);
}

// al-runner issue 8183.
#[test]
fn month_and_weekday_names_measured_by_the_reporter() {
    let f =
        |l: i32, d: &str, p: &str| format!("begin GlobalLanguage({l}); Error('V:[%1]', Format({d}, 0, '{p}')); end;");
    let mar = "DMY2Date(15, 3, 2024)";
    let aug = "DMY2Date(6, 8, 2024)";
    let cells = [
        ("M30", f(1030, mar, "<Month Text>"), "marts"),
        ("M36a", f(1036, aug, "<Month Text>"), "août"),
        ("M36a3", f(1036, aug, "<Month Text,3>"), "aoû"),
        ("M53a", f(1053, aug, "<Month Text>"), "augusti"),
        ("W44", f(1044, mar, "<Weekday Text>"), "fredag"),
        ("W53t", f(1053, aug, "<Weekday Text>"), "tisdag"),
        ("W36t2", f(1036, aug, "<Weekday Text,2>"), "ma"),
        ("W31f", f(1031, mar, "<Weekday Text>"), "Freitag"),
        ("W31t2", f(1031, aug, "<Weekday Text,2>"), "Di"),
        ("D30", f(1030, aug, "<Day,2>. <Month Text> <Year4>"), "06. august 2024"),
    ];
    let mut bodies: Vec<(&str, &str)> = cells.iter().map(|(n, b, _)| (*n, b.as_str())).collect();
    let (n1, n2, n3) = (
        f(1030, "DMY2Date(15, 1, 2024)", "<Month Text>"),
        f(1044, mar, "<Month Text,5>"),
        f(1031, "DMY2Date(17, 3, 2024)", "<Weekday Text>"),
    );
    bodies.extend([
        ("OtherMonth", n1.as_str()),
        ("OtherWidth", n2.as_str()),
        ("OtherWeekday", n3.as_str()),
    ]);
    let o = tests(&bodies);
    for (n, _, v) in &cells {
        reports(&o, n, v, Some(Assumption::MonthWeekdayNamesByLanguage));
    }
    refused(&o, "OtherMonth");
    refused(&o, "OtherWidth");
    refused(&o, "OtherWeekday");
}

// al-runner issue 8200: the Evaluate and Format cells (the CalcDate message cells refuse).
#[test]
fn date_formula_letters_measured_by_the_reporter() {
    let ev = |l: i32, t: &str| {
        format!(
            "var DF: DateFormula; Ok: Boolean; begin GlobalLanguage({l}); Evaluate(DF, '<1Y>', 9); Ok := Evaluate(DF, '{t}'); Error('V:[%1|%2|%3]', Format(Ok, 0, 9), Format(DF, 0, 9), Format(DF)); end;"
        )
    };
    let fm = |l: i32, t: &str| {
        format!(
            "var DF: DateFormula; begin GlobalLanguage({l}); Evaluate(DF, '{t}', 9); Error('V:[%1]', Format(DF)); end;"
        )
    };
    let cells = [
        ("E30K", ev(1030, "1K"), "true|1Q|1K"),
        ("E30W", ev(1030, "1W"), "false|1Y|1Å"),
        ("E30NU", ev(1030, "NU"), "false|1Y|1Å"),
        ("E31T", ev(1031, "1T"), "true|1D|1T"),
        ("E31J", ev(1031, "1J"), "true|1Y|1J"),
        ("E31Q", ev(1031, "1Q"), "true|1Q|1Q"),
        ("E31D", ev(1031, "1D"), "false|1Y|1J"),
        ("E31CW", ev(1031, "CW"), "false|1Y|1J"),
        ("F30WD", fm(1030, "WD1"), "UD1"),
        ("F30CW", fm(1030, "-CW"), "-LU"),
        ("F31Y", fm(1031, "1Y+1M+1D"), "1J+1M+1T"),
        ("F31WD", fm(1031, "WD1"), "WT1"),
        ("F31D", fm(1031, "-1D"), "-1T"),
    ];
    let mut bodies: Vec<(&str, &str)> = cells.iter().map(|(n, b, _)| (*n, b.as_str())).collect();
    let (n1, n2) = (ev(1031, "LM+1T"), ev(1030, "2K+1D"));
    let n3 = "var D: Date; begin GlobalLanguage(1030); D := CalcDate('<1U>', DMY2Date(15, 3, 2024)); end;".to_string();
    bodies.extend([
        ("OtherGermanText", n1.as_str()),
        ("OtherDanishText", n2.as_str()),
        ("DanishCalcDateError", n3.as_str()),
    ]);
    let o = tests(&bodies);
    for (n, _, v) in &cells {
        reports(&o, n, v, Some(Assumption::DateFormulaLettersDanishGerman));
    }
    refused(&o, "OtherGermanText");
    refused(&o, "OtherDanishText");
    refused(&o, "DanishCalcDateError");
}
