//! `al-test-cli`: run the corpus against the runtime and report the pass rate.
//!
//! Exit codes: 0 ok, 1 regression against the baseline, 2 setup error
//! (oracle and corpus disagree, unreadable input, bad arguments).
//!
//! The binary: run each oracle version, report, check the not-passing lists. Exit 0 ok, 1 a list does
//! not hold, 2 setup error. A version runs in passes (`passes`, `run_version_passes`): the main app
//! with the suites of the main environment, then one pass per set of `extra_apps` (oracle.toml) with
//! only those suites; the reports are joined before `--json`/`--markdown` are written. The main pass
//! keeps its compile (`PassCompile`), and the next pass grows it by its own apps (ADR-063;
//! `AL_FRESH_PASS_COMPILE=1` turns it off). Entry points: `main`, `run`, `run_version_passes`,
//! `run_version`, `find_explained`, `drift_summary`.

use al_host::dotnet::DotNetHost;
use al_test_cli::args::{self, Args, Parsed};
use al_test_cli::dotnet::{SidecarConfig, SidecarHost};
use al_test_cli::{
    cell_report, config, corpus, deps, drift, explain, insight, not_passing, oracle, output, pin, report,
    runner::Runner, runtime, select, target, work,
};
use al_testrun::report_render::RenderHost;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// The server base address of a corpus run (ADR-042, decided without owner review 2026-10-20).
const CORPUS_SERVER_URL: &str = "http://bcserver:8080/BC";

/// Where `--permission-shadow` writes the denials the shadow run would have raised.
static SHADOW_FILE: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

/// The shadow run's lines, sorted, to the file `--permission-shadow` named, and a count on stdout.
fn write_permission_shadow() {
    let Some(path) = SHADOW_FILE.get() else {
        return;
    };
    let mut lines = al_builtins::permission_shadow::take_all();
    lines.sort();
    lines.dedup();
    let denies = lines.iter().filter(|l| l.contains("\twould deny\t")).count();
    let text = lines.join("\n") + "\n";
    match std::fs::write(path, text) {
        Ok(()) => println!(
            "permission shadow: {} lines ({denies} would deny, {} unknown) in {}",
            lines.len(),
            lines.len() - denies,
            path.display()
        ),
        Err(e) => eprintln!("error: {}: {e}", path.display()),
    }
}

fn main() -> ExitCode {
    // Keep the executable open from the start: the build hash of the compile cache is made later (issue 8691).
    let _ = al_testrun::compile_cache::pin_running_executable();
    // A crash (SIGSEGV and the like) names the test it hit instead of ending the run without a word.
    al_testrun::crash::install();
    // An AL call nests native frames of the bytecode loop; give the runner thread room.
    if al_testrun::profiling::start_from_env().is_some() {
        eprintln!("{}", al_testrun::profiling::LEGACY_NOTE);
    }
    let worker = std::thread::Builder::new()
        .stack_size(al_testrun::RUNNER_STACK)
        .spawn(run)
        .expect("spawn runner thread");
    let result = worker.join().expect("runner thread panicked");
    write_permission_shadow();
    // `AL_PROFILE` (deprecated): its files are written when the run ends (issue 6133).
    al_testrun::profiling::finish_legacy();
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
    let mut args = match args::parse(std::env::args().skip(1))? {
        Parsed::Help => {
            print!("{}", args::HELP);
            return Ok(ExitCode::SUCCESS);
        }
        Parsed::Run(a) => *a,
    };
    args.strict_order_all = std::env::var_os("AL_STRICT_ORDER_ALL").is_some();
    if args.strict_order_all && args.list_new_cases.is_some() {
        return Err("--list-new-cases cannot run with AL_STRICT_ORDER_ALL: new cases go to the default lists".into());
    }
    if let Some(p) = &args.permission_shadow {
        al_builtins::permission_shadow::enable();
        let _ = SHADOW_FILE.set(p.clone());
    }
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
    // not `localhost` (session-system/session/GetUrlShape.al:283); the host name itself is masked there and stays the
    // host setting of ADR-042. So the corpus tool runs with an address of that shape instead of al-runner's default.
    al_test_cli::host::set_server_url(Some(CORPUS_SERVER_URL.into()));
    if let Some(dir) = &args.files_dir {
        al_test_cli::host::set_files_dir(dir.clone());
    }
    let root = args.root.clone().unwrap_or_else(|| PathBuf::from("."));
    if args.assumptions_doc || args.refusals_doc {
        let path = root.join("docs/probes-needed.md");
        let text = read(&path)?;
        let (new, what) = if args.assumptions_doc {
            (al_builtins::assumptions::probes_needed_doc(&text)?, "assumptions")
        } else {
            (al_test_cli::refusal_keys::probes_needed_doc(&text)?, "refusal keys")
        };
        if new != text {
            write(&path, &new)?;
        }
        println!(
            "{}: {what} table {}",
            path.display(),
            if new == text { "current" } else { "rewritten" }
        );
        return Ok(ExitCode::SUCCESS);
    }
    if let Some(rev) = &args.list_changes {
        return list_changes(&args, &root, rev);
    }
    if let Some(dir) = &args.behavior_coverage_map {
        return behavior_coverage_map(&root, dir.as_deref());
    }
    if let Some(dir) = &args.coverage_map {
        return coverage_map(&root, dir.as_deref());
    }
    if let Some(version) = &args.print_symbols {
        return print_symbols(&args, &root, version);
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

    if args.awaiting_assumptions {
        let newest = newest_version(&versions);
        return awaiting_assumptions(&mut args, &root, &oracle_path, &cfg, newest);
    }

    let mut code = 0u8;
    let mut runs = Vec::new();
    let last = versions.len() - 1;
    for (i, v) in versions.into_iter().enumerate() {
        // The process ends after the last version's run: its compile is not freed (`RunOptions::process_ends_after`).
        let r = run_version_passes(&args, &root, &oracle_path, &cfg, v, None, i == last)?;
        code = code.max(r.code);
        runs.push((v.major, r));
    }
    if runs.len() > 1 && !args.fetch_deps && !args.build_compile_cache {
        print!("{}", drift_summary(&runs));
    }
    Ok(ExitCode::from(code))
}

/// The newest of `versions` (al-runner issue 7625): `--all-versions` sorts them by ascending major.
fn newest_version<'a>(versions: &[&'a config::VersionEntry]) -> &'a config::VersionEntry {
    versions
        .iter()
        .max_by_key(|v| v.major)
        .expect("at least one version is selected")
}

/// `--awaiting-assumptions`: the assumptions that wait for a corpus pin, against the pinned corpus (`awaiting.rs`).
/// Runs the awaited files of the entries whose pin landed, on one version, and fails with one message each.
fn awaiting_assumptions(
    args: &mut Args,
    root: &Path,
    oracle_path: &Path,
    cfg: &config::OracleFile,
    version: &config::VersionEntry,
) -> Result<ExitCode, String> {
    use al_test_cli::awaiting::{self, Presence};
    let corpus = root.join(&cfg.corpus_repo);
    let list: Vec<(awaiting::Entry, Presence)> =
        awaiting::with_extra(std::env::var("AL_AWAITING_EXTRA").ok().as_deref())?
            .into_iter()
            .map(|e| {
                let p = awaiting::presence(&corpus, &e.awaiting);
                (e, p)
            })
            .collect();
    if args.summary {
        println!("{}", awaiting::summary(&list));
        return Ok(ExitCode::SUCCESS);
    }
    for (e, p) in &list {
        if let Presence::Partial { present, missing } = p {
            eprintln!("{}", awaiting::partial_warning(e, present, missing));
        }
    }
    let landed: Vec<&awaiting::Entry> = list
        .iter()
        .filter(|(_, p)| *p == Presence::Landed)
        .map(|(e, _)| e)
        .collect();
    if landed.is_empty() {
        println!("awaiting assumptions: {}, none with its corpus pin landed", list.len());
        return Ok(ExitCode::SUCCESS);
    }
    // Run exactly the awaited files of the landed entries; the verdict, not the ratchet, decides.
    let mut files: Vec<String> = landed
        .iter()
        .flat_map(|e| e.awaiting.files.iter().map(|f| awaiting::split(f).0.to_string()))
        .collect();
    files.sort();
    files.dedup();
    args.select = select::Selection {
        files,
        ..Default::default()
    };
    args.no_ratchet = true;
    args.prune_not_passing = false;
    args.list_new_cases = None;
    let mut cases = Vec::new();
    run_version_passes(args, root, oracle_path, cfg, version, Some(&mut cases), true)?;
    println!();
    for e in &landed {
        let v = awaiting::verdict(e.id, &e.awaiting, &cases);
        println!("{}", awaiting::message(e, &v));
    }
    Ok(ExitCode::from(1))
}

