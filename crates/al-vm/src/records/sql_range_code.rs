//! The table code record/filters/SqlDateFloor1753.al ran around a write of a value the SQL
//! column cannot hold. The error rolls back what that code wrote to the database; the record variable keeps what a trigger set
//! on it (Kept_RecordAfterModifyError: `Hits` 10 in the variable, 0 in the row). `records/sql_range.rs` takes a table with code
//! only when that code has a measured body, statement for statement; every other body keeps the table out of the measured
//! cells.
//!
//! - The table's own trigger (`LT SD Floor Trig`, SqlDateFloor1753.al:38-61): `Hits += 1` and `Hits += 10`, an Integer field
//!   of Rec outside the primary key added to.
//! - The OnModify trigger that logs (`LT SD Floor Trig Log`, :120-140): `Log.Add('<text>' + Format(Dt, 0, 9))` on a local of
//!   the log codeunit, Dt a Date field of Rec.
//! - The OnModify trigger that sets a Date (`LT SD Floor Trig SetDt`, :162-183): `if Dt = <date> then Dt := <date> else
//!   Dt := <date>`.
//! - The subscribers (`LT SD Floor Subscribers`, :223-255): one static subscriber per event, whose body is `Log.Add('<text>')`
//!   on a local of a codeunit whose procedure appends its Text parameter to a Text global (`LT SD Floor Log`, :200-221).
//! - The After insert subscriber that reads Rec (`LT SD Floor Sub Read`, :257-264): `Log.Add('<text>' + Format(Rec.Dt, 0, 9))`.

