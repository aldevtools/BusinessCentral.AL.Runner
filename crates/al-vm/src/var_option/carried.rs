//! A memberless `var` Option parameter handed on to a `var` Option parameter with members (al-runner issue 8971). BC's callee
//! reads the member list the value carries (corpus PR 1569, OptionVarMembersCallee.al and OptionVarMembersBatch.al headers);
//! the run models the measured cases through assumption var-option-carried-members, and refuses every other such call where
//! it is made:
//! - the value carries exactly the callee's list, from a variable with that list (VarHop_Same_*, BatchEvent_*): the usual
//!   binding gives the same;
//! - the value carries another list, from a variable with that list (VarHop_Other_*), or no list, from a memberless variable
//!   (VarHop_Memberless_*, BatchQueued_*): the parameter keeps the value's list (`next_carried_kept`, read by the binding
//!   in `lib.rs`; stores keep it, `var_option::param_store`, in the form only such a frame runs, `ExecProgram::frame_chunk`),
//!   for a callee whose every use of the parameter is one those cells measure (`uses_measured`, `may_keep_carried`).
//!
//! The other way, a `var` parameter with members handed to a memberless one, runs only to a callee that formats it
//! (`hand_on_args`).
//!
//! The origins are marked per frame (`option_carried_of`, `Frame::option_carried`), through memberless `var` hops and a
//! codeunit's `IntegrationEvent(true, false)` (`dispatch.rs`: a with-members origin, as `Carried::Event`), to a static
//! subscriber or a manual one under the measured binding (`manual_takes_carried`, OptionVarMembersBatchManual.al). Entry points:
//! `carried_args` and `hand_on_args` (`interp.rs::finish_call`), `option_carried_of`, `take_option_carried` (`lib.rs`,
//! the frame).

use al_builtins::assumptions::{Assumption, assume};
use al_builtins::builtin_id::ids;
use al_builtins::native::Signal;
use al_builtins::option_members::memberless;
use al_builtins::value::{RtType, Value};
use al_ir as ir;

type Inst = std::rc::Rc<std::cell::RefCell<crate::Instance>>;

/// Where the value of a memberless `var` Option parameter comes from (`option_carried_of`).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) enum Carried {
    /// No origin the cells measure.
    #[default]
    No,
    /// A `var`-section variable with members of a codeunit procedure (VarHop_Same_*, VarHop_Other_*, `RunBatch`).
    Members,
    /// `Members` through the measured event (BatchEvent_*: the variable has the callee's list).
    Event,
    /// A memberless `var`-section variable of a codeunit procedure (VarHop_Memberless_*, `RunQueued`).
    Memberless,
}

/// The refusal of a memberless `var` parameter handed on to a `var` Option parameter with members (`Session::carried_args`).
const CARRIED_REFUSED: &str = "an Option a `var` parameter declared without members hands on to a `var` Option parameter with members, from a caller the probe does not measure (not pinned)";

/// The refusal of a `var` parameter with members handed to a memberless `var` Option parameter (`Session::hand_on_args`).
const HAND_ON_REFUSED: &str = "a `var` parameter with members handed to a `var` Option parameter declared without members that the callee does more with than Format: a rejected Evaluate there is measured to leave the first member (not pinned)";

/// Whether routine `r` of an object of kind `kind`, with no attribute but those named in `allowed`, is one the carried-members cells
/// measure (corpus PR 1569, OptionVarMembersCallee.al, OptionVarMembersBatch.al): a codeunit's or a table's procedure. Every cell
/// hands the value on from and to procedures, never a trigger (`OnRun` as the origin refuses).
fn carried_routine(kind: ir::ObjKind, r: &ir::Routine, allowed: &[&str]) -> bool {
    matches!(kind, ir::ObjKind::Codeunit | ir::ObjKind::Table)
        && !r.is_trigger
        && r.attributes
            .iter()
            .all(|a| allowed.iter().any(|n| a.name.eq_ignore_ascii_case(n)))
}

