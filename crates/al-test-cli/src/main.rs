//! `al-test-cli`: run the corpus against the runtime and report the pass rate.
//!
//! Exit codes: 0 ok, 1 regression against the baseline, 2 setup error
//! (oracle and corpus disagree, unreadable input, bad arguments).

use al_host::dotnet::DotNetHost;
use al_test_cli::args::{self, Args, Parsed};
use al_test_cli::dotnet::{SidecarConfig, SidecarHost};
use al_test_cli::{
    config, corpus, deps, drift, explain, insight, oracle, output, pin, report, runner::Runner, runtime, select, target,
};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[cfg(all(feature = "mimalloc", not(target_family = "wasm")))]
#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// The server base address of a corpus run (ADR-042, decided without owner review 2026-10-20).
const CORPUS_SERVER_URL: &str = "http://bcserver:8080/BC";

fn main() -> ExitCode {
    // A crash (SIGSEGV and the like) names the test it hit instead of ending the run without a word.
    al_testrun::crash::install();
    // An AL call nests native frames of the bytecode loop; give the runner thread room.
    let worker = std::thread::Builder::new()
        .stack_size(512 << 20)
        .spawn(run)
        .expect("spawn runner thread");
    let result = worker.join().expect("runner thread panicked");
    if std::env::var_os("AL_EXEC_REPORT").is_some() {
        eprintln!("{}", al_vm::exec_report());
    }
    match result {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(2)
        }
    }
}

fn read(p: &Path) -> Result<String, String> {
    std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))
}

fn write(p: &Path, s: &str) -> Result<(), String> {
    std::fs::write(p, s).map_err(|e| format!("{}: {e}", p.display()))
}

fn run() -> Result<ExitCode, String> {
    let args = match args::parse(std::env::args().skip(1))? {
        Parsed::Help => {
            print!("{}", args::HELP);
            return Ok(ExitCode::SUCCESS);
        }
        Parsed::Run(a) => *a,
    };
    if args.list_assumptions {
        let root = al_deps::cache::cache_root_from_env(args.cache_dir.as_deref());
        print!("{}", al_testrun::flagged::list(&root));
        return Ok(ExitCode::SUCCESS);
    }
    if !args.timings_tables.is_empty() {
        let mut ts = Vec::new();
        for f in &args.timings_tables {
            ts.push(report::Timings::from_report_json(&read(f)?).map_err(|e| format!("{}: {e}", f.display()))?);
        }
        print!("{}", output::timings_table(&ts));
        return Ok(ExitCode::SUCCESS);
    }
    if let Some(env) = args.environment {
        al_test_cli::host::set_environment(env);
    }
    // The corpus pins that real BC's web address in its CI is http, has a port and the path /BC, and that its host is
    // not `localhost` (session/lt-geturl-shape/GetUrlShape.al:283); the host name itself is masked there and stays the
    // host setting of ADR-042. So the corpus tool runs with an address of that shape instead of al-runner's default.
    al_test_cli::host::set_server_url(Some(CORPUS_SERVER_URL.into()));
    if let Some(dir) = &args.files_dir {
        al_test_cli::host::set_files_dir(dir.clone());
    }
    let root = args.root.clone().unwrap_or_else(|| PathBuf::from("."));
    if let Some(dir) = &args.behavior_coverage_map {
        return behavior_coverage_map(&root, dir.as_deref());
    }
    if let Some(dir) = &args.coverage_map {
        return coverage_map(&root, dir.as_deref());
    }
    if let Some(dir) = &args.must_not_compile {
        return must_not_compile(&args, &root, dir.as_deref());
    }
    if let Some(project) = &args.project {
        return fetch_project(&args, &root, project);
    }
    let oracle_path = args.oracle.clone().unwrap_or_else(|| root.join("oracle/oracle.toml"));
    let cfg = config::OracleFile::parse(&read(&oracle_path)?).map_err(|e| format!("{}: {e}", oracle_path.display()))?;
    if let Some((n, _)) = args
        .suite_baselines
        .iter()
        .find(|(n, _)| !cfg.versions.iter().any(|v| v.suites.iter().any(|s| s.name == *n)))
    {
        return Err(format!(
            "--suite-baseline: no suite named `{n}` in {}",
            oracle_path.display()
        ));
    }
    if args.all_versions && (args.bc_version.is_some() || args.baseline.is_some()) {
        return Err("--all-versions cannot be combined with --bc-version or --baseline".into());
    }
    let versions: Vec<&config::VersionEntry> = if args.all_versions {
        let mut v: Vec<_> = cfg.versions.iter().collect();
        v.sort_by_key(|v| v.major);
        v
    } else {
        vec![cfg.select(args.bc_version.as_deref())?]
    };

    if args.print_builds {
        return print_builds(&versions);
    }
    let pinned = git_head(&root.join(&cfg.corpus_repo))?;
    if pinned != cfg.corpus_commit {
        return Err(format!(
            "corpus pin {pinned} does not match the oracle's corpus_commit {}; the oracle must come from the CI run of the pinned commit",
            cfg.corpus_commit
        ));
    }

    let mut code = 0u8;
    let mut runs = Vec::new();
    for v in versions {
        let r = run_version(&args, &root, &oracle_path, &cfg, v)?;
        code = code.max(r.code);
        runs.push((v.major, r));
    }
    if runs.len() > 1 && !args.fetch_deps && !args.build_compile_cache {
        print!("{}", drift_summary(&runs));
    }
    Ok(ExitCode::from(code))
}