/// `--coverage-map`: which builtin overloads the corpus calls (`coverage.rs`).
/// `--list-changes REV`: how each not-passing list changed against REV, as Markdown (ADR-065, owner decision 2: a line
/// added for a case REV's oracle had is a regression the owner approves in review). Exit 1 when there is one; exit 3 when
/// a line was added to a strict list (`strict_not_run.rs`), which may only shrink; exit 4 when the corpus rename map sends
/// a case that passed at REV to a key no oracle has (ADR-065 amendment 1, item 2).
fn list_changes(args: &Args, root: &Path, rev: &str) -> Result<ExitCode, String> {
    let oracle_path = args.oracle.clone().unwrap_or_else(|| root.join("oracle/oracle.toml"));
    let cfg = config::OracleFile::parse(&read(&oracle_path)?).map_err(|e| format!("{}: {e}", oracle_path.display()))?;
    let base = al_test_cli::base_oracle::Base::open(root, rev)?;
    let oracle_dir = oracle_path.parent().unwrap_or(root);
    let mut changes = Vec::new();
    let mut carries = std::collections::BTreeMap::new();
    for v in &cfg.versions {
        // ADR-065 amendment 1: the base lists in head keys, through the corpus rename map and app moves.
        let carry = base.carry(root, &cfg, oracle_dir, v.major, None)?;
        let lists =
            std::iter::once((None, &v.baseline)).chain(v.suites.iter().map(|s| (Some(s.name.as_str()), &s.baseline)));
        for (suite, rel) in lists {
            let now = read_list(&root.join(rel))?;
            let mut ids = base.oracle_ids(v.major, suite)?;
            let carried = carry.of(suite).cloned().unwrap_or_default();
            ids.extend(carried.known);
            changes.push(not_passing::list_change(
                rel,
                &now,
                &base.list(rel)?,
                &carried.listed,
                &ids,
            ));
        }
        carries.insert(v.major, carry);
    }
    print!("{}", not_passing::render_changes(rev, &changes));
    // ADR-065 amendment 1, item 2: a passing case the map sends to a key no head oracle has makes the bump red. Exit 4.
    let lost: Vec<String> = cfg
        .versions
        .iter()
        .flat_map(|v| {
            carries[&v.major]
                .lost
                .iter()
                .map(|l| al_test_cli::rename_map::lost_line(&v.bc_version, l))
        })
        .collect();
    if !lost.is_empty() {
        println!(
            "\n### Passing cases the rename map sends to no oracle ({})\n",
            lost.len()
        );
        lost.iter().for_each(|l| println!("- {l}"));
    }
    // ADR-070 decision 5: the strict lists only shrink. Exit 3 when a line was added to one the base revision has.
    let mut grown = false;
    for v in &cfg.versions {
        let rel = al_test_cli::strict_not_run::path(v.major);
        let now = read_list(&root.join(&rel))?;
        // A line the bump moved to a renamed case's new key is not added (ADR-065 amendment 1).
        let base_list = match (base.list_opt(&rel)?, carries.get(&v.major)) {
            (Some(l), Some(c)) => Some(c.translate_set(&l)?),
            (l, _) => l,
        };
        let added = al_test_cli::strict_not_run::added(base_list.as_ref(), &now);
        if !added.is_empty() {
            grown = true;
            println!(
                "\n### Lines added to `{rel}` ({}): this list may only shrink\n",
                added.len()
            );
            added.iter().for_each(|c| println!("- {} {}", c.codeunit, c.name));
        }
    }
    let accepted = changes.iter().any(|c| !c.accepted_regressions.is_empty());
    Ok(ExitCode::from(match (grown, lost.is_empty()) {
        (true, _) => 3,
        (false, false) => 4,
        (false, true) => u8::from(accepted),
    }))
}

/// The not-passing list at `path`, empty when the file does not exist.
fn read_list(path: &Path) -> Result<std::collections::BTreeSet<oracle::CaseId>, String> {
    if !path.exists() {
        return Ok(Default::default());
    }
    not_passing::parse(&read(path)?).map_err(|e| format!("{}: {e}", path.display()))
}

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

/// `--print-symbols VERSION`: the corpus preprocessor symbols of a BC version (`al_version::corpus_symbols`),
/// comma-separated on one line. `tools/corpus-writer/compile-legs.sh` and the symbol check of `tools/corpus_bump.sh`
/// read them here, so the rule has one owner (ADR-078 point 3).
fn print_symbols(args: &Args, root: &Path, version: &str) -> Result<ExitCode, String> {
    let v = al_version::BcVersion::parse(version).map_err(|e| format!("--print-symbols `{version}`: {e}"))?;
    let symbols: Vec<String> = al_version::corpus_symbols(&v, &released_minors(args, root)?)
        .into_iter()
        .collect();
    println!("{}", symbols.join(","));
    Ok(ExitCode::SUCCESS)
}

/// The released minors of `--released-minors LIST` (comma-separated, not empty), else those the oracle table records.
fn released_minors(args: &Args, root: &Path) -> Result<al_version::ReleasedMinors, String> {
    if let Some(list) = &args.released_minors {
        let entries: Vec<&str> = list.split(',').map(str::trim).filter(|e| !e.is_empty()).collect();
        return al_version::ReleasedMinors::parse(&entries).map_err(|e| format!("--released-minors: {e}"));
    }
    let oracle_path = args.oracle.clone().unwrap_or_else(|| root.join("oracle/oracle.toml"));
    let cfg = config::OracleFile::parse(&read(&oracle_path)?).map_err(|e| format!("{}: {e}", oracle_path.display()))?;
    Ok(cfg.released())
}

/// What one version's run leaves for the comparison across versions.
struct VersionRun {
    code: u8,
    /// Cases real BC passed in this version's oracle.
    oracle: std::collections::BTreeSet<oracle::CaseId>,
    /// Cases the runtime passed.
    passing: std::collections::BTreeSet<oracle::CaseId>,
    /// The report, for `--json` and `--markdown` (written by the caller once every pass of the version ran).
    rep: Option<report::Report>,
    /// Set instead of an error when a pass of a version that has several has no selected or explained test: the
    /// error, should no pass of the version have one.
    unmatched: Option<String>,
}

impl VersionRun {
    fn empty(code: u8) -> VersionRun {
        VersionRun {
            code,
            oracle: Default::default(),
            passing: Default::default(),
            rep: None,
            unmatched: None,
        }
    }
}

/// How a pass gets its compile (ADR-063): `kept`, the main pass's compile to grow by this pass's apps; `keep`, give this
/// pass's compile back for the next pass (a grown pass gives it back cut to the main pass's apps it grew).
struct PassCompile {
    keep: bool,
    kept: Option<runtime::KeptCompile>,
}

/// One compile and run of a version. The main pass runs the main app and the suites that need no app beyond the
/// corpus CI's main environment; each other pass runs the suites whose CI job installs the same extra apps
/// (`SuiteEntry::extra_apps`), in an environment of their own, without the main app's tests.
struct Pass {
    main: bool,
    /// (id, name) of the apps this pass loads beyond the declared dependencies.
    extra_apps: Vec<(String, String)>,
    /// The service-tier settings of the pass's suites (`SuiteEntry::server_settings`, as written there) and what they ask for.
    server_settings: Vec<String>,
    settings: config::ServerSettings,
}

impl Pass {
    /// The pass's name in `metrics/<major>/work.json` (`work::pass_name`).
    fn work_name(&self) -> String {
        let extra_names: Vec<String> = self.extra_apps.iter().map(|(_, name)| name.clone()).collect();
        work::pass_name(self.main, &extra_names, &self.server_settings)
    }
}

/// The passes of `version`, the main one first, each with the version's table narrowed to its suites.
fn passes(version: &config::VersionEntry) -> Vec<(config::VersionEntry, Pass)> {
    let mut groups: Vec<(Vec<config::ExtraApp>, Vec<config::SuiteEntry>)> = vec![(Vec::new(), Vec::new())];
    for suite in &version.suites {
        // One pass per environment: the same extra apps, the same extra corpus folders and the same server settings.
        let same_env = |(extra, suites): &&mut (Vec<config::ExtraApp>, Vec<config::SuiteEntry>)| {
            *extra == suite.extra_apps
                && suites
                    .first()
                    .map_or(suite.extra_dirs.is_empty() && suite.server_settings.is_empty(), |s| {
                        s.extra_dirs == suite.extra_dirs && s.server_settings == suite.server_settings
                    })
        };
        match groups.iter_mut().find(same_env) {
            Some((_, suites)) => suites.push(suite.clone()),
            None => groups.push((suite.extra_apps.clone(), vec![suite.clone()])),
        }
    }
    groups
        .into_iter()
        .enumerate()
        .map(|(i, (extra, suites))| {
            let server_settings = suites.first().map(|s| s.server_settings.clone()).unwrap_or_default();
            // `OracleFile::parse` checked every suite's settings.
            let settings = suites.first().and_then(|s| s.settings().ok()).unwrap_or_default();
            let mut view = version.clone();
            view.suites = suites;
            let pass = Pass {
                main: i == 0,
                extra_apps: extra.into_iter().map(|a| (a.id, a.name)).collect(),
                server_settings,
                settings,
            };
            (view, pass)
        })
        .collect()
}

