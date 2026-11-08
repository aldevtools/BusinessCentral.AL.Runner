//! The AL profiler of `al-runner test --profile` (`.alcpuprofile` files the AL extension opens). Off unless a front end
//! calls [`Session::start_profiler`]; then the session builds a call tree, one node per call path (root, then the
//! frames as `Session::call` and an event publisher push them), and [`Session::take_profile`] hands it back as plain
//! data. No file, clock thread or environment variable here (ADR-006): the front end (al-testrun `profiling`) owns the
//! ticker, the JSON and the files.
//!
//! Two modes, as BC's AL profiler has:
//! - **Instrumentation**: exact entry and exit times (`Instant`), call counts, statement hit counts and, with
//!   `statement_times`, the time of each statement including the calls it makes.
//! - **Sampling**: no clock read per call. The front end's ticker sets the session's `tick` flag; the next statement
//!   marker, builtin return or record operation takes it and books the time since the last sample to the node (or the
//!   record operation) that is running. Every session has its own flag, so every thread is sampled on every tick.
//!
//! Both modes count the record operations that reach the database on BC (Get, Find*, Next, Insert, Modify, Delete,
//! CalcFields, ...) per node and table: the "Database (in memory)" nodes of the file.
//!
//! Cost when off: one `Option` test per statement, per call entry and exit, per builtin return and per record method.

use crate::Session;
use al_builtins::record_op::RecordOp;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::sync::Arc;
use std::time::Instant;

/// How a session profiles.
#[derive(Clone, Debug)]
pub enum ProfileMode {
    /// Exact times per call; `statement_times` also times each statement (one clock read per statement).
    Instrumentation { statement_times: bool },
    /// A sample whenever `tick` is set (the front end's ticker sets it every interval).
    Sampling { tick: Arc<AtomicBool> },
}

/// The routine of a node: indexes into `Program::objects` and the object's `routines`. `event` marks an event
/// publisher's frame (its subscribers hang below it, as on BC).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ProfFrame {
    pub object: usize,
    pub routine: usize,
    pub event: bool,
}

/// One statement of a node: its position id (`Program::positions`), how often it ran (instrumentation) or how many
/// samples landed on it (sampling), and its time in ns (inclusive of the calls it made; instrumentation with
/// statement times, or the sampled time).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PosStat {
    pub pos: u32,
    pub hits: u64,
    pub ns: u64,
}

/// The record operations of one kind on one table that a node made. `ns` is the time of the operations themselves,
/// without the triggers and subscribers they ran (those are child nodes); in sampling mode it is the sampled time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DbStat {
    pub op: RecordOp,
    pub table: String,
    pub temporary: bool,
    pub count: u64,
    pub ns: u64,
    pub samples: u64,
}

/// One call path.
#[derive(Clone, Debug, Default)]
pub struct ProfNode {
    /// `None` for node 0, the root.
    pub parent: Option<usize>,
    /// `None` for the root.
    pub frame: Option<ProfFrame>,
    pub children: Vec<usize>,
    /// How often the path was entered.
    pub calls: u64,
    /// Instrumentation: the time spent in the path, its children included. For the root, the sum of its children.
    pub total_ns: u64,
    /// Instrumentation: the part of `total_ns` spent in child nodes.
    pub child_ns: u64,
    /// Sampling: the samples that landed in this node itself (not in a record operation).
    pub samples: u64,
    /// Sampling: the time those samples stand for.
    pub sample_ns: u64,
    pub positions: Vec<PosStat>,
    pub db: Vec<DbStat>,
}

/// What a session's profiler collected, by plain indexes into the program it ran.
#[derive(Clone, Debug, Default)]
pub struct ProfileTree {
    pub nodes: Vec<ProfNode>,
    pub sampling: bool,
    /// Sampling: the samples taken (node and record-operation samples).
    pub samples: u64,
    /// The time from `start_profiler` to `take_profile`.
    pub wall_ns: u64,
}

/// The record methods that are a database round trip on BC: the ones the profile counts.
pub fn reaches_database(op: RecordOp) -> bool {
    use RecordOp::*;
    matches!(
        op,
        Get | GetBySystemId
            | Find
            | FindFirst
            | FindLast
            | FindSet
            | Next
            | Insert
            | Modify
            | ModifyAll
            | Delete
            | DeleteAll
            | Rename
            | CalcFields
            | CalcSums
            | Count
            | CountApprox
            | IsEmpty
            | LockTable
    )
}

struct Db {
    op: RecordOp,
    table: u32,
    temporary: bool,
    count: u64,
    ns: u64,
    samples: u64,
}

