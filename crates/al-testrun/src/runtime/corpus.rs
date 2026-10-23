//! `CorpusRuntime`: compiling suites of apps and running them.
use super::*;

impl CorpusRuntime {
    pub fn run(apps: Vec<al_sema::workspace::AppSource>, symbols: &BTreeSet<String>) -> CorpusRuntime {
        Self::run_with_manifests(apps, &[], symbols)
    }

    /// `manifests[i]` belongs to `apps[i]`: the publisher and version `ModuleInfo` reports.
    pub fn run_with_manifests(
        apps: Vec<al_sema::workspace::AppSource>,
        manifests: &[Manifest],
        symbols: &BTreeSet<String>,
    ) -> CorpusRuntime {
        Self::run_with_dotnet(apps, manifests, symbols, al_vm::dotnet::DotNetConfig::default())
    }

    /// [`run_with_manifests`](Self::run_with_manifests) with a `DotNet` host (ADR-014); the
    /// default has none.
    pub fn run_with_dotnet(
        apps: Vec<al_sema::workspace::AppSource>,
        manifests: &[Manifest],
        symbols: &BTreeSet<String>,
        dotnet: al_vm::dotnet::DotNetConfig,
    ) -> CorpusRuntime {
        Self::run_for(apps, manifests, symbols, None, dotnet)
    }

    /// Runs for a target: builtins newer than the target's runtime are refused and version gates
    /// follow it (ADR-015). Without a target, the newest catalog and behaviour are used.
    pub fn run_for(
        apps: Vec<al_sema::workspace::AppSource>,
        manifests: &[Manifest],
        symbols: &BTreeSet<String>,
        target: Option<&al_version::Target>,
        dotnet: al_vm::dotnet::DotNetConfig,
    ) -> CorpusRuntime {
        let kinds = vec![al_sema::AppKind::Workspace; apps.len()];
        Self::run_for_apps(apps, kinds, manifests, symbols, target, dotnet)
    }

    /// [`run_for`](Self::run_for) with the kind of each app. Dependency apps are compiled but no
    /// test is run in them (ADR-013 decision 4).
    pub fn run_for_apps(
        apps: Vec<al_sema::workspace::AppSource>,
        kinds: Vec<al_sema::AppKind>,
        manifests: &[Manifest],
        symbols: &BTreeSet<String>,
        target: Option<&al_version::Target>,
        dotnet: al_vm::dotnet::DotNetConfig,
    ) -> CorpusRuntime {
        Self::run_suites(
            apps,
            kinds,
            manifests,
            symbols,
            target,
            dotnet,
            &[SuiteRun::everything()],
        )
    }

    /// [`run_for_apps`](Self::run_for_apps) for several runs over the same compiled apps: they
    /// share one compile and one install baseline, and each test codeunit of a run is an isolation
    /// unit on a session of its own (`isolation.rs`).
    pub fn run_suites(
        apps: Vec<al_sema::workspace::AppSource>,
        kinds: Vec<al_sema::AppKind>,
        manifests: &[Manifest],
        symbols: &BTreeSet<String>,
        target: Option<&al_version::Target>,
        dotnet: al_vm::dotnet::DotNetConfig,
        suites: &[SuiteRun],
    ) -> CorpusRuntime {
        Self::run_suites_with(
            apps,
            kinds,
            manifests,
            symbols,
            target,
            dotnet,
            suites,
            &RunOptions::default(),
        )
    }

    /// [`run_suites`](Self::run_suites) with a test selection and tracing (`RunOptions`).
    #[allow(clippy::too_many_arguments)]
    pub fn run_suites_with(
        apps: Vec<al_sema::workspace::AppSource>,
        kinds: Vec<al_sema::AppKind>,
        manifests: &[Manifest],
        symbols: &BTreeSet<String>,
        target: Option<&al_version::Target>,
        dotnet: al_vm::dotnet::DotNetConfig,
        suites: &[SuiteRun],
        opts: &RunOptions,
    ) -> CorpusRuntime {
        Self::try_run_suites_with(apps, kinds, manifests, symbols, target, dotnet, suites, opts)
            .expect("a run without `require` does not fail to start")
    }

    /// [`run_suites_with`](Self::run_suites_with), which says why a run could not start: the
    /// compile cache is required (`CompileCache::require`) and has no usable entry.
    #[allow(clippy::too_many_arguments)]
    pub fn try_run_suites_with(
        apps: Vec<al_sema::workspace::AppSource>,
        kinds: Vec<al_sema::AppKind>,
        manifests: &[Manifest],
        symbols: &BTreeSet<String>,
        target: Option<&al_version::Target>,
        dotnet: al_vm::dotnet::DotNetConfig,
        suites: &[SuiteRun],
        opts: &RunOptions,
    ) -> Result<CorpusRuntime, String> {
        // A lean compile leaves workspace objects out: `--affected` needs the hash of each.
        let lean = !opts.object_hashes;
        let compiled = Self::compile_suites_with(apps, kinds, manifests, symbols, target, suites, opts, lean)?;
        let rt = Self::run_compiled(&compiled, target, dotnet.clone(), opts);
        // A lean compile left out the workspace objects no selected test names (`Roots::lean`). A run that met one of
        // their routines was compiled for less than it needs: the apps are compiled completely and the tests run again,
        // so the outcomes are the complete compile's.
        if compiled.lean.is_some() && rt.met_left_out() {
            let lean_time = rt.phases.compile + rt.phases.tests;
            let (apps, kinds) = compiled.into_apps();
            let compiled = Self::compile_suites_with(apps, kinds, manifests, symbols, target, suites, opts, false)?;
            let mut rt = Self::run_compiled(&compiled, target, dotnet, opts);
            rt.phases.lean_redone = true;
            rt.phases.compile += lean_time;
            rt.phases.compiles += 1;
            retire(compiled);
            return Ok(rt);
        }
        // The workspace and the lowered program are big: dropping them takes as long as lowering
        // a tenth of the corpus, and nothing waits for it.
        retire(compiled);
        Ok(rt)
    }

    /// The compile phase of a run: the workspace, the lowered program and what the run phase needs
    /// to know about it ([`run_compiled`](Self::run_compiled)). A warm server keeps this between requests.
    #[allow(clippy::too_many_arguments)]
    pub fn compile_suites(
        apps: Vec<al_sema::workspace::AppSource>,
        kinds: Vec<al_sema::AppKind>,
        manifests: &[Manifest],
        symbols: &BTreeSet<String>,
        target: Option<&al_version::Target>,
        suites: &[SuiteRun],
        opts: &RunOptions,
    ) -> Result<Compiled, String> {
        Self::compile_suites_with(apps, kinds, manifests, symbols, target, suites, opts, false)
    }

    /// [`compile_suites`](Self::compile_suites); with `lean`, selected tests that are free of data are compiled without
    /// the workspace objects they do not name (`al_sema::check::Roots::lean`). Only a caller that runs the tests again
    /// on a complete compile when they meet what was left out (`CorpusRuntime::met_left_out`) may ask for it.
    #[allow(clippy::too_many_arguments)]
    fn compile_suites_with(
        apps: Vec<al_sema::workspace::AppSource>,
        kinds: Vec<al_sema::AppKind>,
        manifests: &[Manifest],
        symbols: &BTreeSet<String>,
        target: Option<&al_version::Target>,
        suites: &[SuiteRun],
        opts: &RunOptions,
        lean: bool,
    ) -> Result<Compiled, String> {
        use al_sema::check::tdd_decl;
        let ranges: std::collections::BTreeMap<String, Vec<(u32, u32)>> = if opts.tdd {
            apps.iter()
                .enumerate()
                .map(|(i, a)| {
                    let r = manifests.get(i).map(|m| m.id_ranges.clone()).unwrap_or_default();
                    (a.id.clone(), r)
                })
                .collect()
        } else {
            Default::default()
        };
        let mut compiled = Self::compile_suites_once(apps, kinds, manifests, symbols, target, suites, opts, lean)?;
        // `--tdd` (ADR-051 decision 10): the fields, enum values and objects the test apps need are declared in a stub
        // file of each, and the apps are compiled again (a stub table's fields are found only once it exists).
        let mut seen = Vec::new();
        for _ in 0..tdd_decl::MAX_RECOMPILES {
            if !opts.tdd || !tdd_decl::take_new(&mut seen, &compiled.lowered.tdd_needs) {
                break;
            }
            let files = tdd_decl::stub_files(&compiled.ws, &seen, &ranges);
            let (mut apps, kinds) = compiled.into_apps();
            tdd_decl::set_stub_files(&mut apps, files);
            compiled = Self::compile_suites_once(apps, kinds, manifests, symbols, target, suites, opts, lean)?;
        }
        Ok(compiled)
    }

