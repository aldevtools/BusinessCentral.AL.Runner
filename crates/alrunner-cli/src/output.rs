//! What a run prints and writes: the summary, the lines per test, JUnit XML.

use al_testrun::Outcome;
use al_testrun::runtime::TestResult;

/// Counts of a set of results.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    pub total: usize,
    pub pass: usize,
    /// Of the passes: the tests that ran through an unprobed assumption (`al_builtins::assumptions`).
    pub flagged: usize,
    pub fail: usize,
    pub not_run: usize,
    /// Of the not run: the tests a missing or unusable .NET helper stopped (the reason starts with `dotnet-unavailable`).
    pub dotnet_unavailable: usize,
    /// `--tdd` (ADR-051): of the passes, failures and tests not run, those that ran a stub.
    pub stubbed_pass: usize,
    pub stubbed_fail: usize,
    pub stubbed_not_run: usize,
}

impl Counts {
    pub fn of<'a>(results: impl IntoIterator<Item = &'a TestResult>) -> Counts {
        let mut c = Counts::default();
        for r in results {
            c.total += 1;
            let stubbed = usize::from(!r.stubs.is_empty());
            match r.outcome {
                Outcome::Pass => {
                    c.pass += 1;
                    c.flagged += usize::from(!r.assumptions.is_empty());
                    c.stubbed_pass += stubbed;
                }
                Outcome::Fail(_) => {
                    c.fail += 1;
                    c.stubbed_fail += stubbed;
                }
                Outcome::NotRun(ref why) => {
                    c.not_run += 1;
                    c.stubbed_not_run += stubbed;
                    if why.starts_with(al_vm::dotnet::UNAVAILABLE) {
                        c.dotnet_unavailable += 1;
                    }
                }
            }
        }
        c
    }
}

/// The exit code of a finished run (ADR-016 rule 2): 6 when nothing was selected, 1 when a test
/// failed, 8 when a test was not run because the .NET helper is unavailable (a problem of the
/// machine, not of the tests; unless `allow_not_run`), 7 when a test was not run (unless
/// `allow_not_run`), else 0. Codes 2 and 3 are decided
/// before this point.
pub fn exit_code(c: &Counts, allow_not_run: bool) -> u8 {
    if c.total == 0 {
        6
    } else if c.fail > 0 {
        1
    } else if c.dotnet_unavailable > 0 && !allow_not_run {
        8
    } else if c.not_run > 0 && !allow_not_run {
        7
    } else {
        0
    }
}

/// Whether a dependency-resolution log line shows without `--verbose`. A symbol package without
/// `NavxManifest.xml` (Microsoft's, in `.alpackages`) is expected and harmless: `--verbose` only.
pub fn dependency_line_is_problem(line: &str) -> bool {
    line.starts_with("dependency skipped")
        || line.starts_with("warning:")
        || (line.starts_with("dependency note") && !line.ends_with("the package has no NavxManifest.xml"))
}

/// The one-line warning for an install baseline in which some install roots did not run to their end
/// (`ran` of `total`): tests may see less setup data than real BC. `None` when all ran or none was tried.
pub fn baseline_warning(ran: usize, total: usize) -> Option<String> {
    (ran < total).then(|| {
        format!("baseline incomplete: {ran} of {total} install roots ran to their end (--verbose for details)")
    })
}

/// The warning for a run whose tests were stopped by a missing or unusable .NET helper, so that it
/// is never mistaken for a lower pass count. `why` is the reason of one such test.
pub fn dotnet_warning(c: &Counts, why: &str) -> Option<String> {
    (c.dotnet_unavailable > 0).then(|| {
        format!(
            "WARNING: DotNet unavailable: {} test(s) not run because the .NET helper could not be used ({}). Pass --dotnet-sidecar PATH/AlDotNetSidecar.dll or set ALRT_DOTNET_SIDECAR; the pass count of this run is not comparable with one that had the helper.",
            c.dotnet_unavailable,
            first_line(why.trim_start_matches(al_vm::dotnet::UNAVAILABLE).trim_start_matches(':').trim(), 160)
        )
    })
}

