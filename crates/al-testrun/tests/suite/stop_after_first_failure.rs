//! Al-runner issue 8210: `RunOptions::stop_after_first_failure` ends a run at its first failing test. The later tests of
//! that codeunit and every later entry are not run and end as not run, with a reason that names the failed test. Default
//! off: everything runs.

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, EntryRun, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const SRC: &str = r#"
codeunit 50710 "Stop After Fail"
{
    Subtype = Test;
    [Test]
    procedure A_Passes()
    begin
    end;
    [Test]
    procedure B_Fails()
    begin
        Error('the first test fails');
    end;
    [Test]
    procedure C_Passes()
    begin
    end;
}
codeunit 50711 "Passing Codeunit"
{
    Subtype = Test;
    [Test]
    procedure D_Passes()
    begin
    end;
}
"#;

fn run(opts: RunOptions) -> CorpusRuntime {
    run_source(SRC, opts)
}

fn run_source(src: &str, opts: RunOptions) -> CorpusRuntime {
    CorpusRuntime::run_suites_with(
        vec![AppSource {
            id: "main".into(),
            name: "main".into(),
            dependencies: vec![],
            files: vec![("main.al".into(), src.into())],
        }],
        vec![al_sema::AppKind::Workspace],
        &[],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &opts,
    )
}

fn outcome(rt: &CorpusRuntime, codeunit: u32, name: &str) -> Outcome {
    rt.first_outcome(&CaseId {
        codeunit,
        name: name.into(),
    })
}

fn entry(id: u32) -> EntryRun {
    EntryRun {
        only: Some(vec![(0, id)]),
        ..EntryRun::default()
    }
}

/// The rows of each entry, in the order of the entries.
fn entry_rows(rt: &CorpusRuntime) -> Vec<Vec<(String, Outcome)>> {
    rt.entries
        .iter()
        .map(|r| {
            rt.results()[r.clone()]
                .iter()
                .map(|t| (t.test.clone(), t.outcome.clone()))
                .collect()
        })
        .collect()
}

fn stopped(o: &Outcome) -> bool {
    matches!(o, Outcome::NotRun(m) if m.contains("B_Fails") && m.contains("stopAfterFirstFailure"))
}

#[test]
fn by_default_the_tests_after_a_failure_run() {
    let rt = run(RunOptions::default());
    assert!(matches!(outcome(&rt, 50710, "B_Fails"), Outcome::Fail(_)));
    assert_eq!(outcome(&rt, 50710, "C_Passes"), Outcome::Pass);
    assert_eq!(outcome(&rt, 50711, "D_Passes"), Outcome::Pass);
}

#[test]
fn with_the_stop_the_later_tests_of_the_codeunit_are_not_run() {
    let rt = run(RunOptions {
        stop_after_first_failure: true,
        ..RunOptions::default()
    });
    assert_eq!(outcome(&rt, 50710, "A_Passes"), Outcome::Pass);
    assert!(matches!(outcome(&rt, 50710, "B_Fails"), Outcome::Fail(_)));
    let c = outcome(&rt, 50710, "C_Passes");
    assert!(stopped(&c), "{c:?}");
}

#[test]
fn with_the_stop_the_entries_after_a_failing_one_are_not_run() {
    let rt = run(RunOptions {
        stop_after_first_failure: true,
        entries: vec![entry(50711), entry(50710), entry(50711), entry(50710)],
        ..RunOptions::default()
    });
    let rows = entry_rows(&rt);
    assert_eq!(rows.len(), 4, "{rows:?}");
    assert_eq!(rows[0], vec![("D_Passes".to_string(), Outcome::Pass)]);
    assert!(matches!(&rows[1][1].1, Outcome::Fail(_)), "{rows:?}");
    assert!(stopped(&rows[1][2].1), "{rows:?}");
    for later in &rows[2..] {
        assert!(!later.is_empty() && later.iter().all(|(_, o)| stopped(o)), "{rows:?}");
    }
    // Upstream #8484's text, byte for byte: with no `stop_group` on any entry the reason is unchanged (PERF-18 T7).
    let upstream = "the test Codeunit50710.B_Fails failed before it, and stopAfterFirstFailure (--stop-after-first-failure) runs no test of its codeunit or of a later entry after a failure";
    for (_, o) in rows[1][2..].iter().chain(rows[2..].iter().flatten()) {
        assert_eq!(o, &Outcome::NotRun(upstream.to_string()));
    }
}

