//! A List over a temporary source table whose action deleted the row it stood on while rows are left (al-runner issue 7940).
//! On the probe's read-only list BC lands on the row BEFORE the deleted one, where a list over a stored table lands on the row
//! after it (pages/W1Update.al:577): open corpus PR 1367, testpage/lifecycle/ReopenSameVariableState.al:469
//! TempListRowGone_CloseReopen_DeleteAgain (`cursor-1` R1, `del-1` `[del(R2) agcr(R1) agcr(R1)]`, `next-in-1` R3, the same on the
//! second open). Only that cell runs, under an assumption; every other such list is refused, because the landing main gives
//! it (the row after) is the one the probe showed wrong.

use super::pinned_shape::{PinnedCode, TestCall, code_stmts, rec_text_log_call};
use super::*;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::builtin_id::ids;
use al_builtins::page_op::PageMethod;

/// ReopenSameVariableState.al:151 "PG6 FL Temp List": OnOpenPage fills Rec, OnAfterGetCurrRecord and OnClosePage log, fields of
/// Rec in a repeater, one action whose OnAction deletes Rec; `Editable = false`, `SourceTableTemporary = true`; the table has
/// OnModify only (:33). `action_props` lets the action in: it sets ApplicationArea, never an expression.
const CODE: PinnedCode = PinnedCode {
    triggers: &["onopenpage", "onaftergetcurrrecord", "onclosepage"],
    control_triggers: &["onaction"],
    properties: &["editable", "sourcetabletemporary"],
    action_props: &["applicationarea"],
    table_triggers: &["onmodify"],
};

impl Session {
    /// Settles a top-level List over a temporary table after its action deleted the row Rec stands on: on the probe's cell it
    /// lands on the row before and reads it as main's landing does (`arrive_after_gone_row`); `false` when the page is not such
    /// a list or no row is left (main's path); refused in every other shape.
    pub(super) fn settle_gone_row_of_temp_list(&mut self, st: &St, rec: &Rec) -> Result<bool, Signal> {
        let object = st.borrow().object as usize;
        let family = self.engine.object_facts[object].page_type == Some(al_ir::PageType::List)
            && rec.borrow().temporary
            && st.borrow().host.is_none();
        if !family {
            return Ok(false);
        }
        let before = self.neighbor_row(rec, -1)?;
        let after = self.neighbor_row(rec, 1)?;
        if before.is_none() && after.is_none() {
            return Ok(false);
        }
        let middle_of_three = before.is_some() && after.is_some() && self.view_row_count(st)? == 2;
        if !(middle_of_three && self.gone_middle_cell(st, rec)?) {
            return unsupported(
                "a list over a temporary table after its action deleted the row it stood on, rows left (measured only for the probe's read-only list, middle row of three)",
            );
        }
        assume(Assumption::TempReadOnlyListGoneMiddleRowLandsBefore);
        if let Some(fields) = before {
            rec.borrow_mut().fields = fields;
        }
        let r = self.arrive_after_gone_row(st);
        self.torn_down_on_error(st, r)?;
        Ok(true)
    }

    /// The facts of the probe's cell, each by equality:
    /// - the page's code is `CODE` (`page_code_is`), `Editable = false`, at the top level without parts, opened by the test with
    ///   OpenView, not modal, not a lookup;
    /// - the triggers have the probe's bodies (`probe_bodies`), so the open stood on the last row of the view, and the action
    ///   asked for no refresh (`refresh` is false);
    /// - the view is the primary key without filters or marks (`rec_view_is`), and the client held the whole view of three rows
    ///   from its start, Rec's key the middle one;
    /// - the test's calls since the open: `GoToKey`, then the action's `Invoke`, with one control read before the `GoToKey` on the
    ///   second open (:474-477, :488-492).
    fn gone_middle_cell(&mut self, st: &St, rec: &Rec) -> Result<bool, Signal> {
        let object = st.borrow().object as usize;
        let opened = {
            let s = st.borrow();
            s.open
                && !s.refresh
                && s.opened_by_test
                && s.mode == PageMode::View
                && !s.modal
                && !s.lookup
                && s.parts.is_empty()
                && s.window_from_start
                && s.window.len() == 3
        };
        if !(opened && flag_denied(&self.engine(), object, "Editable")? && self.rec_view_is(st, 0)) {
            return Ok(false);
        }
        let key = self.row_key(rec)?;
        if !Self::same_key(&st.borrow().window[1].key, &key) {
            return Ok(false);
        }
        let calls = matches!(
            Self::test_calls(st).as_deref(),
            Some(
                [TestCall::Page(PageMethod::GoToKey), TestCall::Other(PageMethod::Invoke)]
                    | [
                        TestCall::Read,
                        TestCall::Page(PageMethod::GoToKey),
                        TestCall::Other(PageMethod::Invoke)
                    ]
            )
        );
        Ok(calls && self.page_code_is(st, &CODE)? && probe_bodies(&self.engine().program, object))
    }
}

