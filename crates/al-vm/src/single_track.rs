//! Which SingleInstance globals a test codeunit read before it wrote them, and what it left in them
//! (ADR-066 amendment 2, al-runner issue 6924).
//!
//! BC runs all test codeunits of a run in one session, so the globals of a `SingleInstance = true` codeunit that
//! test codeunit A left are still there in the next test codeunit B (al-language/codeunits/TestCuP7BReader.al:24-37,
//! against TestCuP7ASetter.al:24-58). al-runner runs each test codeunit on a session of its own. It does not copy
//! state from session to session. It finds the codeunits that depend on such state and runs only those, with the
//! codeunits that wrote it, in one session in BC's order (`al-testrun` `single_lane.rs`).
//!
//! This module is the notes a tracking session takes. A session tracks only the SingleInstance codeunits it is told to
//! watch (`al-testrun` works them out from the static reach of the test codeunits); a session that is told none
//! pays one `Option` test per global access. Per watched global the notes keep:
//!
//! * the fingerprint of its value at its first use when that was a read (what the codeunit saw of the state it started
//!   from), and
//! * at the end, its fingerprint when the codeunit used it or changed it.
//!
//! A fingerprint is a byte string of the value ([`Fp::Val`]) or, for a value that has no comparable form (a .NET
//! object, a page, a stream, a temporary record), [`Fp::Opaque`], which never equals anything.

use crate::Session;
use al_builtins::value::{Instance, Value};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;

/// A global of a single instance: (object index, slot).
pub type GlobalKey = (usize, usize);

/// A value in a form that can go to another thread and be compared.
#[derive(Debug, Clone)]
pub enum Fp {
    Val(Arc<[u8]>),
    /// No comparable form, with what the value is: never equal to any value, not even to itself ([`Fp::same_kind`]).
    Opaque(Arc<str>),
}

impl Fp {
    /// Both are values without a comparable form of the same kind (a cache that holds an object of the .NET process
    /// that the codeunit's own fill makes again). Unsound for an object whose content differs; only the rule for a lazy
    /// cache the codeunit filled itself uses it.
    pub fn same_kind(&self, other: &Fp) -> bool {
        matches!((self, other), (Fp::Opaque(a), Fp::Opaque(b)) if a == b)
    }
}

impl PartialEq for Fp {
    fn eq(&self, other: &Fp) -> bool {
        matches!((self, other), (Fp::Val(a), Fp::Val(b)) if a == b)
    }
}

/// What one test codeunit's session did with the watched single instances.
#[derive(Debug, Clone, Default)]
pub struct SingleOut {
    /// The globals it read before writing them, with the fingerprint each held then.
    pub observed: BTreeMap<GlobalKey, Fp>,
    /// The end fingerprint of each global it used or changed.
    pub ends: BTreeMap<GlobalKey, Fp>,
    /// The globals it left other than it was given: written first, or read first and ended unlike it started.
    pub wrote: BTreeSet<GlobalKey>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Seen {
    Read,
    Written,
}

/// The notes of a session that tracks single instances.
pub(crate) struct Track {
    seen: HashMap<GlobalKey, Seen>,
    observed: BTreeMap<GlobalKey, Fp>,
    /// The values a watched instance's globals held when the session made it: a global that was never read or written
    /// through the notes but holds another value at the end was changed some other way.
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
        (Value::Null, Value::Null) => true,
        (Value::Boolean(x), Value::Boolean(y)) => x == y,
        (Value::Integer(x), Value::Integer(y)) => x == y,
        (Value::BigInteger(x), Value::BigInteger(y)) => x == y,
        (Value::Text(x), Value::Text(y)) | (Value::Code(x), Value::Code(y)) => x == y,
        _ => {
            let (mut x, mut y) = (Vec::new(), Vec::new());
            al_builtins::value_codec::encode(a, &mut x) && al_builtins::value_codec::encode(b, &mut y) && x == y
        }
    }
}

/// Whether `v` is a plain value: no handle, so equal at two times means unchanged.
fn plain(v: &Value) -> bool {
    matches!(
        v,
        Value::Null
            | Value::Boolean(_)
            | Value::Integer(_)
            | Value::BigInteger(_)
            | Value::Text(_)
            | Value::Code(_)
            | Value::Date(..)
            | Value::Time(_)
            | Value::DateTime(_)
            | Value::Duration(_)
    )
}

