//! `RunObject` actions (ADR-012 follow-up): invoking an action that names an object instead of
//! running an `OnAction` trigger. The platform alone decides what happens, so every rule below
//! cites the corpus test that measured it on real BC.

use super::*;
use al_builtins::filter::{FieldFilter, FilterExpr};
use al_builtins::record_op::RecordOp;

/// Filter group `RunPageLink` filters land in (corpus handlers/TestActionRunPageLinkFilterGroup.al:
/// `g0=AAA|g2=|g4=`, unlike a part's SubPageLink).
const RUN_LINK_GROUP: i32 = 0;
/// Filter group the `where(...)` of `RunPageView` lands in (corpus
/// handlers/TestPageActionRunPageView.al:RunPageView_Where_FiltersTheOpenedPage, `g3=KEEP`).
const RUN_VIEW_GROUP: i32 = 3;

impl Session {
    /// Invokes the `RunObject` of action control `idx` of the open page `st`.
    pub(super) fn run_object_action(&mut self, st: &St, idx: usize) -> Result<Value, Signal> {
        let engine = self.engine();
        let ro = page_of(&engine, st).controls[idx]
            .run_object
            .clone()
            .expect("an action with a RunObject");
        if let Some(why) = &ro.unsupported {
            return unsupported(format!("RunObject action: {why}"));
        }
        // The test surface refuses these three before it looks at the target (corpus
        // handlers/TestPageActionRunObjectKinds_Tests.al: RunObjectNamingAReportIsRefusedByTheTestPageSurface,
        // ...ReportWithNoRequestPageIsRefusedTheSameWay, ...AnXmlPortIsRefused..., ...AQueryIsRefusedBeforeItsMetadataIsResolved;
        // the same texts for objects of a precompiled app: ...KindsPrecompiled_Tests.al).
        match ro.kind {
            ir::RunKind::Report => return Err(error("The method RunReport is not supported for TestPages.")),
            ir::RunKind::XmlPort => return Err(error("The method RunXmlPort is not supported for TestPages.")),
            ir::RunKind::Query => {
                return Err(error(
                    "The method GetQueryTableMetadata is not supported for TestPages.",
                ));
            }
            ir::RunKind::Codeunit => self.run_object_codeunit(st, &ro)?,
            ir::RunKind::Page => self.run_object_page(st, &ro)?,
        }
        Ok(Value::Null)
    }

    /// A codeunit target runs on the host's own record, filters and position included, whether or
    /// not `RunPageOnRec` is declared: what the codeunit moves or writes the host shows (corpus
    /// handlers/TestPageActionRunObjectKinds_Tests.al:RunObjectNamingACodeunitRunsItOnTheHostsRow,
    /// ...WithRunPageOnRecIsHandedTheHostsRow; handlers/TestPageActionRunObjectCodeunitRec_Tests.al).
    fn run_object_codeunit(&mut self, st: &St, ro: &ir::RunObject) -> Result<(), Signal> {
        let engine = self.engine();
        let cu = ro.object.expect("a codeunit target");
        let obj = &engine.program.objects[cu];
        let declares = |name: &str, value: &str| {
            obj.properties
                .iter()
                .any(|(k, v)| k.eq_ignore_ascii_case(name) && v.trim().eq_ignore_ascii_case(value))
        };
        if declares("Subtype", "Test") || declares("SingleInstance", "true") {
            return unsupported("RunObject of a test or SingleInstance codeunit (not pinned)");
        }
        if ro.link.is_some() || ro.mode != ir::RunMode::Default {
            return unsupported("RunPageLink, RunPageView or RunPageMode on a codeunit target (not pinned)");
        }
        let on_run = obj
            .routines
            .iter()
            .position(|r| r.is_trigger && r.name.eq_ignore_ascii_case("OnRun"));
        let inst = match obj.rec_slot {
            Some(slot) => {
                let wants = match &obj.globals[slot].ty {
                    RtType::Record(t, _) => *t,
                    _ => return unsupported("RunObject of a codeunit whose Rec is not a record"),
                };
                if !self.has_source(st) {
                    return unsupported("RunObject of a codeunit with TableNo from a page without a source table");
                }
                let rec = self.page_rec(st);
                if rec.borrow().table != wants {
                    return unsupported("RunObject of a codeunit whose TableNo is not the host's source table");
                }
                self.enter_record(cu, rec, None)?.0
            }
            None => self.instance(cu),
        };
        if let Some(on_run) = on_run {
            self.call(cu, on_run, inst, Vec::new())?;
        }
        // What the codeunit did to the host's row is shown like the effects of an OnAction.
        self.settle_after_action(st)?;
        // A list whose edit was saved before the codeunit reads its rows again first, then its row as below
        // (pages/lt-y45-save-before-action/Y45SaveBeforeAction.al:431 List_SetName_RunObjectCodeunit `cu H1 ...;R H1;R H2;R H1;R H1;C H1`).
        // A Document host saved that way reads its parts after its row (:670 Doc_SetName_RunObjectCodeunit `...;R H1;C H1;lC 10000`).
        let saved = std::mem::take(&mut st.borrow_mut().saved_before_run_object);
        if saved && self.is_list(st) {
            let rec = self.page_rec(st);
            if self.stored_row(&rec)?.is_some() {
                let key = self.row_key(&rec)?;
                self.read_list_again_from_row_as(st, &rec, &key, false)?;
            }
        }
        // The host reads its row again after a codeunit, even one that wrote nothing
        // (handlers/TestPageRunObjectContracts.al:248-270).
        self.reread_host_row(st, true)?;
        if saved && !self.is_list(st) && st.borrow().open && !st.borrow().parts.is_empty() {
            self.sync_parts(st)?;
        }
        self.run_refresh(st)
    }

