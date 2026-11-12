//! SetValue on a variable control of a Document that OpenNew started inserts the header before the control validates, so
//! the control's OnValidate finds the row stored with the key the table's OnInsert assigned (corpus
//! testpage/TestPageBlankKeyInsert.al:256 TestPage_OpenNew_VariableControlCanModifyDocument, page 67997; :272
//! TestPage_OpenNew_VariableControlCanModifyDocumentWithPart, pages 67994 and 67995). `activate` (draft.rs) calls it.

use super::pinned_shape::table_code;
use super::*;

impl Session {
    /// Before control `idx` takes a value. A Document without DelayedInsert that inserts, on its untouched OpenNew draft,
    /// when focus moves to the variable control `idx` from another control of the row (`leaving_control`) and its one
    /// trigger is OnValidate: the header is inserted first when the page is the pinned one (`measured_new_document`), and
    /// the move refuses otherwise (main ran that OnValidate on the unstored header). Every other shape keeps main's path.
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
        let validating_variable =
            matches!(page.controls[idx].binding, ir::Binding::Var(_)) && Self::only_on_validate(&page.controls[idx]);
        if !leaving_control
            || !validating_variable
            || engine.object_facts[object].page_type != Some(ir::PageType::Document)
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
        if !self.measured_new_document(st)? {
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

    /// The control's triggers are exactly OnValidate (control Scan of pages 67997 and 67994). What that OnValidate does
    /// is not part of the gate: it runs after the pinned insert as ordinary code.
    fn only_on_validate(c: &ir::Control) -> bool {
        matches!(c.triggers.as_slice(), [(name, _)] if name == "onvalidate")
    }

    /// The pinned page, every fact required (TestPageBlankKeyInsert.al:256 and :272, pages 67997 and 67994, table 60572
    /// "TPBK Row" at testpage/TpbkRow.Table.al:3):
    /// - the draft is the one OpenNew started, and Rec has no filter;
    /// - the page declares no trigger and no property but the plain ones, has no page extension and no event subscriber
    ///   (so no OnNewRecord, OnInsertRecord or their events run);
    /// - its controls are groups, fields of Rec without a trigger, page variables whose only trigger is OnValidate, and at
    ///   most the one part `measured_new_document_part` allows;
    /// - the source table has no table extension and no event subscriber, its only code is OnInsert, its primary key is one
    ///   field (TpbkRow.Table.al:16 `key(PK; "No.")`), and it holds no row (TestPageBlankKeyInsert.al:40 `Row.DeleteAll()`).
    fn measured_new_document(&mut self, st: &St) -> Result<bool, Signal> {
        if !st.borrow().draft_from_open || !self.page_rec(st).borrow().filters.is_empty() {
            return Ok(false);
        }
        if !self.page_declares_within(st, &[], &[]) {
            return Ok(false);
        }
        let engine = self.engine();
        let page = page_of(&engine, st);
        let controls_ok = page.controls.iter().all(|c| match c.kind {
            ir::ControlKind::Group => true,
            ir::ControlKind::Field => match c.binding {
                ir::Binding::Field(_) => c.triggers.is_empty(),
                ir::Binding::Var(_) => Self::only_on_validate(c),
                _ => false,
            },
            ir::ControlKind::Part => true,
            _ => false,
        });
        if !controls_ok {
            return Ok(false);
        }
        let rec = self.page_rec(st);
        let table = rec.borrow().table as usize;
        if table_code(&engine, table) != Some((vec!["oninsert".to_string()], false))
            || self.table(&rec)?.keys[0].len() != 1
            || self.stored_rows_passing(&rec, Vec::new())? != 0
        {
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
        Ok(self.stored_rows_passing(&self.page_rec(part), Vec::new())? == 0)
    }
}
