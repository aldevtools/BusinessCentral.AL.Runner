//! Renders a report as Markdown (CI step summary) and the runtime's outcomes as
//! JUnit in the same shape the corpus CI emits, so the two diff line for line.

use crate::oracle::{CaseId, OracleCase};
use crate::report::{Outcome, Report, Timings};
use al_testrun::assumptions::Assumption;

/// The CI step summary: one line for the run. The numbers per area, the phase times and the
/// other test apps are in the JSON report and in `--timings`.
pub fn markdown(r: &Report) -> String {
    let c = &r.overall;
    let version = r
        .timings
        .as_ref()
        .map_or(String::new(), |t| format!("BC {}: ", t.bc_version));
    let time = r
        .timings
        .as_ref()
        .map_or(String::new(), |t| format!(", {}", secs(t.total_s)));
    let flagged = if c.flagged > 0 {
        format!(" ({} flagged: rely on unprobed assumptions)", c.flagged)
    } else {
        String::new()
    };
    format!(
        "- {version}{} passed{flagged}, {} failed, {} not run of {} ({:.1}%){time}\n",
        c.pass,
        c.fail,
        c.not_run,
        c.total,
        c.rate() * 100.0
    )
}

fn secs(x: f64) -> String {
    format!("{x:.2} s")
}

/// One small table of run times: a row per run, the phases in order, then the total.
pub fn timings_table(ts: &[Timings]) -> String {
    let mut s = String::from(
        "| Version | Load | Compile | Install baseline | Tests | Total |\n|---|---:|---:|---:|---:|---:|\n",
    );
    for t in ts {
        let eager = if t.eager { ", all dependencies compiled" } else { "" };
        s.push_str(&format!(
            "| BC {}{eager} | {} | {} | {} | {} | {} |\n",
            t.bc_version,
            secs(t.load_s),
            secs(t.compile_s),
            secs(t.baseline_s),
            secs(t.tests_s),
            secs(t.total_s)
        ));
    }
    s
}

/// `bc_build` is the Microsoft build the dependency apps came from; it is recorded as a JUnit
/// property so that a build change that flips a case shows its cause next to the result.
pub fn junit(
    oracle: &[OracleCase],
    outcome: &dyn Fn(&CaseId) -> Outcome,
    time: &dyn Fn(&CaseId) -> f64,
    bc_build: Option<&str>,
) -> String {
    junit_with(oracle, outcome, time, &|_| Vec::new(), bc_build)
}

