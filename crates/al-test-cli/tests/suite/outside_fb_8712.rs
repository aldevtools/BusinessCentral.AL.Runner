//! The corpus probe of al-runner issues 8712 and 8713 (corpus pull request 1521, open: tests/al-language/pages/currpage/
//! ListNewLineExtraCode.al, measured on BC 29.1, 11 of 11 green in 3 runs), run here until a bump pins it: a List page without
//! triggers whose new line the table field's OnValidate inserts, with extra code around `CurrPage.SaveRecord()` and
//! `CurrPage.Update` in the control's OnValidate (`al-vm` pages/code_inserted_extra.rs). The probe is read from its copy in the
//! neighbor app. Delete this file when the pin is in the corpus.

use al_builtins::assumptions::Assumption::{self, ListNewLineExtraCode};
use al_test_cli::report::Outcome;

/// The corpus probe, whole, as the neighbor app holds it (its first line says so).
const PROBE: &str = include_str!("../../../../tests/neighbors/8712-new-line-extra-code/src/Probe.al");

/// The neighbors just past the cells, each with BC's answer (3 runs of 3 on BC 29.1).
const NEIGHBORS: &str = include_str!("../../../../tests/neighbors/8712-new-line-extra-code/src/Neighbors.al");

fn outcomes(src: &str, codeunit: &str) -> Vec<(String, Outcome, Vec<Assumption>)> {
    let l = al_testkit::compile_apps(vec![al_testkit::app(src)]);
    let engine = al_vm::Engine::new(l.program.clone(), al_testkit::blocked(&l));
    let oi = l
        .program
        .objects
        .iter()
        .position(|o| o.name == codeunit)
        .expect("the codeunit");
    let mut hits = Vec::new();
    let outcomes = al_testrun::runtime::run_codeunit_full(
        &engine,
        &l,
        oi,
        None,
        al_testrun::isolation::TestIsolation::Codeunit,
        None,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut hits,
        None,
    );
    assert!(!outcomes.is_empty());
    outcomes
        .into_iter()
        .map(|(n, o)| {
            let marks = hits
                .iter()
                .find(|(h, _)| *h == n)
                .map(|(_, m)| m.clone())
                .unwrap_or_default();
            (n, o, marks)
        })
        .collect()
}

/// The measured cells pass, flagged by list-new-line-extra-code: the repro of 8712 (a modify through another record between
/// the save and `Update(false)`) on an empty group and with a row before; the repro of 8713 (a field set before
/// the save) on an empty group and with a row before; a field the table field sets; `Update(true)` after a field set; a field set
/// after the save, which `Update(false)` drops; a write of another table after the save.
#[test]
fn issue_8712_probe_cells() {
    let o = outcomes(PROBE, "LT NE Tests");
    for name in [
        "NewLine_SaveRecordThenModifyThroughAnotherRecord_ShowsAndSavesIt",
        "NewLine_SaveRecordThenModifyThroughAnotherRecord_RowBefore_ShowsAndSavesIt",
        "NewLine_FieldSetBeforeSaveRecord_ShowsAndSavesIt",
        "NewLine_FieldSetBeforeSaveRecord_RowBefore_ShowsAndSavesIt",
        "NewLine_TableFieldSetsFieldBeforeSaveRecord_ShowsAndSavesIt",
        "NewLine_FieldSetThenUpdateTrue_ShowsAndSavesIt",
        "NewLine_FieldSetAfterSaveRecord_UpdateFalseReadsTheStoredRow",
        "NewLine_SaveRecordThenWriteAnotherTable_ShowsAndSavesTheRow",
    ] {
        let (_, r, marks) = o.iter().find(|(n, _, _)| n == name).expect("the test");
        assert_eq!(*r, Outcome::Pass, "{name}");
        assert!(marks.contains(&ListNewLineExtraCode), "{name}: {marks:?}");
    }
    // A write of the row through another record before the save, and Update(true) after the save, are measured (the save
    // writes Rec over the other record's write) but not run: refused. So is OpenEdit with a filter, which no pinned cell opens.
    for name in [
        "NewLine_SaveRecordThenModifyThroughAnotherRecord_OpenEdit_ShowsAndSavesIt",
        "NewLine_ModifyThroughAnotherRecordBeforeSaveRecord_SaveWritesRecOverIt",
        "NewLine_SaveRecordModifyThroughAnotherRecordThenUpdateTrue_SaveWritesRecOverIt",
    ] {
        let (_, r, _) = o.iter().find(|(n, _, _)| n == name).expect("the test");
        assert!(matches!(r, Outcome::NotRun(_)), "{name}: {r:?}");
    }
}

/// Code just past the cells is refused, also where BC's answer follows the same rule: the row and Rec both changed between the
/// save and `Update(false)`, another row of the page's table changed, Rec's view changed, a write of another table before the
/// save (by the table field or the control), `Update(true)` with a row before, and a page with a trigger.
#[test]
fn issue_8712_neighbors_are_refused() {
    let o = outcomes(NEIGHBORS, "LT NN Tests");
    assert_eq!(o.len(), 7);
    for (name, r, _) in &o {
        assert!(matches!(r, Outcome::NotRun(_)), "{name}: {r:?}");
    }
}