/// The first line of a message, cut to `max` characters.
pub fn first_line(s: &str, max: usize) -> String {
    let line = s.lines().next().unwrap_or("");
    // A control character (a NUL in a compared text, say) is shown as `\x00`, a tab stays.
    let shown: String = line
        .chars()
        .map(|c| {
            if c.is_control() && c != '\t' {
                format!("\\x{:02X}", u32::from(c))
            } else {
                c.to_string()
            }
        })
        .collect();
    al_testrun::refusals::clip(&shown, max)
}

/// `Tests: 12 total, 5 passed, 1 failed, 6 not run`, and the warning that goes with a run that
/// has no test data.
pub fn summary_line(c: &Counts, test_data: bool) -> String {
    let mut pass_notes = Vec::new();
    if c.stubbed_pass > 0 {
        pass_notes.push(format!("{} stubbed", c.stubbed_pass));
    }
    if c.flagged > 0 {
        pass_notes.push(format!("{} flagged: rely on unprobed assumptions", c.flagged));
    }
    let flagged = if pass_notes.is_empty() {
        String::new()
    } else {
        format!(" ({})", pass_notes.join(", "))
    };
    let stubbed = |n: usize| {
        if n > 0 {
            format!(" ({n} stubbed)")
        } else {
            String::new()
        }
    };
    format!(
        "Tests: {} total, {} passed{flagged}, {} failed{}, {} not run{}{}",
        c.total,
        c.pass,
        c.fail,
        stubbed(c.stubbed_fail),
        c.not_run,
        stubbed(c.stubbed_not_run),
        if test_data {
            ""
        } else {
            " (no test data: failures are mostly missing setup data and are not trusted; the not-run counts and reasons are)"
        }
    )
}

/// The compact time line of the default output, under the totals table: the total wall time, then
/// compile and run (the two phases a user can act on) and the test rate, the parenthesis dimmed.
/// The other phases and the peak memory are in the `Time:` line of `--verbose`.
pub fn time_line(
    total: std::time::Duration,
    compile: std::time::Duration,
    run: std::time::Duration,
    rate: f64,
) -> String {
    let s = |d: std::time::Duration| format!("{:.2} s", d.as_secs_f64());
    format!(
        "  {:<8}{:>10}   {}",
        "time",
        s(total),
        crate::style::dim(&format!("(compile {}, run {}, {rate:.0} tests/s)", s(compile), s(run)))
    )
}

/// The totals as a small aligned table under the `Tests:` line: count and share per outcome, the flagged
/// passes in cyan. The `Tests:` line itself stays the parseable form.
pub fn summary_table(c: &Counts) -> Vec<String> {
    let pct = |n: usize| {
        if c.total == 0 {
            0.0
        } else {
            n as f64 * 100.0 / c.total as f64
        }
    };
    let row = |label: &str, n: usize, extra: String| format!("  {label:<8}{n:>7}  {:>5.1} %{extra}", pct(n));
    let flagged = if c.flagged > 0 {
        crate::style::cyan(&format!("  ({} flagged)", c.flagged))
    } else {
        String::new()
    };
    vec![
        row("passed", c.pass, flagged),
        row("failed", c.fail, String::new()),
        row("not run", c.not_run, String::new()),
    ]
}

/// JUnit XML grouped by test codeunit. A test that did not run is `<skipped>` with its reason.
/// `bc_build` is recorded as a property, so a build change that flips a test shows its cause.
/// The frames of an error's call stack for a person: one per line, innermost first. A frame whose
/// format is not known is in the stack as a marker; it reads as such.
pub fn stack_lines(stack: &str) -> Vec<String> {
    stack
        .lines()
        .map(|l| {
            if l.contains(al_testrun::UNTRACKED_CALL_STACK) {
                "(a frame whose format is not known)".to_string()
            } else {
                l.to_string()
            }
        })
        .collect()
}