/// Appends the fingerprint of `v`; false when it has none.
fn put(v: &Value, depth: u8, out: &mut Vec<u8>, e: &crate::Engine) -> bool {
    use al_builtins::value_codec::{put_u8, put_u32};
    if depth > 4 {
        return false;
    }
    match v {
        Value::List(l) => {
            let Ok(l) = l.try_borrow() else { return false };
            put_u8(out, 200);
            put_u32(out, l.len() as u32);
            l.iter().all(|x| put(x, depth + 1, out, e))
        }
        Value::Array(a) => {
            let Ok(a) = a.try_borrow() else { return false };
            put_u8(out, 201);
            put_u32(out, a.len() as u32);
            a.iter().all(|x| put(x, depth + 1, out, e))
        }
        Value::Dictionary(d) => {
            let Ok(d) = d.try_borrow() else { return false };
            put_u8(out, 202);
            put_u32(out, d.len() as u32);
            d.iter()
                .all(|(k, x)| put(k, depth + 1, out, e) && put(x, depth + 1, out, e))
        }
        Value::Record(r) => {
            let Ok(r) = r.try_borrow() else { return false };
            if r.temporary || r.temp_id.is_some() || r.crm {
                return false;
            }
            put_u8(out, 203);
            put_u32(out, r.table);
            put_u32(out, r.fields.len() as u32);
            // The system fields that differ from session to session (SystemId .. SystemRowVersion: the run clock and the
            // row version counter) are not part of what a cached row says.
            let def = e.tables.get(r.table as usize).and_then(|t| t.as_ref());
            for (pos, x) in r.fields.iter().enumerate() {
                let id = def.and_then(|d| d.fields.get(pos)).map_or(0, |f| f.id);
                if (2_000_000_000..=2_000_000_008).contains(&id) {
                    continue;
                }
                if !put(x, depth + 1, out, e) {
                    return false;
                }
            }
            // Filters and marks as counts: a cached row is read as it was got.
            put_u32(out, r.filters.len() as u32);
            put_u32(out, r.marks.len() as u32);
            put_u8(out, r.marked_only as u8);
            out.extend_from_slice(r.company.as_deref().unwrap_or("").as_bytes());
            true
        }
        Value::Codeunit(c) => {
            let Ok(c) = c.try_borrow() else { return false };
            put_u8(out, 204);
            put_u32(out, c.object);
            put_u32(out, c.globals.len() as u32);
            c.globals.iter().all(|x| put(x, depth + 1, out, e))
        }
        // An object the runtime serves itself (the server configuration a lazy cache holds) and an enum value have a form;
        // an object of the .NET process is a handle of this session.
        Value::DotNet(d) => match &d.kind {
            al_builtins::value::DotNetKind::Enum {
                clr_type,
                name,
                ordinal,
            } => {
                put_u8(out, 205);
                out.extend_from_slice(format!("{clr_type}|{name}|{ordinal}").as_bytes());
                true
            }
            al_builtins::value::DotNetKind::Internal { type_name, state } => {
                let Ok(state) = state.try_borrow() else { return false };
                put_u8(out, 206);
                out.extend_from_slice(type_name.as_bytes());
                put_u32(out, state.len() as u32);
                state.iter().all(|x| put(x, depth + 1, out, e))
            }
            // The server configuration is the same for every session of a run and nothing in AL changes it.
            al_builtins::value::DotNetKind::Host { clr_type, .. }
                if clr_type == al_builtins::dotnet_sigs::AL_CONFIG_SETTINGS =>
            {
                put_u8(out, 207);
                out.extend_from_slice(clr_type.as_bytes());
                true
            }
            al_builtins::value::DotNetKind::Host { .. } => false,
        },
        _ => al_builtins::value_codec::encode(v, out),
    }
}

fn fp(v: &Value, e: &crate::Engine) -> Fp {
    let mut out = Vec::new();
    if put(v, 0, &mut out, e) {
        Fp::Val(out.into())
    } else {
        Fp::Opaque(al_builtins::describe::Describe::describe(v).into())
    }
}

