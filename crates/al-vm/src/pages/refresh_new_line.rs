//! Where the refresh after a control's OnValidate changed the filters starts (`refresh_after_validate_filter`), including from
//! the new-row line (gap G5-20).
//!
//! Where the refresh after a control's OnValidate changed the filters starts, including from the new-
//! row line, and Update(true) from that line (gap G5-20). From the line to an empty view the page
//! shows the new-row line of that view (al-runner issue 6924, assumption `new-line-switch-to-empty-
//! view`). Entry points: `validate_refresh_start`, `same_filters_on_empty_new_line`,
//! `show_new_line_of_empty_view`, `save_found_row_from_new_line`, `reread_view_from_row` (the refresh
//! that reads the view from its first row, for `refresh_after_validate_filter` and
//! `code_inserted_new_line.rs`, which leaves out the last read of the row on its FactBox host).

use super::pinned_body::{Callees, Names, callees_are, page_is, table_is};
use super::pinned_shape::TestCall;
use super::*;
use crate::pages::state::RecCall;
use al_builtins::record_op::RecordOp;
use al_builtins::value::RtType;

/// The record methods that load a row into Rec or move it, recorded while a control's OnValidate runs (`note_page_rec_call`).
const POSITIONING: [RecordOp; 16] = [
    RecordOp::Find,
    RecordOp::FindFirst,
    RecordOp::FindLast,
    RecordOp::FindSet,
    RecordOp::Get,
    RecordOp::GetBySystemId,
    RecordOp::Next,
    RecordOp::SetPosition,
    RecordOp::Init,
    RecordOp::Clear,
    RecordOp::Copy,
    RecordOp::TransferFields,
    RecordOp::Insert,
    RecordOp::Modify,
    RecordOp::Rename,
    RecordOp::Delete,
];

/// The page call that saves the row page code moved Rec onto from the new-row line (`save_found_row_from_new_line`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum FoundRowSave {
    /// `CurrPage.Update(true)` or `CurrPage.Update()`: pinned.
    UpdateTrue,
    /// `CurrPage.SaveRecord()`: assumed in the measured cells only (`found_row_cell`).
    SaveRecord,
}

/// The lines of the SaveRecord cells' pages after their PageType (`pinned_body::page_lines`): page 73682 "LT FU Worksheet"
/// (StaleUpdateNewLine.al:57-179) and page 73683 "LT FU List" (:185-307) read the same. `«T»` is the table's name.
const FOUND_ROW_PAGE: [&str; 20] = [
    "SourceTable = \"«T»\"",
    "AutoSplitKey = true",
    "DelayedInsert = true",
    "ApplicationArea = All",
    "UsageCategory = None",
    "group Header in - {}",
    "field CurrentBatch in Header = var CurrentBatch: Code[10] {applicationarea=All; caption='Batch'}",
    "  trigger onvalidate: begin Log.Add(('V:start ' + Format(Rec.\"Line No.\"))); if Log.FirstSave() then begin \
     CurrPage.SaveRecord(); end; SetBatch(); if Log.Touch() then begin Rec.\"Description\" := 'code'; end; case Log.Second() of \
     1: begin CurrPage.SaveRecord(); end; 2: begin CurrPage.Update(true); end; 3: begin CurrPage.Update(false); end; end; \
     Log.Add(('V:done ' + Format(Rec.\"Line No.\"))); end",
    "field Boom in Header = var Boom: Integer {applicationarea=All; caption='Boom'}",
    "  trigger onvalidate: begin Log.Add(('B:start ' + Format(Rec.\"Line No.\"))); case Log.BoomMode() of 1: begin \
     CurrPage.Update(true); end; 2: begin CurrPage.Update(false); end; 3: begin CurrPage.SaveRecord(); end; end; \
     Log.Add('B:error'); Error('LT FU boom'); end",
    "repeater Lines in - {}",
    "field Line No. in Lines = Line No. {applicationarea=All}",
    "field No. in Lines = No. {applicationarea=All}",
    "field Description in Lines = Description {applicationarea=All}",
    "trigger onopenpage: begin CurrentBatch := 'B3'; SetBatch(); end",
    "trigger onaftergetrecord: begin Log.Add(((('R:' + Rec.\"Batch Name\") + '/') + Format(Rec.\"Line No.\"))); end",
    "trigger onaftergetcurrrecord: begin Log.Add(((('C:' + Rec.\"Batch Name\") + '/') + Format(Rec.\"Line No.\"))); end",
    "trigger onnewrecord: begin Log.Add(((((('N:' + Rec.\"Batch Name\") + '/') + Format(Rec.\"Line No.\")) + ' below=') + \
     Format(BelowxRec))); end",
    "trigger onmodifyrecord: begin Log.Add(((((((((('M:' + Rec.\"Batch Name\") + '/') + Format(Rec.\"Line No.\")) + '/') + \
     Rec.\"Description\") + ' x=') + xRec.\"Batch Name\") + '/') + Format(xRec.\"Line No.\"))); exit(true); end",
    "trigger oninsertrecord: begin Log.Add(((('I:' + Rec.\"Batch Name\") + '/') + Format(Rec.\"Line No.\"))); exit(true); end",
];

