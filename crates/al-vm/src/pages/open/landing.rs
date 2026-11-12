use super::*;
use al_builtins::record_op::RecordOp;

impl Session {
    /// Whether the page carries the blank line past its data: an editable,
    /// insert-allowed repeater opened with OpenEdit (corpus
    /// testpage/TestPageNewRowLine_Tests.al:1-30, gating arms).
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
        // A tree shows no blank line, opened by Page.Run, Page.RunModal or a Page variable (testpage/AhPageTests.al:986,
        // :1006, :1026, :1044 `rows_initial = T1 T5`).
        let tree = self.tree_indentation(st).is_some();
        // A Worksheet over a temporary source carries it opened with OpenView too: Next from its last row answers true on the
        // blank line (pages/onopenpage-moves-rec/QrMovedTests.al QR_TempWs_OpenView_NoMove `next` `Yes|`, `next log`
        // `C:;`, _FindLast, _RangeFindLast, _GetOutsideRange, QR_TempWsNext_OpenView_NoMove, writer Y57).
        let temp_worksheet_view = st.borrow().mode == PageMode::View
            && engine.object_facts[object].page_type == Some(al_ir::PageType::Worksheet)
            && self.page_rec(st).borrow().temporary;
        let offered = has_repeater
            && !tree
            && (st.borrow().mode == PageMode::Edit || temp_worksheet_view)
            && !self.lookup_list(st)
            && !prop_false("Editable")
            && !flag_denied(&engine, object, "InsertAllowed")?;
        drop(engine);
        // A repeater whose Editable is false shows no line (`repeater_line`).
        Ok(offered && self.repeater_line(st)? != RepeaterLine::Denied)
    }

    /// Whether `st` is a List page in lookup mode. Such a list has no new-row line, pinned for each kind of lookup:
    /// `LookupMode(true)` on a page variable (ui-handlers/TestPageModalNewRowLine_Tests.al:209
    /// Plain_LookupModeTrue_HasNoNewRowLine), the static `Page.RunModal(Id, Rec)` whose return value is used (called as a
    /// statement it is no lookup and keeps the line: :180; pages/triggers/BeRunModalOnLookup.al:616, :916) and the page a
    /// TableRelation lookup opens (testpage/P133RelationLookups.al:271-284: Next from its last row answers false). A page
    /// in lookup mode always has one of the three kinds (`PageState::open_as_lookup`, `set_lookup_mode`): the list of
    /// kinds says what is pinned, it filters nothing today.
    pub(in crate::pages) fn lookup_list(&self, st: &St) -> bool {
        let (on, kind) = {
            let s = st.borrow();
            (s.lookup, s.lookup_kind)
        };
        on && matches!(
            kind,
            LookupKind::Variable | LookupKind::StaticRunModal | LookupKind::Relation
        ) && self.is_list_page(st)
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

    /// Whether the page or one of its page extensions declares trigger `name` (lower case). The extension's trigger runs
    /// after the page's (`page_trigger`), so an open that asks whether OnOpenPage ran code counts both.
    pub(in crate::pages) fn declares_or_ext_declares(&self, st: &St, name: &str) -> bool {
        let engine = self.engine();
        let page = page_of(&engine, st);
        page.triggers.iter().any(|(n, _)| n == name) || page.ext_triggers.iter().any(|(n, _, _)| n == name)
    }

    /// The page has no page extension and no event subscriber: nothing outside its own code observes or changes its triggers.
    pub(in crate::pages) fn unextended_unsubscribed(&self, st: &St) -> bool {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        page_of(&engine, st).extensions.is_empty() && !engine.events.any_subscriber_of(object)
    }

    /// Whether the page declares trigger `name` (lower case) of its own.
    pub(in crate::pages) fn declares(&self, st: &St, name: &str) -> bool {
        let engine = self.engine();
        page_of(&engine, st).triggers.iter().any(|(n, _)| n == name)
    }

    /// FindFirst (`which` "-") or FindLast ("+") of the page's rowset. A page that declares
    /// OnFindRecord serves its rows from the trigger, which receives `Which` and answers whether it
    /// found a row (corpus pages/triggers/TestPageFindRecord.al:112, :160). The strings the client
    /// really passes are `=`, `=>` and `=><` forms the corpus pins only for a deleted row
    /// (pages/currpage/TestPageUpdateGoneRow.al:390); `-` and `+` are assumed (docs/probes-needed.md).
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
    /// many rows it moved (corpus pages/triggers/TestPageFindRecord.al:129).
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
    /// an empty view too (corpus format-evaluate/autoformat/TestAutoFormatExpressionTiming.al:5-12).
    /// Whether `rec` stands on the first row of its view (found on a copy, so `rec` does not move).
    pub(in crate::pages) fn on_first_row_of_view(&mut self, rec: &Rec) -> Result<bool, Signal> {
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
        // pages/extensions/PCCurrRecordReads.al:FindList_OnFindRecordAndOnNextRecord, `open`).
        // A SourceTableView that sorts descending asks the same (pages/run-modal/CePageRun.al
        // OpenView_DescendingWithOnFindRecord_CurrentRow `find(=><)=1`).
        let which = if self.declares(st, "onfindrecord") {
            // A view that filters asks the same `=><` once, list, card and list part alike
            // (pages/triggers/QcFindViewTests.al:78 ViewList_OpenView_FindStringAndRow, :361 ViewCard_OpenView_FindStringAndRow,
            // :505 DocPart_OpenEdit_LinesOfH1).
            // A list page variable run in lookup mode (LookupMode(true)) with no record is asked `=>` (pages/list-order/
            // QgGoToKeyNextCall.al:629 Nav_LookupHandler_GoToKey_ThenOK `log` `F:=>=Yes;R:R1001;N:1=1;...`). So is a list the
            // static Page.RunModal(0 or Id, Rec) runs in lookup mode, positioned or not (corpus branch al-runtime-rs-staging-r6962,
            // pages/run-modal/StaticRunModalFindRecordLookup.al StaticRunModalZero_FilteredNotPositioned_FirstThenOK `F:=>=Yes;R:V2;...`,
            // _FilteredOnRow_, _NoFilterNotPositioned_, StaticRunModalPageId_; issue 6992). What the list of a TableRelation
            // lookup is asked nothing pins: that lookup is refused before the page opens
            // (`refuse_relation_lookup_onto_find_list`), so the `=><` below is never the answer for that kind.
            let (lookup, asks_forward) = {
                let s = st.borrow();
                (
                    s.lookup,
                    matches!(s.lookup_kind, LookupKind::Variable | LookupKind::StaticRunModal),
                )
            };
            if std::mem::take(&mut st.borrow_mut().part_refind_forward) {
                // A part's read again after its own CurrPage.Update(true) (`part_refresh_find_forward.rs`, al-runner issue 7820).
                "=>"
            } else if lookup && asks_forward && self.is_list(st) {
                "=>"
            } else if !lookup && self.is_list(st) && self.onopenpage_left_rec_past_the_first_row(st)? {
                // A list whose OnOpenPage left Rec on a row that is not the first of its view is asked `=>`
                // (pages/list-order/AiListOrderTests.al:851 OpenOnRow_SetPositionInOnOpenPage_LastRow_OpenLog
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
        {
            let mut s = st.borrow_mut();
            s.find_window_at_view_start = false;
            // The page reads its view again: the link values an earlier landing put on Rec are gone (`empty_part_rec`).
            s.empty_list_stamp = None;
        }
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
        // (pages/parts/PartLineBeforeHostKey.al:1797 QH_OpenView_NoRow_BlankKeyLinesExist
        // `part_description=[]`). So does the part of such a host opened for edit over an empty table, which shows no row
        // (`runs_on_blank_record`): :2720 R6920_DocAfterGet_BlankRows_OpenEdit_VisibleFirst, :3578
        // R6920_CardOpenPage_BlankRows_OpenEdit_EditableFirst and the other R6920_*_BlankRows_OpenEdit_* tests `description=[]`,
        // `open=[] fired at least once: LP.OnNewRecord` as without rows (:2727, :2733).
        let engine = self.engine();
        let part_temporary = self.page_rec(st).borrow().temporary;
        let host_shows_no_row_as = |view: bool| {
            st.borrow()
                .host
                .as_ref()
                .and_then(|(h, c)| h.upgrade().map(|h| (h, *c)))
                .is_some_and(|(h, c)| {
                    let hb = h.borrow();
                    let mode_ok = if view {
                        hb.mode == PageMode::View
                    } else {
                        // Pinned for a part linked by a field to the host's table, over a table of its own (the R6920 pages);
                        // an unlinked or temporary part of such a host keeps what it was handed, as before.
                        hb.blank_on_open
                            && !part_temporary
                            && page_of(&engine, &h).controls[c]
                                .part_link
                                .as_ref()
                                .is_some_and(|l| l.entries.iter().any(|e| matches!(e, ir::LinkEntry::Field(..))))
                    };
                    hb.host.is_none()
                        && mode_ok
                        && hb.cursor == Cursor::None
                        && match self.engine.object_facts[hb.object as usize].page_type {
                            Some(al_ir::PageType::Document | al_ir::PageType::Card) => true,
                            // Opened for view, a ListPlus host behaves the same (pages/parts/PartListPlusHostNoRow.al:288-295
                            // _PartTriggers, _PartInGroup, :314 _PartFirst `No`, :336 _HiddenPart; issue 6953).
                            Some(al_ir::PageType::ListPlus) => view,
                            _ => false,
                        }
                })
        };
        let edit_host_no_row = host_shows_no_row_as(false);
        let host_shows_no_row = host_shows_no_row_as(true) || edit_host_no_row;
        if !host_shows_no_row && self.nav_find(st, which)? {
            // A part linked by a field that shows rows of a host that is a new, unsaved record: BC inserts the host at Close although
            // nothing was typed (PartLineBeforeHostKey.al:1057 PYB_OpenNew_LinkedPart_BlankKeyRowsExist `close =
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
            // xRec is blank again at the second OnNewRecord (testpage/PnGRefresh.al:792 QL_Jnl_SheetFG2_OpenOnNoRow
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
        } else if self.runs_on_blank_record(st) || self.list_without_line_on_empty_view(st)? {
            // A Card, a Document or a ListPlus page without a repeater, opened for edit over an empty view, runs OnNewRecord and
            // OnAfterGetCurrRecord on a blank record after OnOpenPage (no OnAfterGetRecord), and shows no row afterwards: the controls
            // read blank and are not editable (pages/properties/PageTypesEmptyTable.al:Card_OpenEdit_EmptyTable,
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
        } else if self.empty_temp_navigate_page_by_set_record(st) {
            let rec = self.page_rec(st);
            let blank = self.initial_fields(rec.borrow().table as usize);
            rec.borrow_mut().fields = blank;
            self.populate_expressions(st)
        } else {
            // A list part that shows no row of a host opened for view runs its OnNewRecord once, the link value set without
            // validation, and then shows no row (pages/parts/PartLineBeforeHostKey.al:1102
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
            // OnAfterGetCurrRecord once; the host runs neither (PartLineBeforeHostKey.al:1722 QH_OpenView_NoRow_Document
            // `[LP.OnNewRecord:]`, :1530 _TraceHost `[TH.OnOpenPage | CP.OnOpenPage | CP.OnNewRecord: below=Yes |
            // CP.OnAfterGetCurrRecord:/0]`, :1562 _UnlinkedPart, :1602 QH_OpenView_RowFilteredAway after the host's filter
            // left no row). Pinned for a Document or Card host, and measured for a ListPlus host (`host_shows_no_row`); another
            // host is refused.
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
                rec.borrow_mut().fields = fresh;
                self.stamp_from_filters(st, false)?;
                self.page_trigger(st, "onnewrecord", vec![Value::Boolean(true)])?;
                self.page_event(st, "onnewrecordevent", None, Some(true))?;
                // The part then runs OnAfterGetCurrRecord and its event subscribers once on the blank row, after OnNewRecord and
                // with the link value in place (PartLineBeforeHostKey.al:1813 QH_OpenView_NoRow_TraceHost `CP.OnNewRecord: below=Yes |
                // CP.OnAfterGetCurrRecord:/0`, :1610 _CurrRecordOnlyPart `OP.OnAfterGetCurrRecord:/0`, :1642 _BlankKeyLinesExist,
                // :1666 _RowFilteredAway `CP.OnNewRecord:D6 below=Yes | CP.OnAfterGetCurrRecord:D6/0` on a host that stands on a row).
                self.populate_expressions(st)?;
                self.page_trigger(st, "onaftergetcurrrecord", vec![])?;
                self.page_event(st, "onaftergetcurrrecordevent", None, None)?;
                // Rec then holds the link values for the part's later code (`empty_part_rec`).
                self.empty_part_rec(st)?;
            } else if self.card_part_of_new_host(st) {
                self.card_part_blank_on_new_host(st)?;
            } else if self.has_source(st) && st.borrow().host.is_some() && st.borrow().mode == PageMode::Edit {
                // A part of a host that moved to a row without lines holds no row of the old link: Rec is blank, so the part's
                // action sees no Line No. (pages/parts/R6889RealShape.al:1213 RunStep4, :1406 PG6889RS_Step4_ `ACT;CU;T.INS(10);...`,
                // a host filtered to D2|D3 and moved there by GoToRecord from D1, whose part held D1's lines). A ListPart of a
                // Card or Document host keeps the values its link pins (`empty_part_rec`).
                self.empty_part_rec(st)?;
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
    /// once on a blank record, as a Card does, and shows no row (pages/triggers/Y50UpdateFromNewLine.al:628
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
        // A repeater whose Editable is false or true by its value is pinned (`repeater_line`; New() on a repeater that says
        // false lands on a blank row and raises nothing, pages/readonly-repeater/Y65ReadOnlyRepeater.al:255 `new=ok:`).
        // Another expression is not pinned: Tests-ERM 139846 ShouldOpenJobQueueEntriesForCreatedJobQueuesIfOptionSelected walks
        // a list that must show no blank line, so landing on it is refused rather than run.
        if self.repeater_line(st)? == RepeaterLine::Unknown {
            return unsupported(
                "the blank new-row line of a list whose repeater's Editable is an expression other than page variables (whether it shows is not pinned)",
            );
        }
        // A new line is not a row `start_draft` started: no typed key is told from it (`PageState::draft_start`).
        st.borrow_mut().draft_start = None;
        st.borrow_mut().draft_from_open = false;
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
        // current (corpus testpage/TestPagePartDraftLineLink.al:LinkedPart_DraftLine_*,
        // testpage/TestPagePartOnNewRecordCount.al). A top-level list does the same, and both then
        // run OnAfterGetCurrRecord on the blank row (corpus testpage/parts/TestPartTriggerOrderAndRefresh.al:
        // 258-273 TopLevelList_BlankLine_RunsOnNewRecord, Part_BlankLine_RunsOnNewRecord).
        // A part whose host record is not saved yet links to a blank value, which is validated like any other
        // (pages/parts/PartLineBeforeHostKey.al:398, :400; see `sync_part`).
        match blank_held {
            Some(before) => self.stamp_from_filters_seen_as(st, before)?,
            None => self.stamp_from_filters(st, true)?,
        }
        self.page_trigger(st, "onnewrecord", vec![Value::Boolean(true)])?;
        self.page_event(st, "onnewrecordevent", None, Some(true))?;
        // xRec is the new record itself there (testpage/PnGNewRecord.al:HandedRecordNoFilters `AGCR(,)`,
        // PnGHandedRecord `AGCR(FILTERED,FILTERED)`).
        self.set_xrec(st);
        self.page_trigger(st, "onaftergetcurrrecord", vec![])?;
        self.page_event(st, "onaftergetcurrrecordevent", None, None)?;
        Ok(())
    }
}

impl Session {
    /// The page is a Worksheet over a temporary source.
    pub(in crate::pages) fn is_temp_worksheet(&self, st: &St) -> bool {
        self.page_rec(st).borrow().temporary
            && matches!(
                self.engine.object_facts[st.borrow().object as usize].page_type,
                Some(al_ir::PageType::Worksheet)
            )
    }
}
