//! Test sources (ADR-052): the `microsoft` source, every Microsoft test app of one BC build, and
//! `al-runner test-microsoft`, which runs them by Microsoft's recipe without the user collecting their paths.
//!
//! The listing of the artifact is `al_deps::test_apps`. Here: which candidates are test apps (they hold a test
//! codeunit; test libraries and the test framework do not), the narrowing by `--app`, and one recipe run per test app.
//! Each test app is its own run, so that one app that does not compile or load does not stop the others, and the
//! memory of one app's run is given back before the next one starts.

use crate::args::TestArgs;
use crate::sources_report::{self, AppOutcome, AppResult};
use crate::{apps, recipe_run};
use al_deps::artifact::{Fetch, HttpCdn};
use al_deps::build::BuildQuery;
use al_deps::remote::RangeSource;
use al_deps::test_apps::{self, Listing};
use std::path::{Path, PathBuf};

/// Some test apps of the artifact carry this prefix in their file name (`_Exclude_APIV2_ Tests`). They are not run by
/// default (decided without owner review, ADR-052); `--app` names them to run them.
const EXCLUDE_PREFIX: &str = "_exclude_";

/// A Microsoft test app the run will run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestApp {
    /// The name in the artifact (`Tests-ERM`).
    pub name: String,
    pub path: PathBuf,
}

/// The test apps of the `microsoft` source, and what was left out and why.
#[derive(Debug, Default)]
pub struct MicrosoftApps {
    pub apps: Vec<TestApp>,
    /// Candidates that are not run: (name, why).
    pub left_out: Vec<(String, String)>,
}

/// The names a run takes, and the candidates it leaves out with the reason.
pub type Choice = (Vec<String>, Vec<(String, String)>);

