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
            // A field control with no Caption of its own takes its source field's caption, so the refusal names the field
            // (localization/app-xliff/lt-r7144-ext-caption/R7187PageCaption.al, al-runner issue 7187).
            "TestPageCaptions_UnderAnotherLanguage",
            "the caption of field No. of table ALR Item",
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
    // record/filters/EnumFilterByCaption.al): the source text.
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
        states: Vec::new(),
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
    procedure Evaluate_Enum_SourceCaption_IsRejected()
    var
        Status: Enum "ALR Status";
    begin
        GlobalLanguage(2067);
        if Evaluate(Status, 'Released') then;
    end;

    [Test]
    procedure TableCaption_WithoutAUnit_KeepsTheSource()
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
    run_with(m)
}

/// The tests of the second app, whose app has manifest `m`.
fn run_with(m: Manifest) -> Vec<(String, Outcome, Vec<al_testrun::assumptions::Assumption>)> {
    run_src(APP2, TESTS2, m)
}

/// The tests `tests` of an app with source `src` and manifest `m`.
fn run_src(src: &str, tests: &str, m: Manifest) -> Vec<(String, Outcome, Vec<al_testrun::assumptions::Assumption>)> {
    let rt = CorpusRuntime::try_run_suites_with(
        vec![app("tests", &["app"], tests), app("app", &[], src)],
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
fn the_issues_three_tests_pass_with_the_apps_translation_file() {
    let got = run_translated(vec![dutch()]);
    for test in [
        "Format_Enum_UsesDutchCaption",
        "Evaluate_Enum_AcceptsDutchCaption",
        "Label_UsesDutchText",
    ] {
        // Pinned: localization/app-xliff/TaXliff.al TA_Enum_Format_And_Evaluate, TA_Label_Text (al-runner issue 5772).
        let (_, outcome, assumed) = find(&got, test);
        assert_eq!(outcome, &Outcome::Pass, "{test}: {got:?}");
        assert!(assumed.is_empty(), "{test}: {assumed:?}");
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
    // A file for nl-BE only, read under nl-NL: only the nl-NL file under nl-BE is pinned (TaXliff.al:146, :176; ADR-053 P3).
    let (_, outcome, _) = find(&got, "Dutch_OfTheNetherlands_IsNotPinned");
    assert!(
        matches!(outcome, Outcome::NotRun(why) if why.contains("the Label 'Hello world'")),
        "{outcome:?}"
    );
}

#[test]
fn the_source_caption_of_a_translated_value_is_rejected_and_a_table_caption_without_a_unit_is_the_source() {
    let got = run_translated(vec![dutch()]);
    // Pinned: localization/app-xliff/TaXliff.al:140 (`Released` false|0 under 1043) and :216 (the table caption through its
    // unit; without one the source stays, as for every text: :138, :173).
    let (_, outcome, _) = find(&got, "Evaluate_Enum_SourceCaption_IsRejected");
    assert_eq!(outcome, &Outcome::Pass, "{got:?}");
    let (_, outcome, _) = find(&got, "TableCaption_WithoutAUnit_KeepsTheSource");
    assert!(matches!(outcome, Outcome::Fail(m) if m.contains("Item")), "{outcome:?}");
}

#[test]
fn a_package_gives_its_translation_files_on_demand() {
    struct Source(Vec<al_ir::TranslationFile>, Vec<al_ir::TranslationFile>);
    impl al_ir::TranslationSource for Source {
        fn files(&self) -> &[al_ir::TranslationFile] {
            &self.0
        }
        fn file(&self, index: usize) -> &al_ir::TranslationFile {
            &self.1[index]
        }
    }
    let read = dutch();
    let head = al_ir::TranslationFile {
        units: Vec::new(),
        ..read.clone()
    };
    let mut m = manifest(true);
    m.translation_source = Some(al_ir::TranslationSourceRef(std::sync::Arc::new(Source(
        vec![head],
        vec![read],
    ))));
    let got = run_with(m);
    // The same as the app folder's file (TaXliff.al TA_Label_Text, TA_Enum_Format_And_Evaluate).
    for test in ["Format_Enum_UsesDutchCaption", "Label_UsesDutchText"] {
        let (_, outcome, _) = find(&got, test);
        assert_eq!(outcome, &Outcome::Pass, "{test}: {got:?}");
    }
}

#[test]
fn a_unit_with_any_state_gives_its_target_flagged() {
    let mut file = dutch();
    file.states = file
        .units
        .iter()
        .map(|(k, _)| {
            if k.starts_with("Enum ") {
                al_ir::UnitState::Other("needs-review-translation".into())
            } else {
                al_ir::UnitState::Translated
            }
        })
        .collect();
    let got = run_translated(vec![file]);
    // Every state gives the target as main did (ADR-053 decision 1), flagged until corpus probe TlTranslationCells.al
    // TL_UnitForms_Dutch pins the states.
    for test in ["Label_UsesDutchText", "Format_Enum_UsesDutchCaption"] {
        let (_, outcome, assumed) = find(&got, test);
        assert_eq!(outcome, &Outcome::Pass, "{test}: {got:?}");
        assert_eq!(
            assumed,
            &[al_testrun::assumptions::Assumption::XliffUnitState],
            "{test}"
        );
    }
}

#[test]
fn the_exact_culture_file_wins_over_a_sibling_file_that_has_the_unit() {
    let mut sibling = dutch();
    sibling.name = "Translations/AL Runner Repros.nl-NL.xlf".into();
    sibling.target_language = "nl-NL".into();
    for (_, t) in &mut sibling.units {
        *t = format!("{t} NL");
    }
    let got = run_translated(vec![dutch(), sibling]);
    let (_, outcome, assumed) = find(&got, "Label_UsesDutchText");
    assert_eq!(outcome, &Outcome::Pass, "{got:?}");
    assert_eq!(
        assumed,
        &[al_testrun::assumptions::Assumption::XliffExactCultureOverSibling]
    );
}

const OPT_APP: &str = r#"
table 50790 "ALR Opt"
{
    Caption = 'Opt';
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Kind; Option) { OptionMembers = Alpha,Beta,Gamma; OptionCaption = 'Alpha,Second,Third'; }
    }
    keys { key(PK; "No.") { } }
}
table 50791 "ALR Opt Base"
{
    fields
    {
        field(1; "No."; Code[20]) { }
    }
    keys { key(PK; "No.") { } }
}
tableextension 50792 "ALR Opt Ext" extends "ALR Opt Base"
{
    fields { field(50792; ExtKind; Option) { OptionMembers = One,Two; OptionCaption = 'One,Two'; } }
}
"#;

const OPT_TESTS: &str = r#"
codeunit 50793 "ALR Opt Tests"
{
    Subtype = Test;

    [Test]
    procedure MemberNameEqualToItsCaption()
    var
        R: Record "ALR Opt";
    begin
        GlobalLanguage(1043);
        if not Evaluate(R.Kind, 'Alpha') then Error('no');
        GlobalLanguage(1033);
        if R.Kind <> R.Kind::Alpha then Error('wrong');
    end;

    [Test]
    procedure MemberNameOtherThanItsCaption()
    var
        R: Record "ALR Opt";
    begin
        GlobalLanguage(1043);
        if Evaluate(R.Kind, 'Gamma') then;
    end;

    [Test]
    procedure FormatOfAnExtensionOptionField()
    var
        R: Record "ALR Opt Base";
    begin
        R.ExtKind := R.ExtKind::Two;
        GlobalLanguage(1043);
        Error(Format(R.ExtKind));
    end;
}
"#;

#[test]
fn option_captions_run_only_in_the_pinned_shapes() {
    let mut m = manifest(true);
    m.translation_files = vec![al_ir::TranslationFile {
        name: "Translations/App.nl-NL.xlf".into(),
        target_language: "nl-NL".into(),
        units: {
            let mut u = vec![
                (
                    unit(&[("Table", "ALR Opt"), ("Field", "Kind"), ("Property", "OptionCaption")]),
                    "Eerste,Tweede,Derde".to_string(),
                ),
                (
                    unit(&[
                        ("TableExtension", "ALR Opt Ext"),
                        ("Field", "ExtKind"),
                        ("Property", "OptionCaption"),
                    ]),
                    "Een,Twee".to_string(),
                ),
            ];
            u.sort();
            u
        },
        error: None,
        states: Vec::new(),
    }];
    let got = run_src(OPT_APP, OPT_TESTS, m);
    // TaXliff.al:220: a member name that is also its source caption evaluates.
    let (_, outcome, _) = find(&got, "MemberNameEqualToItsCaption");
    assert_eq!(outcome, &Outcome::Pass, "{got:?}");
    // Not pinned: a member name that differs from its source caption, and a field a table extension declares.
    for test in ["MemberNameOtherThanItsCaption", "FormatOfAnExtensionOptionField"] {
        let (_, outcome, _) = find(&got, test);
        assert!(matches!(outcome, Outcome::NotRun(_)), "{test}: {outcome:?}");
    }
}

#[test]
fn a_language_with_no_file_of_its_kind_keeps_the_source_text() {
    let got = run_translated(vec![dutch()]);
    // Pinned: localization/app-xliff/TaXliff.al:230 TA_Captions_Language_Without_File.
    let (_, outcome, assumed) = find(&got, "German_HasNoFile_KeepsTheSource");
    assert!(
        matches!(outcome, Outcome::Fail(m) if m.contains("Hello world")),
        "{outcome:?}"
    );
    assert!(assumed.is_empty(), "{assumed:?}");
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

/// al-runner issue 7144: a table of another app (a stand-in for Base Application's Sales Invoice Header) and a table
/// extension of it in the app that ships the translation file.
const EXT_BASE: &str = r#"
table 50810 "ALR Sales Header"
{
    fields
    {
        field(1; "No."; Code[20]) { Caption = 'No.'; }
    }
    keys { key(PK; "No.") { } }
}
"#;

const EXT_APP: &str = r#"
tableextension 50811 "ALR Sales Header Ext" extends "ALR Sales Header"
{
    fields
    {
        field(50811; "ALR Payment ID"; Text[30]) { Caption = 'Payment ID'; }
        field(50812; "ALR Plain"; Text[30]) { Caption = 'Plain caption'; }
    }
}
table 50813 "ALR Own Row"
{
    fields { field(1; "No."; Integer) { } }
    keys { key(PK; "No.") { } }
}
tableextension 50814 "ALR Own Row Ext" extends "ALR Own Row"
{
    fields { field(50814; "ALR Own Ext"; Text[30]) { Caption = 'Own extension'; } }
}
"#;

const EXT_TESTS: &str = r#"
codeunit 50812 "ALR Ext Caption Repro"
{
    Subtype = Test;

    [Test]
    procedure ExtensionFieldCaptionInDutch()
    var
        H: Record "ALR Sales Header";
        Observed: Text;
    begin
        GlobalLanguage(1043);
        Observed := H.FieldCaption("ALR Payment ID") + '|' + H.FieldCaption("ALR Plain");
        GlobalLanguage(1033);
        Error('%1', Observed);
    end;

    [Test]
    procedure ExtensionFieldRefCaptionInDutch()
    var
        RecRef: RecordRef;
        Observed: Text;
    begin
        RecRef.Open(50810);
        GlobalLanguage(1043);
        Observed := RecRef.Field(50811).Caption();
        GlobalLanguage(1033);
        Error('%1', Observed);
    end;

    [Test]
    procedure OwnTableExtensionFieldCaptionInDutch()
    var
        R: Record "ALR Own Row";
        Observed: Text;
    begin
        GlobalLanguage(1043);
        Observed := R.FieldCaption("ALR Own Ext");
        GlobalLanguage(1033);
        Error('%1', Observed);
    end;
}
"#;

fn run_extension_captions() -> Vec<(String, Outcome, Vec<al_testrun::assumptions::Assumption>)> {
    let mut m = manifest(true);
    m.translation_files = vec![al_ir::TranslationFile {
        name: "Translations/App.nl-NL.xlf".into(),
        target_language: "nl-NL".into(),
        units: vec![(
            unit(&[
                ("TableExtension", "ALR Sales Header Ext"),
                ("Field", "ALR Payment ID"),
                ("Property", "Caption"),
            ]),
            "Betalings-ID".to_string(),
        )],
        error: None,
        states: Vec::new(),
    }];
    let rt = CorpusRuntime::try_run_suites_with(
        vec![
            app("tests", &["app", "base"], EXT_TESTS),
            app("app", &["base"], EXT_APP),
            app("base", &[], EXT_BASE),
        ],
        vec![al_sema::AppKind::Workspace; 3],
        &[manifest(false), m, manifest(false)],
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

/// A field a table extension adds takes its caption from the extension's unit of the app's translation file, as a field of
/// the app's own table does; a field without a unit keeps the source.
#[test]
fn a_table_extension_field_caption_is_translated_from_the_apps_file() {
    let got = run_extension_captions();
    for (test, want) in [
        ("ExtensionFieldCaptionInDutch", "Betalings-ID|Plain caption"),
        ("ExtensionFieldRefCaptionInDutch", "Betalings-ID"),
    ] {
        let (_, outcome, assumed) = find(&got, test);
        assert!(
            matches!(outcome, Outcome::Fail(m) if m.contains(want)),
            "{test}: {outcome:?}"
        );
        assert!(
            assumed.is_empty(),
            "{test}: a clean pass, R7144ExtFieldCaption.al:9-12, :80-83 and :99-103 pin it (corpus PR 1156)"
        );
    }
    // An extension of a table of its own app keeps the source caption (R7144ExtFieldCaption.al:83, :103).
    let (_, outcome, _) = find(&got, "OwnTableExtensionFieldCaptionInDutch");
    assert!(
        matches!(outcome, Outcome::Fail(m) if m == "Own extension"),
        "{outcome:?}"
    );
}

/// The exact culture's file decides even when it lacks the unit and a sibling file of the same language has it: the source
/// text stays, as the runtime gave before, flagged under `xliff-exact-culture-over-sibling`.
#[test]
fn the_exact_culture_file_without_the_unit_keeps_the_source_beside_a_sibling_that_has_it() {
    let mut own = dutch();
    let hello = unit(&[("Codeunit", "ALR Labels"), ("NamedType", "HelloLbl")]);
    own.units.retain(|(k, _)| *k != hello);
    let mut sibling = dutch();
    sibling.name = "Translations/AL Runner Repros.nl-NL.xlf".into();
    sibling.target_language = "nl-NL".into();
    let got = run_translated(vec![own, sibling]);
    let (_, outcome, assumed) = find(&got, "Label_UsesDutchText");
    assert!(
        matches!(outcome, Outcome::Fail(m) if m.contains("Label under 2067: Hello world")),
        "{outcome:?}"
    );
    assert_eq!(
        assumed,
        &[al_testrun::assumptions::Assumption::XliffExactCultureOverSibling]
    );
}

const OPT_BASE_APP: &str = r#"
table 50820 "ALR Opt Owner"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Kind; Option) { OptionMembers = Alpha,Beta; OptionCaption = 'First,Second'; }
    }
    keys { key(PK; "No.") { } }
}
"#;

const OPT_EXT_APP: &str = r#"
tableextension 50821 "ALR Opt Owner Ext" extends "ALR Opt Owner"
{
    fields { field(50821; Note; Text[30]) { Caption = 'Note'; } }
}
"#;

const OPT_EXT_TESTS: &str = r#"
codeunit 50822 "ALR Opt Owner Tests"
{
    Subtype = Test;

    [Test]
    procedure FormatOfAnOptionOfAnAppWithoutAFile()
    var
        R: Record "ALR Opt Owner";
    begin
        R.Kind := R.Kind::Beta;
        GlobalLanguage(1031);
        Error(Format(R.Kind));
    end;
}
"#;

/// A table of an app with no translation file, extended by an app with a de-DE file: the option field has no unit, so
/// Format keeps its own caption under 1031 (a value without a unit keeps its source, TaXliff.al:138).
#[test]
fn an_option_without_a_unit_of_an_extended_table_keeps_its_caption() {
    let mut m = manifest(true);
    m.translation_files = vec![al_ir::TranslationFile {
        name: "Translations/Ext.de-DE.xlf".into(),
        target_language: "de-DE".into(),
        units: vec![(
            unit(&[
                ("TableExtension", "ALR Opt Owner Ext"),
                ("Field", "Note"),
                ("Property", "Caption"),
            ]),
            "Notiz".to_string(),
        )],
        error: None,
        states: Vec::new(),
    }];
    let rt = CorpusRuntime::try_run_suites_with(
        vec![
            app("tests", &["ext", "base"], OPT_EXT_TESTS),
            app("ext", &["base"], OPT_EXT_APP),
            app("base", &[], OPT_BASE_APP),
        ],
        vec![al_sema::AppKind::Workspace; 3],
        &[manifest(false), m, manifest(false)],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions::default(),
    )
    .expect("the run starts");
    let got: Vec<_> = rt
        .results()
        .iter()
        .map(|r| (r.test.clone(), r.outcome.clone(), r.assumptions.clone()))
        .collect();
    let (_, outcome, _) = find(&got, "FormatOfAnOptionOfAnAppWithoutAFile");
    assert!(matches!(outcome, Outcome::Fail(m) if m == "Second"), "{outcome:?}");
}
