//! Al-runner issue 8832: `TestPage.GetField` and `TestPart.GetField` take the id the compiler gives a page control (al-vm
//! pages/get_field.rs, assumption testpage-getfield-control-id). The reporter's repro, the cells of corpus pull request 1538
//! (testpage/GetFieldByControlId.al) and the neighbors past them, as the neighbor app tests/neighbors/8832-getfield-control-id
//! holds them.

use super::outside_fb_repros::assert_stub_outcomes as outcomes;
use al_builtins::assumptions::Assumption::TestPageGetFieldControlId;
use al_test_cli::report::Outcome;

const PROBE: &str = include_str!("../../../../tests/neighbors/8832-getfield-control-id/src/Probe.al");
const NEIGHBORS: &str = include_str!("../../../../tests/neighbors/8832-getfield-control-id/src/Neighbors.al");

/// The reporter's test passes: GetField with the Name control's id reads the row's name. Main answered "The field with ID =
/// 356257777 is not found on the page."
#[test]
fn issue_8832_repro_passes() {
    let src = format!(
        "{}\n{}",
        include_str!("../../../../tests/neighbors/8832-getfield-control-id/src/Repro.al"),
        include_str!("../../../../tests/neighbors/8832-getfield-control-id/src/ReproTests.al")
    );
    let o = outcomes(&src, "Repro GetField Tests");
    assert!(
        o.len() == 1 && o[0].1 == Outcome::Pass && o[0].2 == [TestPageGetFieldControlId],
        "{o:#?}"
    );
}

/// Every cell of the probe passes under the entry where GetField found a control, and without it where every id is one main
/// already did not find (:484 a name of no control, :588 a page extension's field). A field in a cuegroup (:646) and Caption
/// of what a part control's id gives (:685) refuse.
#[test]
fn probe_cells_pass_where_measured() {
    let o = outcomes(PROBE, "LT GFI Tests");
    assert_eq!(o.len(), 14);
    let bad: Vec<_> = o
        .iter()
        .filter(|(n, r, marks)| match n.as_str() {
            "List_IdsOfNoField" | "Card_ExtensionControl" => *r != Outcome::Pass || !marks.is_empty(),
            "CueCard_CueField" => !matches!(r, Outcome::NotRun(why) if why.contains("a field in a cuegroup")),
            "Card_PartIdCaptionAndVisible" => {
                !matches!(r, Outcome::NotRun(why) if why.starts_with("TestField.Caption on what TestPage.GetField found"))
            }
            _ => *r != Outcome::Pass || *marks != [TestPageGetFieldControlId],
        })
        .collect();
    assert!(bad.is_empty(), "{bad:#?}");
}

/// The neighbors past the cells: Activate on what GetField found, which no cell calls, refuses, and so does Caption of what the
/// Card's part control id gives.
#[test]
fn neighbors_past_the_cells_refuse() {
    let o = outcomes(&format!("{PROBE}\n{NEIGHBORS}"), "LT GFN Tests");
    let bad: Vec<_> = o
        .iter()
        .filter(|(_, r, _)| !matches!(r, Outcome::NotRun(why) if why.contains("on what TestPage.GetField found by a control id")))
        .collect();
    assert!(o.len() == 2 && bad.is_empty(), "{o:#?}");
}