    /// A page target opens like `Page.Run` does, except in two things: the page opens and runs
    /// its OnOpenPage (and OnNewRecord) before BC looks for a handler, and nothing bound is no
    /// error for an ordinary page, while a dialog is refused afterwards (corpus
    /// handlers/TestPageActionRunObjectNoHandler_Tests.al: arms 1 and 7-9;
    /// handlers/TestPageCreateModeNewRecordArgs.al:ActionCreate_NoHandler_*, DialogCreate_NoHandler_*).
    pub(super) fn run_object_page(&mut self, st: &St, ro: &ir::RunObject) -> Result<(), Signal> {
        let engine = self.engine();
        let target = ro.object.expect("a page target");
        let obj = &engine.program.objects[target];
        let page_type = engine.object_facts[target]
            .page_type
            .as_ref()
            .map_or_else(|| "card".to_string(), |p| p.lower().to_string());
        // A StandardDialog is shown as a dialog and answered by a [ModalPageHandler]; a Card or a
        // List is answered by a [PageHandler] (TestPageActionRunObject_Tests.al:
        // RunObjectActionToAStandardDialogTargetIsAnsweredByTheModalHandler). Other page types are
        // not pinned.
        // List, Document, Worksheet and ListPlus targets are answered by a PageHandler; a
        // NavigatePage by a ModalPageHandler (handlers/TestPageRunObjectContracts.al:288-326).
        let modal = match page_type.as_str() {
            "standarddialog" | "navigatepage" => true,
            "card" | "list" | "document" | "worksheet" | "listplus" => false,
            other => return unsupported(format!("RunObject of a {other} page (the handler kind is not pinned)")),
        };
        let mode = match ro.mode {
            ir::RunMode::Default | ir::RunMode::Edit => PageMode::Edit,
            ir::RunMode::View => PageMode::View,
            ir::RunMode::Create => PageMode::New,
        };
        // A page caught by TestPage.Trap() is not answered by a handler (corpus
        // handlers/TestPageActionRunPageMode.al:RunPageModeCreate_Trapped_Triggers).
        let mut target_state = Rc::new(RefCell::new(PageState::closed(target as u32)));
        let mut trapped = false;
        if !modal {
            if let Some(trap) = self.take_trap(target as u32) {
                trap.borrow_mut().trapping = false;
                trap.borrow_mut().caught_by_trap = true;
                trap.borrow_mut().instance = None;
                target_state = trap;
                trapped = true;
            } else if !self.pages.traps.is_empty() {
                return unsupported("a Trap() for another page than the one the action opens");
            }
        }
        if target_state.borrow().open {
            if !trapped {
                return unsupported("running a page that is already open");
            }
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::TrappedPageLeftOpenIsReplaced);
            self.mark_closed(&target_state);
        }
        target_state.borrow_mut().modal = modal;
        let record = self.run_object_record(st, &target_state, ro, mode)?;
        // What Rec holds in the target's OnOpenPage when only `RunPageLink` hands it the host's values: real BC fills the linked
        // fields and GetFilters() reads '' (corpus issue 712), the runtime holds the link as a filter. The corpus pins GetFilter
        // per filter group there (handlers/TestActionRunPageLinkFilterGroup.al:119-131); the other reads refuse
        // (records.rs run_page_link_open_op).
        // A link of field(filter(...)) entries only is a filter in group 0 there, which GetFilters reads (pages/lt-y37-link-filter/
        // Y37LinkFilterTests.al:357-383 `LL.open f0=[No.: A1|A3]`).
        let field_filters_only = ro
            .link
            .as_ref()
            .is_some_and(|l| l.entries.iter().all(|e| matches!(e, ir::LinkEntry::FieldFilter(..))));
        if !ro.on_rec && !field_filters_only && ro.link.as_ref().is_some_and(|l| !l.entries.is_empty()) {
            let fill = match &record {
                Some(r) => self.run_page_link_fill(r, ro, &page_type)?,
                None => None,
            };
            let mut t = target_state.borrow_mut();
            match fill {
                Some(fill) => {
                    t.open_rec_unpinned = Some(crate::records::RUN_PAGE_LINK_OPEN_FILLED);
                    t.open_rec_fill = Some(fill);
                }
                None => t.open_rec_unpinned = Some(crate::records::RUN_PAGE_LINK_OPEN),
            }
        }
        let kind = if modal { UiKind::ModalPage } else { UiKind::Page };
        // The handler is looked up after OnInit and OnOpenPage; a page nobody answers stops there.
        self.pages.lenient_bind = true;
        let opened = self.open_page(&target_state, mode, record, if trapped { None } else { Some(kind) });
        self.pages.lenient_bind = false;
        let handler = opened?;
        let answered = match handler {
            Some(h) => self.answer_page(&target_state, h, modal).map(|_| ()),
            None if trapped => Ok(()),
            None if modal => {
                // BC refuses a dialog nobody answers, after it opened (NoHandler_Tests.al arms 7 and 8).
                self.mark_closed(&target_state);
                let id = obj.id.unwrap_or(0);
                // Code: handlers/TestPageRunObjectContracts.al:228-240.
                Err(error_code(
                    format!("TestPages that are invoked from a RunObject action must have a handler. Page No. = {id}"),
                    "TestPageInvokedWithoutHandler",
                ))
            }
            None => {
                // An ordinary page opens with nobody to answer it and AL is told nothing: it ran
                // OnInit and OnOpenPage and is dropped without its close triggers
                // (handlers/TestPageRunObjectContracts.al:330-346, TestPageActionRunObjectNoHandler_Tests.al arm 1).
                self.mark_closed(&target_state);
                Ok(())
            }
        };
        target_state.borrow_mut().modal = false;
        answered?;
        self.settle_after_action(st)?;
        // A page target that wrote nothing leaves the host as it was; one that wrote makes the host
        // read its row again (handlers/TestPageRunObjectContracts.al:228-246, :194-215).
        self.reread_host_row(st, false)
    }

    /// What an action's `RunPageLink` fills into the target's Rec before its OnOpenPage, in the shape the corpus pins: a List
    /// or Card target, no `RunPageView`, every entry `field(...)` or `const(...)` of a Code, Text or Integer field. Real BC
    /// fills the linked fields there and leaves the others blank, with or without a matching row
    /// (handlers/TestActionRunPageLinkFilterGroup.al:292 QZ_RunPageLink_RecInOnOpenPage_List_Handler, :313 _List_Trap,
    /// :333 _Card_Trap, :354 _Const, :369 _NoHandler; `open code=P1 number=7 line=0 note=`). With it come the link's filters,
    /// which a by-value copy of Rec does not carry (`filters=`). `None`: another shape, whose Rec stays not pinned.
    fn run_page_link_fill(
        &mut self,
        rec: &Rec,
        ro: &ir::RunObject,
        page_type: &str,
    ) -> Result<Option<al_builtins::value::RunPageLinkFill>, Signal> {
        let Some(link) = &ro.link else { return Ok(None) };
        // A ListPlus, Document and Worksheet target behave as a List or Card (pages/lt-y41-page-probes/Y41RunPageLinkOnOpen.al:440,
        // :571, :589).
        if !matches!(page_type, "list" | "card" | "listplus" | "document" | "worksheet") {
            return Ok(None);
        }
        let table = self.table(rec)?;
        let r = rec.borrow();
        let link_filters: Vec<FieldFilter> = r
            .filters
            .iter()
            .filter(|f| f.group == RUN_LINK_GROUP)
            .cloned()
            .collect();
        // A RunPageView beside the link: the field of each of its single-value filters is filled too, and GetFilters shows only
        // the link (Y41RunPageLinkOnOpen.al:553 RPL_Card_LinkPlusView `code=P1 number=7 ... filters=Code: P1`).
        let view_filters: Vec<&FieldFilter> = r.filters.iter().filter(|f| f.group == RUN_VIEW_GROUP).collect();
        // Only the link's own filters and the view's: a handed record carries nothing else (no RunPageOnRec).
        let only_link_and_view = link_filters.len() + view_filters.len() == r.filters.len();
        if !only_link_and_view
            || link_filters.len() != link.entries.len()
            || (link.view.is_some() != !view_filters.is_empty())
        {
            return Ok(None);
        }
        let mut fill = Vec::new();
        for f in &view_filters {
            let ty = &table.fields[f.field].ty;
            let flow_filter =
                self.engine.field_facts[table.object as usize][f.field].class == ir::FieldClass::FlowFilter;
            let value = match &f.expr {
                FilterExpr::Eq(v) => v.clone(),
                FilterExpr::Range(Some(a), Some(b)) if ops::compare(a, b).is_ok_and(|o| o.is_eq()) => a.clone(),
                _ => return Ok(None),
            };
            let filled = matches!(
                ty,
                RtType::Code(_) | RtType::Text(_) | RtType::Integer | RtType::Option(_) | RtType::Date
            );
            let Ok(value) = ops::convert(value, ty) else {
                return Ok(None);
            };
            if !filled || flow_filter {
                return Ok(None);
            }
            fill.push((f.field, value));
        }
        for entry in &link.entries {
            // A filter('P1') entry of one value is filled as a const one is (Y41RunPageLinkOnOpen.al:535 RPL_Card_FilterEntry
            // `code=P1`); a filter of more values fails the value check below.
            let pf = match entry {
                ir::LinkEntry::Field(pf, _) | ir::LinkEntry::Filter(pf, _) => *pf,
                ir::LinkEntry::FieldFilter(..) => return Ok(None),
            };
            // An Option and a Date field are filled too (:499 RPL_Card_OptionLink `kind=B`, :517 RPL_Card_DateLink); a FlowFilter
            // field of Code is filled with nothing, the link stays its filter (:458 RPL_List_FlowFilterLink `code= ... filters=Code
            // Filter: P1`).
            let flow_filter = self.engine.field_facts[table.object as usize][pf].class == ir::FieldClass::FlowFilter;
            let ty = &table.fields[pf].ty;
            let filled = matches!(
                ty,
                RtType::Code(_) | RtType::Text(_) | RtType::Integer | RtType::Option(_) | RtType::Date
            );
            if !filled || (flow_filter && !matches!(ty, RtType::Code(_))) {
                return Ok(None);
            }
            let Some(f) = link_filters.iter().find(|f| f.field == pf) else {
                return Ok(None);
            };
            let value = match &f.expr {
                FilterExpr::Eq(v) => v.clone(),
                FilterExpr::Range(Some(a), Some(b)) if ops::compare(a, b).is_ok_and(|o| o.is_eq()) => a.clone(),
                _ => return Ok(None),
            };
            let Ok(value) = ops::convert(value, &table.fields[pf].ty) else {
                return Ok(None);
            };
            if !flow_filter {
                fill.push((pf, value));
            }
        }
        Ok(Some((fill, Rc::new(link_filters))))
    }

    /// The record the target opens on: the host's stored row for `RunPageOnRec`, narrowed by
    /// `RunPageLink` (filter group 0) and `RunPageView` (sorting, order and `where` in group 3).
    /// `None` when the action declares none of them: the target opens on its own view.
    fn run_object_record(
        &mut self,
        host: &St,
        target: &St,
        ro: &ir::RunObject,
        mode: PageMode,
    ) -> Result<Option<Rec>, Signal> {
        let engine = self.engine();
        if !ro.on_rec && ro.link.is_none() {
            return Ok(None);
        }
        let object = target.borrow().object as usize;
        let obj = &engine.program.objects[object];
        if obj.rec_slot.is_none() {
            // RunPageOnRec alone to a page without a source table opens it as without the property (pages/W1RunObject.al:425
            // NoSourceOnRec `ok, log=nosource:OnOpenPage;`). A link or a view there is not pinned.
            if ro.on_rec && ro.link.is_none() {
                return Ok(None);
            }
            return unsupported("RunPageOnRec, RunPageLink or RunPageView on a target page without a source table");
        }
        // A target that declares SourceTableView combines it with the handed record as Page.Run does (`open_page`): what its
        // OnOpenPage reads is pinned (pages/W1RunObject.al:410 TargetWithSourceTableView_OrNoSourceTable, a Card: RunPageLink
        // shows `id=0 filters=Grp: 2`, RunPageOnRec the host's row 3 and no filters, RunPageView no filters). On a List, RunPageLink
        // and the target's view both apply, a sorting order(descending) view opens on its first row in that order, a where view on
        // its first row, and the action's RunPageView where filter is added while its sorting does not replace the target's
        // (pages/lt-rz-outside-page-moves/RzRunPageLinkView.al, the RZ_Link_* tests). RunPageOnRec, and RunPageView without
        // RunPageLink, on such a List are not pinned: an assumption.
        let target_view = al_ir::ObjectFacts::of(obj).has_source_table_view;
        // RunPageMode = Create beside them: the target's OnOpenPage sees the handed record, then the new row starts blank,
        // a linked field stamped (pages/W1RunObject.al:388 Create_WithOnRec_Link_View, a Card: `id=3`, `filters=Grp: 2`,
        // `filters=`; misc/lt-probes-needed-7/PnGRunObjectOptions.al:65 CreateModeCombinations, a Card with RunPageOnRec
        // `Card.OnNewRecord;Card.AGCR(,)`, :89 CreateModeWithLinkAndView, a List with RunPageLink `List.AGCR(RP2,RP2)` and with
        // RunPageView `List.AGCR(,)`). A List with RunPageOnRec, or a target with SourceTableView, is not pinned.
        if mode == PageMode::New && (target_view || (ro.on_rec && self.is_list(target))) {
            return unsupported("RunPageMode = Create with RunPageOnRec, RunPageLink or RunPageView (not pinned)");
        }
        if target_view {
            let linked = ro.link.as_ref().is_some_and(|l| !l.entries.is_empty());
            if self.is_list(target) && (ro.on_rec || !linked) {
                al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::RunObjectTargetViewWithoutLink);
            }
        }
        self.page_instance(target)?;
        let base = self.page_rec(target);
        let rec = Rc::new(RefCell::new(base.borrow().clone()));
        let table = self.table(&rec)?;
        let host_rec = if self.has_source(host) {
            Some(self.page_rec(host))
        } else {
            None
        };
        if ro.on_rec {
            // The target's Rec is the STORED row, not what the host's page code left in its buffer,
            // and it carries none of the host's filters (corpus
            // handlers/TestPageActionRunObjectPageStoredRow_Tests.al:TargetOnOpenPageSeesHostRow,
            // handlers/TestPageActionRunObjectPageRec_Tests.al:FilteredHostTargetRowSet).
            let Some(host_rec) = &host_rec else {
                return unsupported("RunPageOnRec from a page without a source table");
            };
            if host_rec.borrow().table != rec.borrow().table {
                return unsupported("RunPageOnRec to a page over another table (not pinned)");
            }
            let mut pk = table.primary_key(&host_rec.borrow().fields);
            self.ret_used = true;
            if !matches!(self.record_op(RecordOp::Get, &rec, &mut pk)?, Value::Boolean(true)) {
                return unsupported("RunPageOnRec from a host row that is not stored (not pinned)");
            }
        }
        if let Some(link) = &ro.link {
            for entry in &link.entries {
                // `field(filter(F))`: the host's value as a filter text in filter group 0, no filter when it is blank
                // (pages/lt-y37-link-filter/Y37LinkFilterTests.al:357-383 `LL.open f0=[No.: A1|A3]`).
                if let ir::LinkEntry::FieldFilter(pf, hf) = entry {
                    let Some(host_rec) = &host_rec else {
                        return unsupported("RunPageLink from a page without a source table");
                    };
                    if host.borrow().cursor == Cursor::None {
                        return unsupported(
                            "a RunPageLink field(filter(...)) from a host that shows no row (not pinned)",
                        );
                    }
                    let ty = table.fields[*pf].ty.clone();
                    let expr = self.link_filter_of_host_field(host_rec, *hf, &ty)?;
                    let mut r = rec.borrow_mut();
                    r.filters.retain(|f| !(f.group == RUN_LINK_GROUP && f.field == *pf));
                    if let Some(expr) = expr {
                        r.filters.push(FieldFilter {
                            field: *pf,
                            expr,
                            group: RUN_LINK_GROUP,
                        });
                    }
                    continue;
                }
                let (field, expr) = match entry {
                    ir::LinkEntry::Field(pf, hf) => {
                        let ty = table.fields[*pf].ty.clone();
                        let Some(host_rec) = &host_rec else {
                            return unsupported("RunPageLink from a page without a source table");
                        };
                        // The host's current row; a host that shows no row links to the blank value
                        // (as a part's link does, testpart/TestPartProviderMove.al).
                        let value = if host.borrow().cursor == Cursor::None {
                            let host_ty = self.table(host_rec)?.fields[*hf].ty.clone();
                            Value::default_for(&host_ty)
                        } else {
                            host_rec.borrow().fields[*hf].clone()
                        };
                        let value = ops::convert(value, &ty)?;
                        (*pf, FilterExpr::Range(Some(value.clone()), Some(value)))
                    }
                    ir::LinkEntry::Filter(pf, text) => {
                        // An empty term selects the rows whose field is blank (pages/lt-ao-testpage/AoTests.al:513-560 RunPageLink, :845-887
                        // SubPageLink).
                        let ty = table.fields[*pf].ty.clone();
                        let expr = self
                            .parse_filter_text(text, &ty)
                            .map_err(|e| Signal::Unsupported(format!("RunPageLink filter: {e}")))?;
                        (*pf, expr)
                    }
                    ir::LinkEntry::FieldFilter(..) => unreachable!("handled above"),
                };
                let mut r = rec.borrow_mut();
                r.filters.retain(|f| !(f.group == RUN_LINK_GROUP && f.field == field));
                r.filters.push(FieldFilter {
                    field,
                    expr,
                    group: RUN_LINK_GROUP,
                });
            }
            if let Some(view) = &link.view {
                self.set_view_in(
                    &rec,
                    &table,
                    &crate::views::property_view(view),
                    Some(RUN_VIEW_GROUP),
                    false,
                    false,
                    false,
                )?;
            }
        }
        Ok(Some(rec))
    }

    /// After a page target closed: the host shows its current row as stored now, and its
    /// OnAfterGetRecord and OnAfterGetCurrRecord ran again for it. The row it holds in `xRec` stays
    /// the one it loaded (corpus handlers/TestPageActionRunObjectPageRec_Tests.al:
    /// HostSaveAfterTheTargetEditedTheRowCarriesXRec, HostOnAfterGetRecordAfterTheTargetWroteTheRow).
    /// Whether the triggers also run when the target wrote nothing is not pinned.
    fn reread_host_row(&mut self, st: &St, always: bool) -> Result<(), Signal> {
        if !st.borrow().open || !self.has_source(st) {
            return Ok(());
        }
        if st.borrow().cursor != Cursor::Row || st.borrow().draft || st.borrow().dirty {
            return Ok(());
        }
        let rec = self.page_rec(st);
        if self.stored_row(&rec)?.is_none() {
            return Ok(());
        }
        let mut pk = self.table(&rec)?.primary_key(&rec.borrow().fields);
        let shown = rec.borrow().fields.clone();
        let loaded = self.page_xrec_fields(st);
        self.ret_used = true;
        self.record_op(RecordOp::Get, &rec, &mut pk)?;
        // Nothing was written when the stored row is the one the host loaded.
        // (A Blob field is a shared buffer and compares unequal to its own copy.)
        let unchanged = loaded.len() == rec.borrow().fields.len()
            && loaded.iter().zip(rec.borrow().fields.iter()).all(|(a, b)| {
                matches!(a, Value::Blob(_)) || Self::same_values(std::slice::from_ref(a), std::slice::from_ref(b))
            });
        if !always && unchanged {
            rec.borrow_mut().fields = shown;
            return Ok(());
        }
        let r = (|| {
            self.calc_row_flowfields(st)?;
            self.populate_expressions(st)?;
            self.page_trigger(st, "onaftergetrecord", vec![])?;
            self.page_trigger(st, "onaftergetcurrrecord", vec![])?;
            self.page_event(st, "onaftergetcurrrecordevent", None, None).map(|_| ())
        })();
        self.torn_down_on_error(st, r)
    }
}
