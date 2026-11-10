//! `Clear` of a TestPage variable: the variable then holds a fresh, closed page (`al_builtins::native::values::clear` asks
//! `Ctx::clear_test_page`). One that is open or waits for `Trap()` is closed first.

use super::*;

impl Session {
    /// `Clear` of a TestPage or Page variable that holds `page`: the fresh, closed page of the same object the variable holds
    /// afterwards (corpus testpage/TestPageEditable_Tests.al:93 EditableBoundToPageVariable_FollowsTheRow). A page that is
    /// open, waits for Trap() or is a part of a host goes through `clear_test_page` first. `al_builtins::native::values::
    /// clear` did the first and last step itself before the page state moved to this crate (ADR-072 decision 3).
    pub(crate) fn cleared_test_page(
        &mut self,
        page: &al_builtins::page_handle::PageHandle,
    ) -> Result<al_builtins::page_handle::PageHandle, Signal> {
        let st = st_of(page);
        let busy = {
            let s = st.borrow();
            s.open || s.trapping || s.host.is_some()
        };
        if busy {
            self.clear_test_page(st)?;
        }
        let object = st.borrow().object;
        Ok(state::closed_page(object))
    }

    /// Closes an open TestPage without running anything and drops its trap, before `Clear` gives the variable a fresh page.
    ///
    /// Pinned: an open card cleared runs nothing, reading or closing the variable then raises TestPageNotOpened and OpenEdit
    /// runs OnInit again (pages/controls/W2PageVariableInit.al:234-264 Clear_OpenTestPage, pages/lifecycle/
    /// PageVariableLifecycle.al:154-195 TestPageVar_ClearClosedOpenAndReopen, testpage/AnTestPageTests.al:309-337
    /// An10_ClearPartTestPage for a ListPart opened by OpenView); a trapping one cleared does not catch the next Page.Run, which
    /// then has no handler (W2PageVariableInit.al:266-291 Clear_TrappingTestPage).
    ///
    /// Not pinned, refused: a page that declares OnQueryClosePage or OnClosePage (none of those pages does), one with an unsaved
    /// value, a modal or trapped page, a page with parts, and a part of a host.
    pub(crate) fn clear_test_page(&mut self, st: &St) -> Result<(), Signal> {
        let (open, unpinned) = {
            let s = st.borrow();
            (
                s.open,
                s.host.is_some() || (s.open && (s.dirty || s.modal || s.caught_by_trap || !s.parts.is_empty())),
            )
        };
        if unpinned
            || (open
                && ["onqueryclosepage", "onclosepage"]
                    .iter()
                    .any(|t| self.declares_or_ext_declares(st, t)))
        {
            return Err(Signal::Unsupported(
                "Clear of a TestPage that is open, trapping or a part (not pinned)".into(),
            ));
        }
        if st.borrow().trapping {
            st.borrow_mut().trapping = false;
            self.pages.traps.retain(|t| !Rc::ptr_eq(t, st));
        }
        if open {
            self.mark_closed(st);
        }
        Ok(())
    }
}
