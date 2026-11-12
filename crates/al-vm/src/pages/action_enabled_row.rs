//! `TestAction.Enabled()` right after a `GoToKey` on a list whose OnAfterGetRecord sets the page variable the action's Enabled
//! reads (al-runner issue 7916). The list read its rows at open, so the variable holds what OnAfterGetRecord computed for the
//! last row read; BC answers from the row the GoToKey landed on: the reporter's repro on BC 29 gives `e2=Yes rel=349` after
//! `GoToKey(Dimension Set Entry, 3)` where the earlier code answered No. Only the reporter's shape runs here, under the
//! assumption `action-enabled-follows-row-after-go-to-key`; every other shape keeps the variable's current value.

use super::pinned_shape::{PinnedCode, TestCall};
use super::*;
use al_builtins::page_op::PageMethod;

/// The page of the repro (al-runner issue 7916, page 50313 "PX Enabled Walk"): a List over the virtual table Field with
/// `Editable = false`, OnAfterGetRecord as its only trigger, fields of Rec in a repeater and actions whose Enabled is an
/// expression and whose only trigger is OnAction.
const CODE: PinnedCode = PinnedCode {
    triggers: &["onaftergetrecord"],
    control_triggers: &["onaction"],
    properties: &["editable"],
    action_props: &["enabled"],
};

/// The virtual table `Field` the repro's page shows.
const FIELD_TABLE: u32 = 2_000_000_041;

impl Session {
    /// Whether the page is the repro's page (see `CODE`).
    fn action_enabled_row_page(&mut self, st: &St) -> Result<bool, Signal> {
        let object = st.borrow().object as usize;
        let list = self.engine.object_facts[object].page_type == Some(al_ir::PageType::List);
        if !list || !self.has_source(st) {
            return Ok(false);
        }
        let table = self.table(&self.page_rec(st))?.object as usize;
        let on_field = self.engine.program.objects[table].id == Some(FIELD_TABLE);
        Ok(on_field && super::flag_denied(&self.engine(), object, "Editable")? && self.page_code_is(st, &CODE)?)
    }

    /// The global slots an action's Enabled expression of the repro's page reads, for `note_row_prop_vars` to keep per row.
    pub(super) fn action_enabled_row_slots(&mut self, st: &St) -> Result<Vec<usize>, Signal> {
        if !self.action_enabled_row_page(st)? {
            return Ok(Vec::new());
        }
        let engine = self.engine();
        let mut out = Vec::new();
        for (i, c) in page_of(&engine, st).controls.iter().enumerate() {
            if c.kind == ir::ControlKind::Action {
                for slot in self.prop_global_slots(st, i, "enabled").unwrap_or_default() {
                    if !out.contains(&slot) {
                        out.push(slot);
                    }
                }
            }
        }
        Ok(out)
    }

    /// `Enabled()` of action `idx` as the row the page stands on had it, when every fact of the repro holds:
    /// - the page is the repro's page (`action_enabled_row_page`), at the top level, run by AL code (`Page.Run`) into a
    ///   trapped TestPage, not modal and not in lookup mode;
    /// - the action's Enabled is an expression of the page's variables (`prop_global_slots`);
    /// - the test's call just before this one was a `GoToKey` on the page that found its row;
    /// - the page stands on a stored row OnAfterGetRecord ran for, whose variables `row_prop_vars` kept, and OnAfterGetRecord
    ///   ran last for another row (on that row itself the variables already hold its values).
    ///
    /// `None` for every other shape: the caller answers from the variables as they are.
    pub(super) fn action_enabled_after_go_to_key(&mut self, st: &St, idx: usize) -> Result<Option<bool>, Signal> {
        let is_action = {
            let engine = self.engine();
            page_of(&engine, st).controls[idx].kind == ir::ControlKind::Action
        };
        let Some(slots) = is_action.then(|| self.prop_global_slots(st, idx, "enabled")).flatten() else {
            return Ok(None);
        };
        let run = {
            let s = st.borrow();
            s.open
                && s.caught_by_trap
                && !s.opened_by_test
                && !s.modal
                && !s.lookup
                && s.host.is_none()
                && s.cursor == Cursor::Row
                && !s.draft
        };
        let after_go_to_key = {
            let s = st.borrow();
            let n = s.tree_calls.len();
            n >= 2
                && !s.tree_calls_unknown
                && s.tree_calls[n - 1].part.is_none()
                && s.tree_calls[n - 1].call == TestCall::Other(PageMethod::Enabled)
                && s.tree_calls[n - 2].part.is_none()
                && s.tree_calls[n - 2].call == TestCall::Page(PageMethod::GoToKey)
                && !s.tree_calls[n - 2].no
                && !s.tree_calls[n - 2].failed
        };
        if slots.is_empty() || !run || !after_go_to_key || !self.action_enabled_row_page(st)? {
            return Ok(None);
        }
        let key = self.row_key(&self.page_rec(st))?;
        let held = {
            let s = st.borrow();
            let last_is_here = s.agr_row.as_ref().is_some_and(|k| Self::same_key(k, &key));
            let row = s
                .row_prop_vars
                .iter()
                .find(|(k, _)| Self::same_key(k, &key))
                .map(|(_, v)| v.clone())
                .filter(|v| slots.iter().all(|slot| v.iter().any(|(n, _)| n == slot)));
            row.filter(|_| !last_is_here)
        };
        let Some(held) = held else {
            return Ok(None);
        };
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::ActionEnabledFollowsRowAfterGoToKey);
        let inst = self.owner_instance(st, None);
        let saved: Vec<(usize, Value)> = held
            .iter()
            .map(|(slot, _)| (*slot, inst.borrow().globals[*slot].clone()))
            .collect();
        for (slot, v) in held {
            inst.borrow_mut().globals[slot] = v;
        }
        let r = self.prop_bool_now(st, idx, "enabled", true);
        for (slot, v) in saved {
            inst.borrow_mut().globals[slot] = v;
        }
        r.map(Some)
    }
}
