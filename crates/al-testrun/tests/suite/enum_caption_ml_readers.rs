//! Readers of an enum value's `CaptionML` other than Format and Evaluate (al-runner pull request 7237 made those two read the
//! session language; corpus format-evaluate/EvalLanguageBoolean.al EL_Enum_* pins only them). A page control, a filter text,
//! a key value and a field's members would show the name or the plain caption under another GlobalLanguage, a silent wrong
//! result, so they refuse the test (hard rule 4) until a corpus probe pins them.
//!
//! Filter texts are pinned since corpus format-evaluate/EnumCaptionReaders.al (ECR_FilterTexts_1033 and _2067, :573-662;
//! ECR_FilterWordsStatus_2067, :738-753): a filter set and read under the same GlobalLanguage shows and accepts the caption of
//! that language. A filter set under one language and read under another is not pinned and still refuses.

use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, Manifest, RunOptions, SuiteRun};

const APP: &str = r#"
enum 50810 "EM Status"
{
    Extensible = true;
    value(0; Open) { CaptionML = ENU = 'Open cap', NLB = 'Open BE'; }
    value(1; Closed) { CaptionML = ENU = 'Closed cap', NLB = 'Gesloten BE'; }
}
enum 50811 "EM Plain"
{
    Extensible = true;
    value(0; Open) { Caption = 'Open plain'; }
}
table 50810 "EM Row"
{
    fields
    {
        field(1; "No."; Code[10]) { }
        field(2; Status; Enum "EM Status") { }
        field(3; Plain; Enum "EM Plain") { }
    }
    keys { key(PK; "No.") { } }
}
page 50810 "EM Card"
{
    PageType = Card;
    SourceTable = "EM Row";
    layout { area(Content) { field("No."; Rec."No.") { } field(Status; Rec.Status) { } field(Plain; Rec.Plain) { } } }
}
"#;

/// Test name, body and the error text it ends with: shapes the corpus pins (see the module comment).
const PINNED: &[(&str, &str, &str)] = &[
    (
        "GetFilterSameLanguage",
        "GlobalLanguage(2067); Row.SetRange(Status, Row.Status::Closed); Error('%1', Row.GetFilter(Status));",
        "Gesloten BE",
    ),
    (
        "SetFilterCaption",
        "GlobalLanguage(2067); Row.SetFilter(Status, 'Gesloten BE'); Error('%1', Row.Count());",
        "0",
    ),
];

/// Test name and body; each test is a codeunit of its own because a refused test leaves the session language changed.
const CASES: &[(&str, &str)] = &[
    (
        "PageValue",
        "Row.\"No.\" := 'A'; Row.Status := Row.Status::Closed; Row.Insert(); Card.OpenView(); GlobalLanguage(2067); Error('%1', Card.Status.Value);",
    ),
    (
        "PageSetValue",
        "Row.\"No.\" := 'A'; Row.Insert(); Card.OpenEdit(); GlobalLanguage(2067); Card.Status.SetValue('Gesloten BE'); Error('%1', 'set');",
    ),
    (
        "GetFilter",
        "Row.SetRange(Status, Row.Status::Closed); GlobalLanguage(2067); Error('%1', Row.GetFilter(Status));",
    ),
    // A FieldRef filter set under 1033, then another filter on the table under 2067: the entry keeps the language it was set under.
    (
        "FieldRefSetRange",
        "Fld := RecRef.Field(2); Fld.SetRange(Row.Status::Closed); GlobalLanguage(2067); Other.SetRange(\"No.\", 'A'); Error('%1', Fld.GetFilter());",
    ),
    (
        "FieldRefGetFilters",
        "Fld := RecRef.Field(2); Fld.SetRange(Row.Status::Closed); GlobalLanguage(2067); Other.SetRange(\"No.\", 'A'); Error('%1', RecRef.GetFilters());",
    ),
    (
        "FieldRefOptionString",
        "GlobalLanguage(2067); Error('%1', RecRef.Field(2).OptionCaption);",
    ),
];

