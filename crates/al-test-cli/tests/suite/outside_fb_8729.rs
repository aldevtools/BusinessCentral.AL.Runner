//! Al-runner issue 8729, through the neighbor app tests/neighbors/8729-handler-caught-error-close (a copy of corpus PR
//! 1524, tests/al-language/pages/close-errors/CemHandlerCaughtErrors.al, its CEM fixtures, and the shapes next to it):
//! a page handler that caught an error closes the page as one that caught none (`al-vm` pages/page_run_close.rs
//! `page_run_cell`, pages/document_handler_error.rs `close_document_after_handler_error`).

use al_builtins::assumptions::Assumption::{self, PageHandlerCaughtErrorClose};
use al_test_cli::report::Outcome;
use al_testrun::isolation::TestIsolation;

fn outcomes(codeunit: &str) -> Vec<(String, Outcome, Vec<Assumption>)> {
    let src = [
        include_str!("../../../../tests/neighbors/8729-handler-caught-error-close/src/Probe.al"),
        include_str!("../../../../tests/neighbors/8729-handler-caught-error-close/src/CemFixtures.al"),
        include_str!("../../../../tests/neighbors/8729-handler-caught-error-close/src/Neighbors.al"),
        include_str!("../../../../tests/neighbors/8729-handler-caught-error-close/src/Assert.al"),
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

/// The cells of Page.Run: a handler that caught an error closes the card and the list as the pinned cell of a handler that
/// caught none. The closes of the pinned matrix (a handler error, a return while OnQueryClosePage raises) run through the
/// assumption; a return or OK with no error is no cell of the matrix and passes clean, as before, and so does the handler
/// that caught nothing.
#[test]
fn issue_8729_page_run_cells() {
    let o = outcomes("CEM Caught Run Tests");
    assert_eq!(o.len(), 13);
    for (name, r, marks) in &o {
        assert_eq!(*r, Outcome::Pass, "{name}");
        let flagged = !name.contains("_HandlerCaughtNothing_")
            && (name.ends_with("_HandlerError") || name.ends_with("_QueryCloseError"));
        assert_eq!(
            marks.contains(&PageHandlerCaughtErrorClose),
            flagged,
            "{name}: {marks:?}"
        );
    }
}

/// The cells of Page.RunModal of the document: the handler that caught an error raises, and the document closes as after a
/// handler error with nothing caught, flagged, where it was refused.
#[test]
fn issue_8729_document_run_modal_cells() {
    let o = outcomes("CEM Caught RunModal Tests");
    for (name, r, marks) in o.iter().filter(|(n, ..)| n.starts_with("DocumentStaticRunModal_")) {
        assert_eq!(*r, Outcome::Pass, "{name}");
        assert!(marks.contains(&PageHandlerCaughtErrorClose), "{name}: {marks:?}");
    }
}

/// The shapes next to the cells keep the answer they had: the card from a Page variable and the list at OK after a caught
/// error pass clean, and the document whose close raises after a caught error is refused.
#[test]
fn issue_8729_neighbors() {
    let run = outcomes("CHN Run Tests");
    assert_eq!(run.len(), 2);
    for (name, r, marks) in &run {
        assert_eq!(*r, Outcome::Pass, "{name}");
        assert!(marks.is_empty(), "{name}: {marks:?}");
    }
    let modal = outcomes("CHN RunModal Tests");
    assert!(
        matches!(&modal[0].1, Outcome::NotRun(why) if why.contains("caught an error")),
        "{:?}",
        modal[0]
    );
}
