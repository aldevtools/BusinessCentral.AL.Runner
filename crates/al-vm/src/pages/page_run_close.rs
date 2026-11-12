//! How a page that AL code opened with a non-modal `Page.Run` and a `[PageHandler]` answered closes when a close trigger or
//! the save of its row fails (al-runner issue 7480; corpus pages/close-errors/CemPageHandlerTests.al, codeunit 99522, over the
//! pages of CemFixtures.al).
//!
//! The rule runs only in the cells the corpus measured (`cell_pinned`): the page and how it was run (`PinnedRun`), how the
//! close was asked (`CloseBy`), the row at the close (`RowAtClose`) and the error (`CloseError`). What each close does:
//! - The platform closes the page after the handler returned: the error does not reach the code that ran the page. The
//!   trigger stops at the error, the page closes, `Page.Run` returns.
//! - The handler raised an error (its own, or `Cancel` that these pages do not offer): the platform closes the page all the
//!   same, with the same rule for its errors, and `Page.Run` raises the handler's error.
//! - OK in the handler: an error of OnClosePage comes out wrapped as at `TestPage.Close()`; an error of OnQueryClosePage is
//!   shown as a message no handler takes, and that error comes out with the code `TestWrapped:MissingUIHandler`.
//! - `TestPage.Close()` in the handler: an unhandled Confirm of OnClosePage comes out wrapped (`page_close`).
//!
//! Every error source is the one the fixtures raise, before any write to the database: an `Error` with a literal text
//! (CemFixtures.al:153, :163, :174; the table's OnModify, :51) or a `Confirm` no handler takes (:165). What the last error
//! is after a dropped error is not pinned: it refuses to be read until the next error replaces it. Every other page, run,
//! close, row and error keeps the close it had.

use super::*;
use crate::ui::UiKind;
use al_builtins::native::{AlError, ExceptionKind};

/// The refusal of a failed save at the platform's close of a pinned page whose error is not a pinned one.
const SAVE_FAILED: &str = "an error raised while the page saved its edited row as the platform closed it after the handler returned (not pinned for this error)";

/// A non-modal `Page.Run` by AL code that a PageHandler answers (`PageState::page_run`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageRun {
    /// A Page variable's `Run()`, not the static `Page.Run(Id, Rec)`.
    pub by_variable: bool,
    pub phase: PageRunPhase,
}

/// Where the run of a `PageRun` page is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageRunPhase {
    /// The page is open: before or inside its handler.
    Open,
    /// The handler returned and left the page open: the platform closes it.
    Returned,
    /// The handler raised an error and left the page open: the platform closes it.
    HandlerFailed,
}

/// The page and run of the pins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PinnedRun {
    /// `Page.Run(Page::"CEM Card", Row)` (CemFixtures.al:476-479).
    CardStatic,
    /// `CardPage.SetRecord(Row); CardPage.Run()` (CemFixtures.al:481-487).
    CardVariable,
    /// `Page.Run(Page::"CEM List", Row)` (CemFixtures.al:497-500).
    ListStatic,
    /// `Wizard.SetRecord(Row); Wizard.Run()` of a NavigatePage (CemFixtures.al:489-495).
    NavigateVariable,
}

/// How the close was asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CloseBy {
    /// The platform, after the handler returned.
    Returned,
    /// The platform, after the handler raised an error.
    HandlerFailed,
    /// `OK().Invoke()` in the handler.
    Ok,
    /// `TestPage.Close()` in the handler.
    TestClose,
}

/// The row of the page when the close starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RowAtClose {
    /// Nothing typed (handler modes 0, 1, 2, 3, 7).
    Unchanged,
    /// A field of the existing row typed, not saved (modes 4, 5, 8: `P.Name.SetValue(...)`).
    Edited,
}

/// The error a close trigger or the save raised.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CloseError {
    /// OnQueryClosePage raised `Error('CEM query close error')` (CemFixtures.al:150-155).
    Query,
    /// OnClosePage raised `Error('CEM close page error')` (:163).
    ClosePage,
    /// OnClosePage asked `Confirm('CEM close confirm?')` and no handler took it (:165).
    ClosePageConfirm,
    /// The save of the edited row: the page's OnModifyRecord raised `Error('CEM page modify error')` in the codeunit it calls
    /// (:174, :206-210), or the table's OnModify raised `Error('CEM table modify refused')` (:45-52).
    Save,
}

