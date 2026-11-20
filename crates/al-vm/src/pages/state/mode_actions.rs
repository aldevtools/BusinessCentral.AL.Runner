#![deny(clippy::wildcard_enum_match_arm)]
//! The built-in View and Edit actions a TestPage variable handed out (`P.View()`, `P.Edit()`) and the open of the page each one
//! is bound to (assumption `mode-actions-bound-to-earlier-open`, corpus PR 1467, open): an action taken once stays bound to that
//! open after Close; the next open of the variable does not take it over, and `Clear(P)` frees both. The rules that read it are in
//! `pages/builtin_mode_action.rs`.

use super::PageMode;

/// One built-in action of the variable.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ModeAction {
    /// Not taken since the variable was declared or cleared: the next `P.View()` or `P.Edit()` binds it to the open page.
    #[default]
    Free,
    /// Taken on the page that is open now (or was, until its Close).
    Live,
    /// Taken on an earlier open of the variable: that page closed in `mode`, with `test_close` by a clean `TestPage.Close()`.
    Bound { mode: PageMode, test_close: bool },
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

    /// The variable opens again: an action taken on the earlier open stays bound to it, which closed in `mode`, by a clean
    /// `TestPage.Close()` when `test_close`. An action bound to an open before that keeps it.
    pub fn page_reopens(&mut self, mode: PageMode, test_close: bool) {
        for a in [&mut self.view, &mut self.edit] {
            if *a == ModeAction::Live {
                *a = ModeAction::Bound { mode, test_close };
            }
        }
    }

    /// The earlier open the built-in Edit (`edit`) or View action is bound to: the mode that page closed in, and whether a clean
    /// `TestPage.Close()` closed it. `None`: the action is free or taken on the open page.
    pub fn bound(&self, edit: bool) -> Option<(PageMode, bool)> {
        match if edit { self.edit } else { self.view } {
            ModeAction::Bound { mode, test_close } => Some((mode, test_close)),
            ModeAction::Free | ModeAction::Live => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        a.page_reopens(PageMode::Edit, true);
        assert_eq!(a.bound(false), Some((PageMode::Edit, true)));
        assert_eq!(a.bound(true), None);
        a.taken(true);
        assert_eq!(a.bound(true), None);
    }

    #[test]
    fn a_bound_action_keeps_its_page_over_later_opens() {
        let mut a = ModeActions::default();
        a.taken(true);
        a.page_reopens(PageMode::View, true);
        a.taken(true);
        a.page_reopens(PageMode::New, false);
        assert_eq!(a.bound(true), Some((PageMode::View, true)));
    }

    #[test]
    fn a_reopen_with_nothing_taken_binds_nothing() {
        let mut a = ModeActions::default();
        a.page_reopens(PageMode::Edit, true);
        assert_eq!(a, ModeActions::default());
    }
}
