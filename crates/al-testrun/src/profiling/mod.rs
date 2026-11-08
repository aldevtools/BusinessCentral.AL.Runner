//! The AL profiler of a run (`al-runner test --profile`, serve `profile`): the front-end side of al-vm's
//! `Session::start_profiler` (ADR-006: the ticker thread, the clock of the file, the JSON and the files live here).
//!
//! A front end calls [`start`] before the run and [`finish`] after it. Every test codeunit's session is profiled
//! (`runtime/run.rs` `new_session` -> [`begin`], end of the codeunit -> [`end`]); its tree is resolved against the
//! program right away (names, apps, source positions), so a later compile of the same run (a lean compile run again)
//! cannot mix positions. A codeunit that runs twice in a run keeps its last profile, the one its outcome comes from.
//!
//! Sampling: one ticker thread for the process sets the tick flag of every registered session each interval, so every
//! thread is sampled on every tick (al-runner issue 6924: the old `AL_PROFILE` took one thread per tick).

pub mod alcpuprofile;
pub mod summary;
pub mod write;

use al_ir::Program;
use al_vm::{ProfileMode, ProfileTree, Session};
use std::cell::Cell;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering::Relaxed};
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Instrumentation (exact times per call, BC's default for a snapshot) or sampling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Instrumentation,
    Sampling,
}

impl Mode {
    pub fn parse(s: &str) -> Result<Mode, String> {
        match s.to_ascii_lowercase().as_str() {
            "instrumentation" => Ok(Mode::Instrumentation),
            "sampling" => Ok(Mode::Sampling),
            _ => Err(format!("profile mode must be instrumentation or sampling, not `{s}`")),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Mode::Instrumentation => "instrumentation",
            Mode::Sampling => "sampling",
        }
    }
}

/// How a run profiles.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    pub mode: Mode,
    /// Sampling: microseconds between ticks.
    pub interval_us: u64,
    /// Instrumentation: time each statement (CodeLens times in VS Code); one clock read per statement.
    pub statement_times: bool,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            mode: Mode::Instrumentation,
            interval_us: 1000,
            statement_times: true,
        }
    }
}

/// The app that declares a node's code (`declaringApplication`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AppRef {
    pub id: String,
    pub name: String,
    pub publisher: String,
    pub version: String,
}

/// A statement of a node, 0-based as the file wants it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Statement {
    pub line: u32,
    pub column: u32,
    pub hits: u64,
    pub ns: u64,
}

/// One node of a resolved tree.
#[derive(Clone, Debug, Default)]
pub struct Node {
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    /// The routine (`OnValidate` of a field: `Quantity.OnValidate`); for the root, the thread's label.
    pub function: String,
    /// BC's object type name (`CodeUnit`, `Table`, ..., `System` for the root).
    pub object_type: &'static str,
    /// The AL keyword of the object (`Codeunit`), for the `al-preview` link of code without a file.
    pub keyword: &'static str,
    pub object_name: String,
    pub object_id: u32,
    pub app: AppRef,
    /// The file the routine is in, as the program names it (relative to its app; a dependency's starts with its name).
    pub file: Option<String>,
    /// 0-based position of the routine's first statement (BC's entry statement).
    pub line: u32,
    pub column: u32,
    pub event: bool,
    pub calls: u64,
    pub total_ns: u64,
    pub child_ns: u64,
    pub samples: u64,
    pub sample_ns: u64,
    pub statements: Vec<Statement>,
    pub db: Vec<al_vm::DbStat>,
    /// The name the text summary uses (`codeunit Calib.CpuWork`).
    pub frame_name: String,
}

/// A thread of the run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Thread {
    pub id: usize,
    pub label: String,
}

/// The profile of one test codeunit's run.
#[derive(Clone, Debug)]
pub struct CodeunitProfile {
    pub codeunit: u32,
    pub name: String,
    pub app: AppRef,
    pub thread: Thread,
    pub sampling: bool,
    /// Microseconds since the Unix epoch when the session's profile began.
    pub start_unix_us: u64,
    pub wall_ns: u64,
    pub samples: u64,
    /// Node 0 is the root.
    pub nodes: Vec<Node>,
    /// The tests in the order they ran, with their wall time.
    pub tests: Vec<(String, Duration)>,
}

