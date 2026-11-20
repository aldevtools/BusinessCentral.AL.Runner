//! Where the refresh after a control's OnValidate changed the filters starts (`refresh_after_validate_filter`), including from
//! the new-row line (gap G5-20).

use super::pinned_shape::TestCall;
use super::*;
use al_builtins::record_op::RecordOp;

/// The page call that saves the row page code moved Rec onto from the new-row line (`save_found_row_from_new_line`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum FoundRowSave {
    /// `CurrPage.Update(true)` or `CurrPage.Update()`: pinned.
    UpdateTrue,
    /// `CurrPage.SaveRecord()`: assumed in the measured cells only (`found_row_cell`).
    SaveRecord,
}

/// The page triggers every page of the SaveRecord cells declares (StaleUpdateNewLine.al, pages 73682 and 73683).
const FOUND_ROW_TRIGGERS: [&str; 6] = [
    "onopenpage",
    "onaftergetrecord",
    "onaftergetcurrrecord",
    "onnewrecord",
    "onmodifyrecord",
    "oninsertrecord",
];

impl Session {
    /// `Some(true)` on a stored row, `Some(false)` on no row (`Cursor::None`), `None` for a start that is not pinned.
    ///
    /// From the untouched new-row line of an empty view, to a view with rows, the page leaves the line and refreshes as from a
    /// stored row: it stands on the row Rec is on, else the next row of the view after Rec's key, else the one before
    /// (pages/triggers/Y50UpdateFromNewLine.al:556 G520_FromNewLine_ToBatchWithRows `V:done 10000;R:B2/10000;
    /// R:B2/20000;R:B2/10000;R:B2/10000;C:B2/10000` as Y17JournalBatchSwitch.al:466 from a row; :574 _NoFind, Rec still the
    /// new-line buffer, lands on 20000; :592 _ThenLast; :610 _UpdateTrue; :646 the same on a List). To a view without rows it
    /// shows the new-row line of that view, as from a stored row (`show_new_line_of_empty_view`; measured, see
    /// `new_line_to_empty_view`).
    pub(super) fn validate_refresh_start(&mut self, st: &St, without_curr: bool) -> Result<Option<bool>, Signal> {
        let cursor = st.borrow().cursor;
        match cursor {
            Cursor::Row => {
                let rec = self.page_rec(st);
                Ok(self.stored_row(&rec)?.is_some().then_some(true))
            }
            Cursor::None => Ok(Some(false)),
            Cursor::NewLine => {
                if !st.borrow().placeholder_line {
                    return Ok(None);
                }
                if self.view_row_count(st)? == 0 {
                    self.new_line_to_empty_view(st, without_curr)?;
                    return Ok(Some(true));
                }
                let mut s = st.borrow_mut();
                s.cursor = Cursor::Row;
                s.draft = false;
                s.placeholder_line = false;
                Ok(Some(true))
            }
        }
    }

    /// From the untouched new-row line of an empty view to a view that is empty too (a Movement Worksheet switched to a
    /// worksheet without lines, al-runner issue 6924): BC shows the new-row line of the new view. OnNewRecord and
    /// OnAfterGetCurrRecord run twice after the OnValidate, Rec holding the values of the new filters; no row is read or
    /// stored, and page code that reads Rec next (an action) sees the new view. Pinned by pages/journal-batch-switch/
    /// EmptyToEmptyBatchSwitch.al:402 (MovementWs_EmptyToEmpty_ThenAction `V:start;V:done;N:B/L2;C:B/L2;N:B/L2;C:B/L2;`, current
    /// `T1/B/L2/10000`, action `A:B/L2;...`, result `T1/B/L2`; _NoAction; _SecondTest; JournalWs_EmptyToEmpty_ThenAction with
    /// AutoSplitKey, current `T1/B/L2/0`). Microsoft's Tests-SCM 137293 RequisitionLineForItemWithSKU passes through it. A
    /// refresh that leaves OnAfterGetCurrRecord out is not measured and stays refused.
    fn new_line_to_empty_view(&mut self, st: &St, without_curr: bool) -> Result<(), Signal> {
        if without_curr || !st.borrow().new_line {
            return unsupported(
                "CurrPage.Update in a control's OnValidate that changed the filters of a list on its new-row line, to a view without rows (whether the new row is read again with the values of the new filters is not pinned)",
            );
        }
        Ok(())
    }

    /// The same OnValidate when the filters it set are the ones the page had (the name the page already shows): the new-row
    /// line is shown again all the same, OnNewRecord and OnAfterGetCurrRecord twice (EmptyToEmptyBatchSwitch.al:459
    /// MovementWs_SameName_ThenAction `V:start;V:done;N:A/L1;C:A/L1;N:A/L1;C:A/L1;`). `false` for every other page or state:
    /// the refresh runs as before.
    pub(super) fn same_filters_on_empty_new_line(
        &mut self,
        st: &St,
        without_curr: bool,
        without_parts: bool,
    ) -> Result<bool, Signal> {
        let shape = {
            let s = st.borrow();
            s.cursor == Cursor::NewLine && s.placeholder_line && s.new_line && !without_curr
        };
        if !shape
            || self.declares(st, "onfindrecord")
            || self.declares(st, "onnextrecord")
            || self.view_row_count(st)? != 0
        {
            return Ok(false);
        }
        self.show_new_line_of_empty_view(st, without_parts)?;
        Ok(true)
    }

