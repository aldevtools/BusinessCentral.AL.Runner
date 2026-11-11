//! What a test can change that a later test of the same codeunit reads. Under TestIsolation = Codeunit the tests of a
//! codeunit share the codeunit's instance (its globals and the codeunit variables they hold), the single-instance
//! codeunits and the rest of the session (the `Random` sequence, `WorkDate`, number sequences, bound subscribers), and,
//! unless Function isolation restores it, the database. A test that did not run to its end left that state other than
//! BC did; a later failure is trusted only when nothing the earlier test could have changed is read by the failing test
//! (`run.rs`).
//!
//! Everything here is static: the reach of a test is its calls, the triggers of the tables its records use, the routines
//! of what it runs by a constant id, and the subscribers of the events all of these can raise. A refused routine, or a
//! codeunit, page, report or xmlport run by an id known only at run time, makes the reach unknown: it may change
//! anything.
use super::*;
use al_builtins::value::RtType;
use al_ir::{Arg, Const, Expr, ObjKind, Place, Stmt};
use std::collections::{BTreeMap, HashMap};
use std::sync::OnceLock;

/// Session state that lasts from one test of a codeunit to the next, as bits of [`ReachFacts::state_reads`] and
/// [`ReachFacts::state_writes`].
pub(crate) mod state {
    /// The session's `Random` sequence (`Random`, `Randomize`). Whether a later test reads it is what it drew when it
    /// ran (`run.rs`), not its reach.
    pub const RANDOM: u16 = 1;
    /// `WorkDate`.
    pub const WORK_DATE: u16 = 1 << 1;
    /// `GlobalLanguage` and the session settings (`SessionSettings.RequestSessionUpdate`).
    pub const LANGUAGE: u16 = 1 << 2;
    /// Number sequences.
    pub const NUMBER_SEQUENCE: u16 = 1 << 3;
    /// Isolated storage.
    pub const ISOLATED_STORAGE: u16 = 1 << 4;
    /// Subscribers bound with `BindSubscription`.
    pub const BINDING: u16 = 1 << 5;
    /// Files.
    pub const FILES: u16 = 1 << 6;
    /// What the analysis does not follow: a .NET object, a background task, a builtin that is not classified.
    pub const OTHER: u16 = 1 << 7;
    /// What an HTTP server outside the session received (`HttpClient`). Only a test that sends a request itself can see
    /// it (al-runner issue 5610: a refused HttpClient.Get made every later failure of the codeunit not trusted).
    pub const HTTP: u16 = 1 << 8;
    /// `Randomize(n)` with a seed: the sequence then goes on into the next test of the codeunit
    /// (types/TestRandomSeededContracts.al:111-131), past the run seed's restart of each test (ADR-061). Set with
    /// [`RANDOM`].
    pub const RANDOM_SEED: u16 = 1 << 9;
    /// State every test is taken to read: what AL code reads without naming it (the posting date defaults to
    /// `WorkDate`, `Format` follows the language, any event can reach a bound subscriber).
    pub const READ_BY_ALL: u16 = WORK_DATE | LANGUAGE | BINDING | OTHER;

    pub fn name(bit: u16) -> &'static str {
        match bit {
            RANDOM => "the Random sequence",
            WORK_DATE => "WorkDate",
            LANGUAGE => "the session language",
            NUMBER_SEQUENCE => "a number sequence",
            ISOLATED_STORAGE => "isolated storage",
            BINDING => "the bound event subscribers",
            FILES => "files",
            HTTP => "what an HTTP server received",
            RANDOM_SEED => "the Random seed",
            _ => "session state the analysis does not follow",
        }
    }
}

/// What [`Walk::entered`] notes besides the bits of [`ops`].
const EVENTS: u8 = 1 << 6;
const RUN: u8 = 1 << 7;
/// What [`Walk::entered`] notes for a table already in [`ReachFacts::touched`] (a table is an object).
const TOUCHED: u8 = 1 << 5;

/// Record operations that run table triggers (the mapping of `al_sema` `check/demand.rs`).
mod ops {
    use al_ir::{Arg, Const, Expr};
    pub const INSERT: u8 = 1;
    pub const MODIFY: u8 = 2;
    pub const DELETE: u8 = 4;
    pub const RENAME: u8 = 8;
    pub const VALIDATE: u8 = 16;
    pub const ALL: u8 = INSERT | MODIFY | DELETE | RENAME | VALIDATE;

    /// The operations a record method performs that can run triggers.
    pub fn of(method: &str) -> u8 {
        match method {
            "Insert" => INSERT,
            "Modify" => MODIFY,
            "Delete" | "DeleteAll" => DELETE,
            "Rename" => RENAME,
            "Validate" => VALIDATE,
            "ModifyAll" => MODIFY,
            _ => 0,
        }
    }

    /// Whether the call runs the triggers: `Rename` and `Validate` always; `Insert`, `Modify`, `Delete`, `DeleteAll`
    /// and `ModifyAll` when their RunTrigger argument is not the constant false (it is false when left out).
    pub fn runs_triggers(method: &str, args: &[Arg]) -> bool {
        let run_trigger = match method {
            "Rename" | "Validate" => return true,
            "ModifyAll" => args.get(2),
            _ => args.first(),
        };
        !matches!(run_trigger, None | Some(Arg::Value(Expr::Const(Const::Boolean(false)))))
    }
}

/// What the static reach of a routine holds: the tables it reads or writes, the globals of codeunits it reads or
/// writes, the session state it reads or writes, and whether part of it is unknown.
#[derive(Debug, Default, Clone)]
pub(crate) struct ReachFacts {
    /// Tables whose records appear in a slot of anything reached, or that a query reached reads.
    pub touched: BTreeSet<u32>,
    /// Tables a reached routine writes: Insert, Modify, Delete, DeleteAll, ModifyAll and Rename on a record of a known
    /// table, and the source table of a page variable.
    pub writes: BTreeSet<u32>,
    /// The tables of `writes` a reached routine can add a row to: Insert on a record of a known table that is not
    /// declared temporary, and the source table of a page variable unless the page says SourceTableTemporary (the page
    /// client inserts). Modify, Delete, DeleteAll, ModifyAll and Rename need a row that is already there, and a
    /// temporary record adds no row to the table (`login.rs`).
    pub inserts: BTreeSet<u32>,
    /// A write whose table is not known statically (a `RecordRef`, an xmlport import, media): it may write any table.
    pub writes_any: bool,
    /// Global variables of codeunits (object, slot) a reached routine names.
    pub globals_read: BTreeSet<(usize, usize)>,
    /// Global variables of codeunits (object, slot) a reached routine assigns, hands to a `var` parameter, calls a
    /// method on, or clears. Globals of tables, pages and reports live with their variable, which is a slot of
    /// its own.
    pub globals_written: BTreeSet<(usize, usize)>,
    /// The session state (bits of [`state`]) a reached routine reads.
    pub state_reads: u16,
    /// The session state a reached routine changes.
    pub state_writes: u16,
    /// Why part of the reach is not known (a refused routine, a run by an id known only at run time): it may change
    /// anything.
    pub unknown: Option<String>,
    /// The first report reached that the runtime refuses when it runs (also part of `unknown`, which keeps the first reason of any kind).
    pub report_cannot_run: Option<String>,
}

impl ReachFacts {
    /// Adds what `other` holds: the reach of two routines taken together (the first reason of `unknown` stays).
    pub fn absorb(&mut self, other: &ReachFacts) {
        self.touched.extend(other.touched.iter().copied());
        self.writes.extend(other.writes.iter().copied());
        self.inserts.extend(other.inserts.iter().copied());
        self.writes_any |= other.writes_any;
        self.globals_read.extend(other.globals_read.iter().copied());
        self.globals_written.extend(other.globals_written.iter().copied());
        self.state_reads |= other.state_reads;
        self.state_writes |= other.state_writes;
        if self.unknown.is_none() {
            self.unknown.clone_from(&other.unknown);
        }
        if self.report_cannot_run.is_none() {
            self.report_cannot_run.clone_from(&other.report_cannot_run);
        }
    }

    /// Whether the reach can draw from or reseed the session's `Random` sequence.
    pub fn random(&self) -> bool {
        self.state_writes & state::RANDOM != 0 || self.unknown.is_some()
    }

    /// Whether the reach can seed the session's `Random` sequence (`Randomize(n)`), so that a later test goes on with it
    /// instead of the run seed's restart (ADR-061).
    pub fn seeds_random(&self) -> bool {
        self.state_writes & state::RANDOM_SEED != 0 || self.unknown.is_some()
    }

    /// What this reach (of a test that did not run to its end) can change that the reach `later` reads, named for the
    /// reason text; `None` when nothing. `tables` is false under Function isolation, which restores the database after
    /// each test. `drew` says whether the later test drew numbers from the `Random` sequence it found.
    /// `settled` lists globals that hold, when the later test starts, the only value any earlier test can have left in
    /// them ([`settled_flags`]): they do not count.
    pub fn changes_read_by(
        &self,
        later: &ReachFacts,
        tables: bool,
        drew: bool,
        settled: &BTreeSet<(usize, usize)>,
        lowered: &Lowered,
    ) -> Option<String> {
        let objs = &lowered.program.objects;
        if let Some(why) = &self.unknown {
            return Some(format!(
                "part of its reach is not known ({why}), so it may change anything"
            ));
        }
        let shared = self.state_writes & (state::READ_BY_ALL | later.state_reads);
        if shared != 0 {
            let bit = 1 << shared.trailing_zeros();
            return Some(format!("it can change {}", state::name(bit)));
        }
        if drew && self.state_writes & state::RANDOM != 0 {
            return Some(format!("it can draw numbers from {}", state::name(state::RANDOM)));
        }
        if let Some(&(o, g)) = self
            .globals_written
            .intersection(&later.globals_read)
            .find(|g| !settled.contains(g))
        {
            return Some(format!(
                "it can change the global variable {}.{} that this test reads",
                objs[o].name, objs[o].globals[g].name
            ));
        }
        if tables {
            let met = if self.writes_any {
                later.touched.iter().next()
            } else {
                self.writes.intersection(&later.touched).next()
            };
            if let Some(&t) = met {
                return Some(format!(
                    "it can write the table {} that this test reads",
                    objs.get(t as usize).map_or("?", |o| o.name.as_str())
                ));
            }
        }
        None
    }
}

