//! A test that ends while a duplicate-key `Insert` is still deferred (al-runner issues 5603, 5788). The corpus pins that
//! the statement form returns without an error and the next access to the table or a `Commit` raises it
//! (record/TestRecordRuntimeTextsAndViews.al:180-209). At the end of the test, real BC fails the test with the
//! 'already exists' error and an AL call stack without a test frame, and rolls the test back; a row committed before
//! stays (measured on Windows BC 29.0, corpus master ccb6e72f, tests/al-language-isolation-probe/src/
//! IsolProbeFixture.Codeunit.al T8-T11; the pin waits for the Windows nightly). A registered assumption until then:
//! only the measured shape (TestIsolation = Codeunit, the default TransactionModel) fails; any other is not run.

use al_testkit::{blocked, compile, outcomes_in};
use al_testrun::Outcome;
use al_testrun::isolation::TestIsolation;

const SRC: &str = r#"
table 69970 "DI Row"
{
    fields { field(1; "Key"; Code[20]) { } field(2; Value; Integer) { } }
    keys { key(PK; "Key") { Clustered = true; } }
}
codeunit 69971 T
{
    Subtype = Test;
    [Test]
    procedure EndsDeferred()
    var Row: Record "DI Row";
    begin Row.DeleteAll(); Row."Key" := 'K1'; Row.Insert(); Row."Key" := 'K1'; Row.Insert(); end;
    [Test]
    procedure SeesTheRollback()
    var Row: Record "DI Row";
    begin if not Row.IsEmpty() then Error('rows: %1', Row.Count()); end;
    [Test]
    procedure CommitsThenEndsDeferred()
    var Row: Record "DI Row";
    begin Row."Key" := 'KC'; Row.Insert(); Commit(); Row."Key" := 'KC'; Row.Insert(); end;
    [Test]
    procedure SeesTheCommittedRow()
    var Row: Record "DI Row";
    begin Row.Get('KC'); if Row.Count() <> 1 then Error('rows: %1', Row.Count()); end;
}
codeunit 69972 U
{
    Subtype = Test;
    [Test]
    procedure RaisedByTheNextRead()
    var Row: Record "DI Row";
    begin Row.DeleteAll(); Row."Key" := 'K2'; Row.Insert(); Row."Key" := 'K2'; Row.Insert(); if Row.Count() = 0 then; end;
    [Test]
    procedure GuardedReturnsFalse()
    var Row: Record "DI Row";
    begin Row.DeleteAll(); Row."Key" := 'K3'; Row.Insert(); Row."Key" := 'K3'; if Row.Insert() then Error('true'); end;
}
codeunit 69973 R
{
    Subtype = Test;
    [Test]
    [TransactionModel(TransactionModel::AutoRollback)]
    procedure EndsDeferredUnderAutoRollback()
    var Row: Record "DI Row";
    begin Row.DeleteAll(); Row."Key" := 'K4'; Row.Insert(); Row."Key" := 'K4'; Row.Insert(); end;
}
"#;

type Run = (
    Vec<(String, Outcome)>,
    Vec<(String, String)>,
    Vec<(String, Vec<al_testrun::assumptions::Assumption>)>,
);

fn run(codeunit: &str, isolation: TestIsolation) -> Run {
    let l = compile(SRC);
    let engine = al_vm::Engine::new(l.program.clone(), blocked(&l));
    let oi = l.program.objects.iter().position(|o| o.name == codeunit).unwrap();
    let (mut stacks, mut hits) = (Vec::new(), Vec::new());
    let out = al_testrun::runtime::run_codeunit_full(
        &engine,
        &l,
        oi,
        None,
        isolation,
        None,
        &mut stacks,
        &mut Vec::new(),
        &mut hits,
        None,
    );
    (out, stacks, hits)
}

fn of(out: &[(String, Outcome)], n: &str) -> Outcome {
    out.iter().find(|(t, _)| t == n).map(|(_, o)| o.clone()).unwrap()
}

#[test]
fn a_test_that_ends_with_a_deferred_duplicate_insert_fails_and_is_rolled_back() {
    let (out, stacks, hits) = run("T", TestIsolation::Codeunit);
    assert_eq!(
        of(&out, "EndsDeferred"),
        Outcome::Fail("The record in table DI Row already exists. Identification fields and values: Key='K1'".into())
    );
    assert_eq!(of(&out, "SeesTheRollback"), Outcome::Pass);
    assert_eq!(
        of(&out, "CommitsThenEndsDeferred"),
        Outcome::Fail("The record in table DI Row already exists. Identification fields and values: Key='KC'".into())
    );
    assert_eq!(of(&out, "SeesTheCommittedRow"), Outcome::Pass);
    // The error comes from the runner, not from the test method: no AL frame of the test.
    assert!(
        stacks
            .iter()
            .all(|(t, _)| t != "EndsDeferred" && t != "CommitsThenEndsDeferred"),
        "{stacks:?}"
    );
    let flagged = |n: &str| hits.iter().any(|(t, m)| t == n && !m.is_empty());
    assert!(
        flagged("EndsDeferred") && flagged("CommitsThenEndsDeferred"),
        "{hits:?}"
    );
}

#[test]
fn a_deferred_duplicate_insert_raised_inside_the_test_is_unchanged() {
    let out = outcomes_in(SRC, "U");
    match of(&out, "RaisedByTheNextRead") {
        Outcome::Fail(m) => assert!(m.contains("already exists"), "{m}"),
        other => panic!("{other:?}"),
    }
    assert_eq!(of(&out, "GuardedReturnsFalse"), Outcome::Pass);
}

#[test]
fn an_unmeasured_shape_that_ends_deferred_is_not_run() {
    let (out, _, _) = run("R", TestIsolation::Codeunit);
    match of(&out, "EndsDeferredUnderAutoRollback") {
        Outcome::NotRun(r) => assert!(r.contains("deferred"), "{r}"),
        other => panic!("{other:?}"),
    }
    let (out, _, _) = run("T", TestIsolation::Function);
    match of(&out, "EndsDeferred") {
        Outcome::NotRun(r) => assert!(r.contains("deferred"), "{r}"),
        other => panic!("{other:?}"),
    }
}
