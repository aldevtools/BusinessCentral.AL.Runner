//! What the client reads from the page's own Boolean OnInsertRecord or OnModifyRecord (al-runner issue 7628).
//!
//! - An `exit(value)` that ran is the answer: `exit(false)` vetoes (testpage/TestPageRecordTriggers.al:76
//!   OnInsertRecord_ReturningFalse_SuppressesTheInsert, pages/triggers/OnInsertRecordFalse.al).
//! - A trigger without any `exit(value)` and without a named return value does not veto (pages/currpage/
//!   PageUpdateNewRowCard.Page.al: OnInsertRecord assigns the key and exits nothing, and the row is saved).
//! - A named return value that ends true saves (open corpus PR 1367, pages/triggers/RecordTriggerReturnValue.al:446-541, the
//!   True cells; the rule main had).
//! - An `exit(value)` that did not run, without a named return value, saves (RecordTriggerReturnValue.al, the Cond cells; corpus
//!   testpage/CcPageLeads.al:341), and a named return value assigned false vetoes (the False cells). Both run only in the
//!   measured shapes (`probe_cell`, `mod_mode_cell`), under an assumption; every other shape of them is refused, because the
//!   earlier rule (the default value of an unnamed return is a veto, a named one never vetoes) is the one the probe showed
//!   wrong. A named return value with an `exit(value)` that did not run keeps the earlier answer, the value it ends with (not
//!   measured with such an exit).
//! - A named return value that is never assigned keeps the earlier answer (no veto): not measured either way.

