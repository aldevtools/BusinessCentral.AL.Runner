//! The cell report (`--cell-report FILE`, JUnit properties): per test that uses the observer fixture, how many `Expect` cells
//! matched (`al_vm::observer_tap`). Report only: it reads the run's results and changes no outcome or list.
use crate::oracle::CaseId;
use crate::report::Outcome;
use al_testrun::runtime::TestResult;
use al_vm::observer_tap::CellRecord;
use serde::Serialize;
use std::collections::HashMap;

/// One test's cells.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TestCells {
    pub suite: String,
    pub codeunit: u32,
    pub test: String,
    /// `pass`, `fail` or `not-run`.
    pub outcome: &'static str,
    pub expected: u32,
    pub matched: u32,
    /// Cells no `Verify` looked at (the test ended or failed first).
    pub unreached: u32,
    /// Observations beyond the expected ones.
    pub extra: u32,
    /// The first of them, as `key=value`.
    pub first_extra: Option<String>,
    pub mismatched: Vec<Mismatch>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Mismatch {
    pub key: String,
    pub expected: String,
    /// `None`: the test did not observe a pair at that position.
    pub actual: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct Totals {
    pub tests: u32,
    pub expected: u64,
    pub matched: u64,
    pub not_passing_tests: u32,
    pub not_passing_expected: u64,
    pub not_passing_matched: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CellReport {
    pub totals: Totals,
    pub tests: Vec<TestCells>,
}

fn outcome_word(o: &Outcome) -> &'static str {
    match o {
        Outcome::Pass => "pass",
        Outcome::Fail(_) => "fail",
        Outcome::NotRun(_) => "not-run",
    }
}

/// The report of the results of a run; `suite_name` names a result's suite (the main run has the empty name).
pub fn build(results: &[TestResult], suite_name: &dyn Fn(usize) -> String) -> CellReport {
    let mut totals = Totals::default();
    let mut tests = Vec::new();
    for r in results {
        let Some(c) = &r.cells else { continue };
        totals.tests += 1;
        totals.expected += u64::from(c.expected);
        totals.matched += u64::from(c.matched);
        if !matches!(r.outcome, Outcome::Pass) {
            totals.not_passing_tests += 1;
            totals.not_passing_expected += u64::from(c.expected);
            totals.not_passing_matched += u64::from(c.matched);
        }
        tests.push(TestCells {
            suite: suite_name(r.suite),
            codeunit: r.codeunit,
            test: r.test.clone(),
            outcome: outcome_word(&r.outcome),
            expected: c.expected,
            matched: c.matched,
            unreached: c.unreached(),
            extra: c.extra,
            first_extra: c.first_extra.clone(),
            mismatched: c
                .mismatched
                .iter()
                .map(|m| Mismatch {
                    key: m.key.clone(),
                    expected: m.expected.clone(),
                    actual: m.actual.clone(),
                })
                .collect(),
        });
    }
    CellReport { totals, tests }
}

impl CellReport {
    /// `cells: N matched of M (in K not-passing cases: X matched)`.
    pub fn summary(&self) -> String {
        let t = &self.totals;
        format!(
            "cells: {} matched of {} (in {} not-passing cases: {} matched of {})",
            t.matched, t.expected, t.not_passing_tests, t.not_passing_matched, t.not_passing_expected
        )
    }
}

/// The file, the JSON and the summary line of the pass named `pass_name` (`main`: the main pass keeps `path`, another pass
/// adds its name to the file name).
pub fn prepare(
    path: &std::path::Path,
    main: bool,
    pass_name: &str,
    results: &[TestResult],
    suite_name: &dyn Fn(usize) -> String,
) -> (std::path::PathBuf, String, String) {
    let report = build(results, suite_name);
    let path = if main {
        path.to_path_buf()
    } else {
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        path.with_file_name(format!("{stem}-{pass_name}.json"))
    };
    let json = serde_json::to_string_pretty(&report).expect("cell report serializes");
    (path, json, report.summary())
}

/// The JUnit properties of a test: `cellsExpected`, `cellsMatched` and `firstMismatch` (the last only when a cell is wrong).
pub fn properties(c: &CellRecord) -> Vec<(String, String)> {
    let mut p = vec![
        ("cellsExpected".to_string(), c.expected.to_string()),
        ("cellsMatched".to_string(), c.matched.to_string()),
    ];
    if let Some(m) = c.first_mismatch() {
        p.push(("firstMismatch".to_string(), m));
    }
    p
}

/// The cells by (suite name, codeunit, lower-cased test), to answer a case in constant time.
pub fn index<'a>(
    results: &'a [TestResult],
    suite_name: &dyn Fn(usize) -> String,
) -> HashMap<(String, u32, String), &'a CellRecord> {
    results
        .iter()
        .filter_map(|r| {
            r.cells
                .as_ref()
                .map(|c| ((suite_name(r.suite), r.codeunit, r.test.to_lowercase()), c))
        })
        .collect()
}

/// The JUnit properties of `case` in the run named `suite`.
pub fn properties_of(
    cells: &HashMap<(String, u32, String), &CellRecord>,
    suite: &str,
    case: &CaseId,
) -> Vec<(String, String)> {
    cells
        .get(&(suite.to_string(), case.codeunit, case.name.to_lowercase()))
        .map(|c| properties(c))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use al_vm::observer_tap::CellMismatch;

    #[test]
    fn properties_carry_the_counts_and_the_first_wrong_cell() {
        let c = CellRecord {
            expected: 3,
            matched: 2,
            mismatched: vec![CellMismatch {
                key: "k".into(),
                expected: "1".into(),
                actual: Some("k=2".into()),
            }],
            extra: 0,
            first_extra: None,
            verifies: 1,
        };
        assert_eq!(
            properties(&c),
            vec![
                ("cellsExpected".to_string(), "3".to_string()),
                ("cellsMatched".to_string(), "2".to_string()),
                ("firstMismatch".to_string(), "k: expected [1] actual [k=2]".to_string()),
            ]
        );
    }

    #[test]
    fn a_clean_test_has_no_first_mismatch() {
        let c = CellRecord {
            expected: 1,
            matched: 1,
            ..CellRecord::default()
        };
        assert_eq!(properties(&c).len(), 2);
    }
}