#[derive(Default)]
struct Node {
    parent: u32,
    frame: Option<ProfFrame>,
    children: Vec<u32>,
    calls: u64,
    total_ns: u64,
    child_ns: u64,
    samples: u64,
    sample_ns: u64,
    /// Statement stats by `pos - pos_base` (a routine's positions are close together): (hits, ns).
    pos_base: u32,
    pos: Vec<(u64, u64)>,
    db: Vec<Db>,
}

impl Node {
    fn stat(&mut self, p: u32) -> &mut (u64, u64) {
        if self.pos.is_empty() {
            self.pos_base = p;
        } else if p < self.pos_base {
            let shift = (self.pos_base - p) as usize;
            self.pos.splice(0..0, std::iter::repeat_n((0, 0), shift));
            self.pos_base = p;
        }
        let i = (p - self.pos_base) as usize;
        if self.pos.len() <= i {
            self.pos.resize(i + 1, (0, 0));
        }
        &mut self.pos[i]
    }

    fn db(&mut self, op: RecordOp, table: u32, temporary: bool) -> &mut Db {
        let i = match self
            .db
            .iter()
            .position(|d| d.op == op && d.table == table && d.temporary == temporary)
        {
            Some(i) => i,
            None => {
                self.db.push(Db {
                    op,
                    table,
                    temporary,
                    count: 0,
                    ns: 0,
                    samples: 0,
                });
                self.db.len() - 1
            }
        };
        &mut self.db[i]
    }
}

struct Entry {
    node: u32,
    /// Instrumentation: when the call began.
    start: Option<Instant>,
    /// Instrumentation: the time of the calls that returned to this one.
    child_ns: u64,
    /// Instrumentation with statement times: the statement running and since when.
    stmt: Option<(u32, Instant)>,
}

/// A session's profiler (`CoverageState::profiler`).
pub(crate) struct Profiler {
    tick: Option<Arc<AtomicBool>>,
    statement_times: bool,
    nodes: Vec<Node>,
    stack: Vec<Entry>,
    started: Instant,
    last_sample: Instant,
    samples: u64,
}

/// What [`Profiler::db_begin`] notes before a record operation.
pub(crate) struct DbMark {
    start: Option<Instant>,
    child_ns: u64,
}

impl Profiler {
    pub(crate) fn new(mode: ProfileMode) -> Profiler {
        let now = Instant::now();
        let (tick, statement_times) = match mode {
            ProfileMode::Instrumentation { statement_times } => (None, statement_times),
            ProfileMode::Sampling { tick } => (Some(tick), false),
        };
        Profiler {
            tick,
            statement_times,
            nodes: vec![Node::default()],
            stack: vec![Entry {
                node: 0,
                start: None,
                child_ns: 0,
                stmt: None,
            }],
            started: now,
            last_sample: now,
            samples: 0,
        }
    }

    fn instrumenting(&self) -> bool {
        self.tick.is_none()
    }

    fn top(&self) -> u32 {
        self.stack.last().map_or(0, |e| e.node)
    }

    pub(crate) fn enter(&mut self, frame: ProfFrame) {
        let parent = self.top();
        let found = self.nodes[parent as usize]
            .children
            .iter()
            .copied()
            .find(|&c| self.nodes[c as usize].frame == Some(frame));
        let node = match found {
            Some(c) => c,
            None => {
                let c = self.nodes.len() as u32;
                self.nodes.push(Node {
                    parent,
                    frame: Some(frame),
                    ..Node::default()
                });
                self.nodes[parent as usize].children.push(c);
                c
            }
        };
        self.nodes[node as usize].calls += 1;
        let start = self.instrumenting().then(Instant::now);
        self.stack.push(Entry {
            node,
            start,
            child_ns: 0,
            stmt: None,
        });
    }

    pub(crate) fn leave(&mut self) {
        if self.stack.len() <= 1 {
            return;
        }
        let Some(e) = self.stack.pop() else { return };
        let Some(start) = e.start else { return };
        let now = Instant::now();
        let ns = now.duration_since(start).as_nanos() as u64;
        let n = &mut self.nodes[e.node as usize];
        n.total_ns += ns;
        n.child_ns += e.child_ns;
        if let Some((p, since)) = e.stmt {
            n.stat(p).1 += now.duration_since(since).as_nanos() as u64;
        }
        if let Some(parent) = self.stack.last_mut() {
            parent.child_ns += ns;
        }
    }

    /// The statement at `p` begins; `running` is the statement that ran until now (the frame's position before it
    /// moves), for a sample.
    pub(crate) fn statement(&mut self, p: u32, running: u32) {
        if !self.instrumenting() {
            self.maybe_sample(running);
            return;
        }
        let Some(e) = self.stack.last_mut() else { return };
        let n = &mut self.nodes[e.node as usize];
        n.stat(p).0 += 1;
        if self.statement_times {
            let now = Instant::now();
            if let Some((q, since)) = e.stmt.replace((p, now)) {
                n.stat(q).1 += now.duration_since(since).as_nanos() as u64;
            }
        }
    }

