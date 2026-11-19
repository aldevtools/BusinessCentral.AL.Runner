#![deny(clippy::wildcard_enum_match_arm)]
//! What a TestPage GoToKey left on a list without OnAfterGetCurrRecord when it moved the cursor: the client reads the row once
//! more before every later action (pages/list-order/AiListOrderTests.al:39 Keep_Nav `update` = `R:R1002;R:R1002;A:filter...`;
//! QgGoToKeyNextCall.al:297 Plain_GoToKey_ThenActionWithoutUpdate_Twice `second_action_log`), until a move clears it. The mark
//! carries what came between that GoToKey and the next call (`GoToKeyLeft`). The rules that read it are at the callers in
//! `pages/pageops.rs`, `pages/controls.rs` and `pages/actions.rs`.

/// The mark a GoToKey left. Nothing is set (`Default`) until `moved`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AfterGoToKey {
    /// The mark stands: a GoToKey moved the list and no move cleared it since.
    active: bool,
    /// What came between that GoToKey and the next call.
    left: GoToKeyLeft,
}

/// What came between the GoToKey and the next call, taken from the mark and put back by the caller that keeps it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GoToKeyLeft {
    /// A call that is not pinned (a property read, an `As*` read) came in between: whether the row is still read again before
    /// an action is not pinned (the action is refused). A Value read and SetValue are pinned (QgGoToKeyNextCall.al:236, :317)
    /// and do not set it.
    doubt: bool,
    /// The GoToKey ran no page trigger (the key is among the rows the client holds): Close then reads the row once
    /// (pages/list-order/QgGoToKeyNextCall.al:223 Plain_GoToKey_ThenValueRead, :277 Plain_GoToKey_ThenClose `close_log`
    /// `R:R1003`). A GoToKey that read rows leaves Close reading nothing (testpage/TestPageSystemPartListWindowContracts.al:219).
    quiet: bool,
    /// A SetValue came in between.
    set: bool,
    /// A Previous came after the GoToKey and no action since: what Close then reads is not pinned (it is refused).
    prev: bool,
}

impl GoToKeyLeft {
    /// A call that is not pinned came in between.
    pub fn doubt(self) -> bool {
        self.doubt
    }

    /// The GoToKey ran no page trigger.
    pub fn quiet(self) -> bool {
        self.quiet
    }

    /// A SetValue came in between.
    pub fn set(self) -> bool {
        self.set
    }

    /// A Previous came after the GoToKey and no action since.
    pub fn prev(self) -> bool {
        self.prev
    }

    /// The same, after a call that is not pinned.
    pub fn with_doubt(self) -> Self {
        Self { doubt: true, ..self }
    }

    /// The same, after a Previous.
    pub fn after_previous(self) -> Self {
        Self { prev: true, ..self }
    }
}

impl AfterGoToKey {
    /// A GoToKey moved the list without OnAfterGetCurrRecord; `quiet` when it ran no page trigger. What an earlier mark
    /// noted in between stays.
    pub fn moved(&mut self, quiet: bool) {
        self.active = true;
        self.left.quiet = quiet;
    }

    /// The mark stands.
    pub fn active(&self) -> bool {
        self.active
    }

    /// A SetValue came while the mark stands.
    pub fn set_value_between(&mut self) {
        if self.active {
            self.left.set = true;
        }
    }

    /// A call that is not pinned came while the mark stands.
    pub fn unpinned_call_between(&mut self) {
        if self.active {
            self.left.doubt = true;
        }
    }

    /// An action starts: a SetValue or a Previous before it no longer counts; the mark stays. Answers whether the mark
    /// stands and whether a call that is not pinned came in between.
    pub fn action_starts(&mut self) -> (bool, bool) {
        self.left.set = false;
        self.left.prev = false;
        (self.active, self.left.doubt)
    }

    /// Takes the mark off: what came in between when it stood, `None` when it did not.
    pub fn take(&mut self) -> Option<GoToKeyLeft> {
        let active = std::mem::take(&mut self.active);
        let left = std::mem::take(&mut self.left);
        active.then_some(left)
    }

    /// Puts a taken mark back.
    pub fn keep(&mut self, left: GoToKeyLeft) {
        self.active = true;
        self.left = left;
    }
}

#[cfg(test)]
mod tests {
    use super::{AfterGoToKey, GoToKeyLeft};

    #[test]
    fn a_new_mark_does_not_stand() {
        let mut a = AfterGoToKey::default();
        assert!(!a.active());
        assert_eq!(a.action_starts(), (false, false));
        assert_eq!(a.take(), None);
    }

    #[test]
    fn a_move_sets_the_mark_and_whether_it_was_quiet() {
        let mut a = AfterGoToKey::default();
        a.moved(true);
        assert!(a.active());
        let left = a.take().unwrap();
        assert!(left.quiet() && !left.doubt() && !left.set() && !left.prev());
        assert!(!a.active());
    }

    #[test]
    fn a_second_move_keeps_what_came_between() {
        let mut a = AfterGoToKey::default();
        a.moved(true);
        a.set_value_between();
        a.unpinned_call_between();
        a.moved(false);
        let left = a.take().unwrap();
        assert!(!left.quiet() && left.set() && left.doubt());
    }

    #[test]
    fn calls_between_count_only_while_the_mark_stands() {
        let mut a = AfterGoToKey::default();
        a.set_value_between();
        a.unpinned_call_between();
        a.moved(false);
        assert_eq!(a.take(), Some(GoToKeyLeft::default()));
    }

    #[test]
    fn an_action_clears_set_value_and_previous_and_keeps_the_mark_and_the_doubt() {
        let mut a = AfterGoToKey::default();
        a.moved(false);
        a.set_value_between();
        a.unpinned_call_between();
        let left = a.take().unwrap();
        a.keep(left.after_previous());
        assert_eq!(a.action_starts(), (true, true));
        let left = a.take().unwrap();
        assert!(!left.set() && !left.prev() && left.doubt());
    }

    #[test]
    fn a_taken_mark_put_back_is_the_same() {
        let mut a = AfterGoToKey::default();
        a.moved(true);
        a.set_value_between();
        let left = a.take().unwrap();
        a.keep(left.with_doubt());
        assert!(a.active());
        assert_eq!(a.take(), Some(left.with_doubt()));
    }
}
