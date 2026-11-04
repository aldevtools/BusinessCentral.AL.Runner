//! Reads of a table of the subscriber's own app that is empty in either install order (al-runner issue 6924).
//!
//! While an app installs, a subscriber of an app with no dependency path to it runs exactly when BC installed its app first
//! (corpus install-upgrade/install-triggers/Y79InstallUnrelated.al:6-12), and that order is not known (`dispatch.rs`,
//! `order_unknown_apps`). A subscriber that only goes on after it found a row of a table of its own app
//! (`if not Reg.Get(TableId) then exit;`) does nothing in the order where it runs when that table is empty then. The table
//! is: it holds no row now (the rows of the dependency apps' install and of the install code that ran so far are in the
//! store), and no install code of any app under test writes it (`Session::set_install_written_tables`), so the install
//! code that runs before this point in the other order cannot have filled it either.
//!
//! This module answers, for a record local of such a table, what `Get`, `IsEmpty`, `FindFirst`, `FindLast`, `FindSet` and
//! `Count` give (no row), and which statements on it have no effect (`SetRange`, `Reset`). `db_event_no_op.rs` uses the
//! answers to follow the branches the reads decide. A `Get` whose result is not used raises an error on a missing row, so
//! it is not answered; nor is one whose key values are not of the key fields' types (a conversion could fail).

use crate::Session;
use al_builtins::value::RtType;
use al_data::TableDef;
use al_ir as ir;
use std::collections::HashMap;
use std::sync::Arc;

/// What a read of an empty table gives.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Answer {
    Bool(bool),
    Int(i32),
}

/// The builtins this module knows.
struct Ids {
    get: Option<ir::BuiltinId>,
    is_empty: Option<ir::BuiltinId>,
    find_first: Option<ir::BuiltinId>,
    find_last: Option<ir::BuiltinId>,
    find_set: [Option<ir::BuiltinId>; 2],
    count: Option<ir::BuiltinId>,
    set_range: Option<ir::BuiltinId>,
    reset: Option<ir::BuiltinId>,
}

impl Ids {
    fn new() -> Self {
        let of = ir::BuiltinId::of;
        Ids {
            get: of("Record.Get#1"),
            is_empty: of("Record.IsEmpty#1"),
            find_first: of("Record.FindFirst#1"),
            find_last: of("Record.FindLast#1"),
            find_set: [of("Record.FindSet#1"), of("Record.FindSet#2")],
            count: of("Record.Count#1"),
            set_range: of("Record.SetRange#1"),
            reset: of("Record.Reset#1"),
        }
    }
}

/// The record locals of one routine whose table is empty in either install order.
pub(crate) struct EmptyReads {
    tables: HashMap<u32, Arc<TableDef>>,
    ids: Ids,
}

impl EmptyReads {
    /// Answers nothing.
    pub(crate) fn none() -> Self {
        EmptyReads {
            tables: HashMap::new(),
            ids: Ids::new(),
        }
    }

    /// The table of `receiver` when it is a declared record local (not a parameter, not temporary) of an empty table.
    fn table(&self, routine: &ir::Routine, receiver: Option<&ir::Arg>) -> Option<&TableDef> {
        let l = match receiver? {
            ir::Arg::Ref(ir::Place::Local(l)) | ir::Arg::Value(ir::Expr::Load(ir::Place::Local(l))) => *l,
            _ => return None,
        };
        if l < routine.params || Some(l) == routine.ret_local {
            return None;
        }
        match routine.locals.get(l)?.ty {
            RtType::Record(t, false) => self.tables.get(&t).map(|d| &**d),
            _ => None,
        }
    }