/// Whether `carried_args` may bind parameter `i` of routine `r` of an object of kind `kind`, a `var` Option parameter with
/// members, to a value that carries another list or none: a codeunit's or a table's routine with no attribute whose uses
/// of the parameter are all measured (`uses_measured`). Its stores then keep that list (`exec/compile.rs`); any other
/// parameter's stores take the usual path.
pub(crate) fn may_keep_carried(program: &ir::Program, kind: ir::ObjKind, r: &ir::Routine, i: usize) -> bool {
    match (r.by_var.get(i), &r.locals[i].ty) {
        (Some(true), RtType::Option(own)) if !memberless(own) => {
            carried_routine(kind, r, &[]) && uses_measured(program, r, i, own).is_some()
        }
        _ => false,
    }
}

/// Parameter `i` of `r`, a `var` Option parameter declared with list `d`, handed `arg`, a `var` Option parameter `s` of
/// `caller`: `(d, s, whether s is memberless)`. `None` for any other parameter or argument.
fn var_option_pair<'a>(
    r: &'a ir::Routine,
    caller: &ir::Routine,
    i: usize,
    arg: &ir::Arg,
) -> Option<(&'a [String], usize, bool)> {
    let (Some(true), RtType::Option(d), ir::Arg::Ref(ir::Place::Local(s))) =
        (r.by_var.get(i).copied(), &r.locals[i].ty, arg)
    else {
        return None;
    };
    match &caller.locals[*s].ty {
        RtType::Option(m) if *s < caller.params && caller.by_var.get(*s) == Some(&true) => Some((d, *s, memberless(m))),
        _ => None,
    }
}

/// A use `uses_measured` lets through that `carried_args` checks against the value: an `Evaluate` into the parameter of a
/// literal text or of a by-value parameter of the callee (`measured_text`), and Clear, measured for a value that carries a
/// list only (CalleeOps_VarHop_Other `Clear inside` `>First|0`; from a value that carries none, BC leaves the callee's
/// list in the memberless callers, CalleeClear_VarHop_Memberless_ThenReuse `then 1` `Skip|1`, which the run does not
/// model).
#[derive(Clone, Debug)]
enum Checked {
    Literal(String),
    Param(usize),
    Clear,
}

/// Whether `Evaluate` of text `t` into a value that carries list `has` (another than the callee's, or none) is one the cells
/// measure (OptionVarMembersCallee.al CalleeOps_VarHop_Other, CalleeOps_VarHop_Memberless, CalleeErrors_VarHop_*): a name of
/// `has` as it is spelled (`Second`), digits (`5`), or a text that names no member of `has` in any case (`Skip`, `update`,
/// `Nope`). A blank text, blanks around it and another spelling of a member of `has` are not measured.
fn measured_text(t: &str, has: &[String]) -> bool {
    !t.is_empty()
        && t.trim() == t
        && (t.bytes().all(|b| b.is_ascii_digit())
            || has.iter().any(|m| m == t)
            || !has.iter().any(|m| m.eq_ignore_ascii_case(t)))
}

/// Whether every use of parameter `p` of routine `r` is `Format(p)` or `Format(p, 0, 2)`: the body of the measured hand-on
/// target (OptionVarMembersCallee.al `HandOnMemberless`, `HandOnByValue`).
/// A body that never reads it (an empty one) is not measured.
fn format_only(r: &ir::Routine, p: usize) -> bool {
    let mut read = false;
    ir::types::for_each_expr_post(r.body.stmts(), &mut |e| {
        read |= matches!(e, ir::Expr::Load(ir::Place::Local(l)) if *l == p)
    });
    read && uses(None, r, p, &[], &mut Vec::new())
}