/// `junit`, with the unprobed assumptions each test ran through. A passing test that hit any stays a pass
/// (CI tools see no failure) and carries an `assumptions` property listing their ids.
pub fn junit_with(
    oracle: &[OracleCase],
    outcome: &dyn Fn(&CaseId) -> Outcome,
    time: &dyn Fn(&CaseId) -> f64,
    assumptions: &dyn Fn(&CaseId) -> Vec<Assumption>,
    bc_build: Option<&str>,
) -> String {
    type Row<'a> = (&'a CaseId, Outcome, Vec<Assumption>);
    let mut by_cu: std::collections::BTreeMap<u32, Vec<Row>> = Default::default();
    for c in oracle.iter().filter(|c| !c.bc_failed) {
        let o = outcome(&c.id);
        // Only a pass carries its assumptions in this dialect.
        let marks = if matches!(o, Outcome::Pass) {
            assumptions(&c.id)
        } else {
            Vec::new()
        };
        by_cu.entry(c.id.codeunit).or_default().push((&c.id, o, marks));
    }
    let suites: Vec<al_testrun::junit::Suite> = by_cu
        .iter()
        .map(|(cu, cases)| al_testrun::junit::Suite {
            name: format!("Codeunit {cu}"),
            codeunit: *cu,
            properties: Vec::new(),
            cases: cases
                .iter()
                .map(|(id, o, marks)| al_testrun::junit::Case {
                    class: format!("Codeunit {cu}"),
                    name: &id.name,
                    time: time(id),
                    outcome: o,
                    assumptions: marks,
                    stack: String::new(),
                    disabled: false,
                    properties: Vec::new(),
                })
                .collect(),
        })
        .collect();
    al_testrun::junit::write(&suites, al_testrun::junit::Dialect::Oracle, bc_build, 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::build;

    fn id(cu: u32, n: &str) -> CaseId {
        CaseId {
            codeunit: cu,
            name: n.into(),
        }
    }
    fn oracle() -> Vec<OracleCase> {
        vec![
            OracleCase {
                id: id(1, "a"),
                bc_failed: false,
            },
            OracleCase {
                id: id(1, "b<&>"),
                bc_failed: false,
            },
            OracleCase {
                id: id(2, "c"),
                bc_failed: false,
            },
        ]
    }
    fn outcome(c: &CaseId) -> Outcome {
        match c.name.as_str() {
            "a" => Outcome::Pass,
            "c" => Outcome::NotRun("unsupported builtin `Format`".into()),
            _ => Outcome::Fail("Assert.AreEqual failed. <1> & <2>".into()),
        }
    }
    fn areas(cu: u32) -> Option<String> {
        Some(if cu == 1 { "text" } else { "record" }.into())
    }

    #[test]
    fn markdown_is_one_line_with_the_overall_counts_and_the_total_time() {
        let mut r = build(&oracle(), &areas, &outcome);
        assert_eq!(markdown(&r), "- 1 passed, 1 failed, 1 not run of 3 (33.3%)\n");
        r.timings = Some(timings());
        assert_eq!(
            markdown(&r),
            "- BC 29.0: 1 passed, 1 failed, 1 not run of 3 (33.3%), 18.75 s\n"
        );
    }

    #[test]
    fn markdown_counts_flagged_passes_inside_the_passed_number() {
        let marks = |c: &CaseId| {
            if c.name == "a" {
                vec![Assumption::QueryClosedAfterLastRow]
            } else {
                Vec::new()
            }
        };
        let r = crate::report::build_with(&oracle(), &areas, &outcome, &marks);
        assert_eq!(
            markdown(&r),
            "- 1 passed (1 flagged: rely on unprobed assumptions), 1 failed, 1 not run of 3 (33.3%)\n"
        );
    }

    #[test]
    fn a_flagged_pass_is_still_a_pass_in_junit_and_lists_its_ids() {
        let marks = |c: &CaseId| {
            if c.name == "a" {
                vec![Assumption::QueryClosedAfterLastRow, Assumption::NextWholeDecimal]
            } else {
                Vec::new()
            }
        };
        let xml = junit_with(&oracle(), &outcome, &|_| 0.0, &marks, None);
        assert!(
            xml.contains(r#"<property name="assumptions" value="query-closed-after-last-row,next-whole-decimal" />"#),
            "{xml}"
        );
        // The same cases pass and fail as without the property.
        let back = crate::oracle::parse_junit(&xml).unwrap();
        let ids: Vec<_> = back.iter().map(|c| (c.id.clone(), c.bc_failed)).collect();
        assert_eq!(
            ids,
            vec![(id(1, "a"), false), (id(1, "b<&>"), true), (id(2, "c"), true)]
        );
        assert!(!junit(&oracle(), &outcome, &|_| 0.0, None).contains("assumptions"));
    }

    #[test]
    fn markdown_leaves_out_areas_milestones_and_phase_tables() {
        let mut r = build(&oracle(), &areas, &outcome);
        r.timings = Some(timings());
        let md = markdown(&r);
        for gone in ["By area", "M2", "milestone", "| record |", "How long it took", "Load"] {
            assert!(!md.contains(gone), "{gone} in {md}");
        }
        assert_eq!(md.lines().count(), 1, "{md}");
    }

    #[test]
    fn the_json_report_names_the_build_and_an_old_one_still_reads() {
        let mut r = build(&oracle(), &areas, &outcome);
        r.bc_build = Some("27.5.46862.56249".into());
        assert!(
            serde_json::to_string(&r)
                .unwrap()
                .contains(r#""bc_build":"27.5.46862.56249""#)
        );
        assert!(!serde_json::to_string(&r).unwrap().contains("milestones"));
        // A report written before the sidecar field existed still reads.
        let json = r#"{"timings":{"bc_version":"29.0","eager":false,"load_s":1.0,"compile_s":1.0,"baseline_s":1.0,"tests_s":1.0,"total_s":4.0,"suites":[]}}"#;
        assert!(crate::report::Timings::from_report_json(json).unwrap().dotnet.is_none());
    }

    #[test]
    fn junit_round_trips_through_the_oracle_parser() {
        let xml = junit(&oracle(), &outcome, &|_| 0.0, None);
        let back = crate::oracle::parse_junit(&xml).unwrap();
        let ids: Vec<_> = back.iter().map(|c| (c.id.clone(), c.bc_failed)).collect();
        assert_eq!(
            ids,
            vec![(id(1, "a"), false), (id(1, "b<&>"), true), (id(2, "c"), true)]
        );
    }

    #[test]
    fn the_microsoft_build_is_a_junit_property_and_the_oracle_parser_still_reads_the_file() {
        let xml = junit(&oracle(), &outcome, &|_| 0.0, Some("27.5.46862.56249"));
        assert!(
            xml.contains(r#"<property name="bc_build" value="27.5.46862.56249" />"#),
            "{xml}"
        );
        assert_eq!(crate::oracle::parse_junit(&xml).unwrap().len(), 3);
        assert!(!junit(&oracle(), &outcome, &|_| 0.0, None).contains("bc_build"));
    }

    fn timings() -> crate::report::Timings {
        crate::report::Timings {
            bc_version: "29.0".into(),
            eager: false,
            load_s: 1.0,
            compile_s: 2.5,
            baseline_s: 3.0,
            tests_s: 12.25,
            total_s: 18.75,
            suites: vec![
                crate::report::SuiteTime {
                    name: "corpus".into(),
                    tests_s: 11.0,
                },
                crate::report::SuiteTime {
                    name: "OnPrem".into(),
                    tests_s: 1.25,
                },
            ],
            dotnet: None,
            compile_cache: None,
            slowest_tests: vec![crate::report::SlowTest {
                codeunit: 60002,
                name: "A_Works".into(),
                seconds: 1.5,
            }],
        }
    }

    #[test]
    fn the_json_report_carries_the_timings() {
        let mut r = build(&oracle(), &areas, &outcome);
        r.timings = Some(timings());
        let v: serde_json::Value = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
        let t = &v["timings"];
        for key in ["load_s", "compile_s", "baseline_s", "tests_s", "total_s"] {
            assert!(t[key].is_number(), "{key} in {t}");
        }
        assert_eq!(t["bc_version"], "29.0");
        assert_eq!(t["suites"][1]["name"], "OnPrem");
    }

    #[test]
    fn a_timing_table_is_read_back_from_report_files() {
        let mut r = build(&oracle(), &areas, &outcome);
        r.timings = Some(timings());
        let json = serde_json::to_string_pretty(&r).unwrap();
        let t = crate::report::Timings::from_report_json(&json).unwrap();
        let mut eager = t.clone();
        eager.eager = true;
        eager.bc_version = "28.0".into();
        let table = timings_table(&[t, eager]);
        assert!(
            table.contains("| Version | Load | Compile | Install baseline | Tests | Total |"),
            "{table}"
        );
        assert!(table.contains("| BC 29.0 |"), "{table}");
        assert!(table.contains("| BC 28.0, all dependencies compiled |"), "{table}");
        assert!(crate::report::Timings::from_report_json("{}").is_err());
    }

    #[test]
    fn junit_never_emits_characters_xml_forbids() {
        // XML 1.0 allows no control characters but tab, LF and CR; a reason that carries one
        // (a qualified-name separator) made the CI artifact unparseable.
        let outcome = |_: &CaseId| Outcome::NotRun("a\u{1f}b\u{0}c\td".into());
        let xml = junit(&oracle(), &outcome, &|_| 0.0, None);
        assert!(
            !xml.chars()
                .any(|c| (c as u32) < 0x20 && !matches!(c, '\t' | '\n' | '\r')),
            "{xml:?}"
        );
        assert!(
            xml.contains('a') && xml.contains('b') && xml.contains("c\td"),
            "{xml:?}"
        );
    }

    #[test]
    fn junit_carries_the_time_of_each_test_and_the_sum_for_its_codeunit() {
        let xml = junit(&oracle(), &outcome, &|c| if c.name == "a" { 1.25 } else { 0.5 }, None);
        assert!(xml.contains("name=\"a\" time=\"1.250\""), "{xml}");
        // the oracle parser still reads the file
        assert_eq!(crate::oracle::parse_junit(&xml).unwrap().len(), 3);
        assert!(xml.contains("time=\"0.500\""), "{xml}");
    }

    #[test]
    fn the_json_report_names_the_slowest_tests() {
        let mut r = build(&oracle(), &areas, &outcome);
        r.timings = Some(timings());
        let v: serde_json::Value = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
        assert_eq!(v["timings"]["slowest_tests"][0]["name"], "A_Works");
        assert_eq!(v["timings"]["slowest_tests"][0]["seconds"], 1.5);
    }

    #[test]
    fn junit_marks_not_run_as_skipped_and_fail_as_failure() {
        let xml = junit(&oracle(), &outcome, &|_| 0.0, None);
        assert!(
            xml.contains("<skipped message=\"unsupported builtin `Format`\""),
            "{xml}"
        );
        assert!(
            xml.contains("<failure message=\"Assert.AreEqual failed. &lt;1&gt; &amp; &lt;2&gt;\""),
            "{xml}"
        );
        assert!(
            xml.contains("<testsuites tests=\"3\" failures=\"1\" skipped=\"1\">"),
            "{xml}"
        );
    }
}
