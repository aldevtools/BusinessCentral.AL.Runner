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
    /// The SingleInstance globals with no comparable form (a .NET object, a stream, a temporary record): what they hold
    /// cannot be compared, so two sessions holding one are never known to hold the same.
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
        out.lines.push(format!("session {:?}", stamp.state));
        // Temporary tables and codeunit instances are numbered by first sight, in this fixed order of the globals.
        let mut render = Render {
            session: self,
            temp_tables: Vec::new(),
            instances: Vec::new(),
            lines: Vec::new(),
        };
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
        out.lines.append(&mut render.lines);
        let mut bound: Vec<u32> = (self.events.bindings.iter())
            .filter_map(|b| b.instance.upgrade().map(|i| id(i.borrow().object as usize)))
            .collect();
        bound.sort_unstable();
        out.lines.push(format!("bindings {bound:?}"));
        out.lines.push(format!("random {:?}", stamp.random));
        out.lines.push(format!("group1 {:?}", stamp.group1));
        out.lines.push(format!(
            "counters clock {} guid key {} draws {} row versions {} committed {}",
            self.waited_ms, self.guid_key, self.next_guid, self.next_row_version, self.tx.committed_row_version
        ));
        let autoinc: BTreeMap<&(String, u32), &i64> = self.autoinc.iter().collect();
        out.lines.push(format!("autoincrement {autoinc:?}"));
        out.lines.push(format!("last error {:?}", self.last_error));
        out
    }
}

/// Renders the values of SingleInstance globals as text that compares across sessions ([`Session::state_lines`]). A value
/// with no comparable form gives `Err` with why and its path (ids, indexes and type names only); [`Session::state_lines`]
/// then names it as opaque, never as equal.
struct Render<'a> {
    session: &'a Session,
    /// The temporary tables seen, by id: records that share one (`Copy(x, true)`) name one table.
    temp_tables: Vec<u64>,
    /// The codeunit instances seen, by address (never written out): two globals that hold one instance name one number,
    /// two instances two; a cycle ends at an instance already numbered.
    instances: Vec<usize>,
    /// What a numbered table or instance holds, one line each, written once.
    lines: Vec<String>,
}

