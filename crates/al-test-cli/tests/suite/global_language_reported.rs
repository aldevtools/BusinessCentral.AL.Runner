//! Reporter repros of the GlobalLanguage area (al-runner issues 8178, 8182, 8183, 8199, 8200): the measured shape runs through
//! its assumption entry (a flagged pass), and each neighbor keeps main's answer or refuses.

use al_builtins::assumptions::Assumption;
use al_testkit::{blocked, compile};
use al_testrun::isolation::TestIsolation;

/// Each test of codeunit `T` in `src`: its outcome (debug form) and the assumptions it ran through. Each `[Test]` procedure
/// runs in a codeunit of its own (the procedures before the first test are kept for each): a test refused while it runs makes
/// the next test of its codeunit NotRun, so a refusal of a later test would be asserted for another reason.
pub(super) fn run(src: &str) -> Vec<(String, String, Vec<Assumption>)> {
    let starts: Vec<usize> = src
        .match_indices("[Test]")
        .map(|(i, _)| src[..i].rfind('\n').map_or(0, |n| n + 1))
        .collect();
    let Some(&first) = starts.first() else {
        return run_codeunit(src);
    };
    let end = src.trim_end().rfind('}').expect("the end of codeunit T");
    let mut out = Vec::new();
    for (k, &start) in starts.iter().enumerate() {
        let stop = starts.get(k + 1).copied().unwrap_or(end);
        out.extend(run_codeunit(&format!(
            "{}{}{}",
            &src[..first],
            &src[start..stop],
            &src[end..]
        )));
    }
    out
}

fn run_codeunit(src: &str) -> Vec<(String, String, Vec<Assumption>)> {
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
        al_testrun::host::RunClock::begin(),
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
pub(super) fn tests(bodies: &[(&str, &str)]) -> Vec<(String, String, Vec<Assumption>)> {
    let mut src = String::from("codeunit 51700 T\n{\n    Subtype = Test;\n");
    for (name, body) in bodies {
        src += &format!("    [Test]\n    procedure {name}()\n    {body}\n");
    }
    src += "}\n";
    run(&src)
}

pub(super) fn get<'a>(o: &'a [(String, String, Vec<Assumption>)], name: &str) -> (&'a str, &'a [Assumption]) {
    o.iter()
        .find(|(n, ..)| n == name)
        .map(|(_, o, h)| (o.as_str(), h.as_slice()))
        .unwrap_or_else(|| panic!("no test {name}"))
}

/// The test failed with exactly the reporter's `V:[...]` text and ran through `flag` (or through no entry).
pub(super) fn reports(o: &[(String, String, Vec<Assumption>)], name: &str, value: &str, flag: Option<Assumption>) {
    let (out, hits) = get(o, name);
    assert!(
        out.starts_with("Fail") && out.contains(format!("{:?}", format!("V:[{value}]")).trim_matches('"')),
        "{name}: {out}"
    );
    match flag {
        Some(a) => assert!(hits.contains(&a), "{name}: {hits:?}"),
        None => assert!(hits.is_empty(), "{name}: {hits:?}"),
    }
}

/// The test was refused by the runtime itself, not because an earlier test of its codeunit was.
pub(super) fn refused(o: &[(String, String, Vec<Assumption>)], name: &str) {
    let (out, _) = get(o, name);
    assert!(
        out.starts_with("NotRun") && !out.contains("earlier test of this codeunit"),
        "{name}: {out}"
    );
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
    let flag = None;
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
    let flag = None;
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
    assert!(hits.is_empty(), "{hits:?}");
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
        reports(&o, n, v, None);
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
    // The reporter measured `NU` under 1030 too, but the corpus pin does not hold it
    // (localization/global-language/GlobalLanguageDateFormula.al:95-96 lists the texts): it refuses.
    let n4 = ev(1030, "NU");
    // A single local letter in brackets is measured since (global_language_probe.rs); two terms are not.
    let n3 =
        "var D: Date; begin GlobalLanguage(1030); D := CalcDate('<1U+1K>', DMY2Date(15, 3, 2024)); end;".to_string();
    bodies.extend([
        ("OtherGermanText", n1.as_str()),
        ("OtherDanishText", n2.as_str()),
        ("DanishCalcDateTwoTerms", n3.as_str()),
        ("DanishNu", n4.as_str()),
    ]);
    let o = tests(&bodies);
    for (n, _, v) in &cells {
        reports(&o, n, v, None);
    }
    refused(&o, "OtherGermanText");
    refused(&o, "OtherDanishText");
    refused(&o, "DanishCalcDateTwoTerms");
    refused(&o, "DanishNu");
}

