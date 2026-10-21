use super::*;
use al_builtins::page_op::PageMethod;
use al_builtins::record_op::RecordOp;

impl Session {
    /// `TestPage.New()` (corpus handlers/TestPageInsertAllowed_Tests.al:70 New_OnInsertAllowedFalsePage_IsDenied,
    /// handlers/TestPageOpenNewAfterGetCurrRecord.al New_OnAnOpenCard_RunsOnAfterGetCurrRecordForTheNewRecord).
    pub(super) fn new_record(&mut self, st: &St) -> Result<Value, Signal> {
        self.refuse_read_only_part(st, "New()")?;
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let insert_denied = flag_denied(&engine, object, "InsertAllowed")?;
        // A request page is pinned only where InsertAllowed is false: the error names the report and `Id = 0`
        // (report/lt-y-request-page/YRequestPageTests.al:161). Which row a new line gets and whether the request page writes the
        // row it leaves are not pinned (the AutoSplitKey cases of the same file go by rules the corpus does not settle).
        if self.is_request_page(st) && !insert_denied {
            let changed = {
                let s = st.borrow();
                s.dirty && !s.draft && s.cursor == Cursor::Row
            };
            return unsupported(if changed {
                "New() on a request page after a changed row (whether the request page writes the row it leaves is not pinned)"
            } else {
                "New() on a request page that allows inserts (the new line is not pinned)"
            });
        }
        if insert_denied {
            // The text names the page and its id, the code is DB:ClientInsertDenied (corpus
            // pageext/lt-z-page-ext-props/ZPageExtProps.al:ZInsertAllowed_1, _4).
            let o = &engine.program.objects[object];
            let id = if self.is_request_page(st) { Some(0) } else { o.id };
            let Some(id) = id else {
                return unsupported("New() on a page without an id denied by InsertAllowed (the text is not pinned)");
            };
            return Err(error_code(
                format!(
                    "New method failed because Insert is not allowed. Page = {}, Id = {id}.",
                    o.name
                ),
                "DB:ClientInsertDenied",
            ));
        }
        let (left, origin) = self.traced(st, |s| s.leave_row(st, PageMethod::New));
        if let (Err(Signal::Error(_)), Some("onmodifyrecord")) = (&left, origin.as_deref()) {
            return unsupported("an error in OnModifyRecord on New() (only Next() is pinned)");
        }
        match left? {
            Left::Stay => Ok(Value::Null),
            Left::Go => {
                let writes = self.write_mark();
                let (r, origin) = self.traced(st, |s| s.start_draft(st, true));
                match (r, origin.as_deref()) {
                    (Err(Signal::Error(e)), Some("onnewrecord")) => {
                        self.swallowed_trigger_error(st, "OnNewRecord", "New()", &e, writes)?;
                        // What GetLastErrorText reads after the swallowed error is not pinned.
                        self.last_error = al_builtins::native::unpinned_error();
                        Ok(Value::Null)
                    }
                    (r, _) => r.map(|_| Value::Null),
                }
            }
        }
    }

    /// The error of `TestPage.OpenNew` on an API page: `Sorry, the current permissions prevented the action. (Page <id> <name>
    /// Insert: <app>)`, code DB:ClientInsertDenied, and no trigger runs (corpus pages/lt-cf-page-leads/CfPageLeads.al:144-180
    /// ApiPage_OnInsertRecordInsertsAndReturnsFalse and :182-218 _ReturnsTrue, both on an API page with DelayedInsert = true that
    /// declares OnInsertRecord and nothing else, CfPageObjects.al:122-189). Any other API page shape is refused: not pinned.
    pub(super) fn api_open_new_denied(&self, st: &St) -> Result<Signal, Signal> {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let delayed =
            effective_prop(&engine, object, "DelayedInsert")?.is_some_and(|v| v.trim().eq_ignore_ascii_case("true"));
        let o = &engine.program.objects[object];
        let own_props_pinned = ["Editable", "InsertAllowed", "SourceTableView", "SourceTableTemporary"]
            .iter()
            .all(|p| effective_prop(&engine, object, p).ok().flatten().is_none());
        let triggers_pinned = page_of(&engine, st).triggers.iter().all(|(n, _)| n == "oninsertrecord");
        let extended = o.page.as_ref().is_some_and(|p| !p.extensions.is_empty());
        if !delayed || !own_props_pinned || !triggers_pinned || !self.declares(st, "oninsertrecord") || extended {
            return unsupported(
                "OpenNew of an API page other than one with DelayedInsert that declares only OnInsertRecord (not pinned)",
            );
        }
        let Some(id) = o.id else {
            return unsupported("OpenNew of an API page without an id (the error text is not pinned)");
        };
        let app = engine
            .program
            .apps
            .get(o.app as usize)
            .map(|a| a.name.clone())
            .unwrap_or_default();
        Ok(error_code(
            format!(
                "Sorry, the current permissions prevented the action. (Page {id} {} Insert: {app})",
                o.name
            ),
            "DB:ClientInsertDenied",
        ))
    }

