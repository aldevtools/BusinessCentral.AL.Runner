//! `TestPage.Close()` of a Card whose save of the changed row fails with an AL Error (an error of the table's OnModify or of the
//! page's OnModifyRecord), pinned by testpageclose/lt-y67-close-save-error/Y67CloseSaveError.al:239-428:
//! - the Close raises the error with its own text and the code TestWrapped; OnQueryClosePage and OnClosePage do not run, and
//!   the row stays as stored (:239 R1_TableModifyError_SecondClose, :336 R1_PageModifyError_SecondClose, :409
//!   R1_NoCloseTriggers_TableModifyError on a Card without close triggers);
//! - the page still answers a field read with the value set, and accepts a SetValue (:256 R1_TableModifyError_CorrectedThenClose);
//! - a second Close() succeeds and runs no trigger and no save, also after a SetValue that removed the cause (:256);
//! - a page left open so ends the test with nothing run again (:285 R1_TableModifyError_LeftOpen and :298 _Read, :311, :324,
//!   :447 R1_NoCloseTriggers_LeftOpen_Read).
//!
//! Only these steps are pinned: any other TestPage call on such a page is refused, and so is a SetValue of a control that
//! runs a validate trigger. Another page type, a page run modally or by a handler, and another error code keep the earlier
//! behavior of `page_close`.

use super::*;
use al_builtins::page_op::{PageMethod, PageOp};

impl Session {
    /// The client's save of the row at a close (`close_with`): an AL Error of the save is noted for `page_close`.
    pub(super) fn close_save_row(&mut self, st: &St) -> Result<(), Signal> {
        let r = self.save_row(st);
        if matches!(&r, Err(Signal::Error(e)) if e.code == "Dialog") {
            self.pages.close_save_error = true;
        }
        r
    }

    /// After the close of `page_close`: the error of a failed save of a Card the test opened becomes TestWrapped, and the page
    /// enters the state of this module. `None` leaves the outcome to `page_close`.
    pub(super) fn after_failed_close_save(
        &mut self,
        st: &St,
        r: &Result<Value, Signal>,
    ) -> Option<Result<Value, Signal>> {
        if !std::mem::take(&mut self.pages.close_save_error) {
            return None;
        }
        let Err(Signal::Error(e)) = r else { return None };
        let pinned = {
            let s = st.borrow();
            !s.modal && s.host.is_none() && s.parts.is_empty() && s.mode != al_builtins::value::PageMode::New
        } && self.is_card(st)
            && self.ui_state.page_handler_depth == 0;
        if !pinned {
            return None;
        }
        st.borrow_mut().close_save_failed = true;
        let mut wrapped = (**e).clone();
        wrapped.code = "TestWrapped".into();
        Some(Err(Signal::Error(Box::new(wrapped))))
    }

    /// A second `Close()` of a page whose save failed at Close: it closes, with no trigger and no save (Y67CloseSaveError.al:239,
    /// :256).
    pub(super) fn close_after_failed_save(&mut self, st: &St) -> Option<Result<Value, Signal>> {
        if !st.borrow().close_save_failed {
            return None;
        }
        {
            let mut s = st.borrow_mut();
            s.close_save_failed = false;
            s.dirty = false;
        }
        self.mark_closed(st);
        Some(Ok(Value::Null))
    }

    /// Refuses a TestPage call on a page whose save failed at Close (only a second Close is pinned).
    pub(super) fn refuse_after_failed_close_save(&self, st: &St) -> Result<(), Signal> {
        if st.borrow().close_save_failed {
            return unsupported(
                "a TestPage call on a page whose save failed at TestPage.Close(), other than Close (not pinned)",
            );
        }
        Ok(())
    }

    /// The check of `refuse_unpinned_state` for a control call: on a page whose save failed at Close, only a field read and a
    /// SetValue of a control that runs no validate trigger are pinned (Y67CloseSaveError.al:239, :256).
    pub(super) fn refuse_unpinned_control_state(&self, c: &ControlRef, method: PageOp) -> Result<(), Signal> {
        if c.page.borrow().close_save_failed {
            let ControlTarget::Control(idx) = c.target else {
                return self.refuse_after_failed_close_save(&c.page);
            };
            let engine = self.engine();
            let plain = page_of(&engine, &c.page).controls[idx].trigger("onvalidate").is_none()
                && self.control_field(&c.page, idx).is_some_and(|f| {
                    f.on_validate.is_none() && f.ext_before_validate.is_empty() && f.ext_after_validate.is_empty()
                });
            if !(method.method() == PageMethod::Value || method.method() == PageMethod::SetValue && plain) {
                return unsupported(format!(
                    "TestField.{} on a page whose save failed at TestPage.Close() (not pinned)",
                    method.method()
                ));
            }
        }
        self.refuse_unpinned_state_only(&c.page)
    }
}
