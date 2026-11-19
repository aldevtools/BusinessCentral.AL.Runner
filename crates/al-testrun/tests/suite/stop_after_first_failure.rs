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
    CorpusRuntime::run_suites_with(
        vec![AppSource {
            id: "main".into(),
            name: "main".into(),
            dependencies: vec![],
            files: vec![("main.al".into(), SRC.into())],
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