    /// The error of `TestPage.OpenNew` on a card without a source table, after OnOpenPage: the text and code of the API page
    /// denial (misc/lt-protected-var-abs-query-views/P146PageExprSetValue.al:460 QG_NoSource_OpenNew_StepLog `Sorry, the current
    /// permissions prevented the action. (Page 86704 LT QG NoSource Card Insert: AL Language Coverage Tests)`,
    /// `DB:ClientInsertDenied`). An extended page is not pinned.
    pub(super) fn no_source_open_new_denied(&self, st: &St) -> Result<Signal, Signal> {
        let engine = self.engine();
        let o = &engine.program.objects[st.borrow().object as usize];
        if o.page.as_ref().is_some_and(|p| !p.extensions.is_empty()) {
            return unsupported("OpenNew of an extended page without a source table (not pinned)");
        }
        let Some(id) = o.id else {
            return unsupported("OpenNew of a page without an id (the error text is not pinned)");
        };
        let app = engine
            .program
            .apps
            .get(o.app as usize)
            .map(|a| a.name.clone())
            .unwrap_or_default();
        Ok(error_code(
            format!(
                "Sorry, the current permissions prevented the action. (Page {id} {} Insert: {app})",
                o.name
            ),
            "DB:ClientInsertDenied",
        ))
    }

    /// After the page inserted its new row: a row outside the page's filters. SetValue on a field outside the key of a list
    /// raises `Validation error for Field: ,  Message = 'The view is filtered, and the entry is outside the filter. Some actions
    /// may not work.'` (TestValidation) and the row stays stored (corpus pages/lt-ao-testpage/AoTests.al:467-487
    /// FilterGroup0_NewRowField and :489-509 FilterGroup2_NewRowField, a filter on a field outside the key; events/
    /// lt-cm-trigger-leads/CmTriggerLeads.al:405-440 Item40_PageInsert_FilteredView, a TestPage filter on the key). Whether
    /// the field was validated before the error, what the page shows and saves afterwards (also when its variable goes out of
    /// scope) are not pinned: such a field with validation code is refused, and so is every later call on the page and the
    /// end of the variable's scope. The insert outside the filters at Close raises nothing (pages/lt-ap-testpage-642/
    /// ApTestPageTests.al:126-154 Ap02_OpenNew_NonKeyFilter_SetValue, `closed-row |typed|` outside `G7`), and so does the save
    /// of a filter change (misc FilterChangeSavesThePendingRow); this is only called for the insert of a SetValue.
    pub(super) fn inserted_outside_view(&mut self, st: &St, idx: usize) -> Result<(), Signal> {
        if st.borrow().draft || !self.has_source(st) {
            return Ok(());
        }
        let rec = self.page_rec(st);
        if rec.borrow().filters.is_empty() {
            return Ok(());
        }
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        self.ret_used = true;
        if matches!(
            self.record_op(RecordOp::Find, &probe, &mut [Value::Text("=".into())])?,
            Value::Boolean(true)
        ) {
            return Ok(());
        }
        let shape = self.is_list_page(st) && st.borrow().host.is_none() && st.borrow().parts.is_empty();
        let engine = self.engine();
        let ctl = &page_of(&engine, st).controls[idx];
        let plain_field = ctl.trigger("onvalidate").is_none()
            && self.control_field(st, idx).is_some_and(|f| {
                f.on_validate.is_none()
                    && f.ext_before_validate.is_empty()
                    && f.ext_after_validate.is_empty()
                    && !f.relation
            });
        if !shape || !plain_field {
            return unsupported(
                "a page inserted a new row outside its filters on SetValue of a field with validation code, or on a page other than a list (not pinned)",
            );
        }
        {
            let mut s = st.borrow_mut();
            s.unpinned_state = Some("a list page after it inserted a new row outside its filters (not pinned)".into());
            s.unpinned_close = true;
        }
        if !self.pages.scope_unpinned.iter().any(|p| Rc::ptr_eq(p, st)) {
            self.pages.scope_unpinned.push(st.clone());
        }
        Err(error_code(
            "Validation error for Field: ,  Message = 'The view is filtered, and the entry is outside the filter. Some actions may not work.'",
            "TestValidation",
        ))
    }