/// Runs every pass of `version` and joins them: one exit code, the main app's sets, one report.
#[allow(clippy::too_many_arguments)]
fn run_version_passes(
    args: &Args,
    root: &Path,
    oracle_path: &Path,
    cfg: &config::OracleFile,
    version: &config::VersionEntry,
    mut awaited: Option<&mut Vec<al_test_cli::awaiting::AwaitedCase>>,
    process_ends_after: bool,
) -> Result<VersionRun, String> {
    let all = passes(version);
    let multi = all.len() > 1;
    let last = all.len() - 1;
    let mut joined: Option<VersionRun> = None;
    let mut unmatched = Vec::new();
    // ADR-063: the main pass keeps its compile when another pass follows, and the next pass grows it by its own apps
    // instead of compiling everything again; a grown pass keeps its compile, cut back to the main pass's apps, for the
    // pass after it. AL_FRESH_PASS_COMPILE=1: every pass compiles from source.
    let mut kept: Option<runtime::KeptCompile> = None;
    // The report renderer's warning is said once for the version, by the first pass whose renderer was unavailable.
    let mut render_note = not_passing::RenderNote::default();
    let share = multi && !runtime::fresh_pass_compile();
    // The test codeunits of the main app: the same for every pass.
    let index = corpus::discover(&root.join(&cfg.corpus_app)).map_err(|e| format!("corpus: {e}"))?;
    for (i, (view, pass)) in all.iter().enumerate() {
        let grow = PassCompile {
            keep: share && i < last,
            kept: kept.take(),
        };
        let (r, left) = run_version(
            args,
            root,
            oracle_path,
            cfg,
            view,
            pass,
            multi,
            awaited.as_deref_mut(),
            process_ends_after && i == last,
            grow,
            &index,
            &mut render_note,
        )?;
        kept = left;
        if let Some(why) = r.unmatched {
            unmatched.push(why);
            continue;
        }
        joined = Some(match joined {
            None => r,
            Some(mut j) => {
                j.code = j.code.max(r.code);
                j.oracle.extend(r.oracle);
                j.passing.extend(r.passing);
                match (&mut j.rep, r.rep) {
                    (Some(a), Some(b)) => join_reports(a, b),
                    (a @ None, b) => *a = b,
                    (Some(_), None) => {}
                }
                j
            }
        });
    }
    let Some(mut joined) = joined else {
        // No pass has a selected test: what a version without other passes says.
        if args.explain.is_some() && args.all_versions {
            println!(
                "BC {}: no test matches `{}`",
                version.bc_version,
                args.explain.as_deref().unwrap_or_default()
            );
            return Ok(VersionRun::empty(0));
        }
        return Err(unmatched.into_iter().next().unwrap_or_default());
    };
    if let Some(rep) = &joined.rep {
        joined.code = joined.code.max(work_ratchet(args, root, version, rep)?);
        let all = args.all_versions;
        if let Some(p) = &args.json {
            write(
                &per_version(p, version.major, all),
                &serde_json::to_string_pretty(rep).expect("report serializes"),
            )?;
        }
        if let Some(p) = &args.markdown {
            write(&per_version(p, version.major, all), &output::markdown(rep))?;
        }
    }
    Ok(joined)
}

/// The work ratchet (`work.rs`): the counts of every pass of `version` against `metrics/<major>/work.json`, or written to
/// it with `--update-work`. The exit code: 1 when a count is more than the tolerance above the file.
fn work_ratchet(args: &Args, root: &Path, version: &config::VersionEntry, rep: &report::Report) -> Result<u8, String> {
    let rel = format!("metrics/{}/work.json", version.major);
    let path = root.join(&rel);
    let passes: Vec<(String, work::Counts)> = rep.work.clone().into_iter().collect();
    let actual = work::WorkFile::of(rep.bc_build.clone(), &passes);
    let skip = work::not_full_corpus(args).or_else(|| work::not_full_corpus_env(|v| std::env::var_os(v).is_some()));
    if args.update_work {
        if let Some(why) = skip {
            return Err(format!(
                "--update-work needs a run of the full corpus, not one with {why}"
            ));
        }
        write(&path, &actual.to_text())?;
        println!("work ratchet: BC {}: wrote {rel}", version.bc_version);
        return Ok(0);
    }
    if skip.is_some() || args.fetch_deps || args.build_compile_cache {
        return Ok(0);
    }
    let annotate = std::env::var_os("GITHUB_ACTIONS").is_some();
    let expected = match std::fs::read_to_string(&path) {
        Ok(text) => work::WorkFile::parse(&text).map_err(|e| format!("{rel}: {e}"))?,
        Err(_) => {
            println!(
                "{}work ratchet: BC {}: {rel} is missing; make it with `al-test-cli --root . --update-work --bc-version {}`",
                if annotate { "::error::" } else { "" },
                version.bc_version,
                version.bc_version
            );
            return Ok(1);
        }
    };
    let findings = work::compare(&expected, &actual);
    for f in &findings {
        let tag = match (annotate, f.fails()) {
            (false, _) => "",
            (true, true) => "::error::",
            (true, false) => "::notice::",
        };
        println!("{tag}{}", f.line(&version.bc_version, &rel));
    }
    if findings.iter().any(work::Finding::fails) {
        println!(
            "work ratchet: BC {}: the run did more work than {rel} allows. If that is meant, run `al-test-cli --root . --all-versions --update-work` and commit the file in the same pull request; the reviewer sees the counts in the diff.",
            version.bc_version
        );
        return Ok(1);
    }
    let n: usize = actual.passes.values().map(|c| c.len()).sum();
    println!(
        "work ratchet: BC {}: {n} counts within {}% of {rel}",
        version.bc_version,
        work::TOLERANCE_PERCENT
    );
    Ok(0)
}

/// `b`, the report of another pass of the same version, added to `a`: its suites, and its times.
fn join_reports(a: &mut report::Report, b: report::Report) {
    a.suites.extend(b.suites);
    a.work.extend(b.work);
    if let (Some(ta), Some(tb)) = (&mut a.timings, b.timings) {
        ta.load_s += tb.load_s;
        ta.compile_s += tb.compile_s;
        ta.baseline_s += tb.baseline_s;
        ta.tests_s += tb.tests_s;
        ta.total_s += tb.total_s;
        ta.suites.extend(tb.suites.into_iter().filter(|s| s.name != "corpus"));
    }
}

