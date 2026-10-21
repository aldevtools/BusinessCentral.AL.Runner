//! `al-runner test --recipe microsoft`: one run per lane, with the company and the disabled tests Microsoft's CI uses.
//!
//! The planning (which codeunit gets which company) is `recipe.rs`; the files come from `bcapps.rs`. This file turns a
//! plan into runs of the ordinary test command and adds the lines that put the lanes together.

use crate::args::TestArgs;
use crate::recipe::{self, AppUnits, Lane, Recipe};
use crate::run::{self, WarmCtx};
use crate::{apps, bcapps, sources_report};
use std::path::{Path, PathBuf};

/// What a lane's run came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaneResult {
    pub lane: Lane,
    pub company: String,
    pub codeunits: usize,
    pub outcome: LaneOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaneOutcome {
    /// The lane ran; the counts of its report and its exit code.
    Ran {
        passed: u64,
        failed: u64,
        not_run: u64,
        total: u64,
        code: u8,
    },
    /// The runner cannot build this lane's company yet (said in one line); its tests are not run.
    Refused(String),
}

/// The BC release (`major`, `minor`) the recipe files are taken from: `--bc-version` when it names a minor or a build,
/// else the build of the Microsoft test apps.
fn release(a: &TestArgs, inputs: &[apps::AppInput]) -> Result<(u32, u32), String> {
    if let Some(v) = a.bc_version.as_deref() {
        let mut parts = v.trim().split('.').map(|p| p.parse::<u32>());
        if let (Some(Ok(major)), Some(Ok(minor))) = (parts.next(), parts.next()) {
            return Ok((major, minor));
        }
    }
    if let Some(b) = apps::own_build(inputs) {
        return Ok((b.major(), b.minor()));
    }
    Err("--recipe microsoft needs the BC release (major.minor) to pick the BCApps branch: the tests are not Microsoft's build; name it with --bc-version MAJOR.MINOR".into())
}

/// The arguments of one lane's run: a copy of the command line with the lane's codeunits, its company, Microsoft's
/// disabled tests skipped, and its own output files.
pub fn lane_args(base: &TestArgs, lane: Lane, company: &str, codeunits: &[u32], recipe: &Recipe) -> TestArgs {
    let mut a = base.clone();
    a.recipe = false;
    a.bcapps_dir = None;
    a.codeunits = if base.codeunits.is_empty() {
        codeunits.to_vec()
    } else {
        codeunits
            .iter()
            .copied()
            .filter(|c| base.codeunits.contains(c))
            .collect()
    };
    a.disabled_tests_dir = Some(recipe.disabled_tests_dir());
    a.parity = true;
    match lane {
        Lane::Legacy => {
            a.company_package = true;
            a.test_data_company = Some(company.to_string());
        }
        // An empty company: Company-Initialize and the installs only, no demo data.
        Lane::UnitTest => {
            a.no_test_data = true;
            a.company_name = Some(company.to_string());
        }
        Lane::Integration | Lane::Uncategorized => a.company_name = Some(company.to_string()),
    }
    a.junit = base.junit.as_deref().map(|p| lane_file(p, lane));
    a.output_json = match &base.output_json {
        Some(Some(p)) => Some(Some(lane_file(p, lane))),
        other => other.clone(),
    };
    a
}

/// `FILE.<lane>.xml` for `FILE.xml`: a file for every lane, since the runs are separate.
pub fn lane_file(path: &Path, lane: Lane) -> PathBuf {
    let stem = path
        .file_stem()
        .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
    let ext = path.extension().map(|e| e.to_string_lossy().into_owned());
    let name = match ext {
        Some(e) => format!("{stem}.{}.{e}", lane.test_type().to_lowercase()),
        None => format!("{stem}.{}", lane.test_type().to_lowercase()),
    };
    path.with_file_name(name)
}

/// Why the runner cannot build a lane's company, if it cannot.
pub fn company_refusal(lane: Lane) -> Option<String> {
    match lane {
        Lane::Legacy | Lane::UnitTest => None,
        Lane::Integration | Lane::Uncategorized => Some(format!(
            "the {} company is built by Microsoft's Contoso Demo Tool ({}); the runner cannot run it yet (the Contoso steps stop at later refusals; see docs/alrunner-buckets.md), so these tests are not run rather than run against a different company",
            lane.test_type(),
            if lane == Lane::Integration {
                "setup data"
            } else {
                "all demo data"
            }
        )),
    }
}

/// The lines under the lane runs.
pub fn summary_lines(results: &[LaneResult], unrun: usize) -> Vec<String> {
    let mut out = vec!["Microsoft recipe, by lane:".to_string()];
    for r in results {
        let head = format!(
            "  {:<14} {:>5} codeunits, company '{}'",
            r.lane.test_type(),
            r.codeunits,
            r.company
        );
        match &r.outcome {
            LaneOutcome::Ran {
                passed,
                failed,
                not_run,
                total,
                ..
            } => out.push(format!(
                "{head}: {passed} passed, {failed} failed, {not_run} not run of {total}"
            )),
            LaneOutcome::Refused(why) => out.push(format!("{head}: NOT RUN, {why}")),
        }
    }
    if unrun > 0 {
        out.push(format!(
            "  {unrun} codeunits have a TestType that no lane runs (AITest); they are left out"
        ));
    }
    out
}