// al-runner issue 8197: the reporter's repro, its objects unchanged; the Danish date filter text and the Enum shape stay refused.
pub(super) const FILTER_OBJECTS: &str = r#"
enum 58320 "Ref Color" { value(0; Red) { } value(1; Green) { } value(2; Blue) { } }

table 58320 "Ref Row"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; Kind; Option) { OptionMembers = Alpha,Beta,Gamma; }
        field(3; Color; Enum "Ref Color") { }
        field(4; "Doc Date"; Date) { }
        field(5; Active; Boolean) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}
"#;

#[test]
fn set_filter_shapes_measured_by_the_reporter() {
    let o = run(&format!(
        "{FILTER_OBJECTS}{}",
        r#"
codeunit 58321 T
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure OptionNotAMember()
    var
        R: Record "Ref Row";
    begin
        asserterror R.SetFilter(Kind, 'Delta');
        if StrPos(GetLastErrorText(), 'The existing options are: Alpha,Beta,Gamma') = 0 then
            Error('BC names the options: %1', GetLastErrorText());
    end;

    [Test]
    procedure DanishBooleanWords()
    var
        R: Record "Ref Row";
    begin
        GlobalLanguage(1030);
        R.DeleteAll();
        R."Code" := 'A';
        R.Active := true;
        R.Insert();
        R.SetFilter(Active, 'Ja');
        if R.IsEmpty() then
            Error('BC reads Ja as true under 1030');
        asserterror R.SetFilter(Active, 'Yes');
    end;

    [Test]
    procedure DanishDateTextUnderEnUsRegion()
    var
        R: Record "Ref Row";
    begin
        GlobalLanguage(1030);
        asserterror R.SetFilter("Doc Date", '15.03.2024');
    end;

    [Test]
    procedure OptionFullText()
    var
        R: Record "Ref Row";
    begin
        GlobalLanguage(1033);
        asserterror R.SetFilter(Kind, 'Delta');
        Error('V:[%1]', GetLastErrorText());
    end;

    [Test]
    procedure DanishYesText()
    var
        R: Record "Ref Row";
    begin
        GlobalLanguage(1030);
        asserterror R.SetFilter(Active, 'Yes');
        Error('V:[%1]', GetLastErrorText());
    end;

    [Test]
    procedure DanishTrueWord()
    var
        R: Record "Ref Row";
    begin
        GlobalLanguage(1030);
        R.SetFilter(Active, 'true');
    end;

    [Test]
    procedure OptionWordWithDigit()
    var
        R: Record "Ref Row";
    begin
        GlobalLanguage(1033);
        asserterror R.SetFilter(Kind, 'Delta2');
    end;

    [Test]
    procedure EnumNotAMember()
    var
        R: Record "Ref Row";
    begin
        GlobalLanguage(1033);
        asserterror R.SetFilter(Color, 'Purple');
    end;
}
"#
    ));
    for t in ["OptionNotAMember", "DanishBooleanWords"] {
        let (out, hits) = get(&o, t);
        assert_eq!(out, "Pass", "{t}");
        assert!(hits.is_empty(), "{t}: {hits:?}");
    }
    reports(
        &o,
        "OptionFullText",
        "The filter \"Delta\" is not valid for the Kind field on the Ref Row table. 'Delta' is not an option. The existing options are: Alpha,Beta,Gamma",
        None,
    );
    reports(
        &o,
        "DanishYesText",
        "Filteret \"Yes\" er ikke gyldigt for feltet Active i tabellen Ref Row. V\u{e6}rdien \"Yes\" kan ikke evalueres til typen Boolean.",
        None,
    );
    refused(&o, "DanishDateTextUnderEnUsRegion");
    refused(&o, "DanishTrueWord");
    refused(&o, "OptionWordWithDigit");
    refused(&o, "EnumNotAMember");
}