/// What the reach walk looks up by object: the extensions of a table, page or report, objects by kind and id, the
/// subscribers to the events of each object, and the subscribers of the `Global Triggers` database events. Built once
/// per engine (`Engine::derived`): it depends on the program only.
pub(crate) struct ReachIndex {
    by_id: HashMap<(ObjKind, u32), usize>,
    extensions: HashMap<(ObjKind, u32), Vec<usize>>,
    /// Per publisher object: the subscriber routines of all its events.
    subscribers: HashMap<usize, Vec<(usize, usize)>>,
    /// The subscribers of the `Global Triggers` codeunit, which fire on database writes.
    global_triggers: Vec<(usize, usize)>,
    /// Per object and routine: its [`Summary`], recorded at the first walk that visits it. It depends on the program
    /// and on the event registry the index is built from, which every walk on the index is given.
    summaries: Vec<Box<[OnceLock<Box<Summary>>]>>,
    /// Per object, the number of routines of the objects before it, and the total at the end: a walk's routine bits.
    first_routine: Vec<usize>,
    /// The routines (bits as in a walk's `seen`) whose whole reach is known to hold no report the runtime refuses: a
    /// walk that ended without finding one visited a set closed under what its routines reach, so each routine in it
    /// is clean, and a later question about a report skips them ([`report_that_cannot_run`]).
    report_clean: std::sync::Mutex<Vec<u64>>,
}

impl ReachIndex {
    pub fn new(events: &al_vm::events::Registry, lowered: &Lowered) -> ReachIndex {
        // Heads only: the index makes no object (ADR-054).
        let objs = lowered.program.objects.heads();
        let mut by_id = HashMap::new();
        let mut extensions: HashMap<(ObjKind, u32), Vec<usize>> = HashMap::new();
        for (i, o) in objs.iter().enumerate() {
            if let Some(id) = o.id {
                by_id.entry((o.kind, id)).or_insert(i);
            }
            let base = match o.kind {
                ObjKind::TableExtension => ObjKind::Table,
                ObjKind::PageExtension => ObjKind::Page,
                ObjKind::ReportExtension => ObjKind::Report,
                _ => continue,
            };
            if let Some(b) = o.extends_id {
                extensions.entry((base, b)).or_default().push(i);
            }
        }
        let mut subscribers: HashMap<usize, Vec<(usize, usize)>> = HashMap::new();
        let mut global_triggers = Vec::new();
        for ((po, _), subs) in &events.subscribers {
            let is_global_triggers = *po == al_vm::events::GLOBAL_TRIGGERS
                || objs
                    .get(*po)
                    .is_some_and(|o| o.kind == ObjKind::Codeunit && o.id == Some(2_000_000_002));
            for s in subs {
                subscribers.entry(*po).or_default().push((s.object, s.routine));
                if is_global_triggers {
                    global_triggers.push((s.object, s.routine));
                }
            }
        }
        // `events.subscribers` is a hash map: sort, so the walk visits subscribers in the same order on every run and
        // names the same unknown part ("part of its reach is not known (X)") each time.
        for subs in subscribers.values_mut() {
            subs.sort_unstable();
            subs.dedup();
        }
        global_triggers.sort_unstable();
        global_triggers.dedup();
        let summaries = objs
            .iter()
            .map(|o| (0..o.routine_count).map(|_| OnceLock::new()).collect())
            .collect();
        let mut first_routine = Vec::with_capacity(objs.len() + 1);
        let mut total = 0;
        for o in objs {
            first_routine.push(total);
            total += o.routine_count as usize;
        }
        first_routine.push(total);
        ReachIndex {
            by_id,
            extensions,
            subscribers,
            global_triggers,
            summaries,
            report_clean: std::sync::Mutex::new(vec![0; total.div_ceil(64)]),
            first_routine,
        }
    }

    /// How many routines are known to reach no report the runtime refuses.
    #[cfg(test)]
    fn report_clean_routines(&self) -> usize {
        let bits = self.report_clean.lock().unwrap_or_else(|e| e.into_inner());
        bits.iter().map(|w| w.count_ones() as usize).sum()
    }

    /// The [`Summary`] of routine `r` of `o`, recorded on first use; `None` for a routine the index does not know of
    /// (none: the program does not change under an engine).
    fn summary(&self, events: &al_vm::events::Registry, lowered: &Lowered, o: usize, r: usize) -> Option<&Summary> {
        let cell = self.summaries.get(o)?.get(r)?;
        Some(cell.get_or_init(|| Box::new(Rec::record(events, lowered, self, o, r))))
    }

    /// How many routines have a recorded [`Summary`].
    #[cfg(test)]
    fn recorded(&self) -> usize {
        self.summaries.iter().flatten().filter(|c| c.get().is_some()).count()
    }
}

/// The reach facts of routine `routine` of `object`.
pub(crate) fn reach_facts(
    events: &al_vm::events::Registry,
    lowered: &Lowered,
    index: &ReachIndex,
    object: usize,
    routine: usize,
) -> ReachFacts {
    walk_reach(events, lowered, index, &[(object, routine)], &|_| false)
}

/// The facts of the calls of routine `routine` of `object`: the routines it calls and those they call, without the table
/// triggers, the subscribers of the events they raise (integration, business, table and object events, and the `Global
/// Triggers` database events), and the objects run. What a routine writes through its own calls, for the tables whose rows a test can only have from it
/// (`login.rs`): the event fan-out of a walk reaches most of Base Application from one record variable.
pub(crate) fn call_reach_facts(
    events: &al_vm::events::Registry,
    lowered: &Lowered,
    index: &ReachIndex,
    object: usize,
    routine: usize,
) -> ReachFacts {
    let mut walk = Walk::new(events, lowered, index);
    walk.calls_only = true;
    walk.stack.push((object, routine));
    walk.run(&|_| false);
    walk.out
}

/// What a login subscriber (routine `routine` of `object`) leaves through its own code (`login.rs`): its calls, the
/// objects it runs by a constant id or through a codeunit variable (login code that runs in a codeunit of its own, so an
/// error cannot stop the login), the triggers of the tables those write, and the subscribers in apps under test of the
/// events of apps under test those raise, without the subscribers of Microsoft's events (their fan-out writes most of
/// Base Application). The test side asks its own calls with the objects run ([`union_call_reach_facts`]); the table triggers
/// and the app's own subscribers are followed here too because what they leave is there before the first test as well
/// (al-runner issue 7717).
pub(crate) fn login_subscriber_reach_facts(
    events: &al_vm::events::Registry,
    lowered: &Lowered,
    index: &ReachIndex,
    object: usize,
    routine: usize,
) -> ReachFacts {
    let mut walk = Walk::new(events, lowered, index);
    walk.calls_only = true;
    walk.runs_in_calls = true;
    walk.login = true;
    walk.stack.push((object, routine));
    walk.run(&|_| false);
    walk.out
}

/// [`login_subscriber_reach_facts`] of the login subscribers `roots` together, without entering the routines of the
/// objects `skip` (`login.rs` asks the calls into those itself), and the routines the walk visited.
pub(crate) fn login_subscribers_reach_without(
    events: &al_vm::events::Registry,
    lowered: &Lowered,
    index: &ReachIndex,
    roots: &[(usize, usize)],
    skip: &[usize],
) -> (ReachFacts, Vec<(usize, usize)>) {
    let mut walk = Walk::new(events, lowered, index);
    walk.calls_only = true;
    walk.runs_in_calls = true;
    walk.login = true;
    walk.visited = Some(Vec::new());
    for &o in skip {
        for r in 0..lowered.program.objects[o].routines.len() {
            walk.first_visit(o, r);
        }
    }
    walk.stack.extend(roots.iter().rev().copied());
    walk.run(&|_| false);
    (walk.out, walk.visited.unwrap_or_default())
}

/// The facts of the joined reach of the routines `roots` (one walk of the union of their reaches, as
/// [`union_reach_facts_skipping`] with nothing skipped), and every routine the walk entered (`shared_pass.rs`).
pub(crate) fn union_reach_visited(
    events: &al_vm::events::Registry,
    lowered: &Lowered,
    index: &ReachIndex,
    roots: &[(usize, usize)],
) -> UnionReach {
    let mut walk = Walk::new(events, lowered, index);
    walk.visited = Some(Vec::new());
    walk.unknowns = Some(BTreeSet::new());
    walk.stack.extend(roots.iter().rev().copied());
    walk.run(&|_| false);
    UnionReach {
        visited: walk.visited.take().unwrap_or_default(),
        unknowns: walk.unknowns.take().unwrap_or_default(),
        facts: walk.out,
    }
}

/// What [`union_reach_visited`] found.
pub(crate) struct UnionReach {
    pub facts: ReachFacts,
    /// The routines visited.
    pub visited: Vec<(usize, usize)>,
    /// Every reason the walk could not follow part of the reach (each routine's first of each kind), with its kind.
    pub unknowns: BTreeSet<(Unknown, String)>,
}

/// The kind of a part of the reach the walk cannot follow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Unknown {
    /// A routine that is not compiled (`al_sema::check::is_not_compiled`), or the triggers or the run of an object that
    /// is not compiled: a call of it is refused, so it never runs in part.
    NotCompiled,
    /// Anything else: a refused construct, a run by an id known only at run time, a report that cannot run.
    Other,
}

/// The facts of the joined reach of the routines `roots`, one walk of the union of their reaches, that does not enter
/// the routines `skip` (nor what only they reach).
pub(crate) fn union_reach_facts_skipping(
    events: &al_vm::events::Registry,
    lowered: &Lowered,
    index: &ReachIndex,
    roots: &[(usize, usize)],
    skip: &[(usize, usize)],
) -> ReachFacts {
    let mut walk = Walk::new(events, lowered, index);
    for &(o, r) in skip {
        walk.first_visit(o, r);
    }
    walk.stack.extend(roots.iter().rev().copied());
    walk.run(&|_| false);
    walk.out
}

/// The facts of what the routines `roots` reach through calls alone, one walk of the union: the routines they call, those
/// they call, what their page, report and xmlport variables drive (`Lowered::calls_into`), and the routines of the
/// objects they run by a constant id or through a codeunit variable (a codeunit's OnRun, a page's routines), without the
/// routines `skip`. As [`call_reach_facts`], it leaves out the table triggers and the subscribers of every event. What
/// a test does itself, apart from what any write or event brings into its reach (`login.rs`).
pub(crate) fn union_call_reach_facts(
    events: &al_vm::events::Registry,
    lowered: &Lowered,
    index: &ReachIndex,
    roots: &[(usize, usize)],
    skip: &[(usize, usize)],
) -> ReachFacts {
    let mut walk = Walk::new(events, lowered, index);
    walk.calls_only = true;
    walk.runs_in_calls = true;
    for &(o, r) in skip {
        walk.first_visit(o, r);
    }
    walk.stack.extend(roots.iter().rev().copied());
    walk.run(&|_| false);
    walk.out
}