    /// Whether control `idx` shows a field of the page's source table outside its primary key.
    pub(super) fn control_outside_key(&self, st: &St, idx: usize) -> bool {
        let engine = self.engine();
        let ir::Binding::Field(pos) = page_of(&engine, st).controls[idx].binding else {
            return false;
        };
        let table = self.page_rec(st).borrow().table as usize;
        engine.program.objects[table]
            .table
            .as_ref()
            .is_some_and(|t| !t.keys[0].contains(&pos))
    }

    /// SetValue on a field outside the key of a card whose OnNewRecord inserted the new row: the page inserts the row again
    /// (OnInsertRecord, then the table's OnInsert), the insert fails on the stored row and SetValue raises `Validation error for
    /// Field: <control>,  Message = 'This field cannot be validated since the key fields are not valid.'` (TestValidation) without
    /// validating the field; Close then saves nothing (corpus pages/lt-cf-page-leads/CfPageLeads.al:241-289
    /// Card_OnNewRecordInserts_OpenNewSetClose). Any other outcome of the insert, and every later call but Close, is refused.
    pub(super) fn insert_of_trigger_inserted_row(
        &mut self,
        st: &St,
        idx: usize,
        activated: Result<(), Signal>,
    ) -> Result<Signal, Signal> {
        match activated {
            Err(Signal::Error(e)) if e.code == "DB:RecordExists" => {}
            Err(Signal::Error(_)) | Ok(()) => {
                return unsupported(
                    "SetValue on a card whose OnNewRecord inserted the new row, when the page's insert does not fail on the stored row (not pinned)",
                );
            }
            Err(other) => return Err(other),
        }
        // The text names the control; the corpus control has one name for the control, its field and its caption.
        let engine = self.engine();
        let ctl = &page_of(&engine, st).controls[idx];
        let field = self.control_field(st, idx).map(|f| f.name.clone()).unwrap_or_default();
        let name = ctl.name.clone();
        let caption = self.control_caption(st, idx)?;
        if !name.eq_ignore_ascii_case(&field) || caption != field {
            return unsupported(
                "SetValue on a card whose OnNewRecord inserted the new row, on a control whose name, field and caption differ (which one the error names is not pinned)",
            );
        }
        {
            let mut s = st.borrow_mut();
            s.new_row_inserted_by_trigger = false;
            s.dirty = false;
            s.unpinned_state = Some(
                "a card after its insert of a row its OnNewRecord had inserted failed (only Close is pinned)".into(),
            );
            s.unpinned_close = false;
        }
        Ok(error_code(
            format!(
                "Validation error for Field: {field},  Message = 'This field cannot be validated since the key fields are not valid.'"
            ),
            "TestValidation",
        ))
    }

    /// An AL error of OnInsertRecord when SetValue on a non-key control inserts the new row of a card opened for editing: a
    /// TestValidation error that names no field (corpus pages/lt-cc-page-leads/CcPageLeads.al:585-595 TriggerError_OnInsertRecord
    /// `Validation error for Field: ,  Message = 'LT CC failure in OnInsertRecord'`). The page stays open; what it shows and
    /// saves afterwards is not pinned, so every later call on it is refused (as for `swallowed_trigger_error`).
    pub(super) fn insert_trigger_error(
        &mut self,
        st: &St,
        e: &al_builtins::native::AlError,
        writes: (i64, u64),
    ) -> Result<Signal, Signal> {
        self.swallowed_trigger_error(st, "OnInsertRecord", "SetValue", e, writes)?;
        Ok(error_code(
            format!("Validation error for Field: ,  Message = '{}'", e.message),
            "TestValidation",
        ))
    }

    /// An AL error of OnNewRecord on `New()`, or of OnModifyRecord when `Next()` saves the row, raises nothing on a card
    /// opened for editing (corpus pages/lt-cc-page-leads/CcPageLeads.al:572-582 TriggerError_OnNewRecord and :598-608
    /// TriggerError_OnModifyRecord, `step = ok`). Which row the page shows then, and what a later save or Close does, is not
    /// pinned (docs/probes-needed.md): every later call on the page is refused. Another error, another page shape, or a trigger
    /// that wrote to the database before it failed (whether the write survives is not pinned) is refused at once.
    pub(super) fn swallowed_trigger_error(
        &mut self,
        st: &St,
        trigger: &str,
        call: &str,
        e: &al_builtins::native::AlError,
        writes: (i64, u64),
    ) -> Result<(), Signal> {
        let pinned_shape = e.code == "Dialog"
            && self.is_card(st)
            && st.borrow().mode == PageMode::Edit
            && !st.borrow().modal
            && st.borrow().host.is_none()
            && st.borrow().parts.is_empty()
            && !self.is_request_page(st)
            && self.write_mark() == writes;
        if !pinned_shape {
            return unsupported(format!(
                "an error in {trigger} on {call} of this page (only an Error of a card opened for editing that wrote nothing is pinned)"
            ));
        }
        st.borrow_mut().unpinned_state = Some(format!(
            "a page after an error in {trigger} on {call} (the row it shows and what it saves are not pinned)"
        ));
        st.borrow_mut().unpinned_close = true;
        Ok(())
    }