/// The compile cache of a corpus run (ADR-022): with `--compile-cache`, under the cache root.
fn corpus_compile_cache(args: &Args) -> Option<al_testrun::compile_cache::CompileCache> {
    args.compile_cache.then(|| al_testrun::compile_cache::CompileCache {
        dir: al_testrun::compile_cache::CompileCache::under(&al_deps::cache::cache_root_from_env(
            args.cache_dir.as_deref(),
        )),
        require: args.require_compile_cache,
        rebuild: false,
        country: String::new(),
    })
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

/// One pass of a version (`Pass`): `version` holds that pass's suites only. With `multi` (the version has other
/// passes), finding no selected or explained test is `VersionRun::unmatched`, not an error.
#[allow(clippy::too_many_arguments)]
fn run_version(
    args: &Args,
    root: &Path,
    oracle_path: &Path,
    cfg: &config::OracleFile,
    version: &config::VersionEntry,
    pass: &Pass,
    multi: bool,
    awaited: Option<&mut Vec<al_test_cli::awaiting::AwaitedCase>>,
    process_ends_after: bool,
    grow: PassCompile,
    index: &corpus::CorpusIndex,
    render_note: &mut not_passing::RenderNote,
) -> Result<(VersionRun, Option<runtime::KeptCompile>), String> {
    let mut kept_out = None;
    let r = run_version_inner(
        args,
        root,
        oracle_path,
        cfg,
        version,
        pass,
        multi,
        awaited,
        process_ends_after,
        grow,
        &mut kept_out,
        index,
        render_note,
    )?;
    Ok((r, kept_out))
}

/// The sidecar configuration of a corpus run of BC `major` (ADR-014).
fn sidecar_config(args: &Args, root: &Path, major: u16) -> SidecarConfig {
    SidecarConfig {
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
        // No service tier is loaded, so the BC major's table picks the .NET major (al_testrun::dotnet_runtime). A test
        // (config.rs `every_pinned_build_runs_on_the_net_major_of_the_corpus_sidecar_table`) keeps it equal to the measured
        // major of each `bc_build` of oracle/oracle.toml.
        bc_major: Some(u32::from(major)),
        timeout: Duration::from_secs(args.dotnet_timeout_secs.unwrap_or(60)),
        ..Default::default()
    }
}

/// The DotNet configuration of a run on `host`, with its report renderer (ADR-040 amendment 2026-11-15): BC's Word merger and
/// Html converter of the pinned build `bc_build`, started on the first measured Html save. The renderer comes back too, for
/// what `--baseline-diff` says of it ([`say_renderer`]).
fn dotnet_config(
    host: Arc<dyn al_host::dotnet::DotNetHost>,
    args: &Args,
    root: &Path,
    major: u16,
    bc_build: Option<&str>,
) -> (al_vm::dotnet::DotNetConfig, Arc<RenderHost>) {
    let mut dotnet = al_test_cli::dotnet::sidecar_config(host);
    let renderer = al_testrun::report_render_pieces::run_renderer(
        sidecar_config(args, root, major),
        bc_build.and_then(|b| al_deps::build::BuildId::parse(b).ok()),
        &al_deps::cache::cache_root_from_env(args.cache_dir.as_deref()),
        !args.no_download,
    );
    dotnet.renderer = renderer.clone();
    dotnet.require_renderer = args.require_report_renderer;
    (dotnet, renderer)
}

/// The end of a `--fetch-deps` run of one version. With `--fetch-render-pieces` the report render pieces of the build
/// `bc_build` are fetched too, by the renderer's own fetch (`report_render_pieces::fetch_run_pieces`).
fn fetch_deps_done(
    args: &Args,
    root: &Path,
    major: u16,
    bc: &impl std::fmt::Display,
    bc_build: Option<&str>,
) -> Result<VersionRun, String> {
    if args.fetch_render_pieces {
        let build = bc_build
            .and_then(|b| al_deps::build::BuildId::parse(b).ok())
            .ok_or_else(|| format!("--fetch-render-pieces: no BC build was resolved for BC {bc}"))?;
        let notes = al_testrun::report_render_pieces::fetch_run_pieces(
            sidecar_config(args, root, major).assembly_dirs,
            build,
            &al_deps::cache::cache_root_from_env(args.cache_dir.as_deref()),
            !args.no_download,
        )
        .map_err(|e| format!("--fetch-render-pieces: BC {bc}: {e}"))?;
        // A note here is a fact of the build, or (with --no-download) what the cache lacks: a run then has what it would
        // have after its own fetch on the first save. A failed download is the Err above: the job's fetch step fails and
        // the bc-artifacts action does not save the entry.
        for note in &notes {
            println!("render pieces of BC {bc}: {note}");
        }
        if notes.is_empty() {
            println!("render pieces ready for BC {bc}");
        }
    }
    println!("dependencies ready for BC {bc}");
    Ok(VersionRun::empty(0))
}

/// The corpus apps published before the main app (`deps::corpus_publish_dirs`), without the apps that run as suites.
///
/// The folders some suite lists in `extra_dirs` are left out (CI publishes them in that suite's job only); `own` is
/// the pass's own list, which follows the internals fixture in its order.
fn publish_dirs(root: &Path, cfg: &config::OracleFile, suites: &[config::SuiteEntry]) -> Result<Vec<PathBuf>, String> {
    let mut skip: Vec<PathBuf> = suites.iter().map(|s| root.join(&s.app)).collect();
    let all_extra = cfg.versions.iter().flat_map(|v| &v.suites).flat_map(|s| &s.extra_dirs);
    skip.extend(all_extra.map(|d| root.join(d)));
    let mut dirs = deps::corpus_publish_dirs_without(&root.join(&cfg.corpus_app), &skip)?;
    let own: Vec<PathBuf> = suites
        .first()
        .map(|s| s.extra_dirs.iter().map(|d| root.join(d)).collect())
        .unwrap_or_default();
    // CI's `app_dirs` lists the internals fixture first, then these apps, then the cross-app fixtures.
    let at = dirs
        .iter()
        .position(|d| d.ends_with("al-language-internals-fixture"))
        .map_or(0, |i| i + 1);
    for (n, d) in own.into_iter().enumerate() {
        dirs.insert(at + n, d);
    }
    Ok(dirs)
}

/// The test codeunits of a pass that run once more with `--strict-order` (ADR-066 amendment 12, owner decision of
/// 2026-11-09; `al_test_cli::strict_order`): those of the selected cases whose file `metrics/strict-order-files.txt`
/// lists.
struct StrictOrder {
    /// As `RunOptions::only` numbers them: (app index, codeunit id).
    only: Vec<(usize, u32)>,
    /// (run name, codeunit id), for `CorpusRuntime::adopt_cases`.
    listed: std::collections::BTreeSet<(String, u32)>,
}

impl StrictOrder {
    #[allow(clippy::too_many_arguments)]
    fn of(
        root: &Path,
        index: &corpus::CorpusIndex,
        main_app: usize,
        cases: &[oracle::OracleCase],
        version: &config::VersionEntry,
        app_index: &[(&str, usize)],
        suite_data: &[(Vec<oracle::OracleCase>, corpus::CorpusIndex)],
        suite_cases: &[Vec<oracle::OracleCase>],
    ) -> Result<StrictOrder, String> {
        let files = al_test_cli::strict_order::load(root)?;
        let mut s = StrictOrder {
            only: Vec::new(),
            listed: Default::default(),
        };
        let mut add = |run: &str, app: usize, ix: &corpus::CorpusIndex, cases: &[oracle::OracleCase]| {
            for id in al_test_cli::strict_order::codeunits(&files, ix, cases.iter().map(|c| c.id.codeunit)) {
                s.only.push((app, id));
                s.listed.insert((run.to_string(), id));
            }
        };
        add("", main_app, index, cases);
        for ((suite, (_, ix)), cases) in version.suites.iter().zip(suite_data).zip(suite_cases) {
            if let Some((_, app)) = app_index.iter().find(|(a, _)| *a == suite.app) {
                add(&suite.name, *app, ix, cases);
            }
        }
        s.only.sort_unstable();
        s.only.dedup();
        Ok(s)
    }
}

/// The apps a pass compiles (`run_pass_compile`).
struct PassApps<'a> {
    apps: Vec<al_sema::workspace::AppSource>,
    kinds: Vec<al_sema::AppKind>,
    manifests: &'a [runtime::Manifest],
    symbols: &'a std::collections::BTreeSet<String>,
    target: Option<&'a al_version::Target>,
}

/// Compiles and runs a pass as `grow` says (ADR-063); a compile kept for the next pass goes to `kept_out`. `own` are the
/// suite apps of this pass alone (indexes into the apps).
#[allow(clippy::too_many_arguments)]
fn run_pass_compile(
    grow: PassCompile,
    main: bool,
    own: &[usize],
    p: PassApps<'_>,
    dotnet: al_vm::dotnet::DotNetConfig,
    runs: &[runtime::SuiteRun],
    opts: &runtime::RunOptions,
    kept_out: &mut Option<runtime::KeptCompile>,
) -> Result<runtime::CorpusRuntime, String> {
    let PassApps {
        apps,
        kinds,
        manifests,
        symbols,
        target,
    } = p;
    Ok(match grow {
        PassCompile { kept: Some(kept), keep } => {
            let (rt, kept) = runtime::CorpusRuntime::try_run_suites_grown(
                kept, keep, apps, kinds, manifests, symbols, target, dotnet, runs, opts,
            )?;
            *kept_out = kept;
            rt
        }
        // Only the main pass keeps a compile from source: the apps of another pass's environment are not every later
        // pass's.
        PassCompile { keep: true, .. } if main => {
            // The other passes have every app of this one but its own suite apps.
            let shared: Vec<usize> = (0..apps.len()).filter(|i| !own.contains(i)).collect();
            let (rt, kept) = runtime::CorpusRuntime::try_run_suites_growable(
                apps, kinds, manifests, symbols, target, dotnet, runs, opts, &shared,
            )?;
            *kept_out = kept;
            rt
        }
        PassCompile { .. } => {
            runtime::CorpusRuntime::try_run_suites_with(apps, kinds, manifests, symbols, target, dotnet, runs, opts)?
        }
    })
}

/// ADR-046 decision 2: a `--companies` list names AL Runner's session company first.
fn check_companies(companies: &al_host::Companies) -> Result<(), String> {
    companies
        .check_session_first(&al_host::Identity::company_name(&al_host::AlRunnerIdentity))
        .map_err(|e| format!("--companies: {e}"))
}

/// What the run reads from the artifacts of its BC build: the license (ADR-060) and the emit version of the
/// missing-object error (al-vm `emit_version`, read on its first use).
fn build_facts(
    build: &al_deps::build::BuildId,
    provision: &deps::Provision,
    cache_dir: Option<&Path>,
) -> Result<(al_testrun::license::LicenseUse, al_vm::emit_version::EmitVersion), String> {
    let fetch = al_deps::artifact::Fetch {
        cache_root: al_deps::cache::cache_root_from_env(cache_dir),
        download: provision.download,
        query: provision.query.clone(),
        local_artifacts: provision.local_artifacts.clone(),
        country: provision.country.clone(),
    };
    let license =
        al_testrun::license::LicenseUse::from_artifact(&al_deps::artifact::HttpCdn::from_env(), &fetch, build)?;
    let emit_version = al_testrun::emit_version::of_build(
        build,
        &fetch.cache_root,
        fetch.download,
        fetch.local_artifacts.as_deref(),
    );
    Ok((license, emit_version))
}

