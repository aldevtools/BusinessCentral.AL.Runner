//! Dependency apps for a corpus run (ADR-013): the project's `.alpackages` is
//! the first place looked up, then any `--deps` directories.

use al_deps::artifact::{ArtifactUse, Fetch, HttpCdn, load_with_artifacts};
use al_deps::build::BuildQuery;
use al_deps::load::{AppJson, Request, WorkspaceApp, load_dependencies, parse_app_json};
use al_deps::select::MissingPolicy;
use al_sema::workspace::AppSource;
use std::path::{Path, PathBuf};

/// Microsoft's Test Runner app (`TestFramework/TestRunner/Test Runner.Source.zip`).
pub const TEST_RUNNER_APP_ID: &str = "23de40a6-dfe8-4f80-80db-d70f83ce8caf";

pub struct Dependencies {
    /// Dependency apps in dependency order.
    pub apps: Vec<AppSource>,
    /// The manifest facts `ModuleInfo` reports for each (publisher and version).
    pub manifests: Vec<crate::runtime::Manifest>,
    /// `name version` of each, for the report.
    pub summary: Vec<String>,
    /// One line per app loaded, and one per problem, for the run's log.
    pub log: Vec<String>,
    /// The Microsoft build the feed was consulted for (Layer A), when some dependency was not
    /// in `.alpackages` or `--deps`. `None`: every dependency was found on disk.
    pub build: Option<ArtifactUse>,
    /// The ids the `Application` aggregate depends on (see [`Dependencies::implied_dependencies`]).
    pub application_aggregate: Vec<String>,
    /// Per app of `apps`: set when the package was read lazily (`load_project_with`), in which case
    /// the app's files hold no text yet.
    pub lazy: Vec<Option<al_sema::LazySource>>,
    /// Per app of `apps`: the package file or the app folder it was read from.
    pub paths: Vec<PathBuf>,
}

impl Dependencies {
    /// What an app with a `platform` and/or `application` floor depends on without declaring it
    /// (only with `implied` on in [`load_project`]): `System`, `Application` and the apps
    /// `Application` propagates. The caller adds these to its own apps' dependencies.
    pub fn implied_dependencies(&self, platform: bool, application: bool) -> Vec<String> {
        al_deps::load::Loaded {
            application_aggregate: self.application_aggregate.clone(),
            ..Default::default()
        }
        .implied_dependencies(platform, application)
    }
}

/// How a run may use Microsoft's artifact feed (ADR-013 Layer A).
pub struct Provision {
    /// `--cache-dir`; else the environment and the default (`al_deps::cache::cache_root_from_env`).
    pub cache_dir: Option<PathBuf>,
    /// False for `--no-download`.
    pub download: bool,
    pub query: BuildQuery,
    /// Extracted artifacts on disk (`~/.bcartifacts.cache/sandbox`), see
    /// [`al_deps::artifact::Fetch::local_artifacts`].
    pub local_artifacts: Option<PathBuf>,
    /// `--country` (lower case); empty or `w1`: the platform artifact's Microsoft apps (ADR-020 amendment).
    pub country: String,
}

fn app_json(dir: &Path) -> Result<AppJson, String> {
    let p = dir.join("app.json");
    let text = std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
    parse_app_json(&text).map_err(|e| format!("{}: {e}", p.display()))
}

/// Resolves and reads the dependencies of the app in `app_dir`. Apps in
/// `source_dirs` are built from source by the caller and satisfy their own dependents.
///
/// A dependency no package supplies is skipped and logged, not an error: the objects it would
/// supply stay unresolved and are refused. This is the offline form, with no feed (the golden
/// test of the unresolved references uses it); see [`load_with`].
pub fn load(app_dir: &Path, source_dirs: &[PathBuf], extra_dirs: &[PathBuf]) -> Result<Dependencies, String> {
    load_with(app_dir, source_dirs, extra_dirs, None)
}

/// Like [`load`], and with `Some(provision)` a dependency that `.alpackages` and `--deps` do not
/// supply is downloaded from Microsoft's artifact feed (the test libraries the corpus names, for
/// one). With `download: false` it is skipped and logged, as in the offline form. A feed that does
/// not answer is an error: a run that expected the test libraries must not quietly lose them.
pub fn load_with(
    app_dir: &Path,
    source_dirs: &[PathBuf],
    extra_dirs: &[PathBuf],
    provision: Option<&Provision>,
) -> Result<Dependencies, String> {
    let project = app_json(app_dir)?;
    let mut workspace_apps = Vec::new();
    for d in source_dirs {
        workspace_apps.push(WorkspaceApp {
            json: app_json(d)?,
            origin: format!("source {}", d.display()),
        });
    }
    let mut search_dirs = vec![app_dir.join(".alpackages")];
    search_dirs.extend(extra_dirs.iter().cloned());
    load_project(&project, &workspace_apps, &search_dirs, provision, None, false)
}

