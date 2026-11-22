//! The page with code of al-runner issue 8434: page 97492 or 97493 of ListNewLineFactboxesCode.al (corpus pull request 1454,
//! merged, not pinned yet), with its table, its three CardPart FactBoxes (one enqueues a page background task), the tables
//! they read and the codeunits they call, all line for line (`code_inserted_pinned.rs` `coded_code`). The probe measures it on
//! Business Central 29.1 (11 of 11 green), under the assumption `list-new-line-coded-factboxes` until a bump pins it. Opened
//! by a Page.RunModal statement on a record filtered to one group, on an empty view (:1401-1476):
//! - the open runs the parts' OnAfterGetCurrRecord on the blank record (`open=P.OPEN;W.AGCR();`), `coded_open_on_empty_view`;
//! - the first SetValue after `New()` runs it again, before the validation (`setname=W.AGCR();T.VN(Station);...`),
//!   `coded_line_before_set_value`;
//! - the validation lands on the row as on the FactBox host (`code_inserted_cell`), and its refresh reads the row twice in a
//!   secondary key and once in the primary key, then the parts (`P.AGR(Station/);P.AGR(Station/);R.AGR(Station/0);
//!   W.AGCR(Station/s);W.DONE(K2);`, :1407 against :1445), `coded_refresh`;
//! - up to two later SetValues whose validation saves the row and calls `CurrPage.Update(false)` refresh the same way
//!   (:1407 `setvalue`, :1539 the value already there, :1576 a second one), then `OK()` runs nothing (`ok=`).
//!
//! Any other page, or other code, keeps main's way (`coded_page`). The table may have manual subscribers nobody binds
//! (:255-274, :1592 binds one: refused). A row before the line (:1477), an error in the validation (:1499), a Value before any
//! Name (:1553) and every other call are refused.

use super::pinned_shape::TestCall;
use super::state::{CodedLine, FactboxStep};
use super::*;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::page_op::PageMethod;

impl Session {
    /// Whether page `st` is the page with code: the probe's objects line for line (`coded_code`), shown as the probe shows
    /// them: Rec stored; no subscriber to the page or its table but manual ones (:255-274; whether one is bound matters only
    /// from the landing on, where `no_active_subscriber` refuses it, :1592); its three parts shown, each over the host's
    /// table, stored, not marked and filtered by its link only (`coded_part`).
    pub(super) fn coded_page(&self, st: &St) -> Result<bool, Signal> {
        let manual_only = |object: usize| self.engine().events.manual_subscribers_of(object).is_some();
        let rec = self.page_rec(st);
        let table = rec.borrow().table as usize;
        let parts = st.borrow().parts.clone();
        if !(self.has_source(st)
            && !rec.borrow().temporary
            && manual_only(st.borrow().object as usize)
            && manual_only(table)
            && st.borrow().hidden_parts.is_empty()
            && parts.len() == 3)
        {
            return Ok(false);
        }
        for (_, part) in &parts {
            if !self.coded_part(st, part)? {
                return Ok(false);
            }
        }
        Ok(self.coded_code(st))
    }

    /// Whether part `part` of host `st` shows its rows as the probe's parts do: over the host's table, stored, not marked,
    /// filtered by its link on the primary key only, with no page extension and no subscriber.
    fn coded_part(&self, st: &St, part: &St) -> Result<bool, Signal> {
        if !self.has_source(part) || !self.unextended_unsubscribed(part) {
            return Ok(false);
        }
        let (host_rec, part_rec) = (self.page_rec(st), self.page_rec(part));
        let key = self.table(&host_rec)?.keys[0].clone();
        let (h, p) = (host_rec.borrow(), part_rec.borrow());
        Ok(h.table == p.table
            && !p.temporary
            && !p.marked_only
            && p.filters
                .iter()
                .all(|f| f.group == super::parts::LINK_GROUP && key.contains(&f.field)))
    }

    /// Whether every subscriber to an event of table object `table` is a manual one with no instance bound now (the probe's
    /// :255-275, which its tests do not bind but :1592).
    pub(super) fn no_active_subscriber(&self, table: usize) -> bool {
        let engine = self.engine();
        let subscribers = engine.events.manual_subscribers_of(table);
        drop(engine);
        subscribers.is_some_and(|objects| objects.iter().all(|&o| !self.is_bound_object(o)))
    }

