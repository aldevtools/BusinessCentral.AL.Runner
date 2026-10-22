//! Test sources (ADR-052): the `microsoft` source, every Microsoft test app of one BC build, and
//! `al-runner test-microsoft`, which runs them by Microsoft's recipe without the user collecting their paths.
//!
//! The listing of the artifact is `al_deps::test_apps`. Here: which candidates are test apps (they hold a test
//! codeunit; test libraries and the test framework do not), the narrowing by `--app`, and one recipe run per test app.
//! Each test app is its own run, so that one app that does not compile or load does not stop the others, and the
//! memory of one app's run is given back before the next one starts.

use crate::args::TestArgs;
use crate::sources_report::{self, AppOutcome, AppResult};
use crate::{apps, bcapps, bcapps_apps, recipe_run};
use al_deps::artifact::{Fetch, HttpCdn};
use al_deps::build::BuildQuery;
use al_deps::remote::RangeSource;
use al_deps::test_apps::{self, Listing};
use std::path::{Path, PathBuf};

/// A Microsoft test app the run will run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestApp {
    /// The name in the artifact (`Tests-ERM`).
    pub name: String,
    pub path: PathBuf,
    /// From its app.json.
    pub id: String,
    pub version: String,
    /// Apps the artifact lacks that this test app needs, as BCApps source folders (`--dep`).
    pub deps: Vec<PathBuf>,
}

/// The test apps of the `microsoft` source, and what was left out and why.
#[derive(Debug, Default)]
pub struct MicrosoftApps {
    pub apps: Vec<TestApp>,
    /// Candidates that are not run: (name, why).
    pub left_out: Vec<(String, String)>,
    /// The test apps taken from BCApps because the artifact does not hold them.
    pub from_bcapps: Vec<String>,
    /// Lines about test folders of Microsoft's project that BCApps does not have, or a reason the BCApps apps were
    /// not read.
    pub bcapps_notes: Vec<String>,
}

/// Which candidates the run takes before anything is read: the names `--app` gives (an unknown name is an error that
/// lists the names that exist), else every candidate. The `_Exclude_` apps (`_Exclude_APIV2_ Tests`) are among them:
/// Microsoft's CI dispatches them (BCApps releases/29.x, run 39519586834: "Dispatching '_Exclude_APIV1_ Tests'").
pub fn choose(names: &[&str], wanted: &[String]) -> Result<Vec<String>, String> {
    if wanted.is_empty() {
        return Ok(names.iter().map(|n| n.to_string()).collect());
    }
    let mut out: Vec<String> = Vec::new();
    for w in wanted {
        match names.iter().find(|n| n.eq_ignore_ascii_case(w)) {
            Some(n) if !out.iter().any(|o| o == n) => out.push(n.to_string()),
            Some(_) => {}
            None => {
                return Err(format!(
                    "--app {w}: the build has no Microsoft test app of that name; the names are: {}",
                    names.join(", ")
                ));
            }
        }
    }
    Ok(out)
}

/// The number of test codeunits in an app's source.
fn test_codeunits(input: &apps::AppInput) -> usize {
    input
        .source
        .files
        .iter()
        .map(|(_, text)| al_testrun::discover::test_codeunit_types(text).len())
        .sum()
}

/// Gets the chosen candidates and keeps those that hold a test codeunit. One that `--app` named and that has none is an
/// error; one taken by default is a dependency (a test library), left out with a line.
pub fn test_apps_of<S: RangeSource>(
    listing: &Listing<S>,
    wanted: &[String],
    extra: &BcappsApps,
) -> Result<MicrosoftApps, String> {
    let mut all: Vec<&str> = listing.names();
    all.extend(extra.candidates.iter().map(|c| c.name.as_str()));
    all.sort_by_key(|n| n.to_ascii_lowercase());
    let names = choose(&all, wanted)?;
    let mut left_out = Vec::new();
    let build = listing.build.to_string();
    let mut out = MicrosoftApps {
        bcapps_notes: extra.notes.clone(),
        ..MicrosoftApps::default()
    };
    for name in names {
        let (path, deps) = match extra.candidates.iter().find(|c| c.name == name) {
            Some(c) => {
                let (path, deps) = extra.fetch(c)?;
                out.from_bcapps.push(name.clone());
                (path, deps)
            }
            None => (listing.get(&name)?, Vec::new()),
        };
        let input = apps::load(&path, Some(&build)).map_err(|e| format!("test app {name}: {e}"))?;
        if test_codeunits(&input) > 0 {
            out.apps.push(TestApp {
                name,
                path,
                id: input.json.id.clone(),
                version: input.manifest.version.clone(),
                deps,
            });
        } else if !wanted.is_empty() {
            return Err(format!(
                "--app {name}: it holds no test codeunit (it is a test library or part of the test framework); the run loads it as a dependency of the test apps that need it"
            ));
        } else {
            left_out.push((
                name,
                "no test codeunit: a test library, loaded where a test app needs it".into(),
            ));
        }
    }
    out.left_out = left_out;
    Ok(out)
}

/// `FILE.<app>.xml` for `FILE.xml`: a file for every test app (the recipe then adds the lane).
pub fn app_file(path: &Path, app: &str) -> PathBuf {
    let safe: String = app
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let stem = path
        .file_stem()
        .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
    let name = match path.extension() {
        Some(e) => format!("{stem}.{safe}.{}", e.to_string_lossy()),
        None => format!("{stem}.{safe}"),
    };
    path.with_file_name(name)
}

/// The arguments of one test app's recipe run.
fn app_args(a: &TestArgs, app: &TestApp, build: &str) -> TestArgs {
    let mut t = a.clone();
    t.apps = vec![app.path.clone()];
    t.deps.extend(app.deps.iter().cloned());
    t.microsoft_apps.clear();
    t.recipe = true;
    t.bc_version = Some(build.to_string());
    t.junit = a.junit.as_deref().map(|p| app_file(p, &app.name));
    t.output_json = match &a.output_json {
        Some(Some(p)) => Some(Some(app_file(p, &app.name))),
        other => other.clone(),
    };
    t
}

