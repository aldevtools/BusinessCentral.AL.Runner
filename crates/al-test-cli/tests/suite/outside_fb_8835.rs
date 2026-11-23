//! The corpus probes of al-runner issues 8835 and 8836 (corpus pull request 1540, open: tests/al-language/pages/currpage/
//! ListNewLineExtraCodeCodedTableTriggers.al and ListNewLineExtraCodeCodedTaskError.al, measured on BC 29.1, 10 of 10 green in 3
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
}

/// The cells of the FactBox with OnPageBackgroundTaskError pass, flagged: the repro of 8836, a task that does not fail, one
/// that fails with the error handled, and one that fails with it unhandled, whose error text reaches the test.
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
