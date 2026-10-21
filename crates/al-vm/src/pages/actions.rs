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
        // The client refreshes the page after an AssistEdit trigger: OnAfterGetRecord runs again
        // (pages/lt-page-runtime/P82Lookups.al:AssistEdit_Trigger, `OnAssistEdit;OnAfterGetRecord(L1)`). A DrillDown and a Lookup are not pinned to.
        // The row is not read again: a field the trigger set on the record stays (pages/lt-page-editability-width-lookups/
        // P115Lookups.al:P115_AssistEdit_Trigger `after = assisted`).
        if method == PageMethod::AssistEdit
            && st.borrow().open
            && self.has_source(st)
            && st.borrow().cursor == Cursor::Row
            && !st.borrow().draft
            && !self.declares(st, "onfindrecord")
            && !self.declares(st, "onnextrecord")
            && (self.declares(st, "onaftergetrecord") || self.declares(st, "onaftergetcurrrecord"))
        {
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
            engine.program.objects.iter().position(|o| {
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
        if !Self::same_values(
            std::slice::from_ref(&current),
            std::slice::from_ref(&Value::default_for(&field.ty)),
        ) {
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
        let page_object = engine.program.objects.iter().position(|o| {
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
        let Some(target) = engine.program.objects.iter().position(|o| {
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
        // A duplicate-key Insert the action made comes out at the page's next read of the table, still inside Invoke, with the
        // wrapped code (errors/lt-az-error-texts/AzErrorTexts.al:167-185 PageAction_DuplicateInsert: `TestWrapped:CSideDuplicateKey`).
        // One that was pending before the action is not pinned and keeps its code.
        let pending_before = self.tx.deferred_insert.is_some();
        let raised_before = self.tx.deferred_raised;
        match self.action_invoke_body(st, idx) {
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
            st.borrow_mut().refresh = true;
            self.run_refresh(st)?;
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
            if let Some(shape) = empty {
                if self.empty_page_after_action(st, shape)? {
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
            } else if read_after_goto && reads_rows && self.is_list(st) && !self.declares(st, "onaftergetcurrrecord") {
                // After such a GoToKey an action without CurrPage.Update on a list with OnFindRecord and OnNextRecord is followed by
                // the same refresh as one with it (pages/lt-ai-list-order/QgGoToKeyNextCall.al:566 Nav_GoToKey_ThenActionWithoutUpdate_Twice
                // `R:R1003;R:R1003;A:act=R1003;R:R1003;F:=>=Yes;R:R1003;N:1=1;...` against :607 Nav_GoToKey_ThenValueRead_ThenUpdateAction).
                // Without that GoToKey it is not pinned and stays as it was.
                st.borrow_mut().refresh = true;
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
                return unsupported(
                    "an action left another row (another primary key) in Rec on a list that is not read again from that row (not pinned)",
                );
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
        if n > 1 {
            return Ok(false);
        }
        let rec = self.page_rec(st);
        let table = rec.borrow().table as usize;
        if n == 0 {
            // A page opened for view still shows nothing, also when the action stored a row the view hides (pages/W1ShowRow.al:277
            // FilteredList_InsertedRowTheViewHides, a page without row triggers). Which triggers run then is not pinned.
            if shape != EmptyShape::EditList {
                return Ok(false);
            }
            st.borrow_mut().refresh = false;
            st.borrow_mut().save_record_called = false;
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

    /// The reads of an action on a list whose cursor is not on the first row: the row again, the rows from it on (up to 20),
    /// the row again when the view goes on past it, the rows back to the first one, and the row twice more before the refresh's
    /// own read. Pinned by misc/lt-probes-needed-6/PnF05ActionReads.al:Action_List_LaterRow (row 2 and 3 of 3 rows: 7 and 6
    /// reads after the action) and :Action_List_ManyRows (row 6 of 25: 29 reads), and, with the row identities, by
    /// misc/lt-probes-needed-7/PnGNewRecord.al:ListPendingNewRowSavedBeforeAction (last row of three: `P1;P1;N2;N1;P1;P1;C`)
    /// and misc/lt-probes-needed-7/PnGListUpdate.al:ListModifyWithAndWithoutUpdate (`U2;U2;U3;U2;U1;U2;U2;C`). Only a page that declares both row triggers and serves its rows from the table is
    /// pinned; the identities of the rows beyond the window held are the table order.
    fn read_list_again_from_row(&mut self, st: &St, rec: &Rec, key: &[Value]) -> Result<(), Signal> {
        let ok = self.declares(st, "onaftergetrecord") && st.borrow().window_from_start;
        if !ok {
            // Other shapes are not pinned: the refresh alone stays (docs/probes-needed.md).
            return Ok(());
        }
        let find = self.declares(st, "onfindrecord");
        let with_current = self.declares(st, "onaftergetcurrrecord");
        let mut pk = key.to_vec();
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
        self.populate_current(st)?;
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
        if has_before || find {
            // The way back is read in blocks of 20 rows: after a block the client reads the row it started from, then goes on from the
            // last row of the block again (pages/lt-testpage-list-rows/ListRowsActionNew.al:NewRowThenAction_Ping30Rows_ReadsLogged:
            // `G100;G30;...;G11;G100;G11;G10;...;G1;G100;G100;C100`).
            let mut block = 0;
            while self.nav_next(st, -1)? {
                self.populate_current(st)?;
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
        }
        Ok(())
    }

    /// `CurrPage.Update` on a list with OnFindRecord, OnNextRecord and OnAfterGetRecord (no OnAfterGetCurrRecord) whose action just
    /// changed the view so that the stored row the cursor stands on is outside it: the call itself reads the row (pages/lt-f3-pages/
    /// F3ListTests.al:72 Nav_ActionChangesViewThenUpdate `after_update` begins `AGR(R1003) | AGR(R1003) | OnlyBig done`, both reads
    /// before the action: the one after the GoToKey and the one before every action). Remembers it for the refresh.
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
    pub(super) fn run_refresh(&mut self, st: &St) -> Result<(), Signal> {
        let without_curr = std::mem::take(&mut st.borrow_mut().refresh_without_curr);
        let without_parts = std::mem::take(&mut st.borrow_mut().refresh_without_parts);
        let read_done = std::mem::take(&mut st.borrow_mut().refresh_read_done);
        if !std::mem::take(&mut st.borrow_mut().refresh) || read_done {
            return Ok(());
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
            return Ok(());
        }
        let rec = self.page_rec(st);
        if self.stored_row(&rec)?.is_none() {
            return Ok(());
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
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::UpdateRefreshListOnFindRecord);
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
            self.page_trigger(st, "onaftergetrecord", vec![])?;
        }
        if !self.nav_find(st, "=>")? {
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
        self.show_row_page_code_positioned(st)?;
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
            if !new_line && st.borrow().mode != PageMode::View {
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
