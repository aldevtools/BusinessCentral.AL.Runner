//! Runs test codeunits the way BC's test runner does, as far as the corpus
//! pins it: one instance per test codeunit, `OnRun` once before the tests
//! (codeunit/TestCodeunitOnRun.al), tests in declaration order.

use crate::explain::{self, TestDetail};
use crate::isolation::{Checkpoint, TestIsolation, runner_isolation};
use crate::{CaseId, Outcome};
use al_builtins::assumptions::Assumption;
use al_builtins::describe::Describe;
use al_builtins::native::{AlError, Signal};
use al_sema::lower::Lowered;
use al_vm::{Engine, Session, UiKind};
use std::collections::BTreeSet;
use std::sync::Arc;

/// Reads an app folder: its `app.json` manifest and every `.al` file under it.
pub fn load_app(dir: &std::path::Path) -> Result<al_sema::workspace::AppSource, String> {
    let manifest: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(dir.join("app.json")).map_err(|e| format!("{}: {e}", dir.display()))?,
    )
    .map_err(|e| format!("{}/app.json: {e}", dir.display()))?;
    let mut files = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).map_err(|e| e.to_string())? {
            let p = e.map_err(|e| e.to_string())?.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "al") {
                let rel = p.strip_prefix(dir).unwrap_or(&p).to_string_lossy().into_owned();
                files.push((
                    rel,
                    std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?,
                ));
            }
        }
    }
    files.sort();
    let s = |k: &str| manifest[k].as_str().unwrap_or_default().to_string();
    Ok(al_sema::workspace::AppSource {
        id: s("id"),
        name: s("name"),
        dependencies: manifest["dependencies"]
            .as_array()
            .map(|a| a.iter().filter_map(|d| d["id"].as_str().map(String::from)).collect())
            .unwrap_or_default(),
        files,
    })
}

/// What only an app's manifest says that `ModuleInfo` reports (publisher and version).
#[derive(Debug, Clone, Default)]
pub struct Manifest {
    pub publisher: String,
    pub version: String,
    /// The files under the manifest's `resourceFolders`, named relative to their folder.
    pub resources: Vec<(String, Vec<u8>)>,
    /// `resourceExposurePolicy.includeSourceInSymbolFile`: the Code Coverage table shows the source lines.
    pub include_source: bool,
    /// `target` (`Cloud` when the manifest leaves it out); empty when not known (a dependency package).
    pub target: String,
    /// `preprocessorSymbols` of the app's own app.json. A dependency app is compiled with these;
    /// the project's symbols apply to the workspace apps only (the publisher compiled the dependency).
    pub symbols: Vec<String>,
    /// The files the app's reports name with `LayoutFile`: path relative to the app (`/` separators), content.
    pub layout_files: Vec<(String, Vec<u8>)>,
}

/// The components of `path` made absolute, with `.` and `..` resolved without touching the file system.
fn lexical(path: &std::path::Path) -> Vec<String> {
    let abs = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let mut parts: Vec<String> = Vec::new();
    for c in abs.components() {
        match c {
            std::path::Component::ParentDir => {
                parts.pop();
            }
            std::path::Component::CurDir => {}
            c => parts.push(c.as_os_str().to_string_lossy().to_string()),
        }
    }
    parts
}

