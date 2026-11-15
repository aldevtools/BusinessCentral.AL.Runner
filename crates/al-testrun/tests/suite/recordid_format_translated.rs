//! `Format(RecordId, 0, 9)` and `Format(RecordId, 0, 1)` in a language other than 1033 when an app ships a translation file of
//! that language.
//!
//! al-runner issue 8123: format 9 names the table and never its caption, so the session language does not change it, also
//! when an app with a translation file for that language extends the table. Until the corpus pins it, the format runs
//! through the assumption `recordid-format-nine-extended-table`.
//!
//! al-runner issue 8135: format 1 names the table by its caption as the source declares it, never translated: the unit for
//! the table's own caption gives `TableCaption` its text, not format 1, and an app that extends the table leaves format 1 as
//! it is (corpus PR 1413). Until the corpus pins it, format 1 runs in those two shapes through the assumption
//! `recordid-format-one-source-caption`.

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

/// The app under test: it extends tables of the other apps and ships a da-DK file with units for its own field and for the
/// caption of its own table ALR Ns Gadget.
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
table 50841 "ALR Ns Gadget"
{
    Caption = 'Ns Gadget';
    fields { field(1; "No."; Code[20]) { Caption = 'No.'; } }
    keys { key(PK; "No.") { } }
}
"#;

/// An app outside any namespace that ships a da-DK file with a unit for its own table's caption, as the corpus app does for
/// LT TA Item.
const OWN: &str = r#"
table 50840 "ALR Own Gadget"
{
    Caption = 'Gadget';
    fields { field(1; "No."; Code[20]) { Caption = 'No.'; } }
    keys { key(PK; "No.") { } }
}
"#;

const TESTS: &str = r#"
namespace ALR.Tests;

using ALR.Base;
using ALR.Ext;

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

    [Test]
    procedure Format1OfAnOwnTableOfTheTranslatingAppUnderDanish()
    var
        R: Record "ALR Own Gadget";
    begin
        R."No." := 'G1';
        GlobalLanguage(1030);
        Error('%1', Format(R.RecordId(), 0, 1));
    end;

    [Test]
    procedure TableCaptionOfAnOwnTableOfTheTranslatingAppUnderDanish()
    var
        R: Record "ALR Own Gadget";
    begin
        GlobalLanguage(1030);
        Error('%1', R.TableCaption());
    end;

    [Test]
    procedure Format1OfAnOwnTableInANamespaceUnderDanish()
    var
        R: Record "ALR Ns Gadget";
    begin
        R."No." := 'N1';
        GlobalLanguage(1030);
        Error('%1', Format(R.RecordId(), 0, 1));
    end;

    [Test]
    procedure Format1OfATableWhoseCaptionAnExtensionGivesUnderDanish()
    var
        R: Record "ALR Captionless";
    begin
        R."No." := 'C1';
        GlobalLanguage(1030);
        Error('%1', Format(R.RecordId(), 0, 1));
    end;

    [Test]
    procedure Format1OfAnExtendedTableOutsideANamespaceUnderDanish()
    var
        R: Record "ALR Bare Row";
    begin
        R."No." := 'B1';
        GlobalLanguage(1030);
        Error('%1', Format(R.RecordId(), 0, 1));
    end;

    [Test]
    procedure Format1OfAnOwnTableTheTranslatingAppAlsoExtendsUnderDanish()
    var
        R: Record "ALR Own Row";
    begin
        R."No." := 'O1';
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

/// The XLIFF id of the parts `(kind, name)`.
fn unit_id(parts: &[(&str, &str)]) -> String {
    parts
        .iter()
        .map(|(k, n)| al_ir::xliff_part(k, n))
        .collect::<Vec<_>>()
        .join(" - ")
}

/// A da-DK file named `name` with `units` (XLIFF id, target).
fn danish(name: &str, units: Vec<(String, String)>) -> al_ir::TranslationFile {
    al_ir::TranslationFile {
        name: format!("Translations/{name}.da-DK.xlf"),
        target_language: "da-DK".into(),
        units,
        error: None,
        states: Vec::new(),
    }
}