impl Session {
    /// Starts noting the use of the globals of the single instances of `watch` (see the module). Call before the test
    /// codeunit's code runs; instances the session made already (the company open) count with the values they hold.
    pub fn track_single_instances(&mut self) {
        let mut t = Track {
            seen: HashMap::new(),
            observed: BTreeMap::new(),
            start: HashMap::new(),
        };
        let made: Vec<(usize, Rc<RefCell<Instance>>)> = self.singletons.iter().map(|(o, i)| (*o, i.clone())).collect();
        self.si_track = Some(Box::new(t_snapshot_all(&mut t, &made)));
    }

    /// A single instance the session just made: watched when its object is.
    pub(crate) fn single_made(&mut self, object: usize, inst: &Rc<RefCell<Instance>>) {
        let Some(t) = self.si_track.as_mut() else { return };
        if !inst.borrow().globals.is_empty() {
            inst.borrow_mut().single = true;
            snapshot(t, object, inst);
        }
    }

    /// A read of global `g` of `inst`.
    #[inline]
    pub(crate) fn single_read(&mut self, inst: &Rc<RefCell<Instance>>, g: usize) {
        if self.si_track.is_none() || !inst.borrow().single {
            return;
        }
        self.single_read_slow(inst, g);
    }

    #[cold]
    #[inline(never)]
    fn single_read_slow(&mut self, inst: &Rc<RefCell<Instance>>, g: usize) {
        let object = inst.borrow().object as usize;
        let Some(t) = self.si_track.as_mut() else { return };
        let key = (object, g);
        if t.seen.contains_key(&key) {
            return;
        }
        t.seen.insert(key, Seen::Read);
        let f = match inst.try_borrow() {
            Ok(i) => i.globals.get(g).map_or(Fp::Opaque("?".into()), |v| fp(v, &self.engine)),
            Err(_) => Fp::Opaque("borrowed".into()),
        };
        t.observed.insert(key, f);
    }

    /// A write of global `g` of `inst`.
    #[inline]
    pub(crate) fn single_wrote(&mut self, inst: &Rc<RefCell<Instance>>, g: usize) {
        if self.si_track.is_none() || !inst.borrow().single {
            return;
        }
        let object = inst.borrow().object as usize;
        if let Some(t) = self.si_track.as_mut() {
            t.seen.entry((object, g)).or_insert(Seen::Written);
        }
    }

    /// What the session did with the watched single instances; `None` when it tracked none.
    pub fn single_out(&self) -> Option<SingleOut> {
        let t = self.si_track.as_ref()?;
        let mut out = SingleOut {
            observed: t.observed.clone(),
            ..Default::default()
        };
        for (&object, inst) in &self.singletons {
            if !inst.borrow().single {
                continue;
            }
            let Ok(inst) = inst.try_borrow() else { continue };
            for (g, v) in inst.globals.iter().enumerate() {
                let key = (object, g);
                match t.seen.get(&key) {
                    Some(Seen::Written) => {
                        out.ends.insert(key, fp(v, &self.engine));
                        out.wrote.insert(key);
                    }
                    // A plain value that is what it was at the start needs no second fingerprint.
                    Some(Seen::Read) if plain(v) && t.start.get(&key).is_some_and(|s| same(s, v)) => {
                        if let Some(f) = t.observed.get(&key) {
                            out.ends.insert(key, f.clone());
                        }
                    }
                    Some(Seen::Read) => {
                        let end = fp(v, &self.engine);
                        if t.observed.get(&key) != Some(&end) {
                            out.wrote.insert(key);
                        }
                        out.ends.insert(key, end);
                    }
                    None => {
                        if t.start.get(&key).is_some_and(|s| !same(s, v)) {
                            out.ends.insert(key, fp(v, &self.engine));
                            out.wrote.insert(key);
                        }
                    }
                }
            }
        }
        Some(out)
    }
}

fn t_snapshot_all(t: &mut Track, made: &[(usize, Rc<RefCell<Instance>>)]) -> Track {
    for (o, inst) in made {
        if !inst.borrow().globals.is_empty() {
            inst.borrow_mut().single = true;
            snapshot(t, *o, inst);
        }
    }
    Track {
        seen: std::mem::take(&mut t.seen),
        observed: std::mem::take(&mut t.observed),
        start: std::mem::take(&mut t.start),
    }
}