/// Whether every use of parameter `p` (declared with members `own`) in the body of `r` is one the cells measure for a value
/// that carries another list or none, and the uses `carried_args` checks against the value (`Checked`):
/// `p = p::M` (OptionVarMembersCallee.al `Confirm`, OptionVarMembersBatch.al `ConfirmOnce`), `case p of` with own members as
/// the choices, `p := p::M`, `Format(p)` and `Format(p, 0, 2)`, `M` one of `own`; and (OptionVarMembersCallee.al `ConfirmOp`,
/// `EvaluateStatement`, `WriteThenError`): `Evaluate(p, t)` with `t` a literal or a by-value parameter, `Clear(p)`, `p := 1`,
/// `p` handed to a memberless `var` parameter or by value to a memberless parameter of a routine that only formats it.
/// Anything else (another read, another hand-on, a loop variable) is not measured for it.
fn uses_measured(program: &ir::Program, r: &ir::Routine, p: usize, own: &[String]) -> Option<Vec<Checked>> {
    let mut checks = Vec::new();
    uses(Some(program), r, p, own, &mut checks).then_some(checks)
}

/// The scan of `uses_measured` (`program`: `Some`) and of `format_only` (`None`: only the Format reads).
fn uses(program: Option<&ir::Program>, r: &ir::Routine, p: usize, own: &[String], checks: &mut Vec<Checked>) -> bool {
    let is_p = |e: &ir::Expr| matches!(e, ir::Expr::Load(ir::Place::Local(l)) if *l == p);
    // `p::M`, as a comparison holds it and as an assignment converts it to the parameter's type.
    let member = |e: &ir::Expr| matches!(e, ir::Expr::Const(ir::Const::Option(_, m)) if m.as_slice() == own);
    let own_member = |e: &ir::Expr| match e {
        ir::Expr::Convert(RtType::Option(m), x) => **m == *own && member(x),
        e => member(e),
    };
    // `p := 1` (ConfirmOp op 5).
    let own_one = |e: &ir::Expr| {
        matches!(e, ir::Expr::Convert(RtType::Option(m), x)
            if **m == *own && matches!(**x, ir::Expr::Const(ir::Const::Integer(1))))
    };
    let int = |a: &ir::Arg, n: i32| matches!(a, ir::Arg::Value(ir::Expr::Const(ir::Const::Integer(v))) if *v == n);
    let arg_is_p = |a: &ir::Arg| matches!(a, ir::Arg::Value(e) if is_p(e));
    let ref_p = |a: &ir::Arg| matches!(a, ir::Arg::Ref(ir::Place::Local(l)) if *l == p);
    // `p` converted to a memberless Option, handed by value.
    let by_value_p =
        |a: &ir::Arg| matches!(a, ir::Arg::Value(ir::Expr::Convert(RtType::Option(m), x)) if memberless(m) && is_p(x));
    let text = |a: &ir::Arg| match a {
        ir::Arg::Value(ir::Expr::Const(ir::Const::Text(t))) => Some(Checked::Literal(t.to_string())),
        ir::Arg::Value(ir::Expr::Load(ir::Place::Local(k)))
            if *k < r.params && r.by_var.get(*k) == Some(&false) && matches!(r.locals[*k].ty, RtType::Text(_)) =>
        {
            Some(Checked::Param(*k))
        }
        _ => None,
    };
    let (mut loads, mut measured, mut other) = (0usize, 0usize, false);
    let body = r.body.stmts();
    let more = program.is_some();
    let stored = |e: &ir::Expr| own_member(e) || (more && own_one(e));
    if !stmts_measured(body, p, &is_p, &own_member, &stored, &mut measured) {
        return false;
    }
    ir::types::for_each_expr_post(body, &mut |e| match e {
        _ if is_p(e) => loads += 1,
        ir::Expr::Binary(ir::BinOp::Eq, a, b) if is_p(a) && own_member(b) => measured += 1,
        ir::Expr::Builtin {
            id,
            receiver: None,
            args,
            ..
        } if (*id == ids::SYSTEM_FORMAT_1 || *id == ids::SYSTEM_FORMAT_2)
            && args.first().is_some_and(arg_is_p)
            && (args.len() == 1 || (args.len() == 3 && int(&args[1], 0) && int(&args[2], 2))) =>
        {
            measured += 1
        }
        ir::Expr::Builtin {
            id,
            receiver: None,
            args,
            ..
        } if more && *id == ids::SYSTEM_EVALUATEVARIABLEOPTION_1 && args.len() == 2 && ref_p(&args[0]) => {
            match text(&args[1]) {
                Some(t) => checks.push(t),
                None => other = true,
            }
        }
        ir::Expr::Builtin {
            id,
            receiver: None,
            args,
            ..
        } if more && *id == ids::SYSTEM_CLEAR_1 && args.len() == 1 && ref_p(&args[0]) => checks.push(Checked::Clear),
        ir::Expr::Call {
            object, routine, args, ..
        } if let Some(program) = program.filter(|_| args.iter().any(|a| ref_p(a) || by_value_p(a))) => {
            let t = &program.objects[*object];
            let target = &t.routines[*routine];
            for (j, a) in args.iter().enumerate() {
                let (ref_a, value_a) = (ref_p(a), by_value_p(a));
                if !ref_a && !value_a {
                    continue;
                }
                let ok = carried_routine(t.kind, target, &[])
                    && j < target.params
                    && target.by_var.get(j) == Some(&ref_a)
                    && matches!(&target.locals[j].ty, RtType::Option(m) if memberless(m))
                    && format_only(target, j);
                match (ok, value_a) {
                    (true, true) => measured += 1,
                    (true, false) => {}
                    (false, _) => other = true,
                }
            }
        }
        ir::Expr::Call { args, .. } | ir::Expr::InterfaceCall { args, .. } | ir::Expr::DotNet { args, .. } => {
            other |= args.iter().any(ref_p)
        }
        ir::Expr::Builtin { receiver, args, .. } => other |= args.iter().chain(receiver.as_deref()).any(ref_p),
        _ => {}
    });
    !other && loads == measured
}