/// The `--output-json` document, shaped like AL Runner v2's (`Reporter.SerializeJsonOutput`), pretty printed so that the
/// document starts with a `{` in column 0.
///
/// - `tests[].name` is `Codeunit<ID>.<Method>`; `status` is `pass`, `fail` or `error`; `durationMs`, `message` and
///   `stackTrace` (the AL call stack, one frame per line) are left out when empty.
/// - v2 told an assertion failure (`fail`) from an unexpected exception (`error`). v3 has no such difference to
///   draw: every failing test is `fail`.
/// - A test that did not run is `error` with the message `NotRun: out-of-scope: <reason>`, and `notRun: true`; one that
///   the time limit stopped is `error` with `Test exceeded <N>s timeout.` (the sentence v2 used). So `errors` counts the
///   tests not run, and `skipped` is 0.
/// - `codeunit`, `codeunitName` and `method` are v3's additions.
pub fn json(results: &[&TestResult], exit_code: u8, wall_s: f64) -> String {
    json_tdd(results, exit_code, wall_s, None)
}

/// [`json`] of a run with `--tdd` (ADR-051 decision 8) when `tdd_stubs` (every stub of the program) is given: the
/// summary adds `stubbed` (`passed`, `failed`, `error`) and `generatedStubs`. A stubbed test's row has `stubbed: true`
/// and `generatedStubs` with or without it.
pub fn json_tdd(results: &[&TestResult], exit_code: u8, wall_s: f64, tdd_stubs: Option<&[String]>) -> String {
    use serde_json::{Value, json};
    let c = Counts::of(results.iter().copied());
    let tests: Vec<Value> = results
        .iter()
        .map(|r| {
            let mut row = json_row(r.codeunit, &r.codeunit_name, &r.test, &r.outcome, &r.call_stack, r.time);
            add_stubs(&mut row, &r.stubs);
            Value::Object(row)
        })
        .collect();
    let mut doc = json!({
        "tests": tests,
        "passed": c.pass,
        "failed": c.fail,
        "errors": c.not_run,
        "skipped": 0,
        "notRun": c.not_run,
        "total": c.total,
        "exitCode": exit_code,
        "wallSeconds": (wall_s * 1000.0).round() / 1000.0,
    });
    if let Some(all) = tdd_stubs {
        doc["stubbed"] = json!({"passed": c.stubbed_pass, "failed": c.stubbed_fail, "error": c.stubbed_not_run});
        doc["generatedStubs"] = json!(all);
    }
    serde_json::to_string_pretty(&doc).expect("a JSON value serializes")
}

/// `stubbed: true` and `generatedStubs` on the JSON row of a test that ran stubs (ADR-051); nothing otherwise.
pub fn add_stubs(row: &mut serde_json::Map<String, serde_json::Value>, stubs: &[String]) {
    if !stubs.is_empty() {
        row.insert("stubbed".into(), serde_json::json!(true));
        row.insert("generatedStubs".into(), serde_json::json!(stubs));
    }
}

/// The closing block of a `--tdd` run on stderr, as v2 printed it: the stubs of the program, then the tests that ran
/// one with their outcome and stubs. Empty when the program has no stub.
pub fn tdd_block(results: &[&TestResult], all: &[String]) -> Vec<String> {
    if all.is_empty() {
        return Vec::new();
    }
    let mut out = vec![format!("--tdd: generated {} member(s) this run:", all.len())];
    out.extend(all.iter().map(|s| format!("  {s}")));
    let reached: Vec<&&TestResult> = results.iter().filter(|r| !r.stubs.is_empty()).collect();
    out.push(format!(
        "--tdd: {} test(s) reach generated stubs this run:",
        reached.len()
    ));
    for r in reached {
        let outcome = match r.outcome {
            Outcome::Pass => "pass",
            Outcome::Fail(_) => "fail",
            Outcome::NotRun(_) => "not run",
        };
        out.push(format!(
            "  {}.{} ({outcome}): {}",
            r.codeunit_name,
            r.test,
            r.stubs.join(", ")
        ));
    }
    out
}