/// What a run collected.
#[derive(Clone, Debug)]
pub struct Collected {
    pub options: Options,
    pub codeunits: Vec<CodeunitProfile>,
}

struct State {
    options: Options,
    codeunits: Vec<CodeunitProfile>,
    ticks: Vec<Weak<AtomicBool>>,
}

static ON: AtomicBool = AtomicBool::new(false);
static STATE: Mutex<Option<State>> = Mutex::new(None);
/// Ticker generation: a ticker stops when it no longer matches (one per `start`).
static GENERATION: AtomicU64 = AtomicU64::new(0);

fn state() -> std::sync::MutexGuard<'static, Option<State>> {
    STATE.lock().unwrap_or_else(|e| e.into_inner())
}

/// Profiles the test codeunits of the runs that follow, until [`finish`].
pub fn start(options: Options) {
    let generation = GENERATION.fetch_add(1, Relaxed) + 1;
    let sampling = options.mode == Mode::Sampling;
    let interval = Duration::from_micros(options.interval_us.max(50));
    *state() = Some(State {
        options,
        codeunits: Vec::new(),
        ticks: Vec::new(),
    });
    ON.store(true, Relaxed);
    if sampling {
        std::thread::Builder::new()
            .name("al-profile-ticker".into())
            .spawn(move || ticker(generation, interval))
            .ok();
    }
}

fn ticker(generation: u64, interval: Duration) {
    while GENERATION.load(Relaxed) == generation {
        std::thread::sleep(interval);
        let mut st = state();
        let Some(s) = st.as_mut() else { return };
        s.ticks.retain(|w| match w.upgrade() {
            Some(t) => {
                t.store(true, Relaxed);
                true
            }
            None => false,
        });
    }
}

/// Stops profiling and returns what the runs since [`start`] collected; `None` when profiling was off.
pub fn finish() -> Option<Collected> {
    ON.store(false, Relaxed);
    GENERATION.fetch_add(1, Relaxed);
    let s = state().take()?;
    let mut codeunits = s.codeunits;
    codeunits.sort_by(|a, b| a.codeunit.cmp(&b.codeunit).then(a.name.cmp(&b.name)));
    Some(Collected {
        options: s.options,
        codeunits,
    })
}

/// Whether a profile is being collected.
pub fn active() -> bool {
    ON.load(Relaxed)
}

/// A test codeunit's session starts: profiles it when profiling is on.
pub(crate) fn begin(session: &mut Session) {
    if !ON.load(Relaxed) {
        return;
    }
    let mut st = state();
    let Some(s) = st.as_mut() else { return };
    let mode = match s.options.mode {
        Mode::Instrumentation => ProfileMode::Instrumentation {
            statement_times: s.options.statement_times,
        },
        Mode::Sampling => {
            let tick = Arc::new(AtomicBool::new(false));
            s.ticks.push(Arc::downgrade(&tick));
            ProfileMode::Sampling { tick }
        }
    };
    drop(st);
    session.start_profiler(mode);
}

/// A test codeunit's session ends: its tree is resolved and kept.
pub(crate) fn end(session: &mut Session, program: &Program, object: usize, tests: &[(String, Duration)]) {
    let Some(tree) = session.take_profile() else { return };
    let start_unix_us = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_micros() as u64)
        .saturating_sub(tree.wall_ns / 1000);
    let p = resolve(&tree, program, object, tests, thread(), start_unix_us);
    if let Some(s) = state().as_mut() {
        s.codeunits.retain(|c| c.codeunit != p.codeunit || c.name != p.name);
        s.codeunits.push(p);
    }
}

thread_local! {
    static THREAD_ID: Cell<usize> = const { Cell::new(0) };
}

