use super::*;
use al_builtins::record_op::RecordOp;

impl Session {
    /// Whether the page carries the blank line past its data: an editable,
    /// insert-allowed repeater opened with OpenEdit (corpus
    /// handlers/TestPageNewRowLine_Tests.al:1-30, gating arms).
    pub(in crate::pages) fn offers_new_line(&mut self, st: &St) -> Result<bool, Signal> {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let page = page_of(&engine, st);
        let has_repeater = page.controls.iter().any(|c| c.kind == ir::ControlKind::Repeater);
        let props = &engine.program.objects[object].properties;
        let prop_false = |n: &str| {
            props
                .iter()
                .any(|(k, v)| k.eq_ignore_ascii_case(n) && v.trim().eq_ignore_ascii_case("false"))
        };
        // A tree shows no blank line, opened by Page.Run, Page.RunModal or a Page variable (pages/lt-ah-pages/AhPageTests.al:986,
        // :1006, :1026, :1044 `rows_initial = T1 T5`).
        let tree = self.tree_indentation(st).is_some();
        Ok(has_repeater
            && !tree
            && st.borrow().mode == PageMode::Edit
            // A list handed over in lookup mode has no new-row line (corpus
            // handlers/TestPageModalNewRowLine_Tests.al:209 Plain_LookupModeTrue_HasNoNewRowLine). A LookupMode(true) call
            // does that, and so does Page.RunModal(id, Rec) whose return value is used; called as a statement it keeps the
            // line (:180; pages/lt-be-runmodal-onlookup/BeRunModalOnLookup.al:616, :916).
            && !(st.borrow().lookup_set && st.borrow().lookup && self.is_list_page(st))
            && !prop_false("Editable")
            && !flag_denied(&engine, object, "InsertAllowed")?)
    }

    /// Whether a repeater of the page declares `Editable` with anything but the literal `true`.
    pub(super) fn repeater_says_editable(&self, st: &St) -> bool {
        let engine = self.engine();
        page_of(&engine, st).controls.iter().any(|c| {
            c.kind == ir::ControlKind::Repeater
                && match c.prop("editable") {
                    Some(ir::PropVal::Lit(v)) => !v.trim().eq_ignore_ascii_case("true"),
                    Some(_) => true,
                    None => false,
                }
        })
    }

    // ---- rows ---------------------------------------------------------------------------

    /// The page is a part its host left out of the control tree (`hidden_parts`: Visible came out false).
    pub(in crate::pages) fn is_hidden_part(&self, st: &St) -> bool {
        let host = st.borrow().host.as_ref().and_then(|(h, _)| h.upgrade());
        host.is_some_and(|h| h.borrow().hidden_parts.iter().any(|(_, c)| Rc::ptr_eq(c, st)))
    }

    /// A Page variable the test client does not show: it never ran, or its page closed. Not a part, not a typed handler's page.
    pub(in crate::pages) fn page_var_not_shown(&self, st: &St) -> bool {
        let s = st.borrow();
        !s.open && !s.typed_handler && s.host.is_none()
    }

    /// A Page variable that never ran (see `page_var_not_shown`).
    pub(in crate::pages) fn never_ran_page_var(&self, st: &St) -> bool {
        self.page_var_not_shown(st) && !st.borrow().ran
    }

    /// Whether the page declares trigger `name` (lower case) of its own.
    pub(in crate::pages) fn declares(&self, st: &St, name: &str) -> bool {
        let engine = self.engine();
        page_of(&engine, st).triggers.iter().any(|(n, _)| n == name)
    }

