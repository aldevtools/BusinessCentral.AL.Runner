//! Al-runner issues 8698 and 8717: a control's OnValidate that calls CurrPage.Update, with or without a filter on Rec, on a List
//! page without OnAfterGetCurrRecord (al-vm pages/validate_view_refresh.rs, assumption list-validate-view-refresh) and on the
//! other measured pages, the Card of issue 8717 among them (assumption validate-update-false-rereads-row). The reporters'
//! repros, the cells of corpus pull request 1518 (pages/currpage/ListValidateFilterRefresh.al) and the neighbors past them, as
//! the neighbor app tests/neighbors/8698-refresh-filter-update-false holds them.

use super::outside_fb_repros::assert_stub_outcomes as outcomes;
use al_builtins::assumptions::Assumption::{ListValidateViewRefresh, ValidateUpdateFalseRereadsRow};
use al_test_cli::report::Outcome;

const PROBE: &str = include_str!("../../../../tests/neighbors/8698-refresh-filter-update-false/src/Probe.al");
const NEIGHBORS: &str = include_str!("../../../../tests/neighbors/8698-refresh-filter-update-false/src/Neighbors.al");
const PROBE_READS: &str =
    include_str!("../../../../tests/neighbors/8698-refresh-filter-update-false/src/ProbeReads.al");

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
    assert_eq!(o.len(), 86);
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
        (42, 41)
    );
    let edit = outcomes(PROBE, "LT RFU Edit Tests");
    assert!(
        edit.len() == 5
            && edit
                .iter()
                .all(|(_, r, marks)| *r == Outcome::Pass && *marks == [ValidateUpdateFalseRereadsRow]),
        "{edit:#?}"
    );
}

/// Past the cells: four or two rows, on the List and on the Card, and a page whose OnValidate calls Update(true), keep main's
/// path; Next or the action after the cell's steps, and a second SetValue on the Card of issue 8717, refuse; AssertEquals and
/// Editable() after the cell's steps only read, and the cell's First and walk follow.
#[test]
fn neighbors_keep_main_or_refuse() {
    let o = outcomes(&format!("{PROBE}\n{NEIGHBORS}"), "LT RFU Neighbor Tests");
    assert_eq!(o.len(), 9);
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
            "Lines_ReadOnlyCallsAfterSetValues" => {
                assert!(*r == Outcome::Pass && *marks == [ListValidateViewRefresh], "{n}: {r:?}")
            }
            _ => assert!(marks.is_empty() && *r != Outcome::Pass, "{n}: {r:?}"),
        }
    }
}

/// The read-only calls pass at the points the read cells of corpus pull request 1529 measure (Visible() on Name after the
/// cell's two SetValues, before First); after First, in a cell the read cells do not measure (the row dropped), and at two
/// points of one cell they refuse.
#[test]
fn read_only_calls_pass_only_where_measured() {
    let outcome = |from: &str, to: &str| {
        assert!(NEIGHBORS.contains(from));
        let o = outcomes(
            &format!("{PROBE}\n{}", NEIGHBORS.replacen(from, to, 1)),
            "LT RFU Neighbor Tests",
        );
        o.into_iter()
            .find(|(n, ..)| n == "Lines_ReadOnlyCallsAfterSetValues")
            .unwrap()
    };
    let at = "                    Lines.Name.AssertEquals('C');\n";
    let (_, r, marks) = outcome(at, "                    if Lines.Name.Visible() then;\n");
    assert!(
        r == Outcome::Pass && marks == [ListValidateViewRefresh],
        "{r:?} {marks:?}"
    );
    let first = "                    Lines.First();\n";
    let open = "        Lines.Last();\n        Note('open', Log.Take());\n        Lines.Name.SetValue(Lines.Name.Value() + '2');\n";
    for (from, to) in [
        (
            first,
            "                    Lines.First();\n                    if Lines.Name.Visible() then;\n",
        ),
        (
            "        Initialize(1, 1, 3);\n        RunLines(3);\n",
            "        Initialize(4, 1, 3);\n        RunLines(3);\n",
        ),
        (
            open,
            "        Lines.Last();\n        Lines.Name.AssertEquals('C');\n        Note('open', Log.Take());\n        Lines.Name.SetValue(Lines.Name.Value() + '2');\n",
        ),
    ] {
        let (_, r, _) = outcome(from, to);
        assert!(
            matches!(&r, Outcome::NotRun(why) if why.starts_with(UNMEASURED)),
            "{to}: {r:?}"
        );
    }
}

/// Corpus pull request 1535 (ListValidateViewReads.al): on page 97498, the issue's page with controls of every type, the
/// issue's cell and the As* reads, OptionCount, GetOption and IsExpanded at its points pass under the rule. GetField by a
/// control id refuses once the rule ran and keeps main's (wrong) answer before; the bad SetValue on Qty and the tree page
/// keep main's path.
#[test]
fn wide_page_reads_pass_where_measured() {
    let o = outcomes(PROBE_READS, "LT RVR Tests");
    assert_eq!(o.len(), 22);
    for (n, r, marks) in &o {
        match n.as_str() {
            "Lines_GetFieldById_BetweenSteps" | "Lines_GetFieldById_AfterSteps" => assert!(
                matches!(r, Outcome::NotRun(why) if why.starts_with(UNMEASURED)),
                "{n}: {r:?}"
            ),
            "Lines_SetValueQtyNotANumber_ValidationError" | "Tree_SetValueQtyNotANumber_ValidationError" => {
                assert!(*r == Outcome::Pass && marks.is_empty(), "{n}: {r:?} {marks:?}")
            }
            _ if n.starts_with("Lines_") && !n.starts_with("Lines_GetFieldById_") => {
                assert!(
                    *r == Outcome::Pass && *marks == [ListValidateViewRefresh],
                    "{n}: {r:?} {marks:?}"
                )
            }
            _ => assert!(*r != Outcome::Pass && marks.is_empty(), "{n}: {r:?} {marks:?}"),
        }
    }
}
