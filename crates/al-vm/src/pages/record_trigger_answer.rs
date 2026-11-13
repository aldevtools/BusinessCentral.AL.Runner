//! What the client reads from the page's own Boolean OnInsertRecord or OnModifyRecord (al-runner issue 7628).
//!
//! - An `exit(value)` that ran is the answer: `exit(false)` vetoes (testpage/TestPageRecordTriggers.al:76
//!   OnInsertRecord_ReturningFalse_SuppressesTheInsert, pages/triggers/OnInsertRecordFalse.al).
//! - A trigger without any `exit(value)` and without a named return value does not veto (pages/currpage/
//!   PageUpdateNewRowCard.Page.al: OnInsertRecord assigns the key and exits nothing, and the row is saved).
//! - A named return value that ends true saves (open corpus PR 1367, pages/triggers/RecordTriggerReturnValue.al:446-541, the
//!   True cells; the rule main had).
//! - An `exit(false)` that did not run saves, and a named return value assigned false vetoes (RecordTriggerReturnValue.al, the
//!   Cond and False cells). Both run only in the probe's shape (`probe_cell`), under an assumption; every other shape of them is
//!   refused, because the earlier rule (the default value of an unnamed return is a veto, a named one never vetoes) is the one
//!   the probe showed wrong.
//! - A named return value that is never assigned keeps the earlier answer (no veto): not measured either way.

use super::pinned_shape::{PinnedCode, TestCall};
use super::*;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::page_op::PageMethod;

/// The probe's cards (RecordTriggerReturnValue.al:49 "PG6 RT Cond Card", :94 "PG6 RT False Card"): the three record triggers,
/// fields of Rec in a group, no action, plain properties; the table logs in OnInsert and OnModify (:34, :41).
const CODE: PinnedCode = PinnedCode {
    triggers: &["oninsertrecord", "onmodifyrecord", "ondeleterecord"],
    control_triggers: &[],
    properties: &[],
    action_props: &[],
    table_triggers: &["oninsert", "onmodify"],
};

impl Session {
    /// The answer of the page's own trigger `name` (`routine`), which returned `v`; `None` lets the row be saved. The page's
    /// own call noted in `PageClients::own_exited` whether an `exit(value)` ran.
    pub(super) fn own_record_trigger_answer(
        &mut self,
        st: &St,
        name: &str,
        routine: Option<usize>,
        v: Option<Value>,
    ) -> Result<Option<bool>, Signal> {
        let (Some(r), Some(Value::Boolean(b))) = (routine, v) else {
            return Ok(None);
        };
        if self.pages.own_exited {
            return Ok(Some(b));
        }
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let routine = &engine.program.objects[object].routines[r];
        if super::has_value_exit(&routine.body) {
            // RecordTriggerReturnValue.al:252 Cond_Insert_OpenNewSetValueClose, :276 _Next, :301 Cond_Modify_SetValueNext,
            // :326 _Close: `if Blocked then exit(false);` with Blocked false saves.
            // corpus testpage/CcPageLeads.al:341 ModifyRecord_AssignsAnotherRecord agrees: its `case` runs no exit and sets Rec to
            // row 2, and the save leaves both rows as they were (`row1` a, `row2` b).
            let cell =
                (only_exit_false_last(&routine.body) && self.probe_cell(st, name)?) || self.mod_mode_cell(st, name)?;
            if routine.ret_local.is_none() && cell {
                assume(Assumption::PageRecordTriggerExitNotTakenSaves);
                return Ok(None);
            }
            return unsupported(format!(
                "the page's {name} trigger ended without running its exit(value) (measured only for the probe's Card)"
            ));
        }
        let Some(local) = routine.ret_local else {
            return Ok(None);
        };
        if b || !assigns(&routine.body, local) {
            return Ok(None);
        }
        // RecordTriggerReturnValue.al:350 False_Insert_OpenNewSetValueClose, :373 _Next, :397 False_Modify_SetValueNext,
        // :422 _Close: `Ok := false;` vetoes.
        if assigns_false_last(&routine.body, local) && self.probe_cell(st, name)? {
            assume(Assumption::PageRecordTriggerNamedFalseVetoes);
            self.pages.named_false_insert = name == "oninsertrecord";
            return Ok(Some(false));
        }
        unsupported(format!(
            "the page's {name} trigger ended with its named return value set to false (measured only for the probe's Card)"
        ))
    }

