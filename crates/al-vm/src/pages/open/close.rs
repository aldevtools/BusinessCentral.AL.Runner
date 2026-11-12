use super::*;
use al_builtins::record_op::RecordOp;

impl Session {
    /// `TestPage.Close()`: the page closes the way a Cancel does (corpus
    /// testpage/lifecycle/TestPageClosingActions.al:Dialog arms, `qcp:Cancel` after Close()).
    pub(in crate::pages) fn page_close(&mut self, st: &St) -> Result<Value, Signal> {
        // Card, List, Document, Worksheet and ListPlus report OK (testpage/TestPageCloseActionByPageTypeContracts.al:
        // TestPageClose_IsOk_ForEveryPageType; a Card also in testpage/TestPageCloseTriggersSelectionCardPageContracts.al:
        // OnQueryClosePage_PageAndExtension_BothRun_AndBothMustAgreeToClose); the dialogs report Cancel.
        // Other page types are not pinned and keep Cancel.
        let object = st.borrow().object as usize;
        let ok_type = self.engine.object_facts[object]
            .page_type
            .as_ref()
            .is_some_and(al_ir::PageType::is_plain);
        // A Card run modally, not in lookup mode, reports OK too (pages/triggers/PageModalQueryCloseVeto.al:177
        // Modal_HandlerClose_VetoAlways `seen=OK;`); a lookup-mode modal is not pinned and keeps Cancel.
        let modal_card = {
            let s = st.borrow();
            s.modal && !s.lookup && self.engine.object_facts[object].page_type_lower() == "card"
        };
        // A NavigatePage (temporary or real source, after a footer action, OpenEdit or OpenView), a CardPart and a ListPart
        // not run modally report OK too (corpus branch al-runtime-rs-staging-y65,
        // testpage/lifecycle/Y65ClosePageTypes.al:505-558 and :636-660; al-runner issue 6670). Not on the
        // pinned corpus yet: assumption close-action-ok-navigate-and-parts.
        let navigate_or_part = !st.borrow().modal
            && matches!(
                self.engine.object_facts[object].page_type_lower(),
                "navigatepage" | "cardpart" | "listpart"
            );
        let action = if (ok_type && (!st.borrow().modal || modal_card)) || navigate_or_part {
            "OK"
        } else {
            "Cancel"
        };
        if let Some(r) = self.close_after_failed_save(st) {
            return r;
        }
        // The save of a value longer than its field fails at the Close with the code TestWrapped; the page stays open and a
        // second Close succeeds (Y80SetValueTypes.al Y80_ST_Card_ShortCtl_Integers `close`, as Y67CloseSaveError.al:239).
        let deferred = st.borrow_mut().deferred_overlong.take();
        if let Some((_, _, message)) = deferred {
            return Err(error_code(message, "TestWrapped"));
        }
        st.borrow_mut().close_error = false;
        self.pages.close_save_error = None;
        self.pages.test_close = true;
        let page_run = self.page_run_close(st, action, false, true)?;
        let (r, origin) = self.traced(st, |s| s.close_with(st, action, false, false));
        if let Some(r) = self.after_failed_close_save(st, &r) {
            return r;
        }
        // A refused TestPage.Close() leaves the variable not open all the same (corpus
        // testpage/TestPageCloseVeto_Tests.al: PlainVeto_SecondCloseRaisesNotOpen), and so does an error of OnQueryClosePage
        // shown as an unhandled message (testpage/CcPageLeads.al:611-621 TriggerError_OnQueryClosePage).
        let r = match r {
            Err(Signal::Error(e)) if st.borrow().close_error => {
                self.mark_closed(st);
                Err(Signal::Error(e))
            }
            // An error of OnClosePage comes wrapped, with the code TestWrapped, and the page is closed (CcPageLeads.al:624-633
            // TriggerError_OnClosePage `One or more errors occurred while closing the page.\\<text>`: the two CR LF between
            // the parts show as one backslash each in the error text, as `Error` shows a CR LF, CcPageLeads.al:634,
            // ChReportLeads.al:82, ACControlAddIn.al:250). Pinned for an AL
            // Error (code Dialog) and for a control add-in call (testpage/ACControlAddIn.al TZ_CallInOnClosePage_TestPageClose,
            // BC28PLUS); another error's text and code cannot be read.
            Err(Signal::Error(e)) if origin.as_deref() == Some("onclosepage") => {
                self.mark_closed(st);
                // A Confirm no handler took, in a relation lookup (CemLookupTests.al) or a pinned Page.Run page closed in its
                // handler (`page_run_test_close_confirm`).
                let confirm = (e.code == "MissingUIHandler" && self.relation_closed_in_handler(st))
                    || self.page_run_test_close_confirm(page_run, st, &e);
                if e.code != "Dialog" && e.code != crate::control_add_in::NOT_CREATED && !confirm {
                    return Err(Signal::Error(Box::new(al_builtins::native::unpinned_error())));
                }
                // Pinned by testpage/ACControlAddIn.al TZ_CallInOnAfterGetRecord_TestPageOpen, TZ_CallInOnAfterGetCurrRecord_TestPageOpen, TZ_CallInOnClosePage_TestPageClose, TZ_TwoProceduresFromActions_PageStaysUsable (was assumption `control-add-in-call-trigger-contexts`).
                Err(crate::pages::relation_lookup_close::closing_error(&e.message))
            }
            other => other,
        };
        if r.is_ok() {
            // A Close() the page's OnQueryClosePage refused inside the handler of a modal run closes the TestPage all the same;
            // RunModal then raises (PageModalQueryCloseVeto.al:167 Modal_HandlerClose_VetoAlways, :236
            // Modal_HandlerOkThenClose_VetoAlways).
            let vetoed_in_handler = {
                let s = st.borrow();
                s.close_vetoed && s.modal && self.ui_state.page_handler_depth > 0
            };
            if vetoed_in_handler {
                st.borrow_mut().test_close_vetoed = true;
            } else if st.borrow().modal && self.ui_state.page_handler_depth > 0 && !st.borrow().close_vetoed {
                st.borrow_mut().test_closed_in_handler = true;
            } else if st.borrow().close_vetoed {
                // A Close() the page's own OnQueryClosePage refused returns, and the veto is the last error, also inside a
                // TryFunction and over an earlier error (testpage/TestPageCloseVeto_Tests.al:245 RP_PlainVeto_LastErrorAfterClose,
                // :269 _LastErrorPerOpenMode, :335 _ReplacesAnEarlierLastError, :351 RP_ConfirmNoVeto_LastErrorAfterClose;
                // al-runner issue 5769). Pinned outside a page handler only.
                self.last_error = if self.ui_state.page_handler_depth == 0 {
                    al_builtins::native::AlError {
                        message: "The page cannot be closed at this time.".into(),
                        code: "FormCloseNotAllowed".into(),
                        ..Default::default()
                    }
                } else {
                    al_builtins::native::unpinned_error()
                };
            }
            self.mark_closed_by_test(st);
        }
        r
    }

