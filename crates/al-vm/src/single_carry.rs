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
use crate::single_codec::{BindingRanks, reaches_any, CarryVal, Enc, IdUse, Ids, binding_ranks, bound_in};
use al_builtins::describe::Describe;
use al_builtins::value::{Instance, Value};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::Rc;

/// A global of a single instance: (object index, slot).
pub type GlobalKey = (usize, usize);

/// Not a global: the manual bindings that reach the next test codeunit (BC keeps them,
/// TestEventSingleInstanceBindingBoundary.al:98-99). Its value names the bound single instances and the globals that
/// hold a bound instance, each with its place among the bindings; an event with a manual subscriber reads it. A bound
/// instance held where the codec cannot write it makes it a value that is not carried (refused where it is read).
pub const BINDINGS_KEY: GlobalKey = (usize::MAX, 0);

/// The value of a global, in a form that can go to another thread and be compared.
#[derive(Debug, Clone)]
pub enum Carry {
    /// A value with what it holds (`single_codec.rs`).
    Val(std::sync::Arc<CarryVal>),
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

/// The value of [`BINDINGS_KEY`] for bindings `b`, the globals' carried values in `held`.
fn bindings_carry(b: &BindingRanks, held: &[(GlobalKey, &Carry)]) -> Carry {
    let mut lines: Vec<(u32, String)> = b.singles.iter().map(|&(o, r)| (r, format!("s {o} {r}"))).collect();
    let mut found = HashSet::new();
    for &((o, g), c) in held {
        if let Carry::Val(v) = c {
            let mut ranks = Vec::new();
            bound_in(v, &mut ranks);
            for r in ranks {
                if found.insert(r) {
                    lines.push((r, format!("h {o} {g} {r}")));
                }
            }
        }
    }
    if b.deep {
        return Carry::Opaque("a manual binding of a codeunit instance held deeper than al-runner hands on".into());
    }
    if let Some(r) = b.ranks.values().find(|r| !found.contains(r)) {
        let _ = r;
        let what = held
            .iter()
            .find_map(|(_, c)| match c {
                Carry::Opaque(why) => Some(why.as_str()),
                Carry::Val(_) => None,
            })
            .unwrap_or("a value al-runner cannot hand on");
        return Carry::Opaque(format!(
            "a manual binding of a codeunit instance that a SingleInstance codeunit holds in {what}"
        ));
    }
    lines.sort();
    Carry::Val(std::sync::Arc::new(CarryVal::Plain(
        lines
            .into_iter()
            .map(|(_, l)| l)
            .collect::<Vec<_>>()
            .join("\n")
            .into_bytes(),
    )))
}

/// What one test codeunit's session did with the single instances, from the point the session began to track them.
#[derive(Debug, Clone, Default)]
pub struct SingleOut {
    /// The globals it read before writing them, with the value each held then.
    pub observed: BTreeMap<GlobalKey, Carry>,
    /// The end value of each global it used or changed, and of [`BINDINGS_KEY`] when the bindings changed.
    pub wrote: BTreeMap<GlobalKey, Carry>,
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
    /// The value of [`BINDINGS_KEY`] when the session began to track (after the restore).
    bind_start: Option<Carry>,
    /// A `BindSubscription` or `UnbindSubscription` ran: what the globals hold may be bound otherwise now.
    bindings_changed: bool,
    /// The restored globals that hold a bound instance: an event with a manual subscriber reads them.
    held: Vec<GlobalKey>,
    /// Bindings of the restore, bound once it is done: (place, instance).
    pending: Vec<(u32, Rc<RefCell<Instance>>)>,
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
        // The simple values most globals hold: compared as they are, without writing both out.
        (Value::Null, Value::Null) => true,
        (Value::Boolean(x), Value::Boolean(y)) => x == y,
        (Value::Integer(x), Value::Integer(y)) => x == y,
        (Value::BigInteger(x), Value::BigInteger(y)) => x == y,
        (Value::Byte(x), Value::Byte(y)) => x == y,
        (Value::Char(x), Value::Char(y)) => x == y,
        (Value::Date(x, c), Value::Date(y, d)) => x == y && c == d,
        (Value::Time(x), Value::Time(y)) | (Value::DateTime(x), Value::DateTime(y)) | (Value::Duration(x), Value::Duration(y)) => x == y,
        (Value::Enum(x, m), Value::Enum(y, n)) => x == y && m == n,
        (Value::Text(x), Value::Text(y)) | (Value::Code(x), Value::Code(y)) => x == y,
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
        let b = self.binding_carry();
        if let Some(t) = self.si_track.as_mut() {
            t.bind_start = Some(b);
        }
    }

