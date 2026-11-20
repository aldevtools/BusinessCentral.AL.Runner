//! The equality check of the workspace layer of the compile cache (ADR-022 amendment 2026-11-15, "Equality
//! requirement"): the corpus apps (or one app folder) compiled with Microsoft's apps of the pinned build as dependency
//! layers: without the workspace layer (the reference), with it (a miss that writes the entry), and with it again (a
//! read). The canonical dumps (`al_testrun::workspace_cache::canonical_dump`) of the reference and of the read compile
//! must be identical.
//!
//! [`run`] makes the three compiles in one process; [`prepare`] and [`compile`] make one, so that a check can run each
//! in a process of its own over a kept cache folder (the read is then a new compile in a new process; the test
//! `workspace_cache_equality` does so). The Microsoft apps come from the artifact cache (`al-test-cli --root .
//! --all-versions --fetch-deps`).

use al_testrun::runtime::{self, CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::workspace_cache::{self as wc, WorkspaceCacheOptions, WorkspaceReport, WorkspaceState};
use std::path::Path;
use std::time::{Duration, Instant};

/// One compile of the check.
#[derive(Debug)]
pub struct Step {
    pub name: &'static str,
    pub took: Duration,
    pub report: Option<WorkspaceReport>,
    pub dump_lines: usize,
}

/// What the check found.
#[derive(Debug)]
pub struct Outcome {
    pub steps: Vec<Step>,
    /// The first line where the read compile's dump differs from the reference's, or `None` when they are equal.
    pub difference: Option<String>,
    /// The size of the reference dump.
    pub bytes: usize,
}

impl Outcome {
    /// Equal dumps, and the second compile built the entry the third read.
    pub fn ok(&self) -> bool {
        let state = |i: usize| self.steps.get(i).and_then(|s| s.report.as_ref()).map(|r| r.state);
        self.difference.is_none() && state(1) == Some(WorkspaceState::Built) && state(2) == Some(WorkspaceState::Read)
    }
}

/// The inputs of the compiles of the check.
pub struct Prepared {
    apps: Vec<al_sema::workspace::AppSource>,
    kinds: Vec<al_sema::AppKind>,
    manifests: Vec<runtime::Manifest>,
    lazy: Vec<Option<al_sema::LazySource>>,
    symbols: std::collections::BTreeSet<String>,
    target: al_version::Target,
}

/// The apps of the check for BC `major` of `root`'s oracle. `app`: one app folder instead of the corpus apps.
pub fn prepare(root: &Path, major: u16, app: Option<&Path>) -> Result<Prepared, String> {
    let main_app = app.map_or_else(|| root.join("corpus/tests/al-language"), Path::to_path_buf);
    let publish = match app {
        Some(_) => Vec::new(),
        None => crate::deps::corpus_publish_dirs(&main_app)?,
    };
    let mut apps = publish
        .iter()
        .map(|d| runtime::load_app(d))
        .collect::<Result<Vec<_>, _>>()?;
    apps.push(runtime::load_app(&main_app)?);
    let mut kinds = vec![al_sema::AppKind::Workspace; apps.len()];
    let mut manifests = vec![runtime::Manifest::default(); apps.len()];
    let mut lazy = vec![None; apps.len()];
    let path = root.join("oracle/oracle.toml");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let oracle = crate::config::OracleFile::parse(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    let version = oracle
        .versions
        .iter()
        .find(|v| v.major == major)
        .ok_or_else(|| format!("no BC {major} in oracle.toml"))?;
    let build = version.bc_build.as_deref().ok_or("no pinned build")?;
    let provision = crate::deps::Provision {
        cache_dir: None,
        download: false,
        query: al_deps::build::BuildQuery::new(Some(build), major, None),
        local_artifacts: None,
        country: String::new(),
    };
    let deps = crate::deps::load_corpus(&main_app, &publish, &[], Some(&provision), &[])?;
    kinds.extend(deps.apps.iter().map(|_| al_sema::AppKind::Dependency));
    manifests.extend(deps.manifests);
    lazy.extend(deps.lazy);
    apps.extend(deps.apps);
    Ok(Prepared {
        apps,
        kinds,
        manifests,
        lazy,
        symbols: al_version::corpus_symbols(major),
        target: al_version::Target::for_bc(al_version::BcVersion::parse(build).map_err(|e| e.to_string())?),
    })
}

/// One compile of `p` over the cache folder `cache` (kept as it is), with the workspace layer or without: its canonical
/// dump, what the layer did, and how long it took. The entry is written before it returns.
pub fn compile(
    p: &Prepared,
    cache: &Path,
    workspace: bool,
) -> Result<(String, Option<WorkspaceReport>, Duration), String> {
    let opts = RunOptions {
        compile_cache: Some(al_testrun::compile_cache::CompileCache {
            dir: cache.to_path_buf(),
            require: false,
            rebuild: false,
            country: String::new(),
        }),
        workspace_cache: workspace.then(|| WorkspaceCacheOptions {
            compile_options: String::new(),
            app_json: Vec::new(),
            write_now: true,
            verbose: true,
        }),
        lazy: p.lazy.clone(),
        ..Default::default()
    };
    let started = Instant::now();
    let c = CorpusRuntime::compile_suites(
        p.apps.clone(),
        p.kinds.clone(),
        &p.manifests,
        &p.symbols,
        Some(&p.target),
        &[SuiteRun::everything()],
        &opts,
    )?;
    let took = started.elapsed();
    Ok((wc::canonical_dump(&c), c.workspace_cache().cloned(), took))
}

/// The first line where `read` differs from `fresh`, or `None`.
pub fn difference(fresh: &str, read: &str) -> Option<String> {
    (fresh != read).then(|| match fresh.lines().zip(read.lines()).position(|(a, b)| a != b) {
        Some(n) => format!(
            "line {n}:\n fresh: {:.300}\n read:  {:.300}",
            fresh.lines().nth(n).unwrap_or(""),
            read.lines().nth(n).unwrap_or("")
        ),
        None => format!("{} lines against {}", fresh.lines().count(), read.lines().count()),
    })
}

/// The three compiles in one process, over the cache folder `cache` (emptied first). `keep`: called with each step's
/// name and dump (the example writes them to files).
pub fn run(
    root: &Path,
    major: u16,
    cache: &Path,
    app: Option<&Path>,
    keep: &mut dyn FnMut(&str, &str),
) -> Result<Outcome, String> {
    let p = prepare(root, major, app)?;
    let _ = std::fs::remove_dir_all(cache);
    let mut steps = Vec::new();
    let mut dumps = Vec::new();
    for (name, workspace) in [("fresh", false), ("built", true), ("read", true)] {
        let (dump, report, took) = compile(&p, cache, workspace)?;
        keep(name, &dump);
        steps.push(Step {
            name,
            took,
            report,
            dump_lines: dump.lines().count(),
        });
        // The built dump is a fresh compile's; only the reference and the read one are compared.
        dumps.push(if name == "built" { String::new() } else { dump });
    }
    let bytes = dumps[0].len();
    Ok(Outcome {
        steps,
        difference: difference(&dumps[0], &dumps[2]),
        bytes,
    })
}
