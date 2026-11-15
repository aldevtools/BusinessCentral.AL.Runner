//! al-runner issue 8123: `Format(RecordId, 0, 9)` names the table and never its caption, so the session language does not
//! change it, also when an app with a translation file for that language extends the table. The runtime refused the format
//! because it asked for the table's caption, which the ADR-053 gate refuses there. Until the corpus pins it, the format runs
//! through the assumption `recordid-format-nine-extended-table`; format 1, which shows the caption, stays refused.

use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::assumptions::Assumption;
use al_testrun::runtime::{CorpusRuntime, Manifest, RunOptions, SuiteRun};

/// A stand-in for Base Application: an app without a translation file, its tables in a namespace, each with a Caption.
const BASE: &str = r#"
namespace ALR.Base;

table 50830 "ALR Unit"
{
    Caption = 'Unit';
    fields { field(1; Code; Code[10]) { Caption = 'Code'; } }
    keys { key(PK; Code) { } }
}
table 50831 "ALR Movement Line"
{
    Caption = 'Movement Line';
    fields { field(1; "No."; Code[20]) { Caption = 'No.'; } }
    keys { key(PK; "No.") { } }
}
table 50836 "ALR Captionless"
{
    fields { field(1; "No."; Code[20]) { Caption = 'No.'; } }
    keys { key(PK; "No.") { } }
}
"#;

/// An app without a translation file whose table is outside any namespace.
const BARE: &str = r#"
table 50838 "ALR Bare Row"
{
    Caption = 'Bare Row';
    fields { field(1; "No."; Code[20]) { Caption = 'No.'; } }
    keys { key(PK; "No.") { } }
}
"#;

/// The app under test: it extends one of the two tables and ships a da-DK file with a unit for its own field only.
const EXT: &str = r#"
namespace ALR.Ext;

using ALR.Base;

tableextension 50832 "ALR Unit Ext" extends "ALR Unit"
{
    fields { field(50832; Note; Text[50]) { Caption = 'Note'; } }
}
table 50834 "ALR Own Row"
{
    Caption = 'Own Row';
    fields { field(1; "No."; Code[20]) { Caption = 'No.'; } }
    keys { key(PK; "No.") { } }
}
tableextension 50835 "ALR Own Row Ext" extends "ALR Own Row"
{
    fields { field(50835; Note; Text[50]) { Caption = 'Note'; } }
}
tableextension 50837 "ALR Captionless Ext" extends "ALR Captionless"
{
    Caption = 'Captionless';
    fields { field(50837; Note; Text[50]) { Caption = 'Note'; } }
}
tableextension 50839 "ALR Bare Row Ext" extends "ALR Bare Row"
{
    fields { field(50839; Note; Text[50]) { Caption = 'Note'; } }
}
"#;

const TESTS: &str = r#"
namespace ALR.Tests;

using ALR.Base;

codeunit 50833 "ALR RecordId Format Tests"
{
    Subtype = Test;

    [Test]
    procedure Format9OfTheExtendedTableUnderDanish()
    var
        R: Record "ALR Unit";
        English: Text;
        Danish: Text;
    begin
        R.Code := 'U1';
        English := Format(R.RecordId(), 0, 9);
        GlobalLanguage(1030);
        Danish := Format(R.RecordId(), 0, 9);
        GlobalLanguage(1033);
        Error('%1|%2', English, Danish);
    end;

    [Test]
    procedure Format9OfATableNotExtendedUnderDanish()
    var
        R: Record "ALR Movement Line";
        Danish: Text;
    begin
        R."No." := 'M1';
        GlobalLanguage(1030);
        Danish := Format(R.RecordId(), 0, 9);
        GlobalLanguage(1033);
        Error('%1', Danish);
    end;

    [Test]
    procedure Format9OfAnOwnTableOfTheTranslatingAppUnderDanish()
    var
        R: Record "ALR Own Row";
    begin
        R."No." := 'O1';
        GlobalLanguage(1030);
        Error('%1', Format(R.RecordId(), 0, 9));
    end;

    [Test]
    procedure Format9OfATableWhoseCaptionAnExtensionGivesUnderDanish()
    var
        R: Record "ALR Captionless";
    begin
        R."No." := 'C1';
        GlobalLanguage(1030);
        Error('%1', Format(R.RecordId(), 0, 9));
    end;

    [Test]
    procedure Format9OfAnExtendedTableOutsideANamespaceUnderDanish()
    var
        R: Record "ALR Bare Row";
    begin
        R."No." := 'B1';
        GlobalLanguage(1030);
        Error('%1', Format(R.RecordId(), 0, 9));
    end;

    [Test]
    procedure Format1OfTheExtendedTableUnderDanish()
    var
        R: Record "ALR Unit";
    begin
        R.Code := 'U1';
        GlobalLanguage(1030);
        Error('%1', Format(R.RecordId(), 0, 1));
    end;
}
"#;

fn app(id: &str, deps: &[&str], src: &str) -> AppSource {
    AppSource {
        id: id.into(),
        name: id.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![(format!("{id}.al"), src.to_string())],
    }
}

