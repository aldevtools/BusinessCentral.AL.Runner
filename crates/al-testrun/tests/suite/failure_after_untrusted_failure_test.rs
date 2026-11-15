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

codeunit 50433 "Untrusted Chain"
{
    Subtype = Test;
    var
        Ready: Boolean;
        Done: Boolean;
        Third: Boolean;
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
    procedure C_ReadsDoneThenSetsThird()
    begin
        if not Done then Error('done lost');
        Third := true;
    end;
    [Test]
    procedure D_ReadsThird()
    begin
        if not Third then Error('third lost');
    end;
}

codeunit 50434 "Untrusted Then Random"
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
    procedure B_DrawsThenChecksReady()
    var
        I: Integer;
    begin
        Randomize(5);
        I := Random(10);
        if not Ready then Error('not ready');
    end;
    [Test]
    procedure C_DrawsAndFails()
    var
        I: Integer;
    begin
        I := Random(10);
        Error('real failure');
    end;
}

codeunit 50438 "Untrusted Publisher"
{
    [IntegrationEvent(false, false)]
    procedure OnThing()
    begin
    end;
}

codeunit 50439 "Untrusted Subscriber"
{
    EventSubscriberInstance = Manual;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Untrusted Publisher", 'OnThing', '', false, false)]
    local procedure H()
    begin
    end;
}

codeunit 50435 "Untrusted Then Binding"
{
    Subtype = Test;
    var
        Ready: Boolean;
        Sub: Codeunit "Untrusted Subscriber";
    [Test]
    procedure A_RefusedBeforeItSetsReady()
    begin
        Sleep(-1);
        Ready := true;
    end;
    [Test]
    procedure B_BindsThenChecksReady()
    begin
        BindSubscription(Sub);
        if not Ready then Error('not ready');
        UnbindSubscription(Sub);
    end;
    [Test]
    procedure C_Fails()
    begin
        Error('real failure');
    end;
}

codeunit 50445 "Untrusted Noop"
{
    trigger OnRun()
    begin
    end;
}

codeunit 50436 "Untrusted Unknown Reach"
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
    procedure B_ChecksReadyThenRunsById()
    var
        Id: Integer;
    begin
        if not Ready then Error('not ready');
        Id := 50445;
        Codeunit.Run(Id);
    end;
    [Test]
    procedure C_Fails()
    begin
        Error('real failure');
    end;
}

codeunit 50437 "Untrusted Then Timeout"
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
    procedure C_WaitsForDone()
    begin
        if not Done then Sleep(5000);
    end;
    [Test]
    procedure D_WaitsForNothing()
    var
        I: Integer;
    begin
        if I = 0 then Sleep(5000);
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

/// A chain of two failures judged not trusted: each reason names the test before it once, with that test's own error, and
/// does not repeat that test's reason.
#[test]
fn a_chain_of_untrusted_failures_names_each_earlier_failure_once_with_its_own_error() {
    let rt = run();
    let c = outcome(&rt, 50433, "C_ReadsDoneThenSetsThird");
    assert!(
        matches!(&c, Outcome::NotRun(m) if m.starts_with(
            "an earlier test of this codeunit failed and its failure is not trusted (B_ChecksReadyThenSetsDone: failed, not trusted (not ready)): "
        ) && m.ends_with("; this failure is not trusted: done lost")),
        "{c:?}"
    );
    let d = outcome(&rt, 50433, "D_ReadsThird");
    assert!(
        matches!(&d, Outcome::NotRun(m) if m.starts_with(
            "an earlier test of this codeunit failed and its failure is not trusted (C_ReadsDoneThenSetsThird: failed, not trusted (done lost)): "
        ) && m.contains("Third") && !m.contains("B_ChecksReadyThenSetsDone")),
        "{d:?}"
    );
}

/// A later test that draws from the `Random` sequence after an untrusted failure that could draw: its failure is not
/// trusted (the untrusted test counts with its whole reach, which includes the draws).
#[test]
fn a_draw_after_an_untrusted_failure_that_draws_is_not_trusted() {
    let rt = run();
    let b = outcome(&rt, 50434, "B_DrawsThenChecksReady");
    assert!(not_trusted(&b), "{b:?}");
    let c = outcome(&rt, 50434, "C_DrawsAndFails");
    assert!(
        not_trusted(&c)
            && matches!(&c, Outcome::NotRun(m) if m.contains("B_DrawsThenChecksReady") && m.contains("Random")),
        "{c:?}"
    );
}

/// An untrusted failure that left a manual binding the later test starts with: the later failure is not trusted, by the
/// binding reason.
#[test]
fn a_binding_left_by_an_untrusted_failure_makes_a_later_failure_not_trusted() {
    let rt = run();
    let b = outcome(&rt, 50435, "B_BindsThenChecksReady");
    assert!(not_trusted(&b), "{b:?}");
    let c = outcome(&rt, 50435, "C_Fails");
    assert!(
        not_trusted(&c)
            && matches!(&c, Outcome::NotRun(m) if m.contains("B_BindsThenChecksReady") && m.contains("manual event subscription")),
        "{c:?}"
    );
}

/// An untrusted failure whose reach is not fully known: every later failure of the codeunit is not trusted.
#[test]
fn an_untrusted_failure_with_an_unknown_reach_makes_every_later_failure_not_trusted() {
    let rt = run();
    let b = outcome(&rt, 50436, "B_ChecksReadyThenRunsById");
    assert!(not_trusted(&b), "{b:?}");
    let c = outcome(&rt, 50436, "C_Fails");
    assert!(
        not_trusted(&c) && matches!(&c, Outcome::NotRun(m) if m.contains("part of its reach is not known")),
        "{c:?}"
    );
}

const LIMIT: &str = "the test used more than the time limit of 1 s";

/// A later timeout that reads what the untrusted failure would have set is not trusted; one that reads nothing it can
/// change stays a plain timeout.
#[test]
fn a_timeout_reading_what_an_untrusted_failure_would_have_set_is_not_trusted() {
    let rt = run_under(RunOptions {
        test_time_limit: Some(std::time::Duration::from_secs(1)),
        ..RunOptions::default()
    });
    let c = outcome(&rt, 50437, "C_WaitsForDone");
    assert!(
        matches!(&c, Outcome::NotRun(m) if m.contains("B_ChecksReadyThenSetsDone")
            && m.contains(&format!("this timeout is not trusted: {LIMIT}"))),
        "{c:?}"
    );
    let d = outcome(&rt, 50437, "D_WaitsForNothing");
    assert!(matches!(&d, Outcome::NotRun(m) if m.starts_with(LIMIT)), "{d:?}");
}