use al_builtins::builtin_id::ids;
use al_builtins::format::day_from_civil;
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

    /// Whether routine `routine` of table `t` (its own trigger) has the measured log body: the one statement
    /// `Log.Add('<text>' + Format(<Date field of Rec>, 0, 9))` (`log_calls`).
    pub(super) fn own_trigger_logs(&self, t: &TableDef, routine: usize) -> bool {
        let object = t.object as usize;
        let Some(rec) = self.engine.program.objects[object].rec_slot else {
            return false;
        };
        let rec = ir::Place::Global(rec);
        self.one_statement(object, routine) && self.log_calls(object, routine, &|a| text_and_date(a, &rec, t))
    }

    /// Whether the body of routine `routine` of `object` is one statement.
    fn one_statement(&self, object: usize, routine: usize) -> bool {
        let body = self.engine.program.objects[object].routines[routine].body.stmts();
        body.iter().filter(|s| !matches!(s, ir::Stmt::Pos(_))).count() == 1
    }

    /// The field and the day the `then` branch sets, when routine `routine` of table `t` (its own trigger) has the measured
    /// body that sets a Date: the one statement `if F = <date> then F := <date in 1753> else F := <date>`, F a Date field of
    /// Rec outside the primary key, the compared date and the `else` date ones the column holds (from 1754-01-01).
    pub(super) fn own_trigger_sets_date(&self, t: &TableDef, routine: usize) -> Option<(usize, i32)> {
        let object = t.object as usize;
        let rec = ir::Place::Global(self.engine.program.objects[object].rec_slot?);
        let stmts: Vec<&ir::Stmt> = self.engine.program.objects[object].routines[routine]
            .body
            .stmts()
            .iter()
            .filter(|s| !matches!(s, ir::Stmt::Pos(_)))
            .collect();
        let [ir::Stmt::If(ir::Expr::Binary(ir::BinOp::Eq, a, b), then, els)] = stmts.as_slice() else {
            return None;
        };
        let field = date_field_of(a, &rec, t).filter(|f| !t.keys[0].contains(f))?;
        let compared = date_const(b)?;
        let (set_field, set) = one_date_assign(then, &rec)?;
        let (else_field, other) = one_date_assign(els, &rec)?;
        let (d1753, d1754) = (day_from_civil(1753, 1, 1), day_from_civil(1754, 1, 1));
        let measured = set_field == field
            && else_field == field
            && (d1753..d1754).contains(&set)
            && compared >= d1754
            && other >= d1754
            && self.runnable(object, routine);
        measured.then_some((field, set))
    }

    /// Whether `event` of table `t` reaches one subscriber and it has the measured log body.
    pub(super) fn one_log_subscriber(&mut self, t: &TableDef, event: &str) -> Result<bool, Signal> {
        let subs = self.event_subscribers(t.object as usize, event)?;
        Ok(match subs.as_slice() {
            [s] => !s.manual && self.log_calls(s.object, s.routine, &text_const),
            _ => false,
        })
    }

    /// Whether `event` of table `t` reaches one subscriber and its body logs a Date field of its Rec parameter: the one
    /// statement `Log.Add('<text>' + Format(Rec.<Date field>, 0, 9))`, Rec the first parameter.
    pub(super) fn one_reader_subscriber(&mut self, t: &TableDef, event: &str) -> Result<bool, Signal> {
        let subs = self.event_subscribers(t.object as usize, event)?;
        Ok(match subs.as_slice() {
            [s] => {
                let r = &self.engine.program.objects[s.object].routines[s.routine];
                let rec_param = r.params >= 1
                    && r.locals
                        .first()
                        .is_some_and(|l| l.ty == RtType::Record(t.object, false));
                let rec = ir::Place::Local(0);
                !s.manual
                    && rec_param
                    && self.one_statement(s.object, s.routine)
                    && self.log_calls(s.object, s.routine, &|a| text_and_date(a, &rec, t))
            }
            _ => false,
        })
    }

    /// Whether routine `routine` of `object` only calls the measured log: each statement is `Local.Proc(<argument>)` on a
    /// local of a codeunit whose procedure `Proc` has the body of `log_body`, the argument one `arg` takes.
    fn log_calls(&self, object: usize, routine: usize, arg: &dyn Fn(&ir::Expr) -> bool) -> bool {
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
                        && matches!(args.as_slice(), [ir::Arg::Value(a)] if arg(a))
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

/// `'<text>' + Format(<Date field of rec>, 0, 9)`, converted to Text or not.
fn text_and_date(e: &ir::Expr, rec: &ir::Place, t: &TableDef) -> bool {
    match e {
        ir::Expr::Convert(RtType::Text(None), inner) => text_and_date(inner, rec, t),
        ir::Expr::Binary(ir::BinOp::Add, a, b) => text_const(a) && format_date_9(b, rec, t),
        _ => false,
    }
}

/// `Format(<Date field of rec>, 0, 9)`.
fn format_date_9(e: &ir::Expr, rec: &ir::Place, t: &TableDef) -> bool {
    match e {
        ir::Expr::Convert(RtType::Text(None), inner) => format_date_9(inner, rec, t),
        ir::Expr::Builtin {
            id,
            receiver: None,
            args,
            ..
        } => {
            *id == ids::SYSTEM_FORMAT_1
                && matches!(args.as_slice(), [ir::Arg::Value(v), ir::Arg::Value(len), ir::Arg::Value(fmt)]
                    if date_field_of(v, rec, t).is_some() && int_const(len) == Some(0) && int_const(fmt) == Some(9))
        }
        _ => false,
    }
}

/// The position of the Date field of `rec` that `e` loads.
fn date_field_of(e: &ir::Expr, rec: &ir::Place, t: &TableDef) -> Option<usize> {
    match e {
        ir::Expr::Load(ir::Place::Field(r, pos)) if **r == *rec && t.fields.get(*pos)?.ty == RtType::Date => Some(*pos),
        _ => None,
    }
}

/// The day of a Date constant (not 0D), converted to Date or not.
fn date_const(e: &ir::Expr) -> Option<i32> {
    match e {
        ir::Expr::Const(ir::Const::Date(d)) if *d > 0 => Some(*d),
        ir::Expr::Convert(RtType::Date, inner) => date_const(inner),
        _ => None,
    }
}

/// An Integer constant, converted to Integer or not.
fn int_const(e: &ir::Expr) -> Option<i32> {
    match e {
        ir::Expr::Const(ir::Const::Integer(n)) => Some(*n),
        ir::Expr::Convert(RtType::Integer, inner) => int_const(inner),
        _ => None,
    }
}

/// The field and day of a branch that is one statement `F := <date>`, F a field of `rec`.
fn one_date_assign(stmts: &[ir::Stmt], rec: &ir::Place) -> Option<(usize, i32)> {
    let body: Vec<&ir::Stmt> = stmts.iter().filter(|s| !matches!(s, ir::Stmt::Pos(_))).collect();
    match body.as_slice() {
        [ir::Stmt::Assign(ir::Place::Field(r, pos), e)] if **r == *rec => Some((*pos, date_const(e)?)),
        _ => None,
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
