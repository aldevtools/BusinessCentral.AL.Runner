//! The refusals around the run's one login a Disabled codeunit on the hub holds (held_login.rs `login_once_on_hub`,
//! assumption `hub-disabled-login-once`): which later codeunits are not measured, the carry refusal a session that
//! starts from what another codeunit left keeps, and what a test of a held session may not read (`held_reads`), and how each session logs in (`login_plan`). The
//! neighbor apps `tests/neighbors/disabled-login-writes*` run the same shapes end to end.
use super::held_login::{
    HeldOpened, LoginPlan, carried_company_open, first_login, held_reads, login_plan, login_rows_as_held,
    own_session_id, runs_own_login,
};
use super::*;
use crate::install::CompanyOpenWrites;

const SRC: &str = "table 50100 \"Login Row\" { fields { field(1; No; Integer) { } } keys { key(PK; No) { } } }
    table 50101 Other { fields { field(1; No; Integer) { } } keys { key(PK; No) { } } }
    codeunit 50102 Memo { SingleInstance = true; var N: Integer; procedure Get(): Integer begin exit(N); end; }
    codeunit 50103 T { Subtype = Test;
        [Test] procedure ReadsLoginRow() var R: Record \"Login Row\"; begin R.FindFirst(); end;
        [Test] procedure ReadsOther() var R: Record Other; begin R.FindFirst(); end;
        [Test] procedure ReadsMemo() var M: Codeunit Memo; begin M.Get(); end; }";

fn object(l: &al_sema::lower::Lowered, name: &str) -> usize {
    (l.program.objects.iter()).position(|o| o.name == name).expect(name)
}

/// The login wrote `Login Row`: what `open_company` noted of it.
fn login_wrote(l: &al_sema::lower::Lowered) -> CompanyOpenWrites {
    CompanyOpenWrites {
        login: Some("Login Subs".into()),
        login_tables: [object(l, "Login Row") as u32].into(),
        raised: true,
        ..Default::default()
    }
}

fn held(l: &al_sema::lower::Lowered) -> al_vm::HeldLogin {
    let (_, engine) = super::tests::engine_for(SRC);
    held_in(l, &mut Session::new(engine))
}

/// The login held in `session`, whose data is what the login left.
fn held_in(l: &al_sema::lower::Lowered, session: &mut Session) -> al_vm::HeldLogin {
    let opened = HeldOpened::of(session, login_wrote(l), al_builtins::assumptions::HitBits::default());
    al_vm::HeldLogin {
        image: Err("none".into()),
        singles: vec![object(l, "Memo")],
        opened: Box::new(opened),
    }
}

/// A session of `engine` whose committed data holds one row of `Login Row`, as the login left it.
fn with_login_row(l: &al_sema::lower::Lowered, engine: &std::sync::Arc<al_vm::Engine>) -> Session {
    use al_data::Store;
    let mut session = Session::new(engine.clone());
    let table = engine.tables[object(l, "Login Row")].clone().expect("a table");
    let cx = al_data::AccessContext {
        company: session.access_company().into(),
        user: session.user_id().into(),
    };
    let row: Vec<al_builtins::value::Value> = (table.fields.iter())
        .map(|f| al_builtins::value::Value::default_for(&f.ty))
        .collect();
    assert!(session.store_mut().insert(&cx, &table, row).is_ok());
    session.commit_transaction();
    session
}

/// Why each test of codeunit T is not run in a session with `wrote` (`None`: it runs).
fn refusals(l: &al_sema::lower::Lowered, engine: &al_vm::Engine, wrote: &CompanyOpenWrites) -> Vec<Option<String>> {
    let index = written::ReachIndex::new(&engine.events, l);
    let t = object(l, "T");
    (0..l.program.objects[t].routines.len())
        .map(|r| wrote.refuses(&written::test_reach_facts(&engine.events, l, &index, t, r), l))
        .collect()
}

// Runner 130450 is the one start after the held login that runs a login of its own (corpus PR 1644,
// LgnHeldLoginRunners.al:608, :656, :697); every hub isolation thaws the held login (:452, :491, :528).
#[test]
fn only_runner_130450_codeunits_after_the_held_login_run_their_own_login() {
    assert!(runs_own_login(false, TestIsolation::Codeunit));
    assert!(!runs_own_login(true, TestIsolation::Function));
    assert!(!runs_own_login(true, TestIsolation::Disabled));
    assert!(!runs_own_login(true, TestIsolation::Codeunit));
}

