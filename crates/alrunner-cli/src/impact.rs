//! `al-runner test --impact-of APP` (ADR-043 steps 1 to 3: two runs or dynamic selection, the diff, the accept file,
//! the JSON diff and the JUnit of the run with the extension).
//!
//! Runs the selected tests of the APPs without the extensions, then with them (the extensions given as further
//! APPs, their statements counted per test), and compares the outcome of each test. The report names the
//! extension procedures each test entered. No impact index and no static selection: every selected test runs on
//! both sides (ADR-043 section 6, steps 2 and 5 add those).

use crate::args::TestArgs;
use crate::output;
use crate::run::{self, Collected};
use crate::style;
use al_testrun::coverage::CoverageStatement;
use al_testrun::runtime::{NarrowInput, TestResult};
use al_testrun::{CaseId, Outcome, refusals};
use std::collections::{BTreeMap, HashMap};

/// How a test's outcome changed (ADR-043 section 3), in report order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Class {
    /// Passed without, fails with.
    NewlyFailing,
    /// Failed on both sides, with another error text.
    FailsDifferently,
    /// Ran without, not run with.
    NewlyRefused,
    /// Changed, and neither it nor an earlier test of its codeunit reached an impact point in the index's run: the
    /// index missed something (a runtime bug, ADR-043 section 2.1).
    SelectionMiss,
    /// Failed without, passes with.
    NewlyPassing,
    /// Not run without, runs with.
    NewlyRuns,
    /// The same outcome, and extension code ran.
    SameReached,
    /// The same outcome, and no extension code ran. Step 1 knows only the extension code a test entered; passive
    /// impact points (a new enum value, a field property) are not seen until the impact index (step 2).
    SameNotReached,
    /// Not run on either side.
    NotVerified,
}

impl Class {
    pub const ALL: [Class; 9] = [
        Class::NewlyFailing,
        Class::FailsDifferently,
        Class::NewlyRefused,
        Class::SelectionMiss,
        Class::NewlyPassing,
        Class::NewlyRuns,
        Class::SameReached,
        Class::SameNotReached,
        Class::NotVerified,
    ];

    /// The heading of the class in the report.
    pub fn heading(self) -> &'static str {
        match self {
            Class::NewlyFailing => "newly failing",
            Class::FailsDifferently => "fails differently",
            Class::NewlyRefused => "newly refused",
            Class::SelectionMiss => "selection miss",
            Class::NewlyPassing => "newly passing",
            Class::NewlyRuns => "newly runs",
            Class::SameReached => "same, reached",
            Class::SameNotReached => "same, not reached",
            Class::NotVerified => "not verified",
        }
    }

    /// Its key in the `counts` of the JSON diff.
    pub fn key(self) -> &'static str {
        match self {
            Class::NewlyFailing => "newlyFailing",
            Class::FailsDifferently => "failsDifferently",
            Class::NewlyRefused => "newlyRefused",
            Class::SelectionMiss => "selectionMiss",
            Class::NewlyPassing => "newlyPassing",
            Class::NewlyRuns => "newlyRuns",
            Class::SameReached => "sameReached",
            Class::SameNotReached => "sameNotReached",
            Class::NotVerified => "notVerified",
        }
    }

    /// The class whose heading is `h`.
    fn of_heading(h: &str) -> Option<Class> {
        Class::ALL.into_iter().find(|c| c.heading() == h)
    }

    /// Its count in the summary line.
    fn count_label(self) -> &'static str {
        match self {
            Class::SameReached => "same (reached)",
            Class::SameNotReached => "same (not reached)",
            other => other.heading(),
        }
    }

    /// A change the extension's author must look at: exit 1 (ADR-043 section 3).
    pub fn breaks(self) -> bool {
        matches!(
            self,
            Class::NewlyFailing | Class::FailsDifferently | Class::NewlyRefused | Class::SelectionMiss
        )
    }

    /// The outcome changed.
    fn changed(self) -> bool {
        !matches!(self, Class::SameReached | Class::SameNotReached | Class::NotVerified)
    }
}

/// The class of one test, from its outcome without and with the extensions (`None`: the test is missing on that
/// side, which counts as not run) and whether it entered extension code.
pub fn classify(without: Option<&Outcome>, with: Option<&Outcome>, reached: bool) -> Class {
    use Outcome::*;
    // A failure the runner does not trust is a failure here when the test ran on the "without" side: the earlier refused
    // tests of its codeunit are the same on both sides, so what they did not write cannot be what differs (see `run_failure`).
    let unwrapped = run_failure(without, with).map(Fail);
    let with = unwrapped.as_ref().or(with);
    let same = if reached {
        Class::SameReached
    } else {
        Class::SameNotReached
    };
    match (without, with) {
        (Some(Pass), Some(Pass)) => same,
        (Some(Pass), Some(Fail(_))) => Class::NewlyFailing,
        (Some(Fail(a)), Some(Fail(b))) => {
            if a == b {
                same
            } else {
                Class::FailsDifferently
            }
        }
        (Some(Fail(_)), Some(Pass)) => Class::NewlyPassing,
        (Some(Pass | Fail(_)), Some(NotRun(_)) | None) => Class::NewlyRefused,
        (Some(NotRun(_)) | None, Some(Pass | Fail(_))) => Class::NewlyRuns,
        (Some(NotRun(_)) | None, Some(NotRun(_)) | None) => Class::NotVerified,
    }
}

/// The text of a failure the runner does not trust on the "with" side, when the same test ran (passed or failed) on the
/// "without" side. The runner reports such a failure as not run, because an earlier test of the codeunit was refused and
/// might have written what this test reads. An impact run refuses the same earlier tests on both sides, so a test that
/// passes without the extension and fails with it did not fail because of them (ADR-043, "Step 1 as built").
pub fn run_failure(without: Option<&Outcome>, with: Option<&Outcome>) -> Option<String> {
    match (without, with) {
        (Some(Outcome::Pass | Outcome::Fail(_)), Some(Outcome::NotRun(m))) => {
            m.split_once(UNTRUSTED).map(|(_, failure)| failure.to_string())
        }
        _ => None,
    }
}

/// The procedures and triggers of the extensions, from the coverage statements of the "with" run.
pub struct Routines {
    /// `Object.Procedure`, per routine.
    pub names: Vec<String>,
    /// The statement whose runs count the calls of each routine: its first statement in the source.
    first: Vec<u32>,
    /// The routine of each statement of an extension.
    of_pos: HashMap<u32, usize>,
}

impl Routines {
    /// The routines of the statements of the apps `is_extension` accepts (app index as given to the run).
    pub fn of(statements: &[CoverageStatement], is_extension: impl Fn(usize) -> bool) -> Routines {
        let mut index: BTreeMap<(usize, &str, &str, &str), usize> = BTreeMap::new();
        let mut names = Vec::new();
        let mut first: Vec<(u32, u32, u32)> = Vec::new();
        let mut of_pos = HashMap::new();
        for s in statements.iter().filter(|s| is_extension(s.app)) {
            let key = (s.app, s.file.as_str(), s.object.as_str(), s.scope.as_str());
            let r = *index.entry(key).or_insert_with(|| {
                names.push(format!("{}.{}", s.object, s.scope));
                first.push((s.line, s.column, s.pos));
                names.len() - 1
            });
            if (s.line, s.column) < (first[r].0, first[r].1) {
                first[r] = (s.line, s.column, s.pos);
            }
            of_pos.insert(s.pos, r);
        }
        Routines {
            names,
            first: first.into_iter().map(|(_, _, pos)| pos).collect(),
            of_pos,
        }
    }

    /// The extension routines a test entered, with how often: the runs of the routine's first statement (the
    /// most runs of any of its statements if the first did not run). In the order of `names`.
    pub fn entered(&self, coverage: &[(u32, u32)]) -> Vec<(usize, u32)> {
        let mut calls: BTreeMap<usize, (Option<u32>, u32)> = BTreeMap::new();
        for &(pos, runs) in coverage {
            if let Some(&r) = self.of_pos.get(&pos) {
                let e = calls.entry(r).or_insert((None, 0));
                if self.first[r] == pos {
                    e.0 = Some(runs);
                }
                e.1 = e.1.max(runs);
            }
        }
        calls
            .into_iter()
            .filter(|(_, (_, most))| *most > 0)
            .map(|(r, (first, most))| (r, first.unwrap_or(most)))
            .collect()
    }
}

/// One test of the report.
pub struct Row<'a> {
    pub class: Class,
    pub without: Option<&'a TestResult>,
    pub with: Option<&'a TestResult>,
    /// The extension routines it entered on the "with" side, with their calls.
    pub entered: Vec<(usize, u32)>,
    /// The impact points it reached in the index's run (dynamic selection).
    pub points: Vec<String>,
    /// The change is in the accept file (`--impact-accept`): listed, but it does not fail the run.
    pub accepted: bool,
}

