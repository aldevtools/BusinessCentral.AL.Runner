//! `FailureStop` (al-runner issue 8210): a run that stops at its first failing test. No test starts after the failure;
//! every selected test not reached is `NotRun` with the stop's reason; a test that is not run, a failure of a test the
//! selection does not count and a failure whose result is not final do not trip it. T2, T3, T6 and T7 are guards: they
//! pass without the stop's implementation, because the stop is off or does not apply there.

use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::events::Hooks;
use al_testrun::failure_stop::{FailureStop, REASON_GROUP, REASON_REQUEST, StopScope};
use al_testrun::runtime::{CorpusRuntime, EntryRun, Manifest, RunOptions, SuiteRun};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

fn manifest() -> Manifest {
    Manifest {
        publisher: "Me".into(),
        version: "1.0.0.0".into(),
        resources: vec![],
        include_source: false,
        target: String::new(),
        logo: None,
        description: None,
        layout_files: Vec::new(),
        id_ranges: Vec::new(),
        translations: false,
        package_data: false,
        package_data_files: Vec::new(),
        translation_files: Vec::new(),
        translation_source: None,
        runtime: None,
        app_json_keys: Vec::new(),
        platform_internals_visible_to: Vec::new(),
        symbols: vec![],
    }
}

fn run(src: &str, opts: &RunOptions) -> CorpusRuntime {
    CorpusRuntime::try_run_suites_with(
        vec![AppSource {
            id: "main".into(),
            name: "main".into(),
            dependencies: vec![],
            files: vec![("main.al".into(), src.to_string())],
        }],
        vec![al_sema::AppKind::Workspace],
        &[manifest()],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        opts,
    )
    .expect("the run starts")
}

/// T1's fixture: the second test fails.
const FOUR: &str = "codeunit 60130 Four { Subtype = Test;
    [Test] procedure Passes() begin end;
    [Test] procedure Fails() begin Error('boom'); end;
    [Test] procedure ThenX() begin end;
    [Test] procedure ThenY() begin end; }";

/// The issue's shape: the first test fails, the rest would pass.
const ISSUE: &str = "codeunit 50110 Slow { Subtype = Test;
    [Test] procedure FailsFirst() begin Error('killed'); end;
    [Test] procedure SlowTwo() begin end;
    [Test] procedure SlowThree() begin end;
    [Test] procedure SlowFour() begin end; }";

fn counting_start() -> (Arc<AtomicUsize>, Hooks) {
    let starts = Arc::new(AtomicUsize::new(0));
    let seen = starts.clone();
    let hooks = Hooks {
        on_start: Some(Arc::new(move |_, _| {
            seen.fetch_add(1, Ordering::SeqCst);
        })),
        ..Default::default()
    };
    (starts, hooks)
}

fn with_stop(stop: &FailureStop, mut hooks: Hooks) -> Hooks {
    hooks.stop_at_failure = Some(stop.clone());
    hooks
}

fn outcomes(rt: &CorpusRuntime) -> Vec<Outcome> {
    rt.results().iter().map(|t| t.outcome.clone()).collect()
}

fn per_entry(rt: &CorpusRuntime) -> Vec<Vec<Outcome>> {
    rt.entries
        .iter()
        .map(|r| rt.results()[r.clone()].iter().map(|t| t.outcome.clone()).collect())
        .collect()
}

fn not_run(why: &str) -> Outcome {
    Outcome::NotRun(why.to_string())
}

fn is_fail(o: &Outcome) -> bool {
    matches!(o, Outcome::Fail(_))
}

fn entry_with(hooks: Hooks) -> EntryRun {
    EntryRun {
        hooks,
        ..Default::default()
    }
}

#[test]
fn a_failure_stops_the_rest_of_its_codeunit() {
    let (starts, hooks) = counting_start();
    let stop = FailureStop::new(StopScope::Request);
    let rt = run(
        FOUR,
        &RunOptions {
            hooks: with_stop(&stop, hooks),
            ..Default::default()
        },
    );
    let got = outcomes(&rt);
    assert_eq!(got.len(), 4, "{got:?}");
    assert!(matches!(got[0], Outcome::Pass), "{got:?}");
    assert!(is_fail(&got[1]), "{got:?}");
    assert_eq!(got[2..], [not_run(REASON_REQUEST), not_run(REASON_REQUEST)], "{got:?}");
    assert_eq!(starts.load(Ordering::SeqCst), 2, "{got:?}");
}

#[test]
fn without_a_stop_every_test_runs() {
    let (starts, hooks) = counting_start();
    let rt = run(
        FOUR,
        &RunOptions {
            hooks,
            ..Default::default()
        },
    );
    let got = outcomes(&rt);
    assert_eq!(got.len(), 4, "{got:?}");
    assert!(got.iter().all(|o| !matches!(o, Outcome::NotRun(_))), "{got:?}");
    assert_eq!(starts.load(Ordering::SeqCst), 4, "{got:?}");
}

#[test]
fn a_test_that_is_not_run_does_not_trip_the_stop() {
    let src = "codeunit 60131 Refused { Subtype = Test;
        [Test] procedure Refuses() var T: Text; begin T := ApplicationPath(); end;
        [Test] procedure ThenX() begin end;
        [Test] procedure ThenY() begin end; }";
    let stop = FailureStop::new(StopScope::Request);
    let rt = run(
        src,
        &RunOptions {
            hooks: with_stop(&stop, Hooks::default()),
            ..Default::default()
        },
    );
    let got = outcomes(&rt);
    assert_eq!(got.len(), 3, "{got:?}");
    assert!(matches!(got[0], Outcome::NotRun(_)), "{got:?}");
    assert!(
        matches!(got[1], Outcome::Pass) && matches!(got[2], Outcome::Pass),
        "{got:?}"
    );
    assert!(!stop.is_set(), "a test that is not run tripped the stop");
}