/// The files the `.al` files of `dir` name with `LayoutFile = '...'`, read where they exist: the path is relative to `dir`
/// (the written path from the app's folder), content bytes.
fn load_layout_files(dir: &std::path::Path) -> Vec<(String, Vec<u8>)> {
    let mut out: Vec<(String, Vec<u8>)> = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else { continue };
        for e in entries.flatten() {
            let p = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            if p.is_dir() {
                if !name.starts_with('.') && name != "target" && name != "node_modules" {
                    stack.push(p);
                }
                continue;
            }
            if !name.to_ascii_lowercase().ends_with(".al") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&p) else {
                continue;
            };
            let lower = text.to_ascii_lowercase();
            let mut from = 0;
            while let Some(at) = lower[from..].find("layoutfile") {
                let start = from + at + "layoutfile".len();
                from = start;
                let rest = text[start..].trim_start();
                let Some(rest) = rest.strip_prefix('=') else { continue };
                let Some(rest) = rest.trim_start().strip_prefix('\'') else {
                    continue;
                };
                let Some(end) = rest.find('\'') else { continue };
                let written = &rest[..end];
                let target = dir.join(written);
                let Ok(bytes) = std::fs::read(&target) else { continue };
                let (root, full) = (lexical(dir), lexical(&target));
                let rel = full[root.len().min(full.len())..].join("/");
                if !out.iter().any(|(r, _)| *r == rel) {
                    out.push((rel, bytes));
                }
            }
        }
    }
    out.sort();
    out
}

/// The manifest fields `NavApp.GetCurrentModuleInfo` reports, from `dir/app.json`.
pub fn load_manifest(dir: &std::path::Path) -> Result<Manifest, String> {
    let manifest: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(dir.join("app.json")).map_err(|e| format!("{}: {e}", dir.display()))?,
    )
    .map_err(|e| format!("{}/app.json: {e}", dir.display()))?;
    let s = |k: &str| manifest[k].as_str().unwrap_or_default().to_string();
    let mut resources = Vec::new();
    for folder in manifest["resourceFolders"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|f| f.as_str())
    {
        let root = dir.join(folder);
        let mut stack = vec![root.clone()];
        while let Some(d) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&d) else { continue };
            for e in entries {
                let p = e.map_err(|e| e.to_string())?.path();
                if p.is_dir() {
                    stack.push(p);
                } else {
                    let rel = p.strip_prefix(&root).unwrap_or(&p).to_string_lossy().replace('\\', "/");
                    resources.push((rel, std::fs::read(&p).map_err(|e| format!("{}: {e}", p.display()))?));
                }
            }
        }
    }
    resources.sort();
    Ok(Manifest {
        publisher: s("publisher"),
        version: s("version"),
        resources,
        include_source: manifest["resourceExposurePolicy"]["includeSourceInSymbolFile"]
            .as_bool()
            .unwrap_or(false),
        layout_files: load_layout_files(dir),
        target: manifest["target"].as_str().unwrap_or("Cloud").to_string(),
        symbols: manifest["preprocessorSymbols"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|s| s.as_str().map(String::from))
            .collect(),
    })
}

/// Every corpus test's outcome on this runtime, keyed by (codeunit id, procedure).
pub struct CorpusRuntime {
    /// Keyed by (suite, test codeunit id, lower-cased procedure). A suite is one test runner run
    /// over some of the apps; the same codeunit can be in two (the isolation probe).
    outcomes: std::collections::HashMap<(usize, u32, String), Outcome>,
    /// How long each test that ran took, keyed like `outcomes`.
    times: std::collections::HashMap<(usize, u32, String), std::time::Duration>,
    suites: Vec<String>,
    pub panics: usize,
    /// How long the compile, the install baseline and the tests took.
    pub phases: Phases,
    /// With `RunOptions::trace`: where each test of the run codeunits was refused or failed.
    details: std::collections::HashMap<(usize, u32, String), TestDetail>,
    /// Every test the run decided, codeunit by codeunit.
    results: Vec<TestResult>,
    /// The unprobed assumptions each test ran through, same key as `outcomes`; tests with none are absent.
    marks: std::collections::HashMap<(usize, u32, String), Vec<Assumption>>,
    /// With `RunOptions::coverage`: every statement of the workspace's apps.
    coverage_statements: Vec<crate::coverage::CoverageStatement>,
    /// With `RunOptions::impact_trace`: the keys `TestResult::impact` numbers (`impact_trace::keys`).
    impact_keys: Vec<String>,
    /// With `RunOptions::impact_trace`: what the install baseline raised and touched (numbers of `impact_keys`);
    /// `None` when no baseline ran.
    impact_baseline: Option<Vec<u32>>,
    /// The apps of the compiled program: (id, name, version), in program order.
    program_apps: Vec<(String, String, String)>,
    /// `--tdd`: every stub of the compiled program (`Program::tdd_stubs`).
    tdd_stubs: Vec<String>,
    /// The compile errors of the apps under test (`compile_errors`).
    compile_errors: Vec<CompileError>,
}

