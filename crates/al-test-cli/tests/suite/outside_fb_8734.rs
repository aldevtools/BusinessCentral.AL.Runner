//! The corpus probe of al-runner issue 8734 (corpus pull request 1532, open: tests/al-language/pages/currpage/
//! ListNewLineExtraCodeFactbox.al, measured on BC 29.1, 18 of 18 green in 3 runs), run here until a bump pins it: extra code
//! around `CurrPage.SaveRecord()` and `CurrPage.Update(false)` on the new line the table field's OnValidate inserted, on a List
//! page with OnAfterGetRecord, OnAfterGetCurrRecord and a CardPart FactBox (`al-vm` pages/code_inserted_new_line.rs,
//! `Triggers::FactboxExtra`). The probe is read from its copy in the neighbor app. Delete this file when the pin is in the
//! corpus.

use al_builtins::assumptions::Assumption::{ListNewLineExtraCode, ListNewLineExtraCodeFactbox};
use al_test_cli::report::Outcome;

/// The corpus probe, whole, as the neighbor app holds it (its first line says so).
const PROBE: &str = include_str!("../../../../tests/neighbors/8734-new-line-extra-code-factbox/src/Probe.al");

/// The neighbors just past the cells, each with BC's answer (3 runs of 3 on BC 29.1).
const NEIGHBORS: &str = include_str!("../../../../tests/neighbors/8734-new-line-extra-code-factbox/src/Neighbors.al");

/// Every cell passes, flagged by both assumptions: the repro of 8734 on an empty group and with a row before, and the logging
/// page (the page's and the part's triggers, step by step) with the repro's code, only the save, a modify through another record
/// after the save and a field set before it, each on an empty group and with a row before; and after the save, a field of Rec
/// set, a row of another table inserted or one deleted, each on an empty group and with a row before, and the save without an
/// Update after it, on an empty group and with a row before (the codeunit after the tests, :664-762).
#[test]
fn issue_8734_probe_cells() {
    let mut o = super::outside_fb_8712::outcomes(PROBE, "LT NK Tests");
    assert_eq!(o.len(), 10);
    let more = super::outside_fb_8712::outcomes(PROBE, "LT NK More Tests");
    assert_eq!(more.len(), 8);
    o.extend(more);
    for (name, r, marks) in &o {
        assert_eq!(*r, Outcome::Pass, "{name}");
        assert!(
            marks.contains(&ListNewLineExtraCodeFactbox) && marks.contains(&ListNewLineExtraCode),
            "{name}: {marks:?}"
        );
    }
}

/// Code just past the cells keeps a refusal: the page's OnAfterGetRecord sets a field of Rec, the part's OnAfterGetRecord
/// writes another table, a modify of the row through another record before the save, a write of another table before the save,
/// a page without OnAfterGetCurrRecord, CurrPage.Update(true) or Update() without a save or Update(true) after it (only the
/// page without triggers measures Update(true) after the save, ListNewLineExtraCode.al:476), Previous() or Next() after the
/// landing, Previous(), Previous() then Next(), or Close() after a SetValue, Close() right after the landing,
/// OnAfterGetCurrRecord setting a field of the new line at New(), and OnAfterGetCurrRecord setting a filter on Rec in the
/// refresh. The save without an Update after it, with a row before, runs as the cells :701 and :713 measure it, flagged.
#[test]
fn issue_8734_neighbors_are_refused() {
    let mut o = super::outside_fb_8712::outcomes(NEIGHBORS, "LT NG Tests");
    assert_eq!(o.len(), 15);
    // Each in its own codeunit, so that no earlier refused test hides its result.
    for cu in ["LT NG Prev Next Tests", "LT NG Close Tests"] {
        let one = super::outside_fb_8712::outcomes(NEIGHBORS, cu);
        assert_eq!(one.len(), 1, "{cu}");
        o.extend(one);
    }
    let ran: Vec<_> = o
        .iter()
        .filter(|(name, r, marks)| match name.as_str() {
            "NewLine_SaveRecordWithoutUpdate_RowBefore_ShowsAndSavesTheRow" => {
                !(*r == Outcome::Pass && marks.contains(&ListNewLineExtraCodeFactbox))
            }
            _ => !matches!(r, Outcome::NotRun(_)),
        })
        .map(|(name, r, _)| format!("{name}: {r:?}"))
        .collect();
    assert!(ran.is_empty(), "{ran:#?}");
}
