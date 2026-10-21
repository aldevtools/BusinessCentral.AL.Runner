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

/// Which candidates the run takes before anything is read: the names `--app` gives (an unknown name is an error that
/// lists the names that exist), else every candidate but the `_Exclude_` ones.
pub fn choose(names: &[&str], wanted: &[String]) -> Result<(Vec<String>, Vec<(String, String)>), String> {
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
            left_out.push((name, "no test codeunit: a test library, loaded where a test app needs it".into()));
        }
    }
    out.left_out = left_out;
    Ok(out)
}

/// `FILE.<app>.xml` for `FILE.xml`: a file for every test app (the recipe then adds the lane).
pub fn app_file(path: &Path, app: &str) -> PathBuf {
    let safe: String = app
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') { c } else { '_' })
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

/// `al-runner test-microsoft` without an APP (ADR-052 section 2, build step 1).
pub fn test_microsoft(a: &TestArgs) -> Result<u8, String> {
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
    let build = listing.build.to_string();
    let set = test_apps_of(&listing, &a.microsoft_apps)?;
    let header = format!(
        "Microsoft tests: BC {build} {}, {} test app(s), from {}",
        listing.country,
        set.apps.len(),
        listing.from
    );
    eprintln!("{header}");
    for (name, why) in &set.left_out {
        eprintln!("  left out: {name}: {why}");
    }
    // The Legacy lane builds its company from the artifact's RapidStart package; it is fetched once here, so that a run
    // from the artifact feed does not depend on an extracted artifact on disk.
    let package = listing.rapidstart_package()?;
    let mut results: Vec<AppResult> = Vec::new();
    for (i, app) in set.apps.iter().enumerate() {
        eprintln!("== test app {} ({} of {}) ==", app.name, i + 1, set.apps.len());
        let outcome = match recipe_run::run_lanes(&app_args(a, app, &build, package.as_deref())) {
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
        assert!(e.contains("Tests-Z") && e.contains("Tests-A, _Exclude_X, Tests-B"), "{e}");
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