    /// The value of [`BINDINGS_KEY`] now.
    fn binding_carry(&self) -> Carry {
        self.binding_carry_with(binding_ranks(self))
    }

    /// [`Session::binding_carry`] for the bindings `b` ([`binding_ranks`] walks the single instances' globals, so it is
    /// made once per question).
    fn binding_carry_with(&self, b: BindingRanks) -> Carry {
        if b.singles.is_empty() && b.ranks.is_empty() && !b.deep {
            return Carry::Val(std::sync::Arc::new(CarryVal::Plain(Vec::new())));
        }
        // Only an ordinary instance a single instance's global holds has a place among the bindings in the encoding
        // (`Enc::bind_ranks`), so the globals' values name no bound instance when there is none: nothing to encode.
        if b.ranks.is_empty() {
            return bindings_carry(&b, &[]);
        }
        let bound = self.si_track.as_ref().map_or(0, |t| t.bound);
        let (objects, groups) = self.single_groups();
        let enc = Enc::new(self, &groups, bound, b.ranks.clone());
        let carry_of = |only_holders: bool| -> Carry {
            let carried: Vec<(GlobalKey, Carry)> = objects
                .iter()
                .zip(&groups)
                .filter(|(_, globals)| !only_holders || reaches_any(globals, &b.ranks))
                .flat_map(|(&o, globals)| {
                    carry_group(&enc, globals)
                        .into_iter()
                        .enumerate()
                        .map(move |(g, c)| ((o, g), c))
                })
                .collect();
            let held: Vec<(GlobalKey, &Carry)> = carried.iter().map(|(k, c)| (*k, c)).collect();
            bindings_carry(&b, &held)
        };
        // Only the globals that hold a bound instance name one. When that does not give the whole answer (a handle
        // could not be carried), the reason text is made from all the groups, as before.
        match carry_of(true) {
            Carry::Val(v) => Carry::Val(v),
            Carry::Opaque(_) => carry_of(false),
        }
    }

    /// A carried instance bound at the end of the codeunit before, at place `rank` (`single_codec.rs`).
    pub(crate) fn carried_binding(&mut self, rank: u32, inst: &Rc<RefCell<Instance>>) {
        if let Some(t) = self.si_track.as_mut() {
            t.pending.push((rank, inst.clone()));
        }
    }

    /// Binds what the restore carried, in the order of the session before and before any binding of this one.
    fn bind_carried(&mut self) {
        let Some(t) = self.si_track.as_mut() else { return };
        let mut pending = std::mem::take(&mut t.pending);
        if pending.is_empty() {
            return;
        }
        pending.sort_by_key(|(r, _)| *r);
        pending.dedup_by(|a, b| Rc::ptr_eq(&a.1, &b.1));
        let carried: Vec<crate::dispatch::Binding> = pending
            .iter()
            .map(|(_, i)| crate::dispatch::Binding {
                instance: Rc::downgrade(i),
            })
            .collect();
        self.events.bindings.splice(0..0, carried);
        self.sync_active_instances();
    }

    /// A `BindSubscription` or `UnbindSubscription` changed the bindings.
    pub(crate) fn single_bindings_changed(&mut self) {
        if let Some(t) = self.si_track.as_mut() {
            t.bindings_changed = true;
        }
    }

    /// An event picks its manual subscribers: it reads which carried bindings there are ([`BINDINGS_KEY`]) and the
    /// globals that hold a carried bound instance.
    pub(crate) fn single_bindings_read(&mut self) -> Result<(), crate::Signal> {
        let Some(t) = self.si_track.as_mut() else { return Ok(()) };
        if t.seen.contains_key(&BINDINGS_KEY) {
            return Ok(());
        }
        if let Some(why) = t.poisoned.get(&BINDINGS_KEY) {
            return Err(crate::Signal::Unsupported(why.clone()));
        }
        t.seen.insert(BINDINGS_KEY, Seen::Read);
        if let Some(c) = t.bind_start.clone() {
            t.observed.insert(BINDINGS_KEY, c);
        }
        for (o, g) in t.held.clone() {
            if let Some(inst) = self.singletons.get(&o).cloned() {
                self.single_read(&inst, g)?;
            }
        }
        Ok(())
    }

