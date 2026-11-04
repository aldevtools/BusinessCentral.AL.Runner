//! Subscribers whose guard exits for the value raised: their place in an unpinned cross-app order does not matter
//! (al-runner issue 6924).
//!
//! For apps that do not depend on each other BC visits the apps in ascending app id, each app's dependencies before it
//! (events/subscriber-order/BwSubscriberOrderTests.al, cited at `Session::event_targets`); whether the walk starts from
//! every app or only from the apps no other app depends on is not pinned, so an event whose subscribers come out in
//! another order under another reading is refused (`dispatch.rs`, `visit_orders`). The refusal is only needed where the
//! order can change a result, the same argument as for the install order (ADR-046, `empty_reads.rs`,
//! `setup_no_op.rs`): a subscriber that does nothing for this raise has the same effect wherever it runs.
//!
//! A subscriber is **guard-exited** for one raise when all of these hold:
//! 1. Its body (the `Pos` markers aside) starts with `if P <> C then exit;` or `if not (P = C) then exit;`, or is one
//!    `if P = C then begin ... end` without `else`, or one `case P of C1: ...; C2: ...; end` without `else`. P is a
//!    parameter of the subscriber, each C a constant (an enum value, option member or integer). Comparing an enum,
//!    option or integer value with a constant of its own type raises nothing and has no effect.
//! 2. The value raised for P is not one of the constants, so the subscriber exits (or runs nothing) before any effect.
//! 3. No subscriber that is not guard-exited can change P's argument while the event runs: it does not assign its
//!    parameter for that argument, pass it to a `var` parameter or a method receiver, or use it as a loop variable; and
//!    when the argument is a global of the publisher (shared with the publisher while the event runs), it makes no call
//!    at all. Then every guard sees the value the publisher passed, wherever it runs.
//!
//! The event is served when the subscribers that are not guard-exited come out in the same order under every reading;
//! all subscribers then run in the pinned reading's order. `Handled` guards (`if Handled then exit`) do not qualify:
//! whether they exit depends on the subscribers before them.

use crate::Session;
use crate::events::Subscriber;
use al_builtins::value::Value;
use al_ir as ir;

/// The arguments of one raise, as `raise_to_subscribers` holds them.
pub(crate) struct Raised<'a> {
    /// The publisher's parameter names, matching `args`.
    pub(crate) names: &'a [String],
    pub(crate) args: &'a [Value],
    /// Per argument: whether it is a global of the publisher (the same variable while the event runs).
    pub(crate) aliased: Vec<bool>,
}

/// The guard of a subscriber: it does nothing when parameter `param` holds none of `values`.
struct Guard {
    param: usize,
    values: Vec<ir::Const>,
}

/// The statements of `stmts` without the position markers.
fn code(stmts: &[ir::Stmt]) -> impl Iterator<Item = &ir::Stmt> {
    stmts.iter().filter(|s| !matches!(s, ir::Stmt::Pos(_)))
}

/// `P` and `C` of `P = C` or `C = P`, for a parameter `P` and a constant `C`.
fn param_eq_const(a: &ir::Expr, b: &ir::Expr, params: usize) -> Option<(usize, ir::Const)> {
    match (a, b) {
        (ir::Expr::Load(ir::Place::Local(l)), ir::Expr::Const(c))
        | (ir::Expr::Const(c), ir::Expr::Load(ir::Place::Local(l)))
            if *l < params && comparable(c) =>
        {
            Some((*l, c.clone()))
        }
        _ => None,
    }
}

fn comparable(c: &ir::Const) -> bool {
    matches!(c, ir::Const::Enum(..) | ir::Const::Option(..) | ir::Const::Integer(_))
}

/// The guard of `routine` (module comment, item 1).
fn guard(routine: &ir::Routine) -> Option<Guard> {
    let params = routine.params;
    let mut body = code(routine.body.stmts());
    let first = body.next()?;
    let only = body.next().is_none();
    match first {
        ir::Stmt::If(cond, then, otherwise) if otherwise.is_empty() => {
            let exits = {
                let mut t = code(then);
                matches!(t.next(), Some(ir::Stmt::Exit(None))) && t.next().is_none()
            };
            let ne = match cond {
                ir::Expr::Binary(ir::BinOp::Ne, a, b) => param_eq_const(a, b, params),
                ir::Expr::Unary(ir::UnOp::Not, x) => match &**x {
                    ir::Expr::Binary(ir::BinOp::Eq, a, b) => param_eq_const(a, b, params),
                    _ => None,
                },
                _ => None,
            };
            if exits && let Some((param, c)) = ne {
                return Some(Guard { param, values: vec![c] });
            }
            // The whole body is `if P = C then begin ... end`.
            match cond {
                ir::Expr::Binary(ir::BinOp::Eq, a, b) if only => {
                    param_eq_const(a, b, params).map(|(param, c)| Guard { param, values: vec![c] })
                }
                _ => None,
            }
        }
        // The whole body is `case P of C1: ...; end` without `else`.
        ir::Stmt::Case {
            scrutinee: ir::Expr::Load(ir::Place::Local(l)),
            branches,
            otherwise,
        } if only && *l < params && otherwise.is_empty() => {
            let mut values = Vec::new();
            for (set, _) in branches {
                for e in set {
                    match e {
                        ir::SetElem::One(ir::Expr::Const(c)) if comparable(c) => values.push(c.clone()),
                        _ => return None,
                    }
                }
            }
            Some(Guard { param: *l, values })
        }
        _ => None,
    }
}

