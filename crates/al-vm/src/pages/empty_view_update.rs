//! A list page that shows no row before an action, and the `CurrPage.Update` after that action that fills it: the client
//! reads the page as when it opens and the first row of the new view becomes current. `action_invoke_body` (actions.rs)
//! asks `empty_list_view` before the action's trigger and `update_on_empty_list_view` after it.

use super::pinned_shape::{NextEnd, PinnedEnd, capital_word, open_page_filters_then_find, own_table_code};
use super::*;

impl Session {
    /// A list opened for view that shows no row (its view was empty), with or without OnFindRecord and OnNextRecord, or the
    /// pinned Worksheet opened for edit (`empty_edit_worksheet`).
    pub(super) fn empty_list_view(&mut self, st: &St) -> Result<bool, Signal> {
        let shape = {
            let s = st.borrow();
            s.open
                && s.host.is_none()
                && s.parts.is_empty()
                && s.window.is_empty()
                && !s.dirty
                && s.cursor == Cursor::None
        };
        if !shape || !self.is_list(st) || !self.has_source(st) || self.is_request_page(st) {
            return Ok(false);
        }
        let mode = st.borrow().mode;
        match mode {
            PageMode::View => Ok(true),
            PageMode::Edit => self.empty_edit_worksheet(st),
            _ => Ok(false),
        }
    }

