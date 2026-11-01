//! A TestPage local to the test method that is still open when the test method returns closes then, as `Close()` closes
//! it: OnQueryClosePage with the action OK, then OnClosePage; an unsaved SetValue is saved first; a Confirm in the trigger
//! goes to the test's ConfirmHandler; a veto (the trigger answers false) leaves the test passing. OpenEdit and OpenView are
//! measured for a page with OnQueryClosePage; a global TestPage is not closed (testpageclose/Y47CloseAtTestEnd.al, corpus branch al-runtime-rs-staging-y47;
//! al-runner issue 6587; assumption test-end-closes-open-testpages until the file is on corpus master).

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
        for st in pages {
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::TestEndClosesOpenTestPages);
            match self.page_close(&st) {
                Ok(_) => {}
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