// A Disabled codeunit whose login wrote and whose session holds no login (it starts from what another codeunit left)
// keeps main's refusal; one whose session holds the login carries it.
#[test]
fn a_disabled_codeunit_whose_login_wrote_carries_only_from_a_held_login() {
    let (l, _) = super::tests::engine_for(SRC);
    let wrote = login_wrote(&l);
    let not_held = carried_company_open(false, true, TestIsolation::Disabled, false, true, &wrote);
    assert!(not_held.is_err_and(|why| why.starts_with("Disabled test isolation with a Company Triggers subscriber")));
    assert_eq!(
        carried_company_open(true, true, TestIsolation::Disabled, false, true, &wrote),
        Ok(true)
    );
    // Not on the hub: main's refusal, held or not.
    assert!(carried_company_open(true, false, TestIsolation::Disabled, false, true, &wrote).is_err());
}

#[test]
fn only_a_session_from_the_install_baseline_starts_the_runs_login() {
    let (_, engine) = super::tests::engine_for(SRC);
    let mut session = Session::new(engine);
    assert!(first_login(true, TestIsolation::Disabled, &session));
    assert!(!first_login(false, TestIsolation::Disabled, &session));
    assert!(!first_login(true, TestIsolation::Codeunit, &session));
    // A session that starts from what a test codeunit left.
    session.in_test_codeunit = true;
    let left = session.capture_baseline();
    let mut later = Session::new(session.engine.clone());
    later.restore_baseline(&left);
    assert!(!first_login(true, TestIsolation::Disabled, &later));
}

// Runner 130450 runs every codeunit after the held login in one session, not the hub's (corpus PR 1657,
// LgnRunnerSessions.al:423 not the hub's; :436 every earlier such codeunit had the same id): each answers the same
// SessionId, not the hub session's. A hub codeunit, a session without a held login and a scratch run keep the host's.
#[test]
fn every_runner_130450_codeunit_after_the_held_login_answers_one_session_id() {
    let (l, _) = super::tests::engine_for(SRC);
    let held = held(&l);
    let hub = 1;
    let first = own_session_id(Some(&held), false, TestIsolation::Codeunit, false, hub).map(|o| o.id);
    let second = own_session_id(Some(&held), false, TestIsolation::Function, false, hub).map(|o| o.id);
    assert!(first.is_some_and(|id| id != hub), "{first:?}");
    assert_eq!(first, second);
    assert_eq!(
        own_session_id(Some(&held), true, TestIsolation::Codeunit, false, hub),
        None
    );
    assert_eq!(own_session_id(None, false, TestIsolation::Codeunit, false, hub), None);
    assert_eq!(
        own_session_id(Some(&held), false, TestIsolation::Codeunit, true, hub),
        None
    );
}

// A later codeunit of the session runner 130450 runs its codeunits in after the held login, whose login an earlier one
// ran (corpus PR 1657, LgnRunnerSessions.al:424, :433), refuses nothing: with the image thawed or not, and with the login
// rows as that login left them or changed by an earlier codeunit of the session. Main ran these shapes too (review of PR
// 9426, round 1, findings H2 and H3).
#[test]
fn a_later_runner_130450_codeunit_refuses_nothing() {
    let (l, engine) = super::tests::engine_for(SRC);
    let runner = held_in(&l, &mut with_login_row(&l, &engine));
    for (thawed, rows_as_held) in [(true, true), (false, true), (true, false), (false, false)] {
        let wrote = CompanyOpenWrites {
            held: held_reads(&runner, true, thawed, rows_as_held),
            ..Default::default()
        };
        assert!(wrote.held.is_none(), "thawed {thawed}, rows as held {rows_as_held}");
        let r = refusals(&l, &engine, &wrote);
        assert_eq!((&r[0], &r[1], &r[2]), (&None, &None, &None), "{r:?}");
    }
}

