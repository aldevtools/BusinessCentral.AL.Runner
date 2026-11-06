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
//! Carried are simple values ([`al_builtins::value_codec`]), codeunit values, .NET enum values and the .NET objects the
//! runtime serves itself, each with what it holds: a global that holds another handle (a record, a list, a .NET object
//! of the session's .NET process) is carried as [`Carry::Opaque`], and a later codeunit that reads it first is refused
//! (`Signal::Unsupported`), as the handle belongs to another session.

use crate::Session;
use al_builtins::describe::Describe;
use al_builtins::value::{DotNetKind, DotNetObject, Instance, Value};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

/// A global of a single instance: (object index, slot).
pub type GlobalKey = (usize, usize);

/// The value of a global, in a form that can go to another thread and be compared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Carry {
    /// A simple value or a codeunit value, as [`carry_encode`] writes it.
    Plain(Vec<u8>),
    /// A handle, described (`Value::describe`): not carried.
    Opaque(String),
}

impl Carry {
    pub fn of(v: &Value) -> Carry {
        let mut out = Vec::new();
        match carry_encode(v, &mut out, 0) {
            Ok(()) => Carry::Plain(out),
            Err(what) => Carry::Opaque(what),
        }
    }
}

/// How deep codeunit values inside codeunit values are carried.
const DEPTH: usize = 8;

/// Writes `v`: a simple value (tag 0, then [`al_builtins::value_codec`]), or a codeunit value (tag 1: object, whether
/// its globals were set up, then each global the same way). A codeunit value carries its instance's globals: a
/// SingleInstance codeunit's facade often holds its implementation codeunit (System Application "Server Setting"
/// holds "Server Setting Impl."). Anything else is described: it is not carried.
fn carry_encode(v: &Value, out: &mut Vec<u8>, depth: usize) -> Result<(), String> {
    use al_builtins::value_codec::{put_i64, put_str, put_u8, put_u32};
    if let Value::Codeunit(inst) = v {
        if depth >= DEPTH {
            return Err(v.describe());
        }
        let inst = inst.try_borrow().map_err(|_| v.describe())?;
        put_u8(out, 1);
        put_u32(out, inst.object);
        put_u8(out, u8::from(inst.initialized));
        put_u32(out, inst.globals.len() as u32);
        for g in &inst.globals {
            carry_encode(g, out, depth + 1)?;
        }
        return Ok(());
    }
    // A .NET object the runtime serves itself (`dotnet/registry.rs`, such as `ALConfigSettings.Instance()`) or a .NET
    // enum value lives in the runtime, not in a session's .NET process: its state is carried. A host object is not.
    if let Value::DotNet(obj) = v {
        match &obj.kind {
            DotNetKind::Enum {
                clr_type,
                name,
                ordinal,
            } => {
                put_u8(out, 2);
                put_str(out, clr_type);
                put_str(out, name);
                put_i64(out, *ordinal);
                return Ok(());
            }
            DotNetKind::Internal { type_name, state } if depth < DEPTH => {
                let state = state.try_borrow().map_err(|_| v.describe())?;
                put_u8(out, 3);
                put_str(out, type_name);
                put_u32(out, state.len() as u32);
                for x in state.iter() {
                    carry_encode(x, out, depth + 1)?;
                }
                return Ok(());
            }
            _ => return Err(v.describe()),
        }
    }
    put_u8(out, 0);
    if al_builtins::value_codec::encode(v, out) {
        Ok(())
    } else {
        Err(v.describe())
    }
}

/// Reads what [`carry_encode`] wrote.
fn carry_decode(r: &mut al_builtins::value_codec::Reader) -> Option<Value> {
    match r.u8()? {
        0 => al_builtins::value_codec::decode(r),
        1 => {
            let object = r.u32()?;
            let initialized = r.u8()? != 0;
            let n = r.u32()? as usize;
            let mut globals = Vec::with_capacity(n.min(1024));
            for _ in 0..n {
                globals.push(carry_decode(r)?);
            }
            Some(Value::Codeunit(Rc::new(RefCell::new(Instance {
                object,
                globals,
                initialized,
                ..Default::default()
            }))))
        }
        2 => {
            let clr_type = r.str()?;
            let name = r.str()?;
            let ordinal = r.i64()?;
            Some(Value::DotNet(Rc::new(DotNetObject::new(DotNetKind::Enum {
                clr_type,
                name,
                ordinal,
            }))))
        }
        3 => {
            let type_name = r.str()?;
            let n = r.u32()? as usize;
            let mut state = Vec::with_capacity(n.min(1024));
            for _ in 0..n {
                state.push(carry_decode(r)?);
            }
            Some(Value::DotNet(Rc::new(DotNetObject::new(DotNetKind::Internal {
                type_name,
                state: RefCell::new(state),
            }))))
        }
        _ => None,
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
        _ => match (Carry::of(a), Carry::of(b)) {
            (Carry::Plain(x), Carry::Plain(y)) => x == y,
            _ => false,
        },
    }
}

impl Session {
    /// Starts noting the use of the single instances' globals (see the module). The instances the session holds now
    /// (made by install code or the company open) count with the values they hold now.
    pub fn track_single_instances(&mut self) {
        let mut track = Track::default();
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
            let name = self.engine.program.objects[object].name.clone();
            for (&(o, g), carry) in state.range((object, 0)..=(object, usize::MAX)) {
                debug_assert_eq!(o, object);
                if g >= inst.borrow().globals.len() {
                    continue;
                }
                match carry {
                    Carry::Plain(bytes) => {
                        let mut r = al_builtins::value_codec::Reader::new(bytes);
                        if let Some(v) = carry_decode(&mut r) {
                            inst.borrow_mut().globals[g] = v;
                        }
                    }
                    Carry::Opaque(what) => {
                        let var = self.engine.program.objects[object].globals[g].name.clone();
                        if let Some(t) = self.si_track.as_mut() {
                            t.poisoned.insert(
                                (object, g),
                                format!(
                                    "variable {var} of SingleInstance codeunit {name}: an earlier test codeunit of the run \
                                     left it holding {what}, and al-runner carries only simple values from one test \
                                     codeunit to the next"
                                ),
                            );
                        }
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
        let mut objects: Vec<&usize> = self.singletons.keys().collect();
        objects.sort_unstable();
        for &object in objects {
            let inst = self.singletons[&object].borrow();
            for (g, v) in inst.globals.iter().enumerate() {
                let key = (object, g);
                let changed = match t.seen.get(&key) {
                    Some(_) => true,
                    None => t.start.get(&key).is_none_or(|s| !same(s, v)),
                };
                if changed {
                    out.wrote.insert(key, Carry::of(v));
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
        if let Some(v) = inst.borrow().globals.get(g) {
            t.observed.insert(key, Carry::of(v));
        }
        Ok(())
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
