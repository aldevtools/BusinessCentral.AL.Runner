//! Joins the oracle with the runtime's outcomes into the pass-rate report,
//! and enforces the ratchet: a case that passed before must still pass.

use crate::oracle::{CaseId, OracleCase};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub use al_testrun::Outcome;
use al_testrun::assumptions::Assumption;

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct Counts {
    pub total: usize,
    pub pass: usize,
    pub fail: usize,
    pub not_run: usize,
    /// Of `pass`, the passes that ran through an unprobed assumption (`al_builtins::assumptions`): they
    /// count as passes, and are flagged because a probe may still contradict the assumed behavior.
    pub flagged: usize,
}

impl Counts {
    pub fn rate(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.pass as f64 / self.total as f64
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub overall: Counts,
    pub areas: BTreeMap<String, Counts>,
    /// Cases real BC itself did not pass; excluded from every count.
    pub excluded_bc_failed: usize,
    /// Oracle cases whose codeunit is not in the corpus: the oracle and the
    /// corpus pin disagree, which makes the whole report untrustworthy.
    pub unmapped: Vec<String>,
    /// The dependency apps the run compiled against, as `name version` (ADR-013). A different
    /// build of one of them can change results, so the report says which it was.
    pub dependencies: Vec<String>,
    /// The Microsoft build the dependency apps came from, when the artifact feed was used (ADR-013
    /// Layer A). `None` when every dependency was found in `.alpackages` or `--deps`.
    pub bc_build: Option<String>,
    /// The other test apps (`[[version.suite]]` in `oracle.toml`), each against its own oracle.
    /// Their cases are not part of `overall`.
    pub suites: Vec<SuiteReport>,
    /// How long the run took, by phase. `None` only where no run happened (unit tests).
    pub timings: Option<Timings>,
    /// The flagged passes of the main app, one entry per test, with the ids of its assumptions.
    pub flagged_passes: Vec<FlaggedPass>,
    /// The assumptions the tests of every test app ran through, ranked by flagged passes
    /// (`--top-assumptions`); empty when none was hit.
    pub assumptions: Vec<AssumptionCount>,
    /// What each pass of the run did, counted (`work.rs`, the work ratchet), by pass name. Empty where no run happened.
    pub work: BTreeMap<String, crate::work::Counts>,
}

/// A passing test that ran through unprobed assumptions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FlaggedPass {
    pub codeunit: u32,
    pub test: String,
    pub assumptions: Vec<&'static str>,
}

/// How many tests ran through one assumption.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AssumptionCount {
    pub id: &'static str,
    pub flagged_passes: usize,
    pub other_tests: usize,
}

impl AssumptionCount {
    pub fn of(rows: &[al_testrun::flagged::Row]) -> Vec<AssumptionCount> {
        rows.iter()
            .map(|r| AssumptionCount {
                id: r.assumption.info().id,
                flagged_passes: r.flagged,
                other_tests: r.other,
            })
            .collect()
    }
}

/// Wall time of one version's run, by phase, in seconds. The report always carries it, so a reader
/// of a CI summary sees what the runtime itself costs, apart from the CI step around it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Timings {
    pub bc_version: String,
    /// The run compiled every dependency routine (`AL_EAGER_DEPENDENCIES`), not only what the
    /// tests reach. That is the slower comparison run, not the normal one.
    pub eager: bool,
    /// Reading the apps, and finding or downloading the dependency apps.
    pub load_s: f64,
    /// Parsing, checking and lowering to the IR.
    pub compile_s: f64,
    /// Running the install code of the Microsoft apps once, to fill the starting database.
    pub baseline_s: f64,
    /// Running the tests of all test apps.
    pub tests_s: f64,
    /// From the start of the version's run to the report; includes the oracle and file reading.
    pub total_s: f64,
    /// The tests phase split by test app. The main corpus app is named `corpus`.
    pub suites: Vec<SuiteTime>,
    /// The .NET sidecar processes (ADR-014), when any started. Their start-up is part of the
    /// tests phase but is not the runtime's own cost, so it is reported apart.
    #[serde(default)]
    pub dotnet: Option<DotNetTimes>,
    /// The ten tests that took longest, slowest first (wall time of the test alone).
    #[serde(default)]
    pub slowest_tests: Vec<SlowTest>,
    /// The compile cache of the dependency apps (ADR-022), when the run used it.
    #[serde(default)]
    pub compile_cache: Option<CacheTimes>,
}

