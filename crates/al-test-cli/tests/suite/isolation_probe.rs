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
    // The app has 49 test methods per runner since writer SD's runners 61304-61320 (corpus PR 735, al-runner issues
    // 5607/5609); BC's isolation oracle covers the eleven of the first probe codeunits.
    assert_eq!(
        rt.cases_in(suite),
        49,
        "the probe app runs 49 methods under {suite} isolation"
    );
    let bc = real_bc(file);
    assert_eq!(bc.len(), 11, "the oracle has eleven methods");
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

/// StartSession under the runners' TestIsolation (SdIsolStartSession.Codeunit.al:22-30, measured on BC 27.5, 28.5 and
/// 29.0): under Disabled (61304) and no property (61305) S1-S3 pass; under Function (61300) and Codeunit (61301) S2 and S3
/// fail. Al-runner issue 5609.
#[test]
fn start_session_ends_as_measured_under_every_runner() {
    let dir = root().join("corpus/tests/al-language-isolation-probe");
    let app = runtime::load_app(&dir).unwrap();
    let runners = [
        ("function", 61300),
        ("codeunit", 61301),
        ("disabled", 61304),
        ("none", 61305),
    ];
    let suites: Vec<SuiteRun> = runners
        .iter()
        .map(|(name, id)| SuiteRun {
            name: (*name).into(),
            app: Some(0),
            runner: Some(*id),
        })
        .collect();
    let rt = CorpusRuntime::run_suites(
        vec![app],
        vec![al_sema::AppKind::Workspace],
        &[],
        &Default::default(),
        None,
        Default::default(),
        &suites,
    );
    let case = |name: &str| CaseId {
        codeunit: 61318,
        name: name.into(),
    };
    for (suite, _) in runners {
        let starts = matches!(suite, "disabled" | "none");
        assert_eq!(
            rt.outcome_in(suite, &case("S1_RemovesTheSessionRow")),
            Outcome::Pass,
            "{suite}"
        );
        for t in ["S2_StartSessionInATest", "S3_NextTestSeesTheSessionRow"] {
            let o = rt.outcome_in(suite, &case(t));
            if starts {
                assert_eq!(o, Outcome::Pass, "{suite} {t}");
            } else {
                assert!(matches!(o, Outcome::Fail(_)), "{suite} {t}: {o:?}");
            }
        }
    }
}

/// The isolation probe app (pinned since corpus bump 72-78) under all four of its runners, against the JUnit files corpus
/// writer SD measured on Windows containers. Local only, until the nightly measures runners 61304 and 61305:
/// `AL_ISOLATION_PROBE_APP=<corpus-sd>/tests/al-language-isolation-probe AL_ISOLATION_PROBE_ORACLE=<dir of
/// junit-isolation-<version>-<runner>.xml> [AL_ISOLATION_PROBE_VERSION=28.5] cargo test -p al-test-cli --test
/// isolation_probe -- --ignored --nocapture`. A case real BC passed must pass and one it failed must fail; a case not run
/// is listed.
#[test]
#[ignore]
fn the_pr_735_probe_under_every_runner_ends_as_measured() {
    let (Some(app), Some(oracle)) = (
        std::env::var_os("AL_ISOLATION_PROBE_APP"),
        std::env::var_os("AL_ISOLATION_PROBE_ORACLE"),
    ) else {
        panic!("set AL_ISOLATION_PROBE_APP and AL_ISOLATION_PROBE_ORACLE");
    };
    let version = std::env::var("AL_ISOLATION_PROBE_VERSION").unwrap_or_else(|_| "28.5".into());
    let runners = [
        ("function", 61300),
        ("codeunit", 61301),
        ("disabled", 61304),
        ("none", 61305),
    ];
    let app = runtime::load_app(Path::new(&app)).unwrap();
    let suites: Vec<SuiteRun> = runners
        .iter()
        .map(|(name, id)| SuiteRun {
            name: (*name).into(),
            app: Some(0),
            runner: Some(*id),
        })
        .collect();
    let rt = CorpusRuntime::run_suites(
        vec![app],
        vec![al_sema::AppKind::Workspace],
        &[],
        &Default::default(),
        None,
        Default::default(),
        &suites,
    );
    // Known, outside this runner work: under TestIsolation = Codeunit (the default of this runtime, unchanged) a
    // NumberSequence.Insert of S3 is gone in S8 on BC, after S4 failed; this runtime keeps it. When BC drops it (S4's
    // failure, or a test end) is not measured.
    let known = [
        ("codeunit", "S8_NumberSequenceStillExists"),
        ("codeunit", "S9_NumberSequenceNotReset"),
    ];
    let mut wrong = Vec::new();
    for (name, id) in runners {
        let file = Path::new(&oracle).join(format!("junit-isolation-{version}-{id}.xml"));
        let bc = oracle::parse_junit(&std::fs::read_to_string(&file).unwrap()).unwrap();
        let (mut same, mut not_run) = (0, Vec::new());
        for c in &bc {
            match (c.bc_failed, rt.outcome_in(name, &c.id)) {
                (false, Outcome::Pass) | (true, Outcome::Fail(_)) => same += 1,
                _ if known.contains(&(name, c.id.name.as_str())) => println!("  known difference: {}", c.id.name),
                (_, Outcome::NotRun(why)) => not_run.push(format!("{} {}: {why}", c.id.codeunit, c.id.name)),
                (bc_failed, ours) => wrong.push(format!(
                    "{name} {} {}: BC {}, runtime {ours:?}",
                    c.id.codeunit,
                    c.id.name,
                    if bc_failed { "failed" } else { "passed" }
                )),
            }
        }
        println!(
            "runner {id} ({name}): {same} of {} as on BC, {} not run",
            bc.len(),
            not_run.len()
        );
        for n in not_run {
            println!("  not run: {}", n.chars().take(200).collect::<String>());
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}