    #[allow(clippy::too_many_arguments)]
    fn compile_suites_once(
        apps: Vec<al_sema::workspace::AppSource>,
        kinds: Vec<al_sema::AppKind>,
        manifests: &[Manifest],
        symbols: &BTreeSet<String>,
        target: Option<&al_version::Target>,
        suites: &[SuiteRun],
        opts: &RunOptions,
        lean: bool,
    ) -> Result<Compiled, String> {
        // AL_NO_BASELINE=1 leaves the install baseline out (to measure what it costs).
        let baseline_wanted =
            kinds.contains(&al_sema::AppKind::Dependency) && std::env::var_os("AL_NO_BASELINE").is_none();
        // The baseline runs only when a selected test can observe it (`observe.rs`).
        // AL_FORCE_BASELINE=1 runs it always: the other side of the differential check, which
        // must show no outcome that changes. A run with a custom test runner keeps it too.
        let try_skip = baseline_wanted
            && std::env::var_os("AL_FORCE_BASELINE").is_none()
            && !opts.execute
            // A company built by a codeunit in the baseline is the company every test runs in.
            && opts.company_data.is_none()
            && suites.iter().all(|s| s.runner.is_none());
        let compile_started = std::time::Instant::now();
        // The compile cache holds the dependency apps, which come first in the program (ADR-022).
        // The caller's order is kept in `order` (program app -> caller app) for what it gets back.
        let use_cache = opts.compile_cache.is_some()
            && kinds.contains(&al_sema::AppKind::Dependency)
            && std::env::var_os("AL_EAGER_DEPENDENCIES").is_none();
        let n_apps = apps.len();
        let (apps, mut kinds, manifests, suites_in, only, order) = if use_cache {
            // Dependency apps first, Microsoft's before the others (the layers of the compile
            // cache follow this order), then the workspace's.
            let deps: Vec<usize> = (0..n_apps)
                .filter(|&i| kinds[i] == al_sema::AppKind::Dependency)
                .collect();
            let dep_apps: Vec<&al_sema::workspace::AppSource> = deps.iter().map(|&i| &apps[i]).collect();
            let dep_manifests: Vec<Manifest> = deps
                .iter()
                .map(|&i| manifests.get(i).cloned().unwrap_or_default())
                .collect();
            let order: Vec<usize> = crate::compile_cache::dependency_order(&dep_apps, &dep_manifests)
                .into_iter()
                .map(|k| deps[k])
                .chain((0..n_apps).filter(|&i| kinds[i] != al_sema::AppKind::Dependency))
                .collect();
            let mut new_of = vec![0; n_apps];
            for (new, &old) in order.iter().enumerate() {
                new_of[old] = new;
            }
            let mut slots: Vec<Option<al_sema::workspace::AppSource>> = apps.into_iter().map(Some).collect();
            let apps: Vec<_> = order.iter().map(|&i| slots[i].take().expect("each app once")).collect();
            let kinds: Vec<_> = order.iter().map(|&i| kinds[i]).collect();
            let manifests: Vec<Manifest> = order
                .iter()
                .map(|&i| manifests.get(i).cloned().unwrap_or_default())
                .collect();
            let suites: Vec<SuiteRun> = suites
                .iter()
                .map(|s| SuiteRun {
                    app: s.app.map(|a| new_of[a]),
                    ..s.clone()
                })
                .collect();
            let only = opts
                .only
                .as_ref()
                .map(|v| v.iter().map(|&(a, id)| (new_of[a], id)).collect::<Vec<_>>());
            (apps, kinds, manifests, suites, only, order)
        } else {
            (
                apps,
                kinds,
                manifests.to_vec(),
                suites.to_vec(),
                opts.only.clone(),
                (0..n_apps).collect(),
            )
        };
        let suites = &suites_in[..];
        let manifests = &manifests[..];
        // A dependency app is compiled with the symbols of its own app.json, the project's apps with the
        // project's (hard rule 6: a project that defines CLEAN25 does not change Base Application).
        let dep_symbols: Vec<Option<BTreeSet<String>>> = kinds
            .iter()
            .enumerate()
            .map(|(i, k)| {
                (*k == al_sema::AppKind::Dependency).then(|| {
                    manifests
                        .get(i)
                        .map(|m| m.symbols.iter().cloned().collect())
                        .unwrap_or_default()
                })
            })
            .collect();
        // The apps whose files hold no text yet, in program order.
        let lazy: Vec<Option<al_sema::LazySource>> =
            order.iter().map(|&i| opts.lazy.get(i).cloned().flatten()).collect();
        let identities: Vec<Option<[u8; 32]>> = lazy.iter().map(|l| l.as_ref().map(|l| l.identity)).collect();
        let any_lazy = lazy.iter().any(Option::is_some);
        let fill_all = |apps: &mut Vec<al_sema::workspace::AppSource>| {
            for (a, l) in apps.iter_mut().zip(&lazy) {
                if let Some(l) = l {
                    l.fill(a);
                }
            }
        };
        let roots = al_sema::check::Roots {
            baseline: baseline_wanted,
            company_data: opts
                .company_data
                .iter()
                .flat_map(|c| c.steps())
                .map(|s| s.codeunit)
                .collect(),
            company_methods: opts
                .company_data
                .iter()
                .flat_map(|c| c.steps())
                .filter_map(|s| Some((s.codeunit, s.method?)))
                .collect(),
            only_tests: only.clone(),
            tdd: opts.tdd,
            object_hashes: opts.object_hashes,
            ..Default::default()
        };
        // Whether the install baseline runs, and why it does when it was weighed.
        let mut baseline = baseline_wanted;
        let mut baseline_note: Option<String> = None;
        let catalog = match target {
            Some(t) => al_builtins::catalog::Catalog::for_target(t),
            None => al_builtins::catalog::Catalog::load(),
        };
        let mut apps = apps;
        // Selected tests that look free of data are compiled first the way the demand-driven path
        // does it, without the baseline's roots and without the compile cache: that touches only what
        // the tests reach (a few routines), where the cache decodes every dependency object and the
        // demand-driven compile with the baseline reaches 98,000 routines of Base Application. With `lean`
        // the workspace objects no selected test names are left out of it too (`Roots::lean`). If the rule
        // (`observe.rs`) then says the tests cannot observe the baseline, that is the compile. If it says
        // they may, the apps go on to the compile they would have had. `--require-compile-cache` (CI:
        // the measured run must be a hit) never takes this path.
        let mut compile_cache_skipped = false;
        let mut weighed = false;
        let mut compiles = 1;
        // The workspace a lean pre-check loaded and found not worth a lean compile, for the compile that follows.
        let mut preloaded: Option<(al_sema::Workspace, std::time::Duration)> = None;
        let mut compiled: Option<(al_sema::Workspace, Lowered)> = None;
        let require_cache = opts.compile_cache.as_ref().is_some_and(|c| c.require);
        let mut source_index: Option<&'static str> = None;
        if try_skip && !require_cache && crate::observe::text_looks_pure(&apps, &kinds, symbols, only.as_deref()) {
            let started = std::time::Instant::now();
            let kinds_again = kinds.clone();
            // The dependency apps that were read lazily are loaded from their index when the cache folder
            // has one: no source is read or scanned. Without one the sources are read, scanned, and the
            // index is saved for the next run.
            let cc = opts.compile_cache.as_ref();
            let (ws, filled) = match (any_lazy, cc) {
                (true, Some(cc)) => match lazy_index(cc, &apps, manifests, &lazy, &identities) {
                    Ok(deps) => {
                        source_index = Some("read");
                        (
                            al_sema::Workspace::load_apps_symbols(apps, kinds, symbols, &dep_symbols, deps),
                            false,
                        )
                    }
                    Err(key) => {
                        fill_all(&mut apps);
                        let ws = al_sema::Workspace::load_apps_symbols(apps, kinds, symbols, &dep_symbols, Vec::new());
                        save_index(cc, &ws, &lazy, &key);
                        source_index = Some("built");
                        (ws, true)
                    }
                },
                _ => {
                    fill_all(&mut apps);
                    (
                        al_sema::Workspace::load_apps_symbols(apps, kinds, symbols, &dep_symbols, Vec::new()),
                        true,
                    )
                }
            };
            let load = started.elapsed();
            // A lean compile is tried only when the code it would compile looks free of data too: when what the
            // tests name reads a table, the demand reaches the dependency apps, the compile costs as much as the
            // complete one, and the rule says "may observe" after it. The apps then go straight to the compile
            // with the baseline; without the compile cache it starts from this load.
            if lean
                && !only
                    .as_deref()
                    .is_none_or(|keep| crate::observe::lean_closure_looks_pure(&ws, keep))
            {
                if use_cache {
                    let mut ws = ws;
                    apps = std::mem::take(&mut ws.apps);
                    kinds = kinds_again;
                    retire(ws);
                } else {
                    preloaded = Some((ws, load));
                    apps = Vec::new();
                    kinds = Vec::new();
                }
            } else {
                // Without the baseline's roots, and without the workspace's procedures no selected test names:
                // a helper beside the tests that touches tables must not pull the dependency apps in. Lean, also
                // without the workspace objects no selected test names: the corpus's tables, pages, listeners and
                // install code reach most of Base Application through the demand (98 000 of its 155 000 routines).
                let first_roots = al_sema::check::Roots {
                    baseline: false,
                    reach_only: true,
                    lean,
                    ..roots.clone()
                };
                let (mut ws, lowered) = al_sema::lower::compile_loaded(ws, load, &catalog, first_roots);
                let selected = selected_tests(&lowered, suites, &only);
                weighed = true;
                baseline_note = crate::observe::may_observe_baseline(&lowered, &selected);
                baseline = baseline_note.is_some();
                if baseline {
                    // Back to the apps as they were; a file not read yet is read when the compile that
                    // follows needs it.
                    let _ = filled;
                    apps = std::mem::take(&mut ws.apps);
                    kinds = kinds_again;
                    retire((ws, lowered));
                    compiles += 1;
                } else {
                    compile_cache_skipped = use_cache;
                    compiled = Some((ws, lowered));
                    apps = Vec::new();
                    kinds = Vec::new();
                }
            }
        }
        let (ws, mut lowered, cache_report, layers) = match compiled {
            Some((ws, lowered)) => (ws, lowered, None, None),
            None => match opts.compile_cache.as_ref().filter(|_| use_cache) {
                Some(cc) => {
                    let (ws, lowered, report, layers) = compile_with_cache(
                        cc,
                        apps,
                        kinds,
                        manifests,
                        symbols,
                        target,
                        &lazy,
                        &identities,
                        roots.clone(),
                    )?;
                    if try_skip && !weighed {
                        let selected = selected_tests(&lowered, suites, &only);
                        baseline_note = crate::observe::may_observe_baseline(&lowered, &selected);
                        baseline = baseline_note.is_some();
                    }
                    (ws, lowered, report, layers)
                }
                None => {
                    let (ws, load) = match preloaded.take() {
                        Some(loaded) => loaded,
                        None => {
                            fill_all(&mut apps);
                            let started = std::time::Instant::now();
                            let ws =
                                al_sema::Workspace::load_apps_symbols(apps, kinds, symbols, &dep_symbols, Vec::new());
                            (ws, started.elapsed())
                        }
                    };
                    // Without a weighed answer, weigh on the compiled program, which has the baseline's
                    // roots: the rule only looks at what the tests reach.
                    let (ws, lowered) = al_sema::lower::compile_loaded(ws, load, &catalog, roots.clone());
                    if try_skip && !weighed {
                        let selected = selected_tests(&lowered, suites, &only);
                        baseline_note = crate::observe::may_observe_baseline(&lowered, &selected);
                        baseline = baseline_note.is_some();
                    }
                    (ws, lowered, None, None)
                }
            },
        };
        let source_index = source_index.or(cache_report.as_ref().and_then(|c| c.source_index));
        // A dependency file that does not parse takes every object in it out of the compile, and the
        // tests that need one are refused as "outside the workspace". Say so, once, with the first lines.
        // A file whose second parse gave another result is a defect, not a property of the file (issue 15).
        for flake in ws.parse_flakes() {
            eprintln!("parse: WARNING {flake}");
        }
        let mut unparsed = ws.lazy_parse_problems();
        // The files the cache supplies were parsed when it was built.
        if let Some(c) = &cache_report {
            unparsed.extend(c.parse_problems.iter().cloned());
            unparsed.sort();
            unparsed.dedup();
        }
        if !unparsed.is_empty() {
            eprintln!(
                "parse: {} dependency file(s) did not parse; their objects are unusable and the tests that need them are not run",
                unparsed.len()
            );
            for (_, why) in unparsed.iter().take(10) {
                eprintln!("parse: {why}");
            }
            if unparsed.len() > 10 {
                eprintln!("parse: ... {} more", unparsed.len() - 10);
            }
        }
        // Code the AL compiler rejects (must-not-compile corpus, `alcodes.rs`). Said, not acted on: the
        // run goes on, so an error found here that BC would not give shows in the output, not as a refusal. With
        // `stop_on_compile_errors` the caller prints them (`CorpusRuntime::compile_errors`) and no test runs.
        if !lowered.al_diagnostics.is_empty() && !opts.stop_on_compile_errors {
            eprintln!(
                "compile: {} error(s) the AL compiler would give for the workspace's code",
                lowered.al_diagnostics.len()
            );
            for d in lowered.al_diagnostics.iter().take(100) {
                eprintln!("compile: error {} {}: {}", d.code, d.file, d.message);
            }
        }
        if let Some(how) = std::env::var_os("AL_REACH_REPORT") {
            if how == "tests" {
                for (test, refused) in crate::reach::refused_by_test(&lowered) {
                    let n: usize = refused.iter().map(|(_, v)| v.len()).sum();
                    eprintln!("reach: {test}: {n} refused routines, {} reasons", refused.len());
                    for (reason, routines) in &refused {
                        eprintln!("reach:   {:>4}  {reason}  [{}]", routines.len(), routines.join("; "));
                    }
                }
            } else {
                eprintln!("{}", crate::reach::report(&ws, &lowered));
            }
        }
        // Nothing else holds the program yet, so this does not copy it.
        for (app, m) in std::sync::Arc::make_mut(&mut lowered.program)
            .apps
            .iter_mut()
            .zip(manifests)
        {
            app.publisher = m.publisher.clone();
            app.version = m.version.clone();
            app.resources = m.resources.clone();
            app.include_source = m.include_source;
            app.target = (!m.target.is_empty()).then(|| m.target.clone());
            app.layout_files = m.layout_files.clone();
        }
        // The source lines of an app that includes its source in the symbols (the Code Coverage `Line`).
        {
            let program = std::sync::Arc::make_mut(&mut lowered.program);
            for sf in &mut program.sources {
                let Some(app) = program.apps.get(sf.app as usize).filter(|a| a.include_source) else {
                    continue;
                };
                let _ = app;
                if let Some((_, text)) = ws
                    .apps
                    .get(sf.app as usize)
                    .and_then(|a| a.files.iter().find(|(p, _)| *p == sf.path))
                {
                    sf.lines = text.split('\n').map(|l| l.trim_end_matches('\r').to_string()).collect();
                }
            }
        }
        let opts_only = only;
        let compile_time = compile_started.elapsed();
        // (left out, all) of the workspace objects, when the compile that runs the tests is lean.
        let lean = {
            let all = lowered.dependency.iter().filter(|d| !**d).count();
            let out = lowered.left_out.iter().filter(|l| **l).count();
            (out > 0).then_some((out, all))
        };
        Ok(Compiled {
            ws,
            lowered,
            cache_report,
            baseline_wanted,
            baseline,
            baseline_note,
            order,
            suites: suites_in,
            opts_only,
            compile_cache_skipped,
            source_index,
            compile_time,
            lean,
            compiles,
            incr: layers.map(|layers| Incr {
                layers,
                roots,
                catalog,
                try_skip,
                symbols: symbols.clone(),
            }),
        })
    }

