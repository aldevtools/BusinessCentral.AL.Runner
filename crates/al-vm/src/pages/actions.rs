use super::*;
use al_builtins::page_op::PageMethod;
use al_builtins::record_op::RecordOp;

impl Session {
    /// An action with no `OnAction` trigger (its own or a page extension's) and no `RunObject` is not
    /// on the TestPage: naming it raises TestActionNotFound (handlers/TestPageLabelControlContracts.al:
    /// 131-145, BC 27.5). Whether other behaviour (a `RunPageLink` without a `RunObject`, a report
    /// action) also counts is not pinned.
    pub(super) fn action_without_behaviour(&self, st: &St, idx: usize) -> bool {
        let engine = self.engine();
        let ctl = &page_of(&engine, st).controls[idx];
        matches!(ctl.kind, ir::ControlKind::Action)
            && ctl.action_ref.is_none()
            && ctl.run_object.is_none()
            && !ctl.triggers.iter().any(|(n, _)| n.eq_ignore_ascii_case("onaction"))
            && !ctl
                .ext_triggers
                .iter()
                .any(|(n, _, _)| n.eq_ignore_ascii_case("onaction"))
    }

    pub(super) fn control_eliminated(&self, st: &St, idx: usize) -> bool {
        let engine = self.engine();
        let page = page_of(&engine, st);
        // Actions are not eliminated, whatever their Visible says (corpus
        // handlers/TestPageActionReachability_Tests.al:100-122).
        if matches!(
            page.controls[idx].kind,
            ir::ControlKind::Action | ir::ControlKind::ActionGroup
        ) {
            return false;
        }
        let mut at = Some(idx);
        while let Some(i) = at {
            if matches!(page.controls[i].prop("visible"), Some(ir::PropVal::Lit(v)) if v.trim().eq_ignore_ascii_case("false"))
            {
                return true;
            }
            at = page.controls[i].parent;
        }
        false
    }

    /// `TestField.DrillDown`, `AssistEdit` and `Lookup`: run the control's trigger of that name, the
    /// one it states itself or the one a page extension's `modify` block adds (corpus
    /// pageextensiontrigger/TestModifiedControlValidateTriggers_Tests.al: AModifyBlocksOnDrillDownRunsForTheModifiedControl,
    /// AModifyBlocksOnLookupRunsForTheModifiedControl, ABaseControlsOwnOnAssistEditRuns,
    /// AModifyBlocksOnAssistEditRunsForTheModifiedControl, OnAssistEditIsRaisedOnlyForTheControlItWasCalledOn).
    /// A DrillDown or AssistEdit on a control with no trigger raises a fixed platform error (handlers/TestPageFieldDrillDown_Tests.al:
    /// FieldDrillDownWithNoTriggerIsRefused), unless a DrillDown has a `DrillDownPageId` to open; a Lookup without a
    /// trigger goes to [`field_lookup`](Self::field_lookup).
    pub(super) fn control_navigate(&mut self, st: &St, idx: usize, method: PageMethod) -> Result<Value, Signal> {
        let engine = self.engine();
        let ctl = &page_of(&engine, st).controls[idx];
        let name = match method {
            PageMethod::DrillDown => "ondrilldown",
            PageMethod::AssistEdit => "onassistedit",
            _ => "onlookup",
        };
        let own = ctl.trigger(name).map(|r| (ctl.owner, r));
        let added: Vec<(usize, usize)> = ctl
            .ext_triggers
            .iter()
            .filter(|(n, _, _)| n == name)
            .map(|(_, e, r)| (*e, *r))
            .collect();
        let found: Vec<(Option<usize>, usize)> = own
            .into_iter()
            .chain(added.into_iter().map(|(e, r)| (Some(e), r)))
            .collect();
        let lit = |p: &str| match ctl.prop(p) {
            Some(ir::PropVal::Lit(v)) => Some(v.trim().trim_matches('"').to_string()),
            _ => None,
        };
        let drill_page = lit("drilldownpageid");
        if method == PageMethod::DrillDown {
            // `DrillDown = false` refuses the drill-down before any trigger runs, for a control of the page and of a page
            // extension alike (pageext/lt-u-page-props/UPageProps.al:113-127, `base_off`, `ext_off`).
            if lit("drilldown").is_some_and(|v| v.eq_ignore_ascii_case("false")) {
                // Pinned on pages only; on a request page it waits for a probe (docs/probes-needed.md).
                if self.is_request_page(st) {
                    return unsupported(format!(
                        "TestRequestPage DrillDown on control {} with DrillDown = false on a request page (not pinned; probe wanted: its error text and code, and whether OnDrillDown runs)",
                        ctl.name
                    ));
                }
                return Err(error_code(
                    "The NavDrilldownAction method is not supported.",
                    "MethodNotSupported",
                ));
            }
            if !found.is_empty() && drill_page.is_some() {
                return unsupported(format!(
                    "TestField.DrillDown on control {} with both an OnDrillDown trigger and a DrillDownPageId (not pinned)",
                    ctl.name
                ));
            }
        }
        // `Lookup = false` on a control over a field with a TableRelation, with no OnLookup: Lookup() opens no page and raises
        // nothing (pageext/lt-u-page-props/UPageProps.al:147-154 PageExt_LookupOnAndOff `ext_off_pages_opened = 0`, a page
        // extension's control). With an OnLookup, or on a request page, it is not pinned.
        if method == PageMethod::Lookup && lit("lookup").is_some_and(|v| v.eq_ignore_ascii_case("false")) {
            if !found.is_empty() || self.is_request_page(st) {
                return unsupported(format!(
                    "TestField.Lookup on control {} with Lookup = false and an OnLookup trigger, or on a request page (not pinned)",
                    ctl.name
                ));
            }
            return Ok(Value::Null);
        }
        let (owner, routine) = match found.as_slice() {
            [one] => *one,
            [] if method == PageMethod::DrillDown => {
                if let Some(target) = drill_page {
                    return self.drill_down_page(st, idx, &target);
                }
                // A FlowField opens the DrillDownPageId of its CalcFormula's table (record/lt-x-table-props/XTablePropsTests.al:
                // 646-658); the view that page gets is not pinned, so it is refused.
                if self.control_field(st, idx).is_some_and(|f| !f.stored) {
                    return unsupported(format!(
                        "TestField.DrillDown on FlowField control {} (the CalcFormula table's page opens; its view is not pinned)",
                        ctl.name
                    ));
                }
                // No trigger and no page: a fixed platform error with the code MethodNotSupported
                // (report/lt-bj-request-drilldown/BjRequestDrillDown.al:226-227, UPageProps.al:122-123).
                return Err(error_code(
                    "The NavDrilldownAction method is not supported.",
                    "MethodNotSupported",
                ));
            }
            [] if method == PageMethod::Lookup => return self.field_lookup(st, idx),
            // AssistEdit with no trigger: a fixed platform error, also on a control with a LookupPageId or over a table
            // relation (pages/lt-page-editability-width-lookups/P115Lookups.al:182-190, lookups/W2Lookups.al:569, :593).
            [] if method == PageMethod::AssistEdit => {
                return Err(error_code(
                    "The NavAssistEditAction method is not supported.",
                    "MethodNotSupported",
                ));
            }
            [] => return unsupported(format!("TestField.{method} on a control with no {name} trigger")),
            _ => {
                return unsupported(format!(
                    "TestField.{method} on a control with more than one {name} trigger"
                ));
            }
        };
        st.borrow_mut().refresh = false;
        let marked_only_before = !self.has_source(st) || self.page_rec(st).borrow().marked_only;
        let shown_before =
            (method == PageMethod::DrillDown && self.has_source(st)).then(|| self.page_rec(st).borrow().fields.clone());
        let rec_before = (method == PageMethod::AssistEdit && self.has_source(st))
            .then(|| (self.page_rec(st).borrow().fields.clone(), st.borrow().dirty));
        let result = if method == PageMethod::Lookup {
            // OnLookup(var Text): Boolean. The text it writes back replaces the field's value, and
            // the Boolean says whether the user picked anything (corpus
            // handlers/TestPageFieldTableLookup.al:72 Lookup_ControlWithTrigger_WinsOverTheTableFieldsOnLookup,
            // handlers/TestPageModalHandler_Tests.al:132). The text the trigger starts with is the
            // control's current text (assumed, docs/probes-needed.md).
            let object = owner.unwrap_or(st.borrow().object as usize);
            let inst = self.owner_instance(st, owner);
            let current = self.control_value(st, idx)?;
            let shown = self.control_text(st, idx, &current)?;
            self.call(object, routine, inst, vec![Value::Text(shown.into())])
                .and_then(|(v, finals)| match (v, finals.first()) {
                    (Value::Boolean(true), Some(Value::Text(t))) => {
                        // Written back as the client would type it. The OnLookup of a page extension's modify block
                        // leaves a Code field in the case it wrote (`Text := 'FromExt'` reads back FromExt), the page's
                        // own trigger does not (FROMCTL) (lookups/W2Lookups.al:
                        // ModifyBlockOnLookup_ReturnsTrue, BC 27.5 and 29.0).
                        let r = self.control_set_value(st, idx, &mut [Value::Text(t.clone())]);
                        let binding = page_of(&self.engine(), st).controls[idx].binding.clone();
                        if let ir::Binding::Field(pos) = binding {
                            let rec = self.page_rec(st);
                            let is_code = matches!(rec.borrow().fields[pos], Value::Code(_));
                            if is_code && owner.is_some() && r.is_ok() {
                                rec.borrow_mut().fields[pos] = Value::Code(t.clone());
                            }
                        }
                        r
                    }
                    (Value::Boolean(true), other) => {
                        unsupported(format!("an OnLookup trigger whose Text is {}", other.describe()))
                    }
                    _ => Ok(Value::Null),
                })
        } else {
            let r = self.owner_routine(st, owner, routine).map(|_| Value::Null);
            match r {
                // An Error() of OnDrillDown comes out of the TestPage call with the test framework's prefix on its code
                // (UPageProps.al:120-121, :124-125 `TestWrapped:Dialog`). Other codes, and errors of OnAssistEdit, are not
                // pinned and keep their code.
                Err(Signal::Error(mut e)) if method == PageMethod::DrillDown && e.code == "Dialog" => {
                    e.code = "TestWrapped:Dialog".into();
                    Err(Signal::Error(e))
                }
                other => other,
            }
        };
        if result.is_err() {
            st.borrow_mut().refresh = false;
        }
        let v = result?;
        self.drilldown_marked_only_refresh(st, method, marked_only_before, shown_before);
        // The client refreshes the page after an AssistEdit trigger: OnAfterGetRecord runs again
        // (pages/lt-page-runtime/P82Lookups.al:AssistEdit_Trigger, `OnAssistEdit;OnAfterGetRecord(L1)`). A DrillDown and a Lookup are not pinned to.
        // The row is not read again: a field the trigger set on the record stays (pages/lt-page-editability-width-lookups/
        // P115Lookups.al:P115_AssistEdit_Trigger `after = assisted`).
        // A trigger that changed Rec gets no refresh: OnAfterGetRecord does not run (pages/lt-sn-assistedit-refresh/
        // SnAssistEditRefresh.al:207 and :235 `set = OnAssistEdit(Set)`, on a card and on a list).
        let changed_by_trigger = rec_before
            .as_ref()
            .is_some_and(|(before, _)| !Self::same_values(before, &self.page_rec(st).borrow().fields));
        if method == PageMethod::AssistEdit
            && !changed_by_trigger
            && st.borrow().open
            && self.has_source(st)
            && st.borrow().cursor == Cursor::Row
            && !st.borrow().draft
            && !self.declares(st, "onfindrecord")
            && !self.declares(st, "onnextrecord")
            && (self.declares(st, "onaftergetrecord") || self.declares(st, "onaftergetcurrrecord"))
        {
            // On a List page whose row holds no edit of the client the refresh reads the row as on a card
            // (SnAssistEditRefresh.al:233-234 in edit mode, :257-258 in view mode). After the client edited the row BC logs no
            // OnAfterGetRecord (errors/lt-ms-refusals-129/P129CallstackPageControls.al:252, no `assistedit 2`, after SetValue)
            // under conditions this runtime does not tell apart yet: refused.
            let edited = rec_before.as_ref().is_some_and(|(_, dirty)| *dirty);
            if edited && self.engine.object_facts[st.borrow().object as usize].page_type == Some(al_ir::PageType::List)
            {
                return unsupported(
                    "TestField.AssistEdit on a List page that declares OnAfterGetRecord, after the client edited the row: whether the page is refreshed afterwards is not pinned",
                );
            }
            let r = (|| {
                self.populate_current(st)?;
                self.page_trigger(st, "onaftergetcurrrecord", vec![])?;
                self.page_event(st, "onaftergetcurrrecordevent", None, None)
            })();
            self.torn_down_on_error(st, r)?;
            self.sync_parts(st)?;
        }
        self.run_refresh(st)?;
        Ok(v)
    }