/// An error in the code of an app under test that stops it from compiling: one the AL compiler gives (an `AL` code
/// from `al_sema::alcodes` or the checker, pinned by the must-not-compile corpus), or source the parser does not accept
/// (`code` is empty). Dependency apps are not judged: the AL compiler has already vetted them.
#[derive(Debug, Clone, PartialEq)]
pub struct CompileError {
    /// The app, as the caller numbered the apps of the run.
    pub app: usize,
    /// The file, relative to its app.
    pub file: String,
    /// 1-based line and column of the error.
    pub line: usize,
    pub column: usize,
    /// The AL compiler's code (`AL0118`); empty for a parse error.
    pub code: String,
    pub message: String,
}

/// The compile errors of the workspace apps of a compiled program, in file order; `order[i]` is the caller's number of
/// program app `i`.
pub(crate) fn compile_errors(ws: &al_sema::Workspace, lowered: &Lowered, order: &[usize]) -> Vec<CompileError> {
    let parse = ws.diagnostics.iter().filter(|d| d.code == "AR0000");
    let mut out: Vec<CompileError> = lowered
        .al_diagnostics
        .iter()
        .chain(parse)
        .filter(|d| ws.kinds.get(d.app).is_some_and(|k| *k != al_sema::AppKind::Dependency))
        .map(|d| {
            let text = ws
                .files
                .iter()
                .find(|f| f.app == d.app && f.path == d.file)
                .and_then(|f| ws.apps.get(f.app).and_then(|a| a.files.get(f.file_in_app)))
                .map_or("", |(_, t)| t.as_str());
            let before = &text[..d.offset.min(text.len())];
            let line = before.matches('\n').count() + 1;
            let column = before.rsplit('\n').next().map_or(0, |l| l.chars().count()) + 1;
            CompileError {
                app: order.get(d.app).copied().unwrap_or(d.app),
                file: d.file.clone(),
                line,
                column,
                code: if d.code == "AR0000" {
                    String::new()
                } else {
                    d.code.to_string()
                },
                message: d.message.clone(),
            }
        })
        .collect();
    out.sort_by(|a, b| (a.app, &a.file, a.line, a.column).cmp(&(b.app, &b.file, b.line, b.column)));
    out.dedup();
    out
}

/// One test as the run left it, in the order the test codeunits were scheduled.
#[derive(Debug, Clone)]
pub struct TestResult {
    /// Index into the `SuiteRun`s of the run.
    pub suite: usize,
    /// Index of the app (as given to the compiler) that holds the test codeunit.
    pub app: usize,
    pub codeunit: u32,
    pub codeunit_name: String,
    pub test: String,
    pub outcome: Outcome,
    /// For a failure with an AL error: the call stack of the error, innermost frame first, one per
    /// line (`GetLastErrorCallStack` form). Empty otherwise.
    pub call_stack: String,
    /// The unprobed assumptions the test ran through (`al_builtins::assumptions`), in registry order. A test
    /// that passed with a non-empty list is a flagged pass.
    pub assumptions: Vec<Assumption>,
    /// The wall time the test took; zero for a test that was not started.
    pub time: std::time::Duration,
    /// With `RunOptions::coverage`: the statements the test ran, as (position id, runs), sorted by id. Position ids
    /// index `CorpusRuntime::coverage_statements`' `pos` and `Program::positions`.
    pub coverage: Vec<(u32, u32)>,
    /// With `RunOptions::execute`: the text of the `Message` calls the run made, in order.
    pub messages: Vec<String>,
    /// With `RunOptions::capture`: what the capture saw while the test ran.
    pub captured: Option<crate::events::CapturedAt>,
    /// With `RunOptions::impact_trace`: what the test raised and touched, as numbers of
    /// `CorpusRuntime::impact_keys`, sorted. The first test of a codeunit also holds what ran before it.
    pub impact: Vec<u32>,
    /// `--tdd` (ADR-051): the signatures of the stubs that ran while the test ran, sorted. A test with any is stubbed,
    /// whatever its outcome. Empty without `--tdd`.
    pub stubs: Vec<String>,
}