    /// The run phase: engine, install baseline, the test codeunits. Leaves `compiled` as it was, so a warm
    /// server runs it again for the next request.
    pub fn run_compiled(
        compiled: &Compiled,
        target: Option<&al_version::Target>,
        dotnet: al_vm::dotnet::DotNetConfig,
        opts: &RunOptions,
    ) -> CorpusRuntime {
        let Compiled {
            ws,
            lowered,
            cache_report,
            baseline_wanted,
            baseline,
            baseline_note,
            order,
            suites,
            opts_only,
            compile_cache_skipped,
            source_index,
            compile_time: compile_phase,
            lean,
            compiles,
            incr: _,
        } = compiled;
        let (baseline_wanted, baseline, compile_cache_skipped, source_index) =
            (*baseline_wanted, *baseline, *compile_cache_skipped, *source_index);
        let baseline_note = baseline_note.clone();
        // The selection of this request, in program order: a warm compile may reach more than the request asks for.
        let in_program_order = |v: &[(usize, u32)]| {
            let mut new_of = vec![0; order.len()];
            for (new, &old) in order.iter().enumerate() {
                new_of[old] = new;
            }
            v.iter().map(|&(a, id)| (new_of[a], id)).collect::<Vec<_>>()
        };
        let mut opts_only = match &opts.only {
            Some(v) => Some(in_program_order(v)),
            None => opts_only.clone(),
        };
        let suites = &suites[..];
        let compile_errors = super::compile_errors(ws, lowered, order);
        // A parse error alone does not stop the run (`CompileError`): its file's routines refuse instead.
        if opts.stop_on_compile_errors && compile_errors.iter().any(|e| !e.code.is_empty()) {
            // A debugger answers its client's `launch` with the first error; no test runs.
            if let (Some(ready), Some(e)) = (&opts.debug_ready, compile_errors.iter().find(|e| !e.code.is_empty())) {
                (ready.0)(&DebugReady {
                    program: &lowered.program,
                    order,
                    first_error: Some(format!(
                        "error {} {}:{}:{}: {}",
                        e.code, e.file, e.line, e.column, e.message
                    )),
                });
            }
            return CorpusRuntime {
                outcomes: Default::default(),
                times: Default::default(),
                suites: suites.iter().map(|s| s.name.clone()).collect(),
                panics: 0,
                phases: Phases {
                    compile: *compile_phase,
                    compile_parts: lowered.timings,
                    cache: cache_report.clone(),
                    lean: *lean,
                    compiles: *compiles,
                    ..Default::default()
                },
                details: Default::default(),
                results: Vec::new(),
                marks: Default::default(),
                coverage_statements: Vec::new(),
                impact_keys: Vec::new(),
                impact_baseline: None,
                program_apps: Vec::new(),
                tdd_stubs: lowered.program.tdd_stubs.clone(),
                compile_errors,
                entries: Vec::new(),
            };
        }
        let blocked: Vec<Vec<Option<String>>> = lowered
            .unsupported
            .iter()
            .map(|o| o.iter().map(|r| r.first().cloned()).collect())
            .collect();
        let limit = opts.test_time_limit;
        // A debugged run is one thread, so the tests stop one at a time (ADR-048 decision 3).
        let workers = if opts.trace || opts.hooks.debugger.is_some() {
            1
        } else {
            test_workers()
        };
        let baseline_skipped = baseline_wanted && !baseline;
        let engine_started = std::time::Instant::now();
        let make_engine = || {
            let mut engine = Engine::with_config(
                lowered.program.clone(),
                blocked.clone(),
                target.cloned(),
                dotnet.clone(),
            );
            global_triggers_find_unavailable(&mut engine, lowered);
            if let Some(e) = Arc::get_mut(&mut engine) {
                e.test_data = opts.test_data.clone();
            }
            engine
        };
        let mut engine = make_engine();
        let engine_time = engine_started.elapsed();
        let compile_time = *compile_phase + engine_time;
        let started = std::time::Instant::now();
        // With several test threads the tests get a second engine once the baseline has run (below).
        // It does not depend on the baseline, so it is built beside it, on another core.
        // The baseline of an earlier run of the same program, when the compile cache is on and nothing outside the
        // program can have changed what the install code reads (`baseline_key`).
        let baseline_key = if baseline {
            baseline_key(
                ws,
                lowered,
                cache_report.as_ref(),
                suites,
                opts_only.as_deref(),
                target,
                &dotnet,
                opts,
            )
        } else {
            None
        };
        // A kept baseline is taken only when the run clock can start at the instant its run began: the rows carry
        // it. A clock that has started already at another instant (something read it, or an earlier run in this
        // process began later) makes the file unusable here, and the install code runs.
        let kept = baseline_key
            .as_ref()
            .zip(opts.compile_cache.as_ref())
            // `--rebuild-compile-cache`: the file is not read, and the run writes a new one.
            .filter(|(_, cc)| !cc.rebuild)
            // The impact index and an impact run's selection need what the install code raised and touched: the
            // baseline runs.
            .filter(|_| !opts.impact_trace && opts.narrow.is_none())
            // An entry with resource files of its own (issue 5635): whether the install code read resources is known
            // only when it runs here.
            .filter(|_| opts.entries.iter().all(|e| e.resources.is_empty()))
            .and_then(|(key, cc)| crate::baseline_cache::read(&cc.dir, key))
            .filter(|(_, at)| {
                crate::host::SystemClock::begin_at(*at)
                    || (crate::host::SystemClock::started() && crate::host::SystemClock::run_start() == *at)
            })
            // Over test data the file holds the tables the install code wrote to; the rest is the restored database.
            .map(|(mut run, _)| {
                run.baseline = run.baseline.backed_by(&engine);
                run
            });
        let from_cache = kept.is_some();
        let (baseline_run, mut spare_engine) = std::thread::scope(|scope| {
            let spare = (baseline && workers > 1)
                .then(|| {
                    std::thread::Builder::new()
                        .stack_size(64 << 20)
                        .spawn_scoped(scope, make_engine)
                        .ok()
                })
                .flatten();
            let run = if baseline {
                kept.or_else(|| {
                    crate::baseline::run_traced(
                        &engine,
                        lowered,
                        opts.impact_trace || opts.narrow.is_some(),
                        opts.company_data.as_ref(),
                    )
                })
            } else {
                None
            };
            (run, spare.map(|h| h.join().expect("the second engine is built")))
        });
        // The engine the install code ran on (the tests may get another below).
        let install_read_resources =
            baseline_run.is_some() && engine.resources_read.load(std::sync::atomic::Ordering::Relaxed);
        if let (false, Some(key), Some(cc), Some(run)) = (
            from_cache,
            baseline_key.as_ref(),
            opts.compile_cache.as_ref(),
            baseline_run.as_ref(),
        ) {
            // The clock has started by now if the install code read it; if not, no row carries it and it starts here.
            crate::baseline_cache::write(&cc.dir, key, run, crate::host::SystemClock::run_start());
        }
        // The tests run against an engine that knows which tables the install baseline could not
        // seed. The baseline itself ran before it knew; with several test threads, which each run
        // the baseline again (its data is not for sharing between threads), the engine that ran it
        // stays as it was and the tests get another.
        let unseeded: std::collections::HashSet<usize> = baseline_run
            .as_ref()
            .map(|b| b.unseeded.iter().map(|&t| t as usize).collect())
            .unwrap_or_default();
        let baseline_engine = (workers > 1 && baseline_run.is_some()).then(|| engine.clone());
        let mut baseline_ids: Vec<String> = Vec::new();
        let mut baseline_roots: Option<(usize, usize)> = None;
        if let Some(b) = &baseline_run {
            baseline_roots = Some((b.ran, b.total));
            if !opts.quiet_baseline {
                if from_cache {
                    eprintln!("baseline: read from the cache of an earlier run of this program");
                }
                for n in &b.notes {
                    eprintln!("baseline: {n}");
                }
                for l in b.restored_lines() {
                    eprintln!("baseline: {l}");
                }
                eprintln!(
                    "baseline: {} of {} install roots ran to their end; {} tables hold rows, {} are unseeded, {:.2} s",
                    b.ran,
                    b.total,
                    b.filled,
                    b.unseeded.len(),
                    started.elapsed().as_secs_f64()
                );
            }
            baseline_ids = crate::baseline::assumption_ids(&b.notes);
            if baseline_engine.is_some() {
                engine = spare_engine.take().unwrap_or_else(make_engine);
            }
            if let Some(e) = Arc::get_mut(&mut engine) {
                e.unseeded = unseeded;
            }
        }
        // A company built by a codeunit in the baseline (`company_data.rs`): when the codeunit did not run to its end, or
        // the baseline did not run, no test runs.
        let company_refusal = opts.company_data.as_ref().and_then(|c| match &baseline_run {
            Some(b) => crate::company_data::blocked(&b.notes),
            None => Some(format!(
                "{}the install baseline that runs codeunit {} \"{}\" did not run",
                crate::company_data::BLOCKED_NOTE,
                c.codeunit,
                c.name
            )),
        });
        if let Some(why) = &company_refusal {
            // A baseline that ran printed it with its notes.
            if baseline_run.is_none() || opts.quiet_baseline {
                eprintln!("baseline: {why}");
            }
            // Everything the codeunit could stop at, not only where it stopped: the routines lowering refuses that it
            // reaches through its calls.
            if let Some(c) = &opts.company_data {
                let objs = &lowered.program.objects;
                // The first step (the codeunit's method or `OnRun`).
                let root = crate::baseline::step_routine(lowered, &c.steps()[0]).ok();
                if let Some(root) = root {
                    // A codeunit whose app came from the compile cache has no recorded calls: its whole app is listed.
                    let (refused, what) = if lowered.provided.get(root.0).copied().unwrap_or(false) {
                        (
                            crate::reach::refused_in_app(lowered, objs[root.0].app),
                            "'s app (from the compile cache) holds",
                        )
                    } else {
                        (crate::reach::refused_reached_from(lowered, &[root]), " reaches")
                    };
                    let n: usize = refused.iter().map(|(_, v)| v.len()).sum();
                    eprintln!(
                        "baseline: codeunit {} \"{}\"{what} {n} routines that are refused, for {} reasons{}",
                        c.codeunit,
                        c.name,
                        refused.len(),
                        if opts.quiet_baseline {
                            " (--verbose lists them)"
                        } else {
                            ":"
                        }
                    );
                    if !opts.quiet_baseline {
                        for (reason, routines) in &refused {
                            eprintln!(
                                "baseline:   {:>4}  {}  [{}{}]",
                                routines.len(),
                                crate::refusals::clip(reason, 200),
                                routines.iter().take(3).cloned().collect::<Vec<_>>().join("; "),
                                if routines.len() > 3 { "; ..." } else { "" }
                            );
                        }
                    }
                }
            }
            match Arc::get_mut(&mut engine) {
                Some(e) => e.company_refusal = Some(why.clone()),
                None => panic!("the test engine is shared before the tests start"),
            }
        }
        // Without test data the tests run against the corpus database's company, whose demo rows the runtime does not
        // have (ADR-047 rejected): a Count or IsEmpty of one of those tables before the test wrote to it is refused (`demo_row_tables`).
        // A company the baseline built (`--company-data demotool`) holds the rows its codeunit wrote, not the corpus
        // database's. The refusal is off there because nothing is missing: every row comes from Microsoft's own code
        // (Company-Initialize and the DemoTool, compiled from their source and run to their end, else no test runs),
        // so a Count or IsEmpty answers from the rows real BC would hold after the same code.
        if opts.test_data.is_none() && opts.company_data.is_none() {
            let demo = crate::baseline::demo_row_tables(lowered);
            if !demo.is_empty() {
                match Arc::get_mut(&mut engine) {
                    Some(e) => e.demo_tables = demo,
                    None => panic!("the test engine is shared before the tests start"),
                }
            }
        }
        let baseline_time = started.elapsed();
        let mut baseline_run = baseline_run;
        let baseline_trace = baseline_run.as_mut().and_then(|b| b.trace.take());
        let baseline = baseline_run.map(|b| b.baseline);
        // An impact run selects its test codeunits here: after the compile and the install baseline, whose trace says
        // whether the install code reached an impact point (`RunOptions::narrow`).
        if let Some(narrow) = &opts.narrow {
            let input = NarrowInput {
                program: &lowered.program,
                lowered,
                order,
                only: opts.only.as_deref(),
                baseline: baseline_trace.as_deref(),
            };
            if let Some(only) = (narrow.0)(&input) {
                opts_only = Some(in_program_order(&only));
            }
        }
        // A debugger is told the program (to place its breakpoints) and starts the tests when its client says so.
        if let Some(ready) = &opts.debug_ready {
            (ready.0)(&DebugReady {
                program: &lowered.program,
                order,
                first_error: lowered
                    .al_diagnostics
                    .first()
                    .map(|d| format!("error {} {}: {}", d.code, d.file, d.message)),
            });
        }
        let tests_started = std::time::Instant::now();
        let mut outcomes = std::collections::HashMap::new();
        let mut marks_by_test: std::collections::HashMap<(usize, u32, String), Vec<Assumption>> =
            std::collections::HashMap::new();
        let mut times_by_case: std::collections::HashMap<(usize, u32, String), std::time::Duration> =
            Default::default();
        let mut details = std::collections::HashMap::new();
        let mut panics = 0;
        let mut codeunit_times: Vec<CodeunitTime> = Vec::new();
        let order_arc: Arc<Vec<usize>> = Arc::new(order.clone());
        let mut list: Vec<TestResult> = Vec::new();
        let mut impact_keys = crate::impact_trace::Interner::default();
        let impact_baseline: Option<Vec<u32>> = baseline_trace.map(|keys| impact_keys.ids(keys));
        let program_apps: Vec<(String, String, String)> = lowered
            .program
            .apps
            .iter()
            .map(|a| (a.id.clone(), a.name.clone(), a.version.clone()))
            .collect();
        let mut suite_work: Vec<(usize, std::time::Duration)> = Vec::new();
        // Each test thread starts from its own copy of the baseline (a baseline holds `Rc`s and cannot be shared
        // between threads), and reading a snapshot is far cheaper than running the install code again.
        let snapshot: Option<Vec<u8>> = baseline_engine
            .is_some()
            .then(|| baseline.as_ref().and_then(al_vm::Baseline::to_bytes))
            .flatten();
        // The passes of the run: one, or one per entry (issue 5635), each over the same compile and install baseline.
        let passes: Vec<Pass> = if opts.entries.is_empty() {
            vec![Pass {
                only: opts_only.clone(),
                run_only: opts.run_only.clone(),
                hooks: opts.hooks.clone(),
                resources: Default::default(),
            }]
        } else {
            opts.entries
                .iter()
                .map(|e| Pass {
                    only: e.only.as_deref().map(in_program_order).or_else(|| opts_only.clone()),
                    run_only: e.run_only.clone(),
                    hooks: e.hooks.clone(),
                    resources: e
                        .resources
                        .iter()
                        .filter_map(|(caller, r)| {
                            let p = order.iter().position(|o| o == caller)?;
                            Some((p, Arc::new(r.clone())))
                        })
                        .collect(),
                })
                .collect()
        };
        let mut entry_ranges: Vec<std::ops::Range<usize>> = Vec::new();
        for pass in &passes {
            let start = list.len();
            if let Ok(mut o) = engine.resource_overrides.write() {
                o.clone_from(&pass.resources);
            }
            // The install code read resource files, and this entry has resource files of its own: its install baseline
            // would be another one, which is not made per entry.
            let refused = (install_read_resources && !pass.resources.is_empty()).then(|| {
            "the install code read a resource file, and an entry with resource files of its own would need an install baseline of its own (al-runner issue 5635)".to_string()
        });
            let opts_only = &pass.only;
            // The test codeunits to run, in the order the report lists them.
            let mut units: Vec<Unit> = Vec::new();
            // Codeunits with RequiredTestIsolation = Disabled under the TestRunnerHub that are not run (suite, id, object, why).
            let mut refused_disabled: Vec<(usize, u32, usize, String)> = Vec::new();
            for (si, suite) in suites.iter().enumerate() {
                if opts.execute {
                    // A scratch run: the `OnRun` trigger of one codeunit of the suite's app.
                    if let Some(oi) = on_run_target(lowered, suite.app) {
                        units.push(Unit {
                            si,
                            oi,
                            id: lowered.program.objects[oi].id.unwrap_or(0),
                            isolation: TestIsolation::default(),
                            without_baseline: false,
                            coverage: opts.coverage,
                            execute: true,
                            capture: opts.capture,
                            order: order_arc.clone(),
                            hooks: pass.hooks.clone(),
                            clock_start: None,
                            impact: opts.impact_trace,
                            run_only: None,
                            hub: false,
                            leak: None,
                        });
                    }
                    continue;
                }
                let isolation = match suite.runner {
                    None => Ok(TestIsolation::default()),
                    Some(r) => runner_isolation(lowered, r),
                };
                for (oi, o) in lowered.program.objects.iter().enumerate() {
                    let is_test = is_selected_test(lowered, oi, suite, opts_only);
                    let Some(id) = o.id.filter(|_| is_test) else { continue };
                    let isolation = match (&isolation, &refused) {
                        (_, Some(why)) => &Err(why.clone()),
                        (i, None) => i,
                    };
                    // The corpus CI on BC 28 and later runs a codeunit without handlers through the TestRunnerHub, under
                    // its RequiredTestIsolation (`RunOptions::required_isolation`).
                    let this = match isolation {
                        Ok(i) if suite.runner.is_none() && opts.required_isolation && opts.isolation.is_none() => {
                            hub_isolation(lowered, oi).map(|h| h.map_or((*i, false), |h| (h, true)))
                        }
                        Ok(i) => Ok((opts.isolation.unwrap_or(*i), false)),
                        Err(why) => Err(why.clone()),
                    };
                    match &this {
                        Ok((i, hub)) => units.push(Unit {
                            si,
                            oi,
                            id,
                            isolation: *i,
                            hub: *hub,
                            leak: None,
                            without_baseline: false,
                            coverage: opts.coverage,
                            execute: false,
                            capture: opts.capture,
                            order: order_arc.clone(),
                            hooks: pass.hooks.clone(),
                            clock_start: opts.clock_starts.as_ref().and_then(|m| m.get(&id).copied()),
                            impact: opts.impact_trace,
                            run_only: pass.run_only.clone(),
                        }),
                        Err(why) => {
                            if suite.runner.is_none() && opts.required_isolation && requires_disabled(o) {
                                refused_disabled.push((si, id, oi, why.clone()));
                            }
                            for r in test_routines(o) {
                                if refused.is_some() {
                                    if let Some(f) = &pass.hooks.on_test {
                                        f(crate::events::TestEvent {
                                            codeunit: id,
                                            codeunit_name: o.name.clone(),
                                            test: r.name.clone(),
                                            outcome: Outcome::NotRun(why.clone()),
                                            call_stack: String::new(),
                                            time: std::time::Duration::ZERO,
                                            slept_ms: 0,
                                            coverage: Vec::new(),
                                            captured: None,
                                            stubs: Vec::new(),
                                            entry: None,
                                        });
                                    }
                                }
                                outcomes.insert((si, id, r.name.to_lowercase()), Outcome::NotRun(why.clone()));
                                list.push(TestResult {
                                    suite: si,
                                    app: order[o.app as usize],
                                    codeunit: id,
                                    codeunit_name: o.name.clone(),
                                    test: r.name.clone(),
                                    outcome: Outcome::NotRun(why.clone()),
                                    call_stack: String::new(),
                                    assumptions: Vec::new(),
                                    time: std::time::Duration::ZERO,
                                    coverage: Vec::new(),
                                    messages: Vec::new(),
                                    captured: None,
                                    impact: Vec::new(),
                                    stubs: Vec::new(),
                                    slept_ms: 0,
                                    entered: Vec::new(),
                                    dotnet: false,
                                });
                            }
                        }
                    }
                }
            }
            // AL_CHECK_BASELINE_RULE=1: the verification of `observe.rs`. The baseline runs as usual, but
            // each test codeunit the rule calls free of it starts on an empty store. A run compared with a
            // normal one must show no outcome that differs (`tools/baseline_rule_check.sh`).
            if std::env::var_os("AL_CHECK_BASELINE_RULE").is_some() && baseline.is_some() {
                let list = std::env::var("AL_CHECK_BASELINE_RULE").is_ok_and(|v| v == "list");
                for u in &mut units {
                    let why = crate::observe::may_observe_baseline(lowered, &[u.oi]);
                    if let (true, Some(why)) = (list, &why) {
                        eprintln!(
                            "baseline rule check: {} may observe it: {why}",
                            lowered.program.objects[u.oi].name
                        );
                    }
                    u.without_baseline = why.is_none();
                }
                eprintln!(
                    "baseline rule check: {} of {} test codeunits run without the baseline",
                    units.iter().filter(|u| u.without_baseline).count(),
                    units.len()
                );
            }
            // Codeunits with RequiredTestIsolation = Disabled under the TestRunnerHub run first, on this thread, in id order:
            // each starts from what the one before it in its suite left, and leaves everything for the later codeunits
            // (QuCommitIsolation.al:695-718 QU_D11_NextCodeunitSees; codeunits run in ascending id order, :18-20).
            enum Link {
                Run(usize),
                Refused(usize, String),
            }
            let mut chain: Vec<(usize, u32, Link)> = (0..units.len())
                .filter(|&k| units[k].hub && units[k].isolation == TestIsolation::Disabled)
                .map(|k| (units[k].si, units[k].id, Link::Run(k)))
                .chain(
                    refused_disabled
                        .into_iter()
                        .map(|(si, id, oi, why)| (si, id, Link::Refused(oi, why))),
                )
                .collect();
            chain.sort_by_key(|(si, id, _)| (*si, *id));
            let mut slots: Vec<Option<UnitResult>> = (0..units.len()).map(|_| None).collect();
            // Per suite, in id order: the codeunit id and what it left, with the tables of every one before it.
            let mut carried: Vec<(usize, u32, super::run::Carried)> = Vec::new();
            for (si, id, link) in chain {
                let before = carried.iter().rev().find(|(s, _, _)| *s == si);
                let mut left = match link {
                    Link::Run(k) => {
                        units[k].leak = before.map(|(_, _, c)| Arc::new(c.tables.clone()));
                        let start = match before {
                            Some((_, _, c)) => c.baseline.as_ref(),
                            None => baseline.as_ref(),
                        };
                        let mut r = run_unit(&engine, lowered, start, &units[k], None, limit);
                        let left = super::run::CARRIED.with(|c| c.borrow_mut().take());
                        if let (Some((_, _, b)), Ok(done)) = (before, &mut r.result) {
                            if done.5.leak_hit || b.everything {
                                if let Some(why) = &b.unfinished {
                                    refuse_all(&mut done.0, why);
                                }
                            }
                        }
                        slots[k] = Some(r);
                        left.unwrap_or_else(|| not_run_link(lowered, units[k].oi, "it was not run as a whole".into()))
                    }
                    Link::Refused(oi, why) => not_run_link(lowered, oi, why),
                };
                if let Some((_, _, b)) = before {
                    left.tables.extend(b.tables.iter().copied());
                    left.unfinished = left.unfinished.or_else(|| b.unfinished.clone());
                    left.everything |= b.everything;
                    if left.baseline.is_none() {
                        left.baseline = b.baseline.clone();
                    }
                }
                carried.push((si, id, left));
            }
            // The other codeunits run as before; one that comes after a Disabled codeunit notes whether it reads a table that
            // codeunit left rows in, and then runs again below, from what it left.
            for u in units.iter_mut() {
                if let Some((_, _, c)) = carried.iter().rev().find(|(si, id, _)| *si == u.si && *id < u.id) {
                    if u.leak.is_none() {
                        u.leak = Some(Arc::new(c.tables.clone()));
                    }
                }
            }
            let rest: Vec<usize> = (0..units.len()).filter(|k| slots[*k].is_none()).collect();
            let results: Vec<UnitResult> = if workers <= 1 || rest.len() < 2 {
                rest.iter()
                    .map(|&k| &units[k])
                    .map(|u| {
                        let mut observe = |name: &str, _: &Outcome, seen: &Observed| {
                            let mut d = explain::detail(
                                &lowered.program,
                                seen.session.fault_site(),
                                seen.error.map(|e| explain::ErrorInfo {
                                    message: e.message.clone(),
                                    code: e.code.clone(),
                                    call_stack: e.call_stack.clone(),
                                }),
                            );
                            d.refused_in = seen
                                .refused_in
                                .map(|(o, r)| explain::describe(&lowered.program, o, r, al_ir::NO_POS));
                            details.insert((u.si, u.id, name.to_lowercase()), d);
                        };
                        let trace: Option<&mut Observer> = opts.trace.then_some(&mut observe);
                        run_unit(&engine, lowered, baseline.as_ref(), u, trace, limit)
                    })
                    .collect()
            } else {
                // Test codeunits run on a session each and do not share anything (the install
                // baseline is put back for each), so worker threads take them in turn. A worker
                // sets up its own copy of the baseline.
                let next = std::sync::atomic::AtomicUsize::new(0);
                let pull = |baseline: Option<&al_vm::Baseline>| {
                    let mut done = Vec::new();
                    loop {
                        let k = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let Some(u) = rest.get(k).map(|&i| &units[i]) else {
                            return done;
                        };
                        done.push((k, run_unit(&engine, lowered, baseline, u, None, limit)));
                    }
                };
                std::thread::scope(|scope| {
                    let mut handles = Vec::new();
                    for _ in 1..workers.min(rest.len()) {
                        let spawned = std::thread::Builder::new()
                            .stack_size(TEST_STACK)
                            .spawn_scoped(scope, || {
                                // The baseline the first run left, as its snapshot says (over the engine's test data, if
                                // it has any); only a baseline that has no snapshot is run again, once per thread.
                                let own = baseline_engine.as_ref().and_then(|e| {
                                    snapshot
                                        .as_deref()
                                        .and_then(al_vm::Baseline::from_bytes)
                                        .map(|b| b.backed_by(e))
                                        .or_else(|| {
                                            crate::baseline::run(e, lowered, opts.company_data.as_ref())
                                                .map(|b| b.baseline)
                                        })
                                });
                                pull(own.as_ref())
                            });
                        match spawned {
                            Ok(h) => handles.push(h),
                            // No room for another thread (an address space limit): fewer will do.
                            Err(_) => break,
                        }
                    }
                    let mut done = pull(baseline.as_ref());
                    for h in handles {
                        done.extend(h.join().expect("a test worker panicked"));
                    }
                    // Results in the order of the units, whichever worker finished first.
                    done.sort_by_key(|(k, _)| *k);
                    done.into_iter().map(|(_, r)| r).collect()
                })
            };
            for (k, mut r) in rest.into_iter().zip(results) {
                let before = carried
                    .iter()
                    .rev()
                    .find(|(si, id, _)| *si == units[k].si && *id < units[k].id);
                let hit =
                    r.result.as_ref().is_ok_and(|done| done.5.leak_hit) || before.is_some_and(|(_, _, c)| c.everything);
                if let (true, Some((_, _, c))) = (hit, before) {
                    let o = &lowered.program.objects[units[k].oi];
                    let handlers = o.routines.iter().any(|r| {
                        r.attributes
                            .iter()
                            .any(|a| a.name.eq_ignore_ascii_case("HandlerFunctions"))
                    });
                    let refused = match &c.unfinished {
                        Some(why) => Some(why.clone()),
                        // BC's CI runs a codeunit with handlers through another runner than the TestRunnerHub
                        // (QuCommitIsolation.al:11-13): whether it runs before or after the Disabled codeunit is not pinned.
                        None if handlers => Some(
                            "an earlier codeunit with RequiredTestIsolation = Disabled wrote a table this test reads, and \
                             whether BC runs this codeunit, which has handlers, before or after it is not pinned"
                                .to_string(),
                        ),
                        None => None,
                    };
                    match refused {
                        Some(why) => {
                            if let Ok(done) = &mut r.result {
                                refuse_all(&mut done.0, &why);
                            }
                        }
                        None => {
                            units[k].leak = None;
                            r = run_unit(&engine, lowered, c.baseline.as_ref(), &units[k], None, limit);
                        }
                    }
                }
                slots[k] = Some(r);
            }
            let results: Vec<UnitResult> = slots.into_iter().flatten().collect();
            for r in results {
                suite_work.push((r.si, r.took));
                codeunit_times.push(CodeunitTime {
                    id: r.id,
                    name: lowered.program.objects[r.oi].name.clone(),
                    took: r.took,
                    sidecar_start: r.sidecar_start,
                });
                let o = &lowered.program.objects[r.oi];
                let push = |list: &mut Vec<TestResult>,
                            test: &str,
                            outcome: &Outcome,
                            call_stack: String,
                            assumptions: Vec<Assumption>,
                            time: std::time::Duration,
                            coverage: Vec<(u32, u32)>,
                            messages: Vec<String>,
                            captured: Option<crate::events::CapturedAt>,
                            impact: Vec<u32>| {
                    list.push(TestResult {
                        suite: r.si,
                        app: order[o.app as usize],
                        codeunit: r.id,
                        codeunit_name: o.name.clone(),
                        test: test.to_string(),
                        outcome: outcome.clone(),
                        call_stack,
                        assumptions,
                        time,
                        coverage,
                        messages,
                        captured,
                        impact,
                        stubs: Vec::new(),
                        slept_ms: 0,
                        entered: Vec::new(),
                        dotnet: false,
                    })
                };
                match r.result {
                    Ok((done, stacks, times, hits, mut covs, mut scratch)) => {
                        for (name, outcome) in done {
                            let stack = stacks.iter().find(|(n, _)| *n == name).map(|(_, s)| s.clone());
                            let marks = hits.iter().find(|(n, _)| *n == name).map(|(_, h)| h.clone());
                            let marks = marks.unwrap_or_default();
                            if !marks.is_empty() {
                                marks_by_test.insert((r.si, r.id, name.to_lowercase()), marks.clone());
                            }
                            let time = times
                                .iter()
                                .find(|(n, _)| *n == name)
                                .map_or(std::time::Duration::ZERO, |(_, d)| *d);
                            let cov = covs
                                .iter()
                                .position(|(n, _)| *n == name)
                                .map(|i| std::mem::take(&mut covs[i].1))
                                .unwrap_or_default();
                            let said = scratch
                                .messages
                                .iter()
                                .position(|(n, _)| *n == name)
                                .map(|i| std::mem::take(&mut scratch.messages[i].1))
                                .unwrap_or_default();
                            let seen = scratch
                                .captured
                                .iter()
                                .position(|(n, _)| *n == name)
                                .map(|i| std::mem::take(&mut scratch.captured[i].1));
                            let touched = scratch
                                .traces
                                .iter()
                                .position(|(n, _)| *n == name)
                                .map(|i| impact_keys.ids(std::mem::take(&mut scratch.traces[i].1)))
                                .unwrap_or_default();
                            push(
                                &mut list,
                                &name,
                                &outcome,
                                stack.unwrap_or_default(),
                                marks,
                                time,
                                cov,
                                said,
                                seen,
                                touched,
                            );
                            if let Some((_, ms)) = scratch.slept.iter().find(|(n, _)| *n == name) {
                                if let Some(last) = list.last_mut() {
                                    last.slept_ms = *ms;
                                }
                            }
                            if let Some(i) = scratch.stubbed.iter().position(|(n, _)| *n == name) {
                                if let Some(last) = list.last_mut() {
                                    last.stubs = std::mem::take(&mut scratch.stubbed[i].1);
                                }
                            }
                            if let Some(i) = scratch.entered.iter().position(|(n, _, _)| *n == name) {
                                if let Some(last) = list.last_mut() {
                                    last.entered = std::mem::take(&mut scratch.entered[i].1);
                                    last.dotnet = scratch.entered[i].2;
                                }
                            }
                            times_by_case.insert((r.si, r.id, name.to_lowercase()), time);
                            outcomes.insert((r.si, r.id, name.to_lowercase()), outcome);
                        }
                    }
                    Err(msg) => {
                        panics += 1;
                        let panicked: Vec<&al_ir::Routine> = if opts.execute {
                            lowered.program.objects[r.oi]
                                .routines
                                .iter()
                                .filter(|t| t.is_trigger && t.name.eq_ignore_ascii_case("OnRun"))
                                .collect()
                        } else {
                            test_routines(&lowered.program.objects[r.oi]).collect()
                        };
                        for t in panicked {
                            let outcome = Outcome::NotRun(format!("runtime panic (bug): {msg}"));
                            push(
                                &mut list,
                                &t.name,
                                &outcome,
                                String::new(),
                                Vec::new(),
                                std::time::Duration::ZERO,
                                Vec::new(),
                                Vec::new(),
                                None,
                                Vec::new(),
                            );
                            outcomes.insert((r.si, r.id, t.name.to_lowercase()), outcome);
                        }
                    }
                }
            }
            if !opts.entries.is_empty() {
                entry_ranges.push(start..list.len());
            }
        }
        if let Ok(mut o) = engine.resource_overrides.write() {
            o.clear();
        }
        let tests_time = tests_started.elapsed();
        // Test codeunits of different runs may overlap in time: a run's time is what its codeunits took.
        let suite_times: Vec<(String, std::time::Duration)> = suites
            .iter()
            .enumerate()
            .map(|(si, suite)| {
                (
                    suite.name.clone(),
                    suite_work.iter().filter(|(i, _)| *i == si).map(|(_, d)| *d).sum(),
                )
            })
            .collect();
        codeunit_times.sort_by_key(|c| std::cmp::Reverse(c.took));
        codeunit_times.truncate(SLOWEST);
        let suites = suites.iter().map(|s| s.name.clone()).collect();
        // The workspace and the lowered program are big: dropping them takes as long as lowering
        // a tenth of the corpus, and nothing waits for it.
        let compile_parts = lowered.timings;
        let coverage_statements = if opts.coverage {
            crate::coverage::statements(&lowered.program, |oi| !lowered.dependency[oi], |a| order[a])
        } else {
            Vec::new()
        };
        // The engines hold every compiled routine and the baseline's tables: freeing them on this
        // thread kept the report waiting after the last test.
        retire((engine, baseline_engine, spare_engine));
        CorpusRuntime {
            outcomes,
            times: times_by_case,
            suites,
            panics,
            phases: Phases {
                baseline_assumptions: baseline_ids,
                baseline_roots,
                compile: compile_time,
                baseline: baseline_time,
                tests: tests_time,
                suites: suite_times,
                compile_parts,
                cache: cache_report.clone(),
                engine: engine_time,
                baseline_skipped,
                baseline_why: baseline_note,
                compile_cache_skipped,
                source_index,
                slowest: codeunit_times,
                lean: *lean,
                lean_redone: false,
                compiles: *compiles,
            },
            details,
            results: list,
            marks: marks_by_test,
            coverage_statements,
            impact_keys: impact_keys.keys,
            impact_baseline,
            program_apps,
            tdd_stubs: lowered.program.tdd_stubs.clone(),
            compile_errors,
            entries: entry_ranges,
        }
    }
}

