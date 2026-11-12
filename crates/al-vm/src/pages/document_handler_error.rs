//! A Document page with hidden parts run by the static `Page.RunModal` with a record, whose ModalPageHandler raised an error
//! or whose close by OK, Cancel or the platform failed (al-runner issue 7473; corpus pages/close-errors/CemDocumentTests.al,
//! codeunit 99523, page CEM Doc of CemFixtures.al). Pinned only for the exact shape of that page and of the handler's calls
//! (`document_close_pinned`); every other page keeps the closes it had (`modal_handler_error.rs` refuses a page with an unsaved
//! row or a hidden part).

use super::pinned_shape::{TestCall, TreeCall};
use super::relation_lookup_close::CLOSING_ERRORS;
use super::*;
use al_builtins::page_op::PageMethod;

impl Session {
    /// Whether `st` is the page of the pins, every fact required (CemFixtures.al:348-424, CemDocumentTests.al `CemDocModal`):
    /// - a top-level Document run modally in lookup mode by the static `Page.RunModal` (CemDocumentTests.al:177);
    /// - its own triggers exactly OnModifyRecord, OnQueryClosePage and OnClosePage; its control triggers exactly one
    ///   OnValidate (Name: `CurrPage.Update()`); its controls groups, fields of Rec and parts, declared by the page itself;
    /// - its parts' pages declare no trigger; neither the page, nor a part page, nor the source table has an extension or a
    ///   subscriber;
    /// - the handler's calls on the page: a value typed into one field or not, then nothing, `OK().Invoke()` or
    ///   `Cancel().Invoke()`, or `First` and a value typed on the ListPart before them (handler modes 0, 1, 2, 4, 5, 7, 8, 9,
    ///   11, 12).
    pub(super) fn document_close_pinned(&self, st: &St) -> Result<bool, Signal> {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let shape = {
            let s = st.borrow();
            s.host.is_none() && s.modal && s.lookup && s.run_static == Some(true) && !s.tree_calls_unknown
        };
        if !shape || engine.object_facts[object].page_type != Some(ir::PageType::Document) {
            return Ok(false);
        }
        let plain = |s: &St| {
            let page = page_of(&engine, s);
            page.extensions.is_empty()
                && page.ext_triggers.is_empty()
                && !engine.events.any_subscriber_of(s.borrow().object as usize)
        };
        let page = page_of(&engine, st);
        let mut triggers: Vec<&str> = page.triggers.iter().map(|(n, _)| n.as_str()).collect();
        triggers.sort_unstable();
        let control_triggers: Vec<&str> = page
            .controls
            .iter()
            .flat_map(|c| c.triggers.iter().map(|(n, _)| n.as_str()))
            .collect();
        let controls = page.controls.iter().all(|c| {
            c.owner.is_none()
                && c.ext_triggers.is_empty()
                && match c.kind {
                    ir::ControlKind::Group => true,
                    ir::ControlKind::Field => matches!(c.binding, ir::Binding::Field(_)),
                    ir::ControlKind::Part => c.triggers.is_empty(),
                    _ => false,
                }
        });
        let parts: Vec<St> = st.borrow().parts.iter().map(|(_, p)| p.clone()).collect();
        let parts_ok = parts
            .iter()
            .all(|p| plain(p) && page_of(&engine, p).triggers.is_empty());
        if triggers != ["onclosepage", "onmodifyrecord", "onqueryclosepage"]
            || control_triggers != ["onvalidate"]
            || !controls
            || !parts_ok
            || !plain(st)
        {
            return Ok(false);
        }
        let table = self.table(&self.page_rec(st))?.object as usize;
        let table_ok = engine.program.objects[table]
            .table
            .as_ref()
            .is_some_and(|x| x.extensions.is_empty())
            && !engine.events.any_subscriber_of(table);
        Ok(table_ok && Self::document_handler_calls_pinned(&st.borrow().tree_calls))
    }

    /// The handler modes of `CemDocModal`: a value typed into a field of the page, or `First` and a value typed on the
    /// ListPart (modes 11, 12), or neither; then nothing, OK or Cancel.
    fn document_handler_calls_pinned(calls: &[TreeCall]) -> bool {
        use PageMethod::{Cancel, First, Invoke, OK};
        use TestCall::{Other, Page, Write};
        let on_page = |c: &TreeCall| c.part.is_none();
        let rest = match calls {
            [w, rest @ ..] if on_page(w) && matches!(w.call, Write(_)) => rest,
            [f, w, rest @ ..]
                if f.part.is_some() && f.part == w.part && f.call == Page(First) && matches!(w.call, Write(_)) =>
            {
                rest
            }
            all => all,
        };
        if !rest.iter().all(on_page) {
            return false;
        }
        let rest: Vec<TestCall> = rest.iter().map(|c| c.call).collect();
        matches!(
            rest.as_slice(),
            [] | [Page(OK)] | [Page(OK), Other(Invoke)] | [Page(Cancel)] | [Page(Cancel), Other(Invoke)]
        )
    }

