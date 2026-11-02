//! A TestPage local to the test method that is still open when the test method returns closes then, as `Close()` closes
//! it: OnQueryClosePage with the action OK, then OnClosePage; an unsaved SetValue is saved first; a Confirm in the trigger
//! goes to the test's ConfirmHandler; a veto (the trigger answers false) leaves the test passing. OpenEdit and OpenView are
//! measured for a page with OnQueryClosePage; a global TestPage is not closed (testpageclose/Y47CloseAtTestEnd.al:127-321;
//! al-runner issue 6587). The file leaves one page open per test: two or more close in the order of their declaration,
//! assumption test-end-close-order.

use super::*;

impl Session {
    /// Closes the open TestPages among the declared locals of `frame`, the frame of `routine` that just returned with
    /// `outcome`, when it is the outermost frame of a `[Test]` method that passed so far.
    pub(crate) fn close_test_pages_at_test_end(
        &mut self,
        frame: &crate::Frame,
        routine: &al_ir::Routine,
        outcome: Result<(), Signal>,
    ) -> Result<(), Signal> {
        if outcome.is_err()
            || !self.frames.is_empty()
            || !routine.attributes.iter().any(|a| a.name.eq_ignore_ascii_case("Test"))
        {
            return outcome;
        }
        let mut pages: Vec<St> = Vec::new();
        for v in frame.locals.iter().take(routine.locals.len()) {
            // Only a page that declares a close trigger and was opened with OpenEdit or OpenView: what the close does to
            // another (a save of a changed row, an OpenNew page) is not measured, and those pages stay as they were.
            if let Value::TestPage(st) = v
                && st.borrow().open
                && st.borrow().mode != al_builtins::value::PageMode::New
                && !pages.iter().any(|p| Rc::ptr_eq(p, st))
                && ["onqueryclosepage", "onclosepage"].iter().any(|t| self.declares(st, t))
            {
                pages.push(st.clone());
            }
        }
        if pages.len() > 1 {
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::TestEndCloseOrder);
        }
        for st in pages {
            match self.page_close(&st) {
                Ok(_) => {}
                // An error of OnQueryClosePage (a Confirm with no ConfirmHandler, an AL Error with no MessageHandler) stops the
                // close and the test passes; OnClosePage does not run (corpus CI of branch al-runtime-rs-staging-y65,
                // testpageclose/lt-y65-close-page-types/Y65ClosePageTypes.al:853 Y65_Open07, :907 Y65_Open10, :977 Y65_Open12,
                // :996 Y65_Open13 and their _Read tests `open card;qc card OK;`; al-runner issue 6673). Not on the pinned corpus
                // yet: assumption test-page-close-query-close-unhandled.
                Err(Signal::Error(_)) if st.borrow().close_error => {
                    al_builtins::assumptions::assume(
                        al_builtins::assumptions::Assumption::TestPageCloseQueryCloseUnhandled,
                    );
                }
                Err(Signal::Error(_)) => {
                    return Err(Signal::Unsupported(
                        "an error while an open TestPage closes at the end of the test (not pinned)".into(),
                    ));
                }
                Err(e) => return Err(e),
            }
        }
        outcome
    }
}