impl CorpusRuntime {
    /// Whether a test met a routine a lean compile left out (`al_sema::check::LEFT_OUT`): refused for it, before it
    /// started (its static calls) or while it ran, also inside the test runner's codeunits.
    pub(crate) fn met_left_out(&self) -> bool {
        self.results.iter().any(|r| match &r.outcome {
            Outcome::NotRun(why) | Outcome::Fail(why) => why.contains(al_sema::check::LEFT_OUT),
            Outcome::Pass => false,
        })
    }

    /// With `RunOptions::impact_trace`: the keys that `TestResult::impact` numbers.
    pub fn impact_keys(&self) -> &[String] {
        &self.impact_keys
    }

    /// With `RunOptions::impact_trace`: what the install baseline raised and touched; `None` when none ran.
    pub fn impact_baseline(&self) -> Option<&[u32]> {
        self.impact_baseline.as_deref()
    }

    /// The apps of the compiled program: (id, name, version), in program order.
    pub fn program_apps(&self) -> &[(String, String, String)] {
        &self.program_apps
    }

    /// `--tdd` (ADR-051): the signature of every stub of the compiled program, sorted; empty without `--tdd`.
    pub fn tdd_stubs(&self) -> &[String] {
        &self.tdd_stubs
    }

    /// The errors in the apps under test (`CompileError`); with `RunOptions::stop_on_compile_errors` a run that has
    /// one with an AL code ran no test.
    pub fn compile_errors(&self) -> &[CompileError] {
        &self.compile_errors
    }

