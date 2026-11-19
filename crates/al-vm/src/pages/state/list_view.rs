#![deny(clippy::wildcard_enum_match_arm)]
//! Where the built-in View and Edit left a list without a card that was opened for edit (al-runner issue 6977): View switched
//! it to view (testpage/R6977ListViewNext.al:138, R6977ListAfterView.al:125), and Edit after that switched it back
//! (R6977ListAfterView.al:187). The rules that read it are in `pages/list_view_switch.rs`.

/// What View and Edit left. Four states: View, Edit and View again leave the list in view with the switch back noted.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ListViewSwitch {
    /// Neither switch ran since the page opened.
    #[default]
    None,
    /// The built-in View switched the list from edit to view: the built-in Edit switches it back.
    InView,
    /// The built-in Edit switched the list back to edit after View. Only Editable, the fields, the Edit and View Enabled reads,
    /// ValidationErrorCount and Close are pinned in this state.
    EditBack,
    /// View came again after that switch back: Edit switches it back once more.
    EditBackThenView,
}

impl ListViewSwitch {
    /// The built-in View switched the list to view. An earlier switch back stays.
    pub fn switched_to_view(&mut self) {
        *self = match *self {
            Self::None | Self::InView => Self::InView,
            Self::EditBack | Self::EditBackThenView => Self::EditBackThenView,
        };
    }

    /// The built-in Edit was invoked: after a View it switches the list back and answers true; otherwise nothing changes.
    pub fn edit_invoked(&mut self) -> bool {
        match *self {
            Self::InView | Self::EditBackThenView => {
                *self = Self::EditBack;
                true
            }
            Self::None | Self::EditBack => false,
        }
    }

    /// The page opens again: nothing of an earlier switch counts.
    pub fn reset(&mut self) {
        *self = Self::None;
    }

    /// Edit switched the list back to edit after View.
    pub fn edit_back(&self) -> bool {
        match self {
            Self::EditBack | Self::EditBackThenView => true,
            Self::None | Self::InView => false,
        }
    }
}

/// The two flags `ListViewSwitch` replaced, with the bodies the call sites had, for the bounded comparison. Delete it when a
/// probed behavior change makes the old flags wrong.
#[cfg(test)]
mod model {
    #[derive(Debug, Clone, Default)]
    pub struct Flags {
        pub switched: bool,
        pub edit_back: bool,
    }

    impl Flags {
        pub fn switched_to_view(&mut self) {
            self.switched = true;
        }
        pub fn edit_invoked(&mut self) -> bool {
            if !self.switched {
                return false;
            }
            self.switched = false;
            self.edit_back = true;
            true
        }
        pub fn reset(&mut self) {
            *self = Self::default();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ListViewSwitch, model::Flags};

    #[test]
    fn every_sequence_of_four_steps_answers_as_the_old_flags() {
        for n in 0..3usize.pow(4) {
            let (mut new, mut old) = (ListViewSwitch::default(), Flags::default());
            let mut rest = n;
            for _ in 0..4 {
                match rest % 3 {
                    0 => {
                        new.switched_to_view();
                        old.switched_to_view();
                    }
                    1 => assert_eq!(new.edit_invoked(), old.edit_invoked(), "sequence {n}"),
                    _ => {
                        new.reset();
                        old.reset();
                    }
                }
                rest /= 3;
                assert_eq!(new.edit_back(), old.edit_back, "sequence {n}");
                let mut probe = new;
                assert_eq!(probe.edit_invoked(), old.clone().edit_invoked(), "sequence {n}");
            }
        }
    }

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