/// What the compile cache did for a run, in seconds.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CacheTimes {
    /// The entry was there and used as it was.
    pub hit: bool,
    /// This run built the entry. Its time is in `build_s`, and in `compile_s` too.
    pub built: bool,
    /// Making the key and reading and opening the file.
    pub open_s: f64,
    /// Building and writing the entry; zero on a hit.
    pub build_s: f64,
    pub mb: f64,
    /// Dependency objects of the entry, and how many of them the run compiled again because the
    /// workspace extends or implements what they embed.
    pub objects: usize,
    pub relowered: usize,
}

impl CacheTimes {
    pub fn of(r: &al_testrun::compile_cache::CacheReport) -> CacheTimes {
        CacheTimes {
            hit: r.hit,
            built: r.built,
            open_s: r.open.as_secs_f64(),
            build_s: r.build.as_secs_f64(),
            mb: r.bytes as f64 / 1e6,
            objects: r.objects,
            relowered: r.relowered,
        }
    }

    pub fn line(&self) -> String {
        let how = if self.hit {
            format!("hit, opened in {:.2} s", self.open_s)
        } else if self.built {
            format!("miss, built in {:.2} s", self.build_s)
        } else {
            "not used".to_string()
        };
        format!(
            "{how}; {:.1} MB, {} dependency objects, {} compiled again because the workspace extends or implements what they embed",
            self.mb, self.objects, self.relowered
        )
    }
}

/// What the .NET sidecar processes of a run cost (`SidecarStats`), in seconds.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DotNetTimes {
    pub processes: usize,
    /// Spawning the processes up to their first answer, summed over all of them (they overlap
    /// when several test codeunits start one at the same time).
    pub start_s: f64,
    /// The longest single start.
    pub start_max_s: f64,
    /// Finding `dotnet` and the sidecar (`dotnet --list-runtimes`), once per run.
    #[serde(default)]
    pub find_s: f64,
    /// How long sessions waited for their process. With standby processes (started while the
    /// compile ran) this is far less than `start_s`; it is the part that adds to the tests phase.
    #[serde(default)]
    pub waited_s: f64,
    pub calls: usize,
    /// Time inside `DotNet` requests after the start, summed over all sessions.
    pub calls_s: f64,
}

impl DotNetTimes {
    pub fn of(s: &crate::dotnet::SidecarStats) -> Option<DotNetTimes> {
        (s.started > 0).then_some(DotNetTimes {
            processes: s.started,
            start_s: s.start_total.as_secs_f64(),
            start_max_s: s.start_max.as_secs_f64(),
            find_s: s.find_total.as_secs_f64(),
            waited_s: s.waited_total.as_secs_f64(),
            calls: s.calls,
            calls_s: s.call_total.as_secs_f64(),
        })
    }

    /// One line, for `--timings` and the markdown report.
    pub fn line(&self) -> String {
        format!(
            "{} .NET sidecar process(es) started: starting {:.2} s in total, {:.2} s at most; finding dotnet {:.2} s; sessions waited {:.2} s for theirs; {} request(s) {:.2} s",
            self.processes, self.start_s, self.start_max_s, self.find_s, self.waited_s, self.calls, self.calls_s
        )
    }
}

/// A test and how long it took.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SlowTest {
    pub codeunit: u32,
    pub name: String,
    pub seconds: f64,
}

/// The time the tests of one test app took.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SuiteTime {
    pub name: String,
    pub tests_s: f64,
}

impl Timings {
    /// The `timings` of a report written with `--json`.
    pub fn from_report_json(json: &str) -> Result<Timings, String> {
        let v: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
        match v.get("timings") {
            Some(t) if !t.is_null() => serde_json::from_value(t.clone()).map_err(|e| format!("timings: {e}")),
            _ => Err("the report has no `timings`".into()),
        }
    }
}

/// One extra test app's pass rate.
#[derive(Debug, Serialize)]
pub struct SuiteReport {
    pub name: String,
    /// The BC build its oracle was recorded on.
    pub oracle_leg: String,
    pub counts: Counts,
    pub areas: BTreeMap<String, Counts>,
    /// Cases real BC failed. A control suite has some on purpose (the isolation probe under
    /// `TestIsolation = Codeunit` fails its `T5`); they are excluded from `counts`.
    pub bc_failed: usize,
    /// Of those, how many the runtime fails too. Equal to `bc_failed` when it agrees with BC.
    pub bc_failed_reproduced: usize,
    pub flagged_passes: Vec<FlaggedPass>,
}