/// The ten slowest tests of a run that took any time, slowest first (`report::Timings::slowest_tests`).
fn slowest_tests(results: &[al_testrun::runtime::TestResult]) -> Vec<report::SlowTest> {
    let mut all: Vec<&al_testrun::runtime::TestResult> = results.iter().collect();
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
}

#[allow(clippy::too_many_arguments)]
fn run_version_inner(
    args: &Args,
    root: &Path,
    oracle_path: &Path,
    cfg: &config::OracleFile,
    version: &config::VersionEntry,
    pass: &Pass,
    multi: bool,
    awaited: Option<&mut Vec<al_test_cli::awaiting::AwaitedCase>>,
    process_ends_after: bool,
    grow: PassCompile,
    kept_out: &mut Option<runtime::KeptCompile>,
    index: &corpus::CorpusIndex,
    render_note: &mut not_passing::RenderNote,
) -> Result<VersionRun, String> {
    // ADR-007: the oracle may be recorded for an older corpus commit when the pin only added files.
    pin::check(
        &root.join(&cfg.corpus_repo),
        version.oracle_commit(&cfg.corpus_commit),
        &cfg.corpus_commit,
    )
    .map_err(|e| format!("BC {}: {e}", version.bc_version))?;
    // ADR-046 decision 2: the companies of the oracle's database (`companies` in oracle.toml, amendment 1 A6), or the
    // command line's. The corpus runs in AL Runner's session company, which a list must name first.
    let companies = if args.one_company {
        al_host::Companies::One
    } else {
        match &args.companies {
            Some(c) => c.clone(),
            None => cfg
                .companies_of(version)
                .map_err(|e| format!("{}: {e}", oracle_path.display()))?,
        }
    };
    check_companies(&companies)?;
    al_test_cli::host::set_companies(companies);
    let started = Instant::now();
    let junit_path = oracle_path.parent().unwrap_or(Path::new(".")).join(&version.junit);
    let junit_text = read(&junit_path)?;
    let mut all_cases = oracle::parse_junit(&junit_text).map_err(|e| format!("{}: {e}", junit_path.display()))?;
    // A pass of other suites runs none of the main app's tests (its compile still holds the main app: they depend on it).
    if !pass.main {
        all_cases.clear();
    }
    // When real BC ran each test codeunit: the run clock of a codeunit starts there, so what the oracle recorded from
    // `Today` is what the run sees.
    let mut clocks = oracle::parse_clocks(&junit_text).map_err(|e| format!("{}: {e}", junit_path.display()))?;
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
        Some(pattern) => match find_explained(pattern, &args.select, &all_cases, index, &suite_data, version)? {
            Some(found) => Some(found),
            None if multi => {
                return Ok(VersionRun {
                    unmatched: Some(format!("--explain: no test matches `{pattern}`")),
                    ..VersionRun::empty(0)
                });
            }
            None if !args.all_versions => {
                return Err(format!("--explain: no test matches `{pattern}`"));
            }
            None => {
                // A test an older oracle does not list is not an error across versions.
                println!("BC {}: no test matches `{pattern}`", version.bc_version);
                return Ok(VersionRun::empty(0));
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
            args.select.filter(&all_cases, index),
            suite_data.iter().map(|(c, ix)| args.select.filter(c, ix)).collect(),
        ),
    };
    if selecting && cases.is_empty() && suite_cases_all.iter().all(Vec::is_empty) {
        let why = format!(
            "BC {}: the selection ({}) matches no test",
            version.bc_version,
            args.select.describe()
        );
        if multi {
            return Ok(VersionRun {
                unmatched: Some(why),
                ..VersionRun::empty(0)
            });
        }
        return Err(why);
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
    let symbols = al_version::corpus_symbols(&chosen.target.bc, &cfg.released());
    // The corpus apps CI publishes before the main app (the internals fixture, the cross-app fixtures, including
    // fixture D, which the main app does not depend on), in CI's order, then the main app (corpus
    // `.github/workflows/ci.yml`, `app_dirs`; `deps::corpus_publish_dirs`).
    let corpus_deps = publish_dirs(root, cfg, &version.suites)?;
    // So the apps under test install in that order, also two that do not depend on each other (`install::InstallView`).
    al_test_cli::host::set_publish_order_known(true);
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
    let (mut skipped_dependencies, mut layout_sources) = (Vec::new(), Vec::new());
    let mut bc_build = None;
    // ADR-060: the license of the BC artifacts the run uses; none without them (`--no-deps`, or every dependency on disk).
    let mut license = al_testrun::license::LicenseUse::none("no BC artifact is used by this run");
    let mut emit_version = al_vm::emit_version::EmitVersion::default();
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
        // ADR-013 amendment of 2026-11-10: the corpus tree's `.alpackages` is not searched (`deps::load_corpus`).
        let deps = deps::load_corpus(
            &root.join(&cfg.corpus_app),
            &corpus_deps,
            &args.deps,
            Some(&provision),
            &pass.extra_apps,
        )?;
        deps.log.iter().for_each(|line| eprintln!("{line}"));
        match &deps.build {
            Some(u) => {
                println!("bc build: {} ({})", u.build, origin_word(u.origin));
                chosen.target.bc.resolved_to(&u.build.to_string())?;
                bc_build = Some(u.build.to_string());
                (license, emit_version) = build_facts(&u.build, &provision, args.cache_dir.as_deref())?;
            }
            None => println!("bc build: none needed (every dependency came from --deps)"),
        }
        kinds.extend(deps.apps.iter().map(|_| al_sema::AppKind::Dependency));
        layout_sources = deps.layout_sources();
        manifests.extend(deps.manifests);
        dependency_summary = deps.summary;
        skipped_dependencies = deps.skipped;
        apps.extend(deps.apps);
    }
    println!("{}", license.line());
    if args.fetch_deps {
        return fetch_deps_done(args, root, version.major, &chosen.target.bc, bc_build.as_deref());
    }
    let compile_cache = corpus_compile_cache(args);
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
        return Ok(VersionRun::empty(0));
    }
    // ADR-014: the sidecar starts lazily, at the first `DotNet` call that reaches it.
    let host = Arc::new(SidecarHost::new(sidecar_config(args, root, version.major)));
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
    let (dotnet, renderer) = dotnet_config(host, args, root, version.major, bc_build.as_deref());
    let mut runs = vec![runtime::SuiteRun {
        name: String::new(),
        app: None,
        runner: None,
        driver: None,
    }];
    // The main run takes the corpus app's test codeunits only, as before; another pass does not run them.
    runs[0].app = Some(main_app);
    if !pass.main {
        runs.clear();
    }
    for suite in &version.suites {
        let app = app_index.iter().find(|(a, _)| *a == suite.app).map(|(_, i)| *i);
        runs.push(runtime::SuiteRun {
            name: suite.name.clone(),
            app,
            runner: suite.runner,
            // ADR-074: the driver the suite's oracle was recorded with (its format says which).
            driver: Some(suite.format.driver()),
        });
    }
    // A selection roots the compile at the selected test codeunits (app index, object id); so does a pass without the
    // main app's tests, at its suites' codeunits.
    let only = (selecting || !pass.main).then(|| {
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
    let mut opts = runtime::RunOptions {
        // ADR-061 section 5: the fixed run seed 1 keeps the ratchet repeatable; `--seed N` is for local checks.
        run_seed: match args.seed {
            Some(n) => al_testrun::seed::RunSeed::new(n, al_testrun::seed::SeedSource::Option),
            None => Default::default(),
        },
        only,
        trace: explained.is_some(),
        // The corpus has no test data: its oracle is a database the cases fill themselves.
        test_data: None,
        license: license.license.clone(),
        emit_version,
        company_data: None,
        workspace_cache: al_test_cli::workspace_leg::options(compile_cache.is_some()),
        compile_cache,
        test_time_limit: None,
        lazy: Vec::new(),
        test_runner: args.test_runner.unwrap_or_default(),
        // The corpus CI runs a test codeunit without handlers through BC's TestRunnerHub, which takes its isolation from
        // the codeunit's RequiredTestIsolation, on BC 28 and later; on BC 27 it runs every codeunit under Microsoft's runner
        // 130450 (TestIsolation = Codeunit), where the property has no effect. A difference of the CI's harness, not of
        // the platform (corpus codeunits/run-onrun/QuCommitIsolation.al:10-16 and 36-39).
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
        cell_tap: args.junit.is_some() || args.cell_report.is_some(),
        narrow: None,
        // The oracle leaves the runner's OnRun row out (oracle::RUNNER_PSEUDO_CASE).
        on_run_rows: runtime::OnRunRows::PerTest,
        tdd: false,
        debug_ready: None,
        // The corpus compiles on real BC: an error found here is the front end's, printed, never a reason not to run.
        stop_on_compile_errors: false,
        run_only: None,
        entries: Vec::new(),
        skipped_dependencies,
        layout_sources,
        process_ends_after,
        // The corpus checks permissions where it pins them: a test that lowers its own is never run without them.
        ignore_permission_mock: false,
        no_client: args.no_client,
        // The corpus run is the conformance result: an unpinned error text is never reported as a failure (issue 6487).
        report_unpinned_errors: false,
        permissions_off: false,
        // The suite's CI job sets DisableWriteInsideTryFunctions=false (oracle.toml `server_settings`, job "BC TryWrites").
        allow_write_in_try_functions: pass.settings.allow_write_in_try_functions,
        // `dev_endpoint_ui` stays off: the corpus runs as Microsoft's Test Runner app does, so unhandled UI is an error.
        private_baseline: std::env::var_os("AL_PRIVATE_BASELINE").is_some(), // ADR-069 decision 11
        login_replay_check: std::env::var_os("ALRUNNER_LOGIN_REPLAY_CHECK").is_some(), // issue 8211, internal
        login_replay: std::env::var_os("ALRUNNER_LOGIN_REPLAY").is_some_and(|v| v == "1"), // issue 8211, internal
        login_replay_limits: runtime::LoginReplayLimits::read(|v| std::env::var_os(v))?, // issue 8211
        // Internal: the whole corpus in BC's order (`AL_STRICT_ORDER_ALL`; ADR-070 decision 5).
        strict_order: args.strict_order_all,
        ..Default::default()
    };
    let load_time = started.elapsed();
    // The suite apps of this pass alone: the other passes have every other app of this one.
    let own: Vec<usize> = app_index.iter().map(|(_, a)| *a).collect();
    let target = Some(&chosen.target);
    // ADR-066 amendment 12: the order probes of `metrics/strict-order-files.txt` run once more in BC's order.
    let strict = StrictOrder::of(
        root,
        index,
        main_app,
        &cases,
        version,
        &app_index,
        &suite_data,
        &suite_cases_all,
    )?;
    // The runtime runs them once more over the compile of the pass (no second compile; none under AL_STRICT_ORDER_ALL).
    opts.strict_after = (!strict.only.is_empty()).then_some(runtime::StrictAfter {
        only: strict.only,
        listed: strict.listed,
    });
    let compiled = PassApps {
        apps,
        kinds,
        manifests: &manifests,
        symbols: &symbols,
        target,
    };
    let rt = run_pass_compile(grow, pass.main, &own, compiled, dotnet, &runs, &opts, kept_out)?;
    // Nothing waits for the drop of the runtime after the report is written.
    let rt = runtime::Retiring::new(rt);
    let p = &rt.phases;
    al_test_cli::workspace_leg::say(&version.bc_version, &pass.work_name(), p.workspace_cache.as_ref());
    p.login_replay.report.iter().for_each(|l| eprintln!("{l}")); // ALRUNNER_LOGIN_REPLAY(_CHECK) (issue 8211)
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
        slowest_tests: slowest_tests(rt.results()),
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
            let (n, took) = al_sema::provided::decoded_on_touch();
            eprintln!(
                "compile cache: {n} dependency objects decoded on first touch, {:.2} s",
                took.as_secs_f64()
            );
        }
        for line in slowest_and_chain_lines(p) {
            eprintln!("{line}");
        }
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
    rep.work.insert(pass.work_name(), work::counts_of(&rt.phases.work));
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
    // `--awaiting-assumptions`: every selected case with its file relative to the corpus root.
    if let Some(out) = awaited {
        let corpus = root.join(&cfg.corpus_repo);
        let mut take = |cases: &[oracle::OracleCase], ix: &corpus::CorpusIndex, suite: &str| {
            for c in cases {
                let Some(cu) = ix.codeunits.get(&c.id.codeunit) else {
                    continue;
                };
                let file = cu.file.strip_prefix(&corpus).unwrap_or(&cu.file);
                out.push(al_test_cli::awaiting::AwaitedCase {
                    file: file.to_string_lossy().replace('\\', "/"),
                    name: c.id.name.clone(),
                    outcome: rt.outcome_in(suite, &c.id),
                    bc_failed: c.bc_failed,
                    marks: rt.assumptions_in(suite, &c.id).iter().map(|a| a.info().id).collect(),
                });
            }
        };
        take(&cases, index, "");
        for ((suite, cases), (_, ix)) in version.suites.iter().zip(&suite_cases).zip(&suite_data) {
            take(cases, ix, &suite.name);
        }
    }

    // `--json` and `--markdown` are written by `run_version_passes`, from the reports of every pass.
    let all = args.all_versions;
    let suite_name = |i: usize| rt.phases.suites.get(i).map(|s| s.0.clone()).unwrap_or_default();
    let cells = cell_report::index(rt.results(), &suite_name);
    if let Some(p) = args.junit.as_ref().filter(|_| pass.main) {
        write(
            &per_version(p, version.major, all),
            &output::junit_with_properties(
                &cases,
                &outcome,
                &|c| rt.first_time(c).as_secs_f64(),
                &marks_of,
                &|c| cell_report::properties_of(&cells, "", c),
                bc_build.as_deref(),
            ),
        )?;
    }
    if let Some(p) = &args.cell_report {
        let (path, json, line) = cell_report::prepare(p, pass.main, &pass.work_name(), rt.results(), &suite_name);
        write(&per_version(&path, version.major, all), &json)?;
        println!("BC {} ({}): {line}", version.bc_version, pass.work_name());
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
                &output::junit_with_properties(
                    cases,
                    &|c| rt.outcome_in(&suite.name, c),
                    &|c| rt.time_in(&suite.name, c).as_secs_f64(),
                    &|c| rt.assumptions_in(&suite.name, c).to_vec(),
                    &|c| cell_report::properties_of(&cells, &suite.name, c),
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
    if pass.main {
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
    }
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
            println!("{}", al_testrun::flagged::render_explain(marks, &outcome));
        }
        return Ok(VersionRun {
            code: 0,
            oracle: oracle_ok,
            passing,
            rep: Some(rep),
            unmatched: None,
        });
    }

    // One not-passing list per oracle file (ADR-065): the main app's, then each other test app's.
    let mut targets = Vec::new();
    targets.push(Target {
        label: version.bc_version.clone(),
        major: version.major,
        suite: None,
        list_path: Some(args.baseline.clone().unwrap_or_else(|| root.join(&version.baseline))),
        cases: &cases,
        all: &all_cases,
        recorded: Box::new(|c| rt.first_recorded(c)),
        area_of: Box::new(|cu| index.area_of(cu).map(str::to_string)),
        selected: Box::new(|c| match &explained {
            Some((_, e)) => *c == e.id,
            None => !args.select.is_active() || args.select.matches(c, index),
        }),
    });
    // A pass without the main app's tests leaves its list alone.
    if !pass.main {
        targets.clear();
    }
    for ((suite, suite_cases), (suite_all, suite_index)) in version.suites.iter().zip(&suite_cases).zip(&suite_data) {
        // `--baseline FILE` is the main app's alone (a run without the dependency apps has its own
        // file); an extra app is checked then only against a file named with `--suite-baseline`.
        let named = args.suite_baselines.iter().find(|(n, _)| *n == suite.name);
        let checked = args.baseline.is_none() || named.is_some();
        targets.push(Target {
            label: format!("{} {}", version.bc_version, suite.name),
            major: version.major,
            suite: Some(suite.name.clone()),
            list_path: checked.then(|| named.map_or_else(|| root.join(&suite.baseline), |(_, f)| f.clone())),
            cases: suite_cases,
            all: suite_all,
            recorded: Box::new(|c| rt.recorded_in(&suite.name, c)),
            area_of: Box::new(|cu| suite_index.area_of(cu).map(str::to_string)),
            selected: Box::new(|c| match &explained {
                Some((_, e)) => *c == e.id,
                None => !args.select.is_active() || args.select.matches(c, suite_index),
            }),
        });
    }
    if let Some(n) = args.top_refusals {
        for t in targets.iter().filter(|t| !t.cases.is_empty()) {
            let in_scope = |id: &oracle::CaseId| {
                args.top_refusals_area
                    .as_ref()
                    .is_none_or(|a| (t.area_of)(id.codeunit).is_some_and(|x| x.eq_ignore_ascii_case(a)))
            };
            let outcome = |c: &oracle::CaseId| {
                (t.recorded)(c).unwrap_or_else(|| {
                    report::Outcome::NotRun("test procedure not found in the compiled corpus".into())
                })
            };
            let list = insight::top_refusals(t.cases, &in_scope, &outcome);
            let title = match &args.top_refusals_area {
                Some(a) => format!("BC {} area {a}", t.label),
                None => format!("BC {}", t.label),
            };
            println!("\n{}", insight::render_top(&title, &list, n));
        }
    }
    let code = if args.no_ratchet && !args.baseline_diff {
        0
    } else {
        check_lists(args, root, &targets, Some((&renderer, render_note)))?
    };
    Ok(VersionRun {
        code: if args.no_ratchet { 0 } else { code },
        oracle: oracle_ok,
        passing,
        rep: Some(rep),
        unmatched: None,
    })
}

