//! How a page that AL code opened with a non-modal `Page.Run` (the static `Page.Run(Id, Rec)` or a Page variable's `Run()`)
//! and a `[PageHandler]` answered closes when a close trigger or the save of its row fails (al-runner issue 7480; corpus
//! pages/close-errors/CemPageHandlerTests.al, codeunit 99522, a Card, a List and a NavigatePage).
//!
//! The pins, by how the page closes:
//! - The handler returned and the platform closes the page: an error of OnQueryClosePage, of OnClosePage (an AL error or a
//!   Confirm no handler takes) and of the save of the edited row does not reach the code that ran the page. The trigger stops
//!   at the error, the page closes, `Page.Run` returns (`_HandlerReturns_ClosePageError`, `_HandlerReturns_ClosePageConfirmUnhandled`,
//!   `_HandlerReturns_QueryCloseError` `returned=Q-card OK;`, `_HandlerTableRefusesReturns_NoError` `returned=M-card FAILMOD;
//!   T-mod R1 FAILMOD;` with the row as it was).
//! - The handler raised an error (its own, or `Cancel` that a Card, List or NavigatePage does not offer): the platform closes the
//!   page all the same, with the same rule for its errors, and `Page.Run` raises the handler's error (`_HandlerError_NoError`
//!   `CEM handler error` `log=Q-card OK; C-card;`, `_HandlerCancel_NoError` `TestActionNotFound`,
//!   `WizardRun_HandlerError_ClosePageConfirmUnhandled`).
//! - OK or `TestPage.Close()` in the handler: an error of OnClosePage comes out wrapped, as at `TestPage.Close()`
//!   (`_HandlerOk_ClosePageError` `One or more errors occurred while closing the page.\\CEM close page error` `TestWrapped`,
//!   `_HandlerClose_ClosePageConfirmUnhandled` `...page.\\Unhandled UI: Confirm CEM close confirm?`).
//!
//! Pinned only for a page with no part, no page extension and no subscriber to its events (the CEM pages); a write to the
//! database by the failing trigger before its error is not pinned (the CEM triggers write none). What the last error is after
//! a swallowed error is not pinned: it refuses to be read until the next error replaces it.

use super::*;

/// The refusal of a failed save at the platform's close after the handler returned, for every page but a pinned one.
pub(super) const SAVE_FAILED_AT_PLATFORM_CLOSE: &str = "an error raised while the page saved its edited row as the platform closed it after the handler returned (not pinned)";
/// The refusal of an OnQueryClosePage that refused the platform's close, for every page but a pinned (or modal) one.
pub(super) const QUERY_CLOSE_REFUSED: &str = "a page whose OnQueryClosePage refused the platform's close";

impl Session {
    /// Starts a run of page `st` that a handler answers: modal for `RunModal`, a non-modal `Page.Run` by AL code otherwise
    /// (a trapped page is the test's own).
    pub(super) fn begin_handled_run(&self, st: &St, modal: bool, trapped: bool) {
        let mut s = st.borrow_mut();
        s.modal = modal && !trapped;
        s.page_run_by_code = !modal && !trapped;
    }

    /// Whether page `st` is a non-modal `Page.Run` by AL code in the shape the corpus pins (module doc).
    pub(super) fn page_run_close_pinned(&self, st: &St) -> bool {
        let engine = self.engine();
        let s = st.borrow();
        let object = s.object as usize;
        let page = page_of(&engine, st);
        s.page_run_by_code
            && !s.modal
            && !s.lookup
            && !s.opened_by_test
            && s.host.is_none()
            && s.parts.is_empty()
            && s.hidden_parts.is_empty()
            && matches!(
                engine.object_facts[object].page_type_lower(),
                "card" | "list" | "navigatepage"
            )
            && page.extensions.is_empty()
            && page.ext_triggers.is_empty()
            && !engine.events.any_subscriber_of(object)
    }

    /// An error of OnClosePage `r` of a pinned page (module doc): `Some(Ok)` when the platform closes the page (the error is
    /// dropped and the close goes on), `Some(Err)` with the wrapped error when an action closes it. `changes` is the store's
    /// change counter before the trigger ran. `None` for every other page and error.
    pub(super) fn page_run_close_page_error(
        &mut self,
        st: &St,
        r: &Result<Option<Value>, Signal>,
        platform: bool,
        changes: u64,
    ) -> Option<Result<(), Signal>> {
        let Err(Signal::Error(e)) = r else {
            return None;
        };
        if !matches!(e.code.as_str(), "Dialog" | "MissingUIHandler")
            || !self.page_run_close_pinned(st)
            || self.store_mut().change_counter() != changes
        {
            return None;
        }
        if platform {
            self.last_error = al_builtins::native::unpinned_error();
            return Some(Ok(()));
        }
        self.mark_closed(st);
        Some(Err(error_code(
            format!("{}\\\\{}", super::relation_lookup_close::CLOSING_ERRORS, e.message),
            "TestWrapped",
        )))
    }

