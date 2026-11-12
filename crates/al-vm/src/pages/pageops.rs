use super::*;
use al_builtins::page_op::{PageMethod, PageOp, PageRecv};
use al_builtins::record_op::RecordOp;

impl Session {
    /// The `TestPage.*` / `TestField.*` methods (ADR-012).
    pub(crate) fn page_op(
        &mut self,
        method: PageOp,
        recv: Option<&mut Value>,
        args: &mut [Value],
    ) -> Result<Value, Signal> {
        if self.without_client() {
            self.clientless_page_guard(method, recv.as_deref(), args)?;
        }
        // CurrPage calls from the page's own code do not count as the test code touching the page.
        let test_page = match (recv.as_deref(), method.recv()) {
            (Some(Value::TestPage(st)), al_builtins::page_op::PageRecv::TestPage) => Some(st_of(st).clone()),
            _ => None,
        };
        if let Some(st) = &test_page {
            self.flush_data_caption(st)?;
        }
        let r = self.outside_used_tries(|s| s.page_op_outer(method, recv, args));
        if let (Ok(_), Some(st)) = (&r, &test_page) {
            self.page_row_size_guard(st)?;
        }
        self.save_abort_swallowed(method, r)
    }

    /// A TestPage whose record holds a placeholder's size in a field is refused after the step that loaded it: the page
    /// compares its row with the stored one and with xRec to see what changed (`same_values`), which counts a size that
    /// does not compare as a change (ADR-040 amendment 2026-11-12b, point 4: record comparison by value).
    fn page_row_size_guard(&mut self, st: &St) -> Result<(), Signal> {
        if !al_builtins::placeholder_output::sizes_made() {
            return Ok(());
        }
        let object = st.borrow().object as usize;
        let open = st.borrow().instance.is_some();
        if !open || self.engine.program.objects[object].rec_slot.is_none() {
            return Ok(());
        }
        let rec = self.page_rec(st);
        let r = rec.borrow();
        r.fields
            .iter()
            .try_for_each(|v| al_builtins::placeholder_output::size_read(v, "a page on a record with a field holding the value"))
    }

    /// A save the page refused with SaveAbort (ModifyAllowed is false, see `save_row`) ends the step without raising: the error becomes
    /// the last error and the move is not made. GoToKey raises TestRowNotFound instead (pages/modify-denied/Y80ModifyDenied.al
    /// Y80_MD_List_Set_goto: ok No, `The row does not exist on the TestPage.`). The answer of a move that did not happen is not
    /// pinned (the corpus discards it: `P.Next();` in Y80_MD_List_Set_Next_Next), so it is an unknown value: a discarded answer
    /// runs, any use of it is refused (hard rule 3).
    fn save_abort_swallowed(&mut self, method: PageOp, r: Result<Value, Signal>) -> Result<Value, Signal> {
        let Err(Signal::Error(e)) = r else { return r };
        if e.code != "SaveAbort" {
            return Err(Signal::Error(e));
        }
        match method.method() {
            PageMethod::GoToKey => Err(error_code("The row does not exist on the TestPage.", "TestRowNotFound")),
            m => {
                self.last_error = *e;
                Ok(match m {
                    PageMethod::Next | PageMethod::Previous | PageMethod::First | PageMethod::Last => {
                        Value::Unknown(std::sync::Arc::from(
                            "the answer of a TestPage move that did not happen after a refused save (not pinned)",
                        ))
                    }
                    _ => Value::Null,
                })
            }
        }
    }

    fn page_op_outer(&mut self, method: PageOp, recv: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
        // A TestPage begins a transaction of its own (see `in_page_transaction`). Page.Run and Page.RunModal with a handler that writes are allowed under None, and the
        // transaction the page began has ended afterwards
        // (codeunits/TestWriteTxNoneModelContracts.al:200-216 and :163-185).
        if recv.is_none() && self.no_transaction() {
            if matches!(method, PageOp::PageRunStatic | PageOp::PageRunModalStatic) {
                return self.in_page_transaction(|s| s.page_op_inner(method, recv, args));
            }
            return unsupported(format!("{method} under TransactionModel::None (not pinned)"));
        }
        if recv.is_some() && self.no_transaction() {
            return self.in_page_transaction(|s| s.page_op_inner(method, recv, args));
        }
        self.page_op_inner(method, recv, args)
    }