/// The lines of their table 73680 "LT FU Line" (StaleUpdateNewLine.al:24-52; `pinned_body::table_lines`).
const FOUND_ROW_TABLE: [&str; 8] = [
    "DataClassification = SystemMetadata",
    "field 1 \"Batch Name\": Code[10]",
    "field 2 \"Line No.\": Integer",
    "field 3 \"No.\": Code[20]",
    "field 4 \"Description\": Text[30]",
    "key \"Batch Name\", \"Line No.\" {Clustered=true}",
    "trigger OnInsert: begin Log.Add(((('L.OnInsert:' + Rec.\"Batch Name\") + '/') + Format(Rec.\"Line No.\"))); end",
    "trigger OnModify: begin Log.Add(((((('L.OnModify:' + Rec.\"Batch Name\") + '/') + Format(Rec.\"Line No.\")) + '/') + \
     Rec.\"Description\")); end",
];

/// The procedures the pages and the table call (`pinned_body::callees_are`): codeunit 73681 "LT FU Log" (`«L»`;
/// StaleUpdateNewLine.al:322-327 Add, :345-363 FirstSave, Second, Touch, BoomMode) and the page's SetBatch (`«P»`, :168-174).
const FOUND_ROW_CALLEES: [&str; 6] = [
    "\"«L»\".Add: begin if (Entries <> '') then begin Entries := (Entries + ';'); end; Entries := (Entries + Entry); end",
    "\"«L»\".FirstSave: begin exit(DoFirstSave); end",
    "\"«L»\".Second: begin exit(SecondStep); end",
    "\"«L»\".Touch: begin exit(DoTouch); end",
    "\"«L»\".BoomMode: begin exit(Boom); end",
    "\"«P»\".SetBatch: begin Rec.FilterGroup(2); Rec.SetRange(field#0, CurrentBatch); Rec.FilterGroup(0); if Rec.Find('-') then \
     begin end; end",
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
        self.show_new_line_runs(st, without_parts, false).map(|_| ())
    }

    /// `show_new_line_of_empty_view`; with `compare`, true when the second landing left the page's variables and Rec's fields
    /// as the first did (false without `compare`).
    fn show_new_line_runs(&mut self, st: &St, without_parts: bool, compare: bool) -> Result<bool, Signal> {
        let page_state = |s: &Self| {
            let rec = s.page_rec(st);
            let globals = st
                .borrow()
                .instance
                .as_ref()
                .map(|i| format!("{:?}", i.borrow().globals));
            (globals, format!("{:?}", rec.borrow().fields))
        };
        {
            let mut s = st.borrow_mut();
            s.window = Vec::new();
            s.focus = None;
            s.focus_in_row = false;
        }
        let r = (|| {
            self.land_on_new_line(st)?;
            let first = compare.then(|| page_state(self));
            self.land_on_new_line(st)?;
            Ok(first.is_some_and(|f| f == page_state(self)))
        })();
        let same = self.torn_down_on_error(st, r)?;
        st.borrow_mut().refresh.cancel();
        if !without_parts {
            self.sync_parts(st)?;
        }
        Ok(same)
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
    /// (the rows it held do not matter), the row again unless `read_row_again` is false, then OnAfterGetCurrRecord
    /// (`refresh_after_validate_filter`, and the new line the table code inserted, `code_inserted_new_line.rs`:
    /// ListNewLineCodeInsertUpdate.al:419 `ExpectedSetName`).
    pub(super) fn reread_view_from_row(
        &mut self,
        st: &St,
        without_curr: bool,
        without_parts: bool,
        read_row_again: bool,
    ) -> Result<(), Signal> {
        let rec = self.page_rec(st);
        st.borrow_mut().window_from_start = true;
        let key = self.row_key(&rec)?;
        self.read_list_again_from_row_as(st, &rec, &key, false)?;
        let mut pk = self.table(&rec)?.primary_key(&rec.borrow().fields);
        self.ret_used = true;
        self.record_op(RecordOp::Get, &rec, &mut pk)?;
        if read_row_again {
            self.populate_current(st)?;
        } else {
            st.borrow_mut().dirty = false;
            self.set_xrec(st);
        }
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
    /// pages/failed-update/StaleUpdateNewLine.al:498, :914, :1242, :1318), only in `found_row_cell` and only when the save was
    /// the measured one (`modified_alone`: OnModifyRecord answered true and wrote nothing, then the page's own Modify ran);
    /// another SaveRecord in this place is refused. True when it saved.
    pub(super) fn save_found_row_from_new_line(&mut self, st: &St, call: FoundRowSave) -> Result<bool, Signal> {
        let shape = {
            let s = st.borrow();
            s.cursor == Cursor::NewLine && s.placeholder_line && !s.dirty && s.host.is_none()
        };
        if !shape || self.stored_row(&self.page_rec(st))?.is_none() {
            return Ok(false);
        }
        let cell = self.found_row_cell(st, call)?;
        if call == FoundRowSave::SaveRecord {
            if !cell {
                return unsupported(
                    "CurrPage.SaveRecord() on the new-row line after page code moved Rec onto a stored row, outside the measured cells (whether the row is modified is not pinned)",
                );
            }
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::NewLineSaveRecordFoundRow);
        }
        let changes = self.store.change_counter();
        {
            let mut s = st.borrow_mut();
            s.cursor = Cursor::Row;
            s.draft = false;
            s.placeholder_line = false;
            s.dirty = true;
            // Set before the save: an error raised inside it is refused as one after it (`refuse_error_after_found_row_save`).
            if call == FoundRowSave::SaveRecord {
                s.found_row_save_assumed = true;
            }
        }
        // The row is written even though it holds what is stored (the buffer before it was the blank new line).
        let modify = self.save_row_outcome(st)?;
        self.set_xrec(st);
        // The cells' OnModifyRecord logs and answers true (StaleUpdateNewLine.al:156-160), the page's own Modify writes the row.
        // `found_row_code_is` already requires that body; this checks the save that ran.
        let modified_alone = modify.is_some_and(|m| m.answered_true && m.changes_before == changes);
        if call == FoundRowSave::SaveRecord {
            if !modified_alone {
                return unsupported(
                    "CurrPage.SaveRecord() on the new-row line whose row the page did not modify alone (OnModifyRecord answered false or wrote to the store; not measured)",
                );
            }
            // The refresh of the cells is the one Update(true) asks for (StaleUpdateNewLine.al:498 and :602 log the same reads).
            st.borrow_mut().refresh.ask_by_update(true);
        }
        if cell
            && modified_alone
            && self.engine.object_facts[st.borrow().object as usize].page_type == Some(al_ir::PageType::List)
        {
            if let Some(start) = st.borrow_mut().filters_before_validate.as_mut() {
                start.left_new_line = true;
            }
        }
        Ok(true)
    }

    /// A TestControl call on page `st` ended with `result`: an AL error after the assumed SaveRecord of
    /// `save_found_row_from_new_line` in that call (the OnValidate failing after it, or the refresh after it) is not measured,
    /// and refused. Takes `PageState::found_row_save_assumed`.
    pub(super) fn refuse_error_after_found_row_save(
        &mut self,
        st: &St,
        result: Result<Value, Signal>,
    ) -> Result<Value, Signal> {
        let saved = std::mem::take(&mut st.borrow_mut().found_row_save_assumed);
        match result {
            Err(Signal::Error(_)) if saved => unsupported(
                "a SetValue that fails after CurrPage.SaveRecord() saved the row page code found from the new-row line (not measured)",
            ),
            result => result,
        }
    }

    /// Page code called record method `method` on `rec` (`Session::record_op_from_al`): when `rec` is the Rec of a page whose
    /// control OnValidate runs and the method is one of `POSITIONING`, that OnValidate records the call
    /// (`ControlOnValidate::positioned`, read by `found_row_cell`).
    pub(crate) fn note_page_rec_call(&mut self, method: RecordOp, rec: &Rec, args: &[Value]) {
        if !POSITIONING.contains(&method) {
            return;
        }
        let which = match (method, args.first()) {
            (RecordOp::Find, Some(Value::Text(w))) => Some(w.as_str().to_string()),
            _ => None,
        };
        for st in self.pages.onvalidate_pages.clone() {
            let Some(slot) = self.engine.program.objects[st.borrow().object as usize].rec_slot else {
                continue;
            };
            let own = st
                .borrow()
                .instance
                .as_ref()
                .is_some_and(|i| matches!(&i.borrow().globals[slot], Value::Record(r) if Rc::ptr_eq(r, rec)));
            if let Some(v) = st.borrow_mut().control_onvalidate.as_mut().filter(|_| own) {
                v.positioned = Some(RecCall {
                    op: method,
                    which: which.clone(),
                });
            }
        }
    }

    /// Page code called `CurrPage.SaveRecord()` (`save_record`) or `CurrPage.Update(save)` on `st`: the running control
    /// OnValidate records it (`ControlOnValidate::saves`, read by `found_row_cell`).
    pub(super) fn note_currpage_save(st: &St, save_record: bool, save: bool) {
        use crate::pages::state::CurrPageSave;
        if let Some(v) = st.borrow_mut().control_onvalidate.as_mut() {
            v.saves.push(if save_record {
                CurrPageSave::SaveRecord
            } else {
                CurrPageSave::Update(save)
            });
        }
    }

    /// Whether `refresh_after_validate_filter` runs on this page: one that offers the new-row line is pinned on a Worksheet
    /// (Y17JournalBatchSwitch.al), and on a List from that line (`validate_refresh_start`); from a row a List that offers it keeps
    /// the refresh after it. A List the OnValidate's save moved from that line (`ValidateStart::left_new_line`) reads the view
    /// from its first row as from the line: assumption `new-line-found-row-list-refresh` (corpus pull request 1470,
    /// StaleUpdateNewLine.al:1242 SaveRecord and :1280 Update(true), `R:B2/10000;R:B2/20000;R:B2/10000;R:B2/10000;C:B2/10000`).
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
    /// of a List after it or after `CurrPage.Update(true)`), every fact by equality:
    /// - the page: PageType Worksheet (page 73682, :54) or List (page 73683, :182), the probe's code line for line
    ///   (`found_row_code_is`), over a database table, opened by the test with OpenEdit, no part shown;
    /// - the test's calls since the open are value reads and the running SetValue of a control bound to a page variable, none of
    ///   them failed (a failed earlier step changes the refresh: :514-580, not run here);
    /// - the CurrPage calls of that OnValidate, this one last (`ControlOnValidate::saves`): on the Worksheet SaveRecord alone
    ///   (:914) or after a first SaveRecord (:498, :1318); on the List SaveRecord (:1242) or Update(true) (:1280) after a first
    ///   SaveRecord; nothing written to the store since the OnValidate began;
    /// - the last record call of page code on Rec in that OnValidate (`ControlOnValidate::positioned`) is Find('-') (SetBatch,
    ///   :168-174; Get, FindFirst and FindSet are not measured);
    /// - the row: `found_row_first_of_view`, Rec as stored, or on the Worksheet after a first SaveRecord with one Text field
    ///   outside the key set by code (:1318 `Rec.Description := 'code'`).
    fn found_row_cell(&mut self, st: &St, call: FoundRowSave) -> Result<bool, Signal> {
        use crate::pages::state::CurrPageSave::{SaveRecord as Save, Update};
        let object = st.borrow().object as usize;
        let worksheet = match self.engine.object_facts[object].page_type {
            Some(al_ir::PageType::Worksheet) => true,
            Some(al_ir::PageType::List) => false,
            _ => return Ok(false),
        };
        let page_ok = self.found_row_code_is(st, worksheet) && !self.page_rec(st).borrow().temporary;
        let (opened, onvalidate) = {
            let s = st.borrow();
            let opened = s.test_open_edit() && s.parts.is_empty() && s.hidden_parts.is_empty();
            (opened, s.control_onvalidate.clone())
        };
        let Some(onvalidate) = onvalidate.filter(|_| page_ok && opened) else {
            return Ok(false);
        };
        let calls_ok = self.only_var_control_written(st);
        let first_save = matches!(onvalidate.saves.as_slice(), [Save, _]);
        let saves_ok = matches!(
            (worksheet, call, onvalidate.saves.as_slice()),
            (true, FoundRowSave::SaveRecord, [Save] | [Save, Save])
                | (false, FoundRowSave::SaveRecord, [Save, Save])
                | (false, FoundRowSave::UpdateTrue, [Save, Update(true)])
        );
        let touch = worksheet && call == FoundRowSave::SaveRecord && first_save;
        let find_first_row = onvalidate.positioned
            == Some(RecCall {
                op: RecordOp::Find,
                which: Some("-".to_string()),
            });
        Ok(calls_ok
            && saves_ok
            && find_first_row
            && self.store.change_counter() == onvalidate.changes
            && self.found_row_first_of_view(st, touch)?)
    }

    /// The test's calls on page `st` since the open are value reads and one SetValue of a control bound to a page variable, the
    /// running one, and none of them failed.
    fn only_var_control_written(&self, st: &St) -> bool {
        let calls: Option<Vec<TestCall>> = Self::test_calls_all_succeeded(st)
            .map(|calls| calls.into_iter().filter(|c| *c != TestCall::Read).collect());
        matches!(calls.as_deref(), Some([TestCall::Write(i)])
            if matches!(page_of(&self.engine(), st).controls[*i].binding, ir::Binding::Var(_)))
    }

    /// `CurrPage.Update` in a control's OnValidate that changed the filters, on the untouched new-row line of a list over an
    /// empty view that stays empty, when the page does not declare both OnAfterGetRecord and OnAfterGetCurrRecord (the
    /// pinned refresh of `refresh_after_validate_filter` needs both). `false`, main's refresh, when the line's values from the
    /// filters (`filter_stamps`) do not change. Where they change, the line is shown again with the new values
    /// (`show_new_line_of_empty_view`) only in the cell of Microsoft's Tests-SCM 137156 (`get_record_free_cell`), through
    /// Assumption::NewLineEmptyRefreshWithoutGetRecord; every other such page is refused.
    pub(super) fn empty_new_line_without_get_record(
        &mut self,
        st: &St,
        start: &crate::pages::state::ValidateStart,
        without_curr: bool,
        without_parts: bool,
    ) -> Result<bool, Signal> {
        let on_line = {
            let s = st.borrow();
            s.cursor == Cursor::NewLine && s.placeholder_line && s.new_line
        };
        if !on_line || self.view_row_count(st)? != 0 {
            return Ok(false);
        }
        let (_, stamps) = self.filter_stamps(st)?;
        let rec = self.page_rec(st);
        let changes = stamps
            .iter()
            .any(|(pos, v)| !ops::compare(&rec.borrow().fields[*pos], v).is_ok_and(|o| o.is_eq()));
        if !changes {
            return Ok(false);
        }
        if !self.get_record_free_cell(st, start, &stamps, without_curr)? {
            return unsupported(
                "CurrPage.Update in a control's OnValidate that changed a single-value filter on the new-row line of a list over an empty view, on a page that does not declare both OnAfterGetRecord and OnAfterGetCurrRecord, outside the shape of Microsoft's Tests-SCM 137156 (whether the line takes the new filter value there is not pinned)",
            );
        }
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::NewLineEmptyRefreshWithoutGetRecord);
        let changes = self.store.change_counter();
        let same_runs = self.show_new_line_runs(st, without_parts, true)?;
        if !same_runs || self.store.change_counter() != changes {
            return unsupported(
                "OnAfterGetCurrRecord that changes the page or writes to the store, run on the new-row line after CurrPage.Update in a control's OnValidate that changed a single-value filter, on a page that does not declare OnAfterGetRecord (how often it runs there is not pinned)",
            );
        }
        Ok(true)
    }

    /// The shape of pages 7002 "Sales Prices" and 7004 "Sales Line Discounts" in Microsoft's Tests-SCM 137156 "SCM Orders IV"
    /// (CheckValueOfSalesTypeOnSalesPricesPage, CheckValueOfSalesTypeOnSalesLineDiscounts), every fact by equality:
    /// - the page: PageType List, DelayedInsert true, AutoSplitKey false, over a database table, opened by the test with
    ///   OpenEdit, no part; it declares OnAfterGetCurrRecord and none of OnAfterGetRecord, OnNewRecord, OnFindRecord and
    ///   OnNextRecord; no page extension declares OnAfterGetCurrRecord, nothing subscribes to the page's
    ///   OnAfterGetCurrRecordEvent, and the trigger's code leaves nothing behind outside the page (`Engine::lasting_effect`:
    ///   no store write, no dialog, no write to a SingleInstance codeunit, no subscriber that does one);
    /// - the test's calls since the open: value reads and the running SetValue of a control bound to a page variable
    ///   (`only_var_control_written`); `CurrPage.Update(false)` without OnAfterGetCurrRecord left out (`without_curr` false);
    /// - that OnValidate called `CurrPage.SaveRecord()` then `CurrPage.Update(false)`, made no record call on Rec that loads
    ///   or moves a row, assigned no field of Rec (`ValidateStart::fields`) and wrote nothing to the store;
    /// - the filters (`measured_filter_cell`): page 7002, one single-value filter in group 0 on an Enum field of the primary
    ///   key, changed; or page 7004, two single-value filters in group 0 on fields of the primary key, the one on the key's
    ///   first field (an Enum) the same before and after and equal to the line's value, the other (an Option) changed.
    fn get_record_free_cell(
        &mut self,
        st: &St,
        start: &crate::pages::state::ValidateStart,
        stamps: &[(usize, Value)],
        without_curr: bool,
    ) -> Result<bool, Signal> {
        use crate::pages::state::CurrPageSave::{SaveRecord as Save, Update};
        let object = st.borrow().object as usize;
        let page_ok = self.engine().object_facts[object].page_type == Some(al_ir::PageType::List)
            && self.page_property_true(st, "DelayedInsert")
            && !self.page_property_true(st, "AutoSplitKey");
        let declares = [
            "onaftergetcurrrecord",
            "onaftergetrecord",
            "onnewrecord",
            "onfindrecord",
            "onnextrecord",
        ]
        .map(|t| self.declares(st, t));
        let opened = {
            let s = st.borrow();
            s.test_open_edit() && s.parts.is_empty() && s.hidden_parts.is_empty()
        };
        let rec = self.page_rec(st);
        let Some(onvalidate) = &start.ran else {
            return Ok(false);
        };
        let rec_unchanged = Self::same_values(&start.fields, &rec.borrow().fields);
        Ok(page_ok
            && declares == [true, false, false, false, false]
            && self.curr_record_trigger_leaves_nothing(st)
            && opened
            && !rec.borrow().temporary
            && !without_curr
            && self.only_var_control_written(st)
            && matches!(onvalidate.saves.as_slice(), [Save, Update(false)])
            && onvalidate.positioned.is_none()
            && rec_unchanged
            && self.store.change_counter() == onvalidate.changes
            && self.measured_filter_cell(&rec, &start.filters, stamps)?)
    }

    /// The page's OnAfterGetCurrRecord, which `show_new_line_of_empty_view` runs twice, is its own trigger alone (no page
    /// extension declares it, nothing subscribes to OnAfterGetCurrRecordEvent) and its code leaves nothing behind outside the
    /// page (`Engine::lasting_effect` is `None` and no Record write is let through, `caption_checked_write`). What it writes
    /// to the page is checked when it ran (`empty_new_line_without_get_record`).
    fn curr_record_trigger_leaves_nothing(&self, st: &St) -> bool {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let page = page_of(&engine, st);
        let own: Vec<usize> = page
            .triggers
            .iter()
            .filter(|(n, _)| n == "onaftergetcurrrecord")
            .map(|(_, r)| *r)
            .collect();
        let ext = page.ext_triggers.iter().any(|(n, _, _)| n == "onaftergetcurrrecord");
        matches!(own.as_slice(), [routine]
            if !ext
                && !engine.events.is_subscribed(object, "onaftergetcurrrecordevent")
                && engine.lasting_effect(object, *routine).is_none()
                && engine.caption_checked_write(object, *routine).is_none())
    }

    /// The filters of the two measured pages before (`before`) and after the OnValidate, with the line's values from the
    /// filters after it (`stamps`, field order): page 7002 "Sales Prices", whose source table's "Sales Type" is an Enum
    /// field of the primary key, filtered to one value (one filter, changed); page 7004 "Sales Line Discounts", whose
    /// OnOpenPage and OnValidate set `Type` (the key's first field, an Enum) to Item and "Sales Type" (an Option, in the
    /// key) to the chosen value (two filters, `Type` unchanged).
    fn measured_filter_cell(
        &mut self,
        rec: &Rec,
        before: &[al_builtins::filter::FieldFilter],
        stamps: &[(usize, Value)],
    ) -> Result<bool, Signal> {
        use al_builtins::filter::{FieldFilter, FilterExpr};
        let table = self.table(rec)?;
        let r = rec.borrow();
        let single = |f: &FieldFilter| match &f.expr {
            FilterExpr::Eq(v) => Some(v.clone()),
            FilterExpr::Range(Some(a), Some(b))
                if Self::same_values(std::slice::from_ref(a), std::slice::from_ref(b)) =>
            {
                Some(a.clone())
            }
            _ => None,
        };
        let key_filter = |f: &FieldFilter, enum_field: bool| {
            let ty_ok = if enum_field {
                matches!(table.fields[f.field].ty, RtType::Enum(_))
            } else {
                matches!(table.fields[f.field].ty, RtType::Option(_))
            };
            f.group == 0 && table.keys[0].contains(&f.field) && ty_ok && single(f).is_some()
        };
        let on = |filters: &[FieldFilter], field: usize| -> Option<FieldFilter> {
            match filters
                .iter()
                .filter(|f| f.field == field)
                .collect::<Vec<_>>()
                .as_slice()
            {
                [f] => Some((*f).clone()),
                _ => None,
            }
        };
        let is_line = |field: usize, v: &Value| Self::same_values(&r.fields[field..=field], std::slice::from_ref(v));
        let now = &r.filters;
        Ok(match (before, now.as_slice(), stamps) {
            // Page 7002: "Sales Type".
            ([b], [a], [(field, v)]) => {
                b.field == *field
                    && a.field == *field
                    && key_filter(b, true)
                    && key_filter(a, true)
                    && !is_line(*field, v)
            }
            // Page 7004: `Type` (the key's first field) unchanged, "Sales Type" changed. `stamps` is in field order, which
            // need not be key order.
            ([_, _], [_, _], [(f1, v1), (f2, v2)]) => {
                let Some(&first) = table.keys[0].first() else {
                    return Ok(false);
                };
                let ((kept_field, kept), (changed, v)) = match (*f1 == first, *f2 == first) {
                    (true, false) => ((*f1, v1), (*f2, v2)),
                    (false, true) => ((*f2, v2), (*f1, v1)),
                    _ => return Ok(false),
                };
                let pair = (
                    on(before, kept_field),
                    on(now, kept_field),
                    on(before, changed),
                    on(now, changed),
                );
                let (Some(kb), Some(ka), Some(cb), Some(ca)) = pair else {
                    return Ok(false);
                };
                [&kb, &ka].iter().all(|f| key_filter(f, true))
                    && [&cb, &ca].iter().all(|f| key_filter(f, false))
                    && kb.expr.identical(&ka.expr)
                    && is_line(kept_field, kept)
                    && !is_line(changed, v)
            }
            _ => false,
        })
    }

    /// Whether page `st`, its source table and the procedures they call read line for line as the probe's (`FOUND_ROW_PAGE` after
    /// the PageType of `worksheet`, `FOUND_ROW_TABLE`, `FOUND_ROW_CALLEES`), with no extension and no subscriber of the page, the
    /// table or a callee's object (`pinned_body::page_is`, `table_is`, `callees_are`).
    fn found_row_code_is(&self, st: &St, worksheet: bool) -> bool {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let table = self.page_rec(st).borrow().table as usize;
        let mut want = vec![if worksheet {
            "PageType = Worksheet"
        } else {
            "PageType = List"
        }];
        want.extend_from_slice(&FOUND_ROW_PAGE);
        let (mut callees, mut names) = (Callees::new(), Names::default());
        page_is(&engine, object, &want, &mut callees, &mut names)
            && table_is(&engine, table, &FOUND_ROW_TABLE, &mut callees, &mut names)
            && callees_are(&engine, &callees, &FOUND_ROW_CALLEES, &mut names)
    }

    /// Rec stands on the first row of a view whose only filter is the one the cells' SetBatch sets (StaleUpdateNewLine.al:168-174):
    /// an equality on the primary key's first field in filter group 2, to Rec's value, with Rec's filter group set back to 0, in
    /// the primary key's ascending order, not limited to marked rows. Rec holds that row as stored, or (`touch`) differs from it
    /// in one Text field outside the primary key.
    fn found_row_first_of_view(&mut self, st: &St, touch: bool) -> Result<bool, Signal> {
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        let view_ok = {
            let r = rec.borrow();
            let sorted = r.key == 0 && r.ascending && r.field_descending == 0 && r.adhoc.is_none() && !r.marked_only;
            let batch = |f: &al_builtins::filter::FieldFilter| {
                let value = &r.fields[f.field..=f.field];
                f.group == 2
                    && table.keys[0].first() == Some(&f.field)
                    && matches!(&f.expr, al_builtins::filter::FilterExpr::Range(Some(a), Some(b))
                        if Self::same_values(std::slice::from_ref(a), value) && Self::same_values(std::slice::from_ref(b), value))
            };
            sorted && r.filter_group == 0 && matches!(r.filters.as_slice(), [f] if batch(f))
        };
        let Some(stored) = self.stored_row(&rec)?.filter(|_| view_ok) else {
            return Ok(false);
        };
        let key = table.primary_key(&stored);
        let first =
            matches!(self.view_rows(&rec, 1)?.as_slice(), [row] if Self::same_key(&table.primary_key(row), &key));
        let fields = rec.borrow().fields.clone();
        let changed: Vec<usize> = (0..stored.len().min(fields.len()))
            .filter(|&i| !Self::same_values(&stored[i..=i], &fields[i..=i]))
            .collect();
        let as_stored = stored.len() == fields.len()
            && match changed.as_slice() {
                [] => true,
                [i] => touch && !table.keys[0].contains(i) && matches!(table.fields[*i].ty, RtType::Text(_)),
                _ => false,
            };
        Ok(first && as_stored)
    }
}