    /// Every test the run decided, with its codeunit's name and app, codeunit by codeunit.
    pub fn results(&self) -> &[TestResult] {
        &self.results
    }

    /// With `RunOptions::coverage`: every statement of the workspace's apps (the position ids that
    /// `TestResult::coverage` counts).
    pub fn coverage_statements(&self) -> &[crate::coverage::CoverageStatement] {
        &self.coverage_statements
    }

    /// Where `case` of the run named `suite` was refused or failed; only with `RunOptions::trace`.
    pub fn detail_in(&self, suite: &str, case: &CaseId) -> Option<&TestDetail> {
        let si = self.suites.iter().position(|s| s == suite)?;
        self.details.get(&(si, case.codeunit, case.name.to_lowercase()))
    }

    /// The unprobed assumptions `case` ran through in the run named `suite`; empty for a clean test.
    pub fn assumptions_in(&self, suite: &str, case: &CaseId) -> &[Assumption] {
        self.suites
            .iter()
            .position(|s| s == suite)
            .and_then(|si| self.marks.get(&(si, case.codeunit, case.name.to_lowercase())))
            .map_or(&[], Vec::as_slice)
    }

    /// How long `case` took in the first run.
    pub fn first_time(&self, case: &CaseId) -> std::time::Duration {
        self.times
            .get(&(0, case.codeunit, case.name.to_lowercase()))
            .copied()
            .unwrap_or_default()
    }