/// Wall time of the phases of a run (`--timings`).
#[derive(Debug, Clone, Default)]
pub struct Phases {
    /// The ids of the assumptions the install baseline relied on (a JUnit suite-level property).
    pub baseline_assumptions: Vec<String>,
    /// Install roots that ran to their end, and how many there were; `None` when no baseline ran.
    pub baseline_roots: Option<(usize, usize)>,
    pub compile: std::time::Duration,
    pub baseline: std::time::Duration,
    pub tests: std::time::Duration,
    /// The tests phase by run, in the order of the `SuiteRun`s (name, wall time).
    pub suites: Vec<(String, std::time::Duration)>,
    /// The parts of the compile.
    pub compile_parts: al_sema::lower::CompileTimings,
    /// What the compile cache did, when the run used it.
    pub cache: Option<crate::compile_cache::CacheReport>,
    /// Building the engine from the lowered program.
    pub engine: std::time::Duration,
    /// The install baseline did not run because no selected test can observe it (`observe.rs`).
    pub baseline_skipped: bool,
    /// Why the baseline ran although the rule was tried: the first construct a selected test
    /// reaches that could observe it. `None`: the rule was not tried (no dependency app,
    /// `AL_FORCE_BASELINE`, a custom test runner) or the baseline was skipped.
    pub baseline_why: Option<String>,
    /// The compile cache was asked for and not used, because the selected tests were compiled
    /// first without the baseline and nothing needed the cache.
    pub compile_cache_skipped: bool,
    /// With such a compile and lazily read dependency apps: where the scan of their sources came from,
    /// `read` (the index in the cache folder) or `built` (scanned now, index saved).
    pub source_index: Option<&'static str>,
    /// The test codeunits (object id, name) that took longest, slowest first (at most
    /// [`SLOWEST`]).
    pub slowest: Vec<CodeunitTime>,
    /// The tests ran on a lean compile (`al_sema::check::Roots::lean`): (workspace objects it left out, workspace
    /// objects). `None`: the compile was complete.
    pub lean: Option<(usize, usize)>,
    /// A lean run met a routine the lean compile had left out, so the tests ran again on the complete compile; the
    /// outcomes are the second run's.
    pub lean_redone: bool,
    /// How many times the apps were compiled for the run: 2 when a compile without the baseline was thrown away
    /// because the rule said "may observe", or when a lean run ran again.
    pub compiles: usize,
}

impl Phases {
    /// One line for `--timings` about a lean compile or a compile that was done twice, `None` when there was one
    /// complete compile.
    pub fn compile_line(&self) -> Option<String> {
        match (self.lean, self.lean_redone) {
            (_, true) => Some(
                "compile: lean, and a selected test met an object the lean compile left out, so the tests ran again on the complete compile"
                    .into(),
            ),
            (Some((out, all)), false) => Some(format!(
                "compile: lean, {out} of {all} workspace objects left out (no selected test names them)"
            )),
            (None, false) if self.compiles > 1 => Some(
                "compile: twice, the compile without the install baseline was thrown away (a selected test may observe the baseline)"
                    .into(),
            ),
            (None, false) => None,
        }
    }