    /// Saves the row the cursor is leaving. A started line whose insert fails does not raise:
    /// New(), Next() and Last() stay on it with one validation error on its key control; Previous()
    /// and First() move on and the insert fails again at Close()
    /// (corpus handlers/TestPageFailedInsert_Tests.al:1-30).
    pub(super) fn leave_row(&mut self, st: &St, how: PageMethod) -> Result<Left, Signal> {
        let r = self.save_row(st);
        match r {
            Err(Signal::Error(e)) if st.borrow().draft && st.borrow().dirty => match how {
                PageMethod::New | PageMethod::Next | PageMethod::Last => {
                    let key_ctl = self.key_control(st);
                    let mut s = st.borrow_mut();
                    al_builtins::native::refuse_unpinned_text(&e, "The error list of a page row")?;
                    if !s.refused {
                        s.errors.push((key_ctl, e.message));
                        s.refused = true;
                    }
                    Ok(Left::Stay)
                }
                PageMethod::Previous | PageMethod::First => {
                    let fields = self.page_rec(st).borrow().fields.clone();
                    let mut s = st.borrow_mut();
                    s.pending = Some(fields);
                    s.draft = false;
                    s.dirty = false;
                    Ok(Left::Go)
                }
                _ => Err(Signal::Error(e)),
            },
            Err(e) => Err(e),
            Ok(()) => Ok(Left::Go),
        }
    }

    /// The control bound to the first field of the primary key.
    pub(super) fn key_control(&self, st: &St) -> usize {
        if !self.has_source(st) {
            return 0;
        }
        let engine = self.engine();
        let page = page_of(&engine, st);
        let table = self.page_rec(st).borrow().table as usize;
        let key0 = engine.program.objects[table].table.as_ref().map(|t| t.keys[0][0]);
        page.controls
            .iter()
            .position(|c| matches!(c.binding, ir::Binding::Field(p) if Some(p) == key0))
            .unwrap_or(0)
    }

    /// xRec := Rec, the row as the client read it.
    pub(super) fn set_xrec(&mut self, st: &St) {
        if !self.has_source(st) {
            return;
        }
        let fields = self.page_rec(st).borrow().fields.clone();
        self.set_xrec_fields(st, fields);
    }

    /// xRec holds `fields`: the row as it was read.
    pub(super) fn set_xrec_fields(&mut self, st: &St, fields: Vec<Value>) {
        if !self.has_source(st) {
            return;
        }
        let object = st.borrow().object as usize;
        let Some(slot) = self.engine.program.objects[object].xrec_slot else {
            return;
        };
        let inst = st.borrow().instance.clone().expect("open page");
        if let Value::Record(x) = &inst.borrow().globals[slot] {
            x.borrow_mut().fields = fields;
        }
    }

    pub(super) fn page_xrec_fields(&self, st: &St) -> Vec<Value> {
        if !self.has_source(st) {
            return Vec::new();
        }
        let object = st.borrow().object as usize;
        let slot = self.engine.program.objects[object]
            .xrec_slot
            .expect("page with a source table");
        let inst = st.borrow().instance.clone().expect("open page");
        let v = inst.borrow().globals[slot].clone();
        match v {
            Value::Record(x) => x.borrow().fields.clone(),
            _ => Vec::new(),
        }
    }

    pub(super) fn same_values(a: &[Value], b: &[Value]) -> bool {
        a.len() == b.len()
            && a.iter()
                .zip(b)
                .all(|(x, y)| ops::compare(x, y).map(|o| o.is_eq()).unwrap_or(false))
    }

    /// The stored row with `rec`'s primary key, whatever the view says.
    pub(super) fn stored_row(&mut self, rec: &Rec) -> Result<Option<Vec<Value>>, Signal> {
        let table = self.table(rec)?;
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        probe.borrow_mut().filters.clear();
        let mut pk = table.primary_key(&rec.borrow().fields);
        self.ret_used = true;
        Ok(match self.record_op(RecordOp::Get, &probe, &mut pk)? {
            Value::Boolean(true) => Some(probe.borrow().fields.clone()),
            _ => None,
        })
    }

