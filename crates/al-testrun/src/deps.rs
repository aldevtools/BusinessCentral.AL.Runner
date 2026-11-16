//! Dependency apps for a run (ADR-013). A project's `.alpackages` is the first place looked up, then any `--deps`
//! directories, then the artifact cache of the chosen Microsoft build ([`load_with`]). A corpus run does not look in
//! the corpus tree's `.alpackages` (ADR-013 amendment of 2026-11-10, [`load_corpus`]): its Microsoft apps come only
//! from the build `oracle/oracle.toml` pins.

use al_deps::artifact::{ArtifactUse, Fetch, HttpCdn, load_with_artifacts};
use al_deps::build::BuildQuery;
use al_deps::load::{AppJson, Request, WorkspaceApp, load_dependencies, parse_app_json};
use al_deps::select::{MissingPolicy, SelectError};
use al_sema::workspace::AppSource;
use std::path::{Path, PathBuf};

/// Microsoft's Test Runner app (`TestFramework/TestRunner/Test Runner.Source.zip`).
pub const TEST_RUNNER_APP_ID: &str = al_builtins::moduleinfo::TEST_RUNNER_APP_ID;

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
    /// The dependency apps that were skipped (no package with their source, or none at the declared version), as
    /// `` `Name` by Publisher ``.
    pub skipped: Vec<String>,
    /// The same skipped dependencies with their parts, for a machine-readable report (al-runner issue 7139).
    pub skipped_details: Vec<SkippedDependency>,
}

/// One dependency app that was not loaded. `reason` is the full message of the resolver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedDependency {
    pub id: String,
    pub name: String,
    pub publisher: String,
    /// The declared version, a minimum. Empty when the entry is not about one declared app (a cycle or conflict).
    pub version: String,
    pub reason: String,
}

impl SkippedDependency {
    /// The `skippedDependencies` entry of the JSON outputs.
    pub fn json(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id, "name": self.name, "publisher": self.publisher,
            "version": self.version, "reason": self.reason,
        })
    }
}

impl Dependencies {
    /// Per dependency app (by app id), the reader of the layout files of its package or folder (read when a run asks for
    /// one: `al_vm` `report_layout_export`).
    pub fn layout_sources(&self) -> Vec<(String, al_ir::LayoutSourceRef)> {
        self.apps
            .iter()
            .zip(&self.paths)
            .map(|(app, path)| {
                let source = al_deps::lazy::PackageFiles::new(path.clone());
                (app.id.clone(), al_ir::LayoutSourceRef(std::sync::Arc::new(source)))
            })
            .collect()
    }

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
    let text = al_deps::load::read_text(&p)?;
    parse_app_json(&text).map_err(|e| format!("{}: {e}", p.display()))
}

/// The source folders of the apps the app in `app_dir` depends on that sit next to it (a folder of the same parent
/// holding an `app.json` whose id the app declares, directly or through another such folder), in dependency order:
/// each app after the apps it depends on, apps that do not depend on each other in the order the app declares them.
/// This is how the corpus's main app finds its corpus dependencies (the internals fixture and the cross-app fixtures),
/// which CI publishes before it in that order (corpus `.github/workflows/ci.yml`, `app_dirs`). A dependency with no
/// such folder (a Microsoft app) is not listed: [`load_with`] resolves it from packages.
pub fn corpus_source_dirs(app_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let project = app_json(app_dir)?;
    let parent = app_dir.parent().unwrap_or(Path::new("."));
    let apps = neighbor_apps(parent, Some(app_dir))?;
    let roots: Vec<usize> = project
        .dependencies
        .iter()
        .filter_map(|d| apps.iter().position(|(_, j)| j.id.eq_ignore_ascii_case(&d.id)))
        .collect();
    in_dependency_order(&apps, &roots)
}

