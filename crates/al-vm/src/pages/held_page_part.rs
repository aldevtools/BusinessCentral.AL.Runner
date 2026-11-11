//! What the code of a page that ran and is closed reaches through `CurrPage.<part>.Page`: the page's own part instances,
//! which its state keeps after the close (`PageState::closed_parts` for the shown parts, `hidden_parts` for the ones a
//! Visible expression hid). What closed the page (`PageState::closed_by`, set where the close happens) decides whether the
//! part answers or the call is refused:
//! - a modal run closed by an action (OK, Cancel) or by the platform after its handler: pages/parts/WdPartAfterRunModal.al,
//!   Y37PartAfterClose.al, NeverRanReadRec.al;
//! - the page's own `CurrPage.Close`: Y37PartAfterClose.al;
//! - `TestPage.Close()` of a page the test opened: pages/parts/HeldSenderPartAccess.al (al-runner issue 6924).
//!
//! Every other close is refused: the close at the end of the test, a refused or failed `TestPage.Close()`, the built-in OK
//! action on a TestPage the test opened, a modal page closed after its handler raised an error.
//! Inside a part reached after `TestPage.Close()` only what the probe measured runs: the part's procedures, its globals, the
//! fields of its Rec and `Rec.Count()`; a page method on the part, any other method of its Rec and its xRec refuse. A field
//! write on its Rec runs and marks the assumption `held-page-part-after-testpage-close`, as does a reopen of the same TestPage
//! variable while a SingleInstance codeunit holds the page (`fresh_page_for_held_reopen`).

use super::*;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::page_op::{PageMethod, PageOp};

const REFUSED: &str =
    "a part of a page that is not open, reached by the code of that page (what it does then is not pinned)";

impl Session {
    /// Closes `st` as `mark_closed` does, with `by` as the cause.
    pub(super) fn mark_closed_as(&mut self, st: &St, by: ClosedBy) {
        self.mark_closed(st);
        st.borrow_mut().closed_by = Some(by);
    }

    /// `CurrPage.Close()` or `Close()` of a Page variable: closes as `close_with` does, the cause `ClosedBy::PageCode`.
    pub(super) fn close_by_page_code(&mut self, st: &St, action: &str) -> Result<Value, Signal> {
        let r = self.close_with(st, action, false, false)?;
        if !st.borrow().open {
            st.borrow_mut().closed_by = Some(ClosedBy::PageCode);
        }
        Ok(r)
    }

    /// `TestPage.Close()` ended and the page is closed: `ClosedBy::TestClose` when it ran to its end on a page the test opened
    /// (no veto, no error in OnQueryClosePage, not a modal page in its handler), else `ClosedBy::Other`.
    pub(super) fn mark_closed_by_test(&mut self, st: &St) {
        let clean = {
            let s = st.borrow();
            s.opened_by_test
                && !s.modal
                && !s.close_vetoed
                && !s.close_error
                && !s.test_close_vetoed
                && !s.test_closed_in_handler
        };
        let by = if clean { ClosedBy::TestClose } else { ClosedBy::Other };
        self.mark_closed_as(st, by);
    }

    /// The part page of control `control` of `st`, a page that is not open, as the page's own code reaches it.
    pub(super) fn closed_page_part(&mut self, st: &St, control: usize) -> Result<St, Signal> {
        let (ran, modal_run, by) = {
            let s = st.borrow();
            (s.ran, s.last_run_modal == Some(true), s.closed_by)
        };
        if !ran {
            return unsupported(REFUSED);
        }
        let shown = own_part(&st.borrow().closed_parts, control);
        let hidden = || {
            own_part(&st.borrow().hidden_parts, control)
                .filter(|_| !(self.part_eliminated(st, control) || self.off_page(st, control)))
        };
        let found = match by {
            // A page the test opened and closed with TestPage.Close(), held in a variable (the sender of its own event a
            // subscriber kept): BC raises no error and the call reaches the page's own part instance, its row and its globals
            // as they were, in the same test and in a later test of the codeunit, also from a factbox event while another
            // TestPage of the same page opens; a part a Visible expression hid answers the same (pages/parts/
            // HeldSenderPartAccess.al:286-307 HS01 `closed.touch` 2, `closed.read`, `closed.mark`; :311-323 HS02; :328-346
            // HS03; :762-789 HS08 and :881-908 HS12, hidden and shown part).
            Some(ClosedBy::TestClose) => {
                let found = shown.or_else(hidden);
                if let Some(c) = &found {
                    self.mark_held_part_rec(c);
                }
                found
            }
            // After its RunModal returned (closed with the OK or Cancel action), the page's code reaches a part that was
            // shown: a procedure of the part reads the rows the part's record holds (pages/parts/WdPartAfterRunModal.al:75-100
            // WD_Part_ReadAfterRunModalOK `count` 1 `value` A, WD_Part_ReadAfterRunModalCancel `count` 1 `value` B;
            // corpus branch al-runtime-rs-staging-1030c, al-runner issue 6321). Posted Purchase Document Lines.CopyLineToDoc
            // reads `CurrPage.PostedRcpts.PAGE` after its RunModal returned.
            // A part a Visible expression hid while the page was open (it opened with the page) hands out its row at close
            // too (pages/parts/Y37PartAfterClose.al:376-395 LT_C314_AfterRunModalOK_OtherPartShown: part A, hidden when an
            // option switched to part B, gives its row 10000). A handler that does nothing leaves the close to the platform,
            // and the part answers the same (pages/parts/NeverRanReadRec.al:101 P182b_HostRanAndClosed_PartReadRec
            // `id=1 name=one`).
            Some(ClosedBy::Action | ClosedBy::PageCode | ClosedBy::Platform) if modal_run => shown.or_else(hidden),
            // A page that closed itself with CurrPage.Close in a trigger or an action, opened by a TestPage, and whose code goes
            // on to call a procedure of a part it showed: the part answers, its globals as its OnClosePage left them
            // (pages/parts/Y37PartAfterClose.al:652 CLP_AGCR_CloseAllowed_PartAccess_Observe, :668
            // CLP_Action_CloseAllowed_PartAccess_Observe: `PART.CLOSE;PARENT.CLOSE;AFTER.CLOSE;BEFORE.PART;PART.TOUCH:1042:1`,
            // corpus PR 1205).
            Some(ClosedBy::PageCode) => shown,
            _ => None,
        };
        found.map_or_else(|| unsupported(REFUSED), Ok)
    }