/// Before the diff of a pass, on stderr like the other notes about the machine: the report renderer's own reason when a
/// Word-layout save of the pass found it unavailable (its probe ran; nothing starts it here). Once a version.
fn say_renderer(renderer: &RenderHost, note: &mut not_passing::RenderNote, targets: &[Target]) {
    let Some(major) = targets.first().map(|t| t.major) else {
        return;
    };
    if let Some(line) = note.line(&format!("BC {major}"), renderer.probed(&al_vm::WORD_HTML_NEEDS)) {
        eprintln!("{line}");
    }
}

/// The outcome a run recorded for a case, `None` when it recorded none.
type Recorded<'a> = dyn Fn(&oracle::CaseId) -> Option<report::Outcome> + 'a;

/// One not-passing list and the cases it is checked over (ADR-065).
struct Target<'a> {
    label: String,
    major: u16,
    /// The suite's name; `None` for the main app.
    suite: Option<String>,
    /// `None`: this app is not checked (`--baseline` names the main app's file alone).
    list_path: Option<PathBuf>,
    /// The selected oracle cases.
    cases: &'a [oracle::OracleCase],
    /// Every case of the oracle.
    all: &'a [oracle::OracleCase],
    /// The outcome the run recorded, `None` when it recorded none.
    recorded: Box<Recorded<'a>>,
    area_of: Box<dyn Fn(u32) -> Option<String> + 'a>,
    /// Whether the selection picks a case (every case when nothing is selected).
    selected: Box<dyn Fn(&oracle::CaseId) -> bool + 'a>,
}