impl SuiteReport {
    pub fn new(
        name: &str,
        oracle_leg: &str,
        oracle: &[OracleCase],
        areas: &dyn Fn(u32) -> Option<String>,
        outcome: &dyn Fn(&CaseId) -> Outcome,
        assumptions: &dyn Fn(&CaseId) -> Vec<Assumption>,
    ) -> (SuiteReport, Vec<String>) {
        let r = build_with(oracle, areas, outcome, assumptions);
        let bc_failed_reproduced = oracle
            .iter()
            .filter(|c| c.bc_failed && matches!(outcome(&c.id), Outcome::Fail(_)))
            .count();
        (
            SuiteReport {
                name: name.into(),
                oracle_leg: oracle_leg.into(),
                counts: r.overall,
                areas: r.areas,
                bc_failed: r.excluded_bc_failed,
                bc_failed_reproduced,
                flagged_passes: r.flagged_passes,
            },
            r.unmapped,
        )
    }
}

pub fn build(
    oracle: &[OracleCase],
    area_of: &dyn Fn(u32) -> Option<String>,
    outcome: &dyn Fn(&CaseId) -> Outcome,
) -> Report {
    build_with(oracle, area_of, outcome, &|_| Vec::new())
}

/// `build`, with the unprobed assumptions each test ran through: a pass with any counts as flagged.
pub fn build_with(
    oracle: &[OracleCase],
    area_of: &dyn Fn(u32) -> Option<String>,
    outcome: &dyn Fn(&CaseId) -> Outcome,
    assumptions: &dyn Fn(&CaseId) -> Vec<Assumption>,
) -> Report {
    let mut r = Report {
        flagged_passes: Vec::new(),
        assumptions: Vec::new(),
        overall: Counts::default(),
        areas: BTreeMap::new(),
        excluded_bc_failed: 0,
        unmapped: Vec::new(),
        dependencies: Vec::new(),
        bc_build: None,
        suites: Vec::new(),
        timings: None,
        work: BTreeMap::new(),
    };
    for case in oracle {
        if case.bc_failed {
            r.excluded_bc_failed += 1;
            continue;
        }
        let Some(area) = area_of(case.id.codeunit) else {
            r.unmapped.push(format!("{} {}", case.id.codeunit, case.id.name));
            continue;
        };
        let o = outcome(&case.id);
        let marks = if o == Outcome::Pass {
            assumptions(&case.id)
        } else {
            Vec::new()
        };
        if !marks.is_empty() {
            r.flagged_passes.push(FlaggedPass {
                codeunit: case.id.codeunit,
                test: case.id.name.clone(),
                assumptions: marks.iter().map(|a| a.info().id).collect(),
            });
        }
        let mut targets = vec![&mut r.overall];
        let area_counts = r.areas.entry(area.clone()).or_default();
        targets.push(area_counts);
        for c in targets {
            c.total += 1;
            match o {
                Outcome::Pass => {
                    c.pass += 1;
                    c.flagged += usize::from(!marks.is_empty());
                }
                Outcome::Fail(_) => c.fail += 1,
                Outcome::NotRun(_) => c.not_run += 1,
            }
        }
    }
    r
}

/// Cases listed as passing in the baseline that do not pass now.
pub fn regressions(baseline: &BTreeSet<CaseId>, outcome: &dyn Fn(&CaseId) -> Outcome) -> Vec<CaseId> {
    baseline
        .iter()
        .filter(|c| outcome(c) != Outcome::Pass)
        .cloned()
        .collect()
}

/// The marker the VM puts on a refusal that comes from the machine having no usable .NET
/// (`al_vm::dotnet::UNAVAILABLE`, ADR-014 section 11).
pub const DOTNET_UNAVAILABLE: &str = al_vm::dotnet::UNAVAILABLE;

/// The ratchet over a baseline: regressions, and cases skipped because .NET is unavailable.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Ratchet {
    pub regressions: Vec<CaseId>,
    /// Baseline cases that did not run only because the machine has no usable .NET, with the
    /// reason. They are listed, not counted as regressions, and no line leaves the baseline.
    pub skipped: Vec<(CaseId, String)>,
}

pub fn ratchet(baseline: &BTreeSet<CaseId>, outcome: &dyn Fn(&CaseId) -> Outcome) -> Ratchet {
    let mut r = Ratchet::default();
    for c in baseline {
        match outcome(c) {
            Outcome::Pass => {}
            Outcome::NotRun(why) if why.contains(DOTNET_UNAVAILABLE) => r.skipped.push((c.clone(), why)),
            _ => r.regressions.push(c.clone()),
        }
    }
    r
}

