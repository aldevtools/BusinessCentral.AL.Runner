//! SetValue on a variable control of a Document that OpenNew started inserts the header before the control validates, so
//! the control's OnValidate finds the row stored with the key the table's OnInsert assigned (corpus
//! testpage/TestPageBlankKeyInsert.al:256 TestPage_OpenNew_VariableControlCanModifyDocument, page 67997; :272
//! TestPage_OpenNew_VariableControlCanModifyDocumentWithPart, pages 67994 and 67995). `activate` (draft.rs) calls it.

use super::pinned_shape::table_code;
use super::*;

impl Session {
    /// Before control `idx` takes a value. A Document without DelayedInsert that inserts, on its untouched OpenNew draft:
    /// a variable control inserts the header first when the page is the pinned one (`measured_new_document`), and refuses
    /// otherwise. `leaving_control`: focus moves to `idx` from another control of the row.
    pub(super) fn insert_document_before_variable_validation(
        &mut self,
        st: &St,
        idx: usize,
        leaving_control: bool,
    ) -> Result<(), Signal> {
        let object = {
            let page = st.borrow();
            if !page.open
                || !page.draft
                || page.draft_stored
                || page.dirty
                || page.cursor == Cursor::None
                || page.host.is_some()
            {
                return Ok(());
            }
            page.object as usize
        };
        let engine = self.engine();
        let page = page_of(&engine, st);
        if engine.object_facts[object].page_type != Some(ir::PageType::Document)
            || !matches!(
                page.controls[idx].binding,
                ir::Binding::Var(_) | ir::Binding::BaseVar(_)
            )
            || !self.has_source(st)
        {
            return Ok(());
        }
        if flag_denied(&engine, object, "InsertAllowed")?
            || engine.program.objects[object].properties.iter().any(|(key, value)| {
                key.eq_ignore_ascii_case("DelayedInsert") && value.trim().eq_ignore_ascii_case("true")
            })
        {
            return Ok(());
        }
        if !(leaving_control && self.measured_new_document(st, idx)?) {
            return unsupported(
                "SetValue on a variable control of a new Document outside the measured OpenNew focus move (not pinned)",
            );
        }
        match self.insert_draft(st) {
            Err(Signal::Error(_)) => {
                return unsupported(
                    "an insert error before validating a variable control of an OpenNew Document (not pinned)",
                );
            }
            result => result?,
        }
        self.inserted_outside_view(st, idx)
    }

    /// The pinned page, every fact required (TestPageBlankKeyInsert.al:256 and :272, pages 67997 and 67994, table 60572
    /// "TPBK Row" in TpbkRow.Table.al):
    /// - the draft is the one OpenNew started, and Rec has no filter;
    /// - the page declares no trigger and no property but the plain ones, has no page extension and no event subscriber
    ///   (so no OnNewRecord, OnInsertRecord or their events run);
    /// - its controls are groups, fields of Rec without a trigger, page variables whose only trigger is OnValidate (`idx` is
    ///   one), and at most the one part `measured_new_document_part` allows;
    /// - the source table has no table extension and no event subscriber, and its only code is OnInsert.
    fn measured_new_document(&mut self, st: &St, idx: usize) -> Result<bool, Signal> {
        if !st.borrow().draft_from_open || !self.page_rec(st).borrow().filters.is_empty() {
            return Ok(false);
        }
        if !self.page_declares_within(st, &[], &[]) {
            return Ok(false);
        }
        let engine = self.engine();
        let page = page_of(&engine, st);
        let variable = |c: &ir::Control| {
            matches!(c.binding, ir::Binding::Var(_)) && c.triggers.iter().all(|(name, _)| name == "onvalidate")
        };
        let controls_ok = page.controls.iter().all(|c| match c.kind {
            ir::ControlKind::Group => true,
            ir::ControlKind::Field => match c.binding {
                ir::Binding::Field(_) => c.triggers.is_empty(),
                ir::Binding::Var(_) => variable(c),
                _ => false,
            },
            ir::ControlKind::Part => true,
            _ => false,
        });
        if !controls_ok || !variable(&page.controls[idx]) {
            return Ok(false);
        }
        let table = self.page_rec(st).borrow().table as usize;
        if table_code(&engine, table) != Some((vec!["oninsert".to_string()], false)) {
            return Ok(false);
        }
        self.measured_new_document_part(st)
    }

    /// No part, or the one part of TestPageBlankKeyInsert.al:272 (page 67994's `Lines`, page 67995), every fact required:
    /// - one part control, in the content area (not a FactBox), with no property but ApplicationArea and a SubPageLink of
    ///   one `field(..)` entry on the header's whole primary key, no Provider and no SubPageView;
    /// - the part is an open ListPart on no row that declares no trigger and no property but the plain ones, has no page
    ///   extension and no event subscriber;
    /// - the part's source table holds no row (the test's `Line.DeleteAll()`).
    fn measured_new_document_part(&mut self, st: &St) -> Result<bool, Signal> {
        let engine = self.engine();
        let page = page_of(&engine, st);
        let mut controls = page.controls.iter().filter(|c| c.kind == ir::ControlKind::Part);
        let Some(control) = controls.next() else {
            return Ok(true);
        };
        let Some(link) = &control.part_link else {
            return Ok(false);
        };
        let [ir::LinkEntry::Field(_, header_field)] = link.entries.as_slice() else {
            return Ok(false);
        };
        let parts = Self::open_parts_with_hidden(st);
        let [(_, part)] = parts.as_slice() else {
            return Ok(false);
        };
        let control_ok = controls.next().is_none()
            && !control.factbox
            && link.provider.is_none()
            && link.view.is_none()
            && control
                .props
                .iter()
                .all(|(key, _)| key.eq_ignore_ascii_case("applicationarea") || key.eq_ignore_ascii_case("subpagelink"));
        let part_object = part.borrow().object as usize;
        if !control_ok
            || !part.borrow().open
            || part.borrow().cursor == Cursor::Row
            || engine.object_facts[part_object].page_type != Some(ir::PageType::Other("listpart".into()))
            || self.table(&self.page_rec(st))?.keys[0] != [*header_field]
            || !self.page_declares_within(part, &[], &[])
        {
            return Ok(false);
        }
        let probe = Rc::new(RefCell::new(self.page_rec(part).borrow().clone()));
        {
            let mut rows = probe.borrow_mut();
            rows.filters.clear();
            rows.marks.clear();
            rows.marked_only = false;
        }
        self.ret_used = true;
        Ok(matches!(
            self.record_op(al_builtins::record_op::RecordOp::IsEmpty, &probe, &mut [])?,
            Value::Boolean(true)
        ))
    }
}