    /// How long `case` took in suite `suite` (zero for a test that was not started).
    pub fn time_in(&self, suite: &str, case: &CaseId) -> std::time::Duration {
        self.suites
            .iter()
            .position(|s| s == suite)
            .and_then(|si| self.times.get(&(si, case.codeunit, case.name.to_lowercase())))
            .copied()
            .unwrap_or_default()
    }

    /// The outcome of `case` in the run named `suite` (the first run has the empty name).
    pub fn outcome_in(&self, suite: &str, case: &CaseId) -> Outcome {
        self.suites
            .iter()
            .position(|s| s == suite)
            .and_then(|si| self.outcomes.get(&(si, case.codeunit, case.name.to_lowercase())))
            .cloned()
            .unwrap_or_else(|| Outcome::NotRun("test procedure not found in the compiled corpus".into()))
    }
}

impl CorpusRuntime {
    /// The outcome of `case` in the first run (the main corpus app, or the only run).
    pub fn first_outcome(&self, case: &CaseId) -> Outcome {
        self.outcomes
            .get(&(0, case.codeunit, case.name.to_lowercase()))
            .cloned()
            .unwrap_or_else(|| Outcome::NotRun("test procedure not found in the compiled corpus".into()))
    }
}

/// The key under which the install baseline of this run is kept on disk, or `None` when it must run every time.
/// The baseline is what the install code of the dependency apps and `Company-Initialize` write, followed by the
/// workspace's own install codeunits. It is the same for the same program and the same world around it, which is
/// everything the key holds: this runtime build, the dependency layers (the key of the top layer holds every app of
/// them and the target), the source of every other app, the test codeunits that were compiled, the BC target, the
/// environment of the host and the day. The rows carry the time the run began (`SystemClock`), and a test may ask how
/// old one is. The file therefore holds that instant, and a run that reads the file starts its own clock there, so
/// the rows and the clock agree as they do in a run that began then and ran the install code. The day only bounds how
/// far in the past a run may think it began.
///
/// Not kept at all: a run that reads test data, runs .NET, lets `File` use a real directory, reads the real clock as
/// it goes, records, compares two executable forms or has a test runner of its own: what such a run does is not in
/// the key.
#[allow(clippy::too_many_arguments)]
fn baseline_key(
    ws: &al_sema::Workspace,
    lowered: &Lowered,
    report: Option<&crate::compile_cache::CacheReport>,
    suites: &[SuiteRun],
    only: Option<&[(usize, u32)]>,
    target: Option<&al_version::Target>,
    dotnet: &al_vm::dotnet::DotNetConfig,
    opts: &RunOptions,
) -> Option<String> {
    let report = report?;
    opts.compile_cache.as_ref()?;
    if opts.trace
        || opts.execute
        || opts.coverage
        || suites.iter().any(|s| s.runner.is_some())
        || crate::host::uses_real_files()
        || crate::host::real_clock()
    {
        return None;
    }
    let mut key = crate::baseline_cache::KeyBuilder::new();
    // The restored database: which backup, as the provider says (file, length, modification time, company). A
    // provider that cannot say gets nothing kept.
    match &opts.test_data {
        Some(td) => key.text("test_data", &td.provider.identity()?),
        None => key.text("test_data", ""),
    };
    // Whether `DotNet` calls of the install code could run. A baseline made without a usable .NET holds what the
    // refused calls left out, and one made with it holds what they did.
    key.text(
        "dotnet",
        if dotnet.host.availability().is_ok() {
            "available"
        } else {
            "unavailable"
        },
    );
    key.bytes("runtime_build", &crate::compile_cache::runtime_build());
    // The codeunit that builds the company, and what its files and its app's source are.
    match &opts.company_data {
        Some(c) => key.text("company_data", &format!("{} {} {}", c.codeunit, c.name, c.identity)),
        None => key.text("company_data", ""),
    };
    key.text("dependencies", &report.file.file_name()?.to_string_lossy());
    for (i, app) in ws.apps.iter().enumerate() {
        if ws.kinds.get(i) != Some(&al_sema::AppKind::Dependency) {
            key.text("app", &app.id);
            key.text("app_name", &app.name);
            key.text("app_dependencies", &app.dependencies.join(","));
            // The preprocessor symbols the app was parsed with: they select the code that is compiled.
            let symbols: Vec<&str> = ws.symbols_of(i).iter().map(String::as_str).collect();
            key.text("app_symbols", &symbols.join(","));
            key.bytes("source", &crate::compile_cache::source_hash(app));
            // The install code can read the app's resource files, and a warm server hands it new ones without a new
            // compile (issue 5635).
            for (name, content) in lowered
                .program
                .apps
                .get(i)
                .map(|a| a.resources.as_slice())
                .unwrap_or_default()
            {
                key.text("resource", name);
                key.bytes("resource_content", content);
            }
        }
    }
    let only: Vec<String> = only.unwrap_or(&[]).iter().map(|(a, id)| format!("{a}/{id}")).collect();
    key.text("only", &only.join(","));
    key.text("only_set", if opts.only.is_some() { "yes" } else { "no" });
    match target {
        Some(t) => key.text("target", &format!("{}/{}", t.bc, t.runtime)),
        None => key.text("target", "none"),
    };
    key.text("environment", crate::host::environment_text());
    // GetUrl answers with it, and Company-Initialize stores that answer (ADR-042).
    key.text("server_url", &crate::host::server_url().0);
    // CompanyName() answers it, and Company-Initialize may store that answer.
    key.text("company_name", crate::host::company_name().as_deref().unwrap_or(""));
    // Read the wall clock without starting the run clock: a run that finds its baseline starts the clock at the
    // instant the baseline's run began (`SystemClock::begin_at`).
    let day = crate::host::SystemClock::wall().div_euclid(86_400_000);
    key.text("day", &day.to_string());
    // The lowering reads it (ADR-023), so the install code it produces depends on it.
    key.text(
        "base_access",
        &std::env::var("AL_UNPINNED_BASE_ACCESS").unwrap_or_default(),
    );
    Some(key.finish())
}

