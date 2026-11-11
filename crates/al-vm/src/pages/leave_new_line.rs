//! A move off the new-row line of a list that nothing was typed on.
//!
//! On a List or a ListPlus page of its own (not a part, not a tree) with OnAfterGetRecord and OnAfterGetCurrRecord and without
//! OnFindRecord and OnNextRecord, a move off the untouched new-row line `New()` started reads the row it lands on twice and then
//! runs OnAfterGetCurrRecord; it reads no other row (testpage/NewRowThenMove.al, al-runner issue 7452):
//! - First: the first row (:540 List_TwoStoredRows_OpenEdit_New_First and :663 ListPlus_TwoStoredRows_OpenEdit_New_First
//!   `log after FIRST` = `R:S1 R:S1 C:S1`; :444 List_TypedRow_New_First and :581 ListPlus_TypedRow_New_First on a list opened with
//!   OpenNew whose typed row was saved), also when CurrPage.Update(true) in a control's OnValidate saved what was typed on the
//!   line (:639 ListPlus_TypedRow_New_SecondRowTyped_First). Over an empty view the line is started again (OnNewRecord,
//!   OnAfterGetCurrRecord) and First answers No (:562 List_EmptyTable_OpenEdit_New_First `N C: `).
//! - Previous: the last row (:480 List_TypedRow_New_Previous, :617 ListPlus_TypedRow_New_Previous).
//! - Last: the line is reached twice before the client settles on the last row (:462 List_TypedRow_New_Last, :599
//!   ListPlus_TypedRow_New_Last `N C: N C: R:FORMULA1 R:FORMULA1 C:FORMULA1`).
//! - GoToKey onto a stored row (:516 List_TypedRow_New_GoToKey).
//!
//! Only these shapes are pinned: List or ListPlus with both triggers, `New()` started below the last row of the view (or on an
//! empty List opened for edit), the row counts and modes named above. Every other move of this kind (First, Previous, Last or
//! GoToKey off a line `New()` started on a list with OnAfterGetRecord or OnAfterGetCurrRecord) is refused before it changes
//! anything: the old moves read other rows or none (al-runner issue 7452). Next keeps the moves of `rows.rs`. Also here: Previous
//! off the new-row line of a list opened on a moved Rec (see `previous_off_new_line_onto_held_row`).

use super::*;
use al_builtins::page_op::PageMethod;
use al_builtins::record_op::RecordOp;

impl Session {
    /// Whether `how` from where `st` stands leaves the line `New()` started, on a list whose reads are seen (OnAfterGetRecord or
    /// OnAfterGetCurrRecord, no OnFindRecord or OnNextRecord, not a part or a tree): `Some(true)` for a pinned move (the module
    /// comment), `Some(false)` for any other move of that kind, which the caller refuses before it changes anything; `None` when
    /// `st` is not on such a line or `how` is Next (Next keeps the moves of `rows.rs`, testpage/NewRowThenMove.al:498).
    pub(super) fn off_new_line_pinned(&mut self, st: &St, how: PageMethod) -> Result<Option<bool>, Signal> {
        let (untouched, saved, mode, anchor, by_update) = {
            let s = st.borrow();
            let untouched = s.cursor == Cursor::NewLine
                && s.draft
                && !s.placeholder_line
                && (s.after_new.is_some() || s.mode != PageMode::New);
            // The line the page's code saved (CurrPage.Update(true) in a control's OnValidate), before the next move.
            let saved = how == PageMethod::First && s.cursor == Cursor::Row && !s.draft && s.after_new.is_some();
            let line = (untouched || saved)
                && !s.dirty
                && s.host.is_none()
                && !s.opened_on_moved_rec
                && !s.refreshed_after_gone_move;
            if !line {
                return Ok(None);
            }
            (untouched, saved, s.mode, s.after_new.clone(), s.after_new_by_update)
        };
        let (agr, agcr) = (
            self.declares(st, "onaftergetrecord"),
            self.declares(st, "onaftergetcurrrecord"),
        );
        if how == PageMethod::Next
            || !self.is_list(st)
            || !(agr || agcr)
            || self.declares(st, "onfindrecord")
            || self.declares(st, "onnextrecord")
            || self.tree_indentation(st).is_some()
        {
            return Ok(None);
        }
        let page_type = self.engine.object_facts[st.borrow().object as usize].page_type.clone();
        let (list, list_plus) = (
            matches!(page_type, Some(al_ir::PageType::List)),
            matches!(page_type, Some(al_ir::PageType::ListPlus)),
        );
        if !(agr && agcr && (list || list_plus)) {
            return Ok(Some(false));
        }
        let rows = self.view_row_count(st)?;
        let below = |s: &mut Self, which: &str| -> Result<bool, Signal> {
            let Some(anchor) = &anchor else { return Ok(false) };
            let probe = Rc::new(RefCell::new(s.page_rec(st).borrow().clone()));
            s.ret_used = true;
            let op = if which == "-" {
                RecordOp::FindFirst
            } else {
                RecordOp::FindLast
            };
            if !matches!(s.record_op(op, &probe, &mut [])?, Value::Boolean(true)) {
                return Ok(false);
            }
            let table = s.table(&probe)?;
            Ok(Self::same_key(
                &table.primary_key(&probe.borrow().fields),
                &table.primary_key(anchor),
            ))
        };
        // The pins open the page with OpenEdit or OpenNew; OpenView and every other mode are not pinned.
        let (edit, new) = (mode == PageMode::Edit, mode == PageMode::New);
        // The one row of the OpenNew pins was typed and saved by CurrPage.Update(true) in the OnValidate of the control typed last,
        // before New() (:407 TypeOneRowThenNewList, :419 TypeOneRowThenNewPlus; :73, :171 the OnValidate); a row New() saves
        // itself is not pinned.
        let one_saved = new && by_update && rows == 1;
        let pinned = match how {
            // :540, :663 OpenEdit, Last, New on two rows; :444, :581 OpenNew with one saved row; :562 OpenEdit on an empty List.
            PageMethod::First if untouched => {
                (edit && rows == 2 && below(self, "+")?)
                    || (one_saved && below(self, "+")?)
                    || (edit && rows == 0 && list && anchor.is_none())
            }
            // :639 the second typed row saved on a ListPlus opened with OpenNew, New() started below the first.
            PageMethod::First => saved && list_plus && new && rows == 2 && below(self, "-")?,
            // :480, :617, :462, :599 OpenNew with one saved row.
            PageMethod::Previous | PageMethod::Last => one_saved && below(self, "+")?,
            // :516 OpenNew with one saved row, a List.
            PageMethod::GoToKey => list && one_saved && below(self, "+")?,
            _ => false,
        };
        Ok(Some(pinned))
    }

