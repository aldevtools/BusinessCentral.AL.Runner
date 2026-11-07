//! A database trigger event subscriber whose install order is not known and that does nothing for the table raised (al-runner
//! issue 6852).
//!
//! While an app installs, a subscriber of an app with no dependency path to it runs only if BC installed its app first
//! (ADR-046 amendment 6: install-upgrade/install-order/InstallOrderSubscriberTests.al:66-79, :104-117), and that
//! order is not known, so the event is refused (`dispatch.rs`, `order_unknown_apps`). The refusal is only needed where the
//! order can change a result. A subscriber to `Global Triggers.OnDatabaseInsert`, `OnDatabaseModify`, `OnDatabaseDelete`,
//! `OnDatabaseRename` or `OnGlobalRename` for which the code that runs for this table has no effect writes nothing and
//! raises nothing, so the result is the same whether it runs or not (ADR-046 amendment 6, item 6 gives the same argument
//! for the setup event).
//!
//! "No effect" is shown statically for the table id raised. The statements that run for it may only: assign a Boolean or
//! Integer local from constants, locals and `RecRef.Number()`; exit; and branch (`if`, `case`) on such values. A branch the
//! table id decides is followed; one it does not decide must have every arm inert. Anything else (a call, a record or
//! `RecordRef` operation, a loop, a global) is not inert, and the event stays refused.
//!
//! A read of a table of the subscriber's own app that is empty in either install order is decided too (empty_reads.rs,
//! al-runner issue 6924): `if not Reg.Get(RecRef.Number()) then exit;` exits for every table, so what follows does not
//! run. The same check serves the setup event (setup_no_op.rs) and any other event (`Session::inert_in_either_order`).

use crate::Session;
use crate::empty_reads::{Answer, EmptyReads, KnownValue};
use al_builtins::value::RtType;
use al_ir as ir;
use std::collections::BTreeSet;

/// The events this module answers for.
const EVENTS: [&str; 5] = [
    "ondatabaseinsert",
    "ondatabasemodify",
    "ondatabasedelete",
    "ondatabaserename",
    "onglobalrename",
];

pub(crate) struct Cx<'a> {
    pub(crate) routine: &'a ir::Routine,
    /// The table id raised, when known.
    pub(crate) table: Option<i32>,
    /// The parameter that holds the table id (`TableId` of the setup event).
    pub(crate) table_param: Option<usize>,
    pub(crate) number: Option<ir::BuiltinId>,
    pub(crate) reads: EmptyReads,
}