/// The facts of a close of a pinned page and run, taken when the close starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PageRunClose {
    run: PinnedRun,
    pub(super) by: CloseBy,
    row: RowAtClose,
}

/// Whether the corpus measured `error` at the close `by` of `run` with `row` (CemPageHandlerTests.al, the line of each cell).
fn cell_pinned(c: PageRunClose, error: Option<CloseError>) -> bool {
    use CloseBy as B;
    use CloseError as E;
    use PinnedRun as P;
    use RowAtClose as R;
    match (c.by, error, c.run, c.row) {
        // The handler returned: :255 CardStaticRun_HandlerReturns_QueryCloseError, :391 _HandlerEditReturns_QueryCloseError,
        // :888 CardVariableRun_HandlerReturns_QueryCloseError, :970 _HandlerEditReturns_QueryCloseError, :1219
        // ListStaticRun_HandlerReturns_QueryCloseError, :1576 WizardRun_HandlerReturns_QueryCloseError.
        (B::Returned, Some(E::Query), P::CardStatic | P::CardVariable, R::Unchanged | R::Edited)
        | (B::Returned, Some(E::Query), P::ListStatic | P::NavigateVariable, R::Unchanged) => true,
        // :474, :998, :1328, :1631 _HandlerReturns_ClosePageError; :583, :1053, :1383, :1686 _HandlerReturns_ClosePageConfirmUnhandled.
        (B::Returned, Some(E::ClosePage | E::ClosePageConfirm), _, R::Unchanged) => true,
        // :692 CardStaticRun_HandlerEditReturns_PageModifyRecordError, :227 _HandlerTableRefusesReturns_NoError, :446
        // _HandlerTableRefusesReturns_QueryCloseError, :748 _HandlerTableRefusesReturns_PageModifyRecordError.
        (B::Returned, Some(E::Save), P::CardStatic, R::Edited) => true,
        // The handler raised an error: :144 CardStaticRun_HandlerError_NoError, :89 _HandlerCancel_NoError, :1192
        // ListStaticRun_HandlerError_NoError, :1549 WizardRun_HandlerError_NoError, :1494 WizardRun_HandlerCancel_NoError.
        (B::HandlerFailed, None, P::CardStatic | P::ListStatic | P::NavigateVariable, R::Unchanged) => true,
        // :364 CardStaticRun_HandlerError_QueryCloseError, :310 _HandlerCancel_QueryCloseError, :1301 ListStaticRun_HandlerError_QueryCloseError.
        (B::HandlerFailed, Some(E::Query), P::CardStatic | P::ListStatic, R::Unchanged) => true,
        // :1795 WizardRun_HandlerError_ClosePageConfirmUnhandled, :1741 WizardRun_HandlerCancel_ClosePageConfirmUnhandled.
        (B::HandlerFailed, Some(E::ClosePageConfirm), P::NavigateVariable, R::Unchanged) => true,
        // OK: :283, :502, :611 CardStaticRun_HandlerOk_*; :419, :556, :665 _HandlerEditOk_*; :916, :1026, :1081
        // CardVariableRun_HandlerOk_*; :1247, :1356, :1411 ListStaticRun_HandlerOk_*; :1604, :1659, :1714 WizardRun_HandlerOk_*
        // (QueryCloseError, ClosePageError, ClosePageConfirmUnhandled).
        (B::Ok, Some(E::Query | E::ClosePage | E::ClosePageConfirm), P::CardStatic, R::Unchanged | R::Edited)
        | (B::Ok, Some(E::Query | E::ClosePage | E::ClosePageConfirm), _, R::Unchanged) => true,
        // TestPage.Close(): :638 CardStaticRun_HandlerClose_ClosePageConfirmUnhandled, :1768 WizardRun_HandlerClose_ClosePageConfirmUnhandled.
        (B::TestClose, Some(E::ClosePageConfirm), P::CardStatic | P::NavigateVariable, R::Unchanged) => true,
        _ => false,
    }
}

/// `e` is an `Error` with a literal text (the fixtures' form, `ExceptionKind::Dialog`).
fn literal_error(e: &AlError) -> bool {
    e.code == "Dialog" && e.exception == ExceptionKind::Dialog
}