/// The exit code of the recipe run, by the shared rule of ADR-052 (1.7, [`sources_report::worst`]): a lane that
/// failed gives 1; a lane that was not run, or whose tests were not run, gives 7; 6 only when no lane selected a test.
pub fn exit_code(results: &[LaneResult]) -> u8 {
    sources_report::worst(results.iter().map(|r| match &r.outcome {
        LaneOutcome::Refused(_) => 7,
        LaneOutcome::Ran { code, .. } => *code,
    }))
}

/// What a recipe run came to: the lanes, and the codeunits of a `TestType` no lane runs.
pub struct RecipeRun {
    pub results: Vec<LaneResult>,
    pub unrun: usize,
}

/// Runs the recipe.
pub fn test(a: &TestArgs) -> Result<u8, String> {
    let run = run_lanes(a)?;
    for line in summary_lines(&run.results, run.unrun) {
        println!("{line}");
    }
    Ok(exit_code(&run.results))
}

/// One lane of the recipe as planned: its company, the arguments of its run, and why it cannot run, if it cannot.
pub struct PlannedLane {
    pub lane: Lane,
    pub company: String,
    pub args: TestArgs,
    pub refusal: Option<String>,
}

/// The lanes of the recipe for the APPs of `a`, and how many codeunits no lane runs.
pub struct Planned {
    pub lanes: Vec<PlannedLane>,
    pub unrun: usize,
    /// Where the BCApps files came from (one line).
    pub how: String,
}

/// Plans the recipe's lanes for the APPs of `a`: lanes without a codeunit are left out.
pub fn plan_lanes(a: &TestArgs) -> Result<Planned, String> {
    let inputs = a
        .apps
        .iter()
        .map(|p| apps::load(p, a.bc_version.as_deref()))
        .collect::<Result<Vec<_>, String>>()?;
    let (major, minor) = release(a, &inputs)?;
    let cache_root = al_deps::cache::cache_root_from_env(a.cache_dir.as_deref());
    let url = std::env::var("ALRUNNER_BCAPPS_URL").unwrap_or_else(|_| bcapps::DEFAULT_URL.to_string());
    let source = bcapps::resolve(
        a.bcapps_dir.as_deref(),
        major,
        minor,
        &cache_root,
        a.no_download,
        &url,
        &bcapps::git_fetch,
        &bcapps::has_recipe,
    )?;
    let recipe = Recipe::load(&source.dir, a.country())?;
    let mut units: Vec<AppUnits> = Vec::new();
    for i in &inputs {
        let mut codeunits = Vec::new();
        for (_, text) in &i.source.files {
            codeunits.extend(al_testrun::discover::test_codeunit_types(text));
        }
        units.push(AppUnits {
            app_name: i.json.name.clone(),
            codeunits,
        });
    }
    let plan = recipe::plan(&recipe, &units);
    let mut lanes = Vec::new();
    for (lane, ids) in &plan.lanes {
        let company = recipe.company_name(*lane).unwrap_or("").to_string();
        let args = lane_args(a, *lane, &company, ids, &recipe);
        if args.codeunits.is_empty() {
            continue;
        }
        lanes.push(PlannedLane {
            lane: *lane,
            refusal: company_refusal(*lane),
            company,
            args,
        });
    }
    Ok(Planned {
        lanes,
        unrun: plan.unrun.len(),
        how: source.how,
    })
}