    /// Sampling: a builtin returned while `running` ran.
    pub(crate) fn builtin_returned(&mut self, running: u32) {
        if !self.instrumenting() {
            self.maybe_sample(running);
        }
    }

    /// Takes a pending tick: the time since the last sample goes to the running node and statement.
    fn maybe_sample(&mut self, running: u32) {
        let Some(tick) = &self.tick else { return };
        if !tick.load(Relaxed) || !tick.swap(false, Relaxed) {
            return;
        }
        let ns = self.since_last_sample();
        let node = self.top() as usize;
        let n = &mut self.nodes[node];
        n.samples += 1;
        n.sample_ns += ns;
        if node != 0 && running != al_ir::NO_POS {
            let s = n.stat(running);
            s.0 += 1;
            s.1 += ns;
        }
    }

    fn since_last_sample(&mut self) -> u64 {
        let now = Instant::now();
        let ns = now.duration_since(self.last_sample).as_nanos() as u64;
        self.last_sample = now;
        self.samples += 1;
        ns
    }

    pub(crate) fn db_begin(&self) -> DbMark {
        DbMark {
            start: self.instrumenting().then(Instant::now),
            child_ns: self.stack.last().map_or(0, |e| e.child_ns),
        }
    }

    pub(crate) fn db_end(&mut self, mark: DbMark, op: RecordOp, table: u32, temporary: bool) {
        let node = self.top() as usize;
        let sampled = match &self.tick {
            Some(t) if t.load(Relaxed) && t.swap(false, Relaxed) => Some(self.since_last_sample()),
            _ => None,
        };
        let own = mark.start.map(|s| {
            let children = self.stack.last().map_or(0, |e| e.child_ns) - mark.child_ns;
            (s.elapsed().as_nanos() as u64).saturating_sub(children)
        });
        let d = self.nodes[node].db(op, table, temporary);
        d.count += 1;
        d.ns += own.unwrap_or(0);
        if let Some(ns) = sampled {
            d.samples += 1;
            d.ns += ns;
        }
    }

    fn into_tree(mut self, table_name: impl Fn(u32) -> String) -> ProfileTree {
        // Calls still open (the tree is taken inside a routine) end now.
        while self.stack.len() > 1 {
            self.leave();
        }
        let root_child_ns = self.stack.first().map_or(0, |e| e.child_ns);
        let sampling = !self.instrumenting();
        let nodes = self
            .nodes
            .into_iter()
            .enumerate()
            .map(|(i, n)| {
                let positions = n
                    .pos
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| s.0 > 0 || s.1 > 0)
                    .map(|(k, &(hits, ns))| PosStat {
                        pos: n.pos_base + k as u32,
                        hits,
                        ns,
                    })
                    .collect();
                let db = n
                    .db
                    .iter()
                    .map(|d| DbStat {
                        op: d.op,
                        table: table_name(d.table),
                        temporary: d.temporary,
                        count: d.count,
                        ns: d.ns,
                        samples: d.samples,
                    })
                    .collect();
                let (total_ns, child_ns) = if i == 0 {
                    (root_child_ns, root_child_ns)
                } else {
                    (n.total_ns, n.child_ns)
                };
                ProfNode {
                    parent: (i != 0).then_some(n.parent as usize),
                    frame: n.frame,
                    children: n.children.iter().map(|&c| c as usize).collect(),
                    calls: n.calls,
                    total_ns,
                    child_ns,
                    samples: n.samples,
                    sample_ns: n.sample_ns,
                    positions,
                    db,
                }
            })
            .collect();
        ProfileTree {
            nodes,
            sampling,
            samples: self.samples,
            wall_ns: self.started.elapsed().as_nanos() as u64,
        }
    }
}

/// The name of a frame in the text summary: the object type and name, then the routine. A field trigger carries its
/// field (`table Sales Line.Quantity.OnValidate`; for a table extension the field of the extended table), and a
/// trigger of a page control, action or report data item carries its owner's name.
pub fn frame_name(program: &al_ir::Program, object: usize, routine: usize) -> String {
    let obj = &program.objects[object];
    match routine_label(program, object, routine) {
        Some(r) => format!("{} {}.{r}", obj.kind, obj.name),
        None => format!("{} {}.?", obj.kind, obj.name),
    }
}

