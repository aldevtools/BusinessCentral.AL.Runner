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
        out.push(
            "  codeunits left: of a lane the runner cannot build yet, or of a TestType no lane runs (the lines per app say which); their tests are not run"
                .to_string(),
        );
    }
    out
}

/// One part of a run with test sources (ADR-052 1.6): the own tests or one source, as a row of the summary table.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PartRow {
    pub name: String,
    /// The own tests: failed and not run count, the change classes do not.
    pub own: bool,
    pub tests: usize,
    pub failed: usize,
    pub not_run: usize,
    pub reached: usize,
    pub newly_failing: usize,
    pub fails_differently: usize,
    pub newly_refused: usize,
    pub selection_miss: usize,
    pub not_verified: usize,
    pub not_selected: usize,
    pub code: u8,
    /// Why the part could not run (its first line).
    pub error: Option<String>,
}

impl PartRow {
    pub fn own(tests: usize, failed: usize, not_run: usize, code: u8) -> PartRow {
        PartRow {
            name: "own tests".into(),
            own: true,
            tests,
            failed,
            not_run,
            code,
            ..Default::default()
        }
    }

    pub fn source(name: &str) -> PartRow {
        PartRow {
            name: name.into(),
            ..Default::default()
        }
    }

    /// A part that could not run: exit 2.
    pub fn error(name: &str, e: &str) -> PartRow {
        PartRow {
            name: name.into(),
            own: name == "own tests",
            code: 2,
            error: Some(e.lines().next().unwrap_or_default().to_string()),
            ..Default::default()
        }
    }

    /// Adds one impact run of the source. ADR-052 1.7: a run that selected nothing gives 0, not 6, and tests that
    /// are not verified never give 7 or 8.
    pub fn add(&mut self, part: &crate::impact::Part) {
        use crate::impact::Class;
        self.tests += part.tests();
        self.reached += part.reached;
        self.newly_failing += part.count(Class::NewlyFailing);
        self.fails_differently += part.count(Class::FailsDifferently);
        self.newly_refused += part.count(Class::NewlyRefused);
        self.selection_miss += part.count(Class::SelectionMiss);
        self.not_verified += part.count(Class::NotVerified);
        let code = if matches!(part.code, 6..=8) { 0 } else { part.code };
        self.code = worst([self.code, code]);
    }

    /// What makes the part fail, for the `Result:` line.
    fn why(&self) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(e) = &self.error {
            out.push(format!("not run: {e}"));
        }
        let counts: [(usize, &str); 5] = if self.own {
            [
                (self.failed, "failed"),
                (if self.code == 7 { self.not_run } else { 0 }, "not run"),
                (0, ""),
                (0, ""),
                (0, ""),
            ]
        } else {
            [
                (self.newly_failing, "newly failing"),
                (self.fails_differently, "fails differently"),
                (self.newly_refused, "newly refused"),
                (self.selection_miss, "selection miss"),
                (0, ""),
            ]
        };
        out.extend(counts.iter().filter(|(n, _)| *n > 0).map(|(n, w)| format!("{n} {w}")));
        out
    }
}

/// The summary table of a run with test sources and its `Result:` line (ADR-052 1.6, exit code by [`worst`]).
pub fn table_lines(rows: &[PartRow]) -> Vec<String> {
    let dash = |own: bool, n: usize| if own { "-".to_string() } else { n.to_string() };
    let mut table = vec![[
        "part".to_string(),
        "tests run".into(),
        "ran your code".into(),
        "newly failing".into(),
        "fails differently".into(),
        "newly refused".into(),
        "not verified".into(),
        "exit".into(),
    ]];
    for r in rows {
        table.push([
            r.name.clone(),
            r.tests.to_string(),
            dash(r.own, r.reached),
            dash(r.own, r.newly_failing + r.selection_miss),
            dash(r.own, r.fails_differently),
            dash(r.own, r.newly_refused),
            if r.own {
                format!("{} not run", r.not_run)
            } else {
                r.not_verified.to_string()
            },
            r.code.to_string(),
        ]);
    }
    let widths: Vec<usize> = (0..8)
        .map(|c| table.iter().map(|row| row[c].len()).max().unwrap_or(0))
        .collect();
    let mut out = vec![String::new(), "Sources".to_string()];
    for row in &table {
        let cells: Vec<String> = row
            .iter()
            .enumerate()
            .map(|(c, v)| format!("{v:<w$}", w = widths[c]))
            .collect();
        out.push(format!("  {}", cells.join("  ").trim_end()));
    }
    let code = worst(rows.iter().map(|r| r.code));
    let verdict = match code {
        0 => "PASS",
        1 => "FAIL",
        6 => "NOTHING SELECTED",
        7 => "TESTS NOT RUN",
        8 => "DOTNET UNAVAILABLE",
        _ => "ERROR",
    };
    let why: Vec<String> = rows
        .iter()
        .filter_map(|r| {
            let w = r.why();
            (!w.is_empty()).then(|| format!("{}: {}", r.name, w.join(", ")))
        })
        .collect();
    out.push(if why.is_empty() {
        format!("Result: {verdict} (exit {code})")
    } else {
        format!("Result: {verdict} ({}) (exit {code})", why.join("; "))
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sources_table_has_a_row_per_part_and_the_result_names_what_failed() {
        let own = PartRow::own(212, 2, 10, 1);
        let mut ms = PartRow::source("microsoft");
        ms.tests = 1840;
        ms.reached = 412;
        ms.newly_failing = 1;
        ms.newly_refused = 3;
        ms.not_verified = 220;
        ms.code = 1;
        let lines = table_lines(&[own, ms]);
        assert_eq!(lines[1], "Sources");
        assert!(lines[2].starts_with("  part       tests run  ran your code  newly failing"), "{lines:?}");
        assert_eq!(
            lines[3].split_whitespace().collect::<Vec<_>>(),
            ["own", "tests", "212", "-", "-", "-", "-", "10", "not", "run", "1"]
        );
        assert_eq!(
            lines[4].split_whitespace().collect::<Vec<_>>(),
            ["microsoft", "1840", "412", "1", "0", "3", "220", "1"]
        );
        assert_eq!(
            lines[5],
            "Result: FAIL (own tests: 2 failed; microsoft: 1 newly failing, 3 newly refused) (exit 1)"
        );
        // A source that could not load is exit 2 and says why; nothing failing is a pass.
        let lines = table_lines(&[PartRow::error("microsoft", "no artifact\nmore")]);
        assert_eq!(lines.last().unwrap(), "Result: ERROR (microsoft: not run: no artifact) (exit 2)");
        assert_eq!(table_lines(&[PartRow::source("microsoft")]).last().unwrap(), "Result: PASS (exit 0)");
    }
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