    /// A StandardDialog in lookup mode whose code changed the primary key of its stored row (a field's OnValidate sets
    /// `Rec.Code := 'CUSTOM'`, the Select Sending Options shape): LookupOK and LookupCancel rename the row, with the other
    /// fields as the page holds them. The table's OnRename runs (xRec the old key), not OnModifyRecord or OnModify, before
    /// OnQueryClosePage; after LookupCancel OnAfterGetCurrRecord runs once more (corpus master 97c29f86,
    /// pages/save-deleted-row/Y17SaveRowGone.al:459 PgSave_TempKeyChangedByValidate_LookupOK `T.OnRename:CUSTOM x=DEF
    /// temp=Yes;P.OnQueryClosePage:LookupOK:CUSTOM`, :480 after a Copy from a stored row, :530 _Cancel `T.OnRename:CUSTOM x=DEF
    /// temp=Yes;P.AGCR:CUSTOM;P.OnQueryClosePage:LookupCancel:CUSTOM`, :551 a database source). Another page type, close action
    /// or a page with parts keeps the refusal of `save_row`.
    pub(super) fn rename_changed_key_at_close(&mut self, st: &St, action: &str) -> Result<(), Signal> {
        let shape = {
            let s = st.borrow();
            s.dirty && !s.draft && s.cursor == Cursor::Row && s.host.is_none() && s.parts.is_empty()
        };
        let object = st.borrow().object as usize;
        if !shape
            || !self.has_source(st)
            || self.engine.object_facts[object].page_type != Some(al_ir::PageType::StandardDialog)
            || !matches!(action, "LookupOK" | "LookupCancel")
        {
            return Ok(());
        }
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        let held = self.page_xrec_fields(st);
        let old_key = table.primary_key(&held);
        let mut new_key = table.primary_key(&rec.borrow().fields);
        if Self::same_key(&old_key, &new_key) {
            return Ok(());
        }
        // The row is renamed from the key the page read it under.
        {
            let mut r = rec.borrow_mut();
            for (&field, v) in table.keys[0].iter().zip(old_key.iter()) {
                r.fields[field] = v.clone();
            }
        }
        self.ret_used = false;
        self.record_op(RecordOp::Rename, &rec, &mut new_key)?;
        st.borrow_mut().dirty = false;
        self.set_xrec(st);
        if action == "LookupCancel" {
            self.page_trigger(st, "onaftergetcurrrecord", vec![])?;
            self.page_event(st, "onaftergetcurrrecordevent", None, None)?;
        }
        Ok(())
    }

