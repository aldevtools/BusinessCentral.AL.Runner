//! Al-runner issue 7517: `RunOptions::skip_after_timeout` reports the tests of a codeunit after one the time limit
//! stopped as not run, without running them. Default off: a later test still runs and its pass counts.

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const SRC: &str = r#"
codeunit 50700 "Skip After Timeout"
{
    Subtype = Test;
    [Test]
    procedure A_Quick()
    begin
    end;
    [Test]
    procedure B_TimesOut()
    begin
        Sleep(5000);
    end;
    [Test]
    procedure C_Quick()
    begin
    end;
    [Test]
    procedure D_TimesOutToo()
    begin
        Sleep(5000);
    end;
}
codeunit 50701 "Other Codeunit"
{
    Subtype = Test;
    [Test]
    procedure E_Quick()
    begin
    end;
}
"#;

fn run(skip: bool) -> CorpusRuntime {
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
        &RunOptions {
            test_time_limit: Some(std::time::Duration::from_secs(1)),
            skip_after_timeout: skip,
            ..RunOptions::default()
        },
    )
}

fn outcome(rt: &CorpusRuntime, codeunit: u32, name: &str) -> Outcome {
    rt.first_outcome(&CaseId {
        codeunit,
        name: name.into(),
    })
}

#[test]
fn by_default_the_tests_after_a_timeout_run() {
    let rt = run(false);
    assert!(
        matches!(outcome(&rt, 50700, "C_Quick"), Outcome::Pass),
        "{:?}",
        outcome(&rt, 50700, "C_Quick")
    );
}

#[test]
fn with_the_option_the_rest_of_the_codeunit_is_not_run_naming_the_stopped_test() {
    let rt = run(true);
    assert!(matches!(outcome(&rt, 50700, "A_Quick"), Outcome::Pass));
    for t in ["C_Quick", "D_TimesOutToo"] {
        let o = outcome(&rt, 50700, t);
        assert!(
            matches!(&o, Outcome::NotRun(m) if m.contains("B_TimesOut") && m.contains("--skip-after-timeout")),
            "{t}: {o:?}"
        );
    }
    // Another codeunit is not affected.
    assert!(matches!(outcome(&rt, 50701, "E_Quick"), Outcome::Pass));
}