/// Runs every lane of the recipe and returns what each came to; prints the lanes' own reports.
pub fn run_lanes(a: &TestArgs) -> Result<RecipeRun, String> {
    let planned = plan_lanes(a)?;
    eprintln!("{}", planned.how);
    let mut results: Vec<LaneResult> = Vec::new();
    for PlannedLane {
        lane,
        company,
        args: la,
        refusal,
    } in planned.lanes
    {
        let n = la.codeunits.len();
        if let Some(why) = refusal {
            eprintln!(
                "== lane {}: {n} codeunits, company '{company}': {why}",
                lane.test_type()
            );
            results.push(LaneResult {
                lane,
                company,
                codeunits: n,
                outcome: LaneOutcome::Refused(why),
            });
            continue;
        }
        eprintln!("== lane {}: {n} codeunits, company '{company}'", lane.test_type());
        let mut warm = WarmCtx::default();
        let code = run::test_with(&la, Some(&mut warm))?;
        let count = |k: &str| {
            warm.report
                .as_ref()
                .and_then(|r| r.doc.get(k))
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
        };
        results.push(LaneResult {
            lane,
            company,
            codeunits: n,
            outcome: LaneOutcome::Ran {
                passed: count("passed"),
                failed: count("failed"),
                not_run: count("notRun"),
                total: count("total"),
                code,
            },
        });
    }
    Ok(RecipeRun {
        results,
        unrun: planned.unrun,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recipe() -> Recipe {
        let mut r = Recipe::parse(
            r#"{"testType":"UnitTest","companyName":"Empty Company","buildModes":["L"],
                "ConditionalSettings":[{"buildModes":["L"],"settings":{"testType":"Legacy","bucketNumber":1,"companyName":"CRONUS International Ltd."}}]}"#,
            r#"{"LegacyTests-Bucket1":["Tests-ERM"]}"#,
        )
        .unwrap();
        r.root = PathBuf::from("/bc");
        r
    }

    fn base() -> TestArgs {
        TestArgs {
            recipe: true,
            bcapps_dir: Some("/bc".into()),
            junit: Some("out/r.xml".into()),
            ..Default::default()
        }
    }

    #[test]
    fn the_legacy_lane_gets_the_package_company_and_skips_the_disabled_tests() {
        let a = lane_args(&base(), Lane::Legacy, "CRONUS International Ltd.", &[5, 6], &recipe());
        assert!(a.company_package && !a.no_test_data && !a.recipe);
        assert_eq!(a.test_data_company.as_deref(), Some("CRONUS International Ltd."));
        assert_eq!(a.codeunits, vec![5, 6]);
        assert_eq!(a.disabled_tests_dir, Some(PathBuf::from("/bc/src/DisabledTests")));
        assert!(a.parity);
        assert_eq!(a.junit, Some(PathBuf::from("out/r.legacy.xml")));
    }

    #[test]
    fn the_unit_test_lane_runs_without_demo_data() {
        let a = lane_args(&base(), Lane::UnitTest, "Empty Company", &[7], &recipe());
        assert!(a.no_test_data && !a.company_package);
        assert_eq!(a.company_name.as_deref(), Some("Empty Company"));
        assert_eq!(a.junit, Some(PathBuf::from("out/r.unittest.xml")));
    }

    #[test]
    fn a_codeunit_selection_on_the_command_line_narrows_the_lane() {
        let mut b = base();
        b.codeunits = vec![6, 99];
        let a = lane_args(&b, Lane::Legacy, "C", &[5, 6], &recipe());
        assert_eq!(a.codeunits, vec![6]);
    }

    #[test]
    fn the_contoso_lanes_are_refused_with_a_reason_not_run_on_another_company() {
        assert!(company_refusal(Lane::Legacy).is_none());
        assert!(company_refusal(Lane::UnitTest).is_none());
        assert!(company_refusal(Lane::Integration).unwrap().contains("Contoso"));
        assert!(company_refusal(Lane::Uncategorized).unwrap().contains("all demo data"));
    }

    #[test]
    fn the_exit_code_is_the_worst_lane() {
        let ran = |code| LaneResult {
            lane: Lane::Legacy,
            company: String::new(),
            codeunits: 1,
            outcome: LaneOutcome::Ran {
                passed: 0,
                failed: 0,
                not_run: 0,
                total: 0,
                code,
            },
        };
        let refused = LaneResult {
            outcome: LaneOutcome::Refused("x".into()),
            ..ran(0)
        };
        assert_eq!(exit_code(&[]), 6);
        assert_eq!(exit_code(&[ran(0), ran(0)]), 0);
        assert_eq!(exit_code(&[ran(0), refused.clone()]), 7);
        assert_eq!(exit_code(&[ran(1), refused.clone()]), 1);
        assert_eq!(exit_code(&[refused, ran(1)]), 1);
        // ADR-052 1.7: a lane that selected nothing does not hide another lane's pass (the old rule gave 6).
        assert_eq!(exit_code(&[ran(6), ran(0)]), 0);
        assert_eq!(exit_code(&[ran(6), ran(6)]), 6);
    }

    #[test]
    fn the_summary_names_each_lane_and_what_was_left_out() {
        let r = vec![
            LaneResult {
                lane: Lane::Legacy,
                company: "CRONUS International Ltd.".into(),
                codeunits: 3,
                outcome: LaneOutcome::Ran {
                    passed: 10,
                    failed: 1,
                    not_run: 2,
                    total: 13,
                    code: 1,
                },
            },
            LaneResult {
                lane: Lane::Integration,
                company: "My Company".into(),
                codeunits: 4,
                outcome: LaneOutcome::Refused("no Contoso".into()),
            },
        ];
        let lines = summary_lines(&r, 2);
        assert!(lines[1].contains("Legacy") && lines[1].contains("10 passed, 1 failed, 2 not run of 13"));
        assert!(lines[2].contains("IntegrationTest") && lines[2].contains("NOT RUN, no Contoso"));
        assert!(lines[3].contains("2 codeunits") && lines[3].contains("AITest"));
    }
}