/// The rows of both sides, in the order of the "without" run, then any test only the "with" run has.
pub fn diff<'a>(without: &'a [TestResult], with: &'a [TestResult], routines: &Routines) -> Vec<Row<'a>> {
    diff_reached(without, with, routines, None)
}

/// The impact points a test of the "without" side reached in the index's run.
type PointsOf<'f> = dyn Fn(&TestResult) -> Vec<String> + 'f;

/// [`diff`] for dynamic selection: `points_of` names the impact points each test of the "without" side reached in the
/// index's run. A test that reached one counts as reached, and a changed test that did not, after no test of its
/// codeunit that did, is a selection miss.
pub fn diff_reached<'a>(
    without: &'a [TestResult],
    with: &'a [TestResult],
    routines: &Routines,
    points_of: Option<&PointsOf<'_>>,
) -> Vec<Row<'a>> {
    let key = |r: &TestResult| (r.app, r.codeunit, r.test.clone());
    let with_by: HashMap<_, &TestResult> = with.iter().map(|r| (key(r), r)).collect();
    let mut seen = std::collections::HashSet::new();
    let mut rows = Vec::new();
    let row = |w: Option<&'a TestResult>, x: Option<&'a TestResult>| {
        let entered = x.map(|x| routines.entered(&x.coverage)).unwrap_or_default();
        let points = match (points_of, w) {
            (Some(f), Some(w)) => f(w),
            _ => Vec::new(),
        };
        Row {
            class: classify(
                w.map(|r| &r.outcome),
                x.map(|r| &r.outcome),
                !entered.is_empty() || !points.is_empty(),
            ),
            without: w,
            with: x,
            entered,
            points,
            accepted: false,
        }
    };
    for w in without {
        let k = key(w);
        let x = with_by.get(&k).copied();
        seen.insert(k);
        rows.push(row(Some(w), x));
    }
    for x in with {
        if !seen.contains(&key(x)) {
            rows.push(row(None, Some(x)));
        }
    }
    if points_of.is_some() {
        // Tests of one codeunit share its state: a change after a test that reached an impact point is no miss.
        let mut reached_codeunits = std::collections::HashSet::new();
        for r in &mut rows {
            let side = r.with.or(r.without).expect("a row has a side");
            let unit = (side.app, side.codeunit);
            if r.points.is_empty() && r.class.changed() && !reached_codeunits.contains(&unit) {
                r.class = Class::SelectionMiss;
            }
            if !r.points.is_empty() {
                reached_codeunits.insert(unit);
            }
        }
    }
    rows
}

/// What one side of a row shows in the JSON diff and the accept file: its outcome (`pass`, `fail`, `not run` or
/// `missing`), whether it is a flagged pass, and its message (the error text or the refusal reason).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Side {
    pub outcome: &'static str,
    pub flagged: bool,
    pub message: Option<String>,
}

fn side_of(r: Option<&TestResult>) -> Side {
    let Some(r) = r else {
        return Side {
            outcome: "missing",
            flagged: false,
            message: None,
        };
    };
    let (outcome, message) = match &r.outcome {
        Outcome::Pass => ("pass", None),
        Outcome::Fail(m) => ("fail", Some(m.clone())),
        Outcome::NotRun(m) => ("not run", Some(m.clone())),
    };
    Side {
        outcome,
        flagged: matches!(r.outcome, Outcome::Pass) && !r.assumptions.is_empty(),
        message,
    }
}

/// The "with" side of a row as the report counts it: a failure the runner does not trust is a failure when the test ran
/// without the extension (see [`run_failure`]).
pub fn with_side(row: &Row) -> Side {
    match run_failure(row.without.map(|r| &r.outcome), row.with.map(|r| &r.outcome)) {
        Some(failure) => Side {
            outcome: "fail",
            flagged: false,
            message: Some(failure),
        },
        None => side_of(row.with),
    }
}

/// One change of the accept file (`--impact-accept`, ADR-043 section 3): the test and its outcome with the extension.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Accept {
    pub codeunit: u32,
    pub codeunit_name: String,
    pub test: String,
    /// `pass`, `fail`, `not run` or `missing`.
    pub outcome: String,
    pub message: Option<String>,
}

/// The changes a partner accepts. The file has the shape of the JSON diff (`--impact-json`): every entry of `tests[]`
/// whose class is a change is accepted; entries of an unchanged class are left out, so the diff of an earlier run can be
/// given as it is.
#[derive(Debug, Default)]
pub struct Accepts {
    pub entries: Vec<Accept>,
}

impl Accepts {
    pub fn load(path: &std::path::Path) -> Result<Accepts, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("--impact-accept {}: {e}", path.display()))?;
        Accepts::parse(&text).map_err(|e| format!("--impact-accept {}: {e}", path.display()))
    }

    pub fn parse(text: &str) -> Result<Accepts, String> {
        let doc: serde_json::Value =
            serde_json::from_str(text).map_err(|e| format!("not a JSON document of an impact run: {e}"))?;
        let tests = doc
            .get("tests")
            .and_then(|t| t.as_array())
            .ok_or("no `tests` list (the file has the shape --impact-json writes)")?;
        let mut entries = Vec::new();
        for (i, t) in tests.iter().enumerate() {
            let at = |what: &str| format!("tests[{i}]: {what}");
            if let Some(class) = t.get("class") {
                let class = class
                    .as_str()
                    .and_then(Class::of_heading)
                    .ok_or_else(|| at(&format!("unknown class {class}")))?;
                if !class.changed() {
                    continue;
                }
            }
            let codeunit = t
                .get("codeunit")
                .and_then(|c| c.as_u64())
                .and_then(|c| u32::try_from(c).ok())
                .ok_or_else(|| at("needs `codeunit`, a codeunit id"))?;
            let test = t
                .get("test")
                .and_then(|c| c.as_str())
                .ok_or_else(|| at("needs `test`, the test's name"))?;
            let (outcome, message) = match t.get("with") {
                None | Some(serde_json::Value::Null) => ("missing".to_string(), None),
                Some(w) => {
                    let outcome = w
                        .get("outcome")
                        .and_then(|o| o.as_str())
                        .filter(|o| matches!(*o, "pass" | "fail" | "not run" | "missing"))
                        .ok_or_else(|| at("`with.outcome` must be pass, fail, not run or missing"))?;
                    let message = match w.get("message") {
                        None | Some(serde_json::Value::Null) => None,
                        Some(m) => Some(m.as_str().ok_or_else(|| at("`with.message` must be text"))?.to_string()),
                    };
                    (outcome.to_string(), message)
                }
            };
            entries.push(Accept {
                codeunit,
                codeunit_name: t
                    .get("codeunitName")
                    .and_then(|c| c.as_str())
                    .unwrap_or_default()
                    .to_string(),
                test: test.to_string(),
                outcome,
                message,
            });
        }
        Ok(Accepts { entries })
    }

    /// Marks the changed rows the file accepts: same codeunit, test (names compare without case, as AL does), outcome
    /// with the extension and message. Returns the entries no row matched.
    pub fn apply<'s>(&'s self, rows: &mut [Row]) -> Vec<&'s Accept> {
        let mut used = vec![false; self.entries.len()];
        for row in rows.iter_mut().filter(|r| r.class.changed()) {
            let any = row.with.or(row.without).expect("a row has a side");
            let side = with_side(row);
            if let Some(i) = self.entries.iter().position(|e| {
                e.codeunit == any.codeunit
                    && e.test.eq_ignore_ascii_case(&any.test)
                    && e.outcome == side.outcome
                    && e.message == side.message
            }) {
                row.accepted = true;
                used[i] = true;
            }
        }
        self.entries
            .iter()
            .zip(used)
            .filter(|(_, used)| !used)
            .map(|(e, _)| e)
            .collect()
    }
}

/// What the JSON diff says about the run besides its rows.
pub struct JsonRun<'a> {
    /// `all` (both sides ran) or `dynamic` (the "without" side came from the index).
    pub mode: &'a str,
    pub extensions: &'a [std::path::PathBuf],
    pub test_apps: &'a [std::path::PathBuf],
    pub not_selected: Option<(usize, usize)>,
    pub exit_code: u8,
}

