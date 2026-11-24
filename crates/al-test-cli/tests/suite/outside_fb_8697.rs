//! Al-runner issue 8697, through the neighbor app tests/neighbors/8697-bg-worker-last-error (a copy of corpus PR 1516,
//! tests/al-language/pages/background-tasks/BgTaskWorkerLastError.al, and the shapes next to it): a page background task's
//! worker has a last error and an error collection of its own (`al-vm` pages/background.rs `ChildParked`).

use al_builtins::assumptions::Assumption::{self, BgTaskWorkerLastError};
use al_test_cli::report::Outcome;
use al_testrun::isolation::TestIsolation;

fn outcomes(codeunit: &str) -> Vec<(String, Outcome, Vec<Assumption>)> {
    outcomes_with("", codeunit)
}

/// The outcomes of `codeunit`, with the AL source `extra` next to the neighbor app's files.
fn outcomes_with(extra: &str, codeunit: &str) -> Vec<(String, Outcome, Vec<Assumption>)> {
    let src = [
        include_str!("../../../../tests/neighbors/8697-bg-worker-last-error/src/Probe.al"),
        include_str!("../../../../tests/neighbors/8697-bg-worker-last-error/src/Neighbors.al"),
        include_str!("../../../../tests/neighbors/8697-bg-worker-last-error/src/Assert.al"),
        include_str!("../../../../tests/neighbors/8697-bg-worker-last-error/src/ProbeShapes.al"),
        include_str!("../../../../tests/neighbors/8697-bg-worker-last-error/src/ProbeLastErrorObject.al"),
        include_str!("../../../../tests/neighbors/8697-bg-worker-last-error/src/CemFixtures.al"),
        include_str!("../../../../tests/neighbors/8697-bg-worker-last-error/src/NeighborsMore.al"),
        extra,
    ]
    .join("\n");
    let l = al_testkit::compile_apps(vec![al_testkit::app(&src)]);
    let engine = al_vm::Engine::new(l.program.clone(), al_testkit::blocked(&l));
    let oi = l
        .program
        .objects
        .iter()
        .position(|o| o.name == codeunit)
        .expect("the codeunit");
    let mut hits = Vec::new();
    let outcomes = al_testrun::runtime::run_codeunit_full(
        &engine,
        &l,
        oi,
        None,
        TestIsolation::Codeunit,
        None,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut hits,
        None,
    );
    assert!(!outcomes.is_empty());
    outcomes
        .into_iter()
        .map(|(n, o)| {
            let marks = hits
                .iter()
                .find(|(h, _)| *h == n)
                .map(|(_, m)| m.clone())
                .unwrap_or_default();
            (n, o, marks)
        })
        .collect()
}

/// Every cell of the probe passes. The cells whose worker raises an error it does not catch, caught by the test session through
/// a procedure of its own (as before issue 8697), pass clean; every other cell runs through the assumption.
#[test]
fn issue_8697_probe_cells() {
    let o = outcomes("LEW Tests");
    assert_eq!(o.len(), 26);
    for (name, r, marks) in &o {
        assert_eq!(*r, Outcome::Pass, "{name}");
        let clean = name == "WorkerUnhandledError_TestSessionCatchesIt"
            || name == "ErrorTrigger_AfterAnUnhandledWorkerError_ReadsTheTestSessionsLastError"
            || name.ends_with("_AssertErrorOnAProcedure_TheWorkersError");
        assert_eq!(marks.contains(&BgTaskWorkerLastError), !clean, "{name}: {marks:?}");
    }
}

/// The shapes next to the cells (an enqueued task, a test session error from a TryFunction, a worker's guarded Codeunit.Run)
/// pass through the assumption; a TryFunction around the call records the worker's error, as before issue 8697, clean.
#[test]
fn issue_8697_neighbors() {
    let o = outcomes("LEN Tests");
    assert_eq!(o.len(), 4);
    for (name, r, marks) in &o {
        assert_eq!(*r, Outcome::Pass, "{name}");
        let clean = name == "WorkerUnhandledError_TryFunctionAroundTheCall_TheWorkersError";
        assert_eq!(marks.contains(&BgTaskWorkerLastError), !clean, "{name}: {marks:?}");
    }
}

