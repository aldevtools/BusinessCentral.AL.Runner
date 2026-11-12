//! How the list page a TableRelation lookup opened (`LookupKind::Relation`) closes, and what the lookup writes back, by
//! where the looked-up control is (al-runner issue 7479; corpus pages/close-errors/CemLookupTests.al, codeunit 99524).
//! Pinned for a field of a top-level Card and for a field of a ListPart on a Document; every other place keeps the
//! closes and the write-back it had (only `LookupOK` writes, `TestPage.Close()` in the handler is refused).

use super::*;

/// The start of the text of an error out of OnClosePage (`page_close`, CcPageLeads.al:624-633).
const CLOSING_ERRORS: &str = "One or more errors occurred while closing the page.";

/// Where the control whose `Lookup()` opened the relation's list page sits, for the places the corpus pins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationOrigin {
    /// A field of a Card the test opened (CemLookupTests.al:34 `Card.Ref.Lookup()`, page CEM Card).
    Card,
    /// A field of a ListPart on a Document the test opened (CemLookupTests.al `Doc.Lines.RefCode.Lookup()`, page CEM Doc).
    DocumentListPart,
}

/// What closed the relation's list page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationClose {
    /// The handler invoked OK (LookupOK).
    Ok,
    /// The handler invoked Cancel (LookupCancel).
    Cancel,
    /// The handler called `TestPage.Close()`.
    TestClose,
    /// The handler returned and the platform closed the page.
    Platform,
}

impl Session {
    /// The pinned place of control page `st`, or `None` for every other place.
    pub(super) fn relation_origin(&self, st: &St) -> Option<RelationOrigin> {
        let engine = self.engine();
        let page_type = |s: &St| engine.object_facts[s.borrow().object as usize].page_type.clone();
        let (host, modal, lookup) = {
            let s = st.borrow();
            (s.host.clone(), s.modal, s.lookup)
        };
        if modal || lookup {
            return None;
        }
        match host {
            None if page_type(st) == Some(ir::PageType::Card) => Some(RelationOrigin::Card),
            Some((h, _)) => {
                let h = h.upgrade()?;
                let top = {
                    let s = h.borrow();
                    s.host.is_none() && !s.modal && !s.lookup
                };
                (top && page_type(&h) == Some(ir::PageType::Document)
                    && matches!(page_type(st), Some(ir::PageType::Other(t)) if t == "listpart"))
                .then_some(RelationOrigin::DocumentListPart)
            }
            None => None,
        }
    }

    /// `close_with` asks the relation's list page `st` to close: notes what asked (`via_action`: an action the handler
    /// invoked, `test_close`: `TestPage.Close()`, else the platform after the handler returned).
    pub(super) fn note_relation_close(&self, st: &St, requested: &str, test_close: bool, via_action: bool) {
        let mut s = st.borrow_mut();
        if s.relation_origin.is_none() {
            return;
        }
        s.relation_close = Some(match (via_action, test_close) {
            (true, _) if matches!(requested, "OK" | "LookupOK") => RelationClose::Ok,
            (true, _) => RelationClose::Cancel,
            (false, true) => RelationClose::TestClose,
            (false, false) => RelationClose::Platform,
        });
    }