    /// A new record on the page: xRec stays on the row the cursor was on, Rec starts from the table's
    /// defaults, OnNewRecord runs with BelowxRec true (corpus
    /// handlers/TestPageCreateModeNewRecordArgs.al:243-270), then the row is populated
    /// (handlers/TestPageOpenNewAfterGetCurrRecord.al:1-12: OnAfterGetCurrRecord runs after OnNewRecord).
    pub(super) fn start_draft(&mut self, st: &St, keep_focus: bool) -> Result<(), Signal> {
        if !self.has_source(st) {
            return unsupported("a new record on a page without a source table");
        }
        let rec = self.page_rec(st);
        let part_xrec = if keep_focus { self.part_new_xrec(st)? } else { None };
        self.set_xrec(st);
        // A new line of a list starts below the last row the client holds: OnNewRecord sees that row as xRec, while the
        // insert sees the row the cursor was on (misc/W2BelowXRec.al:NewInTheMiddle_CursorOnFirstRow_KeyBetween,
        // NewInTheMiddle_CursorOnMiddleRow_KeyBefore, NewTwice_InTheMiddle: xRec=30 at OnNewRecord, 10 at OnInsertRecord).
        // A list longer than its window first reads on to the end of the view (the new line goes after the last row).
        let reads_on = {
            let s = st.borrow();
            keep_focus
                && s.host.is_none()
                && s.cursor == Cursor::Row
                && !s.draft
                && s.window_from_start
                && s.window.len() == Self::WINDOW
                && !s.window_eof
        } && self.is_list(st)
            && self.declares(st, "onaftergetrecord")
            && self.declares(st, "onaftergetcurrrecord")
            && !self.declares(st, "onfindrecord")
            && !self.declares(st, "onnextrecord");
        if reads_on {
            self.read_block_past_the_window(st, false)?;
        }
        let last_held = {
            let s = st.borrow();
            (keep_focus
                && s.host.is_none()
                && self.is_list(st)
                && s.cursor == Cursor::Row
                && !s.draft
                && s.window_from_start
                && (s.window.len() < Self::WINDOW || s.window_whole))
                .then(|| s.window.last().map(|w| w.stored.clone()))
                .flatten()
        };
        // New() from the untouched placeholder line the cursor moved onto keeps the row above it as the lower neighbour and
        // burns one interval on the placeholder (pages/lt-cd-testpage-leads/CdTestPageLeads.al:587 Item59_EnumSubPageLink_NewLine:
        // Next() onto the blank line below 10000, New(), the typed line is 30000), as an empty grid does (see `auto_split_key`).
        let from_placeholder = {
            let s = st.borrow();
            s.cursor == Cursor::NewLine && s.placeholder_line && !s.dirty && s.lower_key.is_some()
        };
        if from_placeholder {
            st.borrow_mut().split_burned = true;
        } else {
            self.note_lower_key(st)?;
            st.borrow_mut().split_burned = false;
        }
        st.borrow_mut().placeholder_line = false;
        // A list line started below a stored row: remember the row (see `PageState::after_new`).
        let anchor = (keep_focus && st.borrow().cursor == Cursor::Row && !st.borrow().draft)
            .then(|| rec.borrow().fields.clone());
        st.borrow_mut().after_new = anchor;
        let table = rec.borrow().table as usize;
        let fresh = self.initial_fields(table);
        rec.borrow_mut().fields = fresh;
        self.stamp_from_filters(st, true)?;
        let engine = self.engine();
        let page = page_of(&engine, st);
        let first = page.controls.iter().position(|c| c.kind == ir::ControlKind::Field);
        {
            let mut s = st.borrow_mut();
            s.cursor = if page.controls.iter().any(|c| c.kind == ir::ControlKind::Repeater) {
                Cursor::NewLine
            } else {
                Cursor::Row
            };
            s.draft = true;
            s.new_by_call = true;
            s.dirty = false;
            s.refused = false;
            if !keep_focus {
                s.focus = first;
                s.focus_in_row = true;
            } else {
                // Focus stays on the control of the row the cursor left
                // (corpus handlers/TestPageBlankKeyInsert.al, List_NewRow_FocusStays...).
                s.focus_in_row = false;
            }
        }
        let existed = self.stored_row(&rec)?.is_some();
        st.borrow_mut().draft_stored = existed;
        st.borrow_mut().typed_before_key = false;
        if let Some(last) = last_held.or(part_xrec) {
            self.set_xrec_fields(st, last);
        }
        st.borrow_mut().new_row_inserted_by_trigger = false;
        self.page_trigger(st, "onnewrecord", vec![Value::Boolean(true)])?;
        self.page_event(st, "onnewrecordevent", None, Some(true))?;
        // A card opened with OpenNew whose OnNewRecord inserts the new row itself still holds it as a new row: the next write to
        // a field outside the key inserts it again (corpus pages/lt-cf-page-leads/CfPageLeads.al:241-289
        // Card_OnNewRecordInserts_OpenNewSetClose: `OnInsertRecord:DOC1,Doc.OnInsert:DOC1` at SetValue). Only that shape is
        // pinned: a list, a part, a host of parts or New() on an open card is refused.
        if !existed && self.stored_row(&rec)?.is_some() {
            let pinned = {
                let s = st.borrow();
                s.test_opened_new && s.host.is_none() && s.parts.is_empty() && !s.modal && s.mode == PageMode::New
            } && self.is_card(st)
                && !self.is_request_page(st);
            if !pinned {
                return unsupported(
                    "OnNewRecord inserted the new row itself on a page other than a card opened with OpenNew (not pinned)",
                );
            }
            st.borrow_mut().new_row_inserted_by_trigger = true;
        }
        if self.is_list(st) && st.borrow().host.is_none() {
            // xRec is the new record itself when the line becomes current (misc/lt-probes-needed-7/PnGNewRecord.al:
            // HandedRecordNoFilters `AGCR(,)`).
            self.set_xrec(st);
        } else if self.is_list(st)
            && st
                .borrow()
                .host
                .as_ref()
                .and_then(|(h, _)| h.upgrade())
                .is_some_and(|h| !self.is_list(&h))
        {
            // The same for a ListPart in a Document page: OnNewRecord sees the last row as xRec, OnAfterGetCurrRecord the blank new
            // row (pages/lt-qj-part-propagation/QjPartPropagation.al:763 QJ_SubformSaveRecord_NewRow
            // `S.NEW(x=1 x.ln=10000) S.AGCR(0 r=0 x=0)`).
            self.set_xrec(st);
        }
        // A new row nobody has saved raises OnAfterGetCurrRecord but no OnAfterGetRecord (corpus
        // pageupdate/TestPageUpdateNewRow.al:OpenNew_TraceIsOneOnAfterGetCurrRecord).
        self.populate_expressions(st)?;
        self.page_trigger(st, "onaftergetcurrrecord", vec![])?;
        self.page_event(st, "onaftergetcurrrecordevent", None, None)?;
        // Page code that inserts the new row itself (a template step) hands the page the stored row.
        if !existed && !st.borrow().new_row_inserted_by_trigger && self.stored_row(&rec)?.is_some() {
            let mut s = st.borrow_mut();
            s.draft = false;
            s.cursor = Cursor::Row;
            drop(s);
            self.set_xrec(st);
        }
        Ok(())
    }

