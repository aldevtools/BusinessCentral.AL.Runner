//! A List with the new-row line, opened for edit, that lost its only row to an action stands on its blank line (the action
//! raised OnNewRecord and OnAfterGetCurrRecord three times, `settle_after_action`). What the moves and a later action do there
//! (corpus master pages/deleted-rows/Y80GoneRows.al, codeunit 94230):
//! - First, Next, Last and Previous in a row answer No each and log `N:0;C:0;N:0;C:0` in all (Y80_Gone_All_OpenEdit_DeleteAllUpdate,
//!   _DeleteAll, _DeleteCurrent `moves log`). First raises nothing (testpage/F3ListTests.al:189 DeleteOnlyRow_WithoutUpdate
//!   `log_after_first`, testpage/PnGRefresh.al ListLosesOnlyRowDeleteNoUpdate, lt-outside-worksheet/
//!   RmWorksheetGoneRow.al:436 `close log`). As a single move (pages/deleted-rows-navigation/Y89GoneSingleMove.al
//!   Y89_All_OpenEdit_delall{,upd}_{first,next,previous,last,goto}) Last raises the pair twice and First, Next, Previous and GoToKey
//!   raise nothing; every one answers No.
//! - A second action that leaves the table empty again raises the pair twice (`second action log`).

use super::*;
use al_builtins::page_op::PageMethod;
use al_builtins::record_op::RecordOp;

impl Session {
    /// Whether the page stands on the blank line of a list that lost its only row, with no row in the view.
    fn on_gone_blank_line(&mut self, st: &St) -> Result<bool, Signal> {
        let shape = {
            let s = st.borrow();
            s.open
                && s.gone_blank_shows_line
                && s.new_line
                && s.host.is_none()
                && matches!(s.cursor, Cursor::None | Cursor::NewLine)
                && s.mode == PageMode::Edit
        } && self.is_list(st)
            && !self.declares(st, "onfindrecord")
            && !self.declares(st, "onnextrecord");
        if !shape {
            return Ok(false);
        }
        let rec = self.page_rec(st);
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        self.ret_used = true;
        let rows = matches!(
            self.record_op(RecordOp::FindFirst, &probe, &mut [])?,
            Value::Boolean(true)
        );
        Ok(!rows)
    }

    /// A move on that list: `Some(answer)` when the page is in that state.
    pub(super) fn gone_blank_line_move(&mut self, st: &St, how: PageMethod) -> Result<Option<bool>, Signal> {
        if !self.on_gone_blank_line(st)? {
            return Ok(None);
        }
        if matches!(how, PageMethod::Last) {
            // Last raises the pair twice, Next and Previous raise nothing (Y89GoneSingleMove.al Y89_All_OpenEdit_delall{,upd}_last
            // `move log N:0;C:0;N:0;C:0`, _next and _previous `move log` empty); all answer No.
            st.borrow_mut().window.clear();
            self.land_on_new_line(st)?;
            self.land_on_new_line(st)?;
        }
        Ok(Some(false))
    }

    /// A second action on that list, before the page stands on the line again: the table is empty again.
    pub(super) fn gone_blank_line_second_action(&mut self, st: &St) -> Result<bool, Signal> {
        if st.borrow().cursor != Cursor::None || !self.on_gone_blank_line(st)? {
            return Ok(false);
        }
        st.borrow_mut().refresh = false;
        st.borrow_mut().window.clear();
        self.land_on_new_line(st)?;
        self.land_on_new_line(st)?;
        Ok(true)
    }

