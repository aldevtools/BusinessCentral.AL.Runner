//! What a compile produces and keeps (`Compiled`), the warm slot (`Warm`), and what a run asks for.
use super::*;

/// What a run is asked to do beyond running everything.
#[derive(Debug, Clone, Default)]
pub struct RunOptions {
    /// Only these test codeunits (app index of the compile, object id) are roots of the compile
    /// and are run. `None`: all of them.
    pub only: Option<Vec<(usize, u32)>>,
    /// Record where tests are refused or fail (`--explain`).
    pub trace: bool,
    /// Rows of a restored company, read when a session first touches a table (ADR-020). `None`:
    /// a run without test data, as every corpus run is.
    pub test_data: Option<al_host::testdata::TestData>,
    /// Use the compile cache of the dependency apps (ADR-022). `None`: the demand-driven compile
    /// of every run, which is the reference path.
    pub compile_cache: Option<crate::compile_cache::CompileCache>,
    /// A test that runs longer than this is stopped and reported as not run. `None`: no limit.
    pub test_time_limit: Option<std::time::Duration>,
    /// Per app (in the order given to the run): a dependency app whose files hold no text yet, and how
    /// to read them (`deps::load_project_with`). The run reads what it needs; a missing entry means
    /// the app has its text.
    pub lazy: Vec<Option<al_sema::LazySource>>,
    /// `al-runner test --isolation`: the isolation of every test codeunit, instead of the runner's property (`None`: the runner's,
    /// else Codeunit).
    pub isolation: Option<TestIsolation>,
    /// Count how often each statement of the workspace's apps runs, per test (`al-runner test --coverage`).
    /// Off, a run pays one branch per statement.
    pub coverage: bool,
    /// A scratch run (`al-runner serve`'s `execute`): the `OnRun` trigger of one codeunit per `SuiteRun`'s app runs
    /// instead of the tests, and the text of its `Message` calls is kept in `TestResult::messages`.
    pub execute: bool,
    /// Capture the values of the locals and the loops of the apps' code, per test (`al-runner serve`, ADR-030). Off,
    /// a run pays nothing for it.
    pub capture: al_vm::CaptureConfig,
    /// Callbacks while the run goes on: a line per test as it finishes, and a flag that stops the run after the
    /// current test (`al-runner serve`).
    pub hooks: crate::events::Hooks,
    /// Where the run clock of a test codeunit starts (codeunit id to Unix milliseconds), instead of now: the corpus
    /// oracle recorded values derived from `Today` on the day real BC ran, so the harness starts each codeunit at the
    /// time of that codeunit's suite in the oracle.
    pub clock_starts: Option<Arc<std::collections::HashMap<u32, i64>>>,
    /// Do not print the `baseline:` detail lines to stderr (`al-runner test` prints them only with `--verbose`). The counts
    /// stay in `Phases::baseline_roots`. Default `false`: print, as the corpus job does.
    pub quiet_baseline: bool,
    /// `al-runner impact-index build` (ADR-043 step 2): note per test what it raised and touched
    /// (`TestResult::impact`), and what the install baseline raised and touched (`CorpusRuntime::impact_baseline`).
    /// The baseline then runs instead of being read from the compile cache. Off, a run pays one branch per raise and
    /// record operation.
    pub impact_trace: bool,
    /// Called once the compile is done and the install baseline has run, before any test runs: it may narrow the test
    /// codeunits to run (an impact run that selects the codeunits that reach the extension's impact points). The
    /// baseline then runs with the trace on (`NarrowInput::baseline`) instead of being read from the compile cache.
    /// `None`: the selection stays.
    pub narrow: Option<Narrow>,
    /// `al-runner test --tdd` (ADR-051): a call in a test app of a procedure that does not exist runs as a stub.
    pub tdd: bool,
    /// `al-runner debug` (ADR-048): called once the compile is done and the install baseline has run, before any test
    /// runs; it returns when the tests may start. The tests then run on one thread, one after the other, with the
    /// debugger of `hooks.debugger`.
    pub debug_ready: Option<DebugReadyHook>,
}

/// What [`RunOptions::debug_ready`] sees of the compile.
pub struct DebugReady<'a> {
    pub program: &'a Arc<al_ir::Program>,
    /// Per app of the program: its index as the caller gave it.
    pub order: &'a [usize],
    /// The first error the AL compiler would give for the workspace's code (code, file and message), if any.
    pub first_error: Option<String>,
}

