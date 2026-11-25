//! The corpus probe of al-runner issue 8842 (corpus pull request 1541, open: tests/al-language/pages/currpage/
//! ListNewLineExtraCodeCodedPageSubscriber.al, measured on BC 29.1, 4 of 4 green in 3 runs), run here until a bump pins it: the
//! page of the page-with-code shape (`al-vm` pages/code_inserted_coded.rs `CodedObjects::Extra`) with a manual subscriber to the
//! page. The probe is read from its copy in the neighbor app. Delete this file when the pin is in the corpus.

use al_builtins::assumptions::Assumption::{ListNewLineExtraCode, ListNewLineExtraCodeCoded};
use al_test_cli::report::Outcome;

/// The probe, whole, as the neighbor app holds it (its first line says so).
const PROBE: &str =
    include_str!("../../../../tests/neighbors/8842-coded-page-manual-subscriber/src/ProbePageSubscriber.al");

/// The neighbors just past the cells: two hold BC's answer (3 runs of 3 on BC 29.1), and four cover the review's refusal
/// boundary.
const NEIGHBORS: &str = include_str!("../../../../tests/neighbors/8842-coded-page-manual-subscriber/src/Neighbors.al");

/// A manual subscriber to the page that is not bound when the page opens runs nothing: the repro of 8842 and the one bound and
/// unbound before the open pass, flagged. One bound for the open, and an automatic one, run at each modify of the page: they
/// keep main's refusal.
#[test]
fn issue_8842_probe_cells() {
    let o = super::outside_fb_8712::outcomes(PROBE, "LT NU4 Tests");
    assert_eq!(o.len(), 4);
    for (name, r, marks) in &o {
        match name.as_str() {
            "NewLine_BoundPageSubscriber_RunsAtEachModifyOfThePage"
            | "NewLine_AutomaticPageSubscriber_RunsAtEachModifyOfThePage" => {
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

/// An unbound manual subscriber to another event of the page passes, flagged. A subscriber bound after the open keeps main's
/// outcome when the step ends or cannot land; a landing call, or SaveRecord/Update after the binding, is refused. The neighbors
/// use the probe's table and parts, so they compile with it.
#[test]
fn issue_8842_neighbors() {
    let src = format!("{PROBE}\n{NEIGHBORS}");
    let o = super::outside_fb_8712::outcomes(&src, "LT NU5 Tests");
    assert_eq!(o.len(), 3);
    for (name, r, marks) in &o {
        match name.as_str() {
            "NewLine_SubscriberBoundThenOnlyOk_KeepsMain" => {
                assert_eq!(*r, Outcome::Pass, "{name}");
                assert!(marks.contains(&ListNewLineExtraCodeCoded), "{name}: {marks:?}");
            }
            "NewLine_SubscriberBoundThenUnlandingWrite_KeepsMain" => assert!(
                matches!(r, Outcome::NotRun(w) if !w.contains("while a subscriber of the page is bound")),
                "{name}: {r:?}"
            ),
            _ => {
                assert_eq!(*r, Outcome::Pass, "{name}");
                assert!(marks.contains(&ListNewLineExtraCodeCoded), "{name}: {marks:?}");
            }
        }
    }
    for codeunit in [
        "LT NU5 Bound Call Tests",
        "LT NU5 Bound Before Save Tests",
        "LT NU5 Bound After Save Tests",
    ] {
        let o = super::outside_fb_8712::outcomes(&src, codeunit);
        assert_eq!(o.len(), 1, "{codeunit}");
        let (name, r, _) = &o[0];
        assert!(
            matches!(r, Outcome::NotRun(w) if w.contains("while a subscriber of the page is bound")),
            "{codeunit}: {name}: {r:?}"
        );
    }
}
