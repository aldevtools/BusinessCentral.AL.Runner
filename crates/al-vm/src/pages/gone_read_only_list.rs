//! A List with Editable = false over a temporary source, without the new-row line, that lost its only row to an action
//! (al-runner issue 6664). Real BC values from corpus writer Y66 (pages/temp-list-deleted-row/Y66TempListGoneRow.al, not on
//! corpus master yet): run modally (Page.RunModal, also in lookup mode) it stands on a blank Rec and raises OnNewRecord and
//! OnAfterGetCurrRecord once (:235 `action log R:10;N:0;C:0;`, :263 with CurrPage.Update(false), :291 two rows, :319 LookupMode,
//! :350 ascending); opened by a TestPage for view or edit it raises nothing (:371 OpenView, :400 OpenEdit `R:10;`). Either way no
//! row is shown and First, Last, Next and Previous find none. A later action is built in `settle_gone_view_list_again`.

use super::*;
use al_builtins::record_op::RecordOp;

impl Session {
    /// Settles the page when it has the shape above; false when it has another shape.
    pub(super) fn gone_read_only_temp_list(&mut self, st: &St, rec: &Rec) -> Result<bool, Signal> {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let read_only = engine.program.objects[object]
            .properties
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("Editable") && v.trim().eq_ignore_ascii_case("false"));
        let shape = read_only
            && engine.object_facts[object].page_type == Some(al_ir::PageType::List)
            && rec.borrow().temporary
            && self.tree_indentation(st).is_none()
            && st.borrow().host.is_none();
        // Opened for view without being modal, the page takes the pinned path of `read_only_view_list_shows_nothing`
        // (RmWorksheetGoneRow.al:481), which Y66's OpenView agrees with.
        let pinned_elsewhere = !st.borrow().modal && st.borrow().mode == PageMode::View;
        if shape && pinned_elsewhere {
            // A later action is pinned for this page too (`settle_gone_view_list_again`; Y66_Desc_OpenView_Filter0).
            let mut s = st.borrow_mut();
            s.gone_temp_list = true;
            s.gone_view_list = true;
        }
        if !shape || pinned_elsewhere {
            return Ok(false);
        }
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::ReadOnlyTempListGoneOnlyRow);
        if st.borrow().modal {
            self.settle_on_blank_rec(st, rec)?;
            // A later action is refused (`settle_after_action`): its refill is not built.
            st.borrow_mut().gone_view_list = true;
            st.borrow_mut().gone_temp_list = true;
            return Ok(true);
        }
        let fresh = self.initial_fields(rec.borrow().table as usize);
        rec.borrow_mut().fields = fresh;
        let mut s = st.borrow_mut();
        s.refresh = false;
        s.cursor = Cursor::None;
        s.gone_view_list = true;
        s.gone_temp_list = true;
        Ok(true)
    }
}

impl Session {
    /// A later action on that list (corpus Y66TempListGoneRow.al, the `second action log` and `refill log` of every test): one
    /// that leaves the table empty again raises OnNewRecord and OnAfterGetCurrRecord once more on a modal page (:243 `N:0;C:0;`)
    /// and nothing on a page a TestPage opened (:421 `second action log` empty); one that refills it shows the rows with the
    /// cursor on the last inserted one, as an action on a list with rows does.
    pub(super) fn settle_gone_view_list_again(&mut self, st: &St) -> Result<(), Signal> {
        if !st.borrow().gone_temp_list {
            // A list over a stored table: a second action that leaves the table empty again raises nothing and shows no row, for
            // the lists of `read_only_view_list_shows_nothing` (corpus master pages/deleted-rows/Y80GoneRows.al:
            // Y80_Gone_ReadOnly_OpenEdit_DeleteAllUpdate `second action log` empty, also Y80_Gone_NoNew_OpenView_*). One that put
            // rows back, or a modal page, is not pinned.
            let rec = self.page_rec(st);
            let probe = Rc::new(RefCell::new(rec.borrow().clone()));
            self.ret_used = true;
            let refilled = matches!(
                self.record_op(RecordOp::FindFirst, &probe, &mut [])?,
                Value::Boolean(true)
            );
            if refilled || st.borrow().modal {
                return unsupported(
                    "a second action on a list without the new-row line that lost its only row to an action, that put rows back or ran modally (not pinned)",
                );
            }
            return Ok(());
        }
        if st.borrow().modal {
            let rec = self.page_rec(st);
            self.settle_on_blank_rec(st, &rec)?;
        }
        Ok(())
    }

    /// Whether the view of the gone-row list has rows again after an action.
    pub(super) fn gone_temp_list_refilled(&mut self, st: &St) -> Result<bool, Signal> {
        if !(st.borrow().gone_temp_list && st.borrow().open && st.borrow().cursor == Cursor::None) {
            return Ok(false);
        }
        let rec = self.page_rec(st);
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        self.ret_used = true;
        Ok(matches!(
            self.record_op(RecordOp::FindFirst, &probe, &mut [])?,
            Value::Boolean(true)
        ))
    }