    /// The refresh of a page that offers the new-row line, on a view without rows: it shows that line, OnNewRecord and
    /// OnAfterGetCurrRecord run twice, the key from the filters, nothing is inserted (corpus master 97c29f86,
    /// pages/journal-batch-switch/Y17JournalBatchSwitch.al:485 UpdWs_BatchSwitch_ToEmptyBatch `V:done 20000;N:B3/0 below=Yes;
    /// C:B3/0;N:B3/0 below=Yes;C:B3/0`, :560 without the Find in code).
    pub(super) fn show_new_line_of_empty_view(&mut self, st: &St, without_parts: bool) -> Result<(), Signal> {
        {
            let mut s = st.borrow_mut();
            s.window = Vec::new();
            s.focus = None;
            s.focus_in_row = false;
        }
        let r = (|| {
            self.land_on_new_line(st)?;
            self.land_on_new_line(st)
        })();
        self.torn_down_on_error(st, r)?;
        st.borrow_mut().refresh.cancel();
        if !without_parts {
            self.sync_parts(st)?;
        }
        Ok(())
    }

    /// The end of a refresh after a control's OnValidate: OnAfterGetCurrRecord unless `without_curr`, then the parts read again
    /// unless `without_parts`.
    pub(super) fn finish_validate_refresh(
        &mut self,
        st: &St,
        without_curr: bool,
        without_parts: bool,
    ) -> Result<(), Signal> {
        let r = (|| {
            if without_curr {
                return Ok(true);
            }
            self.page_trigger(st, "onaftergetcurrrecord", vec![])?;
            self.page_event(st, "onaftergetcurrrecordevent", None, None)
        })();
        self.torn_down_on_error(st, r)?;
        st.borrow_mut().refresh.cancel();
        if !without_parts {
            self.sync_parts(st)?;
        }
        Ok(())
    }

    /// The refresh after a control's OnValidate from the stored row Rec stands on: the client reads the view from its first row
    /// (the rows it held do not matter), the row again, then OnAfterGetCurrRecord (`refresh_after_validate_filter`, and the
    /// new line the table code inserted, `code_inserted_new_line.rs`: ListNewLineCodeInsertUpdate.al:419 `ExpectedSetName`).
    pub(super) fn reread_view_from_row(
        &mut self,
        st: &St,
        without_curr: bool,
        without_parts: bool,
    ) -> Result<(), Signal> {
        let rec = self.page_rec(st);
        st.borrow_mut().window_from_start = true;
        let key = self.row_key(&rec)?;
        self.read_list_again_from_row_as(st, &rec, &key, false)?;
        let mut pk = self.table(&rec)?.primary_key(&rec.borrow().fields);
        self.ret_used = true;
        self.record_op(RecordOp::Get, &rec, &mut pk)?;
        self.populate_current(st)?;
        self.finish_validate_refresh(st, without_curr, without_parts)?;
        // The client holds the rows the refresh read (`read_list_again_from_row_as`).
        st.borrow_mut().window_top_unread = false;
        st.borrow_mut().read_up_from_last = ReadUpFromLast::Off;
        Ok(())
    }

