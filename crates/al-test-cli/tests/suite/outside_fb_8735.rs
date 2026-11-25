//! The corpus probe of al-runner issue 8735 (corpus pull request 1527, merged, and 1536, open, the review's cells:
//! tests/al-language/pages/currpage/ListNewLineWritesOtherTable.al, measured on BC 29.1, 19 of 19 green in 3 runs), run here until a bump pins it: writes of
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
/// before the control writes the line through another record (the rewritten row); and the review of #8806's cells: the control
/// inserts one, then `Update(true)`; the table field modifies them, then `Update(true)`; the table field and the control both
/// insert one before the save; a FlowField over the written table; a control that writes another table and rewrites the line,
/// in either order (the save writes Rec over the line); and the follow-ups: the control modifies another table, then
/// `Update(true)`; the table field and the control both insert one, then `Update(true)`. A write before the insert and a delete
/// keep the refusal: the store does not tell which table they went to.
#[test]
fn issue_8735_probe_cells() {
    let o = super::outside_fb_8712::outcomes(PROBE, "LT NW Tests");
    assert_eq!(o.len(), 19);
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
    for name in [
        "NewLine_ControlWritesAnotherTableAndTheLineBeforeSaveRecord_SaveWritesRecOverTheLine",
        "NewLine_ControlWritesTheLineThenAnotherTableBeforeSaveRecord_SaveWritesRecOverTheLine",
    ] {
        let (_, outcome, marks) = o.iter().find(|(n, _, _)| n == name).unwrap();
        assert_eq!(*outcome, Outcome::Pass, "{name}");
        assert!(
            marks.contains(&ListNewLineWritesOtherTable) && marks.contains(&ListNewLineExtraCode),
            "{name}: {marks:?}"
        );
    }
}

/// Writes of another table just past the cells keep the refusal: under TestPage.OpenEdit with a filter, before `Update(true)` with
/// a row before, with a second row of the page's table inserted, with the row before modified, and with the line modified again
/// through Rec. The two neighbors of the review of #8806 run as their cells do (:642, :666): the table field and the control both
/// writing another table before the save, and a FlowField over the written table. After the insert alone, the control rewriting
/// the line twice, with or without one insert of another table, keeps the refusal: the cells (:630, :654) measure one rewrite.
/// One rewrite next to a Modify of another table keeps it too, whether the control or the table field called Modify:
/// those cells measure at most one Insert there.
#[test]
fn issue_8735_neighbors_are_refused() {
    let o = super::outside_fb_8712::outcomes(NEIGHBORS, "LT NX Tests");
    assert_eq!(o.len(), 11);
    let run = [
        "NewLine_TableFieldAndControlWriteAnotherTable_ShowsAndSavesTheRow",
        "NewLine_FlowFieldOverTheWrittenTable_ShowsTheCount",
    ];
    let ran: Vec<_> = o
        .iter()
        .filter(|(name, r, marks)| match run.contains(&name.as_str()) {
            true => !(*r == Outcome::Pass && marks.contains(&ListNewLineWritesOtherTable)),
            false => !matches!(r, Outcome::NotRun(_)),
        })
        .map(|(name, r, _)| format!("{name}: {r:?}"))
        .collect();
    assert!(ran.is_empty(), "{ran:#?}");
}
