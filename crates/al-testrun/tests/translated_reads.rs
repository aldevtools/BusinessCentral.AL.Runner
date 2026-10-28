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

// ADR-053 step 2: the translation file is applied.

/// A unit id of the test app, from the names (`Codeunit`, `ALR Labels`, `NamedType`, `HelloLbl`, ...).
fn unit(parts: &[(&str, &str)]) -> String {
    parts
        .iter()
        .map(|(k, n)| al_ir::xliff_part(k, n))
        .collect::<Vec<_>>()
        .join(" - ")
}

/// The app's `nl-BE` file: the issue's two units, a field caption, and a unit with no target.
fn dutch() -> al_ir::TranslationFile {
    let mut units = vec![
        (
            unit(&[("Codeunit", "ALR Labels"), ("NamedType", "HelloLbl")]),
            "Hallo wereld".to_string(),
        ),
        (
            unit(&[
                ("Codeunit", "ALR Labels"),
                ("Method", "GetLocal"),
                ("NamedType", "LocalLbl"),
            ]),
            "Lokaal".to_string(),
        ),
        (
            unit(&[("Enum", "ALR Status"), ("EnumValue", "Rel"), ("Property", "Caption")]),
            "Vrijgegeven".to_string(),
        ),
        (
            unit(&[("Table", "ALR Item"), ("Field", "No."), ("Property", "Caption")]),
            "Nummer".to_string(),
        ),
    ];
    units.sort();
    al_ir::TranslationFile {
        name: "Translations/AL Runner Repros.nl-BE.xlf".into(),
        target_language: "nl-BE".into(),
        units,
        error: None,
    }
}

const APP2: &str = r#"
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
        OtherLbl: Label 'Other';

    procedure GetHello(): Text
    begin
        exit(HelloLbl);
    end;

    procedure GetOther(): Text
    begin
        exit(OtherLbl);
    end;

    procedure GetLocal(): Text
    var
        LocalLbl: Label 'Local';
    begin
        exit(LocalLbl);
    end;
}
table 50705 "ALR Item"
{
    Caption = 'Item';
    fields
    {
        field(1; "No."; Code[20]) { Caption = 'Number'; }
        field(2; Name; Text[30]) { Caption = 'Name'; }
    }
    keys { key(PK; "No.") { } }
}
"#;

const TESTS2: &str = r#"
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
        if Status.AsInteger() <> 1 then Error('ordinal %1', Status.AsInteger());
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
    procedure Texts_WithoutAUnit_KeepTheSource()
    var
        Labels: Codeunit "ALR Labels";
        Item: Record "ALR Item";
        Status: Enum "ALR Status";
        Observed: Text;
    begin
        GlobalLanguage(2067);
        Status := Status::Open;
        Observed := Labels.GetOther() + '|' + Labels.GetLocal() + '|' + Format(Status) + '|' + Item.FieldCaption("No.") + '|' + Item.FieldCaption(Name);
        GlobalLanguage(1033);
        Observed += '|' + Labels.GetHello() + '|' + Item.FieldCaption("No.");
        Error('%1', Observed);
    end;

    [Test]
    procedure Evaluate_Enum_SourceCaption_IsNotPinned()
    var
        Status: Enum "ALR Status";
    begin
        GlobalLanguage(2067);
        if Evaluate(Status, 'Released') then;
    end;

    [Test]
    procedure TableCaption_StaysRefused()
    var
        Item: Record "ALR Item";
    begin
        GlobalLanguage(2067);
        Error('%1', Item.TableCaption());
    end;
}

codeunit 50758 "ALR Other Languages"
{
    Subtype = Test;

    [Test]
    procedure Dutch_OfTheNetherlands_IsNotPinned()
    var
        Labels: Codeunit "ALR Labels";
    begin
        GlobalLanguage(1043);
        Error('%1', Labels.GetHello());
    end;
}

codeunit 50759 "ALR No File"
{
    Subtype = Test;

    [Test]
    procedure German_HasNoFile_KeepsTheSource()
    var
        Labels: Codeunit "ALR Labels";
    begin
        GlobalLanguage(1031);
        Error('%1', Labels.GetHello());
    end;
}
"#;

