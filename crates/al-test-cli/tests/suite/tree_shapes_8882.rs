//! Al-runner issue 8882: a control's OnValidate that calls CurrPage.Update(false) on a tree page (ShowAsTree), over rows and
//! with calls the cells of corpus pull request 1539 do not hold (al-vm pages/validate_view_refresh.rs, assumption
//! tree-validate-view-refresh). The cells of corpus pull request 1554 (pages/currpage/TreeValidateFilterShapes.al) and the
//! neighbors past them, as the neighbor app tests/neighbors/8882-tree-shapes holds them.

use super::outside_fb_repros::assert_stub_outcomes as outcomes;
use al_builtins::assumptions::Assumption::TreeValidateViewRefresh;
use al_test_cli::report::Outcome;

const PROBE: &str = include_str!("../../../../tests/neighbors/8882-tree-shapes/src/Probe.al");
const NEIGHBORS: &str = include_str!("../../../../tests/neighbors/8882-tree-shapes/src/Neighbors.al");

/// Every cell passes under the entry: the rows of the tree in other groups, indentations and numbers, Expand(false) after Last
/// and Expand(true) on the first row; the first row dropped on the four trees of 1539 and on page 97714 of 1539 with CollapseAll
/// and with OnAfterGetCurrRecord. Main read only the typed row, saved nothing and raised no error.
#[test]
fn shape_cells_pass() {
    let mut o = outcomes(PROBE, "LT TVS Tests");
    o.extend(outcomes(PROBE, "LT TVS Drop Tests"));
    let bad: Vec<_> = o
        .iter()
        .filter(|(_, r, marks)| *r != Outcome::Pass || *marks != [TreeValidateViewRefresh])
        .collect();
    assert!(o.len() == 12 && bad.is_empty(), "{o:#?}");
}

/// The neighbors past the cells keep main's path, each measured on BC: rows of one group under the first row, five rows, a
/// cell's rows on the page with both row triggers, Expand(true) on the first row there; the filter leaving the last of four
/// top-level rows out, or the first row whose children stay; Update(true) from the first row, which main refuses.
#[test]
fn neighbors_keep_mains_path() {
    let src = format!("{PROBE}\n{NEIGHBORS}");
    let keep = outcomes(&src, "LT TVS Neighbor Tests");
    assert!(
        keep.len() == 4
            && keep
                .iter()
                .all(|(_, r, marks)| matches!(r, Outcome::Fail(_)) && marks.is_empty()),
        "{keep:#?}"
    );
    let drop = outcomes(&src, "LT TVS Neighbor Drop Tests");
    let bad: Vec<_> = drop
        .iter()
        .filter(|(n, r, marks)| {
            !marks.is_empty()
                || !match n.as_str() {
                    "FourRows_LastRowDrops" | "FirstRowDrops_ChildrenStay" => matches!(r, Outcome::Fail(_)),
                    "FirstRowDrops_UpdateTrue" => {
                        matches!(r, Outcome::NotRun(why) if why.starts_with("CurrPage.Update on a list whose filters left the row"))
                    }
                    _ => false,
                }
        })
        .collect();
    assert!(drop.len() == 3 && bad.is_empty(), "{drop:#?}");
}

/// The reviews of 01:31Z and 04:05Z (`LT TVS Neighbor Path Tests`, each measured on BC): over rows 0, 0, 2 Last then a SetValue
/// on Value first, or on Name with Update(true); Last, Expand(false), then the SetValue on Name with Update(true); and a SetValue
/// on Value without First after a cell that kept the row. A cell over the rows starts with the steps so far, so the rule starts and
/// reads the row; the coming call is told from a cell only once the OnValidate ran, and each refuses. Main read only the typed row
/// where BC reads and saves more.
#[test]
fn calls_after_a_cells_prefix() {
    let src = format!("{PROBE}\n{NEIGHBORS}");
    let o = outcomes(&src, "LT TVS Neighbor Path Tests");
    let bad: Vec<_> = o
        .iter()
        .filter(|(_, r, _)| !matches!(r, Outcome::NotRun(why) if why.starts_with("a call on a tree page of ")))
        .collect();
    assert!(o.len() == 4 && bad.is_empty(), "{o:#?}");
}

/// The review of 04:05Z (`LT TVS Neighbor Row Tests`, each measured on BC): over rows a cell of corpus PR 1539 holds, Last and a
/// call no cell over them makes next (the flat rows with the SetValue on Value and with the action, the child in G0 with the
/// SetValue on Value, the rows G0, G1, G5 with the SetValue on Name) start the rule and refuse, as on main.
#[test]
fn calls_no_cell_makes_next_over_held_rows_refuse() {
    let src = format!("{PROBE}\n{NEIGHBORS}");
    let o = outcomes(&src, "LT TVS Neighbor Row Tests");
    let bad: Vec<_> = o
        .iter()
        .filter(|(_, r, _)| !matches!(r, Outcome::NotRun(why) if why.starts_with("a call on a tree page of ")))
        .collect();
    assert!(o.len() == 4 && bad.is_empty(), "{o:#?}");
}
