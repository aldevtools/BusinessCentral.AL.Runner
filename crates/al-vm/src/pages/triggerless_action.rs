//! An action with no `OnAction` trigger and no `RunObject`: what Invoke raises for the subscribers of its events and the
//! page extensions' triggers (events/page-events/PageActionEventsWithoutTrigger.al).

use super::actions::action_error_code;
use super::*;

impl Session {
    /// Whether a compiled subscriber, or one declared and not compiled, names action `element` for OnBeforeActionEvent or
    /// OnAfterActionEvent. A subscriber for every action (an empty element) does not count.
    pub(super) fn names_action_subscriber(&self, st: &St, element: &str) -> bool {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        ["onbeforeactionevent", "onafteractionevent"].iter().any(|ev| {
            engine
                .events
                .subscribers
                .get(&(object, ev.to_string()))
                .is_some_and(|v| v.iter().any(|s| s.element.eq_ignore_ascii_case(element)))
                || engine
                    .events
                    .missing
                    .get(&(object, ev.to_string()))
                    .is_some_and(|v| v.iter().any(|m| m.element.eq_ignore_ascii_case(element)))
        })
    }

    /// Refuses an invoke while a subscriber to the action events names no action of the page, or every action: BC runs
    /// those (or does not compile a name the page lacks, AL0271) and no corpus case pins what the client raises for them.
    /// This was refused when the page opened (`check_page_events`); a page nothing invokes opens.
    pub(super) fn refuse_unpinned_action_subscribers(&self, st: &St) -> Result<(), Signal> {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        for ev in ["onbeforeactionevent", "onafteractionevent"] {
            let compiled = engine
                .events
                .subscribers
                .get(&(object, ev.to_string()))
                .into_iter()
                .flatten()
                .map(|s| s.element.as_str());
            let missing = engine
                .events
                .missing
                .get(&(object, ev.to_string()))
                .into_iter()
                .flatten()
                .map(|m| m.element.as_str());
            let page = page_of(&engine, st);
            let on_page = |element: &str| {
                !element.is_empty()
                    && page
                        .controls
                        .iter()
                        .any(|c| c.kind == ir::ControlKind::Action && c.name.eq_ignore_ascii_case(element))
            };
            if let Some(e) = compiled.chain(missing).find(|e| !on_page(e)) {
                return unsupported(format!(
                    "page event {ev} of action `{e}`: a subscriber for every action, or for a name the page lacks, is not pinned for the test client"
                ));
            }
        }
        Ok(())
    }

    /// Invoke of an action with no OnAction trigger and no RunObject: the subscriber's OnBeforeActionEvent, the page
    /// extensions' OnBeforeAction, their OnAfterAction, the subscriber's OnAfterActionEvent, each every time, with no error
    /// (events/page-events/PageActionEventsWithoutTrigger.al:493 ActionEvents_Invoke_PlainActionWithAndWithoutTrigger,
    /// :519 _GroupHiddenDisabledNoTrigger, :547 _ExtensionActionAndExtensionTriggersOnly; a second Invoke repeats them).
    /// The effects on the row of a page with row triggers are not pinned and are refused.
    pub(super) fn triggerless_action(
        &mut self,
        st: &St,
        element: &str,
        before_ext: &[(usize, usize)],
        after_ext: &[(usize, usize)],
    ) -> Result<Value, Signal> {
        if before_ext.len() > 1 {
            return unsupported(format!(
                "page extension trigger OnBeforeAction of action {element} in more than one page extension (the order is not pinned)"
            ));
        }
        if self.has_source(st)
            && [
                "onaftergetrecord",
                "onaftergetcurrrecord",
                "onfindrecord",
                "onnextrecord",
            ]
            .iter()
            .any(|t| self.declares(st, t))
        {
            return unsupported(format!(
                "action {element} without OnAction or RunObject on a page with row triggers (the reads after it are not pinned)"
            ));
        }
        let run = self
            .page_action_event(st, "onbeforeactionevent", element)
            .and_then(|()| self.run_ext_after_action(st, before_ext))
            .and_then(|()| self.run_ext_after_action(st, after_ext))
            .and_then(|()| self.page_action_event(st, "onafteractionevent", element));
        match run {
            Err(Signal::Error(mut e)) => {
                e.code = action_error_code(&e.code, &e.message);
                Err(Signal::Error(e))
            }
            other => other.map(|()| Value::Null),
        }
    }
}