/// What the CLI does about a ratchet result.
#[derive(Debug, PartialEq, Eq)]
pub enum Decision {
    Ok,
    /// Exit 1: a baseline case no longer passes.
    Regression,
    /// Exit 2: the run cannot be trusted or the update cannot be made; the text says why.
    Setup(String),
}

pub fn decide(r: &Ratchet, update_baseline: bool, require_dotnet: bool) -> Decision {
    if !r.regressions.is_empty() {
        return Decision::Regression;
    }
    if !r.skipped.is_empty() {
        if require_dotnet {
            return Decision::Setup(format!(
                "--require-dotnet: {} baseline case(s) could not run because .NET is unavailable",
                r.skipped.len()
            ));
        }
        if update_baseline {
            return Decision::Setup(format!(
                "--update-baseline refused: {} baseline case(s) were skipped because .NET is unavailable, so the new baseline would be incomplete",
                r.skipped.len()
            ));
        }
    }
    Decision::Ok
}

/// Baseline file format: one `<codeunit> <procedure>` per line, `#` comments.
pub fn parse_baseline(text: &str) -> Result<BTreeSet<CaseId>, String> {
    let mut set = BTreeSet::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (cu, name) = line
            .split_once(' ')
            .ok_or_else(|| format!("line {}: expected `<codeunit> <procedure>`", n + 1))?;
        let codeunit = cu
            .parse()
            .map_err(|_| format!("line {}: `{cu}` is not a codeunit number", n + 1))?;
        set.insert(CaseId {
            codeunit,
            name: name.trim().to_string(),
        });
    }
    Ok(set)
}