/// The JSON diff of an impact run (`--impact-json`): the counts per class and every test with both sides. Its shape is
/// also the accept file's.
pub fn json(rows: &[Row], routines: &Routines, run: &JsonRun) -> String {
    use serde_json::{Value, json};
    let side = |s: Side, present: bool| -> Value {
        if !present {
            return Value::Null;
        }
        let mut m = serde_json::Map::new();
        m.insert("outcome".into(), json!(s.outcome));
        m.insert("flagged".into(), json!(s.flagged));
        if let Some(msg) = s.message {
            m.insert("message".into(), json!(msg));
        }
        Value::Object(m)
    };
    let mut counts = serde_json::Map::new();
    for c in Class::ALL {
        counts.insert(c.key().into(), json!(rows.iter().filter(|r| r.class == c).count()));
    }
    let paths = |p: &[std::path::PathBuf]| -> Vec<String> { p.iter().map(|p| p.display().to_string()).collect() };
    let tests: Vec<Value> = rows
        .iter()
        .map(|r| {
            let any = r.with.or(r.without).expect("a row has a side");
            json!({
                "codeunit": any.codeunit,
                "codeunitName": any.codeunit_name,
                "test": any.test,
                "class": r.class.heading(),
                "accepted": r.accepted,
                "without": side(side_of(r.without), r.without.is_some()),
                "with": side(with_side(r), r.with.is_some()),
                "reached": r.points,
                "ran": r.entered.iter().map(|&(i, calls)| json!({"routine": routines.names[i], "calls": calls})).collect::<Vec<_>>(),
            })
        })
        .collect();
    let mut doc = json!({
        "format": 1,
        "mode": run.mode,
        "extensions": paths(run.extensions),
        "testApps": paths(run.test_apps),
        "counts": counts,
        "accepted": rows.iter().filter(|r| r.accepted).count(),
        "exitCode": run.exit_code,
        "tests": tests,
    });
    if let Some((tests, codeunits)) = run.not_selected {
        doc["notSelected"] = json!({"tests": tests, "codeunits": codeunits});
    }
    serde_json::to_string_pretty(&doc).expect("a JSON value serializes")
}

/// What ends the reason of a failure the runner does not trust, before the failure's text (al-testrun `run.rs`).
const UNTRUSTED: &str = "this failure is not trusted: ";

fn outcome_word(r: Option<&TestResult>) -> String {
    let Some(r) = r else {
        return "missing".into();
    };
    let flag = if matches!(r.outcome, Outcome::Pass) && !r.assumptions.is_empty() {
        " (flagged)"
    } else {
        ""
    };
    match r.outcome {
        Outcome::Pass => format!("pass{flag}"),
        Outcome::Fail(_) => "fail".into(),
        Outcome::NotRun(_) => "not run".into(),
    }
}

fn message(r: Option<&TestResult>) -> Option<&str> {
    match r.map(|r| &r.outcome) {
        Some(Outcome::Fail(m) | Outcome::NotRun(m)) => Some(m),
        _ => None,
    }
}

/// The lines of one test in its class's list.
fn row_lines(row: &Row, routines: &Routines) -> Vec<String> {
    let any = row.with.or(row.without).expect("a row has a side");
    let (before, mut after) = (outcome_word(row.without), outcome_word(row.with));
    // A failure the runner does not trust, where the test ran without the extension, counts as a failure: show it as one.
    let counted = run_failure(row.without.map(|r| &r.outcome), row.with.map(|r| &r.outcome));
    if counted.is_some() {
        after = "fail".into();
    }
    let mut head = format!("  {} {}.{}: ", any.codeunit, any.codeunit_name, any.test);
    if before == after {
        head.push_str(&after);
    } else {
        head.push_str(&format!("{before} -> {after}"));
    }
    match &counted {
        Some(failure) => head.push_str(&format!(": {}", output::first_line(failure, 300))),
        None => {
            if let Some(m) = message(row.with) {
                head.push_str(&format!(": {}", output::first_line(m, 300)));
            }
        }
    }
    if row.accepted {
        head.push_str(" (accepted)");
    }
    let mut lines = vec![head];
    // A failure the runner does not trust and that did not run on the "without" side either stays "not run" with the
    // failure at the end of the reason: show it, it is what the extension changed.
    if counted.is_none()
        && let Some(m) = message(row.with)
        && matches!(row.with.map(|r| &r.outcome), Some(Outcome::NotRun(_)))
        && let Some((_, failure)) = m.split_once(UNTRUSTED)
    {
        lines.push(format!("    failure: {}", output::first_line(failure, 300)));
    }
    if row.class == Class::FailsDifferently
        && let Some(m) = message(row.without)
    {
        lines.push(format!("    without: {}", output::first_line(m, 300)));
    }
    if !row.points.is_empty() {
        let mut shown: Vec<&str> = row.points.iter().take(3).map(String::as_str).collect();
        let more = row.points.len().saturating_sub(3);
        let tail = format!("and {more} more");
        if more > 0 {
            shown.push(&tail);
        }
        lines.push(format!("    reached: {}", shown.join(", ")));
    }
    if !row.entered.is_empty() {
        let ran: Vec<String> = row
            .entered
            .iter()
            .map(|&(r, calls)| {
                format!(
                    "{} ({calls} call{})",
                    routines.names[r],
                    if calls == 1 { "" } else { "s" }
                )
            })
            .collect();
        lines.push(format!("    ran: {}", ran.join(", ")));
    }
    lines
}

/// The impact report: the counts per class, then the tests of each class but the unchanged unreached ones, the
/// extension procedures the tests ran, and the refusal reasons of the tests that ran on neither side.
#[cfg(test)]
pub fn report(rows: &[Row], routines: &Routines, verbose: bool) -> Vec<String> {
    report_with(rows, routines, verbose, None, &[])
}

fn plural(n: usize, word: &str) -> String {
    format!("{n} {word}{}", if n == 1 { "" } else { "s" })
}

/// [`report`], with the tests dynamic selection left out: (tests, codeunits), and the entries of the accept file that
/// matched no change.
pub fn report_with(
    rows: &[Row],
    routines: &Routines,
    verbose: bool,
    not_selected: Option<(usize, usize, &str)>,
    stale: &[&Accept],
) -> Vec<String> {
    let mut out = Vec::new();
    let count = |c: Class| rows.iter().filter(|r| r.class == c).count();
    let counts: Vec<String> = Class::ALL
        .iter()
        .map(|&c| format!("{} {}", count(c), c.count_label()))
        .collect();
    out.push(style::bold(&format!(
        "Impact: {} tests: {}",
        rows.len(),
        counts.join(", ")
    )));
    if let Some((tests, codeunits, why)) = not_selected {
        out.push(format!(
            "not selected: {} in {}: {why}",
            plural(tests, "test"),
            plural(codeunits, "codeunit")
        ));
    }
    for class in Class::ALL {
        if matches!(class, Class::SameNotReached | Class::NotVerified) || count(class) == 0 {
            continue;
        }
        out.push(String::new());
        let accepted = rows.iter().filter(|r| r.class == class && r.accepted).count();
        let head = if accepted > 0 {
            format!("{} ({}, {accepted} accepted)", class.heading(), count(class))
        } else {
            format!("{} ({})", class.heading(), count(class))
        };
        out.push(if class.breaks() {
            style::red(&head)
        } else {
            style::bold(&head)
        });
        // Unchanged tests can be many (every test that posts runs a posting subscriber): a few unless --verbose.
        let limit = if class == Class::SameReached && !verbose {
            30
        } else {
            usize::MAX
        };
        for row in rows.iter().filter(|r| r.class == class).take(limit) {
            out.extend(row_lines(row, routines));
        }
        if count(class) > limit {
            out.push(format!("  ... {} more (--verbose lists them)", count(class) - limit));
        }
    }
    // Which extension procedures the tests ran, and in how many tests.
    let mut tests_of: BTreeMap<usize, usize> = BTreeMap::new();
    for row in rows {
        for &(r, _) in &row.entered {
            *tests_of.entry(r).or_default() += 1;
        }
    }
    out.push(String::new());
    out.push(style::bold(&format!(
        "extension procedures the tests ran: {} of {}",
        tests_of.len(),
        routines.names.len()
    )));
    for (r, n) in &tests_of {
        out.push(format!(
            "  {} in {n} test{}",
            routines.names[*r],
            if *n == 1 { "" } else { "s" }
        ));
    }
    let not_verified = count(Class::NotVerified);
    if not_verified > 0 {
        let list = refusals::rank(rows.iter().filter(|r| r.class == Class::NotVerified).filter_map(|r| {
            let side = r.with.or(r.without)?;
            Some((
                CaseId {
                    codeunit: side.codeunit,
                    name: side.test.clone(),
                },
                message(Some(side)).unwrap_or("missing from the run").to_string(),
            ))
        }));
        out.push(String::new());
        out.push(format!(
            "not verified ({not_verified}): not run on either side, so the run cannot say whether the extension changes them"
        ));
        out.push(refusals::render_top("not verified tests", &list, 10));
    }
    if !stale.is_empty() {
        out.push(String::new());
        out.push(style::yellow(&format!(
            "accept file: {} no longer {} (the test changed in another way, or not at all):",
            plural(stale.len(), "accepted change"),
            if stale.len() == 1 { "matches" } else { "match" }
        )));
        for e in stale {
            let mut line = format!(
                "  {} {}.{}: accepted {}",
                e.codeunit, e.codeunit_name, e.test, e.outcome
            );
            if let Some(m) = &e.message {
                line.push_str(&format!(": {}", output::first_line(m, 300)));
            }
            out.push(line);
        }
    }
    out
}

