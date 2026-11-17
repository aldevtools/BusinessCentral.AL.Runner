//! A List page on the line `New()` started, whose table field's OnValidate inserted Rec itself (`if "Entry No." = 0 then
//! Insert(true)`, an AutoIncrement key field) before the control's OnValidate calls `CurrPage.Update()` or `CurrPage.Update(false)`
//! (al-runner issue 8186). BC treats the line as the stored row from then on: `Update()` modifies it (OnModifyRecord and the
//! table's OnModify run inside the OnValidate, OnInsertRecord never runs), `Update(false)` saves nothing, and both refresh onto
//! that row; the next SetValue lands on it. Measured by corpus pull request 1425 (merged, not pinned yet),
//! pages/currpage/ListNewLineCodeInsertUpdate.al:515-610 and :783-810; assumption `list-new-line-code-insert-update` until a bump pins it.

use super::pinned_shape::TestCall;
use super::*;
use al_builtins::page_op::PageMethod;

/// The page triggers the probe measured without a part (ListNewLineCodeInsertUpdate.al): pages 97032 and 97033 (OnAfterGetRecord
/// and OnAfterGetCurrRecord; `Update()` and `Update(false)`) and page 97039 (neither row trigger; `Update()`). The host with
/// OnAfterGetRecord only and CardPart FactBoxes (page 97036) is measured too, but its parts read at other steps than the runner's
/// (the relink at `New()`, the reads after the next SetValue and after a move): it keeps the refusal.
enum MeasuredPage {
    RowTriggers,
    Quiet,
}

impl Session {
    /// The measured kind of `st`, or `None`: a top-level List page without parts (PageType List, no DelayedInsert, no OnFindRecord
    /// or OnNextRecord, a database source) opened for edit, on the line `New()` started, inside a control's validation that
    /// inserted the line itself (`inserted_by_validation`). Every other page, mode and line keeps main's way.
    fn code_inserted_new_line(&mut self, st: &St) -> Result<Option<MeasuredPage>, Signal> {
        let shape = {
            let s = st.borrow();
            s.open
                && s.host.is_none()
                && s.parts.is_empty()
                && s.hidden_parts.is_empty()
                && s.mode == PageMode::Edit
                && !s.test_opened_new
                && s.cursor == Cursor::NewLine
                && s.draft
                && s.new_by_call
                && !s.draft_stored
                && s.validating_control
        };
        if !shape
            || self.page_rec(st).borrow().temporary
            || !self.is_list_page(st)
            || !self.has_source(st)
            || self.declares(st, "onfindrecord")
            || self.declares(st, "onnextrecord")
        {
            return Ok(None);
        }
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let on = |name: &str| -> Result<bool, Signal> {
            Ok(effective_prop(&engine, object, name)?.is_some_and(|v| v.trim().eq_ignore_ascii_case("true")))
        };
        if on("DelayedInsert")? || on("SourceTableTemporary")? {
            return Ok(None);
        }
        let kind = match (
            self.declares(st, "onaftergetrecord"),
            self.declares(st, "onaftergetcurrrecord"),
        ) {
            (true, true) => MeasuredPage::RowTriggers,
            (false, false) => MeasuredPage::Quiet,
            _ => return Ok(None),
        };
        Ok(self.inserted_by_validation(st)?.then_some(kind))
    }

    /// Whether Rec is a row the validation inserted from the line: the primary key has one AutoIncrement field, blank when the line
    /// started, and its other fields are the line's; the row is stored now and was written after the line started (its
    /// SystemRowVersion is not older than the store's mark then). The probe's table is that shape; a key the code typed is not
    /// measured.
    fn inserted_by_validation(&mut self, st: &St) -> Result<bool, Signal> {
        let (Some(start), Some((mark, _))) = ({
            let s = st.borrow();
            (s.draft_start.clone(), s.new_line_writes)
        }) else {
            return Ok(false);
        };
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        let engine = self.engine();
        let Some(schema) = engine.program.objects[table.object as usize].table.as_ref() else {
            return Ok(false);
        };
        let key = &table.keys[0];
        let auto: Vec<usize> = key
            .iter()
            .copied()
            .filter(|&k| schema.fields[k].auto_increment)
            .collect();
        let [auto] = auto[..] else {
            return Ok(false);
        };
        let blank_at_start = matches!(start.get(auto), Some(Value::Integer(0) | Value::BigInteger(0)));
        let line_key = key
            .iter()
            .filter(|&&k| k != auto)
            .all(|&k| Self::same_values(&rec.borrow().fields[k..=k], &start[k..=k]));
        if !blank_at_start || !line_key {
            return Ok(false);
        }
        let Some(stored) = self.stored_row(&rec)? else {
            return Ok(false);
        };
        Ok(crate::stale_rename::row_version(&table, &stored).is_some_and(|v| v >= mark))
    }