/// The page's triggers have the bodies of ReopenSameVariableState.al:151-212, the page has no other routine, and the action
/// with the OnAction is its only action (:172-181):
/// - OnOpenPage (:184-194): `if Row.FindSet() then repeat Rec := Row; Rec.Insert(); until Row.Next() = 0;` with `Row` a local
///   record, then one log call (`rec_text_log_call`): Rec ends on the last row it filled;
/// - the action's OnAction (:174-178): one log call, then `Rec.Delete()` without argument;
/// - OnAfterGetCurrRecord and OnClosePage (:196-206): one log call.
fn probe_bodies(program: &ir::Program, object: usize) -> bool {
    let o = &program.objects[object];
    let (Some(rec), Some(page)) = (o.rec_slot, o.page.as_ref()) else {
        return false;
    };
    let log = |s: &ir::Stmt| matches!(s, ir::Stmt::Expr(e) if rec_text_log_call(program, object, e));
    let on_rec = |e: &ir::Expr, want, args: &[ir::Arg]| {
        matches!(e, ir::Expr::Builtin { id, receiver: Some(r), args: a, .. } if *id == want && a.as_slice() == args
            && matches!(&**r, ir::Arg::Value(ir::Expr::Load(ir::Place::Global(g))) | ir::Arg::Ref(ir::Place::Global(g)) if *g == rec))
    };
    let body = |r: usize| code_stmts(program, object, r);
    let actions: Vec<usize> = page
        .controls
        .iter()
        .flat_map(|c| c.triggers.iter().map(|(_, r)| *r))
        .collect();
    let routines = page.triggers.len() + actions.len();
    let one_action = page
        .controls
        .iter()
        .filter(|c| c.kind == ir::ControlKind::Action)
        .map(|c| c.triggers.len())
        .eq([1]);
    let action = matches!(actions.as_slice(), [r] if matches!(body(*r).as_slice(),
        [l, ir::Stmt::Expr(d)] if log(l) && on_rec(d, ids::RECORD_DELETE_1, &[])));
    let triggers = page
        .triggers
        .iter()
        .all(|(name, r)| match (name.as_str(), body(*r).as_slice()) {
            ("onopenpage", [ir::Stmt::If(find, then, otherwise), l]) => {
                otherwise.is_empty() && log(l) && fill_loop(program, object, *r, rec, find, then)
            }
            ("onaftergetcurrrecord" | "onclosepage", [l]) => log(l),
            _ => false,
        });
    o.routines.len() == routines && one_action && action && triggers
}

/// `if <find> then <then>` of OnOpenPage (routine `r`) is `if Row.FindSet() then repeat Rec := Row; Rec.Insert(); until
/// Row.Next() = 0;` with `Row` a local record of the routine and Rec global slot `rec`.
fn fill_loop(program: &ir::Program, object: usize, r: usize, rec: usize, find: &ir::Expr, then: &[ir::Stmt]) -> bool {
    let locals = &program.objects[object].routines[r].locals;
    let local_of = |a: &ir::Arg| match a {
        ir::Arg::Value(ir::Expr::Load(ir::Place::Local(l))) | ir::Arg::Ref(ir::Place::Local(l)) => locals
            .get(*l)
            .is_some_and(|s| matches!(s.ty, RtType::Record(..)))
            .then_some(*l),
        _ => None,
    };
    fn builtin(e: &ir::Expr, want: ir::BuiltinId) -> Option<&ir::Arg> {
        match e {
            ir::Expr::Builtin {
                id,
                receiver: Some(r),
                args,
                ..
            } if *id == want && args.is_empty() => Some(r),
            _ => None,
        }
    }
    fn strip(e: &ir::Expr) -> &ir::Expr {
        match e {
            ir::Expr::Convert(_, inner) => inner,
            e => e,
        }
    }
    let Some(row) = builtin(find, ids::RECORD_FINDSET_1).and_then(local_of) else {
        return false;
    };
    let stmts: Vec<&ir::Stmt> = then.iter().filter(|s| !matches!(s, ir::Stmt::Pos(_))).collect();
    let [ir::Stmt::Repeat(body, until)] = stmts.as_slice() else {
        return false;
    };
    let body: Vec<&ir::Stmt> = body.iter().filter(|s| !matches!(s, ir::Stmt::Pos(_))).collect();
    let copy = |s: &ir::Stmt| {
        matches!(s, ir::Stmt::Assign(ir::Place::Global(g), e) if *g == rec
            && matches!(strip(e), ir::Expr::Load(ir::Place::Local(l)) if *l == row))
    };
    let insert = |s: &ir::Stmt| {
        matches!(s, ir::Stmt::Expr(e) if builtin(e, ids::RECORD_INSERT_1).is_some_and(|r|
            matches!(r, ir::Arg::Value(ir::Expr::Load(ir::Place::Global(g))) | ir::Arg::Ref(ir::Place::Global(g)) if *g == rec)))
    };
    let next_is_zero = match until {
        ir::Expr::Binary(ir::BinOp::Eq, a, b) => {
            builtin(strip(a), ids::RECORD_NEXT_1).and_then(local_of) == Some(row)
                && matches!(strip(b), ir::Expr::Const(ir::Const::Integer(0)))
        }
        _ => false,
    };
    matches!(body.as_slice(), [c, i] if copy(c) && insert(i)) && next_is_zero
}
