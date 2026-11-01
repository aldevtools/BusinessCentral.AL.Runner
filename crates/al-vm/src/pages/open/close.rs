use super::*;
use al_builtins::record_op::RecordOp;

impl Session {
    /// `TestPage.Close()`: the page closes the way a Cancel does (corpus
    /// testpageclose/TestPageClosingActions.al:Dialog arms, `qcp:Cancel` after Close()).
    pub(in crate::pages) fn page_close(&mut self, st: &St) -> Result<Value, Signal> {
        // Card, List, Document, Worksheet and ListPlus report OK (handlers/TestPageCloseActionByPageTypeContracts.al:
        // TestPageClose_IsOk_ForEveryPageType; a Card also in handlers/TestPageCloseTriggersSelectionCardPageContracts.al:
        // OnQueryClosePage_PageAndExtension_BothRun_AndBothMustAgreeToClose); the dialogs report Cancel.
        // Other page types are not pinned and keep Cancel.
        let object = st.borrow().object as usize;
        let ok_type = self.engine.object_facts[object]
            .page_type
            .as_ref()
            .is_some_and(al_ir::PageType::is_plain);
        // A Card run modally, not in lookup mode, reports OK too (pages/lt-modal-query-close-veto/PageModalQueryCloseVeto.al:177
        // Modal_HandlerClose_VetoAlways `seen=OK;`); a lookup-mode modal is not pinned and keeps Cancel.
        let modal_card = {
            let s = st.borrow();
            s.modal && !s.lookup && self.engine.object_facts[object].page_type_lower() == "card"
        };
        let action = if ok_type && (!st.borrow().modal || modal_card) {
            "OK"
        } else {
            "Cancel"
        };
        st.borrow_mut().close_error = false;
        self.pages.test_close = true;
        let (r, origin) = self.traced(st, |s| s.close_with(st, action, false, false));
        // A refused TestPage.Close() leaves the variable not open all the same (corpus
        // handlers/TestPageCloseVeto_Tests.al: PlainVeto_SecondCloseRaisesNotOpen), and so does an error of OnQueryClosePage
        // shown as an unhandled message (pages/lt-cc-page-leads/CcPageLeads.al:611-621 TriggerError_OnQueryClosePage).
        let r = match r {
            Err(Signal::Error(e)) if st.borrow().close_error => {
                self.mark_closed(st);
                Err(Signal::Error(e))
            }
            // An error of OnClosePage comes wrapped, with the code TestWrapped, and the page is closed (CcPageLeads.al:624-633
            // TriggerError_OnClosePage `One or more errors occurred while closing the page.\n\n<text>`). Pinned for an AL
            // Error (code Dialog) and for a control add-in call (lt-ac/ACControlAddIn.al TZ_CallInOnClosePage_TestPageClose,
            // BC28PLUS); another error's text and code cannot be read.
            Err(Signal::Error(e)) if origin.as_deref() == Some("onclosepage") => {
                self.mark_closed(st);
                if e.code != "Dialog" && e.code != crate::control_add_in::NOT_CREATED {
                    return Err(Signal::Error(Box::new(al_builtins::native::unpinned_error())));
                }
                // Pinned by lt-ac/ACControlAddIn.al TZ_CallInOnAfterGetRecord_TestPageOpen, TZ_CallInOnAfterGetCurrRecord_TestPageOpen, TZ_CallInOnClosePage_TestPageClose, TZ_TwoProceduresFromActions_PageStaysUsable (was assumption `control-add-in-call-trigger-contexts`).
                Err(error_code(
                    format!("One or more errors occurred while closing the page.\n\n{}", e.message),
                    "TestWrapped",
                ))
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
                // TryFunction and over an earlier error (handlers/TestPageCloseVeto_Tests.al:245 RP_PlainVeto_LastErrorAfterClose,
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
            self.mark_closed(st);
        }
        r
    }

    /// A StandardDialog in lookup mode whose code changed the primary key of its stored row (a field's OnValidate sets
    /// `Rec.Code := 'CUSTOM'`, the Select Sending Options shape): LookupOK and LookupCancel rename the row, with the other
    /// fields as the page holds them. The table's OnRename runs (xRec the old key), not OnModifyRecord or OnModify, before
    /// OnQueryClosePage; after LookupCancel OnAfterGetCurrRecord runs once more (corpus master 97c29f86,
    /// pages/lt-y17-save-row-gone/Y17SaveRowGone.al:459 PgSave_TempKeyChangedByValidate_LookupOK `T.OnRename:CUSTOM x=DEF
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
    /// report/lt-y-request-page/YRequestPageTests.al:471 RS_RequestPageRec_ReadInReportTriggers: `typed` set on a control bound to
    /// Rec, then `rows` `A/20000/typed` and `pre_rec` `[A/20000/typed]`). Pinned for a row of a table that runs no code when it is
    /// written (no trigger, extension trigger, event subscriber or global trigger) on a page without OnModifyRecord, whose
    /// ModifyAllowed is not false; whether such code runs, and any other shape, are not pinned and stay refused.
    pub(super) fn request_ok_writes_changed_row(&mut self, st: &St, action: &str) -> Result<bool, Signal> {
        let object = st.borrow().object as usize;
        if action != "OK" || st.borrow().draft || flag_denied(&self.engine(), object, "ModifyAllowed")? {
            return Ok(false);
        }
        if self.declares(st, "onmodifyrecord") || self.engine().events.is_subscribed(object, "onmodifyrecordevent") {
            return Ok(false);
        }
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        if rec.borrow().temporary
            || self.has_write_hooks(&table)?
            || self.global_enabled(&table, crate::globaltriggers::GlobalOp::Modify)?
        {
            return Ok(false);
        }
        Ok(true)
    }

    /// Closes the page with `action`. The client first saves what was typed; a save the table
    /// refuses raises at Close() but not at an OK, which counts one validation error and leaves the
    /// page open (corpus testpageclose/TestPageClosingActions.al, Card_OK_AfterATableRefusedDuplicateKey_*,
    /// handlers/TestPageFailedInsert_Tests.al). Then OnQueryClosePage(action) decides, then
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
        let mapped = self.close_action(st, action);
        let action = mapped.as_str();
        // Cancel on a request page whose ModifyAllowed is false drops a changed row: no error, the stored row unchanged
        // (report/lt-y-request-page/YRequestPageTests.al:130-162, `modify = ok changed` then `rows` as seeded). OK writes it in the
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
        // A page that still holds a refused entry drops its unsaved row when it closes (pages/lt-page-controls-aa/AaFieldProps.al:
        // AA_CharAllowed_PageField `stored` and AA_CharAllowed_TableField `stored-after-page`: entries accepted before the refused one
        // are not stored either).
        // The same for a refused MinValue, MaxValue or number conversion (pages/lt-ah-pages/AhPageTests.al:73, :94, :116, :137:
        // `stored` as seeded, Close answers Yes). A refusal raised by the table's or the control's OnValidate is validated again
        // at Close (:155-159): Close answers Yes, stores nothing, and leaves that error as the last error with the code
        // ClientValidateField:NSValidateField. The trigger is not run again here: whether its other effects happen again at
        // Close is not pinned (docs/probes-needed.md). With two such refusals which error is left last is not pinned: the last
        // error then refuses to be read (pages/lt-f3-pages/F3ValidationAddinFilter.al:253 closes such a page and reads nothing).
        // A list drops its row the same way after a refusal of an OnValidate trigger, a new row included (pages/lt-ah-pages/
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
        // (corpus testpart/TestOkPartRow.al:OkInvoke_HeaderOnModifySeesThePartRowSavedFirst).
        self.save_parts(st)?;
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
        let save = self.close_denied_modify(st).and_then(|()| self.save_row(st));
        match save {
            Err(Signal::Error(e)) if swallow_save_errors => {
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
        // (testpart/TestPartCloseLinkEditableContracts.al:
        // Parts_CloseTriggers_AllPartsQueryClose_ThenHost_ThenPartsClose_ThenHost).
        // A part hidden by a Visible expression opened with the host and closes with it; one a literal `Visible = false` left
        // out never opened (pages/W1Parts.al:408-411). The order of a hidden part's close triggers among shown parts is not
        // pinned: refused when the hidden part declares them.
        let (open_parts, mixed_order) = {
            let s = st.borrow();
            let mut all: Vec<(usize, St, bool)> = s
                .parts
                .iter()
                .map(|(i, c)| (*i, c.clone(), false))
                .chain(s.hidden_parts.iter().map(|(i, c)| (*i, c.clone(), true)))
                .filter(|(_, c, _)| c.borrow().open)
                .collect();
            all.sort_by_key(|(i, _, _)| *i);
            let shown = all.iter().any(|(_, _, h)| !h);
            let hidden: Vec<St> = all.iter().filter(|(_, _, h)| *h).map(|(_, c, _)| c.clone()).collect();
            let parts: Vec<St> = all.into_iter().map(|(_, c, _)| c).collect();
            (parts, if shown { hidden } else { Vec::new() })
        };
        for hidden in &mixed_order {
            if ["onqueryclosepage", "onclosepage"]
                .iter()
                .any(|t| self.declares(hidden, t))
            {
                return unsupported(
                    "closing a host with a shown part and a part hidden by a Visible expression that has close triggers (order not pinned)",
                );
            }
        }
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
            // is not asked (pages/lt-modal-query-close-veto/PageModalQueryCloseVeto.al:530 RP_ListWithFactBox_PartVetoes_HostAllows:
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
        // (report/lt-request-page-record-triggers/RequestPageOnAfterGetCurrRecord.al:P195_RequestPage_Cancel_Order `e4`, _OpenThenOk_Order).
        if via_action
            && action == "Cancel"
            && self.engine.program.objects[st.borrow().object as usize].is_kind(al_ir::ObjKind::Report)
        {
            self.page_trigger(st, "onaftergetcurrrecord", vec![])?;
        }
        // An AL error raised in OnQueryClosePage is shown as a message and refuses the close
        // (corpus handlers/TestPageQueryCloseError_Tests.al:1-25).
        let veto = match self.query_close_answer(st, vec![Value::Option(ordinal, members)]) {
            Err(Signal::Error(e)) => {
                al_builtins::native::refuse_unpinned_text(&e, "The message shown for an error of OnQueryClosePage")?;
                st.borrow_mut().close_error = true;
                self.ui_message(e.message)?;
                // With a MessageHandler, Close() returns and the variable is not open (handlers/TestPageCloseVeto_Tests.al:170
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
        } else {
            refuse_error(r, "OnClosePage at a close by an action or a handler")?;
        }
        let r = self.page_event(st, "onclosepageevent", None, None).map(|_| None);
        refuse_error(r, "an OnClosePageEvent subscriber")?;
        // SaveValues: what the page's variable-bound controls hold is stored unless the Cancel
        // action closed it (corpus handlers/TestPageSaveValues.al, testpageclose/TestPageClosingActions.al:937-977;
        // Close() and a platform close store too, docs/probes-needed.md).
        if !(via_action && matches!(action, "Cancel" | "LookupCancel" | "No")) {
            self.store_page_values(st);
        } else {
            self.note_cancelled_saved_filters(st);
        }
        // Not on the blank new-line row of a page: corpus misc/lt-page-run-overloads RunModal_NonTempRecord_ShowsDbRows keeps
        // `Row.Code` = D1 after a handler that read the page down to its new line and pressed OK.
        // A line started with New() is the row the page ends on, blank or typed (pages/lt-page-runmodal-rec/
        // PageRunModalRecAfterClose.al:P174_NonTemp_StartA_HandlerNewRowThenOk, _NewRowTypedKeyThenOk, _NewRowThenCancel), and
        // Cancel leaves the row the cursor stood on in Rec as OK does (:P174_NonTemp_StartA_HandlerNextThenCancel).
        let on_row = st.borrow().cursor == Cursor::Row && !st.borrow().draft;
        let on_new = st.borrow().draft && st.borrow().new_by_call;
        // A page the platform closes after the handler returned hands back nothing on Cancel (handlers/
        // TestPageActionRunObjectPageRec_Tests.al:PageRunModalTargetMovingItsRecAndTheCallersRecord).
        // `Page.RunModal(Id, Rec)` called as a statement closes with OK and Cancel. When the handler invoked one, the row is kept
        // to tell whether Rec would change (`hand_back_selection` refuses a later use of Rec then). A page the platform closes
        // leaves Rec as it was (handlers/TestPageActionRunObjectPageRec_Tests.al:296
        // PageRunModalTargetMovingItsRecAndTheCallersRecord, called as a statement: `Row` stays on B).
        let statement = st.borrow().statement_run;
        let hands_back = action == "LookupOK"
            || (action == "LookupCancel" && (via_action || st.borrow().field_run))
            || (statement && via_action && (action == "OK" || action == "Cancel"));
        if hands_back && self.has_source(st) && (on_row || on_new) {
            let fields = self.page_rec(st).borrow().fields.clone();
            st.borrow_mut().selection = Some(fields);
        }
        self.mark_closed(st);
        st.borrow_mut().result = Some(action.to_string());
        Ok(Value::Null)
    }

    /// The built-in actions a page type offers (corpus handlers/TestPageBuiltInActions_Tests.al:1-30,
    /// testpageclose/TestPageClosingActions.al:25-30: a card or a list offers OK and no Cancel; a
    /// StandardDialog both; a NavigatePage only OK; a PromptDialog Cancel, and OK unless it declares
    /// systemaction(OK); a ConfirmationDialog neither).
    /// Whether `OK` of a lookup-mode modal page that shows no row is missing. Pinned for a list page variable with
    /// LookupMode(true) (errors/lt-ms-refusals-129/P129PageMisc.al:726 TV_GetRecord_AfterLookupOk_FilteredToNoRow, :772
    /// TV_GetRecord_AfterSetRecordAndLookupOk_FilteredToNoRow): the action is not found. The lookup of `Page.RunModal(Id, Rec)`
    /// and other page types are not pinned and are refused.
    pub(in crate::pages) fn lookup_ok_on_no_row(&mut self, st: &St) -> Result<bool, Signal> {
        let (modal, lookup, static_lookup) = {
            let s = st.borrow();
            (s.modal, s.lookup, s.static_lookup)
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
        if !self.is_list_page(st) {
            return Err(Signal::Unsupported(format!(
                "OK of a {page_type} page in lookup mode that shows no row (only a list page is pinned)"
            )));
        }
        // The static Page.RunModal(Id, Rec) on a list that shows no row is the same: an empty table with Rec cleared or holding
        // values, a list with Editable = false, a table with rows and Rec filtered to none (corpus PR 896,
        // errors/lt-ms-refusals-129/Y9LookupNoRow.al Y9_Lookup_RunModalIdRec_EmptyTable_Ok, _PrefilledRec_Ok, _ReadOnlyList_Ok,
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
    /// (errors/lt-ms-refusals-129/P129PageMisc.al:675-818 on corpus master, writer TV: SetTableView to no row).
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
        // (pages/lt-modal-handler-typed-page/PageTypedHandlerRec.al:214-270, not pinned here).
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
        // handlers/TestPageModalQueryClose_Tests.al (b2) and (d)).
        if !dialog && matches!(which, "OK" | "Cancel") && st.borrow().modal && st.borrow().lookup {
            return true;
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
}