impl Render<'_> {
    /// `v` as text, by the first rule that fits:
    /// - an array element by element ([`Render::array`]; on a real app an instance a login leaves holds one);
    /// - a codeunit instance by its number, and once its codeunit id and globals by these rules (on a real app a global of
    ///   a SingleInstance codeunit holds one after the login);
    /// - a value with a fingerprint by the fingerprint (`single_track::fp`);
    /// - a ModuleInfo by value (System Application's "System Initialization Impl." holds one after the login on BC 28.5);
    /// - a temporary record by everything it carries (view, filters of every group, key, position, buffer) and its
    ///   table's rows, once per table (Base Application's "Acc. Sched. KPI Event Handler" and "Change Log Management"
    ///   hold some after the login on BC 28.5).
    fn value(&mut self, v: &Value, depth: u8) -> Result<String, String> {
        if let Value::Codeunit(rc) = v {
            return self.instance(rc, depth);
        }
        if let Value::Array(a) = v {
            return self.array(a, depth);
        }
        let what = match crate::single_track::fp(v, &self.session.engine) {
            crate::single_track::Fp::Val(bytes) => return Ok(format!("{bytes:?}")),
            crate::single_track::Fp::Opaque(what) => what,
        };
        if let Some(m) = module_info(Some(v)) {
            return Ok(m);
        }
        let Value::Record(r) = v else {
            return Err(what.to_string());
        };
        let Ok(r) = r.try_borrow() else {
            return Err(format!("{what} (in use)"));
        };
        let Some(temp) = (r.temp_id.as_ref()).filter(|t| r.temporary && !r.crm && !t.rows_unmeasured()) else {
            return Err(what.to_string());
        };
        let n = match self.temp_tables.iter().position(|t| *t == temp.id()) {
            Some(n) => n,
            None => {
                self.temp_tables.push(temp.id());
                let n = self.temp_tables.len() - 1;
                let rows = self.session.temps.get(&temp.id()).map(|s| s.export_current());
                for t in rows.unwrap_or_default() {
                    for (_, row) in &t.rows {
                        self.lines
                            .push(format!("temp table {n} row {}|{}|{row:?}", t.company, t.object));
                    }
                }
                n
            }
        };
        let mut view = r.clone();
        view.temp_id = None;
        Ok(format!("temp record of table {n}: {view:?}"))
    }

    /// An array: `array of <n> elements [...]`, every element by these rules (an instance in it keeps its number), runs of
    /// equal elements written once as `[a..b]: <element>` (exact: no element is sampled or left out). `Err`: the first
    /// element with no comparable form (`array of <n> elements, element [<i>]: ...`).
    fn array(&mut self, a: &Rc<RefCell<Vec<Value>>>, depth: u8) -> Result<String, String> {
        let Ok(a) = a.try_borrow() else {
            return Err("array (in use)".into());
        };
        if depth > 8 {
            return Err(format!("array of {} elements: nested more than 8 deep", a.len()));
        }
        let mut runs: Vec<(usize, usize, String)> = Vec::new();
        for (i, e) in a.iter().enumerate() {
            let text = self
                .value(e, depth + 1)
                .map_err(|why| format!("array of {} elements, element [{i}]: {why}", a.len()))?;
            match runs.last_mut() {
                Some((_, last, same)) if *same == text => *last = i,
                _ => runs.push((i, i, text)),
            }
        }
        let runs: Vec<String> = (runs.into_iter())
            .map(|(first, last, text)| match first == last {
                true => format!("[{first}]: {text}"),
                false => format!("[{first}..{last}]: {text}"),
            })
            .collect();
        Ok(format!("array of {} elements [{}]", a.len(), runs.join(", ")))
    }

    /// A codeunit instance: `instance <n>`, and once `instance <n> codeunit <id> globals [...]`. `Err`: why it has no
    /// comparable form, with the path to the value that has none (`instance of codeunit <id> global <k>: ...`).
    fn instance(&mut self, rc: &Rc<RefCell<Instance>>, depth: u8) -> Result<String, String> {
        let address = Rc::as_ptr(rc) as usize;
        if let Some(n) = self.instances.iter().position(|a| *a == address) {
            return Ok(format!("instance {n}"));
        }
        let Ok(inst) = rc.try_borrow() else {
            return Err("Codeunit instance (in use)".into());
        };
        let id = self.session.engine.program.objects[inst.object as usize]
            .id
            .unwrap_or(0);
        if depth > 8 {
            return Err(format!("instance of codeunit {id}: nested more than 8 instances deep"));
        }
        self.instances.push(address);
        let n = self.instances.len() - 1;
        let globals = (inst.globals.iter().enumerate())
            .map(|(k, g)| {
                self.value(g, depth + 1)
                    .map_err(|why| format!("instance of codeunit {id} global {k}: {why}"))
            })
            .collect::<Result<Vec<String>, String>>()?;
        self.lines
            .push(format!("instance {n} codeunit {id} globals [{}]", globals.join(", ")));
        Ok(format!("instance {n}"))
    }
}

/// A `ModuleInfo` value as text ([`Session::state_lines`]): the manifest it holds, or that it holds none yet.
fn module_info(v: Option<&Value>) -> Option<String> {
    let Some(Value::Library(al_builtins::library_kind::LibraryKind::ModuleInfo, l)) = v else {
        return None;
    };
    match &*l.try_borrow().ok()? {
        al_builtins::value::LibraryValue::ModuleInfo(m) => Some(format!("{m:?}")),
        al_builtins::value::LibraryValue::Empty => Some("ModuleInfo (none)".into()),
        _ => None,
    }
}