/// The names of the APPs, for the first line of the run.
fn names(paths: &[std::path::PathBuf]) -> String {
    paths
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

/// The options both sides of an impact run share.
fn side_args(a: &TestArgs) -> TestArgs {
    let mut side = a.clone();
    side.impact_of.clear();
    side.impact = None;
    // The impact run writes these itself, for the run with the extension and the diff.
    side.impact_accept = None;
    side.impact_json = None;
    side.junit = None;
    side.output_json = None;
    side.test_apps = Some(a.apps.len());
    side.quiet = !a.verbose;
    // Microsoft's disabled tests are left out of an impact run (ADR-043 owner decision 2).
    side.parity = a.disabled_tests_dir.is_some();
    side
}

/// The command that builds the index this run lacks: the run's own arguments, as `impact-index build`, without the
/// extension (ADR-043 section 2.3: it repeats the test apps and flags exactly).
fn build_command(args: &[String]) -> String {
    let mut out = vec!["al-runner".to_string(), "impact-index".into(), "build".into()];
    let mut it = args.iter().skip_while(|a| *a != "test").skip(1);
    while let Some(a) = it.next() {
        if a == "--impact-of" || a == "--impact" {
            it.next();
            continue;
        }
        out.push(
            if a.is_empty() || a.contains(|c: char| c.is_whitespace() || c == '\'') {
                format!("'{}'", a.replace('\'', "'\\''"))
            } else {
                a.clone()
            },
        );
    }
    out.join(" ")
}

/// What static selection misses (ADR-043 section 2.3), for the warning of a run with no cached index.
const STATIC_MISSES: &str = "Codeunit, Page and Report.Run with an id from a variable or a table field, RecordRef on a table number from a variable, events raised by the platform and setup-driven dispatch";

/// The warning of a run with no cached index (ADR-043 section 2.3, owner decision 1). `cannot_rule_out`: the number of
/// selected tests that reach a site static reach cannot see through, once the selection is known.
fn no_index_warning(a: &TestArgs, cannot_rule_out: Option<usize>) -> String {
    let args: Vec<String> = std::env::args().collect();
    let marked = match cannot_rule_out {
        Some(n) => format!(
            "{n} selected test{} reach such a site (\"cannot rule out\") and were run",
            if n == 1 { "" } else { "s" }
        ),
        None => "selected tests that reach such a site are marked \"cannot rule out\" and run".to_string(),
    };
    format!(
        "warning: no impact index for these test apps and options (country {}, this runtime build): tests were selected by static reach, which misses {STATIC_MISSES}; {marked}. Build the index once with: {}",
        a.country(),
        a.index_command.clone().unwrap_or_else(|| build_command(&args))
    )
}

/// `al-runner test --impact-of ...`: `--impact all` runs everything on both sides; `--impact static`, or no cached
/// index, selects by static reach; else dynamic selection with the index. Exit 1 when a test newly fails, fails
/// differently, is newly not run or is a selection miss; a run that selected nothing or stopped exits as that run did.
pub fn run(a: &TestArgs) -> Result<u8, String> {
    run_kept(a, &Keep::default())
}

/// What one impact run came to, for a caller that puts several together (ADR-052: a test source, per test app and
/// lane).
#[derive(Default)]
pub struct Part {
    pub code: u8,
    /// `all`, `static` or `dynamic`; empty when the run stopped before its report.
    pub mode: &'static str,
    /// The tests of the report, per change class.
    pub counts: BTreeMap<Class, usize>,
    /// The tests that ran their code with the extension.
    pub reached: usize,
    /// The tests the selection left out, when it selected.
    pub not_selected: Option<usize>,
    /// The "with" run's results with their impact properties, for one JUnit file of all parts.
    pub junit: Option<output::JunitPart>,
}

impl Part {
    /// The tests of the report.
    pub fn tests(&self) -> usize {
        self.counts.values().sum()
    }

    pub fn count(&self, c: Class) -> usize {
        self.counts.get(&c).copied().unwrap_or(0)
    }
}

/// Where a run leaves its [`Part`].
type Keep = std::cell::RefCell<Option<Part>>;

/// [`run`], and what it came to. A run that stopped before its report gives its exit code only.
pub fn run_part(a: &TestArgs) -> Result<Part, String> {
    let keep = Keep::default();
    let code = run_kept(a, &keep)?;
    let mut part = keep.into_inner().unwrap_or_default();
    part.code = code;
    Ok(part)
}

fn run_kept(a: &TestArgs, keep: &Keep) -> Result<u8, String> {
    let started = std::time::Instant::now();
    // A wrong accept file or an unwritable output path stops the run before the long part.
    let accepts = a.impact_accept.as_deref().map(Accepts::load).transpose()?;
    for path in [&a.impact_json, &a.junit].into_iter().flatten() {
        run::write_file(path, "")?;
    }
    if let Some(Some(path)) = &a.output_json {
        run::write_file(path, "")?;
    }
    let accepts = accepts.as_ref();
    match a.impact.as_deref() {
        Some("all") => return run_all(a, None, accepts, keep, started),
        Some("static") => return run_static(a, false, accepts, keep, started),
        _ => {}
    }
    match crate::impact_index::lookup(&side_args(a))? {
        Some((index, path)) => run_dynamic(a, index, &path, accepts, keep, started),
        None => run_static(a, true, accepts, keep, started),
    }
}

/// Both sides run every selected test (`--impact all`, or no index).
fn run_all(
    a: &TestArgs,
    warning: Option<String>,
    accepts: Option<&Accepts>,
    keep: &Keep,
    started: std::time::Instant,
) -> Result<u8, String> {
    let side = side_args(a);
    println!(
        "impact of {} on the tests of {}: every selected test runs without, then with the extension{}",
        names(&a.impact_of),
        names(&a.apps),
        if warning.is_some() { "" } else { " (--impact all)" }
    );
    if let Some(w) = &warning {
        println!("{}", style::yellow(w));
    }

    println!("\nwithout the extension:");
    let mut without = Collected::default();
    let code = run::test_collect(&side, &mut without)?;
    if !matches!(code, 0 | 1 | 7 | 8) {
        return Ok(code);
    }

    println!("\nwith the extension:");
    let mut with_side = side.clone();
    with_side.apps.extend(a.impact_of.iter().cloned());
    with_side.coverage = true;
    let mut with = Collected::default();
    let code = run::test_collect(&with_side, &mut with)?;
    if !matches!(code, 0 | 1 | 7 | 8) {
        return Ok(code);
    }

    let tests = a.apps.len();
    let routines = Routines::of(&with.statements, |app| app >= tests);
    let mut rows = diff(&without.results, &with.results, &routines);
    Finish {
        a,
        mode: "all",
        warning: warning.as_deref(),
        not_selected: None,
        not_selected_why: "",
        after: Vec::new(),
        accepts,
        keep,
        started,
    }
    .run(&mut rows, &routines, &with)
}

/// The exit code of an impact run (ADR-043 section 3): 1 when a change for the worse is not in the accept file, else 0.
pub fn exit_code(rows: &[Row]) -> u8 {
    u8::from(rows.iter().any(|r| r.class.breaks() && !r.accepted))
}

fn verdict(rows: &[Row]) -> u8 {
    let breaking = rows.iter().filter(|r| r.class.breaks() && !r.accepted).count();
    let accepted = rows.iter().filter(|r| r.accepted).count();
    let besides = if accepted > 0 {
        format!(" ({} not counted)", plural(accepted, "accepted change"))
    } else {
        String::new()
    };
    let verdict = if breaking > 0 {
        style::red(&format!(
            "Result: {breaking} test(s) changed for the worse with the extension{besides} (exit 1)"
        ))
    } else {
        style::green(&format!(
            "Result: no test changed for the worse with the extension{besides} (exit 0)"
        ))
    };
    println!("\n{}", style::bold(&verdict));
    exit_code(rows)
}

/// How an impact run ends, whatever its mode: the report, the verdict and the files asked for.
struct Finish<'a> {
    a: &'a TestArgs,
    /// `all`, `static` or `dynamic`, for the JSON diff.
    mode: &'static str,
    warning: Option<&'a str>,
    not_selected: Option<(usize, usize)>,
    /// Why the tests not selected cannot change, for the report's `not selected:` line.
    not_selected_why: &'a str,
    /// Lines printed after the report (dynamic selection: the tests the index's run refused, with their static
    /// verdict).
    after: Vec<String>,
    accepts: Option<&'a Accepts>,
    keep: &'a Keep,
    started: std::time::Instant,
}