    /// What `e` gives when it reads an empty table. `ty` gives the type of a key value when it has no effect.
    pub(crate) fn answer(
        &self,
        routine: &ir::Routine,
        e: &ir::Expr,
        ty: &dyn Fn(&ir::Expr) -> Option<RtType>,
    ) -> Option<Answer> {
        let ir::Expr::Builtin {
            id,
            receiver,
            args,
            ret_used,
        } = e
        else {
            return None;
        };
        let t = self.table(routine, receiver.as_deref())?;
        let id = Some(*id);
        let ids = &self.ids;
        if args.is_empty() && id == ids.is_empty {
            return Some(Answer::Bool(true));
        }
        if args.is_empty() && id == ids.count {
            return Some(Answer::Int(0));
        }
        // A Get or Find whose result is not used raises an error when no row is found.
        if !*ret_used {
            return None;
        }
        let bool_consts = args
            .iter()
            .all(|a| matches!(a, ir::Arg::Value(ir::Expr::Const(ir::Const::Boolean(_)))));
        if (args.is_empty() && (id == ids.find_first || id == ids.find_last))
            || (bool_consts && ids.find_set.contains(&id))
        {
            return Some(Answer::Bool(false));
        }
        if id == ids.get && keys_typed(t, args, ty) {
            return Some(Answer::Bool(false));
        }
        None
    }

    /// Whether statement expression `e` only sets or clears filters of a record of an empty table.
    pub(crate) fn inert(&self, routine: &ir::Routine, e: &ir::Expr, ty: &dyn Fn(&ir::Expr) -> Option<RtType>) -> bool {
        let ir::Expr::Builtin { id, receiver, args, .. } = e else {
            return false;
        };
        let Some(t) = self.table(routine, receiver.as_deref()) else {
            return false;
        };
        let id = Some(*id);
        if id == self.ids.reset {
            return args.is_empty();
        }
        if id != self.ids.set_range {
            return false;
        }
        // `SetRange(Field)`, `SetRange(Field, Value)`, `SetRange(Field, From, To)` with values of the field's type.
        let Some((ir::Arg::Value(ir::Expr::Const(ir::Const::FieldPos(p))), values)) = args.split_first() else {
            return false;
        };
        let Some(f) = t.fields.get(*p as usize) else {
            return false;
        };
        values.len() <= 2 && values.iter().all(|a| value_typed(a, &f.ty, ty))
    }
}

/// Whether the key values of a `Get` are of the types of the primary key fields, so no conversion can fail.
fn keys_typed(t: &TableDef, args: &[ir::Arg], ty: &dyn Fn(&ir::Expr) -> Option<RtType>) -> bool {
    let Some(pk) = t.keys.first() else {
        return false;
    };
    args.len() <= pk.len()
        && args
            .iter()
            .zip(pk)
            .all(|(a, &p)| t.fields.get(p).is_some_and(|f| value_typed(a, &f.ty, ty)))
}

fn value_typed(a: &ir::Arg, field: &RtType, ty: &dyn Fn(&ir::Expr) -> Option<RtType>) -> bool {
    let ir::Arg::Value(e) = a else {
        return false;
    };
    matches!(field, RtType::Integer | RtType::Boolean) && ty(e).as_ref() == Some(field)
}

impl Session {
    /// The tables of routine `r` of `object`'s record locals that are empty in either install order (module comment):
    /// tables of the subscriber's own app that hold no row and that no install code of an app under test writes.
    pub(crate) fn empty_reads(&self, object: usize, r: usize) -> EmptyReads {
        let mut reads = EmptyReads::none();
        let Some(written) = self.events.install_written.as_ref() else {
            return reads;
        };
        let engine = &self.engine;
        let objects = &engine.program.objects;
        let app = objects[object].app;
        let Some(routine) = objects[object].routines.get(r) else {
            return reads;
        };
        for slot in &routine.locals[routine.params.min(routine.locals.len())..] {
            let RtType::Record(t, false) = slot.ty else { continue };
            if reads.tables.contains_key(&t)
                || objects.heads().get(t as usize).is_none_or(|o| o.app != app)
                || written.contains(&t)
                || self.store.has_rows(t)
            {
                continue;
            }
            if let Some(Some(def)) = engine.tables.get(t as usize) {
                reads.tables.insert(t, def.clone());
            }
        }
        reads
    }
}
