//! A memberless `var` Option parameter handed on to a `var` Option parameter with members (al-runner issue 8971). BC's callee
//! reads the member list the value carries (corpus PR 1569, OptionVarMembersCallee.al and OptionVarMembersBatch.al headers);
//! the run models the measured cases through assumption var-option-carried-members, and refuses every other such call where
//! it is made:
//! - the value carries exactly the callee's list, from a variable with that list (VarHop_Same_*, BatchEvent_*): the usual
//!   binding gives the same;
//! - the value carries another list, from a variable with that list (VarHop_Other_*), or no list, from a memberless variable
//!   (VarHop_Memberless_*, BatchQueued_*): the parameter keeps the value's list (`next_carried_kept`, read by
//!   `var_binding_at`; stores keep it, `var_option::param_store`), for a callee whose every use of the parameter is one those
//!   cells measure (`uses_measured`).
//!
//! The origins are marked per frame (`option_carried_of`, `Frame::option_carried`), through memberless `var` hops and a
//! codeunit's `IntegrationEvent(true, false)` (`dispatch.rs`: a with-members origin, as `Carried::Event`). Entry points: `carried_args`
//! (`interp.rs::finish_call`), `option_carried_of`, `take_option_carried` (`lib.rs`, the frame), `var_binding_at` (`lib.rs`,
//! the binding).

use al_builtins::assumptions::{Assumption, assume};
use al_builtins::builtin_id::ids;
use al_builtins::native::Signal;
use al_builtins::option_members::memberless;
use al_builtins::value::{RtType, Value};
use al_ir as ir;

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

/// Whether routine `r` of an object of kind `kind`, with no attribute but those named in `allowed`, is one the carried-members cells
/// measure (corpus PR 1569, OptionVarMembersCallee.al, OptionVarMembersBatch.al): a codeunit's or a table's.
fn carried_routine(kind: ir::ObjKind, r: &ir::Routine, allowed: &[&str]) -> bool {
    matches!(kind, ir::ObjKind::Codeunit | ir::ObjKind::Table)
        && r.attributes
            .iter()
            .all(|a| allowed.iter().any(|n| a.name.eq_ignore_ascii_case(n)))
}

/// Whether every use of parameter `p` (declared with members `own`) in the body of `r` is one the cells measure for a value
/// that carries another list or none: `p = p::M` (OptionVarMembersCallee.al `Confirm`, OptionVarMembersBatch.al `ConfirmOnce`), `case p of`
/// with own members as the choices, `p := p::M`, `Format(p)` and `Format(p, 0, 2)`, `M` one of `own`. Anything else (another
/// read, a `var` hand-on, Evaluate, Clear, a loop variable) is not measured for it.
fn uses_measured(r: &ir::Routine, p: usize, own: &[String]) -> bool {
    let is_p = |e: &ir::Expr| matches!(e, ir::Expr::Load(ir::Place::Local(l)) if *l == p);
    // `p::M`, as a comparison holds it and as an assignment converts it to the parameter's type.
    let member = |e: &ir::Expr| matches!(e, ir::Expr::Const(ir::Const::Option(_, m)) if m.as_slice() == own);
    let own_member = |e: &ir::Expr| match e {
        ir::Expr::Convert(RtType::Option(m), x) => **m == *own && member(x),
        e => member(e),
    };
    let int = |a: &ir::Arg, n: i32| matches!(a, ir::Arg::Value(ir::Expr::Const(ir::Const::Integer(v))) if *v == n);
    let arg_is_p = |a: &ir::Arg| matches!(a, ir::Arg::Value(e) if is_p(e));
    let ref_p = |a: &ir::Arg| matches!(a, ir::Arg::Ref(ir::Place::Local(l)) if *l == p);
    let (mut loads, mut measured, mut other) = (0usize, 0usize, false);
    let body = r.body.stmts();
    if !stmts_measured(body, p, &is_p, &own_member, &mut measured) {
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
        ir::Expr::Call { args, .. } | ir::Expr::InterfaceCall { args, .. } | ir::Expr::DotNet { args, .. } => {
            other |= args.iter().any(ref_p)
        }
        ir::Expr::Builtin { receiver, args, .. } => other |= args.iter().chain(receiver.as_deref()).any(ref_p),
        _ => {}
    });
    !other && loads == measured
}

/// The statement-level part of `bare_uses_measured`: `case p of` (counted in `measured`), `p := p::M`; false for any other
/// statement that names `p` as its target.
fn stmts_measured(
    stmts: &[ir::Stmt],
    p: usize,
    is_p: &dyn Fn(&ir::Expr) -> bool,
    own_member: &dyn Fn(&ir::Expr) -> bool,
    measured: &mut usize,
) -> bool {
    let target = |pl: &ir::Place| matches!(pl, ir::Place::Local(l) if *l == p);
    stmts.iter().all(|s| match s {
        ir::Stmt::Assign(pl, e) if target(pl) => own_member(e),
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
                .all(|(_, b)| stmts_measured(b, p, is_p, own_member, measured))
                && stmts_measured(otherwise, p, is_p, own_member, measured)
        }
        ir::Stmt::If(_, a, b) => {
            stmts_measured(a, p, is_p, own_member, measured) && stmts_measured(b, p, is_p, own_member, measured)
        }
        ir::Stmt::While(_, b)
        | ir::Stmt::Repeat(b, _)
        | ir::Stmt::For { body: b, .. }
        | ir::Stmt::Foreach { body: b, .. }
        | ir::Stmt::AssertError(b) => stmts_measured(b, p, is_p, own_member, measured),
        _ => true,
    })
}