    /// After an OnDrillDown that turned MarkedOnly on for Rec and called `CurrPage.Update(false)`, a list with OnAfterGetRecord or
    /// OnAfterGetCurrRecord drops the rows it held, so its moves read the marked rows from the table, as a list without those
    /// triggers does (pages/lt-sc-outside-pages/ScDrillDownMarkedOnly.al, codeunit 89016). With CurrPage.Update(false) and the row
    /// triggers, a page with a part in the content or FactBoxes area does the same (al-runner issue 5886, the reporter's run on
    /// real BC; assumption drilldown-marked-only-with-part until writer SF's pins SF_WithPart_* and SF_WithFactBox_* land); the
    /// other shapes of a page with parts stay as they were.
    fn drilldown_marked_only_refresh(
        &mut self,
        st: &St,
        method: PageMethod,
        marked_only_before: bool,
        shown_before: Option<Vec<Value>>,
    ) {
        let (shape, with_parts) = {
            let s = st.borrow();
            (
                method == PageMethod::DrillDown && s.open && s.host.is_none() && !s.draft,
                !s.parts.is_empty(),
            )
        };
        if !shape
            || marked_only_before
            || !self.has_source(st)
            || !self.page_rec(st).borrow().marked_only
            || !self.is_list(st)
            || self.declares(st, "onfindrecord")
            || self.declares(st, "onnextrecord")
        {
            return;
        }
        let row_triggers = self.declares(st, "onaftergetrecord") || self.declares(st, "onaftergetcurrrecord");
        let update_false = {
            let s = st.borrow();
            s.refresh && s.refresh_by_update && s.refresh_update_false
        };
        // Without CurrPage.Update the page keeps showing the rows without MarkedOnly (`marked_only_unshown_call`). Pinned for a
        // page without the row triggers only (corpus pages/lt-sc-outside-pages/ScDrillDownMarkedOnly.al:422
        // DrilldownWithoutUpdateListAndGoTo); with them the later TestPage calls are refused.
        if !st.borrow().refresh {
            if with_parts {
                return;
            }
            if row_triggers {
                st.borrow_mut().marked_only_unpinned = true;
            } else {
                st.borrow_mut().marked_only_unshown = true;
            }
            return;
        }
        if !update_false || !row_triggers || st.borrow().window.is_empty() {
            return;
        }
        // In a key other than the primary key, on a page without parts standing on a stored row: the drilldown reads nothing
        // and the page keeps showing the row it stood on; the next GoToRecord refills the rows around the row Rec was left on
        // (`go_to_record_after_marked_refill`; al-runner issue 6008, the reporter's run on real BC: `drilldown=4:` with an
        // empty log while Rec was left on row 1). Every other call until then is refused (`marked_only_unshown_call`).
        let in_its_key = !with_parts
            && self.still_reads_once_more(st)
            && st.borrow().cursor == Cursor::Row
            && shown_before.is_some();
        if in_its_key && let Some(shown) = shown_before {
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::DrilldownMarkedRefillInItsKey);
            let rec = self.page_rec(st);
            let left = std::mem::replace(&mut rec.borrow_mut().fields, shown);
            let mut s = st.borrow_mut();
            s.marked_refill_in_key = Some(left);
            s.refresh = false;
            s.window.clear();
            s.window_from_start = false;
            s.window_top_unread = false;
            drop(s);
            if !self.pages.scope_unpinned.iter().any(|p| Rc::ptr_eq(p, st)) {
                self.pages.scope_unpinned.push(st.clone());
            }
            return;
        }
        // Pinned: pages/lt-sc-outside-pages/ScDrillDownMarkedOnly.al (codeunit 89016); with a part in the content area or in the
        // FactBoxes area by :460 SF_WithPart_ListAndGoTo, :480 SF_WithPart_GoToRecordMissesTheUnmarkedRow, :494
        // SF_WithFactBox_ListAndGoTo and :514 SF_WithFactBox_GoToRecordMissesTheUnmarkedRow (al-runner issue 5886).
        let mut s = st.borrow_mut();
        s.window.clear();
        s.window_from_start = false;
        s.window_top_unread = false;
    }

    /// After an OnDrillDown that turned MarkedOnly on for Rec without `CurrPage.Update`, on a list without OnFindRecord,
    /// OnNextRecord and the row triggers: the client still shows the rows of the view without MarkedOnly, so First, Next and
    /// GoToRecord walk them (al-runner issue 5844, corpus writer's probe pages/lt-sc-outside-pages/ScDrillDownMarkedOnly.al
    /// DrilldownWithoutUpdateListAndGoTo `listed=[ 10 20] goto20=Yes`). Close ends it; any
    /// other TestPage call before that is not pinned and is refused.
    pub(super) fn marked_only_unshown_call(
        &mut self,
        method: al_builtins::page_op::PageOp,
        st: &St,
        f: impl FnOnce(&mut Self) -> Result<Value, Signal>,
    ) -> Result<Value, Signal> {
        use al_builtins::page_op::PageOp;
        // After the marking drilldown on a list in another key (`drilldown_marked_only_refresh`): reading what a field shows and
        // GoToRecord are measured (al-runner issue 6008); nothing else is.
        if st.borrow().marked_refill_in_key.is_some()
            && !matches!(method, PageOp::TestFieldValue | PageOp::TestPageGoToRecord)
        {
            return unsupported(format!(
                "{method} after an OnDrillDown turned MarkedOnly on and called CurrPage.Update(false) on a list in a key other than \
                 the primary key (not pinned)"
            ));
        }
        if st.borrow().marked_only_unpinned {
            if method == PageOp::TestPageClose {
                st.borrow_mut().marked_only_unpinned = false;
                return f(self);
            }
            return unsupported(format!(
                "{method} after an OnDrillDown turned MarkedOnly on without CurrPage.Update on a page with OnAfterGetRecord or \
                 OnAfterGetCurrRecord (what the page shows then is not pinned)"
            ));
        }
        if !st.borrow().marked_only_unshown {
            return f(self);
        }
        match method {
            PageOp::TestPageClose => {
                st.borrow_mut().marked_only_unshown = false;
                f(self)
            }
            PageOp::TestFieldValue => f(self),
            PageOp::TestPageFirst | PageOp::TestPageNext | PageOp::TestPageGoToRecord => {
                let rec = self.page_rec(st);
                rec.borrow_mut().marked_only = false;
                let r = f(self);
                rec.borrow_mut().marked_only = true;
                r
            }
            _ => unsupported(format!(
                "{method} after an OnDrillDown turned MarkedOnly on without CurrPage.Update (what the page shows then is not pinned)"
            )),
        }
    }

    /// `Lookup()` on a control with no OnLookup trigger of its own: the source table field's
    /// OnLookup runs on the page's row (corpus handlers/TestPageFieldTableLookup.al:55
    /// Lookup_ControlWithoutTrigger_RunsTheTableFieldsOnLookup); else a field with a TableRelation
    /// opens the related table's lookup page modally, and a confirmed pick is written back
    /// (handlers/TestPageFieldRelationLookup_Tests.al:104, :122, :142); a field with neither does
    /// nothing (:168).
    pub(super) fn field_lookup(&mut self, st: &St, idx: usize) -> Result<Value, Signal> {
        let Some(field) = self.control_field(st, idx) else {
            return unsupported("TestField.Lookup on a control that is not bound to a table field");
        };
        let (ctl_relation, ctl_page, ctl_name) = {
            let engine = self.engine();
            let c = &page_of(&engine, st).controls[idx];
            let lit = |p: &str| match c.prop(p) {
                Some(ir::PropVal::Lit(v)) => Some(v.trim().to_string()),
                _ => None,
            };
            (lit("tablerelation"), lit("lookuppageid"), c.name.clone())
        };
        if let Some(text) = ctl_relation {
            return self.control_relation_lookup(st, idx, &field, &text, ctl_page.as_deref());
        }
        // A control LookupPageId over a field with no relation: the Lookup raises nothing, needs no handler and leaves the
        // value (pageext/lt-z-modify-controls/ZModifyControls.al:435-452, the property set by a modify block). Over a
        // field with a TableRelation, which page opens is not pinned.
        if ctl_page.is_some() && field.relation {
            return unsupported(format!(
                "Lookup on control {ctl_name} with a LookupPageId over a field with a TableRelation (which page opens is not pinned)"
            ));
        }
        if let Some((object, routine)) = field.on_lookup {
            let rec = self.page_rec(st);
            st.borrow_mut().refresh = false;
            self.call_on_record(object, routine, rec, None, Vec::new())?;
            // The control now shows what the trigger wrote, as an edit of the row (assumed,
            // docs/probes-needed.md).
            st.borrow_mut().dirty = true;
            self.run_refresh(st)?;
            return Ok(Value::Null);
        }
        if !field.relation {
            return Ok(Value::Null);
        }
        let engine = self.engine();
        let [arm] = field.relation_arms.as_slice() else {
            return unsupported("Lookup over a TableRelation with several arms or an unreadable one (not pinned)");
        };
        let (Some(table), true, Some(wheres)) = (arm.table, arm.conditions.is_empty(), &arm.wheres) else {
            return unsupported("Lookup over a conditional TableRelation or one into a table outside the workspace");
        };
        // The page is the related table's LookupPageId, else its DrillDownPageId; a table with
        // neither still shows a form, which nobody can answer (handlers/TestPageFieldRelationLookup_Tests.al:253).
        let table_obj = &engine.program.objects[table];
        let page_of_table = ["LookupPageId", "DrillDownPageId"].iter().find_map(|prop| {
            let name = table_obj
                .properties
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(prop))
                .map(|(_, v)| v.trim().trim_matches('"').to_string())?;
            engine.program.objects.heads().iter().position(|o| {
                o.is_kind(al_ir::ObjKind::Page)
                    && (o.name.eq_ignore_ascii_case(&name) || o.id.is_some_and(|i| i.to_string() == name))
            })
        });
        // A page of a dependency app that nothing reached is not compiled (ADR-013); BC has it, and
        // it would open.
        if page_of_table.is_none() {
            for prop in ["LookupPageId", "DrillDownPageId"] {
                let named = table_obj
                    .properties
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(prop))
                    .map(|(_, v)| v.trim().trim_matches('"').to_string());
                if let Some(name) = named.filter(|n| names_uncompiled_page(&engine, n)) {
                    return unsupported(format!(
                        "the {prop} of table {} is page {name}, which belongs to a dependency app and was not compiled (no code the workspace reaches uses it)",
                        table_obj.name
                    ));
                }
            }
        }
        let Some(page_object) = page_of_table else {
            // An `asserterror` around this call is the only way a test sees the error. How far it
            // rolls back when the page's operation, not the test, raised it is not pinned: with
            // writes pending from before the call the run is refused (the corpus test seeds its
            // host row without a Commit, handlers/TestPageFieldRelationLookup_Tests.al:253).
            if self.store.has_uncommitted() {
                return unsupported(
                    "Lookup over a relation to a table with no lookup page, with writes pending (the extent of an asserterror rollback around a TestPage call is not pinned)",
                );
            }
            return Err(error("Unhandled UI: ModalPage"));
        };
        if !wheres.is_empty() {
            return unsupported(
                "Lookup over a TableRelation with a where() clause (the lookup page's filter is not pinned)",
            );
        }
        self.lookup_through_page(st, idx, &field, page_object, arm.field)
    }

    /// `Lookup()` on a control with its own `TableRelation` (and no OnLookup): the lookup page opens modally and the pick
    /// writes the relation's field. The page is the control's `LookupPageId` (pages/lt-page-editability-width-lookups/
    /// P115Lookups.al:108-124 P115_Lookup_PageIdAndRelation_OK: the related table has none; a pick writes `Descr`, the
    /// relation's field) or else the related table's `LookupPageId` (pages/lt-page-autoformat-lookups/P113Lookups.al:253-270
    /// P113_LK_LookupWithPageRelation). Not pinned, so refused: a where() or conditional relation, a control relation over
    /// a field with its own relation or OnLookup, two different pages, a table with only a DrillDownPageId or no page.
    fn control_relation_lookup(
        &mut self,
        st: &St,
        idx: usize,
        field: &ir::Field,
        text: &str,
        ctl_page: Option<&str>,
    ) -> Result<Value, Signal> {
        if field.relation || field.on_lookup.is_some() {
            return unsupported(
                "Lookup on a control with its own TableRelation over a field with a TableRelation or an OnLookup of its own (which one wins is not pinned)",
            );
        }
        let Some((table_name, field_name)) = simple_relation(text) else {
            return unsupported(format!(
                "Lookup on a control whose own TableRelation `{text}` is not a plain Table.Field (a where() or conditional control relation is not pinned)"
            ));
        };
        let engine = self.engine();
        let names =
            |o: &al_ir::Object, n: &str| o.name.eq_ignore_ascii_case(n) || o.id.is_some_and(|i| i.to_string() == n);
        let tables: Vec<usize> = (0..engine.program.objects.len())
            .filter(|&i| {
                let o = &engine.program.objects[i];
                o.is_kind(al_ir::ObjKind::Table) && o.table.is_some() && names(o, &table_name)
            })
            .collect();
        let [table] = tables.as_slice() else {
            return unsupported(format!(
                "Lookup on a control whose own TableRelation names table {table_name}, which is not one table of the workspace"
            ));
        };
        let table_obj = &engine.program.objects[*table];
        let pos = match &field_name {
            None => None,
            Some(n) => {
                let Some(p) = table_obj
                    .table
                    .as_ref()
                    .and_then(|t| t.fields.iter().position(|f| f.name.eq_ignore_ascii_case(n)))
                else {
                    return unsupported(format!(
                        "Lookup on a control whose own TableRelation names field {n}, which table {table_name} does not have"
                    ));
                };
                Some(p)
            }
        };
        let table_page = table_obj
            .properties
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("LookupPageId"))
            .map(|(_, v)| v.trim().trim_matches('"').to_string());
        let ctl_page = ctl_page.map(|p| p.trim_matches('"').to_string());
        let find_page = |n: &str| {
            engine
                .program
                .objects
                .iter()
                .position(|o| o.is_kind(al_ir::ObjKind::Page) && names(o, n))
        };
        let wanted = match (&ctl_page, &table_page) {
            (Some(c), Some(t)) if find_page(c) != find_page(t) => {
                return unsupported(format!(
                    "Lookup on a control whose LookupPageId {c} differs from the LookupPageId {t} of the related table (which one opens is not pinned)"
                ));
            }
            (Some(c), _) => c.clone(),
            (None, Some(t)) => t.clone(),
            (None, None) => {
                return unsupported(format!(
                    "Lookup on a control with its own TableRelation to table {table_name}, which has no LookupPageId, and no LookupPageId on the control (not pinned)"
                ));
            }
        };
        let Some(page_object) = find_page(&wanted) else {
            if names_uncompiled_page(&engine, &wanted) {
                return unsupported(format!(
                    "the lookup page {wanted} belongs to a dependency app and was not compiled (no code the workspace reaches uses it)"
                ));
            }
            return unsupported(format!("the lookup page {wanted} names no page of the workspace"));
        };
        let source_table = engine.program.objects[page_object]
            .properties
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("SourceTable"))
            .map(|(_, v)| v.trim().trim_matches('"').to_string());
        if !source_table.is_some_and(|t| names(table_obj, &t)) {
            return unsupported(format!(
                "Lookup through page {wanted}, whose source table is not the related table {table_name} (not pinned)"
            ));
        }
        self.lookup_through_page(st, idx, field, page_object, pos)
    }

    /// The lookup page of a relation opens modally on a blank field and a confirmed pick is written back through the
    /// control: `pos` is the related field (`None`: the first primary key field). Pinned for a table field's relation
    /// (handlers/TestPageFieldRelationLookup_Tests.al:104, :122, :142) and a control's own relation (P115Lookups.al:108-124,
    /// P113Lookups.al:253-270).
    fn lookup_through_page(
        &mut self,
        st: &St,
        idx: usize,
        field: &ir::Field,
        page_object: usize,
        pos: Option<usize>,
    ) -> Result<Value, Signal> {
        let current = self.control_value(st, idx)?;
        let holds_value = !Self::same_values(
            std::slice::from_ref(&current),
            std::slice::from_ref(&Value::default_for(&field.ty)),
        );
        // A field that holds a value: the lookup page opens on the related row (pages/lt-page-runtime/P82Lookups.al:247
        // Lookup_TableRelation_Ok `handler-first = G1`; pages/lt-an-testpage-640/AnTestPageTests.al:480). Not built yet: making it
        // run lets pageext/lt-u-page-props/UPageProps.al:PageExt_LookupOnAndOff pass, after which four later tests of that
        // codeunit run and fail on behaviors that are not modelled (a page extension's DataCaptionExpression, an API page's
        // caption, UpdatePropagation on a card over the part's table, a profileextension's Enabled).
        if holds_value {
            return unsupported(
                "Lookup over a TableRelation from a field that already holds a value (the page's position is not pinned)",
            );
        }
        let related = Rc::new(RefCell::new(PageState::closed(page_object as u32)));
        related.borrow_mut().lookup = true;
        self.page_instance(&related)?;
        let blank = Rc::new(RefCell::new(self.page_rec(&related).borrow().clone()));
        let outcome = self.run_page_ui(&related, Some(blank), true)?;
        let confirmed = matches!(&outcome, Value::Option(n, members)
            if members.get(*n as usize).is_some_and(|m| m == "LookupOK"));
        if !confirmed {
            return Ok(Value::Null);
        }
        let picked = self.page_rec(&related);
        let table_def = self.table(&picked)?;
        let pos = pos.unwrap_or(table_def.keys[0][0]);
        let value = picked.borrow().fields[pos].clone();
        st.borrow_mut().refresh = false;
        // The control's OnAfterLookup(Selected: RecordRef) runs with the picked row before the value is written, so Rec still holds
        // the old value (pages/lt-pages-xmlport-upload-members/P112Pages.al:P112_Lookup_BasePageField_OK `base afterLookup sel=B rec=[]`,
        // P112_Lookup_ExtensionField_OK, P112_Lookup_ViewMode).
        {
            let engine = self.engine();
            let ctl = &page_of(&engine, st).controls[idx];
            let own = ctl.trigger("onafterlookup").map(|r| (ctl.owner, r));
            let added: Vec<(Option<usize>, usize)> = ctl
                .ext_triggers
                .iter()
                .filter(|(n, _, _)| n == "onafterlookup")
                .map(|(_, e, r)| (Some(*e), *r))
                .collect();
            let selected = Value::Library(
                "RecordRef".into(),
                Rc::new(RefCell::new(al_builtins::value::LibraryValue::RecordRef(
                    picked.clone(),
                ))),
            );
            for (owner, routine) in own.into_iter().chain(added) {
                let object = owner.unwrap_or(st.borrow().object as usize);
                let inst = self.owner_instance(st, owner);
                self.call(object, routine, inst, vec![selected.clone()])?;
            }
        }
        self.control_set_value(st, idx, &mut [value])?;
        Ok(Value::Null)
    }

    /// `TestField.DrillDown` on a control with a `DrillDownPageId` and no OnDrillDown trigger: that page opens non-modally
    /// and a `PageHandler` answers it; the control keeps its value (pages/lt-page-autoformat-lookups/P113Lookups.al:300-314
    /// P113_LK_DrilldownNonModal, a page of another table; pageext/lt-page-ext-properties/PageExtDrillDownPageId.al:64-79,
    /// the property set by a page extension's modify block). On a request page nothing opens and nothing is raised
    /// (report/lt-bj-request-drilldown/BjRequestDrillDown.al:7-9, :223-231). Which rows the page shows and the row it
    /// opens on are not pinned (both probes have one row in the target table), so a target table with more rows, or a
    /// field with a relation, is refused.
    fn drill_down_page(&mut self, st: &St, idx: usize, target: &str) -> Result<Value, Signal> {
        if self.is_request_page(st) {
            return Ok(Value::Null);
        }
        let engine = self.engine();
        let ctl = &page_of(&engine, st).controls[idx];
        if ctl.prop("tablerelation").is_some() || self.control_field(st, idx).is_some_and(|f| f.relation) {
            return unsupported(format!(
                "DrillDown through the DrillDownPageId of control {} over a TableRelation (the filter of the page that opens is not pinned)",
                ctl.name
            ));
        }
        let page_object = engine.program.objects.heads().iter().position(|o| {
            o.is_kind(al_ir::ObjKind::Page)
                && (o.name.eq_ignore_ascii_case(target) || o.id.is_some_and(|i| i.to_string() == target))
        });
        let Some(page_object) = page_object else {
            if names_uncompiled_page(&engine, target) {
                return unsupported(format!(
                    "the DrillDownPageId of control {} is page {target}, which belongs to a dependency app and was not compiled (no code the workspace reaches uses it)",
                    ctl.name
                ));
            }
            return unsupported(format!(
                "the DrillDownPageId {target} of control {} names no page of the workspace",
                ctl.name
            ));
        };
        let related = Rc::new(RefCell::new(PageState::closed(page_object as u32)));
        self.page_instance(&related)?;
        if self.has_source(&related) {
            let probe = Rc::new(RefCell::new(self.page_rec(&related).borrow().clone()));
            let rows = self.record_op(RecordOp::Count, &probe, &mut [])?;
            if !matches!(rows, Value::Integer(n) if n <= 1) {
                return unsupported(format!(
                    "DrillDown through DrillDownPageId {target} onto a table with more than one row (the filter and the first row of the page that opens are not pinned)"
                ));
            }
        }
        st.borrow_mut().refresh = false;
        self.run_page_ui(&related, None, false)?;
        self.run_refresh(st)?;
        Ok(Value::Null)
    }

    /// The page a list's `CardPageId` names: `None` when it names none.
    pub(super) fn list_card_page(&self, st: &St) -> Result<Option<usize>, Signal> {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let Some(card) = effective_prop(&engine, object, "CardPageId")?.map(|v| v.trim_matches('"').to_string()) else {
            return Ok(None);
        };
        engine
            .program
            .objects
            .heads()
            .iter()
            .position(|o| {
                o.is_kind(al_ir::ObjKind::Page)
                    && (o.name.eq_ignore_ascii_case(&card) || o.id.is_some_and(|i| i.to_string() == card))
            })
            .map(Some)
            .ok_or_else(|| Signal::Unsupported("CardPageId names a page outside the workspace".into()))
    }

    /// Whether the card a list's Edit action would open can be edited. Only `Editable = false`
    /// is pinned (corpus handlers/TestPageModeSwitchAction_Tests.al:
    /// ListWhoseCardIsReadOnlyLeavesTheEditActionVisibleButNotEnabled); any other restriction is refused.
    pub(super) fn card_allows_edit(&self, card: usize) -> Result<bool, Signal> {
        let engine = self.engine();
        let prop = |n: &str| {
            engine.program.objects[card]
                .properties
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(n))
                .map(|(_, v)| v.trim().to_ascii_lowercase())
        };
        if prop("ModifyAllowed").is_some_and(|v| v != "true") {
            return unsupported("the Edit action of a list whose card sets ModifyAllowed (not pinned)");
        }
        match prop("Editable").as_deref() {
            None | Some("true") => Ok(true),
            Some("false") => Ok(false),
            Some(_) => unsupported("the Edit action of a list whose card computes Editable (not pinned)"),
        }
    }

    /// The built-in Edit or View action of a list page: opens the page named by the list's
    /// `CardPageId` on the current row, editable or read-only, as a `RunObject` action would.
    pub(super) fn list_card_action(&mut self, st: &St, edit: bool) -> Result<Value, Signal> {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let Some(card) = effective_prop(&engine, object, "CardPageId")?.map(|v| v.trim_matches('"').to_string()) else {
            return unsupported("the built-in Edit/View action of a list page with no CardPageId (not pinned)");
        };
        let Some(target) = engine.program.objects.heads().iter().position(|o| {
            o.is_kind(al_ir::ObjKind::Page)
                && (o.name.eq_ignore_ascii_case(&card) || o.id.is_some_and(|i| i.to_string() == card))
        }) else {
            return unsupported("CardPageId names a page outside the workspace");
        };
        if st.borrow().cursor != Cursor::Row {
            return unsupported("the built-in Edit/View action of a list that stands on no stored row (not pinned)");
        }
        let ro = ir::RunObject {
            kind: ir::RunKind::Page,
            object: Some(target),
            on_rec: true,
            mode: if edit { ir::RunMode::Edit } else { ir::RunMode::View },
            link: None,
            unsupported: None,
        };
        st.borrow_mut().refresh = false;
        self.run_object_page(st, &ro)?;
        Ok(Value::Null)
    }

    /// `TestAction.Invoke`: runs the action's OnAction when it is enabled; a disabled action
    /// does nothing and raises nothing (corpus handlers/TestPageActionEnabledSourceField_Tests.al,
    /// TPAE_ActionEnabledBoolField_FalseRow_IsDisabledAndItsTriggerIsSkipped).
    pub(super) fn action_invoke(&mut self, st: &St, idx: usize) -> Result<Value, Signal> {
        if st.borrow().expr_set_no_source {
            return unsupported(
                "an action after a SetValue on a control bound to an expression of a page with no source table (the refresh after the action is not modelled)",
            );
        }
        // A duplicate-key Insert the action made comes out at the page's next read of the table, still inside Invoke, with the
        // wrapped code (errors/lt-az-error-texts/AzErrorTexts.al:167-185 PageAction_DuplicateInsert: `TestWrapped:CSideDuplicateKey`).
        // One that was pending before the action is not pinned and keeps its code.
        let pending_before = self.tx.deferred_insert.is_some();
        let raised_before = self.tx.deferred_raised;
        let r = self.action_invoke_body(st, idx);
        self.action_save_captions_done(st);
        match r {
            Err(Signal::Error(mut e))
                if !pending_before && self.tx.deferred_raised > raised_before && e.code == "DB:RecordExists" =>
            {
                e.code = action_error_code(&e.code, &e.message);
                Err(Signal::Error(e))
            }
            // An error with an empty text (`Error('')`, Gen. Jnl.-Post Preview ends every preview with one) out of the action
            // ends the Invoke quietly: the test goes on, the error becomes the last error (empty text, code Dialog) and the
            // rows the action wrote before it stay (pages/lt-qi-empty-error/QiEmptyErrorPages.al:192
            // QI_EE_EmptyAction_NoAsserterror `went-on` yes, `after-invoke|code` Dialog, `rows` 1=seed;100=empty action ran;).
            // The code is the error's own, not wrapped as the test client wraps an error it raises.
            Err(Signal::Error(mut e)) if e.message.is_empty() => {
                if let Some(own) = e.code.strip_prefix("TestWrapped:") {
                    e.code = own.to_string();
                }
                self.error_finals = None;
                self.last_error = *e;
                Ok(Value::Null)
            }
            other => other,
        }
    }

    fn action_invoke_body(&mut self, st: &St, idx: usize) -> Result<Value, Signal> {
        // An actionref invokes the action it stands for.
        let idx = match page_of(&self.engine(), st).controls[idx].action_ref.clone() {
            None => idx,
            Some(ir::ActionRef::Target(t)) => t,
            Some(ir::ActionRef::Unresolved(why)) => return unsupported(why),
        };
        if !self.control_enabled(st, idx)? {
            return Ok(Value::Null);
        }
        let engine = self.engine();
        // A client sends the row it stands on to the server before it runs an action (corpus
        // handlers/TestPageAutoSplitKey_Tests.al:TestPage_ActionInvoke_OnANewSubpageRow_*).
        let was_dirty = st.borrow().dirty;
        if st.borrow().host.is_some() {
            self.save_row(st)?;
        }
        // A new row of a card whose key a SetValue typed is stored before the next action's trigger (pages/lt-table-globals-page/
        // TableGlobalsPageWithoutValidation.al:318 QG_NewDraft_KeyTyped_ThenBumpAction: `stored after the SetValue` No, `stored
        // after BumpOnly` Yes, `Seen after BumpOnly` I0, `LastCount after ReadCount` 2).
        // Pinned for an action with OnAction only; before a RunObject action it is not pinned and stays as it was.
        let plain_draft = page_of(&engine, st).controls[idx].run_object.is_none()
            && page_of(&engine, st).controls[idx].trigger("onaction").is_some()
            && self.plain_card_draft(st)?;
        if was_dirty && plain_draft && self.draft_key_changed(st)? {
            let rec = self.page_rec(st);
            self.reset_table_globals(&rec);
            self.save_row(st)?;
        }
        let draft_before = (plain_draft && st.borrow().draft).then(|| self.page_rec(st).borrow().fields.clone());
        st.borrow_mut().refresh = false;
        // An edit the client sends before the action (a control SetValue left the page dirty) is
        // saved, and the saved row is read again: OnAfterGetRecord of the base runs before the
        // action (corpus handlers/TestPageExtensionBaseAccessContracts.al:338-342, where the base
        // copies a variable an extension control set into a variable the action logs).
        // A page with OnAfterGetRecord reads that row as a clean page does, and nothing more
        // (pageext/PCCurrRecordReads.al:Card_DirtyPage_ReadsAfterEachAction and
        // List_DirtyPage_ReadsAfterEachAction: one OnAfterGetRecord before the action body, no
        // OnAfterGetCurrRecord); the reads after it follow below.
        // After a GoToKey that moved a list without OnAfterGetCurrRecord a SetValue before the action changes nothing in what it
        // reads, also on a list with OnFindRecord and OnNextRecord (pages/lt-ai-list-order/QgGoToKeyNextCall.al:477
        // Nav_GoToKey_ThenSetValue_ThenAction `action_log` as in Nav_GoToKey_ThenActionWithoutUpdate_Twice).
        let goto_nav = st.borrow().read_after_goto
            && self.is_list(st)
            && self.declares(st, "onfindrecord")
            && self.declares(st, "onnextrecord")
            && !self.declares(st, "onaftergetcurrrecord");
        let dirty_reread_only = was_dirty
            && self.declares(st, "onaftergetrecord")
            && (!self.declares(st, "onfindrecord") && !self.declares(st, "onnextrecord")
                || self.declares(st, "onfindrecord") && self.declares(st, "onaftergetcurrrecord")
                || goto_nav)
            && self.has_source(st)
            && st.borrow().open
            && st.borrow().cursor == Cursor::Row
            && !st.borrow().draft
            && page_of(&engine, st).controls[idx].run_object.is_none()
            && page_of(&engine, st).controls[idx].trigger("onaction").is_some();
        if dirty_reread_only && st.borrow().host.is_none() {
            // The edit reaches the server with the action, so the rows read after it hold it
            // (Name reads 'dirty' after the action in the same tests).
            self.save_row(st)?;
        }
        if was_dirty && !self.declares(st, "onfindrecord") && !self.declares(st, "onnextrecord") && !dirty_reread_only {
            // On a Document page this save runs the host's CaptionClass expressions and not the part's
            // (`fill_field_caption_expressions`, QdCaptionClassAreas.al:1015-1049 QO_Doc_OpenEdit_HostSteps).
            let doc = st.borrow().host.is_none()
                && st.borrow().field_captions_opened
                && self.engine.object_facts[st.borrow().object as usize].page_type == Some(al_ir::PageType::Document);
            if doc {
                st.borrow_mut().field_captions_action_save = 1;
            }
            st.borrow_mut().refresh = true;
            st.borrow_mut().refresh_by_update = false;
            let r = self.run_refresh(st);
            if doc {
                st.borrow_mut().field_captions_action_save = 2;
            }
            r?;
            st.borrow_mut().refresh = false;
        }
        // The client reads the row it stands on before an action runs (corpus
        // pageext/PCCurrRecordReads.al:Card_ReadsAfterEachAction and List_ReadsAfterEachAction: one
        // OnAfterGetRecord before the action's own trigger, only when the row is not dirty, which has
        // its re-read above).
        // A list that serves its rows from OnFindRecord and OnNextRecord does the same (misc/W2FindRecordUpdate.al: during_action
        // `AGR(20);ActionBegin;ActionEnd;`).
        // (misc/W2FindRecordUpdate.al for a list; pages/lt-update-on-find/UpdateOnFindRecord.al for a card and a list that
        // declare OnAfterGetCurrRecord: `AGR:2;act:Update(false)` first.)
        let find_list =
            self.declares(st, "onfindrecord") && (self.is_list(st) || self.declares(st, "onaftergetcurrrecord"));
        // A card with OnFindRecord and no OnNextRecord reads its row before the action, again after it, then asks `=` and reads
        // it once more (pages/lt-be-card-onfindrecord/BeCardOnFindRecord.al:292 CardTempFind_PageRun_TempRec `action`
        // `R;A;R;F:=;R`, and the CardTempFind_* and CardDbFind_* tests).
        let find_card = self.declares(st, "onfindrecord") && !self.declares(st, "onnextrecord") && !self.is_list(st);
        let reads_rows = (!was_dirty || dirty_reread_only)
            && self.declares(st, "onaftergetrecord")
            && (find_list || find_card || !self.declares(st, "onnextrecord") && !self.declares(st, "onfindrecord"))
            && self.has_source(st)
            && st.borrow().open
            && st.borrow().cursor == Cursor::Row
            && !st.borrow().draft
            && page_of(&engine, st).controls[idx].run_object.is_none()
            && page_of(&engine, st).controls[idx].trigger("onaction").is_some();
        // Every action after such a GoToKey reads the row twice before its trigger: the second action too, and an action after
        // a Value read, a SetValue, a Previous or a RunObject action (pages/lt-ai-list-order/QgGoToKeyNextCall.al:236, :254, :278,
        // :297, :317, :336 and the Nav_ tests, `action_log` `R:R1003;R:R1003;A:act=R1003`). So the mark stays until a move clears
        // it (`pageops.rs`). A RunObject action reads nothing (:278 `runobject_log` `T:open;H:target`). After a call that is not
        // pinned (a property read) between them the action is refused.
        let (read_after_goto, doubt) = {
            let mut s = st.borrow_mut();
            s.read_after_goto_set = false;
            s.read_after_goto_prev = false;
            (s.read_after_goto, s.read_after_goto_doubt)
        };
        if read_after_goto && doubt {
            return unsupported(
                "an action after a GoToKey moved a list without OnAfterGetCurrRecord, with a field call between them (not pinned)",
            );
        }
        if reads_rows {
            let rec = self.page_rec(st);
            if self.stored_row(&rec)?.is_some() {
                // A GoToKey that moved a list without OnAfterGetCurrRecord: the row is read once more first
                // (pages/lt-ai-list-order/AiListOrderTests.al:39, :233, :301, :335 `R:R1002;R:R1002;A:...`; QgGoToKeyNextCall.al).
                if read_after_goto {
                    self.populate_current(st)?;
                }
                self.populate_current(st)?;
            }
        }
        // RunObject wins over OnAction when an action declares both (handlers/TestPageRunObjectContracts.al:371-390).
        if let Some(ro) = page_of(&engine, st).controls[idx].run_object.clone() {
            // RunObject = Page raises neither event; RunObject = Codeunit raises only OnAfterActionEvent, after the codeunit ran,
            // and an error in it reaches Invoke (events/lt-page-action-subscriber-errors/PageActionSubscriberErrors.al:
            // RunPage_EventOrder, RunPage_AfterSubscriberConfiguredToRaise, RunCodeunit_EventOrder,
            // RunCodeunit_BeforeSubscriberConfiguredToRaise, RunCodeunit_AfterSubscriberRaises). The events around the other kinds
            // are not pinned: a subscriber to them makes the action refused.
            let element = page_of(&engine, st).controls[idx].name.clone();
            let object = st.borrow().object as usize;
            let listens = |ev: &str| {
                engine.events.is_subscribed(object, ev)
                    && engine
                        .events
                        .subscribers
                        .get(&(object, ev.to_string()))
                        .is_some_and(|v| {
                            v.iter()
                                .any(|s| s.element.is_empty() || s.element.eq_ignore_ascii_case(&element))
                        })
                    || engine.events.missing_for(object, ev, Some(&element)).is_some()
            };
            let (before_pinned, after_pinned) = match ro.kind {
                ir::RunKind::Page => (true, true),
                ir::RunKind::Codeunit => (true, true),
                _ => (false, false),
            };
            for (ev, pinned) in [
                ("onbeforeactionevent", before_pinned),
                ("onafteractionevent", after_pinned),
            ] {
                if !pinned && listens(ev) {
                    return unsupported(format!(
                        "page event {ev} of the RunObject action {element}: the order around this kind of RunObject is not pinned"
                    ));
                }
            }
            let run = self.run_object_action(st, idx);
            let run = match (run, ro.kind) {
                (Ok(v), ir::RunKind::Codeunit) => {
                    self.page_action_event(st, "onafteractionevent", &element).map(|()| v)
                }
                (other, _) => other,
            };
            return match run {
                Err(Signal::Error(mut e)) if e.code.as_str() == "Dialog" && ro.kind == ir::RunKind::Codeunit => {
                    e.code = "TestWrapped:Dialog".into();
                    Err(Signal::Error(e))
                }
                other => other,
            };
        } else if let Some(r) = page_of(&engine, st).controls[idx].trigger("onaction") {
            let before = (self.has_source(st) && st.borrow().cursor == Cursor::Row && !st.borrow().draft)
                .then(|| self.page_rec(st).borrow().fields.clone());
            // A card whose row was never stored (a temporary source nothing inserted into: Tasks over "RM Matrix
            // Management") has no row for the action to delete, so it stays open.
            let stored_before = !st.borrow().never_stored;
            st.borrow_mut().save_record_called = false;
            st.borrow_mut().refresh_update_false = false;
            let empty_view = self.empty_list_view(st)?;
            let empty = self.empty_page_shape(st)?;
            let held_before = (empty.is_none()
                && self.is_list(st)
                && self.has_source(st)
                && st.borrow().host.is_none()
                && st.borrow().cursor == Cursor::Row
                && !st.borrow().draft
                && !st.borrow().window.is_empty())
            .then(|| st.borrow().window.iter().map(|w| w.key.clone()).collect::<Vec<_>>());
            if let Some(shape) = empty {
                self.empty_page_before_action(st, shape)?;
            }
            // Subscribers to the action's events: before the trigger (what they write to Rec the trigger reads and the
            // page saves, because `before` was taken first), after it when it did not fail
            // (events/lt-global-triggers-statements/P114PageActionEvents.al).
            let element = page_of(&engine, st).controls[idx].name.clone();
            let shown_outer = {
                let shown = if self.is_list(st) && st.borrow().host.is_none() {
                    before.clone()
                } else {
                    None
                };
                std::mem::replace(&mut st.borrow_mut().action_shown_row, shown)
            };
            let r = match self.page_action_event(st, "onbeforeactionevent", &element) {
                Ok(()) => self.control_routine(st, idx, r),
                err => err.map(|()| Value::Null),
            };
            st.borrow_mut().action_shown_row = shown_outer;
            let r = match r {
                Ok(v) => self.page_action_event(st, "onafteractionevent", &element).map(|()| v),
                err => err,
            };
            // An error an action raises comes out of the test page's Invoke with the test framework's prefix on its code.
            let r = match r {
                Err(Signal::Error(mut e)) => {
                    e.code = action_error_code(&e.code, &e.message);
                    Err(Signal::Error(e))
                }
                other => other,
            };
            if r.is_err() {
                st.borrow_mut().refresh = false;
            }
            r?;
            if empty_view && self.update_on_empty_list_view(st)? {
                return Ok(Value::Null);
            }
            if let Some(shape) = empty {
                if self.empty_page_after_action(st, shape)? {
                    return Ok(Value::Null);
                }
            }
            if before.is_some() && self.reread_after_refill_to_fewer_rows(st)? {
                return Ok(Value::Null);
            }
            // A temporary Worksheet whose action left Rec on another row (a refill: RmTempRefill.al:323-:420) is read again from the
            // row the client stood on and Rec's row (`reread_after_refill_by_action`), with or without CurrPage.Update.
            if let Some(before) = &before {
                let held = self.table(&self.page_rec(st))?.primary_key(before);
                let now = self.row_key(&self.page_rec(st))?;
                if !Self::same_key(&held, &now) && self.temp_refill_shape(st) {
                    st.borrow_mut().refresh = false;
                    self.reread_after_refill_by_action(st, &held)?;
                    return Ok(Value::Null);
                }
            }
            if let Some(held) = &held_before {
                let rec = self.page_rec(st);
                if st.borrow().cursor == Cursor::Row && !st.borrow().draft && self.stored_row(&rec)?.is_some() {
                    let key = self.row_key(&rec)?;
                    if !held.iter().any(|k| Self::same_key(k, &key)) {
                        st.borrow_mut().action_landed_on_new_row = true;
                    }
                }
            }
            // A new row of a card whose key the action set is stored right after the action, on fresh table globals: its
            // OnInsert and the next action see none of what the action set in them (pages/lt-table-globals-page/
            // TableGlobalsPageWithoutValidation.al:235 QG_NewDraft_KeySetByAction_WhenStored `stored after the action` Yes, `Seen
            // read` I0, `LastCount after the second action` 0; :272 _ThenCloseAtOnce; :293 _ThenBumpAction; :187
            // Card_NewDraft_KeySetByActionThenClose). An action that changes nothing leaves it as it is (:348
            // QG_NewDraft_NoKey_BumpAction_ThenClose). One that changes other fields of it and not the key is not pinned.
            if let Some(before) = draft_before.filter(|_| st.borrow().draft && st.borrow().open) {
                let rec = self.page_rec(st);
                if self.stored_row(&rec)?.is_none() && !Self::same_values(&rec.borrow().fields, &before) {
                    if !self.draft_key_changed(st)? {
                        return unsupported(
                            "an action that changes a new row of a card but not its key (when the row is stored is not pinned)",
                        );
                    }
                    self.reset_table_globals(&rec);
                    st.borrow_mut().dirty = true;
                    self.save_row(st)?;
                }
            }
            // An action that put another row (another primary key) into Rec on a list: the client saves nothing, now, on Next or
            // on Close (no OnModifyRecord, no OnModify), reads the list again from that row with its stored values, and the next
            // OnAfterGetCurrRecord sees the row shown before as xRec (misc/lt-probes-needed-7/PnGRefresh.al:387
            // QL_RecAssign_OtherChanged_ThenClose, :416 _OtherChangedUpdate_ThenClose with CurrPage.Update(false), :445
            // _TempChanged_ThenClose from a temporary record, :532 _OtherChanged_StaysOpen_Next, :570 _TempChanged_StaysOpen_Next).
            // Pinned for a top-level list with OnAfterGetRecord and OnAfterGetCurrRecord and without OnFindRecord/OnNextRecord;
            // another page keeps the refusal below.
            let moved_other = match &before {
                Some(before) if st.borrow().cursor == Cursor::Row && !st.borrow().draft && st.borrow().open => {
                    let rec = self.page_rec(st);
                    let table = self.table(&rec)?;
                    !Self::same_key(&table.primary_key(&rec.borrow().fields), &table.primary_key(before))
                }
                _ => false,
            };
            let moved_pinned = moved_other
                && self.is_list(st)
                && st.borrow().host.is_none()
                && self.declares(st, "onaftergetrecord")
                && self.declares(st, "onaftergetcurrrecord")
                && !self.declares(st, "onfindrecord")
                && !self.declares(st, "onnextrecord");
            if moved_pinned {
                st.borrow_mut().rec_moved_other = true;
            }
            // Rec left by CurrPage.Update(false) on a primary key no row has (page 5510 "Production Journal" Post): see the
            // refusal of `rec_moved_other` below.
            let moved_to_unstored_by_update_false =
                moved_pinned && st.borrow().refresh && st.borrow().refresh_update_false && {
                    let rec = self.page_rec(st);
                    self.stored_row(&rec)?.is_none()
                };
            // What the action left in Rec is saved when the client leaves the row (corpus
            // record/PWTableGlobals.al:408-431, PageModify_AfterActionChangedRec_NoValidation).
            if let Some(before) =
                before.filter(|_| st.borrow().cursor == Cursor::Row && !st.borrow().draft && !st.borrow().refresh)
            {
                let rec = self.page_rec(st);
                // A change the action saved itself (`Rec.Modify()` in the action) is not saved again: the
                // page runs no OnModify for it (misc/W2TableGlobals.al:Modify_ActionCallsModifyItself).
                let saved_by_action = self
                    .stored_row(&rec)?
                    .is_some_and(|stored| Self::same_values(&rec.borrow().fields, &stored));
                if !Self::same_values(&rec.borrow().fields, &before) && !saved_by_action {
                    // Another row in Rec saves nothing on the pinned list (above; Tests-ERM 134918 Apply Customer Entries' Post
                    // Application `Rec := TempApplyingCustLedgEntry`). On another page that is not pinned.
                    if moved_other {
                        if !moved_pinned {
                            return unsupported(
                                "an action left another row (another primary key) in Rec with changes to save (not pinned)",
                            );
                        }
                    } else if reads_rows
                        && self.is_list(st)
                        && st.borrow().host.is_none()
                        && st.borrow().window_from_start
                        && !st.borrow().save_record_called
                        && self.declares(st, "onaftergetrecord")
                        && self.declares(st, "onaftergetcurrrecord")
                        && !self.declares(st, "onfindrecord")
                        && !self.declares(st, "onnextrecord")
                    {
                        // On such a list the client first reads the changed row as the action left it, then saves it, then reads
                        // the list again; the next OnAfterGetCurrRecord sees the saved row as xRec (misc/lt-probes-needed-7/
                        // PnGRefresh.al:503 QL_RecAssign_OwnRowChanged_ThenClose `ACT.end(A,own changed x=A,orig A);P.AGR(A,own
                        // changed);P.MODR(...);T.MOD(...);P.AGR(A,own changed);...;P.AGCR(A,own changed x=A,own changed)`).
                        st.borrow_mut().dirty = true;
                        st.borrow_mut().save_after_first_read = true;
                    } else {
                        st.borrow_mut().dirty = true;
                        self.save_row(st)?;
                    }
                }
            }
            let settle = stored_before || self.is_list(st) || {
                let rec = self.page_rec(st);
                self.stored_row(&rec)?.is_some()
            };
            if settle {
                self.settle_after_action(st)?;
            }
            // The client reads the current row again after every action, so OnAfterGetCurrRecord of
            // the page and of its extensions runs again (corpus
            // handlers/TestPageExtensionBaseMembersContracts.al:195-218: "ext AfterGetCurrRecord"
            // after the extension's action and after the base's). A page that serves its rows from
            // OnFindRecord is left as `CurrPage.Update` alone re-reads it (re-anchoring not pinned).
            let refresh_asked = st.borrow().refresh;
            let find_page = self.declares(st, "onfindrecord") || self.declares(st, "onnextrecord");
            let find_pinned = find_page
                && reads_rows
                && self.declares(st, "onfindrecord")
                && !self.is_list(st)
                && (self.declares(st, "onaftergetcurrrecord") || find_card);
            if find_pinned {
                // pages/lt-update-on-find/UpdateOnFindRecord.al:P168_List_Update*, P168_Card_Update*: the reads come before
                // the refresh's `Find:=`. Without CurrPage.Update and without OnAfterGetCurrRecord the refresh is pinned
                // by BeCardOnFindRecord.al (above), not assumed.
                self.read_page_again(st)?;
                let pinned = !refresh_asked && !self.declares(st, "onaftergetcurrrecord");
                st.borrow_mut().refresh = true;
                st.borrow_mut().refresh_by_update = false;
                st.borrow_mut().refresh_pinned = pinned;
                let r = self.run_refresh(st);
                st.borrow_mut().refresh_pinned = false;
                r?;
            } else if !find_page {
                // The reads the page does when it opens come first, then the refresh's own: a card
                // reads its row twice, a list the rows of its window as it opens plus one (corpus
                // pageext/PCCurrRecordReads.al:Card_ReadsAfterEachAction, List_ReadsAfterEachAction,
                // pinned for a card and a list of 3 rows with the cursor on the first).
                // An action that called CurrPage.SaveRecord (and no Update) is followed by one read of the row, not two
                // (pages/lt-page-runtime/P82CurrPage.al:SaveRecord_StoresPendingEdit and SaveRecord_InViewMode:
                // `...;saved;OnAfterGetRecord(A)`). With an Update in the same action it is not pinned and stays as it was.
                let saved = std::mem::take(&mut st.borrow_mut().save_record_called) && !refresh_asked;
                if reads_rows && !saved {
                    self.read_page_again(st)?;
                }
                // On a Document card that read is OnAfterGetRecord alone, no OnAfterGetCurrRecord, then the parts read their rows
                // (pages/lt-qj-part-propagation/QjPartPropagation.al:511 QJ_Both_PartModifiesHost_ThenHostSave
                // `H.ACT.end(...) H.AGR(...) L.AGR(...) L.AGR(...) L.AGCR(...)`, also :548, :585, :627, :665, :703).
                if saved && !self.is_list(st) && !st.borrow().parts.is_empty() {
                    st.borrow_mut().refresh_without_curr = true;
                }
                st.borrow_mut().refresh = true;
                st.borrow_mut().refresh_by_update = false;
            } else if read_after_goto && reads_rows && self.is_list(st) && !self.declares(st, "onaftergetcurrrecord") {
                // After such a GoToKey an action without CurrPage.Update on a list with OnFindRecord and OnNextRecord is followed by
                // the same refresh as one with it (pages/lt-ai-list-order/QgGoToKeyNextCall.al:566 Nav_GoToKey_ThenActionWithoutUpdate_Twice
                // `R:R1003;R:R1003;A:act=R1003;R:R1003;F:=>=Yes;R:R1003;N:1=1;...` against :607 Nav_GoToKey_ThenValueRead_ThenUpdateAction).
                // Without that GoToKey it is not pinned and stays as it was.
                st.borrow_mut().refresh = true;
                st.borrow_mut().refresh_by_update = false;
            }
            if !find_pinned {
                self.run_refresh(st)?;
            }
            // The read again from the other row is pinned only through `read_list_again_from_row`; a list that read itself in
            // another way is refused.
            if std::mem::take(&mut st.borrow_mut().save_after_first_read) {
                return unsupported(
                    "an action changed the shown row of a list that is not read again from that row (not pinned)",
                );
            }
            if std::mem::take(&mut st.borrow_mut().rec_moved_other) {
                // An action that emptied the view of a list without the new-row line and left Rec, with CurrPage.Update(false), on
                // a primary key no row has: the list stands on the blank Rec as when Rec keeps the deleted row's key
                // (`settle_on_blank_rec`; al-runner issue 6027, the reporter's repro on real BC `N:B1;C:0:B1;`; registered
                // assumption `gone-row-rec-on-unheld-key`). Rows left in the view, or no CurrPage.Update(false), are not pinned.
                let on_blank = {
                    let s = st.borrow();
                    s.gone_blank_no_new_line && s.open && s.cursor == Cursor::None && !s.draft
                };
                if !(on_blank && moved_to_unstored_by_update_false) {
                    return unsupported(
                        "an action left another row (another primary key) in Rec on a list that is not read again from that row (not pinned)",
                    );
                }
                al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::GoneRowRecOnUnheldKey);
            }
            // A card without a source table runs OnAfterGetCurrRecord once after each action, also after one that called
            // CurrPage.Update(false) (misc/lt-protected-var-abs-query-views/P146PageExprSetValue.al:351
            // P146_NoSourceTable_SetValue_And_Action `log after bump`, :420 QG_NoSource_OpenEdit_StepLog `qg action;qg agcr;`,
            // `qg update;qg update done;qg agcr;`). Another page type is not pinned.
            if !self.has_source(st) && st.borrow().open && !self.is_request_page(st) {
                if !self.is_card(st) {
                    if self.declares(st, "onaftergetcurrrecord") {
                        return unsupported(
                            "an action on a page without a source table other than a card that declares OnAfterGetCurrRecord (not pinned)",
                        );
                    }
                } else {
                    self.page_trigger(st, "onaftergetcurrrecord", vec![])?;
                    self.page_event(st, "onaftergetcurrrecordevent", None, None)?;
                }
            }
            // An action that closed the page: the client still reads the row once more (pages/lt-page-runtime/P82PageRun.al:
            // RunModal_CurrPageClose_Action `...;OnClosePage;OnAfterGetRecord(A)`).
            // A list whose action put another row into Rec and closed the page: CurrPage.Close() read the shown row back into Rec,
            // and the client reads it once more after the action (misc/lt-probes-needed-7/PnGRefresh.al:608
            // QL_RecAssign_OtherChanged_CloseInAction `P.AGR(A,orig A);P.AGR(A,orig A);ACT.end(A,orig A x=A,orig A);P.AGR(A,orig A)`).
            if std::mem::take(&mut st.borrow_mut().close_restored_row) {
                self.populate_current(st)?;
            } else if reads_rows && !st.borrow().open && !self.is_list(st) && !self.declares(st, "onaftergetcurrrecord")
            {
                let rec = self.page_rec(st);
                if self.stored_row(&rec)?.is_some() {
                    self.populate_current(st)?;
                }
            }
        }
        Ok(Value::Null)
    }

    /// A page over a table that shows no stored row when an action with OnAction runs: a list opened for edit that stands on its
    /// new-row line, a list or a card opened for view that shows nothing (misc/lt-probes-needed-7/PnGShowRow.al, pages/W1ShowRow.al).
    /// A page with OnFindRecord or OnNextRecord, a part, a host of parts, and a page with an edit pending are not this shape.
    fn empty_page_shape(&mut self, st: &St) -> Result<Option<EmptyShape>, Signal> {
        let shape = {
            let s = st.borrow();
            if !s.open || s.host.is_some() || !s.parts.is_empty() || !s.window.is_empty() || s.dirty {
                return Ok(None);
            }
            match (s.cursor, s.draft, s.mode) {
                (Cursor::NewLine, true, PageMode::Edit) if s.new_line => EmptyShape::EditList,
                (Cursor::None, _, PageMode::View) => EmptyShape::View,
                _ => return Ok(None),
            }
        };
        if !self.has_source(st)
            || self.is_request_page(st)
            || self.declares(st, "onfindrecord")
            || self.declares(st, "onnextrecord")
        {
            return Ok(None);
        }
        match (shape, self.is_list(st)) {
            (EmptyShape::EditList, true) => Ok(Some(EmptyShape::EditList)),
            (EmptyShape::View, true) => Ok(Some(EmptyShape::View)),
            // A card opened for view over an empty table that an action gave its one row shows it (pages/W1ShowRow.al:317
            // Card_ViewOpen_InsertThroughAnotherVariable); which row triggers run then is not pinned.
            (EmptyShape::View, false) if self.is_card(st) => {
                if self.declares(st, "onaftergetrecord") || self.declares(st, "onaftergetcurrrecord") {
                    return unsupported(
                        "an action on a card opened for view that shows no row, with row triggers (which run is not pinned)",
                    );
                }
                Ok(Some(EmptyShape::ViewCard))
            }
            _ => Ok(None),
        }
    }

    /// A list opened for view that shows no row (its view was empty), with or without OnFindRecord and OnNextRecord.
    fn empty_list_view(&mut self, st: &St) -> Result<bool, Signal> {
        let shape = {
            let s = st.borrow();
            s.open
                && s.host.is_none()
                && s.parts.is_empty()
                && s.window.is_empty()
                && !s.dirty
                && s.cursor == Cursor::None
                && s.mode == PageMode::View
        };
        Ok(shape && self.is_list(st) && self.has_source(st) && !self.is_request_page(st))
    }

    /// After an action on a list opened for view that showed no row, which changed the filters so the view has rows and
    /// called `CurrPage.Update(false)`, while Rec stands on no row of the view: the client fills the page as when it opens,
    /// from the first row of the new view, which becomes current; a page with OnFindRecord asks `=><` and then `=`
    /// (pages/lt-ai-list-order/AiListOrderTests.al QP_EmptyView_UpdateFalse_ShowAll_Nav, _ShowFromThree_Nav, _ShowAll_Plain,
    /// _ShowFromThree_Plain; pages 87158, 87159 in QpUpdateEmptyList.al; the traces equal the open traces of :105 and :265).
    /// Pinned for views of 4 and 6 rows that the window holds. A view of one row is `empty_page_after_action`'s; Update(true),
    /// no Update, or Rec on a row of the view go on as before.
    fn update_on_empty_list_view(&mut self, st: &St) -> Result<bool, Signal> {
        {
            let s = st.borrow();
            if !(s.open
                && s.refresh
                && s.refresh_by_update
                && s.refresh_update_false
                && !s.dirty
                && s.cursor == Cursor::None)
            {
                return Ok(false);
            }
        }
        let n = self.view_row_count(st)?;
        if n < 2 {
            return Ok(false);
        }
        let rec = self.page_rec(st);
        let on_row = self
            .stored_row(&rec)?
            .is_some_and(|f| rec.borrow().filters.iter().all(|x| x.expr.matches(&f[x.field])));
        if on_row {
            return Ok(false);
        }
        if n > 6 {
            return unsupported(
                "CurrPage.Update(false) after an action on a list opened for view that showed no row, whose new view has more than 6 rows (the fill is pinned for 4 and 6)",
            );
        }
        {
            let mut s = st.borrow_mut();
            s.refresh = false;
            s.refresh_by_update = false;
            s.refresh_update_false = false;
        }
        self.page_first_row(st)?;
        Ok(true)
    }

    /// The rows of the page's view, counted on a copy of its record.
    fn view_row_count(&mut self, st: &St) -> Result<i32, Signal> {
        let rec = self.page_rec(st);
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        self.ret_used = true;
        Ok(match self.record_op(RecordOp::Count, &probe, &mut [])? {
            Value::Integer(n) => n,
            _ => 0,
        })
    }

    /// Before the action's trigger. A list opened for view over an empty table reads its blank record once when the test stored
    /// a row of the view since (misc/lt-probes-needed-7/PnGShowRow.al:459 QG_ViewEmptyList_RowStoredByTest_ThenReads `action_log`
    /// `List.AGR(,);ReadTable(,No);...`), and nothing when the view is still empty (:364 QG_ViewEmptyList_InsertViaRec_StepLog).
    /// Rows stored before the action are pinned for no other shape.
    fn empty_page_before_action(&mut self, st: &St, shape: EmptyShape) -> Result<(), Signal> {
        let n = self.view_row_count(st)?;
        match (shape, n) {
            (_, 0) => Ok(()),
            (EmptyShape::View, 1) => {
                self.calc_row_flowfields(st)?;
                self.populate_expressions(st)?;
                self.page_trigger(st, "onaftergetrecord", vec![])?;
                Ok(())
            }
            // A Worksheet with the new-row line opened for edit on an empty view, left on that line by an action that inserted
            // rows through another record variable: the next action first reads the line, OnAfterGetRecord on the blank Rec
            // (pages/lt-outside-worksheet/RmWorksheetGoneRow.al:573 SH_WsWithNew_EmptyOpen_FillThenDeleteAllUpdate, two rows).
            // Another number of rows is the registered assumption `empty-worksheet-filled-by-action`.
            (EmptyShape::EditList, _)
                if self.engine.object_facts[st.borrow().object as usize].page_type
                    == Some(al_ir::PageType::Worksheet) =>
            {
                if n != 2 {
                    al_builtins::assumptions::assume(
                        al_builtins::assumptions::Assumption::EmptyWorksheetFilledByAction,
                    );
                }
                self.calc_row_flowfields(st)?;
                self.populate_expressions(st)?;
                self.page_trigger(st, "onaftergetrecord", vec![])?;
                Ok(())
            }
            _ => unsupported(
                "an action on a page that shows no row while its view holds rows (what the client reads first is not pinned)",
            ),
        }
    }

    /// After the action's trigger on a page that showed no stored row (`empty_page_shape`). Pinned for a view of none or one row
    /// after the action (misc/lt-probes-needed-7/PnGShowRow.al, writer QG):
    /// - Rec is that row (the action inserted it through Rec): the client reads it three times, then OnAfterGetCurrRecord, with
    ///   xRec on the row in edit mode and blank in view mode, and stays on it (:176 QG_EmptyList_InsertViaRec_StepLog
    ///   `List.AGR(NEW1,Created)` three times `;List.AGCR(NEW1,NEW1)`, :270 _InsertViaRecAndUpdate, :364
    ///   QG_ViewEmptyList_InsertViaRec_StepLog `List.AGCR(NEW1,)`).
    /// - another row (inserted through another variable, or stored by the test): read twice, OnAfterGetCurrRecord with a blank
    ///   xRec; a list opened for edit then goes back to its new-row line, OnNewRecord and OnAfterGetCurrRecord again (:223
    ///   QG_EmptyList_InsertViaOtherVariable_StepLog `List.AGR(NEW2,Created);List.AGR(NEW2,Created);List.AGCR(NEW2,);
    ///   List.OnNewRecord;List.AGCR(,)`), a list opened for view stays on the row (:411, :459).
    /// - no row: a list opened for edit runs its new-row line twice, as when it opened (:317 QG_EmptyList_ActionStoresNothing_StepLog).
    ///
    /// A card opened for view shows the one row (pages/W1ShowRow.al:317). More rows, and an empty view after an action on a page
    /// opened for view, are not pinned.
    /// Answers whether it handled the action; the other cases go on as any action does: a view of more rows (testpart/
    /// BnrBlankPart.al:491 NoRow_Action_TempListTwoInsertsAndFindLast_ShowsTheLastRow) and an empty view of a page with row triggers
    /// opened for view (pages/lt-ap-testpage-642/ApTestPageTests.al:602 Ap10_SetSelectionFilter_FilterHidesAll_TestPageFilter) are pinned
    /// there.
    fn empty_page_after_action(&mut self, st: &St, shape: EmptyShape) -> Result<bool, Signal> {
        if !st.borrow().open {
            return Ok(false);
        }
        let n = self.view_row_count(st)?;
        let rec = self.page_rec(st);
        let table = rec.borrow().table as usize;
        if n > 1 {
            // A Worksheet with the new-row line opened for edit on an empty view whose action stored rows through another record
            // variable: the client reads the rows of the view once each, then the first one twice, runs OnAfterGetCurrRecord on
            // it with a blank xRec and goes back to its new-row line (pages/lt-outside-worksheet/RmWorksheetGoneRow.al:573
            // SH_WsWithNew_EmptyOpen_FillThenDeleteAllUpdate, :601 _FillThenWalk `calculate log =
            // R:10000;R:20000;R:10000;R:10000;C:10000;N:0;C:0;`). Pinned with two rows; more rows up to a window are the
            // registered assumption `empty-worksheet-filled-by-action`; a view longer than a window is refused.
            let worksheet =
                self.engine.object_facts[st.borrow().object as usize].page_type == Some(al_ir::PageType::Worksheet);
            if shape != EmptyShape::EditList || !worksheet || self.stored_row(&rec)?.is_some() {
                return Ok(false);
            }
            if n as usize > Self::WINDOW {
                return unsupported(
                    "an action that stored more rows than a window into an empty Worksheet standing on its new-row line (not pinned)",
                );
            }
            if n != 2 {
                al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::EmptyWorksheetFilledByAction);
            }
            st.borrow_mut().refresh = false;
            st.borrow_mut().save_record_called = false;
            let probe = Rc::new(RefCell::new(rec.borrow().clone()));
            let mut rows = Vec::new();
            self.ret_used = true;
            if matches!(
                self.record_op(RecordOp::FindFirst, &probe, &mut [])?,
                Value::Boolean(true)
            ) {
                loop {
                    rows.push(probe.borrow().fields.clone());
                    self.ret_used = true;
                    if !matches!(self.record_op(RecordOp::Next, &probe, &mut [Value::Integer(1)])?, Value::Integer(n) if n != 0)
                    {
                        break;
                    }
                }
            }
            {
                let mut s = st.borrow_mut();
                s.cursor = Cursor::Row;
                s.draft = false;
                s.dirty = false;
                s.blank_on_open = false;
            }
            for r in rows.iter().chain([&rows[0], &rows[0]]) {
                rec.borrow_mut().fields = r.clone();
                self.populate_current(st)?;
            }
            let blank = self.initial_fields(table);
            self.set_xrec_fields(st, blank);
            self.page_trigger(st, "onaftergetcurrrecord", vec![])?;
            self.page_event(st, "onaftergetcurrrecordevent", None, None)?;
            self.land_on_new_line(st)?;
            st.borrow_mut().empty_list_row_behind = true;
            return Ok(true);
        }
        if n == 0 {
            // A page opened for view still shows nothing, also when the action stored a row the view hides (pages/W1ShowRow.al:277
            // FilteredList_InsertedRowTheViewHides, a page without row triggers). Which triggers run then is not pinned.
            if shape != EmptyShape::EditList {
                return Ok(false);
            }
            st.borrow_mut().refresh = false;
            st.borrow_mut().save_record_called = false;
            // No row is behind the line any more (RmWorksheetGoneRow.al SH_WsWithNew_EmptyOpen_FillThenDeleteAllUpdate `first` No).
            st.borrow_mut().empty_list_row_behind = false;
            self.land_on_new_line(st)?;
            self.land_on_new_line(st)?;
            return Ok(true);
        }
        st.borrow_mut().refresh = false;
        st.borrow_mut().save_record_called = false;
        let own = self
            .stored_row(&rec)?
            .filter(|f| rec.borrow().filters.iter().all(|x| x.expr.matches(&f[x.field])));
        let fields = match own.clone() {
            Some(f) => f,
            None => {
                let probe = Rc::new(RefCell::new(rec.borrow().clone()));
                self.ret_used = true;
                self.record_op(RecordOp::FindFirst, &probe, &mut [])?;
                let found = probe.borrow();
                found.fields.clone()
            }
        };
        rec.borrow_mut().fields = fields.clone();
        {
            let mut s = st.borrow_mut();
            s.cursor = Cursor::Row;
            s.draft = false;
            s.dirty = false;
            s.blank_on_open = false;
        }
        if shape == EmptyShape::ViewCard {
            self.set_xrec(st);
            self.populate_expressions(st)?;
            return Ok(true);
        }
        let reads = if own.is_some() { 3 } else { 2 };
        for _ in 0..reads {
            self.populate_current(st)?;
        }
        // The client holds the row it stays on: First then reads nothing (:176 `first_log` empty, :364). A list opened for edit that
        // went back to its new-row line reads the row twice at First (:223 `first_log` `List.AGR(NEW2,Created);List.AGR(NEW2,Created);
        // List.AGCR(NEW2,NEW2)`), as when it opens.
        let back_to_new_line = shape == EmptyShape::EditList && own.is_none();
        if !back_to_new_line {
            let key = self.row_key(&rec)?;
            let shown = rec.borrow().fields.clone();
            let mut s = st.borrow_mut();
            s.window = vec![al_builtins::value::WindowRow {
                key,
                stored: fields,
                shown,
            }];
            s.window_from_start = true;
            s.settled_first_row = true;
        }
        if !(shape == EmptyShape::EditList && own.is_some()) {
            let blank = self.initial_fields(table);
            self.set_xrec_fields(st, blank);
        }
        self.page_trigger(st, "onaftergetcurrrecord", vec![])?;
        self.page_event(st, "onaftergetcurrrecordevent", None, None)?;
        if back_to_new_line {
            self.land_on_new_line(st)?;
            st.borrow_mut().empty_list_row_behind = true;
        }
        Ok(true)
    }

    /// First on a list that went back to its new-row line with one row behind it (see `pageops.rs`): the row is read twice, then
    /// OnAfterGetCurrRecord with xRec on it.
    pub(super) fn first_after_empty_list_action(&mut self, st: &St) -> Result<Value, Signal> {
        let rec = self.page_rec(st);
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        self.ret_used = true;
        if !matches!(
            self.record_op(RecordOp::FindFirst, &probe, &mut [])?,
            Value::Boolean(true)
        ) {
            return unsupported("First after the row behind the new-row line of an empty list is gone (not pinned)");
        }
        let fields = probe.borrow().fields.clone();
        rec.borrow_mut().fields = fields.clone();
        {
            let mut s = st.borrow_mut();
            s.cursor = Cursor::Row;
            s.draft = false;
            s.dirty = false;
        }
        // The other rows of the view, which the client read after the action (`empty_page_after_action` with two rows or more).
        let mut rest = Vec::new();
        loop {
            self.ret_used = true;
            if !matches!(self.record_op(RecordOp::Next, &probe, &mut [Value::Integer(1)])?, Value::Integer(n) if n != 0)
            {
                break;
            }
            let f = probe.borrow().fields.clone();
            rest.push(al_builtins::value::WindowRow {
                key: self.table(&probe)?.primary_key(&f),
                stored: f.clone(),
                shown: f,
            });
            if rest.len() >= Self::WINDOW {
                return unsupported(
                    "First after an action stored more rows than a window behind the new-row line of an empty list (not pinned)",
                );
            }
        }
        self.populate_current(st)?;
        self.populate_current(st)?;
        let key = self.row_key(&rec)?;
        {
            let shown = rec.borrow().fields.clone();
            let mut s = st.borrow_mut();
            s.window = vec![al_builtins::value::WindowRow {
                key,
                stored: fields,
                shown,
            }];
            s.window_from_start = true;
            // With more rows the client holds them all and walks them as on a list it opened, then onto its new-row line
            // (RmWorksheetGoneRow.al SH_WsWithNew_EmptyOpen_FillThenWalk `walk log = R:10000;R:10000;C:10000;R:20000;R:20000;
            // C:20000;N:0;C:0;`; registered assumption `empty-worksheet-filled-by-action`).
            if !rest.is_empty() {
                s.window.extend(rest);
                s.view_read_to_end = true;
            }
        }
        self.page_trigger(st, "onaftergetcurrrecord", vec![])?;
        self.page_event(st, "onaftergetcurrrecordevent", None, None)?;
        Ok(Value::Boolean(true))
    }

    /// The new row of an open card over a table, not a part, without DelayedInsert, extensions or a view of its own: the shape
    /// pages/lt-table-globals-page/TableGlobalsPageWithoutValidation.al pins for when an action stores it.
    fn plain_card_draft(&self, st: &St) -> Result<bool, Signal> {
        if !st.borrow().open || !st.borrow().draft || st.borrow().host.is_some() || !st.borrow().parts.is_empty() {
            return Ok(false);
        }
        if !self.has_source(st) || !self.is_card(st) || self.is_request_page(st) {
            return Ok(false);
        }
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let delayed =
            effective_prop(&engine, object, "DelayedInsert")?.is_some_and(|v| v.trim().eq_ignore_ascii_case("true"));
        let extended = engine.program.objects[object]
            .page
            .as_ref()
            .is_some_and(|p| !p.extensions.is_empty());
        // A filtered view stamps the new row (misc/W2TableGlobals.al FilterStamp_*): not this shape.
        let own_props_pinned = ["SourceTableView", "SourceTableTemporary", "Editable", "InsertAllowed"]
            .iter()
            .all(|p| effective_prop(&engine, object, p).ok().flatten().is_none());
        Ok(!delayed && !extended && own_props_pinned)
    }

    /// Whether the primary key of the page's new row differs from a new record's.
    fn draft_key_changed(&mut self, st: &St) -> Result<bool, Signal> {
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        let fresh = self.initial_fields(table.object as usize);
        let now = rec.borrow().fields.clone();
        Ok(table.keys[0]
            .iter()
            .any(|&k| !Self::same_values(std::slice::from_ref(&now[k]), std::slice::from_ref(&fresh[k]))))
    }

    /// The reads of an action that did not call `CurrPage.Update`, before the refresh's own (see
    /// `action_invoke`).
    pub(super) fn read_page_again(&mut self, st: &St) -> Result<(), Signal> {
        if !st.borrow().open || st.borrow().cursor != Cursor::Row || st.borrow().draft {
            return Ok(());
        }
        let rec = self.page_rec(st);
        if self.stored_row(&rec)?.is_none() {
            return Ok(());
        }
        if !self.is_list(st) {
            return self.populate_current(st);
        }
        let key = self.row_key(&rec)?;
        // A list part with one row, without OnAfterGetCurrRecord: the row is read once more before the refresh's own read (misc/
        // lt-probes-needed-7/PnGNewRecord.al:PartPendingNewRowSavedBeforeAction: `Part.AGR` twice after the action). More rows are not pinned.
        if st.borrow().host.is_some() && !self.declares(st, "onaftergetcurrrecord") {
            let alone = {
                self.ret_used = true;
                let after = matches!(self.record_op(RecordOp::Next, &rec, &mut [Value::Integer(1)])?, Value::Integer(n) if n != 0);
                let mut pk = key.clone();
                self.ret_used = true;
                self.record_op(RecordOp::Get, &rec, &mut pk)?;
                self.ret_used = true;
                let before = matches!(self.record_op(RecordOp::Next, &rec, &mut [Value::Integer(-1)])?, Value::Integer(n) if n != 0);
                let mut pk = key.clone();
                self.ret_used = true;
                self.record_op(RecordOp::Get, &rec, &mut pk)?;
                !after && !before
            };
            if alone {
                return self.populate_current(st);
            }
            return Ok(());
        }
        if self.declares(st, "onaftergetcurrrecord") && self.declares(st, "onaftergetrecord") {
            return self.read_list_again_from_row(st, &rec, &key);
        }
        let first = {
            let s = st.borrow();
            s.window_from_start
                && s.window.len() < Self::WINDOW
                && s.window.first().is_some_and(|w| Self::same_key(&w.key, &key))
        };
        if !first {
            return self.read_list_again_from_row(st, &rec, &key);
        }
        st.borrow_mut().landing_first = true;
        let r = self.populate_window(st);
        st.borrow_mut().landing_first = false;
        r?;
        st.borrow_mut().window_from_start = true;
        Ok(())
    }

    /// The row a list settles on when the action's filter left the stored row it stood on outside the view: the next row of the
    /// view, else the one before it (pages/lt-ai-list-order/AiListOrderTests.al:335 DropCurrent_RowsAfter_Plain and :301 _FindOnly
    /// read R1003 first, :501 DropCurrent_NoRowsAfter_Plain and :467 _FindOnly read R1005 first). `None` when the row is in the
    /// view; a view left with no row is refused.
    fn row_after_view_left(&mut self, rec: &Rec) -> Result<Option<Vec<Value>>, Signal> {
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        self.ret_used = true;
        if matches!(
            self.record_op(RecordOp::Find, &probe, &mut [Value::Text("=".into())])?,
            Value::Boolean(true)
        ) {
            return Ok(None);
        }
        for step in [1, -1] {
            let probe = Rc::new(RefCell::new(rec.borrow().clone()));
            self.ret_used = true;
            if matches!(self.record_op(RecordOp::Next, &probe, &mut [Value::Integer(step)])?, Value::Integer(n) if n != 0)
            {
                return Ok(Some(probe.borrow().fields.clone()));
            }
        }
        unsupported("an action left the list's view without rows (not pinned)")
    }

    /// `CurrPage.Update` in a control's OnValidate that changed the filters of the page's record, on a list (a Worksheet or a
    /// List, no part) that reads its rows from the table and declares OnAfterGetRecord and OnAfterGetCurrRecord: the client reads
    /// the rows as after an action, without the read of the row before them (`read_list_again_from_row`). It stands on the row,
    /// else on the next row of the filtered view after the old key, else on the one before it; a view left without rows shows a
    /// blank row, and from the blank row a view with rows again opens on its first row (pages/lt-rz-outside-page-moves/
    /// RzUpdateFromValidate.al:27-99 the Worksheet, Update(false) and Update(true), :126 and :151 the filter removed again,
    /// RZ_PlainList_* the List: `R:C;R:E;R:C;R:A;R:C;R:C;C:C`). `false` for every other page or call: the refresh runs as before.
    fn refresh_after_validate_filter(
        &mut self,
        st: &St,
        filters_before: Option<Vec<al_builtins::filter::FieldFilter>>,
        without_curr: bool,
        without_parts: bool,
    ) -> Result<bool, Signal> {
        let Some(before) = filters_before else {
            return Ok(false);
        };
        let shape = {
            let s = st.borrow();
            s.open && !s.dirty && !s.draft && s.host.is_none() && !s.new_line
        };
        if !shape
            || !self.has_source(st)
            || !self.is_list(st)
            || !self.declares(st, "onaftergetrecord")
            || !self.declares(st, "onaftergetcurrrecord")
        {
            return Ok(false);
        }
        let rec = self.page_rec(st);
        let now = rec.borrow().filters.clone();
        let same = before.len() == now.len()
            && before.iter().zip(&now).all(|(a, b)| {
                a.field == b.field
                    && a.group == b.group
                    && matches!((self.filter_text(&a.expr), self.filter_text(&b.expr)), (Ok(x), Ok(y)) if x == y)
            });
        if same {
            return Ok(false);
        }
        match (self.declares(st, "onfindrecord"), self.declares(st, "onnextrecord")) {
            (true, true) if st.borrow().cursor == Cursor::Row && !st.borrow().view_left => {
                self.refresh_list_with_find_as(st, true)?;
                return Ok(true);
            }
            (false, false) => {}
            _ => return Ok(false),
        }
        let on_row = st.borrow().cursor == Cursor::Row;
        if on_row && self.stored_row(&rec)?.is_none() {
            return Ok(false);
        }
        if !on_row && st.borrow().cursor != Cursor::None {
            return Ok(false);
        }
        let curr = |this: &mut Self| -> Result<(), Signal> {
            let r = (|| {
                if without_curr {
                    return Ok(true);
                }
                this.page_trigger(st, "onaftergetcurrrecord", vec![])?;
                this.page_event(st, "onaftergetcurrrecordevent", None, None)
            })();
            this.torn_down_on_error(st, r)?;
            st.borrow_mut().refresh = false;
            if !without_parts {
                this.sync_parts(st)?;
            }
            Ok(())
        };
        if self.view_row_count(st)? == 0 {
            if !on_row {
                return Ok(false);
            }
            // No row left: a blank row, its key from a single-valued filter, and OnAfterGetCurrRecord on it (RZ_PlainWs_B_Empty
            // `A:filter B;A:done B;C:`).
            {
                let mut s = st.borrow_mut();
                s.cursor = Cursor::None;
                s.window = Vec::new();
                s.focus = None;
                s.focus_in_row = false;
            }
            let table = rec.borrow().table as usize;
            let fresh = self.initial_fields(table);
            rec.borrow_mut().fields = fresh;
            self.stamp_from_filters(st, false)?;
            curr(self)?;
            return Ok(true);
        }
        if !on_row {
            // From the blank row: the first row of the view (RZ_PlainWs_B_Empty_ThenCleared `R:A;...;R:F;R:A;R:A;C:A`).
            self.ret_used = true;
            if !matches!(
                self.record_op(RecordOp::Find, &rec, &mut [Value::Text("-".into())])?,
                Value::Boolean(true)
            ) {
                return Ok(false);
            }
            let mut s = st.borrow_mut();
            s.cursor = Cursor::Row;
            s.window_from_start = true;
        }
        // The client reads the new view from its first row: the rows it held do not matter.
        st.borrow_mut().window_from_start = true;
        let key = self.row_key(&rec)?;
        self.read_list_again_from_row_as(st, &rec, &key, false)?;
        let mut pk = self.table(&rec)?.primary_key(&rec.borrow().fields);
        self.ret_used = true;
        self.record_op(RecordOp::Get, &rec, &mut pk)?;
        self.populate_current(st)?;
        curr(self)?;
        // The client holds the rows the refresh read (`read_list_again_from_row_as`).
        st.borrow_mut().window_top_unread = false;
        Ok(true)
    }

    /// The reads of an action on a list whose cursor is not on the first row: the row again, the rows from it on (up to 20),
    /// the row again when the view goes on past it, the rows back to the first one, and the row twice more before the refresh's
    /// own read. Pinned by misc/lt-probes-needed-6/PnF05ActionReads.al:Action_List_LaterRow (row 2 and 3 of 3 rows: 7 and 6
    /// reads after the action) and :Action_List_ManyRows (row 6 of 25: 29 reads), and, with the row identities, by
    /// misc/lt-probes-needed-7/PnGNewRecord.al:ListPendingNewRowSavedBeforeAction (last row of three: `P1;P1;N2;N1;P1;P1;C`)
    /// and misc/lt-probes-needed-7/PnGListUpdate.al:ListModifyWithAndWithoutUpdate (`U2;U2;U3;U2;U1;U2;U2;C`). Only a page that declares both row triggers and serves its rows from the table is
    /// pinned; the identities of the rows beyond the window held are the table order.
    fn read_list_again_from_row(&mut self, st: &St, rec: &Rec, key: &[Value]) -> Result<(), Signal> {
        self.read_list_again_from_row_as(st, rec, key, true)
    }

    /// [`Self::read_list_again_from_row`]; `read_first` false leaves out the read of the row before the others (a refresh after a
    /// control's OnValidate, `refresh_after_validate_filter`).
    fn read_list_again_from_row_as(
        &mut self,
        st: &St,
        rec: &Rec,
        key: &[Value],
        read_first: bool,
    ) -> Result<(), Signal> {
        let ok = self.declares(st, "onaftergetrecord") && st.borrow().window_from_start;
        if !ok {
            // Other shapes are not pinned: the refresh alone stays (docs/probes-needed.md).
            return Ok(());
        }
        let find = self.declares(st, "onfindrecord");
        let with_current = self.declares(st, "onaftergetcurrrecord");
        let mut pk = key.to_vec();
        // A list that reads each row once more in its key (`reads_once_more_in_its_key`) reads the row it starts from once more
        // first, and once more before the refresh's own read (pages/lt-sg-list-key-moves/SgListKeyMovesTests.al SG_Key_FourRows_Moves
        // `update` `R:C;U;R:C;R:C;R:C;R:B;R:A;R:C;R:D;R:C;R:C;R:C;C:C;` against SG_Pk_FourRows_Moves, and the same in
        // SG_Key_OneRow_Open and SG_Key_ThirtyRows_Moves; corpus bump PR 738).
        let once_more = self.still_reads_once_more(st);
        if once_more && !read_first {
            // After a control's OnValidate the first read is the row the page stood on, also when the new filter excludes it
            // (pages/lt-rz-outside-page-moves/RzUpdateFromValidate.al:496 SF_Key_FilterDropsRow_Next_Clear_GoToRecord, :476
            // SF_Key_FilterKeepsRow_Next; al-runner issue 5888). Pinned on a Worksheet only: on another page type it is refused.
            if !self.is_worksheet(st) {
                return unsupported(
                    "the refresh of a list in another key after a control's OnValidate changed its filters (pinned on a Worksheet only)",
                );
            }
            self.populate_current(st)?;
        }
        let view_left = match self.row_after_view_left(rec)? {
            Some(fields) => {
                rec.borrow_mut().fields = fields;
                pk = self.row_key(rec)?;
                true
            }
            None => false,
        };
        let mut rows = Vec::new();
        let save_after = std::mem::take(&mut st.borrow_mut().save_after_first_read);
        let read_xrec = save_after.then(|| self.page_xrec_fields(st));
        if read_first {
            self.populate_current(st)?;
        }
        if let Some(x) = read_xrec {
            self.set_xrec_fields(st, x);
            st.borrow_mut().dirty = true;
            self.save_row(st)?;
            self.set_xrec(st);
            // The next OnAfterGetCurrRecord sees the saved row as xRec (PnGRefresh.al:503 `P.AGCR(A,own changed x=A,own changed)`).
            let saved = self.page_rec(st).borrow().fields.clone();
            st.borrow_mut().agcr_xrec = Some(saved);
        }
        // An action left another row in Rec: that first read sees Rec as the action left it, the rows after it are read from the
        // table (misc/lt-probes-needed-7/PnGRefresh.al:387 `P.AGR(B,changed B);P.AGR(B,orig B)`).
        if std::mem::take(&mut st.borrow_mut().rec_moved_other) {
            let mut at = pk.clone();
            self.ret_used = true;
            self.record_op(RecordOp::Get, rec, &mut at)?;
        }
        if find {
            // The forward reads start with `=>` (pages/lt-update-on-find/UpdateOnFindRecord.al:P168_List_UpdateFalse).
            self.nav_find(st, "=>")?;
        }
        let row = |this: &mut Self| -> Result<al_builtins::value::WindowRow, Signal> {
            let key = this.row_key(rec)?;
            let stored = rec.borrow().fields.clone();
            this.populate_current(st)?;
            Ok(al_builtins::value::WindowRow {
                key,
                stored,
                shown: rec.borrow().fields.clone(),
            })
        };
        rows.push(row(self)?);
        if once_more && read_first {
            self.populate_current(st)?;
        }
        let mut moved = false;
        for _ in 1..Self::WINDOW {
            if !self.nav_next(st, 1)? {
                break;
            }
            moved = true;
            rows.push(row(self)?);
        }
        if moved {
            self.ret_used = true;
            self.record_op(RecordOp::Get, rec, &mut pk)?;
        }
        // Is there a row before it? Asked of the table: the page's own OnNextRecord is asked when the client steps.
        self.ret_used = true;
        let has_before =
            matches!(self.record_op(RecordOp::Next, rec, &mut [Value::Integer(-1)])?, Value::Integer(n) if n != 0);
        self.ret_used = true;
        self.record_op(RecordOp::Get, rec, &mut pk)?;
        let mut more_rows = moved;
        if has_before {
            // Without OnAfterGetCurrRecord the reads end with the way back: the row is not put back (AiListOrderTests.al:233
            // Keep_Plain `...R:R1002;R:R1001`, :501 DropCurrent_NoRowsAfter_Plain `...R:R1002;R:R1001`).
            more_rows = with_current;
            if moved {
                // The client reads the row again when it steps back onto it from the rows past it; the first row
                // has no step back (Action_List_FirstRow reads 6 times after the action, Action_List_LaterRow 7).
                self.populate_current(st)?;
            }
        }
        // The rows read on the way back, nearest first.
        let mut back = Vec::new();
        if has_before || find {
            // The way back is read in blocks of 20 rows: after a block the client reads the row it started from, then goes on from the
            // last row of the block again (pages/lt-testpage-list-rows/ListRowsActionNew.al:NewRowThenAction_Ping30Rows_ReadsLogged:
            // `G100;G30;...;G11;G100;G11;G10;...;G1;G100;G100;C100`).
            let mut block = 0;
            while self.nav_next(st, -1)? {
                let key = self.row_key(rec)?;
                let stored = rec.borrow().fields.clone();
                self.populate_current(st)?;
                back.push(al_builtins::value::WindowRow {
                    key,
                    stored,
                    shown: rec.borrow().fields.clone(),
                });
                block += 1;
                if block == Self::WINDOW && !find {
                    let stop = rec.borrow().fields.clone();
                    let here = self.row_key(rec)?;
                    self.ret_used = true;
                    let before = matches!(
                        self.record_op(RecordOp::Next, rec, &mut [Value::Integer(-1)])?,
                        Value::Integer(n) if n != 0
                    );
                    rec.borrow_mut().fields = stop.clone();
                    if before {
                        let mut at = here.clone();
                        self.ret_used = true;
                        self.record_op(RecordOp::Get, rec, &mut pk)?;
                        self.populate_current(st)?;
                        self.ret_used = true;
                        self.record_op(RecordOp::Get, rec, &mut at)?;
                        self.populate_current(st)?;
                        block = 1;
                    }
                }
            }
        }
        self.ret_used = true;
        self.record_op(RecordOp::Get, rec, &mut pk)?;
        // A view of one row has no step back onto it (a list of one row opens with RRC, not RRRC; Action_List_FewRows).
        if more_rows {
            self.populate_current(st)?;
        }
        if once_more {
            self.populate_current(st)?;
        }
        // Without OnAfterGetCurrRecord the refresh reads nothing more (AiListOrderTests.al:233 Keep_Plain, :335
        // DropCurrent_RowsAfter_Plain).
        if !with_current {
            st.borrow_mut().refresh_read_done = true;
        }
        // The client holds the rows of the new view from the row it settled on (AiListOrderTests.al:336-338, :370-372: the
        // current row is R1003 and First stays on it).
        if view_left && !has_before {
            if rows.len() >= Self::WINDOW {
                return unsupported(
                    "an action left the list's view and the row it settles on has a full window after it (not pinned)",
                );
            }
            rec.borrow_mut().fields = rows[0].stored.clone();
            let mut s = st.borrow_mut();
            s.window = rows;
            s.window_from_start = true;
            s.settled_first_row = true;
        } else if !read_first {
            // After a control's OnValidate the client holds the rows of the new view this refresh read, from the first row of the
            // view to the window after the row it settled on: the next move reads as on a page without a refresh, and a GoToRecord
            // from a later row steps from the first row (pages/lt-rz-outside-page-moves/RzUpdateFromValidate.al:358
            // SF_PlainWs_FilterKeepsRow_Next, :378 SF_PlainWs_FilterDropsRow_Next_Clear_GoToRecord, :406
            // SF_PlainWs_FilterKeepsRow_GoToRecord, :476-544 the same in another key and in the primary key; al-runner issues 5914,
            // 5888). The rows the client held before are those of the old filters (RZ_PlainWs_B_G1 `next` `Yes|E`, `rows`
            // `A,C,E,`), and on a List First and Next list the rows of the new view (RzUpdateFromValidate.al RZ_PlainList_B_G1,
            // RZ_PlainList_F_G1 `rows`). With more rows than a window what the client holds is not pinned: the next move is refused.
            let mut s = st.borrow_mut();
            if back.len() + rows.len() > Self::WINDOW {
                s.window = Vec::new();
                s.window_from_start = false;
                s.validate_refresh_unheld = true;
            } else {
                back.reverse();
                back.extend(rows);
                s.window = back;
                s.window_from_start = true;
            }
        }
        Ok(())
    }

    /// `CurrPage.Update` on a list with OnFindRecord, OnNextRecord and OnAfterGetRecord (no OnAfterGetCurrRecord) whose action just
    /// changed the view so that the stored row the cursor stands on is outside it: the call itself reads the row (pages/lt-f3-pages/
    /// F3ListTests.al:72 Nav_ActionChangesViewThenUpdate `after_update` begins `AGR(R1003) | AGR(R1003) | OnlyBig done`, both reads
    /// before the action: the one after the GoToKey and the one before every action). Remembers it for the refresh.
    fn is_worksheet(&self, st: &St) -> bool {
        self.engine.object_facts[st.borrow().object as usize].page_type == Some(al_ir::PageType::Worksheet)
    }

    pub(super) fn note_view_left(&mut self, st: &St) -> Result<(), Signal> {
        if !self.is_list(st)
            || st.borrow().host.is_some()
            || st.borrow().cursor != Cursor::Row
            || st.borrow().draft
            || !(self.declares(st, "onfindrecord")
                && self.declares(st, "onnextrecord")
                && self.declares(st, "onaftergetrecord"))
            || self.declares(st, "onaftergetcurrrecord")
        {
            return Ok(());
        }
        let rec = self.page_rec(st);
        if self.stored_row(&rec)?.is_none() {
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
        // The call reads nothing itself: the second read before `OnlyBig done` is the one after the GoToKey (no read between
        // `A:filter` and `A:done` in pages/lt-ai-list-order/AiListOrderTests.al:403 DropCurrent_NoRowsAfter_Nav).
        st.borrow_mut().view_left = true;
        Ok(())
    }

    /// The re-read `CurrPage.Update` asked for, once the call that made it has returned: the
    /// stored current row loads again and raises OnAfterGetRecord and OnAfterGetCurrRecord; a new
    /// row nobody has saved has nothing to re-read (corpus pageupdate/TestPageUpdateNewRow.al:1-30,
    /// TestPageUpdateGoneRow.al StoredRow_OnValidateUpdateFalse_Trace). The row is read from the
    /// table unless the client holds edits of it that are not saved yet.
    /// The end of an action that saved a SetValue of a Document page before its trigger: without a CurrPage.Update in the
    /// trigger the part's CaptionClass expressions did not run again, and whether BC runs them then is not pinned
    /// (page-open-caption-class-expressions).
    fn action_save_captions_done(&mut self, st: &St) {
        if std::mem::take(&mut st.borrow_mut().field_captions_action_save) == 2 {
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::PageOpenCaptionClassExpressions);
        }
    }

    pub(super) fn run_refresh(&mut self, st: &St) -> Result<(), Signal> {
        let without_curr = std::mem::take(&mut st.borrow_mut().refresh_without_curr);
        let without_parts = std::mem::take(&mut st.borrow_mut().refresh_without_parts);
        let read_done = std::mem::take(&mut st.borrow_mut().refresh_read_done);
        let by_update = std::mem::take(&mut st.borrow_mut().refresh_by_update);
        let filters_before = std::mem::take(&mut st.borrow_mut().filters_before_validate);
        if !std::mem::take(&mut st.borrow_mut().refresh) || read_done {
            return Ok(());
        }
        if by_update && self.refresh_after_validate_filter(st, filters_before, without_curr, without_parts)? {
            return Ok(());
        }
        // A page that shows no stored row (an empty view, or a list on its new line) whose view now has rows, for example
        // after the page changed its filters: which row the client lands on and which row triggers run is not pinned
        // (pageupdate/TestPageUpdateNewRow.al pins only a card's unsaved new row). Tests-ERM 134022
        // PurchasePaymentAppliedWithToleranceAmount reaches it when General Journal (page 39), opened for view on an empty
        // batch, switches to the batch that holds the test's line.
        if by_update
            && st.borrow().open
            && st.borrow().cursor != Cursor::Row
            && !st.borrow().dirty
            && self.has_source(st)
            && !self.declares(st, "onfindrecord")
            && !self.declares(st, "onnextrecord")
            && self.view_row_count(st)? > 0
        {
            return unsupported(
                "CurrPage.Update on a page that shows no row (empty view or new line) while its view has rows (which row it shows then is not pinned)",
            );
        }
        if !st.borrow().open || !self.has_source(st) || st.borrow().cursor != Cursor::Row || st.borrow().draft {
            return Ok(());
        }
        // A page that serves its rows from OnFindRecord asks the trigger to settle the client on a row
        // again (corpus pageupdate/TestPageUpdateGoneRow.al:390 pins the Which strings of a deleted
        // row only). Which row that is, is not modelled: nothing that shows it may be read before the
        // cursor moves, and the page's row triggers must not matter.
        if self.declares(st, "onfindrecord") || self.declares(st, "onnextrecord") {
            if self.declares(st, "onaftergetrecord") || self.declares(st, "onaftergetcurrrecord") {
                // The client asks OnFindRecord for the current row (`=`), then reads it as it does after
                // any move. Assumed (update-refresh-onfindrecord): Base Application's document pages do this
                // under Microsoft's tests; a page that holds edits or has only OnNextRecord stays refused.
                // A list refills its window around the row (misc/W2FindRecordUpdate.al:Update_True_OnOpenedPage pins that
                // trace); only a card, which has the one row, is served.
                if self.is_list(st) && self.declares(st, "onfindrecord") {
                    return self.refresh_list_with_find(st);
                }
                if !self.declares(st, "onfindrecord") || st.borrow().dirty || self.is_list(st) {
                    let why = if !self.declares(st, "onfindrecord") {
                        "only OnNextRecord"
                    } else if st.borrow().dirty {
                        "unsaved edits"
                    } else {
                        "a list"
                    };
                    return unsupported(format!(
                        "CurrPage.Update on a page with OnFindRecord and row triggers (re-anchoring not pinned; {why})"
                    ));
                }
                if !st.borrow().refresh_pinned {
                    al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::UpdateRefreshOnFindRecord);
                }
                if !self.nav_find(st, "=")? {
                    return unsupported(
                        "OnFindRecord does not find the current row after CurrPage.Update (not pinned)",
                    );
                }
                let r = (|| {
                    self.populate_current(st)?;
                    self.page_trigger(st, "onaftergetcurrrecord", vec![])?;
                    self.page_event(st, "onaftergetcurrrecordevent", None, None)
                })();
                self.torn_down_on_error(st, r)?;
                st.borrow_mut().refresh = false;
                return self.sync_parts(st);
            }
            // The rows the client held are the old ones: it asks the triggers again for whatever it does next.
            let mut s = st.borrow_mut();
            s.stale = true;
            s.window.clear();
            s.window_from_start = false;
            // The rows above the open read went with the window: the next move asks the triggers again, as after any Update
            // (pagefindrecord/TestPageFindRecord.al:113 OnFindRecord_FirstAnswersTheFirstRowOfTheTriggerRowset, a descending find
            // list). Left set, First indexed the empty window (al-runner issue 5802).
            s.window_top_unread = false;
            return Ok(());
        }
        let rec = self.page_rec(st);
        if self.stored_row(&rec)?.is_none() {
            return Ok(());
        }
        // A list whose filters no longer hold the row it stands on, when CurrPage.Update asked for the refresh: which row it
        // settles on, and what a move reads then, is pinned only after an action (`read_list_again_from_row`,
        // pages/lt-ai-list-order/AiListOrderTests.al:335 DropCurrent_RowsAfter_Plain). Tests-SCM 137831
        // BinContentsPage_CanSwitchLocationFilterWithoutRenamingRecord reaches it: Bin Contents (page 7374) filters to a location
        // without rows in a control's OnValidate.
        // On a page without the row triggers, the list settles on the next row of the view after the old key, else the one
        // before it, as after an action (pages/lt-rz-outside-page-moves/RzUpdateFromValidate.al:9-12: the same row on the Worksheet
        // and the List, with and without OnFindRecord and OnNextRecord; without the row triggers nothing else is seen). The pages
        // with the row triggers go through `refresh_after_validate_filter`; there, or with a view left without rows, it stays refused.
        if by_update && self.is_list(st) && !st.borrow().dirty {
            let probe = Rc::new(RefCell::new(rec.borrow().clone()));
            self.ret_used = true;
            if !matches!(
                self.record_op(RecordOp::Find, &probe, &mut [Value::Text("=".into())])?,
                Value::Boolean(true)
            ) {
                let triggers = [
                    "onfindrecord",
                    "onnextrecord",
                    "onaftergetrecord",
                    "onaftergetcurrrecord",
                ]
                .iter()
                .any(|t| self.declares(st, t));
                let mut settled = None;
                if !triggers {
                    for step in [1, -1] {
                        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
                        self.ret_used = true;
                        if matches!(self.record_op(RecordOp::Next, &probe, &mut [Value::Integer(step)])?, Value::Integer(n) if n != 0)
                        {
                            settled = Some(probe.borrow().fields.clone());
                            break;
                        }
                    }
                }
                let Some(fields) = settled else {
                    return unsupported(
                        "CurrPage.Update on a list whose filters left the row it stands on, outside an action (which row it shows then is not pinned)",
                    );
                };
                rec.borrow_mut().fields = fields;
            }
        }
        if !st.borrow().dirty {
            self.ret_used = true;
            let mut pk = self.table(&rec)?.primary_key(&rec.borrow().fields);
            self.record_op(RecordOp::Get, &rec, &mut pk)?;
        }
        let r = (|| {
            self.populate_current(st)?;
            if without_curr {
                return Ok(true);
            }
            self.page_trigger(st, "onaftergetcurrrecord", vec![])?;
            self.page_event(st, "onaftergetcurrrecordevent", None, None)
        })();
        self.torn_down_on_error(st, r)?;
        st.borrow_mut().refresh = false;
        // The parts read their rows again after the host's own triggers (corpus
        // testpart/TestPartTriggerOrderAndRefresh.al:HostCurrPageUpdate_ReReadsThePart:
        // "host.Action;host.AGCR(2);part.AGCR(2);", and no part OnOpenPage).
        if !without_parts {
            self.sync_parts(st)?;
        }
        Ok(())
    }

    /// `CurrPage.Update` on a list that serves its rows from OnFindRecord, with or without OnNextRecord (corpus misc/W2FindRecordUpdate.al:
    /// Update_False_OnOpenedPage, Update_True_OnOpenedPage, Update_StoredRowChangedBehindThePage, Update_True_AfterRecChangedInMemory,
    /// all one trace for a table of 3 rows with the cursor on the middle one): the row is read again, `=>` finds the row to
    /// start from, the client reads on to the end of the view, puts the first of those rows back, steps back to the start of
    /// the view reading each row it moves to, puts the row back, asks `=` for it and reads it as after a move. Only that shape
    /// is pinned, with both triggers; the same steps are taken for other shapes, among them a list with OnFindRecord alone, where
    /// `Next` moves the record (assumption update-refresh-list-onfindrecord), and a view that fills
    /// a window stays refused.
    fn refresh_list_with_find(&mut self, st: &St) -> Result<(), Signal> {
        self.refresh_list_with_find_as(st, false)
    }

    /// [`Self::refresh_list_with_find`]; `after_validate`: the refresh after a control's OnValidate changed the filters, on a list
    /// with OnFindRecord, OnNextRecord and both row triggers. Pinned there (pages/lt-rz-outside-page-moves/RzUpdateFromValidate.al
    /// RZ_LoggedWs_B_G1 `A:done B;F:=>=Yes;R:C;N:1=1;R:E;N:1=0;R:C;N:-1=-1;R:A;N:-1=0;R:C;F:==Yes;R:C;C:C`, RZ_LoggedWs_A_G2, the
    /// Update(true) case): no read of the row first, and `=><` when `=>` finds nothing (RZ_LoggedWs_F_G1 `F:=>=No;F:=><=Yes;R:E`).
    fn refresh_list_with_find_as(&mut self, st: &St, after_validate: bool) -> Result<(), Signal> {
        if !after_validate {
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::UpdateRefreshListOnFindRecord);
        }
        let rec = self.page_rec(st);
        let view_left = std::mem::take(&mut st.borrow_mut().view_left);
        let with_current = self.declares(st, "onaftergetcurrrecord");
        if view_left {
            // The row the cursor stands on is stored but the view no longer holds it: the client asks OnNextRecord(1) for the
            // row to settle on and reads it, where it would read the row again (pages/lt-f3-pages/F3ListTests.al:72
            // Nav_ActionChangesViewThenUpdate `after_update`: `Next(1)=1 | AGR(R1008) | Find(=>)=Yes`). Pinned only when that step
            // lands on the first row of the view (nothing before it), and for a list without OnAfterGetCurrRecord.
            if with_current || !self.declares(st, "onnextrecord") {
                return unsupported(
                    "CurrPage.Update after the view left the current row, on a list with OnAfterGetCurrRecord or without OnNextRecord (not pinned)",
                );
            }
            if !self.nav_next(st, 1)? {
                return unsupported(
                    "CurrPage.Update after the view left the current row, with no row after it (not pinned)",
                );
            }
            self.populate_current(st)?;
        } else {
            // A list with OnFindRecord alone does not tell the view left the row (`note_view_left`): the client reads the row it
            // settles on (AiListOrderTests.al:301 DropCurrent_RowsAfter_FindOnly `A:done;R:R1003;F:=>=Yes`).
            if !self.declares(st, "onnextrecord") {
                if let Some(fields) = self.row_after_view_left(&rec)? {
                    rec.borrow_mut().fields = fields;
                }
            }
            if !after_validate {
                self.page_trigger(st, "onaftergetrecord", vec![])?;
            }
        }
        if !self.nav_find(st, "=>")? && !(after_validate && self.nav_find(st, "=><")?) {
            return unsupported("OnFindRecord finds no row after CurrPage.Update on a list (not pinned)");
        }
        let mut forward = Vec::new();
        loop {
            if forward.len() + 1 >= Self::WINDOW {
                return unsupported("CurrPage.Update on a list whose view fills a window (not pinned)");
            }
            let key = self.row_key(&rec)?;
            let stored = rec.borrow().fields.clone();
            self.populate_current(st)?;
            forward.push(al_builtins::value::WindowRow {
                key,
                stored,
                shown: rec.borrow().fields.clone(),
            });
            if !self.nav_next(st, 1)? {
                break;
            }
        }
        let first = forward[0].clone();
        rec.borrow_mut().fields = first.stored.clone();
        // On the last row the client has not moved, so it puts nothing back (pages/lt-update-on-find/UpdateOnFindRecord.al:
        // P168_List_UpdateOnLastRow: `Next:1;Next:-1;AGR:2`).
        if forward.len() > 1 {
            self.page_trigger(st, "onaftergetrecord", vec![])?;
        }
        let mut before = Vec::new();
        while self.nav_next(st, -1)? {
            if before.len() + forward.len() + 1 >= Self::WINDOW {
                return unsupported("CurrPage.Update on a list whose view fills a window (not pinned)");
            }
            let key = self.row_key(&rec)?;
            let stored = rec.borrow().fields.clone();
            self.populate_current(st)?;
            before.push(al_builtins::value::WindowRow {
                key,
                stored,
                shown: rec.borrow().fields.clone(),
            });
        }
        before.reverse();
        if view_left {
            // No OnAfterGetCurrRecord: the refresh ends after the step back (F3ListTests.al:72 `...Next(1)=0 | AGR(R1008) | Next(-1)=0`).
            if !before.is_empty() {
                return unsupported(
                    "CurrPage.Update after the view left the current row, with rows before the first one read (not pinned)",
                );
            }
            st.borrow_mut().window = forward;
            st.borrow_mut().window_from_start = true;
            st.borrow_mut().window_eof = true;
            st.borrow_mut().refresh = false;
            // F3ListTests.al:72 `after_walk` empty: a First right after this reads nothing (see `rows.rs`).
            st.borrow_mut().settled_first_row = true;
            return self.sync_parts(st);
        }
        rec.borrow_mut().fields = first.stored.clone();
        if !with_current {
            // Without OnAfterGetCurrRecord the refresh ends after the step back: no row is put back and `=` is not asked
            // (pages/lt-ai-list-order/AiListOrderTests.al:39 Keep_Nav, :73 Same_Nav, :170 Same_Nav_OnOpenRow).
            // AiListOrderTests.al:174 Same_Nav_OnOpenRow: a First right after this on the first row reads nothing (see `rows.rs`).
            st.borrow_mut().settled_first_row = before.is_empty() && self.declares(st, "onnextrecord");
            before.extend(forward);
            st.borrow_mut().window = before;
            st.borrow_mut().window_from_start = true;
            st.borrow_mut().refresh = false;
            return self.sync_parts(st);
        }
        // The client puts its row back only when the step back left it (AiListOrderTests.al:107 Keep_NavCurr
        // `...R:R1001;N:-1=0;R:R1002;F:==Yes`); on the first row of the view it asks `=` at once (:199
        // Same_NavCurr_OnOpenRow `...R:R1001;N:-1=0;F:==Yes;R:R1001;C:R1001`).
        if !before.is_empty() {
            self.page_trigger(st, "onaftergetrecord", vec![])?;
        }
        if !self.nav_find(st, "=")? {
            return unsupported("OnFindRecord does not find the current row after CurrPage.Update (not pinned)");
        }
        let r = (|| {
            self.populate_current(st)?;
            self.page_trigger(st, "onaftergetcurrrecord", vec![])?;
            self.page_event(st, "onaftergetcurrrecordevent", None, None)
        })();
        self.torn_down_on_error(st, r)?;
        // AiListOrderTests.al:203 Same_NavCurr_OnOpenRow: a First right after this on the first row reads nothing (see `rows.rs`).
        st.borrow_mut().settled_first_row = before.is_empty() && self.declares(st, "onnextrecord");
        before.extend(forward);
        st.borrow_mut().window = before;
        st.borrow_mut().window_from_start = true;
        st.borrow_mut().refresh = false;
        self.sync_parts(st)
    }

    /// `CurrPage.Close()` in an action of a list that put another row (another primary key) into Rec: the row the list showed is
    /// read back into Rec, with OnAfterGetRecord (misc/lt-probes-needed-7/PnGRefresh.al:608 QL_RecAssign_OtherChanged_CloseInAction
    /// `P.AGR(A,orig A);ACT.end(A,orig A x=A,orig A)`: the action's code after the call sees the shown row). Rec still on the shown
    /// row is left as it is.
    ///
    /// With OnQueryClosePage or OnClosePage the read back comes before them, so both see the shown row as Rec and xRec; a change
    /// the action made to its own row is dropped without a read (no OnModifyRecord, nothing stored), and the client reads the row
    /// once more after the action, also when Rec was not touched (PnGRefresh.al:865 QN_CloseQ_Sheet_OtherChanged_CloseInAction
    /// `P.AGR(A,orig A);P.AGR(A,orig A);P.QCP(A,orig A x=A,orig A a=OK);P.CP(A,orig A x=A,orig A);ACT.end(A,orig A x=A,orig A);
    /// P.AGR(A,orig A)`, :887 _OtherUnchanged, :909 _TempChanged from a temporary record, :931 _OwnRowChanged and :953 _NoChange
    /// `P.AGR(A,orig A);P.QCP(...)...;P.AGR(A,orig A)`, :996 under Page.RunModal `a=LookupCancel`; the List page :1026-1113).
    pub(super) fn restore_shown_row_on_close(&mut self, st: &St) -> Result<(), Signal> {
        let Some(shown) = st.borrow().action_shown_row.clone() else {
            return Ok(());
        };
        if !st.borrow().open || st.borrow().cursor != Cursor::Row || st.borrow().draft {
            return Ok(());
        }
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        let mut key = table.primary_key(&shown);
        let shape = self.declares(st, "onaftergetrecord")
            && self.declares(st, "onaftergetcurrrecord")
            && !self.declares(st, "onfindrecord")
            && !self.declares(st, "onnextrecord");
        let close_triggers = self.declares(st, "onclosepage") || self.declares(st, "onqueryclosepage");
        if Self::same_key(&table.primary_key(&rec.borrow().fields), &key) {
            if shape && close_triggers && self.stored_row(&rec)?.is_some() {
                rec.borrow_mut().fields = shown;
                st.borrow_mut().close_restored_row = true;
            }
            return Ok(());
        }
        let pinned = shape;
        if !pinned {
            return unsupported(
                "CurrPage.Close() in an action that left another row in Rec, on this page (not pinned)",
            );
        }
        self.ret_used = true;
        self.record_op(RecordOp::Get, &rec, &mut key)?;
        self.populate_current(st)?;
        st.borrow_mut().close_restored_row = true;
        Ok(())
    }

    /// What the page does once an action returns: a page that showed no row may show the one the
    /// action inserted; a row the action deleted closes a card, and moves a list to the next row,
    /// else the previous one, else none (corpus pageupdate/TestPageUpdateGoneRow.al:1-30).
    pub(super) fn settle_after_action(&mut self, st: &St) -> Result<(), Signal> {
        if !self.has_source(st) {
            return Ok(());
        }
        if st.borrow().gone_view_list && st.borrow().open && st.borrow().cursor == Cursor::None {
            return unsupported(
                "a second action on a List opened for view that lost its only row to an action (not pinned)",
            );
        }
        self.show_row_page_code_positioned(st)?;
        let again = {
            let s = st.borrow();
            s.gone_blank_no_new_line && s.open && s.cursor == Cursor::None && !s.draft
        };
        if again {
            // A second action on the blank Rec runs OnNewRecord and OnAfterGetCurrRecord once more and reads no row
            // (pages/lt-outside-worksheet/RmWorksheetGoneRow.al:352 RM_Ws_OpenEdit_DeleteAllUpdate_Twice `N:0;C:0;`, an action
            // that leaves the table empty). One that leaves rows in the view is not pinned.
            let rec = self.page_rec(st);
            let probe = Rc::new(RefCell::new(rec.borrow().clone()));
            self.ret_used = true;
            if matches!(
                self.record_op(RecordOp::FindFirst, &probe, &mut [])?,
                Value::Boolean(true)
            ) {
                return unsupported(
                    "an action that put rows into a list without the new-row line that stood on a blank Rec (not pinned)",
                );
            }
            return self.settle_on_blank_rec(st, &rec);
        }
        if st.borrow().cursor != Cursor::None {
            let mut s = st.borrow_mut();
            s.gone_blank_no_new_line = false;
            s.gone_blank_shows_line = false;
        }
        if !st.borrow().open || st.borrow().cursor != Cursor::Row || st.borrow().draft {
            return Ok(());
        }
        let rec = self.page_rec(st);
        if self.stored_row(&rec)?.is_some() {
            return Ok(());
        }
        if !self.is_list(st) {
            let parts: Vec<St> = st.borrow().parts.iter().map(|(_, c)| c.clone()).collect();
            if parts
                .iter()
                .any(|c| self.declares(c, "onclosepage") || self.declares(c, "onqueryclosepage"))
            {
                return unsupported("a card closed by a deleted row whose parts declare close triggers (not pinned)");
            }
            self.page_trigger(st, "onclosepage", vec![])?;
            self.mark_closed(st);
            return Ok(());
        }
        if self.declares(st, "onfindrecord") {
            return self.settle_gone_row_with_find(st, &rec);
        }
        self.ret_used = true;
        let moved = matches!(
            self.record_op(RecordOp::Next, &rec, &mut [Value::Integer(1)])?,
            Value::Integer(1)
        ) || matches!(
            self.record_op(RecordOp::Next, &rec, &mut [Value::Integer(-1)])?,
            Value::Integer(-1)
        );
        if moved {
            let r = self.arrive(st);
            self.torn_down_on_error(st, r)?;
        } else {
            // No stored row is left, whether or not the action called CurrPage.Update. A list opened for editing shows
            // its blank line: OnNewRecord and OnAfterGetCurrRecord on a blank Rec and xRec, three times (misc/
            // lt-probes-needed-7/PnGRefresh.al:120, :142; pageupdate/TestPageUpdateGoneRow.al:List_DeletedByAction_OnlyRow_ShowsNoStoredRow,
            // a list without OnNewRecord). A list opened for viewing raises nothing (pages/W1Update.al:534, :556). A list in
            // another state (an edit list without the new-row line) is not pinned.
            let new_line = self.offers_new_line(st)?;
            if !new_line && self.blank_rec_without_new_line(st)? {
                return self.settle_on_blank_rec(st, &rec);
            }
            if !new_line
                && (st.borrow().mode != PageMode::View
                    || flag_denied(&self.engine(), st.borrow().object as usize, "InsertAllowed")?)
                && !self.read_only_view_list_shows_nothing(st)
            {
                return unsupported("a list without the new-row line that lost its only row to an action (not pinned)");
            }
            st.borrow_mut().refresh = false;
            let table = rec.borrow().table as usize;
            let fresh = self.initial_fields(table);
            rec.borrow_mut().fields = fresh.clone();
            // A list filtered to one value of key fields (a journal's template and batch, in filter group 2 or 0): the blank line takes
            // the filter values, validated, before OnNewRecord, and xRec holds them too, each of the three times (misc/
            // lt-probes-needed-7/PnGRefresh.al:672 QL_Jnl_SheetFG2_DeleteNoUpdate_OnlyRow `T.Template.V(T1);T.Batch.V(B1);P.NEW(T1/B1/0
            // x=T1/B1/0 below=Yes);P.AGCR(T1/B1/0 x=T1/B1/0)` three times, :702 with CurrPage.Update, :732 a List page, :762 filter
            // group 0). A filter on a field outside the primary key is not pinned.
            let mut filtered = false;
            if new_line {
                self.stamp_from_filters(st, false)?;
                let stamped = rec.borrow().fields.clone();
                rec.borrow_mut().fields = fresh.clone();
                filtered = !Self::same_values(&stamped, &fresh);
                let key_fields = self.table(&rec)?.keys[0].clone();
                if stamped.iter().zip(fresh.iter()).enumerate().any(|(i, (a, b))| {
                    !Self::same_values(std::slice::from_ref(a), std::slice::from_ref(b)) && !key_fields.contains(&i)
                }) {
                    return unsupported(
                        "a list filtered to one value of a field outside the key that lost its only row to an action (whether its blank line takes it is not pinned)",
                    );
                }
            }
            st.borrow_mut().cursor = Cursor::None;
            if new_line {
                st.borrow_mut().gone_blank_shows_line = !filtered;
                for _ in 0..3 {
                    let fresh = self.initial_fields(table);
                    rec.borrow_mut().fields = fresh;
                    self.stamp_from_filters(st, true)?;
                    self.set_xrec(st);
                    self.page_trigger(st, "onnewrecord", vec![Value::Boolean(true)])?;
                    self.page_event(st, "onnewrecordevent", None, Some(true))?;
                    self.page_trigger(st, "onaftergetcurrrecord", vec![])?;
                    self.page_event(st, "onaftergetcurrrecordevent", None, None)?;
                }
                // The filtered list then stands on its blank line: the fields read the filter values, a value typed there is
                // inserted at Close with the first Line No. of AutoSplitKey (PnGRefresh.al:672 `shown-after-action` T1/B1/0:,
                // `log-close` P.INSR(T1/B1/10000,typed);T.INS(T1/B1/10000,typed)).
                if filtered {
                    let mut s = st.borrow_mut();
                    s.placeholder_line = true;
                    s.split_burned = false;
                    s.cursor = Cursor::NewLine;
                    s.draft = true;
                    s.typed_before_key = false;
                    s.lower_key = None;
                    s.split_starts_low = true;
                }
            }
        }
        Ok(())
    }

    /// Whether a list without the new-row line that lost its only row to an action stands on a blank Rec: pinned for a page whose
    /// InsertAllowed is false, a Worksheet opened for edit, for view or run modally and a List opened for edit
    /// (pages/lt-outside-worksheet/RmWorksheetGoneRow.al:291, :327, :373, :397, :429). A List opened for view without the line
    /// is not pinned (with the line, pages/W1Update.al:534 pins that it raises nothing), nor is a list without the line for
    /// another reason (Editable = false, lookup mode, a tree).
    fn blank_rec_without_new_line(&mut self, st: &St) -> Result<bool, Signal> {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        if !flag_denied(&engine, object, "InsertAllowed")? {
            return Ok(false);
        }
        let read_only = engine.program.objects[object]
            .properties
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("Editable") && v.trim().eq_ignore_ascii_case("false"));
        let lookup_list = st.borrow().lookup_set && st.borrow().lookup && self.is_list_page(st);
        if read_only || lookup_list {
            return Ok(false);
        }
        let worksheet = engine.object_facts[object].page_type == Some(al_ir::PageType::Worksheet);
        // A Worksheet that shows its rows as a tree, opened for edit, stands on the blank Rec the same way, at the action and at a
        // later one (pages/lt-outside-worksheet/RmWorksheetGoneRow.al:509 SH_TreeWorksheetTemp_OpenEdit_EmptyRecNoUpdate
        // `R:10000;N:0;C:0;`, :535 _EmptyRecTwice `N:0;C:0;`, a page with InsertAllowed, DeleteAllowed and ModifyAllowed false).
        // Another combination is the registered assumption `tree-worksheet-gone-only-row-blank-rec`. A tree list whose blank Rec
        // would take a filter value is not pinned (`settle_on_blank_rec`).
        if self.tree_indentation(st).is_some() {
            let tree_pinned = worksheet && st.borrow().mode == PageMode::Edit;
            if tree_pinned
                && !(flag_denied(&engine, object, "DeleteAllowed")? && flag_denied(&engine, object, "ModifyAllowed")?)
            {
                al_builtins::assumptions::assume(
                    al_builtins::assumptions::Assumption::TreeWorksheetGoneOnlyRowBlankRec,
                );
            }
            return Ok(tree_pinned);
        }
        Ok(match st.borrow().mode {
            PageMode::Edit => worksheet || self.is_list_page(st),
            PageMode::View => worksheet,
            _ => false,
        })
    }

    /// A List with Editable = false opened for view, without the new-row line, that lost its only row to an action raises nothing
    /// and shows no row, as a List with the line opened for view does (pages/W1Update.al:534;
    /// pages/lt-outside-worksheet/RmWorksheetGoneRow.al:481 SH_ListViewNotEditable_OpenView_DeleteCurrentUpdate `R:10000;`, shown
    /// blank, First No). A List opened for view that is editable, and a non-editable one opened for edit, are not pinned.
    fn read_only_view_list_shows_nothing(&mut self, st: &St) -> bool {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let read_only = engine.program.objects[object]
            .properties
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("Editable") && v.trim().eq_ignore_ascii_case("false"));
        let lookup = st.borrow().lookup_set && st.borrow().lookup;
        let pinned = read_only
            && !lookup
            && st.borrow().mode == PageMode::View
            && engine.object_facts[object].page_type == Some(al_ir::PageType::List)
            && self.tree_indentation(st).is_none();
        if pinned {
            // Pinned with InsertAllowed, DeleteAllowed and ModifyAllowed false (RmWorksheetGoneRow.al:707-716); another
            // combination is the registered assumption `read-only-view-list-gone-only-row`.
            let all_denied = ["DeleteAllowed", "ModifyAllowed"].iter().all(|p| {
                engine.program.objects[object]
                    .properties
                    .iter()
                    .any(|(k, v)| k.eq_ignore_ascii_case(p) && v.trim().eq_ignore_ascii_case("false"))
            });
            if !all_denied {
                al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::ReadOnlyViewListGoneOnlyRow);
            }
            st.borrow_mut().gone_view_list = true;
        }
        pinned
    }

    /// The list stands on a blank Rec: OnNewRecord and OnAfterGetCurrRecord once, no row shown, First/Last/Next answer No
    /// (pages/lt-outside-worksheet/RmWorksheetGoneRow.al:291 `R:10000;N:0;C:0;`, blank controls, `first log` empty). The blank
    /// Rec takes a single-value filter of a primary-key field set in filter group 0 or 2, before OnNewRecord; a filter on a field
    /// outside the key or a range is not taken
    /// (pages/lt-outside-worksheet-2/RtFilteredGoneRow.al:355 RT_UfWs_KeyFieldGroup2 `R:5;N:Yes:0:;C:Yes:0:;`, :372 filter group
    /// 0, :389 a field outside the key, :406 both key fields `N:Yes:5:`, :423 a range, :466 a List). Another filter group, a
    /// value from several filters, PopulateAllFields or a stamped field that validates are not pinned.
    fn settle_on_blank_rec(&mut self, st: &St, rec: &Rec) -> Result<(), Signal> {
        let table = rec.borrow().table as usize;
        let fresh = self.initial_fields(table);
        rec.borrow_mut().fields = fresh.clone();
        self.stamp_from_filters(st, false)?;
        let stamped = rec.borrow().fields.clone();
        rec.borrow_mut().fields = fresh.clone();
        if !Self::same_values(&stamped, &fresh) {
            if self.tree_indentation(st).is_some() {
                return unsupported(
                    "a filtered tree list without the new-row line that lost its only row to an action (whether its blank Rec takes the filter is not pinned)",
                );
            }
            let Some(related) = self.blank_rec_takes_key_filters(st, rec, &stamped, &fresh)? else {
                return unsupported(
                    "a filtered list without the new-row line that lost its only row to an action (whether its blank Rec takes the filter is not pinned)",
                );
            };
            rec.borrow_mut().fields = stamped.clone();
            // A stamped key field with a TableRelation: the value is checked against the related table. With the related row
            // there the blank Rec takes it as without the relation; without it nothing runs
            // (pages/lt-outside-worksheet-2/RtFilteredGoneRow.al:533 SH_RelWs_KeyFieldWithTableRelationGroup2_BatchExists, :550
            // _BatchMissing).
            for pos in related {
                self.ret_used = false;
                match self.record_op(
                    RecordOp::Validate,
                    rec,
                    &mut [Value::Integer(pos as i32), stamped[pos].clone()],
                ) {
                    Ok(_) => {}
                    Err(Signal::Error(_)) => return self.blank_rec_relation_failed(st, rec),
                    Err(e) => return Err(e),
                }
            }
        }
        {
            let mut s = st.borrow_mut();
            s.refresh = false;
            s.cursor = Cursor::None;
            s.gone_blank_no_new_line = true;
        }
        // xRec stays the blank record as on an unfiltered list; whether it takes the filter values with Rec is not pinned
        // (RtFilteredGoneRow.al reads only Rec), so a read of a stamped field of xRec is refused.
        let xrec: Vec<Value> = stamped
            .iter()
            .zip(fresh)
            .map(|(a, b)| {
                if Self::same_values(std::slice::from_ref(a), std::slice::from_ref(&b)) {
                    b
                } else {
                    Value::Unknown(std::sync::Arc::from(
                        "xRec of a filtered list's blank Rec after its only row was gone (whether it takes the filter is not pinned)",
                    ))
                }
            })
            .collect();
        self.set_xrec_fields(st, xrec);
        self.page_trigger(st, "onnewrecord", vec![Value::Boolean(true)])?;
        self.page_event(st, "onnewrecordevent", None, Some(true))?;
        self.page_trigger(st, "onaftergetcurrrecord", vec![])?;
        self.page_event(st, "onaftergetcurrrecordevent", None, None)?;
        Ok(())
    }

    /// The relation check of a value the blank Rec takes failed (RtFilteredGoneRow.al:550 _BatchMissing): the error is swallowed, no trigger runs, then or at a later action, and the list shows no row. What Rec and xRec hold
    /// and what GetLastErrorText reads are not known: a read of them is refused.
    fn blank_rec_relation_failed(&mut self, st: &St, rec: &Rec) -> Result<(), Signal> {
        let unknown: Vec<Value> = rec
            .borrow()
            .fields
            .iter()
            .map(|_| {
                Value::Unknown(std::sync::Arc::from(
                    "Rec of a filtered list whose blank Rec failed the table relation of a filter value (not pinned)",
                ))
            })
            .collect();
        rec.borrow_mut().fields = unknown.clone();
        self.set_xrec_fields(st, unknown);
        self.last_error = al_builtins::native::unpinned_error();
        let mut s = st.borrow_mut();
        s.refresh = false;
        s.cursor = Cursor::None;
        s.gone_blank_no_new_line = false;
        Ok(())
    }

    /// Whether every value the filters stamp on the blank Rec (`stamped` against `fresh`) is of the pinned form (see
    /// `settle_on_blank_rec`): one `SetRange` value of a primary-key field in filter group 0 or 2, on a field without OnValidate
    /// or validate subscriber, on a page without PopulateAllFields. `Some` holds the fields among them with a TableRelation,
    /// which the caller checks under an assumption; `None` is not pinned.
    fn blank_rec_takes_key_filters(
        &mut self,
        st: &St,
        rec: &Rec,
        stamped: &[Value],
        fresh: &[Value],
    ) -> Result<Option<Vec<usize>>, Signal> {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        if engine.program.objects[object]
            .properties
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("PopulateAllFields") && v.trim().eq_ignore_ascii_case("true"))
        {
            return Ok(None);
        }
        let mut related = Vec::new();
        let table = self.table(rec)?;
        let schema = engine.program.objects[table.object as usize]
            .table
            .as_ref()
            .expect("table");
        for (pos, (a, b)) in stamped.iter().zip(fresh).enumerate() {
            if Self::same_values(std::slice::from_ref(a), std::slice::from_ref(b)) {
                continue;
            }
            let pinned_filter = {
                let r = rec.borrow();
                let mut on_field = r.filters.iter().filter(|f| f.field == pos);
                // SetRange of one value holds the range `v..v` (records.rs, RecordOp::SetRange).
                matches!(
                    (on_field.next(), on_field.next()),
                    (Some(f), None) if (f.group == 0 || f.group == 2) && matches!(
                        &f.expr,
                        al_builtins::filter::FilterExpr::Range(Some(a), Some(b)) if ops::compare(a, b).is_ok_and(|o| o.is_eq())
                    )
                )
            };
            let f = &schema.fields[pos];
            if !pinned_filter
                || !table.keys[0].contains(&pos)
                || f.on_validate.is_some()
                || !f.ext_before_validate.is_empty()
                || !f.ext_after_validate.is_empty()
                || self.listened(&table, "onbeforevalidateevent", Some(&f.name))?
                || self.listened(&table, "onaftervalidateevent", Some(&f.name))?
            {
                return Ok(None);
            }
            if f.relation {
                related.push(pos);
            }
        }
        Ok(Some(related))
    }

    /// A list that serves its rows from OnFindRecord lost its current row to an action. The client asks
    /// the trigger again (corpus pageupdate/TestPageUpdateGoneRow.al:390 List_DeletedByAction_OnFindRecord_PicksTheRow,
    /// :423 List_DeletedByAction_PassThroughFind_MiddleRow, :456 _LastRow, :483 _OnlyRow):
    /// * when it knows other rows of the list: `=` (the gone row), `=>` (from there on), and `=` again for
    ///   the row it settled on, which is the one `=>` answered, else the row before the gone one;
    /// * when it knows no other row: `=>` and `=><` only, and then it shows the blank line, raising
    ///   OnAfterGetCurrRecord once before the two calls and twice after.
    ///
    /// "Knows other rows" is read as: the rows it populated hold more than the gone one. That reading is
    /// only checked against these four shapes (docs/probes-needed.md); a list the client knows as
    /// one row while the view holds more is refused.
    pub(super) fn settle_gone_row_with_find(&mut self, st: &St, rec: &Rec) -> Result<(), Signal> {
        let gone = self.row_key(rec)?;
        let window = st.borrow().window.clone();
        let at = window.iter().position(|w| Self::same_key(&w.key, &gone));
        if window.len() > 1 {
            let Some(at) = at else {
                return unsupported("a list row an action deleted, which the client had not populated (not pinned)");
            };
            self.nav_find(st, "=")?;
            let after = self.nav_find(st, "=>")?;
            if !after {
                if at == 0 {
                    return unsupported("a deleted first row whose OnFindRecord finds no row after it (not pinned)");
                }
                rec.borrow_mut().fields = window[at - 1].stored.clone();
            }
            if !self.nav_find(st, "=")? {
                return unsupported("OnFindRecord does not find the row the client settled on");
            }
            st.borrow_mut().window = Vec::new();
            st.borrow_mut().window_from_start = false;
            st.borrow_mut().cursor = Cursor::Row;
            let r = self.arrive(st);
            return self.torn_down_on_error(st, r);
        }
        // The client knows no other row: nothing may be left in the view for that to be true.
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        self.ret_used = true;
        if matches!(
            self.record_op(RecordOp::FindFirst, &probe, &mut [])?,
            Value::Boolean(true)
        ) {
            return unsupported(
                "a list whose only populated row an action deleted, with more rows in its view (not pinned)",
            );
        }
        let table = rec.borrow().table as usize;
        let fresh = self.initial_fields(table);
        rec.borrow_mut().fields = fresh;
        st.borrow_mut().cursor = Cursor::None;
        st.borrow_mut().window = Vec::new();
        st.borrow_mut().window_from_start = false;
        self.page_trigger(st, "onaftergetcurrrecord", vec![])?;
        self.page_event(st, "onaftergetcurrrecordevent", None, None)?;
        if self.nav_find(st, "=>")? || self.nav_find(st, "=><")? {
            return unsupported("OnFindRecord finds a row in a view the page showed as empty (not pinned)");
        }
        for _ in 0..2 {
            self.page_trigger(st, "onaftergetcurrrecord", vec![])?;
            self.page_event(st, "onaftergetcurrrecordevent", None, None)?;
        }
        st.borrow_mut().refresh = false;
        Ok(())
    }

    /// A page that showed no row shows one again once page code has inserted a row the view
    /// admits and left Rec on it; code that only changes the buffer, sets a key it never
    /// inserts, or reads a row the view hides shows nothing (corpus
    /// testpart/BnrBlankPart.al:1-30, NoRow_Action_* arms).
    pub(super) fn show_row_page_code_positioned(&mut self, st: &St) -> Result<(), Signal> {
        if !st.borrow().open || st.borrow().cursor != Cursor::None {
            return Ok(());
        }
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        let pk = table.primary_key(&rec.borrow().fields);
        for (n, &f) in table.keys[0].iter().enumerate() {
            let mut p = probe.borrow_mut();
            p.filters.retain(|x| x.field != f);
            p.filters.push(al_builtins::filter::FieldFilter {
                field: f,
                expr: al_builtins::filter::FilterExpr::Range(Some(pk[n].clone()), Some(pk[n].clone())),
                group: 0,
            });
        }
        self.ret_used = true;
        if matches!(
            self.record_op(RecordOp::FindFirst, &probe, &mut [])?,
            Value::Boolean(true)
        ) {
            let fields = probe.borrow().fields.clone();
            // The probe replaced the view's filters on key fields with the key; the row must still
            // pass the filters the page has (a Get of a row the view hides shows nothing:
            // testpart/BnrBlankPart.al NoRow_Action_GetOfARowTheFilterHides_StaysBlank).
            if !rec.borrow().filters.iter().all(|f| f.expr.matches(&fields[f.field])) {
                return Ok(());
            }
            rec.borrow_mut().fields = fields;
            st.borrow_mut().cursor = Cursor::Row;
            let r = self.arrive(st);
            self.torn_down_on_error(st, r)?;
        }
        Ok(())
    }
}