    /// OK on a request page with a changed row writes the row, and the report reads it through Rec (corpus
    /// reports/request-page/YRequestPageTests.al:471 RS_RequestPageRec_ReadInReportTriggers: `typed` set on a control bound to
    /// Rec, then `rows` `A/20000/typed` and `pre_rec` `[A/20000/typed]`). Pinned for a row of a table that runs no code when it is
    /// written (no trigger, extension trigger, event subscriber or global trigger) on a page without OnModifyRecord, whose
    /// ModifyAllowed is not false; whether such code runs, and any other shape, are not pinned and stay refused.
    pub(super) fn request_ok_writes_changed_row(&mut self, st: &St, action: &str) -> Result<bool, Signal> {
        let object = st.borrow().object as usize;
        // Cancel writes it too, when ModifyAllowed is not false, before the row is read again (reports/request-page/
        // Y80ReportExtRequestPage.al Y80_RX_SetTxt_Cancel `rows` = `[10|typed|1]...`); OnModifyRecord of the page and of its
        // extension run first (`base modify typed | ext modify -`, Y80_RX_SetTxt_OK).
        if !matches!(action, "OK" | "Cancel")
            || st.borrow().draft
            || flag_denied(&self.engine(), object, "ModifyAllowed")?
        {
            return Ok(false);
        }
        if self.engine().events.is_subscribed(object, "onmodifyrecordevent") {
            return Ok(false);
        }
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        if rec.borrow().temporary || self.global_enabled(&table, crate::globaltriggers::GlobalOp::Modify)? {
            return Ok(false);
        }
        // The table's own code at that write (OnModify, its extensions', the table events) is not observed by the pinning
        // cases, and no Microsoft test needs it, so it is refused (needs probe: docs/probes-needed.md
        // "request page close writes a changed row: table code").
        if self.has_write_hooks(&table)? {
            return unsupported(format!(
                "{action} on a request page with a changed row of a table that runs code when it is written (only a table without OnModify, extension triggers and table events is pinned)"
            ));
        }
        Ok(true)
    }