/// The isolation BC's TestRunnerHub runs test codeunit `oi` under: its `RequiredTestIsolation`, `None` when that is
/// Codeunit or not set (the codeunit then runs as under Microsoft's runner 130450, QuCommitIsolation.al:331-399 and
/// :719-860 QU_C and QU_N). What the corpus pins are codeunits without handlers or an `OnRun` trigger
/// (QuCommitIsolation.al:10-16); the rest is refused, with the reason.
fn hub_isolation(lowered: &Lowered, oi: usize) -> Result<Option<TestIsolation>, String> {
    let o = &lowered.program.objects[oi];
    let Some(value) = o
        .properties
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("RequiredTestIsolation"))
        .map(|(_, v)| v.trim().trim_matches(['"', '\'']).to_string())
    else {
        return Ok(None);
    };
    let isolation = match TestIsolation::parse(&value) {
        Some(TestIsolation::Codeunit) => return Ok(None),
        Some(i) => i,
        None => {
            return Err(format!(
                "RequiredTestIsolation = {value}: how BC's TestRunnerHub runs a test codeunit with it is not pinned"
            ));
        }
    };
    let name = isolation.name();
    if o.routines.iter().any(|r| {
        r.attributes
            .iter()
            .any(|a| a.name.eq_ignore_ascii_case("HandlerFunctions"))
    }) {
        return Err(format!(
            "RequiredTestIsolation = {name} on a test codeunit with handlers: which test runner BC uses for it, and so its \
             isolation, is not pinned"
        ));
    }
    if o.routines
        .iter()
        .any(|r| r.is_trigger && r.name.eq_ignore_ascii_case("OnRun"))
    {
        return Err(format!(
            "RequiredTestIsolation = {name} on a test codeunit with an OnRun trigger: what the isolation does with the \
             writes of OnRun is not pinned"
        ));
    }
    Ok(Some(isolation))
}