/// A collectible TestField the worker raises while the test session collects fails the task, as BC does; before issue
/// 8697 it was collected in the test session's scope (corpus PR 1516, BgTaskWorkerSessionShapes.al:98).
#[test]
fn issue_8697_worker_collectible_testfield() {
    let o = outcomes("LWS Tests");
    let (name, r, marks) = o
        .iter()
        .find(|(n, ..)| n == "TestSessionCollects_WorkerCollectibleTestField_FailsTheTask")
        .expect("the cell");
    assert_eq!(*r, Outcome::Pass, "{name}");
    assert!(marks.contains(&BgTaskWorkerLastError), "{name}: {marks:?}");
}

/// A PageHandler whose worker caught an error closes the page as one that caught none: the worker's count of caught errors
/// is its own (corpus PR 1516, BgTaskWorkerSessionShapes.al:191, :198, :205; a Document's ModalPageHandler and a card from a
/// Page variable next to them, NeighborsMore.al).
#[test]
fn issue_8697_worker_caught_errors_in_a_page_handler() {
    let cells = outcomes("LWS Handler Tests");
    let neighbors = outcomes("LEN More Tests");
    let mut seen = 0;
    for (name, r, marks) in cells.iter().chain(&neighbors) {
        if name.contains("_WorkerCaughtItsError_") {
            seen += 1;
            assert_eq!(*r, Outcome::Pass, "{name}");
            assert!(marks.contains(&BgTaskWorkerLastError), "{name}: {marks:?}");
        }
    }
    assert_eq!(seen, 5);
}

/// A worker's GetLastErrorObject before an error of its own is null, as after ClearLastError, where the test session's refuses
/// (corpus PR 1516, BgTaskWorkerLastErrorObject.al:91, :103, :114, :126; two tasks in a row, NeighborsMore.al).
#[test]
fn issue_8697_worker_last_error_object() {
    let cells = outcomes("LWO Tests");
    assert_eq!(cells.len(), 4);
    let neighbors = outcomes("LEN More Tests");
    let two = neighbors
        .iter()
        .filter(|(n, ..)| n.ends_with("_GetLastErrorObjectNull"));
    for (name, r, marks) in cells.iter().chain(two) {
        assert_eq!(*r, Outcome::Pass, "{name}");
        // The worker's own exception after it caught its error was the answer before issue 8697 too.
        let clean = name == "WorkerCaughtItsError_WorkerGetLastErrorObject_DialogException";
        assert_eq!(marks.contains(&BgTaskWorkerLastError), !clean, "{name}: {marks:?}");
    }
}

/// The test session's last error that a worker's end gave back is flagged until the first read of another one: the same
/// error raised again later reads as it would have before issue 8697 (review of PR 8705).
#[test]
fn issue_8697_kept_last_error_ends_at_the_first_read_of_another() {
    let extra = r#"
codeunit 97389 "LEK Tests"
{
    Subtype = Test;

    [Test]
    procedure KeptEndsAtTheFirstReadOfAnother()
    var
        Host: Record "LEW Host";
        Hosts: TestPage "LEW Hosts";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
        I: Integer;
    begin
        Host.DeleteAll();
        Host."No." := 'H';
        Host.Insert();
        Commit();
        Parameters.Add('Mode', 'catch');
        for I := 1 to 2 do begin
            Raise();
            if I = 1 then begin
                Hosts.OpenView();
                Results := Hosts.RunPageBackgroundTask(Codeunit::"LWS Reader", Parameters, false);
                Hosts.Close();
                asserterror Error('Another');
                Seen := GetLastErrorText();
            end;
        end;
        Seen := GetLastErrorText();
    end;

    local procedure Raise()
    var
        Host: Record "LEW Host";
    begin
        asserterror Host.Get('MISSING');
    end;
}
"#;
    let o = outcomes_with(extra, "LEK Tests");
    let (name, r, marks) = &o[0];
    assert_eq!(*r, Outcome::Pass, "{name}");
    assert!(!marks.contains(&BgTaskWorkerLastError), "{name}: {marks:?}");
}