// Round-2 review neighbors of al-runner issue 8197 shape 1: only Record.SetFilter with the whole filter text equal to one word,
// no placeholder, an Option without captions or blank members, under 1033 runs; each other shape keeps main's refusal.
const OPTION_NEIGHBOR_OBJECTS: &str = r#"
table 58322 "Ref Capt"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; Capt; Option) { OptionMembers = Alpha,Beta,Gamma; OptionCaption = 'First,Second,Third'; }
        field(3; Blk; Option) { OptionMembers = " ",A,B; }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}
"#;

#[test]
fn option_non_member_neighbors_keep_mains_refusal() {
    let rec = |stmt: &str| format!("var R: Record \"Ref Row\"; C: Record \"Ref Capt\"; begin {stmt} end;");
    let bodies = [
        ("Alternatives", rec("asserterror R.SetFilter(Kind, 'Alpha|Delta');")),
        ("NotEqual", rec("asserterror R.SetFilter(Kind, '<>Delta');")),
        ("LeadingBlank", rec("asserterror R.SetFilter(Kind, ' Delta');")),
        ("TrailingBlank", rec("asserterror R.SetFilter(Kind, 'Delta ');")),
        ("Placeholder", rec("asserterror R.SetFilter(Kind, '%1', 'Delta');")),
        ("Caption", rec("asserterror C.SetFilter(Capt, 'First');")),
        ("BlankMember", rec("asserterror C.SetFilter(Blk, 'Delta');")),
        ("Danish", rec("GlobalLanguage(1030); asserterror R.SetFilter(Kind, 'Delta');")),
        (
            "FieldRef",
            "var RR: RecordRef; FR: FieldRef; begin RR.Open(58320); FR := RR.Field(2); asserterror FR.SetFilter('Delta'); end;"
                .to_string(),
        ),
    ];
    let mut src = format!("{FILTER_OBJECTS}{OPTION_NEIGHBOR_OBJECTS}codeunit 51700 T\n{{\n    Subtype = Test;\n");
    for (name, body) in &bodies {
        src += &format!("    [Test]\n    procedure {name}()\n    {body}\n");
    }
    src += "}\n";
    let o = run(&src);
    for (name, _) in &bodies {
        refused(&o, name);
    }
}

// Round-2 review neighbors of al-runner issue 8197 shape 3: only Record.SetFilter with the whole text Ja, Nej or Yes as written.
#[test]
fn danish_boolean_filter_neighbors_refuse() {
    let rec = |stmt: &str| format!("var R: Record \"Ref Row\"; begin GlobalLanguage(1030); {stmt} end;");
    let bodies = [
        ("Alternatives", rec("R.SetFilter(Active, 'Ja|Nej');")),
        ("NotEqual", rec("R.SetFilter(Active, '<>Ja');")),
        ("LeadingBlank", rec("R.SetFilter(Active, ' Ja');")),
        ("UpperCase", rec("R.SetFilter(Active, 'JA');")),
        ("LowerCase", rec("R.SetFilter(Active, 'nej');")),
        ("WithYes", rec("asserterror R.SetFilter(Active, 'Ja|Yes');")),
        ("NotYes", rec("asserterror R.SetFilter(Active, '<>Yes');")),
        ("Placeholder", rec("R.SetFilter(Active, '%1', 'Ja');")),
        ("SetView", rec("R.SetView('WHERE(Active=FILTER(Ja))');")),
        (
            "FieldRef",
            "var RR: RecordRef; FR: FieldRef; begin GlobalLanguage(1030); RR.Open(58320); FR := RR.Field(5); FR.SetFilter('Ja'); end;"
                .to_string(),
        ),
    ];
    let mut src = format!("{FILTER_OBJECTS}codeunit 51700 T\n{{\n    Subtype = Test;\n");
    for (name, body) in &bodies {
        src += &format!("    [Test]\n    procedure {name}()\n    {body}\n");
    }
    src += "}\n";
    let o = run(&src);
    for (name, _) in &bodies {
        refused(&o, name);
    }
}