use super::pinned_shape::{PinnedCode, TestCall, code_stmts, rec_text_log_call};
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
        let Some(local) = routine.ret_local else {
            if !super::has_value_exit(&routine.body) {
                return Ok(None);
            }
            // RecordTriggerReturnValue.al:252 Cond_Insert_OpenNewSetValueClose, :276 _Next, :301 Cond_Modify_SetValueNext,
            // :326 _Close: `if Blocked then exit(false);` with Blocked false saves.
            // corpus testpage/CcPageLeads.al:341 ModifyRecord_AssignsAnotherRecord agrees: its `case` runs no exit and sets Rec to
            // row 2, and the save leaves both rows as they were (`row1` a, `row2` b).
            if self.probe_cell(st, name, ProbeForm::Cond)? || self.mod_mode_cell(st, name)? {
                assume(Assumption::PageRecordTriggerExitNotTakenSaves);
                return Ok(None);
            }
            return unsupported(format!(
                "the page's {name} trigger ended without running its exit(value) (measured only for the probe's Card)"
            ));
        };
        // A named return value answers with the value it ends with, as before, also next to an `exit(value)` that did not run.
        if b || !assigns(&routine.body, local) {
            return Ok(None);
        }
        // RecordTriggerReturnValue.al:350 False_Insert_OpenNewSetValueClose, :373 _Next, :397 False_Modify_SetValueNext,
        // :422 _Close: `Ok := false;` vetoes.
        if self.probe_cell(st, name, ProbeForm::False)? {
            assume(Assumption::PageRecordTriggerNamedFalseVetoes);
            self.pages.named_false_insert = name == "oninsertrecord";
            return Ok(Some(false));
        }
        unsupported(format!(
            "the page's {name} trigger ended with its named return value set to false (measured only for the probe's Card)"
        ))
    }

    /// Whether the page, its code, its open and the test's calls are those of a cell of the probe:
    /// - a top-level Card over a stored table, without parts, with the probe's code (`CODE`), opened by the test, not modal;
    /// - the page has no routine beside its three record triggers, and each one has the body `form` (:72-90 the Cond card,
    ///   :117-135 the False card), so the trigger that runs earlier in the cell (OnInsertRecord before OnModifyRecord on
    ///   OpenNew) has it too;
    /// - OnInsertRecord on OpenNew after the calls `SetValue` of the key field, then `SetValue` of a field outside the key
    ///   (:258-261: the second SetValue inserts);
    /// - OnModifyRecord on OpenNew after those two calls and `Next` or `Close` (:283-287, :259-262), or on OpenEdit after
    ///   `GoToKey`, `SetValue` of a field outside the key and `Next` or `Close` (:308-312, :333-336).
    fn probe_cell(&mut self, st: &St, name: &str, form: ProbeForm) -> Result<bool, Signal> {
        let mode = st.borrow().mode;
        if !self.top_card_of_test(st) {
            return Ok(false);
        }
        if !self.page_code_is(st, &CODE)? {
            return Ok(false);
        }
        let bodies = {
            let engine = self.engine();
            let program = &engine.program;
            let object = st.borrow().object as usize;
            let triggers = &page_of(&engine, st).triggers;
            let routines = &program.objects[object].routines;
            routines.len() == triggers.len()
                && routines.iter().all(|r| r.is_trigger && r.field.is_none())
                && triggers
                    .iter()
                    .all(|(_, r)| probe_form(program, object, *r) == Some(form))
        };
        if !bodies {
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
        if !self.page_code_is(st, &MOD_MODE)? || !self.mod_mode_body(st) {
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

    /// The page has no routine beside its OnModifyRecord, whose body is the fixture's (CcFixtures.al:260-275): one call of a
    /// procedure of a codeunit held in a local variable, then one `case` without `else`, nothing after it.
    fn mod_mode_body(&self, st: &St) -> bool {
        let engine = self.engine();
        let program = &engine.program;
        let object = st.borrow().object as usize;
        let routines = &program.objects[object].routines;
        let [(_, r)] = page_of(&engine, st).triggers.as_slice() else {
            return false;
        };
        if routines.len() != 1 {
            return false;
        }
        match code_stmts(program, object, *r).as_slice() {
            [ir::Stmt::Expr(call), ir::Stmt::Case { otherwise, .. }] => {
                otherwise.is_empty() && local_codeunit_call(program, object, *r, call)
            }
            _ => false,
        }
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

/// The body forms of the probe's record triggers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProbeForm {
    /// RecordTriggerReturnValue.al:72-77, :79-84, :86-91: `Log.Add(..); if Blocked then exit(false);`, no named return value.
    Cond,
    /// :117-121, :123-127, :129-133: `Log.Add(..); Ok := false;`, `Ok` the named return value.
    False,
}

/// The form of routine `r` of page `object`: one log call (`rec_text_log_call`), then the last statement of the form; `None`
/// for every other body.
fn probe_form(program: &ir::Program, object: usize, r: usize) -> Option<ProbeForm> {
    let routine = &program.objects[object].routines[r];
    let stmts = code_stmts(program, object, r);
    let [ir::Stmt::Expr(call), last] = stmts.as_slice() else {
        return None;
    };
    if !rec_text_log_call(program, object, call) {
        return None;
    }
    let false_const = |e: &ir::Expr| matches!(e, ir::Expr::Const(ir::Const::Boolean(false)));
    match (last, routine.ret_local) {
        (ir::Stmt::If(cond, then, otherwise), None) => {
            let flag = matches!(cond, ir::Expr::Load(ir::Place::Global(g))
                if program.objects[object].globals.get(*g).is_some_and(|s| s.ty == RtType::Boolean));
            let exit_false = matches!(plain(then).as_slice(), [ir::Stmt::Exit(Some(e))] if false_const(e));
            (flag && exit_false && otherwise.is_empty()).then_some(ProbeForm::Cond)
        }
        (ir::Stmt::Assign(ir::Place::Local(l), e), Some(local)) => {
            (*l == local && false_const(e)).then_some(ProbeForm::False)
        }
        _ => None,
    }
}

/// `e` calls a procedure of a codeunit that a local variable of routine `r` of `object` holds, not as a try call.
fn local_codeunit_call(program: &ir::Program, object: usize, r: usize, e: &ir::Expr) -> bool {
    let locals = &program.objects[object].routines[r].locals;
    matches!(e, ir::Expr::Call { object: callee, instance: Some(inst), try_call: false, .. }
        if program.objects[*callee].kind == ir::ObjKind::Codeunit
            && matches!(&**inst, ir::Expr::Load(ir::Place::Local(l))
                if locals.get(*l).is_some_and(|s| matches!(s.ty, RtType::Codeunit(_)))))
}

/// The statements of `body` without the position marks.
fn plain(body: &[ir::Stmt]) -> Vec<&ir::Stmt> {
    body.iter().filter(|s| !matches!(s, ir::Stmt::Pos(_))).collect()
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