    pub(super) fn page_op_inner(
        &mut self,
        method: PageOp,
        recv: Option<&mut Value>,
        args: &mut [Value],
    ) -> Result<Value, Signal> {
        match recv {
            None if matches!(
                method,
                PageOp::PageGetBackgroundParameters | PageOp::PageSetBackgroundTaskResult
            ) =>
            {
                self.bg_static(method, args)
            }
            None if method == PageOp::PageRunStatic || method == PageOp::PageRunModalStatic => {
                let used = self.ret_used;
                // The run names its page by id and hands a record (`PageRunCall::of_static`).
                let (by_id, record_arg) = (
                    matches!(args.first(), Some(Value::Integer(_))),
                    matches!(args.get(1), Some(Value::Record(_))),
                );
                if let Some(Value::Text(name)) = args.first() {
                    // An empty name: with a record AL:MetadataNotFound like page 0, without one an uncatchable
                    // NullReferenceException on BC, refused (pages/missing-object-fqn/Y89FqnMissing.al:125, :153; :113 is no [Test]).
                    if name.trim().is_empty() {
                        return match args.len() > 1 {
                            true => Err(self.missing_object("Page", 0)),
                            false => {
                                unsupported("Page.Run with an empty page name and no record (ends the session on BC)")
                            }
                        };
                    }
                    let id = self.page_id_by_qualified_name(name)?;
                    args[0] = Value::Integer(id);
                }
                let n = match args.first() {
                    Some(Value::Integer(n)) => *n,
                    other => return unsupported(format!("Page.Run with {} as the page", other.describe())),
                };
                let engine = self.engine();
                let record = match args.get(1) {
                    Some(Value::Record(r)) => Some(r.clone()),
                    // A Variant that holds a RecordRef runs the page on the RecordRef's record: the handler sees it, and a pick or an
                    // edit reaches the RecordRef and the Variant (pages/run-modal/CePageRun.al:382 PageRunModal_VariantHoldingRecordRef,
                    // :548 RP_RunModal_VariantRecRef_FieldZero_Card, :594 RP_RunModal_VariantRecRef_FieldNo_ListPick, :633
                    // RP_Run_VariantRecRef_FieldZero_Card; al-runner issue 5759).
                    Some(Value::Library(_, lib))
                        if matches!(&*lib.borrow(), al_builtins::value::LibraryValue::RecordRef(_)) =>
                    {
                        match &*lib.borrow() {
                            al_builtins::value::LibraryValue::RecordRef(r) => Some(r.clone()),
                            _ => None,
                        }
                    }
                    // A Variant that holds an Integer or a Char is no record (CePageRun.al:654 RP_RunModal_VariantNotARecord_FieldZero).
                    Some(v @ (Value::Integer(_) | Value::Char(_))) => {
                        let from = if matches!(v, Value::Integer(_)) {
                            "NavInteger"
                        } else {
                            "NavChar"
                        };
                        return Err(error_code(
                            format!(
                                "Unable to convert from Microsoft.Dynamics.Nav.Runtime.{from} to Microsoft.Dynamics.Nav.Runtime.INavRecordHandle."
                            ),
                            "Conversion",
                        ));
                    }
                    None => None,
                    other => return unsupported(format!("Page.Run with {} as the record", other.describe())),
                };
                // A third argument is a field of the record (`Page.RunModal(Id, Rec, Rec.Field)`), arriving as its value;
                // the pinned case is the page with no handler (see `run_page_ui_with`).
                let field = match args.len() {
                    0..=2 => false,
                    3 => true,
                    _ => return unsupported("Page.Run with more than three arguments"),
                };
                // A FieldRef nobody assigned raises before anything opens (pages/parts/P106RunErrors.al:
                // P106_Err_FieldRefVariants, observation 'uninitialized').
                if let Some(Value::Library(_, lib)) = args.get(2) {
                    if matches!(
                        &*lib.borrow(),
                        al_builtins::value::LibraryValue::UserNameFieldRef { .. }
                    ) {
                        return unsupported("Page.Run with a FieldRef to a user-name system FlowField (not pinned)");
                    }
                    if !matches!(
                        &*lib.borrow(),
                        al_builtins::value::LibraryValue::FieldRef(..)
                            | al_builtins::value::LibraryValue::KeyFieldRef(..)
                    ) {
                        return Err(error_code(
                            "Microsoft.Dynamics.Nav.Runtime.NavFieldRef variable not initialized.",
                            "NotInitialized",
                        ));
                    }
                }
                // Id 0 with a record resolves the page from the record's table LookupPageId; a
                // table without one refuses like any unknown id (corpus
                // pages/run-modal/TestPageRunModalLookupPageId_Tests.al).
                let mut id = n as u32;
                if n == 0 {
                    if let Some(r) = &record {
                        let table = r.borrow().table as usize;
                        let lookup = engine.program.objects[table]
                            .properties
                            .iter()
                            .find(|(k, _)| k.eq_ignore_ascii_case("LookupPageId"))
                            .map(|(_, v)| v.trim().trim_matches('"').to_string());
                        if let Some(l) = lookup {
                            let found = engine.program.objects.heads().iter().find(|o| {
                                o.is_kind(al_ir::ObjKind::Page)
                                    && (o.name.eq_ignore_ascii_case(&l) || o.id.is_some_and(|i| i.to_string() == l))
                            });
                            if let Some(i) = found.and_then(|o| o.id) {
                                id = i;
                            } else if names_uncompiled_page(&engine, &l) {
                                return unsupported(format!(
                                    "the LookupPageId of table {} is page {l}, which belongs to a dependency app and was not compiled (no code the workspace reaches uses it)",
                                    engine.program.objects[table].name
                                ));
                            }
                        }
                    }
                }
                let Some(object) = engine
                    .program
                    .objects
                    .heads()
                    .iter()
                    .position(|o| o.is_kind(al_ir::ObjKind::Page) && o.id == Some(id))
                else {
                    // A page of a dependency app that nothing reached is not compiled (ADR-013); BC has it.
                    if engine
                        .program
                        .objects
                        .heads()
                        .iter()
                        .any(|o| o.is_shell_of(al_ir::ObjKind::Page) && o.id == Some(id))
                    {
                        return unsupported(format!(
                            "page {n} belongs to a dependency app and was not compiled (no code the workspace reaches uses it)"
                        ));
                    }
                    return Err(self.missing_object("Page", i64::from(n)));
                };
                let st = Rc::new(RefCell::new(PageState::closed(object as u32)));
                // The static RunModal reports the lookup values, with or without a record: OK reads
                // LookupOK, Cancel LookupCancel (corpus ui-handlers/TestPageModalHandlerStatic_Tests.al:48-72
                // for the record overload;
                // pages/run-modal/TestPageModalResultsAndTriggersContracts.al:206 RunModal_Dialog_StaticVersusVariable_ReportedAction
                // and :240 RunModal_Card_StaticWithAndWithoutRecord_VersusVariable_ReportedAction for the others). A page variable reports OK and Cancel.
                // With a record, whether the caller uses the return value decides: used (assigned, passed on, compared),
                // the list opens in lookup mode with no new-row line; called as a statement it is a plain modal page
                // (pages/triggers/BeRunModalOnLookup.al:18-27, :616 Direct_NoLookupPageId_KeyFilter_WalkOnly,
                // :916 Shape_ReturnNotUsed_WalkOnly, :949 Shape_ReturnInIf_WalkOnly, :1100-1126 Shape_RunModalZero_*).
                // Without a record the same: a statement leaves a card without its Cancel action
                // (ui-handlers/P132TwoModalHandlers.al:466 PW_ProbeB_Statement, :508 PW_ProbeB_InIf).
                if method == PageOp::PageRunModalStatic {
                    st.borrow_mut().static_run_modal(used, record.is_some(), field, n == 0);
                }
                // A record of another table than the page's is refused before anything opens, and so is a field number the table
                // lacks (a negative one or 0 goes on; pages/parts/P106RunErrors.al: Err_WrongTableRecord,
                // Err_FieldNoNotExisting).
                if let (Some(r), true) = (&record, self.has_source(&st)) {
                    self.page_instance(&st)?;
                    if self.page_rec(&st).borrow().table != r.borrow().table {
                        return Err(error_code("The table IDs do not match.", "TableIdMismatch"));
                    }
                    if let Some(Value::Integer(no)) = args.get(2) {
                        let table = self.table(r)?;
                        if *no > 0 && !table.fields.iter().any(|f| i64::from(f.id) == i64::from(*no)) {
                            return Err(error_code(
                                format!(
                                    "The supplied field number '{no}' cannot be found in the '{}' table.",
                                    self.caption(&table)
                                ),
                                "FieldNotFound",
                            ));
                        }
                    }
                }
                // `--dev-endpoint-ui` (al-runner issue 3697): the measured shape gets the dev endpoint's answer.
                if let Some(v) =
                    self.dev_endpoint_page_run(&st, method == PageOp::PageRunModalStatic, record.is_some() || field)
                {
                    return Ok(v);
                }
                if method != PageOp::PageRunModalStatic {
                    self.note_page_run(&st, PageRunCall::of_static(by_id, record_arg, field, n, id));
                }
                self.run_page_ui_with(&st, record, method == PageOp::PageRunModalStatic, field)
            }
            Some(Value::TestPage(st)) => {
                self.fresh_page_for_held_reopen(method, st);
                let st = st_of(st).clone();
                Self::note_page_code_call(&st, method);
                self.refuse_held_part_page_method(&st, method)?;
                self.pending_add_in_reread(&st, method.method())?;
                if method.method() != PageMethod::New {
                    st.borrow_mut().saved_by_update = false;
                }
                let arg_false = matches!(args.first(), Some(Value::Boolean(false)));
                self.lookup_refresh_deferred_call(method, &st, None)?;
                let moved = self.code_close_client_call(method, &st, |s| {
                    s.marked_only_unshown_call(method, &st, |s| {
                        s.shown_client_call(method, &st, None, arg_false, |s| s.test_page_op_held(method, &st, args))
                    })
                });
                self.test_call_ended(&st, &moved);
                let moved = moved?;
                self.lookup_move_refresh_after(&st, &moved)?;
                Ok(moved)
            }
            Some(Value::TestControl(c)) => {
                let c = control_of(c).clone();
                let page = c.page.clone();
                self.pending_add_in_reread(&page, method.method())?;
                page.borrow_mut().saved_by_update = false;
                self.lookup_refresh_deferred_call(method, &page, Some(&c.target))?;
                let result = self.code_close_client_call(method, &page, |s| {
                    s.marked_only_unshown_call(method, &page, |s| {
                        s.shown_client_call(method, &c.page, Some(&c.target), false, |s| {
                            s.test_control_op_held(method, &c, args)
                        })
                    })
                });
                self.test_call_ended(&page, &result);
                result
            }
            other => unsupported(format!("{method} on {}", other.describe())),
        }
    }

    /// A call that reads a field's value and changes nothing.
    pub(super) fn field_read(name: PageMethod) -> bool {
        matches!(
            name,
            PageMethod::Value
                | PageMethod::AsBoolean
                | PageMethod::AsDate
                | PageMethod::AsDateTime
                | PageMethod::AsDecimal
                | PageMethod::AsInteger
                | PageMethod::AsTime
                | PageMethod::GetField
        )
    }

    /// Only Close, OK and Cancel are pinned on a request page after New() (`request_new_blank_row`), and only Close and a part's
    /// Visible() and Editable() after a swallowed insert of a new host (`refuse_after_swallowed_part_insert`).
    fn refuse_after_request_new(&self, st: &St, name: PageMethod) -> Result<PageMethod, Signal> {
        self.refuse_after_swallowed_part_insert(st, Some(name))?;
        if st.borrow().request_new_blank && !matches!(name, PageMethod::Close | PageMethod::OK | PageMethod::Cancel) {
            return unsupported(format!(
                "TestPage.{name} on a request page after New() (only Close, OK and Cancel are pinned)"
            ));
        }
        self.refuse_after_list_edit_back(st, name)?;
        Ok(name)
    }