#[test]
fn without_the_stop_every_entry_runs() {
    let rt = run(RunOptions {
        entries: vec![entry(50710), entry(50711)],
        ..RunOptions::default()
    });
    let rows = entry_rows(&rt);
    assert_eq!(rows[1], vec![("D_Passes".to_string(), Outcome::Pass)], "{rows:?}");
    assert_eq!(rows[0][2], ("C_Passes".to_string(), Outcome::Pass), "{rows:?}");
}

/// Whether the row of `name` in codeunit `codeunit` is marked as not run by the stop (`TestResult::skipped_by_option`).
fn marked(rt: &CorpusRuntime, codeunit: u32, name: &str) -> Vec<bool> {
    rt.results()
        .iter()
        .filter(|t| t.codeunit == codeunit && t.test == name)
        .map(|t| t.skipped_by_option)
        .collect()
}

/// The rows the stop leaves out are marked as left out by an option of the run, not refused; the failed test and the
/// tests that ran are not (al-runner issue 8492).
#[test]
fn the_rows_the_stop_leaves_out_are_marked_and_the_others_are_not() {
    let rt = run(RunOptions {
        stop_after_first_failure: true,
        entries: vec![entry(50710), entry(50711)],
        ..RunOptions::default()
    });
    assert_eq!(marked(&rt, 50710, "A_Passes"), [false]);
    assert_eq!(marked(&rt, 50710, "B_Fails"), [false]);
    assert_eq!(marked(&rt, 50710, "C_Passes"), [true]);
    assert_eq!(marked(&rt, 50711, "D_Passes"), [true], "the row of the later entry");
    let all = run(RunOptions {
        entries: vec![entry(50710), entry(50711)],
        ..RunOptions::default()
    });
    assert!(all.results().iter().all(|t| !t.skipped_by_option));
}

const TIMES_OUT_FIRST: &str = r#"
codeunit 50712 "Times Out First"
{
    Subtype = Test;
    [Test]
    procedure A_TimesOut()
    begin
        Sleep(5000);
    end;
    [Test]
    procedure B_Fails()
    begin
        Error('fails after the timeout');
    end;
    [Test]
    procedure C_Passes()
    begin
    end;
}
"#;

/// A test the time limit stopped is not a failure: the codeunit goes on, and stops at the failing test after it
/// (al-runner issue 8492).
#[test]
fn a_timeout_before_the_failing_test_does_not_stop_the_codeunit() {
    let rt = run_source(
        TIMES_OUT_FIRST,
        RunOptions {
            stop_after_first_failure: true,
            test_time_limit: Some(std::time::Duration::from_secs(1)),
            ..RunOptions::default()
        },
    );
    let a = outcome(&rt, 50712, "A_TimesOut");
    assert!(matches!(&a, Outcome::NotRun(m) if m.contains("time limit")), "{a:?}");
    assert!(matches!(outcome(&rt, 50712, "B_Fails"), Outcome::Fail(_)));
    let c = outcome(&rt, 50712, "C_Passes");
    assert!(stopped(&c), "{c:?}");
    assert_eq!(marked(&rt, 50712, "A_TimesOut"), [false]);
}

const SECOND_FAILING: &str = r#"
codeunit 50713 "Second Failing"
{
    Subtype = Test;
    [Test]
    procedure E_Fails()
    begin
        Error('the second codeunit fails');
    end;
    [Test]
    procedure F_Passes()
    begin
    end;
}
"#;