/// The `microsoft` source of a run: the build and its test apps.
struct Microsoft {
    build: String,
    country: String,
    set: MicrosoftApps,
    /// Where the listing came from (`the artifact feed`, a local artifact).
    from: String,
}

/// Lists the Microsoft test apps of the run's build and country (ADR-052 1.3). The Legacy lane builds its company with
/// the DemoTool (`recipe_run::lane_args`), so no RapidStart package is fetched.
fn microsoft(a: &TestArgs) -> Result<Microsoft, String> {
    let fetch = Fetch {
        cache_root: al_deps::cache::cache_root_from_env(a.cache_dir.as_deref()),
        download: !a.no_download,
        query: BuildQuery::new(a.bc_version.as_deref(), crate::run::NEWEST_VERIFIED, None),
        local_artifacts: al_deps::cache::bcartifacts_sandbox_from_env(),
        country: a.country().to_string(),
    };
    let listing = test_apps::list(&fetch, &HttpCdn::from_env())?;
    for l in &listing.log {
        eprintln!("{l}");
    }
    let extra = bcapps_apps_of(a, &listing);
    let set = test_apps_of(&listing, &a.microsoft_apps, &extra)?;
    Ok(Microsoft {
        build: listing.build.to_string(),
        country: listing.country.to_string(),
        set,
        from: listing.from.to_string(),
    })
}

/// The test apps of Microsoft's project that the artifact does not hold, with where their sources are.
#[derive(Default)]
pub struct BcappsApps {
    /// Every app of the project's folders, from their app.json.
    apps: Vec<bcapps_apps::BcApp>,
    /// The test apps (test folders) the artifact does not hold, by name.
    candidates: Vec<bcapps_apps::BcApp>,
    /// The artifact's app names (every source archive).
    in_artifact: Vec<String>,
    notes: Vec<String>,
    /// Where the sources are: a `--bcapps` checkout (nothing to fetch) or the cache folder of the release.
    root: PathBuf,
    explicit: bool,
    offline: bool,
    url: String,
    branch: String,
}

impl BcappsApps {
    /// The folder of a test app, fetched with the apps it needs that the artifact lacks; and those apps' folders.
    fn fetch(&self, app: &bcapps_apps::BcApp) -> Result<(PathBuf, Vec<PathBuf>), String> {
        let names: Vec<&str> = self.in_artifact.iter().map(String::as_str).collect();
        let in_artifact = |n: &str| bcapps_apps::has_name(&names, n);
        let need = bcapps_apps::closure(&self.apps, app, &in_artifact);
        if !self.explicit {
            let patterns: Vec<String> = need.iter().map(|a| bcapps_apps::folder_pattern(&a.folder)).collect();
            bcapps::ensure_sources(
                &self.root,
                &patterns,
                self.offline,
                &self.url,
                &self.branch,
                &bcapps::git_sparse_fetch,
            )
            .map_err(|e| format!("test app {}: {e}", app.name))?;
        }
        let path = bcapps_apps::path_of(&self.root, app);
        let deps = need[1..].iter().map(|d| bcapps_apps::path_of(&self.root, d)).collect();
        Ok((path, deps))
    }
}

/// Reads which test apps Microsoft's CI runs (the folders of the projects its `Test Apps <COUNTRY>` project tests)
/// and keeps those the artifact does not hold. Anything that stops it is a note, not an error: the artifact's test
/// apps still run.
fn bcapps_apps_of<S: RangeSource>(a: &TestArgs, listing: &Listing<S>) -> BcappsApps {
    let mut out = BcappsApps {
        in_artifact: listing.source_apps.clone(),
        offline: a.no_download,
        url: std::env::var("ALRUNNER_BCAPPS_URL").unwrap_or_else(|_| bcapps::DEFAULT_URL.to_string()),
        ..BcappsApps::default()
    };
    let (major, minor) = (listing.build.major(), listing.build.minor());
    out.branch = bcapps::branch(major, minor);
    let cache_root = al_deps::cache::cache_root_from_env(a.cache_dir.as_deref());
    let recipe = match bcapps::resolve(
        a.bcapps_dir.as_deref(),
        major,
        minor,
        &cache_root,
        a.no_download,
        &out.url,
        &bcapps::git_fetch,
        &bcapps::has_recipe,
    ) {
        Ok(s) => s.dir,
        Err(e) => {
            out.notes
                .push(format!("the test apps only in BCApps are not read: {e}"));
            return out;
        }
    };
    let folders = match bcapps_apps::project_folders(&recipe, &listing.country) {
        Ok(Some(f)) => f,
        Ok(None) => return out,
        Err(e) => {
            out.notes
                .push(format!("the test apps only in BCApps are not read: {e}"));
            return out;
        }
    };
    out.explicit = a.bcapps_dir.is_some();
    out.root = if out.explicit {
        recipe
    } else {
        bcapps::sources_dir(&cache_root, major, minor)
    };
    let names: Vec<String> = folders.iter().map(|f| f.path.clone()).collect();
    if !out.explicit {
        let patterns: Vec<String> = names.iter().map(|f| bcapps_apps::app_json_pattern(f)).collect();
        if let Err(e) = bcapps::ensure_sources(
            &out.root,
            &patterns,
            a.no_download,
            &out.url,
            &out.branch,
            &bcapps::git_sparse_fetch,
        ) {
            out.notes
                .push(format!("the test apps only in BCApps are not read: {e}"));
            return out;
        }
    }
    let (apps, missing) = bcapps_apps::index(&out.root, &names);
    let test_folder = |path: &str| folders.iter().find(|f| f.path == path && f.test);
    for folder in missing.iter().filter_map(|m| test_folder(m)) {
        out.notes.push(format!(
            "not in BCApps: {} (a test folder of {} without app.json)",
            folder.path, folder.project
        ));
    }
    let listed: Vec<&str> = listing.names();
    let artifact: Vec<&str> = out.in_artifact.iter().map(String::as_str).collect();
    out.candidates = apps
        .iter()
        .filter(|x| test_folder(&x.folder).is_some())
        .filter(|x| !bcapps_apps::has_name(&listed, &x.name) && !bcapps_apps::has_name(&artifact, &x.name))
        .cloned()
        .collect();
    out.apps = apps;
    out
}