    /// After an action on a list opened for view that showed no row, which changed the filters so the view has rows and
    /// called `CurrPage.Update(false)`, while Rec stands on no row of the view: the client fills the page as when it opens,
    /// from the first row of the new view, which becomes current; a page with OnFindRecord asks `=><` and then `=`
    /// (pages/list-order/AiListOrderTests.al QP_EmptyView_UpdateFalse_ShowAll_Nav, _ShowFromThree_Nav, _ShowAll_Plain,
    /// _ShowFromThree_Plain; pages 87158, 87159 in QpUpdateEmptyList.al; the traces equal the open traces of :105 and :265).
    /// Pinned for views of 4 and 6 rows that the window holds. A view of one row is `empty_page_after_action`'s; Update(true),
    /// no Update, or Rec on a row of the view go on as before. On the blank row an earlier action left (`empty_after_action`)
    /// Update(true) fills the page the same way, read as at open from the first row (pages/actions/ActionFilterEmptyView.al
    /// Action_NoRowsLeft_ThenCleared_UpdateTrue `cleared` `[1|1]:a0;u0;R1;R2;R3;R4;R1;R1;C1`, as _UpdateFalse).
    /// The pinned Worksheet opened for edit (`empty_edit_worksheet`) is filled the same way by Update(false) when the action
    /// stored the one row of its view and left Rec as `empty_edit_worksheet_kept` requires (pages/currpage/SelectionFilterNoRow.al:372
    /// TestPage_Update_NewWorksheetRow_IsSelectable and :385 TestPage_First_AfterWorksheetUpdate_FindsImportedRow: the
    /// action after it selects that row, and First() finds it). After that fill the test may read values, invoke one action,
    /// move with First (which ends the limit) and close the page, as those two tests do (`PinnedEnd`).
    pub(super) fn update_on_empty_list_view(&mut self, st: &St) -> Result<bool, Signal> {
        {
            let s = st.borrow();
            if !(s.open
                && s.refresh
                && s.refresh_by_update
                && (s.refresh_update_false || s.empty_after_action)
                && !s.dirty
                && s.cursor == Cursor::None)
            {
                return Ok(false);
            }
        }
        let n = self.view_row_count(st)?;
        let mode = st.borrow().mode;
        match mode {
            PageMode::Edit => {
                if !(n == 1 && st.borrow().refresh_update_false && self.empty_edit_worksheet_kept(st)?) {
                    return Ok(false);
                }
            }
            _ if n < 2 => return Ok(false),
            _ => {}
        }
        let rec = self.page_rec(st);
        let row = self.stored_row(&rec)?;
        let on_row = al_builtins::filter::passing(&rec.borrow().filters, row)?.is_some();
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
            s.empty_after_action = false;
        }
        self.page_first_row(st)?;
        if mode == PageMode::Edit {
            st.borrow_mut().pinned_end = Some(PinnedEnd {
                reason: "a call of the test after the refresh of an empty Worksheet opened for edit, other than a read, one \
                         action, First and Close (not pinned)",
                reads: true,
                one_action: true,
                first_ends: true,
                part_visible: false,
                part_moves: false,
                next: NextEnd::Refused,
            });
        }
        Ok(true)
    }

    /// The Worksheet of SelectionFilterNoRow.al:372 (page 67996) opened for edit by the test page and showing no row, every
    /// fact required: not modal, not run by a handler, no draft, no part; it declares OnOpenPage and no other trigger, no
    /// property but the plain ones, SourceTableView, DelayedInsert = true and InsertAllowed = false, and has no page extension
    /// and no event subscriber; OnOpenPage is `open_page_filters_then_find`; Rec is a stored table filtered as
    /// `empty_edit_worksheet_filters` requires, in filter group 0, whose code is `refresh_sheet_table_code`; the view is
    /// empty and the table holds one row, the one the filters hide (SelectionFilterNoRow.al:401-405 `Initialize`: Entry No.
    /// 1, HIDDEN).
    fn empty_edit_worksheet(&mut self, st: &St) -> Result<bool, Signal> {
        let object = {
            let s = st.borrow();
            if s.modal || s.typed_handler || s.draft || !s.hidden_parts.is_empty() {
                return Ok(false);
            }
            s.object as usize
        };
        if self.engine.object_facts[object].page_type != Some(ir::PageType::Worksheet)
            || !self.declares(st, "onopenpage")
            || !self.page_declares_within(
                st,
                &["onopenpage"],
                &["sourcetableview", "delayedinsert", "insertallowed"],
            )
            || !open_page_filters_then_find(&self.engine.program, object, page_of(&self.engine, st))
            || !flag_denied(&self.engine, object, "InsertAllowed")?
            || !effective_prop(&self.engine, object, "DelayedInsert")?.is_some_and(|v| v.eq_ignore_ascii_case("true"))
            || self.page_rec(st).borrow().temporary
            || self.page_rec(st).borrow().filter_group != 0
            || !self.empty_edit_worksheet_filters(st)?
            || !self.refresh_sheet_table_code(st)
        {
            return Ok(false);
        }
        Ok(self.view_row_count(st)? == 0 && self.stored_rows_passing(&self.page_rec(st), Vec::new())? == 1)
    }

    /// After the action on the pinned Worksheet: Rec as the action of page 67996 leaves it, which does not touch Rec
    /// (SelectionFilterNoRow.al ImportRow): its filters, key and order as before the action (`view_before_action`), filter
    /// group 0, and the blank primary key OnOpenPage's `FindFirst` left on no row.
    fn empty_edit_worksheet_kept(&mut self, st: &St) -> Result<bool, Signal> {
        let Some((filters, key, adhoc, ascending)) = st.borrow().view_before_action.clone() else {
            return Ok(false);
        };
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        let initial = self.initial_fields(table.object as usize);
        let r = rec.borrow();
        Ok(crate::records::same_filters(&filters, &r.filters)
            && key == r.key
            && adhoc == r.adhoc.as_ref().map(|(_, f)| f.clone())
            && ascending == r.ascending
            && r.filter_group == 0
            && Self::same_key(&table.primary_key(&r.fields), &table.primary_key(&initial)))
    }

    /// Rec's filters are the two of page 67996 (SelectionFilterNoRow.al): the SourceTableView of one `const` condition and
    /// OnOpenPage's `SetRange` of one `capital_word` Text or Code value on another field that is not in the primary key, both in
    /// filter group 2. No marks. The primary key is one field (ALTUniversal.al `key(PK; "Entry No.")`).
    fn empty_edit_worksheet_filters(&mut self, st: &St) -> Result<bool, Signal> {
        let object = st.borrow().object as usize;
        let Some(raw) = self.engine.program.objects[object]
            .properties
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("SourceTableView"))
            .map(|(_, v)| v.clone())
        else {
            return Ok(false);
        };
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        if table.keys[0].len() != 1 {
            return Ok(false);
        }
        let Some(view_field) = crate::views::single_where_const(&table, &crate::views::property_view(&raw)) else {
            return Ok(false);
        };
        let r = rec.borrow();
        let [a, b] = r.filters.as_slice() else {
            return Ok(false);
        };
        let set_range = |f: &al_builtins::filter::FieldFilter| {
            f.field != view_field
                && !table.keys[0].contains(&f.field)
                && matches!(&f.expr, al_builtins::filter::FilterExpr::Range(Some(lo), Some(hi))
                    if matches!((lo, hi), (Value::Text(a), Value::Text(b)) | (Value::Code(a), Value::Code(b)) if capital_word(a) && a == b))
        };
        let pair = (a.field == view_field && set_range(b)) || (b.field == view_field && set_range(a));
        Ok(pair && a.group == 2 && b.group == 2 && r.marks.is_empty() && !r.marked_only)
    }

    /// The code the source table of page 67996 can run in the corpus run of SelectionFilterNoRow.al:372, every fact
    /// required: table "ALT Universal" (_fixtures/tables/ALTUniversal.al:1) declares no trigger and no procedure; its one
    /// table extension (_fixtures/tables/ALTUniversalValidated.TableExt.al:5) declares one routine, the OnValidate of a field
    /// it adds; its one event subscriber (_fixtures/events/ALTManualTableEventSubscriber.al:28) is of OnAfterInsertEvent, in
    /// a codeunit with EventSubscriberInstance = Manual that has no bound instance.
    fn refresh_sheet_table_code(&mut self, st: &St) -> bool {
        let table = self.page_rec(st).borrow().table as usize;
        let engine = self.engine();
        let program = &engine.program;
        let own = own_table_code(&program.objects[table]) == (Vec::new(), false);
        let extension_ok = match program.objects[table].table.as_ref().map(|t| t.extensions.as_slice()) {
            Some([ext]) => matches!(program.objects[*ext].routines.as_slice(),
                [r] if r.is_trigger && r.field.is_some() && r.name.eq_ignore_ascii_case("onvalidate")),
            _ => false,
        };
        let mut events = engine
            .events
            .subscribers
            .iter()
            .filter(|((o, _), v)| *o == table && !v.is_empty());
        let subscriber = match (events.next(), events.next()) {
            (Some(((_, event), subscribers)), None) => match subscribers.as_slice() {
                [s] if event == "onafterinsertevent" && s.manual => Some(s.object),
                _ => None,
            },
            _ => None,
        };
        let no_missing = !engine.events.missing.keys().any(|(o, _)| *o == table);
        drop(engine);
        own && extension_ok && no_missing && subscriber.is_some_and(|cu| self.bound_instances(cu).is_empty())
    }
}
