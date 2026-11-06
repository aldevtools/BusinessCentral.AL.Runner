//! SingleInstance state carried from one test codeunit to the next (probe P7 of ADR-066, al-runner issue 6924).
//!
//! Within one test runner run, BC keeps the globals of a `SingleInstance = true` codeunit from one test codeunit to the
//! next: what test codeunit A set, in a test method and in its OnRun, is still there in the next test codeunit B
//! (al-language/codeunits/TestCuP7BReader.al:24-37, against TestCuP7ASetter.al:24-58; identical on BC 27.5, 28.5 and
//! 29.0). al-runner runs each test codeunit on a session of its own, so the runner hands the state on itself.
//!
//! While a session [`tracks`](Session::track_single_instances) them, each global of a single instance is noted at its
//! first use in the session: read first (its value then is what the codeunit saw of the state it started from) or
//! written first (whatever the state held, the codeunit never saw it). A run can then be checked against the state the
//! serial run would have given it: when every global it read first held the same value in both, the run is the serial
//! one. What it hands on ([`SingleOut::wrote`]) is the end value of every global it used, and of every other global
//! whose value changed by a way the notes do not see.
//!
//! What is carried, and what each session-relative part of a value means in the next session, is in
//! `single_codec.rs`: simple values, codeunit values, .NET enum values and the .NET objects the runtime serves itself,
//! lists, dictionaries, arrays, records and temporary records with their rows, RecordRefs and RecordIds. A value that
//! has no exact form in another session (a .NET object of the session's .NET process, a stream, a record buffer of a
//! row BC rolled back, ...) is carried as [`Carry::Opaque`], and a later codeunit that reads it first is refused
//! (`Signal::Unsupported`).

use crate::Session;
use crate::single_codec::{CarryVal, Enc, IdUse, Ids};
use al_builtins::describe::Describe;
use al_builtins::value::{Instance, Value};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::Rc;

/// A global of a single instance: (object index, slot).
pub type GlobalKey = (usize, usize);

/// The value of a global, in a form that can go to another thread and be compared.
#[derive(Debug, Clone)]
pub enum Carry {
    /// A value with what it holds (`single_codec.rs`).
    Val(CarryVal),
    /// A value that has no exact form in another session, described with the reason: not carried.
    Opaque(String),
}

/// Two carried values are the same state when both are carried and equal. A value that is not carried never equals
/// anything, not even another one with the same description: a run that read it cannot be shown to have started
/// from the serial state.
impl PartialEq for Carry {
    fn eq(&self, other: &Carry) -> bool {
        match (self, other) {
            (Carry::Val(a), Carry::Val(b)) => a == b,
            _ => false,
        }
    }
}

/// What one test codeunit's session did with the single instances, from the point the session began to track them.
#[derive(Debug, Clone, Default)]
pub struct SingleOut {
    /// The globals it read before writing them, with the value each held then.
    pub observed: BTreeMap<GlobalKey, Carry>,
    /// The end value of each global it used or changed.
    pub wrote: BTreeMap<GlobalKey, Carry>,
    /// Single instances with a manual subscription still bound at the end (`BindSubscription` without
    /// `UnbindSubscription`): whether BC keeps such a binding for the next test codeunit is not pinned.
    pub bound: Vec<usize>,
}

/// How a global was first used in this session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Seen {
    Read,
    Written,
}

/// The notes of a session that tracks its single instances.
#[derive(Default)]
pub(crate) struct Track {
    seen: HashMap<GlobalKey, Seen>,
    observed: BTreeMap<GlobalKey, Carry>,
    /// Globals restored as handles of another session: reading one first is refused.
    poisoned: HashMap<GlobalKey, String>,
    /// Each global's value when the session started to track it (or when the instance was made): a global that was
    /// neither read nor written through the notes but no longer holds it was changed by another way.
    start: HashMap<GlobalKey, Value>,
    /// The session's committed row version when it began to track: a record buffer naming a higher one is not carried
    /// (`single_codec.rs`).
    bound: i64,
    /// Each single instance's globals as one group (`single_codec.rs`), written at the first read of one of them: what
    /// a global read first held at the start, also when a global of the same codeunit sharing a handle with it
    /// changed the handle before.
    group_start: HashMap<usize, Vec<Carry>>,
}