    /// One line for `--timings`: whether the install baseline ran, and if the rule that skips it
    /// was tried and said no, the first construct a selected test reaches that could observe it.
    /// `None` when there was nothing to say (no dependency app).
    pub fn baseline_line(&self) -> Option<String> {
        if self.baseline_skipped {
            Some(format!(
                "install baseline: not run, no selected test can observe it{}",
                match (self.compile_cache_skipped, self.source_index) {
                    (true, Some(how)) => format!(
                        "; compile cache: not used, compiling what the tests reach was enough; source index: {how}"
                    ),
                    (true, None) => "; compile cache: not used, compiling what the tests reach was enough".to_string(),
                    (false, _) => String::new(),
                }
            ))
        } else {
            self.baseline_why
                .as_ref()
                .map(|why| format!("install baseline: ran, a selected test may observe it: {why}"))
        }
    }
}

/// How long one test codeunit took.
#[derive(Debug, Clone)]
pub struct CodeunitTime {
    pub id: u32,
    pub name: String,
    /// Wall time of the whole codeunit.
    pub took: std::time::Duration,
    /// The part of `took` spent starting .NET sidecar processes (ADR-014): start-up of a helper
    /// process, not the runtime. A budget on the runtime subtracts it.
    pub sidecar_start: std::time::Duration,
}

/// How many test codeunits `--timings` names.
pub const SLOWEST: usize = 8;

/// A test codeunit to run: its object, id and the isolation of the runner it belongs to.
pub(crate) struct Unit {
    si: usize,
    oi: usize,
    id: u32,
    isolation: TestIsolation,
    /// Run without putting the install baseline back (`AL_CHECK_BASELINE_RULE`).
    without_baseline: bool,
    /// Count the statements each test runs (`RunOptions::coverage`).
    coverage: bool,
    /// Run the `OnRun` trigger as the one test (`RunOptions::execute`).
    execute: bool,
    /// What to capture of each test (`RunOptions::capture`).
    capture: al_vm::CaptureConfig,
    /// See `Scratch::order`.
    order: Arc<Vec<usize>>,
    hooks: crate::events::Hooks,
    /// See `Scratch::clock_start`.
    clock_start: Option<i64>,
    /// Note what each test raised and touched (`RunOptions::impact_trace`).
    impact: bool,
}

/// What a scratch run or a capture leaves besides outcomes, per test.
#[derive(Default)]
pub(crate) struct Scratch {
    /// What to capture (`RunOptions::capture`).
    capture: al_vm::CaptureConfig,
    /// The apps of the program, as the caller numbered them (to place what a capture names).
    order: Arc<Vec<usize>>,
    /// The text of the `Message` calls of an `execute` run.
    messages: Vec<(String, Vec<String>)>,
    /// What the capture saw.
    captured: Vec<(String, crate::events::CapturedAt)>,
    /// Where the run clock of the codeunit starts, in Unix milliseconds (`RunOptions::clock_starts`); `None`: now.
    clock_start: Option<i64>,
    /// Note what each test raised and touched (`RunOptions::impact_trace`).
    trace: bool,
    /// What each test raised and touched, as keys (`impact_trace::keys`).
    traces: Vec<(String, Vec<String>)>,
    /// `--tdd`: the stubs each test reached (`TestResult::stubs`).
    stubbed: Vec<(String, Vec<String>)>,
}

/// What a test codeunit left: each test's outcome, the call stack of each AL error that failed one, how long
/// each test that ran took (wall time), and the unprobed assumptions each test ran through.
type UnitOutcomes = (
    Vec<(String, Outcome)>,
    Vec<(String, String)>,
    Vec<(String, std::time::Duration)>,
    Vec<(String, Vec<Assumption>)>,
    Vec<(String, Vec<(u32, u32)>)>,
    Scratch,
);

pub(crate) struct UnitResult {
    si: usize,
    oi: usize,
    id: u32,
    /// The outcome of each test, or the message of a Rust panic.
    result: Result<UnitOutcomes, String>,
    took: std::time::Duration,
    sidecar_start: std::time::Duration,
}