    /// A card whose row an action deleted moves to another row, then asks OnQueryClosePage before it runs OnClosePage. A card
    /// without OnQueryClosePage moves the same way: OnAfterGetRecord runs for the next row, else the row before, and OnClosePage
    /// sees Rec on it, or blank when no row is left (pages/deleted-rows-navigation/Y89GoneSingleMove.al
    /// Y89_NQ_{OpenEdit,OpenView}_On20_delcur `D:20/R:20;R:30;X:30`, _On30_delcur `D:30/R:30;R:20;X:20`, _On20_delall
    /// `R:20;X:0`, _Only_delcur `R:10;X:0`, Y89_NQ_Modal_* the same with `result` LookupCancel).
    pub(super) fn card_query_close_before_forced_close(&mut self, st: &St) -> Result<(), Signal> {
        // The card asks OnQueryClosePage first, then runs OnClosePage (corpus master pages/deleted-rows/Y80GoneRows.al
        // Y80_Gone_Card_OpenEdit_DeleteAllUpdate, _DeleteCurrent, _OpenView_* `action log R:10;D:10;Q;X`). It receives the action
        // of a close by the page itself: OK, LookupCancel when run modally in lookup mode (pages/deleted-rows-navigation/
        // Y89GoneSingleMove.al Y89_CA_True_{OpenEdit,OpenView}_* `Q:OK:..`, Y89_CA_Modal_True_* `Q:LookupCancel:..`). A veto is
        // not pinned to the end (the page stays open and asks again in shapes that are not modelled): refused.
        self.rec_after_forced_close(st)?;
        if self.declares(st, "onqueryclosepage") {
            let action = self.close_action(st, "Close");
            let members: std::sync::Arc<[String]> = al_builtins::value::ACTION_MEMBERS
                .iter()
                .map(|m| m.to_string())
                .collect();
            let at = al_builtins::value::ACTION_MEMBERS
                .iter()
                .position(|m| *m == action)
                .unwrap_or(0) as i32;
            if self.query_close_answer(st, vec![Value::Option(at, members)])? == Some(false) {
                return unsupported(
                    "a card whose OnQueryClosePage vetoes the close its deleted row forces (not pinned)",
                );
            }
        }
        Ok(())
    }

    /// The Rec of a card whose row is gone, as its OnQueryClosePage sees it: the next row of the view, else the nearest row
    /// before it, else a blank record (Y89GoneSingleMove.al Y89_CA_True_*_2Row_delcur `Q:OK:20`, _1Row_* and
    /// _2Row_delall{,upd} `Q:OK:0`; corpus pages/deleted-rows-navigation/Y89GoneSingleMove.al:4328 Y89_CA_True_*_3Row_delcur_last
    /// `D:30/Q:OK:20;X:20`). OnAfterGetRecord runs for the row the card moves to, before OnQueryClosePage, whether it lies
    /// after or before the deleted one (:4372 Y89_CA_Get_OpenEdit_3Row_delcur_last `D:30/G:30;G:20;Q:OK:20;X:20`, :4378
    /// _delcur_first `D:10/G:10;G:20;Q:OK:20;X:20`, :4420 Y89_CA_Modal_Get_3Row_DeleteCurrentLast; OnAfterGetCurrRecord does
    /// not run for it). A page that finds its own rows is not pinned and is refused.
    fn rec_after_forced_close(&mut self, st: &St) -> Result<(), Signal> {
        if self.declares(st, "onfindrecord") || self.declares(st, "onnextrecord") {
            return unsupported("a card that finds its own rows, closed by its deleted row (not pinned)");
        }
        let rec = self.page_rec(st);
        for which in [">", "<"] {
            let probe = Rc::new(RefCell::new(rec.borrow().clone()));
            self.ret_used = true;
            if matches!(
                self.record_op(RecordOp::Find, &probe, &mut [Value::Text(which.into())])?,
                Value::Boolean(true)
            ) {
                let found = probe.borrow().clone();
                *rec.borrow_mut() = found;
                st.borrow_mut().gone_closed_on_row = true;
                return self.populate_current(st);
            }
        }
        let table = rec.borrow().table as usize;
        let fresh = self.initial_fields(table);
        let mut r = rec.borrow_mut();
        r.fields = fresh;
        r.position = None;
        Ok(())
    }

    /// Whether the client reads the row once more after an action closed a card: a stored row, unless the card was closed by
    /// its deleted row and moved to another row, which OnAfterGetRecord already read before OnQueryClosePage (no read follows
    /// the close: Y89GoneSingleMove.al:4372 and :4378, the log ends `X:20`).
    pub(super) fn reads_row_after_close(&mut self, st: &St, rec: &Rec) -> Result<bool, Signal> {
        if std::mem::take(&mut st.borrow_mut().gone_closed_on_row) {
            return Ok(false);
        }
        Ok(self.stored_row(rec)?.is_some())
    }
}