/// Whether `a` and `b` are the same value: equal simple values, or the same handle.
fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Record(x), Value::Record(y)) => Rc::ptr_eq(x, y),
        (Value::List(x), Value::List(y)) => Rc::ptr_eq(x, y),
        (Value::Dictionary(x), Value::Dictionary(y)) => Rc::ptr_eq(x, y),
        (Value::Array(x), Value::Array(y)) => Rc::ptr_eq(x, y),
        (Value::Codeunit(x), Value::Codeunit(y)) => Rc::ptr_eq(x, y),
        (Value::Library(_, x), Value::Library(_, y)) => Rc::ptr_eq(x, y),
        (Value::TestPage(x), Value::TestPage(y)) => Rc::ptr_eq(x, y),
        (Value::Query(x), Value::Query(y)) => Rc::ptr_eq(x, y),
        (Value::DotNet(x), Value::DotNet(y)) => Rc::ptr_eq(x, y),
        _ => {
            let (mut x, mut y) = (Vec::new(), Vec::new());
            al_builtins::value_codec::encode(a, &mut x) && al_builtins::value_codec::encode(b, &mut y) && x == y
        }
    }
}

impl Session {
    /// Starts noting the use of the single instances' globals (see the module). The instances the session holds now
    /// (made by install code or the company open) count with the values they hold now.
    pub fn track_single_instances(&mut self) {
        let mut track = Track {
            bound: self.tx.committed_row_version,
            ..Track::default()
        };
        for (&object, inst) in &self.singletons {
            snapshot(&mut track, object, inst);
        }
        self.si_track = Some(Box::new(track));
    }

    /// Puts back what earlier test codeunits left in the single instances: each global of `state` gets its value; a
    /// handle (`Carry::Opaque`) is refused when the codeunit reads it before writing it. Call after
    /// [`Session::track_single_instances`].
    pub fn restore_single_instances(&mut self, state: &BTreeMap<GlobalKey, Carry>) {
        let mut objects: Vec<usize> = state.keys().map(|k| k.0).collect();
        objects.dedup();
        for object in objects {
            let inst = self.codeunit_instance(object);
            // The globals of one single instance are one group: decoded in order, with one set of numbered handles.
            let mut ids = Ids::new();
            let name = self.engine.program.objects[object].name.clone();
            for (&(o, g), carry) in state.range((object, 0)..=(object, usize::MAX)) {
                debug_assert_eq!(o, object);
                if g >= inst.borrow().globals.len() {
                    continue;
                }
                let bound = self.si_track.as_ref().map_or(0, |t| t.bound);
                let refused = match carry {
                    Carry::Val(c) => match self.carry_decode(c, bound, &mut ids) {
                        Ok(v) => {
                            inst.borrow_mut().globals[g] = v;
                            None
                        }
                        Err(why) => Some(why),
                    },
                    Carry::Opaque(what) => Some(what.clone()),
                };
                if let Some(what) = refused {
                    let var = self.engine.program.objects[object].globals[g].name.clone();
                    if let Some(t) = self.si_track.as_mut() {
                        t.poisoned.insert(
                            (object, g),
                            format!(
                                "variable {var} of SingleInstance codeunit {name}: an earlier test codeunit of the run \
                                 left it holding {what}, which al-runner cannot hand on to the next test codeunit \
                                 exactly"
                            ),
                        );
                    }
                }
                let v = inst.borrow().globals[g].clone();
                if let Some(t) = self.si_track.as_mut() {
                    t.start.insert((object, g), v);
                }
            }
        }
    }

