//! Al-runner issue 8845: the Value of a field control in a cuegroup (al-vm pages/cue_value.rs, assumption cuegroup-field-value).
//! The reporter's repro, the cue cells of corpus pull request 1538 (testpage/GetFieldByControlId.al) and the neighbors past them,
//! as the neighbor apps tests/neighbors/8845-cue-field-value and 8832-getfield-control-id hold them.

use super::outside_fb_repros::assert_stub_outcomes as outcomes;
use al_builtins::assumptions::Assumption::{CuegroupFieldValue, TestPageGetFieldControlId};
use al_test_cli::report::Outcome;

const PROBE: &str = include_str!("../../../../tests/neighbors/8832-getfield-control-id/src/Probe.al");

/// The reporter's test passes: the Text field in the cuegroup reads `0`, the same field in a group its text. Main read the text.
/// A Text in a cuegroup is pinned (testpage/ABTests.al:747), so the pass is clean.
#[test]
fn issue_8845_repro_passes() {
    let src = format!(
        "{}\n{}",
        include_str!("../../../../tests/neighbors/8845-cue-field-value/src/Repro.al"),
        include_str!("../../../../tests/neighbors/8845-cue-field-value/src/ReproTests.al")
    );
    let o = outcomes(&src, "Repro Cue Tests");
    assert!(o.len() == 1 && o[0].1 == Outcome::Pass && o[0].2.is_empty(), "{o:#?}");
}

/// The cue cells pass: each type by its name and through GetField (:742), and the Text field of :653 through GetField and by
/// its name (a pinned Text, so only the GetField mark).
#[test]
fn cue_cells_pass() {
    let o = outcomes(PROBE, "LT GFI Tests");
    let bad: Vec<_> = o
        .iter()
        .filter(|(n, r, marks)| match n.as_str() {
            "CueTypes_ValueByNameAndById" => {
                *r != Outcome::Pass
                    || !(marks.len() == 2
                        && marks.contains(&CuegroupFieldValue)
                        && marks.contains(&TestPageGetFieldControlId))
            }
            "CueCard_CueField" => *r != Outcome::Pass || *marks != [TestPageGetFieldControlId],
            _ => false,
        })
        .collect();
    assert!(bad.is_empty(), "{bad:#?}");
}

/// The neighbors past the cells refuse: a DateFormula field in a cuegroup, a Boolean field in a wide one.
#[test]
fn neighbors_past_the_cells_refuse() {
    let o = outcomes(
        include_str!("../../../../tests/neighbors/8845-cue-field-value/src/Neighbors.al"),
        "LT CFN Tests",
    );
    let bad: Vec<_> = o
        .iter()
        .filter(
            |(_, r, _)| !matches!(r, Outcome::NotRun(why) if why.starts_with("Value of a field control in a cuegroup")),
        )
        .collect();
    assert!(o.len() == 2 && bad.is_empty(), "{o:#?}");
}
