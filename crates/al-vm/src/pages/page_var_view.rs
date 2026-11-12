//! `SetTableView` and `SetRecord` on a Page variable: what the page takes of the record it is handed, and what
//! `SetSelectionFilter` hands back after the page closed.

use super::pinned_shape::{TestCall, open_page_filters_then_find};
use super::*;
use al_builtins::page_op::PageMethod;

impl Session {
    /// `P.SetTableView(Rec)` hands the page variable the view of Rec (filters and sorting) but not its position, so the page opens
    /// on its first row (corpus ui-handlers/TestPageModalNewRowLine_Tests.al: ReEditable_LookupModeTrue_HasNoNewRowLine,
    /// Plain_LookupModeFalse_HasTheNewRowLine). The marks and MarkedOnly go with the view: the page shows only the marked rows, also
    /// for a temporary record and with no mark at all (ui-handlers/Y41PageVarMarkedOnly.al:140 _Shown, :186
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
        // same record and its page reads Rec in OnOpenPage). Pinned in either order, for a list and a card, with the record inside and
        // outside the view: pages/currpage/PageVarSetRecordTableView.al P157_List_SetRecordThenSetTableView_*,
        // P157_Card_SetRecordThenSetTableView, SF_List_SetRecordThenSetTableView_Run.
        let kept = st.borrow().set_record.as_ref().map(|p| p.borrow().fields.clone());
        let fields = match kept {
            Some(f) if f.len() == copy.borrow().fields.len() => f,
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
    /// rows and stands on Rec's row (ui-handlers/Y41PageVarMarkedOnly.al:240 PVM_SetRecord_MarkedOnly, a record
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
    /// Yes, Group: 1`, count 2), pinned for a Sel without filters or marks by ui-handlers/Y41PageVarMarkedOnly.al:158
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

    /// A Page variable whose Page-typed ModalPageHandler called SetRecord, after the run: SetSelectionFilter hands the key filter on
    /// that row as on a row the page stands on, after LookupOK and LookupCancel, and replaces the record's filters, group 2 too
    /// (ui-handlers/Y41SelectionAfterClose.al:198 G401_PageVar_HandlerSetRecord_LookupOK `Id: 3`, count 1, :221
    /// _FilterGroup2View, :272 _Cancel).
    pub(super) fn closed_on_handler_set_record(&self, st: &St) -> bool {
        let s = st.borrow();
        s.handler_set_record && !s.open && !s.typed_handler && s.host.is_none()
    }

    /// The blank new row of a list opened with OpenNew: SetSelectionFilter in OnAfterGetCurrRecord hands the filter on the blank
    /// key, `Id: 0` and a count of 0 (ui-handlers/Y41SelectionAfterClose.al:309 G401_OpenNew_List_NoView).
    pub(super) fn on_open_new_row(&self, st: &St) -> bool {
        let s = st.borrow();
        s.open && s.draft && s.mode == PageMode::New && s.host.is_none() && self.is_list(st)
    }

    /// Whether `src` holds the initial values of its table in every field (the blank record).
    pub(super) fn is_blank_row(&mut self, src: &Rec) -> bool {
        let initial = self.initial_fields(src.borrow().table as usize);
        let s = src.borrow();
        s.fields.len() == initial.len()
            && s.fields
                .iter()
                .zip(&initial)
                .all(|(a, b)| al_builtins::ops::compare(a, b).ok() == Some(std::cmp::Ordering::Equal))
    }

    /// The new line of an editable list of its own, with the blank key (OnNewRecord may have filled other fields): SetSelectionFilter
    /// hands the filter on the blank key, `Id: 0` and a count of 0, and marks nothing, whether the list opened on the new line because its
    /// view has no row or Next moved past the last row onto it (testpage/Y61SelectionComposite.al:391
    /// G612_EditableList_NewLine_OnNewRecordFills_NoRow, :416 _Rows, :453 G612_EditableList_NewLine_NoOnNewRecord_NoRow, a List with
    /// DelayedInsert run from a Page variable whose Rec a procedure filtered).
    pub(super) fn on_list_new_line_blank_key(&mut self, st: &St, src: &Rec) -> Result<bool, Signal> {
        let on_line = {
            let s = st.borrow();
            s.open && s.draft && s.cursor == Cursor::NewLine && s.host.is_none()
        };
        if !on_line || !self.is_list(st) {
            return Ok(false);
        }
        let table = self.table(src)?;
        let initial = self.initial_fields(src.borrow().table as usize);
        let blank_key: Vec<Value> = table.keys[0].iter().map(|&f| initial[f].clone()).collect();
        Ok(Self::same_key(&self.row_key(src)?, &blank_key))
    }

    /// A Card or NavigatePage over a temporary table that SetRecord of a Page variable handed a record and that has no row: GetRecord
    /// after the run hands back what its Rec holds, the blank row with what an action assigned (pages/lt-y61-page-probes/
    /// Y61StoredRecToTempCard.al:313 G617b_SetRecord_RunModal_Agree `getrecord` = `//Yes`, :333 _Ok `//No`, :468
    /// G617b_NavigatePage_SetRecord_RunModal_Agree `//Yes`).
    pub(super) fn empty_temp_card_by_set_record(&mut self, st: &St, src: &Rec) -> Result<bool, Signal> {
        let page_type = self.engine.object_facts[st.borrow().object as usize].page_type.clone();
        if !src.borrow().temporary
            || st.borrow().set_record.is_none()
            || !matches!(page_type, Some(al_ir::PageType::Card | al_ir::PageType::NavigatePage))
        {
            return Ok(false);
        }
        let probe = Rc::new(RefCell::new(src.borrow().clone()));
        probe.borrow_mut().filters.clear();
        self.ret_used = true;
        Ok(matches!(
            self.record_op(al_builtins::record_op::RecordOp::IsEmpty, &probe, &mut [])?,
            Value::Boolean(true)
        ))
    }

    /// A fresh destination, or one holding just the blank-key filter of an earlier selection
    /// (HiddenPartSelectionFilter.al: `rec-before=Id: 0/0`).
    pub(super) fn plain_no_row_selection_destination(&self, dst: &Rec, initial: &[Value]) -> Result<bool, Signal> {
        let table = self.table(dst)?;
        let destination = dst.borrow();
        let key = &table.keys[0];
        Ok(destination.marks.is_empty()
            && !destination.marked_only
            && destination.filters.iter().all(|filter| {
                key.len() == 1
                    && filter.field == key[0]
                    && match &filter.expr {
                        al_builtins::filter::FilterExpr::Eq(value) => {
                            al_builtins::ops::compare(value, &initial[filter.field]).is_ok_and(|order| order.is_eq())
                        }
                        al_builtins::filter::FilterExpr::Range(Some(a), Some(b)) => {
                            let expected = &initial[filter.field];
                            al_builtins::ops::compare(a, expected).is_ok_and(|order| order.is_eq())
                                && al_builtins::ops::compare(b, expected).is_ok_and(|order| order.is_eq())
                        }
                        _ => false,
                    }
            }))
    }

    /// An action on the empty modal Worksheet of pages/currpage/SelectionFilterNoRow.al:346
    /// (Page_SetSelectionFilter_EmptyModalWorksheet_FiltersDefaultKey, page 67999) selects the default key: the selection
    /// filters the primary key on its blank value and finds nothing. Every fact required: the page ran by a statement
    /// `Page.RunModal(Page::X)` without a record and without its return value used, under a typed modal handler that invokes
    /// an action, the only call the handler made on the page so far; a Worksheet on no row that declares OnOpenPage and no
    /// other trigger, no property but the plain ones, DelayedInsert = true and InsertAllowed = false, with no page extension
    /// and no event subscriber; OnOpenPage is `open_page_filters_then_find`; Rec is stored and holds only OnOpenPage's
    /// `SetRange` of one non-blank Text value on a field outside its one-field Integer primary key, in filter group 2; the
    /// view is empty and no stored row has the blank key (the test stores Id 1, HIDDEN); the destination record is fresh:
    /// its initial fields, no filter, filter group 0, the primary key, ascending.
    pub(super) fn empty_modal_worksheet_selection(
        &mut self,
        st: &St,
        src: &Rec,
        dst: &Rec,
        initial: &[Value],
    ) -> Result<bool, Signal> {
        let object = {
            let page = st.borrow();
            if !page.open
                || !page.modal
                || page.run_static != Some(true)
                || page.lookup
                || page.statement_run
                || page.field_run
                || !page.in_action
                || page.host.is_some()
                || page.cursor != Cursor::None
            {
                return Ok(false);
            }
            page.object as usize
        };
        if !self.declares(st, "onopenpage")
            || !self.page_declares_within(st, &["onopenpage"], &["delayedinsert", "insertallowed"])
            || !open_page_filters_then_find(&self.engine.program, object, page_of(&self.engine, st))
            || Self::test_calls(st).as_deref() != Some(&[TestCall::Other(PageMethod::Invoke)])
        {
            return Ok(false);
        }
        let facts = &self.engine.object_facts[object];
        let delayed = self.engine.program.objects[object]
            .properties
            .iter()
            .any(|(key, value)| key.eq_ignore_ascii_case("DelayedInsert") && value.trim().eq_ignore_ascii_case("true"));
        if facts.page_type != Some(ir::PageType::Worksheet)
            || !delayed
            || self.ui_state.page_handler_depth == 0
            || !flag_denied(&self.engine, object, "InsertAllowed")?
        {
            return Ok(false);
        }
        let source = src.borrow();
        let destination = dst.borrow();
        let fresh = destination.key == 0
            && destination.adhoc.is_none()
            && destination.ascending
            && destination.field_descending == 0
            && destination.fields.len() == initial.len()
            && destination
                .fields
                .iter()
                .zip(initial)
                .all(|(a, b)| al_builtins::ops::compare(a, b).is_ok_and(|order| order.is_eq()));
        if source.temporary
            || destination.temporary
            || !fresh
            || !destination.filters.is_empty()
            || destination.filter_group != 0
            || source.filter_group != 0
            || source.marked_only
            || !source.marks.is_empty()
        {
            return Ok(false);
        }
        let [filter] = source.filters.as_slice() else {
            return Ok(false);
        };
        let table = self.table(src)?;
        let key = &table.keys[0];
        let exact_text = match &filter.expr {
            al_builtins::filter::FilterExpr::Range(Some(Value::Text(lower)), Some(Value::Text(upper))) => {
                !lower.is_empty() && lower == upper
            }
            _ => false,
        };
        if filter.group != 2
            || key.len() != 1
            || key.contains(&filter.field)
            || !matches!(initial[key[0]], Value::Integer(0))
            || !exact_text
            || !Self::same_key(&table.primary_key(&source.fields), &table.primary_key(initial))
        {
            return Ok(false);
        }
        let blank_key = al_builtins::filter::FieldFilter::new(
            key[0],
            al_builtins::filter::FilterExpr::Eq(initial[key[0]].clone()),
            0,
        );
        drop(source);
        drop(destination);
        Ok(self.view_row_count(st)? == 0 && self.stored_rows_passing(src, vec![blank_key])? == 0)
    }

    /// A TestPage whose own filter shows no row of a table that has rows hands the record that filter
    /// (pages/currpage/SelectionFilterNoRow.al:TestPage_FilteredToNothing: the page filter
    /// `Id: 99` on key field Id, `Count` 0, `Id: 99`). Only that shape (one range of one value on the first key
    /// field, a record without filters or marks) is pinned.
    pub(super) fn selection_of_a_test_page_filtered_to_nothing(
        &mut self,
        st: &St,
        src: &Rec,
        dst: &Rec,
    ) -> Result<bool, Signal> {
        let t = self.table(dst)?;
        let page_filters = src.borrow().filters.clone();
        let single_key_value = page_filters.len() == 1
            && page_filters[0].field == t.keys[0][0]
            && match &page_filters[0].expr {
                al_builtins::filter::FilterExpr::Eq(_) => true,
                al_builtins::filter::FilterExpr::Range(Some(a), Some(b)) => {
                    al_builtins::ops::compare(a, b).ok() == Some(std::cmp::Ordering::Equal)
                }
                _ => false,
            };
        if single_key_value
            && st.borrow().open
            && dst.borrow().filters.is_empty()
            && dst.borrow().marks.is_empty()
            && !dst.borrow().marked_only
        {
            dst.borrow_mut().filters = page_filters;
            return Ok(true);
        }
        Ok(false)
    }
}