/// The statement-level part of `uses`: `case p of` with choices `own_member` takes (counted in `measured`), `p := x` with `x`
/// one `stored` takes; false for any other statement that names `p` as its target.
fn stmts_measured(
    stmts: &[ir::Stmt],
    p: usize,
    is_p: &dyn Fn(&ir::Expr) -> bool,
    own_member: &dyn Fn(&ir::Expr) -> bool,
    stored: &dyn Fn(&ir::Expr) -> bool,
    measured: &mut usize,
) -> bool {
    let target = |pl: &ir::Place| matches!(pl, ir::Place::Local(l) if *l == p);
    stmts.iter().all(|s| match s {
        ir::Stmt::Assign(pl, e) if target(pl) => stored(e),
        ir::Stmt::For { var, .. } | ir::Stmt::Foreach { var, .. } if target(var) => false,
        ir::Stmt::Case {
            scrutinee,
            branches,
            otherwise,
        } => {
            if is_p(scrutinee) {
                if !branches
                    .iter()
                    .all(|(set, _)| set.iter().all(|x| matches!(x, ir::SetElem::One(e) if own_member(e))))
                {
                    return false;
                }
                *measured += 1;
            }
            branches
                .iter()
                .all(|(_, b)| stmts_measured(b, p, is_p, own_member, stored, measured))
                && stmts_measured(otherwise, p, is_p, own_member, stored, measured)
        }
        ir::Stmt::If(_, a, b) => {
            stmts_measured(a, p, is_p, own_member, stored, measured)
                && stmts_measured(b, p, is_p, own_member, stored, measured)
        }
        ir::Stmt::While(_, b)
        | ir::Stmt::Repeat(b, _)
        | ir::Stmt::For { body: b, .. }
        | ir::Stmt::Foreach { body: b, .. }
        | ir::Stmt::AssertError(b) => stmts_measured(b, p, is_p, own_member, stored, measured),
        _ => true,
    })
}

