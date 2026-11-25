//! The corpus probes of al-runner issues 8835 and 8836 (corpus pull request 1540, open: tests/al-language/pages/currpage/
//! ListNewLineExtraCodeCodedTableTriggers.al and ListNewLineExtraCodeCodedTaskError.al, measured on BC 29.1, 15 of 15 green in 3
//! runs), run here until a bump pins them: the page of the page-with-code shape (`al-vm` pages/code_inserted_coded.rs
//! `CodedObjects::Extra`) over a table with OnInsert and OnDelete, and with a FactBox that declares OnPageBackgroundTaskError. The
//! probes are read from their copies in the neighbor app. Delete this file when the pin is in the corpus.

use al_builtins::assumptions::Assumption::{ListNewLineExtraCode, ListNewLineExtraCodeCoded};
use al_test_cli::report::Outcome;

/// The table probe, whole, as the neighbor app holds it (its first line says so).
const TABLE_PROBE: &str =
    include_str!("../../../../tests/neighbors/8835-coded-page-table-triggers-task-error/src/ProbeTableTriggers.al");

/// The task error probe, whole, as the neighbor app holds it.
const TASK_PROBE: &str =
    include_str!("../../../../tests/neighbors/8835-coded-page-table-triggers-task-error/src/ProbeTaskError.al");

/// The neighbors just past the cells, each with BC's answer (3 runs of 3 on BC 29.1).
const NEIGHBORS: &str =
    include_str!("../../../../tests/neighbors/8835-coded-page-table-triggers-task-error/src/Neighbors.al");

/// The neighbors of the review of this pull request (01:32Z), each in its own codeunit with BC's answer (3 runs of 3 on BC 29.1).
const REVIEW: &str =
    include_str!("../../../../tests/neighbors/8835-coded-page-table-triggers-task-error/src/ReviewNeighbors.al");