    /// Whether the page, its open and the test's calls are those of a cell of the probe:
    /// - a top-level Card over a stored table, without parts, with the probe's code (`CODE`), opened by the test, not modal;
    /// - OnInsertRecord on OpenNew after the calls `SetValue` of the key field, then `SetValue` of a field outside the key
    ///   (:258-261: the second SetValue inserts);
    /// - OnModifyRecord on OpenNew after those two calls and `Next` or `Close` (:283-287, :259-262), or on OpenEdit after
    ///   `GoToKey`, `SetValue` of a field outside the key and `Next` or `Close` (:308-312, :333-336).
    fn probe_cell(&mut self, st: &St, name: &str) -> Result<bool, Signal> {
        let mode = st.borrow().mode;
        if !self.top_card_of_test(st) {
            return Ok(false);
        }
        if !self.page_code_is(st, &CODE)? {
            return Ok(false);
        }
        let Some(calls) = Self::test_calls(st) else {
            return Ok(false);
        };
        let key = |c: &TestCall| matches!(c, TestCall::Write(i) if !self.control_outside_key(st, *i));
        let other = |c: &TestCall| matches!(c, TestCall::Write(i) if self.control_outside_key(st, *i));
        let save = |c: &TestCall| matches!(c, TestCall::Page(PageMethod::Next | PageMethod::Close));
        let steps = calls.as_slice();
        Ok(match (name, mode, steps) {
            ("oninsertrecord", PageMode::New, [a, b]) => key(a) && other(b),
            ("onmodifyrecord", PageMode::New, [a, b, c]) => key(a) && other(b) && save(c),
            ("onmodifyrecord", PageMode::Edit, [TestCall::Page(PageMethod::GoToKey), b, c]) => other(b) && save(c),
            _ => false,
        })
    }
}

impl Session {
    /// Whether the page is corpus testpage/CcFixtures.al:244 "LT CC Mod Mode" (a Card over a stored table, OnModifyRecord its
    /// only trigger, fields of Rec, no table code), at the top level, opened by the test with OpenEdit, after the calls
    /// `GoToKey`, `SetValue` of a field outside the key and `Close` (CcPageLeads.al:318-321).
    fn mod_mode_cell(&mut self, st: &St, name: &str) -> Result<bool, Signal> {
        const MOD_MODE: PinnedCode = PinnedCode {
            triggers: &["onmodifyrecord"],
            control_triggers: &[],
            properties: &[],
            action_props: &[],
            table_triggers: &[],
        };
        if name != "onmodifyrecord" || st.borrow().mode != PageMode::Edit || !self.top_card_of_test(st) {
            return Ok(false);
        }
        if !self.page_code_is(st, &MOD_MODE)? {
            return Ok(false);
        }
        Ok(match Self::test_calls(st).as_deref() {
            Some(
                [
                    TestCall::Page(PageMethod::GoToKey),
                    TestCall::Write(i),
                    TestCall::Page(PageMethod::Close),
                ],
            ) => self.control_outside_key(st, *i),
            _ => false,
        })
    }

    /// A Card over a stored table at the top level, without parts, opened by the test, not modal, not a lookup.
    fn top_card_of_test(&self, st: &St) -> bool {
        let shape = {
            let s = st.borrow();
            s.open && s.opened_by_test && !s.modal && !s.lookup && s.host.is_none() && s.parts.is_empty()
        };
        shape && self.is_card(st) && self.has_source(st) && !self.page_rec(st).borrow().temporary
    }
}

/// The statements of `body` without the position marks.
fn plain(body: &[ir::Stmt]) -> Vec<&ir::Stmt> {
    body.iter().filter(|s| !matches!(s, ir::Stmt::Pos(_))).collect()
}

/// `body` is calls, then `if <condition> then exit(false);` (RecordTriggerReturnValue.al:72-77, :79-84).
fn only_exit_false_last(body: &[ir::Stmt]) -> bool {
    let stmts = plain(body);
    let Some((last, before)) = stmts.split_last() else {
        return false;
    };
    let ir::Stmt::If(_, then, otherwise) = last else {
        return false;
    };
    before.iter().all(|s| matches!(s, ir::Stmt::Expr(_)))
        && otherwise.is_empty()
        && matches!(
            plain(then).as_slice(),
            [ir::Stmt::Exit(Some(ir::Expr::Const(ir::Const::Boolean(false))))]
        )
}

/// `body` is calls, then `<named return value> := false;` (RecordTriggerReturnValue.al:117-121, :123-127).
fn assigns_false_last(body: &[ir::Stmt], local: usize) -> bool {
    let stmts = plain(body);
    let Some((last, before)) = stmts.split_last() else {
        return false;
    };
    before.iter().all(|s| matches!(s, ir::Stmt::Expr(_)))
        && matches!(
            last,
            ir::Stmt::Assign(ir::Place::Local(l), ir::Expr::Const(ir::Const::Boolean(false))) if *l == local
        )
}

/// Whether a statement of `body`, at any depth, assigns local `local`.
fn assigns(body: &[ir::Stmt], local: usize) -> bool {
    use ir::Stmt;
    body.iter().any(|s| match s {
        Stmt::Assign(ir::Place::Local(l), _) => *l == local,
        Stmt::If(_, a, b) => assigns(a, local) || assigns(b, local),
        Stmt::While(_, b) | Stmt::Repeat(b, _) | Stmt::AssertError(b) => assigns(b, local),
        Stmt::For { body, .. } | Stmt::Foreach { body, .. } => assigns(body, local),
        Stmt::Case {
            branches, otherwise, ..
        } => branches.iter().any(|(_, b)| assigns(b, local)) || assigns(otherwise, local),
        _ => false,
    })
}