/// `--coverage-map`: which builtin overloads the corpus calls (`coverage.rs`).
fn coverage_map(root: &Path, dir: Option<&Path>) -> Result<ExitCode, String> {
    let commit = git_head(&root.join("corpus"))?;
    let map = al_test_cli::coverage::corpus_coverage(root, &commit)?;
    let dir = dir.map(Path::to_path_buf).unwrap_or_else(|| root.join("docs"));
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let write = |name: &str, text: String| {
        std::fs::write(dir.join(name), text).map_err(|e| format!("{}: {e}", dir.join(name).display()))
    };
    write("corpus-coverage.md", al_test_cli::coverage::to_markdown(&map))?;
    write("corpus-coverage.json", al_test_cli::coverage::to_json(&map))?;
    let (pinned, all) = map.total();
    println!(
        "corpus coverage: pinned {pinned} of {all} builtin overloads; written to {}",
        dir.display()
    );
    Ok(ExitCode::SUCCESS)
}

/// `--behavior-coverage-map`: which language behavior the corpus pins (`behavior_coverage.rs`).
fn behavior_coverage_map(root: &Path, dir: Option<&Path>) -> Result<ExitCode, String> {
    let commit = git_head(&root.join("corpus"))?;
    let map = al_test_cli::behavior_coverage::corpus_behavior_coverage(root, &commit)?;
    let dir = dir.map(Path::to_path_buf).unwrap_or_else(|| root.join("docs"));
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let write = |name: &str, text: String| {
        std::fs::write(dir.join(name), text).map_err(|e| format!("{}: {e}", dir.join(name).display()))
    };
    write(
        "corpus-behavior-coverage.md",
        al_test_cli::behavior_coverage::to_markdown(&map),
    )?;
    write(
        "corpus-behavior-coverage.json",
        al_test_cli::behavior_coverage::to_json(&map),
    )?;
    for d in al_test_cli::behavior_coverage::Dim::ALL {
        let items = map.of(d);
        println!(
            "behavior coverage: {}: pinned {} of {}",
            d.title(),
            items.iter().filter(|i| i.site.is_some()).count(),
            items.len()
        );
    }
    println!("written to {}", dir.display());
    Ok(ExitCode::SUCCESS)
}

/// `--must-not-compile`: the verdict on every case of the corpus folder (`mnc.rs`).
fn must_not_compile(args: &Args, root: &Path, dir: Option<&Path>) -> Result<ExitCode, String> {
    let dir = dir
        .map(Path::to_path_buf)
        .unwrap_or_else(|| root.join("corpus/tests/must-not-compile"));
    let major: u16 = match &args.bc_version {
        Some(v) => v
            .split('.')
            .next()
            .and_then(|m| m.parse().ok())
            .ok_or_else(|| format!("--bc-version `{v}` is not a BC version"))?,
        None => 29,
    };
    let cases = al_test_cli::mnc::read_cases(&dir)?;
    let outcomes: Vec<_> = cases.iter().map(|c| al_test_cli::mnc::run_case(c, major)).collect();
    print!("{}", al_test_cli::mnc::render(&outcomes, major));
    let bad = al_test_cli::mnc::Summary::of(&outcomes).mismatched;
    Ok(ExitCode::from(u8::from(bad > 0)))
}

/// What one version's run leaves for the comparison across versions.
struct VersionRun {
    code: u8,
    /// Cases real BC passed in this version's oracle.
    oracle: std::collections::BTreeSet<oracle::CaseId>,
    /// Cases the runtime passed.
    passing: std::collections::BTreeSet<oracle::CaseId>,
}

/// `pass-rate.json` becomes `pass-rate-29.json` when several versions run.
fn per_version(p: &Path, major: u16, all: bool) -> PathBuf {
    if !all {
        return p.to_path_buf();
    }
    let stem = p.file_stem().unwrap_or_default().to_string_lossy();
    let name = match p.extension() {
        Some(e) => format!("{stem}-{major}.{}", e.to_string_lossy()),
        None => format!("{stem}-{major}"),
    };
    p.with_file_name(name)
}

fn drift_summary(runs: &[(u16, VersionRun)]) -> String {
    let oracle_sets: Vec<_> = runs.iter().map(|(m, r)| (m.to_string(), r.oracle.clone())).collect();
    let passing_sets: Vec<_> = runs.iter().map(|(m, r)| (m.to_string(), r.passing.clone())).collect();
    // A case one oracle lacks (older corpus commit, ADR-007) cannot show a runtime difference.
    let passing_sets = drift::restrict(&passing_sets, &drift::common(&oracle_sets));
    let mut out = String::from("\nVersion comparison\n");
    out.push_str(&drift::table("oracle", &oracle_sets));
    out.push_str(&drift::table("runtime", &passing_sets));
    out
}

