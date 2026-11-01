//! `SetTableView` and `SetRecord` on a Page variable: what the page takes of the record it is handed.

use super::*;

impl Session {
    /// `P.SetTableView(Rec)` hands the page variable the view of Rec (filters and sorting) but not its position, so the page opens
    /// on its first row (corpus handlers/TestPageModalNewRowLine_Tests.al: ReEditable_LookupModeTrue_HasNoNewRowLine,
    /// Plain_LookupModeFalse_HasTheNewRowLine). The marks and MarkedOnly go with the view: the page shows only the marked rows, also
    /// for a temporary record and with no mark at all (pages/lt-y41-page-probes/Y41PageVarMarkedOnly.al:140 _Shown, :186
    /// _FilterGroups, :222 _NoMarks, :258 _TempRec).
    pub(super) fn page_var_set_table_view(&mut self, st: &St, r: &Rec) -> Result<Value, Signal> {
        let copy = Rc::new(RefCell::new(r.borrow().clone()));
        // The view is its filters and sorting only: the rows of a temporary record stay with it (Item Tracking Summary is handed
        // `SetTableView(TempGlobalEntrySummary)` and then fills its own temporary Rec; the page must keep that Rec).
        copy.borrow_mut().temporary = false;
        copy.borrow_mut().temp_id = None;
        let table = copy.borrow().table as usize;
        // A record SetRecord handed the page earlier stays the position the page opens on: SetRecord gives the
        // position and SetTableView the filters (Microsoft's VendEntry-Apply Posted Entries calls both with the
        // same record and its page reads Rec in OnOpenPage; not pinned, docs/probes-needed.md).
        let kept = st.borrow().set_record.as_ref().map(|p| p.borrow().fields.clone());
        let fields = match kept {
            Some(f) if f.len() == copy.borrow().fields.len() => {
                al_builtins::assumptions::assume(
                    al_builtins::assumptions::Assumption::SetTableViewKeepsSetRecordPosition,
                );
                f
            }
            _ => self.initial_fields(table),
        };
        copy.borrow_mut().fields = fields;
        let mut s = st.borrow_mut();
        s.set_record = Some(copy);
        s.set_view = true;
        drop(s);
        self.seed_unopened_rec(st);
        Ok(Value::Null)
    }

    /// `P.SetRecord(Rec)` with a record whose MarkedOnly is on hands the page neither the marks nor MarkedOnly: the page shows all
    /// rows and stands on Rec's row (pages/lt-y41-page-probes/Y41PageVarMarkedOnly.al:240 PVM_SetRecord_MarkedOnly, a record
    /// without filters, before any SetTableView). Inside a Page-typed handler, after a SetTableView and with filters on the record
    /// it is not pinned.
    pub(super) fn page_var_set_record_marked_only(&self, st: &St, r: &Rec) -> Result<(), Signal> {
        let r = r.borrow();
        let s = st.borrow();
        if r.marked_only && (s.typed_handler || s.set_view || !r.filters.is_empty()) {
            return unsupported(
                "SetRecord on a Page variable with a record whose MarkedOnly is on, in a handler, after SetTableView or with filters (not pinned)",
            );
        }
        Ok(())
    }

    /// The copy SetRecord keeps of a record on MarkedOnly drops the marks and MarkedOnly ([`Self::page_var_set_record_marked_only`]).
    pub(super) fn without_marked_only(copy: &Rec) {
        let mut c = copy.borrow_mut();
        if c.marked_only {
            c.marks.clear();
            c.marked_only = false;
        }
    }

    /// `P.SetSelectionFilter(Sel)` of a Page variable whose page SetTableView handed a record on MarkedOnly, after the page closed
    /// with LookupOK on one of the rows: Sel gets the marked set, not the row (the page's filters, marks and MarkedOnly; `Marked:
    /// Yes, Group: 1`, count 2), pinned for a Sel without filters or marks by pages/lt-y41-page-probes/Y41PageVarMarkedOnly.al:158
    /// PVM_SetTableView_MarkedOnly_SelectionAfter and :186 _FilterGroups (`Marked: Yes`: the group-2 and OR filters of the view are
    /// not shown in group 0). Answers false when the page or Sel is not of that kind; another closing action is refused.
    pub(super) fn selection_of_a_closed_marked_only_page(&mut self, st: &St, dst: &Rec) -> Result<bool, Signal> {
        let (closed, set_view, result) = {
            let s = st.borrow();
            (!s.open && s.instance.is_some(), s.set_view, s.result.clone())
        };
        if !closed || !set_view || !self.has_source(st) {
            return Ok(false);
        }
        let rec = self.page_rec(st);
        if !rec.borrow().marked_only || Rc::ptr_eq(&rec, dst) {
            return Ok(false);
        }
        let plain = {
            let d = dst.borrow();
            d.filters.is_empty() && d.marks.is_empty() && !d.marked_only && d.table == rec.borrow().table
        };
        if !plain || result.as_deref() != Some("LookupOK") {
            return unsupported(
                "Page.SetSelectionFilter of a page shown on MarkedOnly, into a record with filters or marks or after another action than LookupOK (not pinned)",
            );
        }
        let r = rec.borrow();
        let mut d = dst.borrow_mut();
        d.filters = r.filters.clone();
        d.marks = r.marks.clone();
        d.marked_only = true;
        Ok(true)
    }
}
