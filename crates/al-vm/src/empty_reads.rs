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
//!
//! A table of the subscriber's own app that only that app's install code writes (al-runner issue 6971) is not empty in the
//! order where the subscriber runs: it holds the rows that install code inserted. When those rows are known from constants
//! (`InstallRows`, found by al-testrun `install_rows.rs`; a field value that is not a constant matches every filter), a read
//! is answered as for an empty table when no such row matches the filters the routine set on the record so far
//! (`SetRange` with a value the table id decides, `Reset`). In the other order the subscriber does not run at all, so
//! when it then exits before it assigns a flag, the result is the same in both orders. A filter set in a branch the table id
//! does not decide is not followed.

use crate::Session;
use al_builtins::value::RtType;
use al_data::TableDef;
use al_ir as ir;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

/// A field value of a row install code inserts.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KnownValue {
    Int(i32),
    Bool(bool),
}

/// The tables of the apps under test that only their own app's install code writes, each with the rows that code may
/// insert: per field position the value, `None` when it is not a constant (module comment).
#[derive(Debug, Default, Clone)]
pub struct InstallRows {
    pub tables: HashMap<u32, Vec<KnownRow>>,
}

/// A row install code may insert: per field position its value, `None` when not known.
pub type KnownRow = Vec<Option<KnownValue>>;

/// A filter the routine set on one field: to a value, or to a value not known here.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Filter {
    Is(KnownValue),
    Unknown,
}

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
    set_current_key: Option<ir::BuiltinId>,
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
            set_current_key: of("Record.SetCurrentKey#1"),
        }
    }
}

/// The value a key or filter expression has for the table asked about, when the table id decides it; given the field's type.
pub(crate) type ValueOf<'a> = &'a dyn Fn(&ir::Expr, &RtType) -> Option<KnownValue>;

/// The record locals of one routine whose table is empty in either install order, or holds only rows known from its own
/// app's install code (module comment).
pub(crate) struct EmptyReads {
    tables: HashMap<u32, (Arc<TableDef>, Arc<[KnownRow]>)>,
    ids: Ids,
    /// Per record local, the filters set on it so far, by field position.
    filters: RefCell<HashMap<usize, BTreeMap<usize, Filter>>>,
    /// Above 0 while a branch the table id does not decide is followed: a filter set there is not followed.
    undecided: Cell<u32>,
}

impl EmptyReads {
    /// Answers nothing.
    pub(crate) fn none() -> Self {
        EmptyReads {
            tables: HashMap::new(),
            ids: Ids::new(),
            filters: RefCell::new(HashMap::new()),
            undecided: Cell::new(0),
        }
    }

    /// Follows a branch the table id does not decide (`true`), or ends it (`false`).
    pub(crate) fn undecided(&self, on: bool) {
        let n = self.undecided.get();
        self.undecided.set(if on { n + 1 } else { n.saturating_sub(1) });
    }

    /// The local, table and known rows of `receiver` when it is a declared record local (not a parameter, not temporary)
    /// of a table this module answers for.
    fn table(&self, routine: &ir::Routine, receiver: Option<&ir::Arg>) -> Option<(usize, &TableDef, &[KnownRow])> {
        let l = match receiver? {
            ir::Arg::Ref(ir::Place::Local(l)) | ir::Arg::Value(ir::Expr::Load(ir::Place::Local(l))) => *l,
            _ => return None,
        };
        if l < routine.params || Some(l) == routine.ret_local {
            return None;
        }
        match routine.locals.get(l)?.ty {
            RtType::Record(t, false) => self.tables.get(&t).map(|(d, rows)| (l, &**d, &**rows)),
            _ => None,
        }
    }

    /// Whether no row of `rows` matches `filters` (a field value or filter not known matches).
    fn none_match(rows: &[KnownRow], filters: &BTreeMap<usize, Filter>) -> bool {
        rows.iter().all(|row| {
            filters.iter().any(|(p, f)| match (f, row.get(*p).copied().flatten()) {
                (Filter::Is(v), Some(x)) => *v != x,
                _ => false,
            })
        })
    }