impl Finish<'_> {
    fn run(&self, rows: &mut [Row], routines: &Routines, with: &Collected) -> Result<u8, String> {
        let stale = self.accepts.map(|x| x.apply(rows)).unwrap_or_default();
        println!();
        if let Some(w) = self.warning {
            println!("{}\n", style::yellow(w));
        }
        for line in report_with(
            rows,
            routines,
            self.a.verbose,
            self.not_selected.map(|(t, c)| (t, c, self.not_selected_why)),
            &stale,
        ) {
            println!("{line}");
        }
        let limit = if self.a.verbose { usize::MAX } else { 32 };
        for line in self.after.iter().take(limit) {
            println!("{line}");
        }
        if self.after.len() > limit {
            println!("  ... {} more (--verbose lists them)", self.after.len() - limit);
        }
        let code = verdict(rows);
        let a = self.a;
        if let Some(path) = &a.impact_json {
            let doc = json(
                rows,
                routines,
                &JsonRun {
                    mode: self.mode,
                    extensions: &a.impact_of,
                    test_apps: &a.apps,
                    not_selected: self.not_selected,
                    exit_code: code,
                },
            );
            run::write_file(path, &doc)?;
        }
        let wall = self.started.elapsed().as_secs_f64();
        let results: Vec<&TestResult> = with.results.iter().collect();
        let key = |r: &TestResult| (r.app, r.codeunit, r.test.clone());
        let by: HashMap<_, &Row> = rows.iter().filter_map(|r| Some((key(r.with?), r))).collect();
        let properties = |r: &TestResult| -> Vec<(String, String)> {
            let Some(row) = by.get(&key(r)) else {
                return Vec::new();
            };
            let mut p = vec![
                ("impact_class".to_string(), row.class.heading().to_string()),
                ("impact_without".to_string(), outcome_word(row.without)),
            ];
            if row.accepted {
                p.push(("impact_accepted".into(), "1".into()));
            }
            p
        };
        let mut counts: BTreeMap<Class, usize> = BTreeMap::new();
        for r in rows.iter() {
            *counts.entry(r.class).or_default() += 1;
        }
        *self.keep.borrow_mut() = Some(Part {
            code,
            mode: self.mode,
            counts,
            reached: rows.iter().filter(|r| !r.entered.is_empty()).count(),
            not_selected: self.not_selected.map(|(t, _)| t),
            junit: Some(output::JunitPart {
                results: with.results.clone(),
                properties: with.results.iter().map(properties).collect(),
                disabled: with.disabled.clone(),
                bc_build: with.bc_build.clone(),
                baseline_assumptions: with.baseline_assumptions.clone(),
                ..Default::default()
            }),
        });
        if let Some(path) = &a.junit {
            let disabled: std::collections::HashSet<(u32, &str)> = with
                .results
                .iter()
                .zip(&with.disabled)
                .filter(|(_, d)| **d)
                .map(|(r, _)| (r.codeunit, r.test.as_str()))
                .collect();
            run::write_file(
                path,
                &output::junit_with(
                    &results,
                    with.bc_build.as_deref(),
                    wall,
                    &|cu, t| disabled.contains(&(cu, t)),
                    &with.baseline_assumptions,
                    &properties,
                ),
            )?;
        }
        if let Some(Some(path)) = &a.output_json {
            run::write_file(path, &output::json(&results, code, wall))?;
        }
        Ok(code)
    }
}

/// What dynamic selection decided once the "with" side was compiled.
enum Plan {
    /// Run these test codeunits (caller's app index, id).
    Select(Vec<(usize, u32)>),
    /// The index cannot select for some impact points: run every test codeunit; the reasons say why.
    Everything(Vec<String>),
    /// The index was recorded on other Microsoft apps: it cannot be used.
    Stale(String),
}

struct Decision {
    plan: Plan,
    /// The test codeunits the run would run without selection.
    would: Vec<(usize, u32)>,
    /// Per test of the index (app, codeunit, test): the impact points it reached.
    reached: std::collections::HashMap<(usize, u32, String), Vec<String>>,
}

/// Dynamic selection over `index` for the compiled "with" side (ADR-043 section 2.2).
fn decide(index: &crate::impact_index::Index, input: &NarrowInput) -> Decision {
    use al_testrun::impact_trace::{Matcher, impact_points};
    let program = input.program;
    let mut would: Vec<(usize, u32)> = match input.only {
        Some(o) => o.to_vec(),
        None => index.tests.iter().map(|t| (t.app, t.codeunit)).collect(),
    };
    would.sort_unstable();
    would.dedup();
    let mut d = Decision {
        plan: Plan::Select(Vec::new()),
        would,
        reached: Default::default(),
    };
    // The Microsoft apps must be the ones the index was recorded on.
    for (id, name, version) in &index.apps {
        match program.apps.iter().find(|p| &p.id == id) {
            Some(p) if &p.version == version => {}
            Some(p) => {
                d.plan = Plan::Stale(format!("{name} is {} here and {version} in the index", p.version));
                return d;
            }
            None => {
                d.plan = Plan::Stale(format!("{name} {version} of the index is not in this run"));
                return d;
            }
        }
    }
    // The extensions: every app of the compile the index's run did not have.
    let known: std::collections::HashSet<&str> = index.apps.iter().map(|(id, _, _)| id.as_str()).collect();
    let extension: Vec<bool> = program.apps.iter().map(|p| !known.contains(p.id.as_str())).collect();
    let points = impact_points(input.lowered, |oi| {
        extension
            .get(program.objects[oi].app as usize)
            .copied()
            .unwrap_or(false)
    });
    let mut reasons: Vec<String> = points
        .iter()
        .filter_map(|p| match &p.matcher {
            Matcher::Unsupported(why) => Some(why.clone()),
            _ => None,
        })
        .collect();
    // An impact point the install code reached changes the data every test starts from.
    if let Some(baseline) = index.baseline_keys() {
        let set: std::collections::HashSet<&str> = baseline.iter().map(String::as_str).collect();
        for p in points
            .iter()
            .filter(|p| p.runs_code && !matches!(p.matcher, Matcher::Unsupported(_)))
        {
            if p.reached_by(&baseline, |k| set.contains(k)) {
                reasons.push(format!(
                    "{}: the install code reached it in the index's run, so it changes the data every test starts from",
                    p.what
                ));
            }
        }
    }
    let mut per_point = vec![0usize; points.len()];
    let mut selected = std::collections::BTreeSet::new();
    for t in &index.tests {
        let keys = index.keys_of(t);
        let set: std::collections::HashSet<&str> = keys.iter().map(String::as_str).collect();
        let mut hit = Vec::new();
        for (i, p) in points.iter().enumerate() {
            if !matches!(p.matcher, Matcher::Unsupported(_)) && p.reached_by(&keys, |k| set.contains(k)) {
                per_point[i] += 1;
                hit.push(p.what.clone());
            }
        }
        if !hit.is_empty() {
            selected.insert((t.app, t.codeunit));
            d.reached.insert((t.app, t.codeunit, t.test.to_lowercase()), hit);
        }
    }
    println!("dynamic selection: {} impact point(s) of the extension", points.len());
    for (p, n) in points.iter().zip(&per_point) {
        match &p.matcher {
            Matcher::Unsupported(_) => println!("  {}: not covered by the index", p.what),
            _ => println!("  {}: reached by {} of {} tests", p.what, n, index.tests.len()),
        }
    }
    d.plan = if reasons.is_empty() {
        Plan::Select(d.would.iter().copied().filter(|u| selected.contains(u)).collect())
    } else {
        Plan::Everything(reasons)
    };
    d
}