/// Every outcome of a codeunit becomes not run, for `why`.
fn refuse_all(done: &mut [(String, Outcome)], why: &str) {
    for (_, o) in done.iter_mut() {
        *o = Outcome::NotRun(why.to_string());
    }
}

/// The test codeunit's `RequiredTestIsolation` is Disabled.
fn requires_disabled(o: &al_ir::Object) -> bool {
    o.properties.iter().any(|(k, v)| {
        k.eq_ignore_ascii_case("RequiredTestIsolation") && TestIsolation::parse(v) == Some(TestIsolation::Disabled)
    })
}

/// What a codeunit with RequiredTestIsolation = Disabled that did not run leaves for the later ones: nothing known, so a
/// later codeunit that names a table its tests name is not run.
fn not_run_link(lowered: &Lowered, oi: usize, why: String) -> super::run::Carried {
    let o = &lowered.program.objects[oi];
    let mut tables = std::collections::BTreeSet::new();
    for (r, routine) in o.routines.iter().enumerate() {
        if routine.attributes.iter().any(|a| a.name.eq_ignore_ascii_case("Test")) {
            tables.extend(tables_touched(lowered, oi, r));
        }
    }
    super::run::Carried {
        baseline: None,
        tables,
        unfinished: Some(format!(
            "an earlier codeunit with RequiredTestIsolation = Disabled, {}, can write a table this test reads, and it was \
             not run ({}): what BC's run of it left is not known",
            o.name,
            why.chars().take(160).collect::<String>()
        )),
        everything: false,
    }
}
