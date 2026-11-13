//! `TestAction.Enabled()` right after a `GoToKey` on a list whose OnAfterGetRecord sets the page variable the action's Enabled
//! reads (al-runner issue 7916). The list read its rows at open, so the variable holds what OnAfterGetRecord computed for the
//! last row read; BC answers from the row the GoToKey landed on: the reporter's repro on real BC gives `e2=Yes rel=349` after
//! `GoToKey(Dimension Set Entry, 3)` where the earlier code answered No, and open corpus PR 1367 (testpage/
//! ActionEnabledAfterGoToKey.al:610 FieldTable_TrapPageRun_EnabledAfterGoToKey, :430 List_TrapPageRunFiltered) measures the same
//! after a trapped static Page.Run (group G1 passes on every major of its CI run). Only the reporter's shape runs here, under the assumption
//! `action-enabled-follows-row-after-go-to-key`; every other shape keeps the variable's current value.

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
    /// `TestField.Enabled()` and `TestAction.Enabled()`: a field of a part also answers what its part control says
    /// (`part_field_gate`); an action right after a GoToKey on the repro's page answers for the row
    /// (`action_enabled_after_go_to_key`); else the control's Enabled now.
    pub(super) fn test_control_enabled(&mut self, st: &St, idx: usize) -> Result<Value, Signal> {
        if let Some(b) = self.part_field_gate(st, false)? {
            return Ok(Value::Boolean(b));
        }
        if let Some(b) = self.action_enabled_after_go_to_key(st, idx)? {
            return Ok(Value::Boolean(b));
        }
        Ok(Value::Boolean(self.control_enabled(st, idx)?))
    }

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

    /// The global slots an action's Enabled expression of the repro's page reads, for `note_row_prop_vars` to note per row:
    /// `action_enabled_after_go_to_key` answers only on a row the page read.
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

    /// `Enabled()` of action `idx` for the row the page stands on, when every fact of the repro holds:
    /// - the page is the repro's page (`action_enabled_row_page`), at the top level, run by the static `Page.Run(Page::X, Rec)`
    ///   into a trapped TestPage (`trapped_static_run`), not modal and not in lookup mode;
    /// - the action's Enabled is an expression of the page's variables (`prop_global_slots`), each one set by OnAfterGetRecord
    ///   from Rec alone and set by no other code of the page (`row_function_slots`);
    /// - the test's call just before this one was a `GoToKey` on the page that found its row;
    /// - the page read that row (`row_prop_vars` has it) and OnAfterGetRecord ran last for another row (on that row itself
    ///   the variables already hold its values).
    ///
    /// Then OnAfterGetRecord runs once more on the row (it only assigns those variables), Enabled is read, and the variables
    /// get back the values they held. `None` for every other shape: the caller answers from the variables as they are.
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
                && s.trapped_static_run
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
        let Some((agr, assigned)) = row_function_slots(&self.engine().program, st.borrow().object as usize) else {
            return Ok(None);
        };
        if !slots.iter().all(|slot| assigned.contains(slot)) {
            return Ok(None);
        }
        let key = self.row_key(&self.page_rec(st))?;
        let read_here = {
            let s = st.borrow();
            let last_is_here = s.agr_row.as_ref().is_some_and(|k| Self::same_key(k, &key));
            let read = s.row_prop_vars.iter().any(|(k, _)| Self::same_key(k, &key));
            read && !last_is_here
        };
        if !read_here {
            return Ok(None);
        }
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::ActionEnabledFollowsRowAfterGoToKey);
        let inst = self.owner_instance(st, None);
        let saved: Vec<(usize, Value)> = assigned
            .iter()
            .map(|slot| (*slot, inst.borrow().globals[*slot].clone()))
            .collect();
        let r = self
            .owner_routine(st, None, agr)
            .and_then(|_| self.prop_bool_now(st, idx, "enabled", true));
        for (slot, v) in saved {
            inst.borrow_mut().globals[slot] = v;
        }
        r.map(Some)
    }
}

/// The OnAfterGetRecord routine of page `object` and the page variables it sets, when it is a function of Rec alone and the
/// only code of the page that sets them, as in the repro (al-runner issue 7916: `NextEnabled := Rec.RelationTableNo <> 0`):
/// each statement assigns a page variable (not Rec or xRec) an expression of constants and fields of Rec, and no other
/// routine of the page assigns those variables, passes them to a `var` parameter of any call or runs `ClearAll`. Run again on a row, it
/// sets those variables from the row's fields and does nothing else. `None` for every other page.
fn row_function_slots(program: &ir::Program, object: usize) -> Option<(usize, Vec<usize>)> {
    fn of_rec(e: &ir::Expr, rec: usize) -> bool {
        match e {
            ir::Expr::Const(_) => true,
            ir::Expr::Load(ir::Place::Field(base, _)) => matches!(base.as_ref(), ir::Place::Global(g) if *g == rec),
            ir::Expr::Unary(_, a) | ir::Expr::Convert(_, a) => of_rec(a, rec),
            ir::Expr::Binary(_, a, b) => of_rec(a, rec) && of_rec(b, rec),
            _ => false,
        }
    }
    fn root_global(p: &ir::Place) -> Option<usize> {
        match p {
            ir::Place::Global(g) => Some(*g),
            ir::Place::Index(base, _) | ir::Place::Field(base, _) => root_global(base),
            _ => None,
        }
    }
    fn by_ref(a: &ir::Arg, assigned: &[usize]) -> bool {
        matches!(a, ir::Arg::Ref(p) if root_global(p).is_some_and(|g| assigned.contains(&g)))
    }
    let o = &program.objects[object];
    let rec = o.rec_slot?;
    let page = o.page.as_ref()?;
    let (_, agr) = page.triggers.iter().find(|(n, _)| n == "onaftergetrecord")?;
    let mut assigned = Vec::new();
    for s in o.routines.get(*agr)?.body.stmts() {
        match s {
            ir::Stmt::Pos(_) => {}
            ir::Stmt::Assign(ir::Place::Global(g), e) if *g != rec && Some(*g) != o.xrec_slot && of_rec(e, rec) => {
                if !assigned.contains(g) {
                    assigned.push(*g);
                }
            }
            _ => return None,
        }
    }
    for r in (0..o.routines.len()).filter(|r| r != agr) {
        let written = super::written_by_routine(&o.routines, object, r)?;
        let mut clears = false;
        // Every expression of every statement (`for_each_expr_post`, `exit(...)` and case scrutinees too): a call of any kind
        // that hands an assigned variable to a `var` parameter writes it.
        ir::types::for_each_expr_post(o.routines[r].body.stmts(), &mut |e| {
            let args: &[ir::Arg] = match e {
                ir::Expr::Builtin { id, receiver, args, .. } => {
                    clears |= *id == al_builtins::builtin_id::ids::SYSTEM_CLEARALL_1
                        || receiver.as_deref().is_some_and(|a| by_ref(a, &assigned));
                    args
                }
                ir::Expr::Call { args, .. } | ir::Expr::InterfaceCall { args, .. } | ir::Expr::DotNet { args, .. } => {
                    args
                }
                _ => &[],
            };
            clears |= args.iter().any(|a| by_ref(a, &assigned));
        });
        if clears || assigned.iter().any(|g| written.contains(g)) {
            return None;
        }
    }
    Some((*agr, assigned))
}