/// The cells of the table with OnInsert and OnDelete pass, flagged: the repro of 8835, OnInsert setting a field, reading the
/// table for a sorting number, calling a codeunit, and raising an error, which reaches the test. An OnInsert that writes
/// another table before the line's insert keeps the refusal: the store does not tell which table that write went to.
#[test]
fn issue_8835_probe_cells() {
    let o = super::outside_fb_8712::outcomes(TABLE_PROBE, "LT NE2 Tests");
    assert_eq!(o.len(), 6);
    for (name, r, marks) in &o {
        match name.as_str() {
            "NewLine_OnInsertWritesAnotherTable_ShowsAndSavesTheRow" => {
                assert!(matches!(r, Outcome::NotRun(_)), "{name}: {r:?}")
            }
            "NewLine_OnInsertRaisesAnError_TheSetValueFails" => {
                assert_eq!(*r, Outcome::Pass, "{name}");
                assert!(marks.contains(&ListNewLineExtraCodeCoded), "{name}: {marks:?}");
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
    // The other errors of the first SetValue the cells measure (:1249, :1259, :1269): a TestField of the table field before the
    // insert, an error of it after the insert, the control's error before its save; each reaches the test.
    // And the control's error when the table field skips its insert, after the control's plain save or before it (:1674,
    // :1684): main answers both so, and BC too (review of 07:30Z). And the control's error after it inserted, modified or
    // deleted a row of another table or Rec (:2121-:2238): main answers each so, and BC too (review of 09:29Z).
    for (cu, cells) in [("LT NE4 Tests", 3), ("LT NE5 Tests", 2), ("LT NE6 Tests", 11)] {
        let o = super::outside_fb_8712::outcomes(TABLE_PROBE, cu);
        assert_eq!(o.len(), cells, "{cu}");
        for (name, r, marks) in &o {
            assert_eq!(*r, Outcome::Pass, "{name}");
            assert!(marks.contains(&ListNewLineExtraCodeCoded), "{name}: {marks:?}");
        }
    }
}

/// The cells of the FactBox with OnPageBackgroundTaskError pass, flagged: the repro of 8836, a task that does not fail, one
/// that fails with the error handled, and one that fails with it unhandled, whose error text reaches the test with the code
/// ClientPageAbort:FormAbort (:589).
#[test]
fn issue_8836_probe_cells() {
    let o = super::outside_fb_8712::outcomes(TASK_PROBE, "LT NQ2 Tests");
    assert_eq!(o.len(), 4);
    for (name, r, marks) in &o {
        assert_eq!(*r, Outcome::Pass, "{name}");
        assert!(marks.contains(&ListNewLineExtraCodeCoded), "{name}: {marks:?}");
    }
}

/// An error of OnInsert that the handler catches, the page then driven on: the cell's error leaves the handler (:746), a
/// later call on the page is not measured and refused.
#[test]
fn issue_8835_error_caught_in_the_handler_is_refused() {
    let handler = "VarErrorHandler(var Lines: TestPage \"LT NE2 Var Lines\")\n    begin\n        Lines.New();\n        Lines.Name.SetValue('A');";
    assert!(TABLE_PROBE.contains(handler));
    let caught = TABLE_PROBE.replace(
        handler,
        &handler.replace("Lines.Name.SetValue", "asserterror Lines.Name.SetValue"),
    );
    let o = super::outside_fb_8712::outcomes(&caught, "LT NE2 Tests");
    let (_, r, _) = o
        .iter()
        .find(|(name, _, _)| name == "NewLine_OnInsertRaisesAnError_TheSetValueFails")
        .unwrap();
    assert!(
        matches!(r, Outcome::NotRun(w) if w.contains("after the validation of its first SetValue raised an error")),
        "{r:?}"
    );
}

/// An error trigger that writes another table, and one that calls CurrPage.Update(false), keep a refusal. The neighbors use the
/// task error probe's table and parts, so they compile with it.
#[test]
fn issue_8836_neighbors_are_refused() {
    let o = super::outside_fb_8712::outcomes(&format!("{TASK_PROBE}\n{NEIGHBORS}"), "LT NQ3 Tests");
    assert_eq!(o.len(), 2);
    let ran: Vec<_> = o
        .iter()
        .filter(|(_, r, _)| !matches!(r, Outcome::NotRun(_)))
        .map(|(name, r, _)| format!("{name}: {r:?}"))
        .collect();
    assert!(ran.is_empty(), "{ran:#?}");
}

/// The review's neighbors are refused: a handler that catches OnInsert's error and returns, one that catches the unhandled task
/// error and goes on, a worker that fails on the blank record at the open, or on the parts' read of the new line before the
/// validation (review of 07:30Z), and an error trigger that calls CurrPage.Update (its own refusal). The shapes that review left
/// deferred are refused too: an error trigger that raises its own error or enqueues another task, and a worker that fails
/// with a run-time error (docs/probes-needed.md).
#[test]
fn issue_8840_review_neighbors() {
    let src = format!("{TABLE_PROBE}\n{TASK_PROBE}\n{REVIEW}");
    let codeunits: Vec<&str> = REVIEW
        .lines()
        .filter_map(|l| {
            l.strip_prefix("codeunit ")?
                .split_once(' ')?
                .1
                .strip_prefix('"')?
                .strip_suffix('"')
        })
        .filter(|n| n.ends_with(" Tests"))
        .collect();
    assert_eq!(codeunits.len(), 15);
    for cu in codeunits {
        let o = super::outside_fb_8712::outcomes(&src, cu);
        assert_eq!(o.len(), 1, "{cu}");
        let (name, r, marks) = &o[0];
        if cu == "LT NQ6 Ignore No Trigger Tests" {
            assert_eq!(*r, Outcome::Pass, "{name}: {r:?}");
            assert!(marks.contains(&ListNewLineExtraCodeCoded), "{name}: {marks:?}");
            continue;
        }
        assert!(matches!(r, Outcome::NotRun(_)), "{name}: {r:?}");
        let reason = match cu {
            "LT NQ4 Update Tests" => "called CurrPage.Update",
            "LT NQ4 Trigger Error Tests" => "raised an error of its own",
            "LT NQ4 Requeue Tests" => "enqueued another task",
            "LT NQ4 Run Time Error Tests" => "failed with a run-time error",
            "LT NQ5 Unhandled Update Tests" => "called CurrPage.Update",
            "LT NQ5 Unhandled Requeue Tests" => "enqueued another task",
            "LT NQ5 Complete Again Tests" => "enqueued another task",
            "LT NQ5 Ignore Runtime Tests" => "failed with a run-time error",
            "LT NQ5 Ignore Trigger Tests" => "raised an error of its own",
            "LT NQ7 Duplicate Key Tests" => "did not leave the line",
            _ => "",
        };
        assert!(matches!(r, Outcome::NotRun(w) if w.contains(reason)), "{r:?}");
    }
}
