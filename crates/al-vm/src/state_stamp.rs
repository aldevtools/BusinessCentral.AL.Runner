//! What a run of code left in the session besides the database rows: the session language, WorkDate, the number
//! sequences and isolated storage (al-testrun `login.rs` compares it before and after the apps' own login subscribers
//! ran in a session where what BC's login does is not known; the store's watch follows the rows).

use crate::Session;
use crate::sessionops::Sequence;
use al_builtins::value::{Instance, RecordState, Value};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

/// The session state [`Session::state_stamp`] takes.
#[derive(PartialEq)]
pub struct StateStamp {
    language: i32,
    work_date: Option<i32>,
    sequences: BTreeMap<(String, String), Sequence>,
    isolated_storage: BTreeMap<(String, String), (String, bool, bool)>,
}

impl StateStamp {
    /// The first kind of state that differs between `self` and `later`, for a reason text.
    pub fn changed(&self, later: &StateStamp) -> Option<&'static str> {
        if self.language != later.language {
            Some("the session language")
        } else if self.work_date != later.work_date {
            Some("WorkDate")
        } else if self.sequences != later.sequences {
            Some("a number sequence")
        } else if self.isolated_storage != later.isolated_storage {
            Some("isolated storage")
        } else {
            None
        }
    }
}

impl Session {
    /// The session language, WorkDate, number sequences and isolated storage now.
    pub fn state_stamp(&self) -> StateStamp {
        StateStamp {
            language: self.global_language(),
            work_date: self.work_date_day(),
            sequences: self.sequences.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            isolated_storage: self
                .isolated_storage
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        }
    }
}

/// What a run of code left in the session besides the database rows, wider than [`StateStamp`]: also the SingleInstance
/// codeunits and their globals, the manual event bindings, the `Random` sequence and the filters in filter group 1 (al-testrun
/// `partner_login.rs` compares it before and after the partner dependency apps' login subscribers ran in the test session).
pub struct LeftStamp {
    state: StateStamp,
    singles: BTreeMap<usize, Single>,
    /// The live manual bindings ([`Session::bound_subscriptions`]).
    bindings: Vec<usize>,
    random: (bool, i32, u64),
    group1: BTreeMap<u32, String>,
}

/// One SingleInstance codeunit's instance in a [`LeftStamp`].
struct Single {
    /// The fingerprint of each global.
    globals: Vec<crate::single_track::Fp>,
    /// Per global: it holds what a fresh instance of the codeunit holds ([`at_declared`]). Empty when the instance has
    /// another number of globals than a fresh one.
    declared: Vec<bool>,
}

/// What differs between two [`LeftStamp`]s, other than rows and group 1 filters ([`LeftStamp::changed`]).
#[derive(Debug, Default, PartialEq)]
pub struct LeftChange {
    /// State every code reads without naming it, for a reason text: the session language, WorkDate, the manual event
    /// bindings.
    pub session: Option<&'static str>,
    pub number_sequences: bool,
    pub isolated_storage: bool,
    pub random: bool,
    /// The SingleInstance codeunits (object index) whose state changed, each with the globals (index) that changed: of an
    /// instance that was there before, those that hold another value (all of them when it went away or its number of
    /// globals differs); of a new instance, those that do not hold what a fresh instance holds. A global with no
    /// comparable form (a .NET object, a stream, a temporary record) counts as changed.
    pub singles: BTreeMap<usize, BTreeSet<usize>>,
}

impl LeftChange {
    /// Nothing changed.
    pub fn is_empty(&self) -> bool {
        *self == LeftChange::default()
    }
}

impl LeftStamp {
    /// What differs between `self` and `later`, other than rows and group 1 filters.
    pub fn changed(&self, later: &LeftStamp) -> LeftChange {
        let (a, b) = (&self.state, &later.state);
        let session = if a.language != b.language {
            Some("the session language")
        } else if a.work_date != b.work_date {
            Some("WorkDate")
        } else if self.bindings != later.bindings {
            Some("the manual event bindings")
        } else {
            None
        };
        let objects: BTreeSet<usize> = self.singles.keys().chain(later.singles.keys()).copied().collect();
        let singles = objects
            .into_iter()
            .filter_map(|o| {
                let changed = match (self.singles.get(&o), later.singles.get(&o)) {
                    (Some(x), Some(y)) if x.globals.len() == y.globals.len() => {
                        (0..x.globals.len()).filter(|&g| x.globals[g] != y.globals[g]).collect()
                    }
                    (None, Some(y)) if y.declared.len() == y.globals.len() => {
                        (0..y.declared.len()).filter(|&g| !y.declared[g]).collect()
                    }
                    (Some(x), _) => (0..x.globals.len()).collect(),
                    (None, Some(y)) => (0..y.globals.len()).collect(),
                    (None, None) => BTreeSet::new(),
                };
                (!changed.is_empty()).then_some((o, changed))
            })
            .collect();
        LeftChange {
            session,
            number_sequences: a.sequences != b.sequences,
            isolated_storage: a.isolated_storage != b.isolated_storage,
            random: self.random != later.random,
            singles,
        }
    }

