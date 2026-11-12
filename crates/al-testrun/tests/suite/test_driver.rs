//! ADR-074 (owner decision 2026-11-12, option 1): the test driver. `bc-linux`, the default, runs a test codeunit's OnRun
//! once; `bccontainerhelper` first runs BcContainerHelper's discovery pass (the OnRun of every test codeunit, without
//! test isolation, what it writes staying), then OnRun again inside the runner's isolation. Pinned by corpus
//! tests/al-language-isolation-probe/src/SwOnRunRunner.al:19-26 (Windows nightly run 43058375098 and BC 27.5, 28.5,
//! 29.0); the corpus case itself (`OnRunWrites_DiscoveryRowAndRollback`) is the red/green test, run with
//! `al-runner test --test-driver bccontainerhelper --strict-order`. These tests pin the switch and the refusals.

use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun, TestDriver};

const LOG: &str = r#"
table 50700 "Log" { fields { field(1; N; Integer) { } } keys { key(PK; N) { Clustered = true; } } }
codeunit 50701 "Writes"
{
    Subtype = Test;
    trigger OnRun()
    var
        L: Record "Log";
    begin
        L.N := L.Count() + 1;
        L.Insert();
    end;

    [Test]
    procedure SeesTwoRows()
    var
        L: Record "Log";
    begin
        if L.Count() <> 2 then
            Error('rows %1', L.Count());
    end;
}
codeunit 50702 "Later"
{
    Subtype = Test;
    [Test]
    procedure SeesTheDiscoveryRowOnly()
    var
        L: Record "Log";
    begin
        if L.Count() <> 1 then
            Error('rows %1', L.Count());
    end;
}
"#;

fn run(src: &str, driver: Option<TestDriver>, strict: bool) -> Vec<(u32, String, Outcome)> {
    let opts = RunOptions {
        test_driver: driver.unwrap_or_default(),
        strict_order: strict,
        strict_sequential: strict,
        ..Default::default()
    };
    let rt = CorpusRuntime::run_suites_with(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("main.al".into(), src.to_string())],
        }],
        vec![al_sema::AppKind::Workspace],
        &[],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &opts,
    );
    let mut out: Vec<_> = rt
        .results()
        .iter()
        .map(|r| (r.codeunit, r.test.clone(), r.outcome.clone()))
        .collect();
    out.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
    out
}

fn outcome(out: &[(u32, String, Outcome)], test: &str) -> Outcome {
    out.iter()
        .find(|(_, t, _)| t == test)
        .map(|(_, _, o)| o.clone())
        .unwrap_or_else(|| panic!("{test} has no row: {out:?}"))
}

#[test]
fn bc_linux_is_the_default_and_runs_onrun_once() {
    let out = run(LOG, None, false);
    assert!(
        matches!(outcome(&out, "SeesTwoRows"), Outcome::Fail(ref m) if m == "rows 1"),
        "{out:?}"
    );
    assert_eq!(run(LOG, Some(TestDriver::BcLinux), false), out);
}

#[test]
fn bccontainerhelper_keeps_the_discovery_row_and_rolls_back_the_runs_row() {
    for strict in [false, true] {
        let out = run(LOG, Some(TestDriver::BcContainerHelper), strict);
        assert_eq!(outcome(&out, "SeesTwoRows"), Outcome::Pass, "strict {strict}: {out:?}");
        assert_eq!(
            outcome(&out, "SeesTheDiscoveryRowOnly"),
            Outcome::Pass,
            "strict {strict}: {out:?}"
        );
    }
}

#[test]
fn an_earlier_codeunit_that_reads_what_a_later_discovery_wrote_is_refused() {
    let src = format!(
        "{LOG}\ncodeunit 50699 \"Earlier\" {{ Subtype = Test; [Test] procedure ReadsLog() var L: Record \"Log\"; begin if L.IsEmpty() then; end; }}"
    );
    let out = run(&src, Some(TestDriver::BcContainerHelper), false);
    assert!(
        matches!(outcome(&out, "ReadsLog"), Outcome::NotRun(ref why) if why.contains("later codeunit 50701")),
        "{out:?}"
    );
    assert_eq!(outcome(&out, "SeesTwoRows"), Outcome::Pass, "{out:?}");
    // bc-linux has no discovery pass: the earlier codeunit runs.
    assert_eq!(outcome(&run(&src, None, false), "ReadsLog"), Outcome::Pass);
}

const RAISES: &str = r#"
codeunit 50710 "State"
{
    SingleInstance = true;
    var
        Calls: Integer;
    procedure Bump(): Integer begin Calls += 1; exit(Calls); end;
    procedure Get(): Integer begin exit(Calls); end;
}
codeunit 50711 "Raises First"
{
    Subtype = Test;
    var
        S: Codeunit "State";
    trigger OnRun()
    begin
        if S.Bump() = 1 then
            Error('only the first call raises');
    end;

    [Test]
    procedure NoRow()
    begin
    end;
}
codeunit 50712 "Reader"
{
    Subtype = Test;
    var
        S: Codeunit "State";
    [Test]
    procedure OnRunRanTwice()
    begin
        if S.Get() <> 2 then
            Error('calls %1', S.Get());
    end;
}
"#;

#[test]
fn a_codeunit_whose_onrun_raised_in_discovery_reports_no_rows_and_its_onrun_runs_again() {
    let out = run(RAISES, Some(TestDriver::BcContainerHelper), true);
    assert!(out.iter().all(|(id, _, _)| *id != 50711), "{out:?}");
    assert_eq!(outcome(&out, "OnRunRanTwice"), Outcome::Pass, "{out:?}");
}

#[test]
fn without_strict_order_a_test_that_reads_a_single_instance_the_pass_changed_is_refused() {
    let out = run(RAISES, Some(TestDriver::BcContainerHelper), false);
    assert!(
        matches!(outcome(&out, "OnRunRanTwice"), Outcome::NotRun(ref why) if why.contains("--strict-order")),
        "{out:?}"
    );
}