    /// FindFirst (`which` "-") or FindLast ("+") of the page's rowset. A page that declares
    /// OnFindRecord serves its rows from the trigger, which receives `Which` and answers whether it
    /// found a row (corpus pagefindrecord/TestPageFindRecord.al:112, :160). The strings the client
    /// really passes are `=`, `=>` and `=><` forms the corpus pins only for a deleted row
    /// (pageupdate/TestPageUpdateGoneRow.al:390); `-` and `+` are assumed (docs/probes-needed.md).
    pub(in crate::pages) fn nav_find(&mut self, st: &St, which: &str) -> Result<bool, Signal> {
        if self.declares(st, "onfindrecord") {
            let v = self.page_trigger(st, "onfindrecord", vec![Value::Text(which.into())])?;
            return Ok(matches!(v, Some(Value::Boolean(true))));
        }
        let rec = self.page_rec(st);
        self.ret_used = true;
        Ok(matches!(
            match which {
                "-" => self.record_op(RecordOp::FindFirst, &rec, &mut [])?,
                "+" => self.record_op(RecordOp::FindLast, &rec, &mut [])?,
                w => self.record_op(RecordOp::Find, &rec, &mut [Value::Text(w.into())])?,
            },
            Value::Boolean(true)
        ))
    }

    /// Next(steps) of the page's rowset: OnNextRecord when the page declares it, which answers how
    /// many rows it moved (corpus pagefindrecord/TestPageFindRecord.al:129).
    pub(in crate::pages) fn nav_next(&mut self, st: &St, steps: i32) -> Result<bool, Signal> {
        if self.declares(st, "onnextrecord") {
            let v = self.page_trigger(st, "onnextrecord", vec![Value::Integer(steps)])?;
            return Ok(matches!(v, Some(Value::Integer(n)) if n != 0));
        }
        let rec = self.page_rec(st);
        self.ret_used = true;
        let moved = self.record_op(RecordOp::Next, &rec, &mut [Value::Integer(steps)])?;
        Ok(matches!(moved, Value::Integer(n) if n != 0))
    }

    /// Lands on the first row of the view when the page opens: a stored row, else
    /// the new-row line, else nothing. The expressions of the controls are evaluated for
    /// an empty view too (corpus autoformat/TestAutoFormatExpressionTiming.al:5-12).
    /// Whether `rec` stands on the first row of its view (found on a copy, so `rec` does not move).
    pub(super) fn on_first_row_of_view(&mut self, rec: &Rec) -> Result<bool, Signal> {
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        self.ret_used = true;
        if !matches!(
            self.record_op(RecordOp::FindFirst, &probe, &mut [])?,
            Value::Boolean(true)
        ) {
            return Ok(false);
        }
        Ok(Self::same_key(&self.row_key(&probe)?, &self.row_key(rec)?))
    }

    /// Whether the OnOpenPage of a page of its own left Rec on a row of its view, with a primary key that is not blank, other than
    /// the first row of the view (the shape of AiListOrderTests.al:851, :871).
    pub(super) fn onopenpage_left_rec_past_the_first_row(&mut self, st: &St) -> Result<bool, Signal> {
        if st.borrow().host.is_some() || !self.declares(st, "onopenpage") {
            return Ok(false);
        }
        let rec = self.page_rec(st);
        let key = self.row_key(&rec)?;
        let table = self.table(&rec)?;
        let blank: Vec<Value> = table.keys[0]
            .iter()
            .map(|&f| self.default_value(&table.fields[f].ty))
            .collect();
        if Self::same_key(&key, &blank) {
            return Ok(false);
        }
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        self.ret_used = true;
        if !matches!(
            self.record_op(RecordOp::Find, &probe, &mut [Value::Text("=".into())])?,
            Value::Boolean(true)
        ) {
            return Ok(false);
        }
        Ok(!self.on_first_row_of_view(&rec)?)
    }