    pub(super) fn test_page_op(&mut self, method: PageOp, st: &St, args: &mut [Value]) -> Result<Value, Signal> {
        let name = self.refuse_after_request_new(st, method.method())?;
        if method.recv() == PageRecv::TestPart && st.borrow().link_validate_failed {
            // QjPartPropagation.al:3766 QT_ValPartSplit_Empty_Last_KeyValidateErrors `last = ok No`, `log-last` empty.
            if name == PageMethod::Last {
                return Ok(Value::Boolean(false));
            }
            return unsupported(format!(
                "TestPart.{name} on a part whose link value failed its validation as the host opened (not pinned)"
            ));
        }
        if name != PageMethod::Close || st.borrow().unpinned_close {
            self.refuse_unpinned_state(st)?;
        }
        if method.recv() == PageRecv::TestPart {
            self.first_part_access(st, name, false)?;
        } else if st.borrow().new_host_part_rows
            && st.borrow().host.is_none()
            && !matches!(
                name,
                PageMethod::Close | PageMethod::New | PageMethod::OK | PageMethod::Next | PageMethod::Last
            )
        {
            // A new host whose part shows rows of the blank key: Close and New() are pinned (PartLineBeforeHostKey.al:1944
            // QH_BlankRows_OpenNew_CloseOnly, :1909 QH_BlankRows_HostNew_ThenClose), and Next and Last, which answer No, run no
            // trigger and store nothing (pages/activate-field-blank-host-rows/Y17ActivateNewHost.al:573 QH_BlankRows_HostNext, :602
            // QH_BlankRows_HostLast); nothing else.
            return unsupported(format!(
                "TestPage.{name} on a new, unsaved host whose part shows rows of the blank key (not pinned)"
            ));
        }
        // A card whose OnNewRecord inserted its new row: only Close of the unchanged row is pinned (corpus
        // testpage/CfPageLeads.al:241-289 Card_OnNewRecordInserts_OpenNewSetClose).
        if st.borrow().new_row_inserted_by_trigger
            && st.borrow().open
            && (name != PageMethod::Close || st.borrow().dirty)
        {
            return unsupported(format!(
                "TestPage.{name} on a card whose OnNewRecord inserted the new row itself (not pinned)"
            ));
        }
        // The row the host stands on before a First, Last, GoToKey or GoToRecord, to tell a move from a stay (see below).
        let before = if matches!(
            name,
            PageMethod::First | PageMethod::Last | PageMethod::GoToKey | PageMethod::GoToRecord
        ) && st.borrow().host.is_none()
            && !st.borrow().parts.is_empty()
            && st.borrow().open
            && self.has_source(st)
        {
            Some(self.row_key(&self.page_rec(st))?)
        } else {
            None
        };
        let waiting = if st.borrow().host.is_none() && !st.borrow().parts.is_empty() {
            self.parts_waiting_refresh(st)
        } else {
            Vec::new()
        };
        // After a GoToKey that moved a list without OnAfterGetCurrRecord every action reads the row twice before its trigger
        // (`actions.rs`), until a move clears it (pages/list-order/QgGoToKeyNextCall.al, writer QG).
        // A move after it reads only what the move reads: Next (AiListOrderTests.al:1012-1021 Paging_GoToKeyThenWalk_Nav, every
        // `at` log empty), GoToKey, Last and First (testpage/TestPageRuntimeContracts.al:342-357 on a list with OnAfterGetRecord
        // alone). Previous reads nothing and the next action reads the new row twice (QgGoToKeyNextCall.al:254
        // Plain_GoToKey_ThenPrevious `previous_log` empty, `action_log` `R:R1002;R:R1002;A:act=R1002`, and Nav_GoToKey_ThenPrevious);
        // what Close reads right after it is not pinned (refused). OK of a lookup-mode page reads the row once (`controls.rs`).
        // Close after a GoToKey that ran no trigger reads the row once, also after field value reads and after an action
        // (QgGoToKeyNextCall.al:223 Plain_GoToKey_ThenValueRead, :277 Plain_GoToKey_ThenClose, :236 Plain_GoToKey_ThenSetValue_ThenAction,
        // and the Nav_ tests: `close_log` `R:R1003`); after a GoToKey that read rows it reads nothing (handlers/
        // TestPageSystemPartListWindowContracts.al:219 List_Window_SmallTable: the next open's log holds no read of the Close). After a
        // SetValue between them (no action since) Close reads nothing: not pinned, kept for baseline case 68164
        // TestPage_SetValue_OfTheValueAlreadyHeld_StillRunsOnValidate (it does not observe the Close).
        // After an action stored one row through another variable while a list opened for edit stood on its new-row line over an
        // empty view, the list is back on that line (`actions.rs`, empty_page_after_action). First then reads the row twice and
        // runs OnAfterGetCurrRecord with xRec on it (testpage/PnGShowRow.al:223
        // QG_EmptyList_InsertViaOtherVariable_StepLog `first_log` `List.AGR(NEW2,Created);List.AGR(NEW2,Created);
        // List.AGCR(NEW2,NEW2)`, :56 EmptyListInsertViaOtherVariable); Close reads nothing (`close_log` empty). Other calls are not
        // pinned.
        if method.recv() != PageRecv::Page && std::mem::take(&mut st.borrow_mut().empty_list_row_behind) {
            match name {
                PageMethod::First
                    if st.borrow().open && st.borrow().cursor == Cursor::NewLine && !st.borrow().dirty =>
                {
                    return self.first_after_empty_list_action(st);
                }
                PageMethod::Close => {}
                _ => {
                    return unsupported(format!(
                        "TestPage.{} after an action stored a row behind the new-row line of an empty list (not pinned)",
                        name.name()
                    ));
                }
            }
        }
        if method.recv() != PageRecv::Page
            && matches!(
                name,
                PageMethod::Next
                    | PageMethod::Previous
                    | PageMethod::Last
                    | PageMethod::GoToKey
                    | PageMethod::GoToRecord
                    | PageMethod::FindFirstField
                    | PageMethod::FindNextField
                    | PageMethod::FindPreviousField
            )
            && self.tree_over_variable(st)?
        {
            return unsupported(format!(
                "TestPage.{}() on a tree whose IndentationColumn is not a field of the source table (which rows it shows is not modelled)",
                name.name()
            ));
        }
        // See `PageState::action_landed_on_new_row`: a move then is not pinned; Close and the rest leave the mark behind.
        if method.recv() != PageRecv::Page
            && std::mem::take(&mut st.borrow_mut().action_landed_on_new_row)
            && matches!(
                name,
                PageMethod::First | PageMethod::Last | PageMethod::Next | PageMethod::Previous
            )
        {
            return unsupported(format!(
                "TestPage.{}() after an action put a list on a row the client did not hold (what it reads is not pinned)",
                name.name()
            ));
        }
        // See `PageState::action_refresh_unheld`.
        if method.recv() != PageRecv::Page
            && std::mem::take(&mut st.borrow_mut().action_refresh_unheld)
            && matches!(
                name,
                PageMethod::First
                    | PageMethod::Last
                    | PageMethod::Next
                    | PageMethod::Previous
                    | PageMethod::GoToKey
                    | PageMethod::GoToRecord
            )
        {
            return unsupported(format!(
                "TestPage.{}() after an action changed the view of a list with OnFindRecord and its refresh read more rows than the client holds (not pinned)",
                name.name()
            ));
        }
        // See `PageState::validate_refresh_unheld`.
        if method.recv() != PageRecv::Page
            && std::mem::take(&mut st.borrow_mut().validate_refresh_unheld)
            && matches!(
                name,
                PageMethod::First
                    | PageMethod::Last
                    | PageMethod::Next
                    | PageMethod::Previous
                    | PageMethod::GoToKey
                    | PageMethod::GoToRecord
            )
        {
            return unsupported(format!(
                "TestPage.{}() after a refresh from a control's OnValidate read more rows than the client holds (not pinned)",
                name.name()
            ));
        }
        // The page's own code (CurrPage in the action's trigger) is not a call of the test client: the mark stays as it is.
        let client = method.recv() != PageRecv::Page;
        let read_after_goto = client && std::mem::take(&mut st.borrow_mut().read_after_goto);
        let doubt = client && std::mem::take(&mut st.borrow_mut().read_after_goto_doubt);
        let quiet = client && std::mem::take(&mut st.borrow_mut().read_after_goto_quiet);
        let set = client && std::mem::take(&mut st.borrow_mut().read_after_goto_set);
        let prev = client && std::mem::take(&mut st.borrow_mut().read_after_goto_prev);
        if read_after_goto {
            let keep = |st: &St, doubt: bool, prev: bool| {
                let mut s = st.borrow_mut();
                s.read_after_goto = true;
                s.read_after_goto_doubt = doubt;
                s.read_after_goto_quiet = quiet;
                s.read_after_goto_set = set;
                s.read_after_goto_prev = prev;
            };
            match name {
                PageMethod::Invoke => keep(st, doubt, prev),
                // QgGoToKeyNextCall.al:406 Plain_LookupHandler_GoToKey_ThenOK, :426 _ValueRead_ThenOK (lookup-mode modal only).
                PageMethod::OK if st.borrow().lookup => keep(st, doubt, prev),
                PageMethod::Previous => keep(st, doubt, true),
                PageMethod::Close if prev => {
                    return unsupported(
                        "TestPage.Close right after a Previous that followed a GoToKey on a list without OnAfterGetCurrRecord (what it reads is not pinned)",
                    );
                }
                PageMethod::Close if quiet && !set && method.recv() != PageRecv::Page && st.borrow().open => {
                    let rec = self.page_rec(st);
                    if self.stored_row(&rec)?.is_some() {
                        self.populate_current(st)?;
                    }
                }
                PageMethod::Next | PageMethod::GoToKey | PageMethod::Last | PageMethod::First | PageMethod::Close => {}
                // A read of the page itself is a field call as in `controls.rs` (kept for baseline case 68164
                // TestPage_Control_Texts_GuidDurationAndBlankNumbers, IsExpanded): an action after it is refused.
                PageMethod::IsExpanded
                | PageMethod::Caption
                | PageMethod::Editable
                | PageMethod::ValidationErrorCount
                | PageMethod::GetValidationError => keep(st, true, prev),
                _ => {
                    return unsupported(format!(
                        "TestPage.{} right after a GoToKey moved a list without OnAfterGetCurrRecord (whether the row is read again is not pinned)",
                        name.name()
                    ));
                }
            }
        }
        // First reads nothing only right after the refresh that settled the list (a field read between them is pinned:
        // AiListOrderTests.al:163-166, :336-338); any other call ends that.
        if name != PageMethod::First && !Self::field_read(name) {
            st.borrow_mut().settled_first_row = false;
        }
        let triggers_before = st.borrow().triggers_run;
        let r = match self.test_page_op_inner(method, st, args) {
            // `CurrPage` and a `Page` variable are the page's own code, not a TestPage the test closed: BC does not raise
            // "The TestPage is not open." for them. What a method of a page that is not open (any more) does is not pinned, so
            // the call is refused instead of failing with the TestPage error (Posted Sales Document Lines.CopyLineToDoc reads
            // `CurrPage.PostedShpts.PAGE` after its RunModal returned).
            // `Run` and `RunModal` only pass on what their handler raised (ui-handlers/TestPageModalSelfClose_Tests.al:104).
            Err(Signal::Error(e))
                if e.code == "TestPageNotOpened"
                    && method.recv() == PageRecv::Page
                    && !matches!(name, PageMethod::Run | PageMethod::RunModal) =>
            {
                if matches!(name, PageMethod::Update | PageMethod::SaveRecord) && self.closed_after_modal_ok(st) {
                    return Ok(Value::Null);
                }
                return unsupported(format!(
                    "{} called by the code of a page that is not open (what it does then is not pinned)",
                    method.method()
                ));
            }
            r => r?,
        };
        // The client runs the page's background tasks before the call returns (corpus
        // testpage/TestPageBgTask_Tests.al:EnqueueBackgroundTask_CompletesBeforeOpenViewReturns). The tasks an
        // `EnqueueBackgroundTask` call queued run once it wrote its task id back (`Session::bg_drain_after_call`).
        if name != PageMethod::EnqueueBackgroundTask {
            self.bg_drain(st)?;
        }
        // A move of the host re-points its parts; a move of a part re-points the parts that take
        // their link from it (corpus testpage/parts/TestPartProviderMove.al).
        let moved = match name {
            PageMethod::First
            | PageMethod::Last
            | PageMethod::Next
            | PageMethod::Previous
            | PageMethod::GoToKey
            | PageMethod::GoToRecord
            | PageMethod::FindFirstField
            | PageMethod::FindNextField
            | PageMethod::FindPreviousField => !matches!(r, Value::Boolean(false)),
            PageMethod::New => true,
            // CurrPage.Update leaves a host's parts as they are: they are not opened again (corpus
            // testpage/TestPageNoSrcPart_Tests.al:DirectOpen_NoSrcHost_HostWriteIsVisibleThroughTheSameInstance);
            // whether they read their rows again is docs/probes-needed.md.
            _ => false,
        };
        // pages/list-order/AiListOrderTests.al:38-39 (Keep_Nav), :232-233 (Keep_Plain), :300-301, :334-335: after a
        // GoToKey that moved a list without OnAfterGetCurrRecord nothing is read (`goto_log` empty), and the next action
        // reads the row twice before its trigger.
        if name == PageMethod::GoToKey
            && moved
            && st.borrow().open
            && self.is_list(st)
            && self.declares(st, "onaftergetrecord")
            && !self.declares(st, "onaftergetcurrrecord")
        {
            let quiet = st.borrow().triggers_run == triggers_before;
            let mut s = st.borrow_mut();
            s.read_after_goto = true;
            s.read_after_goto_quiet = quiet;
        }
        if moved && st.borrow().open {
            if st.borrow().host.is_some() {
                self.sync_dependents_then_host_follows(st, name)?;
            } else {
                // First and Last that move the host, Next and Previous: only the linked parts follow
                // (`sync_parts_after_move`, pages/parts/PartReadsOpen.al:269-283, :336-379). What a First or
                // Last that stays on the row does, and what GoToKey, GoToRecord, the Find*Field methods and New do
                // to a part without a link, is not pinned: they keep re-reading every part as before.
                let stayed = match &before {
                    Some(key) => self.has_source(st) && Self::same_key(&self.row_key(&self.page_rec(st))?, key),
                    None => false,
                };
                let skipped_unlinked = match name {
                    PageMethod::Next | PageMethod::Previous => {
                        if name == PageMethod::Next {
                            self.restore_held_xrec_of_empty_parts(st);
                        }
                        self.during_host_move(HostMove::moved(name), |s| s.sync_parts_after_move_fn(st, name))?;
                        true
                    }
                    PageMethod::First | PageMethod::Last if before.is_some() && !stayed => {
                        self.sync_parts_after_move(st, name == PageMethod::First)?;
                        true
                    }
                    PageMethod::GoToKey | PageMethod::GoToRecord if stayed && !st.borrow().walk_synced_parts => {
                        self.during_host_move(HostMove::stayed(name), |s| s.sync_parts_stayed(st))?;
                        false
                    }
                    PageMethod::New => {
                        // The parts a host's New() links afresh run no OnNewRecord (see `land_on_part_open_line`).
                        let before = std::mem::replace(&mut self.host_new_running, true);
                        let r = self.sync_parts(st);
                        self.host_new_running = before;
                        r?;
                        false
                    }
                    _ => {
                        self.during_host_move(HostMove::moved(name), |s| {
                            s.sync_parts_after_other_move(st, name, before.is_some() && !stayed)
                        })?;
                        false
                    }
                };
                self.refresh_parts_unfollow(st, &waiting, name == PageMethod::Next, skipped_unlinked)?;
            }
        }
        Ok(r)
    }
    /// The save of `CurrPage.Update(true)`, `CurrPage.Update()` and `CurrPage.SaveRecord()` on an open page.
    fn save_from_update(&mut self, method: PageOp, st: &St) -> Result<Value, Signal> {
        if method == PageOp::PageSaveRecord {
            st.borrow_mut().save_record_called = true;
        }
        // CurrPage.SaveRecord() in the OnValidate of a part's control changes neither Rec nor xRec of the stored row; a new
        // row gets its inserted key in Rec while xRec stays the blank new row
        // (pages/parts/QjPartPropagation.al:733 QJ_SubformSaveRecord_StoredRow `S.V.after(10000 r=5 x=1 ...)`,
        // :764 QJ_SubformSaveRecord_NewRow `S.V.after(20000 r=5 x=0 x.ln=0 delta=5)`). Pinned for a ListPart only.
        // On the pages `pages/validate_xrec.rs` pins, any save inside a control's validation keeps xRec until the step ends.
        let step_save_xrec = self.validate_step_save_xrec(st)?;
        let keep_xrec = step_save_xrec.is_some()
            || (method == PageOp::PageSaveRecord
                && {
                    let s = st.borrow();
                    s.validating_control && s.host.is_some()
                }
                && self.is_list(st));
        if keep_xrec {
            let xrec = self.page_xrec_fields(st);
            if let Some(seen) = step_save_xrec {
                self.set_xrec_fields(st, seen);
            }
            self.save_record_now(st)?;
            let stored = self.page_rec(st).borrow().fields.clone();
            self.set_xrec_fields(st, xrec);
            self.validate_step_saved(st, &stored);
            st.borrow_mut().xrec_after_validate = Some(stored);
        } else {
            self.save_record_now(st)?;
        }
        // What `CurrPage.Update(true)` saved (al-vm pages/leave_new_line.rs, testpage/NewRowThenMove.al:73).
        st.borrow_mut().saved_by_update = method != PageOp::PageSaveRecord;
        Ok(Value::Null)
    }