/// See [`RunOptions::debug_ready`].
#[derive(Clone)]
pub struct DebugReadyHook(pub Arc<dyn Fn(&DebugReady) + Send + Sync>);

impl std::fmt::Debug for DebugReadyHook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DebugReadyHook(..)")
    }
}

/// What [`RunOptions::narrow`] sees of the compile.
pub struct NarrowInput<'a> {
    pub program: &'a al_ir::Program,
    /// The lowering `program` is part of: its call graph and why routines are refused (static selection).
    pub lowered: &'a Lowered,
    /// Per app of the program: its index as the caller gave it.
    pub order: &'a [usize],
    /// The test codeunits the run would run (caller's app index, codeunit id); `None`: all.
    pub only: Option<&'a [(usize, u32)]>,
    /// What the install baseline raised and touched (`impact_trace::keys`); `None`: it did not run.
    pub baseline: Option<&'a [String]>,
}

/// The test codeunits to run instead, as (caller's app index, codeunit id); `None` keeps the selection.
pub type NarrowFn = dyn Fn(&NarrowInput) -> Option<Vec<(usize, u32)>> + Send + Sync;

/// See [`RunOptions::narrow`].
#[derive(Clone)]
pub struct Narrow(pub Arc<NarrowFn>);

impl std::fmt::Debug for Narrow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Narrow(..)")
    }
}

/// What the compile phase of a run leaves for its run phase (`CorpusRuntime::compile_suites`).
pub struct Compiled {
    #[allow(dead_code)] // kept alive with `lowered`: a warm server keeps both
    pub(crate) ws: al_sema::Workspace,
    pub(crate) lowered: Lowered,
    pub(crate) cache_report: Option<crate::compile_cache::CacheReport>,
    pub(crate) baseline_wanted: bool,
    pub(crate) baseline: bool,
    pub(crate) baseline_note: Option<String>,
    pub(crate) order: Vec<usize>,
    pub(crate) suites: Vec<SuiteRun>,
    pub(crate) opts_only: Option<Vec<(usize, u32)>>,
    pub(crate) compile_cache_skipped: bool,
    pub(crate) source_index: Option<&'static str>,
    pub(crate) compile_time: std::time::Duration,
    /// The compile is lean (`al_sema::check::Roots::lean`): (workspace objects it left out, workspace objects).
    pub(crate) lean: Option<(usize, usize)>,
    /// How many compiles it took (`Phases::compiles`).
    pub(crate) compiles: usize,
    /// What the incremental update needs, when the compile used the dependency cache (`update_files`).
    pub(crate) incr: Option<Incr>,
}

/// What [`Compiled::update_files`] needs of the compile it renews.
pub(crate) struct Incr {
    pub(crate) layers: crate::compile_cache::Layers,
    pub(crate) roots: al_sema::check::Roots,
    pub(crate) catalog: al_builtins::catalog::Catalog,
    /// Whether the weighing of the install baseline was on (`compile_suites`).
    pub(crate) try_skip: bool,
    pub(crate) symbols: BTreeSet<String>,
}

/// A workspace file whose text is not the one the compile has.
pub struct FileChange<'a> {
    /// The app, as the caller numbered the apps of the run.
    pub app: usize,
    pub path: &'a str,
    pub text: &'a str,
}

impl Compiled {
    /// The apps of the compile in the order the caller gave them, with their kinds: what another compile of the same
    /// run starts from. A file of a lazily read app that was not read yet is read when that compile needs it.
    pub(crate) fn into_apps(self) -> (Vec<al_sema::workspace::AppSource>, Vec<al_sema::AppKind>) {
        let Compiled {
            mut ws, lowered, order, ..
        } = self;
        let apps = std::mem::take(&mut ws.apps);
        let kinds = std::mem::take(&mut ws.kinds);
        retire((ws, lowered));
        // `order` maps the program's app index to the caller's.
        let mut slots: Vec<Option<(al_sema::workspace::AppSource, al_sema::AppKind)>> =
            (0..order.len()).map(|_| None).collect();
        for ((app, kind), &caller) in apps.into_iter().zip(kinds).zip(&order) {
            slots[caller] = Some((app, kind));
        }
        slots.into_iter().map(|s| s.expect("each app once")).unzip()
    }

    /// The lowered program of the compile.
    pub fn lowered(&self) -> &Lowered {
        &self.lowered
    }