    /// What the session did with the single instances since [`Session::track_single_instances`]; `None` when it did not
    /// track them.
    pub fn single_out(&self) -> Option<SingleOut> {
        let t = self.si_track.as_ref()?;
        let mut out = SingleOut {
            observed: t.observed.clone(),
            ..Default::default()
        };
        let (objects, groups) = self.single_groups();
        let enc = Enc::new(self, &groups, t.bound);
        for (&object, globals) in objects.iter().zip(&groups) {
            // A global of a group changed: the whole group is handed on, written together (its members may share
            // handles, and one changed through another).
            let changed = globals.iter().enumerate().any(|(g, v)| {
                let key = (object, g);
                t.seen.contains_key(&key) || t.start.get(&key).is_none_or(|s| !same(s, v))
            });
            if changed {
                for (g, c) in carry_group(&enc, globals).into_iter().enumerate() {
                    out.wrote.insert((object, g), c);
                }
            }
            if self.events.bindings.iter().any(|b| {
                b.instance
                    .upgrade()
                    .is_some_and(|i| Rc::ptr_eq(&i, &self.singletons[&object]))
            }) {
                out.bound.push(object);
            }
        }
        Some(out)
    }

    /// A read of global `g` of `inst` (noted when `inst` is a single instance and the session tracks them).
    #[inline]
    pub(crate) fn single_read(&mut self, inst: &Rc<RefCell<Instance>>, g: usize) -> Result<(), crate::Signal> {
        if self.si_track.is_none() || !inst.borrow().single {
            return Ok(());
        }
        self.single_read_slow(inst, g)
    }

    #[cold]
    #[inline(never)]
    fn single_read_slow(&mut self, inst: &Rc<RefCell<Instance>>, g: usize) -> Result<(), crate::Signal> {
        let object = inst.borrow().object as usize;
        let Some(t) = self.si_track.as_mut() else { return Ok(()) };
        let key = (object, g);
        if t.seen.contains_key(&key) {
            return Ok(());
        }
        if let Some(why) = t.poisoned.get(&key) {
            return Err(crate::Signal::Unsupported(why.clone()));
        }
        t.seen.insert(key, Seen::Read);
        if !t.group_start.contains_key(&object) {
            let bound = t.bound;
            let (_, groups) = self.single_groups();
            let globals = inst.borrow().globals.clone();
            let mut carried = carry_group(&Enc::new(self, &groups, bound), &globals);
            let Some(t) = self.si_track.as_mut() else { return Ok(()) };
            // A global written before this first read: the handles it shares with the others are no longer those
            // of the start, so what those others held at the start is not known.
            let written = (0..globals.len())
                .any(|j| j != g && t.seen.get(&(object, j)) == Some(&Seen::Written));
            if written {
                for (c, v) in carried.iter_mut().zip(&globals) {
                    if matches!(c, Carry::Val(x) if IdUse::of(x).any()) {
                        *c = Carry::Opaque(format!(
                            "{} that shares a handle with a global the codeunit wrote before reading this one",
                            v.describe()
                        ));
                    }
                }
            }
            t.group_start.insert(object, carried);
        }
        let Some(t) = self.si_track.as_mut() else { return Ok(()) };
        if let Some(c) = t.group_start.get(&object).and_then(|c| c.get(g)).cloned() {
            t.observed.insert(key, c);
        }
        Ok(())
    }

    /// The single instances in object order, each with its globals (one group each).
    fn single_groups(&self) -> (Vec<usize>, Vec<Vec<Value>>) {
        let mut objects: Vec<usize> = self.singletons.keys().copied().collect();
        objects.sort_unstable();
        let groups = objects
            .iter()
            .map(|o| self.singletons[o].borrow().globals.clone())
            .collect();
        (objects, groups)
    }

