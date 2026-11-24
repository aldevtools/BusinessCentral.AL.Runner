//! What a run of code left in the session besides the database rows: the session language, WorkDate, the number
//! sequences and isolated storage (al-testrun `login.rs` compares it before and after the apps' own login subscribers
//! ran in a session where what BC's login does is not known; the store's watch follows the rows).
//!
//! the session language, WorkDate, number sequences and isolated storage at one moment, to see what a
//! run of code changed besides rows (al-testrun `login.rs`, a watched login); `LeftStamp`, wider:
//! also the SingleInstance codeunits and their globals, the manual bindings, the Random sequence and
//! the group 1 filters (al-testrun `partner_login.rs`); `LeftStamp::changed` gives a `LeftChange` by
//! kind (for SingleInstance codeunits, the globals that changed), a new SingleInstance instance whose
//! globals hold what a fresh one holds is no change; `Session::state_lines` renders what `LeftStamp`
//! takes plus the counters code advances (clock, Guid draws, row versions, AutoIncrement) and the
//! last error as lines that compare across sessions (al-testrun `login_replay.rs`), every
//! SingleInstance global through `state_render.rs`. Entry points: `Session::state_stamp`,
//! `StateStamp::changed`, `Session::left_stamp`, `LeftStamp::changed`, `LeftStamp::group1_changed`,
//! `Session::state_lines`.

use crate::Session;
use crate::sessionops::Sequence;
use crate::state_render::content;
use al_builtins::value::{Instance, LibraryValue, RecordState, Value};
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
    /// Per global: it holds what a fresh instance of the codeunit holds ([`Session::at_declared`]). Empty when the instance has
    /// another number of globals than a fresh one.
    declared: Vec<bool>,
    /// A global holds a codeunit instance a call made whose globals hold their declared values, and it took the fingerprint
    /// of the fresh variable ([`Session::at_declared`], assumption clear-shared-codeunit).
    declared_form: bool,
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
    /// comparable form (a .NET object, a stream, a temporary record) counts as changed unless it holds what a fresh one
    /// holds at both ends (`Session::at_declared`).
    pub singles: BTreeMap<usize, BTreeSet<usize>>,
    /// The SingleInstance codeunits (object index) one of whose globals came to hold a codeunit instance a call made, with
    /// every global of it at its declared value, and counts as unchanged only through assumption clear-shared-codeunit
    /// ([`Session::at_declared`]): the caller marks their reads with it (`Session::mark_single_assumed`).
    pub declared_form: BTreeSet<usize>,
}

