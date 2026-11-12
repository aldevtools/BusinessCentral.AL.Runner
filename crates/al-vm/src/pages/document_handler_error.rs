//! A Document page with hidden parts run by the static `Page.RunModal` with a record, whose ModalPageHandler raised an error
//! or whose close by OK, Cancel or the platform failed (al-runner issue 7473; corpus pages/close-errors/CemDocumentTests.al,
//! codeunit 99523, page CEM Doc of CemFixtures.al). Pinned only for the exact shape of that page and of the handler's calls
//! (`document_close_pinned`); every other page keeps the closes it had (`modal_handler_error.rs` refuses a page with an unsaved
//! row or a hidden part).

use super::pinned_shape::{TestCall, TreeCall};
use super::relation_lookup_close::CLOSING_ERRORS;
use super::*;
use al_builtins::page_op::PageMethod;

/// The kinds of a part of CEM Doc (CemFixtures.al:383-410), in its order: the content ListPart linked to the row, the
/// CardPart factbox linked to it, the same hidden by `Visible = false`, the same unlinked and shown by a page variable.
#[derive(PartialEq)]
enum CemPart {
    Lines,
    Info,
    Hidden,
    Extra,
}

impl Session {
    /// Whether `st` is page CEM Doc of the pins, run as they run it, every fact required (CemFixtures.al:348-424,
    /// CemDocumentTests.al:168-924, al-runner PR 7816 review, round 3):
    /// - a top-level Document of a stored table, run modally in lookup mode by the static `Page.RunModal`
    ///   (CemDocumentTests.al:177); no page property but PageType, SourceTable, ApplicationArea, UsageCategory and Caption;
    /// - its own triggers exactly OnModifyRecord (a procedure of a codeunit variable, then `exit(true)`), OnQueryClosePage
    ///   (`exit` of a procedure of a codeunit variable) and OnClosePage (a procedure of a codeunit variable); what those
    ///   procedures do is checked where it shows: an error code the pins do not show, a veto or an unhandled dialog of OnClosePage is
    ///   refused when the close runs;
    /// - its controls groups, fields of Rec with ApplicationArea only, exactly one control trigger, an OnValidate whose body is
    ///   `CurrPage.Update()`, and the four parts of `CemPart`;
    /// - the part pages declare no trigger and no property but the plain ones; their controls are a repeater and fields of
    ///   Rec without a trigger;
    /// - no page, part page or table has an extension or a subscriber; the source table has no trigger but OnInsert and
    ///   OnModify, the ListPart's table none but OnModify, neither a field trigger or a procedure;
    /// - the handler's calls are those of `CemDocModal` (`document_handler_calls_pinned`).
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
        let Some((name, lines)) = self.cem_doc_controls(&engine, object) else {
            return Ok(false);
        };
        if !Self::cem_doc_triggers(&engine, object)
            || !Self::document_handler_calls_pinned(&st.borrow().tree_calls, name, lines)
        {
            return Ok(false);
        }
        let rec = self.page_rec(st);
        if rec.borrow().temporary {
            return Ok(false);
        }
        let table = self.table(&rec)?.object as usize;
        let parts: Vec<St> = st.borrow().parts.iter().map(|(_, p)| p.clone()).collect();
        let mut part_tables = Vec::new();
        for p in &parts {
            part_tables.push(self.table(&self.page_rec(p))?.object as usize);
        }
        let code_within = |object: usize, triggers: &[&str]| {
            pinned_shape::table_code(&engine, object)
                .is_some_and(|(have, more)| !more && have.iter().all(|t| triggers.contains(&t.as_str())))
        };
        let line_table = parts
            .iter()
            .zip(&part_tables)
            .find(|(p, _)| p.borrow().object as usize == lines.1)
            .map(|(_, t)| *t);
        Ok(code_within(table, &["oninsert", "onmodify"])
            && line_table.is_some_and(|t| code_within(t, &["onmodify"]))
            && part_tables.iter().all(|&t| t == table || Some(t) == line_table))
    }

    /// The page code of CEM Doc (CemFixtures.al:412-423): its own triggers, their bodies, no extension and no subscriber.
    fn cem_doc_triggers(engine: &Engine, object: usize) -> bool {
        let program = &engine.program;
        let page = program.objects[object].page.as_ref().expect("a page");
        let call = |e: &ir::Expr| pinned_shape::codeunit_var_call(program, object, e);
        let body_ok = |name: &str, r: usize| {
            let mut stmts = pinned_shape::code_stmts(program, object, r);
            // The lowering ends a trigger with a return value by an `exit` of its default, unreachable after an `exit`.
            if let [.., ir::Stmt::Exit(Some(_)), ir::Stmt::Exit(Some(_))] = stmts.as_slice() {
                stmts.pop();
            }
            match (name, stmts.as_slice()) {
                ("onmodifyrecord", [ir::Stmt::Expr(e), ir::Stmt::Exit(Some(t))]) => {
                    call(e)
                        && matches!(t, ir::Expr::Const(ir::Const::Boolean(true)))
                            | matches!(t, ir::Expr::Convert(_, b) if matches!(**b, ir::Expr::Const(ir::Const::Boolean(true))))
                }
                ("onqueryclosepage", [ir::Stmt::Exit(Some(e))]) | ("onclosepage", [ir::Stmt::Expr(e)]) => call(e),
                _ => false,
            }
        };
        let mut names: Vec<&str> = page.triggers.iter().map(|(n, _)| n.as_str()).collect();
        names.sort_unstable();
        names == ["onclosepage", "onmodifyrecord", "onqueryclosepage"]
            && page.triggers.iter().all(|(n, r)| body_ok(n, *r))
            && page.extensions.is_empty()
            && page.ext_triggers.is_empty()
            && !engine.events.any_subscriber_of(object)
            && program.objects[object]
                .properties
                .iter()
                .all(|(k, _)| pinned_shape::PLAIN_PROPERTIES.contains(&k.to_ascii_lowercase().as_str()))
    }

    /// The controls of CEM Doc (CemFixtures.al:356-410, see `document_close_pinned`): the control index of the field with
    /// the OnValidate, and the control index and page object of the ListPart. `None` for any other page.
    fn cem_doc_controls(&self, engine: &Engine, object: usize) -> Option<(usize, (usize, usize))> {
        let program = &engine.program;
        let page = program.objects[object].page.as_ref()?;
        let lit = |c: &ir::Control, allowed: &[&str]| {
            c.props
                .iter()
                .all(|(k, v)| allowed.contains(&k.as_str()) && matches!(v, ir::PropVal::Lit(_)))
        };
        let mut validate = None;
        let mut parts = Vec::new();
        for (i, c) in page.controls.iter().enumerate() {
            if c.owner.is_some() || !c.ext_triggers.is_empty() {
                return None;
            }
            match c.kind {
                ir::ControlKind::Group if c.binding == ir::Binding::None && c.triggers.is_empty() => {}
                ir::ControlKind::Field
                    if matches!(c.binding, ir::Binding::Field(_)) && lit(c, &["applicationarea"]) =>
                {
                    match c.triggers.as_slice() {
                        [] => {}
                        [(n, r)] if n == "onvalidate" && validate.is_none() => {
                            let body = pinned_shape::code_stmts(program, object, *r);
                            if !matches!(body.as_slice(), [s] if pinned_shape::is_curr_page_update(s, page.curr_page_slot))
                            {
                                return None;
                            }
                            validate = Some(i);
                        }
                        _ => return None,
                    }
                }
                ir::ControlKind::Part if c.triggers.is_empty() => parts.push((i, self.cem_part(engine, c)?)),
                _ => return None,
            }
        }
        let kinds: Vec<&CemPart> = parts.iter().map(|(_, (k, _))| k).collect();
        if kinds != [&CemPart::Lines, &CemPart::Info, &CemPart::Hidden, &CemPart::Extra] {
            return None;
        }
        let (lines, (_, lines_page)) = parts[0];
        Some((validate?, (lines, lines_page)))
    }

    /// Which part of CEM Doc the part control `c` is (`CemPart`), with its page object; `None` for any other part.
    fn cem_part(&self, engine: &Engine, c: &ir::Control) -> Option<(CemPart, usize)> {
        let program = &engine.program;
        let part = c.part_page?;
        let page = program.objects[part].page.as_ref()?;
        let plain = page.triggers.is_empty()
            && page.extensions.is_empty()
            && page.ext_triggers.is_empty()
            && !engine.events.any_subscriber_of(part)
            && program.objects[part]
                .properties
                .iter()
                .all(|(k, _)| pinned_shape::PLAIN_PROPERTIES.contains(&k.to_ascii_lowercase().as_str()))
            && page.controls.iter().all(|pc| {
                pc.owner.is_none()
                    && pc.triggers.is_empty()
                    && pc.ext_triggers.is_empty()
                    && pc.props.iter().all(|(_, v)| matches!(v, ir::PropVal::Lit(_)))
                    && match pc.kind {
                        ir::ControlKind::Repeater => pc.binding == ir::Binding::None,
                        ir::ControlKind::Field => matches!(pc.binding, ir::Binding::Field(_)),
                        _ => false,
                    }
            });
        if !plain {
            return None;
        }
        // An unlinked part has a link without entries or none.
        let linked = c.part_link.as_ref().filter(|l| !l.entries.is_empty()).map(|l| {
            l.provider.is_none() && l.view.is_none() && l.entries.iter().all(|e| matches!(e, ir::LinkEntry::Field(..)))
        });
        if c.part_link
            .as_ref()
            .is_some_and(|l| l.entries.is_empty() && (l.provider.is_some() || l.view.is_some()))
        {
            return None;
        }
        let visible = c.props.iter().find(|(k, _)| k == "visible").map(|(_, v)| v);
        let others_plain = c.props.iter().all(|(k, v)| {
            k == "visible"
                || (matches!(k.as_str(), "applicationarea" | "subpagelink") && matches!(v, ir::PropVal::Lit(_)))
        });
        let page_type = engine.object_facts[part].page_type.clone();
        let list_part = page_type == Some(ir::PageType::Other("listpart".into()));
        let card_part = page_type == Some(ir::PageType::Other("cardpart".into()));
        let kind = match (list_part, card_part, linked, visible) {
            (true, _, Some(true), None) => CemPart::Lines,
            (_, true, Some(true), None) => CemPart::Info,
            (_, true, Some(true), Some(ir::PropVal::Lit(v))) if v.eq_ignore_ascii_case("false") => CemPart::Hidden,
            (_, true, None, Some(ir::PropVal::Routine(_))) => CemPart::Extra,
            _ => return None,
        };
        others_plain.then_some((kind, part))
    }

    /// The handler modes of `CemDocModal` (CemDocumentTests.al:886-923) as the calls they make on the page: nothing, or a
    /// value typed into Name (the field `name` with the OnValidate), or `First` and a value typed into a field of the ListPart
    /// (part control `lines`, modes 11 and 12); then nothing, OK or Cancel. A typed value that raised ends the calls (modes 8
    /// and 9 stop there). A handler that raises its own error makes no call (mode 7, see `close_document_after_handler_error`).
    fn document_handler_calls_pinned(calls: &[TreeCall], name: usize, lines: (usize, usize)) -> bool {
        use PageMethod::{Cancel, First, Invoke, OK};
        use TestCall::{Other, Page, Write};
        let ok = |c: &TreeCall| !c.failed && !c.no;
        let rest = match calls {
            [w] if w.part.is_none() && w.call == Write(name) => return true,
            [w, rest @ ..] if w.part.is_none() && w.call == Write(name) && ok(w) => rest,
            [f, w, rest @ ..]
                if f.part == Some(lines.0)
                    && w.part == Some(lines.0)
                    && f.call == Page(First)
                    && matches!(w.call, Write(_))
                    && ok(f)
                    && ok(w) =>
            {
                rest
            }
            all => all,
        };
        // Only the last call may have raised: the close OK or Cancel asked failed (:349 HandlerOk_QueryCloseError).
        if !rest.iter().all(|c| c.part.is_none()) || rest.iter().rev().skip(1).any(|c| c.failed) {
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
    /// A handler that caught an error before it raised its own is not the pinned mode 7 (it raises with nothing caught) and
    /// is refused (al-runner issue 7826, item 1).
    pub(super) fn close_document_after_handler_error(
        &mut self,
        st: &St,
        e: &Signal,
        caught: bool,
    ) -> Result<(), Signal> {
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
        let calls = st.borrow().tree_calls.clone();
        let typed_failed = matches!(
            calls.as_slice(),
            [TreeCall {
                part: None,
                call: TestCall::Write(_),
                failed: true,
                ..
            }]
        );
        if typed_failed && err.code == "TestValidation" {
            self.mark_closed(st);
            return Ok(());
        }
        // The handler's own error is pinned only for a handler that made no call (mode 7); a value typed before it, also one
        // whose save failed and that the handler caught, keeps main's refusal (al-runner PR 7816 review, round 3).
        if caught && calls.is_empty() {
            return unsupported(
                "a page left open by a ModalPageHandler that caught an error before it raised its own (not pinned)",
            );
        }
        if !calls.is_empty() || st.borrow().dirty || st.borrow().draft {
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
