//! The assumption a CurrPage.Update refresh of a card with OnFindRecord or OnNextRecord and a row trigger runs under.
//!
//! A card with only OnNextRecord (no OnFindRecord) stays on its row as the OnFindRecord card does
//! (pages/currpage/UpdateOnFindRecord.al:357 CurrPage_Update_CardWithOnlyOnNextRecord_KeepsCurrentRow, corpus pull request 1199,
//! al-runner issue 7268): the default Find('=') reads the row again. A list with only OnNextRecord, or such a card with unsaved
//! edits, is not pinned and stays refused in `actions.rs`.

use super::*;
use al_builtins::assumptions::{Assumption, assume};

impl Session {
    /// Marks the assumption of the refresh: update-refresh-onfindrecord (the trigger order of the refresh) unless a pin covers
    /// this page's refresh. A card with only OnNextRecord staying on its row is pinned (UpdateOnFindRecord.al:357); the trigger
    /// order of its refresh is not, so it is marked the same way.
    pub(super) fn assume_card_refresh(&self, st: &St) {
        if !st.borrow().refresh_pinned {
            assume(Assumption::UpdateRefreshOnFindRecord);
        }
    }
}