/// `al-runner test-microsoft` without an APP (ADR-052 section 2, build step 1); with APPs it is a run with the
/// `microsoft` source and no own tests.
pub fn test_microsoft(a: &TestArgs) -> Result<u8, String> {
    if !a.apps.is_empty() {
        return affected(a, true);
    }
    let ms = microsoft(a)?;
    let header = format!(
        "Microsoft tests: BC {} {}, {} test app(s), from {}",
        ms.build,
        ms.country,
        ms.set.apps.len(),
        ms.from
    );
    eprintln!("{header}");
    for (name, why) in &ms.set.left_out {
        eprintln!("  left out: {name}: {why}");
    }
    if !ms.set.from_bcapps.is_empty() {
        eprintln!("  from BCApps (not in the artifact): {}", ms.set.from_bcapps.join(", "));
    }
    for note in &ms.set.bcapps_notes {
        eprintln!("  {note}");
    }
    if a.list_lanes {
        return list_lanes(a, &ms);
    }
    let mut results: Vec<AppResult> = Vec::new();
    for (i, app) in ms.set.apps.iter().enumerate() {
        eprintln!("== test app {} ({} of {}) ==", app.name, i + 1, ms.set.apps.len());
        let outcome = match recipe_run::run_lanes(&app_args(a, app, &ms.build)) {
            Ok(run) => {
                for line in recipe_run::summary_lines(&run.results, run.unrun) {
                    println!("{line}");
                }
                AppOutcome::Ran {
                    lanes: run.results,
                    unrun: run.unrun,
                }
            }
            Err(e) => {
                eprintln!("error: test app {}: {e}", app.name);
                AppOutcome::Error(e.lines().next().unwrap_or_default().to_string())
            }
        };
        results.push(AppResult {
            name: app.name.clone(),
            outcome,
        });
    }
    for line in sources_report::summary_lines(&header, &results) {
        println!("{line}");
    }
    Ok(sources_report::worst(results.iter().map(AppResult::exit_code)))
}

/// `al-runner test-microsoft --list-lanes`: the lane (Microsoft's build mode) of every test app's codeunits, with
/// codeunits, tests, the tests Microsoft disables and the tests that would run, then one total per lane. Only the
/// sources are read; nothing is compiled or run. A test app whose lanes cannot be planned is one error line (exit 2).
fn list_lanes(a: &TestArgs, ms: &Microsoft) -> Result<u8, String> {
    let words = |w: &[&str]| w.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let mut rows = vec![words(&[
        "test app",
        "lane",
        "codeunits",
        "tests",
        "disabled",
        "would run",
    ])];
    let mut verbose = vec![words(&[
        "codeunit",
        "test app",
        "lane",
        "tests",
        "disabled",
        "would run",
    ])];
    // Lane -> (codeunits, tests, disabled).
    let mut totals: Vec<(String, [usize; 3])> = Vec::new();
    let mut code = 0;
    for app in &ms.set.apps {
        let planned = match recipe_run::plan_lanes(&app_args(a, app, &ms.build)) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("error: test app {}: {e}", app.name);
                code = 2;
                continue;
            }
        };
        // Every lane of the app reads the same disabled tests (the app's own folder): read them once per app.
        let disabled = match planned.lanes.first() {
            Some(lane) => crate::run::load_disabled(&lane.args)?,
            None => None,
        };
        for lane in &planned.lanes {
            let off = |c: u32, t: &str| disabled.as_ref().is_some_and(|d| d.is_disabled(c, t));
            let mut sum = [0usize; 3];
            for &c in &lane.args.codeunits {
                let names = planned.tests.get(&c).map_or(&[][..], Vec::as_slice);
                let d = names.iter().filter(|t| off(c, t)).count();
                sum[0] += 1;
                sum[1] += names.len();
                sum[2] += d;
                if a.verbose {
                    verbose.push(vec![
                        format!("codeunit {c}"),
                        app.name.clone(),
                        lane.build_mode.clone(),
                        names.len().to_string(),
                        d.to_string(),
                        (names.len() - d).to_string(),
                    ]);
                }
            }
            rows.push(vec![
                app.name.clone(),
                lane.build_mode.clone(),
                sum[0].to_string(),
                sum[1].to_string(),
                sum[2].to_string(),
                (sum[1] - sum[2]).to_string(),
            ]);
            match totals.iter_mut().find(|(m, _)| *m == lane.build_mode) {
                Some((_, t)) => (0..3).for_each(|i| t[i] += sum[i]),
                None => totals.push((lane.build_mode.clone(), sum)),
            }
        }
        if planned.unrun > 0 {
            rows.push(vec![
                app.name.clone(),
                "no lane (AITest, or RequiredTestIsolation = Disabled outside UnitTest)".into(),
                planned.unrun.to_string(),
            ]);
        }
    }
    let mut lines = crate::matrix::table(&rows);
    lines.push(String::new());
    let mut total_rows = vec![words(&[
        "total per lane",
        "codeunits",
        "tests",
        "disabled",
        "would run",
    ])];
    for (mode, t) in &totals {
        total_rows.push(vec![
            format!("lane {mode}"),
            t[0].to_string(),
            t[1].to_string(),
            t[2].to_string(),
            (t[1] - t[2]).to_string(),
        ]);
    }
    lines.extend(crate::matrix::table(&total_rows));
    if a.verbose {
        lines.push(String::new());
        lines.extend(crate::matrix::table(&verbose));
    }
    for l in lines {
        println!("{l}");
    }
    Ok(code)
}