/// A `TableRelation` of the plain shape `Table` or `Table.Field` (each name bare or in double quotes): the table and
/// field names. Any other shape (`where`, `if`, `filter`) gives `None`.
fn simple_relation(text: &str) -> Option<(String, Option<String>)> {
    let mut names = Vec::new();
    let mut rest = text.trim();
    loop {
        let (name, after) = if let Some(r) = rest.strip_prefix('"') {
            let end = r.find('"')?;
            (r[..end].to_string(), &r[end + 1..])
        } else {
            let end = rest
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .unwrap_or(rest.len());
            if end == 0 {
                return None;
            }
            (rest[..end].to_string(), &rest[end..])
        };
        names.push(name);
        let after = after.trim_start();
        if after.is_empty() {
            break;
        }
        rest = after.strip_prefix('.')?.trim_start();
    }
    match names.as_slice() {
        [t] => Some((t.clone(), None)),
        [t, f] => Some((t.clone(), Some(f.clone()))),
        _ => None,
    }
}

/// The code an error of an action's OnAction carries after the test page's Invoke. Pinned:
/// - `Dialog` (Error()): pages/lt-page-runtime/P82CurrPage.al:431 ActionError_Surfaces, errors/lt-az-error-texts/AzErrorTexts.al:162.
/// - `MissingUIHandler`: errors/lt-ms-refusals-133/P133MessageInHandlers.al:241.
/// - `TestField`: AzErrorTexts.al:160 (`TestWrapped:TestField`).
/// - `DB:RecordNotFound` (Get): AzErrorTexts.al:158 (`TestWrapped:CSideRecordNotFound`).
/// - `DB:RecordExists` of an Insert of a duplicate primary key ("The record in table ... already exists."): AzErrorTexts.al:181
///   (`TestWrapped:CSideDuplicateKey`). The same code of a unique index violation is not pinned.
///
/// A code that is already wrapped, or already unpinned, stays. Every other code has no pinned wrapped form: it becomes the
/// unpinned mark, so a read of it refuses the test (docs/probes-needed.md, "TestPage action error codes").
fn action_error_code(code: &str, message: &str) -> String {
    match code {
        "Dialog" | "MissingUIHandler" | "TestField" => format!("TestWrapped:{code}"),
        "DB:RecordNotFound" => "TestWrapped:CSideRecordNotFound".into(),
        "DB:RecordExists" if message.starts_with("The record in table ") => "TestWrapped:CSideDuplicateKey".into(),
        c if c.starts_with("TestWrapped") || c == al_builtins::native::UNPINNED_ERROR => c.into(),
        _ => al_builtins::native::UNPINNED_ERROR.into(),
    }
}

/// What `empty_page_shape` found: a list opened for edit on its new-row line, a list or a card opened for view showing nothing.
#[derive(Clone, Copy, PartialEq, Eq)]
enum EmptyShape {
    EditList,
    View,
    ViewCard,
}