/// The `[Test]` routines of a codeunit.
fn test_routines(o: &al_ir::Object) -> impl Iterator<Item = &al_ir::Routine> {
    o.routines
        .iter()
        .filter(|r| r.attributes.iter().any(|a| a.name.eq_ignore_ascii_case("Test")))
}

/// Runs one test codeunit. A Rust panic is a runtime bug; it is contained to this codeunit.
fn run_unit(
    engine: &Arc<Engine>,
    lowered: &Lowered,
    baseline: Option<&al_vm::Baseline>,
    u: &Unit,
    observe: Option<&mut Observer>,
    limit: Option<std::time::Duration>,
) -> UnitResult {
    let started = std::time::Instant::now();
    let clock_start = u.clock_start;
    crate::dotnet::take_start_time();
    let baseline = baseline.filter(|_| !u.without_baseline);
    crate::events::install(u.hooks.clone());
    // A crash of the process names this codeunit (and, once it starts, the test).
    crate::crash::running(u.id, "");
    // A cancelled run starts no further codeunit.
    if crate::events::cancelled() {
        crate::crash::idle();
        crate::events::uninstall();
        return UnitResult {
            si: u.si,
            oi: u.oi,
            id: u.id,
            result: Ok((
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Scratch::default(),
            )),
            took: started.elapsed(),
            sidecar_start: std::time::Duration::ZERO,
        };
    }
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut stacks = Vec::new();
        let mut times = Vec::new();
        let mut hits = Vec::new();
        let mut scratch = Scratch {
            capture: u.capture,
            order: u.order.clone(),
            clock_start,
            trace: u.impact,
            ..Default::default()
        };
        if u.coverage {
            crate::coverage::begin();
        }
        let done = run_codeunit_core(
            engine,
            lowered,
            u.oi,
            baseline,
            u.isolation,
            observe,
            &mut stacks,
            &mut times,
            &mut hits,
            &mut scratch,
            limit,
            u.execute,
        );
        (done, stacks, times, hits, crate::coverage::finish(), scratch)
    }))
    .map_err(|p| {
        p.downcast_ref::<String>()
            .cloned()
            .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_default()
    });
    // Tests the loop did not report as they finished (a codeunit refused as a whole, a panic) are reported now.
    if crate::events::wants_events() {
        let have = crate::events::reported();
        let name = lowered.program.objects[u.oi].name.clone();
        let event = |test: &str, outcome: Outcome, call_stack: String, time, coverage| crate::events::TestEvent {
            codeunit: u.id,
            codeunit_name: name.clone(),
            test: test.to_string(),
            outcome,
            call_stack,
            time,
            coverage,
            captured: None,
            stubs: Vec::new(),
        };
        match &result {
            Ok((done, stacks, times, _, covs, _)) => {
                for (test, outcome) in done.iter().filter(|(t, _)| !have.contains(t)) {
                    let stack = stacks.iter().find(|(n, _)| n == test).map(|(_, s)| s.clone());
                    let time = times
                        .iter()
                        .find(|(n, _)| n == test)
                        .map_or(std::time::Duration::ZERO, |(_, d)| *d);
                    let cov = covs
                        .iter()
                        .find(|(n, _)| n == test)
                        .map(|(_, c)| c.clone())
                        .unwrap_or_default();
                    crate::events::report(event(test, outcome.clone(), stack.unwrap_or_default(), time, cov));
                }
            }
            Err(msg) => {
                for r in test_routines(&lowered.program.objects[u.oi]).filter(|r| !have.contains(&r.name)) {
                    let why = Outcome::NotRun(format!("runtime panic (bug): {msg}"));
                    crate::events::report(event(
                        &r.name,
                        why,
                        String::new(),
                        std::time::Duration::ZERO,
                        Vec::new(),
                    ));
                }
            }
        }
    }
    let took = started.elapsed();
    let sidecar_start = crate::dotnet::take_start_time();
    crate::crash::idle();
    crate::events::uninstall();
    UnitResult {
        si: u.si,
        oi: u.oi,
        id: u.id,
        result,
        took,
        sidecar_start,
    }
}