    /// Whether `other` holds the same lowering as this one: the program, why routines are refused, the
    /// calls and the diagnostics. An error names the first thing that differs. What the incremental update
    /// has to give (ADR-029).
    pub fn same_lowering_as(&self, other: &Compiled) -> Result<(), String> {
        let (a, b) = (&self.lowered, &other.lowered);
        if a.program != b.program {
            let objects = a
                .program
                .objects
                .iter()
                .zip(&b.program.objects)
                .position(|(x, y)| x != y);
            return Err(format!(
                "the program differs (first object that differs: {:?}; positions equal: {}; counts {} and {})",
                objects.map(|i| format!("{} {}", a.program.objects[i].kind, a.program.objects[i].name)),
                a.program.positions == b.program.positions,
                a.program.objects.len(),
                b.program.objects.len()
            ));
        }
        if a.unsupported != b.unsupported {
            return Err("the reasons routines are refused differ".into());
        }
        if a.calls != b.calls {
            return Err("the calls differ".into());
        }
        if a.al_diagnostics != b.al_diagnostics {
            return Err(format!(
                "the diagnostics differ: {:?} and {:?}",
                a.al_diagnostics, b.al_diagnostics
            ));
        }
        if (&a.dependency, &a.provided) != (&b.dependency, &b.provided) {
            return Err("the object flags differ".into());
        }
        if (self.baseline, &self.baseline_note) != (other.baseline, &other.baseline_note) {
            return Err(format!(
                "the baseline decision differs: {:?} {:?} and {:?} {:?}",
                self.baseline, self.baseline_note, other.baseline, other.baseline_note
            ));
        }
        Ok(())
    }

    /// Takes new texts of workspace files into the compile without compiling everything again
    /// (ADR-029): when the files differ from the old ones only in the code of routines of codeunits,
    /// only the objects of those files are checked and lowered again, and the result is what a complete
    /// compile of the new texts gives. An error says why not (a declaration changed, another kind of
    /// object, the compile did not use the dependency cache, ...); the compile is then in no usable
    /// state and the caller compiles again.
    pub fn update_files(&mut self, changes: &[FileChange]) -> Result<(), String> {
        let started = std::time::Instant::now();
        let Compiled {
            ws,
            lowered,
            baseline,
            baseline_note,
            order,
            suites,
            opts_only,
            incr,
            compile_time,
            ..
        } = self;
        let incr = incr.as_ref().ok_or("the compile did not use the dependency cache")?;
        let mut files = Vec::new();
        for c in changes {
            let app = order
                .iter()
                .position(|&o| o == c.app)
                .ok_or("an app the compile does not have")?;
            let fi = ws
                .files
                .iter()
                .position(|f| f.app == app && f.path == c.path)
                .ok_or("a file the compile does not have")?;
            files.push((fi, c.text));
        }
        let mut dirty: Vec<usize> = (0..ws.objects.len())
            .filter(|&oi| files.iter().any(|&(fi, _)| ws.objects[oi].file == fi))
            .collect();
        let t_replace = std::time::Instant::now();
        // (A text the update cannot take leaves the workspace half changed: the caller compiles all of it again.)
        let mut declared = false;
        for &(fi, text) in &files {
            declared |= ws.replace_file_declared(fi, text.to_string())?.1;
        }
        let t_replace = t_replace.elapsed();
        if declared {
            // A declaration changed: the objects that name the changed ones may have read it, so they are
            // checked and lowered again with them (ADR-033 step 5).
            al_sema::incremental::eligible_for_declarations(ws, &dirty)?;
            let limit = ws.objects.len() / 5 + 20;
            let readers = al_sema::incremental::readers(ws, &dirty, limit)?;
            dirty.extend(readers);
            dirty.sort_unstable();
        } else {
            al_sema::incremental::eligible(ws, &dirty)?;
        }
        // A complete compile of tests that look free of data would try the demand-driven compile first.
        if incr.try_skip && crate::observe::text_looks_pure(&ws.apps, &ws.kinds, &incr.symbols, opts_only.as_deref()) {
            return Err("the tests look free of data: a complete compile weighs the baseline first".into());
        }
        let src = incr.layers.stack().map_err(|why| format!("compile cache: {why}"))?;
        al_sema::lower::relower_objects(ws, &incr.catalog, incr.roots.clone(), &src, lowered, &dirty)?;
        if incr.try_skip {
            let selected = selected_tests(lowered, suites, opts_only);
            *baseline_note = crate::observe::may_observe_baseline(lowered, &selected);
            *baseline = baseline_note.is_some();
        }
        // The source lines of an app that includes its source in the symbols.
        {
            let program = std::sync::Arc::make_mut(&mut lowered.program);
            for sf in &mut program.sources {
                let Some(_) = program.apps.get(sf.app as usize).filter(|a| a.include_source) else {
                    continue;
                };
                if !files
                    .iter()
                    .any(|&(fi, _)| ws.files[fi].path == sf.path && ws.files[fi].app == sf.app as usize)
                {
                    continue;
                }
                if let Some((_, text)) = ws
                    .apps
                    .get(sf.app as usize)
                    .and_then(|a| a.files.iter().find(|(p, _)| *p == sf.path))
                {
                    sf.lines = text.split('\n').map(|l| l.trim_end_matches('\r').to_string()).collect();
                }
            }
        }
        if !lowered.al_diagnostics.is_empty() {
            eprintln!(
                "compile: {} error(s) the AL compiler would give for the workspace's code",
                lowered.al_diagnostics.len()
            );
            for d in lowered.al_diagnostics.iter().take(100) {
                eprintln!("compile: error {} {}: {}", d.code, d.file, d.message);
            }
        }
        *compile_time = started.elapsed();
        if std::env::var_os("ALRUNNER_INCR_TIMINGS").is_some() {
            eprintln!(
                "incremental: replacing {} file(s) {:.3} s, all {:.3} s",
                files.len(),
                t_replace.as_secs_f64(),
                compile_time.as_secs_f64()
            );
        }
        Ok(())
    }
}