/// The number of objects in an app's source.
fn objects(input: &apps::AppInput) -> usize {
    input
        .source
        .files
        .iter()
        .map(|(_, text)| {
            let parsed = al_syntax::parse(text, &Default::default());
            al_syntax::ast::SourceFile::cast(parsed.syntax()).map_or(0, |f| f.objects().count())
        })
        .sum()
}

/// The APPs of a run with test sources (ADR-052 1.1): the extension (apps with an object that is not a test
/// codeunit) and the own test apps (apps with a test codeunit). An app with both is both.
#[derive(Debug, Default)]
pub struct Split {
    pub extension: Vec<PathBuf>,
    pub own_tests: Vec<PathBuf>,
    /// `name version` of each own test app.
    pub own_names: Vec<String>,
    /// (id, name) of each extension app.
    pub extension_apps: Vec<(String, String)>,
    /// The id of every APP.
    pub app_ids: Vec<String>,
}

pub fn split(a: &TestArgs) -> Result<Split, String> {
    let mut out = Split::default();
    for p in &a.apps {
        let input = apps::load(p, a.bc_version.as_deref())?;
        let tests = test_codeunits(&input);
        out.app_ids.push(input.json.id.clone());
        if tests > 0 {
            out.own_tests.push(p.clone());
            out.own_names
                .push(format!("{} {}", input.json.name, input.manifest.version));
        }
        if tests == 0 || objects(&input) > tests {
            out.extension.push(p.clone());
            out.extension_apps
                .push((input.json.id.clone(), input.json.name.clone()));
        }
    }
    Ok(out)
}

fn quote(s: &str) -> String {
    if s.is_empty() || s.contains(|c: char| c.is_whitespace() || c == '\'') {
        format!("'{}'", s.replace('\'', "'\\''"))
    } else {
        s.to_string()
    }
}

/// The command that builds the impact index of one run of a source (ADR-052 1.8): `head`, then the options that are
/// part of the index's key.
fn index_command(a: &TestArgs, head: String) -> String {
    let mut out = head;
    if a.country() != "w1" {
        out.push_str(&format!(" --country {}", a.country()));
    }
    for (flag, path) in [("--bcapps", &a.bcapps_dir), ("--cache-dir", &a.cache_dir)] {
        if let Some(p) = path {
            out.push_str(&format!(" {flag} {}", quote(&p.to_string_lossy())));
        }
    }
    for d in &a.deps {
        out.push_str(&format!(" --dep {}", quote(&d.to_string_lossy())));
    }
    for d in &a.package_caches {
        out.push_str(&format!(" --package-cache {}", quote(&d.to_string_lossy())));
    }
    for d in &a.defines {
        out.push_str(&format!(" --define {}", quote(d)));
    }
    out
}

/// The command that builds the index of one Microsoft test app.
fn microsoft_index_command(a: &TestArgs, app: &str, build: &str) -> String {
    index_command(
        a,
        format!(
            "al-runner impact-index build --from microsoft --microsoft-app {} --bc-version {build}",
            quote(app)
        ),
    )
}

/// The arguments of one lane's impact run of a Microsoft test app: the lane's codeunits, the extension, and none of
/// the options that apply to own tests only (ADR-052 1.2).
fn source_lane_args(lane: &TestArgs, extension: &[PathBuf], index_command: String) -> TestArgs {
    let mut t = lane.clone();
    t.impact_of = extension.to_vec();
    t.affected_from.clear();
    t.junit = None;
    t.output_json = None;
    t.impact_json = None;
    t.accept_source = Some(MICROSOFT.into());
    t.index_command = Some(index_command);
    t
}

/// The options of own tests only (`--filter`, `--codeunit`, ...) are not given to the sources.
fn source_base(a: &TestArgs) -> TestArgs {
    let mut t = a.clone();
    t.codeunits.clear();
    t.filters.clear();
    t.exact.clear();
    t.excludes.clear();
    t
}

/// The name of the `microsoft` source in the report, JUnit and the JSON documents.
const MICROSOFT: &str = "microsoft";

/// What a source given as a path refuses when the app or one of its dependencies has no AL source (ADR-052 1.5).
const COMPILED_ONLY: &str = "is a compiled-only app; AL Runner compiles test apps from their AL source (ADR-013). Ask the vendor for a package with source, or a `*.Source.zip`.";

/// An ISV test app given as a source (ADR-052 1.1).
#[derive(Debug, Clone)]
pub struct Isv {
    pub path: PathBuf,
    pub id: String,
    /// From its app.json: a source is named by it, never by its path.
    pub name: String,
    pub version: String,
}

/// One test source of a run, in the order given.
#[derive(Debug, Clone)]
pub enum Source {
    Microsoft,
    Isv(Isv),
}

/// The apps a path given as a source holds: the path itself, or for a folder without `app.json` every app folder,
/// `.app` and `*.Source.zip` in it, in name order.
fn source_paths(given: &Path) -> Result<Vec<PathBuf>, String> {
    if !given.is_dir() || given.join("app.json").is_file() {
        return Ok(vec![given.to_path_buf()]);
    }
    let mut out: Vec<PathBuf> = std::fs::read_dir(given)
        .map_err(|e| format!("--affected-from {}: {e}", given.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_default();
            if p.is_dir() {
                p.join("app.json").is_file()
            } else {
                name.ends_with(".app") || name.ends_with(".source.zip")
            }
        })
        .collect();
    out.sort();
    if out.is_empty() {
        return Err(format!(
            "--affected-from {}: the folder holds no test app (an app folder with an app.json, an .app or a *.Source.zip)",
            given.display()
        ));
    }
    Ok(out)
}