/// This thread's number in the run (1, 2, ... in the order threads first ran a codeunit) and label.
fn thread() -> Thread {
    static NEXT: AtomicUsize = AtomicUsize::new(1);
    let id = THREAD_ID.with(|t| {
        if t.get() == 0 {
            t.set(NEXT.fetch_add(1, Relaxed));
        }
        t.get()
    });
    let name = std::thread::current().name().unwrap_or("test worker").to_string();
    let label = match name.as_str() {
        "main" => format!("Thread {id} (main)"),
        n => format!("Thread {id} ({n})"),
    };
    Thread { id, label }
}

/// BC's object type names in a profile (`ApplicationObjectDefinition.objectType`, the Nav enum).
pub fn bc_object_type(kind: al_ir::ObjKind) -> &'static str {
    use al_ir::ObjKind as K;
    match kind {
        K::Table => "Table",
        K::TableExtension => "TableExtension",
        K::Page => "Page",
        K::PageExtension => "PageExtension",
        K::Codeunit => "CodeUnit",
        K::Report => "Report",
        K::ReportExtension => "ReportExtension",
        K::Query => "Query",
        K::XmlPort => "XmlPort",
        K::Enum => "Enum",
        K::EnumExtension => "EnumExtension",
        K::Interface => "Interface",
        K::PermissionSet => "PermissionSet",
        K::PermissionSetExtension => "PermissionSetExtension",
        K::Profile => "Profile",
        K::Other => "System",
    }
}

fn keyword(kind: al_ir::ObjKind) -> &'static str {
    use al_ir::ObjKind as K;
    match kind {
        K::Table => "Table",
        K::TableExtension => "TableExtension",
        K::Page => "Page",
        K::PageExtension => "PageExtension",
        K::Codeunit => "Codeunit",
        K::Report => "Report",
        K::ReportExtension => "ReportExtension",
        K::Query => "Query",
        K::XmlPort => "XmlPort",
        K::Enum => "Enum",
        K::EnumExtension => "EnumExtension",
        K::Interface => "Interface",
        K::PermissionSet => "PermissionSet",
        K::PermissionSetExtension => "PermissionSetExtension",
        K::Profile => "Profile",
        K::Other => "Object",
    }
}

fn app_ref(program: &Program, app: u32) -> AppRef {
    program.apps.get(app as usize).map_or_else(AppRef::default, |a| AppRef {
        id: a.id.clone(),
        name: a.name.clone(),
        publisher: a.publisher.clone(),
        version: a.version.clone(),
    })
}

/// The first statement marker of a list of statements, depth first.
fn first_pos(stmts: &[al_ir::Stmt]) -> Option<u32> {
    use al_ir::Stmt;
    for s in stmts {
        let found = match s {
            Stmt::Pos(p) => Some(*p),
            Stmt::If(_, a, b) => first_pos(a).or_else(|| first_pos(b)),
            Stmt::While(_, b) | Stmt::Repeat(b, _) | Stmt::AssertError(b) => first_pos(b),
            Stmt::For { body, .. } | Stmt::Foreach { body, .. } => first_pos(body),
            _ => None,
        };
        if found.is_some() {
            return found;
        }
    }
    None
}