    /// A write of global `g` of `inst` (see [`Session::single_read`]).
    #[inline]
    pub(crate) fn single_wrote(&mut self, inst: &Rc<RefCell<Instance>>, g: usize) {
        if self.si_track.is_none() || !inst.borrow().single {
            return;
        }
        let object = inst.borrow().object as usize;
        if let Some(t) = self.si_track.as_mut() {
            t.seen.entry((object, g)).or_insert(Seen::Written);
            t.poisoned.remove(&(object, g));
        }
    }

    /// Notes a single instance the session just made (its start values).
    pub(crate) fn single_made(&mut self, object: usize, inst: &Rc<RefCell<Instance>>) {
        inst.borrow_mut().single = true;
        if let Some(t) = self.si_track.as_mut() {
            snapshot(t, object, inst);
        }
    }
}

fn snapshot(t: &mut Track, object: usize, inst: &Rc<RefCell<Instance>>) {
    for (g, v) in inst.borrow().globals.iter().enumerate() {
        t.start.insert((object, g), v.clone());
    }
}

/// The single instances of a finished session, moved (not copied) to the next session on the same thread: the
/// serial check (ADR-066 amendment 1), which runs the codeunits of a BC session one after the other and hands the very
/// same objects on, as BC's one session keeps them. Holds `Rc`s: it never leaves its thread.
pub struct MovedSingles {
    singletons: std::collections::HashMap<usize, Rc<RefCell<Instance>>>,
    temps: std::collections::HashMap<u64, al_data::memory::MemStore>,
    next_temp: u64,
    temp_release: Rc<RefCell<Vec<u64>>>,
}

impl Session {
    /// Moves the single instances and the temporary tables out of this finished session (see [`MovedSingles`]).
    pub fn take_single_instances(&mut self) -> MovedSingles {
        MovedSingles {
            singletons: std::mem::take(&mut self.singletons),
            temps: std::mem::take(&mut self.temps),
            next_temp: self.next_temp,
            temp_release: self.temp_release.clone(),
        }
    }

    /// Call on a new session before it makes any temporary record: its temporary table ids continue those of the
    /// session `moved` came from, so the moved records' tables keep their ids.
    pub fn continue_temp_ids(&mut self, moved: &MovedSingles) {
        self.next_temp = self.next_temp.max(moved.next_temp);
        self.temp_release = moved.temp_release.clone();
    }

    /// Takes over the single instances and temporary tables of the session before (see [`MovedSingles`]).
    pub fn adopt_single_instances(&mut self, moved: MovedSingles) {
        self.next_temp = self.next_temp.max(moved.next_temp);
        self.temps.extend(moved.temps);
        self.singletons.extend(moved.singletons);
    }
}

/// The globals of one single instance, written as one group (`single_codec.rs`). A global that names a handle whose
/// first place could not be written is not carried either.
fn carry_group(enc: &Enc, globals: &[Value]) -> Vec<Carry> {
    enc.begin_group();
    let out: Vec<Result<CarryVal, String>> = globals.iter().map(|v| enc.enc(v)).collect();
    let mut defs = HashSet::new();
    for c in out.iter().flatten() {
        defs.extend(IdUse::of(c).defs);
    }
    out.into_iter()
        .zip(globals)
        .map(|(r, v)| match r {
            Ok(c) if IdUse::of(&c).refs.is_subset(&defs) => Carry::Val(c),
            Ok(_) => Carry::Opaque(format!(
                "{} that shares a handle with another global of the codeunit that al-runner cannot hand on",
                v.describe()
            )),
            Err(what) => Carry::Opaque(what),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_value_that_is_not_carried_equals_nothing() {
        let o = Carry::Opaque("temporary Record of table object 1".into());
        assert_ne!(o, o.clone());
        let v = Carry::Val(CarryVal::Plain(vec![0]));
        assert_eq!(v, v.clone());
        assert_ne!(v, o);
    }
}