// Round-2 review neighbors of al-runner issue 8200: only the measured texts, numbers and forms.
#[test]
fn date_formula_neighbors_refuse_or_keep_mains_answer() {
    let used = |l: i32, t: &str, n: &str| {
        format!(
            "var DF: DateFormula; Ok: Boolean; begin GlobalLanguage({l}); Ok := Evaluate(DF, '{t}'{n}); Error('V:[%1|%2]', Format(Ok, 0, 9), Format(DF, 0, 9)); end;"
        )
    };
    let fm = |l: i32, t: &str| {
        format!(
            "var DF: DateFormula; begin Evaluate(DF, '{t}', 9); GlobalLanguage({l}); Error('V:[%1]', Format(DF)); end;"
        )
    };
    let (k9, t9, w9, k12, t0) = (
        used(1030, "1K", ", 9"),
        used(1031, "1T", ", 9"),
        "var DF: DateFormula; begin GlobalLanguage(1030); asserterror Evaluate(DF, 'W', 9); end;".to_string(),
        used(1030, "12K", ""),
        used(1031, "0T", ""),
    );
    let (cq, cq31, m31, d9, k_stmt) = (
        fm(1030, "CQ+1D"),
        fm(1031, "CQ"),
        fm(1031, "1M"),
        used(1030, "1D", ", 9"),
        "var DF: DateFormula; begin GlobalLanguage(1030); Evaluate(DF, '1K'); end;".to_string(),
    );
    let o = tests(&[
        ("DanishQuarterNumberNine", &k9),
        ("GermanDayNumberNine", &t9),
        ("DanishNoNumberNine", &w9),
        ("DanishTwelveQuarters", &k12),
        ("GermanZeroDays", &t0),
        ("DanishFormatCurrentQuarter", &cq),
        ("GermanFormatCurrentQuarter", &cq31),
        ("GermanFormatMonth", &m31),
        ("DanishDayNumberNine", &d9),
        ("DanishQuarterStatement", &k_stmt),
    ]);
    for n in [
        "DanishQuarterNumberNine",
        "GermanDayNumberNine",
        "DanishNoNumberNine",
        "DanishTwelveQuarters",
        "GermanZeroDays",
        "DanishFormatCurrentQuarter",
        "GermanFormatCurrentQuarter",
        "DanishQuarterStatement",
    ] {
        refused(&o, n);
    }
    reports(&o, "GermanFormatMonth", "1M", None);
    reports(&o, "DanishDayNumberNine", "true|1D", None);
}

