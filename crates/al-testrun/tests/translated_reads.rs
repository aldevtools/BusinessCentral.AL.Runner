//! Translated reads under another language (al-runner issue 5772, ADR-053 step 1): an app that ships a translation file
//! (`Translations/*.xlf`, not the generated `*.g.xlf`) has its labels, captions and enum captions translated by BC after
//! `GlobalLanguage(n)`. The runtime does not apply translations yet, so such a read in a language other than 1033 refuses
//! the test instead of answering the source (English) text.

use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, Manifest, RunOptions, SuiteRun};

fn app(id: &str, deps: &[&str], src: &str) -> AppSource {
    AppSource {
        id: id.into(),
        name: id.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![(format!("{id}.al"), src.to_string())],
    }
}

fn manifest(translations: bool) -> Manifest {
    Manifest {
        publisher: "AL Harness".into(),
        version: "1.0.0.0".into(),
        translations,
        ..Default::default()
    }
}

/// The objects of the issue's app (`ALRStatus.Enum.al`, `ALRLabels.Codeunit.al`) and a table with captions.
const APP: &str = r#"
enum 50704 "ALR Status"
{
    Extensible = true;
    value(0; Open) { Caption = 'Open'; }
    value(1; Rel) { Caption = 'Released'; }
}
codeunit 50704 "ALR Labels"
{
    var
        HelloLbl: Label 'Hello world';
        LockedLbl: Label 'Locked text', Locked = true;

    procedure GetHello(): Text
    begin
        exit(HelloLbl);
    end;

    procedure GetLocked(): Text
    begin
        exit(LockedLbl);
    end;
}
table 50705 "ALR Item"
{
    Caption = 'Item';
    fields
    {
        field(1; "No."; Code[20]) { Caption = 'Number'; }
        field(2; Kind; Option) { OptionMembers = A,B; OptionCaption = 'First,Second'; }
    }
    keys { key(PK; "No.") { } }
}
page 50707 "ALR Item Card"
{
    PageType = Card;
    SourceTable = "ALR Item";
    Caption = 'Item Card';
    layout { area(Content) { field(No; Rec."No.") { } } }
}

"#;

/// The issue's test codeunit (`ALRTranslationRepro.Codeunit.al`), with `Error` in place of Library Assert, and reads
/// that stay as they are.
const TESTS: &str = r#"
codeunit 50756 "ALR Translation Repro"
{
    Subtype = Test;

    [Test]
    procedure Format_Enum_UsesDutchCaption()
    var
        Status: Enum "ALR Status";
        Observed: Text;
    begin
        Status := Status::Rel;
        GlobalLanguage(2067);
        Observed := Format(Status);
        GlobalLanguage(1033);
        if Observed <> 'Vrijgegeven' then Error('Format(enum) under 2067: %1', Observed);
    end;

    [Test]
    procedure Evaluate_Enum_AcceptsDutchCaption()
    var
        Status: Enum "ALR Status";
        Ok: Boolean;
    begin
        GlobalLanguage(2067);
        Ok := Evaluate(Status, 'Vrijgegeven');
        GlobalLanguage(1033);
        if not Ok then Error('Evaluate(enum, Vrijgegeven) under 2067');
    end;

    [Test]
    procedure Label_UsesDutchText()
    var
        Labels: Codeunit "ALR Labels";
        Observed: Text;
    begin
        GlobalLanguage(2067);
        Observed := Labels.GetHello();
        GlobalLanguage(1033);
        if Observed <> 'Hallo wereld' then Error('Label under 2067: %1', Observed);
    end;

    [Test]
    procedure Captions_UnderAnotherLanguage()
    var
        Item: Record "ALR Item";
    begin
        GlobalLanguage(2067);
        Error('%1', Item.TableCaption());
    end;

    [Test]
    procedure FieldCaption_UnderAnotherLanguage()
    var
        Item: Record "ALR Item";
    begin
        GlobalLanguage(2067);
        Error('%1', Item.FieldCaption("No."));
    end;

    [Test]
    procedure OptionCaption_UnderAnotherLanguage()
    var
        Item: Record "ALR Item";
    begin
        Item.Kind := Item.Kind::B;
        GlobalLanguage(2067);
        Error('%1', Format(Item.Kind));
    end;

    [Test]
    procedure EvaluateOptionField_UnderAnotherLanguage()
    var
        Item: Record "ALR Item";
    begin
        GlobalLanguage(2067);
        Evaluate(Item.Kind, 'B');
        Error('%1', Item.Kind);
    end;

    [Test]
    procedure TestPageCaptions_UnderAnotherLanguage()
    var
        Card: TestPage "ALR Item Card";
    begin
        Card.OpenView();
        GlobalLanguage(2067);
        Error('%1', Card.No.Caption());
    end;

    [Test]
    procedure TestPageCaption_UnderAnotherLanguage()
    var
        Card: TestPage "ALR Item Card";
    begin
        Card.OpenView();
        GlobalLanguage(2067);
        Error('%1', Card.Caption());
    end;
}

