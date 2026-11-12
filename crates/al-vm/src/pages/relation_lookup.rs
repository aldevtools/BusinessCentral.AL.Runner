//! `TestField.Lookup()` over a TableRelation: which arm of a conditional relation applies, the lookup page it opens and the
//! write-back of the picked row.

use super::*;

impl Session {
    /// The closed state of the lookup page a relation opens, in lookup mode, of the kind `LookupKind::Relation`. What is pinned
    /// for that kind: the lookup list has no new-row line, Next from its last row answers false
    /// (testpage/P133RelationLookups.al:271-284; `lookup_list`).
    pub(super) fn relation_lookup_page(page_object: usize) -> St {
        let related = Rc::new(RefCell::new(PageState::closed(page_object as u32)));
        related.borrow_mut().open_as_lookup(LookupKind::Relation);
        related
    }

    /// A relation lookup onto a list page with OnFindRecord: the `Which` its open is asked is not pinned. `=>` is pinned for
    /// `LookupMode(true)` on a page variable (pages/list-order/QgGoToKeyNextCall.al:629) and for the static Page.RunModal
    /// (pages/run-modal/StaticRunModalFindRecordLookup.al), `=><` for a page that is no lookup
    /// (pages/extensions/PCCurrRecordReads.al FindList_OnFindRecordAndOnNextRecord); the one corpus file with a relation lookup
    /// next to OnFindRecord (testpage/AnTestPageTests.al:444) logs no trigger. A page that does `Rec.Find(Which)` on a filtered
    /// view finds another row with one than with the other, so the lookup is refused before the page opens.
    fn refuse_relation_lookup_onto_find_list(&self, related: &St) -> Result<(), Signal> {
        if self.is_list(related) && self.declares(related, "onfindrecord") {
            return unsupported(
                "Lookup over a TableRelation onto a list page with OnFindRecord (which rows its open asks for is not pinned)",
            );
        }
        Ok(())
    }

    /// The arm of the relation of `field` (the source field of control `idx`) that applies to the page's row, for
    /// `TestField.Lookup`: `if (Kind = const(1)) A else if (Kind = const(2)) B` opens the lookup page of the table whose
    /// condition holds for the row as the page shows it, also after a SetValue on the condition field
    /// (testpage/P133RelationLookups.al:262-296 first and second arm, :318-338 the condition changed on the page, :340-356
    /// cancel; al-runner issue 7454). `None` when no arm applies: no page opens, no handler is needed and the value stays
    /// (:298-316). Conditions other than `Field = const(x)` on a stored field are not pinned for a lookup and refused.
    pub(super) fn lookup_relation_arm(
        &mut self,
        st: &St,
        idx: usize,
        field: &ir::Field,
    ) -> Result<Option<ir::RelationArm>, Signal> {
        // One arm without a condition, as before (testpage/TestPageFieldRelationLookup_Tests.al:104).
        if let [arm] = field.relation_arms.as_slice() {
            if arm.conditions.is_empty() {
                return Ok(Some(arm.clone()));
            }
        }
        if field.relation_arms.is_empty() {
            return unsupported(
                "Lookup over a TableRelation with filter() conditions or an unreadable one (not pinned)",
            );
        }
        let engine = self.engine();
        let ir::Binding::Field(pos) = page_of(&engine, st).controls[idx].binding else {
            return unsupported("TestField.Lookup on a control that is not bound to a table field");
        };
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        let on_flow_field = field
            .relation_arms
            .iter()
            .flat_map(|arm| &arm.conditions)
            .any(|c| !table.fields[c.field].stored);
        if on_flow_field {
            return unsupported(
                "Lookup over a TableRelation whose condition names a FlowField or a FlowFilter (not pinned)",
            );
        }
        self.relation_arm(&rec, &table, pos)
    }

    /// `Lookup()` on a control with its own `TableRelation` (and no OnLookup): the lookup page opens modally and the pick
    /// writes the relation's field. The page is the control's `LookupPageId` (pages/controls/
    /// P115Lookups.al:108-124 P115_Lookup_PageIdAndRelation_OK: the related table has none; a pick writes `Descr`, the
    /// relation's field) or else the related table's `LookupPageId` (pages/run-modal/P113Lookups.al:253-270
    /// P113_LK_LookupWithPageRelation). Not pinned, so refused: a where() or conditional relation, a control relation over
    /// a field with its own relation or OnLookup, two different pages, a table with only a DrillDownPageId or no page.
    pub(super) fn control_relation_lookup(
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
    /// (testpage/TestPageFieldRelationLookup_Tests.al:104, :122, :142) and a control's own relation (P115Lookups.al:108-124,
    /// P113Lookups.al:253-270).
    pub(super) fn lookup_through_page(
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
        let related = Self::relation_lookup_page(page_object);
        self.page_instance(&related)?;
        self.refuse_relation_lookup_onto_find_list(&related)?;
        // A field that holds a value: the lookup page opens on the related row that holds it, a value that is no row opens on the
        // first row like a blank field (pages/run-modal/Y80LookupHoldsValue.al Y80_LK_Card_HoldsMiddle, _HoldsNoRow,
        // _HoldsBlank, _HoldsLowerCase, _AltHoldsMiddle; P82Lookups.al:247 Lookup_TableRelation_Ok for the first row). Pinned for a
        // related table of up to four rows; a larger table is refused (how much of it the page reads around the row is not pinned).
        let start = if holds_value {
            self.lookup_start_row(&related, &current, pos)?
        } else {
            None
        };
        let blank = Rc::new(RefCell::new(self.page_rec(&related).borrow().clone()));
        self.lookup_pre_open(&related, &blank, start)?;
        let origin = self.relation_origin(st, idx, field, &related)?;
        related.borrow_mut().relation_origin = origin;
        let outcome = match self.run_page_ui(&related, Some(blank), true) {
            Ok(v) => v,
            Err(e) => return self.relation_lookup_error(&related, e),
        };
        let confirmed = self.relation_pick_written(&related, &outcome).unwrap_or_else(|| {
            matches!(&outcome, Value::Option(n, members)
                if members.get(*n as usize).is_some_and(|m| m == "LookupOK"))
        });
        if !confirmed {
            return Ok(Value::Null);
        }
        let picked = self.page_rec(&related);
        let table_def = self.table(&picked)?;
        let pos = pos.unwrap_or(table_def.keys[0][0]);
        let value = picked.borrow().fields[pos].clone();
        st.borrow_mut().refresh = false;
        // The control's OnAfterLookup(Selected: RecordRef) runs with the picked row before the value is written, so Rec still holds
        // the old value (pages/pages-xmlports-upload-members/P112Pages.al:P112_Lookup_BasePageField_OK `base afterLookup sel=B rec=[]`,
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
                self.page_call(st, object, routine, inst, vec![selected.clone()])?;
            }
        }
        self.control_set_value(st, idx, &mut [value])?;
        Ok(Value::Null)
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
