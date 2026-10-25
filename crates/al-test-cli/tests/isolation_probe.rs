//! The isolation probe (corpus tests/al-language-isolation-probe) against what real BC did.
//!
//! The probe only runs in the corpus's Windows nightly, under two custom test runners that differ
//! in `TestIsolation` alone; `oracle/28/xunit-isolation-*.xml` are those two runs. This test runs
//! the same app under the same two runners and compares every method with BC: the outcome, and
//! for a failure the exact message. It needs the `corpus` submodule and no dependency app.

use al_test_cli::oracle::{self, CaseId};
use al_test_cli::report::Outcome;
use al_test_cli::runtime::{self, CorpusRuntime, SuiteRun};
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn run() -> CorpusRuntime {
    let dir = root().join("corpus/tests/al-language-isolation-probe");
    assert!(dir.join("app.json").exists(), "corpus submodule not checked out");
    let app = runtime::load_app(&dir).unwrap();
    let run = |name: &str, runner| SuiteRun {
        name: name.into(),
        app: Some(0),
        runner: Some(runner),
    };
    CorpusRuntime::run_suites(
        vec![app],
        vec![al_sema::AppKind::Workspace],
        &[],
        &Default::default(),
        None,
        Default::default(),
        &[run("function", 61300), run("codeunit", 61301)],
    )
}

fn real_bc(file: &str) -> Vec<oracle::XunitCase> {
    let text = std::fs::read_to_string(root().join("oracle/28").join(file)).unwrap();
    oracle::parse_xunit(&text).unwrap()
}

fn agree(rt: &CorpusRuntime, suite: &str, file: &str) {
    let bc = real_bc(file);
    assert_eq!(bc.len(), 11, "the probe has eleven methods");
    for c in &bc {
        let ours = rt.outcome_in(suite, &c.case.id);
        match (&c.message, &ours) {
            (None, Outcome::Pass) => {}
            // BC's text is the message of the AL `Error` the fixture raises; ours is the same.
            (Some(m), Outcome::Fail(f)) => assert_eq!(f, m, "{} {}", c.case.id.codeunit, c.case.id.name),
            _ => panic!("{suite} {}: BC {:?}, runtime {ours:?}", c.case.id.name, c.message),
        }
    }
}

#[test]
fn under_function_isolation_every_method_ends_as_in_bc() {
    // T8 and T10 fail with the deferred duplicate-key error, the rest pass
    // (IsolProbeFixture.Codeunit.al:118-160).
    agree(&run(), "function", "xunit-isolation-function.xml");
}

#[test]
fn under_codeunit_isolation_t5_fails_with_bcs_message_and_every_method_ends_as_in_bc() {
    // Also T8, T10 (the deferred duplicate-key error) and T11 (the committed row of T10 stays).
    let rt = run();
    agree(&rt, "codeunit", "xunit-isolation-codeunit.xml");
    let t5 = rt.outcome_in(
        "codeunit",
        &CaseId {
            codeunit: 61302,
            name: "T5_RowInsertedByT1IsRolledBack".into(),
        },
    );
    assert!(matches!(t5, Outcome::Fail(_)), "{t5:?}");
}
