#![deny(clippy::wildcard_enum_match_arm)]
//! Where the built-in View and Edit left a list without a card that was opened for edit (al-runner issue 6977): View switched
//! it to view (testpage/R6977ListViewNext.al:138, R6977ListAfterView.al:125), and Edit after that switched it back
//! (R6977ListAfterView.al:187). The rules that read it are in `pages/list_view_switch.rs`.

/// What View and Edit left. Nothing is set (`Default`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ListViewSwitch {
    /// The built-in View switched the list from edit to view: the built-in Edit switches it back.
    switched_to_view: bool,
    /// The built-in Edit switched the list back to edit after View. Only Editable, the fields, the Edit and View Enabled reads,
    /// ValidationErrorCount and Close are pinned in this state.
    edit_back: bool,
}

impl ListViewSwitch {
    /// The built-in View switched the list to view. An earlier switch back stays.
    pub fn switched_to_view(&mut self) {
        self.switched_to_view = true;
    }

    /// The built-in Edit was invoked: after a View it switches the list back and answers true; otherwise nothing changes.
    pub fn edit_invoked(&mut self) -> bool {
        if !self.switched_to_view {
            return false;
        }
        self.switched_to_view = false;
        self.edit_back = true;
        true
    }

    /// The page opens again: nothing of an earlier switch counts.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Edit switched the list back to edit after View.
    pub fn edit_back(&self) -> bool {
        self.edit_back
    }
}

#[cfg(test)]
mod tests {
    use super::ListViewSwitch;

    #[test]
    fn edit_without_a_view_changes_nothing() {
        let mut l = ListViewSwitch::default();
        assert!(!l.edit_invoked());
        assert!(!l.edit_back());
    }

    #[test]
    fn view_then_edit_switches_back_once() {
        let mut l = ListViewSwitch::default();
        l.switched_to_view();
        assert!(!l.edit_back());
        assert!(l.edit_invoked());
        assert!(l.edit_back());
        assert!(!l.edit_invoked());
        assert!(l.edit_back());
    }

    #[test]
    fn view_edit_view_keeps_the_switch_back() {
        let mut l = ListViewSwitch::default();
        l.switched_to_view();
        l.edit_invoked();
        l.switched_to_view();
        assert!(l.edit_back());
        assert!(l.edit_invoked());
        assert!(l.edit_back());
    }

    #[test]
    fn a_reset_forgets_both() {
        let mut l = ListViewSwitch::default();
        l.switched_to_view();
        l.edit_invoked();
        l.switched_to_view();
        l.reset();
        assert!(!l.edit_back());
        assert!(!l.edit_invoked());
    }
}
