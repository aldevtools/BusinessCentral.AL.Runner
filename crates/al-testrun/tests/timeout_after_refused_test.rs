//! Al-runner issue 5776: a timeout after a test that did not run to its end is judged as a failure in the same place
//! is. The time limit stops a test that started from the state the earlier test left, so when that test could have
//! changed what the stopped test reads, the timeout is not trusted either; otherwise it stays a plain timeout.

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const SRC: &str = r#"
table 50510 "Written" { fields { field(1; "No."; Integer) { } } keys { key(PK; "No.") { } } }
table 50511 "Other" { fields { field(1; "No."; Integer) { } } keys { key(PK; "No.") { } } }

codeunit 50500 "MidRun Then Timeout"
{
    Subtype = Test;
    [Test]
    procedure A_RefusedMidRunWritesWritten()
    var
        W: Record "Written";
    begin
        Sleep(-1);
        W.Insert();
    end;
    [Test]
    procedure B_ReadsOtherTimesOut()
    var
        O: Record "Other";
    begin
        if O.IsEmpty() then Sleep(5000);
    end;
    [Test]
    procedure C_ReadsWrittenTimesOut()
    var
        W: Record "Written";
    begin
        if W.IsEmpty() then Sleep(5000);
    end;
    [Test]
    procedure D_ReadsWrittenFails()
    var
        W: Record "Written";
    begin
        if W.IsEmpty() then Error('boom');
    end;
}
"#;

fn run() -> CorpusRuntime {
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
            ..RunOptions::default()
        },
    )
}

fn outcome(rt: &CorpusRuntime, name: &str) -> Outcome {
    rt.first_outcome(&CaseId {
        codeunit: 50500,
        name: name.into(),
    })
}

const LIMIT: &str = "the test used more than the time limit of 1 s";

#[test]
fn a_timeout_reading_what_the_refused_test_can_write_is_not_trusted_like_a_failure() {
    let rt = run();
    let c = outcome(&rt, "C_ReadsWrittenTimesOut");
    assert!(
        matches!(&c, Outcome::NotRun(m) if m.contains("refused while it ran")
            && m.contains(&format!("this timeout is not trusted: {LIMIT}"))),
        "{c:?}"
    );
    // The failure in the same place, for comparison.
    let d = outcome(&rt, "D_ReadsWrittenFails");
    assert!(
        matches!(&d, Outcome::NotRun(m) if m.contains("this failure is not trusted")),
        "{d:?}"
    );
}

#[test]
fn a_timeout_the_refused_test_cannot_reach_stays_a_plain_timeout() {
    let rt = run();
    let b = outcome(&rt, "B_ReadsOtherTimesOut");
    assert!(matches!(&b, Outcome::NotRun(m) if m.starts_with(LIMIT)), "{b:?}");
}