fn tests() -> String {
    let mut s = String::new();
    let all = CASES.iter().copied().chain(PINNED.iter().map(|(n, b, _)| (*n, *b)));
    for (i, (name, body)) in all.enumerate() {
        s.push_str(&format!(
            "codeunit {} \"EM {name}\"\n{{\n    Subtype = Test;\n    [Test]\n    procedure T_{name}()\n    var\n        Row: Record \"EM Row\";\n        Card: TestPage \"EM Card\";\n        RecRef: RecordRef;\n        Fld: FieldRef;\n        Other: Record \"EM Row\";\n    begin\n        RecRef.Open(Database::\"EM Row\");\n        {body}\n    end;\n}}\n",
            50820 + i
        ));
    }
    // A global record keeps the filter an earlier test method set under 1033; the next one reads it under 2067.
    s.push_str(
        "codeunit 50841 \"EM Carried\"\n{\n    Subtype = Test;\n    var\n        G: Record \"EM Row\";\n    [Test]\n    procedure T_CarryA()\n    begin\n        G.SetRange(Status, G.Status::Closed);\n    end;\n    [Test]\n    procedure T_CarryB()\n    var\n        Other: Record \"EM Row\";\n    begin\n        GlobalLanguage(2067); Other.SetRange(\"No.\", 'A'); Error('%1', G.GetFilter(Status));\n    end;\n}\n",
    );
    s.push_str(
        "codeunit 50840 \"EM Plain Reads\"\n{\n    Subtype = Test;\n    [Test]\n    procedure T_Plain()\n    var\n        Row: Record \"EM Row\";\n        Card: TestPage \"EM Card\";\n    begin\n        Row.\"No.\" := 'A'; Row.Insert(); Card.OpenView(); Error('%1', Card.Plain.Value);\n    end;\n}\n",
    );
    s
}

fn app(id: &str, deps: &[&str], src: String) -> AppSource {
    AppSource {
        id: id.into(),
        name: id.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![(format!("{id}.al"), src)],
    }
}

fn run() -> Vec<(String, Outcome)> {
    let manifest = Manifest {
        publisher: "AL Harness".into(),
        version: "1.0.0.0".into(),
        ..Default::default()
    };
    let rt = CorpusRuntime::try_run_suites_with(
        vec![app("tests", &["app"], tests()), app("app", &[], APP.to_string())],
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Workspace],
        &[manifest.clone(), manifest],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions::default(),
    )
    .expect("the run starts");
    rt.results()
        .iter()
        .map(|r| (r.test.clone(), r.outcome.clone()))
        .collect()
}

#[test]
fn readers_of_an_enum_caption_ml_other_than_format_and_evaluate_refuse_the_test() {
    let got = run();
    for (name, _) in CASES {
        let t = format!("T_{name}");
        let (_, o) = got
            .iter()
            .find(|(x, _)| *x == t)
            .unwrap_or_else(|| panic!("no {t}: {got:?}"));
        assert!(
            matches!(o, Outcome::NotRun(why) if why.contains("CaptionML; not pinned for")),
            "{t}: {o:?}"
        );
    }
}

#[test]
fn a_filter_a_global_record_carried_from_an_earlier_test_under_another_language_refuses() {
    let got = run();
    let (_, a) = got.iter().find(|(x, _)| x == "T_CarryA").expect("T_CarryA");
    assert!(matches!(a, Outcome::Pass), "T_CarryA: {a:?}");
    let (_, o) = got.iter().find(|(x, _)| x == "T_CarryB").expect("T_CarryB");
    assert!(
        matches!(o, Outcome::NotRun(why) if why.contains("CaptionML; not pinned for")),
        "T_CarryB: {o:?}"
    );
}

#[test]
fn a_filter_set_and_read_under_the_same_language_shows_and_accepts_its_caption() {
    let got = run();
    for (name, _, want) in PINNED {
        let t = format!("T_{name}");
        let (_, o) = got
            .iter()
            .find(|(x, _)| *x == t)
            .unwrap_or_else(|| panic!("no {t}: {got:?}"));
        assert!(matches!(o, Outcome::Fail(m) if m == want), "{t}: {o:?}");
    }
}

#[test]
fn an_enum_value_with_a_plain_caption_is_still_read() {
    let got = run();
    let (_, o) = got.iter().find(|(x, _)| x == "T_Plain").expect("T_Plain");
    assert!(matches!(o, Outcome::Fail(m) if m.contains("Open plain")), "{o:?}");
}
