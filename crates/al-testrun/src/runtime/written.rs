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
use std::collections::HashMap;

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
            _ => "session state the analysis does not follow",
        }
    }
}

/// What [`Walk::entered`] notes besides the bits of [`ops`].
const EVENTS: u8 = 1 << 6;
const RUN: u8 = 1 << 7;

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
}

impl ReachFacts {
    /// Whether the reach can draw from or reseed the session's `Random` sequence.
    pub fn random(&self) -> bool {
        self.state_writes & state::RANDOM != 0 || self.unknown.is_some()
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
}

impl ReachIndex {
    pub fn new(events: &al_vm::events::Registry, lowered: &Lowered) -> ReachIndex {
        let objs = &lowered.program.objects;
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
        ReachIndex {
            by_id,
            extensions,
            subscribers,
            global_triggers,
        }
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
    walk_reach(events, lowered, index, object, routine, false)
}

/// The reach facts of an unfinished test (the earlier side of [`ReachFacts::changes_read_by`]): the walk stops at
/// the first part of the reach that is not known, because that alone decides both questions asked of it
/// (`changes_read_by` and `random` answer from `unknown` before anything else). The rest of the facts are then
/// incomplete and must not be read. A walk of the whole reach visits about 49 000 routines with Microsoft's apps
/// (150 ms), once per unfinished test of a codeunit.
pub(crate) fn unfinished_reach_facts(
    events: &al_vm::events::Registry,
    lowered: &Lowered,
    index: &ReachIndex,
    object: usize,
    routine: usize,
) -> ReachFacts {
    walk_reach(events, lowered, index, object, routine, true)
}

fn walk_reach(
    events: &al_vm::events::Registry,
    lowered: &Lowered,
    index: &ReachIndex,
    object: usize,
    routine: usize,
    stop_at_unknown: bool,
) -> ReachFacts {
    let mut walk = Walk {
        events,
        lowered,
        index,
        out: ReachFacts::default(),
        seen: BTreeSet::new(),
        entered: BTreeSet::new(),
        stack: vec![(object, routine)],
        global_triggers: false,
    };
    while let Some((o, r)) = walk.stack.pop() {
        if stop_at_unknown && walk.out.unknown.is_some() {
            break;
        }
        if walk.seen.insert((o, r)) {
            walk.routine(o, r);
        }
    }
    walk.out
}

struct Walk<'a> {
    events: &'a al_vm::events::Registry,
    lowered: &'a Lowered,
    index: &'a ReachIndex,
    out: ReachFacts,
    seen: BTreeSet<(usize, usize)>,
    /// (object, what) already pushed: [`EVENTS`], [`RUN`] or a bit of [`ops`].
    entered: BTreeSet<(usize, u8)>,
    stack: Vec<(usize, usize)>,
    global_triggers: bool,
}

impl Walk<'_> {
    /// Part of the reach is not known: a refused routine or construct (owner's priority, coordinator's decision
    /// 2026-10-19: it counts as "may change anything"), or a run by an id known only at run time.
    fn unknown(&mut self, why: impl FnOnce() -> String) {
        if self.out.unknown.is_none() {
            self.out.unknown = Some(why().chars().take(160).collect());
        }
    }

    /// The subscribers of the events of object `o` and of its extensions (a table's events fire whether or not its
    /// triggers run).
    fn events(&mut self, o: usize) {
        if !self.entered.insert((o, EVENTS)) {
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

    /// The triggers of table `t` and of its extensions that the operations `ops` (bits of [`ops`]) run.
    fn table_ops(&mut self, t: usize, ops: u8) {
        let lowered = self.lowered;
        let objs = &lowered.program.objects;
        for bit in [ops::INSERT, ops::MODIFY, ops::DELETE, ops::RENAME, ops::VALIDATE] {
            if ops & bit == 0 || !self.entered.insert((t, bit)) {
                continue;
            }
            for o in std::iter::once(t).chain(self.extensions_of(t)) {
                let obj = &objs[o];
                if obj.uncompiled {
                    self.unknown(|| format!("the triggers of {} are not compiled", obj.name));
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
        self.events(t);
    }

    /// What running object `o` reaches: a codeunit's `OnRun`, every routine of a page (and its source table's
    /// triggers), the triggers of a report or xmlport, with their extensions and the subscribers of their events.
    fn run_object(&mut self, o: usize) {
        if !self.entered.insert((o, RUN)) {
            return;
        }
        let lowered = self.lowered;
        let obj = &lowered.program.objects[o];
        if obj.uncompiled {
            self.unknown(|| format!("runs {}, which is not compiled", obj.name));
            return;
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
        if obj.kind == ObjKind::Page {
            if let Some(RtType::Record(t, _)) = obj.rec_slot.map(|i| &obj.globals[i].ty) {
                self.table_ops(*t as usize, ops::ALL);
            }
        }
        if let Some(query) = &obj.query {
            self.out.touched.extend(query.items.iter().map(|i| i.table as u32));
        }
        self.events(o);
    }

    /// The object of `kind` with id `id`, run; unknown when there is none.
    fn enter_by_id(&mut self, kind: ObjKind, id: i32) {
        match u32::try_from(id).ok().and_then(|id| self.index.by_id.get(&(kind, id))) {
            Some(&o) => self.run_object(o),
            None => self.unknown(|| format!("runs {kind} {id}, which is not in the program")),
        }
    }

    fn note_table_write(&mut self, table: Option<u32>) {
        match table {
            Some(t) => {
                self.out.writes.insert(t);
            }
            None => self.out.writes_any = true,
        }
        // The `Global Triggers` database events fire on writes.
        if !self.global_triggers {
            self.global_triggers = true;
            self.stack.extend(self.index.global_triggers.iter().copied());
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
                self.unknown(|| format!("{}.{}: {reason}", obj.name, rt.name));
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
                self.stack.push((sub.object, sub.routine));
            }
        }
        for s in rt.locals.iter().chain(&obj.globals) {
            let mut ty = &s.ty;
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
                        Some(RtType::Record(t, _)) => {
                            self.out.touched.insert(*t);
                            self.note_table_write(Some(*t));
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
        self.places(o, &rt.body);
        al_ir::types::for_each_expr_post(&rt.body, &mut |e| self.expr(o, r, e));
        lowered.calls_into(o, r, &mut self.stack);
    }

    /// The global of a codeunit a place names, as (object, slot).
    fn global_of(&self, o: usize, p: &Place) -> Option<(usize, usize)> {
        let objs = &self.lowered.program.objects;
        let mut p = p;
        loop {
            match p {
                Place::Index(b, _) | Place::Field(b, _) => p = b,
                Place::Global(g) => {
                    return (objs[o].kind == ObjKind::Codeunit).then_some((o, *g));
                }
                // A global of the base page or report: it lives with the page or report instance.
                Place::BaseGlobal(_) | Place::Local(_) => return None,
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
                Stmt::Expr(_) | Stmt::Exit(_) | Stmt::Break | Stmt::Pos(_) | Stmt::Assume(_) | Stmt::Continue => {}
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
                    let table = match receiver.and_then(|p| self.slot_ty(o, r, p)) {
                        Some(RtType::Record(t, _)) => Some(t),
                        _ => None,
                    };
                    if op != ops::VALIDATE {
                        self.note_table_write(table);
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
                "StartSession" | "SetDocumentServiceToken" => self.out.state_writes |= OTHER,
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