/// One row of `tests[]` of the `--output-json` document, also the body of a `test` line of `al-runner serve`.
pub fn json_row(
    codeunit: u32,
    codeunit_name: &str,
    test: &str,
    outcome: &Outcome,
    call_stack: &str,
    time: std::time::Duration,
) -> serde_json::Map<String, serde_json::Value> {
    use serde_json::json;
    let mut t = serde_json::Map::new();
    t.insert("name".into(), json!(format!("Codeunit{codeunit}.{test}")));
    t.insert("codeunit".into(), json!(codeunit));
    t.insert("codeunitName".into(), json!(codeunit_name));
    t.insert("method".into(), json!(test));
    let (status, message) = match outcome {
        Outcome::Pass => ("pass", None),
        Outcome::Fail(m) => ("fail", Some(m.clone())),
        Outcome::NotRun(why) => {
            t.insert("notRun".into(), json!(true));
            ("error", Some(not_run_message(why)))
        }
    };
    t.insert("status".into(), json!(status));
    if !time.is_zero() {
        t.insert("durationMs".into(), json!(time.as_millis() as u64));
    }
    if let Some(m) = message {
        t.insert("message".into(), json!(m));
    }
    if !call_stack.is_empty() {
        t.insert("stackTrace".into(), json!(stack_lines(call_stack).join("\n")));
    }
    t
}

/// The message of a not-run test in the JSON document: v2's timeout sentence for a test the time limit stopped, else the
/// refusal reason in the form LethAL recognizes as "the runner refused" (`<Type>: out-of-scope: ...`).
fn not_run_message(why: &str) -> String {
    const LIMIT: &str = "the test used more than the time limit of ";
    if let Some(rest) = why.strip_prefix(LIMIT) {
        if let Some(n) = rest
            .split_whitespace()
            .next()
            .filter(|n| n.bytes().all(|b| b.is_ascii_digit()))
        {
            return format!("Test exceeded {n}s timeout.");
        }
    }
    format!("NotRun: out-of-scope: {}", why.lines().next().unwrap_or(""))
}

pub fn junit(
    results: &[&TestResult],
    bc_build: Option<&str>,
    wall_s: f64,
    disabled: &dyn Fn(u32, &str) -> bool,
    baseline_assumptions: &[String],
) -> String {
    junit_with(results, bc_build, wall_s, disabled, baseline_assumptions, &|_| {
        Vec::new()
    })
}

