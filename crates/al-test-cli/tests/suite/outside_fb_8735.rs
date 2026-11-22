//! The corpus probe of al-runner issue 8735 (corpus pull request 1527, open: tests/al-language/pages/currpage/
//! ListNewLineWritesOtherTable.al, measured on BC 29.1, 11 of 11 green in 3 runs), run here until a bump pins it: writes of
//! another table before `CurrPage.SaveRecord()` or `CurrPage.Update(true)` on the new line the table field's OnValidate inserted,
//! on a List page without triggers (`al-vm` pages/code_inserted_extra.rs, `only_other_tables_written`). The probe is read from
//! its copy in the neighbor app. Delete this file when the pin is in the corpus.

use al_builtins::assumptions::Assumption::{ListNewLineExtraCode, ListNewLineWritesOtherTable};
use al_test_cli::report::Outcome;

/// The corpus probe, whole, as the neighbor app holds it (its first line says so).
const PROBE: &str = include_str!("../../../../tests/neighbors/8735-new-line-writes-other-table/src/Probe.al");

/// The neighbors just past the cells, each with BC's answer (3 runs of 3 on BC 29.1).
const NEIGHBORS: &str = include_str!("../../../../tests/neighbors/8735-new-line-writes-other-table/src/Neighbors.al");

/// The measured cells pass, flagged by both assumptions: the repro of 8735 (the table field inserts two rows of another table)
/// on an empty group and with a row before; the control inserts a row of another table, on both; the table field inserts one
/// before `Update(true)`; the table field modifies them, on both; the control modifies them; and the table field inserts one
/// before the control writes the line through another record (the rewritten row). A write before the insert and a delete keep
/// the refusal: the store does not tell which table they went to.
#[test]
fn issue_8735_probe_cells() {
    let o = super::outside_fb_8712::outcomes(PROBE, "LT NW Tests");
    assert_eq!(o.len(), 11);
    for (name, r, marks) in &o {
        let refused = [
            "NewLine_TableFieldWritesAnotherTableBeforeTheInsert_ShowsAndSavesTheRow",
            "NewLine_TableFieldDeletesAnotherTablesRowsBeforeSaveRecord_ShowsAndSavesTheRow",
        ]
        .contains(&name.as_str());
        match refused {
            true => assert!(matches!(r, Outcome::NotRun(_)), "{name}: {r:?}"),
            false => {
                assert_eq!(*r, Outcome::Pass, "{name}");
                assert!(
                    marks.contains(&ListNewLineWritesOtherTable) && marks.contains(&ListNewLineExtraCode),
                    "{name}: {marks:?}"
                );
            }
        }
    }
}

/// Writes of another table just past the cells keep the refusal: under TestPage.OpenEdit with a filter, before `Update(true)` with
/// a row before, with a second row of the page's table inserted, with the row before modified, and with the
/// line modified again through Rec.
#[test]
fn issue_8735_neighbors_are_refused() {
    let o = super::outside_fb_8712::outcomes(NEIGHBORS, "LT NX Tests");
    assert_eq!(o.len(), 5);
    let ran: Vec<_> = o
        .iter()
        .filter(|(_, r, _)| !matches!(r, Outcome::NotRun(_)))
        .map(|(name, r, _)| format!("{name}: {r:?}"))
        .collect();
    assert!(ran.is_empty(), "{ran:#?}");
}
