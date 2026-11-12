//! How the list page a TableRelation lookup opened (`LookupKind::Relation`) closes, and what the lookup writes back, by
//! where the looked-up control is (al-runner issue 7479; corpus pages/close-errors/CemLookupTests.al, codeunit 99524).
//! Pinned for a field of a top-level Card and for a field of a ListPart on a Document, in the exact shape of the pins
//! (`relation_origin`, `pinned_origin`); every other shape keeps the closes and the write-back it had (only `LookupOK`
//! writes, `TestPage.Close()` in the handler is refused). A veto of the close in the pinned shape is refused.

use super::pinned_shape::{PinnedCode, TestCall};
use super::*;
use al_builtins::page_op::PageMethod;

/// The start of the text of an error out of OnClosePage (`page_close`, CcPageLeads.al:624-633).
pub(super) const CLOSING_ERRORS: &str = "One or more errors occurred while closing the page.";

/// The error an error of OnClosePage with the text `message` comes out as: `CLOSING_ERRORS`, two line breaks (shown as two
/// backslashes) and the text, with the code TestWrapped (CcPageLeads.al:624-633).
pub(in crate::pages) fn closing_error(message: &str) -> Signal {
    error_code(format!("{CLOSING_ERRORS}\\\\{message}"), "TestWrapped")
}

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
    /// The pinned place of control `idx` of page `st`, bound to the table field `field`, whose lookup opens the list page `related`, or `None` for every other
    /// shape (the lookup then closes and writes as before). Every fact of the pins is required (CemFixtures.al: pages CEM Card,
    /// CEM Doc with CEM Lines Part, lookup page CEM Codes; CemLookupTests.al opens the card and the document with OpenEdit only):
    /// - the origin: a top-level Card, or a ListPart on a top-level Document, opened by the test with OpenEdit, neither modal
    ///   nor in lookup mode, with no page extension and no subscriber;
    /// - the control: a field of Rec declared by the page, with no trigger; its table field has no OnValidate or OnLookup
    ///   trigger, and the table no extension and no subscriber;
    /// - the lookup page: a List with `Editable = false` whose only code is its OnQueryClosePage and OnClosePage, whose controls
    ///   are a repeater and fields of Rec without triggers (no action, no part), with no extension and no subscriber, over a
    ///   table that runs no code (`page_code_is`).
    pub(super) fn relation_origin(
        &self,
        st: &St,
        idx: usize,
        field: &ir::Field,
        related: &St,
    ) -> Result<Option<RelationOrigin>, Signal> {
        let engine = self.engine();
        let page_type = |s: &St| engine.object_facts[s.borrow().object as usize].page_type.clone();
        let plain_page = |s: &St| {
            let page = page_of(&engine, s);
            page.extensions.is_empty()
                && page.ext_triggers.is_empty()
                && !engine.events.any_subscriber_of(s.borrow().object as usize)
        };
        let opened_edit = |s: &St| {
            let s = s.borrow();
            s.host.is_none() && s.opened_by_test && s.mode == PageMode::Edit && !s.modal && !s.lookup
        };
        let (host, modal, lookup) = {
            let s = st.borrow();
            (s.host.clone(), s.modal, s.lookup)
        };
        if modal || lookup || !plain_page(st) {
            return Ok(None);
        }
        let origin = match host {
            None => (page_type(st) == Some(ir::PageType::Card) && opened_edit(st)).then_some(RelationOrigin::Card),
            Some((h, _)) => h.upgrade().and_then(|h| {
                (opened_edit(&h)
                    && plain_page(&h)
                    && page_type(&h) == Some(ir::PageType::Document)
                    && matches!(page_type(st), Some(ir::PageType::Other(t)) if t == "listpart"))
                .then_some(RelationOrigin::DocumentListPart)
            }),
        };
        let Some(origin) = origin else {
            return Ok(None);
        };
        let ctl = &page_of(&engine, st).controls[idx];
        if !matches!(ctl.binding, ir::Binding::Field(_))
            || ctl.kind != ir::ControlKind::Field
            || ctl.owner.is_some()
            || !ctl.triggers.is_empty()
            || !ctl.ext_triggers.is_empty()
        {
            return Ok(None);
        }
        let table = self.table(&self.page_rec(st))?;
        let table_object = table.object as usize;
        let field_ok = field.on_validate.is_none()
            && field.on_lookup.is_none()
            && field.ext_before_validate.is_empty()
            && field.ext_after_validate.is_empty()
            && engine.program.objects[table_object]
                .table
                .as_ref()
                .is_some_and(|x| x.extensions.is_empty())
            && !engine.events.any_subscriber_of(table_object);
        let editable_false = engine.program.objects[related.borrow().object as usize]
            .properties
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Editable"))
            .is_some_and(|(_, v)| v.trim().eq_ignore_ascii_case("false"));
        let lookup_page = page_type(related) == Some(ir::PageType::List)
            && editable_false
            && self.page_code_is(
                related,
                &PinnedCode {
                    triggers: &["onqueryclosepage", "onclosepage"],
                    control_triggers: &[],
                    properties: &["editable"],
                },
            )?;
        Ok((field_ok && lookup_page).then_some(origin))
    }

    /// The pinned place of the relation's list page `st` while the handler stays inside the pinned calls: none, or `GoToKey`
    /// first, then nothing, `OK().Invoke()`, `Cancel().Invoke()` or `Close()` (CemLookupTests.al `CemCodesModal` modes 0, 1, 2,
    /// 3, 11, 12, 13). After any other call the lookup closes and writes as before.
    fn pinned_origin(st: &St) -> Option<RelationOrigin> {
        use PageMethod::{Cancel, Close, GoToKey, Invoke, OK};
        use TestCall::{Other, Page};
        let origin = {
            let s = st.borrow();
            if s.lookup_kind != LookupKind::Relation {
                return None;
            }
            s.relation_origin?
        };
        let calls = Self::test_calls(st)?;
        let rest = match calls.as_slice() {
            [Page(GoToKey), rest @ ..] => rest,
            all => all,
        };
        matches!(
            rest,
            [] | [Page(Close)]
                | [Page(OK)]
                | [Page(OK), Other(Invoke)]
                | [Page(Cancel)]
                | [Page(Cancel), Other(Invoke)]
        )
        .then_some(origin)
    }

    /// OnQueryClosePage of the relation's list page `st` from a pinned place refused the close: no pin has a veto (every
    /// OnQueryClosePage of CemFixtures.al exits true or raises), so the lookup is refused.
    pub(super) fn refuse_relation_close_veto(&self, st: &St) -> Result<(), Signal> {
        if Self::pinned_origin(st).is_some() {
            return unsupported("OnQueryClosePage of a TableRelation lookup page refused the close (not pinned)");
        }
        Ok(())
    }

    /// `close_with` asks the relation's list page `st` to close: notes what asked (`via_action`: an action the handler
    /// invoked, `test_close`: `TestPage.Close()`, else the platform after the handler returned).
    pub(super) fn note_relation_close(&self, st: &St, requested: &str, test_close: bool, via_action: bool) {
        if Self::pinned_origin(st).is_none() {
            return;
        }
        st.borrow_mut().relation_close = Some(match (via_action, test_close) {
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
        (Self::pinned_origin(st) == Some(RelationOrigin::DocumentListPart)).then_some("LookupOK")
    }

    /// A `TestPage.Close()` in the handler of the relation's list page from a pinned place: the lookup comes back without an
    /// error (CemLookupTests.al:124 CardField_Lookup_HandlerClose_NoError and ListPartField_Lookup_HandlerClose_NoError
    /// `end|text` CEM_NO_ERROR); what it writes is `relation_pick_written`. `false` for every other page (refused by the caller).
    pub(super) fn relation_closed_in_handler(&self, st: &St) -> bool {
        Self::pinned_origin(st).is_some()
    }

    /// Whether the lookup writes the row the relation's list page stands on into the control, for a page from a pinned place;
    /// `None` for every other page (the caller writes on LookupOK only, as before). The page must have closed cleanly.
    /// - From a Card field: every close but Cancel writes, also Close() and the platform's close after the handler returned,
    ///   although the page reports LookupCancel (CemLookupTests.al:34 HandlerReturns `value=C1`, :124 HandlerClose `value=C1`,
    ///   :213 HandlerPicksC2Returns `value=C2`, :243 HandlerPicksC2Close `value=C2`, :94 HandlerCancel `value=`).
    /// - From a ListPart field only OK writes (ListPartField_Lookup_HandlerOkFirstRow_NoError `value=C1`, _HandlerPicksC2Ok
    ///   `value=C2`; _HandlerReturns, _HandlerCancel, _HandlerClose `value=`).
    pub(super) fn relation_pick_written(&self, related: &St, outcome: &Value) -> Option<bool> {
        let origin = Self::pinned_origin(related)?;
        let s = related.borrow();
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
    ///   last error refuses to be read until the next error replaces it.
    /// - Every other error comes out wrapped: an AL error of the handler with the code `TestWrapped:Dialog` (:154
    ///   CardField_Lookup_HandlerError_NoError), the unhandled message of OnQueryClosePage after OK, Cancel or Close() with
    ///   `TestWrapped:MissingUIHandler` (:303 CardField_Lookup_HandlerOkFirstRow_QueryCloseError, :332, :361); a code already
    ///   wrapped stays (OnClosePage at Close(), CardField_Lookup_HandlerClose_ClosePageError `TestWrapped`). Another code has no
    ///   pinned wrapped form: it becomes the unpinned mark.
    pub(super) fn relation_lookup_error(&mut self, related: &St, e: Signal) -> Result<Value, Signal> {
        let origin = Self::pinned_origin(related);
        let how = related.borrow().relation_close;
        let (Some(_), Signal::Error(mut e)) = (origin, e.clone()) else {
            return Err(e);
        };
        let close_page_error = e.code == "TestWrapped" && e.message.starts_with(CLOSING_ERRORS);
        let query_message = e.code == "MissingUIHandler" && related.borrow().close_error;
        if how == Some(RelationClose::Platform) && (query_message || close_page_error) {
            self.mark_closed(related);
            // What the last error is then is not pinned: every pin raises its own error right after the lookup. Measured on
            // the runner before this rule (PR 7816 review round 2): the message of OnQueryClosePage, or the error before the
            // lookup after an error of OnClosePage; neither is BC's, so the last error refuses to be read.
            self.last_error = al_builtins::native::unpinned_error();
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
        Some(closing_error(&e.message))
    }
}