    /// The reads of a refill (see above).
    pub(super) fn read_refilled_temp_list(&mut self, st: &St) -> Result<bool, Signal> {
        if !self.gone_temp_list_refilled(st)? {
            return Ok(false);
        }
        let rec = self.page_rec(st);
        {
            let mut s = st.borrow_mut();
            s.gone_blank_no_new_line = false;
            s.refresh = false;
        }
        // The cursor stands on the row Rec holds. The page reads it once (:252 `refill log` `R:10;` before) and then lists its rows as
        // when it opens on that row (:252 `R:10;R:10;R:10;C:10;` after one row, :459 `R:20;R:20;R:10;R:20;R:20;C:20;` with the
        // cursor on the second of two rows).
        let mut key = self.table(&rec)?.primary_key(&rec.borrow().fields);
        self.ret_used = true;
        if !matches!(self.record_op(RecordOp::Get, &rec, &mut key)?, Value::Boolean(true)) {
            return Ok(true);
        }
        let first = self.on_first_row_of_view(&rec)?;
        {
            let mut s = st.borrow_mut();
            s.cursor = Cursor::Row;
            s.window = Vec::new();
            s.draft = false;
            s.refresh = false;
        }
        self.page_trigger(st, "onaftergetrecord", vec![])?;
        if first {
            st.borrow_mut().landing_first = true;
            let r = self.arrive(st);
            let mut s = st.borrow_mut();
            s.landing_first = false;
            s.window_from_start = true;
            return r.map(|()| true);
        }
        // The cursor on the last of two rows (:459 `R:20;R:20;R:10;R:20;R:20;C:20;`): the window reads from the row up, the row
        // is read once more, then once again before OnAfterGetCurrRecord. Another row count is not pinned.
        if self.view_row_count(st)? != 2 {
            return unsupported(
                "a refilled list without the new-row line that lost its only row to an action, with the cursor on a row after the first of other than two rows (not pinned)",
            );
        }
        st.borrow_mut().open_reads_above = true;
        let r = self.populate_window(st);
        st.borrow_mut().open_reads_above = false;
        r?;
        self.page_trigger(st, "onaftergetrecord", vec![])?;
        st.borrow_mut().window_from_start = true;
        self.current_record_trigger(st, None)?;
        Ok(true)
    }
}

impl Session {
    /// A List with Editable = false opened for view, without the new-row line, that lost its only row to an action raises nothing
    /// and shows no row, as a List with the line opened for view does (pages/W1Update.al:534;
    /// testpage/RmWorksheetGoneRow.al:481 SH_ListViewNotEditable_OpenView_DeleteCurrentUpdate `R:10000;`, shown
    /// blank, First No). A List opened for view that is editable, and a non-editable one opened for edit, are not pinned.
    pub(super) fn read_only_view_list_shows_nothing(&mut self, st: &St) -> bool {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let read_only = engine.program.objects[object]
            .properties
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("Editable") && v.trim().eq_ignore_ascii_case("false"));
        let lookup = st.borrow().lookup_set && st.borrow().lookup;
        let denied = |p: &str| {
            engine.program.objects[object]
                .properties
                .iter()
                .any(|(k, v)| k.eq_ignore_ascii_case(p) && v.trim().eq_ignore_ascii_case("false"))
        };
        // An editable List with InsertAllowed, ModifyAllowed and DeleteAllowed false does the same (pages/lt-uj-outside-leads/
        // UjTempList.al UJ_Temp_List_EmptyAll `view no update` and `view update`: `invoke;R:1;`, shown blank, First No; corpus
        // branch al-runtime-rs-staging-uj 1dc037a8, former assumption `list-action-rebuilds-rows` (pinned on corpus master since bump 1030) until the bump).
        let all_denied_editable = !read_only
            && ["InsertAllowed", "ModifyAllowed", "DeleteAllowed"]
                .iter()
                .all(|p| denied(p));
        // Pinned by corpus master pages/deleted-rows/Y80GoneRows.al (codeunit 94230, the Y80_Gone_ReadOnly_*, _NoNew_OpenView_*
        // and _NoInsMod_OpenView_* tests, a list whose Insert, Modify and Delete are allowed unless the page denies them): an
        // Editable = false list opened for view or for edit, and an editable list without the new-row line (InsertAllowed = false)
        // opened for view, raise nothing and show no row after an action deleted its only row, with or without
        // CurrPage.Update(false) and whether the action deleted every row or only Rec.
        let insert_denied = denied("InsertAllowed");
        let pinned = (read_only
            || (!read_only && (all_denied_editable || insert_denied) && st.borrow().mode == PageMode::View))
            && !lookup
            && matches!(st.borrow().mode, PageMode::View | PageMode::Edit)
            && engine.object_facts[object].page_type == Some(al_ir::PageType::List)
            && self.tree_indentation(st).is_none();
        if pinned {
            st.borrow_mut().gone_view_list = true;
        }
        pinned
    }
}