    /// The handler of the pinned page raised `e` and left the page open: what the platform does then.
    /// - The error came out of the close OK or Cancel asked in the handler, an OnQueryClosePage message nobody handled: the
    ///   close is not asked again, and the error comes out wrapped (CemDocumentTests.al:349 HandlerOk_QueryCloseError, :376
    ///   HandlerCancel_QueryCloseError, :430 HandlerEditOk_QueryCloseError `TestWrapped:MissingUIHandler`, `log` `Q-doc LookupOK;`).
    /// - The SetValue the handler made last raised: the save of the typed value, which the table's OnModify or the page's
    ///   OnModifyRecord refused, failed and left the row unsaved (the control's OnValidate calls `CurrPage.Update()`): the page
    ///   closes without OnQueryClosePage or OnClosePage, and the handler's error comes out as it is (:212
    ///   HandlerTableRefusesReturns_NoError, :239 _TableRefusesOk_, :483, :510, the `close` cases, and the PageModifyRecordError
    ///   cases, `log` `M-doc FAILMOD; T-mod R1 FAILMOD;` `TestValidation`).
    /// - Any other error with nothing unsaved: the platform asks the close with LookupCancel, OnQueryClosePage and OnClosePage
    ///   run with the hidden parts, and the handler's error comes out (:185 HandlerError_NoError `Q-doc LookupCancel; C-doc;`
    ///   `Dialog`); an error of that close comes out instead, an OnQueryClosePage message as it is (:457
    ///   HandlerError_QueryCloseError `MissingUIHandler`), an OnClosePage error wrapped (HandlerError_ClosePageError
    ///   `One or more errors occurred while closing the page.\\CEM close page error` `TestWrapped`).
    pub(super) fn close_document_after_handler_error(&mut self, st: &St, e: &Signal) -> Result<(), Signal> {
        let Signal::Error(err) = e else {
            return Ok(());
        };
        if st.borrow().close_error {
            self.mark_closed(st);
            if err.code != "MissingUIHandler" {
                return Err(Signal::Error(Box::new(al_builtins::native::unpinned_error())));
            }
            let mut wrapped = (**err).clone();
            wrapped.code = "TestWrapped:MissingUIHandler".into();
            return Err(Signal::Error(Box::new(wrapped)));
        }
        let typed_last = matches!(
            st.borrow().tree_calls.last(),
            Some(TreeCall {
                part: None,
                call: TestCall::Write(_),
                ..
            })
        );
        if typed_last && err.code == "TestValidation" {
            self.mark_closed(st);
            return Ok(());
        }
        if st.borrow().dirty || st.borrow().draft || st.borrow().tree_calls.iter().any(|c| c.part.is_some()) {
            return unsupported(
                "a page left open by a ModalPageHandler that raised an error, with an unsaved row or a hidden part (not pinned)",
            );
        }
        let a = self.unattended_action(st);
        match self.close_with(st, a, false, false) {
            Ok(_) if !st.borrow().open => Ok(()),
            Err(Signal::Error(q)) if q.code == "MissingUIHandler" || q.code == "TestWrapped" => {
                self.mark_closed(st);
                Err(Signal::Error(q))
            }
            Ok(_) | Err(Signal::Error(_)) => unsupported(
                "a page left open by a ModalPageHandler that raised an error, whose close then failed or was refused (not pinned)",
            ),
            Err(other) => Err(other),
        }
    }

    /// An error of OnClosePage `r` when an action or the platform closes the pinned page: the page closes and the error comes
    /// out wrapped as at `TestPage.Close()` (CemDocumentTests.al:592 HandlerReturns_ClosePageError, :619 HandlerOk_, :646
    /// HandlerCancel_, :673 HandlerEditReturns_, HandlerEditOk_ClosePageError `One or more errors occurred while closing the
    /// page.\\CEM close page error` `TestWrapped`). `None` for every other page and error (an AL error only).
    pub(super) fn document_close_page_error(
        &mut self,
        st: &St,
        r: &Result<Option<Value>, Signal>,
    ) -> Result<Option<Signal>, Signal> {
        let Err(Signal::Error(e)) = r else {
            return Ok(None);
        };
        if e.code != "Dialog" || !self.document_close_pinned(st)? {
            return Ok(None);
        }
        self.mark_closed(st);
        Ok(Some(error_code(
            format!("{CLOSING_ERRORS}\\\\{}", e.message),
            "TestWrapped",
        )))
    }

    /// The save of the ListPart's line `saved` failed when the platform or an action closes the pinned page. After the handler
    /// returned, the page closes without OnQueryClosePage and OnClosePage and the error comes out with the code TestWrapped
    /// (CemDocumentTests.al:266 HandlerLineRefusesReturns_NoError `CEM line modify refused` `TestWrapped`, `log`
    /// `L-mod R1 1 -1;`, and the `QueryCloseError` and `ClosePageError` cases of the same handler). At OK in the handler,
    /// RunModal answers None without an error (:293 HandlerLineRefusesOk_NoError); that path is refused (the error would
    /// already have left the handler). `None` for every other page and error.
    pub(super) fn document_part_save_error(
        &mut self,
        st: &St,
        saved: &Result<(), Signal>,
        via_action: bool,
    ) -> Result<Option<Signal>, Signal> {
        let Err(Signal::Error(e)) = saved else {
            return Ok(None);
        };
        if e.code != "Dialog" || !self.document_close_pinned(st)? {
            return Ok(None);
        }
        if via_action || self.ui_state.page_handler_depth > 0 {
            return unsupported(
                "OK of a Document run modally whose ListPart line the table refused to save, inside the handler (not pinned)",
            );
        }
        self.mark_closed(st);
        Ok(Some(error_code(e.message.clone(), "TestWrapped")))
    }
}
