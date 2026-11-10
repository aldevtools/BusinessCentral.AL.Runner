//! A TestPage local to the test method that is still open when the test method returns closes then, as `Close()` closes
//! it: OnQueryClosePage with the action OK, then OnClosePage; an unsaved SetValue is saved first; a Confirm in the trigger
//! goes to the test's ConfirmHandler; a veto (the trigger answers false) leaves the test passing. OpenEdit and OpenView are
//! measured for a page with OnQueryClosePage; a global TestPage is not closed (testpage/lifecycle/Y47CloseAtTestEnd.al:127-321;
//! al-runner issue 6587). Two or more close in reverse order of their declaration, whatever order they were opened in
//! (testpageclose/lt-y86-close-order/Y86CloseOrderAtTestEnd.al).

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
            // Under a test runner codeunit (ADR-066) its frames stay below the test method's.
            || self.frames.len() != self.test_frame_base()
            || !routine.attributes.iter().any(|a| a.name.eq_ignore_ascii_case("Test"))
        {
            return outcome;
        }
        let mut pages: Vec<St> = Vec::new();
        for v in frame.locals.iter().take(routine.locals.len()) {
            if let Value::TestPage(st) = v
                && let st = st_of(st)
                && st.borrow().open
                // A page whose save failed at Close() is left alone (pages/close_save_failed.rs).
                && !st.borrow().close_save_failed
                && !pages.iter().any(|p| Rc::ptr_eq(p, st))
                && self.closes_at_test_end(st)?
            {
                pages.push(st.clone());
            }
        }
        pages.reverse();
        for st in pages {
            match self.page_close(&st) {
                Ok(_) => {}
                // An error of OnQueryClosePage (a Confirm with no ConfirmHandler, an AL Error with no MessageHandler) stops the
                // close and the test passes; OnClosePage does not run (corpus CI of branch al-runtime-rs-staging-y65,
                // testpage/lifecycle/Y65ClosePageTypes.al:853 Y65_Open07, :907 Y65_Open10, :977 Y65_Open12,
                // :996 Y65_Open13 and their _Read tests `open card;qc card OK;`; al-runner issue 6673). Not on the pinned corpus
                // yet: assumption test-page-close-query-close-unhandled.
                Err(Signal::Error(_)) if st.borrow().close_error => {}
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

    /// Whether an open TestPage closes at the end of the test. A page that declares a close trigger and was opened with
    /// OpenEdit or OpenView does (Y47CloseAtTestEnd.al). A Card does too, with or without OnQueryClosePage, opened with
    /// OpenEdit or OpenNew (on a new key, or then moved to a stored row with GoToRecord): the value set on it is saved as
    /// `Close()` saves it, and OnQueryClosePage runs with CloseAction OK after the value is set (corpus
    /// pages/lifecycle/Y7345SaveOnLeftOpen.al:153 Y7345_Write1_EditValid_NoQuery, :172 _Write2_EditValid_Query, :217
    /// _Write6_OpenNewNewKey_NoQuery, :227 _Write7, :237 _Write8_OpenNewGoToExisting_NoQuery, :248 _Write9, read by :291
    /// Y7345_Read2_RowsAfterAllCards; al-runner issue 7345). A Card whose last SetValue failed in OnValidate does not close:
    /// nothing is saved, a valid value set before the failed one neither, and OnQueryClosePage does not run (:183
    /// _Write3_EditInvalid_NoQuery, :194 _Write4, :205 _Write5_EditValidThenInvalid_Query, :271 _Write11, :281 _Write12).
    ///
    /// The probe measured a Card that stands on a stored row and may modify it. A Card without a close trigger, or opened with
    /// OpenNew, in another state (its row is no longer stored or changed its key, ModifyAllowed is false, a save already
    /// failed, a new row not inserted yet) is not measured, nor is another page type without a close trigger or opened with
    /// OpenNew: those pages stay as they were.
    fn closes_at_test_end(&mut self, st: &St) -> Result<bool, Signal> {
        let top_card = self.is_card(st) && st.borrow().host.is_none();
        if top_card && !st.borrow().errors.is_empty() {
            return Ok(false);
        }
        if st.borrow().mode != crate::pages::PageMode::New
            && ["onqueryclosepage", "onclosepage"].iter().any(|t| self.declares(st, t))
        {
            return Ok(true);
        }
        let measured = {
            let s = st.borrow();
            top_card && s.cursor == Cursor::Row && !s.draft && !s.save_failed && !s.refused
        };
        if !measured
            || !self.has_source(st)
            || flag_denied(&self.engine(), st.borrow().object as usize, "ModifyAllowed")?
        {
            return Ok(false);
        }
        let probe = Rc::new(RefCell::new(self.page_rec(st).borrow().clone()));
        let mut key = self.row_key(&probe)?;
        self.ret_used = true;
        Ok(matches!(
            self.record_op(al_builtins::record_op::RecordOp::Get, &probe, &mut key)?,
            Value::Boolean(true)
        ))
    }
}