/// The codeunit a scratch run executes in the app `app` (the program's numbering; `None`: any app that is not a
/// dependency): of the codeunits with an `OnRun` trigger the one with the lowest id, a codeunit that declares no test
/// before one that does.
fn on_run_target(lowered: &Lowered, app: Option<usize>) -> Option<usize> {
    lowered
        .program
        .objects
        .iter()
        .enumerate()
        .filter(|(oi, o)| {
            o.is_kind(al_ir::ObjKind::Codeunit)
                && o.id.is_some()
                && !lowered.dependency[*oi]
                && app.is_none_or(|a| o.app as usize == a)
                && o.routines
                    .iter()
                    .any(|r| r.is_trigger && r.name.eq_ignore_ascii_case("OnRun"))
        })
        .min_by_key(|(_, o)| {
            let is_test = test_routines(o).next().is_some()
                || o.properties
                    .iter()
                    .any(|(k, v)| k.eq_ignore_ascii_case("Subtype") && v.eq_ignore_ascii_case("Test"));
            (is_test, o.id)
        })
        .map(|(oi, _)| oi)
}

/// Whether object `oi` is a test codeunit that `suite` runs under the selection `only`.
fn is_selected_test(lowered: &Lowered, oi: usize, suite: &SuiteRun, only: &Option<Vec<(usize, u32)>>) -> bool {
    let o = &lowered.program.objects[oi];
    o.is_kind(al_ir::ObjKind::Codeunit)
        && !lowered.dependency[oi]
        && suite.app.is_none_or(|a| o.app as usize == a)
        && only
            .as_ref()
            .is_none_or(|only| o.id.is_some_and(|id| only.contains(&(o.app as usize, id))))
        && o.properties
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("Subtype") && v.trim().eq_ignore_ascii_case("Test"))
}

/// The objects (indices) of every test codeunit the suites run.
fn selected_tests(lowered: &Lowered, suites: &[SuiteRun], only: &Option<Vec<(usize, u32)>>) -> Vec<usize> {
    (0..lowered.program.objects.len())
        .filter(|&oi| suites.iter().any(|s| is_selected_test(lowered, oi, s, only)))
        .collect()
}

/// How many threads run test codeunits. `AL_TEST_THREADS` sets it (1 runs them one after the
/// other on the calling thread); the default is the number of cores.
fn test_workers() -> usize {
    std::env::var("AL_TEST_THREADS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(1, |n| n.get()))
        .clamp(1, 64)
}

/// Stack of a test thread: the VM recurses on the native stack per AL call (`MAX_CALL_DEPTH`).
const TEST_STACK: usize = 256 << 20;

/// Drops `value` on another thread. The process does not wait for it when it ends.
pub fn retire<T: Send + 'static>(value: T) {
    std::thread::spawn(move || drop(value));
}

/// Holds `value` and [`retire`]s it when it goes out of scope, so the end of a function that
/// holds a big runtime does not wait for the drop.
pub struct Retiring<T: Send + 'static>(Option<T>);

impl<T: Send + 'static> Retiring<T> {
    pub fn new(value: T) -> Self {
        Retiring(Some(value))
    }
}

impl<T: Send + 'static> std::ops::Deref for Retiring<T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.0.as_ref().expect("held until dropped")
    }
}

impl<T: Send + 'static> Drop for Retiring<T> {
    fn drop(&mut self) {
        if let Some(v) = self.0.take() {
            retire(v);
        }
    }
}

mod blocked;
mod cache;
mod compiled;
mod corpus;
mod handlers;
mod run;
mod runner_reset;
#[cfg(test)]
mod tests;
pub(crate) mod written;

pub use self::blocked::*;
pub use self::cache::*;
pub use self::compiled::*;
use self::handlers::*;
pub use self::run::*;