/// The folders a source's dependencies are looked up in (ADR-052 1.2 item 3): `--dep`, `--package-cache`, the
/// `.alpackages` folder of each folder APP and of the source folder.
fn search_paths(a: &TestArgs, source: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = a.deps.clone();
    out.extend(a.package_caches.iter().cloned());
    for p in a.apps.iter().map(PathBuf::as_path).chain([source]) {
        let packages = p.join(".alpackages");
        if p.is_dir() && packages.is_dir() && !out.contains(&packages) {
            out.push(packages);
        }
    }
    out
}

/// A dependency of the source that is found only as a package without AL source (ADR-052 1.5). Microsoft's apps come
/// from the artifact feed and are not looked at here; a dependency that is not found at all fails the source's run,
/// as for any APP.
fn compiled_only_dependency(a: &TestArgs, split: &Split, source: &Path, input: &apps::AppInput) -> Result<(), String> {
    let search = search_paths(a, source);
    for d in &input.json.dependencies {
        if apps::is_microsoft(&d.publisher) || split.app_ids.iter().any(|id| id.eq_ignore_ascii_case(&d.id)) {
            continue;
        }
        let found: Vec<al_deps::load::Found> = search
            .iter()
            .flat_map(|p| al_deps::load::scan_path(p, a.bc_version.as_deref()).0)
            .filter(|f| f.manifest.id.eq_ignore_ascii_case(&d.id))
            .collect();
        if let Some(f) = found.first().filter(|_| found.iter().all(|f| !f.has_source)) {
            return Err(format!(
                "{} {} (a dependency of {}, {}) {COMPILED_ONLY}",
                f.manifest.name,
                f.manifest.version,
                input.json.name,
                f.path.display()
            ));
        }
    }
    Ok(())
}

/// The sources of a run, read and checked before anything runs (ADR-052 1.5): a compiled-only source or dependency,
/// a source that depends on the extension, the same app twice and a source that is also an APP exit 2.
pub fn resolve(a: &TestArgs, split: &Split) -> Result<Vec<Source>, String> {
    let mut out: Vec<Source> = Vec::new();
    for given in &a.affected_from {
        if given == MICROSOFT {
            out.push(Source::Microsoft);
            continue;
        }
        for path in source_paths(Path::new(given))? {
            let input = apps::load(&path, a.bc_version.as_deref())
                .map_err(|e| format!("--affected-from {}: {e}", path.display()))?;
            let label = format!("{} {}", input.json.name, input.manifest.version);
            if input.source.files.is_empty() {
                return Err(format!("--affected-from {}: {label} {COMPILED_ONLY}", path.display()));
            }
            if split.app_ids.iter().any(|id| id.eq_ignore_ascii_case(&input.json.id)) {
                return Err(format!(
                    "--affected-from {}: {label} is also an APP: an app is either under test or a test source",
                    path.display()
                ));
            }
            let twice = out
                .iter()
                .any(|s| matches!(s, Source::Isv(i) if i.id.eq_ignore_ascii_case(&input.json.id)));
            if twice {
                return Err(format!("--affected-from {label} is given twice ({})", path.display()));
            }
            for d in &input.json.dependencies {
                if let Some((_, name)) = split
                    .extension_apps
                    .iter()
                    .find(|(id, _)| id.eq_ignore_ascii_case(&d.id))
                {
                    return Err(format!(
                        "--affected-from {}: {label} depends on {name}: it is your own test app; pass it as an APP",
                        path.display()
                    ));
                }
            }
            compiled_only_dependency(a, split, &path, &input)?;
            out.push(Source::Isv(Isv {
                path,
                id: input.json.id.clone(),
                name: input.json.name.clone(),
                version: input.manifest.version.clone(),
            }));
        }
    }
    Ok(out)
}

/// One impact run of a source (a Microsoft test app's lane, or the one run of an ISV test app).
struct RunOut {
    test_app: Option<String>,
    lane: Option<String>,
    mode: &'static str,
    code: u8,
    /// Its `--impact-json` document (format 1).
    body: Option<serde_json::Value>,
}

/// What one source came to: its row of the summary, its JUnit parts, its runs and the accept file's entries no run of
/// it matched.
struct SourceOut {
    row: sources_report::PartRow,
    /// `microsoft` or `path`.
    kind: &'static str,
    /// The path given, for a source given as a path.
    path: Option<PathBuf>,
    /// (name, id, version) of its test apps.
    test_apps: Vec<(String, String, String)>,
    junit: Vec<crate::output::JunitPart>,
    runs: Vec<RunOut>,
    /// Entries that matched no change in any run of this source so far; `None` before the first run.
    stale: Option<Vec<crate::impact::Accept>>,
    modes: std::collections::BTreeMap<&'static str, usize>,
    not_selected: usize,
}

impl SourceOut {
    fn new(name: &str, kind: &'static str) -> SourceOut {
        SourceOut {
            row: sources_report::PartRow::source(name),
            kind,
            path: None,
            test_apps: Vec::new(),
            junit: Vec::new(),
            runs: Vec::new(),
            stale: None,
            modes: Default::default(),
            not_selected: 0,
        }
    }

    fn add(&mut self, part: crate::impact::Part, test_app: Option<&str>, lane: Option<&str>) {
        self.row.add(&part);
        if !part.mode.is_empty() {
            *self.modes.entry(part.mode).or_default() += 1;
        }
        self.not_selected += part.not_selected.unwrap_or(0);
        if let Some(mut j) = part.junit {
            j.suite_properties = vec![("test_source".into(), self.row.name.clone())];
            self.junit.push(j);
        }
        self.stale = Some(match self.stale.take() {
            None => part.stale,
            Some(prev) => prev.into_iter().filter(|e| part.stale.contains(e)).collect(),
        });
        self.runs.push(RunOut {
            test_app: test_app.map(str::to_string),
            lane: lane.map(str::to_string),
            mode: part.mode,
            code: part.code,
            body: part.json,
        });
    }