/// What a warm server keeps between requests (ADR-028): the last compile, and which test codeunits it was compiled for.
#[derive(Default)]
pub struct Warm {
    pub(crate) slot: Option<WarmSlot>,
}

pub(crate) struct WarmSlot {
    pub(crate) key: String,
    /// The text of every workspace file the compile was made from: (app, path, hash of the text).
    pub(crate) files: Vec<(usize, String, u64)>,
    /// The codeunits the compile reached, by (app, id) as the caller gave them; `None`: all of them.
    pub(crate) compiled_for: Option<std::collections::BTreeSet<(usize, u32)>>,
    pub(crate) compiled: Compiled,
}

/// What a warm run did about the compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarmUse {
    /// The compile of an earlier request was used as it was.
    Reused,
    /// The code of some routines changed: only the objects of those files were compiled again (ADR-029).
    Updated,
    /// The inputs changed or the earlier compile did not reach the selected tests: compiled again.
    Compiled,
}

impl Warm {
    pub fn new() -> Warm {
        Warm::default()
    }

    /// The compile that is kept.
    pub fn compiled(&self) -> Option<&Compiled> {
        self.slot.as_ref().map(|s| &s.compiled)
    }

    /// Runs like [`CorpusRuntime::try_run_suites_with`], keeping the compile for the next call. `key` names everything
    /// the compile depends on except the selection and the text of the workspace files (the dependency apps, the
    /// symbols, the target, the names of the files); `files` are those texts. With the same key and a selection the
    /// earlier compile reached, nothing is compiled when the texts are the same, and only what changed is when they
    /// differ in the code of routines ([`Compiled::update_files`]). A different selection compiles for the earlier and
    /// the new one together, so a client that asks for one test after another converges.
    #[allow(clippy::too_many_arguments)]
    pub fn run(
        &mut self,
        key: &str,
        files: &[FileChange<'_>],
        build: impl FnOnce() -> (Vec<al_sema::workspace::AppSource>, Vec<al_sema::AppKind>, Vec<Manifest>),
        symbols: &BTreeSet<String>,
        target: Option<&al_version::Target>,
        dotnet: al_vm::dotnet::DotNetConfig,
        suites: &[SuiteRun],
        opts: &RunOptions,
    ) -> Result<(CorpusRuntime, WarmUse), String> {
        use std::hash::{Hash, Hasher};
        let mut build = Some(build);
        let hashes: Vec<(usize, String, u64)> = files
            .iter()
            .map(|f| {
                let mut h = std::collections::hash_map::DefaultHasher::new();
                f.text.hash(&mut h);
                (f.app, f.path.to_string(), h.finish())
            })
            .collect();
        let wanted: Option<std::collections::BTreeSet<(usize, u32)>> =
            opts.only.as_ref().map(|v| v.iter().copied().collect());
        let covers = |have: &Option<std::collections::BTreeSet<(usize, u32)>>| match (have, &wanted) {
            (None, _) => true,
            (Some(h), Some(w)) => w.is_subset(h),
            (Some(_), None) => false,
        };
        let mut used = WarmUse::Reused;
        let mut reusable = false;
        let mut broken = false;
        if let Some(slot) = self.slot.as_mut().filter(|s| s.key == key && covers(&s.compiled_for)) {
            let same_files = slot.files.len() == hashes.len()
                && slot.files.iter().zip(&hashes).all(|(a, b)| a.0 == b.0 && a.1 == b.1);
            if same_files {
                let changed: Vec<FileChange> = files
                    .iter()
                    .zip(slot.files.iter().zip(&hashes))
                    .filter(|(_, (old, new))| old.2 != new.2)
                    .map(|(f, _)| FileChange {
                        app: f.app,
                        path: f.path,
                        text: f.text,
                    })
                    .collect();
                if changed.is_empty() {
                    reusable = true;
                } else if std::env::var_os("ALRUNNER_NO_INCREMENTAL").is_none() {
                    // (ALRUNNER_NO_INCREMENTAL=1 compiles all of it when a text changed, to measure what the update saves.)
                    match slot.compiled.update_files(&changed) {
                        Ok(()) => {
                            slot.files = hashes.clone();
                            reusable = true;
                            used = WarmUse::Updated;
                            // ALRUNNER_CHECK_INCREMENTAL=1: a complete compile of the same texts must hold the same
                            // lowering. A run that fails here has found a defect of the update.
                            if std::env::var_os("ALRUNNER_CHECK_INCREMENTAL").is_some() {
                                let (apps, kinds, manifests) = build.take().expect("built once")();
                                let compile_opts = RunOptions {
                                    only: slot.compiled_for.as_ref().map(|u| u.iter().copied().collect()),
                                    ..opts.clone()
                                };
                                let full = CorpusRuntime::compile_suites(
                                    apps,
                                    kinds,
                                    &manifests,
                                    symbols,
                                    target,
                                    suites,
                                    &compile_opts,
                                )?;
                                slot.compiled.same_lowering_as(&full).map_err(|why| {
                                    format!("the incremental compile differs from the complete one: {why}")
                                })?;
                                retire(full);
                            }
                        }
                        Err(why) => {
                            eprintln!("compile: all of it again ({why})");
                            broken = true;
                        }
                    }
                }
            }
        }
        if broken {
            if let Some(old) = self.slot.take() {
                retire(old.compiled);
            }
        }
        if !reusable {
            used = WarmUse::Compiled;
            let union = match (self.slot.as_ref().filter(|s| s.key == key), &wanted) {
                (Some(s), Some(w)) => s.compiled_for.as_ref().map(|h| h.union(w).copied().collect()),
                _ => wanted.clone(),
            };
            let (apps, kinds, manifests) = build.take().expect("built once")();
            let compile_opts = RunOptions {
                only: union.as_ref().map(|u| u.iter().copied().collect()),
                ..opts.clone()
            };
            // The old compile goes first: two of them are never in memory together.
            if let Some(old) = self.slot.take() {
                retire(old.compiled);
            }
            let compiled =
                CorpusRuntime::compile_suites(apps, kinds, &manifests, symbols, target, suites, &compile_opts)?;
            self.slot = Some(WarmSlot {
                key: key.to_string(),
                files: hashes,
                compiled_for: union,
                compiled,
            });
        }
        let slot = self.slot.as_ref().expect("a compile is kept");
        let rt = CorpusRuntime::run_compiled(&slot.compiled, target, dotnet, opts);
        Ok((rt, used))
    }

    /// Drops what is kept.
    pub fn clear(&mut self) {
        if let Some(old) = self.slot.take() {
            retire(old.compiled);
        }
    }
}

/// A test runner run over the test codeunits of some apps (the unit of an oracle file).
#[derive(Debug, Clone)]
pub struct SuiteRun {
    /// The name the report uses; empty for the main corpus app.
    pub name: String,
    /// The test codeunits of the app at this index of the apps given to the compiler. `None`:
    /// those of every app that is not a dependency.
    pub app: Option<usize>,
    /// The runner codeunit whose `TestIsolation` the run uses. `None`: Codeunit, which is what
    /// Microsoft's `Test Runner - Isol. Codeunit` the main corpus is recorded under does.
    pub runner: Option<u32>,
}

impl SuiteRun {
    /// Every test codeunit of every non-dependency app, under Codeunit isolation.
    pub fn everything() -> SuiteRun {
        SuiteRun {
            name: String::new(),
            app: None,
            runner: None,
        }
    }
}
