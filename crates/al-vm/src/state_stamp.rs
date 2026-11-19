//! What a run of code left in the session besides the database rows: the session language, WorkDate, the number
//! sequences and isolated storage (al-testrun `login.rs` compares it before and after the apps' own login subscribers
//! ran in a session where what BC's login does is not known; the store's watch follows the rows).

use crate::Session;
use crate::sessionops::Sequence;
use crate::state_render::content;
use al_builtins::value::{Instance, RecordState, Value};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

/// The session state [`Session::state_stamp`] takes.
#[derive(PartialEq, Debug)]
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
    /// Every global holds what a fresh instance of the codeunit holds ([`at_declared`]).
    declared: bool,
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
    /// The SingleInstance codeunits (object index) whose state changed: an instance that was there before and holds
    /// another value in a global or went away, or a new instance whose globals do not all hold what a fresh instance
    /// holds. A global with no comparable form (a .NET object, a stream, a temporary record) counts as changed.
    pub singles: BTreeSet<usize>,
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
        let objects = self.singles.keys().chain(later.singles.keys());
        let singles = objects
            .filter(|o| match (self.singles.get(o), later.singles.get(o)) {
                (Some(x), Some(y)) => x.globals != y.globals,
                (None, Some(y)) => !y.declared,
                _ => true,
            })
            .copied()
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
            let declared = inst.globals.len() == fresh.len()
                && (inst.globals.iter().zip(&fresh)).all(|(v, f)| at_declared(v, f, &engine, 0));
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

/// The session state that [`Session::state_lines`] takes, as lines that compare across sessions.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StateLines {
    /// One line per kind of state, in a fixed order.
    pub lines: Vec<String>,
    /// The SingleInstance globals that hold a value with no comparable form (`state_render.rs`: a .NET object, an open
    /// stream, ...), with why and its path: what they hold cannot be compared, so two sessions holding one are never known
    /// to hold the same.
    pub opaque: Vec<String>,
}

impl Session {
    /// The run seed (ADR-061): the session clock's key (`al_host::seed::clock_key`) and seeded Guids derive from it.
    pub fn run_seed(&self) -> i32 {
        self.host.run_seed
    }

    /// The user the session runs as (`UserId()`).
    pub fn user_id(&self) -> String {
        self.host.identity.user_id()
    }

    /// What [`LeftStamp`] takes (the session language, WorkDate, number sequences, isolated storage, the SingleInstance
    /// codeunits' globals, the manual bindings, the `Random` sequence, the group 1 filters), plus the counters code
    /// advances (the clock, the Guid and SystemId draws and their key, the row versions) and the last error, as lines
    /// that compare across sessions: an instance is named by its codeunit id, not by its address (al-testrun
    /// `login_replay.rs`, al-runner issue 8211).
    pub fn state_lines(&mut self) -> StateLines {
        let stamp = self.left_stamp();
        let id = |o: usize| self.engine.program.objects[o].id.unwrap_or(0);
        let mut out = StateLines::default();
        out.lines.push(format!("session {}", content(&stamp.state)));
        // Temporary tables, codeunit instances and shared handles are numbered by first sight, in this fixed order of the
        // globals.
        let mut render = crate::state_render::Render::new(self);
        for (&o, single) in &stamp.singles {
            let inst = self.singletons.get(&o).map(|i| i.borrow());
            for (g, fp) in single.globals.iter().enumerate() {
                let value = inst.as_ref().and_then(|i| i.globals.get(g));
                let rendered = value.map(|v| render.value(v, 0)).unwrap_or_else(|| {
                    Err(match fp {
                        crate::single_track::Fp::Opaque(what) => what.to_string(),
                        crate::single_track::Fp::Val(_) => "a value".to_string(),
                    })
                });
                match rendered {
                    Ok(text) => out.lines.push(format!("single {} global {g} {text}", id(o))),
                    // Why it has no comparable form, with the path into the instances it is held in.
                    Err(why) => out.opaque.push(format!("codeunit {} global {g}: {why}", id(o))),
                }
            }
        }
        // The manual bindings in the order they were made, each by the codeunit and the instance it holds (a number of the
        // render: a binding of an instance a global holds names that global's instance).
        let mut bound = Vec::new();
        for (k, b) in self.events.bindings.iter().enumerate() {
            let Some(inst) = b.instance.upgrade() else { continue };
            let codeunit = id(inst.borrow().object as usize);
            match render.value(&Value::Codeunit(inst), 0) {
                Ok(text) => bound.push(format!("codeunit {codeunit} {text}")),
                Err(why) => out.opaque.push(format!("binding {k}: {why}")),
            }
        }
        out.lines.append(&mut render.lines);
        out.lines.push(format!("bindings [{}]", bound.join(", ")));
        out.lines.push(format!("random {}", content(&stamp.random)));
        out.lines.push(format!("group1 {}", content(&stamp.group1)));
        out.lines.push(format!(
            "counters clock {} guid key {} draws {} row versions {} committed {}",
            self.waited_ms, self.guid_key, self.next_guid, self.next_row_version, self.tx.committed_row_version
        ));
        let autoinc: BTreeMap<&(String, u32), &i64> = self.autoinc.iter().collect();
        out.lines.push(format!("autoincrement {}", content(&autoinc)));
        out.lines.push(format!("last error {}", content(&self.last_error)));
        out
    }
}