/// [`union_reach_facts_skipping`] without the subscribers of the `Global Triggers` database events, which every write
/// brings into a reach: the calls, the table triggers, the objects run and the subscribers of the other events are
/// followed as before. What a test reaches other than through "some table was written" (`login.rs`).
pub(crate) fn union_reach_facts_without_database_events(
    events: &al_vm::events::Registry,
    lowered: &Lowered,
    index: &ReachIndex,
    roots: &[(usize, usize)],
    skip: &[(usize, usize)],
) -> ReachFacts {
    let mut walk = Walk::new(events, lowered, index);
    // As if they were pushed already (`Walk::note_table_write`).
    walk.global_triggers = true;
    for &(o, r) in skip {
        walk.first_visit(o, r);
    }
    walk.stack.extend(roots.iter().rev().copied());
    walk.run(&|_| false);
    walk.out
}

/// The facts of the reach of the subscribers of the `Global Triggers` database events, not entering the routines `skip`:
/// what a walk of [`union_reach_facts_skipping`] adds at its first write ([`JoinedReach`]). It depends on
/// the program and `skip` only, so a caller that asks it for many roots with the same `skip` keeps it per engine.
pub(crate) fn global_triggers_reach_facts(
    events: &al_vm::events::Registry,
    lowered: &Lowered,
    index: &ReachIndex,
    skip: &[(usize, usize)],
) -> ReachFacts {
    let mut walk = Walk::new(events, lowered, index);
    // Pushed here, as `Walk::note_table_write` pushes them, and not again at a write.
    walk.global_triggers = true;
    for &(o, r) in skip {
        walk.first_visit(o, r);
    }
    walk.stack.extend(index.global_triggers.iter().copied());
    walk.run(&|_| false);
    walk.out
}

/// The reach of [`union_reach_facts_skipping`] of some roots and `skip`, kept as two walks instead of joined: `own`, the
/// [`union_reach_facts_without_database_events`] of the same roots and `skip`, and `database_events`, the
/// [`global_triggers_reach_facts`] of the same `skip`, asked only when `own` writes. The second is the same for every
/// root, so a caller that walks the reach of many test codeunits walks the database event fan-out (in Microsoft's apps,
/// much of Base Application) once instead of once per codeunit, and a question is answered from the two parts without
/// copying that fan-out into each codeunit's facts.
///
/// Why the answers are those of the full walk: a walk visits the routines reachable from its roots, not entering `skip`,
/// over edges that the program fixes (a routine's [`Summary`] steps, a table's triggers, an object's event subscribers,
/// the routines of an object run). What the walk has already entered only decides whether it pushes an edge again, never
/// where an edge leads. The one edge that depends on the walk is the push of the `Global Triggers` subscribers at the
/// first write (`Walk::note_table_write`). So the full reach is the reach without that edge, plus, when that reach writes
/// (a known table or `writes_any`), the reach of those subscribers; a write inside their reach adds nothing, since they
/// are pushed already. Every fact is a union over the routines visited (sets and bits), so it holds of the full reach
/// exactly when it holds of one of the parts.
pub(crate) struct JoinedReach<'a> {
    own: &'a ReachFacts,
    database_events: Option<&'a ReachFacts>,
}

impl<'a> JoinedReach<'a> {
    pub fn new(own: &'a ReachFacts, database_events: impl FnOnce() -> &'a ReachFacts) -> JoinedReach<'a> {
        let writes = !own.writes.is_empty() || own.writes_any;
        JoinedReach {
            own,
            database_events: writes.then(database_events),
        }
    }

    fn parts(&self) -> impl Iterator<Item = &'a ReachFacts> {
        std::iter::once(self.own).chain(self.database_events)
    }

    /// Whether table `t` is in [`ReachFacts::touched`] of the joined reach.
    pub fn touches(&self, t: u32) -> bool {
        self.parts().any(|f| f.touched.contains(&t))
    }

    /// Whether table `t` is in [`ReachFacts::writes`] of the joined reach.
    pub fn writes_table(&self, t: u32) -> bool {
        self.parts().any(|f| f.writes.contains(&t))
    }

    /// Whether global `g` (object, slot) is in [`ReachFacts::globals_read`] of the joined reach.
    pub fn reads_global(&self, g: (usize, usize)) -> bool {
        self.parts().any(|f| f.globals_read.contains(&g))
    }

    /// Whether a global of object `o` is in [`ReachFacts::globals_read`] of the joined reach.
    pub fn reads_a_global_of(&self, o: usize) -> bool {
        self.parts()
            .any(|f| f.globals_read.range((o, 0)..=(o, usize::MAX)).next().is_some())
    }

    /// The facts of the joined reach, built.
    #[cfg(test)]
    fn facts(&self) -> ReachFacts {
        let mut full = self.own.clone();
        if let Some(d) = self.database_events {
            full.absorb(d);
        }
        full
    }
}

/// Whether `fact` holds of the joined reach of the routines `roots`: one walk of the union of their reaches, which
/// stops as soon as `fact` holds. `fact` must be one that, once it holds of part of a reach, holds of all of it (a
/// table, a state bit, a part not known); it then holds of the union exactly when it holds of the reach of one of the
/// roots. A walk per root visits the reach they share once per root; the tests of a codeunit share most of theirs.
pub(crate) fn union_reach_has(
    events: &al_vm::events::Registry,
    lowered: &Lowered,
    index: &ReachIndex,
    roots: &[(usize, usize)],
    fact: &dyn Fn(&ReachFacts) -> bool,
) -> bool {
    fact(&walk_reach(events, lowered, index, roots, fact))
}

/// The reach facts of an unfinished test (the earlier side of [`ReachFacts::changes_read_by`]): the walk stops at
/// the first part of the reach that is not known, because that alone decides both questions asked of it
/// (`changes_read_by` and `random` answer from `unknown` before anything else). The rest of the facts are then
/// incomplete and must not be read. A walk of the whole reach visits about 49 000 routines with Microsoft's apps
/// (150 ms), once per unfinished test of a codeunit.
pub(crate) fn report_that_cannot_run(
    events: &al_vm::events::Registry,
    lowered: &Lowered,
    index: &ReachIndex,
    object: usize,
    routine: usize,
) -> Option<String> {
    // Only the report is asked for: the walk follows the same routines but keeps none of the facts (tables, globals, state).
    // It starts with the routines an earlier walk found clean marked as visited. A walk that finds no report visits the
    // whole reach, so what it visited is clean for the next question; a walk that stops at a report proves nothing.
    let mut walk = Walk::new(events, lowered, index);
    walk.facts = false;
    walk.seen
        .copy_from_slice(&index.report_clean.lock().unwrap_or_else(|e| e.into_inner()));
    walk.stack.push((object, routine));
    walk.run(&|f| f.report_cannot_run.is_some());
    if walk.out.report_cannot_run.is_none() {
        let mut clean = index.report_clean.lock().unwrap_or_else(|e| e.into_inner());
        for (c, s) in clean.iter_mut().zip(&walk.seen) {
            *c |= *s;
        }
    }
    walk.out.report_cannot_run
}

pub(crate) fn unfinished_reach_facts(
    events: &al_vm::events::Registry,
    lowered: &Lowered,
    index: &ReachIndex,
    object: usize,
    routine: usize,
) -> ReachFacts {
    walk_reach(events, lowered, index, &[(object, routine)], &|f| f.unknown.is_some())
}

/// The index in the body of test routine `routine` of `object` of the statement that holds the statement position
/// `pos` (a [`Stmt::Pos`] at the top level or inside a compound statement); `None` when no statement does.
pub(crate) fn statement_at(lowered: &Lowered, object: usize, routine: usize, pos: u32) -> Option<usize> {
    fn holds(body: &[Stmt], pos: u32) -> bool {
        body.iter().any(|s| match s {
            Stmt::Pos(p) => *p == pos,
            Stmt::If(_, a, b) => holds(a, pos) || holds(b, pos),
            Stmt::While(_, b) | Stmt::Repeat(b, _) | Stmt::AssertError(b) => holds(b, pos),
            Stmt::For { body, .. } | Stmt::Foreach { body, .. } => holds(body, pos),
            Stmt::Case {
                branches, otherwise, ..
            } => branches.iter().any(|(_, b)| holds(b, pos)) || holds(otherwise, pos),
            _ => false,
        })
    }
    let body = &lowered.program.objects[object].routines.get(routine)?.body;
    body.iter().position(|s| holds(std::slice::from_ref(s), pos))
}

