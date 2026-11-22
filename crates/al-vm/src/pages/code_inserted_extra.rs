//! Code that is not a pinned page's around `CurrPage.SaveRecord()` and `CurrPage.Update` on the new line its table field
//! inserted, on a List page without page triggers and parts (al-runner issues 8712 and 8713). The cells of
//! `code_inserted_new_line.rs` run only the measured code, line for line (`code_inserted_pinned.rs`); on this page the gate is
//! what the code did to the store, the row and Rec instead, as ListNewLineExtraCode.al (corpus pull request 1521, open, 14 of 14
//! green on BC 29.1 in 3 runs) measures it, under the assumption `list-new-line-extra-code`:
//! - the page's table declares no OnInsert, OnModify, OnDelete or OnRename trigger (:19-43, :102-126, :171-199), so a save runs
//!   no table code;
//! - before the save nothing but the table field's insert wrote the store; the save then writes Rec with every field it holds,
//!   set by the control (:401, :414) or by the table field (:454, :512), also through `CurrPage.Update(true)` (:465);
//! - between the save and `CurrPage.Update(false)` the code modified the row through another record (:342, :355), or set a
//!   field of Rec (:487, :524), or wrote another table (:498, :536), and `Update(false)` reads the row as stored: the page
//!   shows the other record's write and drops Rec's (`extra_code_kept`). Every other row of the page's table is as the save
//!   left it, and so is Rec's view (`view_kept`);
//! - the Update is the validation's last statement: Rec, the table's rows and the store's write mark stay as they were at the
//!   Update until the validation ends (`extra_code_unchanged`).
//!
//! - `CurrPage.Update(true)` after the save writes Rec again, over what another record wrote since (:476), then refreshes as
//!   `Update(false)` does (`extra_code_update_true`).
//!
//! Measured but refused: a modify of the row through another record before the save (:443, the save writes Rec over it) and
//! the cell under TestPage.OpenEdit with a filter (:372), which keeps main's way: no pinned open has a filter
//! (`code_inserted_open`). Every other open, view and page keeps main's refusal (`UNMEASURED_CODE`).

use super::code_inserted_new_line::{Leave, Opened, UNMEASURED_CODE, View};
use super::state::{CodeInsertedSave, SavedView};
use super::*;
use al_builtins::assumptions::{Assumption, assume};

/// A step of the effect gate runs under its assumption.
pub(super) fn extra_code_assume() {
    assume(Assumption::ListNewLineExtraCode);
}

impl Session {
    /// The SaveRecord or Update(true) cell `leave` on page `st` without triggers, opened as `opened` with the line in `view`, runs
    /// code that is not a pinned page's. It runs under the effect gate when the open and the view are measured (Page.RunModal on
    /// one group, empty or with one row before the line, each effect measured both ways, :342, :355, :401, :414, :454, :512,
    /// :487, :524, :498, :536; Update(true) on an empty group, :465), its table declares no
    /// record trigger (`no_record_trigger`), and nothing but the table field's insert wrote the store since the line began;
    /// otherwise refused.
    pub(super) fn extra_code_cell(
        &mut self,
        st: &St,
        opened: Opened,
        view: Option<View>,
        leave: Leave,
    ) -> Result<(), Signal> {
        let measured = matches!(
            (leave, opened, view),
            (
                Leave::SaveRecord,
                Opened::RunModalOneGroup,
                Some(View::Alone | View::AfterRow)
            ) | (Leave::UpdateTrue, Opened::RunModalOneGroup, Some(View::Alone))
        );
        let line = st.borrow().new_line_writes;
        let only_the_insert = line.is_some_and(|(mark, deletes)| self.write_mark() == (mark + 1, deletes));
        if !(measured && only_the_insert && self.no_record_trigger(st)) {
            return Err(Signal::unsupported(UNMEASURED_CODE));
        }
        extra_code_assume();
        st.borrow_mut().code_inserted_extra = true;
        Ok(())
    }

    /// Whether the table of page `st` declares none of OnInsert, OnModify, OnDelete and OnRename: no table code runs at the save
    /// or the saves after it (the cells' tables, :19-43, :102-126, :171-199).
    fn no_record_trigger(&self, st: &St) -> bool {
        let table = self.page_rec(st).borrow().table as usize;
        let engine = self.engine();
        !engine.program.objects[table].routines.iter().any(|r| {
            r.is_trigger
                && r.field.is_none()
                && ["oninsert", "onmodify", "ondelete", "onrename"]
                    .iter()
                    .any(|n| r.name.eq_ignore_ascii_case(n))
        })
    }