/// Dynamic selection: the codeunits that reach an impact point run once, with the extension; the outcomes without it
/// come from the index.
fn run_dynamic(
    a: &TestArgs,
    index: crate::impact_index::Index,
    path: &std::path::Path,
    accepts: Option<&Accepts>,
    keep: &Keep,
    started: std::time::Instant,
) -> Result<u8, String> {
    use std::sync::{Arc, Mutex};
    println!(
        "impact of {} on the tests of {}: dynamic selection with the impact index {}",
        names(&a.impact_of),
        names(&a.apps),
        path.display()
    );
    let index = Arc::new(index);
    let decision: Arc<Mutex<Option<Decision>>> = Arc::new(Mutex::new(None));
    let refused: Arc<Mutex<Vec<String>>> = Arc::default();
    let (ix, dec, refused_out, tests) = (index.clone(), decision.clone(), refused.clone(), a.apps.len());
    let narrow = al_testrun::runtime::Narrow(Arc::new(move |input: &NarrowInput| {
        let d = decide(&ix, input);
        if !matches!(d.plan, Plan::Stale(_)) {
            *refused_out.lock().expect("the refused tests") = refused_static(&ix, input, tests);
        }
        let only = match &d.plan {
            Plan::Select(units) => {
                println!("selected {} of {} test codeunits", units.len(), d.would.len());
                Some(units.clone())
            }
            Plan::Everything(reasons) => {
                println!(
                    "{}",
                    style::yellow(&format!(
                        "warning: the index cannot select for every impact point of the extension, so all {} test codeunits run with it:",
                        d.would.len()
                    ))
                );
                for r in reasons {
                    println!("  {r}");
                }
                None
            }
            Plan::Stale(_) => Some(Vec::new()),
        };
        *dec.lock().expect("the decision") = Some(d);
        only
    }));

    println!("\nwith the extension:");
    let mut with_side = side_args(a);
    with_side.apps.extend(a.impact_of.iter().cloned());
    with_side.coverage = true;
    let mut with = Collected {
        narrow: Some(narrow),
        ..Default::default()
    };
    let code = run::test_collect(&with_side, &mut with)?;
    let Some(d) = decision.lock().expect("the decision").take() else {
        // The run stopped before its compile was done.
        return Ok(code);
    };
    if let Plan::Stale(why) = &d.plan {
        let w = format!(
            "warning: the impact index {} was recorded on other Microsoft apps ({why}); rebuild it with: {}",
            path.display(),
            a.index_command
                .clone()
                .unwrap_or_else(|| build_command(&std::env::args().collect::<Vec<_>>()))
        );
        println!("{}", style::yellow(&w));
        return run_static(a, false, accepts, keep, started);
    }
    if !matches!(code, 0 | 1 | 6 | 7 | 8) {
        return Ok(code);
    }
    let ran: std::collections::HashSet<(usize, u32)> = match &d.plan {
        Plan::Select(units) => units.iter().copied().collect(),
        _ => d.would.iter().copied().collect(),
    };
    let present: std::collections::HashSet<(usize, u32, &str)> = with
        .results
        .iter()
        .map(|r| (r.app, r.codeunit, r.test.as_str()))
        .collect();
    let without: Vec<TestResult> = index
        .tests
        .iter()
        .filter(|t| present.contains(&(t.app, t.codeunit, t.test.as_str())))
        .map(|t| index.result(t))
        .collect();
    let would: std::collections::HashSet<&(usize, u32)> = d.would.iter().collect();
    let left_out: Vec<&crate::impact_index::IndexedTest> = index
        .tests
        .iter()
        .filter(|t| would.contains(&(t.app, t.codeunit)) && !ran.contains(&(t.app, t.codeunit)))
        .collect();
    let left_out_units: std::collections::HashSet<(usize, u32)> =
        left_out.iter().map(|t| (t.app, t.codeunit)).collect();

    let tests = a.apps.len();
    let routines = Routines::of(&with.statements, |app| app >= tests);
    let points_of = |r: &TestResult| -> Vec<String> {
        d.reached
            .get(&(r.app, r.codeunit, r.test.to_lowercase()))
            .cloned()
            .unwrap_or_default()
    };
    let mut rows = diff_reached(&without, &with.results, &routines, Some(&points_of));
    Finish {
        a,
        mode: "dynamic",
        warning: None,
        not_selected: Some((left_out.len(), left_out_units.len())),
        not_selected_why: NOT_SELECTED_DYNAMIC,
        after: std::mem::take(&mut *refused.lock().expect("the refused tests")),
        accepts,
        keep,
        started,
    }
    .run(&mut rows, &routines, &with)
}

/// Why the tests dynamic selection leaves out cannot change (the report's `not selected:` line).
const NOT_SELECTED_DYNAMIC: &str =
    "they reached no impact point of the extension in the index's run, so it cannot change them";

/// Why the tests static selection leaves out cannot change.
const NOT_SELECTED_STATIC: &str = "static reach finds no path from them to an impact point of the extension";

/// The report lines for the tests the index's run refused (ADR-043 section 2.3): the index holds nothing for them,
/// so dynamic selection cannot select them; static reach says whether they reach an impact point. Empty when the
/// index's run refused none.
fn refused_static(index: &crate::impact_index::Index, input: &NarrowInput, tests: usize) -> Vec<String> {
    let refused: std::collections::HashSet<(usize, u32, String)> = index
        .tests
        .iter()
        .filter(|t| matches!(t.outcome, Outcome::NotRun(_)))
        .map(|t| (t.app, t.codeunit, t.test.to_lowercase()))
        .collect();
    if refused.is_empty() {
        return Vec::new();
    }
    // The extension: every app of the compile the index's run did not have (as `decide` has it).
    let known: std::collections::HashSet<&str> = index.apps.iter().map(|(id, _, _)| id.as_str()).collect();
    let ext: Vec<bool> = input
        .program
        .apps
        .iter()
        .map(|p| !known.contains(p.id.as_str()))
        .collect();
    let d = decide_static(
        input,
        tests,
        &ext,
        |u, n| refused.contains(&(u.0, u.1, n.to_string())),
        true,
    );
    let (mut reaches, mut cannot, mut lines) = (0, 0, Vec::new());
    for t in index.tests.iter().filter(|t| matches!(t.outcome, Outcome::NotRun(_))) {
        let Some(texts) = d.reached.get(&(t.app, t.codeunit, t.test.to_lowercase())) else {
            continue;
        };
        let name = format!("{} {}.{}", t.codeunit, t.codeunit_name, t.test);
        if texts.iter().any(|x| !x.contains(CANNOT_RULE_OUT)) {
            reaches += 1;
        } else {
            cannot += 1;
        }
        for x in texts {
            lines.push(match x.split_once(CANNOT_RULE_OUT) {
                Some((what, via)) => format!("  {name}: static: cannot rule out {what} ({via}"),
                None => format!("  {name}: static: reaches {x}"),
            });
        }
    }
    let mut out = vec![
        String::new(),
        format!(
            "refused in the index's run ({}): the index holds nothing for them; static reach: {reaches} reaches an impact point, {cannot} cannot rule one out, {} reaches none",
            refused.len(),
            refused.len() - reaches - cannot
        ),
    ];
    out.extend(lines);
    out
}

/// What static selection decided once the "with" side was compiled (ADR-043 section 2.3).
struct StaticDecision {
    /// The test codeunits to run (caller's app index, id); `None`: every one, for the reasons in `reasons`.
    select: Option<Vec<(usize, u32)>>,
    reasons: Vec<String>,
    /// The test codeunits the run would run without selection.
    would: Vec<(usize, u32)>,
    /// Per test (app, codeunit, lower-case name) that reaches an impact point or cannot rule one out: the points, as
    /// text.
    reached: HashMap<(usize, u32, String), Vec<String>>,
}

impl StaticDecision {
    /// The tests in the codeunits left out, and those codeunits.
    fn left_out(&self, by_unit: &HashMap<(usize, u32), usize>) -> (usize, usize) {
        let Some(units) = &self.select else {
            return (0, 0);
        };
        let chosen: std::collections::HashSet<&(usize, u32)> = units.iter().collect();
        let out: Vec<&(usize, u32)> = self.would.iter().filter(|u| !chosen.contains(u)).collect();
        (
            out.iter().map(|u| by_unit.get(u).copied().unwrap_or(0)).sum(),
            out.len(),
        )
    }
}

/// Marks an impact point a test cannot rule out (static reach passes a site it cannot see through).
const CANNOT_RULE_OUT: &str = " (cannot rule out: ";

/// The test routines of the test codeunits a run would run: ((caller's app, codeunit id), (object, routine), name).
pub(crate) type TestRoutine = ((usize, u32), (usize, usize), String);

