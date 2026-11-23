//! Al-runner issue 8833: a control's OnValidate that calls CurrPage.Update on a tree page (ShowAsTree), with or without a filter
//! on Rec (al-vm pages/validate_view_refresh.rs, assumption tree-validate-view-refresh). The reporter's repro, the cells of
//! corpus pull request 1539 (pages/currpage/TreeValidateFilterRefresh.al) and the neighbors past them, as the neighbor app
//! tests/neighbors/8833-tree-update-false-refresh holds them.

use super::outside_fb_repros::assert_stub_outcomes as outcomes;
use al_builtins::assumptions::Assumption::TreeValidateViewRefresh;
use al_test_cli::report::Outcome;

const PROBE: &str = include_str!("../../../../tests/neighbors/8833-tree-update-false-refresh/src/Probe.al");
const NEIGHBORS: &str = include_str!("../../../../tests/neighbors/8833-tree-update-false-refresh/src/Neighbors.al");

/// The refusal of a call the cells do not make after the rule ran.
const UNMEASURED: &str =
    "a call on a tree page of TreeValidateFilterRefresh.al after the page read its row or the view again";

/// The reporter's test passes: the SetValue reads the row first, the refresh saves the row and reads it, the hidden child and
/// the first row of the new view, and the tree shows the renamed row alone. Main read only the typed row (`AGR(C2);`), saved
/// nothing and kept the rows the client held.
#[test]
fn issue_8833_repro_passes() {
    let src = format!(
        "{}\n{}",
        include_str!("../../../../tests/neighbors/8833-tree-update-false-refresh/src/Repro.al"),
        include_str!("../../../../tests/neighbors/8833-tree-update-false-refresh/src/ReproTests.al")
    );
    let o = outcomes(&src, "Repro Tree Tests");
    assert!(
        o.len() == 1 && o[0].1 == Outcome::Pass && o[0].2 == [TreeValidateViewRefresh],
        "{o:#?}"
    );
}

/// Every cell of the probe passes under the entry. The SetValue whose filter drops the row raises a validation error on BC
/// (:1146): it refuses once the rule read the row.
#[test]
fn probe_cells_pass_on_the_tree_pages() {
    let o = outcomes(PROBE, "LT TVF Tests");
    assert_eq!(o.len(), 21);
    let bad: Vec<_> = o
        .iter()
        .filter(|(n, r, marks)| match n.as_str() {
            "Tree_SetRangeDropsUpdateFalse_Fails" => !matches!(r, Outcome::NotRun(why) if why.starts_with(UNMEASURED)),
            _ => *r != Outcome::Pass || *marks != [TreeValidateViewRefresh],
        })
        .collect();
    assert!(bad.is_empty(), "{bad:#?}");
}

/// The neighbors past the cells: a second SetValue on Name, which no cell makes, refuses once the rule ran; a SetValue on Name
/// from the first row, which no cell starts with, keeps main's path (BC raises a validation error, main raises none).
#[test]
fn neighbors_past_the_cells() {
    let src = format!("{PROBE}\n{NEIGHBORS}");
    let twice = outcomes(&src, "LT TVN Tests");
    assert!(
        twice.len() == 1 && matches!(&twice[0].1, Outcome::NotRun(why) if why.starts_with(UNMEASURED)),
        "{twice:#?}"
    );
    let first = outcomes(&src, "LT TVN First Row Tests");
    assert!(
        first.len() == 1 && matches!(&first[0].1, Outcome::Fail(_)) && first[0].2.is_empty(),
        "{first:#?}"
    );
}
