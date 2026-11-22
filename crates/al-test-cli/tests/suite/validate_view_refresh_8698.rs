//! Al-runner issue 8698: a control's OnValidate that sets a filter on Rec and calls CurrPage.Update(false), on a List page
//! without OnAfterGetCurrRecord (al-vm pages/validate_view_refresh.rs, assumption list-validate-view-refresh). The reporter's
//! repro, the cells of corpus pull request 1518 (pages/currpage/ListValidateFilterRefresh.al) and the neighbors past them, as
//! the neighbor app tests/neighbors/8698-refresh-filter-update-false holds them.

use super::outside_fb_repros::issue_8186_outcomes as outcomes;
use al_builtins::assumptions::Assumption::ListValidateViewRefresh;
use al_test_cli::report::Outcome;

const PROBE: &str = include_str!("../../../../tests/neighbors/8698-refresh-filter-update-false/src/Probe.al");
const NEIGHBORS: &str = include_str!("../../../../tests/neighbors/8698-refresh-filter-update-false/src/Neighbors.al");

/// The refusal of a call the cells do not make after the rule ran.
const UNMEASURED: &str =
    "a call on a page of ListValidateFilterRefresh.al after the page read its row or the view again";

/// The reporter's test passes: the SetValue reads the row first, the refresh reads it again from the table (the typed name is
/// gone) and then the row before it in the filtered view, the second SetValue's refresh reads again, and First lands on B.
/// Main kept the typed name and read only it (`T.V(C2);P.AGR(C2);`).
#[test]
fn issue_8698_repro_passes() {
    let src = format!(
        "{}\n{}",
        include_str!("../../../../tests/neighbors/8698-refresh-filter-update-false/src/Repro.al"),
        include_str!("../../../../tests/neighbors/8698-refresh-filter-update-false/src/ReproTests.al")
    );
    let o = outcomes(&src, "Repro Filter Update Tests");
    let bad: Vec<_> = o
        .iter()
        .filter(|(_, r, marks)| *r != Outcome::Pass || *marks != [ListValidateViewRefresh])
        .collect();
    assert!(o.len() == 1 && bad.is_empty(), "{bad:#?}");
}

/// Every cell on the issue's page (97583) and on page 97584 passes under the assumption; the cell that starts with the test's
/// own filter (:934) keeps main's path (its open reads differ before the rule could run), and so do the pages with other
/// triggers and the Card (their cells measure answers this change does not give).
#[test]
fn issue_8698_probe_cells_pass_on_the_measured_pages() {
    let o = outcomes(PROBE, "LT RFU Tests");
    assert_eq!(o.len(), 38);
    let measured = |n: &str| {
        (n.starts_with("IssueList_") || n.starts_with("List_")) && n != "List_ResetAfterTestFilterUpdateFalse"
    };
    let bad: Vec<_> = o
        .iter()
        .filter(|(n, r, marks)| match measured(n) {
            true => *r != Outcome::Pass || *marks != [ListValidateViewRefresh],
            false => marks.contains(&ListValidateViewRefresh),
        })
        .collect();
    assert!(bad.is_empty(), "{bad:#?}");
    assert_eq!(o.iter().filter(|(n, ..)| measured(n)).count(), 24);
}

/// Past the cells: four or two rows, and a page whose OnValidate calls Update(true), keep main's path; Next or the action
/// after the cell's steps refuse.
#[test]
fn issue_8698_neighbors_keep_main_or_refuse() {
    let o = outcomes(&format!("{PROBE}\n{NEIGHBORS}"), "LT RFU Neighbor Tests");
    assert_eq!(o.len(), 5);
    for (n, r, marks) in &o {
        match n.as_str() {
            "Lines_NextAfterSetValues" | "Lines_ActionAfterSetValues" => {
                assert!(
                    matches!(r, Outcome::NotRun(why) if why.starts_with(UNMEASURED)),
                    "{n}: {r:?}"
                )
            }
            _ => assert!(
                !marks.contains(&ListValidateViewRefresh) && *r != Outcome::Pass,
                "{n}: {r:?}"
            ),
        }
    }
}