    /// A run of the source that did not load or compile: exit 2 for the source; the other runs go on.
    fn failed(&mut self) {
        self.row.code = sources_report::worst([self.row.code, 2]);
    }

    /// The selection line, the accept file's stale entries, and the `not_selected` count on the first JUnit suite.
    fn finish(&mut self, notes: &[String]) {
        let selection: Vec<String> = self
            .modes
            .iter()
            .map(|(m, n)| match (self.kind, *m) {
                (MICROSOFT, m) => format!("{m} selection in {n} run(s)"),
                (_, "all") => "every test ran (--impact all)".to_string(),
                (_, m) => format!("{m} selection"),
            })
            .collect();
        println!(
            "{}: {}; not selected: {} test(s) do not reach your extension",
            self.row.name,
            if selection.is_empty() {
                "no impact run".to_string()
            } else {
                selection.join(", ")
            },
            self.not_selected
        );
        for n in notes {
            println!("  {n}");
        }
        if let Some(stale) = self.stale.as_deref().filter(|s| !s.is_empty()) {
            let refs: Vec<&crate::impact::Accept> = stale.iter().collect();
            for line in crate::impact::stale_lines(&refs) {
                println!("{line}");
            }
        }
        self.row.not_selected = self.not_selected;
        if self.junit.is_empty() {
            self.junit.push(crate::output::JunitPart {
                suite_properties: vec![("test_source".into(), self.row.name.clone())],
                empty_name: self.row.name.clone(),
                ..Default::default()
            });
        }
        self.junit[0].first_suite_properties = vec![("not_selected".to_string(), self.not_selected.to_string())];
    }

    /// The format-1 documents of its runs, each with `source` (and for `microsoft` the test app and lane): the
    /// entries of `--impact-json` format 2 (ADR-052 1.6).
    fn impact_entries(&self) -> Vec<serde_json::Value> {
        self.runs
            .iter()
            .filter_map(|r| {
                let mut body = r.body.clone()?;
                body["source"] = self.row.name.clone().into();
                if let Some(app) = &r.test_app {
                    body["testApp"] = app.clone().into();
                }
                if let Some(lane) = &r.lane {
                    body["lane"] = lane.clone().into();
                }
                Some(body)
            })
            .collect()
    }

    /// Its entry of the `sources` array of `--output-json` (ADR-052 1.6).
    fn output_entry(&self) -> serde_json::Value {
        use serde_json::{Value, json};
        let mut counts = serde_json::Map::new();
        let mut tests: Vec<Value> = Vec::new();
        for entry in self.impact_entries() {
            if let Some(c) = entry["counts"].as_object() {
                for (k, v) in c {
                    let n = counts.get(k).and_then(Value::as_u64).unwrap_or(0) + v.as_u64().unwrap_or(0);
                    counts.insert(k.clone(), json!(n));
                }
            }
            for t in entry["tests"].as_array().into_iter().flatten() {
                let mut t = t.clone();
                for key in ["testApp", "lane"] {
                    if !entry[key].is_null() {
                        t[key] = entry[key].clone();
                    }
                }
                tests.push(t);
            }
        }
        let modes: Vec<&str> = self.modes.keys().copied().collect();
        let mut doc = json!({
            "source": self.row.name,
            "kind": self.kind,
            "testApps": self.test_apps.iter().map(|(name, id, version)| json!({"name": name, "id": id, "version": version})).collect::<Vec<_>>(),
            "mode": match modes.as_slice() {
                [] => Value::Null,
                [one] => json!(one),
                _ => json!("mixed"),
            },
            "counts": counts,
            "notSelected": self.not_selected,
            "exitCode": self.row.code,
            "tests": tests,
        });
        if let Some(p) = &self.path {
            doc["path"] = json!(p.display().to_string());
        }
        if self.kind == MICROSOFT {
            doc["lanes"] = self
                .runs
                .iter()
                .map(|r| json!({"testApp": r.test_app, "lane": r.lane, "mode": r.mode, "exitCode": r.code}))
                .collect();
        }
        if let Some(e) = &self.row.error {
            doc["error"] = json!(e);
        }
        doc
    }
}