/// The reach facts of a test refused while it ran, when the refusal came from a statement of the test method itself
/// (al-runner issue 5961): what it wrote while it ran (`written`: our run rolls those writes back, BC keeps them), and
/// the static reach of what it did not run: the statements of its body from the one at index `from` (the statement it
/// was refused in, which may have run in part) to the end, and its handler functions `handlers`. What it changed
/// outside the database before the refusal (globals, single-instance codeunits, session state) is what BC's run
/// changed too: the code before the refusal is the code BC ran. A test page it still holds closes when the test ends:
/// a state whose end is not pinned refuses there (al-vm `end_unpinned_page_scope`, which `run.rs` sees), else the close
/// changes nothing in our model, and the page's triggers and events are taken into the reach anyway.
///
/// `None` when the rest cannot be bounded this way and the whole routine counts ([`unfinished_reach_facts`]): the
/// routine has a lowering refusal, or the rest uses a page, test page, report or xmlport variable for anything but
/// reading a test page's filter (`TestPage.Filter.GetFilter`, which runs no code). What such a variable drives
/// (`Lowered::calls_into`) is known only for the whole routine.
#[allow(clippy::too_many_arguments)]
pub(crate) fn remainder_reach_facts(
    events: &al_vm::events::Registry,
    lowered: &Lowered,
    index: &ReachIndex,
    object: usize,
    routine: usize,
    from: usize,
    handlers: &[usize],
    written: &BTreeSet<u32>,
) -> Option<ReachFacts> {
    let objs = &lowered.program.objects;
    let obj = &objs[object];
    let rt = obj.routines.get(routine)?;
    if !lowered.unsupported[object][routine].is_empty() {
        return None;
    }
    let rest = rt.body.get(from..)?;
    // Per slot (global or not, index) the rest names: how often, and how often as the test page of a filter read.
    let mut named: BTreeMap<(bool, usize), (usize, usize)> = BTreeMap::new();
    fn root(p: &Place) -> Option<(bool, usize)> {
        let mut p = p;
        loop {
            match p {
                Place::Index(b, _) | Place::Field(b, _) | Place::RecGlobal(b, ..) => p = b,
                Place::Local(i) => return Some((false, *i)),
                Place::Global(i) => return Some((true, *i)),
                Place::BaseGlobal(_) | Place::ExtGlobal(..) => return None,
            }
        }
    }
    fn stmt_places(body: &[Stmt], note: &mut dyn FnMut(&Place)) {
        for s in body {
            match s {
                Stmt::Assign(p, _) => note(p),
                Stmt::For { var, body, .. } | Stmt::Foreach { var, body, .. } => {
                    note(var);
                    stmt_places(body, note);
                }
                Stmt::If(_, a, b) => {
                    stmt_places(a, note);
                    stmt_places(b, note);
                }
                Stmt::While(_, b) | Stmt::Repeat(b, _) | Stmt::AssertError(b) => stmt_places(b, note),
                Stmt::Case {
                    branches, otherwise, ..
                } => {
                    for (_, b) in branches {
                        stmt_places(b, note);
                    }
                    stmt_places(otherwise, note);
                }
                _ => {}
            }
        }
    }
    fn note(named: &mut BTreeMap<(bool, usize), (usize, usize)>, p: &Place) {
        if let Some(k) = root(p) {
            named.entry(k).or_default().0 += 1;
        }
    }
    fn refs(named: &mut BTreeMap<(bool, usize), (usize, usize)>, args: &[Arg]) {
        for a in args {
            if let Arg::Ref(p) = a {
                note(named, p);
            }
        }
    }
    stmt_places(rest, &mut |p| note(&mut named, p));
    let mut calls = Vec::new();
    al_ir::types::for_each_expr_post(rest, &mut |e| match e {
        Expr::Load(p) => note(&mut named, p),
        Expr::Call {
            object, routine, args, ..
        } => {
            calls.push((*object, *routine));
            refs(&mut named, args);
        }
        Expr::InterfaceCall { targets, args, .. } => {
            calls.extend(al_ir::walk::interface_routines(targets));
            refs(&mut named, args);
        }
        Expr::DotNet { args, .. } => refs(&mut named, args),
        Expr::Builtin { id, receiver, args, .. } => {
            refs(&mut named, args);
            if let Some(Arg::Ref(p)) = receiver.as_deref() {
                note(&mut named, p);
            }
            // `TP.Filter.GetFilter(..)`: the use of `TP` under it, counted already, is a filter read.
            if id.name().split('#').next() == Some("TestFilter.GetFilter") {
                if let Some(Arg::Value(Expr::Builtin {
                    id: filter,
                    receiver: Some(page),
                    ..
                })) = receiver.as_deref()
                {
                    if let (true, Arg::Ref(p) | Arg::Value(Expr::Load(p))) = (
                        filter.name().split('#').next() == Some("TestPage.Filter"),
                        page.as_ref(),
                    ) {
                        if let Some(e) = root(p).and_then(|k| named.get_mut(&k)) {
                            e.0 = e.0.saturating_sub(1);
                            e.1 += 1;
                        }
                    }
                }
            }
        }
        _ => {}
    });
    let mut rec = Rec {
        events,
        lowered,
        index,
        out: ReachFacts::default(),
        summary: Summary::default(),
        unknown: [false; 2],
    };
    for (&(global, i), &(uses, _)) in &named {
        let slot = if global { obj.globals.get(i) } else { rt.locals.get(i) };
        let slot = slot?;
        let mut ty = &slot.ty;
        while let RtType::Array(_, inner) = ty {
            ty = inner;
        }
        match ty {
            // Only filter reads: the page runs no code for them.
            RtType::Page(_) | RtType::TestPage(_) if uses == 0 => {}
            RtType::Page(_) | RtType::TestPage(_) => return None,
            RtType::Codeunit(c) if objs[*c as usize].kind != ObjKind::Codeunit => return None,
            _ => rec.slot(ty),
        }
    }
    // A test page the test holds is closed at its end: the page's triggers and the subscribers of its events.
    for s in rt.locals.iter().chain(&obj.globals) {
        if let RtType::Page(p) | RtType::TestPage(p) = &s.ty {
            rec.events(*p as usize);
            let page = &objs[*p as usize];
            let extensions = page
                .id
                .and_then(|id| index.extensions.get(&(ObjKind::Page, id)))
                .cloned()
                .unwrap_or_default();
            for x in std::iter::once(*p as usize).chain(extensions) {
                rec.push(
                    objs[x]
                        .routines
                        .iter()
                        .enumerate()
                        .filter(|(_, r)| r.is_trigger && r.name.to_ascii_lowercase().contains("close"))
                        .map(|(r, _)| (x, r)),
                );
            }
        }
    }
    rec.places(object, rest);
    al_ir::types::for_each_expr_post(rest, &mut |e| rec.expr(object, routine, e));
    rec.push(calls);
    rec.push(handlers.iter().map(|&h| (object, h)));
    let summary = rec.finish();
    let mut walk = Walk::new(events, lowered, index);
    walk.apply(&summary);
    while let Some((o, r)) = walk.stack.pop() {
        if walk.out.unknown.is_some() {
            break;
        }
        if walk.first_visit(o, r) {
            walk.routine(o, r);
        }
    }
    walk.out.writes.extend(written.iter().copied());
    walk.out.touched.extend(written.iter().copied());
    Some(walk.out)
}

/// The facts of the routines `routines` themselves, without what they reach: for a set of routines that ran, which
/// holds every routine the run called (`install::login_work_date`). What a routine's body names counts whether or not
/// the run took that branch.
pub(crate) fn routines_facts(
    events: &al_vm::events::Registry,
    lowered: &Lowered,
    index: &ReachIndex,
    routines: &[(usize, usize)],
) -> ReachFacts {
    let mut walk = Walk::new(events, lowered, index);
    for &(o, r) in routines {
        walk.routine(o, r);
    }
    walk.out
}

fn walk_reach(
    events: &al_vm::events::Registry,
    lowered: &Lowered,
    index: &ReachIndex,
    roots: &[(usize, usize)],
    stop: &dyn Fn(&ReachFacts) -> bool,
) -> ReachFacts {
    walk_reach_with(events, lowered, index, roots, true, stop)
}

/// [`walk_reach`]; with `facts` off the walk keeps only `unknown` and `report_cannot_run` (the routines it follows are the same).
fn walk_reach_with(
    events: &al_vm::events::Registry,
    lowered: &Lowered,
    index: &ReachIndex,
    roots: &[(usize, usize)],
    facts: bool,
    stop: &dyn Fn(&ReachFacts) -> bool,
) -> ReachFacts {
    let mut walk = Walk::new(events, lowered, index);
    walk.facts = facts;
    // Popped first root first, as a walk of each root alone starts.
    walk.stack.extend(roots.iter().rev().copied());
    walk.run(stop);
    walk.out
}

struct Walk<'a> {
    events: &'a al_vm::events::Registry,
    lowered: &'a Lowered,
    index: &'a ReachIndex,
    out: ReachFacts,
    /// The routines visited, one bit per routine at [`ReachIndex::first_routine`] of its object plus its number.
    seen: Vec<u64>,
    /// Per object, what is already pushed: [`EVENTS`], [`RUN`] and the bits of [`ops`].
    entered: Vec<u8>,
    /// What [`Walk::seen`] and [`Walk::entered`] hold for objects the index does not know of (none: the program
    /// does not change under an engine).
    seen_beyond: BTreeSet<(usize, usize)>,
    entered_beyond: BTreeSet<(usize, u8)>,
    stack: Vec<(usize, usize)>,
    /// The subscribers of the `Global Triggers` database events are pushed, or the walk leaves them out
    /// ([`union_reach_facts_without_database_events`]).
    global_triggers: bool,
    /// Whether the walk keeps the tables, globals and state it reaches ([`ReachFacts`]) besides the reasons.
    facts: bool,
    /// Whether the walk follows calls only ([`call_reach_facts`]).
    calls_only: bool,
    /// Whether a walk of calls only also enters the routines of the objects run ([`union_call_reach_facts`]).
    runs_in_calls: bool,
    /// Whether a walk of calls only also enters the table triggers, without the table's events, and the subscribers in
    /// apps under test of the events raised ([`login_subscriber_reach_facts`]).
    login: bool,
    /// The routines visited, when kept ([`login_subscribers_reach_without`]).
    visited: Option<Vec<(usize, usize)>>,
    /// Every reason of `unknown`, when kept ([`union_reach_visited`]).
    unknowns: Option<BTreeSet<(Unknown, String)>>,
}