// A refused test leaves the session language changed, and the runner then refuses the rest of its codeunit: the reads that
// keep the source text are in a codeunit of their own.
codeunit 50757 "ALR Source Texts"
{
    Subtype = Test;

    [Test]
    procedure Reads_Under1033()
    var
        Labels: Codeunit "ALR Labels";
        Status: Enum "ALR Status";
        Item: Record "ALR Item";
    begin
        Status := Status::Rel;
        Item.Kind := Item.Kind::B;
        Error('%1|%2|%3|%4|%5', Format(Status), Labels.GetHello(), Item.TableCaption(), Item.FieldCaption("No."), Format(Item.Kind));
    end;

    [Test]
    procedure LockedLabel_UnderAnotherLanguage()
    var
        Labels: Codeunit "ALR Labels";
    begin
        GlobalLanguage(2067);
        Error('%1', Labels.GetLocked());
    end;
}
"#;

fn run(translations: bool) -> Vec<(String, Outcome)> {
    let rt = CorpusRuntime::try_run_suites_with(
        vec![app("tests", &["app"], TESTS), app("app", &[], APP)],
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Workspace],
        &[manifest(false), manifest(translations)],
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

fn outcome<'a>(got: &'a [(String, Outcome)], test: &str) -> &'a Outcome {
    &got.iter()
        .find(|(t, _)| t == test)
        .unwrap_or_else(|| panic!("no test {test}: {got:?}"))
        .1
}

#[test]
fn the_issues_three_tests_and_other_translatable_reads_are_not_run_when_the_app_ships_a_translation() {
    let got = run(true);
    for (test, what) in [
        (
            "Format_Enum_UsesDutchCaption",
            "the caption of value Rel of enum ALR Status",
        ),
        ("Evaluate_Enum_AcceptsDutchCaption", "the captions of enum ALR Status"),
        ("Label_UsesDutchText", "the Label 'Hello world'"),
        ("Captions_UnderAnotherLanguage", "the caption of table ALR Item"),
        (
            "FieldCaption_UnderAnotherLanguage",
            "the caption of field No. of table ALR Item",
        ),
        (
            "OptionCaption_UnderAnotherLanguage",
            "the OptionCaption of field Kind of table ALR Item",
        ),
        (
            "EvaluateOptionField_UnderAnotherLanguage",
            "the OptionCaption of field Kind of table ALR Item",
        ),
        (
            "TestPageCaptions_UnderAnotherLanguage",
            "the caption of control No of page ALR Item Card",
        ),
        (
            "TestPageCaption_UnderAnotherLanguage",
            "the caption of page ALR Item Card",
        ),
    ] {
        let want = format!("translations are not supported: {what} under GlobalLanguage 2067");
        assert!(
            matches!(outcome(&got, test), Outcome::NotRun(why) if why.contains(&want)),
            "{test}: want {want:?}, got {:?}",
            outcome(&got, test)
        );
    }
}

#[test]
fn reads_in_1033_and_locked_labels_keep_the_source_text() {
    let got = run(true);
    assert!(
        matches!(outcome(&got, "Reads_Under1033"), Outcome::Fail(m) if m.contains("Released|Hello world|Item|Number|Second")),
        "{got:?}"
    );
    assert!(
        matches!(outcome(&got, "LockedLabel_UnderAnotherLanguage"), Outcome::Fail(m) if m.contains("Locked text")),
        "{got:?}"
    );
}

#[test]
fn an_app_without_a_translation_file_keeps_the_source_text_in_another_language() {
    // What the corpus pins under GlobalLanguage 1031 for its apps, which ship no translation file (for example
    // record/lt-enum-filter-caption/EnumFilterByCaption.al): the source text.
    let got = run(false);
    assert!(
        matches!(outcome(&got, "Format_Enum_UsesDutchCaption"), Outcome::Fail(m) if m.contains("Released")),
        "{got:?}"
    );
    assert!(
        matches!(outcome(&got, "Label_UsesDutchText"), Outcome::Fail(m) if m.contains("Hello world")),
        "{got:?}"
    );
}

#[test]
fn an_app_folder_ships_a_translation_file_when_its_translations_folder_holds_an_xlf_that_is_not_generated() {
    let root = std::env::temp_dir().join(format!("al-translated-reads-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("Translations")).unwrap();
    std::fs::write(
        root.join("app.json"),
        r#"{ "publisher": "AL Harness", "version": "1.0.0.0" }"#,
    )
    .unwrap();
    std::fs::write(root.join("Translations/AL Runner Repros.g.xlf"), "<xliff/>").unwrap();
    assert!(!al_testrun::runtime::load_manifest(&root).unwrap().translations);
    std::fs::write(root.join("Translations/AL Runner Repros.nl-BE.xlf"), "<xliff/>").unwrap();
    assert!(al_testrun::runtime::load_manifest(&root).unwrap().translations);
    let _ = std::fs::remove_dir_all(&root);
}