    /// The parts of host `st` as it opens. On the page with code opened as measured on an empty view, they run
    /// OnAfterGetCurrRecord on the blank record (:1407 `open=P.OPEN;W.AGCR();`) and the steps of the page with code begin;
    /// `false` for every other page and open.
    pub(super) fn coded_open_on_empty_view(&mut self, st: &St) -> Result<bool, Signal> {
        let line = {
            let s = st.borrow();
            s.cursor == Cursor::NewLine && s.draft && !s.dirty && !s.draft_stored && s.host.is_none()
        };
        if !(line
            && self.code_inserted_coded_open(st)?
            && self.code_inserted_coded_kind(st)?
            && self.view_row_count(st)? == 0)
        {
            return Ok(false);
        }
        assume(Assumption::ListNewLineCodedFactboxes);
        st.borrow_mut().coded_line = CodedLine::EmptyOpen;
        self.with_blank_landing(st, |s| s.sync_parts_at_open(st))?;
        Ok(true)
    }

    /// The test's `call` on host `st`, before it runs. After the open of `coded_open_on_empty_view`, the first SetValue of a
    /// control of the host after `New()` lets the parts read the new line first, OnAfterGetCurrRecord on the blank record (:1407
    /// `setname=W.AGCR();T.VN(Station);...`), and the validation must then land (`coded_must_have_landed`); any other call
    /// ends the steps.
    pub(super) fn coded_line_before_set_value(&mut self, st: &St, call: Option<TestCall>) -> Result<(), Signal> {
        if st.borrow().coded_line != CodedLine::EmptyOpen {
            return Ok(());
        }
        let Some(call) = call else {
            return Ok(());
        };
        let first_write = matches!(call, TestCall::Write(_))
            && matches!(
                Self::test_calls(st).as_deref(),
                Some([TestCall::Page(PageMethod::New), TestCall::Write(_)])
            )
            && {
                let s = st.borrow();
                s.cursor == Cursor::NewLine && s.draft && !s.dirty && !s.draft_stored && s.new_by_call
            };
        match call {
            _ if first_write => {
                assume(Assumption::ListNewLineCodedFactboxes);
                st.borrow_mut().coded_line = CodedLine::LineRead;
                self.with_blank_landing(st, |s| s.sync_parts(st))
            }
            TestCall::Read | TestCall::Page(PageMethod::New) => Ok(()),
            _ => {
                st.borrow_mut().coded_line = CodedLine::Off;
                Ok(())
            }
        }
    }

    /// Whether the validation of the SetValue that let the parts read the new line has landed on its row: before the landing an
    /// error (:1499) or a validation that does not land (:1553, Value before any Name) is not measured, and refused.
    pub(super) fn coded_must_have_landed(&self, st: &St) -> Result<(), Signal> {
        if st.borrow().coded_line != CodedLine::LineRead {
            return Ok(());
        }
        st.borrow_mut().forget_code_inserted_line();
        unsupported(
            "the validation of the first SetValue on a List page with code and FactBoxes, whose parts read its new line, did not \
             leave the line for the row its table field inserted as measured (not measured)",
        )
    }

    /// A SetValue on the tree of host `st` ended with `result`. When it was the first SetValue that let the parts read the new
    /// line (`CodedLine::LineRead`), or a SetValue on the landed row (`FactboxStep::CodedTyped`), and it raised an error before
    /// or in its validation, or ended without the landing or the refresh it must ask for, it is not measured, and refused.
    pub(super) fn coded_after_set_value(&self, st: &St, result: Result<Value, Signal>) -> Result<Value, Signal> {
        let (root, _) = Self::tree_root(st);
        let pending = {
            let r = root.borrow();
            r.coded_line == CodedLine::LineRead
                || matches!(
                    r.code_inserted_row.as_ref().map(|c| c.factbox),
                    Some(FactboxStep::CodedTyped { .. })
                )
        };
        match result {
            Err(Signal::Error(_)) | Ok(_) if pending => {
                root.borrow_mut().forget_code_inserted_line();
                unsupported(
                    "a SetValue on a List page with code and FactBoxes, on its new line or on the row it became, ended without the \
                     landing or the save and update the probe measures (not measured)",
                )
            }
            r => r,
        }
    }