impl<'a> Walk<'a> {
    fn new(events: &'a al_vm::events::Registry, lowered: &'a Lowered, index: &'a ReachIndex) -> Walk<'a> {
        let routines = index.first_routine.last().copied().unwrap_or(0);
        Walk {
            events,
            lowered,
            index,
            out: ReachFacts::default(),
            seen: vec![0; routines.div_ceil(64)],
            entered: vec![0; index.first_routine.len().saturating_sub(1)],
            seen_beyond: BTreeSet::new(),
            entered_beyond: BTreeSet::new(),
            stack: Vec::new(),
            global_triggers: false,
            facts: true,
            calls_only: false,
            runs_in_calls: false,
            login: false,
            visited: None,
            unknowns: None,
        }
    }

    /// Follows the stack until it is empty or `stop` holds of what the walk found.
    fn run(&mut self, stop: &dyn Fn(&ReachFacts) -> bool) {
        while let Some((o, r)) = self.stack.pop() {
            if stop(&self.out) {
                break;
            }
            if self.first_visit(o, r) {
                if let Some(v) = &mut self.visited {
                    v.push((o, r));
                }
                self.routine(o, r);
            }
        }
    }

    /// Notes routine `r` of `o` as visited; whether it was not before.
    fn first_visit(&mut self, o: usize, r: usize) -> bool {
        let first = &self.index.first_routine;
        match first.get(o) {
            Some(&base) if o + 1 < first.len() && base + r < first[o + 1] => {
                let i = base + r;
                let (word, bit) = (i / 64, 1u64 << (i % 64));
                let new = self.seen[word] & bit == 0;
                self.seen[word] |= bit;
                new
            }
            _ => self.seen_beyond.insert((o, r)),
        }
    }

    /// Table `t` is in the reach.
    fn touch(&mut self, t: u32) {
        if self.facts && self.enter(t as usize, TOUCHED) {
            self.out.touched.insert(t);
        }
    }

    /// Notes `what` ([`EVENTS`], [`RUN`] or a bit of [`ops`]) of object `o` as entered; whether it was not before.
    fn enter(&mut self, o: usize, what: u8) -> bool {
        match self.entered.get_mut(o) {
            Some(bits) => {
                let new = *bits & what == 0;
                *bits |= what;
                new
            }
            None => self.entered_beyond.insert((o, what)),
        }
    }

    /// Part of the reach is not known: a refused routine or construct (owner's priority, coordinator's decision
    /// 2026-10-19: it counts as "may change anything"), or a run by an id known only at run time.
    fn unknown(&mut self, kind: Unknown, why: impl FnOnce() -> String) {
        if self.out.unknown.is_none() || self.unknowns.is_some() {
            let why: String = why().chars().take(160).collect();
            self.note_unknown(kind, &why);
        }
    }

    /// Notes a reason of `unknown` of kind `kind`: the walk's first, and every one when they are kept.
    fn note_unknown(&mut self, kind: Unknown, why: &str) {
        if self.out.unknown.is_none() {
            self.out.unknown = Some(why.to_string());
        }
        if let Some(all) = &mut self.unknowns {
            all.insert((kind, why.to_string()));
        }
    }

    /// The subscribers of the events of object `o` and of its extensions (a table's events fire whether or not its
    /// triggers run).
    fn events(&mut self, o: usize) {
        if !self.enter(o, EVENTS) {
            return;
        }
        if let Some(subs) = self.index.subscribers.get(&o) {
            self.stack.extend(subs.iter().copied());
        }
        for e in self.extensions_of(o) {
            self.events(e);
        }
    }

    fn extensions_of(&self, o: usize) -> Vec<usize> {
        let obj = &self.lowered.program.objects[o];
        obj.id
            .and_then(|id| self.index.extensions.get(&(obj.kind, id)))
            .cloned()
            .unwrap_or_default()
    }

    /// The triggers of table `t` and of its extensions that the operations `ops` (bits of [`ops`]) run, and the
    /// subscribers of the table's events.
    fn table_ops(&mut self, t: usize, ops: u8) {
        self.table_triggers(t, ops);
        self.events(t);
    }

    /// The triggers of table `t` and of its extensions that the operations `ops` (bits of [`ops`]) run.
    fn table_triggers(&mut self, t: usize, ops: u8) {
        let lowered = self.lowered;
        let objs = &lowered.program.objects;
        for bit in [ops::INSERT, ops::MODIFY, ops::DELETE, ops::RENAME, ops::VALIDATE] {
            if ops & bit == 0 || !self.enter(t, bit) {
                continue;
            }
            for o in std::iter::once(t).chain(self.extensions_of(t)) {
                let obj = &objs[o];
                if obj.uncompiled {
                    self.unknown(Unknown::NotCompiled, || {
                        format!("the triggers of {} are not compiled", obj.name)
                    });
                    continue;
                }
                let word = match bit {
                    ops::INSERT => "Insert",
                    ops::MODIFY => "Modify",
                    ops::DELETE => "Delete",
                    ops::RENAME => "Rename",
                    _ => "Validate",
                };
                self.stack.extend(
                    obj.routines
                        .iter()
                        .enumerate()
                        .filter(|(_, rt)| rt.is_trigger && rt.name.contains(word))
                        .map(|(r, _)| (o, r)),
                );
            }
        }
    }

    /// What running object `o` reaches: a codeunit's `OnRun`, every routine of a page (and its source table's
    /// triggers), the triggers of a report or xmlport, with their extensions and the subscribers of their events.
    fn run_object(&mut self, o: usize) {
        if !self.enter(o, RUN) {
            return;
        }
        let lowered = self.lowered;
        let obj = &lowered.program.objects[o];
        if obj.uncompiled {
            self.unknown(Unknown::NotCompiled, || {
                format!("runs {}, which is not compiled", obj.name)
            });
            return;
        }
        // A report the runtime refuses when it runs (`Session::report_schema`): what the test does before it is not followed.
        if let Some(why) = obj.report.as_ref().and_then(|r| r.unsupported.first()) {
            self.unknown(Unknown::Other, || format!("runs {}, which cannot run: {why}", obj.name));
            if self.out.report_cannot_run.is_none() {
                self.out.report_cannot_run = Some(format!("runs {}, which cannot run: {why}", obj.name));
            }
        }
        for x in std::iter::once(o).chain(self.extensions_of(o)) {
            let xo = &lowered.program.objects[x];
            self.stack.extend(
                xo.routines
                    .iter()
                    .enumerate()
                    .filter(|(_, rt)| match obj.kind {
                        ObjKind::Codeunit => rt.is_trigger && rt.name.eq_ignore_ascii_case("OnRun"),
                        ObjKind::Page => true,
                        _ => rt.is_trigger || rt.name.starts_with("__"),
                    })
                    .map(|(r, _)| (x, r)),
            );
        }
        // A walk of calls only enters the routines of the object, not its table's triggers nor its event subscribers.
        if obj.kind == ObjKind::Page && !self.calls_only {
            if let Some(RtType::Record(t, _)) = obj.rec_slot.map(|i| &obj.globals[i].ty) {
                self.table_ops(*t as usize, ops::ALL);
            }
        }
        if let Some(query) = &obj.query {
            for i in &query.items {
                self.touch(i.table as u32);
            }
        }
        if !self.calls_only {
            self.events(o);
        }
    }

    fn note_table_write(&mut self, table: Option<u32>, insert: bool) {
        match table {
            _ if !self.facts => {}
            Some(t) => {
                self.out.writes.insert(t);
                if insert {
                    self.out.inserts.insert(t);
                }
            }
            None => self.out.writes_any = true,
        }
        // The `Global Triggers` database events fire on writes.
        if !self.global_triggers && !self.calls_only {
            self.global_triggers = true;
            self.stack.extend(self.index.global_triggers.iter().copied());
        }
    }

    /// Routine `r` of `o`, from its [`Summary`] (recorded at its first visit by any walk on this index): its own facts,
    /// then its steps in the order the body takes them, so the walk pushes and enters exactly what a walk of the body
    /// would.
    fn routine(&mut self, o: usize, r: usize) {
        // A walk of calls only does not raise events: a publisher's body is empty, its steps are its subscribers. The walk
        // of a login subscriber enters those in apps under test of an event published in an app under test (an event of
        // Microsoft's apps reaches telemetry subscribers of the apps under test too, corpus Y64InstallSubscribers.al).
        if self.calls_only {
            if let Some(p) = self.events.publishers.get(&(o, r)) {
                let lowered = self.lowered;
                if self.login && !lowered.dependency.get(o).copied().unwrap_or(true) {
                    let subs = self
                        .events
                        .subscribers
                        .get(&(p.object, p.event.clone()))
                        .into_iter()
                        .flatten();
                    self.stack.extend(
                        subs.filter(|s| !s.manual && !lowered.dependency.get(s.object).copied().unwrap_or(true))
                            .map(|s| (s.object, s.routine)),
                    );
                }
                return;
            }
        }
        let index = self.index;
        let fresh;
        let s = match index.summary(self.events, self.lowered, o, r) {
            Some(s) => s,
            None => {
                fresh = Rec::record(self.events, self.lowered, index, o, r);
                &fresh
            }
        };
        self.apply(s);
    }

    /// The facts and steps of a [`Summary`].
    fn apply(&mut self, s: &Summary) {
        if self.facts {
            for &t in &s.touched {
                self.touch(t);
            }
            self.out.globals_read.extend(s.globals_read.iter().copied());
            self.out.globals_written.extend(s.globals_written.iter().copied());
            self.out.state_reads |= s.state_reads;
            self.out.state_writes |= s.state_writes;
        }
        for step in &s.steps {
            match step {
                Step::Push(a, b) => self.stack.extend_from_slice(&s.pushes[*a as usize..*b as usize]),
                Step::Unknown(why, kind) => self.note_unknown(*kind, why),
                Step::Run(x) if self.calls_only && self.runs_in_calls => self.run_object(*x),
                Step::TableOps(t, ops) if self.calls_only && self.login => self.table_triggers(*t, *ops),
                Step::Events(_) | Step::TableOps(..) | Step::Run(_) if self.calls_only => {}
                Step::Events(t) => self.events(*t),
                Step::TableOps(t, ops) => self.table_ops(*t, *ops),
                Step::Run(x) => self.run_object(*x),
                Step::Write(t, insert) => self.note_table_write(*t, *insert),
            }
        }
    }
}

/// What routine `r` of `o` adds to every walk that visits it, recorded once per routine (the body is the costly part
/// of a walk: tests of a codeunit share most of their reach). The facts its body names hold whatever the walk entered
/// before; the steps whose effect depends on that (pushing the triggers of a table, the subscribers of an object, the
/// routines of an object run) are kept in body order and taken again by each walk.
#[derive(Default)]
pub(crate) struct Summary {
    touched: Vec<u32>,
    globals_read: Vec<(usize, usize)>,
    globals_written: Vec<(usize, usize)>,
    state_reads: u16,
    state_writes: u16,
    steps: Vec<Step>,
    /// The routines [`Step::Push`] pushes, by range.
    pushes: Vec<(usize, usize)>,
}

#[derive(PartialEq)]
enum Step {
    /// Push `pushes[a..b]`.
    Push(u32, u32),
    /// Part of the reach is not known, for this reason (cut to 160 characters), of this kind. Only the routine's first of
    /// each kind is kept: once it is taken the walk's reason is set.
    Unknown(Box<str>, Unknown),
    /// [`Walk::events`].
    Events(usize),
    /// [`Walk::table_ops`].
    TableOps(usize, u8),
    /// [`Walk::run_object`].
    Run(usize),
    /// [`Walk::note_table_write`]: the table, and whether the write can add a row.
    Write(Option<u32>, bool),
}

/// Records a routine's [`Summary`]: the analysis of its body, with the steps that depend on the walk deferred.
struct Rec<'a> {
    events: &'a al_vm::events::Registry,
    lowered: &'a Lowered,
    index: &'a ReachIndex,
    /// The facts the body names; only the fields [`Summary`] keeps are set.
    out: ReachFacts,
    summary: Summary,
    /// A reason of `unknown` was recorded: of kind [`Unknown::NotCompiled`], of kind [`Unknown::Other`].
    unknown: [bool; 2],
}

