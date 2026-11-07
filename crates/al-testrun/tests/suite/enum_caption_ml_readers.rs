//! Readers of an enum value's `CaptionML` other than Format and Evaluate (al-runner pull request 7237 made those two read the
//! session language; corpus format-evaluate/EvalLanguageBoolean.al EL_Enum_* pins only them). A page control, a filter text,
//! a key value and a field's members would show the name or the plain caption under another GlobalLanguage, a silent wrong
//! result, so they refuse the test (hard rule 4) until a corpus probe pins them.

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
    (
        "SetFilterCaption",
        "GlobalLanguage(2067); Row.SetFilter(Status, 'Gesloten BE'); Error('%1', Row.Count());",
    ),
    (
        "FieldRefOptionString",
        "GlobalLanguage(2067); Error('%1', RecRef.Field(2).OptionCaption);",
    ),
];

fn tests() -> String {
    let mut s = String::new();
    for (i, (name, body)) in CASES.iter().enumerate() {
        s.push_str(&format!(
            "codeunit {} \"EM {name}\"\n{{\n    Subtype = Test;\n    [Test]\n    procedure T_{name}()\n    var\n        Row: Record \"EM Row\";\n        Card: TestPage \"EM Card\";\n        RecRef: RecordRef;\n    begin\n        RecRef.Open(Database::\"EM Row\");\n        {body}\n    end;\n}}\n",
            50820 + i
        ));
    }
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
fn an_enum_value_with_a_plain_caption_is_still_read() {
    let got = run();
    let (_, o) = got.iter().find(|(x, _)| x == "T_Plain").expect("T_Plain");
    assert!(matches!(o, Outcome::Fail(m) if m.contains("Open plain")), "{o:?}");
}