/// `project` with the apps BC's test environment has installed although no test app declares them: Microsoft's Test
/// Runner app, whose codeunits run every test (corpus permissions/lt-ba-testpermissions/BaTpEnv.al:28: AllObj has
/// 130450-130455 on BC 27, 28 and 29; Microsoft's own recipe builds it with the test apps, BCApps
/// `build/projects/Apps W1/.AL-Go/settings.json:120`). It is an optional root, looked up like a declared dependency (the
/// project's packages, `--deps`, the artifact feed when a build is chosen anyway); a run that finds no package for it
/// goes on without it. Loading it does not make it visible: an app sees only what it declares (`al_sema::workspace`).
/// The other apps of the environment are not pinned (Tests-TestLibraries and Prevent Metadata Updates are absent from
/// the corpus environment, BaTpEnv.al:28), so nothing else is added.
fn with_test_environment(project: &AppJson, workspace_apps: &[WorkspaceApp]) -> AppJson {
    let mut project = project.clone();
    let known = |id: &str| id.eq_ignore_ascii_case(TEST_RUNNER_APP_ID);
    let declared = known(&project.id)
        || project.dependencies.iter().any(|d| known(&d.id))
        || workspace_apps.iter().any(|w| known(&w.json.id));
    if !declared {
        project.dependencies.push(al_deps::select::DepDecl {
            id: TEST_RUNNER_APP_ID.into(),
            name: "Test Runner".into(),
            publisher: "Microsoft".into(),
            min: Default::default(),
            optional: true,
        });
    }
    project
}

/// Resolves and reads the dependencies of `project`. `workspace_apps` are built from source by the
/// caller and satisfy their own dependents. `search` is the package search list in order: each
/// entry a directory of packages, a package file or an app folder (`al_deps::load::scan_path`).
/// `source_build` is an exact build that fills Microsoft's build variables in source archives and
/// folders that no record or path names a build for. `implied` makes an app with an `application`
/// or `platform` floor depend on `Application` and `System` without declaring them.
pub fn load_project(
    project: &AppJson,
    workspace_apps: &[WorkspaceApp],
    search: &[PathBuf],
    provision: Option<&Provision>,
    source_build: Option<&str>,
    implied: bool,
) -> Result<Dependencies, String> {
    load_project_with(project, workspace_apps, search, provision, source_build, implied, false)
}