    /// Closes the page with `action`. The client first saves what was typed; a save the table
    /// refuses raises at Close() but not at an OK, which counts one validation error and leaves the
    /// page open (corpus testpage/lifecycle/TestPageClosingActions.al, Card_OK_AfterATableRefusedDuplicateKey_*,
    /// testpage/TestPageFailedInsert_Tests.al). Then OnQueryClosePage(action) decides, then
    /// OnClosePage runs (TestPageClosingActions.al:Card_OK_RaisesTheCloseTriggersOnce).
    pub(in crate::pages) fn close_with(
        &mut self,
        st: &St,
        action: &str,
        swallow_save_errors: bool,
        via_action: bool,
    ) -> Result<Value, Signal> {
        let test_close = std::mem::take(&mut self.pages.test_close);
        self.require_open(st)?;
        st.borrow_mut().close_vetoed = false;
        self.note_relation_close(st, action, test_close, via_action);
        let page_run = self.page_run_close(st, action, via_action, test_close)?;
        let mapped = self.close_action(st, action);
        let action = mapped.as_str();
        // Cancel on a request page whose ModifyAllowed is false drops a changed row: no error, the stored row unchanged
        // (reports/request-page/YRequestPageTests.al:130-162, `modify = ok changed` then `rows` as seeded). OK writes it in the
        // shape `request_ok_writes_changed_row` names; with ModifyAllowed true at Cancel, or another shape at OK, what happens to a
        // changed row of a request page is not pinned.
        if self.is_request_page(st) && self.has_source(st) {
            let changed = {
                let s = st.borrow();
                s.dirty && s.cursor == Cursor::Row
            };
            if changed {
                let rec = self.page_rec(st);
                let differs = !Self::same_values(&rec.borrow().fields, &self.page_xrec_fields(st));
                let object = st.borrow().object as usize;
                // OK that writes the row leaves it changed for the save below (`request_ok_writes_changed_row`).
                let writes = differs && self.request_ok_writes_changed_row(st, action)?;
                if differs
                    && !writes
                    && (action != "Cancel"
                        || st.borrow().draft
                        || !flag_denied(&self.engine(), object, "ModifyAllowed")?)
                {
                    return unsupported(format!(
                        "{action} on a request page with a changed row (only Cancel with ModifyAllowed = false, and OK on a table that runs no code when it is written, are pinned)"
                    ));
                }
                if !writes {
                    st.borrow_mut().dirty = false;
                }
            }
        }
        if st.borrow().refused {
            // The refused line was reported when it was left; closing drops it.
            let mut s = st.borrow_mut();
            s.refused = false;
            s.draft = false;
            s.dirty = false;
        }
        // A page that still holds a refused entry drops its unsaved row when it closes (pages/controls/AaFieldProps.al:
        // AA_CharAllowed_PageField `stored` and AA_CharAllowed_TableField `stored-after-page`: entries accepted before the refused one
        // are not stored either).
        // The same for a refused MinValue, MaxValue or number conversion (testpage/AhPageTests.al:73, :94, :116, :137:
        // `stored` as seeded, Close answers Yes). A refusal raised by the table's or the control's OnValidate is validated again
        // at Close (:155-159): Close answers Yes, stores nothing, and leaves that error as the last error with the code
        // ClientValidateField:NSValidateField. The trigger is not run again here: whether its other effects happen again at
        // Close is not pinned (docs/probes-needed.md). With two such refusals which error is left last is not pinned: the last
        // error then refuses to be read (testpage/F3ValidationAddinFilter.al:253 closes such a page and reads nothing).
        // A list drops its row the same way after a refusal of an OnValidate trigger, a new row included (testpage/
        // PwCloseRevalidate.al:247 PW_List_RefusedOnValidate_ThenClose, :303 PW_List_NewRow_RefusedOnValidate_ThenClose: Close
        // answers ok, runs no OnModify, stores nothing); the last error it leaves is not pinned. After other refusals a list keeps
        // its edits (the earlier behavior; not pinned: docs/probes-needed.md, Close after a refused entry).
        let list = self.is_list(st);
        let list_trigger_refusal = list && !st.borrow().trigger_errors.is_empty();
        if !st.borrow().errors.is_empty() && (!list || list_trigger_refusal) {
            let trigger_errors = st.borrow().trigger_errors.clone();
            match trigger_errors.as_slice() {
                [] => {}
                _ if list => self.last_error = al_builtins::native::unpinned_error(),
                [(_, message)] => {
                    self.last_error = al_builtins::native::AlError {
                        message: message.clone(),
                        code: "ClientValidateField:NSValidateField".into(),
                        call_stack: String::new(),
                        ..Default::default()
                    };
                }
                _ => self.last_error = al_builtins::native::unpinned_error(),
            }
            let mut s = st.borrow_mut();
            s.draft = false;
            s.dirty = false;
        }
        // BC's client saves the child forms before the form's own record on a saving close
        // (corpus testpage/parts/TestOkPartRow.al:OkInvoke_HeaderOnModifySeesThePartRowSavedFirst).
        let saved = self.save_parts(st);
        if let Some(e) = self.document_part_save_error(st, &saved, via_action)? {
            return Err(e);
        }
        saved?;
        let pending = st.borrow_mut().pending.take();
        if let Some(fields) = pending {
            let rec = self.page_rec(st);
            rec.borrow_mut().fields = fields;
            self.ret_used = false;
            self.tx.immediate_errors = true;
            let inserted = self.record_op(RecordOp::Insert, &rec, &mut [Value::Boolean(true)]);
            self.tx.immediate_errors = false;
            inserted?;
        }
        self.rename_changed_key_at_close(st, action)?;
        let host_saves = st.borrow().dirty || st.borrow().draft;
        let quiet = self.quiet_parts_at_new_host_close(st)?;
        let save = self.close_denied_modify(st).and_then(|()| self.close_save_row(st));
        for c in quiet {
            c.borrow_mut().quiet_at_close = false;
        }
        match save {
            Err(Signal::Error(e)) if swallow_save_errors => {
                self.refuse_page_run_save_error_at_action(page_run, st, &e)?;
                al_builtins::native::refuse_unpinned_text(&e, "The error list of a page row")?;
                let key_ctl = self.key_control(st);
                st.borrow_mut().errors.push((key_ctl, e.message));
                return Ok(Value::Null);
            }
            other => other?,
        }
        self.stale_parts_read_at_close(st, host_saves)?;
        let members: std::sync::Arc<[String]> = al_builtins::value::ACTION_MEMBERS
            .iter()
            .map(|m| m.to_string())
            .collect();
        let ordinal = al_builtins::value::ACTION_MEMBERS
            .iter()
            .position(|m| *m == action)
            .unwrap_or(0) as i32;
        // The parts are asked first, with the action None; the host's own OnQueryClosePage follows
        // (testpage/parts/TestPartCloseLinkEditableContracts.al:
        // Parts_CloseTriggers_AllPartsQueryClose_ThenHost_ThenPartsClose_ThenHost).
        // A part hidden by a Visible expression opened with the host and closes with it; one a literal `Visible = false` left
        // out never opened (pages/W1Parts.al:408-411). When no shown part runs close code, a hidden part's close triggers run
        // as with no shown part (testpage/parts/HiddenPartExtensionVariable.al:535 Hidden_OpenEdit_HostActions `closed`
        // `[fb-query fb-close]`, its shown part Lines has none). Their order among shown parts that do run close code is not
        // pinned: refused.
        let open_parts = {
            let s = st.borrow();
            let mut all: Vec<(usize, St, bool)> = s
                .parts
                .iter()
                .map(|(i, c)| (*i, c.clone(), false))
                .chain(s.hidden_parts.iter().map(|(i, c)| (*i, c.clone(), true)))
                .filter(|(_, c, _)| c.borrow().open)
                .collect();
            all.sort_by_key(|(i, _, _)| *i);
            all
        };
        let observes = |hidden: bool| {
            open_parts
                .iter()
                .any(|(_, c, h)| *h == hidden && self.runs_close_code(c))
        };
        if observes(true) && observes(false) {
            return unsupported(
                "closing a host with a shown part and a part hidden by a Visible expression that both have close triggers (order not pinned)",
            );
        }
        let mut open_parts: Vec<St> = open_parts.into_iter().map(|(_, c, _)| c).collect();
        self.no_source_host_part_close_order(st, &mut open_parts)?;
        let none_ordinal = al_builtins::value::ACTION_MEMBERS
            .iter()
            .position(|m| *m == "None")
            .unwrap_or(0) as i32;
        for part in &open_parts {
            if !part.borrow().parts.is_empty() {
                return unsupported("closing a host whose part page has parts of its own (close order not pinned)");
            }
            let answer = match self.query_close_answer(part, vec![Value::Option(none_ordinal, members.clone())]) {
                Err(Signal::Error(_)) => {
                    return unsupported("an error in a part's OnQueryClosePage (not pinned)");
                }
                other => other?,
            };
            // A modal page whose part's own OnQueryClosePage answers false is refused like the host's own veto, and the host
            // is not asked (pages/triggers/PageModalQueryCloseVeto.al:530 RP_ListWithFactBox_PartVetoes_HostAllows:
            // `part_noinvoke_hostcount` 0, None, FormCloseNotAllowed). A part's veto of a page not run modally is not pinned.
            if answer == Some(false) && st.borrow().modal {
                st.borrow_mut().close_error = false;
                st.borrow_mut().close_vetoed = true;
                return Ok(Value::Null);
            }
            if answer == Some(false) || !self.page_event(part, "onqueryclosepageevent", Some(true), None)? {
                return unsupported("a part's OnQueryClosePage that refuses the close (not pinned)");
            }
        }
        // Cancelling a report's request page runs OnAfterGetCurrRecord once more before OnQueryClosePage; OK does not
        // (reports/request-page/RequestPageOnAfterGetCurrRecord.al:P195_RequestPage_Cancel_Order `e4`, _OpenThenOk_Order).
        if via_action
            && action == "Cancel"
            && !st.borrow().request_new_blank
            && self.engine.program.objects[st.borrow().object as usize].is_kind(al_ir::ObjKind::Report)
        {
            // With a source table the row is read again first (OnAfterGetRecord; reports/request-page/
            // Y80ReportExtRequestPage.al Y80_RX_ReadOnly_Cancel `after run log`).
            if self.has_source(st) {
                self.page_trigger(st, "onaftergetrecord", vec![])?;
            }
            self.page_trigger(st, "onaftergetcurrrecord", vec![])?;
        }
        self.modal_card_empty_read_at_close(st)?;
        // An AL error raised in OnQueryClosePage is shown as a message and refuses the close
        // (corpus text/TestPageQueryCloseError_Tests.al:1-25).
        let veto = match self.query_close_answer(st, vec![Value::Option(ordinal, members)]) {
            Err(Signal::Error(e)) => {
                // A Confirm with no ConfirmHandler at TestPage.Close() raises its own error (query_close_unhandled.rs).
                if let Some(raised) = test_close.then(|| self.unhandled_confirm_in_query_close(&e)).flatten() {
                    st.borrow_mut().close_error = true;
                    return Err(raised);
                }
                if let Some(r) = self.page_run_query_close_error(page_run, st, &e) {
                    return r;
                }
                al_builtins::native::refuse_unpinned_text(&e, "The message shown for an error of OnQueryClosePage")?;
                st.borrow_mut().close_error = true;
                // The error stays the last error once a MessageHandler consumed it: GetLastErrorText after Close() is its text, with
                // its code (corpus master text/TestPageQueryCloseErrorMessageHandler_Tests.al: clean row, row dirty from another
                // field, row dirty after a confirmed OnValidate; al-runner issue 7457).
                self.last_error = (*e).clone();
                self.ui_message(e.message)?;
                // With a MessageHandler, Close() returns and the variable is not open (testpage/TestPageCloseVeto_Tests.al:170
                // ErrorConsumedByMessageHandler_SecondCloseRaisesNotOpen, TestPageQueryCloseMessageHandler_Tests.al:193
                // ErrorInQueryClosePage_TestPageClose_MessageConsumed_CallerRegainsControl, TestPageNeverOpened_Tests.al:155
                // ErrorConsumedClose_VariableOpensAgain).
                return Ok(Value::Null);
            }
            other => other?,
        };
        st.borrow_mut().close_error = false;
        // A veto of a modal page with open parts answers as one without parts: the parts are asked first, with None, once per
        // close attempt (PageModalQueryCloseVeto.al:448 RP_ListWithFactBox_HandlerDoesNothing_HostVetoes, :463 _HandlerOk_HostVetoes,
        // :509 RP_CardWithListPart_HostVetoes; al-runner issue 5762).
        st.borrow_mut().close_vetoed = veto == Some(false);
        if veto == Some(false) {
            self.refuse_relation_close_veto(st)?;
            return Ok(Value::Null);
        }
        if !self.page_event(st, "onqueryclosepageevent", Some(true), None)? {
            return Ok(Value::Null);
        }
        // Only an AL error of the page's own OnClosePage at TestPage.Close() is pinned (CcPageLeads.al:624-633, wrapped in
        // `page_close`). An error of a part's OnClosePage or of an OnClosePageEvent subscriber, and an OnClosePage error at a
        // close by an action or a handler, are refused.
        let refuse_error = |r: Result<Option<Value>, Signal>, what: &str| -> Result<(), Signal> {
            match r {
                Err(Signal::Error(_)) => unsupported(format!("an error in {what} while the page closes (not pinned)")),
                other => other.map(|_| ()),
            }
        };
        for part in &open_parts {
            let r = self.page_trigger(part, "onclosepage", vec![]);
            refuse_error(r, "a part's OnClosePage")?;
            let r = self.page_event(part, "onclosepageevent", None, None).map(|_| None);
            refuse_error(r, "an OnClosePageEvent subscriber of a part")?;
        }
        let r = self.page_trigger(st, "onclosepage", vec![]);
        if test_close {
            r?;
        } else if let Some(e) = self.relation_close_page_error(st, &r) {
            return Err(e);
        } else if let Some(e) = self.document_close_page_error(st, &r)? {
            return Err(e);
        } else if let Some(closed) = self.page_run_close_page_error(page_run, st, &r) {
            closed?;
        } else {
            refuse_error(r, "OnClosePage at a close by an action or a handler")?;
        }
        let r = self.page_event(st, "onclosepageevent", None, None).map(|_| None);
        refuse_error(r, "an OnClosePageEvent subscriber")?;
        // SaveValues: what the page's variable-bound controls hold is stored unless the Cancel
        // action closed it (corpus pages/properties/TestPageSaveValues.al, testpage/lifecycle/TestPageClosingActions.al:937-977;
        // Close() and a platform close store too, docs/probes-needed.md).
        // A Cancel keeps what the earlier OK stored, filters included (reports/request-page/Y86RequestFilters.al Y86_CancelKeeps_*).
        if !(via_action && matches!(action, "Cancel" | "LookupCancel" | "No")) {
            self.store_page_values(st);
        }
        // Not on the blank new-line row of a page: corpus misc/lt-page-run-overloads RunModal_NonTempRecord_ShowsDbRows keeps
        // `Row.Code` = D1 after a handler that read the page down to its new line and pressed OK.
        // A line started with New() is the row the page ends on, blank or typed (pages/run-modal/
        // PageRunModalRecAfterClose.al:P174_NonTemp_StartA_HandlerNewRowThenOk, _NewRowTypedKeyThenOk, _NewRowThenCancel), and
        // Cancel leaves the row the cursor stood on in Rec as OK does (:P174_NonTemp_StartA_HandlerNextThenCancel).
        let on_row = st.borrow().cursor == Cursor::Row && !st.borrow().draft;
        let on_new = st.borrow().draft && st.borrow().new_by_call;
        // A page the platform closes after the handler returned hands back nothing on Cancel (handlers/
        // TestPageActionRunObjectPageRec_Tests.al:PageRunModalTargetMovingItsRecAndTheCallersRecord).
        // `Page.RunModal(Id, Rec)` called as a statement closes with OK and Cancel. When the handler invoked one, the row is kept
        // to tell whether Rec would change (`hand_back_selection` refuses a later use of Rec then). A page the platform closes
        // leaves Rec as it was (testpage/TestPageActionRunObjectPageRec_Tests.al:296
        // PageRunModalTargetMovingItsRecAndTheCallersRecord, called as a statement: `Row` stays on B).
        let statement = st.borrow().statement_run;
        let hands_back = action == "LookupOK"
            || (action == "LookupCancel" && (via_action || st.borrow().field_run))
            || (statement && via_action && (action == "OK" || action == "Cancel"));
        if hands_back && self.has_source(st) && (on_row || on_new) {
            let fields = self.page_rec(st).borrow().fields.clone();
            st.borrow_mut().selection = Some(fields);
        }
        let by = if test_close {
            ClosedBy::TestClose
        } else if via_action {
            ClosedBy::Action
        } else {
            ClosedBy::Platform
        };
        self.mark_closed_as(st, by);
        st.borrow_mut().result = Some(action.to_string());
        Ok(Value::Null)
    }