    pub(in crate::pages) fn page_first_row(&mut self, st: &St) -> Result<(), Signal> {
        // A page that serves its rows from OnFindRecord is asked `=><` when it opens (corpus
        // pageext/PCCurrRecordReads.al:FindList_OnFindRecordAndOnNextRecord, `open`).
        // A SourceTableView that sorts descending asks the same (pages/lt-ce-page-run/CePageRun.al
        // OpenView_DescendingWithOnFindRecord_CurrentRow `find(=><)=1`).
        let which = if self.declares(st, "onfindrecord") {
            // A view that filters asks the same `=><` once, list, card and list part alike
            // (pagefindrecord/QcFindViewTests.al:78 ViewList_OpenView_FindStringAndRow, :361 ViewCard_OpenView_FindStringAndRow,
            // :505 DocPart_OpenEdit_LinesOfH1).
            // A list page variable run in lookup mode (LookupMode(true)) with no record is asked `=>` (pages/lt-ai-list-order/
            // QgGoToKeyNextCall.al:629 Nav_LookupHandler_GoToKey_ThenOK `log` `F:=>=Yes;R:R1001;N:1=1;...`). The static RunModal
            // in lookup mode is not pinned; a TableRelation lookup keeps `=><` as before.
            let (lookup, lookup_set, static_lookup) = {
                let s = st.borrow();
                (s.lookup, s.lookup_set, s.static_lookup)
            };
            if lookup && static_lookup && self.is_list(st) {
                return unsupported(
                    "a list with OnFindRecord run in lookup mode by the static Page.RunModal without a record (which Find it is asked is not pinned)",
                );
            }
            if lookup && lookup_set && self.is_list(st) {
                "=>"
            } else if !lookup && self.is_list(st) && self.onopenpage_left_rec_past_the_first_row(st)? {
                // A list whose OnOpenPage left Rec on a row that is not the first of its view is asked `=>`
                // (pages/lt-ai-list-order/AiListOrderTests.al:851 OpenOnRow_SetPositionInOnOpenPage_LastRow_OpenLog
                // `F:=>=Yes;R:R1006;...`, :871 _SecondLastRow_OpenLog `F:=>=Yes;R:R1005;...`). On the first row it is not pinned
                // and keeps `=><`.
                "=>"
            } else {
                "=><"
            }
        } else {
            "-"
        };
        self.page_first_row_from(st, which, false)
    }

