use super::*;
use al_builtins::record_op::RecordOp;

/// Key index, ascending, filters (as text) and primary key of a temporary page's Rec (`temp_rec_position`).
type TempRecPosition = (usize, bool, String, Vec<Value>);
impl Session {
    pub(in crate::pages) fn page_open(&mut self, st: &St, mode: PageMode) -> Result<Value, Signal> {
        // corpus testpageclose/TestPageClosingActions.al:19-24: an open variable cannot be opened.
        if st.borrow().open {
            return Err(error(ALREADY_OPEN));
        }
        // OpenNew of a Worksheet fails with NotSupportedPageViewMode (corpus pageext/lt-bl-implicit-creation/
        // ImplicitCreation.al:308-318 Worksheet_OpenNew), before OnInit or OnOpenPage run (Y8OpenNewPageTypes.al:306-317,
        // `log=` empty for a worksheet that declares both).
        if mode == PageMode::New {
            let object = st.borrow().object as usize;
            let engine = self.engine();
            if engine.object_facts[object].page_type == Some(al_ir::PageType::Worksheet) {
                let Some(id) = engine.program.objects[object].id else {
                    return unsupported("OpenNew of a Worksheet without an id (the error text is not pinned)");
                };
                return Err(error_code(
                    format!("The view mode 'Create' is not supported by the page ID '{id}'."),
                    "NotSupportedPageViewMode",
                ));
            }
            if matches!(&engine.object_facts[object].page_type, Some(al_ir::PageType::Other(t)) if t == "api") {
                return Err(self.api_open_new_denied(st)?);
            }
        }
        // A reopened variable shows a fresh page.
        st.borrow_mut().instance = None;
        st.borrow_mut().inited = false;
        st.borrow_mut().modal = false;
        self.pages.test_open = true;
        self.open_page(st, mode, None, None).map(|_| Value::Null)
    }

    /// Whether the page has a source table (and so a `Rec`).
    pub(in crate::pages) fn has_source(&self, st: &St) -> bool {
        let object = st.borrow().object as usize;
        self.engine.program.objects[object].rec_slot.is_some()
    }

    /// The page instance of `st`, created on first use (a `Page` variable has one before it runs).
    pub(crate) fn page_instance(&mut self, st: &St) -> Result<Rc<RefCell<al_builtins::value::Instance>>, Signal> {
        self.note_page(st.borrow().object as usize);
        if let Some(i) = st.borrow().instance.clone() {
            return Ok(i);
        }
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let page = page_of(&engine, st);
        if let Some(why) = page.unsupported.first() {
            return unsupported(why.clone());
        }
        if let Some(why) = page.big_integer_expressions.first() {
            if !self.has(al_version::Gate::BigIntegerLiteralInControlExpressionFrom28) {
                return unsupported(why.clone());
            }
        }
        let inst = self.instance(object);
        inst.borrow_mut().globals[page.curr_page_slot] = Value::TestPage(st.clone());
        // Each extension runs on an instance of its own that shares the page's Rec, xRec and CurrPage.
        let mut exts = Vec::new();
        for &e in &page.extensions {
            let ext = self.extension_instance(e, &inst);
            let (page_object, ext_object) = (&engine.program.objects[object], &engine.program.objects[e]);
            for (from, to) in [
                (page_object.rec_slot, ext_object.rec_slot),
                (page_object.xrec_slot, ext_object.xrec_slot),
            ] {
                if let (Some(from), Some(to)) = (from, to) {
                    let shared = inst.borrow().globals[from].clone();
                    ext.borrow_mut().globals[to] = shared;
                }
            }
            if let Some(slot) = ext_object.globals.iter().position(|g| g.name == "CurrPage") {
                ext.borrow_mut().globals[slot] = Value::TestPage(st.clone());
            }
            exts.push((e, ext));
        }
        st.borrow_mut().ext_instances = exts;
        st.borrow_mut().instance = Some(inst.clone());
        self.seed_unopened_rec(st);
        Ok(inst)
    }

    /// The Rec of a Page variable that never ran holds what SetRecord and SetTableView handed it: its own procedures read the
    /// row SetRecord gave (pages/lt-bl-unopened-currpage/UnopenedCurrPage.al:214 SetRecord_Then_Update_List `[2] []`) and the
    /// filters SetTableView gave, on the blank row (:233 SetTableView_Then_Update_Card `[0] [Id: 2]`).
    pub(in crate::pages) fn seed_unopened_rec(&mut self, st: &St) {
        if !self.never_ran_page_var(st) {
            return;
        }
        let (Some(inst), Some(given), view) = ({
            let s = st.borrow();
            (s.instance.clone(), s.set_record.clone(), s.set_view)
        }) else {
            return;
        };
        let object = st.borrow().object as usize;
        let Some(slot) = self.engine.program.objects[object].rec_slot else {
            return;
        };
        let Value::Record(rec) = inst.borrow().globals[slot].clone() else {
            return;
        };
        if rec.borrow().table != given.borrow().table {
            return;
        }
        let given = given.borrow();
        let mut rec = rec.borrow_mut();
        if view {
            rec.filters = given.filters.clone();
        } else {
            rec.fields = given.fields.clone();
        }
    }

    /// A call on a `Page` variable constructs the page on first use: its instance, and OnInit once
    /// (handlers/TestPageOnInit_Tests.al:188 POI_SetterBeforeRunModal_RunsAfterOnInitAndSurvives,
    /// handlers/TestPageEnumGlobal*: a getter on a variable that never ran reads the declared default).
    pub(crate) fn construct_page(&mut self, st: &St) -> Result<Rc<RefCell<al_builtins::value::Instance>>, Signal> {
        let inst = self.page_instance(st)?;
        if st.borrow().host.is_none() && !st.borrow().inited {
            st.borrow_mut().inited = true;
            // OnInit reads the page's own Editable, not what the variable was set to before (handlers/PWPageVariableEditable.al:221-240;
            // misc/lt-probes-needed-4/PnD13_PageEditableViewMode.al:197 ViewModePage_GivenEditableTrue `onInit = No` with SetRecord
            // after the Editable call).
            let preset = st.borrow_mut().editable_preset.take();
            let r = self.page_trigger(st, "oninit", vec![]);
            let mut s = st.borrow_mut();
            if s.editable_preset.is_none() {
                s.editable_preset = preset;
            }
            drop(s);
            r?;
        }
        Ok(inst)
    }

    /// `to` takes over the page `from` constructed, if it did, else starts without one.
    pub(in crate::pages) fn adopt_instance(&mut self, from: &St, to: &St) {
        if Rc::ptr_eq(from, to) {
            to.borrow_mut().instance = None;
            to.borrow_mut().inited = false;
            return;
        }
        let (instance, exts, inited) = {
            let mut f = from.borrow_mut();
            (
                f.instance.take(),
                std::mem::take(&mut f.ext_instances),
                std::mem::take(&mut f.inited),
            )
        };
        let engine = self.engine();
        if let Some(inst) = &instance {
            let page = page_of(&engine, to);
            inst.borrow_mut().globals[page.curr_page_slot] = Value::TestPage(to.clone());
            for (e, ext) in &exts {
                let obj = &engine.program.objects[*e];
                if let Some(slot) = obj.globals.iter().position(|g| g.name == "CurrPage") {
                    ext.borrow_mut().globals[slot] = Value::TestPage(to.clone());
                }
            }
        }
        // The part pages the variable's code reached before it ran go with the page (Warehouse Pick Summary.SetRecords hands rows to its
        // part, then Run opens the page the test trapped).
        let early = std::mem::take(&mut from.borrow_mut().hidden_parts);
        for (_, child) in &early {
            let host = child.borrow().host.as_ref().map(|(_, i)| (Rc::downgrade(to), *i));
            child.borrow_mut().host = host;
        }
        let mut t = to.borrow_mut();
        t.instance = instance;
        t.ext_instances = exts;
        t.inited = inited;
        t.hidden_parts = early;
    }

    /// Opens the page: OnInit, OnOpenPage, the control expressions and the first row. `record`
    /// is the record AL code handed to `Page.Run`/`SetRecord`: its filters and position become the
    /// page's view.
    ///
    /// With `bind`, the UI handler for the page is looked up after OnInit and before OnOpenPage
    /// (corpus handlers/TestPageOnInit_Tests.al: an OnInit error surfaces as itself even where no
    /// handler is declared).
    pub(in crate::pages) fn open_page(
        &mut self,
        st: &St,
        mode: PageMode,
        record: Option<Rec>,
        bind: Option<UiKind>,
    ) -> Result<Option<Handler>, Signal> {
        self.open_page_as(st, mode, record, bind, false)
    }