/// [`load_project`] where, with `lazy_sources`, the text of a package's files is left unread
/// (`Dependencies::lazy`). The caller must read it (`LazySource::fill`) or hand the apps to a run that
/// knows (`RunOptions::lazy`); a file of such an app holds an empty text until then.
pub fn load_project_with(
    project: &AppJson,
    workspace_apps: &[WorkspaceApp],
    search: &[PathBuf],
    provision: Option<&Provision>,
    source_build: Option<&str>,
    implied: bool,
    lazy_sources: bool,
) -> Result<Dependencies, String> {
    let project = &with_test_environment(project, workspace_apps);
    let req = Request {
        project,
        workspace_apps,
        search_dirs: search,
        missing: MissingPolicy::Skip,
        source_build,
        application_implies_dependencies: implied,
        lazy_sources,
    };
    let mut log = Vec::new();
    let (loaded, build) = match provision {
        None => (load_dependencies(&req)?, None),
        Some(p) => {
            let fetch = Fetch {
                cache_root: al_deps::cache::cache_root_from_env(p.cache_dir.as_deref()),
                download: p.download,
                query: p.query.clone(),
                local_artifacts: p.local_artifacts.clone(),
                country: p.country.clone(),
            };
            let got = load_with_artifacts(&req, &fetch, &HttpCdn::from_env()).map_err(|e| {
                format!("{e}\n(pass --no-download to run without the dependency apps the artifact feed would supply)")
            })?;
            log.extend(got.log);
            (got.loaded, got.artifact)
        }
    };
    // The Test Runner app is the environment's, not the project's: no package for it is not a problem of the
    // project's dependencies, only a line for the verbose log.
    log.extend(loaded.notes.iter().map(|n| {
        if n.starts_with("optional dependency `Test Runner` ") {
            "test environment: Microsoft's Test Runner app was not found; AllObj reads of its codeunits are refused"
                .into()
        } else {
            format!("dependency note: {n}")
        }
    }));
    for s in &loaded.skipped {
        log.push(format!("dependency skipped: {s}"));
    }
    let mut apps = Vec::new();
    let mut summary = Vec::new();
    let mut manifests = Vec::new();
    let mut lazy = Vec::new();
    let mut paths = Vec::new();
    let application_aggregate = loaded.application_aggregate.clone();
    for mut d in loaded.apps {
        let files = d.app.files.len();
        log.push(format!(
            "dependency: {} {} ({files} files) from {}",
            d.app.name,
            d.version,
            d.path.display()
        ));
        manifests.push(crate::runtime::Manifest {
            publisher: d.publisher.clone(),
            version: d.version.to_string(),
            resources: std::mem::take(&mut d.resources),
            include_source: false,
            // Not read from the package.
            target: String::new(),
            layout_files: Vec::new(),
            id_ranges: Vec::new(),
            symbols: d.preprocessor_symbols.clone(),
        });
        summary.push(format!("{} {}", d.app.name, d.version));
        lazy.push(d.lazy);
        paths.push(d.path);
        apps.push(d.app);
    }
    Ok(Dependencies {
        apps,
        manifests,
        summary,
        log,
        build,
        application_aggregate,
        lazy,
        paths,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app_folder(root: &Path, name: &str, json: &str, files: &[(&str, &str)]) -> PathBuf {
        let dir = root.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("app.json"), json).unwrap();
        for (f, text) in files {
            std::fs::write(dir.join(f), text).unwrap();
        }
        dir
    }

    /// BC's test environment has Microsoft's Test Runner app installed although no test app declares it
    /// (corpus permissions/lt-ba-testpermissions/BaTpEnv.al:28: AllObj has codeunits 130450-130455). The run loads it
    /// from the same search list as the declared dependencies.
    #[test]
    fn the_test_runner_app_is_loaded_although_the_project_does_not_declare_it() {
        let tmp = tempfile::tempdir().unwrap();
        let runner = app_folder(
            tmp.path(),
            "TestRunner",
            &format!(
                r#"{{"id":"{TEST_RUNNER_APP_ID}","name":"Test Runner","publisher":"Microsoft","version":"27.5.1.2","dependencies":[]}}"#
            ),
            &[("Mgt.Codeunit.al", "codeunit 130454 \"Test Runner - Mgt\" { }")],
        );
        let project =
            parse_app_json(r#"{"id":"p","name":"P","publisher":"Me","version":"1.0.0.0","dependencies":[]}"#).unwrap();
        let deps = load_project(&project, &[], &[runner], None, None, false).unwrap();
        assert_eq!(deps.summary, ["Test Runner 27.5.1.2"], "{:?}", deps.log);
    }

    /// An app that declares the Test Runner (or is it) gets it once, as before.
    #[test]
    fn a_project_that_declares_the_test_runner_gets_it_once() {
        let tmp = tempfile::tempdir().unwrap();
        let runner = app_folder(
            tmp.path(),
            "TestRunner",
            &format!(
                r#"{{"id":"{TEST_RUNNER_APP_ID}","name":"Test Runner","publisher":"Microsoft","version":"27.5.1.2","dependencies":[]}}"#
            ),
            &[("Mgt.Codeunit.al", "codeunit 130454 \"Test Runner - Mgt\" { }")],
        );
        let project = parse_app_json(&format!(
            r#"{{"id":"p","name":"P","publisher":"Me","version":"1.0.0.0","dependencies":[{{"id":"{TEST_RUNNER_APP_ID}","name":"Test Runner","publisher":"Microsoft","version":"27.0.0.0"}}]}}"#
        ))
        .unwrap();
        let deps = load_project(&project, &[], &[runner], None, None, false).unwrap();
        assert_eq!(deps.summary, ["Test Runner 27.5.1.2"], "{:?}", deps.log);
    }

    /// Without a package for it the run goes on without it (the AllObj rows of its codeunits stay refused).
    #[test]
    fn a_run_without_the_test_runner_package_goes_on_without_it() {
        let project =
            parse_app_json(r#"{"id":"p","name":"P","publisher":"Me","version":"1.0.0.0","dependencies":[]}"#).unwrap();
        let deps = load_project(&project, &[], &[], None, None, false).unwrap();
        assert!(deps.apps.is_empty(), "{:?}", deps.summary);
        assert!(
            deps.log.iter().all(|l| !l.starts_with("dependency note"))
                && deps.log.iter().any(|l| l.starts_with("test environment:")),
            "{:?}",
            deps.log
        );
    }
}
