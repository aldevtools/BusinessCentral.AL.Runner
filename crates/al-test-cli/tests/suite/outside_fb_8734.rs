//! The corpus probe of al-runner issue 8734 (corpus pull request 1532, open: tests/al-language/pages/currpage/
//! ListNewLineExtraCodeFactbox.al, measured on BC 29.1, 10 of 10 green in 3 runs), run here until a bump pins it: extra code
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
/// after the save and a field set before it, each on an empty group and with a row before.
#[test]
fn issue_8734_probe_cells() {
    let o = super::outside_fb_8712::outcomes(PROBE, "LT NK Tests");
    assert_eq!(o.len(), 10);
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
/// and a page without OnAfterGetCurrRecord.
#[test]
fn issue_8734_neighbors_are_refused() {
    let o = super::outside_fb_8712::outcomes(NEIGHBORS, "LT NG Tests");
    assert_eq!(o.len(), 5);
    let ran: Vec<_> = o
        .iter()
        .filter(|(_, r, _)| !matches!(r, Outcome::NotRun(_)))
        .map(|(name, r, _)| format!("{name}: {r:?}"))
        .collect();
    assert!(ran.is_empty(), "{ran:#?}");
}