    /// Whether control `idx` of host `st` takes a SetValue on the row its new line became (ListNewLineFactboxesCode.al:1407
    /// `setvalue`, :1576 `setvalue2`): a field control over Rec outside the primary key, with a TableRelation and an OnValidate
    /// of its own, other than the control whose SetValue landed.
    pub(super) fn coded_value_control(&self, st: &St, idx: usize) -> bool {
        let landing = match Self::test_calls(st).as_deref() {
            Some([TestCall::Page(PageMethod::New), TestCall::Write(landing), ..]) => *landing,
            _ => return false,
        };
        let engine = self.engine();
        let Some(control) = page_of(&engine, st).controls.get(idx) else {
            return false;
        };
        let ir::Binding::Field(f) = control.binding else {
            return false;
        };
        idx != landing
            && control.trigger("onvalidate").is_some()
            && control.props.iter().any(|(k, _)| k == "tablerelation")
            && self.table(&self.page_rec(st)).is_ok_and(|t| !t.keys[0].contains(&f))
    }

    /// The refresh of the page with code after its new line landed, or after a later SetValue saved the row
    /// (`code_inserted_refresh`): the row once more first in a secondary key (:1407 `P.AGR(Station/);P.AGR(Station/);`, :1445
    /// once in the primary key), then as on the FactBox host. The page's and the parts' triggers leave the rows of the page's
    /// table, Rec as stored and Rec's view as they were; otherwise not measured, and refused.
    pub(super) fn coded_refresh(&mut self, st: &St, without_curr: bool, without_parts: bool) -> Result<(), Signal> {
        let rows = self.coded_rows(st)?;
        let view = self.page_rec(st).borrow().clone();
        let primary_key = {
            let rec = self.page_rec(st);
            let r = rec.borrow();
            r.key == 0 && r.adhoc.is_none()
        };
        if !primary_key {
            self.populate_current(st)?;
        }
        self.reread_view_from_row(st, without_curr, without_parts, false)?;
        // The refresh's reads are its own: on this page without OnAfterGetCurrRecord the mark they leave for the refresh
        // (`note_reads_done`) is spent here, so it does not skip the refresh of the next SetValue (:1407 `setvalue`). Only here,
        // under the assumption: what the mark does on the other callers of `reread_view_from_row` is not measured (review of
        // al-runner PR 8463, round 1, finding 3).
        st.borrow_mut().refresh.spend_reads_done();
        let rec = self.page_rec(st);
        let stored = self.stored_row(&rec)?;
        let rows_kept = match (rows, self.coded_rows(st)?) {
            (Some(a), Some(b)) => a.len() == b.len() && a.iter().zip(&b).all(|(x, y)| Self::same_values(x, y)),
            _ => false,
        };
        let view_kept = {
            let r = rec.borrow();
            crate::records::same_filters(&r.filters, &view.filters)
                && r.filter_group == view.filter_group
                && r.key == view.key
                && r.adhoc == view.adhoc
                && r.ascending == view.ascending
                && r.field_descending == view.field_descending
                && r.marked_only == view.marked_only
        };
        let kept = rows_kept && view_kept && stored.is_some_and(|s| Self::same_values(&rec.borrow().fields, &s));
        if !kept {
            st.borrow_mut().forget_code_inserted_line();
            return unsupported(
                "a trigger of a List page with code and FactBoxes, or of its parts, wrote or changed Rec when the refresh read the \
                 row its new line became (not measured)",
            );
        }
        let steps = match st.borrow().code_inserted_row.as_ref().map(|r| r.factbox) {
            Some(FactboxStep::CodedTyped { steps }) => steps + 1,
            _ => 0,
        };
        if let Some(row) = st.borrow_mut().code_inserted_row.as_mut() {
            row.factbox = FactboxStep::Coded { steps };
        }
        Ok(())
    }

    /// Every stored row of the table of the page's record, in the primary key: what the code between the save and the update
    /// must leave as it is on the page with code (`SavedView::rows`). `None` past 1000 rows.
    pub(super) fn coded_rows(&mut self, st: &St) -> Result<Option<Vec<Vec<Value>>>, Signal> {
        let probe = Rc::new(RefCell::new(self.page_rec(st).borrow().clone()));
        {
            let mut p = probe.borrow_mut();
            p.filters.clear();
            p.marks.clear();
            p.marked_only = false;
            p.key = 0;
            p.ascending = true;
            p.adhoc = None;
            p.field_descending = 0;
        }
        let rows = self.view_rows(&probe, 1001)?;
        Ok((rows.len() <= 1000).then_some(rows))
    }
}