impl Rec<'_> {
    fn record(events: &al_vm::events::Registry, lowered: &Lowered, index: &ReachIndex, o: usize, r: usize) -> Summary {
        let mut rec = Rec {
            events,
            lowered,
            index,
            out: ReachFacts::default(),
            summary: Summary::default(),
            unknown: [false; 2],
        };
        rec.routine(o, r);
        rec.finish()
    }

    /// The [`Summary`] of what was recorded.
    fn finish(self) -> Summary {
        let Rec { out, mut summary, .. } = self;
        summary.touched = out.touched.into_iter().collect();
        summary.globals_read = out.globals_read.into_iter().collect();
        summary.globals_written = out.globals_written.into_iter().collect();
        summary.state_reads = out.state_reads;
        summary.state_writes = out.state_writes;
        summary.steps.shrink_to_fit();
        summary.pushes.shrink_to_fit();
        summary
    }

    /// A step that depends on the walk. One the routine already took is left out: what a walk entered stays entered,
    /// so taking it again changes nothing (a routine names the tables of its object's globals once per slot).
    fn step(&mut self, step: Step) {
        if matches!(step, Step::Push(..) | Step::Unknown(..)) || !self.summary.steps.contains(&step) {
            self.summary.steps.push(step);
        }
    }

    fn push(&mut self, routines: impl IntoIterator<Item = (usize, usize)>) {
        let a = self.summary.pushes.len() as u32;
        self.summary.pushes.extend(routines);
        let b = self.summary.pushes.len() as u32;
        if a == b {
            return;
        }
        match self.summary.steps.last_mut() {
            Some(Step::Push(_, end)) if *end == a => *end = b,
            _ => self.step(Step::Push(a, b)),
        }
    }

    fn unknown(&mut self, why: impl FnOnce() -> String) {
        self.unknown_of(Unknown::Other, why);
    }

    fn unknown_of(&mut self, kind: Unknown, why: impl FnOnce() -> String) {
        let seen = &mut self.unknown[kind as usize];
        if !*seen {
            *seen = true;
            self.step(Step::Unknown(why().chars().take(160).collect::<String>().into(), kind));
        }
    }

    fn events(&mut self, o: usize) {
        self.step(Step::Events(o));
    }

    fn table_ops(&mut self, t: usize, ops: u8) {
        self.step(Step::TableOps(t, ops));
    }

    fn run_object(&mut self, o: usize) {
        self.step(Step::Run(o));
    }

    fn note_table_write(&mut self, table: Option<u32>) {
        self.step(Step::Write(table, false));
    }

    /// A write that can add a row to `table` ([`ReachFacts::inserts`]).
    fn note_table_insert(&mut self, table: u32) {
        self.step(Step::Write(Some(table), true));
    }

    /// The object of `kind` with id `id`, run; unknown when there is none.
    fn enter_by_id(&mut self, kind: ObjKind, id: i32) {
        match u32::try_from(id).ok().and_then(|id| self.index.by_id.get(&(kind, id))) {
            Some(&o) => self.run_object(o),
            None => self.unknown(|| format!("runs {kind} {id}, which is not in the program")),
        }
    }

    fn routine(&mut self, o: usize, r: usize) {
        let lowered = self.lowered;
        let objs = &lowered.program.objects;
        let obj = &objs[o];
        let rt = &obj.routines[r];
        // The routine that writes an expression-bound page control is optional (`blocked_site`).
        if !rt.name.starts_with("__sourceset_") {
            if let Some(reason) = lowered.unsupported[o][r].first() {
                let kind = if al_sema::check::is_not_compiled(reason.reason()) {
                    Unknown::NotCompiled
                } else {
                    Unknown::Other
                };
                self.unknown_of(kind, || format!("{}.{}: {reason}", obj.name, rt.name));
            }
        }
        // The event this routine publishes, when it is a publisher.
        if let Some(p) = self.events.publishers.get(&(o, r)) {
            for sub in self
                .events
                .subscribers
                .get(&(p.object, p.event.clone()))
                .into_iter()
                .flatten()
            {
                self.push([(sub.object, sub.routine)]);
            }
        }
        for s in rt.locals.iter().chain(&obj.globals) {
            self.slot(&s.ty);
        }
        self.places(o, &rt.body);
        al_ir::types::for_each_expr_post(&rt.body, &mut |e| self.expr(o, r, e));
        let mut calls = Vec::new();
        lowered.calls_into(o, r, &mut calls);
        self.push(calls);
    }

    /// What a variable of type `ty` reaches by being there: the events of its table, what a page client does to the
    /// page's source table, the run of a query, an xmlport's import.
    fn slot(&mut self, ty: &RtType) {
        let objs = &self.lowered.program.objects;
        let mut ty = ty;
        while let RtType::Array(_, inner) = ty {
            ty = inner;
        }
        match ty {
            RtType::Record(t, _) => {
                self.out.touched.insert(*t);
                self.events(*t as usize);
            }
            // A page writes its source table; the table is the record of the page's `Rec` slot.
            RtType::Page(p) | RtType::TestPage(p) => {
                let page = &objs[*p as usize];
                match page.rec_slot.map(|i| &page.globals[i].ty) {
                    Some(RtType::Record(t, temporary)) => {
                        self.out.touched.insert(*t);
                        // A page over a temporary source adds no row to the table.
                        if *temporary {
                            self.note_table_write(Some(*t));
                        } else {
                            self.note_table_insert(*t);
                        }
                        // The page client inserts, modifies, deletes and validates with the triggers.
                        self.table_ops(*t as usize, ops::ALL);
                    }
                    _ => self.note_table_write(None),
                }
                self.events(*p as usize);
            }
            RtType::Query(q) => {
                self.run_object(*q as usize);
            }
            // A codeunit, report or xmlport variable: what it runs is in the calls (`Lowered::calls_into`). An
            // xmlport imports into its tables without a record operation in AL.
            RtType::Codeunit(c) if objs[*c as usize].kind == ObjKind::XmlPort => self.note_table_write(None),
            RtType::DotNet(_) => self.out.state_writes |= state::OTHER,
            _ => {}
        }
    }

    /// The global of a codeunit a place names, as (object, slot).
    fn global_of(&self, o: usize, p: &Place) -> Option<(usize, usize)> {
        let objs = &self.lowered.program.objects;
        let mut p = p;
        loop {
            match p {
                Place::Index(b, _) | Place::Field(b, _) | Place::RecGlobal(b, ..) => p = b,
                Place::Global(g) => {
                    return (objs[o].kind == ObjKind::Codeunit).then_some((o, *g));
                }
                // A global of the base page or report: it lives with the page or report instance.
                Place::BaseGlobal(_) | Place::ExtGlobal(..) | Place::Local(_) => return None,
            }
        }
    }

    fn write_place(&mut self, o: usize, p: &Place) {
        if let Some(g) = self.global_of(o, p) {
            self.out.globals_written.insert(g);
            self.out.globals_read.insert(g);
        }
    }

    /// The places statements assign (`:=`, the variable of a `for` or `foreach`).
    fn places(&mut self, o: usize, body: &[Stmt]) {
        for s in body {
            match s {
                Stmt::Assign(p, _) => self.write_place(o, p),
                Stmt::For { var, body, .. } | Stmt::Foreach { var, body, .. } => {
                    self.write_place(o, var);
                    self.places(o, body);
                }
                Stmt::If(_, a, b) => {
                    self.places(o, a);
                    self.places(o, b);
                }
                Stmt::While(_, b) | Stmt::Repeat(b, _) | Stmt::AssertError(b) => self.places(o, b),
                Stmt::Case {
                    branches, otherwise, ..
                } => {
                    for (_, b) in branches {
                        self.places(o, b);
                    }
                    self.places(o, otherwise);
                }
                Stmt::Expr(_)
                | Stmt::Exit(_)
                | Stmt::Break
                | Stmt::Pos(_)
                | Stmt::Assume(_)
                | Stmt::TddMark(_)
                | Stmt::Continue => {}
            }
        }
    }

    fn arg_writes(&mut self, o: usize, args: &[Arg]) {
        for a in args {
            if let Arg::Ref(p) = a {
                self.write_place(o, p);
            }
        }
    }

    fn expr(&mut self, o: usize, r: usize, e: &Expr) {
        let lowered = self.lowered;
        let objs = &lowered.program.objects;
        match e {
            Expr::Load(p) => {
                if let Some(g) = self.global_of(o, p) {
                    self.out.globals_read.insert(g);
                }
            }
            Expr::Call { args, instance, .. } => {
                self.arg_writes(o, args);
                // A procedure of a table, page or report changes the variable it is called on (its fields, the globals
                // of its object). A codeunit variable only refers to its instance, whose globals the callee's own
                // reach names.
                if let Some(Expr::Load(p)) = instance.as_deref() {
                    let codeunit = match self.slot_ty(o, r, p) {
                        Some(RtType::Codeunit(c)) => objs[c as usize].kind == ObjKind::Codeunit,
                        Some(RtType::Interface(_)) => true,
                        _ => false,
                    };
                    if !codeunit {
                        self.write_place(o, p);
                    }
                }
            }
            Expr::InterfaceCall { args, .. } => self.arg_writes(o, args),
            // A `[RunOnClient]` variable's bookkeeping and a call on it write nothing on the service tier: the call runs on
            // the client, which a test session has not, so it is refused when it runs, and the variable never holds an
            // object (onprem-dotnet/runonclient/BgRunOnClient.al:122-150, :186-209, :282-302, :395-483). Only the arguments
            // can be written (Office Host Provider.OnGetHostType, al-runner issue 6924).
            Expr::DotNet {
                op: al_ir::DotNetOp::ClientNull | al_ir::DotNetOp::ClientAssign | al_ir::DotNetOp::ClientCall(_),
                args,
                ..
            } => self.arg_writes(o, args),
            Expr::DotNet { args, .. } => {
                self.arg_writes(o, args);
                self.out.state_writes |= state::OTHER;
            }
            Expr::Refused(why) => {
                let at = format!("{}.{}", objs[o].name, objs[o].routines[r].name);
                self.unknown(|| format!("{at}: {why}"));
            }
            Expr::BaseInstance => {}
            Expr::Builtin { id, receiver, args, .. } => {
                self.arg_writes(o, args);
                // A method call on a variable may change it (a record's filters, a list's items).
                let receiver_place = receiver.as_deref().and_then(|a| match a {
                    Arg::Ref(p) | Arg::Value(Expr::Load(p)) => Some(p),
                    _ => None,
                });
                if let Some(p) = receiver_place {
                    self.write_place(o, p);
                }
                self.builtin(o, r, *id, receiver_place, args);
            }
            _ => {}
        }
    }

    /// The type of the slot a place names, arrays looked through.
    fn slot_ty(&self, o: usize, r: usize, p: &Place) -> Option<RtType> {
        let obj = &self.lowered.program.objects[o];
        let mut p = p;
        while let Place::Index(b, _) = p {
            p = b;
        }
        let mut ty = match p {
            Place::Local(i) => &obj.routines[r].locals.get(*i)?.ty,
            Place::Global(i) => &obj.globals.get(*i)?.ty,
            _ => return None,
        };
        while let RtType::Array(_, inner) = ty {
            ty = inner;
        }
        Some(ty.clone())
    }

    fn builtin(&mut self, o: usize, r: usize, id: al_ir::BuiltinId, receiver: Option<&Place>, args: &[Arg]) {
        use state::*;
        let name = id.name();
        let method = id.method();
        let kind = name.split('.').next().unwrap_or("");
        let const_id = || match args.first() {
            Some(Arg::Value(Expr::Const(Const::Integer(n)))) => Some(*n),
            _ => None,
        };
        let has_args = !args.is_empty();
        match kind {
            "Record" => {
                let op = ops::of(method);
                if op != 0 {
                    let (table, temporary) = match receiver.and_then(|p| self.slot_ty(o, r, p)) {
                        Some(RtType::Record(t, temporary)) => (Some(t), temporary),
                        _ => (None, false),
                    };
                    match table {
                        // An Insert on a record declared temporary adds no row to the table.
                        Some(t) if op == ops::INSERT && !temporary => self.note_table_insert(t),
                        _ if op != ops::VALIDATE => self.note_table_write(table),
                        _ => {}
                    }
                    if ops::runs_triggers(method, args) {
                        match table {
                            Some(t) => self.table_ops(t as usize, op),
                            None => self.unknown(|| format!("{name} on a record whose table is not known")),
                        }
                    }
                }
            }
            "RecordRef" | "FieldRef" => {
                let op = ops::of(method);
                if op != 0 && op != ops::VALIDATE {
                    self.note_table_write(None);
                }
                if op != 0 && ops::runs_triggers(method, args) {
                    self.unknown(|| format!("{name} runs the triggers of a table that is not known"));
                }
            }
            "Media" | "MediaSet" => {
                if method.starts_with("Import") || matches!(method, "Insert" | "Remove") {
                    self.note_table_write(None);
                }
            }
            "DataTransfer" => self.note_table_write(None),
            "NavApp" => {
                if matches!(method, "DeleteArchiveData" | "RestoreArchiveData" | "LoadPackageData") {
                    self.note_table_write(None);
                }
            }
            // Run by id: the object of that id when the id is a constant.
            "Codeunit" | "Report" | "Page" | "Xmlport" | "Query"
                if receiver.is_none()
                    && (method.starts_with("Run")
                        || method.starts_with("SaveAs")
                        || matches!(method, "Execute" | "Print" | "Import" | "Export" | "Open")) =>
            {
                let object_kind = match kind {
                    "Codeunit" => ObjKind::Codeunit,
                    "Report" => ObjKind::Report,
                    "Page" => ObjKind::Page,
                    "Xmlport" => ObjKind::XmlPort,
                    _ => ObjKind::Query,
                };
                match const_id() {
                    Some(n) => self.enter_by_id(object_kind, n),
                    // `Report.SaveAs` and its kind take the id too; a method without an object id runs nothing.
                    None if has_args => self.unknown(|| format!("{name} with an object id known only at run time")),
                    None => {}
                }
                if kind == "Xmlport" {
                    self.note_table_write(None);
                }
                if kind == "Report" && method.starts_with("SaveAs") {
                    self.out.state_writes |= FILES;
                    self.out.state_reads |= FILES;
                }
            }
            "Codeunit" => {
                if let Some(RtType::Codeunit(c)) = receiver.and_then(|p| self.slot_ty(o, r, p)) {
                    self.run_object(c as usize);
                }
            }
            "Report" | "Xmlport" => {
                if kind == "Xmlport" {
                    self.note_table_write(None);
                }
                if method.starts_with("SaveAs") {
                    self.out.state_writes |= FILES;
                    self.out.state_reads |= FILES;
                }
            }
            "System" => match method {
                "Randomize" if has_args => self.out.state_writes |= RANDOM | RANDOM_SEED,
                "Random" | "Randomize" => self.out.state_writes |= RANDOM,
                "WorkDate" if has_args => self.out.state_writes |= WORK_DATE,
                "GlobalLanguage" if has_args => self.out.state_writes |= LANGUAGE,
                // Clears every global of the object.
                "ClearAll" => {
                    let obj = &self.lowered.program.objects[o];
                    if obj.kind == ObjKind::Codeunit {
                        let n = obj.globals.len();
                        self.out.globals_written.extend((0..n).map(|g| (o, g)));
                        self.out.globals_read.extend((0..n).map(|g| (o, g)));
                    }
                }
                "CreateEncryptionKey"
                | "DeleteEncryptionKey"
                | "ImportEncryptionKey"
                | "ImportObjects"
                | "CodeCoverageLoad"
                | "CodeCoverageLog"
                | "CodeCoverageInclude"
                | "CodeCoverageRefresh" => self.out.state_writes |= OTHER,
                _ => {}
            },
            "Session" => match method {
                "BindSubscription" | "UnbindSubscription" => self.out.state_writes |= BINDING,
                // Under Disabled isolation the session runs its codeunit to its end inside the call (al-vm bgsession.rs):
                // what that can change is what a run of the codeunit can (a superset: its own session state is not the
                // caller's). Elsewhere the call raises or is refused and changes nothing.
                "StartSession" => match args.get(1) {
                    Some(Arg::Value(Expr::Const(Const::Integer(n)))) => self.enter_by_id(ObjKind::Codeunit, *n),
                    _ => self.unknown(|| format!("{name} with a codeunit id known only at run time")),
                },
                "SetDocumentServiceToken" => self.out.state_writes |= OTHER,
                _ => {}
            },
            "SessionSettings" if method == "RequestSessionUpdate" => self.out.state_writes |= LANGUAGE,
            "NumberSequence" => {
                self.out.state_reads |= NUMBER_SEQUENCE;
                if !matches!(method, "Exists" | "Current") {
                    self.out.state_writes |= NUMBER_SEQUENCE;
                }
            }
            "IsolatedStorage" => {
                self.out.state_reads |= ISOLATED_STORAGE;
                if !matches!(method, "Get" | "Contains") {
                    self.out.state_writes |= ISOLATED_STORAGE;
                }
            }
            "File" | "FileUpload" => {
                self.out.state_reads |= FILES;
                self.out.state_writes |= FILES;
            }
            "Database" => {
                if matches!(
                    method,
                    "ChangeUserPassword"
                        | "SetUserPassword"
                        | "CopyCompany"
                        | "ImportData"
                        | "ExportData"
                        | "AlterKey"
                        | "RegisterTableConnection"
                        | "UnregisterTableConnection"
                        | "SetDefaultTableConnection"
                ) || method == "LockTimeout" && has_args
                {
                    self.out.state_writes |= OTHER;
                }
            }
            "TaskScheduler" => self.out.state_writes |= OTHER,
            "HttpClient" => {
                self.out.state_reads |= HTTP;
                self.out.state_writes |= HTTP;
            }
            _ => {}
        }
    }
}