    /// What `e` gives when it reads a table with no row that matches its filters. `ty` gives the type of a key value
    /// when it has no effect, `val` its value.
    pub(crate) fn answer(
        &self,
        routine: &ir::Routine,
        e: &ir::Expr,
        ty: &dyn Fn(&ir::Expr) -> Option<RtType>,
        val: ValueOf,
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
        let (l, t, rows) = self.table(routine, receiver.as_deref())?;
        let id = Some(*id);
        let ids = &self.ids;
        let empty = || {
            let filters = self.filters.borrow();
            Self::none_match(rows, filters.get(&l).unwrap_or(&BTreeMap::new()))
        };
        if args.is_empty() && id == ids.is_empty {
            return empty().then_some(Answer::Bool(true));
        }
        if args.is_empty() && id == ids.count {
            return empty().then_some(Answer::Int(0));
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
            return empty().then_some(Answer::Bool(false));
        }
        if id == ids.get && keys_typed(t, args, ty) {
            // Get looks at the primary key values, not at the filters.
            let pk = t.keys.first()?;
            let keys: BTreeMap<usize, Filter> = args
                .iter()
                .zip(pk)
                .map(|(a, &p)| {
                    let v = match a {
                        ir::Arg::Value(e) => t.fields.get(p).and_then(|f| val(e, &f.ty)),
                        ir::Arg::Ref(_) => None,
                    };
                    (p, v.map_or(Filter::Unknown, Filter::Is))
                })
                .collect();
            return Self::none_match(rows, &keys).then_some(Answer::Bool(false));
        }
        None
    }

    /// Whether statement expression `e` only sets or clears filters, or the key, of a record of such a table. The filters
    /// are noted for the reads that follow.
    pub(crate) fn inert(
        &self,
        routine: &ir::Routine,
        e: &ir::Expr,
        ty: &dyn Fn(&ir::Expr) -> Option<RtType>,
        val: ValueOf,
    ) -> bool {
        let ir::Expr::Builtin { id, receiver, args, .. } = e else {
            return false;
        };
        let Some((l, t, rows)) = self.table(routine, receiver.as_deref()) else {
            return false;
        };
        let id = Some(*id);
        // A table with known rows: a filter set where the table id does not decide whether it runs is not followed.
        let followed = rows.is_empty() || self.undecided.get() == 0;
        if id == self.ids.reset {
            if args.is_empty() && followed {
                self.filters.borrow_mut().remove(&l);
            }
            return args.is_empty() && followed;
        }
        if id == self.ids.set_current_key {
            // The fields of a key, or the first fields of one: the key the reads use, not which rows they find.
            let fields: Option<Vec<usize>> = args
                .iter()
                .map(|a| match a {
                    ir::Arg::Value(ir::Expr::Const(ir::Const::FieldPos(p))) => Some(*p as usize),
                    _ => None,
                })
                .collect();
            return fields.is_some_and(|f| !f.is_empty() && t.keys.iter().any(|k| k.starts_with(&f)));
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
        if !(values.len() <= 2 && values.iter().all(|a| value_typed(a, &f.ty, ty)) && followed) {
            return false;
        }
        // The value first: working it out may read the filters.
        let filter = match values {
            [] => None,
            [ir::Arg::Value(v)] => Some(val(v, &f.ty).map_or(Filter::Unknown, Filter::Is)),
            _ => Some(Filter::Unknown),
        };
        let mut filters = self.filters.borrow_mut();
        let of = filters.entry(l).or_default();
        match filter {
            None => of.remove(&(*p as usize)),
            Some(f) => of.insert(*p as usize, f),
        };
        true
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
    /// The tables only their own app's install code writes, with the rows it may insert (al-testrun `install_rows.rs`).
    /// `None`: none known.
    pub fn set_install_rows(&mut self, rows: Option<InstallRows>) {
        self.events.install_rows = rows.map(Arc::new);
    }

    /// The tables of routine `r` of `object`'s record locals that are empty in either install order (module comment):
    /// tables of the subscriber's own app that hold no row and that no install code of an app under test writes; and
    /// those only their own app's install code writes, with the rows it may insert.
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
        let known = self.events.install_rows.as_deref();
        for slot in &routine.locals[routine.params.min(routine.locals.len())..] {
            let RtType::Record(t, false) = slot.ty else { continue };
            if reads.tables.contains_key(&t) || objects.heads().get(t as usize).is_none_or(|o| o.app != app) {
                continue;
            }
            // Written by install code: only its own app's, with rows known from constants (al-runner issue 6971).
            let rows: Arc<[KnownRow]> = if written.contains(&t) {
                match known.and_then(|k| k.tables.get(&t)) {
                    Some(rows) => rows.as_slice().into(),
                    None => continue,
                }
            } else if self.store.has_rows(t) {
                continue;
            } else {
                Arc::from([])
            };
            if let Some(Some(def)) = engine.tables.get(t as usize) {
                reads.tables.insert(t, (def.clone(), rows));
            }
        }
        reads
    }
}