/// The extension app's da-DK file: a unit for the field its table extension adds, and one for its own table's caption.
fn ext_danish() -> al_ir::TranslationFile {
    let note = unit_id(&[
        ("TableExtension", "ALR Unit Ext"),
        ("Field", "Note"),
        ("Property", "Caption"),
    ]);
    let gadget = unit_id(&[("Table", "ALR Ns Gadget"), ("Property", "Caption")]);
    danish("Ext", vec![(note, "Bemærkning".into()), (gadget, "Ns Dims".into())])
}

/// The own app's da-DK file: a unit for its table's caption.
fn own_danish() -> al_ir::TranslationFile {
    let gadget = unit_id(&[("Table", "ALR Own Gadget"), ("Property", "Caption")]);
    danish("Own", vec![(gadget, "Dims".into())])
}

fn run() -> Vec<(String, Outcome, Vec<Assumption>)> {
    let rt = CorpusRuntime::try_run_suites_with(
        vec![
            app("tests", &["ext", "base", "bare", "own"], TESTS),
            app("ext", &["base", "bare"], EXT),
            app("base", &[], BASE),
            app("bare", &[], BARE),
            app("own", &[], OWN),
        ],
        vec![al_sema::AppKind::Workspace; 5],
        &[
            manifest(Vec::new()),
            manifest(vec![ext_danish()]),
            manifest(Vec::new()),
            manifest(Vec::new()),
            manifest(vec![own_danish()]),
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

/// Format 1 of a table an app with a translation file extends names it by its source caption, flagged.
#[test]
fn format_1_of_a_table_an_app_with_a_translation_file_extends_gives_the_source_caption_flagged() {
    let got = run();
    let (_, outcome, assumed) = find(&got, "Format1OfTheExtendedTableUnderDanish");
    assert!(matches!(outcome, Outcome::Fail(m) if m == "Unit: U1"), "{outcome:?}");
    assert_eq!(ids(assumed), ["recordid-format-one-source-caption"]);
}

/// The repro of the format 1 issue: the translating app's own table, whose caption its file translates, is named by the
/// source caption, flagged.
#[test]
fn format_1_of_an_own_table_whose_caption_the_file_translates_gives_the_source_caption_flagged() {
    let got = run();
    let (_, outcome, assumed) = find(&got, "Format1OfAnOwnTableOfTheTranslatingAppUnderDanish");
    assert!(matches!(outcome, Outcome::Fail(m) if m == "Gadget: G1"), "{outcome:?}");
    assert_eq!(ids(assumed), ["recordid-format-one-source-caption"]);
}

/// TableCaption of the same table still reads the unit of the file, and needs no assumption.
#[test]
fn table_caption_of_that_own_table_stays_translated_and_clean() {
    let got = run();
    let (_, outcome, assumed) = find(&got, "TableCaptionOfAnOwnTableOfTheTranslatingAppUnderDanish");
    assert!(matches!(outcome, Outcome::Fail(m) if m == "Dims"), "{outcome:?}");
    assert!(assumed.is_empty(), "{assumed:?}");
}

/// Format 1 in the shapes the probe did not measure stays refused: an own table of a translating app in a namespace (LT TA
/// Item is outside one), a table whose Caption a table extension gives, an extended table outside a namespace (Item and
/// Sales Invoice Header are in one).
#[test]
fn format_1_in_unmeasured_shapes_stays_refused() {
    let got = run();
    for test in [
        "Format1OfAnOwnTableInANamespaceUnderDanish",
        "Format1OfATableWhoseCaptionAnExtensionGivesUnderDanish",
        "Format1OfAnExtendedTableOutsideANamespaceUnderDanish",
    ] {
        let (_, outcome, _) = find(&got, test);
        assert!(
            matches!(outcome, Outcome::NotRun(m) if m.contains("standard format 1 where the table caption is not known")),
            "{test}: {outcome:?}"
        );
    }
}

/// An own table of the translating app that the app also extends: a shape no probe measured, so format 1 stays refused.
#[test]
fn format_1_of_an_own_table_the_translating_app_also_extends_stays_refused() {
    let got = run();
    let (_, outcome, _) = find(&got, "Format1OfAnOwnTableTheTranslatingAppAlsoExtendsUnderDanish");
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
