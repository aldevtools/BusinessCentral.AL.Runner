//! The first OnFindRecord call of a card run modally on a record that names no row (al-runner issue 6924).

use super::*;

impl Session {
    /// A card with OnFindRecord, run modally with a record that names no stored row (a blank one, with or without a filter),
    /// is asked `=` first, which finds nothing, and then `=><` (corpus pages/properties/DataCaptionExprOrder.al:171
    /// DCE_RunModal_TableProcedure_Rows `OnFindRecord(=) | OnFindRecord(=><)`, :195 _NoFilter_Rows, :224 _Empty). Page.Run asks
    /// `=><` alone (:262 DCE_PageRun_Rows). A card in lookup mode or a list keeps what it asked before.
    pub(super) fn modal_card_asks_find_equals(&mut self, st: &St) -> Result<(), Signal> {
        let shape = {
            let s = st.borrow();
            s.modal && !s.lookup && s.host.is_none()
        };
        if shape
            && !self.is_list(st)
            && self.declares(st, "onfindrecord")
            && self.stored_row(&self.page_rec(st))?.is_none()
        {
            self.nav_find(st, "=")?;
        }
        Ok(())
    }
}