    /// `page_first_row`, finding the first row with `which`. With `handed` (a `=><` from a handed record) a row other than the
    /// first of the view starts a window that does not begin at the view's start.
    pub(in crate::pages) fn page_first_row_from(&mut self, st: &St, which: &str, handed: bool) -> Result<(), Signal> {
        // The first row of the view, to tell whether a `=><` landed on it (a copy of the record, so the page's own is not moved).
        let first_key = if handed && self.is_list(st) {
            let rec = self.page_rec(st);
            let probe = Rc::new(RefCell::new(rec.borrow().clone()));
            self.ret_used = true;
            if matches!(
                self.record_op(RecordOp::FindFirst, &probe, &mut [])?,
                Value::Boolean(true)
            ) {
                Some(self.row_key(&probe)?)
            } else {
                None
            }
        } else {
            None
        };
        // A part of a Document or Card host opened for view on no row shows no row, though rows of the blank key exist
        // (pages/lt-part-line-before-host-key/PartLineBeforeHostKey.al:1578 QH_OpenView_NoRow_BlankKeyLinesExist
        // `part_description=[]`).
        let host_shows_no_row = st
            .borrow()
            .host
            .as_ref()
            .and_then(|(h, _)| h.upgrade())
            .is_some_and(|h| {
                let hb = h.borrow();
                hb.host.is_none()
                    && hb.mode == PageMode::View
                    && hb.cursor == Cursor::None
                    && matches!(
                        self.engine.object_facts[hb.object as usize].page_type,
                        Some(al_ir::PageType::Document | al_ir::PageType::Card)
                    )
            });
        if !host_shows_no_row && self.nav_find(st, which)? {
            // A part linked by a field that shows rows of a host that is a new, unsaved record: BC inserts the host at Close although
            // nothing was typed (PartLineBeforeHostKey.al:838 PYB_OpenNew_LinkedPart_BlankKeyRowsExist `close =
            // [HP.OnInsertRecord: | H.OnInsert:]`). Not modelled: refused.
            let new_host_link = st
                .borrow()
                .host
                .as_ref()
                .and_then(|(h, c)| h.upgrade().map(|h| (h, *c)))
                .filter(|(h, c)| {
                    let hb = h.borrow();
                    hb.draft
                        && !hb.draft_stored
                        && page_of(&self.engine(), h).controls[*c]
                            .part_link
                            .as_ref()
                            .is_some_and(|l| l.entries.iter().any(|e| matches!(e, ir::LinkEntry::Field(..))))
                });
            // The host is inserted when a part field is read or the part's First runs, not at Close (PartLineBeforeHostKey.al
            // QH_BlankRows_*; :838 reads a part field before Close): `first_part_access`. What else the host and its part may do
            // in this state is refused (`refuse_new_host_part_rows`).
            if let Some((h, _)) = new_host_link {
                // A Card host is pinned for Close only (QH_BlankRows_CardHost); `new_host_on_part_access` refuses its part calls.
                if h.borrow().host.is_some()
                    || !matches!(
                        self.engine.object_facts[h.borrow().object as usize].page_type,
                        Some(al_ir::PageType::Document | al_ir::PageType::Card)
                    )
                {
                    return unsupported(
                        "a linked part that shows rows while its host is a new, unsaved record of a page other than a Document or Card page (not pinned)",
                    );
                }
                h.borrow_mut().new_host_part_rows = true;
            }
            st.borrow_mut().cursor = Cursor::Row;
            st.borrow_mut().landing_first = true;
            let r = self.arrive(st);
            st.borrow_mut().landing_first = false;
            // An error of the rows a modal page reads when it opens comes out with the code of the client's abort (misc/
            // lt-probes-needed-7/PnGPageClose.al:TeardownInHandlerReportsForNonLookupRunModal `af236`, `af260`, `af285`).
            let r = match r {
                Err(Signal::Error(mut e)) if st.borrow().modal && e.code == "Dialog" => {
                    e.code = "ClientPageAbort:FormAbort".into();
                    Err(Signal::Error(e))
                }
                other => other,
            };
            r?;
            // The window starts at the first row of the view.
            let rec = self.page_rec(st);
            let landed = self.row_key(&rec)?;
            let from_start =
                self.is_list(st) && (!handed || first_key.as_ref().is_some_and(|k| Self::same_key(k, &landed)));
            st.borrow_mut().window_from_start = from_start;
            Ok(())
        } else if st.borrow().new_line {
            if st.borrow().host.is_some() {
                self.land_on_part_open_line(st)?;
            } else {
                self.land_on_new_line(st)?;
            }
            // A list with no row but the blank line reads it twice, as a list of one row reads its row (misc/lt-probes-needed-3/
            // PnC07TestPage.al:New_OnEmptyPage `OnNewRecord` three times with the New() call, misc/lt-probes-needed-7/
            // PnGNewRecord.al:HandedRecordKeyFieldFilter `OnNewRecord;AGCR(FILTERED,FILTERED)` twice).
            // xRec is blank again at the second OnNewRecord (misc/lt-probes-needed-7/PnGRefresh.al:792 QL_Jnl_SheetFG2_OpenOnNoRow
            // `P.NEW(T1/B1/0 x=//0 below=Yes);P.AGCR(T1/B1/0 x=T1/B1/0)` twice).
            if self.is_list(st) && st.borrow().host.is_none() {
                let table = self.page_rec(st).borrow().table as usize;
                let blank = self.initial_fields(table);
                self.set_xrec_fields(st, blank.clone());
                self.land_on_new_line(st)?;
                let stamped = !Self::same_values(&self.page_rec(st).borrow().fields, &blank);
                st.borrow_mut().split_starts_low = stamped;
            }
            self.populate_expressions(st)
        } else if self.runs_on_blank_record(st) {
            // A Card, a Document or a ListPlus page without a repeater, opened for edit over an empty view, runs OnNewRecord and
            // OnAfterGetCurrRecord on a blank record after OnOpenPage (no OnAfterGetRecord), and shows no row afterwards: the controls
            // read blank and are not editable (pages/lt-page-types-empty-table/PageTypesEmptyTable.al:Card_OpenEdit_EmptyTable,
            // Document_OpenEdit_EmptyTable, ListPlus_OpenEdit_EmptyTable; the same pages opened with OpenView run OnOpenPage only,
            // *_OpenView_EmptyTable).
            self.land_on_new_line(st)?;
            {
                let mut s = st.borrow_mut();
                s.cursor = Cursor::None;
                s.draft = false;
                s.blank_on_open = true;
            }
            self.populate_expressions(st)
        } else {
            // A list part that shows no row of a host opened for view runs its OnNewRecord once, the link value set without
            // validation, and then shows no row (pages/lt-part-line-before-host-key/PartLineBeforeHostKey.al:883
            // PYB_OpenView_ExistingHost `[LP.OnNewRecord:D6]`). What the part's fields read afterwards is kept as before (blank).
            // Only a host that shows a stored row is pinned.
            let host_on_row = st
                .borrow()
                .host
                .as_ref()
                .and_then(|(h, _)| h.upgrade())
                .is_some_and(|h| h.borrow().cursor == Cursor::Row && !h.borrow().draft);
            let view_part = self.has_source(st)
                && host_on_row
                && st.borrow().mode == PageMode::View
                && page_of(&self.engine(), st)
                    .controls
                    .iter()
                    .any(|c| c.kind == ir::ControlKind::Repeater);
            // A host over no row: the part runs OnNewRecord once too, with the blank link value and no validation, and
            // OnAfterGetCurrRecord once; the host runs neither (PartLineBeforeHostKey.al:1503 QH_OpenView_NoRow_Document
            // `[LP.OnNewRecord:]`, :1530 _TraceHost `[TH.OnOpenPage | CP.OnOpenPage | CP.OnNewRecord: below=Yes |
            // CP.OnAfterGetCurrRecord:/0]`, :1562 _UnlinkedPart, :1602 QH_OpenView_RowFilteredAway after the host's filter
            // left no row). Pinned for a Document or Card host (`host_shows_no_row`); another host is refused.
            let view_part_no_host_row = self.has_source(st)
                && !host_on_row
                && !host_shows_no_row
                && st.borrow().host.is_some()
                && st.borrow().mode == PageMode::View
                && page_of(&self.engine(), st)
                    .controls
                    .iter()
                    .any(|c| c.kind == ir::ControlKind::Repeater)
                && (self.declares(st, "onnewrecord")
                    || self
                        .engine()
                        .events
                        .is_subscribed(st.borrow().object as usize, "onnewrecordevent"));
            if view_part_no_host_row {
                return unsupported(
                    "OnNewRecord of a list part whose host was opened for view on no row (whether BC runs it is not pinned)",
                );
            }
            if view_part || (host_shows_no_row && self.has_source(st) && st.borrow().mode == PageMode::View) {
                let rec = self.page_rec(st);
                let table = rec.borrow().table as usize;
                let fresh = self.initial_fields(table);
                rec.borrow_mut().fields = fresh.clone();
                self.stamp_from_filters(st, false)?;
                self.page_trigger(st, "onnewrecord", vec![Value::Boolean(true)])?;
                self.page_event(st, "onnewrecordevent", None, Some(true))?;
                rec.borrow_mut().fields = fresh;
            }
            self.populate_expressions(st)
        }
    }