/// An entry with two failing test codeunits: each stops at its own failure, and the later entry is not run, naming the
/// first failure (al-runner issue 8492).
#[test]
fn an_entry_with_several_failing_codeunits_stops_each_at_its_own_failure() {
    let rt = run_source(
        &format!("{SRC}{SECOND_FAILING}"),
        RunOptions {
            stop_after_first_failure: true,
            entries: vec![
                EntryRun {
                    only: Some(vec![(0, 50710), (0, 50713)]),
                    ..EntryRun::default()
                },
                entry(50711),
            ],
            ..RunOptions::default()
        },
    );
    let rows = entry_rows(&rt);
    assert_eq!(rows.len(), 2, "{rows:?}");
    let of = |name: &str| rows[0].iter().find(|(t, _)| t == name).map(|(_, o)| o.clone());
    assert!(matches!(of("B_Fails"), Some(Outcome::Fail(_))), "{rows:?}");
    assert!(of("C_Passes").as_ref().is_some_and(stopped), "{rows:?}");
    assert!(matches!(of("E_Fails"), Some(Outcome::Fail(_))), "{rows:?}");
    assert!(
        matches!(of("F_Passes"), Some(Outcome::NotRun(m)) if m.contains("E_Fails") && m.contains("stopAfterFirstFailure")),
        "{rows:?}"
    );
    assert_eq!(rows[1].len(), 1, "{rows:?}");
    assert!(stopped(&rows[1][0].1), "{rows:?}");
}

// PERF-18: `EntryRun::stop_group` scopes the stop. With the stop on and some entry grouped, a failure skips the later
// entries of its own group (and the rest of its codeunit); an entry without a group neither triggers a stop nor is
// stopped.

fn grouped(id: u32, g: Option<&str>) -> EntryRun {
    EntryRun {
        stop_group: g.map(String::from),
        ..entry(id)
    }
}

fn stop_on(entries: Vec<EntryRun>) -> RunOptions {
    RunOptions {
        stop_after_first_failure: true,
        entries,
        ..RunOptions::default()
    }
}

/// The row was not run because of a failure in group `g`: B_Fails of 50710, named with `stopGroup `g``.
fn stopped_in(o: &Outcome, g: &str) -> bool {
    matches!(o, Outcome::NotRun(m) if m.contains("Codeunit50710.B_Fails")
        && m.contains("stopAfterFirstFailure")
        && m.contains(&format!("stopGroup `{g}`")))
}

fn all_stopped_in(rows: &[(String, Outcome)], g: &str) -> bool {
    !rows.is_empty() && rows.iter().all(|(_, o)| stopped_in(o, g))
}

fn all_pass(rows: &[(String, Outcome)]) -> bool {
    !rows.is_empty() && rows.iter().all(|(_, o)| *o == Outcome::Pass)
}

fn m1_m2_entries() -> Vec<EntryRun> {
    vec![
        grouped(50710, Some("m1")),
        grouped(50711, Some("m1")),
        grouped(50710, Some("m2")),
        grouped(50711, Some("m2")),
    ]
}

#[test]
fn a_failure_skips_only_the_later_entries_of_its_group() {
    let rt = run(stop_on(m1_m2_entries()));
    let rows = entry_rows(&rt);
    assert_eq!(rows.len(), 4, "{rows:?}");
    assert!(all_stopped_in(&rows[1], "m1"), "{rows:?}");
    assert_eq!(rows[2][0], ("A_Passes".to_string(), Outcome::Pass), "{rows:?}");
    assert!(matches!(&rows[2][1].1, Outcome::Fail(_)), "{rows:?}");
    assert!(stopped_in(&rows[2][2].1, "m2"), "{rows:?}");
    assert!(all_stopped_in(&rows[3], "m2"), "{rows:?}");
}