fn run_version(
    args: &Args,
    root: &Path,
    oracle_path: &Path,
    cfg: &config::OracleFile,
    version: &config::VersionEntry,
) -> Result<VersionRun, String> {
    // ADR-007: the oracle may be recorded for an older corpus commit when the pin only added files.
    pin::check(
        &root.join(&cfg.corpus_repo),
        version.oracle_commit(&cfg.corpus_commit),
        &cfg.corpus_commit,
    )
    .map_err(|e| format!("BC {}: {e}", version.bc_version))?;
    let started = Instant::now();
    let junit_path = oracle_path.parent().unwrap_or(Path::new(".")).join(&version.junit);
    let junit_text = read(&junit_path)?;
    let all_cases = oracle::parse_junit(&junit_text).map_err(|e| format!("{}: {e}", junit_path.display()))?;
    // When real BC ran each test codeunit: the run clock of a codeunit starts there, so what the oracle recorded from
    // `Today` is what the run sees.
    let mut clocks = oracle::parse_clocks(&junit_text).map_err(|e| format!("{}: {e}", junit_path.display()))?;
    let index = corpus::discover(&root.join(&cfg.corpus_app)).map_err(|e| format!("corpus: {e}"))?;
    // The other test apps: each against its own oracle, pinned like the main one (ADR-007).
    let mut suite_data = Vec::new();
    for suite in &version.suites {
        // The suite's oracle is described by its own app: changes elsewhere in the corpus do not matter.
        let scope = match suite.app.strip_prefix(cfg.corpus_repo.as_str()) {
            Some(rest) if rest.starts_with('/') => format!("{}/", rest.trim_start_matches('/').trim_end_matches('/')),
            _ => "tests/".to_string(),
        };
        pin::check_in(
            &root.join(&cfg.corpus_repo),
            suite
                .corpus_commit
                .as_deref()
                .unwrap_or_else(|| version.oracle_commit(&cfg.corpus_commit)),
            &cfg.corpus_commit,
            &scope,
        )
        .map_err(|e| format!("BC {} {}: {e}", version.bc_version, suite.name))?;
        let path = oracle_path.parent().unwrap_or(Path::new(".")).join(&suite.oracle);
        let text = read(&path)?;
        if suite.format == config::OracleFormat::Junit {
            clocks.extend(oracle::parse_clocks(&text).map_err(|e| format!("{}: {e}", path.display()))?);
        }
        let cases = match suite.format {
            config::OracleFormat::Junit => oracle::parse_junit(&text),
            config::OracleFormat::Xunit => oracle::parse_xunit(&text).map(|v| v.into_iter().map(|c| c.case).collect()),
        }
        .map_err(|e| format!("{}: {e}", path.display()))?;
        let suite_index = corpus::discover(&root.join(&suite.app)).map_err(|e| format!("{}: {e}", suite.app))?;
        suite_data.push((cases, suite_index));
    }
    // The cases this run is about: everything, the selection, or the one `--explain` names.
    let explained = match &args.explain {
        Some(pattern) => match find_explained(pattern, &args.select, &all_cases, &index, &suite_data, version)? {
            Some(found) => Some(found),
            None if !args.all_versions => {
                return Err(format!("--explain: no test matches `{pattern}`"));
            }
            None => {
                // A test an older oracle does not list is not an error across versions.
                println!("BC {}: no test matches `{pattern}`", version.bc_version);
                return Ok(VersionRun {
                    code: 0,
                    oracle: Default::default(),
                    passing: Default::default(),
                });
            }
        },
        None => None,
    };
    let selecting = args.select.is_active() || explained.is_some();
    let (cases, suite_cases_all): (Vec<oracle::OracleCase>, Vec<Vec<oracle::OracleCase>>) = match &explained {
        Some((suite, case)) => (
            if suite.is_none() {
                vec![case.clone()]
            } else {
                Vec::new()
            },
            (0..suite_data.len())
                .map(|i| {
                    if *suite == Some(i) {
                        vec![case.clone()]
                    } else {
                        Vec::new()
                    }
                })
                .collect(),
        ),
        None => (
            args.select.filter(&all_cases, &index),
            suite_data.iter().map(|(c, ix)| args.select.filter(c, ix)).collect(),
        ),
    };
    if selecting && cases.is_empty() && suite_cases_all.iter().all(Vec::is_empty) {
        return Err(format!(
            "BC {}: the selection ({}) matches no test",
            version.bc_version,
            args.select.describe()
        ));
    }
    if selecting {
        println!(
            "selection: {} of {} cases{}",
            cases.len() + suite_cases_all.iter().map(Vec::len).sum::<usize>(),
            all_cases.len() + suite_data.iter().map(|(c, _)| c.len()).sum::<usize>(),
            if args.select.is_active() {
                format!(" ({})", args.select.describe())
            } else {
                String::new()
            }
        );
    }

    // ADR-015 3.2: the corpus runs as the oracle's BC version with the corpus app's runtime.
    let mut leg = version.bc();
    // `--bc-version` with a full build runs the corpus on that build instead of the pinned one (to try a build Microsoft
    // published later; the ratchet compares with the pinned build's baseline, so use `--no-ratchet`).
    if let Some(asked) = args
        .bc_version
        .as_deref()
        .and_then(|r| al_version::BcVersion::parse(r).ok())
        .filter(|r| r.build.is_some() && (r.major, r.minor) == (leg.major, leg.minor))
    {
        if asked.build != leg.build {
            eprintln!(
                "note: BC build {} instead of the pinned {}",
                asked.build.as_deref().unwrap_or(""),
                leg.build.as_deref().unwrap_or("(none)")
            );
            leg.build = asked.build;
        }
    }
    let app_versions = version.app_versions_for_leg(target::read_app_versions(&root.join(&cfg.corpus_app))?)?;
    let mut chosen = al_version::choose_target(Some(&leg), &app_versions, &leg).map_err(|e| e.to_string())?;
    println!("target: BC {}, runtime {}", chosen.target.bc, chosen.target.runtime);
    // Which build ran: two runs compared as "the same build" must show the same line.
    if let Some(b) = al_test_cli::binary_identity() {
        println!("binary: {b}");
    }
    for note in &chosen.notes {
        eprintln!("note: {note}");
    }
    let symbols = al_version::corpus_symbols(chosen.target.bc.major);
    // The corpus apps CI publishes before the main app (the internals fixture, the cross-app fixtures, including
    // fixture D, which the main app does not depend on), in CI's order, then the main app (corpus
    // `.github/workflows/ci.yml`, `app_dirs`; `deps::corpus_publish_dirs`).
    let corpus_deps = deps::corpus_publish_dirs(&root.join(&cfg.corpus_app))?;
    let main_app = corpus_deps.len();
    let mut app_dirs = corpus_deps.clone();
    app_dirs.push(root.join(&cfg.corpus_app));
    // The other test apps of this version follow the main app (before any dependency app), each
    // once however many runs use it. `app_index` is where the runtime finds its objects.
    let mut app_index: Vec<(&str, usize)> = Vec::new();
    for suite in &version.suites {
        if !app_index.iter().any(|(a, _)| *a == suite.app) {
            app_index.push((&suite.app, app_dirs.len()));
            app_dirs.push(root.join(&suite.app));
        }
    }
    let mut apps = app_dirs
        .iter()
        .map(|d| runtime::load_app(d))
        .collect::<Result<Vec<_>, _>>()?;
    let mut manifests = app_dirs
        .iter()
        .map(|d| runtime::load_manifest(d))
        .collect::<Result<Vec<_>, _>>()?;
    let mut kinds = vec![al_sema::AppKind::Workspace; apps.len()];
    let mut dependency_summary = Vec::new();
    let mut bc_build = None;
    if !args.no_deps {
        // Dependencies go after the corpus apps, so the corpus objects keep their positions.
        // ADR-013 Layer A: the oracle leg is the explicit `--bc-version`, so the Microsoft build
        // is the newest cached (else newest published) build of that minor.
        let provision = deps::Provision {
            cache_dir: args.cache_dir.clone(),
            download: !args.no_download,
            query: target::build_query(Some(&leg.to_string()), &chosen, &app_versions),
            local_artifacts: None,
            country: String::new(),
        };
        let deps = deps::load_with(&root.join(&cfg.corpus_app), &corpus_deps, &args.deps, Some(&provision))?;
        for line in &deps.log {
            eprintln!("{line}");
        }
        match &deps.build {
            Some(u) => {
                println!("bc build: {} ({})", u.build, origin_word(u.origin));
                chosen.target.bc.build = Some(u.build.to_string());
                bc_build = Some(u.build.to_string());
            }
            None => println!("bc build: none needed (every dependency came from .alpackages or --deps)"),
        }
        kinds.extend(deps.apps.iter().map(|_| al_sema::AppKind::Dependency));
        manifests.extend(deps.manifests);
        dependency_summary = deps.summary;
        apps.extend(deps.apps);
    }
    if args.fetch_deps {
        println!("dependencies ready for BC {}", chosen.target.bc);
        return Ok(VersionRun {
            code: 0,
            oracle: Default::default(),
            passing: Default::default(),
        });
    }
    let compile_cache = args.compile_cache.then(|| al_testrun::compile_cache::CompileCache {
        dir: al_testrun::compile_cache::CompileCache::under(&al_deps::cache::cache_root_from_env(
            args.cache_dir.as_deref(),
        )),
        require: args.require_compile_cache,
        rebuild: false,
        country: String::new(),
    });
    if args.build_compile_cache {
        let cc = compile_cache
            .as_ref()
            .expect("--build-compile-cache turns the cache on");
        let first_dep = kinds
            .iter()
            .position(|k| *k == al_sema::AppKind::Dependency)
            .unwrap_or(apps.len());
        let started = std::time::Instant::now();
        let (entry, report) = al_testrun::compile_cache::open_or_build(
            cc,
            &apps[first_dep..],
            &manifests[first_dep..],
            Some(&chosen.target),
        )?;
        let verified = std::time::Instant::now();
        entry
            .verify()
            .map_err(|e| format!("compile cache entry {} is damaged: {e}", report.file.display()))?;
        println!(
            "compile cache: BC {}: {} {} ({:.1} MB), {} dependency objects; {} {:.2} s, checked every block in {:.2} s",
            version.bc_version,
            if report.built { "built" } else { "found" },
            report
                .layers
                .iter()
                .map(|l| l.file.display().to_string())
                .collect::<Vec<_>>()
                .join(" + "),
            report.bytes as f64 / 1e6,
            entry.len(),
            if report.built { "build" } else { "open" },
            if report.built {
                report.build.as_secs_f64()
            } else {
                report.open.as_secs_f64()
            },
            verified.elapsed().as_secs_f64(),
        );
        println!("compile cache build time: {:.2} s", started.elapsed().as_secs_f64());
        return Ok(VersionRun {
            code: 0,
            oracle: Default::default(),
            passing: Default::default(),
        });
    }
    // ADR-014: the sidecar starts lazily, at the first `DotNet` call that reaches it.
    let host = Arc::new(SidecarHost::new(SidecarConfig {
        sidecar: args.dotnet_sidecar.clone(),
        source_root: Some(root.to_path_buf()),
        // Only the user's folders. ADR-014 section 3, step 2 (the service folder of the platform package the run
        // selected) is skipped: the artifact cache holds no service folder, and BcContainerHelper's
        // `~/.bcartifacts.cache/sandbox`, which `al-runner` reads, is a machine's own folder, often of another build
        // of the major. Read here, it made a corpus case pass or be refused by what HOME held (99100
        // AL_JsonTextWriter_SimpleObject needs Newtonsoft.Json from it), so one build gave two results
        // (corpus_harness.rs `outcomes_do_not_depend_on_threads_or_home`). A missing assembly is refused, as on CI.
        assembly_dirs: args.dotnet_assembly_dirs.clone(),
        allow_roll_forward: args.dotnet_allow_roll_forward,
        timeout: Duration::from_secs(args.dotnet_timeout_secs.unwrap_or(60)),
        ..Default::default()
    }));
    if args.require_dotnet {
        if let Err(why) = host.availability() {
            return Err(format!("--require-dotnet: DotNet {why}"));
        }
    }
    let sidecar_host = host.clone();
    // ADR-038: the closed set of pure framework members is answered in process; ALRUNNER_NO_INPROC=1 sends all to the sidecar.
    let host = al_host::inproc::wrap(host);
    if !args.no_deps {
        // The Microsoft apps call .NET (Media, images). Start the helper processes now so the
        // starts overlap the compile; a run without them starts nothing (ADR-014 section 6).
        sidecar_host.prewarm(al_test_cli::dotnet::STANDBY_SIDECARS);
    }
    let dotnet = al_vm::dotnet::DotNetConfig {
        host,
        ..Default::default()
    };
    let mut runs = vec![runtime::SuiteRun {
        name: String::new(),
        app: None,
        runner: None,
    }];
    // The main run takes the corpus app's test codeunits only, as before.
    runs[0].app = Some(main_app);
    for suite in &version.suites {
        let app = app_index.iter().find(|(a, _)| *a == suite.app).map(|(_, i)| *i);
        runs.push(runtime::SuiteRun {
            name: suite.name.clone(),
            app,
            runner: suite.runner,
        });
    }
    // A selection roots the compile at the selected test codeunits (app index, object id).
    let only = selecting.then(|| {
        let mut v: Vec<(usize, u32)> = cases.iter().map(|c| (main_app, c.id.codeunit)).collect();
        for (suite, cases) in version.suites.iter().zip(&suite_cases_all) {
            if let Some((_, app)) = app_index.iter().find(|(a, _)| *a == suite.app) {
                v.extend(cases.iter().map(|c| (*app, c.id.codeunit)));
            }
        }
        v.sort_unstable();
        v.dedup();
        v
    });
    let opts = runtime::RunOptions {
        only,
        trace: explained.is_some(),
        // The corpus has no test data: its oracle is a database the cases fill themselves.
        test_data: None,
        company_data: None,
        compile_cache,
        test_time_limit: None,
        lazy: Vec::new(),
        isolation: None,
        // The corpus CI runs a test codeunit without handlers through BC's TestRunnerHub, which takes its isolation from
        // the codeunit's RequiredTestIsolation, on BC 28 and later; on BC 27 it runs every codeunit under Microsoft's runner
        // 130450 (TestIsolation = Codeunit), where the property has no effect. A difference of the CI's harness, not of
        // the platform (corpus codeunit/lt-qu-commit-isolation/QuCommitIsolation.al:10-16 and 36-39).
        required_isolation: chosen
            .target
            .has(al_version::Gate::TestRunnerHubRequiredIsolationFrom28),
        coverage: false,
        execute: false,
        capture: Default::default(),
        hooks: Default::default(),
        clock_starts: (!clocks.is_empty()).then(|| std::sync::Arc::new(clocks)),
        quiet_baseline: false,
        impact_trace: false,
        narrow: None,
        tdd: false,
        object_hashes: false,
        debug_ready: None,
        // The corpus compiles on real BC: an error found here is the front end's, printed, never a reason not to run.
        stop_on_compile_errors: false,
        run_only: None,
        entries: Vec::new(),
        no_company_reuse: false,
    };
    let load_time = started.elapsed();
    let rt = runtime::CorpusRuntime::try_run_suites_with(
        apps,
        kinds,
        &manifests,
        &symbols,
        Some(&chosen.target),
        dotnet,
        &runs,
        &opts,
    )?;
    // Nothing waits for the drop of the runtime after the report is written.
    let rt = runtime::Retiring::new(rt);
    let p = &rt.phases;
    let sidecar_stats = sidecar_host.stats();
    sidecar_host.shutdown();
    let timings = report::Timings {
        bc_version: version.bc_version.clone(),
        eager: std::env::var_os("AL_EAGER_DEPENDENCIES").is_some(),
        load_s: load_time.as_secs_f64(),
        compile_s: p.compile.as_secs_f64(),
        baseline_s: p.baseline.as_secs_f64(),
        tests_s: p.tests.as_secs_f64(),
        total_s: started.elapsed().as_secs_f64(),
        dotnet: report::DotNetTimes::of(&sidecar_stats),
        compile_cache: p.cache.as_ref().map(report::CacheTimes::of),
        suites: p
            .suites
            .iter()
            .map(|(name, d)| report::SuiteTime {
                name: if name.is_empty() { "corpus".into() } else { name.clone() },
                tests_s: d.as_secs_f64(),
            })
            .collect(),
        slowest_tests: {
            let mut all: Vec<&al_testrun::runtime::TestResult> = rt.results().iter().collect();
            all.sort_by_key(|r| std::cmp::Reverse(r.time));
            all.into_iter()
                .take(10)
                .filter(|r| !r.time.is_zero())
                .map(|r| report::SlowTest {
                    codeunit: r.codeunit,
                    name: r.test.clone(),
                    seconds: r.time.as_secs_f64(),
                })
                .collect()
        },
    };
    if args.timings {
        eprintln!(
            "timings: BC {}: load apps and dependencies {:.2} s, compile {:.2} s, install baseline {:.2} s, run tests {:.2} s, total {:.2} s",
            version.bc_version, timings.load_s, timings.compile_s, timings.baseline_s, timings.tests_s, timings.total_s
        );
        let c = &p.compile_parts;
        eprintln!(
            "compile: load {:.2} s, members {:.2} s, check workspace {:.2} s, baseline roots {:.2} s, demand {:.2} s, check rest {:.2} s, lower {:.2} s, engine {:.2} s",
            c.load.as_secs_f64(),
            c.members.as_secs_f64(),
            c.check.workspace.as_secs_f64(),
            c.check.baseline.as_secs_f64(),
            c.check.demand.as_secs_f64(),
            c.check.rest.as_secs_f64(),
            c.lower.as_secs_f64(),
            p.engine.as_secs_f64()
        );
        if let Some(line) = p.baseline_line() {
            eprintln!("{line}");
        }
        if let Some(line) = p.compile_line() {
            eprintln!("{line}");
        }
        if p.cache.is_some() {
            eprintln!(
                "compile cache: reading the blocks {:.2} s",
                c.cache_blocks.as_secs_f64()
            );
        }
        let slowest: Vec<String> = p
            .slowest
            .iter()
            .map(|c| {
                let start = if c.sidecar_start.is_zero() {
                    String::new()
                } else {
                    format!(" (sidecar start {:.2} s)", c.sidecar_start.as_secs_f64())
                };
                format!("{} {} {:.2} s{start}", c.id, c.name, c.took.as_secs_f64())
            })
            .collect();
        eprintln!("slowest test codeunits: {}", slowest.join("; "));
        if let Some(d) = &timings.dotnet {
            eprintln!("dotnet: {}", d.line());
        }
        if let Some(c) = &timings.compile_cache {
            eprintln!("compile cache: {}", c.line());
        }
    }
    if rt.panics > 0 {
        eprintln!(
            "warning: {} test codeunit(s) hit a runtime panic (reported as not run)",
            rt.panics
        );
    }
    let outcome = |c: &oracle::CaseId| rt.outcome(c);
    let area_of = |cu: u32| index.area_of(cu).map(str::to_string);
    let marks_of = |c: &oracle::CaseId| rt.assumptions_in("", c).to_vec();
    let mut rep = report::build_with(&cases, &area_of, &outcome, &marks_of);
    // The assumptions the run went through, ranked; the counts are kept for --list-assumptions.
    let rows = al_testrun::flagged::rank(
        rt.results()
            .iter()
            .map(|r| (matches!(r.outcome, report::Outcome::Pass), r.assumptions.as_slice())),
    );
    if !selecting {
        al_testrun::flagged::save_hits(&al_deps::cache::cache_root_from_env(args.cache_dir.as_deref()), &rows);
    }
    rep.assumptions = report::AssumptionCount::of(&rows);
    rep.dependencies = dependency_summary;
    rep.bc_build = bc_build.clone();
    rep.timings = Some(timings);
    if !rep.unmapped.is_empty() {
        return Err(format!(
            "{} oracle case(s) have no test codeunit in the corpus, e.g. {}",
            rep.unmapped.len(),
            rep.unmapped[0]
        ));
    }

    // The other test apps, each against its own oracle (loaded above).
    let mut suite_cases = Vec::new();
    for ((suite, (_, suite_index)), cases) in version.suites.iter().zip(&suite_data).zip(suite_cases_all) {
        let (sr, unmapped) = report::SuiteReport::new(
            &suite.name,
            suite.bc_version.as_deref().unwrap_or(&version.bc_version),
            &cases,
            &|cu| suite_index.area_of(cu).map(str::to_string),
            &|c| rt.outcome_in(&suite.name, c),
            &|c| rt.assumptions_in(&suite.name, c).to_vec(),
        );
        if !unmapped.is_empty() {
            return Err(format!(
                "{}: {} oracle case(s) have no test codeunit in {}, e.g. {}",
                suite.name,
                unmapped.len(),
                suite.app,
                unmapped[0]
            ));
        }
        rep.suites.push(sr);
        suite_cases.push(cases);
    }

    let all = args.all_versions;
    if let Some(p) = &args.json {
        write(
            &per_version(p, version.major, all),
            &serde_json::to_string_pretty(&rep).expect("report serializes"),
        )?;
    }
    if let Some(p) = &args.markdown {
        write(&per_version(p, version.major, all), &output::markdown(&rep))?;
    }
    if let Some(p) = &args.junit {
        write(
            &per_version(p, version.major, all),
            &output::junit_with(
                &cases,
                &outcome,
                &|c| rt.first_time(c).as_secs_f64(),
                &marks_of,
                bc_build.as_deref(),
            ),
        )?;
    }
    if let Some(p) = &args.junit {
        for (suite, cases) in version.suites.iter().zip(&suite_cases) {
            let name = format!(
                "{}-{}.{}",
                p.file_stem().unwrap_or_default().to_string_lossy(),
                suite.name,
                p.extension().map_or("xml".into(), |e| e.to_string_lossy())
            );
            write(
                &per_version(&p.with_file_name(name), version.major, all),
                &output::junit_with(
                    cases,
                    &|c| rt.outcome_in(&suite.name, c),
                    &|c| rt.time_in(&suite.name, c).as_secs_f64(),
                    &|c| rt.assumptions_in(&suite.name, c).to_vec(),
                    bc_build.as_deref(),
                ),
            )?;
        }
    }
    let oracle_ok: std::collections::BTreeSet<_> =
        cases.iter().filter(|c| !c.bc_failed).map(|c| c.id.clone()).collect();
    let passing: std::collections::BTreeSet<_> = oracle_ok
        .iter()
        .filter(|c| outcome(c) == report::Outcome::Pass)
        .cloned()
        .collect();
    println!(
        "BC {}: {}/{} pass ({:.1}%){}, {} fail, {} not run",
        version.bc_version,
        rep.overall.pass,
        rep.overall.total,
        rep.overall.rate() * 100.0,
        flagged_note(rep.overall.flagged),
        rep.overall.fail,
        rep.overall.not_run
    );
    for x in rep.suites.iter().filter(|x| !selecting || x.counts.total > 0) {
        println!(
            "BC {} {}: {}/{} pass ({:.1}%){}, {} fail, {} not run; BC fails {} on purpose, the runtime fails {} of them",
            x.oracle_leg,
            x.name,
            x.counts.pass,
            x.counts.total,
            x.counts.rate() * 100.0,
            flagged_note(x.counts.flagged),
            x.counts.fail,
            x.counts.not_run,
            x.bc_failed,
            x.bc_failed_reproduced
        );
    }

    if let Some(n) = args.top_assumptions {
        println!("\n{}", al_testrun::flagged::render_top(&rows, n));
    } else if rep.overall.flagged + rep.suites.iter().map(|x| x.counts.flagged).sum::<usize>() > 0 && !selecting {
        println!(
            "--top-assumptions ranks the unprobed assumptions the flagged passes ran through; --list-assumptions describes them."
        );
    }

    if let Some((suite, case)) = &explained {
        let name = suite.map_or("", |i| version.suites[i].name.as_str());
        let cu = index
            .codeunits
            .get(&case.id.codeunit)
            .or_else(|| suite.and_then(|i| suite_data[i].1.codeunits.get(&case.id.codeunit)));
        let head = format!(
            "BC {}{}: test {} in codeunit {}{}",
            version.bc_version,
            if name.is_empty() {
                String::new()
            } else {
                format!(" {name}")
            },
            case.id.name,
            case.id.codeunit,
            cu.map_or(String::new(), |c| format!(" ({}, area {})", c.file.display(), c.area))
        );
        let oracle_says = if case.bc_failed {
            "fails (excluded from the pass rate)"
        } else {
            "passes"
        };
        let outcome = rt.outcome_in(name, &case.id);
        println!(
            "\n{}",
            explain::render(&head, &outcome, oracle_says, rt.detail_in(name, &case.id))
        );
        let marks = rt.assumptions_in(name, &case.id);
        if !marks.is_empty() {
            println!(
                "{}",
                al_testrun::flagged::render_explain(marks, outcome == report::Outcome::Pass)
            );
        }
        return Ok(VersionRun {
            code: 0,
            oracle: oracle_ok,
            passing,
        });
    }

    // One ratchet per oracle file: the main app's, then each other test app's.
    struct Target<'a> {
        label: String,
        /// `None`: this app is not ratcheted (`--baseline` names the main app's file alone).
        baseline_path: Option<PathBuf>,
        passing: std::collections::BTreeSet<oracle::CaseId>,
        cases: &'a [oracle::OracleCase],
        outcome: Box<dyn Fn(&oracle::CaseId) -> report::Outcome + 'a>,
        area_of: Box<dyn Fn(u32) -> Option<String> + 'a>,
        /// Whether the selection picks a case (every case when nothing is selected).
        selected: Box<dyn Fn(&oracle::CaseId) -> bool + 'a>,
    }
    let mut targets = vec![Target {
        label: version.bc_version.clone(),
        baseline_path: Some(args.baseline.clone().unwrap_or_else(|| root.join(&version.baseline))),
        passing: passing.clone(),
        cases: &cases,
        outcome: Box::new(outcome),
        area_of: Box::new(|cu| index.area_of(cu).map(str::to_string)),
        selected: Box::new(|c| match &explained {
            Some((_, e)) => *c == e.id,
            None => !args.select.is_active() || args.select.matches(c, &index),
        }),
    }];
    for ((suite, suite_cases), (_, suite_index)) in version.suites.iter().zip(&suite_cases).zip(&suite_data) {
        let outcome = |c: &oracle::CaseId| rt.outcome_in(&suite.name, c);
        // `--baseline FILE` is the main app's alone (a run without the dependency apps has its own
        // file); an extra app is ratcheted then only against a file named with `--suite-baseline`.
        let named = args.suite_baselines.iter().find(|(n, _)| *n == suite.name);
        let ratcheted = args.baseline.is_none() || named.is_some();
        targets.push(Target {
            label: format!("{} {}", version.bc_version, suite.name),
            baseline_path: ratcheted.then(|| named.map_or_else(|| root.join(&suite.baseline), |(_, f)| f.clone())),
            passing: suite_cases
                .iter()
                .filter(|c| !c.bc_failed && outcome(&c.id) == report::Outcome::Pass)
                .map(|c| c.id.clone())
                .collect(),
            cases: suite_cases,
            outcome: Box::new(outcome),
            area_of: Box::new(|cu| suite_index.area_of(cu).map(str::to_string)),
            selected: Box::new(|c| match &explained {
                Some((_, e)) => *c == e.id,
                None => !args.select.is_active() || args.select.matches(c, suite_index),
            }),
        });
    }
    // The baseline of a target, narrowed to the selected cases: the others did not run.
    let load_baseline = |t: &Target| -> Result<std::collections::BTreeSet<oracle::CaseId>, String> {
        let path = t
            .baseline_path
            .as_ref()
            .expect("a ratcheted target has a baseline file");
        let mut baseline = report::parse_baseline(&read(path)?).map_err(|e| format!("{}: {e}", path.display()))?;
        // Only the selected cases ran; the others cannot have regressed.
        baseline.retain(|c| (t.selected)(c));
        Ok(baseline)
    };
    if let Some(n) = args.top_refusals {
        for t in targets.iter().filter(|t| !t.cases.is_empty()) {
            let in_scope = |id: &oracle::CaseId| {
                args.top_refusals_area
                    .as_ref()
                    .is_none_or(|a| (t.area_of)(id.codeunit).is_some_and(|x| x.eq_ignore_ascii_case(a)))
            };
            let list = insight::top_refusals(t.cases, &in_scope, &*t.outcome);
            let title = match &args.top_refusals_area {
                Some(a) => format!("BC {} area {a}", t.label),
                None => format!("BC {}", t.label),
            };
            println!("\n{}", insight::render_top(&title, &list, n));
        }
    }
    if args.baseline_diff {
        for t in targets.iter().filter(|t| t.baseline_path.is_some()) {
            let d = insight::baseline_diff(&load_baseline(t)?, t.cases, &*t.outcome);
            println!(
                "\n{}",
                insight::render_diff(&format!("BC {}", t.label), &d, &*t.area_of)
            );
        }
    }
    if args.no_ratchet {
        if args.update_baseline {
            return Err("--no-ratchet cannot be combined with --update-baseline".into());
        }
        return Ok(VersionRun {
            code: 0,
            oracle: oracle_ok,
            passing,
        });
    }
    targets.retain(|t| t.baseline_path.is_some());
    let mut regressed = false;
    for t in &targets {
        let baseline = load_baseline(t)?;
        let baseline_path = t.baseline_path.as_ref().expect("retained for its baseline file");
        let ratchet = report::ratchet(&baseline, &*t.outcome);
        if !ratchet.skipped.is_empty() {
            // ADR-014 section 11: not a regression, but never silent.
            eprintln!(
                "{} baseline case(s) skipped, .NET is unavailable on this machine (CI uses --require-dotnet):",
                ratchet.skipped.len()
            );
            for (c, why) in &ratchet.skipped {
                eprintln!("  {} {}: {why}", c.codeunit, c.name);
            }
        }
        match report::decide(&ratchet, args.update_baseline, args.require_dotnet) {
            report::Decision::Ok => {}
            report::Decision::Setup(why) => return Err(why),
            report::Decision::Regression => {
                eprintln!(
                    "BC {}: {} regression(s): cases in {} no longer pass:",
                    t.label,
                    ratchet.regressions.len(),
                    baseline_path.display()
                );
                for c in &ratchet.regressions {
                    eprintln!("  {} {}", c.codeunit, c.name);
                }
                regressed = true;
            }
        }
    }
    if regressed {
        return Ok(VersionRun {
            code: 1,
            oracle: oracle_ok,
            passing,
        });
    }
    if args.update_baseline {
        for t in &targets {
            let path = t.baseline_path.as_ref().expect("retained for its baseline file");
            write(path, &report::format_baseline(&t.passing))?;
        }
    }
    Ok(VersionRun {
        code: 0,
        oracle: oracle_ok,
        passing,
    })
}

