//! The corpus probe of al-runner issue 8843 (corpus pull request 1541, open: tests/al-language/pages/currpage/
//! ListNewLineExtraCodeCodedSecondSetValue.al, measured on BC 29.1, 12 of 12 green in 3 runs), run here until a bump pins it: on
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

/// The neighbors of the review of this pull request (01:00Z), each in its own codeunit with BC's answer (3 runs of 3 on BC 29.1).
const REVIEW: &str = include_str!("../../../../tests/neighbors/8843-coded-page-second-setvalue/src/ReviewNeighbors.al");

/// The cells the effect gate takes pass, flagged: the repro of 8843, the save and the update only, another table written
/// before the save, the row modified through another record after it, Update(true) after the save, the control with a
/// TableRelation, and the row written by the test between the two SetValues (:871). A filter set before the update,
/// Update(true) without a save, a save without an update, a write after the update and Rec.Modify() before it keep a refusal.
#[test]
fn issue_8843_probe_cells() {
    let o = super::outside_fb_8712::outcomes(PROBE, "LT NV3 Tests");
    assert_eq!(o.len(), 12);
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

/// A stored related value typed as stored ('V', as cell :859) and the row written by the test between the two SetValues run,
/// flagged. A control without code with a TableRelation keeps main's way, unflagged. A related value missing ('W') or in another
/// case ('v'), a field of Rec set after the save, Update(true) after a modify or a field set, a filter set before the save,
/// and Rec moved off the landed row before it are refused; so are a row of another table modified or deleted between the save
/// and the Update, and a relation to a Code key (review of 02:55Z).
#[test]
fn issue_8847_review_neighbors() {
    let src = format!("{PROBE}\n{REVIEW}");
    let codeunits: Vec<&str> = REVIEW
        .lines()
        .filter_map(|l| {
            l.strip_prefix("codeunit ")?
                .split_once(' ')?
                .1
                .strip_prefix('"')?
                .strip_suffix('"')
        })
        .filter(|n| n.ends_with(" Tests"))
        .collect();
    assert_eq!(codeunits.len(), 13);
    for cu in codeunits {
        let o = super::outside_fb_8712::outcomes(&src, cu);
        assert_eq!(o.len(), 1, "{cu}");
        let (name, r, marks) = &o[0];
        match cu {
            "LT NV5 Rel Stored Tests" | "LT NV5 Between Tests" => {
                assert_eq!(*r, Outcome::Pass, "{name}");
                assert!(marks.contains(&ListNewLineExtraCodeCoded), "{name}: {marks:?}");
            }
            "LT NV5 Plain Rel Tests" => assert!(!marks.contains(&ListNewLineExtraCodeCoded), "{name}: {marks:?}"),
            _ => assert!(matches!(r, Outcome::NotRun(_)), "{name}: {r:?}"),
        }
    }
}