#[test]
fn groups_need_not_be_adjacent() {
    let rt = run(stop_on(vec![
        grouped(50710, Some("m1")),
        grouped(50711, Some("m2")),
        grouped(50711, Some("m1")),
    ]));
    let rows = entry_rows(&rt);
    assert_eq!(rows[1], vec![("D_Passes".to_string(), Outcome::Pass)], "{rows:?}");
    assert!(all_stopped_in(&rows[2], "m1"), "{rows:?}");
}

#[test]
fn a_group_without_a_failure_runs_its_entries() {
    let rt = run(stop_on(vec![
        grouped(50710, Some("m2")),
        grouped(50711, Some("m1")),
        grouped(50711, Some("m1")),
    ]));
    let rows = entry_rows(&rt);
    assert!(all_pass(&rows[1]), "{rows:?}");
    assert!(all_pass(&rows[2]), "{rows:?}");
}

#[test]
fn the_rest_of_the_failing_codeunit_names_its_group() {
    let rt = run(stop_on(m1_m2_entries()));
    let rows = entry_rows(&rt);
    assert!(stopped_in(&rows[0][2].1, "m1"), "C_Passes of the first entry: {rows:?}");
    let marks: Vec<Vec<bool>> = rt
        .entries
        .iter()
        .map(|r| rt.results()[r.clone()].iter().map(|t| t.skipped_by_option).collect())
        .collect();
    assert_eq!(marks[0], [false, false, true], "{marks:?}");
    assert_eq!(marks[1], [true], "{marks:?}");
    assert_eq!(marks[2], [false, false, true], "{marks:?}");
    assert_eq!(marks[3], [true], "{marks:?}");
}

/// An entry without a group ignores the stop, also inside its own codeunit: the test after the failing one runs.
#[test]
fn an_entry_without_a_group_ignores_the_stop() {
    let rt = run(stop_on(vec![
        grouped(50710, None),
        grouped(50711, Some("m1")),
        grouped(50711, None),
        grouped(50710, Some("m1")),
    ]));
    let rows = entry_rows(&rt);
    assert_eq!(rows.len(), 4, "{rows:?}");
    assert_eq!(rows[0][0], ("A_Passes".to_string(), Outcome::Pass), "{rows:?}");
    assert!(matches!(&rows[0][1].1, Outcome::Fail(_)), "{rows:?}");
    assert_eq!(
        rows[0][2],
        ("C_Passes".to_string(), Outcome::Pass),
        "C runs after the ungrouped failure: {rows:?}"
    );
    assert!(all_pass(&rows[1]), "{rows:?}");
    assert!(all_pass(&rows[2]), "{rows:?}");
    assert!(matches!(&rows[3][1].1, Outcome::Fail(_)), "{rows:?}");
    assert!(stopped_in(&rows[3][2].1, "m1"), "{rows:?}");
}

#[test]
fn groups_without_the_stop_change_nothing() {
    let rt = run(RunOptions {
        entries: m1_m2_entries(),
        ..RunOptions::default()
    });
    let rows = entry_rows(&rt);
    assert_eq!(rows[0][2], ("C_Passes".to_string(), Outcome::Pass), "{rows:?}");
    assert!(all_pass(&rows[1]) && all_pass(&rows[3]), "{rows:?}");
    assert_eq!(rows[2][2], ("C_Passes".to_string(), Outcome::Pass), "{rows:?}");
}

#[test]
fn a_grouped_failure_leaves_the_ungrouped_entry_and_other_groups_running() {
    let rt = run(stop_on(vec![
        grouped(50710, Some("m1")),
        grouped(50711, None),
        grouped(50711, Some("m2")),
        grouped(50711, Some("m1")),
    ]));
    let rows = entry_rows(&rt);
    assert!(all_pass(&rows[1]), "{rows:?}");
    assert!(all_pass(&rows[2]), "{rows:?}");
    assert!(all_stopped_in(&rows[3], "m1"), "{rows:?}");
}