/// ", M flagged" after a pass rate, or nothing when no pass was flagged.
fn flagged_note(flagged: usize) -> String {
    if flagged == 0 {
        String::new()
    } else {
        format!(" (flagged: {flagged} rely on unprobed assumptions)")
    }
}

/// The one case `--explain PATTERN` means, as (suite index or `None` for the main app, case): a
/// test whose name equals the pattern, else the only one that contains it. The selection flags,
/// when given, narrow the candidates. `None` when no test matches.
fn find_explained(
    pattern: &str,
    selection: &select::Selection,
    main: &[oracle::OracleCase],
    main_index: &corpus::CorpusIndex,
    suites: &[(Vec<oracle::OracleCase>, corpus::CorpusIndex)],
    version: &config::VersionEntry,
) -> Result<Option<(Option<usize>, oracle::OracleCase)>, String> {
    let mut all: Vec<(Option<usize>, &oracle::OracleCase)> = main
        .iter()
        .filter(|c| !selection.is_active() || selection.matches(&c.id, main_index))
        .map(|c| (None, c))
        .collect();
    for (i, (cases, ix)) in suites.iter().enumerate() {
        all.extend(
            cases
                .iter()
                .filter(|c| !selection.is_active() || selection.matches(&c.id, ix))
                .map(|c| (Some(i), c)),
        );
    }
    let lower = pattern.to_lowercase();
    let exact: Vec<_> = all
        .iter()
        .filter(|(_, c)| c.id.name.to_lowercase() == lower)
        .cloned()
        .collect();
    let found = if exact.is_empty() {
        all.iter()
            .filter(|(_, c)| c.id.name.to_lowercase().contains(&lower))
            .cloned()
            .collect()
    } else {
        exact
    };
    match found.as_slice() {
        [] => Ok(None),
        [(suite, case)] => Ok(Some((*suite, (*case).clone()))),
        many => {
            let list: Vec<String> = many
                .iter()
                .take(10)
                .map(|(s, c)| {
                    let suite = s.map_or(String::new(), |i| format!(" ({})", version.suites[i].name));
                    format!("  {} {}{suite}", c.id.codeunit, c.id.name)
                })
                .collect();
            Err(format!(
                "--explain `{pattern}` matches {} tests; give a longer name or add --codeunit ID:\n{}",
                many.len(),
                list.join("\n")
            ))
        }
    }
}

