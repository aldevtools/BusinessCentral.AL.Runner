//! The table code corpus PR 1398 (tests/al-language/record/filters/SqlDateFloor1753.al) ran around a write of a value the SQL
//! column cannot hold, and whose run the probe cannot see: the error rolls back what it wrote, so the probe does not show
//! whether, or in what order, it ran. `records/sql_range.rs` takes a table with code only when that code has the measured
//! body, statement for statement; every other body keeps the table out of the measured cells.
//!
//! - The table's own trigger (`LT SD Floor Trig`, SqlDateFloor1753.al:49-57): `Hits += 1` and `Hits += 10`, an Integer field
//!   of Rec outside the primary key added to.
//! - The subscribers (`LT SD Floor Subscribers`, SqlDateFloor1753.al:140-173): one static subscriber per event, whose body is
//!   `Log.Add('<text>')` on a local of a codeunit whose procedure appends its Text parameter to a Text global
//!   (`LT SD Floor Log`, SqlDateFloor1753.al:117-138).

use al_builtins::native::Signal;
use al_builtins::value::RtType;
use al_data::TableDef;
use al_ir as ir;

use crate::Session;

impl Session {
    /// Whether routine `routine` of table `t` (its own trigger) has the measured trigger body: each statement adds Integer
    /// constants and Integer fields of Rec to an Integer field of Rec outside the primary key.
    pub(super) fn own_trigger_measured(&self, t: &TableDef, routine: usize) -> bool {
        let object = t.object as usize;
        let program = &self.engine.program;
        let Some(rec) = program.objects[object].rec_slot else {
            return false;
        };
        let integer_field = |p: &ir::Place| match p {
            ir::Place::Field(r, pos) => {
                **r == ir::Place::Global(rec) && t.fields.get(*pos).is_some_and(|f| f.ty == RtType::Integer)
            }
            _ => false,
        };
        self.runnable(object, routine)
            && program.objects[object].routines[routine]
                .body
                .stmts()
                .iter()
                .all(|s| match s {
                    ir::Stmt::Pos(_) => true,
                    ir::Stmt::Assign(place @ ir::Place::Field(_, pos), e) => {
                        integer_field(place) && !t.keys[0].contains(pos) && integer_sum(e, &integer_field)
                    }
                    _ => false,
                })
    }

    /// Whether `event` of table `t` reaches one subscriber and it has the measured log body.
    pub(super) fn one_log_subscriber(&mut self, t: &TableDef, event: &str) -> Result<bool, Signal> {
        let subs = self.event_subscribers(t.object as usize, event)?;
        Ok(match subs.as_slice() {
            [s] => !s.manual && self.log_subscriber(s.object, s.routine),
            _ => false,
        })
    }

    /// Whether the subscriber `routine` of codeunit `object` only calls the measured log: each statement is
    /// `Local.Proc('<text>')` on a local of a codeunit whose procedure `Proc` has the body of `log_body`.
    fn log_subscriber(&self, object: usize, routine: usize) -> bool {
        let r = &self.engine.program.objects[object].routines[routine];
        self.runnable(object, routine)
            && r.body.stmts().iter().all(|s| match s {
                ir::Stmt::Pos(_) => true,
                ir::Stmt::Expr(ir::Expr::Call {
                    object: callee,
                    routine: proc,
                    instance: Some(inst),
                    args,
                    try_call: false,
                }) => {
                    matches!(&**inst, ir::Expr::Load(ir::Place::Local(l))
                        if *l >= r.params && r.locals.get(*l).is_some_and(|s| s.ty == RtType::Codeunit(*callee as u32)))
                        && matches!(args.as_slice(), [ir::Arg::Value(a)] if text_const(a))
                        && self.log_body(*callee, *proc)
                }
                _ => false,
            })
    }

    /// Whether procedure `routine` of codeunit `object` has one Text parameter and each statement appends Text constants,
    /// that parameter and Text globals of the codeunit to a Text global (`Entries += What + ';'`).
    fn log_body(&self, object: usize, routine: usize) -> bool {
        let o = &self.engine.program.objects[object];
        let r = &o.routines[routine];
        let text_global = |g: usize| o.globals.get(g).is_some_and(|s| s.ty == RtType::Text(None));
        let text_param = r.params == 1 && r.locals.first().is_some_and(|s| s.ty == RtType::Text(None));
        o.kind == ir::ObjKind::Codeunit
            && text_param
            && self.runnable(object, routine)
            && r.body.stmts().iter().all(|s| match s {
                ir::Stmt::Pos(_) => true,
                ir::Stmt::Assign(ir::Place::Global(g), e) => text_global(*g) && text_sum(e, &text_global),
                _ => false,
            })
    }

    /// The lowering left routine `routine` of `object` runnable (no refusal reason).
    fn runnable(&self, object: usize, routine: usize) -> bool {
        self.engine
            .blocked
            .get(object)
            .and_then(|o| o.get(routine))
            .is_some_and(Option::is_none)
    }
}

/// `e` adds Integer constants and the places `field` takes.
fn integer_sum(e: &ir::Expr, field: &dyn Fn(&ir::Place) -> bool) -> bool {
    match e {
        ir::Expr::Const(ir::Const::Integer(_)) => true,
        ir::Expr::Load(p) => field(p),
        ir::Expr::Convert(RtType::Integer, inner) => integer_sum(inner, field),
        ir::Expr::Binary(ir::BinOp::Add, a, b) => integer_sum(a, field) && integer_sum(b, field),
        _ => false,
    }
}

/// A Text constant, converted to Text or not.
fn text_const(e: &ir::Expr) -> bool {
    match e {
        ir::Expr::Const(ir::Const::Text(_)) => true,
        ir::Expr::Convert(RtType::Text(None), inner) => text_const(inner),
        _ => false,
    }
}

/// `e` joins Text constants, the first parameter (local 0) and the globals `global` takes.
fn text_sum(e: &ir::Expr, global: &dyn Fn(usize) -> bool) -> bool {
    match e {
        ir::Expr::Const(ir::Const::Text(_)) | ir::Expr::Load(ir::Place::Local(0)) => true,
        ir::Expr::Load(ir::Place::Global(g)) => global(*g),
        ir::Expr::Convert(RtType::Text(None), inner) => text_sum(inner, global),
        ir::Expr::Binary(ir::BinOp::Add, a, b) => text_sum(a, global) && text_sum(b, global),
        _ => false,
    }
}
