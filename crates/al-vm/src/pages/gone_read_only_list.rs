//! A List with Editable = false over a temporary source, without the new-row line, that lost its only row to an action
//! (al-runner issue 6664). Real BC values from corpus writer Y66 (pages/lt-y66-temp-list-gone-row/Y66TempListGoneRow.al, not on
//! corpus master yet): run modally (Page.RunModal, also in lookup mode) it stands on a blank Rec and raises OnNewRecord and
//! OnAfterGetCurrRecord once (:235 `action log R:10;N:0;C:0;`, :263 with CurrPage.Update(false), :291 two rows, :319 LookupMode,
//! :350 ascending); opened by a TestPage for view or edit it raises nothing (:371 OpenView, :400 OpenEdit `R:10;`). Either way no
//! row is shown and First, Last, Next and Previous find none. What a later action does is pinned there too but not built here:
//! it is refused.

use super::*;

impl Session {
    /// Settles the page when it has the shape above; false when it has another shape.
    pub(super) fn gone_read_only_temp_list(&mut self, st: &St, rec: &Rec) -> Result<bool, Signal> {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let read_only = engine.program.objects[object]
            .properties
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("Editable") && v.trim().eq_ignore_ascii_case("false"));
        let shape = read_only
            && engine.object_facts[object].page_type == Some(al_ir::PageType::List)
            && rec.borrow().temporary
            && self.tree_indentation(st).is_none()
            && st.borrow().host.is_none();
        // Opened for view without being modal, the page takes the pinned path of `read_only_view_list_shows_nothing`
        // (RmWorksheetGoneRow.al:481), which Y66's OpenView agrees with.
        let pinned_elsewhere = !st.borrow().modal && st.borrow().mode == PageMode::View;
        if !shape || pinned_elsewhere {
            return Ok(false);
        }
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::ReadOnlyTempListGoneOnlyRow);
        if st.borrow().modal {
            self.settle_on_blank_rec(st, rec)?;
            // A later action is refused (`settle_after_action`): its refill is not built.
            st.borrow_mut().gone_view_list = true;
            return Ok(true);
        }
        let fresh = self.initial_fields(rec.borrow().table as usize);
        rec.borrow_mut().fields = fresh;
        let mut s = st.borrow_mut();
        s.refresh = false;
        s.cursor = Cursor::None;
        s.gone_view_list = true;
        Ok(true)
    }
}
