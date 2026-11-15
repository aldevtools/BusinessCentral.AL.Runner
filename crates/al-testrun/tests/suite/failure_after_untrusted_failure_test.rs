//! A failure judged not trusted is not BC's answer: on BC that test may have passed and left its rows and globals. So a
//! later failure of the codeunit is not trusted when that test could have changed what the later test reads, as after a
//! test refused while it ran (its whole static reach counts). A later failure that reads nothing it can change stays a
//! failure, and a failure after a trusted failure keeps main's result.

use al_sema::workspace::AppSource;
use al_testrun::isolation::TestIsolation;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const SRC: &str = r#"
table 50440 "Untrusted Row" { fields { field(1; "No."; Integer) { } } keys { key(PK; "No.") { } } }
table 50441 "Untrusted Other" { fields { field(1; "No."; Integer) { } } keys { key(PK; "No.") { } } }

codeunit 50430 "Untrusted Then Row"
{
    Subtype = Test;
    var
        Ready: Boolean;
    [Test]
    procedure A_RefusedBeforeItSetsReady()
    begin
        Sleep(-1);
        Ready := true;
    end;
    [Test]
    procedure B_ChecksReadyThenInserts()
    var
        R: Record "Untrusted Row";
    begin
        if not Ready then Error('not ready');
        R."No." := 8;
        R.Insert();
    end;
    [Test]
    procedure C_ReadsTheRow()
    var
        R: Record "Untrusted Row";
    begin
        R.Get(8);
    end;
    [Test]
    procedure D_ReadsAnotherTableFails()
    var
        O: Record "Untrusted Other";
    begin
        if O.IsEmpty() then Error('real failure');
    end;
}

codeunit 50431 "Untrusted Then Global"
{
    Subtype = Test;
    var
        Ready: Boolean;
        Done: Boolean;
    [Test]
    procedure A_RefusedBeforeItSetsReady()
    begin
        Sleep(-1);
        Ready := true;
    end;
    [Test]
    procedure B_ChecksReadyThenSetsDone()
    begin
        if not Ready then Error('not ready');
        Done := true;
    end;
    [Test]
    procedure C_ReadsDone()
    begin
        if not Done then Error('done lost');
    end;
}

codeunit 50432 "Trusted Failure Then Row"
{
    Subtype = Test;
    [Test]
    procedure A_FailsBeforeItInserts()
    var
        R: Record "Untrusted Row";
    begin
        Error('real failure');
        R."No." := 9;
        R.Insert();
    end;
    [Test]
    procedure B_ReadsTheRow()
    var
        R: Record "Untrusted Row";
    begin
        R.Get(9);
    end;
}
"#;

fn run() -> CorpusRuntime {
    run_under(RunOptions::default())
}

fn run_under(opts: RunOptions) -> CorpusRuntime {
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

fn not_trusted(o: &Outcome) -> bool {
    matches!(o, Outcome::NotRun(m) if m.contains("this failure is not trusted"))
}

#[test]
fn a_failure_reading_a_row_an_untrusted_failure_would_have_inserted_is_not_trusted() {
    let rt = run();
    let a = outcome(&rt, 50430, "A_RefusedBeforeItSetsReady");
    assert!(matches!(&a, Outcome::NotRun(m) if m.contains("Sleep")), "{a:?}");
    let b = outcome(&rt, 50430, "B_ChecksReadyThenInserts");
    assert!(not_trusted(&b), "{b:?}");
    let c = outcome(&rt, 50430, "C_ReadsTheRow");
    assert!(
        not_trusted(&c)
            && matches!(&c, Outcome::NotRun(m) if m.contains("B_ChecksReadyThenInserts") && m.contains("Untrusted Row")),
        "{c:?}"
    );
}

#[test]
fn a_failure_reading_a_global_an_untrusted_failure_would_have_set_is_not_trusted() {
    let rt = run();
    let b = outcome(&rt, 50431, "B_ChecksReadyThenSetsDone");
    assert!(not_trusted(&b), "{b:?}");
    let c = outcome(&rt, 50431, "C_ReadsDone");
    assert!(
        not_trusted(&c)
            && matches!(&c, Outcome::NotRun(m) if m.contains("B_ChecksReadyThenSetsDone") && m.contains("Done")),
        "{c:?}"
    );
}

#[test]
fn a_failure_that_reads_nothing_the_untrusted_failure_can_change_stays_a_failure() {
    let rt = run();
    let d = outcome(&rt, 50430, "D_ReadsAnotherTableFails");
    assert_eq!(d, Outcome::Fail("real failure".into()));
}

#[test]
fn a_failure_after_a_trusted_failure_keeps_its_result() {
    let rt = run();
    let a = outcome(&rt, 50432, "A_FailsBeforeItInserts");
    assert_eq!(a, Outcome::Fail("real failure".into()));
    let b = outcome(&rt, 50432, "B_ReadsTheRow");
    assert!(matches!(&b, Outcome::Fail(m) if m.contains("does not exist")), "{b:?}");
}

/// Under Function isolation each test's rows are rolled back when it ends, also on BC, so a row the untrusted failure would
/// have inserted is not read by a later test: that later failure keeps main's result (the table part of the reach check
/// takes the isolation, as for a test refused while it ran).
#[test]
fn under_function_isolation_a_row_the_untrusted_failure_would_have_inserted_does_not_count() {
    let rt = run_under(RunOptions {
        isolation: Some(TestIsolation::Function),
        ..RunOptions::default()
    });
    let b = outcome(&rt, 50430, "B_ChecksReadyThenInserts");
    assert!(not_trusted(&b), "{b:?}");
    let c = outcome(&rt, 50430, "C_ReadsTheRow");
    assert!(matches!(&c, Outcome::Fail(m) if m.contains("does not exist")), "{c:?}");
}