/// The rule of ADR-065 section 2 over each target's list: `--list-new-cases` adds the lines of new cases first,
/// `--prune-not-passing` deletes the stale lines, `--baseline-diff` prints the check. The exit code: 1 when a list
/// does not hold. Every list is checked before any is written, so an error leaves every file as it was (al-runner
/// issue 7625). `render`: the pass's report renderer, for the `--baseline-diff` warning when it is unavailable.
fn check_lists(
    args: &Args,
    root: &Path,
    targets: &[Target],
    render: Option<(&RenderHost, &mut not_passing::RenderNote)>,
) -> Result<u8, String> {
    if let Some((renderer, note)) = render.filter(|_| args.baseline_diff) {
        say_renderer(renderer, note, targets);
    }
    let base = match &args.list_new_cases {
        Some(rev) => Some(al_test_cli::base_oracle::Base::open(root, rev)?),
        None => None,
    };
    let carries = match &base {
        Some(b) => carries(args, root, b, targets)?,
        None => Default::default(),
    };
    let mut failed = false;
    let mut writes: Vec<(PathBuf, String, Vec<String>)> = Vec::new();
    for t in targets {
        let Some(path) = &t.list_path else { continue };
        let text = if path.exists() { read(path)? } else { String::new() };
        let mut list = not_passing::parse(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut notes = Vec::new();
        if let Some(base) = &base {
            let c = not_passing::check(&list, t.all, &*t.selected, &*t.recorded);
            if !c.skipped.is_empty() {
                return Err(format!(
                    "--list-new-cases refused: BC {}: {} case(s) were skipped because .NET is unavailable, so their lines would be guesses",
                    t.label,
                    c.skipped.len()
                ));
            }
            let carry = &carries[&t.major];
            let known = carry_lines(
                t,
                path,
                carry,
                &mut CarryOut {
                    list: &mut list,
                    notes: &mut notes,
                    writes: &mut writes,
                },
                root,
            )?;
            failed |= known.1;
            let new = not_passing::new_cases(&list, t.all, &known.0, &*t.recorded);
            if !new.is_empty() {
                notes.push(format!(
                    "BC {}: {} new case(s) that the oracle at {} does not have and the runtime does not pass, added to {}",
                    t.label,
                    new.len(),
                    base.rev(),
                    path.display()
                ));
                list.extend(new);
            }
        }
        // ADR-070 decision 5: in BC's order the strict list joins the default one, and only its lines can be stale.
        let strict = if args.strict_order_all {
            Some(StrictList::of(root, t, &list)?)
        } else {
            None
        };
        let default_check;
        let c = match &strict {
            Some(s) => &s.check.check,
            None => {
                default_check = not_passing::check(&list, t.all, &*t.selected, &*t.recorded);
                &default_check
            }
        };
        if args.baseline_diff {
            println!(
                "\n{}",
                not_passing::render_diff(&format!("BC {}", t.label), c, &*t.area_of)
            );
        }
        if args.no_ratchet {
            continue;
        }
        if !c.skipped.is_empty() {
            // ADR-014 section 11: not a regression, but never silent.
            eprintln!(
                "{} case(s) skipped, .NET is unavailable on this machine (CI uses --require-dotnet):",
                c.skipped.len()
            );
            for (x, why) in &c.skipped {
                eprintln!("  {} {}: {why}", x.codeunit, x.name);
            }
        }
        let stale: Vec<_> = c.stale().cloned().collect();
        // In BC's order only the strict list is pruned and reported against; the default list is the base.
        let prune_path = strict.as_ref().map_or(path.as_path(), |s| s.path.as_path());
        if let Some(s) = &strict {
            s.report_default_passing(t);
        }
        if args.prune_not_passing && !stale.is_empty() {
            let note = format!(
                "BC {}: {} stale line(s) deleted from {}",
                t.label,
                stale.len(),
                prune_path.display()
            );
            match &strict {
                Some(s) => writes.push((s.path.clone(), s.pruned(&stale), vec![note])),
                None => {
                    notes.push(note);
                    stale.iter().for_each(|x| {
                        list.remove(x);
                    });
                }
            }
        }
        if !notes.is_empty() {
            writes.push((path.clone(), not_passing::format(&list), notes));
        }
        let (lists, prefix) = match &strict {
            Some(s) => (s.lists(path), "AL_STRICT_ORDER_ALL=1 "),
            None => (path.display().to_string(), ""),
        };
        report_check(t, &lists, prune_path, c, args.prune_not_passing, prefix);
        match not_passing::decide(c, args.prune_not_passing, args.require_dotnet) {
            not_passing::Decision::Ok => {}
            not_passing::Decision::Setup(why) => return Err(why),
            not_passing::Decision::Fail => failed = true,
        }
    }
    for (path, text, notes) in writes {
        write(&path, &text)?;
        notes.iter().for_each(|n| println!("{n}"));
    }
    Ok(u8::from(failed))
}

/// ADR-065 amendment 1: the base lists of each target's major carried to head keys (`rename_map::carry`). The main
/// app's base list is the path the run checks (`--baseline` may name its own).
fn carries(
    args: &Args,
    root: &Path,
    base: &al_test_cli::base_oracle::Base,
    targets: &[Target],
) -> Result<std::collections::BTreeMap<u16, al_test_cli::rename_map::Carry>, String> {
    let oracle_path = args.oracle.clone().unwrap_or_else(|| root.join("oracle/oracle.toml"));
    let cfg = config::OracleFile::parse(&read(&oracle_path)?).map_err(|e| format!("{}: {e}", oracle_path.display()))?;
    let dir = oracle_path.parent().unwrap_or(root);
    let mut out = std::collections::BTreeMap::new();
    for t in targets {
        if out.contains_key(&t.major) {
            continue;
        }
        let main_list = targets
            .iter()
            .find(|x| x.major == t.major && x.suite.is_none())
            .and_then(|x| x.list_path.as_ref())
            .and_then(|p| p.strip_prefix(root).ok())
            .map(|p| p.to_string_lossy().into_owned());
        out.insert(t.major, base.carry(root, &cfg, dir, t.major, main_list.as_deref())?);
    }
    Ok(out)
}

/// What `carry_lines` changes: the target's list, the notes to print and the files to write (path, text, notes).
struct CarryOut<'a> {
    list: &'a mut std::collections::BTreeSet<oracle::CaseId>,
    notes: &'a mut Vec<String>,
    writes: &'a mut Vec<(PathBuf, String, Vec<String>)>,
}