fn manifest(files: Vec<al_ir::TranslationFile>) -> Manifest {
    Manifest {
        publisher: "AL Harness".into(),
        version: "1.0.0.0".into(),
        translations: !files.is_empty(),
        translation_files: files,
        ..Default::default()
    }
}

/// The extension app's da-DK file: one unit, for the field its table extension adds.
fn danish() -> al_ir::TranslationFile {
    let id = [
        ("TableExtension", "ALR Unit Ext"),
        ("Field", "Note"),
        ("Property", "Caption"),
    ]
    .iter()
    .map(|(k, n)| al_ir::xliff_part(k, n))
    .collect::<Vec<_>>()
    .join(" - ");
    al_ir::TranslationFile {
        name: "Translations/Ext.da-DK.xlf".into(),
        target_language: "da-DK".into(),
        units: vec![(id, "Bemærkning".to_string())],
        error: None,
        states: Vec::new(),
    }
}

fn run() -> Vec<(String, Outcome, Vec<Assumption>)> {
    let rt = CorpusRuntime::try_run_suites_with(
        vec![
            app("tests", &["ext", "base", "bare"], TESTS),
            app("ext", &["base", "bare"], EXT),
            app("base", &[], BASE),
            app("bare", &[], BARE),
        ],
        vec![al_sema::AppKind::Workspace; 4],
        &[
            manifest(Vec::new()),
            manifest(vec![danish()]),
            manifest(Vec::new()),
            manifest(Vec::new()),
        ],
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

fn find<'a>(got: &'a [(String, Outcome, Vec<Assumption>)], test: &str) -> &'a (String, Outcome, Vec<Assumption>) {
    got.iter()
        .find(|(t, ..)| t == test)
        .unwrap_or_else(|| panic!("no test {test}: {got:?}"))
}

fn ids(assumed: &[Assumption]) -> Vec<&'static str> {
    assumed.iter().map(|a| a.info().id).collect()
}

/// The issue's repro: format 9 of the extended table gives the same bracketed name under 1030 as under 1033, flagged.
#[test]
fn format_9_of_a_table_an_app_with_a_translation_file_extends_runs_flagged() {
    let got = run();
    let (_, outcome, assumed) = find(&got, "Format9OfTheExtendedTableUnderDanish");
    assert!(
        matches!(outcome, Outcome::Fail(m) if m == "[ALR.Base.ALR Unit]: U1|[ALR.Base.ALR Unit]: U1"),
        "{outcome:?}"
    );
    assert_eq!(ids(assumed), ["recordid-format-nine-extended-table"]);
}

/// A table no app with a translation file extends: the gate never refused it, and the pass stays clean.
#[test]
fn format_9_of_a_table_not_extended_stays_a_clean_run() {
    let got = run();
    let (_, outcome, assumed) = find(&got, "Format9OfATableNotExtendedUnderDanish");
    assert!(
        matches!(outcome, Outcome::Fail(m) if m == "[ALR.Base.ALR Movement Line]: M1"),
        "{outcome:?}"
    );
    assert!(assumed.is_empty(), "{assumed:?}");
}

/// Format 1 shows the caption, which the gate still refuses for the extended table.
#[test]
fn format_1_of_the_extended_table_stays_refused() {
    let got = run();
    let (_, outcome, _) = find(&got, "Format1OfTheExtendedTableUnderDanish");
    assert!(
        matches!(outcome, Outcome::NotRun(m) if m.contains("standard format 1 where the table caption is not known")),
        "{outcome:?}"
    );
}

/// A captioned table of the translating app itself, which that app also extends: its own file could translate the caption,
/// a shape no probe measured, so format 9 stays refused.
#[test]
fn format_9_of_an_own_table_of_the_translating_app_stays_refused() {
    let got = run();
    let (_, outcome, _) = find(&got, "Format9OfAnOwnTableOfTheTranslatingAppUnderDanish");
    assert!(
        matches!(outcome, Outcome::NotRun(m) if m.contains("standard format 9 where the table caption is not known")),
        "{outcome:?}"
    );
}

/// A table that declares no Caption and gets one from a table extension (lowering merges it into the table's properties):
/// the probe measured only tables with their own Caption, so format 9 stays refused.
#[test]
fn format_9_of_a_table_whose_caption_a_table_extension_gives_stays_refused() {
    let got = run();
    let (_, outcome, _) = find(&got, "Format9OfATableWhoseCaptionAnExtensionGivesUnderDanish");
    assert!(
        matches!(outcome, Outcome::NotRun(m) if m.contains("standard format 9 where the table caption is not known")),
        "{outcome:?}"
    );
}

/// An extended table outside any namespace: both extended tables the probe measured are in a namespace, so format 9 stays
/// refused.
#[test]
fn format_9_of_an_extended_table_outside_a_namespace_stays_refused() {
    let got = run();
    let (_, outcome, _) = find(&got, "Format9OfAnExtendedTableOutsideANamespaceUnderDanish");
    assert!(
        matches!(outcome, Outcome::NotRun(m) if m.contains("standard format 9 where the table caption is not known")),
        "{outcome:?}"
    );
}