    /// `CurrPage.Update(true)` in a control's OnValidate, on the untouched new-row line, after page code moved Rec onto a stored
    /// row (the batch switch with Find('-')): the page leaves the line and modifies that row, OnModifyRecord and the table's
    /// OnModify running inside the OnValidate (Y50UpdateFromNewLine.al:610 G520_FromNewLine_ToBatchWithRows_UpdateTrue
    /// `V:start 0;M:B2/10000;L.OnModify:B2/10000;V:done 10000;...`). `CurrPage.SaveRecord()` in the same place saves the row
    /// too, then the page refreshes as after Update(true): assumption `new-line-saverecord-found-row` (corpus pull request 1470,
    /// pages/failed-update/StaleUpdateNewLine.al:498, :914, :1242, :1318), only in `found_row_cell`; another
    /// SaveRecord in this place is refused. True when it saved.
    pub(super) fn save_found_row_from_new_line(&mut self, st: &St, call: FoundRowSave) -> Result<bool, Signal> {
        let shape = {
            let s = st.borrow();
            s.cursor == Cursor::NewLine && s.placeholder_line && !s.dirty && s.host.is_none()
        };
        if !shape || self.stored_row(&self.page_rec(st))?.is_none() {
            return Ok(false);
        }
        let cell = self.found_row_cell(st);
        if call == FoundRowSave::SaveRecord {
            if !cell {
                return unsupported(
                    "CurrPage.SaveRecord() on the new-row line after page code moved Rec onto a stored row, outside the measured cells (whether the row is modified is not pinned)",
                );
            }
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::NewLineSaveRecordFoundRow);
        }
        {
            let mut s = st.borrow_mut();
            s.cursor = Cursor::Row;
            s.draft = false;
            s.placeholder_line = false;
            s.dirty = true;
        }
        // The row is written even though it holds what is stored (the buffer before it was the blank new line).
        self.save_row(st)?;
        self.set_xrec(st);
        if cell && self.engine.object_facts[st.borrow().object as usize].page_type == Some(al_ir::PageType::List) {
            if let Some(start) = st.borrow_mut().filters_before_validate.as_mut() {
                start.left_new_line = true;
            }
        }
        if call == FoundRowSave::SaveRecord {
            // The refresh of the cells is the one Update(true) asks for (StaleUpdateNewLine.al:498 and :602 log the same reads).
            st.borrow_mut().refresh.ask_by_update(true);
        }
        Ok(true)
    }

    /// Whether `refresh_after_validate_filter` runs on this page: one that offers the new-row line is pinned on a Worksheet
    /// (Y17JournalBatchSwitch.al), and on a List from that line (`validate_refresh_start`); from a row a List that offers it keeps
    /// the refresh after it. A List the OnValidate's save moved from that line (`ValidateStart::left_new_line`) reads the view
    /// from its first row as from the line: assumption `new-line-found-row-list-refresh` (corpus pull request 1470,
    /// StaleUpdateNewLine.al:1242 SaveRecord and :1258 Update(true), `R:B2/10000;R:B2/20000;R:B2/10000;R:B2/10000;C:B2/10000`).
    pub(super) fn validate_refresh_offers_line(&self, st: &St, start: &crate::pages::state::ValidateStart) -> bool {
        if !st.borrow().new_line || self.is_worksheet(st) || st.borrow().cursor == Cursor::NewLine {
            return true;
        }
        if start.left_new_line {
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::NewLineFoundRowListRefresh);
        }
        start.left_new_line
    }

    /// The cells of StaleUpdateNewLine.al that measure the save from the new-row line (`CurrPage.SaveRecord()`, and the refresh
    /// of a List after it or after `CurrPage.Update(true)`): a page the test opened with OpenEdit, of
    /// PageType Worksheet (page 73682, :498, :914, :1318) or List (page 73683, :1242), over a database table, declaring only
    /// PageType, SourceTable, AutoSplitKey and DelayedInsert (both true), ApplicationArea and UsageCategory, exactly the triggers
    /// of `FOUND_ROW_TRIGGERS`, no part, no page extension and no event subscriber; inside the OnValidate of a control bound to a
    /// page variable, and the test's calls since the open are value reads and that SetValue, none of them failed (a failed
    /// earlier step changes the refresh: :514-580, not run here).
    fn found_row_cell(&self, st: &St) -> bool {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let page = page_of(&engine, st);
        let properties = &engine.program.objects[object].properties;
        let has_true = |name: &str| {
            properties
                .iter()
                .any(|(n, v)| n.eq_ignore_ascii_case(name) && v.trim().eq_ignore_ascii_case("true"))
        };
        let measured_property = |(name, value): &(String, String)| match name.to_ascii_lowercase().as_str() {
            "autosplitkey" | "delayedinsert" => value.trim().eq_ignore_ascii_case("true"),
            "pagetype" | "sourcetable" | "applicationarea" | "usagecategory" => true,
            _ => false,
        };
        let names: Vec<&str> = page.triggers.iter().map(|(n, _)| n.as_str()).collect();
        let page_ok = matches!(
            engine.object_facts[object].page_type,
            Some(al_ir::PageType::Worksheet | al_ir::PageType::List)
        ) && properties.iter().all(measured_property)
            && has_true("AutoSplitKey")
            && has_true("DelayedInsert")
            && names.len() == FOUND_ROW_TRIGGERS.len()
            && FOUND_ROW_TRIGGERS.iter().all(|t| names.contains(t))
            && page.controls.iter().filter(|c| c.kind == ir::ControlKind::Part).count() == 0
            && self.unextended_unsubscribed(st)
            && self.has_source(st)
            && !self.page_rec(st).borrow().temporary;
        let state_ok = {
            let s = st.borrow();
            s.test_open_edit()
                && s.control_onvalidate_writes.is_some()
                && s.parts.is_empty()
                && s.hidden_parts.is_empty()
        };
        let calls: Option<Vec<TestCall>> = Self::test_calls_all_succeeded(st)
            .map(|calls| calls.into_iter().filter(|c| *c != TestCall::Read).collect());
        let calls_ok = matches!(calls.as_deref(), Some([TestCall::Write(i)])
            if matches!(page.controls[*i].binding, ir::Binding::Var(_)));
        page_ok && state_ok && calls_ok
    }
}