// Which login a session holds: the first codeunit runner 130450 runs after the held login runs and holds the login of
// its session; a later one shares it, whatever moved into it (review of PR 9426, round 1, finding H1); on the hub a
// session from the install baseline holds the run's login, one with a held login shares it.
#[test]
fn each_session_takes_the_login_plan_of_its_place_in_the_run() {
    let (l, engine) = super::tests::engine_for(SRC);
    let mut session = Session::new(engine.clone());
    assert!(matches!(
        login_plan(&session, true, true, TestIsolation::Codeunit),
        LoginPlan::HoldRunner
    ));
    assert!(matches!(
        login_plan(&session, false, true, TestIsolation::Disabled),
        LoginPlan::HoldHub
    ));
    assert!(matches!(
        login_plan(&session, false, true, TestIsolation::Codeunit),
        LoginPlan::Run
    ));
    assert!(matches!(
        login_plan(&session, false, false, TestIsolation::Disabled),
        LoginPlan::Run
    ));
    let opened = HeldOpened::of(
        &mut session,
        login_wrote(&l),
        al_builtins::assumptions::HitBits::default(),
    );
    session.hold_runner_login(Err("none".into()), Box::new(opened));
    assert!(matches!(
        login_plan(&session, true, true, TestIsolation::Function),
        LoginPlan::Shared { runner: true, .. }
    ));
    assert!(matches!(
        login_plan(&session, false, true, TestIsolation::Codeunit),
        LoginPlan::Run
    ));
    let opened = HeldOpened::of(
        &mut session,
        login_wrote(&l),
        al_builtins::assumptions::HitBits::default(),
    );
    session.hold_login(Err("none".into()), Box::new(opened));
    assert!(matches!(
        login_plan(&session, false, true, TestIsolation::Codeunit),
        LoginPlan::Shared { runner: false, .. }
    ));
}

// On the hub under Codeunit isolation without the held image only the reader of the SingleInstance codeunit is not
// run (its own login set it again); with the image thawed nothing is refused.
#[test]
fn a_hub_codeunit_without_the_held_image_refuses_only_the_single_instance_reader() {
    let (l, engine) = super::tests::engine_for(SRC);
    let wrote = CompanyOpenWrites {
        held: held_reads(&held(&l), false, false, true),
        ..Default::default()
    };
    let r = refusals(&l, &engine, &wrote);
    assert_eq!((&r[0], &r[1]), (&None, &None));
    assert!(r[2].as_ref().is_some_and(|w| w.contains("could not be kept")), "{r:?}");
    assert!(held_reads(&held(&l), false, true, true).is_none());
}

// On the hub a Codeunit or Function codeunit whose login rows are as the held login left them is measured
// (LgnDisabledLoginWrites.al:546; corpus PR 1644, LgnHeldLoginRunners.al:491, :528); after a Disabled codeunit changed
// them its readers of the login's tables are not run, whether the image was thawed or not. A Disabled codeunit is
// measured either way (LgnDisabledLoginWrites.al:471, :636).
#[test]
fn a_hub_codeunit_after_a_disabled_codeunit_changed_the_login_rows_refuses_their_readers() {
    let (l, engine) = super::tests::engine_for(SRC);
    let mut left = with_login_row(&l, &engine);
    let held = held_in(&l, &mut left);
    for isolation in [
        TestIsolation::Codeunit,
        TestIsolation::Function,
        TestIsolation::Disabled,
    ] {
        assert!(
            login_rows_as_held(&held, isolation, &mut left),
            "{isolation:?}: rows as left"
        );
    }
    let mut deleted = Session::new(engine.clone());
    assert!(!login_rows_as_held(&held, TestIsolation::Codeunit, &mut deleted));
    assert!(!login_rows_as_held(&held, TestIsolation::Function, &mut deleted));
    assert!(login_rows_as_held(&held, TestIsolation::Disabled, &mut deleted));
    let mut wrote = CompanyOpenWrites::default();
    assert!(held_reads(&held, false, true, true).is_none());
    for thawed in [true, false] {
        wrote.held = held_reads(&held, false, thawed, false);
        let r = refusals(&l, &engine, &wrote);
        let login_row = r[0].as_deref().unwrap_or_default();
        assert!(
            login_row.contains("changed the rows the run's one login wrote"),
            "{r:?}"
        );
        assert!(login_row.ends_with("this test can read table Login Row"), "{r:?}");
        assert_eq!(r[1], None);
        assert_eq!(r[2].is_some(), !thawed, "{r:?}");
    }
}