    /// A top-level Card, Document or ListPlus page with a source table, no repeater, opened for edit.
    pub(super) fn runs_on_blank_record(&self, st: &St) -> bool {
        let object = st.borrow().object as usize;
        let card = matches!(
            self.engine.object_facts[object].page_type,
            Some(al_ir::PageType::Card | al_ir::PageType::Document | al_ir::PageType::ListPlus)
        ) && !self.is_list(st);
        (card || self.empty_worksheet_without_insert(st))
            && st.borrow().mode == PageMode::Edit
            && st.borrow().host.is_none()
            && self.has_source(st)
    }

    /// A Worksheet with `InsertAllowed = false`, opened for edit over an empty view, runs OnNewRecord and OnAfterGetCurrRecord
    /// once on a blank record, as a Card does, and shows no row (pages/lt-y50-update-from-new-line/Y50UpdateFromNewLine.al:628
    /// G520_NoNewLine_EmptyView_ToBatchWithRows `open` = `/:N:B3/0 below=Yes;C:B3/0`). A List, a tree and a repeater that says
    /// Editable are not pinned: they open as before (Tests-SCM-Warehouse SCM Warehouse VII
    /// PickWithUnreservedQuantityWithAlwaysCreatePickLine opens such a page).
    fn empty_worksheet_without_insert(&self, st: &St) -> bool {
        let object = st.borrow().object as usize;
        matches!(
            self.engine.object_facts[object].page_type,
            Some(al_ir::PageType::Worksheet)
        ) && self.tree_indentation(st).is_none()
            && !self.repeater_says_editable(st)
            && flag_denied(&self.engine(), object, "InsertAllowed").unwrap_or(false)
    }