/// The corpus apps CI publishes before the app in `app_dir`, in that order: [`corpus_source_dirs`], and each app next to
/// it that it does not depend on whose dependencies are all among those (corpus cross-app fixture D, which depends on
/// fixture A only), right after the last of them it depends on; two such apps at one place in folder-name order. Corpus
/// `.github/workflows/ci.yml` `app_dirs` publishes fixture, A, D, B, C, then the main app. An app that depends on the
/// main app (another suite's test app) or on any other app is not published with them. An app with no dependency (XApp N) is published last, with the apps that depend only on it (XApp T), as CI does.
pub fn corpus_publish_dirs(app_dir: &Path) -> Result<Vec<PathBuf>, String> {
    corpus_publish_dirs_without(app_dir, &[])
}

/// [`corpus_publish_dirs`] without the folders in `suites` (the apps that run as test suites of their own, which are
/// not published with the main app's environment).
pub fn corpus_publish_dirs_without(app_dir: &Path, suites: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    let suites: Vec<PathBuf> = suites
        .iter()
        .map(|d| d.canonicalize().unwrap_or_else(|_| d.clone()))
        .collect();
    let mut dirs = corpus_source_dirs(app_dir)?;
    let parent = app_dir.parent().unwrap_or(Path::new("."));
    let apps = neighbor_apps(parent, Some(app_dir))?;
    // The apps of `apps` left, in folder-name order. One that depends on no app at all goes last; one whose dependencies
    // are all loaded already goes right after the last of them (after earlier ones added there). Repeated until no app
    // fits, so an app that depends on an added one comes after it.
    let id_of = |dir: &PathBuf| apps.iter().find(|(p, _)| p == dir).map(|(_, j)| j.id.clone());
    let mut added: Vec<PathBuf> = Vec::new();
    loop {
        let mut progress = false;
        for (dir, json) in &apps {
            if dirs.contains(dir) || suites.contains(&dir.canonicalize().unwrap_or_else(|_| dir.clone())) {
                continue;
            }
            let ids: Vec<Option<String>> = dirs.iter().map(&id_of).collect();
            let at: Option<Vec<usize>> = json
                .dependencies
                .iter()
                .map(|d| {
                    ids.iter()
                        .position(|i| i.as_ref().is_some_and(|i| i.eq_ignore_ascii_case(&d.id)))
                })
                .collect();
            let Some(at) = at else { continue };
            let pos = match at.into_iter().max() {
                None => dirs.len(),
                Some(last) => {
                    let mut pos = last + 1;
                    while dirs.get(pos).is_some_and(|d| added.contains(d)) {
                        pos += 1;
                    }
                    pos
                }
            };
            dirs.insert(pos, dir.clone());
            added.push(dir.clone());
            progress = true;
        }
        if !progress {
            break;
        }
    }
    Ok(dirs)
}

/// The source folders in `tests_dir` of the apps named `names` (the `name` of their `app.json`, any case) and of the
/// apps there they depend on, in the order of [`corpus_source_dirs`]. A name no folder holds is left out (a Microsoft
/// app). For the must-not-compile cases' `@depends` (corpus `tests/must-not-compile/README.md`).
pub fn corpus_apps_named(tests_dir: &Path, names: &[String]) -> Result<Vec<PathBuf>, String> {
    let apps = neighbor_apps(tests_dir, None)?;
    let roots: Vec<usize> = names
        .iter()
        .filter_map(|n| apps.iter().position(|(_, j)| j.name.eq_ignore_ascii_case(n.trim())))
        .collect();
    in_dependency_order(&apps, &roots)
}

