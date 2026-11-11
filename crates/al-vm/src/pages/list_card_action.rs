//! The built-in Edit and View actions of a list page (moved out of `actions.rs`), and of a read-only List standing on the line
//! `New()` started (al-runner issue 7477).

use super::*;

impl Session {
    /// Whether the card a list's Edit action would open can be edited. Only `Editable = false`
    /// is pinned (corpus testpage/TestPageModeSwitchAction_Tests.al:
    /// ListWhoseCardIsReadOnlyLeavesTheEditActionVisibleButNotEnabled); any other restriction is refused.
    pub(super) fn card_allows_edit(&self, card: usize) -> Result<bool, Signal> {
        let engine = self.engine();
        let prop = |n: &str| {
            engine.program.objects[card]
                .properties
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(n))
                .map(|(_, v)| v.trim().to_ascii_lowercase())
        };
        if prop("ModifyAllowed").is_some_and(|v| v != "true") {
            return unsupported("the Edit action of a list whose card sets ModifyAllowed (not pinned)");
        }
        match prop("Editable").as_deref() {
            None | Some("true") => Ok(true),
            Some("false") => Ok(false),
            Some(_) => unsupported("the Edit action of a list whose card computes Editable (not pinned)"),
        }
    }

    /// The built-in Edit or View action of a list page: opens the page named by the list's
    /// `CardPageId` on the current row, editable or read-only, as a `RunObject` action would.
    pub(super) fn list_card_action(&mut self, st: &St, edit: bool) -> Result<Value, Signal> {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let Some(card) = effective_prop(&engine, object, "CardPageId")?.map(|v| v.trim_matches('"').to_string()) else {
            return unsupported("the built-in Edit/View action of a list page with no CardPageId (not pinned)");
        };
        let Some(target) = engine.program.objects.heads().iter().position(|o| {
            o.is_kind(al_ir::ObjKind::Page)
                && (o.name.eq_ignore_ascii_case(&card) || o.id.is_some_and(|i| i.to_string() == card))
        }) else {
            return unsupported("CardPageId names a page outside the workspace");
        };
        // On the line `New()` started of a read-only List the card opens as without a record (`read_only_list_new_line_card`).
        let new_line_card = st.borrow().cursor != Cursor::Row && self.read_only_list_new_line_card(st);
        if st.borrow().cursor != Cursor::Row && !new_line_card {
            return unsupported("the built-in Edit/View action of a list that stands on no stored row (not pinned)");
        }
        let ro = ir::RunObject {
            kind: ir::RunKind::Page,
            object: Some(target),
            on_rec: !new_line_card,
            mode: if edit { ir::RunMode::Edit } else { ir::RunMode::View },
            link: None,
            unsupported: None,
        };
        st.borrow_mut().refresh = false;
        let card = self.run_object_page(st, &ro)?;
        if new_line_card {
            self.read_only_list_follows_card(st, &card)?;
        }
        Ok(Value::Null)
    }

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