fn snapshot(t: &mut Track, object: usize, inst: &Rc<RefCell<Instance>>) {
    for (g, v) in inst.borrow().globals.iter().enumerate() {
        t.start.insert((object, g), v.clone());
    }
}

/// The single instances of a finished session, moved (not copied) to the next session on the same thread: the
/// serial lane runs the codeunits of a BC session one after the other and hands the very same objects on, as BC's one
/// session keeps them. Holds `Rc`s: it never leaves its thread.
pub struct MovedSingles {
    singletons: HashMap<usize, Rc<RefCell<Instance>>>,
    temps: HashMap<u64, al_data::memory::MemStore>,
    next_temp: u64,
    temp_release: Rc<RefCell<Vec<u64>>>,
    /// The manual bindings of single instances (and of codeunits they hold), oldest first.
    bindings: Vec<crate::dispatch::Binding>,
    /// The single instances' `Execute` check state, see [`Session::adopt_single_instances`].
    unclaimed: HashSet<usize>,
}

impl Session {
    /// Moves the single instances and the temporary tables out of this finished session (see [`MovedSingles`]).
    pub fn take_single_instances(&mut self) -> MovedSingles {
        // A binding BC keeps for the next test codeunit (codeunits/TestEventSingleInstanceBindingBoundary.al:98-99):
        // that of a single instance, and that of an ordinary instance a single instance's global holds.
        let mut held: HashSet<usize> = HashSet::new();
        for inst in self.singletons.values() {
            held.insert(Rc::as_ptr(inst) as *const () as usize);
            if let Ok(i) = inst.try_borrow() {
                for v in &i.globals {
                    if let Value::Codeunit(c) = v {
                        held.insert(Rc::as_ptr(c) as *const () as usize);
                    }
                }
            }
        }
        let bindings: Vec<crate::dispatch::Binding> = self
            .events
            .bindings
            .iter()
            .filter(|b| {
                b.instance
                    .upgrade()
                    .is_some_and(|i| held.contains(&(Rc::as_ptr(&i) as *const () as usize)))
            })
            .map(|b| crate::dispatch::Binding {
                instance: b.instance.clone(),
            })
            .collect();
        MovedSingles {
            bindings,
            singletons: std::mem::take(&mut self.singletons),
            temps: std::mem::take(&mut self.temps),
            next_temp: self.next_temp,
            temp_release: self.temp_release.clone(),
            unclaimed: HashSet::new(),
        }
    }

    /// Call on a new session before it makes any temporary record: its temporary table ids continue those of the
    /// session `moved` came from, so the moved records' tables keep their ids.
    pub fn continue_temp_ids(&mut self, moved: &MovedSingles) {
        self.next_temp = self.next_temp.max(moved.next_temp);
        self.temp_release = moved.temp_release.clone();
    }

    /// Takes over the single instances and temporary tables of the session before (see [`MovedSingles`]). The first
    /// use of a moved single instance counts as its making for the Execute check (`Instance::born`), as in a session
    /// that starts without it: BC checks Execute on Format Address (SingleInstance) called through a local variable
    /// under a narrow set (permissions/wildcard-scope/Y93WildScope.al:439, :446-447) although an earlier test codeunit
    /// of the same session made it. Assumed for the other shapes (docs/probes-needed.md).
    pub fn adopt_single_instances(&mut self, moved: MovedSingles) {
        self.next_temp = self.next_temp.max(moved.next_temp);
        self.temps.extend(moved.temps);
        let _ = moved.unclaimed;
        self.singles_unclaimed.extend(moved.singletons.keys().copied());
        self.singletons.extend(moved.singletons);
        self.events.bindings.splice(0..0, moved.bindings);
        self.sync_active_instances();
    }

    /// The first use in this session of a single instance an earlier test codeunit left.
    pub(crate) fn claim_carried_single(&mut self, object: usize) {
        if self.singles_unclaimed.remove(&object)
            && let Some(inst) = self.singletons.get(&object)
        {
            inst.borrow_mut().born = al_builtins::value::Born::default();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_opaque_fingerprint_equals_nothing() {
        let o = Fp::Opaque("a DotNet object".into());
        assert_ne!(o, o.clone());
        assert!(o.same_kind(&o.clone()));
        let v = Fp::Val(vec![1u8].into());
        assert_eq!(v, v.clone());
        assert_ne!(v, o);
    }
}