fn origin_word(o: al_deps::build::Origin) -> &'static str {
    match o {
        al_deps::build::Origin::Cache => "from the artifact cache",
        al_deps::build::Origin::Published => "newest published, downloaded",
        al_deps::build::Origin::Local => "from the local artifacts, ~/.bcartifacts.cache",
    }
}

/// `--print-builds`: one line per version, `<major> <newest published build>`, from the index
/// alone (no cache, no artifact). CI keys its artifact cache by these lines.
fn print_builds(versions: &[&config::VersionEntry]) -> Result<ExitCode, String> {
    let cdn = al_deps::artifact::HttpCdn::from_env();
    for v in versions {
        let q = al_deps::build::BuildQuery::new(Some(&v.bc().to_string()), v.major, None);
        let sel = al_deps::artifact::newest_published(&cdn, &q)?;
        println!("{} {}", v.major, sel.build);
    }
    Ok(ExitCode::SUCCESS)
}

/// `--project DIR --fetch-deps`: the dependency flow for any AL project. The target comes from
/// `al_version::choose_target` (ADR-015 3.2): `--bc-version`, else the floor in the project's
/// app.json, else the newest version the oracle verifies. Layer A then picks the Microsoft build
/// for that target and the missing apps are downloaded into the cache.
fn fetch_project(args: &Args, root: &Path, project: &Path) -> Result<ExitCode, String> {
    if !args.fetch_deps {
        return Err(
            "--project is only built for --fetch-deps so far: running a project's tests is not implemented".into(),
        );
    }
    let app_versions = target::read_app_versions(project)?;
    let requested = args
        .bc_version
        .as_deref()
        .map(al_version::BcVersion::parse)
        .transpose()
        .map_err(|e| format!("--bc-version: {e}"))?;
    let oracle_path = args.oracle.clone().unwrap_or_else(|| root.join("oracle/oracle.toml"));
    let newest_verified = std::fs::read_to_string(&oracle_path)
        .ok()
        .and_then(|t| config::OracleFile::parse(&t).ok())
        .and_then(|c| c.select(None).ok().map(|v| v.bc()))
        .unwrap_or_else(|| al_version::BcVersion::new(al_version::MIN_BC_MAJOR, 0));
    let chosen =
        al_version::choose_target(requested.as_ref(), &app_versions, &newest_verified).map_err(|e| e.to_string())?;
    println!("target: BC {}, runtime {}", chosen.target.bc, chosen.target.runtime);
    for note in &chosen.notes {
        eprintln!("note: {note}");
    }
    let provision = deps::Provision {
        cache_dir: args.cache_dir.clone(),
        download: !args.no_download,
        query: target::build_query(args.bc_version.as_deref(), &chosen, &app_versions),
        local_artifacts: None,
        country: String::new(),
    };
    let deps = deps::load_with(project, &[], &args.deps, Some(&provision))?;
    for line in &deps.log {
        eprintln!("{line}");
    }
    match &deps.build {
        Some(u) => println!("bc build: {} ({})", u.build, origin_word(u.origin)),
        None => println!("bc build: none needed (every dependency came from .alpackages or --deps)"),
    }
    println!("dependencies: {}", deps.summary.join(", "));
    Ok(ExitCode::SUCCESS)
}

fn git_head(repo: &Path) -> Result<String, String> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|e| format!("git: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "{} is not a git checkout; run `git submodule update --init`",
            repo.display()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}