    /// The refusal of a move off the line `New()` started that `off_new_line_pinned` does not pin.
    pub(super) fn refuse_unpinned_new_line_move(how: PageMethod) -> Result<Option<bool>, Signal> {
        unsupported(format!(
            "{how}() off the new-row line New() started, on a list with OnAfterGetRecord or OnAfterGetCurrRecord, in a shape the \
             corpus does not pin (page type, row count, mode or the row New() started below; which rows it reads is not pinned)"
        ))
    }

    /// First, Previous or Last off the line `New()` started (see the module comment); `None` where `off_new_line_pinned` says so.
    pub(super) fn move_off_untouched_new_line(&mut self, st: &St, how: PageMethod) -> Result<Option<bool>, Signal> {
        let which = match (how, self.off_new_line_pinned(st, how)?) {
            (_, None) => return Ok(None),
            (_, Some(false)) => return Self::refuse_unpinned_new_line_move(how),
            (PageMethod::First, Some(true)) => "-",
            (_, Some(true)) => "+",
        };
        st.borrow_mut().after_new = None;
        if how == PageMethod::Last {
            self.land_on_new_line(st)?;
            self.land_on_new_line(st)?;
        }
        if self.nav_find(st, which)? {
            self.read_row_off_new_line(st)?;
            return Ok(Some(true));
        }
        self.land_on_new_line(st)?;
        Ok(Some(false))
    }

    /// GoToKey off the untouched line `New()` started, with `pinned` the answer of `off_new_line_pinned` before the GoToKey's
    /// save: onto the pinned row it reads as the module comment says; any other key or shape is refused; `None` off such a line.
    pub(super) fn go_to_key_off_untouched_new_line(
        &mut self,
        st: &St,
        pinned: Option<bool>,
        want: &[Value],
    ) -> Result<Option<bool>, Signal> {
        match pinned {
            None => return Ok(None),
            Some(false) => return Self::refuse_unpinned_new_line_move(PageMethod::GoToKey),
            Some(true) => {}
        }
        let rec = self.page_rec(st);
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        {
            let table = self.table(&rec)?;
            let mut p = probe.borrow_mut();
            for (&f, v) in table.keys[0].iter().zip(want) {
                p.fields[f] = v.clone();
            }
        }
        self.ret_used = true;
        if !matches!(
            self.record_op(RecordOp::Find, &probe, &mut [Value::Text("=".into())])?,
            Value::Boolean(true)
        ) {
            return Self::refuse_unpinned_new_line_move(PageMethod::GoToKey);
        }
        st.borrow_mut().after_new = None;
        let found = probe.borrow().fields.clone();
        rec.borrow_mut().fields = found;
        self.read_row_off_new_line(st)?;
        Ok(Some(true))
    }

    /// The row Rec landed on is read twice, then OnAfterGetCurrRecord runs. The rows the client holds stay as they are.
    fn read_row_off_new_line(&mut self, st: &St) -> Result<(), Signal> {
        let before = self.xrec_before_move(st);
        {
            let mut s = st.borrow_mut();
            s.cursor = Cursor::Row;
            s.draft = false;
            s.stale = false;
        }
        self.populate_current(st)?;
        self.populate_current(st)?;
        self.current_record_trigger(st, before)
    }

    /// Previous off the new-row line of a list opened on the row its OnOpenPage left Rec on goes back to the last row it holds as a
    /// move between its rows does: the row read twice, then OnAfterGetCurrRecord (corpus
    /// pages/onopenpage-moves-rec/QrMovedTests.al:585 QR_TempList_PageRun_NoMove `previous log` = `R:D;R:D;C:D;`, :615
    /// QR_List_RunModal_FindLast). `Some(true)` when it did; other lists keep the earlier move. Rec stands on the last row of the
    /// view (`found` by the caller's find).
    pub(super) fn previous_off_new_line_onto_held_row(&mut self, st: &St, found: bool) -> Result<Option<bool>, Signal> {
        if !(found
            && (st.borrow().opened_on_moved_rec || st.borrow().refreshed_after_gone_move)
            && st.borrow().host.is_none())
        {
            return Ok(None);
        }
        let key = self.row_key(&self.page_rec(st))?;
        let last = st.borrow().window.len().checked_sub(1);
        let Some(at) = last.filter(|&at| Self::same_key(&st.borrow().window[at].key, &key)) else {
            return Ok(None);
        };
        {
            let mut s = st.borrow_mut();
            s.cursor = Cursor::Row;
            s.draft = false;
            s.stale = false;
            s.refreshed_after_gone_move = false;
        }
        self.read_client_row(st, at)?;
        Ok(Some(true))
    }
}
