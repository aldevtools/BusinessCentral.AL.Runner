//! Al-runner issue 8846: a control's OnValidate whose filter leaves the row out of the view, then CurrPage.Update(false), on a
//! tree page (ShowAsTree) and on the List of the same code (al-vm pages/validate_view_refresh.rs, assumption
//! tree-validate-view-refresh). The reporter's repro, the drop cells of corpus pull request 1539
//! (pages/currpage/TreeValidateFilterRefresh.al) and the neighbors past them, as the neighbor apps
//! tests/neighbors/8846-tree-dropped-row and 8833-tree-update-false-refresh hold them.

use super::outside_fb_repros::assert_stub_outcomes as outcomes;
use al_builtins::assumptions::Assumption::TreeValidateViewRefresh;
use al_test_cli::report::Outcome;

const PROBE: &str = include_str!("../../../../tests/neighbors/8833-tree-update-false-refresh/src/Probe.al");

/// The probe's tests of a filter that leaves the row out (:1146, :1246-:1324, :1347-:1390).
pub(super) const DROP_CELLS: [&str; 10] = [
    "Tree_SetRangeDropsUpdateFalse_Fails",
    "List_FirstRowSetRangeDropsUpdateFalse",
    "List_MiddleRowSetRangeDropsUpdateFalse",
    "List_LastRowSetRangeDropsUpdateFalse",
    "FlatTree_FirstRowSetRangeDropsUpdateFalse_Fails",
    "FlatTree_MiddleRowSetRangeDropsUpdateFalse_Fails",
    "FlatTree_LastRowSetRangeDropsUpdateFalse_Fails",
    "IssueTree_FirstRowSetRangeDropsUpdateFalse_Fails",
    "DropTree_FirstRowSetRangeDropsUpdateFalse_Fails",
    "DropTree_FirstRowSetRangeDropsUpdateFalse_AssertError",
];

/// The reporter's test passes: the SetValue on the tree's first row raises the filtered-view error, and the typed row stays
/// saved. Main raised no error.
#[test]
fn issue_8846_repro_passes() {
    let src = format!(
        "{}\n{}",
        include_str!("../../../../tests/neighbors/8846-tree-dropped-row/src/Repro.al"),
        include_str!("../../../../tests/neighbors/8846-tree-dropped-row/src/ReproTests.al")
    );
    let o = outcomes(&src, "Repro Tree Drop Tests");
    assert!(
        o.len() == 1 && o[0].1 == Outcome::Pass && o[0].2 == [TreeValidateViewRefresh],
        "{o:#?}"
    );
}

/// Every drop cell passes under the entry: on the List the refresh settles on the next row of the view, else the one before
/// it; on the tree the same reads, the row saved, then the filtered-view error.
#[test]
fn drop_cells_pass() {
    let o = outcomes(PROBE, "LT TVF Tests");
    let drops: Vec<_> = o.iter().filter(|(n, ..)| DROP_CELLS.contains(&n.as_str())).collect();
    let bad: Vec<_> = drops
        .iter()
        .filter(|(_, r, marks)| *r != Outcome::Pass || *marks != [TreeValidateViewRefresh])
        .collect();
    assert!(drops.len() == DROP_CELLS.len() && bad.is_empty(), "{drops:#?}");
}

/// The neighbors past the cells keep main's path, each measured on BC: the repro's tree with Update(true) passes (no error, the
/// row saved); with CollapseAll and with OnAfterGetCurrRecord main raises no error and saves nothing, where BC raises the
/// filtered-view error and saves the row; the repro's page over rows that the filter leaves none of refuses.
#[test]
fn neighbors_past_the_cells_keep_mains_path() {
    let src = format!(
        "{}\n{}",
        include_str!("../../../../tests/neighbors/8846-tree-dropped-row/src/Repro.al"),
        include_str!("../../../../tests/neighbors/8846-tree-dropped-row/src/Neighbors.al")
    );
    let o = outcomes(&src, "LT TDN Tests");
    let bad: Vec<_> = o
        .iter()
        .filter(|(n, r, marks)| {
            !marks.is_empty()
                || !match n.as_str() {
                    "DropTreeTrue_FirstRow" => *r == Outcome::Pass,
                    "DropTreeCollapsed_FirstRow" | "DropTreeCurr_FirstRow" => matches!(r, Outcome::Fail(_)),
                    "DropTree_NoRowLeft" => {
                        matches!(r, Outcome::NotRun(why) if why.starts_with("CurrPage.Update on a list whose filters left the row"))
                    }
                    _ => false,
                }
        })
        .collect();
    assert!(o.len() == 4 && bad.is_empty(), "{o:#?}");
}
