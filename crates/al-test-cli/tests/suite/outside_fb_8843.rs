//! The corpus probe of al-runner issue 8843 (corpus pull request 1541, open: tests/al-language/pages/currpage/
//! ListNewLineExtraCodeCodedSecondSetValue.al, measured on BC 29.1, 11 of 11 green in 3 runs), run here until a bump pins it: on
//! the page of the page-with-code shape (`al-vm` pages/code_inserted_coded.rs `CodedObjects::Extra`), after the new line lands, a
//! SetValue on a control with code of its own. The probe is read from its copy in the neighbor app. Delete this file when the
//! pin is in the corpus.

use al_builtins::assumptions::Assumption::{ListNewLineExtraCode, ListNewLineExtraCodeCoded};
use al_test_cli::report::Outcome;

/// The probe, whole, as the neighbor app holds it (its first line says so).
const PROBE: &str =
    include_str!("../../../../tests/neighbors/8843-coded-page-second-setvalue/src/ProbeSecondSetValue.al");

/// The neighbors just past the cells, each with BC's answer (3 runs of 3 on BC 29.1).
const NEIGHBORS: &str = include_str!("../../../../tests/neighbors/8843-coded-page-second-setvalue/src/Neighbors.al");

/// The cells the effect gate takes pass, flagged: the repro of 8843, the save and the update only, another table written
/// before the save, the row modified through another record after it, Update(true) after the save, and the control with a
/// TableRelation. A filter set before the update, Update(true) without a save, a save without an update, a write after the
/// update and Rec.Modify() before it keep a refusal.
#[test]
fn issue_8843_probe_cells() {
    let o = super::outside_fb_8712::outcomes(PROBE, "LT NV3 Tests");
    assert_eq!(o.len(), 11);
    for (name, r, marks) in &o {
        match name.as_str() {
            "NewLine_SecondSetValueSetsAFilterBeforeTheUpdate_ShowsTheRow"
            | "NewLine_SecondSetValueUpdateTrueWithoutSave_ShowsAndSavesTheRow"
            | "NewLine_SecondSetValueSavesWithoutUpdate_ShowsAndSavesTheRow"
            | "NewLine_SecondSetValueWritesAnotherTableAfterTheUpdate_ShowsAndSavesTheRow"
            | "NewLine_SecondSetValueModifiesRecThenUpdates_ShowsAndSavesTheRow" => {
                assert!(matches!(r, Outcome::NotRun(_)), "{name}: {r:?}")
            }
            _ => {
                assert_eq!(*r, Outcome::Pass, "{name}");
                assert!(
                    marks.contains(&ListNewLineExtraCodeCoded) && marks.contains(&ListNewLineExtraCode),
                    "{name}: {marks:?}"
                );
            }
        }
    }
}

/// The control inserting another row of the page's table before its save, a SetValue on the landing control again, and a
/// second SetValue with code keep a refusal. The neighbors use the probe's table, parts and variation page, so they compile
/// with it.
#[test]
fn issue_8843_neighbors_are_refused() {
    let o = super::outside_fb_8712::outcomes(&format!("{PROBE}\n{NEIGHBORS}"), "LT NV4 Tests");
    assert_eq!(o.len(), 3);
    let ran: Vec<_> = o
        .iter()
        .filter(|(_, r, _)| !matches!(r, Outcome::NotRun(_)))
        .map(|(name, r, _)| format!("{name}: {r:?}"))
        .collect();
    assert!(ran.is_empty(), "{ran:#?}");
}