    /// `open_page`, or with `as_part` the opening of a part's page: everything up to and including
    /// the page's OnOpenPage, leaving its rows to the host (`sync_part`), which knows the link.
    pub(in crate::pages) fn open_page_as(
        &mut self,
        st: &St,
        mode: PageMode,
        record: Option<Rec>,
        bind: Option<UiKind>,
        as_part: bool,
    ) -> Result<Option<Handler>, Signal> {
        let test_open = std::mem::take(&mut self.pages.test_open);
        let rec_unpinned = st.borrow_mut().open_rec_unpinned.take();
        st.borrow_mut().run_page_link_opened = rec_unpinned.is_some();
        st.borrow_mut().run_page_link_open_moved = false;
        let rec_fill = st.borrow_mut().open_rec_fill.take();
        let mut handler = None;
        let engine = self.engine();
        let page = page_of(&engine, st);
        for a in &page.assumes {
            al_builtins::assumptions::assume(*a);
        }
        self.page_instance(st)?;
        let language = al_builtins::format::FormatCtx::language(&*self);
        {
            let mut s = st.borrow_mut();
            let first_run = !s.ran;
            s.open = true;
            s.open_language = Some(language);
            s.ran = true;
            s.mode = mode;
            s.cursor = Cursor::None;
            s.draft = false;
            s.new_row_inserted_by_trigger = false;
            s.test_opened_new = test_open && mode == PageMode::New;
            s.never_stored = false;
            s.dirty = false;
            s.frozen_visible = Vec::new();
            s.control_captions = Vec::new();
            s.field_captions = Vec::new();
            s.field_captions_opened = false;
            s.field_captions_settled = false;
            s.field_captions_row = None;
            s.field_captions_quiet = false;
            s.field_captions_pinned_move = false;
            s.moving_next = false;
            s.window = Vec::new();
            s.window_from_start = false;
            s.window_eof = false;
            s.window_whole = false;
            s.opened_on_moved_rec = false;
            s.onopenpage_kept_rec = false;
            s.temp_reads_once_more = false;
            s.reads_once_more_in = None;
            s.focus = None;
            s.focus_in_row = false;
            s.errors = Vec::new();
            s.trigger_errors = Vec::new();
            s.blank_on_open = false;
            s.refused = false;
            s.pending = None;
            s.result = None;
            s.editable_off = false;
            s.editable_off_in_open = false;
            s.editable_set_later = None;
            s.set_controls = Vec::new();
            s.agr_xrec = None;
            s.agcr_xrec = None;
            s.row_vars = Vec::new();
            s.row_exprs = Vec::new();
            s.row_exprs_live = false;
            s.row_prop_vars = Vec::new();
            s.agcr_after_new = false;
            s.part_accessed = false;
            s.new_host_part_rows = false;
            s.expanded = Vec::new();
            s.tree_shown = Vec::new();
            // A caption assigned to the variable before its first run stays.
            // It stays for a second run too (types/lt-caption-assigned-before-run/CaptionAssignedBeforeRun.al:P187_SecondRun_NoReassign,
            // `OnOpenPage [Assigned One]` twice); a caption assigned while the page ran is dropped.
            if !first_run && !s.caption_assigned_closed {
                s.caption = None;
            }
            s.opening = true;
            s.or_group_dropped = false;
            s.unpinned_filter_fields = Vec::new();
            s.placeholder_line = false;
            s.gone_blank_no_new_line = false;
            s.gone_blank_shows_line = false;
            s.gone_view_list = false;
            s.parts = Vec::new();
            s.closed_parts = Vec::new();
            s.areas = self.application_areas.clone();
        }
        if let Err(e) = self.create_parts(st) {
            self.mark_closed(st);
            return Err(e);
        }
        let has_source = self.has_source(st);
        // SourceTableView is the page's own view of its table (corpus
        // pagefindrecord/TestPageFindRecord.al:192 PlainPage_WithoutTheTriggers_WalksTheTableDescending);
        // how it combines with the view of a record handed to Page.Run is pinned by P145PageRunView.al.
        let source_view = engine.program.objects[st.borrow().object as usize]
            .properties
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("SourceTableView"))
            .map(|(_, v)| crate::views::property_view(v.trim()));
        // A request page's SourceTableView leaves its `Rec` without the view's filter: OnOpenPage reads `VERSION(1)
        // SORTING(DocNo,LineNo)`, no filters and every row (report/lt-y-request-page/YRequestPageTests.al:72-87, a view that sorts by
        // the primary key and filters with a CONST). Another sorting or another kind of filter is not pinned.
        let source_view = match source_view {
            Some(view) if self.is_request_page(st) && has_source => {
                self.request_source_view_pinned(st, &view)?;
                None
            }
            other => other,
        };
        // The part control's SubPageView comes first: its filters land in the link group, its
        // sorting and order set the key and direction. The part page's own SourceTableView then
        // sets only the halves it names (corpus handlers/TestPartOwnSortVsSubPageView.al:1-45).
        let sub_view = self.sub_page_view(st).map(|v| crate::views::sub_page_view_text(&v));
        if let (Some(view), true) = (&sub_view, has_source) {
            let rec = self.page_rec(st);
            let table = self.table(&rec)?;
            self.set_view_in(&rec, &table, view, Some(parts::LINK_GROUP), false, true, false)?;
        }
        // A record handed to Page.Run brings its own filters, key and order; they come first and the page's
        // SourceTableView then adds to them: the record's filters stay in their groups, the view's land in group 2 beside the
        // record's own group 2, the view's SORTING replaces the record's key and the record's direction stays (types/lt-code-digits-page-run-view/
        // P145PageRunView.al:P145_A_RecFilterSameFieldAsView, _RecFilterOverlappingView, _RecFilterOtherField, _RecKeyDescending,
        // _RecKeyPrimary, _RecFilterInGroup1, _RecFilterInGroup2). A view that names an ORDER is an assumption (page-run-record-view-order-applies).
        // A stored record handed to a card whose source table is temporary: BC shows the row (OnAfterGetRecord reads it,
        // pages/lt-be-card-onfindrecord/BeCardOnFindRecord.al:374 CardTemp_PageRun_StoredRec `O:1/stored/;R:1/stored/`), but
        // whether the row is copied into the page's temporary table or read from the database (which decides what the page
        // writes, counts and steps to) is not pinned: refused (docs/probes-needed.md).
        // A record keyed on a row-dependent FlowField: refused (`page_record_refusal`).
        self.page_record_refusal(st, record.as_ref(), has_source)?;
        if let (Some(r), true) = (&record, has_source) {
            let rec = self.page_rec(st);
            let src = r.borrow();
            let mut dst = rec.borrow_mut();
            dst.fields = src.fields.clone();
            dst.filters = src.filters.clone();
            dst.key = src.key;
            dst.adhoc = src.adhoc.clone();
            // The marks and MarkedOnly come along: the page shows the marked rows only, of a real and of a temporary record
            // (misc/lt-protected-var-abs-query-views/P146ViewMarkedTemp.al:304 P146_PageRunModal_MarkedOnly_Real, :362
            // P146_PageRunModal_Temporary). A page variable's SetRecord or SetTableView with marks is refused there (not pinned).
            dst.marks = src.marks.clone();
            dst.marked_only = src.marked_only;
            // A record that is no temporary one (a view only, see SetTableView) leaves a page whose source table is temporary on its own rows.
            if src.temporary || !dst.temporary {
                dst.temporary = src.temporary;
                dst.temp_id = src.temp_id.clone();
            }
            dst.ascending = src.ascending;
        }
        if let (Some(view), true) = (source_view, has_source) {
            // A view that names an ORDER on a page run on a record: the view's direction wins over the record's, whatever
            // the record's key and direction (types/lt-code-digits-page-run-view/P145PageRunView.al:228-467, SF_B_*, SF_C_*).
            // A record an action's RunObject hands it has its own assumption (run-page-link-target-view, runobject.rs).
            let rec = self.page_rec(st);
            let table = self.table(&rec)?;
            // The platform applies the view in filter group 2: the page's own AL reads it with
            // FilterGroup(2) and finds group 0 empty (corpus handlers/TestPageSourceTableView.al:
            // FiltersAreInFilterGroup2WhenOnOpenPageRuns).
            // The filters the handed record carries in group 2 (Bank Acc. Reconciliation.OpenWorksheet sets the statement there) stay: the view
            // adds its own to the group instead of replacing it (P145PageRunView.al:P145_A_RecFilterInGroup2).
            let kept: Vec<_> = if record.is_some() {
                rec.borrow().filters.iter().filter(|f| f.group == 2).cloned().collect()
            } else {
                Vec::new()
            };
            let (view, enum_consts, blank_dates) = match crate::views::page_source_view(&table, &view) {
                Ok(v) => v,
                Err(e) => {
                    self.mark_closed(st);
                    return Err(e);
                }
            };
            // The text a `0D` view filter reads back as is not pinned: a page with code that could read it is refused, and a
            // TestPage Filter read of the field refuses (see `test_filter_op`).
            if !blank_dates.is_empty() {
                if self.page_code_runs(st, true) {
                    self.mark_closed(st);
                    return unsupported(
                        "SourceTableView const(0D) or filter(0D) on a page with code that could read the filter (its text is not pinned)",
                    );
                }
                st.borrow_mut().unpinned_filter_fields = blank_dates;
            }
            self.set_view_in(&rec, &table, &view, Some(2), sub_view.is_some(), false, false)?;
            // A field the view filters itself keeps the view's filter alone (two equal filters of one field read as a doubled value
            // when a new line takes it: Purchase Price List's "Price Type").
            let mut r = rec.borrow_mut();
            // An Enum const is held as a typed value, as SetRange holds one (see `page_source_view`).
            for f in r
                .filters
                .iter_mut()
                .filter(|f| f.group == 2 && enum_consts.contains(&f.field))
            {
                if let al_builtins::filter::FilterExpr::Eq(v) = &f.expr {
                    f.expr = al_builtins::filter::FilterExpr::Range(Some(v.clone()), Some(v.clone()));
                }
            }
            let viewed: Vec<usize> = r.filters.iter().filter(|f| f.group == 2).map(|f| f.field).collect();
            r.filters
                .extend(kept.into_iter().filter(|f| !viewed.contains(&f.field)));
        }
        // A repeater with ShowAsTree over an IndentationColumn that is a field but not an Integer field is not pinned. One over a page
        // variable is refused when the TestPage moves between its rows (`tree_over_variable`). The cut itself is made after
        // OnOpenPage (`apply_tree_cut`).
        if has_source {
            if let Some(expr) = self.tree_indentation(st) {
                let rec = self.page_rec(st);
                let table = self.table(&rec)?;
                let name = expr
                    .trim()
                    .strip_prefix("Rec.")
                    .unwrap_or(expr.trim())
                    .trim_matches('"')
                    .to_string();
                match table.fields.iter().find(|f| f.name.eq_ignore_ascii_case(&name)) {
                    Some(f) if !matches!(f.ty, RtType::Integer) => {
                        self.mark_closed(st);
                        return unsupported(
                            "ShowAsTree over an IndentationColumn that is not an Integer field (not pinned)",
                        );
                    }
                    _ => {}
                }
            }
        }
        let result = (|| {
            // A `Page` variable whose procedure ran before this open was constructed then, and its
            // OnInit has run (handlers/TestPageOnInit_Tests.al:188).
            // A second run of the same variable does not run OnInit again (controls/W2PageVariableInit.al:
            // PageVariable_RunModalTwice, BC 27.5 and 29.0); a reopened TestPage is a fresh page (`page_open`).
            if !std::mem::replace(&mut st.borrow_mut().inited, true) {
                self.page_trigger(st, "oninit", vec![])?;
            }
            // `Editable(false)` on the variable before it runs applies after OnInit: OnInit still
            // reads Yes (corpus handlers/PWPageVariableEditable.al:221-240).
            {
                let mut s = st.borrow_mut();
                s.editable_off = s.editable_preset == Some(false);
                s.editable_off_in_open = false;
                s.editable_set_later = None;
            }
            // The parts' OnInit runs between the host's OnInit and its OnOpenPage (corpus
            // testpart/TestPartTriggerOrderAndRefresh.al:236-247).
            self.init_parts(st)?;
            let mut unattended = false;
            if let Some(kind) = bind {
                if self.pages.lenient_bind {
                    // A RunObject action opens the page, runs OnInit and OnOpenPage, and only then
                    // looks for a handler; with none it goes no further, so the first row is never
                    // read (handlers/TestPageRunObjectContracts.al:330-346: "card.OnInit;card.OnOpenPage;").
                    // A page opened in Create mode reads its new row first (:handlers/TestPageCreateModeNewRecordArgs.al:304-327).
                    let object = st.borrow().object;
                    handler = self.try_bind_page_handler(kind, object)?;
                    unattended = handler.is_none() && mode != PageMode::New;
                } else {
                    handler = Some(self.bind_page_handler(kind, st.borrow().object)?);
                }
            }
            // The parameters string of `Report.RunRequestPage(Id, Parameters)`, after OnInit (see `run_request_page`).
            let seed = st.borrow_mut().request_seed.take();
            if let Some(xml) = seed {
                let (inst, exts, object) = {
                    let s = st.borrow();
                    (
                        s.instance.clone().expect("an open request page"),
                        s.ext_instances.clone(),
                        s.object as usize,
                    )
                };
                self.refuse_seeded_view_after_init(&inst, object, &xml)?;
                self.apply_request_parameters(&inst, &exts, object, &xml, false)?;
            }
            self.restore_page_values(st);
            self.check_page_events(st)?;
            // A temporary source: Rec as it stands before OnOpenPage, to tell a row set before the open from one OnOpenPage set.
            let before_open = if has_source { self.temp_rec_position(st)? } else { None };
            if let (Some(why), true) = (rec_unpinned, has_source) {
                self.page_rec(st).borrow_mut().unpinned_buffer = Some(why);
            }
            // The linked fields of an action's `RunPageLink`, filled into Rec before OnOpenPage; the others stay blank
            // (handlers/TestActionRunPageLinkFilterGroup.al:292-369, see `run_page_link_fill`).
            if has_source {
                let rec = self.page_rec(st);
                let mut r = rec.borrow_mut();
                r.run_page_link = None;
                if let Some((fill, link)) = &rec_fill {
                    for (pos, value) in fill {
                        r.fields[*pos] = value.clone();
                    }
                    r.run_page_link = Some(link.clone());
                }
            }
            self.pages.run_page_link_open_moved = false;
            let opened = self.page_trigger(st, "onopenpage", vec![]);
            st.borrow_mut().opening = false;
            st.borrow_mut().run_page_link_open_moved = std::mem::take(&mut self.pages.run_page_link_open_moved);
            if rec_unpinned.is_some() && has_source {
                self.page_rec(st).borrow_mut().unpinned_buffer = None;
            }
            // An error out of the OnOpenPage of a request page with a source table, with its handler bound, is not pinned: it could
            // leave the handler queued for a later codeunit (corpus report/lt-y-request-page/RsRequestPageRec.al:153, report 88563;
            // al-runner issue 5756). Without a handler the run fails before OnOpenPage (YRequestPageTests.al:509).
            if let Err(Signal::Error(_)) = &opened {
                if has_source && self.is_request_page(st) {
                    return unsupported(
                        "an error out of the OnOpenPage of a request page with a source table (whether its handler stays queued is not pinned)",
                    );
                }
            }
            opened?;
            if let Some(before) = before_open {
                let kept = self.temp_rec_position(st)?.is_some_and(|now| {
                    now.0 == before.0 && now.1 == before.1 && now.2 == before.2 && Self::same_key(&now.3, &before.3)
                });
                st.borrow_mut().onopenpage_kept_rec = kept;
            }
            self.page_event(st, "onopenpageevent", None, None)?;
            if has_source {
                self.drop_or_group(st)?;
                self.apply_tree_cut(st)?;
            }
            if unattended {
                return Ok(());
            }
            let mut new_line = has_source && self.offers_new_line(st)?;
            // A record handed to the page filtered to one Enum or Option value the blank row does not hold: no blank line shows
            // (pages/lt-ce-page-run/CePageRun.al RunModal_EnumFilter_RowsAndNewRowDefault, RunModal_OptionFilter_RowsAndNewRowDefault,
            // `[2][5]`; a Boolean filter the blank row matches keeps it, handlers/TestPageModalNewRowLine_Tests.al:180).
            if new_line {
                if let Some(r) = &record {
                    if self.enum_filter_excludes_blank(st, r)? {
                        new_line = false;
                    }
                }
            }
            st.borrow_mut().new_line = new_line;
            // Visible is evaluated once, at open.
            let mut frozen = Vec::new();
            for i in 0..page.controls.len() {
                frozen.push(self.prop_bool_now(st, i, "visible", true)?);
            }
            st.borrow_mut().frozen_visible = frozen;
            // The parts open right after the host's own OnOpenPage, before its first row is read
            // (corpus handlers/TestPagePartAgcr_Tests.al:PartFiresOnHostOpen_NoTouch).
            self.open_parts(st)?;
            if as_part {
                return Ok(());
            }
            let waiting = self.parts_waiting_refresh(st);
            st.borrow_mut().open_read = true;
            let (loaded, origin) = self.traced(st, |s| s.load_first_row(st, mode, record.is_some()));
            st.borrow_mut().open_read = false;
            st.borrow_mut().test_opened_new = false;
            st.borrow_mut().field_captions_settled = true;
            self.open_row_error(st, mode, test_open, loaded, origin)?;
            // A card over a table with no row to show stands on a blank Rec that was never stored: an action cannot have deleted it.
            if has_source && !self.is_list(st) && st.borrow().cursor == Cursor::Row && !st.borrow().draft {
                let rec = self.page_rec(st);
                let never = self.stored_row(&rec)?.is_none();
                st.borrow_mut().never_stored = never;
            }
            self.sync_parts_at_open(st)?;
            self.refresh_parts_asked_by_host(st, &waiting, true, false)
        })();
        if let Err(e) = result {
            // The open raises the error itself and leaves the variable closed
            // (corpus testpageteardown/TestPageTeardown.al:3-24).
            self.mark_closed(st);
            return Err(e);
        }
        // A Worksheet run modally: First right after the open, on the row it opened on, reads nothing (pages/lt-outside-worksheet/
        // RmWorksheetGoneRow.al:397 RM_Ws_RunModal_DeleteAllUpdate `handler first = Yes`, `handler open log` only the open's reads).
        // Opened by OpenEdit, OpenView or Page.Run, or another page type, First keeps reading the row as before (not pinned).
        if st.borrow().modal
            && engine.object_facts[st.borrow().object as usize].page_type == Some(al_ir::PageType::Worksheet)
            && self.is_list(st)
            && st.borrow().cursor == Cursor::Row
            && !st.borrow().draft
            && !self.declares(st, "onfindrecord")
            && !self.declares(st, "onnextrecord")
        {
            st.borrow_mut().settled_first_row = true;
        }
        // Every page of the tree computes its expression values as it opens (QjPartPropagation.al:4635 `opened`).
        if !as_part {
            self.shown_opened(st);
        }
        Ok(handler)
    }

    /// Whether the page declares OnFindRecord and its SourceTableView has a WHERE part: the shape
    /// pagefindrecord/QcFindViewTests.al pins (corpus bump 33).
    pub(in crate::pages) fn find_page_filtered_view(&self, st: &St) -> bool {
        self.declares(st, "onfindrecord")
            && self.engine.object_facts[st.borrow().object as usize].has_source_table_view
            && self.source_view_filters(st)
    }

    /// `find_page_filtered_view` on a page with neither OnAfterGetRecord nor OnAfterGetCurrRecord, the pages of writer QC's probe:
    /// moves inside the rows the client holds ask the page nothing (QcFindViewTests.al:97, :135, :236, :540).
    /// Pinned for an ascending view and for a descending one on a List page; a descending part is refused by
    /// `refuse_descending_filtered_find_move`.
    /// Also a List page (not a part) with Editable = false whose SourceTableView only sorts, ascending or descending: it moves as
    /// the filtered view does (misc/lt-probes-needed-6/PnF06FindListMoves.al:1521-1934 RK_Desc_* and RK_Asc_*, pages 88070-88073;
    /// `sorted_find_list_pinned`).
    pub(in crate::pages) fn filtered_find_list_without_row_triggers(&self, st: &St) -> bool {
        (self.find_page_filtered_view(st) || self.sorted_find_list_pinned(st))
            && !self.declares(st, "onaftergetrecord")
            && !self.declares(st, "onaftergetcurrrecord")
            && (self.page_rec(st).borrow().ascending || self.descending_find_list_pinned(st))
    }

    /// A List page (not a part), Editable = false, with OnFindRecord and a SourceTableView without a WHERE part: the shape of writer
    /// RK's pages 88070-88073 (PnF06FindListMoves.al), whose moves ask the page nothing. An editable page, a part, or a page whose
    /// view sets no key is not pinned.
    pub(super) fn sorted_find_list_pinned(&self, st: &St) -> bool {
        let object = st.borrow().object as usize;
        let props = &self.engine.program.objects[object].properties;
        self.declares(st, "onfindrecord")
            && self.engine.object_facts[object].has_source_table_view
            && !self.source_view_filters(st)
            && props
                .iter()
                .any(|(k, v)| k.eq_ignore_ascii_case("SourceTableView") && v.to_ascii_lowercase().contains("sorting"))
            && props
                .iter()
                .any(|(k, v)| k.eq_ignore_ascii_case("Editable") && v.trim().eq_ignore_ascii_case("false"))
            && self.descending_find_list_pinned(st)
    }

    /// A List page (not a part) with OnFindRecord, a descending filtered SourceTableView and no row trigger: its moves ask the
    /// page nothing, as on an ascending view (misc/lt-probes-needed-6/PnF06FindListMoves.al:825-908 QV_DescKind_FindOnly_Moves
    /// and QV_DescKind_FindNext_Moves, every move's log empty; AL Runner issue 5682). A part is not pinned.
    pub(super) fn descending_find_list_pinned(&self, st: &St) -> bool {
        st.borrow().host.is_none()
            && matches!(
                self.engine.object_facts[st.borrow().object as usize].page_type,
                Some(al_ir::PageType::List)
            )
    }

    /// A move (First, Last, Next, Previous, GoToKey) on a list with OnFindRecord, a descending filtered SourceTableView and no row
    /// trigger: the rows it lands on are pinned (QcFindViewTests.al:257 DescList_OpenAndWalk `first = Yes:5`, `walk = 5,3,1`), whether
    /// the page is asked for them is not: refused.
    pub(in crate::pages) fn refuse_descending_filtered_find_move(&self, st: &St) -> Result<(), Signal> {
        if self.is_list(st)
            && self.find_page_filtered_view(st)
            && !self.declares(st, "onaftergetrecord")
            && !self.declares(st, "onaftergetcurrrecord")
            && !self.page_rec(st).borrow().ascending
            && !self.descending_find_list_pinned(st)
        {
            return unsupported(
                "a move on a list with OnFindRecord and a descending filtered SourceTableView (whether the page is asked is not pinned)",
            );
        }
        Ok(())
    }

    /// Whether the page's SourceTableView has a WHERE part.
    pub(super) fn source_view_filters(&self, st: &St) -> bool {
        let object = st.borrow().object as usize;
        self.engine.program.objects[object]
            .properties
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("SourceTableView"))
            .is_some_and(|(_, v)| v.to_ascii_lowercase().contains("where"))
    }

    /// Whether the blank row of a page run on `rec` fails one of `rec`'s filters on a field the new row does not take from the
    /// filter (a key field, or any field with PopulateAllFields, takes a single value): true for a single Enum or Option value
    /// (pinned, see the caller); any other filter keeps the blank line.
    pub(super) fn enum_filter_excludes_blank(&mut self, st: &St, rec: &Rec) -> Result<bool, Signal> {
        let table = rec.borrow().table as usize;
        let fresh = self.initial_fields(table);
        let object = st.borrow().object as usize;
        let populate_all = self.engine.program.objects[object]
            .properties
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("PopulateAllFields") && is_true(v));
        let Some(t) = self.engine.program.objects[table].table.as_ref() else {
            return Ok(false);
        };
        let mut hidden = false;
        for f in rec.borrow().filters.iter() {
            let single = match &f.expr {
                al_builtins::filter::FilterExpr::Range(Some(a), Some(b)) => ops::compare(a, b).is_ok_and(|o| o.is_eq()),
                al_builtins::filter::FilterExpr::Eq(_) => true,
                _ => false,
            };
            if single && (populate_all || t.keys[0].contains(&f.field)) {
                continue;
            }
            if f.expr.matches(&fresh[f.field]) {
                continue;
            }
            let typed_single = matches!(&f.expr, al_builtins::filter::FilterExpr::Range(Some(_), Some(_))) && single;
            if typed_single && matches!(t.fields[f.field].ty, RtType::Enum(_) | RtType::Option(_)) {
                hidden = true;
                continue;
            }
            // Any other filter the blank row fails keeps the blank line: a Boolean (pages/lt-modal-list-new-row/
            // ModalListNewRowKeyFilters.al P157_NonKeyFilter `[0///No/]` with Active = true), and the Text, Code and range shapes
            // 63 baseline cases walk past (pages/W1Navigation.al, ModalListNewRowKeyFilters.al, P197, P157 and others; the
            // lane-page-views PR lists them).
        }
        Ok(hidden)
    }

    /// Whether the page has code that runs after OnOpenPage and could read its `Rec`: another trigger, a control or action
    /// trigger, an extension; with `open`, OnInit and OnOpenPage count too.
    pub(super) fn page_code_runs(&self, st: &St, open: bool) -> bool {
        let engine = self.engine();
        let page = page_of(&engine, st);
        page.triggers
            .iter()
            .any(|(n, _)| open || (n != "oninit" && n != "onopenpage"))
            || !page.extensions.is_empty()
            || !page.ext_triggers.is_empty()
            || page
                .controls
                .iter()
                .any(|c| !c.triggers.is_empty() || !c.ext_triggers.is_empty())
    }

    /// Filters OnOpenPage left in `FilterGroup(-1)` do not limit the rows the page shows: the record counts the OR rows inside the
    /// trigger, the page shows every row (pages/lt-cd-testpage-leads/CdTestPageLeads.al:776 Item63_FilterGroupMinus1_InOnOpenPage).
    /// What the page's `Rec` holds in group -1 afterwards is not pinned, so a page with code that could read it later (another
    /// trigger, a control or action trigger, an extension) is refused, and so is a TestPage `Filter` read (see `test_filter_op`).
    pub(super) fn drop_or_group(&mut self, st: &St) -> Result<(), Signal> {
        let rec = self.page_rec(st);
        if !rec
            .borrow()
            .filters
            .iter()
            .any(|f| f.group == al_builtins::filter::OR_GROUP)
        {
            return Ok(());
        }
        let reads_later = self.page_code_runs(st, false);
        if reads_later {
            return unsupported(
                "FilterGroup(-1) filters left by OnOpenPage on a page with code that runs later (what Rec holds then is not pinned)",
            );
        }
        rec.borrow_mut()
            .filters
            .retain(|f| f.group != al_builtins::filter::OR_GROUP);
        st.borrow_mut().or_group_dropped = true;
        Ok(())
    }

    /// An AL error escaping the first row a TestPage reads when it opens. OpenEdit of a card turns an error of OnFindRecord,
    /// OnAfterGetRecord or OnAfterGetCurrRecord into "Page <caption> has to close." (corpus pages/lt-cc-page-leads/
    /// CcPageLeads.al:520-557, TriggerError_OnFindRecord, _OnAfterGetRecord, _OnAfterGetCurrRecord). OpenView raises the error
    /// itself (testpageteardown/TestPageTeardown.al:1130-1156, CardTrigger_FailedOpen_*), and an error of a control
    /// expression comes out as it is at OpenEdit too (autoformat/TestAutoFormatExpressionTiming.al:499-506
    /// FailingFormat_OpenEditRaisesTheExpressionError). What OpenEdit of another page type raises for such a trigger error is
    /// not pinned: its text and code cannot be read. Pages opened any other way keep their own rules.
    pub(super) fn open_row_error(
        &mut self,
        st: &St,
        mode: PageMode,
        test_open: bool,
        r: Result<(), Signal>,
        origin: Option<String>,
    ) -> Result<(), Signal> {
        let row_trigger = matches!(
            origin.as_deref(),
            Some("onfindrecord" | "onaftergetrecord" | "onaftergetcurrrecord")
        );
        let e = match r {
            Err(Signal::Error(e)) if test_open && mode == PageMode::Edit && row_trigger => e,
            other => return other,
        };
        // A control add-in call (the add-in is never created in a test session) does not close the page: OpenEdit of a card
        // raises the add-in's error itself, out of OnAfterGetRecord with its own code and out of OnAfterGetCurrRecord with
        // `:TargetInvocation` added (corpus lt-ac/ACControlAddIn.al TZ_CallInOnAfterGetRecord_TestPageOpen,
        // TZ_CallInOnAfterGetCurrRecord_TestPageOpen, BC28PLUS). Out of OnFindRecord it is not pinned.
        if e.code == crate::control_add_in::NOT_CREATED {
            // Pinned by lt-ac/ACControlAddIn.al TZ_CallInOnAfterGetRecord_TestPageOpen, TZ_CallInOnAfterGetCurrRecord_TestPageOpen, TZ_CallInOnClosePage_TestPageClose, TZ_TwoProceduresFromActions_PageStaysUsable (was assumption `control-add-in-call-trigger-contexts`).
            let mut e = e;
            match origin.as_deref() {
                Some("onaftergetrecord") if self.is_card(st) && self.has_source(st) => return Err(Signal::Error(e)),
                Some("onaftergetcurrrecord") if self.is_card(st) && self.has_source(st) => {
                    e.code = format!("{}:TargetInvocation", e.code);
                    return Err(Signal::Error(e));
                }
                _ => return Err(Signal::Error(Box::new(al_builtins::native::unpinned_error()))),
            }
        }
        if self.is_card(st) && self.has_source(st) {
            let caption = self.page_caption(st);
            return Err(self.has_to_close(st, &e, caption));
        }
        Err(Signal::Error(Box::new(al_builtins::native::unpinned_error())))
    }

    /// The first row a page shows when it opens (or when a part is linked afresh).
    pub(in crate::pages) fn load_first_row(
        &mut self,
        st: &St,
        mode: PageMode,
        with_record: bool,
    ) -> Result<(), Signal> {
        let has_source = self.has_source(st);
        if !has_source {
            // A card over no table: OpenNew fails with DB:ClientInsertDenied after OnOpenPage, and no other trigger runs
            // (misc/lt-protected-var-abs-query-views/P146PageExprSetValue.al:460 QG_NoSource_OpenNew_StepLog `qg open;`). Another
            // page type is not pinned.
            if mode == PageMode::New && !self.is_request_page(st) {
                if !self.is_card(st) {
                    return unsupported("OpenNew of a page without a source table other than a card (not pinned)");
                }
                return Err(self.no_source_open_new_denied(st)?);
            }
            // A page over no table shows its one implicit row: the expressions, then OnAfterGetCurrRecord. A card runs no
            // OnAfterGetRecord (P146PageExprSetValue.al:420 QG_NoSource_OpenEdit_StepLog, :440 QG_NoSource_OpenView_StepLog
            // `qg open;qg agcr;`); for another page type it is not pinned and stays as it was.
            st.borrow_mut().cursor = Cursor::Row;
            self.populate_expressions(st)?;
            if !self.is_card(st) || self.is_request_page(st) {
                self.page_trigger(st, "onaftergetrecord", vec![])?;
            }
            self.page_trigger(st, "onaftergetcurrrecord", vec![])?;
            self.page_event(st, "onaftergetcurrrecordevent", None, None)?;
            return Ok(());
        }
        if mode == PageMode::New {
            // corpus handlers/TestPageCreateModeNewRecordArgs.al:243-270: OnOpenPage, then
            // OnNewRecord with BelowxRec true and xRec on the last row.
            // A page with an OnFindRecord trigger is opened the same way: the trigger is not asked, xRec is the last row
            // (pages/lt-opennew-findrecord/OpenNewFindRecord.al:150-230, a card and a list, empty and with rows).
            self.ret_used = true;
            let rec = self.page_rec(st);
            self.record_op(RecordOp::FindLast, &rec, &mut [])?;
            return self.start_draft(st, false);
        }
        if with_record {
            // The page opens on the record the caller handed it, when that row exists
            // (corpus handlers/TestPageRunHandler_Tests.al:PageHandlerReceivesTheRecordPassedToPageRun).
            let rec = self.page_rec(st);
            // A list that serves its rows from OnFindRecord is asked `=>` for the record it was handed, as the page holds it,
            // and reads on from there as when it opens (pages/lt-ai-list-order/AiListOrderTests.al:871 PageRun_RecordOnFifthRow
            // `F:=>=Yes;R:R1005;...;R:R1012;N:1=0;R:R1005;N:-1=-1;R:R1004;...;R:R1001;N:-1=0`, :890 PageRun_RecordWithFilterOnFifthRow).
            // A card is asked `=` once, after OnOpenPage, with Rec as OnOpenPage left it, over a temporary or a database
            // table (pages/lt-be-card-onfindrecord/BeCardOnFindRecord.al:292 CardTempFind_PageRun_TempRec `F:=:1/passed/in memory`,
            // :309 CardTempFind_PageRun_StoredRec, and the CardDbFind_* tests).
            if self.declares(st, "onfindrecord") && self.stored_row(&rec)?.is_some() {
                let which = if self.is_list(st) { "=>" } else { "=" };
                // A filtered view is asked the same (pagefindrecord/QcFindViewTests.al:157 ViewList_PageRunOnFilteredOutRow,
                // :412 ViewCard_PageRunOnFilteredOutRow).
                let filtered_view = self.find_page_filtered_view(st);
                let handed = rec.borrow().clone();
                let handed_row = self.row_key(&rec)?;
                if !self.nav_find(st, which)? {
                    // A card whose filtered view leaves out the handed row is asked `=` and then `=><`, and opens on what that
                    // finds (corpus branch al-runtime-rs-staging-qc eca98051, pagefindrecord/QcFindViewTests.al:412
                    // ViewCard_PageRunOnFilteredOutRow `find(=)=No:4;find(=><)=Yes:5;`, :457 FallbackCard_KeyFilterOnFilteredOutRow).
                    // A list whose `=>` fails, a card that finds nothing with `=><` either, and a page without a filtered view are
                    // not pinned.
                    if !filtered_view || self.is_list(st) {
                        return unsupported("OnFindRecord finds no row for the record handed to the page (not pinned)");
                    }
                    // The second call sees the handed record's filters again, whatever the first call did to them
                    // (QcFindViewTests.al:457 FallbackCard_KeyFilterOnFilteredOutRow: the first call clears the key filter, the
                    // second still fails before it clears it, `find(=><)=No,again=Yes:5`). A first call that moved the record
                    // or changed its key or order is not pinned.
                    let moved = !Self::same_key(&self.row_key(&rec)?, &handed_row);
                    {
                        let now = rec.borrow();
                        if moved || now.key != handed.key || now.ascending != handed.ascending {
                            drop(now);
                            return unsupported(
                                "OnFindRecord('=') moved the record handed to the card or changed its key (what the next call sees is not pinned)",
                            );
                        }
                    }
                    rec.borrow_mut().filters = handed.filters.clone();
                    if !self.nav_find(st, "=><")? {
                        return unsupported(
                            "OnFindRecord finds no row for the record handed to the card, with `=` and `=><` (not pinned)",
                        );
                    }
                }
                st.borrow_mut().cursor = Cursor::Row;
                st.borrow_mut().landing_first = true;
                let r = self.arrive(st);
                st.borrow_mut().landing_first = false;
                r?;
                let from_start = self.is_list(st);
                st.borrow_mut().window_from_start = from_start;
                return Ok(());
            }
            if self.stored_row(&rec)?.is_some() {
                self.ret_used = true;
                let mut pk = self.table(&rec)?.primary_key(&rec.borrow().fields);
                if matches!(self.record_op(RecordOp::Get, &rec, &mut pk)?, Value::Boolean(true)) {
                    // A row the view does not admit is not the page's: it opens on the first row at or after it in the view
                    // (pages/lt-page-setrecord-settableview/PageVarSetRecordTableView.al:P157_List_SetRecordThenSetTableView_RecordOutsideView
                    // `B` over `Grp = 1` opens on `C`, P157_Card_SetRecordOutsideView).
                    if !rec.borrow().filters.is_empty() {
                        if self.nav_find(st, "=>")? {
                            st.borrow_mut().cursor = Cursor::Row;
                            // A list that lands on the first row of its view reads it as a list opened on its first row does:
                            // the window, the row once more, then OnAfterGetCurrRecord, and First afterwards reads nothing
                            // (misc/lt-probes-needed-7/PnGRunObjectOptions.al:130 LinkAndViewOnAList `link`, an action's
                            // RunPageLink on the key handing the list the host's row: `List.AGR(RP2,Two);List.AGR(RP2,Two);
                            // List.AGCR(RP2,RP2)`, the same reads as a handed record that names no row, PnGNewRecord.al:50
                            // HandedRecordNonKeyFilters). Landing on another row of the view keeps the reads it had before.
                            if self.is_list(st) && self.on_first_row_of_view(&rec)? {
                                st.borrow_mut().landing_first = true;
                                let r = self.arrive(st);
                                st.borrow_mut().landing_first = false;
                                r?;
                                st.borrow_mut().window_from_start = true;
                                return Ok(());
                            }
                            return self.arrive(st);
                        }
                    } else {
                        st.borrow_mut().cursor = Cursor::Row;
                        // A modal list handed a record that names the first row of its unfiltered view reads it as a list
                        // opened on its first row does (the window, the row once more, then OnAfterGetCurrRecord), and a GoToKey
                        // then reads the new row twice (pages/lt-ai-list-order/LookupGoToKeyTypedReads.al:204-221
                        // Y7_PageRunModalRecord_GoToKey_TypedReads_OK `open=R:A,R:B,R:C,R:A,R:A,C:A;goto=Yes;after goto=R:B,R:B,C:B`).
                        // A page run without modal keeps the reads it had before (not pinned).
                        if self.is_list(st) && st.borrow().modal && self.on_first_row_of_view(&rec)? {
                            st.borrow_mut().landing_first = true;
                            let r = self.arrive(st);
                            st.borrow_mut().landing_first = false;
                            r?;
                            st.borrow_mut().window_from_start = true;
                            return Ok(());
                        }
                        return self.arrive(st);
                    }
                }
            }
            // A record that names no row of the view (a blank one, a SetTableView, a row outside the view or not in the table)
            // opens the page `=><` from the record's own values: the row itself, else the next one in the view's direction, else
            // the one before. A descending view opened on a blank record lands on its last row (misc/W2PageProperties.al:234
            // SetTableView_DescendingOrder `[A]`; pages/lt-page-sourceview-current-row/PageSourceViewCurrentRow.al:140
            // P184_RunModal_NoRecord_DescendingView `1`, :192 P184_RunModal_RecordWithOwnFilterAndKey `1`), a record past the
            // last row on the last row (platform/lt-xmlport-session-navapp-cookie/P94PageRun.al:88 Run_WithRecordNotInTable
            // `id=3`, record/lt-loadfields-pagesource-validate/P81PageTempSource.al:358 PageVar_SetRecord_TempRecord `D2`).
            // An action's RunPageView opens on the first row of its view instead (handlers/TestPageActionRunPageView.al:296
            // RunPageView_OrderDescending_ReversesTheOpenedPage `cur=4`); `lenient_bind` marks that open (run_object_page).
            if !self.declares(st, "onfindrecord") && !self.pages.lenient_bind {
                // A List run with Page.Run (not modal) on a record that is still blank, whose view is descending, enters on the first
                // row of the view, not where `=><` lands (al-runner issue 6114; corpus writer TW, branch al-runtime-rs-staging-tw,
                // pages/lt-page-sourceview-current-row/TwPageRunDescView.al:161 DescViewAscRecPageRun, :188 DescViewDescRecPageRun
                // `entry=E3S30`, against :120 DescViewAscRec, RunModal, `entry=E2S10`).
                if self.is_list(st)
                    && st.borrow().last_run_modal == Some(false)
                    && st.borrow().set_record.is_none()
                    && st.borrow().host.is_none()
                    && !st.borrow().lookup
                    && !self.page_rec(st).borrow().ascending
                    && self.engine.program.objects[st.borrow().object as usize]
                        .properties
                        .iter()
                        .any(|(k, v)| {
                            k.eq_ignore_ascii_case("SourceTableView")
                                && v.to_ascii_lowercase().replace(' ', "").contains("order(descending)")
                        })
                {
                    let rec = self.page_rec(st);
                    let table = rec.borrow().table as usize;
                    let initial = self.initial_fields(table);
                    if Self::same_values(&rec.borrow().fields, &initial) {
                        // Pinned by pages/lt-page-sourceview-current-row/TwPageRunDescView.al (was assumption `page-run-blank-record-descending-view-first-row`).
                        return self.page_first_row(st);
                    }
                }
                return self.page_first_row_from(st, "=><", true);
            }
        } else if st.borrow().host.is_none()
            && self.declares(st, "onopenpage")
            && (!self.declares(st, "onfindrecord") || self.temp_worksheet_set_before_open(st))
            && !self.is_request_page(st)
        {
            if let Some(modal_list) = self.init_record_on_a_key_no_row_has(st)? {
                return if modal_list {
                    self.open_on_rec_moved_by_onopenpage(st)
                } else {
                    self.page_first_row(st)
                };
            }
            self.read_rec_in_its_key(st)?;
            if self.rec_moved_by_onopenpage(st)? {
                return self.open_on_rec_moved_by_onopenpage(st);
            }
        } else if st.borrow().host.is_none()
            && self.is_request_page(st)
            && self.has_source(st)
            && self.declares(st, "onopenpage")
            && self.request_rec_moved_by_onopenpage(st)?
        {
            return self.open_on_rec_moved_by_onopenpage(st);
        }
        self.page_first_row(st)
    }

    /// A report request page with a source table whose OnOpenPage left Rec on a stored row other than the first of its view: true
    /// when the page opens on that row. Pinned by corpus report/lt-y-request-page/YRequestPageTests.al:443
    /// RS_RequestPageRec_ModifyAndInsert_InOnOpenPage (Rec inserted on `N/1`, the handler reads `docno = N`, `descr = inserted`) and
    /// :471 RS_RequestPageRec_ReadInReportTriggers (FindFirst then Next, the handler reads `A` and `second`). A blank key or Rec on the
    /// first row opens on the first row, as before. A temporary source, another key, a filter, Rec on a key no row has or holding
    /// values its row does not, and the find and record triggers of the page are not pinned for a moved Rec: refused.
    pub(super) fn request_rec_moved_by_onopenpage(&mut self, st: &St) -> Result<bool, Signal> {
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
        if let Value::Boolean(true) = self.record_op(RecordOp::FindFirst, &probe, &mut [])? {
            if Self::same_key(&self.row_key(&probe)?, &key) {
                return Ok(false);
            }
        }
        let refuse = |what: &str| -> Result<bool, Signal> {
            unsupported(format!(
                "a request page whose OnOpenPage left Rec on a row other than the first of its view, {what} (which row it opens on is not pinned)"
            ))
        };
        if rec.borrow().temporary {
            return refuse("over a temporary source");
        }
        if rec.borrow().key != 0 {
            return refuse("in a key other than the primary key");
        }
        if !rec.borrow().filters.is_empty() {
            return refuse("with a filter set");
        }
        if [
            "onfindrecord",
            "onnextrecord",
            "onaftergetrecord",
            "onaftergetcurrrecord",
        ]
        .iter()
        .any(|t| self.declares(st, t))
        {
            return refuse("with find or record triggers");
        }
        match self.stored_row(&rec)? {
            None => refuse("on a key no row has"),
            Some(stored) if !Self::same_values(&stored, &rec.borrow().fields) => {
                refuse("holding values its stored row does not")
            }
            Some(_) => Ok(true),
        }
    }

    /// A Card or List over a database table opened without a record, in a key other than the primary key, whose Rec is still the
    /// initial record (OnOpenPage left it alone) and stands on a primary key no row has because a key field has an InitValue.
    /// `Some(true)` for a List run modally, which opens `=><` from Rec (pages/lt-outside-worksheet/RmInitValueKey.al:290
    /// RM_List_RunModal opens on Line 2, `R:2;R:1;R:3;R:2;R:4;R:2;R:2;R:2;C:2;`); `Some(false)` otherwise, which opens on the first
    /// row of the view (:231 RM_List_OpenEdit, :250 _OpenView, :269 _PageRun on Line 4, :330 RM_ListDesc_OpenEdit on 3, :349
    /// RM_Card_OpenEdit and :369 RM_Card_RunModal on 4). A filtered view, lookup mode or OnNextRecord are not pinned and go on
    /// as before (`read_rec_in_its_key` refuses them).
    pub(super) fn init_record_on_a_key_no_row_has(&mut self, st: &St) -> Result<Option<bool>, Signal> {
        let rec = self.page_rec(st);
        if rec.borrow().key == 0 || rec.borrow().temporary || !rec.borrow().filters.is_empty() {
            return Ok(None);
        }
        let page_type = &self.engine.object_facts[st.borrow().object as usize].page_type;
        if !matches!(page_type, Some(al_ir::PageType::Card | al_ir::PageType::List))
            || st.borrow().lookup
            || self.declares(st, "onnextrecord")
            || self.declares(st, "onfindrecord")
        {
            return Ok(None);
        }
        let table = rec.borrow().table as usize;
        let initial = self.initial_fields(table);
        if !Self::same_values(&rec.borrow().fields, &initial) {
            return Ok(None);
        }
        let key = self.row_key(&rec)?;
        let t = self.table(&rec)?;
        let blank: Vec<Value> = t.keys[0].iter().map(|&f| self.default_value(&t.fields[f].ty)).collect();
        if Self::same_key(&key, &blank) || self.stored_row(&rec)?.is_some() {
            return Ok(None);
        }
        Ok(Some(st.borrow().modal && self.is_list(st)))
    }

    /// A Card or List over a database table opened without a record, whose OnOpenPage left Rec on a row in a key other than the
    /// primary key (a SourceTableView sorting or a SetCurrentKey): before the page finds its row, it reads the row of Rec's
    /// primary key, filters aside, and runs OnAfterGetRecord on it; it then finds from that row in Rec's key.
    /// Pinned by corpus pages/lt-qr-onopenpage-moves-rec/QrMovedTests.al:228 QR_Card_OpenView_SetCurrentKeyGet `O:B;R:B;R:B;C:B;`,
    /// :345 QR_List_OpenView_SetCurrentKeyGet, and the QS_KeyAsc_*, QS_KeyDesc_* and QS_KeyWhere_* tests from :856: QS_KeyAsc_Card_GetC
    /// `O:C;R:C;R:C;C:C;`, QS_KeyAsc_Card_FindFirst `O:D;R:D;R:D;C:D;` (on the first row of the view too), QS_KeyAsc_Card_KeyAssigned
    /// (`"No." := 'C'` without a read opens on `C|bravo`: the stored row's Name is what the key finds from),
    /// QS_KeyWhere_Card_GetD `O:D;R:D;R:C;C:C;` (D is outside the view's WHERE, read all the same), QS_KeyWhere_Card_Run_GetD
    /// (Page.Run). A card run modally does not read first (QS_KeyDesc_Card_RunModal_FindLast `O:D;R:D;C:D;`); a list run modally
    /// in such a key, and a key no row has, are not pinned. A blank primary key reads nothing (QS_KeyAsc_Card_NoMove `O:;R:D;C:D;`).
    pub(super) fn read_rec_in_its_key(&mut self, st: &St) -> Result<(), Signal> {
        let rec = self.page_rec(st);
        if rec.borrow().key == 0 || rec.borrow().temporary {
            return Ok(());
        }
        let page_type = &self.engine.object_facts[st.borrow().object as usize].page_type;
        // A Worksheet over a database table reads the row first by every route, run modally and in lookup mode too
        // (pages/lt-rz-outside-page-moves/RzWorksheetSecondaryKeyOpen.al RZ_Lookup_RecBeforeView `O:HDR;R:HDR;R:X;...`,
        // RZ_RunModal_*, RZ_OpenEdit_*, RZ_OpenView_*, also when Rec is the first row of the view, RZ_Lookup_RecFirstOfView).
        let worksheet = matches!(page_type, Some(al_ir::PageType::Worksheet)) && !self.declares(st, "onnextrecord");
        if !worksheet
            && (!matches!(page_type, Some(al_ir::PageType::Card | al_ir::PageType::List))
                || st.borrow().lookup
                || self.declares(st, "onnextrecord"))
        {
            return Ok(());
        }
        let mut pk = self.row_key(&rec)?;
        let table = self.table(&rec)?;
        let blank: Vec<Value> = table.keys[0]
            .iter()
            .map(|&f| table.fields[f].ty.clone())
            .collect::<Vec<_>>()
            .iter()
            .map(|ty| self.default_value(ty))
            .collect();
        if Self::same_key(&pk, &blank) {
            return Ok(());
        }
        if st.borrow().modal && !worksheet {
            if self.is_list(st) {
                return unsupported(
                    "a list run modally without a record whose OnOpenPage left Rec on a row in a key other than the primary key (not pinned)",
                );
            }
            return Ok(());
        }
        self.ret_used = true;
        if !matches!(self.record_op(RecordOp::Get, &rec, &mut pk)?, Value::Boolean(true)) {
            return unsupported(
                "a page opened without a record whose OnOpenPage left Rec, in a key other than the primary key, on a primary key no row has (not pinned)",
            );
        }
        self.populate_current(st)
    }

    /// A page without OnFindRecord opened without a record, whose OnOpenPage left Rec on a key that is neither blank nor the
    /// first row of the view: true when the page opens `=><` from Rec as OnOpenPage left it. A Card or List page, opened by
    /// TestPage OpenView/OpenEdit, Page.Run or Page.RunModal, opens on the row Rec was left on, in the view OnOpenPage set, and
    /// on the nearest row of the view when Rec is outside the filter it set (corpus
    /// pages/lt-qr-onopenpage-moves-rec/QrMovedTests.al:175 QR_Card_OpenView_Get `C|bravo`, :188 _FindLast, :201 _FindFirstNext,
    /// :214 _SetRangeFindLast, :227 _SetCurrentKeyGet, :240 _KeyAssigned `C` without a read, :253 _GetThenFilterExcludes `B` for
    /// C with filter A..B, :279 QR_Card_OpenEdit_Get, :292-396 the same for a List, :540 QR_Card_PageRun_Get, :555
    /// QR_List_PageRun_Get, :600 QR_Card_RunModal_Get, :615 QR_List_RunModal_FindLast). A temporary source OnOpenPage fills opens
    /// on the row Rec was left on too (:409 QR_TempCard_OpenView_NoMove `D` after the inserts, :474 QR_TempList_OpenView_NoMove,
    /// :422-526, :570, :585, :630). Another page type, a page in lookup mode or with OnNextRecord are not pinned: refused (a
    /// temporary source of another page type keeps the first row, as before: handlers/TestTempIntegerListPart.al:92,
    /// handlers/TestPageVariableControlCaption_Tests.al:120, testpart/BnrBlankPart.al:596 do not tell the two apart). A part is
    /// not asked (the caller checks). Tests-ERM 134710 EmptySetup reaches it (Payment Registration Setup inserts the user's row
    /// in OnOpenPage while another user's row sorts first).
    pub(super) fn rec_moved_by_onopenpage(&mut self, st: &St) -> Result<bool, Signal> {
        let rec = self.page_rec(st);
        let page_type = &self.engine.object_facts[st.borrow().object as usize].page_type;
        let pinned_type = matches!(page_type, Some(al_ir::PageType::Card | al_ir::PageType::List));
        let list_type = matches!(page_type, Some(al_ir::PageType::List));
        let worksheet_type = matches!(page_type, Some(al_ir::PageType::Worksheet));
        // A Worksheet over a temporary source whose row was set before the open is pinned (`set_before` below); one whose
        // OnOpenPage moved Rec opens as a List does under assumption worksheet-temp-source-open-row. A Worksheet over a
        // database table stays refused below.
        let temp_worksheet = rec.borrow().temporary && matches!(page_type, Some(al_ir::PageType::Worksheet));
        let set_before = temp_worksheet && st.borrow().onopenpage_kept_rec;
        if rec.borrow().temporary && !pinned_type && !temp_worksheet {
            return Ok(false);
        }
        let key = self.row_key(&rec)?;
        let table = self.table(&rec)?;
        let blank: Vec<Value> = table.keys[0]
            .iter()
            .map(|&f| table.fields[f].ty.clone())
            .collect::<Vec<_>>()
            .iter()
            .map(|ty| self.default_value(ty))
            .collect();
        if Self::same_key(&key, &blank) {
            return Ok(false);
        }
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        self.ret_used = true;
        let first = match self.record_op(RecordOp::FindFirst, &probe, &mut [])? {
            Value::Boolean(true) => Some(self.row_key(&probe)?),
            _ => None,
        };
        if first.as_ref().is_some_and(|f| Self::same_key(f, &key)) {
            return Ok(false);
        }
        if set_before {
            // A temporary Worksheet whose row was set before the open (a procedure of the page variable filled it and left Rec
            // on a row; OnOpenPage left Rec as it was) opens on that row as a List does, `=><` from Rec (corpus
            // pages/lt-qr-onopenpage-moves-rec/QrMovedTests.al:2410 QT_TempWs_RunModal_B `O:B;R:B;R:C;R:D;R:B;R:A;R:B;R:B;C:B;`,
            // :2426 _BB_Between `C`, :2442 _E_PastLast `D`, :2490 QT_TempWs_Lookup_B, :2506 _Lookup_BB_Between, :2665
            // QT_TempWs_Run_B, :2679 _Run_BB_Between). With OnNextRecord it opens there too, run modally, in lookup mode or not
            // (:2538 QT_TempWsNext_RunModal_B, :2554 _BB_Between, :2570 QT_TempWsNext_Lookup_B); with OnFindRecord and
            // OnNextRecord, run modally outside lookup mode, it is asked `=>` (:2586 QT_TempWsFind_RunModal_B `F:=>>B`, :2602).
            // In a key other than the primary key it is pinned run modally outside lookup mode, without either trigger (:2458
            // QT_TempWs_RunModal_ByName_B). The other forms are past the pins.
            let (modal, lookup) = {
                let s = st.borrow();
                (s.modal, s.lookup)
            };
            let next = self.declares(st, "onnextrecord");
            let find = self.declares(st, "onfindrecord");
            let in_pk = rec.borrow().key == 0;
            let pinned = match (in_pk, next, find) {
                (true, false, false) => modal || !lookup,
                (true, true, false) => modal,
                (true, true, true) => modal && !lookup,
                (false, false, false) => modal && !lookup,
                _ => false,
            };
            if !pinned {
                return unsupported(
                    "a temporary Worksheet whose row was set before it opened, in this mode, key or with these find triggers (which row it opens on is not pinned)",
                );
            }
            return Ok(true);
        }
        if temp_worksheet {
            // A temporary Worksheet whose OnOpenPage moved Rec or changed its key, its order or its filters: both paths below
            // make a choice no corpus test pins (which row it opens on), so both mark the assumption
            // worksheet-temp-source-open-row.
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::WorksheetTempSourceOpenRow);
            // Only a Rec in the primary key, outside lookup mode and without OnNextRecord, from which `=><` reaches a row other
            // than the first of the view, takes the assumed path (page 498 leaves Rec on an entry summary it did not insert,
            // past the purchase line's). Any other temporary Worksheet keeps the first row, as before (the same assumption; the Item Tracking Lines page, whose OnOpenPage changes the key, reaches it in many Microsoft tests).
            if rec.borrow().key != 0 || st.borrow().lookup || self.declares(st, "onnextrecord") {
                return Ok(false);
            }
            // Where `=><` from Rec lands, as the open does (`open_on_rec_moved_by_onopenpage`): on the first row of the view the
            // page opens as it always did.
            let probe = Rc::new(RefCell::new(rec.borrow().clone()));
            self.ret_used = true;
            if !matches!(
                self.record_op(RecordOp::Find, &probe, &mut [Value::Text("=><".into())])?,
                Value::Boolean(true)
            ) {
                return Ok(false);
            }
            let landed = self.row_key(&probe)?;
            if first.as_ref().is_some_and(|f| Self::same_key(f, &landed)) {
                return Ok(false);
            }
            return Ok(true);
        }
        // A temporary List whose row was set before the open opens on that row in lookup mode too (QrMovedTests.al:2650
        // QT_TempList_Lookup_B `O:B;R:B;R:C;R:D;R:B;R:A;R:B;R:B;C:B;`); a List whose OnOpenPage moved Rec, in lookup mode, is not
        // pinned.
        let pinned_lookup = st.borrow().lookup
            && st.borrow().modal
            && rec.borrow().temporary
            && rec.borrow().key == 0
            && st.borrow().onopenpage_kept_rec
            && list_type;
        // A Worksheet over a database table in the primary key, without OnNextRecord (OnFindRecord the caller rules out), opens
        // on the row OnOpenPage left Rec on by every route, in lookup mode too; so does a List in lookup mode whose Rec is on
        // the last row of its view
        // (pages/lt-outside-worksheet-2/RtFillOnOpen.al:263 RT_FillWs_Lookup_ABC_OK `O:C;R:C;R:B;R:A;R:C;R:C;C:C;`, :278 _ACB_OK on
        // B, :293 RunModal, :314 lookup RunModal, :336 Page.Run, :353 and :374 OpenEdit, :395 OpenView, :417 RT_FillList_Lookup_ABC_OK).
        // A List in lookup mode on another row, another key or OnNextRecord are not pinned.
        if !rec.borrow().temporary && rec.borrow().key == 0 && !self.declares(st, "onnextrecord") {
            let list_lookup_on_last = list_type && st.borrow().lookup && {
                let probe = Rc::new(RefCell::new(rec.borrow().clone()));
                self.ret_used = true;
                match self.record_op(RecordOp::FindLast, &probe, &mut [])? {
                    Value::Boolean(true) => Self::same_key(&self.row_key(&probe)?, &key),
                    _ => false,
                }
            };
            if worksheet_type || list_lookup_on_last {
                return Ok(true);
            }
        }
        // The same Worksheet in another key opens on the row `=><` from Rec reaches in that key, by every route, in lookup mode
        // too, with the row triggers reading as in the primary key (pages/lt-rz-outside-page-moves/RzWorksheetSecondaryKeyOpen.al:
        // RZ_Lookup_RecBeforeView `O:HDR;R:HDR;R:X;R:Y;R:X;R:X;R:X;C:X`, RecInsideView, RecAfterView, RecBetweenViewRows,
        // RecBeforeBySecondaryAfterByPrimary, RecFirstOfView, and the RunModal, OpenEdit and OpenView routes). OnNextRecord is not
        // pinned.
        if worksheet_type && !rec.borrow().temporary && rec.borrow().key != 0 && !self.declares(st, "onnextrecord") {
            return Ok(true);
        }
        if !pinned_type || (st.borrow().lookup && !pinned_lookup) || self.declares(st, "onnextrecord") {
            return unsupported(
                "a page opened without a record whose OnOpenPage left Rec on a row other than the first of its view (which row it opens on is not pinned)",
            );
        }
        Ok(true)
    }

    /// The key, order, filters (as text) and primary key of a temporary page's Rec (None for a database source): what tells a row set before
    /// the open from one OnOpenPage set.
    pub(super) fn temp_rec_position(&self, st: &St) -> Result<Option<TempRecPosition>, Signal> {
        let rec = self.page_rec(st);
        if !rec.borrow().temporary {
            return Ok(None);
        }
        let pk = self.row_key(&rec)?;
        let r = rec.borrow();
        Ok(Some((r.key, r.ascending, format!("{:?}", r.filters), pk)))
    }

    /// A Worksheet page over a temporary source whose OnOpenPage left Rec as it stood before the open.
    pub(in crate::pages) fn temp_worksheet_set_before_open(&self, st: &St) -> bool {
        st.borrow().onopenpage_kept_rec
            && self.page_rec(st).borrow().temporary
            && matches!(
                self.engine.object_facts[st.borrow().object as usize].page_type,
                Some(al_ir::PageType::Worksheet)
            )
    }

    /// Opens a Card or List on the row OnOpenPage left Rec on (see `rec_moved_by_onopenpage`): `=><` from Rec, so a Rec outside
    /// the view it set lands on the nearest row of the view (QrMovedTests.al:253 QR_Card_OpenView_GetThenFilterExcludes `B` for
    /// C with filter A..B, `O:C;R:B;C:B;`). In a key other than the primary key the row was read before (`read_rec_in_its_key`),
    /// and a list reads each row once more (`reads_once_more_in_its_key`). A list's open read goes on above the row (`read_rows_above_the_moved_rec`); its rows then start at the first row of the
    /// view. A view with no row is not pinned.
    pub(super) fn open_on_rec_moved_by_onopenpage(&mut self, st: &St) -> Result<(), Signal> {
        let rec = self.page_rec(st);
        // A temporary source in a key other than the primary key is not pinned: a card keeps the read after the find it had, a
        // list is refused.
        let temp_key = rec.borrow().key != 0 && rec.borrow().temporary;
        let list = self.is_list(st);
        // A temporary Worksheet in a key set before the open (`rec_moved_by_onopenpage` lets only the pinned form through) reads
        // as a list over a database table in such a key does: the row of Rec's primary key first, then each row once more
        // (QrMovedTests.al:2458 QT_TempWs_RunModal_ByName_B `O:B;R:B;R:B;R:A;R:B;R:C;R:D;R:B;R:B;R:B;C:B;`, next `R:A;R:A;R:A;C:A;`,
        // as :345 QR_List_OpenView_SetCurrentKeyGet).
        let temp_ws_key = temp_key && self.temp_worksheet_set_before_open(st);
        if temp_ws_key {
            let mut pk = self.row_key(&rec)?;
            self.ret_used = true;
            if !matches!(self.record_op(RecordOp::Get, &rec, &mut pk)?, Value::Boolean(true)) {
                return unsupported(
                    "a temporary Worksheet opened in a key other than the primary key on a primary key no row has (not pinned)",
                );
            }
            self.populate_current(st)?;
            st.borrow_mut().temp_reads_once_more = true;
        }
        let find = self.declares(st, "onfindrecord");
        if temp_key && list && !temp_ws_key {
            return unsupported(
                "a list over a temporary source opened without a record whose OnOpenPage changed its key and left Rec on a row other than the first of its view (not pinned)",
            );
        }
        // The first row of the view, to tell whether `=><` lands on it (a copy of the record, so the page's own is not moved).
        let first = if list {
            let probe = Rc::new(RefCell::new(rec.borrow().clone()));
            self.ret_used = true;
            match self.record_op(RecordOp::FindFirst, &probe, &mut [])? {
                Value::Boolean(true) => Some(self.row_key(&probe)?),
                _ => None,
            }
        } else {
            None
        };
        // A temporary Worksheet with OnFindRecord whose row was set before the open is asked `=>` (QrMovedTests.al:2586
        // QT_TempWsFind_RunModal_B `O:B;F:=>>B;...`, :2602 _BB_Between `F:=>>C`).
        if !self.nav_find(st, if find { "=>" } else { "=><" })? {
            return unsupported(
                "a page opened without a record whose OnOpenPage left Rec where its view has no row (not pinned)",
            );
        }
        // A list whose Rec is outside the view and lands on its first row opens as a list on its first row does
        // (QrMovedTests.al QS_Where_List_GetA `O:A;R:B;R:C;R:D;R:B;R:B;C:B;` = QS_Where_List_NoMove after `O:`;
        // QS_KeyWhere_List_GetD `O:D;R:D;R:C;R:B;R:A;R:C;R:C;R:C;C:C;`).
        let landed = self.row_key(&rec)?;
        if list && first.as_ref().is_some_and(|f| Self::same_key(f, &landed)) {
            if find || temp_ws_key {
                return unsupported(
                    "a temporary Worksheet whose row was set before it opened, landing on the first row of its view from a row outside it (not pinned)",
                );
            }
            return self.page_first_row(st);
        }
        if temp_key && !temp_ws_key {
            self.populate_current(st)?;
        }
        {
            let mut s = st.borrow_mut();
            s.cursor = Cursor::Row;
            s.landing_first = true;
            s.open_reads_above = list;
        }
        let r = self.arrive(st);
        {
            let mut s = st.borrow_mut();
            s.landing_first = false;
            s.open_reads_above = false;
        }
        r?;
        st.borrow_mut().window_from_start = list;
        st.borrow_mut().opened_on_moved_rec = list;
        Ok(())
    }
}