impl Session {
    /// Starts a run of page `st` that a handler answers. A modal run or a trapped page is no `PageRun`; a non-modal one keeps
    /// what `note_page_run` noted.
    pub(super) fn begin_handled_run(&self, st: &St, modal: bool, trapped: bool) {
        let mut s = st.borrow_mut();
        s.modal = modal && !trapped;
        if modal || trapped {
            s.page_run = None;
        }
    }

    /// AL code runs page `st` without a modal: the static `Page.Run(Id, Rec)` or a Page variable's `Run()` (`by_variable`).
    pub(crate) fn note_page_run(st: &St, by_variable: bool) {
        st.borrow_mut().page_run = Some(PageRun {
            by_variable,
            phase: PageRunPhase::Open,
        });
    }

    /// The pinned page and run of `st` (CemFixtures.al:185-500): a Card, List or NavigatePage over a database table, run by AL
    /// code as the pins run it, not modal, not a lookup, not opened by the test, with no host, no part, no page extension and
    /// no subscriber to its events.
    fn pinned_run(&self, st: &St) -> Option<PinnedRun> {
        let engine = self.engine();
        let s = st.borrow();
        let run = s.page_run?;
        let object = s.object as usize;
        let page = page_of(&engine, st);
        let shape = !s.modal
            && !s.lookup
            && !s.opened_by_test
            && s.host.is_none()
            && s.parts.is_empty()
            && s.hidden_parts.is_empty()
            && page.extensions.is_empty()
            && page.ext_triggers.is_empty()
            && !engine.events.any_subscriber_of(object)
            && !engine.program.objects[object]
                .properties
                .iter()
                .any(|(k, _)| k.eq_ignore_ascii_case("SourceTableTemporary"));
        drop(s);
        if !shape || !self.has_source(st) || self.page_rec(st).borrow().temporary {
            return None;
        }
        match (engine.object_facts[object].page_type_lower(), run.by_variable) {
            ("card", false) => Some(PinnedRun::CardStatic),
            ("card", true) => Some(PinnedRun::CardVariable),
            ("list", false) => Some(PinnedRun::ListStatic),
            ("navigatepage", true) => Some(PinnedRun::NavigateVariable),
            _ => None,
        }
    }

    /// The facts of the close of page `st` that `close_with` (`action`, `via_action`, `test_close`) starts, for a pinned page
    /// and run with a pinned row; `None` for every other close.
    pub(super) fn page_run_close(
        &self,
        st: &St,
        action: &str,
        via_action: bool,
        test_close: bool,
    ) -> Option<PageRunClose> {
        let run = self.pinned_run(st)?;
        let phase = st.borrow().page_run?.phase;
        let by = match (via_action, test_close, phase) {
            (true, false, PageRunPhase::Open) if action == "OK" => CloseBy::Ok,
            (false, true, PageRunPhase::Open) => CloseBy::TestClose,
            (false, false, PageRunPhase::Returned) => CloseBy::Returned,
            (false, false, PageRunPhase::HandlerFailed) => CloseBy::HandlerFailed,
            _ => return None,
        };
        let s = st.borrow();
        let existing_row = !s.draft && s.pending.is_none() && s.cursor == Cursor::Row && s.mode == PageMode::Edit;
        let row = match (s.dirty, existing_row) {
            (false, true) => RowAtClose::Unchanged,
            (true, true) => RowAtClose::Edited,
            _ => return None,
        };
        Some(PageRunClose { run, by, row })
    }

