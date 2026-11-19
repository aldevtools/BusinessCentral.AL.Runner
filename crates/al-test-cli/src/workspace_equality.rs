//! The equality check of the workspace layer of the compile cache (ADR-022 amendment 2026-11-15, "Equality
//! requirement"): the corpus apps (or one app folder) compiled with Microsoft's apps of the pinned build as dependency
//! layers, three times in one process over one fresh cache folder: without the workspace layer (the reference), with it
//! (a miss that writes the entry), and with it again (a read). The canonical dumps
//! (`al_testrun::workspace_cache::canonical_dump`) of the reference and of the read compile must be identical.
//!
//! Run by the test `workspace_cache_equality` (CI's release tests) and by the example of the same name. The Microsoft
//! apps come from the artifact cache (`al-test-cli --root . --all-versions --fetch-deps`).

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

/// Runs the check for BC `major` of `root`'s oracle, over the cache folder `cache` (emptied first). `app`: one app folder
/// instead of the corpus apps. `keep`: called with each step's name and dump (the example writes them to files).
pub fn run(
    root: &Path,
    major: u16,
    cache: &Path,
    app: Option<&Path>,
    keep: &mut dyn FnMut(&str, &str),
) -> Result<Outcome, String> {
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
    let symbols = al_version::corpus_symbols(major);
    let target = al_version::Target::for_bc(al_version::BcVersion::parse(build).map_err(|e| format!("{e:?}"))?);
    let _ = std::fs::remove_dir_all(cache);
    let mut steps = Vec::new();
    let mut compile = |name: &'static str, workspace: bool| -> Result<String, String> {
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
            lazy: lazy.clone(),
            ..Default::default()
        };
        let started = Instant::now();
        let c = CorpusRuntime::compile_suites(
            apps.clone(),
            kinds.clone(),
            &manifests,
            &symbols,
            Some(&target),
            &[SuiteRun::everything()],
            &opts,
        )?;
        let took = started.elapsed();
        let dump = wc::canonical_dump(&c);
        keep(name, &dump);
        steps.push(Step {
            name,
            took,
            report: c.workspace_cache().cloned(),
            dump_lines: dump.lines().count(),
        });
        Ok(dump)
    };
    let fresh = compile("fresh", false)?;
    drop(compile("built", true)?);
    let read = compile("read", true)?;
    let difference = (fresh != read).then(|| {
        let at = fresh.lines().zip(read.lines()).position(|(a, b)| a != b);
        match at {
            Some(n) => format!(
                "line {n}:\n fresh: {:.300}\n read:  {:.300}",
                fresh.lines().nth(n).unwrap_or(""),
                read.lines().nth(n).unwrap_or("")
            ),
            None => format!("{} lines against {}", fresh.lines().count(), read.lines().count()),
        }
    });
    Ok(Outcome {
        steps,
        difference,
        bytes: fresh.len(),
    })
}