    pub(in crate::pages) fn land_on_new_line(&mut self, st: &St) -> Result<(), Signal> {
        // The corpus pins the blank line's absence only for `Editable = false` on the page (handlers/TestPageNewRowLine_Tests.al
        // RO arm). Whether a repeater that says Editable (false, or an expression) shows it is not pinned: Tests-ERM 139846
        // ShouldOpenJobQueueEntriesForCreatedJobQueuesIfOptionSelected walks Job Queue Entries, whose repeater says
        // `Editable = false`, and expects no blank line, so landing on it is refused rather than run.
        if self.repeater_says_editable(st) {
            return unsupported(
                "the blank new-row line of a list whose repeater says Editable (whether it shows is not pinned)",
            );
        }
        st.borrow_mut().blank_after_empty_last = false;
        let rec = self.page_rec(st);
        let table = rec.borrow().table as usize;
        self.note_lower_key(st)?;
        let fresh = self.initial_fields(table);
        rec.borrow_mut().fields = fresh;
        {
            let mut s = st.borrow_mut();
            s.placeholder_line = true;
            s.split_burned = false;
            s.cursor = Cursor::NewLine;
            s.draft = true;
            s.typed_before_key = false;
        }
        let held = self.page_xrec_fields(st);
        let blank_held = if st.borrow().host.is_some() {
            self.blank_held_xrec(st, &held)
        } else {
            None
        };
        // The blank line of a part is a started record: the client runs NewRecord when it becomes
        // current (corpus handlers/TestPagePartDraftLineLink.al:LinkedPart_DraftLine_*,
        // handlers/TestPagePartOnNewRecordCount.al). A top-level list does the same, and both then
        // run OnAfterGetCurrRecord on the blank row (corpus testpart/TestPartTriggerOrderAndRefresh.al:
        // 258-273 TopLevelList_BlankLine_RunsOnNewRecord, Part_BlankLine_RunsOnNewRecord).
        // A part whose host record is not saved yet links to a blank value, which is validated like any other
        // (pages/lt-part-line-before-host-key/PartLineBeforeHostKey.al:398, :400; see `sync_part`).
        match blank_held {
            Some(before) => self.stamp_from_filters_seen_as(st, before)?,
            None => self.stamp_from_filters(st, true)?,
        }
        self.page_trigger(st, "onnewrecord", vec![Value::Boolean(true)])?;
        self.page_event(st, "onnewrecordevent", None, Some(true))?;
        // xRec is the new record itself there (misc/lt-probes-needed-7/PnGNewRecord.al:HandedRecordNoFilters `AGCR(,)`,
        // PnGHandedRecord `AGCR(FILTERED,FILTERED)`).
        self.set_xrec(st);
        self.page_trigger(st, "onaftergetcurrrecord", vec![])?;
        self.page_event(st, "onaftergetcurrrecordevent", None, None)?;
        Ok(())
    }
}