    /// An error `e` of OnQueryClosePage in the close `c` (`changes`: the store's change counter before the trigger). At the
    /// platform's close the page closes without OnClosePage and `Page.Run` returns (CemPageHandlerTests.al:255
    /// `returned=Q-card OK;`); at OK the error is shown as a message no handler takes and comes out with the code
    /// `TestWrapped:MissingUIHandler` (:283 `Unhandled UI: Message CEM query close error`). `None` for every other error and close.
    pub(super) fn page_run_query_close_error(
        &mut self,
        c: Option<PageRunClose>,
        st: &St,
        e: &AlError,
        changes: u64,
    ) -> Option<Result<Value, Signal>> {
        let c = c?;
        // No pin binds a MessageHandler (every test of CemPageHandlerTests.al binds only its PageHandler): at the platform's
        // close one is not measured; at OK one that takes the message is refused below.
        let pinned = literal_error(e)
            && self.store_mut().change_counter() == changes
            && (c.by == CloseBy::Ok || !self.has_ui_handler(UiKind::Message))
            && cell_pinned(c, Some(CloseError::Query));
        if !pinned {
            return None;
        }
        if c.by != CloseBy::Ok {
            return Some(Ok(self.page_run_closed_without_error(st)));
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

    /// The error of OnClosePage `r`, when it is a pinned one: an `Error` with a literal text or an unhandled Confirm, with no
    /// write since `changes`.
    fn close_page_error_of(&mut self, r: &Result<Option<Value>, Signal>, changes: u64) -> Option<CloseError> {
        let Err(Signal::Error(e)) = r else {
            return None;
        };
        if self.store_mut().change_counter() != changes {
            return None;
        }
        if literal_error(e) {
            Some(CloseError::ClosePage)
        } else if self.unhandled_ui_is(e, UiKind::Confirm) {
            Some(CloseError::ClosePageConfirm)
        } else {
            None
        }
    }

    /// An error of OnClosePage `r` in the close `c` (`changes`: the store's change counter before the trigger): `Some(Ok)` when
    /// the platform closes the page (the error is dropped and the close goes on, CemPageHandlerTests.al:474 `returned=Q-card OK;
    /// C-card;`), `Some(Err)` with the wrapped error at OK (:502 `One or more errors occurred while closing the page.\\CEM close
    /// page error` `TestWrapped`). `None` for every other error and close.
    pub(super) fn page_run_close_page_error(
        &mut self,
        c: Option<PageRunClose>,
        st: &St,
        r: &Result<Option<Value>, Signal>,
        changes: u64,
    ) -> Option<Result<(), Signal>> {
        let c = c?;
        let error = self.close_page_error_of(r, changes)?;
        if c.by == CloseBy::TestClose || !cell_pinned(c, Some(error)) {
            return None;
        }
        let Err(Signal::Error(e)) = r else {
            return None;
        };
        if c.by != CloseBy::Ok {
            self.last_error = al_builtins::native::unpinned_error();
            return Some(Ok(()));
        }
        self.mark_closed(st);
        Some(Err(error_code(
            format!("{}\\\\{}", super::relation_lookup_close::CLOSING_ERRORS, e.message),
            "TestWrapped",
        )))
    }

    /// Whether the error `r` of OnClosePage at `TestPage.Close()` in the close `c` (`changes`: the store's change counter
    /// when the close started) is an unhandled Confirm the corpus pins there (CemPageHandlerTests.al:638
    /// `...page.\\Unhandled UI: Confirm CEM close confirm?` `TestWrapped`).
    pub(super) fn page_run_test_close_confirm(&mut self, c: Option<PageRunClose>, e: &AlError, changes: u64) -> bool {
        let r = Err(Signal::Error(Box::new(e.clone())));
        c.is_some_and(|c| {
            c.by == CloseBy::TestClose
                && self.close_page_error_of(&r, changes) == Some(CloseError::ClosePageConfirm)
                && cell_pinned(c, Some(CloseError::ClosePageConfirm))
        })
    }

    /// The page closes and `Page.Run` returns; the last error is not pinned.
    fn page_run_closed_without_error(&mut self, st: &St) -> Value {
        self.last_error = al_builtins::native::unpinned_error();
        self.mark_closed(st);
        Value::Null
    }

    /// The platform closes page `st` that its handler left open (`action`: `unattended_action`). `Ok(false)`: not a pinned page
    /// and run, the caller closes it. A save of the edited row that fails with a pinned error closes the page, the row as it was,
    /// and `Page.Run` returns (CemPageHandlerTests.al:692 `returned=M-card EDIT;`, :227 `returned=M-card FAILMOD; T-mod R1
    /// FAILMOD;`); another error of the save is refused.
    pub(super) fn close_page_run_left_open(&mut self, st: &St, action: &str) -> Result<bool, Signal> {
        let Some(run) = st.borrow().page_run else {
            return Ok(false);
        };
        if self.pinned_run(st).is_none() {
            return Ok(false);
        }
        st.borrow_mut().page_run = Some(PageRun {
            phase: PageRunPhase::Returned,
            ..run
        });
        let c = self.page_run_close(st, action, false, false);
        let errors_before = st.borrow().errors.len();
        self.pages.close_save_error = None;
        self.pages.failed_trigger = None;
        let changes = self.store_mut().change_counter();
        self.close_with(st, action, true, false)?;
        // The save of the edited row raised, so the page stayed open (`close_with` keeps the error on the page).
        if !(st.borrow().open && st.borrow().errors.len() > errors_before) {
            return Ok(true);
        }
        let object = st.borrow().object;
        let origin = self
            .pages
            .failed_trigger
            .take()
            .filter(|(o, _)| *o == object)
            .map(|(_, n)| n);
        let saved = self.pages.close_save_error.take();
        let raised_in = self.pages.close_save_raised_in.take();
        let table = self.table(&self.page_rec(st))?.object as usize;
        let source = match (origin.as_deref(), raised_in) {
            (Some("onmodifyrecord"), Some(o)) => self.engine.program.objects[o].kind == al_ir::ObjKind::Codeunit,
            (None, Some(o)) => o == table,
            _ => false,
        };
        let pinned = saved.as_ref().is_some_and(literal_error)
            && source
            && self.store_mut().change_counter() == changes
            && c.is_some_and(|c| cell_pinned(c, Some(CloseError::Save)));
        if !pinned {
            return unsupported(SAVE_FAILED);
        }
        {
            let mut s = st.borrow_mut();
            s.dirty = false;
            s.errors.truncate(errors_before);
        }
        self.page_run_closed_without_error(st);
        Ok(true)
    }

    /// A handler of page `st` raised an error and left the page open. A modal page closes as `close_modal_after_handler_error`
    /// says. On a pinned page and run the platform closes it as after a return, and the caller raises the handler's error
    /// (CemPageHandlerTests.al:144 `CEM handler error` `log=Q-card OK; C-card;`); a close that then fails other than in the
    /// pinned cells, a page with an unsaved row and a close that writes are refused. Every other page is left as it was.
    pub(super) fn close_page_after_handler_error(&mut self, st: &St, e: &Signal, modal: bool) -> Result<(), Signal> {
        if modal {
            return self.close_modal_after_handler_error(st, e);
        }
        let (Some(run), Some(pinned)) = (st.borrow().page_run, self.pinned_run(st)) else {
            return Ok(());
        };
        // The handler's error came from a close it asked that failed in OnQueryClosePage outside the pinned cells: no pin asks
        // the close again, so the page is left as it was.
        if !matches!(
            pinned,
            PinnedRun::CardStatic | PinnedRun::ListStatic | PinnedRun::NavigateVariable
        ) || st.borrow().close_error
        {
            return Ok(());
        }
        if st.borrow().dirty || st.borrow().draft {
            return unsupported(
                "a page left open by a PageHandler that raised an error, with an unsaved row (not pinned)",
            );
        }
        st.borrow_mut().page_run = Some(PageRun {
            phase: PageRunPhase::HandlerFailed,
            ..run
        });
        let a = self.unattended_action(st);
        let changes = self.store_mut().change_counter();
        let r = self.close_with(st, a, true, false);
        if self.store_mut().change_counter() != changes {
            return unsupported(
                "a page left open by a PageHandler that raised an error, whose close wrote to the database (not pinned)",
            );
        }
        match r {
            Ok(_) if !st.borrow().open => Ok(()),
            Err(e) if !matches!(e, Signal::Error(_)) => Err(e),
            _ => unsupported(
                "a page left open by a PageHandler that raised an error, whose close then failed or was refused (not pinned)",
            ),
        }
    }

    /// The save of the edited row failed at OK in a PageHandler of a pinned page: BC then logs neither close trigger and
    /// `Page.Run` returns without an error (CemPageHandlerTests.al:720 `CardStaticRun_HandlerEditOk_PageModifyRecordError`
    /// `returned=M-card EDIT;`); which close that is, is not pinned, so it is refused.
    pub(super) fn refuse_page_run_save_error_at_action(&self, c: Option<PageRunClose>) -> Result<(), Signal> {
        if c.is_some_and(|c| c.by == CloseBy::Ok) {
            return unsupported(
                "an error raised while the page saved its edited row at an action of a PageHandler of Page.Run (not pinned)",
            );
        }
        Ok(())
    }
}