impl crate::Session {
    /// Whether the manual subscriber running on `inst` takes the publisher's origin marks (`dispatch.rs`, the raise): its
    /// codeunit had exactly one instance bound as the raise began (`bound_at_raise`, the manual targets' instances), and
    /// `BindSubscription` bound it with no subscriber running (`Binding::outside_subscriber`). Corpus PR 1569,
    /// OptionVarMembersBatchManual.al: bound in the OnRun of `Codeunit.Run` (ManualRun_* :180/:212), in a procedure
    /// (ManualBound_* :308/:340), by the test itself (ManualTestBound_* :436/:472). No cell binds two instances at once or
    /// binds inside a subscriber: those get no marks, and the hand-on refuses (`carried_args`).
    pub(crate) fn manual_takes_carried(&self, inst: &Inst, bound_at_raise: &[Inst]) -> bool {
        let object = inst.borrow().object;
        let mut bound: Vec<*const _> = bound_at_raise
            .iter()
            .filter(|i| i.borrow().object == object)
            .map(std::rc::Rc::as_ptr)
            .collect();
        bound.sort();
        bound.dedup();
        bound.len() == 1 && self.bindings_of(inst).any(|b| b.outside_subscriber)
    }

    /// The marks `finish_call` or `raise_to_subscribers` left for the frame of a routine with memberless `var` Option
    /// parameters (`option_carried_of`); empty for any other routine.
    pub(crate) fn take_option_carried(&mut self, wanted: bool) -> Vec<Carried> {
        if wanted {
            std::mem::take(&mut self.events.next_option_carried)
        } else {
            Vec::new()
        }
    }

    /// Per parameter of `routine` of `object`, called from frame `f` with `args`: where the value of its `var` Option parameter
    /// declared without members comes from (`Frame::option_carried`, read by `carried_args`; assumption
    /// var-option-carried-members, corpus PR 1569): a variable of the `var` section of a codeunit procedure (no attribute, or
    /// `Test`) with members (OptionVarMembersCallee.al VarHop_Same_*, VarHop_Other_*, OptionVarMembersBatch.al `RunBatch`) or without
    /// (VarHop_Memberless_*, `RunQueued`), or a memberless `var` parameter of the caller that holds one (the hops, two hops,
    /// the subscriber of BatchEvent_*). Not through an interface method (no cell hops through one).
    pub(crate) fn option_carried_of(
        &self,
        object: usize,
        routine: usize,
        args: &[ir::Arg],
        f: &crate::Frame,
    ) -> Vec<Carried> {
        if self.next_interface_call {
            return Vec::new();
        }
        let program = &self.engine.program;
        let r = &program.objects[object].routines[routine];
        let caller = &program.objects[f.object].routines[f.routine];
        let kind = program.objects[f.object].kind;
        (0..r.params.min(args.len()))
            .map(|i| {
                let (Some(true), RtType::Option(d)) = (r.by_var.get(i).copied(), &r.locals[i].ty) else {
                    return Carried::No;
                };
                if !memberless(d) {
                    return Carried::No;
                }
                match &args[i] {
                    ir::Arg::Ref(ir::Place::Local(s))
                        if *s >= caller.params
                            && Some(*s) != caller.ret_local
                            && kind == ir::ObjKind::Codeunit
                            && carried_routine(kind, caller, &["Test"]) =>
                    {
                        match &caller.locals[*s].ty {
                            RtType::Option(m) if memberless(m) => Carried::Memberless,
                            RtType::Option(_) => Carried::Members,
                            _ => Carried::No,
                        }
                    }
                    ir::Arg::Ref(ir::Place::Local(s))
                        if *s < caller.params && carried_routine(kind, caller, &["EventSubscriber"]) =>
                    {
                        f.option_carried.get(*s).copied().unwrap_or_default()
                    }
                    _ => Carried::No,
                }
            })
            .collect()
    }