impl Cx<'_> {
    fn scalar(&self, l: usize) -> bool {
        self.routine
            .locals
            .get(l)
            .is_some_and(|s| matches!(s.ty, RtType::Boolean | RtType::Integer))
    }

    /// A local that may be assigned: a scalar that is not a parameter (a parameter can be `var`, and the setup event's
    /// flags are its result).
    fn assignable(&self, l: usize) -> bool {
        self.scalar(l) && l >= self.routine.params
    }

    /// The type of a key or filter value, when evaluating it has no effect.
    fn value_type(&self, e: &ir::Expr) -> Option<RtType> {
        match e {
            ir::Expr::Const(ir::Const::Integer(_)) => Some(RtType::Integer),
            ir::Expr::Const(ir::Const::Boolean(_)) => Some(RtType::Boolean),
            ir::Expr::Load(ir::Place::Local(l)) if self.scalar(*l) => Some(self.routine.locals[*l].ty.clone()),
            _ if self.is_number(e) => Some(RtType::Integer),
            _ => None,
        }
    }

    /// What `e` gives when it reads an empty table of the subscriber's own app.
    fn read(&self, e: &ir::Expr) -> Option<Answer> {
        self.reads
            .answer(self.routine, e, &|x| self.value_type(x), &|x, ty| self.known(x, ty))
    }

    /// The value of key or filter value `e` of a field of type `ty`, when the table id decides it.
    fn known(&self, e: &ir::Expr, ty: &RtType) -> Option<KnownValue> {
        match ty {
            RtType::Integer => self.num(e).map(KnownValue::Int),
            RtType::Boolean => self.truth(e).map(KnownValue::Bool),
            _ => None,
        }
    }

    /// Runs `f` as a branch the table id does not decide: filters set there are not followed (empty_reads.rs).
    fn undecided<T>(&self, f: impl FnOnce() -> T) -> T {
        self.reads.undecided(true);
        let out = f();
        self.reads.undecided(false);
        out
    }

    /// `Rec.IsTemporary()` of a record parameter (a table event's `Rec`): no effect, its answer not known here.
    fn is_temporary(&self, e: &ir::Expr) -> bool {
        let ir::Expr::Builtin { id, receiver, args, .. } = e else {
            return false;
        };
        Some(*id) == ir::BuiltinId::of("Record.IsTemporary#1")
            && args.is_empty()
            && matches!(
                receiver.as_deref(),
                Some(ir::Arg::Ref(ir::Place::Local(l)) | ir::Arg::Value(ir::Expr::Load(ir::Place::Local(l))))
                    if *l < self.routine.params
            )
    }

    /// `RecRef.Number()` of a record reference parameter.
    fn is_number(&self, e: &ir::Expr) -> bool {
        let ir::Expr::Builtin { id, receiver, args, .. } = e else {
            return false;
        };
        Some(*id) == self.number
            && args.is_empty()
            && matches!(
                receiver.as_deref(),
                Some(ir::Arg::Ref(ir::Place::Local(l)) | ir::Arg::Value(ir::Expr::Load(ir::Place::Local(l))))
                    if *l < self.routine.params
            )
    }

    /// Whether evaluating `e` has no effect.
    fn pure(&self, e: &ir::Expr) -> bool {
        match e {
            ir::Expr::Const(c) => matches!(c, ir::Const::Boolean(_) | ir::Const::Integer(_)),
            ir::Expr::Load(ir::Place::Local(l)) => self.scalar(*l),
            ir::Expr::Unary(_, a) => self.pure(a),
            ir::Expr::Binary(_, a, b) => self.pure(a) && self.pure(b),
            ir::Expr::In(a, set) => self.pure(a) && set.iter().all(|s| self.pure_elem(s)),
            ir::Expr::Builtin { .. } => self.is_number(e) || self.is_temporary(e) || self.read(e).is_some(),
            _ => false,
        }
    }

    fn pure_elem(&self, s: &ir::SetElem) -> bool {
        match s {
            ir::SetElem::One(x) => self.pure(x),
            ir::SetElem::Range(a, b) => [a, b].into_iter().flatten().all(|x| self.pure(x)),
        }
    }

    /// The integer `e` is for this table, when the table id alone decides it.
    fn num(&self, e: &ir::Expr) -> Option<i32> {
        match e {
            ir::Expr::Const(ir::Const::Integer(n)) => Some(*n),
            ir::Expr::Load(ir::Place::Local(l)) if Some(*l) == self.table_param => self.table,
            _ if self.is_number(e) => self.table,
            _ => match self.read(e) {
                Some(Answer::Int(n)) => Some(n),
                _ => None,
            },
        }
    }

    /// The Boolean `e` is for this table, when the table id alone decides it.
    fn truth(&self, e: &ir::Expr) -> Option<bool> {
        match e {
            ir::Expr::Const(ir::Const::Boolean(b)) => Some(*b),
            ir::Expr::Unary(ir::UnOp::Not, a) => self.truth(a).map(|b| !b),
            ir::Expr::Binary(op, a, b) => match op {
                ir::BinOp::And => match (self.truth(a), self.truth(b)) {
                    (Some(false), _) | (_, Some(false)) => Some(false),
                    (Some(true), Some(true)) => Some(true),
                    _ => None,
                },
                ir::BinOp::Or => match (self.truth(a), self.truth(b)) {
                    (Some(true), _) | (_, Some(true)) => Some(true),
                    (Some(false), Some(false)) => Some(false),
                    _ => None,
                },
                ir::BinOp::Eq | ir::BinOp::Ne => {
                    let (x, y) = (self.num(a)?, self.num(b)?);
                    Some((x == y) == matches!(op, ir::BinOp::Eq))
                }
                _ => None,
            },
            ir::Expr::In(a, set) => self.matches(self.num(a)?, set),
            _ => match self.read(e) {
                Some(Answer::Bool(b)) => Some(b),
                _ => None,
            },
        }
    }

    /// Whether `n` is in `set`: `None` when an element is not decided by the table id and none matches.
    fn matches(&self, n: i32, set: &[ir::SetElem]) -> Option<bool> {
        let mut unknown = false;
        for s in set {
            match s {
                ir::SetElem::One(x) => match self.num(x) {
                    Some(v) if v == n => return Some(true),
                    Some(_) => {}
                    None => unknown = true,
                },
                ir::SetElem::Range(a, b) => {
                    let lo = a.as_ref().map(|x| self.num(x));
                    let hi = b.as_ref().map(|x| self.num(x));
                    match (lo, hi) {
                        (Some(Some(l)), Some(Some(h))) => {
                            if (l..=h).contains(&n) {
                                return Some(true);
                            }
                        }
                        _ => unknown = true,
                    }
                }
            }
        }
        if unknown { None } else { Some(false) }
    }

    /// Whether the statements, run for this table, have no effect.
    pub(crate) fn inert(&self, stmts: &[ir::Stmt]) -> bool {
        self.run(stmts).is_some()
    }

    /// Runs the statements for this table: `None` when one of them may have an effect, else whether the run ended in an
    /// `exit` (the statements after it do not run).
    fn run(&self, stmts: &[ir::Stmt]) -> Option<bool> {
        for s in stmts {
            let exited = match s {
                ir::Stmt::Pos(_) => false,
                ir::Stmt::Exit(None) => true,
                ir::Stmt::Assign(ir::Place::Local(l), e) if self.assignable(*l) && self.pure(e) => false,
                ir::Stmt::Expr(e)
                    if self
                        .reads
                        .inert(self.routine, e, &|x| self.value_type(x), &|x, ty| self.known(x, ty)) =>
                {
                    false
                }
                ir::Stmt::If(c, a, b) if self.pure(c) => match self.truth(c) {
                    Some(true) => self.run(a)?,
                    Some(false) => self.run(b)?,
                    None => self.undecided(|| Some(self.run(a)? & self.run(b)?))?,
                },
                ir::Stmt::Case {
                    scrutinee,
                    branches,
                    otherwise,
                } if self.pure(scrutinee) && branches.iter().all(|(set, _)| set.iter().all(|s| self.pure_elem(s))) => {
                    self.run_case(scrutinee, branches, otherwise)?
                }
                _ => return None,
            };
            if exited {
                return Some(true);
            }
        }
        Some(false)
    }

    fn run_case(
        &self,
        scrutinee: &ir::Expr,
        branches: &[(Vec<ir::SetElem>, Vec<ir::Stmt>)],
        otherwise: &[ir::Stmt],
    ) -> Option<bool> {
        let all = || {
            self.undecided(|| {
                let mut exited = self.run(otherwise)?;
                for (_, body) in branches {
                    exited &= self.run(body)?;
                }
                Some(exited)
            })
        };
        let Some(n) = self.num(scrutinee) else {
            return all();
        };
        for (set, body) in branches {
            match self.matches(n, set) {
                Some(true) => return self.run(body),
                Some(false) => {}
                None => return all(),
            }
        }
        self.run(otherwise)
    }
}