/// `al-runner test APP... --affected-from SOURCE...` (ADR-052 sections 1, 2): the own tests as a plain run, then each
/// source in the order given, as impact runs of the extension (the `microsoft` source per test app and lane). One
/// report, one JUnit file, the `--output-json` and `--impact-json` documents of every part, the exit code of the worst
/// part. `only_sources`: `test-microsoft APP`, where an APP with tests is refused.
pub fn affected(a: &TestArgs, only_sources: bool) -> Result<u8, String> {
    let started = std::time::Instant::now();
    let split = split(a)?;
    if only_sources && !split.own_tests.is_empty() {
        return Err(format!(
            "{} holds test codeunits: al-runner test-microsoft APP takes extensions only; use al-runner test APP --affected-from microsoft to also run your own tests",
            split.own_names.join(", ")
        ));
    }
    if split.extension.is_empty() {
        return Err("nothing to measure: --affected-from needs an app without test codeunits. To run Microsoft's tests, use al-runner test-microsoft".into());
    }
    let sources = resolve(a, &split)?;
    // A wrong accept file or an unwritable output path stops the run before the long part.
    let accepts = a
        .impact_accept
        .as_deref()
        .map(crate::impact::Accepts::load)
        .transpose()?;
    for path in [&a.junit, &a.impact_json].into_iter().flatten() {
        crate::run::write_file(path, "")?;
    }
    if let Some(Some(path)) = &a.output_json {
        crate::run::write_file(path, "")?;
    }
    let mut rows: Vec<sources_report::PartRow> = Vec::new();
    let mut junit: Vec<crate::output::JunitPart> = Vec::new();
    let mut own_results: Vec<al_testrun::runtime::TestResult> = Vec::new();
    let mut own_code: Option<u8> = None;

    if !split.own_tests.is_empty() {
        println!("== own tests: {} ==", split.own_names.join(", "));
        let mut own = a.clone();
        own.affected_from.clear();
        own.microsoft_apps.clear();
        own.impact = None;
        own.impact_accept = None;
        own.impact_json = None;
        own.junit = None;
        own.output_json = None;
        let mut c = crate::run::Collected::default();
        let row = match crate::run::test_collect(&own, &mut c) {
            Ok(code) => {
                let failed = c
                    .results
                    .iter()
                    .filter(|r| matches!(r.outcome, al_testrun::Outcome::Fail(_)))
                    .count();
                let not_run = c
                    .results
                    .iter()
                    .filter(|r| matches!(r.outcome, al_testrun::Outcome::NotRun(_)))
                    .count();
                own_code = Some(code);
                if a.output_json.is_some() {
                    own_results = c.results.clone();
                }
                junit.push(crate::output::JunitPart {
                    results: std::mem::take(&mut c.results),
                    disabled: std::mem::take(&mut c.disabled),
                    bc_build: c.bc_build.take(),
                    baseline_assumptions: std::mem::take(&mut c.baseline_assumptions),
                    ..Default::default()
                });
                sources_report::PartRow::own(junit.last().map_or(0, |j| j.results.len()), failed, not_run, code)
            }
            Err(e) => {
                eprintln!("error: own tests: {e}");
                sources_report::PartRow::error("own tests", &e)
            }
        };
        rows.push(row);
    }

    let mut outs: Vec<SourceOut> = Vec::new();
    for source in &sources {
        let out = match source {
            Source::Microsoft => microsoft_source(a, &split.extension),
            Source::Isv(isv) => isv_source(a, isv, &split.extension),
        };
        rows.push(out.row.clone());
        junit.extend(out.junit.iter().cloned());
        outs.push(out);
    }
    // Entries of the accept file for a source this run does not have match nothing.
    if let Some(x) = &accepts {
        let names: Vec<&str> = outs.iter().map(|o| o.row.name.as_str()).collect();
        let mut unknown: std::collections::BTreeMap<&str, usize> = Default::default();
        for e in &x.entries {
            if let Some(s) = e.source.as_deref().filter(|s| !names.contains(s)) {
                *unknown.entry(s).or_default() += 1;
            }
        }
        for (s, n) in unknown {
            println!(
                "accept file: {n} entr{} of the source {s}, which this run does not have: they match nothing",
                if n == 1 { "y" } else { "ies" }
            );
        }
    }

    for line in sources_report::table_lines(&rows) {
        println!("{line}");
    }
    let code = sources_report::worst(rows.iter().map(|r| r.code));
    let wall = started.elapsed().as_secs_f64();
    if let Some(path) = &a.junit {
        crate::run::write_file(path, &crate::output::junit_parts(&junit, wall))?;
    }
    if let Some(path) = &a.impact_json {
        let doc = serde_json::json!({
            "format": 2,
            "exitCode": code,
            "sources": outs.iter().flat_map(SourceOut::impact_entries).collect::<Vec<_>>(),
        });
        crate::run::write_file(
            path,
            &serde_json::to_string_pretty(&doc).expect("a JSON value serializes"),
        )?;
    }
    if let Some(Some(path)) = &a.output_json {
        // The own tests in AL Runner v2's shape, as a plain run writes them, and the sources beside them (additive).
        let refs: Vec<&al_testrun::runtime::TestResult> = own_results.iter().collect();
        let mut doc: serde_json::Value =
            serde_json::from_str(&crate::output::json(&refs, own_code.unwrap_or(code), wall))
                .expect("the run's own document is JSON");
        doc["exitCode"] = code.into();
        doc["sources"] = outs.iter().map(SourceOut::output_entry).collect();
        crate::run::write_file(
            path,
            &serde_json::to_string_pretty(&doc).expect("a JSON value serializes"),
        )?;
    }
    Ok(code)
}

/// An ISV test app as a source (ADR-052 1.2, 1.4): one impact run with the run's normal test data, its dependencies
/// resolved as for any APP (the source folder's `.alpackages` among them).
fn isv_source(a: &TestArgs, isv: &Isv, extension: &[PathBuf]) -> SourceOut {
    println!("== {} {} ({}) ==", isv.name, isv.version, isv.path.display());
    let mut t = source_base(a);
    t.apps = vec![isv.path.clone()];
    t.impact_of = extension.to_vec();
    t.affected_from.clear();
    t.microsoft_apps.clear();
    t.recipe = false;
    t.bcapps_dir = None;
    t.disabled_tests_dir = None;
    t.parity = false;
    t.junit = None;
    t.output_json = None;
    t.impact_json = None;
    t.accept_source = Some(isv.name.clone());
    // The "without" side resolves the source's dependencies where the "with" side does: the extension's
    // `.alpackages` too.
    for p in search_paths(a, &isv.path) {
        if !t.deps.contains(&p) && !t.package_caches.contains(&p) && !p.starts_with(&isv.path) {
            t.package_caches.push(p);
        }
    }
    let mut head = format!("al-runner impact-index build {}", quote(&isv.path.to_string_lossy()));
    if let Some(v) = &a.bc_version {
        head.push_str(&format!(" --bc-version {v}"));
    }
    t.index_command = Some(index_command(&t, head));
    let mut out = SourceOut::new(&isv.name, "path");
    out.path = Some(isv.path.clone());
    out.test_apps = vec![(isv.name.clone(), isv.id.clone(), isv.version.clone())];
    let mut notes = Vec::new();
    match crate::impact::run_part(&t) {
        Ok(part) => out.add(part, None, None),
        Err(e) => {
            eprintln!("error: {}: {e}", isv.name);
            out.failed();
            out.row.error = Some(e.lines().next().unwrap_or_default().to_string());
            notes.push(format!("not run: {}", e.lines().next().unwrap_or_default()));
        }
    }
    println!();
    out.finish(&notes);
    out
}

