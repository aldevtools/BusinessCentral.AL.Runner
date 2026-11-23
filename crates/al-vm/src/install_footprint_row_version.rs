//! Which routines read the SystemRowVersion field (2000000005), for the row version resource of an install footprint
//! (ADR-079 decision 1: "a read of `SystemRowVersion`"). Its value is a stored field of every row, so a read cannot be
//! told apart from a read of another field at the store. The routines a session ran are scanned instead, statically, and
//! every field the scan cannot resolve to a table counts as a read (a gap is a conflict, decision 1):
//! - a field of a record place (`Rec.SystemRowVersion`, also as a `var` argument) and a field passed to a record method
//!   (`SetRange`, `SetFilter`, `SetCurrentKey`, `CalcFields` and every other `FieldPos` argument): the field of the
//!   record's table; a record whose table the scan cannot resolve counts when SystemRowVersion sits at that position in
//!   any table;
//! - `CalcFields`, `CalcSums` and `SetAutoCalcFields` when a FlowField of the program aggregates or filters on
//!   SystemRowVersion;
//! - a field passed to a `var` parameter of a procedure, and every query column read;
//! - the dynamic readers, noted at the builtin call ([`reads_dynamically`]): every `RecordRef`, `FieldRef`, `KeyRef` and
//!   `Query` method and `Record.SetView` (a field number, a key or a view text can name the field).

use al_builtins::value::RtType;
use al_ir::{Arg, Const, Expr, Place, Routine};

/// SystemRowVersion's field id (al-sema `members.rs`).
const SYSTEM_ROW_VERSION: u32 = 2_000_000_005;

/// Whether builtin `receiver.method` can read SystemRowVersion through a field number, key or view the scan cannot see.
pub(crate) fn reads_dynamically(receiver: &str, method: &str) -> bool {
    matches!(receiver, "RecordRef" | "FieldRef" | "KeyRef" | "Query") || (receiver == "Record" && method == "SetView")
}

/// Whether the routines `routines` (object, routine) read SystemRowVersion.
pub(crate) fn routines_read(engine: &crate::Engine, routines: impl IntoIterator<Item = (usize, usize)>) -> bool {
    let scan = Scan::new(engine);
    routines.into_iter().any(|(o, r)| {
        let Some(routine) = engine.program.objects.get(o).and_then(|ob| ob.routines.get(r)) else {
            return true;
        };
        scan.routine(o, routine)
    })
}

struct Scan<'a> {
    engine: &'a crate::Engine,
    /// The positions SystemRowVersion holds in some table: a field at one of them on a record of a table the scan cannot
    /// resolve counts as a read.
    any_position: std::collections::BTreeSet<usize>,
    /// A FlowField of the program aggregates or filters on SystemRowVersion.
    flow_field: bool,
}

impl<'a> Scan<'a> {
    fn new(engine: &'a crate::Engine) -> Self {
        let mut any_position = std::collections::BTreeSet::new();
        for t in engine.tables.iter().flatten() {
            if let Some(p) = t.fields.iter().position(|f| f.id == SYSTEM_ROW_VERSION) {
                any_position.insert(p);
            }
        }
        let is_srv = |table: usize, pos: usize| {
            engine
                .tables
                .get(table)
                .and_then(|t| t.as_ref())
                .and_then(|t| t.fields.get(pos))
                .is_some_and(|f| f.id == SYSTEM_ROW_VERSION)
        };
        let flow_field = engine.program.objects.iter().filter_map(|o| o.table.as_ref()).any(|t| {
            t.fields.iter().filter_map(|f| f.calc.as_ref()).any(|c| {
                c.field.is_some_and(|p| is_srv(c.table, p)) || c.conds.iter().any(|w| is_srv(c.table, w.field))
            })
        });
        Scan {
            engine,
            any_position,
            flow_field,
        }
    }

    fn routine(&self, o: usize, r: &Routine) -> bool {
        let mut hit = false;
        r.body.read(|stmts| {
            al_ir::types::for_each_expr_post(stmts, &mut |e| hit = hit || self.expr(o, r, e));
            // A place an assignment or a `var` argument writes is no read; the expressions in it are walked above.
        });
        hit
    }

    fn expr(&self, o: usize, r: &Routine, e: &Expr) -> bool {
        match e {
            Expr::Load(p) => self.place(o, r, p),
            Expr::FieldOf(_, pos) => self.any_position.contains(pos),
            Expr::Builtin { id, receiver, args, .. } => {
                let table = receiver.as_deref().and_then(|a| match a {
                    Arg::Ref(p) | Arg::Value(Expr::Load(p)) => self.record_table(o, r, p),
                    Arg::Value(_) => None,
                });
                let field = args.iter().any(|a| match a {
                    Arg::Value(Expr::Const(Const::FieldPos(pos))) => self.field_is_srv(table, *pos as usize),
                    Arg::Ref(p) => self.place(o, r, p),
                    Arg::Value(_) => false,
                });
                let method = id
                    .name()
                    .split_once('.')
                    .map_or("", |(_, m)| m.split('#').next().unwrap_or(m));
                field || (self.flow_field && matches!(method, "CalcFields" | "CalcSums" | "SetAutoCalcFields"))
            }
            // A field passed to a `var` parameter is read by the callee.
            Expr::Call { args, .. } | Expr::InterfaceCall { args, .. } => {
                args.iter().any(|a| matches!(a, Arg::Ref(p) if self.place(o, r, p)))
            }
            // A query column can be SystemRowVersion; queries are counted whole.
            Expr::QueryColumn { .. } => true,
            _ => false,
        }
    }

    /// Whether `p` holds a field of a record that is SystemRowVersion (or may be).
    fn place(&self, o: usize, r: &Routine, p: &Place) -> bool {
        match p {
            Place::Field(base, pos) => self.field_is_srv(self.record_table(o, r, base), *pos) || self.place(o, r, base),
            Place::Index(base, _) | Place::RecGlobal(base, ..) => self.place(o, r, base),
            _ => false,
        }
    }

    fn field_is_srv(&self, table: Option<usize>, pos: usize) -> bool {
        match table {
            Some(t) => (self.engine.tables.get(t).and_then(|t| t.as_ref()))
                .is_none_or(|t| t.fields.get(pos).is_none_or(|f| f.id == SYSTEM_ROW_VERSION)),
            None => self.any_position.contains(&pos),
        }
    }

    /// The table (object index) of the record `p` holds, when its declared type says it.
    fn record_table(&self, o: usize, r: &Routine, p: &Place) -> Option<usize> {
        match self.place_type(o, r, p)? {
            RtType::Record(t, _) => Some(t as usize),
            _ => None,
        }
    }

    fn place_type(&self, o: usize, r: &Routine, p: &Place) -> Option<RtType> {
        match p {
            Place::Local(i) => r.locals.get(*i).map(|s| s.ty.clone()),
            Place::Global(g) => self
                .engine
                .program
                .objects
                .get(o)?
                .globals
                .get(*g)
                .map(|s| s.ty.clone()),
            Place::Index(base, _) => match self.place_type(o, r, base)? {
                RtType::Array(_, elem) => Some(*elem),
                _ => None,
            },
            _ => None,
        }
    }
}