impl Session {
    /// The subscribers to `event` (one of the database trigger events) of the apps whose install order is not known that do
    /// nothing for table `table_id`. The raise for that table leaves them out instead of refusing (module comment).
    pub(crate) fn db_event_subscribers_changing_nothing(
        &mut self,
        event: &str,
        table_id: i32,
    ) -> BTreeSet<(usize, usize)> {
        let mut quiet = BTreeSet::new();
        if !EVENTS.contains(&event) {
            return quiet;
        }
        let Some(unknown) = self.events.order_unknown_apps.as_ref() else {
            return quiet;
        };
        let engine = &self.engine;
        let key = (engine.events.global_key, event.to_string());
        let Some(subs) = engine.events.subscribers.get(&key) else {
            return quiet;
        };
        let number = ir::BuiltinId::of("RecordRef.Number#1");
        for s in subs {
            if s.manual || !unknown.contains(&engine.program.objects[s.object].app) {
                continue;
            }
            let Some(routine) = engine.program.objects[s.object].routines.get(s.routine) else {
                continue;
            };
            let cx = Cx {
                routine,
                table: Some(table_id),
                table_param: None,
                number,
                reads: self.empty_reads(s.object, s.routine),
            };
            if cx.inert(routine.body.stmts()) {
                quiet.insert((s.object, s.routine));
            }
        }
        quiet
    }
}

impl Session {
    /// Whether subscriber routine `r` of `object`, whose install order is not known, has no effect for any event and table:
    /// the code that runs assigns only its own locals, branches on them, and reads empty tables of its own app (module
    /// comment). Then the result is the same whether it runs or not.
    pub(crate) fn inert_in_either_order(&self, object: usize, r: usize) -> bool {
        let Some(routine) = self.engine.program.objects[object].routines.get(r) else {
            return false;
        };
        let cx = Cx {
            routine,
            table: None,
            table_param: None,
            number: ir::BuiltinId::of("RecordRef.Number#1"),
            reads: self.empty_reads(object, r),
        };
        cx.inert(routine.body.stmts())
    }
}