    /// Writes what the client holds for the current row: inserts a typed new record, modifies an
    /// edited one when a value really differs (corpus handlers/TestPageSameValueSetValue_Tests.al:1-30).
    /// Closing a page whose row was changed while ModifyAllowed is false raises `Page cannot be saved.` with the
    /// code TestWrapped and leaves the stored row alone (corpus pageext/lt-z-page-ext-props/ZPageExtProps.al:
    /// ZModifyAllowed_1 and _4, a list page opened for edit with one changed field).
    pub(super) fn close_denied_modify(&mut self, st: &St) -> Result<(), Signal> {
        let (cursor, draft, dirty) = {
            let s = st.borrow();
            (s.cursor, s.draft, s.dirty)
        };
        if cursor == Cursor::None || !dirty || draft || !self.has_source(st) {
            return Ok(());
        }
        if !flag_denied(&self.engine(), st.borrow().object as usize, "ModifyAllowed")? {
            return Ok(());
        }
        let rec = self.page_rec(st);
        if Self::same_values(&rec.borrow().fields, &self.page_xrec_fields(st)) {
            return Ok(());
        }
        Err(error_code("Page cannot be saved.", "TestWrapped"))
    }

    pub(super) fn save_row(&mut self, st: &St) -> Result<(), Signal> {
        let (cursor, draft, dirty) = {
            let s = st.borrow();
            (s.cursor, s.draft, s.dirty)
        };
        if cursor == Cursor::None || !dirty || !self.has_source(st) {
            return Ok(());
        }
        if draft {
            return self.insert_draft(st);
        }
        let rec = self.page_rec(st);
        if Self::same_values(&rec.borrow().fields, &self.page_xrec_fields(st)) {
            st.borrow_mut().dirty = false;
            return Ok(());
        }
        // Only a close with a changed row on a page that does not allow modify is pinned (see `close_denied_modify`).
        if flag_denied(&self.engine(), st.borrow().object as usize, "ModifyAllowed")? {
            return unsupported(
                "saving a changed row on a page whose ModifyAllowed is false, other than at Close (not pinned)",
            );
        }
        // corpus handlers/TestPageTriggerEvents.al (ModifyRecordEvent_RunsAfterThePagesOwnOnModifyRecordTrigger)
        let keep = self.page_trigger_answer(st, "onmodifyrecord", vec![])?;
        if keep == Some(false) {
            return Ok(());
        }
        if !self.page_event(st, "onmodifyrecordevent", Some(true), None)? {
            return Ok(());
        }
        self.ret_used = false;
        // The page saves the row on fresh table globals: what an action set in them is gone (corpus
        // record/PWTableGlobals.al:408-431: OnModify reads Counter 0 though the action set 7).
        self.reset_table_globals(&rec);
        self.pages.page_xrec = Some(self.page_xrec_fields(st));
        let modified = self.record_op(RecordOp::Modify, &rec, &mut [Value::Boolean(true)]);
        self.pages.page_xrec = None;
        // The page keeps the row it read; when code behind it has changed that row since, whether the client has read it again
        // first is not pinned (no corpus page test reaches DB:RecordChanged), so the page's own save refuses rather than fails.
        if let Err(Signal::Error(e)) = &modified {
            if e.code == "DB:RecordChanged" {
                return unsupported(
                    "a page saves a row that code changed after the page read it (whether the client reads the row again first is not pinned)",
                );
            }
        }
        modified?;
        st.borrow_mut().dirty = false;
        self.set_xrec(st);
        Ok(())
    }

