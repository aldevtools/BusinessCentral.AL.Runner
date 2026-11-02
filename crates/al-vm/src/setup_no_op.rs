//! A `GetDatabaseTableTriggerSetup` subscriber whose install order is not known and that changes nothing (al-runner
//! issue 6718).
//!
//! While an app installs, a subscriber of an app with no dependency path to it runs only if BC installed its app first
//! (ADR-046 amendment 6: install/lt-install-order-subscribers/InstallOrderSubscriberTests.al:66-79, :104-117), and that
//! order is not known, so the event is refused (`dispatch.rs`, `order_unknown_apps`). For the database trigger setup of
//! one table the refusal is not needed when the subscriber leaves every flag as it gets it: then the setup BC answers is
//! the same whether the subscriber runs or not, in either order and wherever it stands among the other subscribers.
//!
//! The runtime shows that in two steps. The subscriber's code only computes and assigns its own parameters and local
//! variables from constants and those variables (no call, no record, no global, no loop): it has no effect outside its
//! `var` flags and its result depends only on the table id and the flags it gets. Then it is run, for the table asked
//! about, on all 16 combinations of the four flags, and must give each back unchanged.

use crate::Session;
use al_builtins::value::{RtType, Value};
use al_ir as ir;
use std::collections::BTreeSet;

/// The event this module answers for.
const EVENT: &str = "getdatabasetabletriggersetup";
/// The parameters of `Global Triggers.GetDatabaseTableTriggerSetup`, the table id first.
const PARAMS: [&str; 5] = [
    "tableid",
    "ondatabaseinsert",
    "ondatabasemodify",
    "ondatabasedelete",
    "ondatabaserename",
];

/// Whether routine `r` of `object` only assigns its own parameters and locals from constants and those variables.
fn flags_only(program: &ir::Program, object: usize, r: usize) -> bool {
    let Some(routine) = program.objects[object].routines.get(r) else {
        return false;
    };
    let scalar = |l: usize| {
        routine
            .locals
            .get(l)
            .is_some_and(|s| matches!(s.ty, RtType::Boolean | RtType::Integer))
    };
    stmts_flags_only(routine.body.stmts(), &scalar)
}

fn stmts_flags_only(stmts: &[ir::Stmt], scalar: &dyn Fn(usize) -> bool) -> bool {
    stmts.iter().all(|s| match s {
        ir::Stmt::Pos(_) | ir::Stmt::Exit(None) => true,
        ir::Stmt::Assign(ir::Place::Local(l), e) => scalar(*l) && expr_flags_only(e, scalar),
        ir::Stmt::If(c, a, b) => {
            expr_flags_only(c, scalar) && stmts_flags_only(a, scalar) && stmts_flags_only(b, scalar)
        }
        ir::Stmt::Case {
            scrutinee,
            branches,
            otherwise,
        } => {
            expr_flags_only(scrutinee, scalar)
                && branches.iter().all(|(set, body)| {
                    set.iter().all(|e| match e {
                        ir::SetElem::One(x) => expr_flags_only(x, scalar),
                        ir::SetElem::Range(a, b) => [a, b]
                            .into_iter()
                            .flatten()
                            .all(|x| expr_flags_only(x, scalar)),
                    }) && stmts_flags_only(body, scalar)
                })
                && stmts_flags_only(otherwise, scalar)
        }
        _ => false,
    })
}

fn expr_flags_only(e: &ir::Expr, scalar: &dyn Fn(usize) -> bool) -> bool {
    let ok = |e: &ir::Expr| expr_flags_only(e, scalar);
    match e {
        ir::Expr::Const(c) => matches!(c, ir::Const::Boolean(_) | ir::Const::Integer(_)),
        ir::Expr::Load(ir::Place::Local(l)) => scalar(*l),
        ir::Expr::Unary(_, a) => ok(a),
        ir::Expr::Binary(_, a, b) => ok(a) && ok(b),
        _ => false,
    }
}

/// A flag or the table id as it went in and as it came back.
fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Boolean(x), Value::Boolean(y)) => x == y,
        (Value::Integer(x), Value::Integer(y)) => x == y,
        _ => false,
    }
}

impl Session {
    /// The subscribers to `GetDatabaseTableTriggerSetup` of the apps whose install order is not known that leave every flag
    /// unchanged for table `table_id`. The raise for that table leaves them out instead of refusing (module comment).
    pub(crate) fn setup_subscribers_changing_nothing(&mut self, table_id: i32) -> BTreeSet<(usize, usize)> {
        let mut quiet = BTreeSet::new();
        let Some(unknown) = self.events.order_unknown_apps.clone() else {
            return quiet;
        };
        let engine = self.engine.clone();
        let key = (engine.events.global_key, EVENT.to_string());
        let Some(subs) = engine.events.subscribers.get(&key) else {
            return quiet;
        };
        for s in subs {
            if s.manual || !unknown.contains(&engine.program.objects[s.object].app) {
                continue;
            }
            if flags_only(&engine.program, s.object, s.routine) && self.changes_nothing(s.object, s.routine, table_id) {
                quiet.insert((s.object, s.routine));
            }
        }
        quiet
    }

    /// Runs subscriber routine `r` of `object` for `table_id` on every combination of the flags: true when each comes
    /// back as it went in. Only called for a routine that `flags_only` accepts, which has no effect beyond its parameters.
    fn changes_nothing(&mut self, object: usize, r: usize, table_id: i32) -> bool {
        let engine = self.engine.clone();
        let routine = &engine.program.objects[object].routines[r];
        let Some(slots) = routine.locals[..routine.params]
            .iter()
            .map(|p| PARAMS.iter().position(|n| p.name.eq_ignore_ascii_case(n)))
            .collect::<Option<Vec<usize>>>()
        else {
            return false;
        };
        for flags in 0..16u8 {
            let value = |j: usize| match j {
                0 => Value::Integer(table_id),
                f => Value::Boolean(flags & (1 << (f - 1)) != 0),
            };
            let args: Vec<Value> = slots.iter().map(|&j| value(j)).collect();
            let inst = self.instance(object);
            match self.call(object, r, inst, args.clone()) {
                Ok((_, finals)) if finals.len() >= args.len() && args.iter().zip(finals.iter()).all(|(a, f)| same(a, f)) => {}
                _ => return false,
            }
        }
        true
    }
}