    /// The refusal of a call of `routine` of `object` that hands frame `f`'s `var` Option parameter declared with members to its
    /// `var` Option parameter `i` declared without members (al-sema `var_option_arg_refusal` leaves a procedure call to the
    /// run). Runs, through assumption var-option-carried-members, when both routines are a codeunit's or a table's with no
    /// attribute and the callee only formats the parameter (`format_only`: OptionVarMembersCallee.al `ConfirmOp` op 6 to
    /// `HandOnMemberless`, `m Fifth|4`): the usual binding keeps the value's list (`var_option::var_binding`). Any other such
    /// callee refuses: after such a hop a rejected Evaluate leaves the first member (OptionVarMemberlessWrites.al:387).
    pub(crate) fn hand_on_args(
        &self,
        object: usize,
        routine: usize,
        args: &[ir::Arg],
        f: &crate::Frame,
    ) -> Result<(), Signal> {
        let program = &self.engine.program;
        let r = &program.objects[object].routines[routine];
        let caller = &program.objects[f.object].routines[f.routine];
        for (i, arg) in args.iter().enumerate().take(r.params) {
            let Some((d, _, false)) = var_option_pair(r, caller, i, arg) else {
                continue;
            };
            if !memberless(d) {
                continue;
            }
            if !(carried_routine(program.objects[object].kind, r, &[])
                && carried_routine(program.objects[f.object].kind, caller, &[])
                && format_only(r, i))
            {
                return Err(Signal::unsupported(HAND_ON_REFUSED));
            }
            assume(Assumption::VarOptionCarriedMembers);
        }
        Ok(())
    }

    /// The refusal of a call of `routine` of `object` (no publisher) that hands frame `f`'s memberless `var` Option
    /// parameter, holding `values[i]`, to its `var` Option parameter `i` with members (al-sema `var_option_arg_refusal` leaves
    /// it to the run). Runs, through assumption var-option-carried-members, for a callee and a caller of a codeunit or a table
    /// with no attribute (an interface method a codeunit implements included) and a value whose origin `option_carried_of`
    /// marked: with members and carrying exactly the callee's list (VarHop_Same_*, BatchEvent_*); with members and carrying
    /// another list (VarHop_Other_*, not through the event), or memberless and carrying no list (VarHop_Memberless_*,
    /// BatchQueued_*), the callee's uses of the parameter all measured (`uses_measured`): then the binding keeps the value's
    /// list (the returned marks, which `finish_call` hands on as `next_carried_kept`). Every other such argument refuses: an
    /// unmarked origin, a list the origin does not give (a memberless variable that holds one), an unmeasured use.
    pub(crate) fn carried_args(
        &self,
        object: usize,
        routine: usize,
        args: &[ir::Arg],
        values: &[Value],
        f: &crate::Frame,
    ) -> Result<Vec<bool>, Signal> {
        let program = &self.engine.program;
        let r = &program.objects[object].routines[routine];
        let caller = &program.objects[f.object].routines[f.routine];
        let mut kept = Vec::new();
        for i in 0..r.params.min(args.len()) {
            let Some((d, s, true)) = var_option_pair(r, caller, i, &args[i]) else {
                continue;
            };
            if memberless(d) {
                continue;
            }
            let routines = carried_routine(program.objects[object].kind, r, &[])
                && carried_routine(program.objects[f.object].kind, caller, &[]);
            let carried = f.option_carried.get(s).copied().unwrap_or_default();
            let list = match values.get(i) {
                Some(Value::Option(_, has)) => Some(has),
                _ => None,
            };
            let kept_uses = |has: &[String]| {
                uses_measured(program, r, i, d).is_some_and(|checks| {
                    checks.iter().all(|t| match t {
                        Checked::Clear => !memberless(has),
                        Checked::Literal(t) => measured_text(t, has),
                        Checked::Param(k) => values
                            .get(*k)
                            .and_then(al_builtins::native::text_of)
                            .is_some_and(|t| measured_text(t.as_str(), has)),
                    })
                })
            };
            let measured = routines
                && match (carried, list) {
                    (Carried::Members | Carried::Event, Some(has)) if has[..] == *d => true,
                    (Carried::Members, Some(has)) if !memberless(has) => kept_uses(has),
                    (Carried::Memberless, Some(has)) if memberless(has) => kept_uses(has),
                    _ => false,
                };
            if !measured {
                return Err(Signal::unsupported(CARRIED_REFUSED));
            }
            if list.is_some_and(|has| has[..] != *d) {
                kept.resize(i + 1, false);
                kept[i] = true;
            }
            assume(Assumption::VarOptionCarriedMembers);
        }
        Ok(kept)
    }
}