    /// OnInsertRecord decides whether the typed record is inserted at all (corpus
    /// handlers/TestPageRecordTriggers.al:76 OnInsertRecord_ReturningFalse_SuppressesTheInsert).
    pub(super) fn insert_draft(&mut self, st: &St) -> Result<(), Signal> {
        let rec = self.page_rec(st);
        // A part line whose host is a new, unsaved record with a blank link key: BC saves the host first (its OnInsert numbers it)
        // and links the line to it. Which order the triggers run in, and what the line's key holds, are not pinned: refused.
        let host_unsaved = st
            .borrow()
            .host
            .as_ref()
            .and_then(|(h, _)| h.upgrade())
            .is_some_and(|h| h.borrow().draft);
        if host_unsaved {
            let table = self.table(&rec)?;
            let blank_link = rec.borrow().filters.iter().any(|f| {
                table.keys[0].contains(&f.field)
                    && match &f.expr {
                        al_builtins::filter::FilterExpr::Eq(Value::Text(t) | Value::Code(t)) => t.is_empty(),
                        al_builtins::filter::FilterExpr::Range(
                            Some(Value::Text(a) | Value::Code(a)),
                            Some(Value::Text(b) | Value::Code(b)),
                        ) => a.is_empty() && b.is_empty(),
                        _ => false,
                    }
            });
            if blank_link {
                return unsupported(
                    "a part line inserted while its host is a new record whose link key is still blank (BC saves the host first; the order is not pinned)",
                );
            }
        }
        // BelowxRec is Yes for a row added with New() at the end, as it is for OnNewRecord (corpus
        // handlers/TestPageRuntimeContracts.al:197-211 TestPage_New_AtTheEnd_PassesBelowxRecTrueToOnInsertRecord).
        // A row added in the middle is not pinned; it is given the same Yes as OnNewRecord gets.
        let anchor = st.borrow().after_new.clone();
        // In a part whose whole view the client holds, a line started below a row other than the last one (pages/
        // lt-qj-part-propagation/QjPartPropagation.al:3000 QN_PartDelayedOnly_NewTyped_ThenNew `D.INSR(0 a=7 x=3 x.ln=30000)` after
        // New() from row 30000 of two; :3106, :3212, :3318, :3424). From the last row the insert keeps the blank xRec (:925
        // QJ_SubformSaveRecord_NewRow `S.INSR(20000 r=5 x=0 x.ln=0)`).
        let table = self.table(&rec)?;
        let part_middle = |s: &Self, a: &Vec<Value>| {
            s.part_view_held(st)
                && st
                    .borrow()
                    .window
                    .last()
                    .is_some_and(|w| !Self::same_key(&w.key, &table.primary_key(a)))
        };
        if let Some(anchor) =
            anchor.filter(|a| self.is_list(st) && (st.borrow().host.is_none() || part_middle(self, a)))
        {
            // The insert sees the row the line was started below as xRec (misc/W2BelowXRec.al:NewInTheMiddle_*).
            self.set_xrec_fields(st, anchor);
        }
        // A line of a part is inserted while a stored host row may hold an edit of its own. The line goes in first, so the header's
        // OnModify sees it (testpart/ OkInvoke_HeaderOnModifySeesThePartRowSavedFirst); the host then reads its row again, because the
        // line's table code may have changed it, puts its own edit on top and saves (pages/lt-changed-by-other/
        // ChangedByOtherPlain.al:P182b_Plain_SaveRecordAction: `stored-after-line` is `descr=typed info=by line insert counter=1`, the
        // card shows `by line insert` and `1`). Only a stored host row is pinned; a new host row or one without a source stays as before.
        let host = st
            .borrow()
            .host
            .as_ref()
            .and_then(|(h, _)| h.upgrade())
            .filter(|h| self.has_source(h) && !h.borrow().draft && h.borrow().cursor == Cursor::Row);
        // The AutoSplitKey number is assigned before OnInsertRecord, which sees it (pages/lt-part-line-before-host-key/
        // PartLineBeforeHostKey.al:612 `LP.OnInsertRecord:D2/20000 | L.OnInsert:D2/20000`, :551, :945; pages/
        // lt-bg-part-line-all-versions/BgPartLineAllVersions.al:349).
        self.auto_split_key(st)?;
        let go = self.page_trigger_answer(st, "oninsertrecord", vec![Value::Boolean(true)])?;
        if go == Some(false) {
            // A trigger that inserted the row itself and answered false leaves the page on a stored row: its next edit is
            // a modify (as for a template step in `start_draft`). Not pinned for this answer: Base Application's Item
            // Tracking Lines inserts into its temporary source this way (assumption page-oninsertrecord-false-after-code-insert).
            if !st.borrow().draft_stored && self.stored_row(&rec)?.is_some() {
                al_builtins::assumptions::assume(
                    al_builtins::assumptions::Assumption::PageOnInsertRecordFalseAfterCodeInsert,
                );
                {
                    let mut s = st.borrow_mut();
                    s.draft = false;
                    s.dirty = false;
                    s.cursor = Cursor::Row;
                }
                self.set_xrec(st);
            }
            return Ok(());
        }
        if !self.page_event(st, "oninsertrecordevent", Some(true), Some(true))? {
            return Ok(());
        }
        self.ret_used = false;
        self.tx.immediate_errors = true;
        let inserted = self.record_op(RecordOp::Insert, &rec, &mut [Value::Boolean(true)]);
        self.tx.immediate_errors = false;
        inserted?;
        {
            let mut s = st.borrow_mut();
            s.draft = false;
            s.dirty = false;
            s.cursor = Cursor::Row;
        }
        self.set_xrec(st);
        // The inserted row is the current row: the next OnAfterGetCurrRecord sees it as xRec (testpart/QiHostSaveParts.al:580
        // `L.AGCR(D1/30000|typed x=D1/30000)` after the typed line was saved before the host SetValue).
        st.borrow_mut().agcr_xrec = Some(rec.borrow().fields.clone());
        if let Some(host) = &host {
            let hrec = self.page_rec(host);
            if let Some(stored) = self.stored_row(&hrec)? {
                let typed = hrec.borrow().fields.clone();
                let before = self.page_xrec_fields(host);
                let mut merged = stored;
                let mut edited = false;
                for (i, v) in typed.iter().enumerate() {
                    if i < before.len() && !Self::same_values(std::slice::from_ref(v), std::slice::from_ref(&before[i]))
                    {
                        merged[i] = v.clone();
                        edited = true;
                    }
                }
                hrec.borrow_mut().fields = merged;
                if edited && host.borrow().dirty {
                    self.save_row(host)?;
                } else {
                    self.set_xrec(host);
                }
            }
        }
        // The parts of a new host row link to it once it is stored, with the number its OnInsert
        // gave it (corpus handlers/TestPageOpenNewPartLink.al).
        if !st.borrow().parts.is_empty() {
            self.sync_parts(st)?;
        }
        Ok(())
    }

