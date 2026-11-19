#![deny(clippy::wildcard_enum_match_arm)]
//! What a TestPage GoToKey left on a list without OnAfterGetCurrRecord when it moved the cursor: the client reads the row once
//! more before every later action (pages/list-order/AiListOrderTests.al:39 Keep_Nav `update` = `R:R1002;R:R1002;A:filter...`;
//! QgGoToKeyNextCall.al:297 Plain_GoToKey_ThenActionWithoutUpdate_Twice `second_action_log`), until a move clears it. The mark
//! carries what came between that GoToKey and the next call (`GoToKeyLeft`). The rules that read it are at the callers in
//! `pages/pageops.rs`, `pages/controls.rs` and `pages/actions.rs`.

/// The mark a GoToKey left: `None` until `moved`, and while it is `None` nothing that came in between is noted (the details
/// were only ever set while the mark stood, so the old master flag and its four details map onto one `Option`; the bounded
/// comparison in `tests` checks that against the old flags).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AfterGoToKey(Option<GoToKeyLeft>);

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
        self.0.get_or_insert_default().quiet = quiet;
    }

    /// The mark stands.
    pub fn active(&self) -> bool {
        self.0.is_some()
    }

    /// A SetValue came while the mark stands.
    pub fn set_value_between(&mut self) {
        if let Some(left) = &mut self.0 {
            left.set = true;
        }
    }

    /// A call that is not pinned came while the mark stands.
    pub fn unpinned_call_between(&mut self) {
        if let Some(left) = &mut self.0 {
            left.doubt = true;
        }
    }

    /// An action starts: a SetValue or a Previous before it no longer counts; the mark stays. Answers whether the mark
    /// stands and whether a call that is not pinned came in between.
    pub fn action_starts(&mut self) -> (bool, bool) {
        match &mut self.0 {
            Some(left) => {
                left.set = false;
                left.prev = false;
                (true, left.doubt)
            }
            None => (false, false),
        }
    }

    /// Takes the mark off: what came in between when it stood, `None` when it did not.
    pub fn take(&mut self) -> Option<GoToKeyLeft> {
        self.0.take()
    }

    /// Puts a taken mark back.
    pub fn keep(&mut self, left: GoToKeyLeft) {
        self.0 = Some(left);
    }
}

/// The five flags `AfterGoToKey` replaced, with the bodies the call sites had, for the bounded comparison. Delete it when a
/// probed behavior change makes the old flags wrong.
#[cfg(test)]
mod model {
    use super::GoToKeyLeft;

    #[derive(Debug, Clone, Default)]
    pub struct Flags {
        pub active: bool,
        pub doubt: bool,
        pub quiet: bool,
        pub set: bool,
        pub prev: bool,
    }

    impl Flags {
        pub fn moved(&mut self, quiet: bool) {
            self.active = true;
            self.quiet = quiet;
        }
        // The call sites wrote the next two only while `active` was true.
        pub fn set_value_between(&mut self) {
            if self.active {
                self.set = true;
            }
        }
        pub fn unpinned_call_between(&mut self) {
            if self.active {
                self.doubt = true;
            }
        }
        pub fn action_starts(&mut self) -> (bool, bool) {
            self.set = false;
            self.prev = false;
            (self.active, self.doubt)
        }
        pub fn take(&mut self) -> Option<GoToKeyLeft> {
            let left = GoToKeyLeft {
                doubt: std::mem::take(&mut self.doubt),
                quiet: std::mem::take(&mut self.quiet),
                set: std::mem::take(&mut self.set),
                prev: std::mem::take(&mut self.prev),
            };
            std::mem::take(&mut self.active).then_some(left)
        }
        pub fn keep(&mut self, left: GoToKeyLeft) {
            *self = Flags {
                active: true,
                doubt: left.doubt,
                quiet: left.quiet,
                set: left.set,
                prev: left.prev,
            };
        }
        /// No detail is set while the mark is off.
        pub fn details_only_while_active(&self) -> bool {
            self.active || !(self.doubt || self.quiet || self.set || self.prev)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AfterGoToKey, GoToKeyLeft, model::Flags};

    /// Every transition, with each `bool` argument both ways; `keep` puts back what the last `take` gave (or a mark with
    /// every detail set, when nothing was taken).
    const STEPS: usize = 7;

    fn step(i: usize, new: &mut AfterGoToKey, old: &mut Flags, taken: &mut Option<GoToKeyLeft>) {
        let full = GoToKeyLeft {
            doubt: true,
            quiet: true,
            set: true,
            prev: true,
        };
        match i {
            0 | 1 => {
                new.moved(i == 1);
                old.moved(i == 1);
            }
            2 => {
                new.set_value_between();
                old.set_value_between();
            }
            3 => {
                new.unpinned_call_between();
                old.unpinned_call_between();
            }
            4 => assert_eq!(new.action_starts(), old.action_starts()),
            5 => {
                let t = new.take();
                assert_eq!(t, old.take());
                *taken = t;
            }
            _ => {
                let left = taken.unwrap_or(full);
                new.keep(left);
                old.keep(left);
            }
        }
    }

    #[test]
    fn every_sequence_of_four_steps_answers_as_the_old_flags() {
        for n in 0..STEPS.pow(4) {
            let (mut new, mut old, mut taken) = (AfterGoToKey::default(), Flags::default(), None);
            let mut rest = n;
            for _ in 0..4 {
                step(rest % STEPS, &mut new, &mut old, &mut taken);
                rest /= STEPS;
                assert!(old.details_only_while_active(), "sequence {n}");
                assert_eq!(new.active(), old.active, "sequence {n}");
                assert_eq!(new.clone().take(), old.clone().take(), "sequence {n}");
                assert_eq!(new.clone().action_starts(), old.clone().action_starts(), "sequence {n}");
            }
        }
    }

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