    /// Puts back what earlier test codeunits left in the single instances: each global of `state` gets its value; a
    /// handle (`Carry::Opaque`) is refused when the codeunit reads it before writing it. Call after
    /// [`Session::track_single_instances`].
    pub fn restore_single_instances(&mut self, state: std::sync::Arc<BTreeMap<GlobalKey, Carry>>) {
        // Without carried bindings an instance is put back when the session first makes it (`restore_lazy`): a
        // codeunit that uses few single instances reads back few of the many the run carries.
        let no_bindings = match state.get(&BINDINGS_KEY) {
            None => true,
            Some(Carry::Val(v)) => matches!(&**v, CarryVal::Plain(text) if text.is_empty()),
            Some(Carry::Opaque(_)) => false,
        };
        if no_bindings {
            self.lazy_single = Some(state);
            let b = self.binding_carry();
            if let Some(t) = self.si_track.as_mut() {
                t.bind_start = Some(b);
            }
            return;
        }
        self.restore_all(&state);
    }

    /// Puts the carried globals of single instance `object` back when the session makes it (`shared_instance`).
    pub(crate) fn restore_lazy(&mut self, object: usize, inst: &Rc<RefCell<Instance>>) {
        let Some(state) = self.lazy_single.clone() else { return };
        if state.range((object, 0)..=(object, usize::MAX)).next().is_some() {
            self.restore_object(object, inst, &state);
        }
    }

    fn restore_all(&mut self, state: &BTreeMap<GlobalKey, Carry>) {
        let mut objects: Vec<usize> = state.keys().map(|k| k.0).filter(|&o| o != BINDINGS_KEY.0).collect();
        objects.dedup();
        match state.get(&BINDINGS_KEY) {
            Some(Carry::Val(v)) if matches!(&**v, CarryVal::Plain(_)) => {
                let CarryVal::Plain(text) = &**v else { unreachable!() };
                for line in String::from_utf8_lossy(text).lines() {
                    let f: Vec<usize> = line.split(' ').skip(1).filter_map(|x| x.parse().ok()).collect();
                    if line.starts_with("s ")
                        && let [object, rank] = f[..]
                    {
                        let inst = self.codeunit_instance(object);
                        self.ensure_initialized(&inst);
                        self.carried_binding(rank as u32, &inst);
                    }
                }
            }
            Some(c) => {
                let why = match c {
                    Carry::Opaque(why) => why.clone(),
                    Carry::Val(_) => "a value that does not read back".into(),
                };
                if let Some(t) = self.si_track.as_mut() {
                    t.poisoned.insert(
                        BINDINGS_KEY,
                        format!(
                            "an event with a manual subscriber: an earlier test codeunit of the run left {why} bound, \
                             which al-runner cannot hand on to the next test codeunit exactly"
                        ),
                    );
                }
            }
            None => {}
        }
        for object in objects {
            let inst = self.codeunit_instance(object);
            self.restore_object(object, &inst, state);
        }
        self.bind_carried();
        let b = self.binding_carry();
        if let Some(t) = self.si_track.as_mut() {
            t.bind_start = Some(b);
        }
        self.singles_unclaimed.extend(self.singletons.keys().copied());
    }