impl LeftChange {
    /// Nothing changed ([`LeftChange::declared_form`] is no change).
    pub fn is_empty(&self) -> bool {
        let LeftChange {
            session,
            number_sequences,
            isolated_storage,
            random,
            singles,
            declared_form: _,
        } = self;
        session.is_none() && !number_sequences && !isolated_storage && !random && singles.is_empty()
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
                        // Two values that each hold what a fresh one holds hold the same, also where neither has a
                        // comparable form (an untouched temporary record, `untouched_temp`).
                        let both_declared =
                            |g: usize| x.declared.get(g) == Some(&true) && y.declared.get(g) == Some(&true);
                        (0..x.globals.len())
                            .filter(|&g| x.globals[g] != y.globals[g] && !both_declared(g))
                            .collect()
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
        let declared_form = (later.singles.iter())
            .filter(|(o, y)| y.declared_form && !self.singles.get(o).is_some_and(|x| x.declared_form))
            .map(|(o, _)| *o)
            .collect();
        LeftChange {
            declared_form,
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
    /// Marks the reads of each SingleInstance codeunit of `change` that counted unchanged only through assumption
    /// clear-shared-codeunit ([`LeftChange::declared_form`]), when it carries no mark yet (`mark_single_assumed`).
    pub fn mark_declared_form(&mut self, change: &LeftChange) {
        for &o in &change.declared_form {
            self.mark_single_assumed(o, al_builtins::assumptions::Assumption::ClearSharedCodeunit);
        }
    }

    /// The state [`LeftStamp`] takes, now.
    pub fn left_stamp(&mut self) -> LeftStamp {
        let made: Vec<(usize, Rc<RefCell<Instance>>)> = self.singletons.iter().map(|(&o, i)| (o, i.clone())).collect();
        let mut singles = BTreeMap::new();
        for (o, inst) in made {
            let engine = self.engine.clone();
            let fresh = self.slot_defaults(&engine.program.objects[o].globals);
            let held = inst.borrow().globals.clone();
            let declared: Vec<bool> = if held.len() == fresh.len() {
                (held.iter().zip(&fresh))
                    .map(|(v, f)| self.at_declared(v, f, 0, false))
                    .collect()
            } else {
                Vec::new()
            };
            // A global at its declared value takes the fingerprint of the fresh one: a codeunit variable whose first call made
            // its instance, with every global of it at its declared value, reads as one never used (al-runner issue 8912).
            let mut declared_form = false;
            let globals = (held.iter().enumerate())
                .map(|(g, v)| {
                    let own = crate::single_track::fp(v, &engine);
                    match fresh.get(g) {
                        Some(f) if declared.get(g) == Some(&true) => {
                            let fresh = crate::single_track::fp(f, &engine);
                            // Only a codeunit variable's instance counts through its declared form (assumption
                            // clear-shared-codeunit); a temporary record's fingerprints never compare.
                            declared_form |= matches!(v, Value::Codeunit(_)) && fresh != own;
                            fresh
                        }
                        _ => own,
                    }
                })
                .collect();
            singles.insert(
                o,
                Single {
                    globals,
                    declared,
                    declared_form,
                },
            );
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

impl Session {
    /// Whether global value `v` holds what `fresh`, the value a fresh instance's global holds, holds: the same fingerprint;
    /// for a record also the same view (key, sort order, filter group, load and calculated fields, read isolation, security
    /// filtering, company, marks) and nothing read or written through it, and for a temporary record, which has no
    /// fingerprint, a table nothing wrote or shares and the rest of the record state alike (`untouched_temp`); a RecordRef
    /// never opened that nothing shares; an array without a fingerprint element by element (never one of codeunits); for a
    /// codeunit the same of each of its globals. A
    /// codeunit variable whose instance a call made (`ensure_initialized`) and a fresh one nothing used yet hold the same when
    /// each global of the instance holds its declared value: AL code cannot tell them apart, `Clear` gives both a fresh
    /// instance and `ClearAll` resets both in place (open corpus PR 1560, codeunits/clear-instance/ClearSharedCodeunit.al,
    /// every cell alike with X unused and used once; assumption clear-shared-codeunit). Only for a codeunit: a report
    /// variable's first call runs OnInitReport (reports/TestProbeReportContracts.al:ReportVariable_RunTwice_GlobalsReset).
    /// The declared values come from `slot_defaults`, never `Session::instance`, which would mark the session's records as
    /// blob copy sources (`blob_transfer::new_variable`). The instance counts so only while no other variable holds it (one
    /// that does sees what is written through it later), and only when its globals hold their declared values by a
    /// fingerprint: the probe's codeunits hold an Integer, so a temporary record or a RecordRef (`made`) in an instance a
    /// call made counts as changed, except in a store of the shape of open corpus PR 1561 (`Session::login_store`) that
    /// the SingleInstance codeunit holds directly (the same assumption).
    fn at_declared(&mut self, v: &Value, fresh: &Value, depth: u8, made: bool) -> bool {
        if depth > 8 {
            return false;
        }
        let engine = self.engine.clone();
        match (v, fresh) {
            (Value::Record(a), Value::Record(b)) => {
                let (Ok(a), Ok(b)) = (a.try_borrow(), b.try_borrow()) else {
                    return false;
                };
                if a.temporary && b.temporary {
                    let Value::Record(rc) = v else { unreachable!() };
                    return !made && same_view(&a, &b) && self.untouched_temp(rc, &a, &b, 1);
                }
                same_view(&a, &b) && crate::single_track::fp(v, &engine) == crate::single_track::fp(fresh, &engine)
            }
            (Value::Codeunit(a), Value::Codeunit(b)) => {
                let (Ok(a), Ok(b)) = (a.try_borrow(), b.try_borrow()) else {
                    return false;
                };
                let object = a.object as usize;
                let held = a.globals.clone();
                let mut inner_made = made;
                let mut measured_store = false;
                let declared = if a.object != b.object {
                    return false;
                } else if a.initialized && !b.initialized && b.globals.is_empty() && self.plain_codeunit(object) {
                    drop((a, b));
                    let Value::Codeunit(rc) = v else { unreachable!() };
                    if Rc::strong_count(rc) != 2 {
                        return false;
                    }
                    // A store of the shape of open corpus PR 1561 held directly by the SingleInstance codeunit: made by
                    // calls that wrote nothing, it reads as one never made through the owner, in the same test and the
                    // next ones (ClearSharedCodeunitSIOwner.al:5-21, A2_Step1_MadeByCallsThatWriteNothing :370 and
                    // A2_Step2_ReadX :385 against A1_Step2_FirstReadOfX :346). Any other holder or shape keeps `made`.
                    measured_store = depth == 0 && self.login_store(object);
                    inner_made = !measured_store;
                    self.slot_defaults(&engine.program.objects[object].globals)
                } else {
                    b.globals.clone()
                };
                // The elements of an array of temporary records share one table ("CLS Arr Store", ClearSharedCodeunitOwnerKinds.al
                // M2_Step2_ReadX :1205): in a measured store, those sharers are the array itself.
                held.len() == declared.len()
                    && (held.iter().zip(&declared)).all(|(x, y)| {
                        (measured_store && self.untouched_temp_array(x, y))
                            || self.at_declared(x, y, depth + 1, inner_made)
                    })
            }
            // A RecordRef never opened that no other variable holds: what a fresh one holds, every member reads as on one never
            // opened (recordref-fieldref/UhUnopenedRecordRef.al:487 Untouched_Number). A closed or cleared one holds another
            // state (al-builtins `recordref_closed.rs`). It has no fingerprint (al-builtins `value_codec`).
            (Value::Library(k, a), Value::Library(_, b)) if *k == al_builtins::library_kind::LibraryKind::RecordRef => {
                let (Ok(x), Ok(y)) = (a.try_borrow(), b.try_borrow()) else {
                    return false;
                };
                let empty = |l: &LibraryValue| matches!(l, LibraryValue::Empty);
                !made && empty(&x) && empty(&y) && Rc::strong_count(a) == 2
            }
            // An array without a fingerprint compares element by element; never one of codeunits, whose
            // declared form is the codeunit global's alone (`left_stamp`).
            (Value::Array(a), Value::Array(b))
                if crate::single_track::fp(v, &engine) != crate::single_track::fp(fresh, &engine) =>
            {
                let (Ok(x), Ok(y)) = (a.try_borrow(), b.try_borrow()) else {
                    return false;
                };
                let (held, declared) = (x.clone(), y.clone());
                drop((x, y));
                Rc::strong_count(a) == 2
                    && held.len() == declared.len()
                    && !held.iter().any(|e| matches!(e, Value::Codeunit(_)))
                    && (held.iter().zip(&declared)).all(|(e, d)| self.at_declared(e, d, depth + 1, made))
            }
            _ => crate::single_track::fp(v, &engine) == crate::single_track::fp(fresh, &engine),
        }
    }
}

impl Session {
    /// Whether temporary record `a` (held in `rc`) holds what `b`, a fresh variable's temporary record, holds: its table was
    /// never written (no store yet, or one whose change counter never moved, so no row was ever there) and no other record
    /// holds it, and every other part of the record state equals `b`'s. A temporary record's rows are seen only through the
    /// records that share its table: a second variable has a table of its own (record/TestTempTableContracts.al:37-46), and so
    /// do a copy made with `Copy(x, false)` (:82-97) and a copy handed by value (:118-126); `Copy(x, true)` (:101-112) and a var
    /// parameter (:130-138) share it, so a table another record holds (`TempId::sharers`) or a record state another holder
    /// holds counts as changed, as does a by-value copy's table and one whose rows are not measured. The table's number
    /// (`temp_id`) is the identity those shares name, and `probed_source` is the Blob probes' mark of the variable
    /// (al-builtins `ProbedSource`): neither is compared. The caller's copy of the globals holds `rc` once besides the
    /// instance.
    /// `sharers` is how many records hold the table as the variable itself: one, or the elements of an array.
    fn untouched_temp(&self, rc: &Rc<RefCell<RecordState>>, a: &RecordState, b: &RecordState, sharers: usize) -> bool {
        let Some(t) = &a.temp_id else {
            return false;
        };
        let never_written = self.temps.get(&t.id()).is_none_or(|s| s.change_counter() == 0);
        let alone = t.sharers() == sharers && Rc::strong_count(rc) == 2;
        let own = |r: &RecordState| {
            r.own_read.0.is_none()
                && r.rolled_read.0.is_none()
                && r.globals.instances().is_empty()
                && r.globals.xrec_instances().is_empty()
        };
        // A clone drops the table's number, the own read, the rolled read, the probed source and the table globals (checked
        // above): what is left is the rest of the record state, field values and view included.
        let rest = |r: &RecordState| {
            let mut c = r.clone();
            c.temp_id = None;
            c.probed_source = Default::default();
            content(&c)
        };
        never_written
            && alone
            && !t.is_by_value_copy()
            && !t.rows_unmeasured()
            && own(a)
            && own(b)
            && rest(a) == rest(b)
    }
}

impl Session {
    /// Whether `v`, an array of temporary records whose elements share one table, holds what `fresh` holds: no other variable
    /// holds the array, and each element holds what [`Session::untouched_temp`] reads as fresh, its table shared by the
    /// elements only.
    fn untouched_temp_array(&self, v: &Value, fresh: &Value) -> bool {
        let (Value::Array(a), Value::Array(b)) = (v, fresh) else {
            return false;
        };
        let (held, declared) = (a.borrow().clone(), b.borrow().clone());
        let ids: Vec<Option<u64>> = (held.iter())
            .map(|e| match e {
                Value::Record(r) => r.borrow().temp_id.as_ref().map(|t| t.id()),
                _ => None,
            })
            .collect();
        Rc::strong_count(a) == 2
            && held.len() == declared.len()
            && ids
                .first()
                .is_some_and(|id| id.is_some() && ids.iter().all(|x| x == id))
            && (held.iter().zip(&declared)).all(|(e, d)| match (e, d) {
                (Value::Record(rc), Value::Record(f)) => {
                    let (Ok(x), Ok(y)) = (rc.try_borrow(), f.try_borrow()) else {
                        return false;
                    };
                    x.temporary && y.temporary && same_view(&x, &y) && self.untouched_temp(rc, &x, &y, held.len())
                }
                _ => false,
            })
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
        && !a.touched
        && !a.assigned
        && no_row_io(a)
}

/// Whether record `a` read or wrote no row: no buffer loaded, no position, no insert, modify or failed insert, no row
/// origin.
pub(crate) fn no_row_io(a: &RecordState) -> bool {
    a.load.buffer.is_none()
        && a.position.is_none()
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
    /// The values with no comparable form (`state_render.rs`: a .NET object, an open stream, ...; the SQL counters when
    /// not at rest): what they hold cannot be compared, so two sessions holding one are never known to hold the same.
    pub opaque: Vec<NotComparable>,
}

/// A value with no comparable form in [`StateLines`]: where it is held and why it has none, apart, so a report can name
/// the place without cutting a line (al-testrun `login_replay/difference.rs`, issue 8211).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotComparable {
    /// Where the value is held, by ids and indexes only (`codeunit 5 global 1`, `binding 0`, `SQL counters`).
    pub path: String,
    /// Why it has no comparable form, by type names only (`state_render.rs`).
    pub why: String,
}

impl NotComparable {
    /// The value at `path` with no comparable form, for `why`.
    pub(crate) fn new(path: String, why: impl Into<String>) -> NotComparable {
        NotComparable { path, why: why.into() }
    }

    /// `path: why`, the form a digest of the state takes.
    pub fn line(&self) -> String {
        format!("{}: {}", self.path, self.why)
    }
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
    /// codeunits' globals, the manual bindings, the `Random` sequence, the group 1 filters), the state of the `Random`
    /// sequence and its draws by origin, plus the counters code advances (the clock offset, the clock's draws, key and
    /// last system stamp, the Guid and SystemId draws and their key, the row versions), the last error and the facts of
    /// `state_facts.rs`, as lines that compare across sessions: an instance is named by a number by first sight, not by
    /// its address (al-testrun `login_replay.rs`, al-runner issue 8211). It is not a pure read: `left_stamp` builds a
    /// fresh instance's globals to compare with, and each temporary record among them takes a temporary table number
    /// (`Session::new_temp_id`), so the session's counter moves on. Harmless: the number only names a temporary table's
    /// store inside the session, AL code never sees it, and al-testrun's watched and partner logins call `left_stamp`
    /// the same way.
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
                    Err(why) => (out.opaque).push(NotComparable::new(format!("codeunit {} global {g}", id(o)), why)),
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
                Err(why) => out.opaque.push(NotComparable::new(format!("binding {k}"), why)),
            }
        }
        out.lines.append(&mut render.lines);
        out.lines.push(format!("bindings [{}]", bound.join(", ")));
        // The sequence itself, not only how many numbers it drew: `Randomize(n)` restarts it without a draw.
        out.lines.push(format!(
            "random {} sequence {} unseeded base {} test reseeded {} run seeded {} inherited {} carried {}",
            content(&stamp.random),
            content(&self.rng),
            self.unseeded_base,
            self.reseeded_in_test,
            self.run_seeded_in_test,
            self.inherited_draws,
            self.carried_draws
        ));
        out.lines.push(format!("group1 {}", content(&stamp.group1)));
        out.lines.push(format!(
            "counters clock {} draws {} key {} last stamp {} guid key {} draws {} row versions {} committed {}",
            self.waited_ms,
            self.clock_draws,
            self.clock_key,
            self.last_stamp,
            self.guid_key,
            self.next_guid,
            self.next_row_version,
            self.tx.committed_row_version
        ));
        let autoinc: BTreeMap<&(String, u32), &i64> = self.autoinc.iter().collect();
        out.lines.push(format!("autoincrement {}", content(&autoinc)));
        out.lines.push(format!("last error {}", content(&self.last_error)));
        self.fact_lines(&mut out.lines, &mut out.opaque);
        out
    }
}

#[cfg(test)]
mod tests {
    use al_testkit::Built;

    const MEMO: &str = r#"
codeunit 50100 Store
{
    var
        Name: Text;
        Inner: Codeunit Inner;

    procedure Get(): Text
    begin
        exit(Name);
    end;

    procedure Set()
    begin
        Name := 'x';
    end;

    procedure GetInner(): Integer
    begin
        exit(Inner.Get());
    end;
}

codeunit 50101 Inner
{
    var
        N: Integer;

    procedure Get(): Integer
    begin
        exit(N);
    end;
}

report 50103 Rep
{
    ProcessingOnly = true;

    var
        N: Integer;

    procedure Get(): Integer
    begin
        exit(N);
    end;
}

codeunit 50102 Memo
{
    SingleInstance = true;

    var
        Storage: Codeunit Store;
        R: Report Rep;
        Alias: Codeunit Store;
        Ref: RecordRef;
        RefAlias: RecordRef;

    procedure Touch()
    begin
    end;

    procedure Read()
    begin
        Storage.Get();
    end;

    procedure ReadInner()
    begin
        Storage.GetInner();
    end;

    procedure Write()
    begin
        Storage.Set();
    end;

    procedure ReadReport()
    begin
        R.Get();
    end;

    procedure ShareRead()
    begin
        Alias := Storage;
        Storage.Get();
    end;

    procedure ShareRef()
    begin
        RefAlias := Ref;
    end;
}

codeunit 50104 Driver
{
    procedure Touch() var M: Codeunit Memo; begin M.Touch(); end;
    procedure Read() var M: Codeunit Memo; begin M.Read(); end;
    procedure ReadInner() var M: Codeunit Memo; begin M.ReadInner(); end;
    procedure Write() var M: Codeunit Memo; begin M.Write(); end;
    procedure ReadReport() var M: Codeunit Memo; begin M.ReadReport(); end;
    procedure ShareRead() var M: Codeunit Memo; begin M.ShareRead(); end;
    procedure ShareRef() var M: Codeunit Memo; begin M.ShareRef(); end;
}
"#;

    /// The globals of codeunit Memo that changed between its first use (`Touch`) and a call of `routine`, on an instance
    /// made before (`made`) or by the call itself. Driver calls Memo through a variable, so the call reaches the session's
    /// single instance.
    fn memo_change(routine: &str, made: bool) -> std::collections::BTreeSet<usize> {
        memo_left(routine, made).0
    }

    /// [`memo_change`], with the number of codeunits that counted unchanged only through their declared form and whether
    /// nothing changed.
    fn memo_left(routine: &str, made: bool) -> (std::collections::BTreeSet<usize>, usize, bool) {
        let mut b = Built::from_source(MEMO);
        if made {
            b.run_in("Driver", "Touch").unwrap();
        }
        let before = b.session.left_stamp();
        b.run_in("Driver", routine).unwrap();
        let change = before.changed(&b.session.left_stamp());
        let (form, empty) = (change.declared_form.len(), change.is_empty());
        (change.singles.into_values().next().unwrap_or_default(), form, empty)
    }

    // A codeunit global that counts unchanged only through its declared form is named, for the caller to mark its reads
    // with assumption clear-shared-codeunit; one that changed, or a report's, is not.
    #[test]
    fn a_codeunit_global_counted_through_its_declared_form_is_named() {
        for made in [true, false] {
            assert_eq!(memo_left("Read", made).1, 1, "made before: {made}");
            assert!(memo_left("Read", made).2, "made before: {made}");
            assert_eq!(memo_left("Write", made).1, 0, "made before: {made}");
            assert_eq!(memo_left("ReadReport", made).1, 0, "made before: {made}");
        }
    }

    // al-runner issue 8912: the first call on a codeunit variable makes its instance; while every global of it holds its
    // declared value, nothing AL code reads differs, so it is no change, for a new single instance and a known one alike.
    #[test]
    fn a_codeunit_global_first_used_with_its_declared_globals_is_no_change() {
        for made in [true, false] {
            assert!(memo_change("Read", made).is_empty(), "made before: {made}");
            assert!(memo_change("ReadInner", made).is_empty(), "made before: {made}");
        }
    }

    // A report variable's first call runs OnInitReport (reports/TestProbeReportContracts.al:ReportVariable_RunTwice_GlobalsReset),
    // so its first use stays a change.
    #[test]
    fn a_report_global_first_used_is_a_change() {
        for made in [true, false] {
            assert_eq!(memo_change("ReadReport", made), [1].into(), "made before: {made}");
        }
    }

    // A codeunit global made by its first call whose instance a second global holds is a change: a write through either
    // shows in the other, which two fresh variables do not do.
    #[test]
    fn a_codeunit_global_made_by_a_call_and_shared_is_a_change() {
        for made in [true, false] {
            assert!(memo_change("ShareRead", made).contains(&0), "made before: {made}");
        }
    }

    // A RecordRef the SingleInstance codeunit holds directly and never opened holds what a fresh one holds: every member reads
    // as on one never opened (recordref-fieldref/UhUnopenedRecordRef.al:487 Untouched_Number), so it is no change, with no
    // mark. One another RecordRef variable holds is a change.
    #[test]
    fn a_never_opened_record_ref_global_is_no_change_unless_shared() {
        for made in [true, false] {
            assert_eq!(
                memo_left("Touch", made),
                (Default::default(), 0, true),
                "made before: {made}"
            );
            assert!(memo_change("ShareRef", made).contains(&3), "made before: {made}");
        }
    }

    #[test]
    fn a_codeunit_global_whose_global_was_set_is_a_change() {
        for made in [true, false] {
            assert_eq!(memo_change("Write", made), [0].into(), "made before: {made}");
        }
    }
    const TEMP_MEMO: &str = r#"
table 50110 Row
{
    fields
    {
        field(1; Id; Integer) { }
    }
}

codeunit 50111 TempStore
{
    var
        Rows: Record Row temporary;
        Backups: array[2, 3] of RecordRef;
        Indices: array[2] of Integer;

    procedure Get(): Integer
    begin
        exit(Indices[1]);
    end;

    procedure OpenRef()
    begin
        Backups[1, 1].Open(50110, true);
    end;

    procedure ShareRef()
    begin
        Backups[1, 2] := Backups[1, 1];
    end;

    procedure InsertRow()
    begin
        Rows.Insert();
    end;
}

codeunit 50114 TempArrStore
{
    var
        Rows: array[2] of Record Row temporary;
        Backups: array[2, 3] of RecordRef;
        Indices: array[2] of Integer;

    procedure Get(): Integer
    begin
        exit(Indices[1]);
    end;
}

codeunit 50115 TempStoreExtra
{
    var
        Rows: Record Row temporary;
        Backups: array[2, 3] of RecordRef;
        Indices: array[2] of Integer;
        Name: Text;

    procedure Get(): Integer
    begin
        exit(Indices[1]);
    end;
}

codeunit 50118 TempStoreLabel
{
    var
        Rows: Record Row temporary;
        Backups: array[2, 3] of RecordRef;
        NotFound: Label 'Could not find %1.';
        Indices: array[2] of Integer;

    procedure Get(): Integer
    begin
        exit(Indices[1]);
    end;
}

codeunit 50119 LabelFirst
{
    var
        NotFound: Label 'Could not find %1.';
        Rows: Record Row temporary;
        Backups: array[2, 3] of RecordRef;
        Indices: array[2] of Integer;

    procedure Get(): Integer
    begin
        exit(Indices[1]);
    end;
}

codeunit 50120 LabelMid
{
    var
        Rows: Record Row temporary;
        NotFound: Label 'Could not find %1.';
        Backups: array[2, 3] of RecordRef;
        Indices: array[2] of Integer;

    procedure Get(): Integer
    begin
        exit(Indices[1]);
    end;
}

codeunit 50121 LabelLast
{
    var
        Rows: Record Row temporary;
        Backups: array[2, 3] of RecordRef;
        Indices: array[2] of Integer;
        NotFound: Label 'Could not find %1.';

    procedure Get(): Integer
    begin
        exit(Indices[1]);
    end;
}

codeunit 50122 LabelLocked
{
    var
        Rows: Record Row temporary;
        Backups: array[2, 3] of RecordRef;
        Indices: array[2] of Integer;
        NotFound: Label 'Could not find %1.', Locked = true;

    procedure Get(): Integer
    begin
        exit(Indices[1]);
    end;
}

codeunit 50123 ArrStore1
{
    var
        Rows: array[2] of Record Row temporary;
        Refs: array[2] of RecordRef;
        Ints: array[3] of Integer;

    procedure Get(): Integer
    begin
        exit(Ints[1]);
    end;

    procedure Write()
    begin
        Rows[2].Insert();
    end;
}

codeunit 50124 TwoLabels
{
    var
        NotFound: Label 'Could not find %1.';
        Rows: Record Row temporary;
        Backups: array[2, 3] of RecordRef;
        Indices: array[2] of Integer;
        Other: Label 'Other %1.';

    procedure Get(): Integer
    begin
        exit(Indices[1]);
    end;
}

codeunit 50116 TempStoreSwapped
{
    var
        Backups: array[2, 3] of RecordRef;
        Rows: Record Row temporary;
        Indices: array[2] of Integer;

    procedure Get(): Integer
    begin
        exit(Indices[1]);
    end;
}

codeunit 50117 TempWrap
{
    var
        Inner: Codeunit TempStore;

    procedure Get(): Integer
    begin
        exit(Inner.Get());
    end;
}

codeunit 50112 TempMemo
{
    SingleInstance = true;

    var
        Rows: Record Row temporary;
        Store: Codeunit TempStore;
        Other: Record Row temporary;
        Ref: RecordRef;
        ArrStore: Codeunit TempArrStore;
        Extra: Codeunit TempStoreExtra;
        Swapped: Codeunit TempStoreSwapped;
        Wrap: Codeunit TempWrap;
        StoreAlias: Codeunit TempStore;
        Labeled: Codeunit TempStoreLabel;
        First: Codeunit LabelFirst;
        Mid: Codeunit LabelMid;
        Last: Codeunit LabelLast;
        Locked: Codeunit LabelLocked;
        Arr1: Codeunit ArrStore1;
        Two: Codeunit TwoLabels;

    procedure UseFirst() begin First.Get(); end;
    procedure UseMid() begin Mid.Get(); end;
    procedure UseLast() begin Last.Get(); end;
    procedure UseLocked() begin Locked.Get(); end;
    procedure UseArr1() begin Arr1.Get(); end;
    procedure UseArr1Write() begin Arr1.Write(); end;
    procedure UseTwo() begin Two.Get(); end;

    procedure UseLabeled()
    begin
        Labeled.Get();
    end;

    procedure UseArrStore()
    begin
        ArrStore.Get();
    end;

    procedure UseExtra()
    begin
        Extra.Get();
    end;

    procedure UseSwapped()
    begin
        Swapped.Get();
    end;

    procedure UseWrap()
    begin
        Wrap.Get();
    end;

    procedure ShareStore()
    begin
        StoreAlias := Store;
        Store.Get();
    end;

    procedure Touch()
    begin
    end;

    procedure UseStore()
    begin
        Store.Get();
    end;

    procedure ReadRows()
    begin
        if not Rows.IsEmpty() then
            Error('rows');
    end;

    procedure InsertDelete()
    begin
        Rows.Id := 1;
        Rows.Insert();
        Rows.Delete();
        Rows.Id := 0;
    end;

    procedure Share()
    begin
        Other.Copy(Rows, true);
    end;

    procedure Filter()
    begin
        Rows.SetRange(Id, 1);
    end;

    procedure StoreOpenRef()
    begin
        Store.OpenRef();
    end;

    procedure GetTable()
    begin
        Ref.GetTable(Rows);
    end;

    procedure Assign()
    begin
        Other := Rows;
    end;

    procedure Mark()
    begin
        Rows.Mark(true);
    end;

    procedure FilterGroup()
    begin
        Rows.FilterGroup(2);
    end;

    procedure Descending()
    begin
        Rows.Ascending(false);
    end;

    procedure LoadFields()
    begin
        Rows.SetLoadFields(Id);
    end;

    procedure Security()
    begin
        Rows.SecurityFiltering(SecurityFilter::Ignored);
    end;

    procedure StoreShareRef()
    begin
        Store.ShareRef();
    end;

    procedure StoreInsertRow()
    begin
        Store.InsertRow();
    end;
}

codeunit 50113 TempDriver
{
    procedure Touch() var M: Codeunit TempMemo; begin M.Touch(); end;
    procedure UseStore() var M: Codeunit TempMemo; begin M.UseStore(); end;
    procedure ReadRows() var M: Codeunit TempMemo; begin M.ReadRows(); end;
    procedure InsertDelete() var M: Codeunit TempMemo; begin M.InsertDelete(); end;
    procedure Share() var M: Codeunit TempMemo; begin M.Share(); end;
    procedure Filter() var M: Codeunit TempMemo; begin M.Filter(); end;
    procedure StoreOpenRef() var M: Codeunit TempMemo; begin M.StoreOpenRef(); end;
    procedure GetTable() var M: Codeunit TempMemo; begin M.GetTable(); end;
    procedure Assign() var M: Codeunit TempMemo; begin M.Assign(); end;
    procedure Mark() var M: Codeunit TempMemo; begin M.Mark(); end;
    procedure FilterGroup() var M: Codeunit TempMemo; begin M.FilterGroup(); end;
    procedure Descending() var M: Codeunit TempMemo; begin M.Descending(); end;
    procedure LoadFields() var M: Codeunit TempMemo; begin M.LoadFields(); end;
    procedure Security() var M: Codeunit TempMemo; begin M.Security(); end;
    procedure StoreShareRef() var M: Codeunit TempMemo; begin M.StoreShareRef(); end;
    procedure StoreInsertRow() var M: Codeunit TempMemo; begin M.StoreInsertRow(); end;
    procedure UseArrStore() var M: Codeunit TempMemo; begin M.UseArrStore(); end;
    procedure UseExtra() var M: Codeunit TempMemo; begin M.UseExtra(); end;
    procedure UseSwapped() var M: Codeunit TempMemo; begin M.UseSwapped(); end;
    procedure UseWrap() var M: Codeunit TempMemo; begin M.UseWrap(); end;
    procedure ShareStore() var M: Codeunit TempMemo; begin M.ShareStore(); end;
    procedure UseLabeled() var M: Codeunit TempMemo; begin M.UseLabeled(); end;
    procedure UseFirst() var M: Codeunit TempMemo; begin M.UseFirst(); end;
    procedure UseMid() var M: Codeunit TempMemo; begin M.UseMid(); end;
    procedure UseLast() var M: Codeunit TempMemo; begin M.UseLast(); end;
    procedure UseLocked() var M: Codeunit TempMemo; begin M.UseLocked(); end;
    procedure UseArr1() var M: Codeunit TempMemo; begin M.UseArr1(); end;
    procedure UseArr1Write() var M: Codeunit TempMemo; begin M.UseArr1Write(); end;
    procedure UseTwo() var M: Codeunit TempMemo; begin M.UseTwo(); end;
}
"#;

    /// [`memo_left`] for codeunit TempMemo, whose globals are temporary records and a codeunit holding one.
    fn temp_memo_left(routine: &str, made: bool) -> (std::collections::BTreeSet<usize>, usize) {
        let mut b = Built::from_source(TEMP_MEMO);
        if made {
            b.run_in("TempDriver", "Touch").unwrap();
        }
        let before = b.session.left_stamp();
        b.run_in("TempDriver", routine).unwrap();
        let change = before.changed(&b.session.left_stamp());
        (
            change.singles.into_values().next().unwrap_or_default(),
            change.declared_form.len(),
        )
    }

    // al-runner issue 8912 ("Backup Management" after the login of NP Retail): a temporary record global whose table nothing
    // wrote and nothing shares, with the view and the values of a fresh one, holds what a fresh one holds: no change, and no
    // mark.
    #[test]
    fn an_untouched_temporary_record_global_is_no_change() {
        for made in [true, false] {
            assert_eq!(
                temp_memo_left("Touch", made),
                (Default::default(), 0),
                "made before: {made}"
            );
            assert_eq!(
                temp_memo_left("ReadRows", made),
                (Default::default(), 0),
                "made before: {made}"
            );
        }
    }

    // A temporary table written to (even emptied again), shared with another record (`Copy(x, true)`,
    // record/TestTempTableContracts.al:101-112) or filtered is a change.
    #[test]
    fn a_written_shared_or_filtered_temporary_record_global_is_a_change() {
        for made in [true, false] {
            assert!(
                temp_memo_left("InsertDelete", made).0.contains(&0),
                "made before: {made}"
            );
            assert!(temp_memo_left("Share", made).0.contains(&0), "made before: {made}");
            assert!(temp_memo_left("Filter", made).0.contains(&0), "made before: {made}");
            for routine in [
                "GetTable",
                "Mark",
                "FilterGroup",
                "Descending",
                "LoadFields",
                "Security",
            ] {
                assert!(
                    temp_memo_left(routine, made).0.contains(&0),
                    "{routine}, made before: {made}"
                );
            }
        }
    }

    // A store of the shape of open corpus PR 1561 (a temporary record, an array of RecordRef of two dimensions, an array of
    // Integer) that the SingleInstance codeunit holds and a call made while writing nothing reads
    // as one never made (ClearSharedCodeunitSIOwner.al A2_Step2_ReadX): no change, counted through its declared form, so
    // its reads carry assumption clear-shared-codeunit.
    #[test]
    fn a_temp_ref_store_made_by_a_call_that_wrote_nothing_is_no_change() {
        for made in [true, false] {
            assert_eq!(
                temp_memo_left("UseStore", made),
                (Default::default(), 1),
                "made before: {made}"
            );
        }
    }

    // The same store with one global `Label 'text'` before, between the temporary record and the RecordRef array, or after
    // (ClearSharedCodeunitLabelStore.al First_L2_Step2_ReadX :1344, Mid_L2_Step2_ReadX :1541, Last_L2_Step2_ReadX :1738),
    // and "CLS Arr Store" (ClearSharedCodeunitOwnerKinds.al M2_Step2_ReadX :1205) read as never made too.
    #[test]
    fn a_label_store_or_array_store_made_by_a_call_that_wrote_nothing_is_no_change() {
        for made in [true, false] {
            for routine in ["UseFirst", "UseMid", "UseLast", "UseArr1"] {
                assert_eq!(
                    temp_memo_left(routine, made),
                    (Default::default(), 1),
                    "{routine}, made before: {made}"
                );
            }
        }
    }

    // In a codeunit global made by its first call, an opened or shared RecordRef or a written temporary table is a change.
    // So is the made store of any shape or holder 1561 does not measure: an array of temporary records, one more global,
    // a global Label between the RecordRef array and the Integer array, a Label with Locked, two Labels (not measured), an
    // array store whose table was written,
    // another order of the globals, a store inside a codeunit the SingleInstance codeunit holds, and a store another
    // global shares. Nothing counts through the declared form.
    #[test]
    fn a_written_or_unmeasured_store_made_by_a_call_is_a_change() {
        for made in [true, false] {
            for (routine, global) in [
                ("StoreOpenRef", 1),
                ("StoreShareRef", 1),
                ("StoreInsertRow", 1),
                ("UseArrStore", 4),
                ("UseExtra", 5),
                ("UseSwapped", 6),
                ("UseWrap", 7),
                ("UseLabeled", 9),
                ("UseLocked", 13),
                ("UseTwo", 15),
                ("UseArr1Write", 14),
            ] {
                assert_eq!(
                    temp_memo_left(routine, made),
                    ([global].into(), 0),
                    "{routine}, made before: {made}"
                );
            }
            assert!(temp_memo_left("ShareStore", made).0.contains(&1), "made before: {made}");
        }
    }

    // The clock's draws and its last system stamp are state lines: a later stamp lands after the last one, past it by a
    // draw of the clock key (`Session::stamp_millis`), so two sessions that differ only in them stamp differently.
    #[test]
    fn the_clock_draws_and_the_last_stamp_are_state_lines() {
        let (_, l) = al_sema::lower::compile(Vec::new(), &Default::default());
        let mut session = crate::Session::new(crate::Engine::new(l.program, Vec::new()));
        let base = session.state_lines();
        session.clock_draws += 1;
        let drawn = session.state_lines();
        assert_ne!(drawn, base);
        session.clock_draws -= 1;
        session.last_stamp += 1;
        assert_ne!(session.state_lines(), base);
    }
}
