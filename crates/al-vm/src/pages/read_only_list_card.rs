//! The built-in Edit and View actions of a read-only List standing on the line `New()` started (al-runner issue 7477).

use super::*;

impl Session {
    /// Whether the list stands where `list_card_action` may open its card although it shows no stored row: a List whose page
    /// says `Editable = false`, on the untouched line `New()` started. The card opens as
    /// without a record, on the first row of its view (testpage/NewRowThenMove.al:718 ReadOnlyList_EmptyTable_New_Edit `card row` empty, `card editable` Yes, :753
    /// ReadOnlyList_EmptyTable_New_View `card editable` No, :771 ReadOnlyList_TwoRows_New_Edit `card row` S1), and the list
    /// stands on the row the card left afterwards (`list row after` X). Another list or line is not pinned.
    pub(super) fn read_only_list_new_line_card(&self, st: &St) -> bool {
        let object = st.borrow().object as usize;
        let untouched_new_line = {
            let s = st.borrow();
            !s.dirty && s.cursor != Cursor::Row && s.host.is_none() && !s.modal
        };
        untouched_new_line
            && self.is_list_page(st)
            && self.engine.program.objects[object]
                .properties
                .iter()
                .any(|(k, v)| k.eq_ignore_ascii_case("Editable") && v.trim().eq_ignore_ascii_case("false"))
    }

    /// After the card `list_card_action` opened from the line `New()` started closes, the read-only List stands on the row the
    /// card was on (testpage/NewRowThenMove.al:718 `list row after` = X, the row the card added; :771 the same with two rows
    /// stored before). A card that closed on no stored row leaves the list where it was (not pinned).
    pub(super) fn read_only_list_follows_card(&mut self, st: &St, card: &St) -> Result<(), Signal> {
        if !st.borrow().open || card.borrow().instance.is_none() || !self.has_source(card) {
            return Ok(());
        }
        let rec = self.page_rec(card);
        if self.stored_row(&rec)?.is_none() {
            return Ok(());
        }
        let mut key = self.row_key(&rec)?;
        self.ret_used = true;
        self.page_go_to_key(st, &mut key)?;
        Ok(())
    }
}