fn run_translated(
    files: Vec<al_ir::TranslationFile>,
) -> Vec<(String, Outcome, Vec<al_testrun::assumptions::Assumption>)> {
    let mut m = manifest(true);
    m.translation_files = files;
    let rt = CorpusRuntime::try_run_suites_with(
        vec![app("tests", &["app"], TESTS2), app("app", &[], APP2)],
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Workspace],
        &[manifest(false), m],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions::default(),
    )
    .expect("the run starts");
    rt.results()
        .iter()
        .map(|r| (r.test.clone(), r.outcome.clone(), r.assumptions.clone()))
        .collect()
}

fn find<'a>(
    got: &'a [(String, Outcome, Vec<al_testrun::assumptions::Assumption>)],
    test: &str,
) -> &'a (String, Outcome, Vec<al_testrun::assumptions::Assumption>) {
    got.iter()
        .find(|(t, ..)| t == test)
        .unwrap_or_else(|| panic!("no test {test}: {got:?}"))
}

#[test]
fn the_issues_three_tests_pass_with_the_apps_translation_file_as_flagged_passes() {
    let got = run_translated(vec![dutch()]);
    for test in [
        "Format_Enum_UsesDutchCaption",
        "Evaluate_Enum_AcceptsDutchCaption",
        "Label_UsesDutchText",
    ] {
        let (_, outcome, assumed) = find(&got, test);
        assert_eq!(outcome, &Outcome::Pass, "{test}: {got:?}");
        assert!(
            assumed.contains(&al_testrun::assumptions::Assumption::AppTranslationApplied),
            "{test}: {assumed:?}"
        );
    }
}

#[test]
fn a_text_without_a_unit_or_in_1033_keeps_the_source_and_a_field_caption_is_translated() {
    let got = run_translated(vec![dutch()]);
    let (_, outcome, _) = find(&got, "Texts_WithoutAUnit_KeepTheSource");
    assert!(
        matches!(outcome, Outcome::Fail(m) if m.contains("Other|Lokaal|Open|Nummer|Name|Hello world|Number")),
        "{outcome:?}"
    );
}

#[test]
fn unpinned_reads_under_a_translation_stay_refused() {
    let got = run_translated(vec![dutch()]);
    for (test, want) in [
        ("Evaluate_Enum_SourceCaption_IsNotPinned", "Evaluate"),
        ("TableCaption_StaysRefused", "the caption of table ALR Item"),
        // A file for nl-BE only, read under nl-NL: which file BC falls back to is not pinned (ADR-053 P3).
        ("Dutch_OfTheNetherlands_IsNotPinned", "the Label 'Hello world'"),
    ] {
        let (_, outcome, _) = find(&got, test);
        assert!(
            matches!(outcome, Outcome::NotRun(why) if why.contains(want)),
            "{test}: want {want:?}, got {outcome:?}"
        );
    }
}

#[test]
fn a_language_with_no_file_of_its_kind_keeps_the_source_text() {
    let got = run_translated(vec![dutch()]);
    let (_, outcome, assumed) = find(&got, "German_HasNoFile_KeepsTheSource");
    assert!(
        matches!(outcome, Outcome::Fail(m) if m.contains("Hello world")),
        "{outcome:?}"
    );
    assert!(assumed.contains(&al_testrun::assumptions::Assumption::AppTranslationApplied));
}

#[test]
fn an_app_folder_gives_its_parsed_translation_files() {
    let root = std::env::temp_dir().join(format!("al-translation-files-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("Translations")).unwrap();
    std::fs::write(
        root.join("app.json"),
        r#"{ "publisher": "AL Harness", "version": "1.0.0.0" }"#,
    )
    .unwrap();
    std::fs::write(root.join("Translations/App.g.xlf"), "<xliff/>").unwrap();
    std::fs::write(
        root.join("Translations/App.nl-BE.xlf"),
        r#"<xliff><file target-language="nl-BE"><body><trans-unit id="X 1"><source>a</source><target>b</target></trans-unit></body></file></xliff>"#,
    )
    .unwrap();
    let m = al_testrun::runtime::load_manifest(&root).unwrap();
    let _ = std::fs::remove_dir_all(&root);
    assert_eq!(m.translation_files.len(), 1);
    assert_eq!(m.translation_files[0].target_language, "nl-BE");
    assert_eq!(m.translation_files[0].target("X 1"), Some("b"));
}
