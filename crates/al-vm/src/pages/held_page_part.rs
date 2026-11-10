//! What the code of a page that ran and is closed reaches through `CurrPage.<part>.Page`: the page's own part instances,
//! which its state keeps after the close (`PageState::closed_parts` for the shown parts, `hidden_parts` for the ones a
//! Visible expression hid). Which closes are pinned decides whether the part answers or the call is refused:
//! - after a modal run (OK or Cancel): pages/parts/WdPartAfterRunModal.al, Y37PartAfterClose.al;
//! - after the page closed itself with `CurrPage.Close`: Y37PartAfterClose.al;
//! - after the test client closed a page the test opened (`TestPage.Close`, or the close at the end of the test):
//!   assumption `held-page-part-after-testpage-close` (al-runner issue 6924).

use super::*;
use al_builtins::assumptions::{Assumption, assume};

impl Session {
    /// The test client closes `st` (`TestPage.Close`, the close at the end of the test).
    pub(super) fn mark_closed_by_test(&mut self, st: &St) {
        st.borrow_mut().closed_by_test = Some(self.pages.test_methods);
        self.mark_closed(st);
    }

    /// The part page of control `control` of `st`, a page that is not open, as the page's own code reaches it.
    pub(super) fn closed_page_part(&mut self, st: &St, control: usize) -> Result<St, Signal> {
        const REFUSED: &str =
            "a part of a page that is not open, reached by the code of that page (what it does then is not pinned)";
        let (ran, modal_run, by_test, test_opened) = {
            let s = st.borrow();
            (s.ran, s.last_run_modal == Some(true), s.closed_by_test, s.opened_by_test)
        };
        if !ran {
            return unsupported(REFUSED);
        }
        let shown = own_part(&st.borrow().closed_parts, control);
        let hidden = || {
            own_part(&st.borrow().hidden_parts, control)
                .filter(|_| !(self.part_eliminated(st, control) || self.off_page(st, control)))
        };
        if let Some(closed_in) = by_test {
            // A page the test opened and the test client closed (TestPage.Close, or the close at the end of the test), held
            // in a variable (the sender of its own event a subscriber kept): BC raises no error and the call reaches the
            // page's own part instance, its row and its globals as they were, also from the next test and while another
            // page of the same object is open; a part a Visible expression hid answers the same (corpus branch
            // al-runtime-rs-staging-probe-train-1110b, pages/parts/HeldSenderPartAccess.al HS06-HS13, measured on BC
            // 27.5, 28.5 and 29.0; al-runner issue 6924).
            if test_opened {
                let Some(c) = shown.or_else(hidden) else {
                    return unsupported(REFUSED);
                };
                assume(Assumption::HeldPagePartAfterTestPageClose);
                if closed_in != self.pages.test_methods {
                    self.held_part_in_later_test(&c);
                }
                return Ok(c);
            }
            // A page AL code ran modally and a handler closed with TestPage.Close: only a part a Visible expression hid
            // answers, as after the modal run below; a shown part is not pinned.
            if modal_run {
                return hidden().map_or_else(|| unsupported(REFUSED), Ok);
            }
            return unsupported(REFUSED);
        }
        if modal_run {
            // After its RunModal returned, the page's code reaches a part that was shown: a procedure of the part reads the
            // rows the part's record holds, after OK and after Cancel alike (pages/parts/WdPartAfterRunModal.al:75-100
            // WD_Part_ReadAfterRunModalOK `count` 1 `value` A, WD_Part_ReadAfterRunModalCancel `count` 1 `value` B;
            // corpus branch al-runtime-rs-staging-1030c, al-runner issue 6321). Posted Purchase Document Lines.CopyLineToDoc
            // reads `CurrPage.PostedRcpts.PAGE` after its RunModal returned.
            // A part a Visible expression hid while the page was open (it opened with the page) hands out its row at close
            // too (pages/parts/Y37PartAfterClose.al:376-395 LT_C314_AfterRunModalOK_OtherPartShown: part A, hidden when an
            // option switched to part B, gives its row 10000).
            return shown.or_else(hidden).map_or_else(|| unsupported(REFUSED), Ok);
        }
        // A page that closed itself with CurrPage.Close in a trigger or an action, opened by a TestPage, and whose code goes on
        // to call a procedure of a part it showed: the part answers, its globals as its OnClosePage left them
        // (pages/parts/Y37PartAfterClose.al:652 CLP_AGCR_CloseAllowed_PartAccess_Observe, :668
        // CLP_Action_CloseAllowed_PartAccess_Observe: `PART.CLOSE;PARENT.CLOSE;AFTER.CLOSE;BEFORE.PART;PART.TOUCH:1042:1`,
        // corpus PR 1205).
        shown.map_or_else(|| unsupported(REFUSED), Ok)
    }
}

impl Session {
    /// In a test after the one that closed its host, the part's row and globals answer as they were (HS02 `next-test.touch`
    /// 3, `read` id=1 name=one; HS07 `f2.refresh(id=1,n=2)`), but its Rec counts 0 rows with the row in the table (HS02
    /// `count=0`): every method of the part's Rec refuses there (`records::HELD_PART_LATER_TEST`).
    fn held_part_in_later_test(&mut self, part: &St) {
        if part.borrow().instance.is_none() || !self.has_source(part) {
            return;
        }
        let rec = self.page_rec(part);
        let mut r = rec.borrow_mut();
        if r.unpinned_buffer.is_none() {
            r.unpinned_buffer = Some(crate::records::HELD_PART_LATER_TEST);
        }
    }
}

/// The page of part control `control` in `parts`.
fn own_part(parts: &[(usize, St)], control: usize) -> Option<St> {
    parts.iter().find(|(i, _)| *i == control).map(|(_, c)| c.clone())
}