    /// `CurrPage.Update(true)` after the save, when the code since kept what `Update(false)` needs kept (`view_kept`): the page
    /// saves Rec again, over the row another record wrote (:476 `A//V`), and asks the refresh; the save after it notes a new
    /// view (`code_inserted_saved`). Otherwise not measured, and refused.
    pub(super) fn extra_code_update_true(&mut self, st: &St, view: &SavedView) -> Result<(), Signal> {
        if !self.view_kept(st, view)? {
            st.borrow_mut().forget_code_inserted_line();
            return unsupported(
                "the page's record, its view or the table changed between CurrPage.SaveRecord, which moved the page from its new \
                 line onto the row the table field inserted, and the CurrPage.Update(true) after it (not measured)",
            );
        }
        extra_code_assume();
        // The page writes Rec over the row as stored (:476); the table has no OnModify (`no_record_trigger`), so the write is
        // the page's whole save. Rec and xRec are then the same, and the save that follows writes nothing more.
        let rec = self.page_rec(st);
        if self
            .stored_row(&rec)?
            .is_some_and(|stored| !Self::same_values(&rec.borrow().fields, &stored))
        {
            self.record_op(al_builtins::record_op::RecordOp::Modify, &rec, &mut [])?;
        }
        {
            let mut s = st.borrow_mut();
            s.code_inserted_save = CodeInsertedSave::Pending { asked: true };
            s.code_inserted_extra = true;
        }
        self.ask_code_inserted_refresh(st)
    }

    /// Every stored row of the page's table after the save, which the code before `CurrPage.Update(false)` must leave as it is
    /// but the row itself (`view_kept`); past 1000 rows not measured, and refused.
    pub(super) fn extra_code_rows(&mut self, st: &St) -> Result<Vec<Vec<Value>>, Signal> {
        match self.coded_rows(st)? {
            Some(rows) => Ok(rows),
            None => unsupported(
                "a List page with extra code around CurrPage.SaveRecord over a table of more than 1000 rows (not measured)",
            ),
        }
    }

    /// What the page, the table and the store hold at `CurrPage.Update(false)` after the save: Rec, every stored row of the page's
    /// table and the write mark, which the validation must leave as they are (`extra_code_unchanged`). In every cell the Update
    /// is the last statement (review of #8720, point 1).
    pub(super) fn extra_code_at_update(&mut self, st: &St) -> Result<SavedView, Signal> {
        Ok(SavedView {
            rec: self.page_rec(st).borrow().clone(),
            writes: self.write_mark(),
            rows: Some(self.extra_code_rows(st)?),
            extra: true,
        })
    }

    /// Whether the validation ended with Rec, every stored row of the page's table, the store's write mark and Rec's view as
    /// `at` noted them at the `CurrPage.Update` (`extra_code_at_update`, or the save of `Update(true)`, `code_inserted_saved`):
    /// code after the Update is not measured (a modify through another record, a field set; review of #8720, point 1).
    pub(super) fn extra_code_unchanged(&mut self, st: &St, at: &SavedView) -> Result<bool, Signal> {
        let rec = self.page_rec(st);
        let rows = self.coded_rows(st)?;
        let same_rows = match (rows, &at.rows) {
            (Some(now), Some(then)) => {
                now.len() == then.len() && now.iter().zip(then).all(|(a, b)| Self::same_values(a, b))
            }
            _ => false,
        };
        let unchanged =
            self.write_mark() == at.writes && same_rows && Self::same_values(&rec.borrow().fields, &at.rec.fields);
        Ok(unchanged && self.view_kept(st, at)?)
    }

    /// Whether Rec's fields `rec` and the stored row `stored` changed since the save left both as `saved` as measured: the row
    /// through another record only (:342), or Rec only (:487); `CurrPage.Update(false)` then reads the stored row.
    pub(super) fn extra_code_kept(rec: &[Value], stored: &[Value], saved: &[Value]) -> bool {
        Self::same_values(rec, saved) || Self::same_values(stored, saved)
    }
}