/// The folders of `parent` that hold an `app.json` (other than `exclude`), sorted by folder name, one per app id (the
/// first by name wins, so a duplicate id resolves the same way on every machine). A folder whose `app.json` does not
/// parse is no app here.
fn neighbor_apps(parent: &Path, exclude: Option<&Path>) -> Result<Vec<(PathBuf, AppJson)>, String> {
    let own = exclude.and_then(|e| e.canonicalize().ok());
    let mut entries: Vec<PathBuf> = std::fs::read_dir(parent)
        .map_err(|e| format!("{}: {e}", parent.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.join("app.json").is_file())
        .filter(|p| own.is_none() || p.canonicalize().ok() != own)
        .collect();
    entries.sort();
    let mut apps: Vec<(PathBuf, AppJson)> = Vec::new();
    for dir in entries {
        let Ok(json) = app_json(&dir) else { continue };
        if !apps.iter().any(|(_, j)| j.id.eq_ignore_ascii_case(&json.id)) {
            apps.push((dir, json));
        }
    }
    Ok(apps)
}

/// `roots` and the apps of `apps` they depend on, each after the apps it depends on: a depth-first post-order over the
/// declared dependencies in declaration order, so a dependency comes before the first app that needs it.
fn in_dependency_order(apps: &[(PathBuf, AppJson)], roots: &[usize]) -> Result<Vec<PathBuf>, String> {
    fn visit(at: usize, apps: &[(PathBuf, AppJson)], state: &mut [u8], out: &mut Vec<usize>) -> Result<(), String> {
        match state[at] {
            2 => return Ok(()),
            1 => return Err(format!("{}: dependency cycle", apps[at].0.display())),
            _ => {}
        }
        state[at] = 1;
        for d in &apps[at].1.dependencies {
            if let Some(next) = apps.iter().position(|(_, j)| j.id.eq_ignore_ascii_case(&d.id)) {
                visit(next, apps, state, out)?;
            }
        }
        state[at] = 2;
        out.push(at);
        Ok(())
    }
    let mut state = vec![0u8; apps.len()];
    let mut order = Vec::new();
    for &r in roots {
        visit(r, apps, &mut state, &mut order)?;
    }
    Ok(order.into_iter().map(|i| apps[i].0.clone()).collect())
}

/// Resolves and reads the dependencies of the project in `app_dir`: its own `.alpackages` first, then `extra_dirs`
/// (`--deps`). Apps in `source_dirs` are built from source by the caller and satisfy their own dependents.
///
/// A dependency no package supplies is skipped and logged, not an error: the objects it would supply stay unresolved
/// and are refused. With `Some(provision)` such a dependency is downloaded from Microsoft's artifact feed; with
/// `download: false` it is taken from the artifact cache or skipped and logged. A feed that does not answer is an
/// error: a run that expected an app must not quietly lose it.
pub fn load_with(
    app_dir: &Path,
    source_dirs: &[PathBuf],
    extra_dirs: &[PathBuf],
    provision: Option<&Provision>,
) -> Result<Dependencies, String> {
    let mut search_dirs = vec![app_dir.join(".alpackages")];
    search_dirs.extend(extra_dirs.iter().cloned());
    load_app_dir(app_dir, source_dirs, &search_dirs, provision, &[])
}

/// The dependencies of the corpus app in `app_dir`, for a corpus run and for everything else that reads the corpus
/// with Microsoft's apps (the coverage maps, the golden files).
///
/// ADR-013 amendment of 2026-11-10 (owner): the corpus tree's `.alpackages` is not searched. The only places are
/// `extra_dirs` (`--deps`) and the artifact cache of the build `provision` asks for, which is the build
/// `oracle/oracle.toml` pins. So a local run and CI load the same Microsoft apps, and a package someone staged into
/// the corpus checkout (compile symbols for the AL compiler) changes no result.
///
/// The environment also has `extra` installed, given as (id, name): apps the corpus CI installs for a test app although
/// no app declares them (oracle.toml `extra_apps`, corpus `.github/workflows/ci.yml` job test-testlibraries). They are
/// looked up by id like declared dependencies of the app in `app_dir`, with their own dependencies; one that is not
/// found is skipped and reported like a missing dependency. As for the Test Runner app (`with_test_environment`),
/// loading them does not make them visible to an app that does not declare them.
pub fn load_corpus(
    app_dir: &Path,
    source_dirs: &[PathBuf],
    extra_dirs: &[PathBuf],
    provision: Option<&Provision>,
    extra: &[(String, String)],
) -> Result<Dependencies, String> {
    load_app_dir(app_dir, source_dirs, extra_dirs, provision, extra)
}

/// The dependencies of the app in `app_dir` and of the `extra` apps of its environment, from the search list `search`
/// and then the artifact feed of `provision`.
fn load_app_dir(
    app_dir: &Path,
    source_dirs: &[PathBuf],
    search: &[PathBuf],
    provision: Option<&Provision>,
    extra: &[(String, String)],
) -> Result<Dependencies, String> {
    let mut project = app_json(app_dir)?;
    for (id, name) in extra {
        if !project.dependencies.iter().any(|d| d.id.eq_ignore_ascii_case(id)) {
            project.dependencies.push(al_deps::select::DepDecl {
                id: id.clone(),
                name: name.clone(),
                publisher: "Microsoft".into(),
                min: Default::default(),
                optional: false,
            });
        }
    }
    let mut workspace_apps = Vec::new();
    for d in source_dirs {
        workspace_apps.push(WorkspaceApp {
            json: app_json(d)?,
            origin: format!("source {}", d.display()),
        });
    }
    // A dependency package with an `application` floor sees the `Application` aggregate and what it depends on, as
    // alc compiles it (`a_dependency_package_with_an_application_floor_sees_the_application`: Tests-TestLibraries
    // names Base Application's `Notification Context` without declaring Base Application). The corpus apps declare
    // their own dependencies; the flag changes only what the packages see.
    load_project(&project, &workspace_apps, search, provision, None, true)
}

/// `project` with the apps BC's test environment has installed although no test app declares them: Microsoft's Test
/// Runner app, whose codeunits run every test (corpus permissions/testpermissions/BaTpEnv.al:28: AllObj has
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
    let mut skipped = Vec::new();
    let mut skipped_details = Vec::new();
    for s in &loaded.skipped {
        log.push(format!("dependency skipped: {s}"));
        let dep = match s {
            SelectError::Missing { dep, .. } | SelectError::BelowMinimum { dep, .. } => Some(dep),
            _ => None,
        };
        match dep {
            Some(dep) => skipped.push(format!("`{}` by {}", dep.name, dep.publisher)),
            None => skipped.push(s.to_string()),
        }
        skipped_details.push(SkippedDependency {
            id: dep.map(|d| d.id.clone()).unwrap_or_default(),
            name: dep.map(|d| d.name.clone()).unwrap_or_default(),
            publisher: dep.map(|d| d.publisher.clone()).unwrap_or_default(),
            version: dep.map(|d| d.min.to_string()).unwrap_or_default(),
            reason: s.to_string(),
        });
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
            logo: d.logo.take(),
            description: Some(d.description.clone()),
            layout_files: Vec::new(),
            id_ranges: Vec::new(),
            symbols: d.preprocessor_symbols.clone(),
            translations: d.translations,
            // NavApp.LoadPackageData in its install code reads the data files of an app folder; a package's are not
            // read and its load stays refused (al-vm `package_data`, al-runner issue 6649).
            package_data: d.package_data,
            package_data_files: std::mem::take(&mut d.package_data_files),
            translation_files: Vec::new(),
            // Read on demand: Base Application's translation files are tens of MB each (ADR-053 step 2).
            translation_source: d.translations.then(|| {
                al_ir::TranslationSourceRef(std::sync::Arc::new(al_deps::lazy::PackageTranslations::new(
                    d.path.clone(),
                )))
            }),
            runtime: None,
            app_json_keys: Vec::new(),
            platform_internals_visible_to: std::mem::take(&mut d.platform_internals_visible_to),
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
        skipped,
        skipped_details,
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

    fn json(id: &str, deps: &[&str]) -> String {
        let deps: Vec<String> = deps
            .iter()
            .map(|d| format!(r#"{{"id":"{d}","name":"{d}","publisher":"X","version":"1.0.0.0"}}"#))
            .collect();
        format!(
            r#"{{"id":"{id}","name":"{id}","publisher":"X","version":"1.0.0.0","dependencies":[{}]}}"#,
            deps.join(",")
        )
    }

    /// The corpus's main app depends on other corpus apps (the fixture, and cross-app fixtures A, B, C where B depends
    /// on the fixture and A, C on A and B). They are the folders next to it whose app id the main app declares, directly
    /// or through another of them; a Microsoft dependency is not a folder there. Dependencies come before their
    /// dependents, and apps that do not depend on each other keep the order the main app declares them in.
    #[test]
    fn the_corpus_apps_a_corpus_app_depends_on_come_in_dependency_order() {
        let tmp = tempfile::tempdir().unwrap();
        let t = tmp.path();
        app_folder(t, "fixture", &json("f", &[]), &[]);
        app_folder(t, "xapp-a", &json("a", &[]), &[]);
        app_folder(t, "xapp-b", &json("b", &["f", "a"]), &[]);
        app_folder(t, "xapp-c", &json("c", &["a", "b"]), &[]);
        app_folder(t, "unrelated", &json("u", &[]), &[]);
        // Declared in an order that is not a dependency order: C before B.
        let main = app_folder(t, "main", &json("m", &["f", "c", "b", "a", "ms-system"]), &[]);
        let dirs = corpus_source_dirs(&main).unwrap();
        let names: Vec<String> = dirs
            .iter()
            .map(|d| d.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["fixture", "xapp-a", "xapp-b", "xapp-c"]);
        // An app declared only through another corpus app is included too (C declares A before B, B the fixture).
        let only_c = app_folder(t, "main2", &json("m2", &["c"]), &[]);
        let names: Vec<String> = corpus_source_dirs(&only_c)
            .unwrap()
            .iter()
            .map(|d| d.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["xapp-a", "fixture", "xapp-b", "xapp-c"]);
    }

    /// Corpus CI also publishes a cross-app fixture the main app does not depend on (corpus `.github/workflows/ci.yml`,
    /// `app_dirs`: fixture, A, D, B, C, then the main app; D depends on A only). It goes right after the last corpus app
    /// it depends on. An app with no dependency at all (XApp N) goes last, and an app that depends only on such an app
    /// (XApp T) after it: CI publishes both after C, and the subscriber-order probe needs them
    /// (events/subscriber-order/TtSubscriberOrderTests.al:37-44, `N;A;M;T;`). An app that depends on the main app (a
    /// suite's test app) or on an app that is not loaded (a Microsoft app, or an app that is itself left out) is not one of
    /// them; one that depends only on a loaded app (`on-d`, on D) follows it.
    #[test]
    fn a_fixture_app_outside_the_main_apps_dependencies_is_published_after_its_own() {
        let tmp = tempfile::tempdir().unwrap();
        let t = tmp.path();
        app_folder(t, "fixture", &json("f", &[]), &[]);
        app_folder(t, "xapp-a", &json("a", &[]), &[]);
        app_folder(t, "xapp-b", &json("b", &["f", "a"]), &[]);
        app_folder(t, "xapp-c", &json("c", &["a", "b"]), &[]);
        app_folder(t, "xapp-d", &json("d", &["a"]), &[]);
        app_folder(t, "xapp-n", &json("n", &[]), &[]);
        app_folder(t, "xapp-t", &json("t", &["n"]), &[]);
        app_folder(t, "runner-only", &json("r", &["ms-test-runner"]), &[]);
        app_folder(t, "onprem", &json("o", &["m"]), &[]);
        app_folder(t, "on-d", &json("e", &["d"]), &[]);
        app_folder(t, "mixed", &json("x", &["a", "ms-system"]), &[]);
        let main = app_folder(t, "main", &json("m", &["f", "a", "b", "c", "ms-system"]), &[]);
        let names: Vec<String> = corpus_publish_dirs(&main)
            .unwrap()
            .iter()
            .map(|d| d.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            names,
            [
                "fixture", "xapp-a", "xapp-d", "on-d", "xapp-b", "xapp-c", "xapp-n", "xapp-t"
            ]
        );
    }

    /// A dependency-free app next to the main app that runs as a suite of its own is not published with the main app.
    #[test]
    fn a_suite_app_without_dependencies_is_not_published_with_the_main_app() {
        let tmp = tempfile::tempdir().unwrap();
        let t = tmp.path();
        let suite = app_folder(t, "suite", &json("s", &[]), &[]);
        let main = app_folder(t, "main", &json("m", &[]), &[]);
        assert_eq!(corpus_publish_dirs(&main).unwrap(), vec![suite.clone()]);
        assert!(corpus_publish_dirs_without(&main, &[suite]).unwrap().is_empty());
    }

    /// BC's test environment has Microsoft's Test Runner app installed although no test app declares it
    /// (corpus permissions/testpermissions/BaTpEnv.al:28: AllObj has codeunits 130450-130455). The run loads it
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

    /// A Microsoft dependency package with an `application` floor and no declared Base Application sees the
    /// `Application` aggregate and what it depends on, as alc compiles it: Tests-TestLibraries 29.0
    /// (`ResetStateBeforeTestRun.Codeunit.al:12`, table `Notification Context` of Base Application) declares only
    /// System Application Test Library, Permissions Mock and Application Test Library, and has `application`
    /// 29.0.0.0. It compiles in Microsoft's builds and runs before every test of the corpus testlibraries suite.
    #[test]
    fn a_dependency_package_with_an_application_floor_sees_the_application() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = app_folder(
            tmp.path(),
            "Lib",
            r#"{"id":"lib","name":"Lib","publisher":"Microsoft","version":"29.0.0.0","application":"29.0.0.0","platform":"29.0.0.0","dependencies":[]}"#,
            &[("Lib.Codeunit.al", "codeunit 130301 \"Lib\" { }")],
        );
        let project = tmp.path().join("project");
        std::fs::create_dir_all(&project).unwrap();
        std::fs::write(project.join("app.json"), json("p", &[])).unwrap();
        let deps = load_corpus(&project, &[], &[lib], None, &[("lib".into(), "Lib".into())]).unwrap();
        let app = deps.apps.iter().find(|a| a.id == "lib").expect("lib loaded");
        assert!(
            app.dependencies
                .iter()
                .any(|d| d.eq_ignore_ascii_case(al_deps::select::APPLICATION_APP_ID)),
            "{:?}",
            app.dependencies
        );
    }

    /// ADR-013 amendment of 2026-11-10: a corpus run does not search the `.alpackages` of the corpus app, and an
    /// ordinary project still does. The folder here is an app folder (it holds an `app.json`), which a search list
    /// entry may be (`al_deps::load::scan_path`), so the test needs no compiled package.
    #[test]
    fn a_corpus_run_does_not_search_the_alpackages_of_the_corpus_app() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("project");
        std::fs::create_dir_all(&project).unwrap();
        std::fs::write(project.join("app.json"), json("p", &["staged"])).unwrap();
        app_folder(
            &project,
            ".alpackages",
            r#"{"id":"staged","name":"Staged","publisher":"Microsoft","version":"27.5.0.0","dependencies":[]}"#,
            &[("Staged.Codeunit.al", "codeunit 50100 Staged { }")],
        );
        let names = |d: &Dependencies| d.apps.iter().map(|a| a.name.clone()).collect::<Vec<_>>();
        let as_project = load_with(&project, &[], &[], None).unwrap();
        assert_eq!(names(&as_project), ["Staged"], "{:?}", as_project.log);
        let as_corpus = load_corpus(&project, &[], &[], None, &[]).unwrap();
        assert!(names(&as_corpus).is_empty(), "{:?}", names(&as_corpus));
        assert_eq!(as_corpus.skipped, ["`staged` by X"], "{:?}", as_corpus.log);
    }

    /// An AL source app passed with `--dep` hands its data files to the manifest, so NavApp.LoadPackageData in its install
    /// code reads them in the install baseline (al-runner issue 6649).
    #[test]
    fn a_dependency_app_folder_hands_its_package_data_files_to_the_manifest() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = app_folder(
            tmp.path(),
            "Lib",
            &json("lib", &[]),
            &[("Lib.al", "codeunit 50100 Lib { }")],
        );
        std::fs::create_dir_all(lib.join("res")).unwrap();
        std::fs::write(lib.join("res/Tab50100.navxdata"), b"data").unwrap();
        let project = app_folder(tmp.path(), "project", &json("p", &["lib"]), &[]);
        let deps = load_with(&project, &[], &[lib], None).unwrap();
        let i = deps.apps.iter().position(|a| a.id == "lib").expect("lib loaded");
        assert!(deps.manifests[i].package_data);
        assert_eq!(
            deps.manifests[i].package_data_files,
            vec![("res/Tab50100.navxdata".to_string(), b"data".to_vec())]
        );
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