/// Boolean globals of codeunit `object` that every statement of the codeunit that changes them sets to the same
/// constant (`IsInitialized := true`), with that constant. Once such a global holds its constant, every earlier
/// test that ran in BC left it so too: the code before the point where a test here stopped is what BC ran, and BC
/// can only have set it to the same value after that point. A global that is cleared, handed to a `var` parameter,
/// used as the receiver of a method or set to anything else is not one.
pub(crate) fn settled_flags(lowered: &Lowered, object: usize) -> Vec<(usize, bool)> {
    let obj = &lowered.program.objects[object];
    // Per global: Some(value) while every write seen sets that constant; None once another write is seen.
    let mut seen: HashMap<usize, Option<bool>> = HashMap::new();
    let spoil = |seen: &mut HashMap<usize, Option<bool>>, p: &Place| {
        let mut p = p;
        while let Place::Index(b, _) | Place::Field(b, _) = p {
            p = b;
        }
        if let Place::Global(g) = p {
            seen.insert(*g, None);
        }
    };
    fn stmts(body: &[Stmt], f: &mut dyn FnMut(&Stmt)) {
        for s in body {
            f(s);
            match s {
                Stmt::If(_, a, b) => {
                    stmts(a, f);
                    stmts(b, f);
                }
                Stmt::While(_, b)
                | Stmt::Repeat(b, _)
                | Stmt::AssertError(b)
                | Stmt::For { body: b, .. }
                | Stmt::Foreach { body: b, .. } => stmts(b, f),
                Stmt::Case {
                    branches, otherwise, ..
                } => {
                    for (_, b) in branches {
                        stmts(b, f);
                    }
                    stmts(otherwise, f);
                }
                _ => {}
            }
        }
    }
    let mut clear_all = false;
    for rt in &obj.routines {
        stmts(&rt.body, &mut |s| match s {
            Stmt::Assign(Place::Global(g), Expr::Const(Const::Boolean(v))) => {
                let e = seen.entry(*g).or_insert(Some(*v));
                if *e != Some(*v) {
                    *e = None;
                }
            }
            Stmt::Assign(p, _) | Stmt::For { var: p, .. } | Stmt::Foreach { var: p, .. } => spoil(&mut seen, p),
            _ => {}
        });
        al_ir::types::for_each_expr_post(&rt.body, &mut |e| {
            let (args, receiver) = match e {
                Expr::Call { args, .. } | Expr::InterfaceCall { args, .. } | Expr::DotNet { args, .. } => (args, None),
                Expr::Builtin { id, args, receiver, .. } => {
                    if id.name().starts_with("System.ClearAll") {
                        clear_all = true;
                    }
                    (args, receiver.as_deref())
                }
                _ => return,
            };
            for a in args.iter().chain(receiver) {
                match a {
                    Arg::Ref(p) | Arg::Value(Expr::Load(p)) if receiver.is_some_and(|r| std::ptr::eq(r, a)) => {
                        spoil(&mut seen, p)
                    }
                    Arg::Ref(p) => spoil(&mut seen, p),
                    _ => {}
                }
            }
        });
    }
    if clear_all {
        return Vec::new();
    }
    seen.into_iter()
        .filter_map(|(g, v)| Some((g, v?)))
        .filter(|(g, _)| matches!(obj.globals.get(*g).map(|s| &s.ty), Some(RtType::Boolean)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lowered(src: &str) -> Lowered {
        al_sema::lower::compile(
            vec![al_sema::workspace::AppSource {
                id: "a".into(),
                name: "a".into(),
                dependencies: vec![],
                files: vec![("f.al".into(), src.into())],
            }],
            &Default::default(),
        )
        .1
    }

    fn routine(l: &Lowered, object: &str, name: &str) -> (usize, usize) {
        let o = l.program.objects.heads().iter().position(|o| o.name == object).unwrap();
        let r = l.program.objects[o]
            .routines
            .iter()
            .position(|r| r.name == name)
            .unwrap();
        (o, r)
    }

    const SRC: &str = "
        table 50100 T { fields { field(1; K; Integer) { } }
            trigger OnInsert() var F: Codeunit Flag; begin F.SetIt(); end; }
        codeunit 50101 Flag { SingleInstance = true; var Hit: Boolean;
            procedure SetIt() begin Hit := true; end; }
        codeunit 50102 Helper {
            procedure H() var R: Record T; begin R.Insert(true); end; }
        codeunit 50103 Tests { Subtype = Test;
            [Test] procedure A() var R: Record T; Hp: Codeunit Helper; begin R.Insert(true); Hp.H(); end;
            [Test] procedure B() var Hp: Codeunit Helper; begin Hp.H(); end; }";

    // A routine's summary is recorded once and taken again by later walks; what depends on the walk (here the insert
    // trigger, which the first walk had already entered before it reached the helper) is still entered by each walk.
    #[test]
    fn a_later_walk_takes_the_recorded_summary_and_still_enters_the_triggers() {
        let l = lowered(SRC);
        let events = al_vm::events::Registry::build(&l.program);
        let (o, a) = routine(&l, "Tests", "A");
        let (_, b) = routine(&l, "Tests", "B");
        let warm = ReachIndex::new(&events, &l);
        let fa = reach_facts(&events, &l, &warm, o, a);
        let recorded = warm.recorded();
        let fb = reach_facts(&events, &l, &warm, o, b);
        // Only B itself is new: the helper, the trigger and SetIt were recorded by A's walk.
        assert_eq!(warm.recorded(), recorded + 1);
        let flag = routine(&l, "Flag", "SetIt").0;
        assert!(fb.globals_written.iter().any(|&(fo, _)| fo == flag), "{fb:?}");
        // The same facts as walks on an index that recorded nothing yet.
        let cold = |r| format!("{:?}", reach_facts(&events, &l, &ReachIndex::new(&events, &l), o, r));
        assert_eq!(format!("{fa:?}"), cold(a));
        assert_eq!(format!("{fb:?}"), cold(b));
        assert_eq!(
            format!("{fb:?}"),
            format!("{:?}", reach_facts(&events, &l, &warm, o, b))
        );
    }
    // The question "does the reach hold a report the runtime refuses" skips the routines an earlier question found clean,
    // and answers as a walk on a fresh index does; a walk that stops at a report marks nothing.
    #[test]
    fn the_report_question_skips_what_an_earlier_walk_found_clean() {
        let src = "
            report 50110 Refused { ProcessingOnly = true; }
            codeunit 50102 Helper { procedure H() begin end; procedure Rep() begin Report.Run(50110); end; }
            codeunit 50103 Tests { Subtype = Test;
                [Test] procedure A() var Hp: Codeunit Helper; begin Hp.H(); end;
                [Test] procedure B() var Hp: Codeunit Helper; begin Hp.H(); end;
                [Test] procedure C() var Hp: Codeunit Helper; begin Hp.Rep(); end; }";
        let mut l = lowered(src);
        let rep = l
            .program
            .objects
            .heads()
            .iter()
            .position(|o| o.name == "Refused")
            .unwrap();
        std::sync::Arc::get_mut(&mut l.program).unwrap().objects[rep]
            .report
            .as_mut()
            .unwrap()
            .unsupported
            .push("probe".into());
        let events = al_vm::events::Registry::build(&l.program);
        let (o, a) = routine(&l, "Tests", "A");
        let (_, b) = routine(&l, "Tests", "B");
        let (_, c) = routine(&l, "Tests", "C");
        let ask = |index: &ReachIndex, r| report_that_cannot_run(&events, &l, index, o, r);
        let fresh = |r| ask(&ReachIndex::new(&events, &l), r);
        let warm = ReachIndex::new(&events, &l);
        assert_eq!(ask(&warm, c), fresh(c));
        assert!(fresh(c).is_some());
        assert_eq!(
            warm.report_clean_routines(),
            0,
            "a walk that found a report marked routines clean"
        );
        assert_eq!(ask(&warm, a), None);
        let clean = warm.report_clean_routines();
        assert!(clean >= 2, "the walk of A marked {clean}");
        assert_eq!(ask(&warm, b), fresh(b));
        // The clean routines do not hide the report C reaches through the helper's other procedure.
        assert_eq!(ask(&warm, c), fresh(c));
        assert!(ask(&warm, c).is_some());
    }

    // One walk of several roots gives what a walk of each root gives, joined: the questions asked of the tests of a
    // codeunit together (`corpus.rs` `tests_take_numbers`, `reaches_session_storage`) get the answer of asking each.
    #[test]
    fn the_union_walk_joins_the_facts_of_each_root() {
        let src = "
            table 50100 T { fields { field(1; K; Integer) { } } }
            table 50104 U { fields { field(1; K; Integer) { } } }
            codeunit 50103 Tests { Subtype = Test;
                [Test] procedure A() var R: Record T; begin R.Insert(); end;
                [Test] procedure B() var S: Record U; N: Integer; begin S.Insert(); N := NumberSequence.Next('X'); end;
                [Test] procedure C() var Id: Integer; begin Codeunit.Run(Id); end; }";
        let l = lowered(src);
        let events = al_vm::events::Registry::build(&l.program);
        let index = ReachIndex::new(&events, &l);
        let (o, a) = routine(&l, "Tests", "A");
        let (_, b) = routine(&l, "Tests", "B");
        let (_, c) = routine(&l, "Tests", "C");
        let has = |roots: &[(usize, usize)], fact: &dyn Fn(&ReachFacts) -> bool| {
            let any = roots
                .iter()
                .any(|&(_, r)| fact(&reach_facts(&events, &l, &index, o, r)));
            assert_eq!(union_reach_has(&events, &l, &index, roots, fact), any);
            any
        };
        let all = [(o, a), (o, b), (o, c)];
        assert!(has(&all, &|f| f.state_writes & state::NUMBER_SEQUENCE != 0));
        assert!(has(&all, &|f| f.unknown.is_some()));
        assert!(!has(&all[..2], &|f| f.unknown.is_some()));
        assert!(!has(&all[..1], &|f| f.state_writes & state::NUMBER_SEQUENCE != 0));
        assert!(!has(&all, &|f| f.writes_any));
    }

    /// Every fact of `a` and `b`, with only whether there is a reason for `unknown` and `report_cannot_run` (which reason a
    /// walk names first depends on the order it visits routines in).
    fn same_facts(a: &ReachFacts, b: &ReachFacts) -> bool {
        a.touched == b.touched
            && a.writes == b.writes
            && a.inserts == b.inserts
            && a.writes_any == b.writes_any
            && a.globals_read == b.globals_read
            && a.globals_written == b.globals_written
            && a.state_reads == b.state_reads
            && a.state_writes == b.state_writes
            && a.unknown.is_some() == b.unknown.is_some()
            && a.report_cannot_run.is_some() == b.report_cannot_run.is_some()
    }

    // The full walk of a test codeunit (`login.rs`) is put together from the walk without the `Global Triggers` database
    // events and the walk of their subscribers, which a caller keeps per engine: the same facts, for roots that write and
    // roots that do not, with and without routines to skip.
    #[test]
    fn the_full_walk_is_the_walk_without_database_events_and_their_subscribers() {
        let src = r#"
            table 50100 T { fields { field(1; K; Integer) { } } }
            table 50104 Logged { fields { field(1; K; Integer) { } }
                trigger OnInsert() var Id: Integer; begin Codeunit.Run(Id); end; }
            table 50105 Seen { fields { field(1; K; Integer) { } } }
            codeunit 50101 Flag { SingleInstance = true; var Hit: Boolean;
                procedure SetIt() begin Hit := true; end; }
            codeunit 50106 Log {
                [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'OnDatabaseInsert', '', false, false)]
                local procedure OnInsert(RecRef: RecordRef)
                var L: Record Logged; S: Record Seen; F: Codeunit Flag;
                begin if S.Get(1) then; L.Insert(true); F.SetIt(); Randomize(1); end; }
            codeunit 50102 Helper {
                procedure H() var R: Record T; begin R.Insert(true); end;
                procedure Read() var R: Record T; begin if R.Get(1) then; end; }
            codeunit 50103 Tests { Subtype = Test;
                [Test] procedure A() var Hp: Codeunit Helper; begin Hp.H(); end;
                [Test] procedure B() var Hp: Codeunit Helper; begin Hp.Read(); end;
                [Test] procedure C() var R: RecordRef; begin R.Open(50100); R.Insert(); end; }"#;
        let l = lowered(src);
        let events = al_vm::events::Registry::build(&l.program);
        let index = ReachIndex::new(&events, &l);
        let (o, a) = routine(&l, "Tests", "A");
        let (_, b) = routine(&l, "Tests", "B");
        let (_, c) = routine(&l, "Tests", "C");
        let log = routine(&l, "Log", "OnInsert");
        let set_it = routine(&l, "Flag", "SetIt");
        let mut subscribed = 0;
        for skip in [vec![], vec![set_it], vec![log]] {
            let database_events = global_triggers_reach_facts(&events, &l, &index, &skip);
            for roots in [vec![(o, a)], vec![(o, b)], vec![(o, c)], vec![(o, a), (o, b), (o, c)]] {
                let full = union_reach_facts_skipping(&events, &l, &index, &roots, &skip);
                let own = union_reach_facts_without_database_events(&events, &l, &index, &roots, &skip);
                let joined = JoinedReach::new(&own, || &database_events);
                let built = joined.facts();
                assert!(
                    same_facts(&full, &built),
                    "{roots:?} skipping {skip:?}:\n{full:?}\n{built:?}"
                );
                // The questions asked of the parts give the answers of the full walk.
                let tables = (full.touched.iter())
                    .chain(&database_events.touched)
                    .chain(&database_events.writes);
                for &t in tables {
                    assert_eq!(
                        joined.touches(t),
                        full.touched.contains(&t),
                        "{roots:?} {skip:?} table {t}"
                    );
                    assert_eq!(
                        joined.writes_table(t),
                        full.writes.contains(&t),
                        "{roots:?} {skip:?} table {t}"
                    );
                }
                for &g in full.globals_read.iter().chain(&database_events.globals_read) {
                    assert_eq!(
                        joined.reads_global(g),
                        full.globals_read.contains(&g),
                        "{roots:?} {skip:?} {g:?}"
                    );
                    let of = full.globals_read.iter().any(|x| x.0 == g.0);
                    assert_eq!(joined.reads_a_global_of(g.0), of, "{roots:?} {skip:?} {g:?}");
                }
                if full.touched.len() > own.touched.len() {
                    subscribed += 1;
                }
            }
        }
        // The subscriber's reach (table Seen) counts for the tests that write, unless it is skipped; B writes nothing.
        assert_eq!(subscribed, 6);
    }
}