    /// The built-in actions a page type offers (corpus ui-handlers/TestPageBuiltInActions_Tests.al:1-30,
    /// testpage/lifecycle/TestPageClosingActions.al:25-30: a card or a list offers OK and no Cancel; a
    /// StandardDialog both; a NavigatePage only OK; a PromptDialog Cancel, and OK unless it declares
    /// systemaction(OK); a ConfirmationDialog neither).
    /// Whether `OK` of a lookup-mode modal page that shows no row is missing. Pinned for a list page variable with
    /// LookupMode(true) (testpage/P129PageMisc.al:726 TV_GetRecord_AfterLookupOk_FilteredToNoRow, :772
    /// TV_GetRecord_AfterSetRecordAndLookupOk_FilteredToNoRow): the action is not found. The lookup of `Page.RunModal(Id, Rec)`
    /// and other page types are not pinned and are refused.
    pub(in crate::pages) fn lookup_ok_on_no_row(&mut self, st: &St) -> Result<bool, Signal> {
        let (modal, lookup, static_lookup) = {
            let s = st.borrow();
            (s.modal, s.lookup, s.lookup_kind == LookupKind::StaticRunModal)
        };
        if !modal || !lookup || self.is_request_page(st) || !self.has_source(st) {
            return Ok(false);
        }
        let page_type = {
            let engine = self.engine();
            engine.object_facts[st.borrow().object as usize]
                .page_type_lower()
                .to_string()
        };
        if matches!(
            page_type.as_str(),
            "confirmationdialog" | "standarddialog" | "promptdialog" | "navigatepage"
        ) {
            return Ok(false);
        }
        if self.page_view_has_rows(st) {
            return Ok(false);
        }
        // A Card that the static Page.RunModal(Id, Rec) opened on a record filtered to no row is the same (pages/lt-y61-page-probes/
        // Y61StoredRecToTempCard.al:499 G617b_RecordFilteredToNoRow: OK is not found, the answer is LookupCancel), and so is a
        // Card page variable with LookupMode(true) over a view filtered to no row (pages/run-modal/
        // Y68LookupGetRecord.al:425 Q3_TestPageHandler_OK_NoRow_Card, corpus master 0d01b0cc).
        if !self.is_list_page(st) && page_type != "card" {
            return Err(Signal::unsupported(format!(
                "OK of a {page_type} page in lookup mode that shows no row (only a list page is pinned)"
            )));
        }
        // The static Page.RunModal(Id, Rec) on a list that shows no row is the same: an empty table with Rec cleared or holding
        // values, a list with Editable = false, a table with rows and Rec filtered to none (corpus PR 896,
        // pages/run-modal/Y9LookupNoRow.al Y9_Lookup_RunModalIdRec_EmptyTable_Ok, _PrefilledRec_Ok, _ReadOnlyList_Ok,
        // _FilteredToNoRow_Ok).
        if static_lookup {
            // Pinned for a page that showed no row from the start; one whose row left the view while it was open (the handler
            // changed the row in the database) is not.
            if st.borrow().cursor == Cursor::Row {
                return Err(Signal::Unsupported(
                    "OK of the lookup page of Page.RunModal(Id, Rec) whose row left its view while it was open (not pinned)".into(),
                ));
            }
        }
        Ok(true)
    }

