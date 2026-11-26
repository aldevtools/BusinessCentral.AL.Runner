//! The clock a test session reads: the run's clock, given to every codeunit of the run (ADR-045).
use super::*;
use al_sema::workspace::AppSource;

const TODAY: &str = "codeunit 60160 T
{
    Subtype = Test;

    [Test]
    procedure TodayIsTheDayTheOracleRan()
    begin
        if Format(Today, 0, 9) <> '2026-10-15' then Error('Today %1', Format(Today, 0, 9));
        if Format(DT2Date(CurrentDateTime), 0, 9) <> '2026-10-15' then Error('CurrentDateTime %1', Format(CurrentDateTime, 0, 9));
        if Format(WorkDate, 0, 9) <> '2026-10-15' then Error('WorkDate %1', Format(WorkDate, 0, 9));
    end;
}";

// The corpus oracle recorded values derived from Today on the day real BC ran (22:00 UTC on 2026-10-15); a run on
// another day gave "expected 10/15/26 but was 10/16/26". The run clock of a codeunit starts at the oracle's own
// timestamp of that codeunit's suite.
#[test]
fn a_codeunit_run_with_a_clock_start_sees_that_day_as_today() {
    let (ws, l) = al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), TODAY.into())],
        }],
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let engine = Engine::new(l.program.clone(), Vec::new().into_iter().collect());
    let oi = l.program.objects.iter().position(|o| o.id == Some(60160)).unwrap();
    // 2026-10-15T22:00:23Z
    let mut scratch = Scratch::new(crate::host::RunClock::At(1_792_101_623_000));
    let done = run_codeunit_core(
        &engine,
        &l,
        oi,
        None,
        TestIsolation::Codeunit,
        None,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut Vec::new(),
        &mut scratch,
        None,
        false,
    );
    assert_eq!(done, vec![("TodayIsTheDayTheOracleRan".to_string(), Outcome::Pass)]);
}

// One run reads one clock (ADR-045, amendment of 2026-11-22): the install baseline and the test sessions over it. A
// caller that makes the baseline at the run's clock and then runs a codeunit over it must give that codeunit the same
// clock, not a fresh one begun when the codeunit runs.
#[test]
fn a_codeunit_run_over_a_baseline_reads_the_clock_the_baseline_was_made_on() {
    let dep = "table 60165 Stamped { fields { field(1; K; Code[10]) { } } }
        codeunit 60166 Inst { Subtype = Install;
            trigger OnInstallAppPerCompany() var S: Record Stamped; begin S.K := 'A'; S.Insert(); end; }";
    let main = "codeunit 60167 T { Subtype = Test;
        [Test] procedure TodayIsTheDayTheRowWasStamped() var S: Record Stamped;
        begin
            S.Get('A');
            if DT2Date(S.SystemCreatedAt) <> Today then
                Error('stamped %1, today %2', Format(DT2Date(S.SystemCreatedAt), 0, 9), Format(Today, 0, 9));
        end; }";
    let (ws, l) = al_sema::lower::compile_apps_with(
        vec![
            AppSource {
                id: "dep".into(),
                name: "dep".into(),
                dependencies: vec![],
                files: vec![("dep.al".into(), dep.into())],
            },
            AppSource {
                id: "main".into(),
                name: "main".into(),
                dependencies: vec!["dep".into()],
                files: vec![("main.al".into(), main.into())],
            },
        ],
        vec![al_sema::AppKind::Dependency, al_sema::AppKind::Workspace],
        &Default::default(),
        None,
        al_sema::check::Roots {
            baseline: true,
            ..Default::default()
        },
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let engine = Engine::new(l.program.clone(), Vec::new());
    // 2026-10-15T12:34:56.789Z: a day that is not the day the test runs.
    let clock = crate::host::RunClock::At(1_792_067_696_789);
    let base = crate::baseline::run(&engine, &l, None, clock).expect("the dependency app has install code");
    let oi = l.program.objects.iter().position(|o| o.name == "T").unwrap();
    let done = run_codeunit_full(
        &engine,
        &l,
        oi,
        Some(&base.baseline),
        TestIsolation::Codeunit,
        None,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut Vec::new(),
        None,
        clock,
    );
    assert_eq!(done, vec![("TodayIsTheDayTheRowWasStamped".to_string(), Outcome::Pass)]);
}
