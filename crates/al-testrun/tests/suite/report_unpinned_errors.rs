//! al-runner issue 6487 (opt-in): a test that fails with an error whose text is not pinned is not run by default, because
//! a made-up text would stand in its result. With `RunOptions::report_unpinned_errors` (the command line sets it from
//! `ALRUNNER_REPORT_UNPINNED_ERRORS=1`) it is reported as failed, its message marked "[error text not pinned]".

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

// The issue's repro: CalcDate with a weekday of 0 raises an error whose text the corpus does not pin.
const SRC: &str = r#"
codeunit 50550 "T"
{
    Subtype = Test;
    [Test]
    procedure FailsWithUnpinnedText()
    begin
        if CalcDate('<WD0>', 20240101D) = 0D then;
    end;

    [Test]
    procedure FailsWithPinnedText()
    begin
        Error('Plain failure');
    end;
}
"#;

fn outcome(name: &str, report_unpinned_errors: bool) -> Outcome {
    let rt = CorpusRuntime::run_suites_with(
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
            report_unpinned_errors,
            ..Default::default()
        },
    );
    rt.first_outcome(&CaseId {
        codeunit: 50550,
        name: name.into(),
    })
}

#[test]
fn an_unpinned_error_is_not_run_by_default() {
    let o = outcome("FailsWithUnpinnedText", false);
    assert!(
        matches!(&o, Outcome::NotRun(w) if w.starts_with("The test result of an error whose text is not pinned")),
        "{o:?}"
    );
}

#[test]
fn an_unpinned_error_fails_marked_with_the_switch() {
    let o = outcome("FailsWithUnpinnedText", true);
    let Outcome::Fail(m) = &o else { panic!("{o:?}") };
    assert!(m.starts_with("[error text not pinned]"), "{m:?}");
    // No private-use mark of the unpinned text reaches the result.
    assert!(!m.contains(al_builtins::native::UNPINNED_ERROR), "{m:?}");
}

#[test]
fn a_pinned_error_is_unchanged_by_the_switch() {
    for on in [false, true] {
        assert_eq!(
            outcome("FailsWithPinnedText", on),
            Outcome::Fail("Plain failure".into())
        );
    }
}