impl crate::Session {
    /// The marks `finish_call` or `raise_to_subscribers` left for the frame of a routine with memberless `var` Option
    /// parameters (`option_carried_of`); empty for any other routine.
    pub(crate) fn take_option_carried(&mut self, wanted: bool) -> Vec<Carried> {
        if wanted {
            std::mem::take(&mut self.events.next_option_carried)
        } else {
            Vec::new()
        }
    }

    /// The binding of `var` parameter `i` of type `ty` to `v` (`lib.rs`): as it is when `carried_args` let a value that
    /// carries another list or none through to it (`next_carried_kept`), else `var_option::var_binding`.
    pub(crate) fn var_binding_at(
        &self,
        i: usize,
        v: &Value,
        ty: &RtType,
        subscriber: bool,
    ) -> Option<Result<Value, Signal>> {
        if self.events.next_carried_kept.get(i) == Some(&true) {
            return Some(Ok(v.clone()));
        }
        super::var_binding(v, ty, subscriber)
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
        let program = &self.engine.program;
        let r = &program.objects[object].routines[routine];
        let caller = &program.objects[f.object].routines[f.routine];
        let kind = program.objects[f.object].kind;
        (0..r.params.min(args.len()))
            .map(|i| {
                let (Some(true), RtType::Option(d)) = (r.by_var.get(i).copied(), &r.locals[i].ty) else {
                    return Carried::No;
                };
                if !memberless(d) || self.next_interface_call {
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

    /// The refusal of a call of `routine` of `object` (no publisher) that hands frame `f`'s memberless `var` Option
    /// parameter, holding `values[i]`, to its `var` Option parameter `i` with members (al-sema `var_option_arg_refusal` leaves
    /// it to the run). Runs, through assumption var-option-carried-members, for a callee and a caller of a codeunit or a table
    /// with no attribute (an interface method a codeunit implements included) and a value whose origin `option_carried_of`
    /// marked: with members and carrying exactly the callee's list (VarHop_Same_*, BatchEvent_*); with members and carrying
    /// another list (VarHop_Other_*, not through the event), or memberless and carrying no list (VarHop_Memberless_*,
    /// BatchQueued_*), the callee's uses of the parameter all measured (`uses_measured`): then the binding keeps the value's
    /// list (`next_carried_kept`). Every other such argument refuses: an unmarked origin, a list the origin does not give
    /// (a memberless variable that holds one), an unmeasured use.
    pub(crate) fn carried_args(
        &mut self,
        object: usize,
        routine: usize,
        args: &[ir::Arg],
        values: &[Value],
        f: &crate::Frame,
    ) -> Result<(), Signal> {
        let engine = self.engine.clone();
        let program = &engine.program;
        let r = &program.objects[object].routines[routine];
        let caller = &program.objects[f.object].routines[f.routine];
        let mut kept = Vec::new();
        for i in 0..r.params.min(args.len()) {
            let (Some(true), RtType::Option(d), ir::Arg::Ref(ir::Place::Local(s))) =
                (r.by_var.get(i).copied(), &r.locals[i].ty, &args[i])
            else {
                continue;
            };
            let hop = *s < caller.params
                && caller.by_var.get(*s) == Some(&true)
                && matches!(&caller.locals[*s].ty, RtType::Option(m) if memberless(m));
            if memberless(d) || !hop {
                continue;
            }
            let routines = carried_routine(program.objects[object].kind, r, &[])
                && carried_routine(program.objects[f.object].kind, caller, &[]);
            let carried = f.option_carried.get(*s).copied().unwrap_or_default();
            let list = match values.get(i) {
                Some(Value::Option(_, has)) => Some(has),
                _ => None,
            };
            let measured = routines
                && match (carried, list) {
                    (Carried::Members | Carried::Event, Some(has)) if has == d => true,
                    (Carried::Members, Some(has)) if !memberless(has) => uses_measured(r, i, d),
                    (Carried::Memberless, Some(has)) if memberless(has) => uses_measured(r, i, d),
                    _ => false,
                };
            if !measured {
                return Err(Signal::unsupported(CARRIED_REFUSED));
            }
            if list.is_some_and(|has| has != d) {
                kept.resize(i + 1, false);
                kept[i] = true;
            }
            assume(Assumption::VarOptionCarriedMembers);
        }
        self.events.next_carried_kept = kept;
        Ok(())
    }
}