/// Whether value `v` equals constant `c`: `None` when the two are not of one type.
fn equals(v: &Value, c: &ir::Const) -> Option<bool> {
    match (v, c) {
        (Value::Enum(t, o), ir::Const::Enum(ct, co)) if t == ct => Some(o == co),
        (Value::Option(o, _), ir::Const::Option(co, _)) => Some(o == co),
        (Value::Integer(i), ir::Const::Integer(ci)) => Some(i == ci),
        _ => None,
    }
}

/// Whether place `p` is local `l` or a part of it.
fn rooted_at(p: &ir::Place, l: usize) -> bool {
    match p {
        ir::Place::Local(x) => *x == l,
        ir::Place::Index(b, _) | ir::Place::Field(b, _) => rooted_at(b, l),
        _ => false,
    }
}

/// Whether the statements may write local `l` (module comment, item 3), or, with `no_calls`, make any call.
fn may_write(stmts: &[ir::Stmt], l: Option<usize>, no_calls: bool) -> bool {
    let writes = |p: &ir::Place| l.is_some_and(|l| rooted_at(p, l));
    let mut found = false;
    for s in stmts {
        found |= match s {
            ir::Stmt::Assign(p, _) | ir::Stmt::For { var: p, .. } | ir::Stmt::Foreach { var: p, .. } => writes(p),
            _ => false,
        };
        found |= match s {
            ir::Stmt::If(_, a, b) => may_write(a, l, no_calls) || may_write(b, l, no_calls),
            ir::Stmt::While(_, b)
            | ir::Stmt::Repeat(b, _)
            | ir::Stmt::For { body: b, .. }
            | ir::Stmt::Foreach { body: b, .. }
            | ir::Stmt::AssertError(b) => may_write(b, l, no_calls),
            ir::Stmt::Case {
                branches, otherwise, ..
            } => branches.iter().any(|(_, b)| may_write(b, l, no_calls)) || may_write(otherwise, l, no_calls),
            _ => false,
        };
    }
    let by_ref = |args: &[ir::Arg]| args.iter().any(|a| matches!(a, ir::Arg::Ref(p) if writes(p)));
    ir::types::for_each_expr_post(stmts, &mut |e| {
        found |= match e {
            ir::Expr::Call { args, .. } | ir::Expr::InterfaceCall { args, .. } | ir::Expr::DotNet { args, .. } => {
                no_calls || by_ref(args)
            }
            ir::Expr::Builtin { receiver, args, .. } => {
                no_calls || by_ref(args) || receiver.as_deref().is_some_and(|r| by_ref(std::slice::from_ref(r)))
            }
            ir::Expr::ControlAddInCall { .. } | ir::Expr::TddStub { .. } => no_calls,
            _ => false,
        };
    });
    found
}

impl Session {
    /// Per target: whether it is guard-exited for this raise (module comment).
    pub(crate) fn guard_exited(&self, targets: &[Subscriber], raised: &Raised) -> Vec<bool> {
        let objects = &self.engine.program.objects;
        let routine = |s: &Subscriber| objects.get(s.object).and_then(|o| o.routines.get(s.routine));
        // The argument a parameter of the subscriber receives, by name (as `raise_to_subscribers` binds it).
        let arg_of = |r: &ir::Routine, p: usize| {
            let name = &r.locals.get(p)?.name;
            raised.names.iter().position(|n| n.eq_ignore_ascii_case(name))
        };
        // Item 1 and 2: the argument each guard reads, for the subscribers whose guard exits for its value.
        let mut exited: Vec<Option<usize>> = targets
            .iter()
            .map(|s| {
                let r = routine(s)?;
                let g = guard(r)?;
                let j = arg_of(r, g.param)?;
                let v = raised.args.get(j)?;
                let mut hit = false;
                for c in &g.values {
                    hit |= equals(v, c)?;
                }
                (!hit).then_some(j)
            })
            .collect();
        // Item 3: a subscriber that still runs may change the argument a guard reads; then that guard is not trusted, and
        // its subscriber counts as running too.
        loop {
            let mut changed = false;
            for i in 0..targets.len() {
                let Some(j) = exited[i] else { continue };
                let spoiled = targets.iter().zip(&exited).any(|(s, e)| {
                    e.is_none()
                        && routine(s).is_none_or(|r| {
                            let p = (0..r.params).find(|&p| arg_of(r, p) == Some(j));
                            let aliased = raised.aliased.get(j).copied().unwrap_or(true);
                            may_write(r.body.stmts(), p, aliased)
                        })
                });
                if spoiled {
                    exited[i] = None;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        exited.iter().map(Option::is_some).collect()
    }
}