    /// The part's Rec answers its fields and `Count()` only (`records::HELD_PART_REC`); its xRec answers nothing
    /// (`records::HELD_PART_XREC`).
    fn mark_held_part_rec(&mut self, part: &St) {
        let Some(inst) = part.borrow().instance.clone() else {
            return;
        };
        if !self.has_source(part) {
            return;
        }
        let o = &self.engine.program.objects[part.borrow().object as usize];
        for (slot, why) in [
            (o.rec_slot, crate::records::HELD_PART_REC),
            (o.xrec_slot, crate::records::HELD_PART_XREC),
        ] {
            let Some(slot) = slot else { continue };
            if let Value::Record(r) = &inst.borrow().globals[slot] {
                let mut r = r.borrow_mut();
                if r.unpinned_buffer.is_none() {
                    r.unpinned_buffer = Some(why);
                }
            }
        }
    }

    /// Refuses a page method (CurrPage.Update, SetRecord, SaveRecord, Close, ...) on a part reached through a page the test
    /// client closed: HeldSenderPartAccess.al calls only procedures of the part.
    pub(super) fn refuse_held_part_page_method(&self, st: &St, method: PageOp) -> Result<(), Signal> {
        let held = {
            let s = st.borrow();
            !s.open
                && s.host
                    .as_ref()
                    .and_then(|(h, _)| h.upgrade())
                    .is_some_and(|h| !h.borrow().open && h.borrow().closed_by == Some(ClosedBy::TestClose))
        };
        if held {
            return unsupported(format!(
                "{method} on a part of a page the test client closed (only the part's procedures are measured there)"
            ));
        }
        Ok(())
    }

    /// TestPage.OpenEdit, OpenView or OpenNew on a variable whose page a Page variable of a SingleInstance codeunit still
    /// holds (a subscriber kept the page's sender) after TestPage.Close(): BC opens a new page instance for the variable and
    /// the holder keeps the old one with its parts (HeldSenderPartAccess.al:734-752 HS07 and :793-818 HS09 measure it with
    /// another TestPage variable: `h1.upd;f2.refresh(id=1,n=2)` while `h3` is open; :328-346 HS03 reopens the same variable
    /// but does not tell the two pages apart). For the same variable this is the assumption
    /// `held-page-part-after-testpage-close`. The variable gets a fresh page, as `Clear` gives it.
    /// Only such a holder counts: a copy of the TestPage value (a by-value parameter of a helper that opens it, as
    /// Microsoft's SalesStockout OpenSalesOrderPageByNo does) shares the page with the caller's variable. With a holder, a
    /// reopen through such a copy gives only the copy the new page (not measured; listed in the assumption).
    pub(super) fn fresh_page_for_held_reopen(
        &mut self,
        method: PageOp,
        page: &mut al_builtins::page_handle::PageHandle,
    ) {
        if !matches!(
            method.method(),
            PageMethod::OpenEdit | PageMethod::OpenView | PageMethod::OpenNew
        ) {
            return;
        }
        let st = st_of(page).clone();
        let object = {
            let s = st.borrow();
            if s.open || !s.ran || s.trapping || s.host.is_some() || s.closed_by != Some(ClosedBy::TestClose) {
                return;
            }
            s.object
        };
        if !self.held_by_single_instance_page_var(&st) {
            return;
        }
        assume(Assumption::HeldPagePartAfterTestPageClose);
        *page = state::closed_page(object);
    }

    /// Whether a Page-typed global of one of the session's SingleInstance codeunits holds the page `st`.
    fn held_by_single_instance_page_var(&self, st: &St) -> bool {
        let program = &self.engine.program;
        self.singletons.iter().any(|(object, inst)| {
            let inst = inst.borrow();
            program.objects[*object]
                .globals
                .iter()
                .zip(inst.globals.iter())
                .any(|(slot, g)| {
                    matches!(slot.ty, al_builtins::value::RtType::Page(_))
                        && matches!(g, Value::TestPage(h) if Rc::ptr_eq(st_of(h), st))
                })
        })
    }
}

/// The page of part control `control` in `parts`.
fn own_part(parts: &[(usize, St)], control: usize) -> Option<St> {
    parts.iter().find(|(i, _)| *i == control).map(|(_, c)| c.clone())
}