#[test]
fn a_stop_shared_by_entries_skips_the_later_entries() {
    let (starts, hooks) = counting_start();
    let stop = FailureStop::new(StopScope::Request);
    let entries = (0..3).map(|_| entry_with(with_stop(&stop, hooks.clone()))).collect();
    let rt = run(
        ISSUE,
        &RunOptions {
            entries,
            ..Default::default()
        },
    );
    let got = per_entry(&rt);
    assert_eq!(got.len(), 3, "{got:?}");
    assert!(is_fail(&got[0][0]), "{got:?}");
    assert_eq!(got[0][1..], vec![not_run(REASON_REQUEST); 3][..], "{got:?}");
    for later in &got[1..] {
        assert_eq!(*later, vec![not_run(REASON_REQUEST); 4], "{got:?}");
    }
    assert_eq!(starts.load(Ordering::SeqCst), 1, "{got:?}");
}

#[test]
fn a_group_stop_ends_only_its_group() {
    let g1 = FailureStop::new(StopScope::Group);
    let g2 = FailureStop::new(StopScope::Group);
    let entries = [&g1, &g1, &g2]
        .into_iter()
        .map(|s| entry_with(with_stop(s, Hooks::default())))
        .collect();
    let rt = run(
        ISSUE,
        &RunOptions {
            entries,
            ..Default::default()
        },
    );
    let got = per_entry(&rt);
    assert_eq!(got.len(), 3, "{got:?}");
    assert!(is_fail(&got[0][0]), "{got:?}");
    assert_eq!(got[0][1..], vec![not_run(REASON_GROUP); 3][..], "{got:?}");
    assert_eq!(got[1], vec![not_run(REASON_GROUP); 4], "{got:?}");
    assert!(is_fail(&got[2][0]), "{got:?}");
    assert_eq!(got[2][1..], vec![not_run(REASON_GROUP); 3][..], "{got:?}");
}

#[test]
fn a_failure_the_selection_does_not_count_does_not_trip() {
    let stop = FailureStop::new(StopScope::Request).counting(Arc::new(|_, test| test == "SlowTwo"));
    let rt = run(
        ISSUE,
        &RunOptions {
            hooks: with_stop(&stop, Hooks::default()),
            ..Default::default()
        },
    );
    let got = outcomes(&rt);
    assert_eq!(got.len(), 4, "{got:?}");
    assert!(is_fail(&got[0]), "{got:?}");
    assert!(got[1..].iter().all(|o| matches!(o, Outcome::Pass)), "{got:?}");
    assert!(!stop.is_set(), "{got:?}");
}

#[test]
fn under_strict_order_there_is_no_stop() {
    let stop = FailureStop::new(StopScope::Request);
    let rt = run(
        FOUR,
        &RunOptions {
            strict_order: true,
            hooks: with_stop(&stop, Hooks::default()),
            ..Default::default()
        },
    );
    let got = outcomes(&rt);
    assert_eq!(got.len(), 4, "{got:?}");
    assert!(got.iter().all(|o| !matches!(o, Outcome::NotRun(_))), "{got:?}");
    assert!(!stop.is_set(), "{got:?}");
}

/// A test codeunit that runs under Disabled isolation (its `RequiredTestIsolation` on the hub path) has a result that
/// may be replaced afterwards: its own failure does not trip the stop, and once the stop is tripped it does not start.
#[test]
fn a_codeunit_under_disabled_isolation_does_not_trip_and_is_skipped_once_tripped() {
    let src = "codeunit 60140 Plain { Subtype = Test;
        [Test] procedure Fails() begin Error('plain'); end; }
        codeunit 60141 Chained { Subtype = Test; RequiredTestIsolation = Disabled;
        [Test] procedure Fails() begin Error('chained'); end;
        [Test] procedure Passes() begin end; }";
    let opts = |hooks: Hooks| RunOptions {
        required_isolation: true,
        hooks,
        ..Default::default()
    };
    // Its own failure does not trip: both tests run.
    let stop = FailureStop::new(StopScope::Request);
    let rt = run(
        src,
        &RunOptions {
            only: Some(vec![(0, 60141)]),
            ..opts(with_stop(&stop, Hooks::default()))
        },
    );
    let alone: Vec<_> = rt
        .results()
        .iter()
        .filter(|r| r.codeunit == 60141)
        .map(|r| r.outcome.clone())
        .collect();
    assert!(is_fail(&alone[0]) && matches!(alone[1], Outcome::Pass), "{alone:?}");
    // Tripped by the plain codeunit of entry 1, the Disabled one of entry 2 starts no test.
    let (starts, hooks) = counting_start();
    let stop = FailureStop::new(StopScope::Request);
    let first = EntryRun {
        only: Some(vec![(0, 60140)]),
        hooks: with_stop(&stop, hooks.clone()),
        ..Default::default()
    };
    let second = EntryRun {
        only: Some(vec![(0, 60141)]),
        hooks: with_stop(&stop, hooks),
        ..Default::default()
    };
    let rt = run(
        src,
        &RunOptions {
            entries: vec![first, second],
            ..opts(Hooks::default())
        },
    );
    let got = per_entry(&rt);
    assert!(is_fail(&got[0][0]), "{got:?}");
    assert_eq!(got[1], vec![not_run(REASON_REQUEST); 2], "{got:?}");
    assert_eq!(starts.load(Ordering::SeqCst), 1, "{got:?}");
}