    /// The platform's close after the handler returned left a pinned page open: OnQueryClosePage or the save of the row
    /// failed. The page closes and `Page.Run` returns (module doc). `None` for every other page.
    pub(super) fn page_run_platform_close_failed(&mut self, st: &St) -> Option<Result<Value, Signal>> {
        if !self.page_run_close_pinned(st) {
            return None;
        }
        self.last_error = al_builtins::native::unpinned_error();
        self.mark_closed(st);
        Some(Ok(Value::Null))
    }

    /// [`Self::page_run_platform_close_failed`], or the refusal `reason` for every other page.
    pub(super) fn platform_close_failed_or_refuse(&mut self, st: &St, reason: &str) -> Result<Value, Signal> {
        self.page_run_platform_close_failed(st)
            .unwrap_or_else(|| unsupported(reason))
    }

    /// A handler of a pinned page raised an error and left the page open: the platform closes it as after a return, and the
    /// caller raises the handler's error (module doc). A page with an unsaved row is refused (not pinned). Every other page
    /// keeps its close: `close_modal_after_handler_error` for a modal one, none for the rest.
    pub(super) fn close_page_after_handler_error(&mut self, st: &St, e: &Signal, modal: bool) -> Result<(), Signal> {
        if modal {
            return self.close_modal_after_handler_error(st, e);
        }
        if !self.page_run_close_pinned(st) {
            return Ok(());
        }
        if st.borrow().dirty || st.borrow().draft {
            return unsupported(
                "a page left open by a PageHandler that raised an error, with an unsaved row (not pinned)",
            );
        }
        let a = self.unattended_action(st);
        let changes = self.store_mut().change_counter();
        let r = self.close_with(st, a, true, false);
        if self.store_mut().change_counter() != changes {
            return unsupported(
                "a page left open by a PageHandler that raised an error, whose close wrote to the database (not pinned)",
            );
        }
        match r {
            Ok(_) | Err(Signal::Error(_)) => {
                if st.borrow().open {
                    self.page_run_platform_close_failed(st);
                }
                Ok(())
            }
            Err(other) => Err(other),
        }
    }

    /// An error `e` of OnQueryClosePage of a pinned page (module doc). At the platform's close the page closes without
    /// OnClosePage and `Page.Run` returns (`_HandlerReturns_QueryCloseError` `returned=Q-card OK;`). At OK in the handler the
    /// error is shown as a message no handler takes, the page closes, and that error comes out with the code
    /// `TestWrapped:MissingUIHandler` (`_HandlerOk_QueryCloseError` `Unhandled UI: Message CEM query close error`); with a
    /// MessageHandler it is not pinned. `None` for every other page and close.
    pub(super) fn page_run_query_close_error(
        &mut self,
        st: &St,
        e: &al_builtins::native::AlError,
        platform: bool,
        via_action: bool,
    ) -> Option<Result<Value, Signal>> {
        if !(platform || via_action) || !self.page_run_close_pinned(st) {
            return None;
        }
        if platform {
            st.borrow_mut().close_error = true;
            return self.page_run_platform_close_failed(st);
        }
        let shown = self.ui_message(e.message.clone());
        self.mark_closed(st);
        Some(match shown {
            Err(Signal::Error(m)) if m.code == "MissingUIHandler" => {
                Err(error_code(m.message, "TestWrapped:MissingUIHandler"))
            }
            Err(other) => Err(other),
            Ok(()) => unsupported(
                "an error of OnQueryClosePage at OK in a PageHandler of Page.Run, taken by a MessageHandler (not pinned)",
            ),
        })
    }

    /// The save of the edited row failed at OK or Cancel that a PageHandler of a pinned page invoked: BC then logs neither
    /// close trigger and `Page.Run` returns without an error (`CardStaticRun_HandlerEditOk_PageModifyRecordError`
    /// `returned=M-card EDIT;`); which close that is, is not pinned, so it is refused.
    pub(super) fn refuse_page_run_save_error_at_action(&self, st: &St, via_action: bool) -> Result<(), Signal> {
        if via_action && self.page_run_close_pinned(st) {
            return unsupported(
                "an error raised while the page saved its edited row at an action of a PageHandler of Page.Run (not pinned)",
            );
        }
        Ok(())
    }
}
