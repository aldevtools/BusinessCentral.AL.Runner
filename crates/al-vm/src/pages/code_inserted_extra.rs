//! Code that is not a pinned page's around `CurrPage.SaveRecord()` and `CurrPage.Update` on the new line its table field
//! inserted, on a List page without page triggers and parts (al-runner issues 8712 and 8713). The cells of
//! `code_inserted_new_line.rs` run only the measured code, line for line (`code_inserted_pinned.rs`); on this page the gate is
//! what the code did to the store, the row and Rec instead, as ListNewLineExtraCode.al (corpus pull request 1521, open, 11 of 11
//! green on BC 29.1 in 3 runs) measures it, under the assumption `list-new-line-extra-code`:
//! - the page's table declares no OnInsert, OnModify, OnDelete or OnRename trigger (:18-42, :101-125, :170-198), so the save
//!   runs no table code; before the save nothing but the table field's insert wrote the store; the save then writes Rec with
//!   every field it holds,
//!   set by the control (:400, :413) or by the table field (:453), also through `CurrPage.Update(true)` (:464);
//! - between the save and `CurrPage.Update(false)` the code modified the row through another record (:341, :354), or set
//!   a field of Rec (:486), or wrote another table (:497), and `Update(false)` reads the row as stored: the page shows the other
//!   record's write and drops Rec's (`extra_code_kept`). Every other row of the page's table is as the save left it, and so is
//!   Rec's view (`view_kept`).
//!
//! Measured but refused: a modify of the row through another record before the save (:442, the save writes Rec over it) and
//! `Update(true)` after `SaveRecord` (:475). The cell under TestPage.OpenEdit with a filter (:371) keeps main's way: no pinned
//! open has a filter (`code_inserted_open`). Every other open, view and page keeps main's refusal (`UNMEASURED_CODE`).

use super::code_inserted_new_line::{Leave, Opened, UNMEASURED_CODE, View};
use super::state::SavedView;
use super::*;
use al_builtins::assumptions::{Assumption, assume};

/// A step of the effect gate runs under its assumption.
pub(super) fn extra_code_assume() {
    assume(Assumption::ListNewLineExtraCode);
}

impl Session {
    /// The SaveRecord or Update(true) cell `leave` on page `st` without triggers, opened as `opened` with the line in `view`, runs
    /// code that is not a pinned page's. It runs under the effect gate when the open and the view are measured (Page.RunModal on
    /// one group, empty or with a row before, :341, :354, :400, :413; Update(true) on an empty group, :464), its table declares no
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
    /// or the saves after it (the cells' tables, :18-42, :101-125, :170-198).
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
    /// through another record only (:341), or Rec only (:486); `CurrPage.Update(false)` then reads the stored row.
    pub(super) fn extra_code_kept(rec: &[Value], stored: &[Value], saved: &[Value]) -> bool {
        Self::same_values(rec, saved) || Self::same_values(stored, saved)
    }
}