    /// `CurrPage.Update()` (`arg` `None`) or `CurrPage.Update(false)` in that shape: the page leaves the new line for the stored row
    /// before the call goes on, so `Update()` saves it as a stored row (its modify, `P.MOD(X/);T.M(X/);` before the OnValidate ends,
    /// ListNewLineCodeInsertUpdate.al:419 `ExpectedSetName`) and `Update(false)` (measured on the pages with both row triggers only)
    /// saves nothing. The refresh the call asked for reads the view from its first row (`code_inserted_refresh_due`).
    /// `Update(true)` written out is not measured and keeps main's way.
    pub(super) fn land_on_code_inserted_line(&mut self, st: &St, arg: Option<&Value>) -> Result<(), Signal> {
        let save = match arg {
            None => true,
            Some(Value::Boolean(false)) => false,
            _ => return Ok(()),
        };
        match self.code_inserted_new_line(st)? {
            Some(MeasuredPage::RowTriggers) => {}
            Some(MeasuredPage::Quiet) if save => {}
            _ => return Ok(()),
        }
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::ListNewLineCodeInsertUpdate);
        {
            let mut s = st.borrow_mut();
            s.cursor = Cursor::Row;
            s.draft = false;
            s.new_by_call = false;
            s.dirty = false;
            s.refresh_from_code_inserted_line = true;
        }
        if !save {
            self.set_xrec(st);
        }
        let key = self.row_key(&self.page_rec(st))?;
        let calls = st.borrow().test_calls.len();
        st.borrow_mut().code_inserted_row = Some((key, calls));
        Ok(())
    }

    /// Whether the refresh about to run is the one `land_on_code_inserted_line` asked for: `asked` (the mark it left, taken by
    /// every refresh), a refresh of `CurrPage.Update` on the open page standing on the landed row.
    pub(super) fn code_inserted_refresh_due(&mut self, st: &St, asked: bool, by_update: bool) -> Result<bool, Signal> {
        if !asked || !by_update || !st.borrow().open || st.borrow().cursor != Cursor::Row {
            return Ok(false);
        }
        let key = self.row_key(&self.page_rec(st))?;
        Ok(st
            .borrow()
            .code_inserted_row
            .as_ref()
            .is_some_and(|(landed, _)| Self::same_key(landed, &key)))
    }

    /// `Previous()` on the row that refresh landed on, when it is the only row of the view and the test only read and wrote
    /// controls since: BC answers false, runs no trigger and keeps an edit typed since unsaved (ListNewLineCodeInsertUpdate.al:455
    /// `prev` steps of the empty views: `F/X` and an empty log; `Next()` then saves it). `None` for every other move, which also
    /// ends the mark.
    pub(super) fn previous_on_code_inserted_row(&mut self, st: &St, how: PageMethod) -> Result<Option<bool>, Signal> {
        let Some((landed, calls)) = st.borrow_mut().code_inserted_row.take() else {
            return Ok(None);
        };
        if how != PageMethod::Previous || st.borrow().cursor != Cursor::Row {
            return Ok(None);
        }
        // The calls since the landing SetValue: control reads and writes, then this Previous.
        let only_fields = Self::test_calls(st).is_some_and(|c| {
            c.len() > calls
                && c[calls..c.len() - 1]
                    .iter()
                    .all(|t| matches!(t, TestCall::Read | TestCall::Write(_)))
                && c[c.len() - 1] == TestCall::Page(PageMethod::Previous)
        });
        let key = self.row_key(&self.page_rec(st))?;
        if !only_fields || !Self::same_key(&landed, &key) || self.view_row_count(st)? != 1 {
            return Ok(None);
        }
        Ok(Some(false))
    }
}