/// Which candidates the run takes before anything is read: the names `--app` gives (an unknown name is an error that
/// lists the names that exist), else every candidate but the `_Exclude_` ones.
pub fn choose(names: &[&str], wanted: &[String]) -> Result<Choice, String> {
    if wanted.is_empty() {
        let (excluded, take): (Vec<&str>, Vec<&str>) = names
            .iter()
            .partition(|n| n.to_ascii_lowercase().starts_with(EXCLUDE_PREFIX));
        let left = excluded
            .into_iter()
            .map(|n| {
                (
                    n.to_string(),
                    "the artifact names it _Exclude_; name it with --app to run it".to_string(),
                )
            })
            .collect();
        return Ok((take.into_iter().map(str::to_string).collect(), left));
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
    Ok((out, Vec::new()))
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
pub fn test_apps_of<S: RangeSource>(listing: &Listing<S>, wanted: &[String]) -> Result<MicrosoftApps, String> {
    let (names, mut left_out) = choose(&listing.names(), wanted)?;
    let build = listing.build.to_string();
    let mut out = MicrosoftApps::default();
    for name in names {
        let path = listing.get(&name)?;
        let input = apps::load(&path, Some(&build)).map_err(|e| format!("test app {name}: {e}"))?;
        if test_codeunits(&input) > 0 {
            out.apps.push(TestApp { name, path });
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
fn app_args(a: &TestArgs, app: &TestApp, build: &str, package: Option<&Path>) -> TestArgs {
    let mut t = a.clone();
    // The Legacy lane's company comes from this package (`recipe_run::lane_args` turns `company_package` on there).
    t.package_path = package.map(Path::to_path_buf);
    t.apps = vec![app.path.clone()];
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

/// The `microsoft` source of a run: the build, its test apps and the Legacy lane's package.
struct Microsoft {
    build: String,
    country: String,
    set: MicrosoftApps,
    package: Option<PathBuf>,
    /// Where the listing came from (`the artifact feed`, a local artifact).
    from: String,
}

/// Lists the Microsoft test apps of the run's build and country (ADR-052 1.3) and fetches the Legacy lane's package.
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
    let set = test_apps_of(&listing, &a.microsoft_apps)?;
    // The Legacy lane builds its company from the artifact's RapidStart package; it is fetched once here, so that a run
    // from the artifact feed does not depend on an extracted artifact on disk.
    let package = listing.rapidstart_package()?;
    Ok(Microsoft {
        build: listing.build.to_string(),
        country: listing.country.to_string(),
        set,
        package,
        from: listing.from.to_string(),
    })
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
    let mut results: Vec<AppResult> = Vec::new();
    for (i, app) in ms.set.apps.iter().enumerate() {
        eprintln!("== test app {} ({} of {}) ==", app.name, i + 1, ms.set.apps.len());
        let outcome = match recipe_run::run_lanes(&app_args(a, app, &ms.build, ms.package.as_deref())) {
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
}

pub fn split(a: &TestArgs) -> Result<Split, String> {
    let mut out = Split::default();
    for p in &a.apps {
        let input = apps::load(p, a.bc_version.as_deref())?;
        let tests = test_codeunits(&input);
        if tests > 0 {
            out.own_tests.push(p.clone());
            out.own_names
                .push(format!("{} {}", input.json.name, input.manifest.version));
        }
        if tests == 0 || objects(&input) > tests {
            out.extension.push(p.clone());
        }
    }
    Ok(out)
}

/// The command that builds the impact index of one Microsoft test app for this run (ADR-052 1.8): the options that
/// are part of the index's key.
fn index_command(a: &TestArgs, app: &str, build: &str) -> String {
    let quote = |s: &str| {
        if s.is_empty() || s.contains(|c: char| c.is_whitespace() || c == '\'') {
            format!("'{}'", s.replace('\'', "'\\''"))
        } else {
            s.to_string()
        }
    };
    let mut out = format!(
        "al-runner impact-index build --from microsoft --microsoft-app {} --bc-version {build}",
        quote(app)
    );
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

/// The arguments of one lane's impact run of a Microsoft test app: the lane's codeunits, the extension, and none of
/// the options that apply to own tests only (ADR-052 1.2).
fn source_lane_args(lane: &TestArgs, extension: &[PathBuf], index_command: String) -> TestArgs {
    let mut t = lane.clone();
    t.impact_of = extension.to_vec();
    t.affected_from.clear();
    t.junit = None;
    t.output_json = None;
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

/// `al-runner test APP... --affected-from microsoft` (ADR-052 sections 1, 2; build step 2): the own tests as a
/// plain run, then every Microsoft test app, lane by lane, as an impact run of the extension. One report, one JUnit
/// file, the exit code of the worst part. `only_sources`: `test-microsoft APP`, where an APP with tests is refused.
pub fn affected(a: &TestArgs, only_sources: bool) -> Result<u8, String> {
    let started = std::time::Instant::now();
    if let Some(path) = &a.junit {
        crate::run::write_file(path, "")?;
    }
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
    let mut rows: Vec<sources_report::PartRow> = Vec::new();
    let mut junit: Vec<crate::output::JunitPart> = Vec::new();

    if !split.own_tests.is_empty() {
        println!("== own tests: {} ==", split.own_names.join(", "));
        let mut own = a.clone();
        own.affected_from.clear();
        own.microsoft_apps.clear();
        own.impact = None;
        own.junit = None;
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

    let (row, parts) = microsoft_source(a, &split.extension);
    rows.push(row);
    junit.extend(parts);

    for line in sources_report::table_lines(&rows) {
        println!("{line}");
    }
    let code = sources_report::worst(rows.iter().map(|r| r.code));
    if let Some(path) = &a.junit {
        crate::run::write_file(
            path,
            &crate::output::junit_parts(&junit, started.elapsed().as_secs_f64()),
        )?;
    }
    Ok(code)
}

/// The `microsoft` source of a run: per test app and lane, an impact run of the extension. Returns its row of the
/// summary and its JUnit parts (the first carries `not_selected`).
fn microsoft_source(a: &TestArgs, extension: &[PathBuf]) -> (sources_report::PartRow, Vec<crate::output::JunitPart>) {
    let ms = match microsoft(a) {
        Ok(ms) => ms,
        Err(e) => {
            eprintln!("error: microsoft: {e}");
            return (sources_report::PartRow::error("microsoft", &e), Vec::new());
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
    let base = source_base(a);
    let mut row = sources_report::PartRow::source("microsoft");
    let mut parts: Vec<crate::output::JunitPart> = Vec::new();
    let mut not_selected = 0usize;
    let mut notes: Vec<String> = Vec::new();
    let mut modes: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for app in &ms.set.apps {
        let planned = match recipe_run::plan_lanes(&app_args(&base, app, &ms.build, ms.package.as_deref())) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("error: test app {}: {e}", app.name);
                row.code = sources_report::worst([row.code, 2]);
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
            let la = source_lane_args(&lane.args, extension, index_command(a, &app.name, &ms.build));
            match crate::impact::run_part(&la) {
                Ok(part) => {
                    row.add(&part);
                    if !part.mode.is_empty() {
                        *modes.entry(part.mode).or_default() += 1;
                    }
                    not_selected += part.not_selected.unwrap_or(0);
                    if let Some(mut j) = part.junit {
                        j.suite_properties = vec![("test_source".into(), "microsoft".into())];
                        parts.push(j);
                    }
                }
                Err(e) => {
                    eprintln!("error: {} lane {name}: {e}", app.name);
                    row.code = sources_report::worst([row.code, 2]);
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
    let selection: Vec<String> = modes
        .iter()
        .map(|(m, n)| format!("{m} selection in {n} run(s)"))
        .collect();
    println!(
        "microsoft: {}; not selected: {not_selected} test(s) do not reach your extension",
        if selection.is_empty() {
            "no impact run".to_string()
        } else {
            selection.join(", ")
        }
    );
    for n in &notes {
        println!("  {n}");
    }
    row.not_selected = not_selected;
    if parts.is_empty() {
        parts.push(crate::output::JunitPart {
            suite_properties: vec![("test_source".into(), "microsoft".into())],
            empty_name: "microsoft".into(),
            ..Default::default()
        });
    }
    parts[0].first_suite_properties = vec![("not_selected".to_string(), not_selected.to_string())];
    (row, parts)
}

/// `al-runner impact-index build --from microsoft [--microsoft-app NAME ...]` (ADR-052 1.8): the index of every lane
/// of every Microsoft test app, as the runs of the `microsoft` source look it up.
pub fn build_index(a: &TestArgs) -> Result<u8, String> {
    let ms = microsoft(a)?;
    let mut codes = Vec::new();
    for app in &ms.set.apps {
        let planned = recipe_run::plan_lanes(&app_args(&source_base(a), app, &ms.build, ms.package.as_deref()))?;
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
    fn by_default_every_candidate_but_the_excluded_ones() {
        let (take, left) = choose(&["Tests-A", "_Exclude_APIV2_ Tests", "Tests-B"], &[]).unwrap();
        assert_eq!(take, vec!["Tests-A", "Tests-B"]);
        assert_eq!(left.len(), 1);
        assert!(left[0].1.contains("--app"));
    }

    #[test]
    fn app_names_match_without_case_and_an_unknown_one_lists_the_names() {
        let names = ["Tests-A", "_Exclude_X", "Tests-B"];
        let (take, left) = choose(&names, &["tests-b".into(), "_exclude_x".into(), "Tests-B".into()]).unwrap();
        assert_eq!(take, vec!["Tests-B", "_Exclude_X"]);
        assert!(left.is_empty());
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