    pub(super) fn test_page_op_inner(&mut self, method: PageOp, st: &St, args: &mut [Value]) -> Result<Value, Signal> {
        if method.recv() == PageRecv::TestPage
            && self.is_request_page(st)
            && !self.has_source(st)
            && let Some(r) = self.request_page_without_source_op(st, method.method(), args)
        {
            return r;
        }
        match method.method() {
            // `TestPart.Visible` / `.Enabled` read the part control of the host: a part that is
            // reachable at all is visible (corpus testpage/parts/TestTestPart.al:TestPart_Visible_*).
            // A part whose Visible expression came out false answers No (testpage/PnGPartTriggers.al:
            // HiddenPartExpression).
            PageMethod::Visible if method.recv() == PageRecv::TestPart => {
                self.require_open(st)?;
                let host = st.borrow().host.clone();
                match host.and_then(|(h, i)| h.upgrade().map(|h| (h, i))) {
                    Some((h, i)) => Ok(Value::Boolean(self.control_visible(&h, i)?)),
                    None => Ok(Value::Boolean(true)),
                }
            }
            PageMethod::Enabled if method.recv() == PageRecv::TestPart => {
                self.require_open(st)?;
                let host = st.borrow().host.clone();
                match host.and_then(|(h, i)| h.upgrade().map(|h| (h, i))) {
                    Some((h, i)) => Ok(Value::Boolean(self.control_enabled(&h, i)?)),
                    None => unsupported("TestPart.Enabled on a page that is not a part"),
                }
            }
            // The id is a compiler-generated control id; the corpus pins only that a table field
            // number is not one (testpage/parts/TestTestPart.al:TestPart_GetField_TakesAControlIdNotATableFieldNo).
            PageMethod::Filter => {
                self.require_open(st)?;
                Ok(self.filter_handle(st))
            }
            PageMethod::GetField => {
                self.require_open(st)?;
                match args.first() {
                    // Code: testpage/TestPageRuntimeContracts.al:274-275 (page and part).
                    Some(Value::Integer(n)) => Err(error_code(
                        format!("The field with ID = {n} is not found on the page."),
                        "TestFieldNotFound",
                    )),
                    _ => unsupported("TestPart.GetField without an id"),
                }
            }
            PageMethod::OpenEdit | PageMethod::OpenView | PageMethod::OpenNew => {
                st.borrow_mut().caught_by_trap = false;
                let mode = match method.method() {
                    PageMethod::OpenEdit => PageMode::Edit,
                    PageMethod::OpenView => PageMode::View,
                    _ => PageMode::New,
                };
                self.page_open(st, mode)
            }
            PageMethod::Close if method.recv() != PageRecv::Page => self.page_close(st),
            m @ (PageMethod::First | PageMethod::Last | PageMethod::Next | PageMethod::Previous) => {
                self.require_open(st)?;
                self.page_move(st, m)
            }
            PageMethod::GoToKey => {
                self.require_open(st)?;
                self.page_go_to_key_captions(st, args)
            }
            m @ (PageMethod::FindFirstField | PageMethod::FindNextField | PageMethod::FindPreviousField) => {
                self.require_open(st)?;
                self.page_find_field(st, m, args)
            }
            // A page that is not a tree has nothing to expand (docs only; the corpus calls them,
            // TestPageAdvanced.al:104, :283, asserting nothing about the answer).
            // On a tree (a repeater with ShowAsTree) Expand shows or hides the rows below the current one (`tree_expand`).
            PageMethod::Expand => {
                self.require_open(st)?;
                let engine = self.engine();
                let tree = page_of(&engine, st).controls.iter().any(|c| {
                    c.kind == ir::ControlKind::Repeater
                        && matches!(c.prop("showastree"), Some(ir::PropVal::Lit(v)) if unquote(v).eq_ignore_ascii_case("true"))
                });
                if tree {
                    let on = match args.first() {
                        Some(v) => ops::truthy(v)?,
                        None => return unsupported("Expand without an argument on a tree page (not pinned)"),
                    };
                    self.tree_expand(st, on)?;
                }
                Ok(Value::Null)
            }
            PageMethod::IsExpanded => {
                self.require_open(st)?;
                // On a tree, whether the row was expanded with Expand(true) (testpage/AhPageTests.al:953-961).
                if self.has_source(st) && self.tree_indentation(st).is_some() && st.borrow().cursor == Cursor::Row {
                    let key = self.row_key(&self.page_rec(st))?;
                    return Ok(Value::Boolean(
                        st.borrow().expanded.iter().any(|k| Self::same_key(k, &key)),
                    ));
                }
                Ok(Value::Boolean(false))
            }
            PageMethod::SaveAsXml => self.request_save_as_xml(st, args),
            PageMethod::SaveAsPdf => self.request_save_as_render(st, args, "Pdf"),
            PageMethod::SaveAsExcel => self.request_save_as_render(st, args, "Excel"),
            m @ (PageMethod::OK
            | PageMethod::Cancel
            | PageMethod::Yes
            | PageMethod::No
            | PageMethod::Preview
            | PageMethod::Print) => {
                self.require_open(st)?;
                if matches!(m, PageMethod::Preview | PageMethod::Print) {
                    self.request_preview_print_pinned(st, m.name())?;
                }
                if m == PageMethod::OK && self.lookup_ok_on_no_row(st)? {
                    // A list page variable in lookup mode whose view shows no row has no OK action
                    // (testpage/P129PageMisc.al:726 TV_GetRecord_AfterLookupOk_FilteredToNoRow, :772
                    // TV_GetRecord_AfterSetRecordAndLookupOk_FilteredToNoRow).
                    return Err(error_code(
                        "The built-in action = OK is not found on the page.".to_string(),
                        "TestActionNotFound",
                    ));
                }
                if !self.builtin_offered(st, m.name()) {
                    // Code TestActionNotFound (pages/parts/P106RunInstance.al:P106_Inst_RunModal_List `af100`).
                    return Err(error_code(
                        format!("The built-in action = {m} is not found on the page."),
                        "TestActionNotFound",
                    ));
                }
                Ok(test_control(st.clone(), ControlTarget::BuiltIn(m.to_string())))
            }
            m @ (PageMethod::Edit | PageMethod::View) => {
                self.require_open(st)?;
                Ok(test_control(st.clone(), ControlTarget::BuiltIn(m.to_string())))
            }
            // `ObjectId(UseNames)`: `Page <id>`, or with names (the default) `Page <Caption>`, the object name when
            // the page has no Caption. Taken from the probed report form (recordref-fieldref/PWRefusedBuiltins.al:78-107,
            // BC 27.5; the page form is pinned by pages/PRPageObjectId.al). It reads the object, so
            // a page that has not run answers too.
            PageMethod::ObjectId if method.recv() == PageRecv::Page => {
                let names = match args.first() {
                    None => true,
                    Some(Value::Boolean(b)) => *b,
                    Some(other) => return unsupported(format!("Page.ObjectId with {}", other.describe())),
                };
                let engine = self.engine();
                let o = &engine.program.objects[st.borrow().object as usize];
                if !names {
                    return Ok(Value::Text(format!("Page {}", o.id.unwrap_or(0)).into()));
                }
                let caption = match o.properties.iter().find(|(k, _)| k.eq_ignore_ascii_case("Caption")) {
                    Some((_, v)) => match crate::recmeta::string_literal(v) {
                        Some(c) => c,
                        None => return unsupported("Page.ObjectId of a page whose Caption is not a literal"),
                    },
                    None => o.name.clone(),
                };
                Ok(Value::Text(format!("Page {caption}").into()))
            }
            // `CurrPage.Caption := x` sets the page's caption for as long as it stays open; without an
            // argument it answers the caption in force (testpage/TestPageCaption_Page.al:44).
            PageMethod::Caption if method.recv() == PageRecv::Page => {
                // The caption read on a Page variable before the page runs is the page's own (Edit Dimension Set Entries.SetFormCaption
                // reads `CurrPage.Caption` before Dimension Management runs the page); an assignment still needs an open page
                //.
                // Also after a run: P187_SecondRun_Reassigned (CaptionAssignedBeforeRun.al) assigns between two runs.
                if !args.is_empty() && !st.borrow().open {
                    if let Some(t) = args.first().and_then(al_builtins::native::text_of) {
                        st.borrow_mut().caption = Some(t.to_string());
                        st.borrow_mut().caption_assigned_closed = true;
                        return Ok(Value::Null);
                    }
                }
                if args.is_empty() && !st.borrow().open {
                    return Ok(Value::Text(self.page_caption(st).into()));
                }
                self.require_open(st)?;
                match args.first() {
                    Some(v) => match al_builtins::native::text_of(v) {
                        Some(t) => {
                            st.borrow_mut().caption = Some(t.to_string());
                            st.borrow_mut().caption_assigned_closed = false;
                            Ok(Value::Null)
                        }
                        None => unsupported(format!("CurrPage.Caption with {}", v.describe())),
                    },
                    None => Ok(Value::Text(self.page_caption(st).into())),
                }
            }
            // `ObjectId(UseNames)` (CurrPage and a Page variable alike, open or not): `Page <id>`, or with names, the
            // default, `Page <Caption>` and the object name when the page has no Caption (pages/PRPageObjectId.al).
            PageMethod::ObjectId if method.recv() == PageRecv::Page => {
                let names = match args.first() {
                    None => true,
                    Some(v) => ops::truthy(v)?,
                };
                let engine = self.engine();
                let o = &engine.program.objects[st.borrow().object as usize];
                if !names {
                    return Ok(Value::Text(format!("Page {}", o.id.unwrap_or(0)).into()));
                }
                let caption = o
                    .properties
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case("Caption"))
                    .map(|(_, v)| unquote(v))
                    .unwrap_or_else(|| o.name.clone());
                Ok(Value::Text(format!("Page {caption}").into()))
            }
            PageMethod::Caption => {
                self.require_open(st)?;
                Ok(Value::Text(self.test_page_caption(st)?.into()))
            }
            // `CurrPage.PromptMode := PromptMode::Prompt` states what a PromptDialog already starts in:
            // the pages of ui-handlers/TbaPromptDeclared.Page.al and TbaPromptBare.Page.al do it in OnOpenPage
            // and the tests around them (TestPageBuiltInActions_Tests.al (a)-(j)) see an ordinary dialog.
            // The other modes are not pinned.
            PageMethod::PromptMode => match args.first() {
                Some(Value::Option(n, members)) if members.get(*n as usize).is_some_and(|m| m == "Prompt") => {
                    Ok(Value::Null)
                }
                Some(Value::Enum(..)) | Some(Value::Option(..)) => {
                    unsupported("CurrPage.PromptMode other than Prompt (not pinned)")
                }
                other => unsupported(format!("CurrPage.PromptMode with {}", other.describe())),
            },
            PageMethod::Editable if method.recv() == PageRecv::Page => {
                // A Page variable that has not run (Base App sets `Languages.Editable := false` before
                // `RunModal`): no "not open" error. It reads true until set, and what it was set to is
                // what the page opens with, for Run and RunModal alike
                // (pages/run-modal/TestPageVariableEditableContracts.al).
                if !st.borrow().open {
                    return match args.first() {
                        Some(v) => {
                            let on = ops::truthy(v)?;
                            st.borrow_mut().editable_preset = Some(on);
                            Ok(Value::Null)
                        }
                        // What it was set to, else the page's own `Editable` property (corpus
                        // pages/PWPageVariableEditable.al:187-205: a page with `Editable = false`
                        // reads No before it runs).
                        None => {
                            let engine = self.engine();
                            let object = st.borrow().object as usize;
                            let own = !engine.program.objects[object].properties.iter().any(|(k, v)| {
                                k.eq_ignore_ascii_case("Editable") && v.trim().eq_ignore_ascii_case("false")
                            });
                            Ok(Value::Boolean(st.borrow().editable_preset.unwrap_or(own)))
                        }
                    };
                }
                match args.first() {
                    // CurrPage.Editable(false) in OnOpenPage makes the page, page-variable controls
                    // included, read-only; Editable(true) there does not widen a View page. Calls
                    // from later triggers leave TestPage.Editable() alone (corpus
                    // testpage/TestPageControlEditableRoutes.al:536-555, TestPageEditable_Tests.al:150-180;
                    // their effect on Rec-bound controls is docs/probes-needed.md).
                    Some(v) => {
                        let on = ops::truthy(v)?;
                        if st.borrow().opening {
                            if !on {
                                let mut s = st.borrow_mut();
                                if !s.editable_off {
                                    s.editable_off_in_open = true;
                                }
                                s.editable_off = true;
                            }
                        } else if st.borrow().in_action {
                            st.borrow_mut().editable_set_later = Some(on);
                        }
                        Ok(Value::Null)
                    }
                    // After `CurrPage.Editable(false)` in an action, page code in the same action reads No
                    // (pages/bare-page-methods/TcBarePageObjects.al:309-310, TcBarePageMethodsTests.al:359); the next
                    // action reads the page's own value (pages/run-modal/P82CurrPage.al:196-219). What
                    // `Editable(true)` there reads back is not pinned: on an editable page it reads Yes as before, on a read-only page it is refused.
                    None if st.borrow().editable_set_later == Some(false) => Ok(Value::Boolean(false)),
                    None if st.borrow().editable_set_later == Some(true) && !self.page_editable(st) => unsupported(
                        "CurrPage.Editable() read after CurrPage.Editable(true) on a read-only page (not pinned)",
                    ),
                    // Read from page code on an OpenNew page after CurrPage.Editable(false): only TestPage.Editable() is
                    // pinned there (testpage/CdTestPageLeads.al:117).
                    None if self.open_lock(st) == Some(PageMode::New) => unsupported(
                        "CurrPage.Editable() read on a page opened with OpenNew whose OnOpenPage called CurrPage.Editable(false) (not pinned)",
                    ),
                    None => Ok(Value::Boolean(self.page_editable(st))),
                }
            }
            PageMethod::Editable => {
                self.require_open(st)?;
                self.test_page_editable(st).map(Value::Boolean)
            }
            PageMethod::Trap => {
                // The next plain Page.Run of this page type opens into this variable
                // (corpus ui-handlers/TestPageRunHandler_Tests.al:TrapReceivesTheNonModalPageAndItStaysOpen).
                st.borrow_mut().trapping = true;
                // A second Trap() on the same page, from this or another variable, replaces the first.
                let object = st.borrow().object;
                self.pages
                    .traps
                    .retain(|t| !Rc::ptr_eq(t, st) && t.borrow().object != object);
                self.pages.traps.push(st.clone());
                Ok(Value::Null)
            }
            PageMethod::LookupMode => match args.first() {
                Some(v) => {
                    let on = ops::truthy(v)?;
                    st.borrow_mut().set_lookup_mode(on);
                    Ok(Value::Null)
                }
                None => Ok(Value::Boolean(st.borrow().lookup)),
            },
            // `P.SetRecord(Rec)` hands the page variable the record it shows when it runs (corpus
            // testpage/TestPageSetRecord.al).
            PageMethod::SetRecord => match args.first() {
                Some(Value::Record(r)) => {
                    self.page_var_set_record_marked_only(st, r)?;
                    // A call on a Page variable that never ran constructs the page, and its OnInit runs then
                    // (pages/lifecycle/PageVariableLifecycle.al:146 PageVar_MethodCallBeforeRun `afterSetRecord = I`).
                    if !st.borrow().typed_handler && !st.borrow().open {
                        self.construct_page(st)?;
                    }
                    // Inside a Page-typed modal handler the page holds the record already
                    // (ui-handlers/PSModalPageHandlerTyped.al: GetRecord answers the record SetRecord handed).
                    if st.borrow().typed_handler && self.has_source(st) {
                        let rec = self.page_rec(st);
                        rec.borrow_mut().fields = r.borrow().fields.clone();
                        st.borrow_mut().handler_set_record = true;
                        return Ok(Value::Null);
                    }
                    let kept = st.borrow().set_record.clone().filter(|_| st.borrow().set_view);
                    if let Some(view) = kept.filter(|v| v.borrow().table == r.borrow().table) {
                        view.borrow_mut().fields = r.borrow().fields.clone();
                        return Ok(Value::Null);
                    }
                    let copy = Rc::new(RefCell::new(r.borrow().clone()));
                    // The rows of a temporary record stay with it: a page variable given one by SetRecord shows the table's rows
                    // (record/load-fields/P81PageTempSource.al:PageVar_SetRecord_TempRecord: `D1`, `D2`).
                    copy.borrow_mut().temporary = false;
                    copy.borrow_mut().temp_id = None;
                    Self::without_marked_only(&copy);
                    let mut s = st.borrow_mut();
                    s.set_record = Some(copy);
                    s.set_view = false;
                    drop(s);
                    self.seed_unopened_rec(st);
                    Ok(Value::Null)
                }
                other => unsupported(format!("Page.SetRecord with {}", other.describe())),
            },
            // `CurrPage.SetSelectionFilter(var Rec)` on a list the test client stands on a row of: the record is
            // filtered to that row (testpage/TestPageCloseTriggersSelectionCardPageContracts.al:239-250). A
            // record that already has filters, and a page on no row, are not pinned.
            PageMethod::SetSelectionFilter => {
                if let Some(Value::Record(dst)) = args.first() {
                    if self.selection_of_a_closed_marked_only_page(st, dst)? {
                        return Ok(Value::Null);
                    }
                }
                // A Page variable that never ran and has no OnInit is built here for its blank record (the blank-row path below;
                // pages/run-modal/P82PageVar.al:86 BeforeRun_Members `selectionfilter = Yes`). One with OnInit is not pinned.
                if st.borrow().instance.is_none()
                    && self.never_ran_page_var(st)
                    && matches!(args.first(), Some(Value::Record(_)))
                    && !self.declares(st, "oninit")
                {
                    self.construct_page(st)?;
                }
                let (Some(Value::Record(dst)), Some(inst)) = (args.first(), st.borrow().instance.clone()) else {
                    return unsupported("Page.SetSelectionFilter of a page that never ran (not pinned)");
                };
                let object = st.borrow().object as usize;
                let Some(slot) = self.engine.program.objects[object].rec_slot else {
                    return unsupported("Page.SetSelectionFilter of a page without a source table (not pinned)");
                };
                let Value::Record(src) = inst.borrow().globals[slot].clone() else {
                    return unsupported("Page.SetSelectionFilter: the page's Rec is no record");
                };
                if src.borrow().table != dst.borrow().table {
                    return unsupported("Page.SetSelectionFilter into a record of another table (not pinned)");
                }
                // A part on no row whose Rec holds the values its SubPageLink set selects that record: a filter on its key and a
                // count of 0, whether the table has rows of other documents or none (corpus branch al-runtime-rs-staging-r6962,
                // pages/parts/SelectionFilterLinkedPartNoRow.al SetSelectionFilter_LinkedPartOnNoRow_TableHasOtherRows, _TableEmpty
                // `rec=95656/A/0 count=0 filters=Table ID: 95656, No.: A, Line No.: 0`; issue 6964). Marks are not pinned: none is set.
                let linked_no_row = self.linked_part_on_no_row(st, &src, dst);
                let mut empty_modal_worksheet = false;
                if st.borrow().cursor != Cursor::Row && !linked_no_row && !self.closed_on_handler_set_record(st) {
                    // A page on no row because its table is empty selects nothing, and the record keeps no filter
                    // (pages/run-modal/P82CurrPage.al:SetSelectionFilter_EmptyPage: `sel=0 filter=''`). A page on no row
                    // over a table that has rows is not pinned.
                    let table_empty = self.selection_into_empty_table(st, dst);
                    if table_empty {
                        return self.record_op(RecordOp::SetRecFilter, dst, &mut []);
                    }
                    if self.selection_of_a_test_page_filtered_to_nothing(st, &src, dst)? {
                        return Ok(Value::Null);
                    }
                    // A page whose Rec is the blank record (a part that is hidden, or not yet on a row) hands a record without
                    // filters or marks the filter on the blank key, `Id: 0` and a count of 0
                    // (pages/parts/HiddenPartSelectionFilter.al:SelectionFilter_HiddenLiteral_FilterAndCount,
                    // _HiddenByVariable_, _VisiblePart_): that is the blank-row path below.
                    let initial = self.initial_fields(src.borrow().table as usize);
                    let blank_rec = self.is_blank_row(&src);
                    let plain_dst = self.plain_no_row_selection_destination(dst, &initial)?;
                    // Pinned for the page of a part, and for a Page variable that never ran and was handed no record or view: a
                    // ListPart over a temporary Integer (`Number: 0`, count 1 of the Integer table), a List and a Card (`Id: 0`,
                    // count 0) (pages/currpage/UnopenedCurrPage.al:110, :159, :208 Never_Opened_*_TryFunction).
                    let unopened_var = self.never_ran_page_var(st) && {
                        let s = st.borrow();
                        s.set_record.is_none() && !s.set_view
                    };
                    let held = blank_rec && (st.borrow().host.is_some() || unopened_var || self.on_open_new_row(st));
                    empty_modal_worksheet =
                        plain_dst && self.empty_modal_worksheet_selection(st, &src, dst, &initial)?;
                    if empty_modal_worksheet {
                        st.borrow_mut().empty_selection_given = true;
                    }
                    if !(plain_dst && (held || empty_modal_worksheet || self.on_list_new_line_blank_key(st, &src)?)) {
                        return unsupported(
                            "Page.SetSelectionFilter of a page on no row of a table that has rows (not pinned)",
                        );
                    }
                    if Rc::ptr_eq(dst, &src) {
                        st.borrow_mut().selection_on_rec = true;
                    }
                }
                // A record that already has filters ends with the filter of the selected row alone: a filter on another field and
                // on the key field, a range on the key, and the page's own filters copied from Rec are gone; a mark and MarkedOnly stay
                // (testpage/P128SelectionFilter.al:P128_SetSelectionFilter_IntoRecordsWithFilters_SecondRow,
                // _MarkCopyTemporary (without the mark), _CalledTwiceOnOneRecord, _PageWithOwnFilter_CopyOfRec).
                // A record that has marks and MarkedOnly on, with a mark outside the selected row, ends with the selected row marked
                // as well: `Marked: Yes, Code: C` in the filters, MarkedOnly still on and a count of 1
                // (P128_SetSelectionFilter_IntoRecordsWithFilters_MarkCopyTemporary, `marked A only`); marks without MarkedOnly end
                // the same way without `Marked: Yes` (testpage/ApTestPageTests.al:881). MarkedOnly without marks and a
                // mark on the selected row are not pinned.
                let selected_key = {
                    let t = self.table(dst)?;
                    t.primary_key(&src.borrow().fields)
                };
                let marked = !dst.borrow().marks.is_empty();
                let marked_only = dst.borrow().marked_only;
                // The page's record is the blank one (the page stands on no stored row, as in a handler that opened it): the record
                // takes the filter on the blank key and keeps its marks and MarkedOnly as they were, with no mark added
                // (pages/currpage/SelectionFilterNoRow.al:Marks_WithoutMarkedOnly, MarkedOnly_WithoutMarks,
                // Mark_OnSelectedRow, MarkedOnly_MarkOnOtherRow: `Id: 0`, `Marked: Yes, Id: 0`, count 0).
                let blank_row =
                    empty_modal_worksheet || self.is_blank_row(&src) || self.on_list_new_line_blank_key(st, &src)?;
                if !blank_row
                    && ((marked_only && !marked)
                        // A record whose only mark is the selected row (an earlier call on it left that mark) ends as it was
                        // (testpage/P128SelectionFilter.al:P128_SetSelectionFilter_CalledTwiceOnOneRecord,
                        // testpage/P135SelectionFilter.al:
                        // P135_SelFilter_GroupMarkedPresetTwice); the selected row marked among others is not pinned.
                        || (dst.borrow().marks.len() > 1
                            && dst.borrow().marks.iter().any(|m| {
                                m.len() == selected_key.len()
                                    && m.iter().zip(&selected_key).all(|(a, b)| {
                                        al_builtins::ops::compare(a, b).ok() == Some(std::cmp::Ordering::Equal)
                                    })
                            })))
                {
                    return unsupported(
                        "Page.SetSelectionFilter into a record with MarkedOnly and no marks, or with the selected row marked among others (not pinned)",
                    );
                }
                dst.borrow_mut().filters.clear();
                let fields = src.borrow().fields.clone();
                dst.borrow_mut().fields = fields;
                // The primary key's fields alone get the filter, whatever the current key: a record with `SetCurrentKey(Grp, Name)` ends
                // with `Id: 2` only and keeps its key (testpage/P135SelectionFilter.al:
                // P135_SelFilter_FreshRecord_AndOtherFieldFilters `keyed`).
                let t = self.table(dst)?;
                let group = dst.borrow().filter_group;
                let mut d = dst.borrow_mut();
                for &p in &t.keys[0] {
                    let v = d.fields[p].clone();
                    d.filters.push(al_builtins::filter::FieldFilter {
                        lang: al_builtins::filter::filter_language(),
                        field: p,
                        expr: al_builtins::filter::FilterExpr::Range(Some(v.clone()), Some(v)),
                        group,
                    });
                }
                drop(d);
                // The selected row ends as the only marked row: a fresh record gets it marked
                // (testpage/CdTestPageLeads.al:676-708, Item60_SelectionFilter_*, `marked=2,`), and the marks of
                // other rows are gone, with or without MarkedOnly (testpage/ApTestPageTests.al:866-882,
                // Ap15_SetSelectionFilter_IntoMarkedRecord, `rows=C*;after=C`).
                if !blank_row && !linked_no_row {
                    dst.borrow_mut().marks = vec![selected_key];
                }
                Ok(Value::Null)
            }
            // Which record a page variable reports after it ran is not pinned: no corpus test calls GetRecord
            // (docs/probes-needed.md). Refused when reached, like SetSelectionFilter.
            // `P.GetRecord(var Rec)` after the page ran: the row it stood on when it closed, the picked row after
            // OK and the last positioned row after Cancel too, without filters
            // (testpage/TestPageCloseTriggersSelectionCardPageContracts.al:250-285).
            PageMethod::GetRecord => self.page_get_record(st, args),
            PageMethod::SetTableView => match args.first() {
                Some(Value::Record(r)) => self.page_var_set_table_view(st, r),
                other => unsupported(format!("Page.SetTableView with {}", other.describe())),
            },
            PageMethod::EnqueueBackgroundTask => {
                self.require_open(st)?;
                self.bg_enqueue(st, args)
            }
            PageMethod::CancelBackgroundTask => {
                self.require_open(st)?;
                self.bg_cancel(st, args)
            }
            PageMethod::RunPageBackgroundTask => {
                self.require_open(st)?;
                self.bg_run_page_task(st, args)
            }
            PageMethod::Run => self.run_page_var(st, false),
            PageMethod::RunModal => {
                st.borrow_mut().run_static = Some(false);
                self.run_page_var(st, true)
            }
            // `CurrPage.Close()` in a procedure of a Page variable that never ran does nothing and the procedure goes on
            // (pages/currpage/ClosedPageVariableCurrPageClose.al:83-99, Close_NeverRun), and so does
            // `Close()` of a Page variable whose modal page closed: after its handler failed (ClosedPageVariableCurrPageClose.al:55
            // Close_AfterModalClosed) and after it closed with OK (pages/run-modal/P82PageVar.al:138 AfterRunModal_Members).
            // So does `CurrPage.Close()` in OnOpenPage of a Card over no table opened with OpenEdit (`no_source_close_in_open_ignored`).
            PageMethod::Close if method.recv() == PageRecv::Page && self.close_does_nothing(st) => Ok(Value::Null),
            // In a Page-typed PageHandler of Page.Run the handler's Close does nothing: no trigger, and the handler goes on
            // (pages/page-handler-with-record/Y17PageHandlerRecord.al:301 PageTypedHandler_RunWithRecord_Close `handler;closed;`).
            PageMethod::Close
                if method.recv() == PageRecv::Page && st.borrow().typed_run_handler && !st.borrow().open =>
            {
                Ok(Value::Null)
            }
            PageMethod::Close if method.recv() == PageRecv::Page => {
                self.require_open(st)?;
                self.restore_shown_row_on_close(st)?;
                let a = self.close_action(st, "Close");
                let restored = st.borrow().close_restored_row;
                let fields = restored.then(|| self.page_rec(st).borrow().fields.clone());
                let r = self.close_by_page_code(st, &a)?;
                self.note_code_close_veto(st, &a)?;
                // Under Page.RunModal the record variable gets the shown row the call read back, although the page closes with
                // LookupCancel (testpage/PnGRefresh.al:996 QN_CloseQ_Sheet_RunModal_OtherChanged_CloseInAction
                // `run-modal-result = LookupCancel`, `row-after = A,orig A`; :1113 the List page). Pinned for that read back only.
                if let Some(fields) = fields.filter(|_| a == "LookupCancel" && st.borrow().selection.is_none()) {
                    st.borrow_mut().selection = Some(fields);
                }
                Ok(r)
            }
            PageMethod::New => {
                self.require_open(st)?;
                self.save_unsaved_host_first(st)?;
                self.insert_untouched_new_host(st)?;
                self.new_record(st)
            }
            PageMethod::ValidationErrorCount => {
                self.require_open(st)?;
                Ok(Value::Integer(st.borrow().errors.len() as i32))
            }
            PageMethod::GetValidationError => {
                self.require_open(st)?;
                let i = match args.first() {
                    Some(Value::Integer(i)) => *i,
                    // No index: the page's errors joined by CR LF (testpage/F3ValidationAddinFilter.al:GetValidationError_TwoErrors
                    // line 238, two errors; one error and none at :225 and :272). Three or more are not pinned.
                    None => {
                        let s = st.borrow();
                        if s.errors.len() > 2 {
                            return unsupported(
                                "GetValidationError without an index with three or more errors (not pinned)",
                            );
                        }
                        let all: Vec<&str> = s.errors.iter().map(|(_, t)| t.as_str()).collect();
                        return Ok(Value::Text(all.join("\r\n").into()));
                    }
                    _ => return unsupported("GetValidationError with a non-integer index"),
                };
                let s = st.borrow();
                match usize::try_from(i - 1).ok().and_then(|n| s.errors.get(n)) {
                    Some((_, t)) => Ok(Value::Text(t.clone().into())),
                    // Index 0 is pinned (testpage/TestPageRuntimeContracts.al:224-236: text and code);
                    // an index past the last error is assumed to raise the same (docs/probes-needed.md).
                    None => Err(error_code("Index out of bounds.", "IndexOutOfBounds")),
                }
            }
            // A part the page's own code created without opening it (a literal `Visible = false`) has no
            // client to refresh: `CurrPage.Update` in its procedure does nothing, with and without the argument
            // (pages/parts/HiddenPartCurrPageUpdateFalse.al:UpdateFalse_LiteralInvisible_OpenEdit,
            // HiddenPartCurrPageUpdateTrue.al:UpdateTrue_LiteralInvisible_OpenEdit). Base Application's Approval FactBox ends
            // `UpdateApprovalEntriesFromSourceRecord` with it.
            PageMethod::Update if st.borrow().host.is_some() && !st.borrow().open => Ok(Value::Null),
            // A Page variable the client does not show has no client either; called directly or from its own procedure:
            // - never ran: `Update(false)`, `Update(true)` and `SaveRecord()` do nothing and the code goes on
            //   (pages/currpage/ClosedPageVariableCurrPageUpdate.al:83-99 Update_NeverRun;
            //   pages/run-modal/P82PageVar.al:76-81 BeforeRun_Members; pages/currpage/UnopenedCurrPage.al:101-103,
            //   :150-152, :199-201, :227 and :246 after SetRecord and SetTableView);
            // - its modal page closed: `Update(false)` and `SaveRecord()` do nothing (ClosedPageVariableCurrPageUpdate.al:55
            //   Update_AfterModalClosed; P82PageVar.al:136-142 AfterRunModal_Members).
            // - never ran, `Update()` without an argument: does nothing, also on a NavigatePage without a source table and
            //   before a RunModal of the same variable (pages/currpage/Y50UpdateNeverRun.al:241-255 T1, :259 T2,
            //   :278 T3, :317 T5); after a modal run closed with OK, `Update()` and `Update(true)` do nothing (:336-352 T6,
            //   `closed_after_modal_ok` in `test_page_op`).
            PageMethod::Update | PageMethod::SaveRecord
                if method.recv() == PageRecv::Page && self.page_var_not_shown(st) && {
                    let ran = st.borrow().ran;
                    match (method, args.first()) {
                        (PageOp::PageSaveRecord, _) => true,
                        (_, Some(Value::Boolean(false))) => true,
                        (_, Some(Value::Boolean(true)) | None) => !ran,
                        _ => false,
                    }
                } =>
            {
                self.never_run_update(method, st, args)
            }
            PageMethod::Update | PageMethod::SaveRecord => {
                self.require_open(st)?;
                // CurrPage.Update() without an argument saves (the parameter defaults to true).
                let save = (method == PageOp::PageSaveRecord) || !matches!(args.first(), Some(Value::Boolean(false)));
                if !(method == PageOp::PageSaveRecord) {
                    self.refuse_update_in_action(st)?;
                    // The refresh lands after the trigger that asked for it returns (corpus
                    // pages/currpage/TestPageUpdate.al: ValidateBegin;ValidateEnd;HostAGCR).
                    st.borrow_mut().refresh = true;
                    st.borrow_mut().refresh_by_update = true;
                    st.borrow_mut().refresh_update_false = !save;
                    self.note_view_left(st)?;
                }
                if !save {
                    self.validate_step_update_false(st);
                    return Ok(Value::Null);
                }
                self.save_from_update(method, st)
            }
            // `Page.Activate([Refresh])` answers Yes and raises nothing where the test client shows no page:
            // - a Page variable that never ran, with and without Refresh, called directly and from its own procedure
            //   (pages/run-modal/P82PageVar.al:82 BeforeRun_Members; pages/currpage/UnopenedCurrPage.al:104-105,
            //   :153-154, :202-203 Never_Opened_*_TryFunction);
            // - a Page variable whose modal page closed (P82PageVar.al:140 AfterRunModal_Members);
            // - a part its host hid, by a literal `Visible = false` (never opened) or by an expression (opened, not shown), called
            //   from the host's OnAfterGetCurrRecord; no part trigger runs (pages/parts/HiddenPartCurrPageActivate.al:145
            //   Activate_LiteralInvisible_OpenEdit, :174 Activate_ExprInvisible_OpenEdit);
            // - the page of a typed ModalPageHandler (`var P: Page`), with and without Refresh
            //   (metadata/session-system/P94PageModal.al:338 Activate_InsideModalHandler, :355 ..Refresh).
            // A page the test client shows, and a Refresh of an open page whose row triggers would run again, are not pinned.
            PageMethod::Activate if method.recv() == PageRecv::Page => {
                let refresh = matches!(args.first(), Some(Value::Boolean(true)));
                let (open, typed) = {
                    let s = st.borrow();
                    (s.open, s.typed_handler)
                };
                if !open && !typed {
                    return Ok(Value::Boolean(true));
                }
                if !(typed || self.is_hidden_part(st)) {
                    return unsupported("Page.Activate on a page the test client shows (not pinned)");
                }
                if refresh
                    && [
                        "onaftergetrecord",
                        "onaftergetcurrrecord",
                        "onfindrecord",
                        "onnextrecord",
                    ]
                    .iter()
                    .any(|t| self.declares(st, t))
                {
                    return unsupported(
                        "Page.Activate(true) on an open page with row triggers (whether they run again is not pinned)",
                    );
                }
                Ok(Value::Boolean(true))
            }
            PageMethod::GoToRecord => {
                self.require_open(st)?;
                let Some(Value::Record(r)) = args.first() else {
                    return unsupported("GoToRecord without a record");
                };
                let table = self.table(r)?;
                let mut key: Vec<Value> = table.primary_key(&r.borrow().fields);
                self.page_go_to_key_captions(st, &mut key)
            }
            other => unsupported(format!("TestPage.{other}")),
        }
    }

    /// CurrPage.SaveRecord / Update(true): write the page's row now.
    fn save_record_now(&mut self, st: &St) -> Result<(), Signal> {
        {
            // Page code changed Rec: SaveRecord writes it whether or not the client typed anything
            // (corpus testpage/TestPageTriggerEvents.al, ModifyRecordEvent_PageDrivenSave_*).
            // Page code that inserted the new row itself and copied it into Rec hands the page
            // that row (corpus testpage/TestPageOpenNewAfterGetCurrRecord.al:44-63).
            if self.save_found_row_from_new_line(st)? {
                return Ok(());
            }
            if st.borrow().draft && !st.borrow().draft_stored && {
                let rec = self.page_rec(st);
                self.stored_row(&rec)?.is_some()
            } {
                let mut s = st.borrow_mut();
                s.draft = false;
                s.dirty = false;
                drop(s);
                self.set_xrec(st);
                return Ok(());
            }
            // The same on the new line of a ListPart on a Document host whose code changed Rec (a page variable control's OnValidate
            // validated it into a field): CurrPage.Update() inserts the line (pages/parts/PartNewRowPageVariable.al:744
            // Plain_NewRow_VariableWithUpdateFirst `V-UPD 0 | T-V 0 1 | INS 0 1 | T-I 0 1`, :996 DelayedInsert, :1248 AutoSplitKey
            // `INS 20000 1`, :1500, :1752).
            let new_line_changed = {
                let s = st.borrow();
                s.cursor == Cursor::NewLine && s.draft
            } && self.new_row_part_pinned(st)
                && !Self::same_values(&self.page_rec(st).borrow().fields, &self.page_xrec_fields(st));
            if st.borrow().cursor == Cursor::Row || new_line_changed {
                st.borrow_mut().dirty = true;
            }
            self.save_row(st)
        }
    }

    // ---- opening and closing ------------------------------------------------------------
}

impl Session {
    /// Whether SetSelectionFilter of the part `st` stands on no row while its Rec `src` holds the values its SubPageLink set (not the
    /// blank record), into a record `dst` without filters or marks (see the caller for the pin).
    fn linked_part_on_no_row(&mut self, st: &St, src: &Rec, dst: &Rec) -> bool {
        st.borrow().cursor != Cursor::Row && st.borrow().host.is_some() && !self.is_blank_row(src) && {
            let d = dst.borrow();
            d.marks.is_empty() && !d.marked_only && d.filters.is_empty()
        }
    }

    /// Whether the open page `st` hands its selection to a record `dst` without filters whose table holds no row.
    fn selection_into_empty_table(&mut self, st: &St, dst: &Rec) -> bool {
        st.borrow().open
            && dst.borrow().filters.is_empty()
            && matches!(
                self.record_op(RecordOp::IsEmpty, dst, &mut []),
                Ok(Value::Boolean(true))
            )
    }
}