/// The `microsoft` source of a run: per test app and lane, an impact run of the extension.
fn microsoft_source(a: &TestArgs, extension: &[PathBuf]) -> SourceOut {
    let mut out = SourceOut::new(MICROSOFT, MICROSOFT);
    let ms = match microsoft(a) {
        Ok(ms) => ms,
        Err(e) => {
            eprintln!("error: microsoft: {e}");
            out.row = sources_report::PartRow::error(MICROSOFT, &e);
            out.finish(&[]);
            return out;
        }
    };
    println!(
        "== microsoft: BC {} {}, {} test app(s), from {} ==",
        ms.build,
        ms.country,
        ms.set.apps.len(),
        ms.from
    );
    for (name, why) in &ms.set.left_out {
        println!("  left out: {name}: {why}");
    }
    out.test_apps = ms
        .set
        .apps
        .iter()
        .map(|t| (t.name.clone(), t.id.clone(), t.version.clone()))
        .collect();
    let base = source_base(a);
    let mut notes: Vec<String> = Vec::new();
    for app in &ms.set.apps {
        let planned = match recipe_run::plan_lanes(&app_args(&base, app, &ms.build)) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("error: test app {}: {e}", app.name);
                out.failed();
                notes.push(format!(
                    "{}: not run: {}",
                    app.name,
                    e.lines().next().unwrap_or_default()
                ));
                continue;
            }
        };
        eprintln!("{}", planned.how);
        if planned.unrun > 0 {
            notes.push(format!(
                "{}: {} codeunit(s) not verified: no lane of the recipe runs them",
                app.name, planned.unrun
            ));
        }
        for lane in planned.lanes {
            let name = lane.lane.test_type();
            if let Some(why) = &lane.refusal {
                notes.push(format!(
                    "{} lane {name}: {} codeunit(s) not verified: lane not run yet: {why}",
                    app.name,
                    lane.args.codeunits.len()
                ));
                continue;
            }
            println!(
                "\n-- {} lane {name}: {} codeunits --",
                app.name,
                lane.args.codeunits.len()
            );
            let la = source_lane_args(&lane.args, extension, microsoft_index_command(a, &app.name, &ms.build));
            match crate::impact::run_part(&la) {
                Ok(part) => out.add(part, Some(&app.name), Some(name)),
                Err(e) => {
                    eprintln!("error: {} lane {name}: {e}", app.name);
                    out.failed();
                    notes.push(format!(
                        "{} lane {name}: not run: {}",
                        app.name,
                        e.lines().next().unwrap_or_default()
                    ));
                }
            }
        }
    }
    println!();
    out.finish(&notes);
    out
}

/// `al-runner impact-index build --from microsoft [--microsoft-app NAME ...]` (ADR-052 1.8): the index of every lane
/// of every Microsoft test app, as the runs of the `microsoft` source look it up.
pub fn build_index(a: &TestArgs) -> Result<u8, String> {
    let ms = microsoft(a)?;
    let mut codes = Vec::new();
    for app in &ms.set.apps {
        let planned = recipe_run::plan_lanes(&app_args(&source_base(a), app, &ms.build))?;
        eprintln!("{}", planned.how);
        for lane in planned.lanes {
            let name = lane.lane.test_type();
            if let Some(why) = &lane.refusal {
                println!("{} lane {name}: no index: lane not run yet: {why}", app.name);
                continue;
            }
            println!("{} lane {name}:", app.name);
            let mut la = lane.args;
            la.affected_from.clear();
            la.microsoft_apps.clear();
            la.junit = None;
            la.output_json = None;
            codes.push(crate::impact_index::build_run(&la)?);
        }
    }
    Ok(sources_report::worst(codes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn by_default_every_candidate_runs_the_excluded_ones_too() {
        // Microsoft's CI dispatches the `_Exclude_` test apps (BCApps releases/29.x, run 39519586834: "Dispatching
        // '_Exclude_APIV1_ Tests'"), so the default takes them.
        let take = choose(&["Tests-A", "_Exclude_APIV2_ Tests", "Tests-B"], &[]).unwrap();
        assert_eq!(take, vec!["Tests-A", "_Exclude_APIV2_ Tests", "Tests-B"]);
    }

    #[test]
    fn app_names_match_without_case_and_an_unknown_one_lists_the_names() {
        let names = ["Tests-A", "_Exclude_X", "Tests-B"];
        let take = choose(&names, &["tests-b".into(), "_exclude_x".into(), "Tests-B".into()]).unwrap();
        assert_eq!(take, vec!["Tests-B", "_Exclude_X"]);
        let e = choose(&names, &["Tests-Z".into()]).unwrap_err();
        assert!(
            e.contains("Tests-Z") && e.contains("Tests-A, _Exclude_X, Tests-B"),
            "{e}"
        );
    }

    #[test]
    fn every_test_app_gets_its_own_output_files() {
        assert_eq!(
            app_file(Path::new("out/r.xml"), "Tests-ERM"),
            PathBuf::from("out/r.Tests-ERM.xml")
        );
        assert_eq!(
            app_file(Path::new("r.json"), "Data Search Tests"),
            PathBuf::from("r.Data_Search_Tests.json")
        );
    }
}
