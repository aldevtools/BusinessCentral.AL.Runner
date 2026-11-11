//! The body of an action with an OnAction trigger, between what the client saves before it and the refresh after it: the
//! action's events and extension triggers around OnAction (events/platform-triggers/P114PageActionEvents.al), and the read of a
//! part a Visible expression hid that the body made visible (`provider_hidden.rs`).

use super::*;

/// The extension triggers of an action, as (extension, routine) pairs.
type ExtTriggers<'a> = &'a [(usize, usize)];

impl Session {
    /// Runs OnBeforeActionEvent and the extensions' OnBeforeAction, OnAction (routine `r` of control `idx`), the extensions'
    /// OnAfterAction, puts `shown_outer` back as the row the action shows, then OnAfterActionEvent. Subscribers run before the
    /// trigger (what they write to Rec the trigger reads) and after it when it did not fail. `no_edit`: the page and its parts
    /// held no unsaved edit when the action was invoked.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn run_action_body(
        &mut self,
        st: &St,
        idx: usize,
        r: usize,
        element: &str,
        before_ext: ExtTriggers,
        after: ExtTriggers,
        shown_outer: Option<Vec<Value>>,
        no_edit: bool,
    ) -> Result<Value, Signal> {
        let hidden_before = self.before_action_body(st, no_edit)?;
        let v = match self
            .page_action_event(st, "onbeforeactionevent", element)
            .and_then(|()| self.run_ext_after_action(st, before_ext))
        {
            Ok(()) => self.in_page_trigger(st, "onaction", |s| s.control_routine(st, idx, r)),
            err => err.map(|()| Value::Null),
        };
        let v = match v {
            Ok(v) => self.run_ext_after_action(st, after).map(|()| v),
            err => err,
        };
        st.borrow_mut().action_shown_row = shown_outer;
        let v = v.and_then(|v| self.page_action_event(st, "onafteractionevent", element).map(|()| v))?;
        self.read_hidden_parts_the_action_showed(st, &hidden_before)?;
        Ok(v)
    }
}
