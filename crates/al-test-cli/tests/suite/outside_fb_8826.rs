//! The corpus probe of al-runner issue 8826 (corpus pull request 1537, open: tests/al-language/pages/currpage/
//! ListNewLineExtraCodeCodedPage.al, measured on BC 29.1, 7 of 7 green in 3 runs), run here until a bump pins it: extra code
//! around `CurrPage.SaveRecord()` and `CurrPage.Update(false)` on the new line the table field's OnValidate inserted, on a List
//! page of the shape of the page with code (OnOpenPage, OnAfterGetRecord, OnDeleteRecord, actions, three CardPart FactBoxes, one
//! with a page background task; `al-vm` pages/code_inserted_coded.rs `CodedObjects::Extra`). The probe is read from its copy in
//! the neighbor app. Delete this file when the pin is in the corpus.

use al_builtins::assumptions::Assumption::{ListNewLineExtraCode, ListNewLineExtraCodeCoded};
use al_test_cli::report::Outcome;

/// The corpus probe, whole, as the neighbor app holds it (its first line says so).
const PROBE: &str = include_str!("../../../../tests/neighbors/8826-new-line-extra-code-coded-page/src/Probe.al");

/// The neighbors just past the cells, each with BC's answer (3 runs of 3 on BC 29.1).
const NEIGHBORS: &str =
    include_str!("../../../../tests/neighbors/8826-new-line-extra-code-coded-page/src/Neighbors.al");

/// The cells on an empty group pass, flagged by both assumptions: the repro of 8826, and the logging page (the page's and the
/// parts' triggers and the background task, step by step) with the repro's code, only the save, a modify through another
/// record after the save and a field set before it. A row before the new line and a table with OnInsert and OnModify are
/// measured too, and keep main's refusal.
#[test]
fn issue_8826_probe_cells() {
    let o = super::outside_fb_8712::outcomes(PROBE, "LT NZ Tests");
    assert_eq!(o.len(), 7);
    for (name, r, marks) in &o {
        let refused = [
            "NewLine_ExtraCodeOnPageWithCodeAndFactBoxes_RowBefore_LogsTheTriggers",
            "NewLine_ExtraCodeOnPageWithCodeAndFactBoxes_TableWithTriggers_LogsThem",
        ]
        .contains(&name.as_str());
        match refused {
            true => assert!(matches!(r, Outcome::NotRun(_)), "{name}: {r:?}"),
            false => {
                assert_eq!(*r, Outcome::Pass, "{name}");
                assert!(
                    marks.contains(&ListNewLineExtraCodeCoded) && marks.contains(&ListNewLineExtraCode),
                    "{name}: {marks:?}"
                );
            }
        }
    }
}

/// Code just past the cells keeps a refusal: a page that also declares OnAfterGetCurrRecord, a part whose OnAfterGetRecord
/// writes another table, a control that writes another table before the save, and a part that declares
/// OnPageBackgroundTaskError. The neighbors use the probe's table and parts, so they compile with it.
#[test]
fn issue_8826_neighbors_are_refused() {
    let o = super::outside_fb_8712::outcomes(&format!("{PROBE}\n{NEIGHBORS}"), "LT NG2 Tests");
    assert_eq!(o.len(), 4);
    let ran: Vec<_> = o
        .iter()
        .filter(|(_, r, _)| !matches!(r, Outcome::NotRun(_)))
        .map(|(name, r, _)| format!("{name}: {r:?}"))
        .collect();
    assert!(ran.is_empty(), "{ran:#?}");
}