    /// Puts the globals of `state` that belong to single instance `object` (held as `inst`) back.
    fn restore_object(&mut self, object: usize, inst: &Rc<RefCell<Instance>>, state: &BTreeMap<GlobalKey, Carry>) {
        // The globals of one single instance are one group: decoded in order, with one set of numbered handles.
        let mut ids = Ids::new();
        let name = self.engine.program.objects[object].name.clone();
        for (&(o, g), carry) in state.range((object, 0)..=(object, usize::MAX)) {
            debug_assert_eq!(o, object);
            if g >= inst.borrow().globals.len() {
                continue;
            }
            let bound = self.si_track.as_ref().map_or(0, |t| t.bound);
            let pending = self.si_track.as_ref().map_or(0, |t| t.pending.len());
            let refused = match carry {
                Carry::Val(c) => match self.carry_decode(c, bound, &mut ids) {
                    Ok(v) => {
                        inst.borrow_mut().globals[g] = v;
                        if let Some(t) = self.si_track.as_mut()
                            && t.pending.len() > pending
                        {
                            t.held.push((object, g));
                        }
                        None
                    }
                    Err(why) => {
                        // A bound instance inside a value that does not read back: its binding is not carried.
                        if let Some(t) = self.si_track.as_mut()
                            && t.pending.len() > pending
                        {
                            t.pending.truncate(pending);
                            t.poisoned.entry(BINDINGS_KEY).or_insert_with(|| {
                                format!(
                                    "an event with a manual subscriber: an earlier test codeunit of the run left a \
                                     codeunit instance bound in {why}, which al-runner cannot hand on to the next \
                                     test codeunit exactly"
                                )
                            });
                        }
                        Some(why)
                    }
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

    /// The first use in this session of a single instance an earlier test codeunit left: it counts as made now for the
    /// Execute check (`Instance::born`), as in a session that starts without it. BC checks Execute on Format Address
    /// (SingleInstance) called through a local variable under a narrow set (permissions/wildcard-scope/
    /// Y93WildScope.al:439, :446-447) although codeunit 86504 (record/captions/QdCaptionClassAreas.al), which runs before
    /// it in the same session on BC 28.5 and 29.0 (corpus .github/ci-shards.json), made it through its General Ledger
    /// Setup subscriber; the Permissions Mock that a test codeunit's global holds stays unchecked, as in one codeunit
    /// (Y24PermExecute.al:35, :107). Assumed for the other shapes (docs/probes-needed.md, "SingleInstance carry").
    pub(crate) fn claim_carried_single(&mut self, object: usize) {
        if self.singles_unclaimed.remove(&object)
            && let Some(inst) = self.singletons.get(&object)
        {
            inst.borrow_mut().born = al_builtins::value::Born::default();
        }
    }

    /// What the session did with the single instances since [`Session::track_single_instances`]; `None` when it did not
    /// track them.
    pub fn single_out(&self) -> Option<SingleOut> {
        let t0 = std::time::Instant::now();
        let r = self.single_out_inner();
        TIME_OUT.fetch_add(t0.elapsed().as_nanos() as u64, std::sync::atomic::Ordering::Relaxed);
        r
    }
    fn single_out_inner(&self) -> Option<SingleOut> {
        let t = self.si_track.as_ref()?;
        let mut out = SingleOut {
            observed: t.observed.clone(),
            ..Default::default()
        };
        let t1 = std::time::Instant::now();
        let ranks = binding_ranks(self);
        TIME_IN.fetch_add(t1.elapsed().as_nanos() as u64, std::sync::atomic::Ordering::Relaxed);
        let t0 = std::time::Instant::now();
        let (objects, groups) = self.single_groups();
        TIME_GRP.fetch_add(t0.elapsed().as_nanos() as u64, std::sync::atomic::Ordering::Relaxed);
        // Counting what the groups share costs a walk of all their values: only when a group needs writing.
        let enc_cell = std::cell::OnceCell::new();
        let enc = || enc_cell.get_or_init(|| { let t0 = std::time::Instant::now(); let e = Enc::new(self, &groups, t.bound, ranks.ranks.clone()); TIME_ENC.fetch_add(t0.elapsed().as_nanos() as u64, std::sync::atomic::Ordering::Relaxed); e });
        for (&object, globals) in objects.iter().zip(&groups) {
            // After a BindSubscription or UnbindSubscription, a global that holds a bound instance, or held one at the
            // start, may stand for another binding although it holds the same handle.
            let rebound = t.bindings_changed
                && (t.held.iter().any(|k| k.0 == object) || {
                    let mut ranks = Vec::new();
                    carry_group(enc(), globals).iter().for_each(|c| {
                        if let Carry::Val(v) = c {
                            bound_in(v, &mut ranks)
                        }
                    });
                    !ranks.is_empty()
                });
            let changed: Vec<bool> = (0..globals.len())
                .map(|g| {
                    let key = (object, g);
                    rebound || t.seen.contains_key(&key) || t.start.get(&key).is_none_or(|s| !same(s, &globals[g]))
                })
                .collect();
            if !changed.contains(&true) {
                continue;
            }
            let t0 = std::time::Instant::now();
            let carried = carry_group(enc(), globals);
            TIME_GRP.fetch_add(t0.elapsed().as_nanos() as u64, std::sync::atomic::Ordering::Relaxed);
            // Globals that share a handle are handed on together (one may have changed through another). What the
            // others held at the start then counts as read: the walk checks it against the serial state, as what
            // they hand on rests on it.
            let shares = carried.iter().any(|c| matches!(c, Carry::Val(x) if IdUse::of(x).any()));
            let start = (shares && !t.group_start.contains_key(&object)).then(|| self.start_group(t, object, globals));
            for (g, c) in carried.into_iter().enumerate() {
                let key = (object, g);
                if !shares && !changed[g] {
                    continue;
                }
                if shares && !t.seen.contains_key(&key) {
                    let at_start = t
                        .group_start
                        .get(&object)
                        .or(start.as_ref())
                        .and_then(|s| s.get(g))
                        .cloned();
                    if let Some(c) = at_start {
                        out.observed.entry(key).or_insert(c);
                    }
                }
                // A global whose carried value did not read back here and that the codeunit did not write since
                // still stands for what the earlier codeunit left: not carried.
                let c = match t.poisoned.get(&key) {
                    Some(why) => Carry::Opaque(why.clone()),
                    None => c,
                };
                out.wrote.insert(key, c);
            }
        }
        let bindings = match t.poisoned.get(&BINDINGS_KEY) {
            Some(why) => Carry::Opaque(why.clone()),
            None => { let t0 = std::time::Instant::now(); let b = self.binding_carry_with(ranks); TIME_RK.fetch_add(t0.elapsed().as_nanos() as u64, std::sync::atomic::Ordering::Relaxed); b }
        };
        if t.bindings_changed || t.bind_start.as_ref() != Some(&bindings) {
            out.wrote.insert(BINDINGS_KEY, bindings);
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
            // The read is noted as one of a value that is not carried: the walk does not take this run as the serial one
            // when the serial state holds a value this session could not read back (its committed row version below the
            // record's, `single_walk.rs` `key_fits`).
            t.observed.entry(key).or_insert_with(|| Carry::Opaque(why.clone()));
            return Err(crate::Signal::Unsupported(why.clone()));
        }
        t.seen.insert(key, Seen::Read);
        if !t.group_start.contains_key(&object) {
            let bound = t.bound;
            let (_, groups) = self.single_groups();
            let globals = inst.borrow().globals.clone();
            let mut carried = carry_group(&Enc::new(self, &groups, bound, binding_ranks(self).ranks), &globals);
            let Some(t) = self.si_track.as_mut() else { return Ok(()) };
            // A global written before this first read: the handles it shares with the others are no longer those
            // of the start, so what those others held at the start is not known.
            let written = (0..globals.len()).any(|j| j != g && t.seen.get(&(object, j)) == Some(&Seen::Written));
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

    /// What the globals of single instance `object` held when the session began to track them, written as a group
    /// (for those the session used, the values then; a global it never used still holds it).
    fn start_group(&self, t: &Track, object: usize, globals: &[Value]) -> Vec<Carry> {
        let start: Vec<Value> = (0..globals.len())
            .map(|g| t.start.get(&(object, g)).cloned().unwrap_or_else(|| globals[g].clone()))
            .collect();
        let group = vec![start];
        carry_group(&Enc::new(self, &group, t.bound, binding_ranks(self).ranks), &group[0])
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
    /// The manual bindings BC keeps for the next test codeunit (`single_codec::binding_ranks`), oldest first.
    bindings: Vec<crate::dispatch::Binding>,
}

impl Session {
    /// Moves the single instances and the temporary tables out of this finished session (see [`MovedSingles`]).
    pub fn take_single_instances(&mut self) -> MovedSingles {
        let kept = binding_ranks(self);
        let mut bindings: Vec<(u32, crate::dispatch::Binding)> = Vec::new();
        for b in &self.events.bindings {
            let Some(i) = b.instance.upgrade() else { continue };
            let object = i.borrow().object as usize;
            let rank = kept
                .ranks
                .get(&(Rc::as_ptr(&i) as *const () as usize))
                .copied()
                .or_else(|| {
                    kept.singles
                        .iter()
                        .find(|s| s.0 == object && i.borrow().single)
                        .map(|s| s.1)
                });
            if let Some(r) = rank {
                bindings.push((
                    r,
                    crate::dispatch::Binding {
                        instance: Rc::downgrade(&i),
                    },
                ));
            }
        }
        bindings.sort_by_key(|b| b.0);
        MovedSingles {
            bindings: bindings.into_iter().map(|b| b.1).collect(),
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
        self.singles_unclaimed.extend(moved.singletons.keys().copied());
        self.singletons.extend(moved.singletons);
        self.events.bindings.splice(0..0, moved.bindings);
        self.sync_active_instances();
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
            Ok(c) if IdUse::of(&c).refs.is_subset(&defs) => Carry::Val(std::sync::Arc::new(c)),
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
        let v = Carry::Val(std::sync::Arc::new(CarryVal::Plain(vec![0])));
        assert_eq!(v, v.clone());
        assert_ne!(v, o);
    }
}

pub static TIME_OUT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub static TIME_IN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub static TIME_ENC: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub static TIME_GRP: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub static TIME_RK: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