// Round-2 review: a closing date only minus a plain date of the same day, under 1033; the filter texts of a closing date under
// another language; the <Closing> code under French.
#[test]
fn closing_date_neighbors_of_round_two_refuse() {
    let minus = |l: i32, e: &str| format!("var N: Integer; begin GlobalLanguage({l}); asserterror N := {e}; end;");
    let cd = "ClosingDate(DMY2Date(31, 12, 2023))";
    let (other_day, danish, german) = (
        minus(1033, &format!("{cd} - DMY2Date(30, 12, 2023)")),
        minus(1030, &format!("{cd} - DMY2Date(31, 12, 2023)")),
        minus(1031, &format!("{cd} - DMY2Date(31, 12, 2023)")),
    );
    let (two, zero, zero_first) = (
        minus(1033, &format!("{cd} - {cd}")),
        minus(1033, &format!("{cd} - 0D")),
        minus(1033, &format!("0D - {cd}")),
    );
    let filter = |read: &str| {
        format!(
            "var R: Record \"Ref Row\"; begin GlobalLanguage(1030); R.SetRange(\"Doc Date\", {cd}); Error('V:[%1]', {read}); end;"
        )
    };
    let (gf, gfs, gv) = (
        filter("R.GetFilter(\"Doc Date\")"),
        filter("R.GetFilters()"),
        filter("R.GetView()"),
    );
    let french_code = format!("begin GlobalLanguage(1036); Error('V:[%1]', Format({cd}, 0, '<Closing><Day>')); end;");
    let bodies: [(&str, &str); 10] = [
        ("OtherDay", &other_day),
        ("Danish", &danish),
        ("German", &german),
        ("ClosingMinusClosing", &two),
        ("ClosingMinusUndefined", &zero),
        ("UndefinedMinusClosing", &zero_first),
        ("GetFilterDanish", &gf),
        ("GetFiltersDanish", &gfs),
        ("GetViewDanish", &gv),
        ("FrenchClosingCode", &french_code),
    ];
    let mut src = format!("{FILTER_OBJECTS}codeunit 51700 T\n{{\n    Subtype = Test;\n");
    for (name, body) in &bodies {
        src += &format!("    [Test]\n    procedure {name}()\n    {body}\n");
    }
    src += "}\n";
    let o = run(&src);
    for (name, _) in &bodies {
        refused(&o, name);
    }
}

// Round-2 review neighbors of al-runner issues 8182 and 8183.
#[test]
fn boolean_and_name_neighbors_of_round_two_refuse_or_stay_clean() {
    let f =
        |l: i32, v: &str, p: &str| format!("begin GlobalLanguage({l}); Error('V:[%1]', Format({v}, 0, '{p}')); end;");
    let (dt, closing, width0) = (
        f(1030, "CreateDateTime(DMY2Date(15, 3, 2024), 120000T)", "<Month Text>"),
        f(1030, "ClosingDate(DMY2Date(15, 3, 2024))", "<Month Text>"),
        f(1030, "DMY2Date(15, 3, 2024)", "<Month Text,0>"),
    );
    let undefined = f(1031, "0D", "<Weekday Text>");
    let eval = |l: i32, t: &str| {
        format!(
            "var B: Boolean; Ok: Boolean; begin GlobalLanguage({l}); Ok := Evaluate(B, '{t}'); Error('V:[%1]', Ok); end;"
        )
    };
    let (upper, blanks) = (eval(1044, "JA"), eval(1044, " ja "));
    let o = tests(&[
        ("NameOfDateTime", &dt),
        ("NameOfClosingDate", &closing),
        ("NameWidthZero", &width0),
        ("NameOfUndefinedDate", &undefined),
        (
            "FormatTrueNumberOne",
            "begin GlobalLanguage(1044); Error('V:[%1]', Format(true, 0, 1)); end;",
        ),
        ("EvaluateUpperCase", &upper),
        ("EvaluateBlanks", &blanks),
        (
            "EvaluateYesStatement",
            "var B: Boolean; begin GlobalLanguage(1053); asserterror Evaluate(B, 'Yes'); end;",
        ),
    ]);
    for n in [
        "NameOfDateTime",
        "NameOfClosingDate",
        "NameWidthZero",
        "FormatTrueNumberOne",
        "EvaluateUpperCase",
        "EvaluateBlanks",
        "EvaluateYesStatement",
    ] {
        refused(&o, n);
    }
    reports(&o, "NameOfUndefinedDate", "", None);
}