    /// Focus moves to control `idx`. On a new record of a page that inserts as you go, moving from
    /// a control of that row to a non-key control inserts it (corpus
    /// handlers/TestPageBlankKeyInsert.al:1-25 and its List_NewRow_Focus... arm: after New() the
    /// focus still sits on the old row's control).
    pub(super) fn activate(&mut self, st: &St, idx: usize) -> Result<(), Signal> {
        let (prev, in_row) = {
            let s = st.borrow();
            (s.focus, s.focus_in_row)
        };
        if prev == Some(idx) {
            return Ok(());
        }
        {
            let mut s = st.borrow_mut();
            s.focus = Some(idx);
            s.focus_in_row = true;
        }
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let delayed = engine.program.objects[object]
            .properties
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("DelayedInsert") && v.trim().eq_ignore_ascii_case("true"));
        let page = page_of(&engine, st);
        let non_key = match page.controls[idx].binding {
            ir::Binding::Field(pos) => {
                let table = self.page_rec(st).borrow().table as usize;
                !engine.program.objects[table]
                    .table
                    .as_ref()
                    .is_some_and(|t| t.keys[0].contains(&pos))
            }
            _ => false,
        };
        if st.borrow().draft && in_row && prev.is_some() && !delayed && non_key {
            self.insert_draft(st)?;
            self.inserted_outside_view(st, idx)?;
        }
        Ok(())
    }
}