    /// The tables whose filters in filter group 1 differ between `self` and `later`.
    pub fn group1_changed(&self, later: &LeftStamp) -> BTreeSet<u32> {
        let tables = self.group1.keys().chain(later.group1.keys());
        tables
            .filter(|t| self.group1.get(t) != later.group1.get(t))
            .copied()
            .collect()
    }
}

impl Session {
    /// The state [`LeftStamp`] takes, now.
    pub fn left_stamp(&mut self) -> LeftStamp {
        let made: Vec<(usize, Rc<RefCell<Instance>>)> = self.singletons.iter().map(|(&o, i)| (o, i.clone())).collect();
        let mut singles = BTreeMap::new();
        for (o, inst) in made {
            let engine = self.engine.clone();
            let fresh = self.slot_defaults(&engine.program.objects[o].globals);
            let inst = inst.borrow();
            let declared = if inst.globals.len() == fresh.len() {
                (inst.globals.iter().zip(&fresh))
                    .map(|(v, f)| at_declared(v, f, &engine, 0))
                    .collect()
            } else {
                Vec::new()
            };
            let globals = inst
                .globals
                .iter()
                .map(|v| crate::single_track::fp(v, &engine))
                .collect();
            singles.insert(o, Single { globals, declared });
        }
        let group1 = self.global_group1.signatures();
        LeftStamp {
            state: self.state_stamp(),
            singles,
            bindings: self.bound_subscriptions(),
            random: (self.randomized_with_seed, self.unseeded_randomizes, self.random_draws),
            group1,
        }
    }
}

/// Whether global value `v` holds what `fresh`, the value a fresh instance's global holds, holds: the same fingerprint;
/// for a record also the same view (key, sort order, filter group, load and calculated fields, read isolation, security
/// filtering, company, marks) and nothing read or written through it; for a codeunit the same of each of its globals.
fn at_declared(v: &Value, fresh: &Value, engine: &crate::Engine, depth: u8) -> bool {
    if depth > 8 {
        return false;
    }
    match (v, fresh) {
        (Value::Record(a), Value::Record(b)) => {
            let (Ok(a), Ok(b)) = (a.try_borrow(), b.try_borrow()) else {
                return false;
            };
            same_view(&a, &b) && crate::single_track::fp(v, engine) == crate::single_track::fp(fresh, engine)
        }
        (Value::Codeunit(a), Value::Codeunit(b)) => {
            let (Ok(a), Ok(b)) = (a.try_borrow(), b.try_borrow()) else {
                return false;
            };
            a.object == b.object
                && a.globals.len() == b.globals.len()
                && (a.globals.iter().zip(&b.globals)).all(|(x, y)| at_declared(x, y, engine, depth + 1))
        }
        _ => crate::single_track::fp(v, engine) == crate::single_track::fp(fresh, engine),
    }
}

/// Whether record `a` has the view of record `b`, a fresh variable of the same table, and nothing was read into or written
/// from it (no position, buffer, insert, modify or row origin).
fn same_view(a: &RecordState, b: &RecordState) -> bool {
    a.table == b.table
        && a.temporary == b.temporary
        && a.company == b.company
        && a.filters.is_empty()
        && b.filters.is_empty()
        && a.filter_group == b.filter_group
        && a.marks.is_empty()
        && a.marked_only == b.marked_only
        && a.read_isolation == b.read_isolation
        && a.security_filtering == b.security_filtering
        && a.key == b.key
        && a.adhoc == b.adhoc
        && a.ascending == b.ascending
        && a.field_descending == b.field_descending
        && a.auto_calc == b.auto_calc
        && a.load.request == b.load.request
        && a.load.partial == b.load.partial
        && a.load.buffer.is_none()
        && a.position.is_none()
        && !a.touched
        && !a.assigned
        && !a.inserted
        && a.last_modify.is_none()
        && a.failed_insert.is_none()
        && a.row_origin.is_none()
}