pub(crate) fn test_routines(input: &NarrowInput, tests: usize) -> Vec<TestRoutine> {
    let mut out = Vec::new();
    for (oi, o) in input.program.objects.iter().enumerate() {
        let (Some(&app), Some(id)) = (input.order.get(o.app as usize), o.id) else {
            continue;
        };
        if app >= tests
            || !al_testrun::impact_static::is_test_codeunit(o)
            || input.only.is_some_and(|only| !only.contains(&(app, id)))
        {
            continue;
        }
        for (ri, r) in o.routines.iter().enumerate() {
            if r.attributes.iter().any(|a| a.name.eq_ignore_ascii_case("Test")) {
                out.push(((app, id), (oi, ri), r.name.clone()));
            }
        }
    }
    out
}

/// Per app of the program: whether it is part of the extension: an APP given with `--impact-of` (the caller's apps
/// `tests..tests + extensions`) or an app one of them depends on that the test apps do not depend on.
fn extension_apps(input: &NarrowInput, tests: usize, extensions: usize) -> Vec<bool> {
    let apps = &input.program.apps;
    let closure = |roots: Vec<usize>| {
        let mut seen = vec![false; apps.len()];
        let mut stack = roots;
        while let Some(p) = stack.pop() {
            if std::mem::replace(&mut seen[p], true) {
                continue;
            }
            for d in &apps[p].dependencies {
                stack.extend(apps.iter().position(|x| x.id.eq_ignore_ascii_case(d)));
            }
        }
        seen
    };
    let caller = |p: usize| input.order.get(p).copied().unwrap_or(usize::MAX);
    let base = closure((0..apps.len()).filter(|&p| caller(p) < tests).collect());
    let ext = closure(
        (0..apps.len())
            .filter(|&p| (tests..tests + extensions).contains(&caller(p)))
            .collect(),
    );
    ext.iter().zip(&base).map(|(e, b)| *e && !b).collect()
}

/// Static selection over the compiled "with" side: `is_ext_app` says which apps of the program are the extension;
/// `keep` which tests (app, codeunit, lower-case name) to decide for.
fn decide_static(
    input: &NarrowInput,
    tests: usize,
    is_ext_app: &[bool],
    keep: impl Fn(&(usize, u32), &str) -> bool,
    quiet: bool,
) -> StaticDecision {
    use al_testrun::impact_static::{Graph, Install, Verdict, baseline_roots, select};
    use al_testrun::impact_trace::impact_points;
    let program = input.program;
    let is_ext = |oi: usize| {
        is_ext_app
            .get(program.objects[oi].app as usize)
            .copied()
            .unwrap_or(false)
    };
    let points = impact_points(input.lowered, is_ext);
    let routines: Vec<TestRoutine> = test_routines(input, tests)
        .into_iter()
        .filter(|(u, _, n)| keep(u, &n.to_lowercase()))
        .collect();
    let mut would: Vec<(usize, u32)> = routines.iter().map(|(u, _, _)| *u).collect();
    would.sort_unstable();
    would.dedup();
    let started = std::time::Instant::now();
    let graph = Graph::build(input.lowered);
    let units: Vec<(usize, usize)> = routines.iter().map(|(_, t, _)| *t).collect();
    // The install baseline: Company-Initialize and the install codeunits of every app that is not the extension.
    let roots = baseline_roots(program, |oi| !is_ext(oi));
    let install = match input.baseline {
        Some(keys) => Install::Traced { keys, roots: &roots },
        None => Install::NotRun,
    };
    let sel = select(input.lowered, &graph, &points, &units, install);
    let mut reached: HashMap<(usize, u32, String), Vec<String>> = HashMap::new();
    let mut per_point = vec![(0usize, 0usize); points.len()];
    let mut selected = std::collections::BTreeSet::new();
    for (unit, t, name) in &routines {
        let Some(list) = sel.verdicts.get(t) else {
            continue;
        };
        let texts = list
            .iter()
            .map(|(pi, v)| match v {
                Verdict::Reaches => {
                    per_point[*pi].0 += 1;
                    points[*pi].what.clone()
                }
                Verdict::CannotRuleOut(via) => {
                    per_point[*pi].1 += 1;
                    format!("{}{CANNOT_RULE_OUT}{via})", points[*pi].what)
                }
            })
            .collect();
        selected.insert(*unit);
        reached.insert((unit.0, unit.1, name.to_lowercase()), texts);
    }
    if !quiet {
        println!(
            "static selection: {} impact point(s) of the extension (static graph of {} nodes, {:.1} s)",
            points.len(),
            graph.len(),
            started.elapsed().as_secs_f64()
        );
        for ((p, (r, c)), why) in points.iter().zip(&per_point).zip(&sel.everything) {
            match why {
                Some(_) => println!(
                    "  {}: reached by {r} of {} tests, {c} more cannot rule it out; static reach cannot select for it",
                    p.what,
                    routines.len()
                ),
                None => println!(
                    "  {}: reached by {r} of {} tests, {c} more cannot rule it out",
                    p.what,
                    routines.len()
                ),
            }
        }
    }
    let reasons: Vec<String> = sel.everything.iter().flatten().cloned().collect();
    StaticDecision {
        select: reasons
            .is_empty()
            .then(|| would.iter().copied().filter(|u| selected.contains(u)).collect()),
        reasons,
        would,
        reached,
    }
}

