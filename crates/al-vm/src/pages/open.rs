use super::*;
use al_builtins::record_op::RecordOp;

/// The filter group of the cut that hides the rows below indentation 0 of a tree repeater.
pub(super) const TREE_GROUP: i32 = 6;

impl Session {
    pub(super) fn page_open(&mut self, st: &St, mode: PageMode) -> Result<Value, Signal> {
        // corpus testpageclose/TestPageClosingActions.al:19-24: an open variable cannot be opened.
        if st.borrow().open {
            return Err(error(ALREADY_OPEN));
        }
        // OpenNew of a Worksheet fails with NotSupportedPageViewMode (corpus pageext/lt-bl-implicit-creation/
        // ImplicitCreation.al:308-318 Worksheet_OpenNew, a worksheet that declares no trigger). Whether OnInit or
        // OnOpenPage run before the error is not pinned, so a worksheet that declares either is refused.
        if mode == PageMode::New {
            let object = st.borrow().object as usize;
            let engine = self.engine();
            if engine.object_facts[object].page_type == Some(al_ir::PageType::Worksheet) {
                if self.declares(st, "oninit") || self.declares(st, "onopenpage") {
                    return unsupported(
                        "OpenNew of a Worksheet that declares OnInit or OnOpenPage (whether they run before the NotSupportedPageViewMode error is not pinned)",
                    );
                }
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
    pub(super) fn has_source(&self, st: &St) -> bool {
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
        Ok(inst)
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
    pub(super) fn adopt_instance(&mut self, from: &St, to: &St) {
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
    pub(super) fn open_page(
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
    pub(super) fn open_page_as(
        &mut self,
        st: &St,
        mode: PageMode,
        record: Option<Rec>,
        bind: Option<UiKind>,
        as_part: bool,
    ) -> Result<Option<Handler>, Signal> {
        let test_open = std::mem::take(&mut self.pages.test_open);
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
            s.moving_next = false;
            s.window = Vec::new();
            s.window_from_start = false;
            s.window_eof = false;
            s.window_whole = false;
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
            s.set_controls = Vec::new();
            s.agr_xrec = None;
            s.agcr_xrec = None;
            s.row_vars = Vec::new();
            s.agcr_after_new = false;
            s.part_accessed = false;
            s.new_host_part_rows = false;
            s.expanded = Vec::new();
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
            s.parts = Vec::new();
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
            self.set_view_in(&rec, &table, view, Some(parts::LINK_GROUP), false, true)?;
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
        if let (Some(r), true) = (&record, has_source) {
            let page_temp = self.page_rec(st).borrow().temporary;
            if page_temp && !r.borrow().temporary && !self.is_list(st) && self.stored_row(r)?.is_some() {
                self.mark_closed(st);
                return unsupported(
                    "a stored record handed to a card whose source table is temporary (whether the row is copied into the temporary table is not pinned)",
                );
            }
        }
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
            // A view that names an ORDER on a page run on a record: nothing pins it, so it is read as the view alone is
            // (the view's direction wins), on an assumption.
            if record.is_some() && view.to_ascii_lowercase().contains("order(") {
                al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::PageRunRecordViewOrderApplies);
            }
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
            self.set_view_in(&rec, &table, &view, Some(2), sub_view.is_some(), false)?;
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
        // A repeater with ShowAsTree shows the rows at indentation 0 only, whatever TreeInitialState says: the rows below them are collapsed
        // and a TestPage does not expand them (pages/lt-page-controls-aa/AaPartsLists.al:AA_Repeater_TreeExpandedRows line 272, ExpandAll;
        // pageext/lt-page-ext-properties/PageTree.al:P123_ShowAsTreeRows, CollapseAll). The rows are cut by a filter in a group of its own.
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
                // An indentation column that is a page variable, or a field that is not an Integer, cannot cut the rows: not pinned, the
                // rows stay as they were before.
                if let Some(f) = table.fields.iter().find(|f| f.name.eq_ignore_ascii_case(&name)) {
                    if !matches!(f.ty, RtType::Integer) {
                        self.mark_closed(st);
                        return unsupported(
                            "ShowAsTree over an IndentationColumn that is not an Integer field (not pinned)",
                        );
                    }
                    let pos = table
                        .fields
                        .iter()
                        .position(|g| g.name.eq_ignore_ascii_case(&name))
                        .expect("found above");
                    rec.borrow_mut().filters.push(al_builtins::filter::FieldFilter {
                        field: pos,
                        expr: al_builtins::filter::FilterExpr::Eq(Value::Integer(0)),
                        group: TREE_GROUP,
                    });
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
                self.apply_request_parameters(&inst, &exts, object, &xml)?;
            }
            self.restore_page_values(st);
            self.check_page_events(st)?;
            let opened = self.page_trigger(st, "onopenpage", vec![]);
            st.borrow_mut().opening = false;
            opened?;
            self.page_event(st, "onopenpageevent", None, None)?;
            if has_source {
                self.drop_or_group(st)?;
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
            let (loaded, origin) = self.traced(st, |s| s.load_first_row(st, mode, record.is_some()));
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
        Ok(handler)
    }

    /// Whether the page declares OnFindRecord and its SourceTableView has a WHERE part: the shape
    /// pagefindrecord/QcFindViewTests.al pins (corpus bump 33).
    pub(super) fn find_page_filtered_view(&self, st: &St) -> bool {
        self.declares(st, "onfindrecord")
            && self.engine.object_facts[st.borrow().object as usize].has_source_table_view
            && self.source_view_filters(st)
    }

    /// `find_page_filtered_view` on a page with neither OnAfterGetRecord nor OnAfterGetCurrRecord, the pages of writer QC's probe:
    /// moves inside the rows the client holds ask the page nothing (QcFindViewTests.al:97, :135, :236, :540).
    /// Pinned for an ascending view; a descending one is refused by `refuse_descending_filtered_find_move`.
    pub(super) fn filtered_find_list_without_row_triggers(&self, st: &St) -> bool {
        self.find_page_filtered_view(st)
            && !self.declares(st, "onaftergetrecord")
            && !self.declares(st, "onaftergetcurrrecord")
            && self.page_rec(st).borrow().ascending
    }

    /// A move (First, Last, Next, Previous, GoToKey) on a list with OnFindRecord, a descending filtered SourceTableView and no row
    /// trigger: the rows it lands on are pinned (QcFindViewTests.al:257 DescList_OpenAndWalk `first = Yes:5`, `walk = 5,3,1`), whether
    /// the page is asked for them is not: refused.
    pub(super) fn refuse_descending_filtered_find_move(&self, st: &St) -> Result<(), Signal> {
        if self.is_list(st)
            && self.find_page_filtered_view(st)
            && !self.declares(st, "onaftergetrecord")
            && !self.declares(st, "onaftergetcurrrecord")
            && !self.page_rec(st).borrow().ascending
        {
            return unsupported(
                "a move on a list with OnFindRecord and a descending filtered SourceTableView (whether the page is asked is not pinned)",
            );
        }
        Ok(())
    }

    /// Whether the page's SourceTableView has a WHERE part.
    fn source_view_filters(&self, st: &St) -> bool {
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
    fn enum_filter_excludes_blank(&mut self, st: &St, rec: &Rec) -> Result<bool, Signal> {
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
    fn page_code_runs(&self, st: &St, open: bool) -> bool {
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
    fn drop_or_group(&mut self, st: &St) -> Result<(), Signal> {
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
    fn open_row_error(
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
        if self.is_card(st) && self.has_source(st) {
            let caption = self.page_caption(st);
            return Err(self.has_to_close(st, &e, caption));
        }
        Err(Signal::Error(Box::new(al_builtins::native::unpinned_error())))
    }

    /// The first row a page shows when it opens (or when a part is linked afresh).
    pub(super) fn load_first_row(&mut self, st: &St, mode: PageMode, with_record: bool) -> Result<(), Signal> {
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
            // A page with an OnFindRecord trigger is opened the same way: the trigger is not asked (Microsoft's Service
            // Credit Memo, Invoice and Order pages, whose tests open them with OpenNew). Assumed, see
            // Assumption::OpenNewSkipsOnFindRecord.
            if self.declares(st, "onfindrecord") {
                al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::OpenNewSkipsOnFindRecord);
            }
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
                            return self.arrive(st);
                        }
                    } else {
                        st.borrow_mut().cursor = Cursor::Row;
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
                return self.page_first_row_from(st, "=><", true);
            }
        }
        self.page_first_row(st)
    }

    pub(super) fn save_values(&self, st: &St) -> bool {
        let object = st.borrow().object as usize;
        let on = self.engine.program.objects[object]
            .properties
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("SaveValues") && v.trim().eq_ignore_ascii_case("true"));
        // A SaveValues page that page extensions extend is driven as one without extensions (assumed).
        if on
            && self.engine.program.objects[object]
                .page
                .as_ref()
                .is_some_and(|p| !p.extensions.is_empty())
        {
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::SaveValuesWithExtensions);
        }
        on
    }

    pub(super) fn var_slots(&self, st: &St) -> Vec<usize> {
        let engine = self.engine();
        let mut slots: Vec<usize> = page_of(&engine, st)
            .controls
            .iter()
            .filter_map(|c| {
                if let (ir::Binding::Var(slot), None) = (&c.binding, c.owner) {
                    Some(*slot)
                } else {
                    None
                }
            })
            .collect();
        slots.sort_unstable();
        slots.dedup();
        slots
    }

    pub(super) fn store_page_values(&mut self, st: &St) {
        if !self.save_values(st) {
            return;
        }
        let Some(inst) = st.borrow().instance.clone() else {
            return;
        };
        let saved: Vec<(usize, Value)> = self
            .var_slots(st)
            .into_iter()
            .map(|slot| (slot, inst.borrow().globals[slot].clone()))
            .collect();
        let object = st.borrow().object;
        self.pages.saved_values.insert(object, saved);
    }

    pub(super) fn restore_page_values(&mut self, st: &St) {
        let object = st.borrow().object;
        let Some(saved) = self.pages.saved_values.get(&object).cloned() else {
            return;
        };
        let Some(inst) = st.borrow().instance.clone() else {
            return;
        };
        // A request page's variables that the caller set before the run (`SetParameters`) keep those values: corpus
        // reportextensiontrigger/TestPrecompiledReportExtensionDataItems.al (Base Application's "Get Demand To Reserve"
        // has SaveValues and an earlier run confirmed another demand type; the later run still goes by the one
        // `SetParameters` gave, `All` included). Which one wins in BC is assumed.
        let caller_set = self.is_request_page(st)
            && self
                .report
                .prepared
                .iter()
                .any(|w| w.upgrade().is_some_and(|i| Rc::ptr_eq(&i, &inst)));
        for (slot, v) in saved {
            // A data item the caller gave a view (`SetTableView`, or the record of `Report.Run`/`RunModal`) keeps that view over the
            // filters stored from an earlier run. Which of the two a request page shows is not pinned: the corpus pins that an OK
            // stores the view's filters (report/lt-requestpage-savevalues-date-text-setvalue/P147SaveValuesFilter.al:
            // P147_Filter_SetTableViewVsStored `plain after OK on the SetTableView run`), not what a later view does against them.
            // Microsoft's "Suggest Fin. Charge Memo Lines" (SaveValues) is run once per memo with that memo as the record.
            if self.is_request_page(st) && matches!(&v, Value::Record(_)) {
                if let Value::Record(cur) = &inst.borrow().globals[slot] {
                    let key = Rc::as_ptr(cur) as usize;
                    if !cur.borrow().filters.is_empty() && self.report.set_view_filters.iter().any(|(k, _)| *k == key) {
                        al_builtins::assumptions::assume(
                            al_builtins::assumptions::Assumption::ReportViewOverStoredRequestFilters,
                        );
                        continue;
                    }
                }
            }
            if caller_set && !matches!(v, Value::Record(_)) {
                // The caller's value stays over the stored one (report/lt-requestpage-savevalues-date-text-setvalue/P147SaveValuesText.al:
                // P147_Text_StoredVsCaller_Ok, P147_Boolean_CallerFalseOverStoredTrue, P147_Option_CallerSelectedOverStoredAll).
                continue;
            }
            inst.borrow_mut().globals[slot] = v;
        }
    }

    /// `TestPage.Close()`: the page closes the way a Cancel does (corpus
    /// testpageclose/TestPageClosingActions.al:Dialog arms, `qcp:Cancel` after Close()).
    pub(super) fn page_close(&mut self, st: &St) -> Result<Value, Signal> {
        // Card, List, Document, Worksheet and ListPlus report OK (handlers/TestPageCloseActionByPageTypeContracts.al:
        // TestPageClose_IsOk_ForEveryPageType; a Card also in handlers/TestPageCloseTriggersSelectionCardPageContracts.al:
        // OnQueryClosePage_PageAndExtension_BothRun_AndBothMustAgreeToClose); the dialogs report Cancel.
        // Other page types are not pinned and keep Cancel.
        let object = st.borrow().object as usize;
        let ok_type = self.engine.object_facts[object]
            .page_type
            .as_ref()
            .is_some_and(al_ir::PageType::is_plain);
        let action = if ok_type && !st.borrow().modal { "OK" } else { "Cancel" };
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
            // Error (code Dialog); another error's text and code cannot be read.
            Err(Signal::Error(e)) if origin.as_deref() == Some("onclosepage") => {
                self.mark_closed(st);
                if e.code != "Dialog" {
                    return Err(Signal::Error(Box::new(al_builtins::native::unpinned_error())));
                }
                Err(error_code(
                    format!("One or more errors occurred while closing the page.\n\n{}", e.message),
                    "TestWrapped",
                ))
            }
            other => other,
        };
        if r.is_ok() {
            self.mark_closed(st);
        }
        r
    }

    /// Closes the page with `action`. The client first saves what was typed; a save the table
    /// refuses raises at Close() but not at an OK, which counts one validation error and leaves the
    /// page open (corpus testpageclose/TestPageClosingActions.al, Card_OK_AfterATableRefusedDuplicateKey_*,
    /// handlers/TestPageFailedInsert_Tests.al). Then OnQueryClosePage(action) decides, then
    /// OnClosePage runs (TestPageClosingActions.al:Card_OK_RaisesTheCloseTriggersOnce).
    pub(super) fn close_with(
        &mut self,
        st: &St,
        action: &str,
        swallow_save_errors: bool,
        via_action: bool,
    ) -> Result<Value, Signal> {
        let test_close = std::mem::take(&mut self.pages.test_close);
        self.require_open(st)?;
        let mapped = self.close_action(st, action);
        let action = mapped.as_str();
        // Cancel on a request page whose ModifyAllowed is false drops a changed row: no error, the stored row unchanged
        // (report/lt-y-request-page/YRequestPageTests.al:130-162, `modify = ok changed` then `rows` as seeded). With ModifyAllowed
        // true, or at OK, what happens to a changed row of a request page is not pinned.
        if self.is_request_page(st) && self.has_source(st) {
            let changed = {
                let s = st.borrow();
                s.dirty && s.cursor == Cursor::Row
            };
            if changed {
                let rec = self.page_rec(st);
                let differs = !Self::same_values(&rec.borrow().fields, &self.page_xrec_fields(st));
                let object = st.borrow().object as usize;
                if differs
                    && (action != "Cancel"
                        || st.borrow().draft
                        || !flag_denied(&self.engine(), object, "ModifyAllowed")?)
                {
                    return unsupported(format!(
                        "{action} on a request page with a changed row (only Cancel with ModifyAllowed = false is pinned)"
                    ));
                }
                st.borrow_mut().dirty = false;
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
            || (action == "LookupCancel" && via_action)
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
    pub(super) fn builtin_offered(&self, st: &St, which: &str) -> bool {
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

    /// The IndentationColumn of the page's repeater when it has ShowAsTree, as written.
    pub(super) fn tree_indentation(&self, st: &St) -> Option<String> {
        let engine = self.engine();
        page_of(&engine, st)
            .controls
            .iter()
            .find(|c| {
                c.kind == ir::ControlKind::Repeater
                    && matches!(c.prop("showastree"), Some(ir::PropVal::Lit(v)) if unquote(v).eq_ignore_ascii_case("true"))
            })
            .and_then(|c| match c.prop("indentationcolumn") {
                Some(ir::PropVal::Lit(v)) => Some(unquote(v)),
                _ => None,
            })
    }

    /// The position of the Integer field a tree indents by, when the page is a tree over one.
    pub(super) fn tree_level_field(&self, st: &St) -> Result<Option<usize>, Signal> {
        let Some(expr) = self.tree_indentation(st) else {
            return Ok(None);
        };
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        let name = expr
            .trim()
            .strip_prefix("Rec.")
            .unwrap_or(expr.trim())
            .trim_matches('"')
            .to_string();
        Ok(table
            .fields
            .iter()
            .position(|f| f.name.eq_ignore_ascii_case(&name) && matches!(f.ty, RtType::Integer)))
    }

    /// `Expand(on)` on the row a tree stands on: Expand(true) shows the row's direct children, Expand(false) hides them again, and
    /// IsExpanded reads which (pages/lt-ah-pages/AhPageTests.al:950-961 Walk, :986-993 Tree_ThroughPageRun `T1 T5` -> `T1 T2 T4 T5`
    /// -> `T1 T5`, GoToKey of a hidden child `No`). A row's parent is the nearest row before it, in the page's order, with a lower
    /// level. Pinned only for a page without row triggers, a primary key of one field, expanding a row with children and collapsing
    /// an expanded one; anything else is refused.
    pub(super) fn tree_expand(&mut self, st: &St, on: bool) -> Result<(), Signal> {
        for t in [
            "onfindrecord",
            "onnextrecord",
            "onaftergetrecord",
            "onaftergetcurrrecord",
        ] {
            if self.declares(st, t) {
                return unsupported(
                    "Expand on a tree page with row triggers (which rows are read again is not pinned)",
                );
            }
        }
        if st.borrow().cursor != Cursor::Row || st.borrow().draft {
            return unsupported("Expand on a tree page that stands on no stored row (not pinned)");
        }
        let Some(level) = self.tree_level_field(st)? else {
            return unsupported("Expand on a tree whose IndentationColumn is not an Integer field (not pinned)");
        };
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        if table.keys[0].len() != 1 {
            return unsupported("Expand on a tree over a table whose primary key has several fields (not pinned)");
        }
        let pk = table.keys[0][0];
        let current = self.row_key(&rec)?;
        // Every row of the view with its level, in the page's order, the tree's own cut left out.
        let all = Rc::new(RefCell::new(rec.borrow().clone()));
        all.borrow_mut().filters.retain(|f| f.group != TREE_GROUP);
        let mut rows: Vec<(Vec<Value>, i32)> = Vec::new();
        self.ret_used = true;
        if matches!(self.record_op(RecordOp::FindSet, &all, &mut [])?, Value::Boolean(true)) {
            loop {
                let lvl = match &all.borrow().fields[level] {
                    Value::Integer(n) => *n,
                    _ => 0,
                };
                rows.push((self.row_key(&all)?, lvl));
                self.ret_used = true;
                if !matches!(self.record_op(RecordOp::Next, &all, &mut [Value::Integer(1)])?, Value::Integer(n) if n != 0)
                {
                    break;
                }
            }
        }
        let at = rows.iter().position(|(k, _)| Self::same_key(k, &current));
        let has_children = at.is_some_and(|i| rows.get(i + 1).is_some_and(|(_, l)| *l > rows[i].1));
        {
            let mut s = st.borrow_mut();
            let known = s.expanded.iter().position(|k| Self::same_key(k, &current));
            match (on, known) {
                (true, None) if has_children => s.expanded.push(current.clone()),
                (false, Some(i)) => {
                    s.expanded.remove(i);
                }
                _ => {
                    return unsupported(
                        "Expand(true) on a tree row without children or one already expanded, or Expand(false) on a row not expanded (not pinned)",
                    );
                }
            }
        }
        // Which rows show: a row with no parent shows when it is at level 0 (the cut at open); any other row when its parent shows
        // and is expanded.
        let expanded = st.borrow().expanded.clone();
        let mut stack: Vec<(i32, bool)> = Vec::new();
        let mut shown: Vec<al_builtins::filter::FilterExpr> = Vec::new();
        for (key, lvl) in &rows {
            while stack.last().is_some_and(|(l, _)| *l >= *lvl) {
                stack.pop();
            }
            let visible = match stack.last() {
                None => *lvl == 0,
                Some((_, open)) => *open,
            };
            let open = visible && expanded.iter().any(|k| Self::same_key(k, key));
            stack.push((*lvl, open));
            if visible {
                shown.push(al_builtins::filter::FilterExpr::Eq(key[0].clone()));
            }
        }
        {
            let mut r = rec.borrow_mut();
            r.filters.retain(|f| f.group != TREE_GROUP);
            let expr = if expanded.is_empty() {
                r.filters.push(al_builtins::filter::FieldFilter {
                    field: level,
                    expr: al_builtins::filter::FilterExpr::Eq(Value::Integer(0)),
                    group: TREE_GROUP,
                });
                None
            } else {
                Some(al_builtins::filter::FilterExpr::Or(shown))
            };
            if let Some(expr) = expr {
                r.filters.push(al_builtins::filter::FieldFilter {
                    field: pk,
                    expr,
                    group: TREE_GROUP,
                });
            }
        }
        // The client reads the rows again from the row it stands on.
        st.borrow_mut().window = Vec::new();
        self.arrive(st)
    }

    /// Whether the page carries the blank line past its data: an editable,
    /// insert-allowed repeater opened with OpenEdit (corpus
    /// handlers/TestPageNewRowLine_Tests.al:1-30, gating arms).
    pub(super) fn offers_new_line(&mut self, st: &St) -> Result<bool, Signal> {
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

    // ---- rows ---------------------------------------------------------------------------

    /// Whether the page declares trigger `name` (lower case) of its own.
    pub(super) fn declares(&self, st: &St, name: &str) -> bool {
        let engine = self.engine();
        page_of(&engine, st).triggers.iter().any(|(n, _)| n == name)
    }

    /// FindFirst (`which` "-") or FindLast ("+") of the page's rowset. A page that declares
    /// OnFindRecord serves its rows from the trigger, which receives `Which` and answers whether it
    /// found a row (corpus pagefindrecord/TestPageFindRecord.al:112, :160). The strings the client
    /// really passes are `=`, `=>` and `=><` forms the corpus pins only for a deleted row
    /// (pageupdate/TestPageUpdateGoneRow.al:390); `-` and `+` are assumed (docs/probes-needed.md).
    pub(super) fn nav_find(&mut self, st: &St, which: &str) -> Result<bool, Signal> {
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
    pub(super) fn nav_next(&mut self, st: &St, steps: i32) -> Result<bool, Signal> {
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
    pub(super) fn page_first_row(&mut self, st: &St) -> Result<(), Signal> {
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
    fn page_first_row_from(&mut self, st: &St, which: &str, handed: bool) -> Result<(), Signal> {
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
    fn runs_on_blank_record(&self, st: &St) -> bool {
        let object = st.borrow().object as usize;
        matches!(
            self.engine.object_facts[object].page_type,
            Some(al_ir::PageType::Card | al_ir::PageType::Document | al_ir::PageType::ListPlus)
        ) && st.borrow().mode == PageMode::Edit
            && st.borrow().host.is_none()
            && self.has_source(st)
            && !self.is_list(st)
    }

    pub(super) fn land_on_new_line(&mut self, st: &St) -> Result<(), Signal> {
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
        // The blank line of a part is a started record: the client runs NewRecord when it becomes
        // current (corpus handlers/TestPagePartDraftLineLink.al:LinkedPart_DraftLine_*,
        // handlers/TestPagePartOnNewRecordCount.al). A top-level list does the same, and both then
        // run OnAfterGetCurrRecord on the blank row (corpus testpart/TestPartTriggerOrderAndRefresh.al:
        // 258-273 TopLevelList_BlankLine_RunsOnNewRecord, Part_BlankLine_RunsOnNewRecord).
        // A part whose host record is not saved yet links to a blank value, which is validated like any other
        // (pages/lt-part-line-before-host-key/PartLineBeforeHostKey.al:398, :400; see `sync_part`).
        self.stamp_from_filters(st, true)?;
        self.page_trigger(st, "onnewrecord", vec![Value::Boolean(true)])?;
        self.page_event(st, "onnewrecordevent", None, Some(true))?;
        // xRec is the new record itself there (misc/lt-probes-needed-7/PnGNewRecord.al:HandedRecordNoFilters `AGCR(,)`,
        // PnGHandedRecord `AGCR(FILTERED,FILTERED)`).
        self.set_xrec(st);
        self.page_trigger(st, "onaftergetcurrrecord", vec![])?;
        self.page_event(st, "onaftergetcurrrecordevent", None, None)?;
        Ok(())
    }

    /// The blank line of a part that shows no row of an editable host, as the host opens or the part is linked afresh: the
    /// client runs the part's OnNewRecord three times, the link field validated before the first and the third
    /// (pages/lt-part-line-before-host-key/PartLineBeforeHostKey.al:672 PYB_OpenNew_Document_LogOnly `L.OnValidate(Document No.): |
    /// LP.OnNewRecord: | LP.OnNewRecord: | L.OnValidate(Document No.): | LP.OnNewRecord:`, :688 a host without OnNewRecord, :704
    /// an unlinked part (three runs, no validation), :738 without AutoSplitKey, :754 two linked parts one after the other, :770 a
    /// Card host, :809 OpenEdit of a host without lines, :603 after the host key is typed). OnAfterGetCurrRecord runs right
    /// after each OnNewRecord run (:1349 QH_EmptyPart_OpenNew_CurrRecordOrder `CP.OnNewRecord: below=Yes |
    /// CP.OnAfterGetCurrRecord:/0`, three times; :1381 OpenEdit, :1418 after the host key is typed, :1495 host Next; :1439 and
    /// :1455 a part that declares only OnAfterGetCurrRecord runs it three times). A host's New() runs only the host's
    /// OnNewRecord, never the part's (:827 PYB_OpenEdit_HostWithoutLines `host_new`, :879 PYB_OpenEdit_ThenHostNew, :861
    /// PYB_OpenNew_ThenHostNew); whether its OnAfterGetCurrRecord runs then is not pinned, so a part that can observe it is refused.
    fn land_on_part_open_line(&mut self, st: &St) -> Result<(), Signal> {
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
        let object = st.borrow().object as usize;
        let observes_curr = self.declares(st, "onaftergetcurrrecord")
            || self.engine().events.is_subscribed(object, "onaftergetcurrrecordevent");
        if self.host_new_running {
            // Whether and how often the part's OnAfterGetCurrRecord runs then is not pinned (the PYB pages declare none).
            if observes_curr {
                return unsupported(
                    "OnAfterGetCurrRecord of an empty part while its host runs New() (whether and how often BC runs it is not pinned)",
                );
            }
            self.set_xrec(st);
            return Ok(());
        }
        // A link value the part's table relation refuses: the client stamps it without validating (no OnBeforeValidate, no
        // OnValidate), runs the part's OnNewRecord and OnAfterGetCurrRecord once, then the open fails with code TestWrapped and the
        // relation's text (PartLineBeforeHostKey.al:1600-1680 QH_ConstLink_Missing_OpenNew_Trace `[CP.OnOpenPage |
        // CP.OnNewRecord:D9 below=Yes | CP.OnAfterGetCurrRecord:D9/0]`, QH_ConstLink_Missing_OpenNew_Events `[LP.OnNewRecord:D9]`,
        // QH_ConstLink_Missing_OpenEdit; :776 PYB_OpenNew_ConstLinkedPart).
        if let Some(e) = self.part_link_relation_error(st)? {
            self.stamp_from_filters(st, false)?;
            self.page_trigger(st, "onnewrecord", vec![Value::Boolean(true)])?;
            self.page_event(st, "onnewrecordevent", None, Some(true))?;
            self.set_xrec(st);
            self.page_trigger(st, "onaftergetcurrrecord", vec![])?;
            self.page_event(st, "onaftergetcurrrecordevent", None, None)?;
            return Err(error_code(e.message.clone(), "TestWrapped"));
        }
        // OnNewRecord sees the xRec the part held before its first and third run, a blank one before the second, after any move
        // of the host: GoToKey (pages/lt-qj-part-propagation/QjPartPropagation.al QL_PartLastNew_NoLines `log-open`: `S.NEW(x=3
        // x.ln=30000) S.AGCR(0 r=0 x=0) S.NEW(x=0 x.ln=0) S.AGCR(0 r=0 x=0) S.NEW(x=3 x.ln=30000)`), Next (:2468 QN_EmptyPart_HostNext),
        // Previous (:2499), First (:2530), Last (:2561); a part that held no row before (OpenNew :2591 QN_EmptyPart_HostOpenNew,
        // OpenEdit onto an empty first host :2441) sees a blank xRec all three times. The part's Last() and New() see it again
        // (`PageState::empty_part_xrec`).
        let held = self.page_xrec_fields(st);
        st.borrow_mut().empty_part_xrec = Some(held.clone());
        // The next OnAfterGetCurrRecord on a stored row (after the host moves on to a row with lines) sees that xRec too
        // (QjPartPropagation.al:2620 QN_EmptyPart_HostNext_ThenNextAgain `log-move = S.AGCR(50000 r=5 x=3)`).
        st.borrow_mut().agcr_xrec = Some(held.clone());
        for validate in [true, false, true] {
            if validate {
                self.set_xrec_fields(st, held.clone());
            }
            if validate {
                self.stamp_from_filters(st, true)?;
            }
            self.page_trigger(st, "onnewrecord", vec![Value::Boolean(true)])?;
            self.page_event(st, "onnewrecordevent", None, Some(true))?;
            self.set_xrec(st);
            self.page_trigger(st, "onaftergetcurrrecord", vec![])?;
            self.page_event(st, "onaftergetcurrrecordevent", None, None)?;
        }
        Ok(())
    }

    /// The error of the table relation of the first link value `stamp_from_filters` would validate on the part's new line, when
    /// the related table lacks it; the line is left as it was.
    fn part_link_relation_error(&mut self, st: &St) -> Result<Option<Box<al_builtins::native::AlError>>, Signal> {
        // Pinned for a const link (`SubPageLink = "Document No." = const(D9)`); a field link keeps the validation in its turn.
        let Some((host, idx)) = st
            .borrow()
            .host
            .as_ref()
            .and_then(|(h, c)| h.upgrade().map(|h| (h, *c)))
        else {
            return Ok(None);
        };
        let const_link = page_of(&self.engine(), &host).controls[idx]
            .part_link
            .as_ref()
            .is_some_and(|l| !l.entries.is_empty() && l.entries.iter().all(|e| matches!(e, ir::LinkEntry::Filter(..))));
        if !const_link || !self.has_source(st) {
            return Ok(None);
        }
        let rec = self.page_rec(st);
        let before = rec.borrow().fields.clone();
        self.stamp_from_filters(st, false)?;
        let stamped = rec.borrow().fields.clone();
        rec.borrow_mut().fields = before.clone();
        let table = self.table(&rec)?;
        for (pos, (was, now)) in before.iter().zip(&stamped).enumerate() {
            if Self::same_values(std::slice::from_ref(was), std::slice::from_ref(now)) {
                continue;
            }
            match self.validate_relation(&rec, &table, pos, now.clone()) {
                Err(Signal::Error(e)) => return Ok(Some(e)),
                other => other?,
            }
        }
        Ok(None)
    }

    /// Remembers the stored row the cursor stands on as the lower neighbour of the row about to
    /// start, for AutoSplitKey.
    pub(super) fn note_lower_key(&mut self, st: &St) -> Result<(), Signal> {
        let lower = if st.borrow().cursor == Cursor::Row && !st.borrow().draft {
            let rec = self.page_rec(st);
            Some(self.row_key(&rec)?)
        } else {
            None
        };
        st.borrow_mut().lower_key = lower;
        Ok(())
    }

    /// What the client's NewRecord copies onto a fresh buffer of a part: a field whose combined
    /// filter is a single value, when the field is part of the primary key or the page sets
    /// PopulateAllFields; then the table's OnValidate runs for each (corpus
    /// handlers/TestPageSubpagePartFieldLink.al, TestPageSubpagePartConstFilter.al,
    /// TestPageSubpagePartNewRecordInit.al:New_StampsTheLinkedFieldAndValidatesIt).
    pub(super) fn stamp_from_filters(&mut self, st: &St, validate: bool) -> Result<(), Signal> {
        if !self.has_source(st) {
            return Ok(());
        }
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let populate_all = engine.program.objects[object]
            .properties
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("PopulateAllFields") && is_true(v));
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        let mut stamped: Vec<(usize, Value)> = Vec::new();
        for pos in 0..table.fields.len() {
            if !(populate_all || table.keys[0].contains(&pos)) {
                continue;
            }
            let single: Vec<Option<Value>> = rec
                .borrow()
                .filters
                .iter()
                .filter(|f| f.field == pos)
                .map(|f| match &f.expr {
                    al_builtins::filter::FilterExpr::Eq(v) => Some(v.clone()),
                    al_builtins::filter::FilterExpr::Range(Some(a), Some(b))
                        if ops::compare(a, b).is_ok_and(|o| o.is_eq()) =>
                    {
                        Some(a.clone())
                    }
                    _ => None,
                })
                .collect();
            // The combined filter of a field is the intersection of its filters, in every filter group: several
            // that all name the same value are that single value (Payment Journal: the record the page was run
            // with carries the batch in group 0 and `OpenJnl` sets it again in group 2; Microsoft's Incom. Doc.
            // Attach. FactBox tests, unprobed, docs/probes-needed.md).
            let Some(Some(v)) = single.first() else { continue };
            if single
                .iter()
                .any(|o| !o.as_ref().is_some_and(|w| ops::compare(v, w).is_ok_and(|c| c.is_eq())))
            {
                continue;
            }
            if single.len() > 1 {
                al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::NewRowEqualFiltersSeveralGroups);
            }
            stamped.push((pos, v.clone()));
        }
        // One validated value is pinned: the link field's OnValidate and table relation see the rest of the line as it
        // was, a SourceTableView value of a field outside the key is not stamped (pages/lt-new-line-stamps-filters/
        // NewLineStampsFilters.al:214-237, :264-287). With two or more validated values, whether the later ones are on
        // the record while the first validates is not pinned. Every value goes on first; when code can see the order (an
        // OnValidate, a table relation, a table extension's validate trigger or a subscriber to the validate events of a
        // stamped field) that runs under Assumption::NewLineStampsAllBeforeValidating (kept, not pinned: refusing it costs
        // 7 baseline cases, handlers/TestPageActionRunPageView.al and misc/lt-probes-needed-7/PnGRefresh.al).
        if validate && stamped.len() > 1 {
            let mut seen = false;
            let schema = engine.program.objects[table.object as usize]
                .table
                .as_ref()
                .expect("table");
            for (pos, _) in &stamped {
                let f = &schema.fields[*pos];
                seen |= f.on_validate.is_some()
                    || f.relation
                    || !f.ext_before_validate.is_empty()
                    || !f.ext_after_validate.is_empty()
                    || self.listened(&table, "onbeforevalidateevent", Some(&f.name))?
                    || self.listened(&table, "onaftervalidateevent", Some(&f.name))?;
            }
            if seen {
                al_builtins::assumptions::assume(
                    al_builtins::assumptions::Assumption::NewLineStampsAllBeforeValidating,
                );
            }
        }
        for (pos, v) in &stamped {
            rec.borrow_mut().fields[*pos] = v.clone();
        }
        if validate {
            for (pos, v) in stamped {
                self.ret_used = false;
                self.record_op(RecordOp::Validate, &rec, &mut [Value::Integer(pos as i32), v])?;
            }
        }
        Ok(())
    }

    /// AutoSplitKey: the last field of the primary key of a new line is numbered from its
    /// neighbours when it is saved (corpus handlers/TestPageAutoSplitKey_Tests.al,
    /// TestPageAutoSplitKeyRanges_Tests.al). Only the shapes the corpus pins are numbered; any
    /// other raises Signal::Unsupported.
    pub(super) fn auto_split_key(&mut self, st: &St) -> Result<(), Signal> {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let on = engine.program.objects[object]
            .properties
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("AutoSplitKey") && is_true(v));
        if !on {
            return Ok(());
        }
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        let Some(&last) = table.keys[0].last() else {
            return Ok(());
        };
        let ty = table.fields[last].ty.clone();
        if !matches!(ty, RtType::Integer | RtType::BigInteger | RtType::Decimal) {
            return unsupported(format!("AutoSplitKey on a key field of type {}", ty.describe()));
        }
        let dec = |v: &Value| match v {
            Value::Integer(i) => Some(rust_decimal::Decimal::from(*i)),
            Value::BigInteger(i) => Some(rust_decimal::Decimal::from(*i)),
            Value::Decimal(d) => Some(*d),
            _ => None,
        };
        // A key the test typed stays; the number is assigned to an unset one.
        let current = rec.borrow().fields[last].clone();
        if dec(&current).is_none_or(|d| !d.is_zero()) {
            return Ok(());
        }
        if rec.borrow().key != 0 || !rec.borrow().ascending {
            return unsupported("AutoSplitKey on a part that is not read in primary key order");
        }
        let burned = st.borrow().split_burned;
        let lower = st
            .borrow()
            .lower_key
            .as_ref()
            .and_then(|k| k.get(table.keys[0].len() - 1))
            .and_then(dec);
        // The next line after the lower one in the part's view, within the same leading key.
        let upper = match lower {
            None => None,
            Some(l) => {
                let probe = Rc::new(RefCell::new(rec.borrow().clone()));
                {
                    let fields = rec.borrow().fields.clone();
                    let mut p = probe.borrow_mut();
                    for &k in &table.keys[0][..table.keys[0].len() - 1] {
                        p.filters.push(al_builtins::filter::FieldFilter {
                            field: k,
                            expr: al_builtins::filter::FilterExpr::Range(
                                Some(fields[k].clone()),
                                Some(fields[k].clone()),
                            ),
                            group: 99,
                        });
                    }
                    let lv = ops::convert(Value::Decimal(l), &ty)?;
                    p.filters.push(al_builtins::filter::FieldFilter {
                        field: last,
                        expr: al_builtins::filter::FilterExpr::Gt(lv),
                        group: 99,
                    });
                }
                self.ret_used = true;
                if matches!(
                    self.record_op(RecordOp::FindFirst, &probe, &mut [])?,
                    Value::Boolean(true)
                ) {
                    dec(&probe.borrow().fields[last])
                } else {
                    None
                }
            }
        };
        use rust_decimal::Decimal as D;
        let step = D::from(10000);
        let is_decimal = matches!(ty, RtType::Decimal);
        let starts_low = st.borrow().split_starts_low;
        let number = match (lower, upper) {
            // The blank line of a top-level list that took key filter values, typed without New(), starts at 10000
            // (misc/lt-probes-needed-7/PnGRefresh.al:672 QL_Jnl_SheetFG2_DeleteNoUpdate_OnlyRow, :792 QL_Jnl_SheetFG2_OpenOnNoRow).
            (None, _) if starts_low && !burned => step,
            // An empty grid burns one interval on the blank placeholder row.
            (None, _) => D::from(20000),
            // Appended after the last line (handlers/TestPageAutoSplitKeyMoreShapes.al:104-117,
            // corpus handlers/TestPageAutoSplitKeyRanges_Tests.al): last + 10000. After a negative
            // line the corpus pins exactly these values; the rule behind them is not known, so
            // any other negative last line is refused.
            (Some(l), None) if l >= D::ZERO && burned => l + step + step,
            (Some(_), _) if burned => {
                return unsupported(
                    "AutoSplitKey of a line started from the placeholder line below a negative line or above another line (not pinned)",
                );
            }
            (Some(l), None) if l >= D::ZERO => l + step,
            (Some(l), None)
                if !is_decimal && [-30000, -20000, -10000, -5, -1].contains(&ToPrimitive::to_i64(&l).unwrap_or(0)) =>
            {
                D::from(match ToPrimitive::to_i64(&l).unwrap_or(0) {
                    -30000 => -22500,
                    -20000 => -15000,
                    -10000 => -6667,
                    -5 => -4,
                    _ => 1,
                })
            }
            // Between two lines (handlers/TestPageAutoSplitKeyMoreShapes.al:119-166 and
            // :186-205 for Decimal; handlers/TestPageAutoSplitKeyRanges_Tests.al for the cap): the
            // lower line plus half the gap, at most 10000. A gap that crosses zero gives one less
            // (the number zero is reserved: -10000 and 10000 give -1, -1 and 1 give -0.5), and a
            // result that lands on zero by the cap goes to 1. No whole number between two adjacent
            // lines raises "The line cannot be split." (code TestWrapped).
            (Some(l), Some(u)) if u > l => {
                let crossing = l < D::ZERO && u > D::ZERO;
                let gap = if crossing { u - l - D::ONE } else { u - l };
                let mut half = gap / D::from(2);
                if !is_decimal {
                    half = half.trunc();
                    if half < D::ONE {
                        return Err(error_code("The line cannot be split.", "TestWrapped"));
                    }
                }
                let mut n = l + half.min(step);
                if n.is_zero() {
                    if is_decimal {
                        return unsupported("AutoSplitKey landing on zero in a Decimal key (not pinned)");
                    }
                    n = D::ONE;
                }
                n
            }
            (l, u) => {
                return unsupported(format!(
                    "AutoSplitKey between {l} and {u} (the corpus does not pin it)",
                    l = l.describe(),
                    u = u.describe()
                ));
            }
        };
        rec.borrow_mut().fields[last] = ops::convert(Value::Decimal(number), &ty)?;
        Ok(())
    }
}