/// The routine part of [`frame_name`]: `Quantity.OnValidate`, `PageAction.OnAction`, `Run`.
pub fn routine_label(program: &al_ir::Program, object: usize, routine: usize) -> Option<String> {
    let obj = &program.objects[object];
    let r = obj.routines.get(routine)?;
    let owner = match (r.field, &r.container) {
        (Some(f), _) => {
            let table = match obj.kind {
                al_ir::ObjKind::TableExtension => program
                    .objects
                    .heads()
                    .iter()
                    .find(|o| o.kind == al_ir::ObjKind::Table && o.id.is_some() && o.id == obj.extends_id)
                    .and_then(|o| o.table.as_ref()),
                _ => obj.table.as_ref(),
            };
            table.and_then(|t| t.fields.get(f)).map(|fd| fd.name.clone())
        }
        (None, Some((_, owner))) if r.is_trigger && !owner.is_empty() => Some(owner.clone()),
        _ => None,
    };
    Some(match owner {
        Some(o) => format!("{o}.{}", r.name),
        None => r.name.clone(),
    })
}

impl Session {
    /// Starts profiling this session (`al-runner test --profile`). A profile already running starts again.
    pub fn start_profiler(&mut self, mode: ProfileMode) {
        self.coverage.profiler = Some(Box::new(Profiler::new(mode)));
    }

    /// Stops profiling and returns what was collected; `None` when the profiler was not started.
    pub fn take_profile(&mut self) -> Option<ProfileTree> {
        let p = self.coverage.profiler.take()?;
        let tables = self.engine.tables.clone();
        Some(p.into_tree(|t| {
            tables
                .get(t as usize)
                .and_then(Option::as_ref)
                .map_or_else(|| format!("table #{t}"), |d| d.name.clone())
        }))
    }

    /// A frame was pushed (`Session::call`, an event publisher).
    #[inline(always)]
    pub(crate) fn profile_enter(&mut self, object: usize, routine: usize, event: bool) {
        if let Some(p) = self.coverage.profiler.as_deref_mut() {
            p.enter(ProfFrame { object, routine, event });
        }
    }

    /// The frame `profile_enter` pushed is popped, normally or by an error.
    #[inline(always)]
    pub(crate) fn profile_leave(&mut self) {
        if let Some(p) = self.coverage.profiler.as_deref_mut() {
            p.leave();
        }
    }

    /// The statement at `p` begins (called before the frame moves to it).
    #[inline(always)]
    pub(crate) fn profile_statement(&mut self, p: u32) {
        if let Some(prof) = self.coverage.profiler.as_deref_mut() {
            let running = self.call_stack.last().map_or(al_ir::NO_POS, |f| f.pos);
            prof.statement(p, running);
        }
    }

    /// A builtin returned: sampling takes a pending tick.
    #[inline(always)]
    pub(crate) fn profile_builtin(&mut self) {
        if let Some(prof) = self.coverage.profiler.as_deref_mut() {
            let running = self.call_stack.last().map_or(al_ir::NO_POS, |f| f.pos);
            prof.builtin_returned(running);
        }
    }

    /// `record_op` while the profiler runs: counts and times an operation that reaches the database on BC.
    #[cold]
    #[inline(never)]
    pub(crate) fn record_op_profiled(
        &mut self,
        method: RecordOp,
        rec: &crate::records::Rec,
        args: &mut [al_builtins::value::Value],
    ) -> Result<al_builtins::value::Value, al_builtins::native::Signal> {
        if !reaches_database(method) {
            return self.record_op_unprofiled(method, rec, args);
        }
        let (table, temporary) = {
            let r = rec.borrow();
            (r.table, r.temporary)
        };
        let mark = self.coverage.profiler.as_deref().map(Profiler::db_begin);
        let r = self.record_op_unprofiled(method, rec, args);
        if let (Some(p), Some(mark)) = (self.coverage.profiler.as_deref_mut(), mark) {
            p.db_end(mark, method, table, temporary);
        }
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statement_stats_grow_both_ways() {
        let mut n = Node::default();
        n.stat(10).0 += 1;
        n.stat(12).0 += 2;
        n.stat(8).0 += 3;
        assert_eq!(n.pos_base, 8);
        assert_eq!(n.pos, vec![(3, 0), (0, 0), (1, 0), (0, 0), (2, 0)]);
    }

    #[test]
    fn leave_at_the_root_does_nothing() {
        let mut p = Profiler::new(ProfileMode::Instrumentation { statement_times: false });
        p.leave();
        p.enter(ProfFrame {
            object: 1,
            routine: 2,
            event: false,
        });
        p.leave();
        p.leave();
        let t = p.into_tree(|_| String::new());
        assert_eq!(t.nodes.len(), 2);
        assert_eq!(t.nodes[1].calls, 1);
        assert_eq!(t.nodes[0].total_ns, t.nodes[1].total_ns);
    }

    #[test]
    fn database_operations_are_the_round_trips() {
        assert!(reaches_database(RecordOp::FindSet));
        assert!(reaches_database(RecordOp::CalcFields));
        assert!(!reaches_database(RecordOp::SetRange));
        assert!(!reaches_database(RecordOp::Init));
    }
}