    /// The `Action` the relation's list page reports from a ListPart field: LookupOK for every close, OK, Cancel, Close()
    /// and the platform's (CemLookupTests.al ListPartField_Lookup_HandlerReturns_NoError, _HandlerCancel_NoError,
    /// _HandlerClose_NoError `Q-codes LookupOK;`). From a Card field the lookup mapping of `close_action` applies (:34
    /// CardField_Lookup_HandlerReturns_NoError, :124 _HandlerClose_NoError `Q-codes LookupCancel;`). `None`: not such a page.
    pub(super) fn relation_close_action(&self, st: &St) -> Option<&'static str> {
        let s = st.borrow();
        (s.lookup_kind == LookupKind::Relation && s.relation_origin == Some(RelationOrigin::DocumentListPart))
            .then_some("LookupOK")
    }

    /// A `TestPage.Close()` in the handler of the relation's list page from a pinned place: the lookup comes back without an
    /// error (CemLookupTests.al:124 CardField_Lookup_HandlerClose_NoError and ListPartField_Lookup_HandlerClose_NoError
    /// `end|text` CEM_NO_ERROR); what it writes is `relation_pick_written`. `false` for every other page (refused by the caller).
    pub(super) fn relation_closed_in_handler(&self, st: &St) -> bool {
        let s = st.borrow();
        s.lookup_kind == LookupKind::Relation && s.relation_origin.is_some()
    }

    /// Whether the lookup writes the row the relation's list page stands on into the control, for a page from a pinned place;
    /// `None` for every other page (the caller writes on LookupOK only, as before). The page must have closed cleanly.
    /// - From a Card field: every close but Cancel writes, also Close() and the platform's close after the handler returned,
    ///   although the page reports LookupCancel (CemLookupTests.al:34 HandlerReturns `value=C1`, :124 HandlerClose `value=C1`,
    ///   :213 HandlerPicksC2Returns `value=C2`, :243 HandlerPicksC2Close `value=C2`, :94 HandlerCancel `value=`).
    /// - From a ListPart field only OK writes (ListPartField_Lookup_HandlerOkFirstRow_NoError `value=C1`, _HandlerPicksC2Ok
    ///   `value=C2`; _HandlerReturns, _HandlerCancel, _HandlerClose `value=`).
    pub(super) fn relation_pick_written(&self, related: &St, outcome: &Value) -> Option<bool> {
        let s = related.borrow();
        let origin = s.relation_origin?;
        let closed = matches!(outcome, Value::Option(..)) && !s.open;
        let how = s.relation_close?;
        Some(
            closed
                && match origin {
                    RelationOrigin::Card => how != RelationClose::Cancel,
                    RelationOrigin::DocumentListPart => how == RelationClose::Ok,
                },
        )
    }

    /// The handler of the relation's list page raised an error and left it open: the platform asks the close with the action of
    /// a handler that returned, OnQueryClosePage and OnClosePage run, and the lookup raises the handler's error
    /// (CemLookupTests.al:154 CardField_Lookup_HandlerError_NoError `Q-codes LookupCancel; C-codes;`,
    /// ListPartField_Lookup_HandlerError_NoError `Q-codes LookupOK; C-codes;`). An OnQueryClosePage that shows an unhandled message
    /// stops the close there, and the handler's error still comes out (:390 CardField_Lookup_HandlerError_QueryCloseError
    /// `Q-codes LookupCancel;`). An error of OnClosePage then is refused by `close_with`.
    pub(super) fn close_relation_lookup_after_handler_error(&mut self, st: &St) -> Result<(), Signal> {
        // The handler's error is that unhandled message, raised by the close OK or Cancel asked in the handler: the close is
        // not asked again (:303 CardField_Lookup_HandlerOkFirstRow_QueryCloseError `Q-codes LookupOK;`, :332 _HandlerCancel_).
        if st.borrow().close_error {
            self.mark_closed(st);
            return Ok(());
        }
        let a = self.unattended_action(st);
        match self.close_with(st, a, false, false) {
            Ok(_) if !st.borrow().open => Ok(()),
            // OnClosePage failed and the page closed (`relation_close_page_error`): the handler's error still comes out
            // (ListPartField_Lookup_HandlerError_ClosePageError `Q-codes LookupOK; C-codes;` `TestWrapped:Dialog`).
            Err(Signal::Error(_)) if !st.borrow().open => Ok(()),
            Err(Signal::Error(_)) if st.borrow().close_error => {
                self.mark_closed(st);
                Ok(())
            }
            Ok(_) | Err(Signal::Error(_)) => unsupported(
                "a relation lookup page left open by a ModalPageHandler that raised an error, whose close then failed or was refused (not pinned)",
            ),
            Err(e) => Err(e),
        }
    }

    /// What an error out of the relation's list page does to the lookup, for a page from a pinned place; for every other page
    /// the error goes on as it was.
    /// - The platform's close after the handler returned, stopped by an OnQueryClosePage that shows an unhandled message, or
    ///   failed in OnClosePage (CardField_Lookup_HandlerReturns_ClosePageError, _HandlerReturns_ClosePageConfirmUnhandled,
    ///   ListPartField_Lookup_HandlerReturns_ClosePageError `value= | Q-codes ...; C-codes;`): the
    ///   lookup comes back without an error and writes nothing (CemLookupTests.al:273 CardField_Lookup_HandlerReturns_QueryCloseError
    ///   `value= | Q-codes LookupCancel;` CEM_NO_ERROR, :448 _HandlerPicksC2Returns_QueryCloseError,
    ///   ListPartField_Lookup_HandlerReturns_QueryCloseError). Whether that message becomes the last error is not pinned: the
    ///   lookup leaves the last error as it was.
    /// - Every other error comes out wrapped: an AL error of the handler with the code `TestWrapped:Dialog` (:154
    ///   CardField_Lookup_HandlerError_NoError), the unhandled message of OnQueryClosePage after OK, Cancel or Close() with
    ///   `TestWrapped:MissingUIHandler` (:303 CardField_Lookup_HandlerOkFirstRow_QueryCloseError, :332, :361); a code already
    ///   wrapped stays (OnClosePage at Close(), CardField_Lookup_HandlerClose_ClosePageError `TestWrapped`). Another code has no
    ///   pinned wrapped form: it becomes the unpinned mark.
    pub(super) fn relation_lookup_error(&mut self, related: &St, e: Signal) -> Result<Value, Signal> {
        let (origin, how) = {
            let s = related.borrow();
            (s.relation_origin, s.relation_close)
        };
        let (Some(_), Signal::Error(mut e)) = (origin, e.clone()) else {
            return Err(e);
        };
        let close_page_error = e.code == "TestWrapped" && e.message.starts_with(CLOSING_ERRORS);
        let query_message = e.code == "MissingUIHandler" && related.borrow().close_error;
        if how == Some(RelationClose::Platform) && (query_message || close_page_error) {
            self.mark_closed(related);
            return Ok(Value::Null);
        }
        e.code = match e.code.as_str() {
            "Dialog" | "MissingUIHandler" => format!("TestWrapped:{}", e.code),
            c if c.starts_with("TestWrapped") => c.to_string(),
            _ => return Err(Signal::Error(Box::new(al_builtins::native::unpinned_error()))),
        };
        Err(Signal::Error(e))
    }

    /// An error of OnClosePage `r` when an action or the platform closes the relation's list page from a pinned place: the page
    /// closes, and the error comes out wrapped as at `TestPage.Close()`, for an AL error and for a Confirm no handler took
    /// (CemLookupTests.al:537 CardField_Lookup_HandlerOkFirstRow_ClosePageError `One or more errors occurred while closing the
    /// page.\\CEM close page error` `TestWrapped`, CardField_Lookup_HandlerOkFirstRow_ClosePageConfirmUnhandled `...page.\\Unhandled
    /// UI: Confirm CEM close confirm?`, ListPartField_Lookup_HandlerCancel_ClosePageError). `None` for every other page and error.
    pub(super) fn relation_close_page_error(&mut self, st: &St, r: &Result<Option<Value>, Signal>) -> Option<Signal> {
        let Err(Signal::Error(e)) = r else {
            return None;
        };
        if !self.relation_closed_in_handler(st) || !matches!(e.code.as_str(), "Dialog" | "MissingUIHandler") {
            return None;
        }
        self.mark_closed(st);
        Some(error_code(format!("{CLOSING_ERRORS}\\\\{}", e.message), "TestWrapped"))
    }
}
