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

/// Every cell of the probe passes under the entry; the cells whose filter drops the row are al-runner issue 8846's
/// (`tree_dropped_row_8846.rs`).
#[test]
fn probe_cells_pass_on_the_tree_pages() {
    let o = outcomes(PROBE, "LT TVF Tests");
    assert_eq!(o.len(), 36);
    let bad: Vec<_> = o
        .iter()
        .filter(|(n, ..)| !super::tree_dropped_row_8846::DROP_CELLS.contains(&n.as_str()))
        .filter(|(_, r, marks)| *r != Outcome::Pass || *marks != [TreeValidateViewRefresh])
        .collect();
    assert!(bad.is_empty(), "{bad:#?}");
}

/// The neighbors past the cells: a second SetValue on Name, which no cell makes, refuses once the rule ran; a SetValue on Name
/// from the first row over the tree's rows, which no cell of page 97720 measures (its drop cell from the first row has rows none
/// indented, :1300), keeps main's path (BC raises a validation error, main raises none).
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

/// Page 97720 over rows the cells do not hold, by equality of the Indentation of each row (0, 1, 1 and 0, 1, 2 with Last on the
/// first row, 0, 0, 2, four rows), and Expand(false) after Last or Expand(true) on the first row: main's path, no entry.
#[test]
fn shapes_past_the_cells_keep_mains_path() {
    let o = outcomes(&format!("{PROBE}\n{NEIGHBORS}"), "LT TVN Shape Tests");
    assert!(o.len() == 6 && o.iter().all(|(_, _, marks)| marks.is_empty()), "{o:#?}");
}