    /// A list with a source table and no Editable property whose view shows no row of a table that has rows
    /// (testpage/P129PageMisc.al:675-818 on corpus master, writer TV: SetTableView to no row).
    pub(in crate::pages) fn plain_list_with_rows(&mut self, st: &St) -> bool {
        if !self.has_source(st) || !self.is_list_page(st) {
            return false;
        }
        let declares_editable = {
            let engine = self.engine();
            let object = st.borrow().object as usize;
            engine.program.objects[object]
                .properties
                .iter()
                .any(|(k, _)| k.eq_ignore_ascii_case("Editable"))
        };
        // The view (the filters of SetTableView) shows no row; a typed handler's page whose view has rows is another case
        // (pages/run-modal/PageTypedHandlerRec.al:214-270, not pinned here).
        if declares_editable || self.page_view_has_rows(st) {
            return false;
        }
        let probe = Rc::new(RefCell::new(self.page_rec(st).borrow().clone()));
        self.ret_used = true;
        if self.record_op(RecordOp::Reset, &probe, &mut []).is_err() {
            return false;
        }
        self.ret_used = true;
        matches!(
            self.record_op(RecordOp::FindFirst, &probe, &mut []),
            Ok(Value::Boolean(true))
        )
    }

    pub(in crate::pages) fn builtin_offered(&self, st: &St, which: &str) -> bool {
        if self.is_request_page(st) {
            return self.request_builtin_offered(st, which);
        }
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let page_type = engine.object_facts[object].page_type_lower().to_string();
        let dialog = matches!(
            page_type.as_str(),
            "confirmationdialog" | "standarddialog" | "promptdialog" | "navigatepage"
        );
        // A lookup-mode modal offers OK and Cancel whatever its type (corpus
        // ui-handlers/TestPageModalQueryClose_Tests.al (b2) and (d)).
        if !dialog && matches!(which, "OK" | "Cancel") && st.borrow().modal && st.borrow().lookup {
            return !(which == "Cancel" && self.lookup_part_without_cancel(st));
        }
        match (page_type.as_str(), which) {
            ("confirmationdialog", "Yes" | "No") => true,
            ("confirmationdialog", _) => false,
            (_, "Yes" | "No") => false,
            ("standarddialog", _) => true,
            ("navigatepage", "Cancel") => false,
            ("promptdialog", "Cancel") => true,
            ("promptdialog", "OK") => !page_of(&engine, st)
                .controls
                .iter()
                .any(|c| c.kind == ir::ControlKind::SystemAction && c.name.eq_ignore_ascii_case("OK")),
            (_, "Cancel") => false,
            _ => true,
        }
    }

    /// Whether closing part page `st` runs AL code: its or a page extension's OnQueryClosePage or OnClosePage, or a subscriber
    /// of their events.
    pub(in crate::pages) fn runs_close_code(&self, st: &St) -> bool {
        let object = st.borrow().object as usize;
        ["onqueryclosepage", "onclosepage"]
            .iter()
            .any(|t| self.declares_or_ext_declares(st, t))
            || ["onqueryclosepageevent", "onclosepageevent"]
                .iter()
                .any(|t| self.engine().events.is_subscribed(object, t))
    }
}