pub fn format_baseline(cases: &BTreeSet<CaseId>) -> String {
    let mut s = String::from(
        "# Corpus cases the runtime passes. CI fails if any of these stops passing.\n# Regenerate with `al-test-cli --update-baseline`; never edit by hand to hide a regression.\n",
    );
    for c in cases {
        s.push_str(&format!("{} {}\n", c.codeunit, c.name));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn case(cu: u32, n: &str, bc_failed: bool) -> OracleCase {
        OracleCase {
            id: id(cu, n),
            bc_failed,
        }
    }
    fn id(cu: u32, n: &str) -> CaseId {
        CaseId {
            codeunit: cu,
            name: n.into(),
        }
    }
    fn areas(cu: u32) -> Option<String> {
        match cu {
            1 => Some("text".into()),
            2 => Some("record".into()),
            3 => Some("handlers".into()),
            _ => None,
        }
    }

    fn oracle() -> Vec<OracleCase> {
        vec![
            case(1, "a", false),
            case(1, "b", false),
            case(2, "c", false),
            case(3, "d", false),
            case(3, "e", true),
        ]
    }

    fn outcome(c: &CaseId) -> Outcome {
        match c.name.as_str() {
            "a" | "c" => Outcome::Pass,
            "b" => Outcome::Fail("x".into()),
            _ => Outcome::NotRun("no runtime".into()),
        }
    }

    #[test]
    fn counts_per_area_and_overall() {
        let r = build(&oracle(), &areas, &outcome);
        assert_eq!(
            r.overall,
            Counts {
                total: 4,
                pass: 2,
                fail: 1,
                not_run: 1,
                flagged: 0,
            }
        );
        assert_eq!(
            r.areas["text"],
            Counts {
                total: 2,
                pass: 1,
                fail: 1,
                not_run: 0,
                flagged: 0,
            }
        );
        assert_eq!(
            r.areas["record"],
            Counts {
                total: 1,
                pass: 1,
                fail: 0,
                not_run: 0,
                flagged: 0,
            }
        );
        assert_eq!(
            r.areas["handlers"],
            Counts {
                total: 1,
                pass: 0,
                fail: 0,
                not_run: 1,
                flagged: 0,
            }
        );
    }

    #[test]
    fn a_pass_that_ran_through_an_assumption_is_flagged_and_still_a_pass() {
        let marks = |c: &CaseId| match c.name.as_str() {
            "a" => vec![
                Assumption::TextDigitsToInteger,
                Assumption::CodeCoverageSnapshotWhileLogging,
            ],
            // A failing test that hit one is not a flagged pass.
            "b" => vec![Assumption::TextDigitsToInteger],
            _ => Vec::new(),
        };
        let r = build_with(&oracle(), &areas, &outcome, &marks);
        assert_eq!((r.overall.pass, r.overall.flagged), (2, 1));
        assert_eq!(r.areas["text"].flagged, 1);
        assert_eq!(r.areas["record"].flagged, 0);
        assert_eq!(
            r.flagged_passes,
            vec![FlaggedPass {
                codeunit: 1,
                test: "a".into(),
                assumptions: vec!["text-digits-to-integer", "code-coverage-snapshot-while-logging"]
            }]
        );
        assert_eq!(build(&oracle(), &areas, &outcome).overall.flagged, 0);
    }

    #[test]
    fn cases_bc_failed_are_excluded_not_counted() {
        let r = build(&oracle(), &areas, &outcome);
        assert_eq!(r.excluded_bc_failed, 1);
    }

    #[test]
    fn oracle_cases_without_a_corpus_codeunit_are_reported_unmapped() {
        let mut o = oracle();
        o.push(case(99, "z", false));
        let r = build(&o, &areas, &outcome);
        assert_eq!(r.unmapped, vec!["99 z".to_string()]);
    }

    #[test]
    fn rate_is_pass_over_total() {
        assert_eq!(
            Counts {
                total: 4,
                pass: 1,
                fail: 0,
                not_run: 3,
                flagged: 0,
            }
            .rate(),
            0.25
        );
        assert_eq!(Counts::default().rate(), 0.0);
    }

    #[test]
    fn a_baseline_case_that_no_longer_passes_is_a_regression() {
        let base: BTreeSet<_> = [id(1, "a"), id(1, "b"), id(3, "d")].into();
        assert_eq!(regressions(&base, &outcome), vec![id(1, "b"), id(3, "d")]);
    }

    fn dotnet_outcome(c: &CaseId) -> Outcome {
        match (c.codeunit, c.name.as_str()) {
            (1, "a") => Outcome::Pass,
            (2, "needs_dotnet") => Outcome::NotRun(
                "dotnet-unavailable: DotNet 'System.Convert' needs the .NET 8 runtime; 'dotnet' not found".into(),
            ),
            (2, "dotnet_crashed") => Outcome::NotRun("DotNet System.Convert.X: the sidecar process ended".into()),
            _ => Outcome::Fail("x".into()),
        }
    }

    #[test]
    fn a_baseline_case_refused_because_dotnet_is_unavailable_is_skipped_not_a_regression() {
        let base: BTreeSet<_> = [id(1, "a"), id(2, "needs_dotnet"), id(2, "dotnet_crashed"), id(3, "d")].into();
        let r = ratchet(&base, &dotnet_outcome);
        assert_eq!(
            r.regressions,
            vec![id(2, "dotnet_crashed"), id(3, "d")],
            "a crashed sidecar is a regression"
        );
        assert_eq!(r.skipped.len(), 1);
        assert_eq!(r.skipped[0].0, id(2, "needs_dotnet"));
        assert!(r.skipped[0].1.contains("'dotnet' not found"));
    }

    #[test]
    fn exit_codes_follow_the_ratchet_the_update_flag_and_require_dotnet() {
        let base: BTreeSet<_> = [id(1, "a"), id(2, "needs_dotnet")].into();
        let r = ratchet(&base, &dotnet_outcome);
        // Local run without .NET: reported, not a failure.
        assert_eq!(decide(&r, false, false), Decision::Ok);
        // The baseline cannot be rewritten while cases were skipped.
        let Decision::Setup(why) = decide(&r, true, false) else {
            panic!()
        };
        assert!(why.contains("--update-baseline") && why.contains("skipped"), "{why}");
        // CI: unavailability is a hard error, so the ratchet means what it says.
        let Decision::Setup(why) = decide(&r, false, true) else {
            panic!()
        };
        assert!(why.contains("--require-dotnet"), "{why}");
        // A real regression still wins over a skip.
        let base: BTreeSet<_> = [id(3, "d"), id(2, "needs_dotnet")].into();
        assert_eq!(
            decide(&ratchet(&base, &dotnet_outcome), false, false),
            Decision::Regression
        );
        // Nothing skipped, nothing regressed.
        let base: BTreeSet<_> = [id(1, "a")].into();
        assert_eq!(decide(&ratchet(&base, &dotnet_outcome), true, true), Decision::Ok);
    }

    #[test]
    fn baseline_round_trips_and_skips_comments_and_blanks() {
        let text = "# passing cases\n\n60002 A_Works\n60001 Z\n";
        let set = parse_baseline(text).unwrap();
        assert_eq!(set, [id(60001, "Z"), id(60002, "A_Works")].into());
        assert_eq!(parse_baseline(&format_baseline(&set)).unwrap(), set);
    }

    #[test]
    fn baseline_rejects_a_line_without_a_codeunit_number() {
        assert!(parse_baseline("abc A\n").is_err());
        assert!(parse_baseline("60002\n").is_err());
    }
}