/// Resolves a session's tree against the program it ran.
pub(crate) fn resolve(
    tree: &ProfileTree,
    program: &Program,
    object: usize,
    tests: &[(String, Duration)],
    thread: Thread,
    start_unix_us: u64,
) -> CodeunitProfile {
    let obj = &program.objects[object];
    let nodes = tree
        .nodes
        .iter()
        .map(|n| {
            let mut out = Node {
                parent: n.parent,
                children: n.children.clone(),
                calls: n.calls,
                total_ns: n.total_ns,
                child_ns: n.child_ns,
                samples: n.samples,
                sample_ns: n.sample_ns,
                db: n.db.clone(),
                object_type: "System",
                keyword: "Object",
                ..Node::default()
            };
            let Some(f) = n.frame else {
                out.function = thread.label.clone();
                out.object_name = thread.label.clone();
                out.frame_name = thread.label.clone();
                out.app.name = "System".into();
                return out;
            };
            let o = &program.objects[f.object];
            out.event = f.event;
            out.object_type = bc_object_type(o.kind);
            out.keyword = keyword(o.kind);
            out.object_name = o.name.clone();
            out.object_id = o.id.unwrap_or(0);
            out.app = app_ref(program, o.app);
            out.function = al_vm::profile_routine_label(program, f.object, f.routine).unwrap_or_else(|| "?".into());
            out.frame_name = al_vm::profile_frame_name(program, f.object, f.routine);
            if let Some(r) = o.routines.get(f.routine) {
                let entry = first_pos(r.body.stmts()).or(Some(r.pos).filter(|&p| p != al_ir::NO_POS));
                if let Some(p) = entry.and_then(|p| program.pos(p)) {
                    out.file = program.sources.get(p.file as usize).map(|s| s.path.clone());
                    // The runner's lines and columns start at 1 (al-sema `locate`); the file's start at 0.
                    out.line = p.line.saturating_sub(1);
                    out.column = p.col.saturating_sub(1);
                }
            }
            out.statements = n
                .positions
                .iter()
                .filter_map(|s| {
                    let p = program.pos(s.pos)?;
                    Some(Statement {
                        line: p.line.saturating_sub(1),
                        column: p.col.saturating_sub(1),
                        hits: s.hits,
                        ns: s.ns,
                    })
                })
                .collect();
            out
        })
        .collect();
    CodeunitProfile {
        codeunit: obj.id.unwrap_or(0),
        name: obj.name.clone(),
        app: app_ref(program, obj.app),
        thread,
        sampling: tree.sampling,
        start_unix_us,
        wall_ns: tree.wall_ns,
        samples: tree.samples,
        nodes,
        tests: tests.to_vec(),
    }
}

/// Microseconds from 0001-01-01 (BC's `startTime` epoch, .NET ticks / 10) to the Unix epoch.
pub const UNIX_EPOCH_FROM_YEAR_1_US: u64 = 62_135_596_800_000_000;

/// Self time of a node: instrumentation from its times, sampling from its samples.
pub fn self_ns(n: &Node) -> u64 {
    if n.total_ns > 0 || n.child_ns > 0 {
        n.total_ns.saturating_sub(n.child_ns)
    } else {
        n.sample_ns
    }
}

/// The deprecated `AL_PROFILE=<file>` variable (kept for one release): sampling, every `AL_PROFILE_US` µs (default
/// 5000), the text summary written to `<file>` and the merged profile to `<file>.alcpuprofile` when the run ends.
/// Returns the file when the variable is set; the caller says it is deprecated.
pub fn start_from_env() -> Option<std::path::PathBuf> {
    let path = std::env::var_os("AL_PROFILE")?;
    let interval_us = std::env::var("AL_PROFILE_US")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5000);
    start(Options {
        mode: Mode::Sampling,
        interval_us,
        statement_times: false,
    });
    *LEGACY.lock().unwrap_or_else(|e| e.into_inner()) = Some(path.clone().into());
    Some(path.into())
}

static LEGACY: Mutex<Option<std::path::PathBuf>> = Mutex::new(None);

/// The note a front end prints when `AL_PROFILE` is set.
pub const LEGACY_NOTE: &str = "al-runner: AL_PROFILE is deprecated and will be removed in the next release; \
                               use --profile PATH --profile-mode sampling";

/// Writes the files of `AL_PROFILE` when the run ends (nothing when the variable was not set, or a `--profile` run
/// took the profile).
pub fn finish_legacy() {
    let Some(path) = LEGACY.lock().unwrap_or_else(|e| e.into_inner()).take() else {
        return;
    };
    let Some(c) = finish() else { return };
    let _ = std::fs::write(&path, summary::render(&c));
    let mut merged = path.into_os_string();
    merged.push(".alcpuprofile");
    let _ = write::write(std::path::Path::new(&merged), &c, &|_| None);
}

#[cfg(test)]
mod tests;
