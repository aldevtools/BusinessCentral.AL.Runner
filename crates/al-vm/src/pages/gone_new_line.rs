//! A List with the new-row line, opened for edit, that lost its only row to an action stands on its blank line (the action
//! raised OnNewRecord and OnAfterGetCurrRecord three times, `settle_after_action`). What the moves and a later action do there
//! (corpus master pages/lt-y80-gone-rows/Y80GoneRows.al, codeunit 94230):
//! - First, Next, Last and Previous in a row answer No each and log `N:0;C:0;N:0;C:0` in all (Y80_Gone_All_OpenEdit_DeleteAllUpdate,
//!   _DeleteAll, _DeleteCurrent `moves log`). First raises nothing (pages/lt-f3-pages/F3ListTests.al:189 DeleteOnlyRow_WithoutUpdate
//!   `log_after_first`, misc/lt-probes-needed-7/PnGRefresh.al ListLosesOnlyRowDeleteNoUpdate, lt-outside-worksheet/
//!   RmWorksheetGoneRow.al:452 `close log`). Which two of Next, Last and Previous raise is not pinned: the assumption
//!   `gone-blank-line-next-previous` has Next and Previous raise, as the symmetric pair.
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
        if matches!(how, PageMethod::Next | PageMethod::Previous) {
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::GoneBlankLineNextPrevious);
            st.borrow_mut().window.clear();
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

    /// A card whose row an action deleted asks OnQueryClosePage before it runs OnClosePage.
    pub(super) fn card_query_close_before_forced_close(&mut self, st: &St) -> Result<(), Signal> {
        // The card asks OnQueryClosePage first, then runs OnClosePage (corpus master pages/lt-y80-gone-rows/Y80GoneRows.al
        // Y80_Gone_Card_OpenEdit_DeleteAllUpdate, _DeleteCurrent, _OpenView_* `action log R:10;D:10;Q;X`). The CloseAction it
        // receives is not logged there; a veto is not pinned.
        if self.declares(st, "onqueryclosepage") {
            let members: std::sync::Arc<[String]> = al_builtins::value::ACTION_MEMBERS
                .iter()
                .map(|m| m.to_string())
                .collect();
            let none = al_builtins::value::ACTION_MEMBERS
                .iter()
                .position(|m| *m == "None")
                .unwrap_or(0) as i32;
            if self.query_close_answer(st, vec![Value::Option(none, members)])? == Some(false) {
                return unsupported(
                    "a card whose OnQueryClosePage vetoes the close its deleted row forces (not pinned)",
                );
            }
        }
        Ok(())
    }
}