/// [`junit`], with further properties per test (an impact run's change class, ADR-043 step 3).
pub fn junit_with(
    results: &[&TestResult],
    bc_build: Option<&str>,
    wall_s: f64,
    disabled: &dyn Fn(u32, &str) -> bool,
    baseline_assumptions: &[String],
    properties: &dyn Fn(&TestResult) -> Vec<(String, String)>,
) -> String {
    let mut suites: Vec<al_testrun::junit::Suite> = Vec::new();
    let mut i = 0;
    while i < results.len() {
        let first = results[i];
        let mut j = i;
        while j < results.len() && results[j].app == first.app && results[j].codeunit == first.codeunit {
            j += 1;
        }
        suites.push(al_testrun::junit::Suite {
            name: first.codeunit_name.clone(),
            codeunit: first.codeunit,
            cases: results[i..j]
                .iter()
                .map(|r| al_testrun::junit::Case {
                    class: r.codeunit_name.clone(),
                    name: &r.test,
                    time: r.time.as_secs_f64(),
                    outcome: &r.outcome,
                    assumptions: &r.assumptions,
                    stack: if r.call_stack.is_empty() {
                        String::new()
                    } else {
                        stack_lines(&r.call_stack).join("\n")
                    },
                    disabled: disabled(r.codeunit, &r.test),
                    properties: {
                        let mut p = properties(r);
                        // `--tdd` (ADR-051 decision 8): the stubs a test ran.
                        if !r.stubs.is_empty() {
                            p.push(("stubbed".into(), r.stubs.join(", ")));
                        }
                        p
                    },
                })
                .collect(),
        });
        i = j;
    }
    al_testrun::junit::write_with_baseline(
        &suites,
        al_testrun::junit::Dialect::Runner,
        bc_build,
        wall_s,
        baseline_assumptions,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_time_line_has_the_total_first_then_compile_run_and_rate() {
        use std::time::Duration as D;
        let line = time_line(D::from_millis(2280), D::from_millis(1240), D::from_millis(730), 440.0);
        assert_eq!(line, "  time        2.28 s   (compile 1.24 s, run 0.73 s, 440 tests/s)");
    }

    #[test]
    fn the_summary_table_aligns_counts_and_percentages() {
        let c = Counts {
            total: 8,
            pass: 6,
            fail: 0,
            not_run: 2,
            flagged: 0,
            dotnet_unavailable: 0,
            ..Default::default()
        };
        assert_eq!(
            summary_table(&c),
            vec![
                "  passed        6   75.0 %",
                "  failed        0    0.0 %",
                "  not run       2   25.0 %"
            ]
        );
    }

    #[test]
    fn only_dependency_lines_that_matter_show_by_default() {
        assert!(!dependency_line_is_problem(
            "dependency note: C/.alpackages/Microsoft_Any_27.app: the package has no NavxManifest.xml"
        ));
        assert!(dependency_line_is_problem(
            "dependency note: X.app: app.json holds a build variable"
        ));
        assert!(dependency_line_is_problem("dependency skipped: Y"));
        assert!(!dependency_line_is_problem("fetching the feed"));
    }

    #[test]
    fn an_incomplete_baseline_is_one_short_line_and_a_complete_one_is_silent() {
        assert_eq!(
            baseline_warning(16, 17).as_deref(),
            Some("baseline incomplete: 16 of 17 install roots ran to their end (--verbose for details)")
        );
        assert_eq!(baseline_warning(17, 17), None);
        assert_eq!(baseline_warning(0, 0), None);
    }

    fn r(cu: u32, name: &str, test: &str, outcome: Outcome) -> TestResult {
        TestResult {
            suite: 0,
            app: 0,
            codeunit: cu,
            codeunit_name: name.into(),
            test: test.into(),
            outcome,
            call_stack: String::new(),
            assumptions: Vec::new(),
            time: std::time::Duration::ZERO,
            coverage: Vec::new(),
            messages: Vec::new(),
            captured: None,
            impact: Vec::new(),
            stubs: Vec::new(),
        }
    }

    #[test]
    fn json_has_the_shape_of_v2_and_rows_for_the_tests_not_run() {
        let mut slow = r(
            1,
            "A",
            "Slow",
            Outcome::NotRun(
                "the test used more than the time limit of 5 s of CPU time and was stopped (--test-timeout-secs); x"
                    .into(),
            ),
        );
        slow.time = std::time::Duration::from_millis(5000);
        let mut f = r(1, "A", "Bad", Outcome::Fail("boom".into()));
        f.call_stack = "\"A\"(CodeUnit 1).Bad line 1".into();
        f.time = std::time::Duration::from_millis(12);
        let rs = [
            r(1, "A", "Good", Outcome::Pass),
            f,
            r(2, "B", "Refused", Outcome::NotRun("no provider\nsecond line".into())),
            slow,
        ];
        let refs: Vec<&TestResult> = rs.iter().collect();
        let text = json(&refs, 1, 1.23456);
        assert!(text.starts_with("{\n"), "{text}");
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(
            (v["passed"].as_u64(), v["failed"].as_u64(), v["errors"].as_u64()),
            (Some(1), Some(1), Some(2))
        );
        assert_eq!(
            (v["total"].as_u64(), v["skipped"].as_u64(), v["exitCode"].as_u64()),
            (Some(4), Some(0), Some(1))
        );
        assert_eq!(v["wallSeconds"].as_f64(), Some(1.235));
        let t = &v["tests"];
        assert_eq!(t[0]["name"], "Codeunit1.Good");
        assert_eq!(t[0]["status"], "pass");
        assert!(t[0].get("message").is_none() && t[0].get("durationMs").is_none());
        assert_eq!(
            (
                t[1]["status"].as_str(),
                t[1]["message"].as_str(),
                t[1]["durationMs"].as_u64()
            ),
            (Some("fail"), Some("boom"), Some(12))
        );
        assert_eq!(t[1]["stackTrace"], "\"A\"(CodeUnit 1).Bad line 1");
        assert_eq!(t[2]["name"], "Codeunit2.Refused");
        assert_eq!(
            (t[2]["status"].as_str(), t[2]["message"].as_str()),
            (Some("error"), Some("NotRun: out-of-scope: no provider"))
        );
        assert_eq!(t[3]["message"], "Test exceeded 5s timeout.");
    }

    #[test]
    fn exit_codes_follow_the_rank_failure_then_not_run() {
        let c = |pass, fail, not_run| Counts {
            total: pass + fail + not_run,
            pass,
            flagged: 0,
            fail,
            not_run,
            dotnet_unavailable: 0,
            ..Default::default()
        };
        assert_eq!(exit_code(&c(3, 0, 0), false), 0);
        assert_eq!(exit_code(&c(2, 1, 0), false), 1);
        assert_eq!(exit_code(&c(2, 0, 1), false), 7);
        assert_eq!(exit_code(&c(2, 0, 1), true), 0);
        assert_eq!(exit_code(&c(1, 1, 1), true), 1);
        assert_eq!(exit_code(&c(0, 0, 0), true), 6);
    }

    #[test]
    fn tests_stopped_by_a_missing_dotnet_helper_have_their_own_exit_code_and_warning() {
        let rs = [
            r(1, "A", "T1", Outcome::Pass),
            r(
                1,
                "A",
                "T2",
                Outcome::NotRun("dotnet-unavailable: DotNet 'System.Convert' sidecar not built; run ...".into()),
            ),
            r(1, "A", "T3", Outcome::NotRun("something else".into())),
        ];
        let c = Counts::of(rs.iter());
        assert_eq!((c.not_run, c.dotnet_unavailable), (2, 1));
        assert_eq!(exit_code(&c, false), 8);
        assert_eq!(
            exit_code(&c, true),
            0,
            "--allow-not-run is the user's word that not run is fine"
        );
        let w = dotnet_warning(&c, "dotnet-unavailable: DotNet 'System.Convert' sidecar not built").unwrap();
        assert!(
            w.contains("DotNet unavailable: 1 test(s)") && w.contains("ALRT_DOTNET_SIDECAR"),
            "{w}"
        );
        assert!(
            w.contains("sidecar not built") && !w.contains("dotnet-unavailable"),
            "{w}"
        );
        let none = Counts::of(rs[..1].iter().chain(rs[2..].iter()));
        assert_eq!(exit_code(&none, false), 7);
        assert!(dotnet_warning(&none, "").is_none());
    }

    #[test]
    fn a_pass_that_ran_through_an_assumption_is_flagged_in_the_summary_and_junit_and_still_a_pass() {
        let mut flagged = r(1, "A", "T1", Outcome::Pass);
        flagged.assumptions = vec![
            al_testrun::assumptions::Assumption::QueryClosedAfterLastRow,
            al_testrun::assumptions::Assumption::NextWholeDecimal,
        ];
        let rs = [
            flagged,
            r(1, "A", "T2", Outcome::Pass),
            r(1, "A", "T3", Outcome::Fail("x".into())),
        ];
        let c = Counts::of(rs.iter());
        assert_eq!((c.pass, c.flagged), (2, 1));
        assert_eq!(
            summary_line(&c, true),
            "Tests: 3 total, 2 passed (1 flagged: rely on unprobed assumptions), 1 failed, 0 not run"
        );
        let clean = Counts::of(rs[1..].iter());
        assert_eq!(
            summary_line(&clean, true),
            "Tests: 2 total, 1 passed, 1 failed, 0 not run"
        );
        let refs: Vec<&TestResult> = rs.iter().collect();
        let x = junit(&refs, None, 0.0, &|_, _| false, &[]);
        assert!(
            x.contains("name=\"T1\" time=\"0.000\"><properties><property name=\"assumptions\" value=\"query-closed-after-last-row,next-whole-decimal\" /></properties></testcase>"),
            "{x}"
        );
        assert!(x.contains("name=\"T2\" time=\"0.000\" />"), "{x}");
        assert!(x.contains("failures=\"1\""), "{x}");
        assert!(
            !x.contains("name=\"T1\"><skipped") && !x.contains("name=\"T1\"><failure"),
            "{x}"
        );
    }

    #[test]
    fn a_control_character_in_a_message_is_shown_as_an_escape() {
        // Tests-Workflow has a failure whose text holds a NUL (`Expected:<asdf\0>`): printed raw it made
        // the whole output a binary file to grep and to a terminal.
        assert_eq!(
            first_line("Expected:<asdf\u{0}> and \u{7}bell\tab", 80),
            "Expected:<asdf\\x00> and \\x07bell\tab"
        );
    }

    #[test]
    fn junit_groups_by_codeunit_and_escapes() {
        let rs = [
            r(1, "A <b>", "T1", Outcome::Pass),
            r(1, "A <b>", "T2", Outcome::Fail("bad \"x\"\nline2".into())),
            r(2, "B", "T3", Outcome::NotRun("no & way".into())),
        ];
        let refs: Vec<&TestResult> = rs.iter().collect();
        let x = junit(&refs, Some("27.5.1.2"), 1.5, &|_, _| false, &[]);
        assert!(
            x.contains("<testsuites tests=\"3\" failures=\"1\" errors=\"0\" skipped=\"1\" time=\"1.500\">"),
            "{x}"
        );
        assert!(
            x.contains("<testsuite name=\"A &lt;b&gt;\" tests=\"2\" failures=\"1\""),
            "{x}"
        );
        assert!(x.contains("<failure message=\"bad &quot;x&quot;&#10;line2\" />"), "{x}");
        assert!(x.contains("<skipped message=\"no &amp; way\" />"), "{x}");
        assert!(x.contains("name=\"bc_build\" value=\"27.5.1.2\""), "{x}");
        assert_eq!(x.matches("<testsuite ").count(), 2);
    }

    #[test]
    fn junit_carries_the_time_of_each_test_and_the_sum_for_its_codeunit() {
        let mut a = r(1, "A", "T1", Outcome::Pass);
        a.time = std::time::Duration::from_millis(1250);
        let mut b = r(1, "A", "T2", Outcome::Fail("x".into()));
        b.time = std::time::Duration::from_millis(500);
        let mut c = r(1, "A", "T3", Outcome::NotRun("y".into()));
        c.time = std::time::Duration::ZERO;
        let refs = [&a, &b, &c];
        let x = junit(&refs, None, 2.0, &|_, _| false, &[]);
        assert!(
            x.contains("<testcase classname=\"A\" name=\"T1\" time=\"1.250\" />"),
            "{x}"
        );
        assert!(x.contains("name=\"T2\" time=\"0.500\">"), "{x}");
        assert!(x.contains("name=\"T3\" time=\"0.000\">"), "{x}");
        assert!(
            x.contains("<testsuite name=\"A\" tests=\"3\" failures=\"1\" errors=\"0\" skipped=\"1\" time=\"1.750\">"),
            "{x}"
        );
    }

    #[test]
    fn junit_puts_the_call_stack_in_the_failure_and_names_an_unknown_frame() {
        let mut f = r(1, "A", "T1", Outcome::Fail("boom".into()));
        f.call_stack = format!(
            "\"A\"(CodeUnit 1).Inner line 2\n{}\n\"A\"(CodeUnit 1).T1 line 1",
            al_testrun::UNTRACKED_CALL_STACK
        );
        let refs = [&f];
        let x = junit(&refs, None, 0.0, &|_, _| false, &[]);
        assert!(x.contains("<failure message=\"boom\">&quot;A&quot;(CodeUnit 1).Inner line 2&#10;(a frame whose format is not known)&#10;"), "{x}");
        assert!(x.contains("</failure>"), "{x}");
        assert!(!x.contains(al_testrun::UNTRACKED_CALL_STACK), "{x}");
    }
}