/// ADR-065 amendment 1 at `--list-new-cases`, for one target: adds the moved lines to `list`, prints each renamed case
/// that lost its pass and each passing case of this target's base list that the map sends nowhere, and (main app)
/// queues the strict list with its renamed lines moved. Returns the head keys the base cases reached (no new cases) and
/// whether the bump is red.
fn carry_lines(
    t: &Target,
    path: &Path,
    carry: &al_test_cli::rename_map::Carry,
    out: &mut CarryOut<'_>,
    root: &Path,
) -> Result<(std::collections::BTreeSet<oracle::CaseId>, bool), String> {
    use al_test_cli::rename_map;
    let Some(carried) = carry.of(t.suite.as_deref()) else {
        return Ok((Default::default(), false));
    };
    let lines = rename_map::bump_lines(carried, out.list, t.all, &*t.recorded);
    if !lines.moved.is_empty() {
        out.notes.push(format!(
            "BC {}: {} line(s) moved to new keys (the corpus rename map or an app move) in {}",
            t.label,
            lines.moved.len(),
            path.display()
        ));
        out.list.extend(lines.moved);
    }
    let mut red = false;
    for (r, why) in &lines.lost_passes {
        eprintln!("{}", rename_map::lost_pass_line(&t.label, r, why));
        red = true;
    }
    // Each lost case is reported by the target of the base list it came from, so a suite's is not left out.
    // Each lost case is reported by the target of the base list it came from, so a suite's is not left out.
    for l in carry.lost.iter().filter(|l| l.old_suite == t.suite) {
        eprintln!("{}", rename_map::lost_line(&t.label, l));
        red = true;
    }
    if t.suite.is_none() {
        // `--list-new-cases` refuses BC's order (`run`), so the strict list is written here only.
        let strict = root.join(al_test_cli::strict_not_run::path(t.major));
        if strict.exists() {
            if let Some(text) = carry.rewrite_list(&read(&strict)?)? {
                let note = format!("BC {}: renamed lines moved in {}", t.label, strict.display());
                out.writes.push((strict, text, vec![note]));
            }
        }
    }
    Ok((carried.known.clone(), red))
}

/// The strict list of one target in BC's order (`strict_not_run.rs`, ADR-070 decision 5): the main app's
/// `metrics/<major>/strict-order-not-run.txt`; a suite app has none.
struct StrictList {
    path: PathBuf,
    text: String,
    check: al_test_cli::strict_not_run::StrictCheck,
}

impl StrictList {
    fn of(root: &Path, t: &Target, list: &std::collections::BTreeSet<oracle::CaseId>) -> Result<StrictList, String> {
        let path = root.join(al_test_cli::strict_not_run::path(t.major));
        let text = match (&t.suite, path.exists()) {
            (None, true) => read(&path)?,
            _ => String::new(),
        };
        let strict = not_passing::parse(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        let check = al_test_cli::strict_not_run::check(list, &strict, t.all, &*t.selected, &*t.recorded);
        Ok(StrictList { path, text, check })
    }

    /// The lists a case may be on, for the regression line.
    fn lists(&self, default: &Path) -> String {
        format!("{} or {}", default.display(), self.path.display())
    }

    fn pruned(&self, stale: &[oracle::CaseId]) -> String {
        al_test_cli::strict_not_run::prune(&self.text, &stale.iter().cloned().collect())
    }

    fn report_default_passing(&self, t: &Target) {
        let passing = &self.check.default_passing;
        if !passing.is_empty() {
            eprintln!(
                "BC {}: {} case(s) of the default not-passing list pass in BC's order (the default run keeps the line):",
                t.label,
                passing.len()
            );
            passing.iter().for_each(|x| eprintln!("  {} {}", x.codeunit, x.name));
        }
    }
}

/// The failures of a check on standard error: what each case does, and what to do about it. `lists` names the lists a
/// case may be on; `path` is the list whose stale lines `--prune-not-passing` deletes, run with `env` in front.
fn report_check(t: &Target, lists: &str, path: &Path, c: &not_passing::Check, pruned: bool, env: &str) {
    if !c.regressions.is_empty() {
        eprintln!(
            "BC {}: {} regression(s): cases not listed in {} do not pass:",
            t.label,
            c.regressions.len(),
            lists
        );
        for (x, why) in &c.regressions {
            eprintln!("  {} {}  [{why}]", x.codeunit, x.name);
        }
    }
    if !pruned && c.stale().next().is_some() {
        eprintln!(
            "BC {}: {} stale line(s) in {}: delete them with `{env}al-test-cli --root . --prune-not-passing --bc-version {}`:",
            t.label,
            c.stale_passing.len() + c.stale_gone.len(),
            path.display(),
            t.major
        );
        for x in &c.stale_passing {
            eprintln!("  {} {}  [passes]", x.codeunit, x.name);
        }
        for x in &c.stale_gone {
            eprintln!("  {} {}  [not in the oracle]", x.codeunit, x.name);
        }
    }
    if !c.no_outcome.is_empty() {
        eprintln!(
            "BC {}: {} oracle case(s) got no outcome: the runner did not run them and said nothing (a runner bug):",
            t.label,
            c.no_outcome.len()
        );
        for x in &c.no_outcome {
            eprintln!("  {} {}", x.codeunit, x.name);
        }
    }
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

/// The `--timings` lines of the slowest test codeunits and of the carry chain (`al_testrun::runtime::ChainReport`).
fn slowest_and_chain_lines(p: &al_testrun::runtime::Phases) -> Vec<String> {
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
    let mut lines = vec![format!("slowest test codeunits: {}", slowest.join("; "))];
    lines.extend(p.chain.lines());
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    /// al-runner issue 7625: under `--all-versions` the versions are sorted ascending; the newest is the last.
    #[test]
    fn the_newest_version_is_the_highest_major() {
        let cfg = config::OracleFile::parse(
            &std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../oracle/oracle.toml")).unwrap(),
        )
        .unwrap();
        let mut v: Vec<&config::VersionEntry> = cfg.versions.iter().collect();
        v.sort_by_key(|v| v.major);
        let top = cfg.versions.iter().map(|v| v.major).max().unwrap();
        assert_eq!(newest_version(&v).major, top);
    }

    /// ADR-065 amendment 1, item 2: a passing case of a suite's base list that the rename map sends to no oracle makes
    /// that suite's target red; the main app's target reports only its own.
    #[test]
    fn a_lost_case_is_reported_by_the_target_of_its_base_list() {
        use al_test_cli::rename_map::{self, BaseList, HeadOracle};
        let id = |n: &str| oracle::CaseId {
            codeunit: 7,
            name: n.into(),
        };
        let map = rename_map::RenameMap::parse("iso/7/Old\tiso/7/Nowhere\n").unwrap();
        let base = [BaseList {
            suite: Some("iso".into()),
            app: "iso".into(),
            oracle: [id("Old")].into(),
            list: Default::default(),
        }];
        let head = [
            HeadOracle {
                suite: None,
                app: "main".into(),
                cases: Default::default(),
            },
            HeadOracle {
                suite: Some("iso".into()),
                app: "iso".into(),
                cases: [id("Other")].into(),
            },
        ];
        let carry = rename_map::carry(&map, &base, &head).unwrap();
        let d = tempfile::tempdir().unwrap();
        let red = |suite: Option<&str>| {
            let t = Target {
                label: "27.5".into(),
                major: 27,
                suite: suite.map(str::to_string),
                list_path: None,
                cases: &[],
                all: &[],
                recorded: Box::new(|_| None),
                area_of: Box::new(|_| None),
                selected: Box::new(|_| true),
            };
            let (mut list, mut notes, mut writes) = Default::default();
            let mut out = CarryOut {
                list: &mut list,
                notes: &mut notes,
                writes: &mut writes,
            };
            carry_lines(&t, d.path(), &carry, &mut out, d.path()).unwrap().1
        };
        assert!(red(Some("iso")));
        assert!(!red(None));
    }

    /// al-runner issue 7625: an error on a later list leaves every list file as it was, also one an earlier target
    /// would have pruned.
    #[test]
    fn an_error_on_a_later_list_writes_no_list() {
        let d = tempfile::tempdir().unwrap();
        let first = d.path().join("first.txt");
        let second = d.path().join("second.txt");
        std::fs::write(&first, "1 A\n").unwrap();
        std::fs::write(&second, "not a line\n").unwrap();
        let all = vec![oracle::OracleCase {
            id: oracle::CaseId {
                codeunit: 1,
                name: "A".into(),
            },
            bc_failed: false,
        }];
        let target = |path: &Path| Target {
            label: "27.5".into(),
            major: 27,
            suite: None,
            list_path: Some(path.to_path_buf()),
            cases: &all,
            all: &all,
            recorded: Box::new(|_| Some(report::Outcome::Pass)),
            area_of: Box::new(|_| None),
            selected: Box::new(|_| true),
        };
        let args = Args {
            prune_not_passing: true,
            ..Args::default()
        };
        assert!(check_lists(&args, d.path(), &[target(&first), target(&second)], None).is_err());
        assert_eq!(std::fs::read_to_string(&first).unwrap(), "1 A\n");
        // Neighbor: without the error the stale line is pruned, as before.
        assert_eq!(check_lists(&args, d.path(), &[target(&first)], None).unwrap(), 0);
        assert!(!std::fs::read_to_string(&first).unwrap().contains("1 A"));
    }
}
