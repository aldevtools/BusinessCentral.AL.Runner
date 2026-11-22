//! Al-runner issues 8698 and 8717: a control's OnValidate that calls CurrPage.Update, with or without a filter on Rec, on a List
//! page without OnAfterGetCurrRecord (al-vm pages/validate_view_refresh.rs, assumption list-validate-view-refresh) and on the
//! other measured pages, the Card of issue 8717 among them (assumption validate-update-false-rereads-row). The reporters'
//! repros, the cells of corpus pull request 1518 (pages/currpage/ListValidateFilterRefresh.al) and the neighbors past them, as
//! the neighbor app tests/neighbors/8698-refresh-filter-update-false holds them.

use super::outside_fb_repros::issue_8186_outcomes as outcomes;
use al_builtins::assumptions::Assumption::{ListValidateViewRefresh, ValidateUpdateFalseRereadsRow};
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

/// The reporter's test of issue 8717 passes: Update(false) in the Name control's OnValidate reads the stored row again, so
/// the typed name is gone. Main kept it (`AGR(Beta);`).
#[test]
fn issue_8717_repro_passes() {
    let src = format!(
        "{}\n{}",
        include_str!("../../../../tests/neighbors/8698-refresh-filter-update-false/src/Repro8717.al"),
        include_str!("../../../../tests/neighbors/8698-refresh-filter-update-false/src/Repro8717Tests.al")
    );
    let o = outcomes(&src, "Repro Edit Card Tests");
    let bad: Vec<_> = o
        .iter()
        .filter(|(_, r, marks)| *r != Outcome::Pass || *marks != [ValidateUpdateFalseRereadsRow])
        .collect();
    assert!(o.len() == 1 && bad.is_empty(), "{bad:#?}");
}

/// Every cell on the issue's page (97583) and on page 97584 passes under list-validate-view-refresh, and every cell on the
/// List with both row triggers, the List with OnAfterGetCurrRecord only, the Card and the Card of issue 8717 under
/// validate-update-false-rereads-row. The cell that starts with the test's own filter (:934) keeps main's path (its open
/// reads differ before the rule could run), and the List without row triggers needs no rule (main gave BC's answer).
#[test]
fn probe_cells_pass_on_the_measured_pages() {
    let o = outcomes(PROBE, "LT RFU Tests");
    assert_eq!(o.len(), 38);
    let mark = |n: &str| match n {
        "List_ResetAfterTestFilterUpdateFalse" => None,
        _ if n.starts_with("IssueList_") || n.starts_with("List_") => Some(ListValidateViewRefresh),
        _ if ["ListBoth_", "ListCurr_", "Card_"].iter().any(|p| n.starts_with(p)) => {
            Some(ValidateUpdateFalseRereadsRow)
        }
        _ => None,
    };
    let bad: Vec<_> = o
        .iter()
        .filter(|(n, r, marks)| match mark(n) {
            Some(m) => *r != Outcome::Pass || *marks != [m],
            None => !marks.is_empty(),
        })
        .collect();
    assert!(bad.is_empty(), "{bad:#?}");
    let count = |m| o.iter().filter(|(n, ..)| mark(n) == Some(m)).count();
    assert_eq!(
        (count(ListValidateViewRefresh), count(ValidateUpdateFalseRereadsRow)),
        (24, 11)
    );
    let edit = outcomes(PROBE, "LT RFU Edit Tests");
    assert!(
        edit.len() == 1 && edit[0].1 == Outcome::Pass && edit[0].2 == [ValidateUpdateFalseRereadsRow],
        "{edit:#?}"
    );
}

/// Past the cells: four or two rows, on the List and on the Card, and a page whose OnValidate calls Update(true), keep main's
/// path; Next or the action after the cell's steps, and a second SetValue on the Card of issue 8717, refuse.
#[test]
fn neighbors_keep_main_or_refuse() {
    let o = outcomes(&format!("{PROBE}\n{NEIGHBORS}"), "LT RFU Neighbor Tests");
    assert_eq!(o.len(), 8);
    for (n, r, marks) in &o {
        match n.as_str() {
            "Lines_NextAfterSetValues"
            | "Lines_ActionAfterSetValues"
            | "Both_NextAfterSetValues"
            | "EditCard_SecondSetValue" => {
                assert!(
                    matches!(r, Outcome::NotRun(why) if why.starts_with(UNMEASURED)),
                    "{n}: {r:?}"
                )
            }
            _ => assert!(marks.is_empty() && *r != Outcome::Pass, "{n}: {r:?}"),
        }
    }
}