/// Static selection (ADR-043 section 2.3): the "with" side is compiled first and decides the test codeunits; both
/// sides run those. `no_index`: the run fell back to it because no index is cached (it warns).
fn run_static(
    a: &TestArgs,
    no_index: bool,
    accepts: Option<&Accepts>,
    keep: &Keep,
    started: std::time::Instant,
) -> Result<u8, String> {
    use std::sync::{Arc, Mutex};
    println!(
        "impact of {} on the tests of {}: static selection{}",
        names(&a.impact_of),
        names(&a.apps),
        if no_index { "" } else { " (--impact static)" }
    );
    if no_index {
        println!("{}", style::yellow(&no_index_warning(a, None)));
    }
    let (tests, extensions) = (a.apps.len(), a.impact_of.len());
    // The decision, and the number of tests per test codeunit.
    type Decided = Option<(StaticDecision, HashMap<(usize, u32), usize>)>;
    let decision: Arc<Mutex<Decided>> = Arc::new(Mutex::new(None));
    let dec = decision.clone();
    let narrow = al_testrun::runtime::Narrow(Arc::new(move |input: &NarrowInput| {
        let ext = extension_apps(input, tests, extensions);
        let d = decide_static(input, tests, &ext, |_, _| true, false);
        let mut by_unit: HashMap<(usize, u32), usize> = HashMap::new();
        for (u, _, _) in test_routines(input, tests) {
            *by_unit.entry(u).or_default() += 1;
        }
        let only = match &d.select {
            Some(units) => {
                println!(
                    "static selection: selected {} of {} test codeunits",
                    units.len(),
                    d.would.len()
                );
                Some(units.clone())
            }
            None => {
                println!(
                    "{}",
                    style::yellow(&format!(
                        "warning: static reach cannot select for every impact point of the extension, so all {} test codeunits run on both sides:",
                        d.would.len()
                    ))
                );
                for r in &d.reasons {
                    println!("  {r}");
                }
                None
            }
        };
        *dec.lock().expect("the decision") = Some((d, by_unit));
        only
    }));

    println!("\nwith the extension:");
    let mut with_side = side_args(a);
    with_side.apps.extend(a.impact_of.iter().cloned());
    with_side.coverage = true;
    let mut with = Collected {
        narrow: Some(narrow),
        ..Default::default()
    };
    let code = run::test_collect(&with_side, &mut with)?;
    let Some((d, by_unit)) = decision.lock().expect("the decision").take() else {
        // The run stopped before its compile was done.
        return Ok(code);
    };
    if !matches!(code, 0 | 1 | 6 | 7 | 8) {
        return Ok(code);
    }
    let mut without = Collected::default();
    if d.select.as_ref().is_none_or(|u| !u.is_empty()) {
        println!("\nwithout the extension:");
        let units = d.select.clone();
        without.narrow = Some(al_testrun::runtime::Narrow(Arc::new(move |_: &NarrowInput| {
            units.clone()
        })));
        let code = run::test_collect(&side_args(a), &mut without)?;
        if !matches!(code, 0 | 1 | 6 | 7 | 8) {
            return Ok(code);
        }
    }

    let routines = Routines::of(&with.statements, |app| app >= tests);
    let points_of = |r: &TestResult| -> Vec<String> {
        d.reached
            .get(&(r.app, r.codeunit, r.test.to_lowercase()))
            .cloned()
            .unwrap_or_default()
    };
    let mut rows = diff_reached(&without.results, &with.results, &routines, Some(&points_of));
    let cannot = rows
        .iter()
        .filter(|r| !r.points.is_empty() && r.points.iter().all(|p| p.contains(CANNOT_RULE_OUT)))
        .count();
    let warning = no_index.then(|| no_index_warning(a, Some(cannot)));
    Finish {
        a,
        mode: "static",
        warning: warning.as_deref(),
        not_selected: d.select.is_some().then(|| d.left_out(&by_unit)),
        not_selected_why: NOT_SELECTED_STATIC,
        after: Vec::new(),
        accepts,
        keep,
        started,
    }
    .run(&mut rows, &routines, &with)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pass() -> Outcome {
        Outcome::Pass
    }
    fn fail(m: &str) -> Outcome {
        Outcome::Fail(m.into())
    }
    fn not_run(m: &str) -> Outcome {
        Outcome::NotRun(m.into())
    }

    #[test]
    fn classes_follow_the_outcomes_on_both_sides() {
        let c = |w: Option<Outcome>, x: Option<Outcome>, reached| classify(w.as_ref(), x.as_ref(), reached);
        assert_eq!(c(Some(pass()), Some(fail("x")), true), Class::NewlyFailing);
        assert_eq!(c(Some(fail("a")), Some(fail("b")), true), Class::FailsDifferently);
        assert_eq!(c(Some(fail("a")), Some(fail("a")), true), Class::SameReached);
        assert_eq!(c(Some(fail("a")), Some(fail("a")), false), Class::SameNotReached);
        assert_eq!(c(Some(pass()), Some(not_run("r")), false), Class::NewlyRefused);
        assert_eq!(c(Some(fail("a")), None, false), Class::NewlyRefused);
        assert_eq!(c(Some(fail("a")), Some(pass()), true), Class::NewlyPassing);
        assert_eq!(c(Some(not_run("r")), Some(pass()), true), Class::NewlyRuns);
        assert_eq!(c(Some(pass()), Some(pass()), false), Class::SameNotReached);
        assert_eq!(c(Some(not_run("r")), Some(not_run("s")), false), Class::NotVerified);
        assert!(Class::NewlyFailing.breaks() && Class::FailsDifferently.breaks() && Class::NewlyRefused.breaks());
        assert!(!Class::NewlyPassing.breaks() && !Class::SameReached.breaks() && !Class::NotVerified.breaks());
    }

    fn stmt(pos: u32, app: usize, scope: &str, line: u32) -> CoverageStatement {
        CoverageStatement {
            pos,
            app,
            file: "S.al".into(),
            line,
            column: 9,
            end_line: line,
            scope: scope.into(),
            object: "Sub".into(),
        }
    }

    #[test]
    fn a_routine_counts_the_runs_of_its_first_statement_and_only_extension_apps_count() {
        let statements = vec![
            stmt(1, 0, "TestApp", 1),
            stmt(5, 1, "Handler", 11),
            stmt(4, 1, "Handler", 10),
            stmt(6, 1, "Other", 20),
            stmt(7, 1, "Never", 30),
        ];
        let r = Routines::of(&statements, |app| app >= 1);
        assert_eq!(r.names, ["Sub.Handler", "Sub.Other", "Sub.Never"]);
        // Handler: first statement (line 10, pos 4) ran twice, a later one five times (a loop).
        let entered = r.entered(&[(1, 3), (4, 2), (5, 5), (6, 1)]);
        assert_eq!(entered, [(0, 2), (1, 1)]);
        // A test that ran only the test app's statements entered no extension routine.
        assert!(r.entered(&[(1, 1)]).is_empty());
    }

    fn result(test: &str, outcome: Outcome, coverage: Vec<(u32, u32)>) -> TestResult {
        TestResult {
            suite: 0,
            app: 0,
            codeunit: 50510,
            codeunit_name: "Imp Tests".into(),
            test: test.into(),
            outcome,
            call_stack: String::new(),
            assumptions: Vec::new(),
            time: std::time::Duration::ZERO,
            coverage,
            messages: Vec::new(),
            captured: None,
            impact: Vec::new(),
            stubs: Vec::new(),
            entered: Vec::new(),
            dotnet: false,
        }
    }

    #[test]
    fn an_untrusted_failure_is_newly_failing_when_the_test_ran_without_the_extension() {
        let statements = vec![stmt(4, 1, "Handler", 10)];
        let routines = Routines::of(&statements, |app| app >= 1);
        let untrusted = not_run(&format!(
            "an earlier test of this codeunit was refused before it started (X) and can write a table this test reads; {UNTRUSTED}Assert.AreEqual failed"
        ));
        let without = vec![result("T", pass(), vec![])];
        let with = vec![result("T", untrusted.clone(), vec![(4, 2)])];
        let rows = diff(&without, &with, &routines);
        assert_eq!(rows[0].class, Class::NewlyFailing);
        let lines = report(&rows, &routines, false).join("\n");
        assert!(lines.contains("T: pass -> fail: Assert.AreEqual failed"), "{lines}");
        assert!(lines.contains("    ran: Sub.Handler (2 calls)"), "{lines}");
        // The same failure on both sides is the same result, not a new one.
        let without = vec![result("T", fail("Assert.AreEqual failed"), vec![])];
        let rows = diff(&without, &with, &routines);
        assert_eq!(rows[0].class, Class::SameReached);
    }

    #[test]
    fn an_untrusted_failure_stays_not_run_when_the_test_did_not_run_without_the_extension() {
        let routines = Routines::of(&[], |_| false);
        let untrusted = not_run(&format!("x; {UNTRUSTED}boom"));
        let without = vec![result("T", not_run("r"), vec![])];
        let with = vec![result("T", untrusted, vec![])];
        let rows = diff(&without, &with, &routines);
        assert_eq!(rows[0].class, Class::NotVerified);
    }
    #[test]
    fn the_accept_file_accepts_a_change_only_with_the_same_outcome_and_text() {
        let routines = Routines::of(&[], |_| false);
        let without = vec![
            result("T", pass(), vec![]),
            result("U", pass(), vec![]),
            result("S", pass(), vec![]),
        ];
        let with = vec![
            result("T", fail("amount 20"), vec![]),
            result("U", fail("other"), vec![]),
            result("S", pass(), vec![]),
        ];
        let mut rows = diff(&without, &with, &routines);
        let accepts = Accepts::parse(
            r#"{"tests": [
                {"codeunit": 50510, "test": "t", "class": "newly failing", "with": {"outcome": "fail", "message": "amount 20"}},
                {"codeunit": 50510, "test": "U", "class": "newly failing", "with": {"outcome": "fail", "message": "amount 20"}},
                {"codeunit": 50510, "test": "S", "class": "same, not reached", "with": {"outcome": "pass"}},
                {"codeunit": 50510, "test": "Gone", "with": null}
            ]}"#,
        )
        .unwrap();
        // Entries of an unchanged class are not changes to accept.
        assert_eq!(accepts.entries.len(), 3);
        let stale = accepts.apply(&mut rows);
        assert!(rows[0].accepted, "names compare without case");
        assert!(!rows[1].accepted, "another error text is a new difference");
        assert!(!rows[2].accepted);
        assert_eq!(stale.iter().map(|e| e.test.as_str()).collect::<Vec<_>>(), ["U", "Gone"]);
        assert_eq!(exit_code(&rows), 1);
        rows[1].accepted = true;
        assert_eq!(exit_code(&rows), 0);
        // What cannot be read is an error, not an empty list.
        assert!(Accepts::parse("[]").is_err());
        assert!(Accepts::parse(r#"{"tests": [{"test": "T", "with": {"outcome": "fail"}}]}"#).is_err());
        assert!(Accepts::parse(r#"{"tests": [{"codeunit": 1, "test": "T", "with": {"outcome": "broken"}}]}"#).is_err());
        assert!(Accepts::parse(r#"{"tests": [{"codeunit": 1, "test": "T", "class": "renamed"}]}"#).is_err());
    }

    #[test]
    fn the_json_diff_reads_back_as_an_accept_file_that_accepts_every_change() {
        let statements = vec![stmt(4, 1, "Handler", 10)];
        let routines = Routines::of(&statements, |app| app >= 1);
        let without = vec![result("T", pass(), vec![]), result("S", pass(), vec![])];
        let with = vec![result("T", fail("boom"), vec![(4, 1)]), result("S", pass(), vec![])];
        let mut rows = diff(&without, &with, &routines);
        let doc = json(
            &rows,
            &routines,
            &JsonRun {
                mode: "all",
                extensions: &[],
                test_apps: &[],
                not_selected: None,
                exit_code: 1,
            },
        );
        let accepts = Accepts::parse(&doc).unwrap();
        assert_eq!(accepts.entries.len(), 1, "{doc}");
        assert!(accepts.apply(&mut rows).is_empty());
        assert!(rows[0].accepted && !rows[1].accepted);
        assert_eq!(exit_code(&rows), 0);
    }
}
