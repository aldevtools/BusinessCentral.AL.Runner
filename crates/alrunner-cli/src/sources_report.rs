//! The report of a run over several parts (ADR-052 sections 1.6 and 1.7): the exit code shared by every kind of part,
//! and the summary of `al-runner test-microsoft` with one row per Microsoft test app and a total.

use crate::recipe_run::{LaneOutcome, LaneResult};

/// The exit code of a run of several parts, from the exit code of each part (ADR-052 1.7): 2 (or 3, the apps do not
/// compile) if any part could not start; else 1 if any part failed; else 8; else 7; else 6 only when every part
/// selected nothing; else 0. No part at all is 6: nothing was selected.
pub fn worst(codes: impl IntoIterator<Item = u8>) -> u8 {
    let codes: Vec<u8> = codes.into_iter().collect();
    let has = |c: u8| codes.contains(&c);
    if has(2) {
        2
    } else if has(3) {
        3
    } else if has(1) {
        1
    } else if has(8) {
        8
    } else if has(7) {
        7
    } else if codes.iter().all(|&c| c == 6) {
        6
    } else {
        // Every code is 0 or 6 here; any other code a part could give is kept rather than read as a pass.
        codes.iter().copied().filter(|&c| c != 6).max().unwrap_or(0)
    }
}

/// What one Microsoft test app came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppOutcome {
    /// Its lanes ran (or were refused with their reason).
    Ran { lanes: Vec<LaneResult>, unrun: usize },
    /// It could not be fetched, loaded or compiled: the message.
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppResult {
    pub name: String,
    pub outcome: AppOutcome,
}

impl AppResult {
    pub fn exit_code(&self) -> u8 {
        match &self.outcome {
            AppOutcome::Ran { lanes, .. } => crate::recipe_run::exit_code(lanes),
            AppOutcome::Error(_) => 2,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct Counts {
    passed: u64,
    failed: u64,
    not_run: u64,
    total: u64,
    /// Codeunits of lanes the runner cannot build yet, and of a `TestType` no lane runs.
    left_out: usize,
}

fn counts(r: &AppResult) -> Counts {
    let mut c = Counts::default();
    if let AppOutcome::Ran { lanes, unrun } = &r.outcome {
        c.left_out += unrun;
        for l in lanes {
            match &l.outcome {
                LaneOutcome::Ran {
                    passed,
                    failed,
                    not_run,
                    total,
                    ..
                } => {
                    c.passed += passed;
                    c.failed += failed;
                    c.not_run += not_run;
                    c.total += total;
                }
                LaneOutcome::Refused(_) => c.left_out += l.codeunits,
            }
        }
    }
    c
}

/// The lines at the end of `al-runner test-microsoft`: one row per test app, then the total and the result.
pub fn summary_lines(header: &str, results: &[AppResult]) -> Vec<String> {
    let width = results.iter().map(|r| r.name.len()).max().unwrap_or(0).max(9);
    let mut out = vec![
        header.to_string(),
        format!(
            "  {:<width$}  {:>7}  {:>6}  {:>7}  {:>7}  {:>14}  exit",
            "test app", "passed", "failed", "not run", "total", "codeunits left"
        ),
    ];
    let mut sum = Counts::default();
    let mut errors = 0;
    for r in results {
        let c = counts(r);
        sum.passed += c.passed;
        sum.failed += c.failed;
        sum.not_run += c.not_run;
        sum.total += c.total;
        sum.left_out += c.left_out;
        match &r.outcome {
            AppOutcome::Error(e) => {
                errors += 1;
                out.push(format!("  {:<width$}  NOT RUN: {e}  2", r.name));
            }
            AppOutcome::Ran { .. } => out.push(format!(
                "  {:<width$}  {:>7}  {:>6}  {:>7}  {:>7}  {:>14}  {}",
                r.name,
                c.passed,
                c.failed,
                c.not_run,
                c.total,
                c.left_out,
                r.exit_code()
            )),
        }
    }
    let code = worst(results.iter().map(AppResult::exit_code));
    out.push(format!(
        "  {:<width$}  {:>7}  {:>6}  {:>7}  {:>7}  {:>14}  {code}",
        "Total", sum.passed, sum.failed, sum.not_run, sum.total, sum.left_out
    ));
    if errors > 0 {
        out.push(format!(
            "  {errors} test app(s) could not be run; their tests are not counted above"
        ));
    }
    if sum.left_out > 0 {
        out.push(format!(
            "  codeunits left: of a lane the runner cannot build yet, or of a TestType no lane runs (the lines per app say which); their tests are not run"
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recipe::Lane;

    #[test]
    fn the_worst_part_decides() {
        assert_eq!(worst([]), 6);
        assert_eq!(worst([6, 6]), 6);
        assert_eq!(worst([6, 0]), 0, "a part that selected nothing does not hide a pass");
        assert_eq!(worst([0, 7, 1]), 1);
        assert_eq!(worst([1, 2]), 2);
        assert_eq!(worst([1, 3]), 3);
        assert_eq!(worst([7, 8]), 8);
        assert_eq!(worst([0, 7]), 7);
    }

    fn lane(passed: u64, failed: u64, code: u8) -> LaneResult {
        LaneResult {
            lane: Lane::UnitTest,
            company: String::new(),
            codeunits: 2,
            outcome: LaneOutcome::Ran {
                passed,
                failed,
                not_run: 1,
                total: passed + failed + 1,
                code,
            },
        }
    }

    #[test]
    fn one_row_per_test_app_and_a_total() {
        let results = vec![
            AppResult {
                name: "Tests-A".into(),
                outcome: AppOutcome::Ran {
                    lanes: vec![
                        lane(3, 0, 7),
                        LaneResult {
                            lane: Lane::Integration,
                            company: "My Company".into(),
                            codeunits: 4,
                            outcome: LaneOutcome::Refused("Contoso".into()),
                        },
                    ],
                    unrun: 1,
                },
            },
            AppResult {
                name: "Tests-B".into(),
                outcome: AppOutcome::Ran {
                    lanes: vec![lane(5, 2, 1)],
                    unrun: 0,
                },
            },
            AppResult {
                name: "Tests-C".into(),
                outcome: AppOutcome::Error("does not compile".into()),
            },
        ];
        let lines = summary_lines("Microsoft tests", &results);
        let row = |name: &str| lines.iter().find(|l| l.trim_start().starts_with(name)).unwrap().clone();
        let nums = |l: String| l.split_whitespace().skip(1).collect::<Vec<_>>().join(" ");
        assert_eq!(nums(row("Tests-A")), "3 0 1 4 5 7");
        assert_eq!(nums(row("Tests-B")), "5 2 1 8 0 1");
        assert!(row("Tests-C").contains("NOT RUN: does not compile"));
        assert_eq!(nums(row("Total")), "8 2 2 12 5 2");
        assert!(lines.iter().any(|l| l.contains("1 test app(s) could not be run")));
    }
}
