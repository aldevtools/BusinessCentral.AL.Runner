//! SetValue on a variable control of a Document that OpenNew started inserts the header before the control validates, so
//! the control's OnValidate finds the row stored with the key the table's OnInsert assigned (corpus
//! testpage/TestPageBlankKeyInsert.al:256 TestPage_OpenNew_VariableControlCanModifyDocument, page 67997; :272
//! TestPage_OpenNew_VariableControlCanModifyDocumentWithPart, pages 67994 and 67995). `activate` (draft.rs) calls it.

use super::pinned_shape::{NextEnd, PinnedEnd, TestCall, code_stmts, table_code};
use super::*;

impl Session {
    /// Before control `idx` takes a value. A Document without DelayedInsert that inserts, on its untouched OpenNew draft,
    /// when focus moves to the variable control `idx` from another control of the row (`leaving_control`) and its one
    /// trigger is the pinned OnValidate (`pinned_scan_validate`): the header is inserted first when the page and the
    /// test's calls are the pinned ones (`measured_new_document`), and the move refuses otherwise (main ran that OnValidate
    /// on the unstored header, whose Modify failed). After the insert the test may only close the page (`PinnedEnd`).
    /// Every other shape keeps main's path.
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
        if !leaving_control
            || engine.object_facts[object].page_type != Some(ir::PageType::Document)
            || !self.has_source(st)
        {
            return Ok(());
        }
        let key = self.table(&self.page_rec(st))?.keys[0].clone();
        if !Self::pinned_scan_validate(&engine.program, object, &page_of(&engine, st).controls[idx], &key) {
            return Ok(());
        }
        if flag_denied(&engine, object, "InsertAllowed")?
            || engine.program.objects[object].properties.iter().any(|(key, value)| {
                key.eq_ignore_ascii_case("DelayedInsert") && value.trim().eq_ignore_ascii_case("true")
            })
        {
            return Ok(());
        }
        if !self.measured_new_document(st, idx)? {
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
        st.borrow_mut().pinned_end = Some(PinnedEnd {
            reason: "a call of the test after the header of an OpenNew Document was stored before its variable control \
                     validated, other than Close (not pinned)",
            reads: false,
            caption: false,
            part_reads: false,
            one_action: false,
            first_ends: false,
            part_visible: false,
            part_moves: false,
            no_writes: false,
            next: NextEnd::Refused,
            close: true,
            ok: false,
        });
        self.inserted_outside_view(st, idx)
    }

    /// Control `c` of page `object` is control Scan of pages 67997 and 67994 (TestPageBlankKeyInsert.al): bound to a page
    /// variable, its one trigger is OnValidate, and that OnValidate is, statement by statement, `Rec.<field> := <the
    /// variable>; Rec.Modify();` on a field outside the primary key `key` (`Rec.Description := InputText; Rec.Modify();`).
    fn pinned_scan_validate(program: &ir::Program, object: usize, c: &ir::Control, key: &[usize]) -> bool {
        use al_builtins::builtin_id::ids;
        let (ir::Binding::Var(var), [(name, routine)]) = (&c.binding, c.triggers.as_slice()) else {
            return false;
        };
        let Some(rec) = program.objects[object].rec_slot else {
            return false;
        };
        let is_rec = |p: &ir::Place| matches!(p, ir::Place::Global(g) if *g == rec);
        let loads_var = |e: &ir::Expr| match e {
            ir::Expr::Convert(_, inner) => matches!(&**inner, ir::Expr::Load(ir::Place::Global(g)) if g == var),
            e => matches!(e, ir::Expr::Load(ir::Place::Global(g)) if g == var),
        };
        let modify = |s: &ir::Stmt| {
            matches!(s, ir::Stmt::Expr(ir::Expr::Builtin { id, receiver: Some(receiver), args, .. })
                if *id == ids::RECORD_MODIFY_1 && args.is_empty()
                    && matches!(&**receiver, ir::Arg::Value(ir::Expr::Load(p)) | ir::Arg::Ref(p) if is_rec(p)))
        };
        name == "onvalidate"
            && match code_stmts(program, object, *routine).as_slice() {
                [ir::Stmt::Assign(ir::Place::Field(target, field), value), save] => {
                    is_rec(target) && loads_var(value) && modify(save) && !key.contains(field)
                }
                _ => false,
            }
    }

    /// The pinned page, every fact required (TestPageBlankKeyInsert.al:256 and :272, pages 67997 and 67994, table 60572
    /// "TPBK Row" at testpage/TpbkRow.Table.al:3):
    /// - the draft is the one OpenNew started, and Rec has no filter;
    /// - the test's only call on the page since OpenNew is this SetValue on control `idx` (TestPageBlankKeyInsert.al:263
    ///   `Document.OpenNew(); Document.Scan.Value('SCANNED');`);
    /// - the page declares no trigger and no property but the plain ones, has no page extension and no event subscriber
    ///   (so no OnNewRecord, OnInsertRecord or their events run);
    /// - its controls are, in this order, one group, one field of Rec on the primary key field without a trigger, the
    ///   control `idx`, and at most the one part `measured_new_document_part` allows (group General, No, Scan, Lines);
    /// - the source table has no table extension and no event subscriber, its only code is OnInsert, its primary key is one
    ///   field (TpbkRow.Table.al:16 `key(PK; "No.")`), and it holds no row (TestPageBlankKeyInsert.al:40 `Row.DeleteAll()`).
    fn measured_new_document(&mut self, st: &St, idx: usize) -> Result<bool, Signal> {
        if !st.borrow().draft_from_open
            || !self.page_rec(st).borrow().filters.is_empty()
            || Self::test_calls(st).as_deref() != Some(&[TestCall::Write(idx)])
        {
            return Ok(false);
        }
        if !self.page_declares_within(st, &[], &[]) {
            return Ok(false);
        }
        let engine = self.engine();
        let page = page_of(&engine, st);
        let rec = self.page_rec(st);
        let key = self.table(&rec)?.keys[0].clone();
        let row_controls: Vec<(usize, &ir::Control)> = page
            .controls
            .iter()
            .enumerate()
            .filter(|(_, c)| c.kind != ir::ControlKind::Part)
            .collect();
        let controls_ok = match row_controls.as_slice() {
            [(_, group), (_, no), (scan, _)] => {
                group.kind == ir::ControlKind::Group
                    && no.kind == ir::ControlKind::Field
                    && matches!(no.binding, ir::Binding::Field(f) if key == [f])
                    && no.triggers.is_empty()
                    && *scan == idx
            }
            _ => false,
        };
        if !controls_ok {
            return Ok(false);
        }
        let table = rec.borrow().table as usize;
        if table_code(&engine, table) != Some((vec!["oninsert".to_string()], false))
            || key.len() != 1
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
