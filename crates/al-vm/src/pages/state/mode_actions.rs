//! The built-in View and Edit actions a TestPage variable handed out and the open each is bound to
//! (`ModeActions`: free, taken on the open page, bound to an earlier open with the mode it closed in
//! and the test method it opened in); it survives Close and reopen, `Clear(P)` frees it (al-runner
//! issue 8501, assumption `mode-actions-bound-to-earlier-open`). Rules in `builtin_mode_action.rs`.
//! Entry points: `ModeActions`, field `PageState::mode_actions`.
#![deny(clippy::wildcard_enum_match_arm)]
//! The built-in View and Edit actions a TestPage variable handed out (`P.View()`, `P.Edit()`) and the open of the page each one
//! is bound to (assumption `mode-actions-bound-to-earlier-open`, corpus PR 1467, open): an action taken once stays bound to that
//! open after Close; the next open of the variable does not take it over, and `Clear(P)` frees both. The rules that read it are in
//! `pages/builtin_mode_action.rs`.

use super::PageMode;

/// An earlier open of the variable: the mode its page closed in, whether a clean `TestPage.Close()` closed it, and the test
/// method it opened in (`PageState::opened_in_test_method`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EarlierOpen {
    pub mode: PageMode,
    pub test_close: bool,
    pub test_method: u32,
}

/// One built-in action of the variable.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ModeAction {
    /// Not taken since the variable was declared or cleared: the next `P.View()` or `P.Edit()` binds it to the open page.
    #[default]
    Free,
    /// Taken on the page that is open now (or was, until its Close).
    Live,
    /// Taken on an earlier open of the variable.
    Bound(EarlierOpen),
}

/// The variable's built-in View and Edit actions (`PageState::mode_actions`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ModeActions {
    view: ModeAction,
    edit: ModeAction,
}

impl ModeActions {
    fn slot(&mut self, edit: bool) -> &mut ModeAction {
        if edit { &mut self.edit } else { &mut self.view }
    }

    /// The test took the built-in Edit (`edit`) or View action from the variable while the page is open.
    pub fn taken(&mut self, edit: bool) {
        let a = self.slot(edit);
        if *a == ModeAction::Free {
            *a = ModeAction::Live;
        }
    }

    /// The variable opens again: an action taken on the earlier open (`earlier`) stays bound to it. An action bound to an open
    /// before that keeps it.
    pub fn page_reopens(&mut self, earlier: EarlierOpen) {
        for a in [&mut self.view, &mut self.edit] {
            if *a == ModeAction::Live {
                *a = ModeAction::Bound(earlier);
            }
        }
    }

    /// The earlier open the built-in Edit (`edit`) or View action is bound to. `None`: the action is free or taken on the open
    /// page.
    pub fn bound(&self, edit: bool) -> Option<EarlierOpen> {
        match if edit { self.edit } else { self.view } {
            ModeAction::Bound(earlier) => Some(earlier),
            ModeAction::Free | ModeAction::Live => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn closed(mode: PageMode, test_close: bool, test_method: u32) -> EarlierOpen {
        EarlierOpen {
            mode,
            test_close,
            test_method,
        }
    }

    #[test]
    fn an_action_taken_on_the_open_page_is_not_bound_to_an_earlier_one() {
        let mut a = ModeActions::default();
        a.taken(false);
        assert_eq!(a.bound(false), None);
        assert_eq!(a.bound(true), None);
    }

    #[test]
    fn a_reopen_binds_only_the_actions_taken_before_it() {
        let mut a = ModeActions::default();
        a.taken(false);
        a.page_reopens(closed(PageMode::Edit, true, 1));
        assert_eq!(a.bound(false), Some(closed(PageMode::Edit, true, 1)));
        assert_eq!(a.bound(true), None);
        a.taken(true);
        assert_eq!(a.bound(true), None);
    }

    #[test]
    fn a_bound_action_keeps_its_page_over_later_opens() {
        let mut a = ModeActions::default();
        a.taken(true);
        a.page_reopens(closed(PageMode::View, true, 1));
        a.taken(true);
        a.page_reopens(closed(PageMode::New, false, 2));
        assert_eq!(a.bound(true), Some(closed(PageMode::View, true, 1)));
    }

    #[test]
    fn a_reopen_with_nothing_taken_binds_nothing() {
        let mut a = ModeActions::default();
        a.page_reopens(closed(PageMode::Edit, true, 1));
        assert_eq!(a, ModeActions::default());
    }
}
