//! The refusals around the run's one login a Disabled codeunit on the hub holds (run.rs `login_once_on_hub`,
//! assumption `hub-disabled-login-once`): which later codeunits are not measured, the carry refusal a session that
//! starts from what another codeunit left keeps, and what a test of a held session may not read (`held_reads`). The
//! neighbor apps `tests/neighbors/disabled-login-writes*` run the same shapes end to end.
use super::run::{carried_company_open, first_login, held_reads, login_held_unmeasured};
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
    al_vm::HeldLogin {
        image: Err("none".into()),
        singles: vec![object(l, "Memo")],
        opened: Box::new((login_wrote(l), al_builtins::assumptions::HitBits::default())),
    }
}

/// Why each test of codeunit T is not run in a session with `wrote` (`None`: it runs).
fn refusals(l: &al_sema::lower::Lowered, engine: &al_vm::Engine, wrote: &CompanyOpenWrites) -> Vec<Option<String>> {
    let index = written::ReachIndex::new(&engine.events, l);
    let t = object(l, "T");
    (0..l.program.objects[t].routines.len())
        .map(|r| wrote.refuses(&written::test_reach_facts(&engine.events, l, &index, t, r), l))
        .collect()
}

#[test]
fn runner_130450_and_hub_function_codeunits_after_the_held_login_are_not_measured() {
    let runner = login_held_unmeasured(false, TestIsolation::Codeunit).expect("runner 130450");
    assert!(runner.contains("which runner 130450 runs"), "{runner}");
    let function = login_held_unmeasured(true, TestIsolation::Function).expect("hub Function");
    assert!(function.contains("on the hub with this isolation"), "{function}");
    assert_eq!(login_held_unmeasured(true, TestIsolation::Disabled), None);
    assert_eq!(login_held_unmeasured(true, TestIsolation::Codeunit), None);
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

// Under runner 130450 (or Function on the hub) a test that reads the table the login wrote or the SingleInstance
// codeunit it left is not run; the test that reads neither runs, as on main.
#[test]
fn an_unmeasured_codeunit_after_the_held_login_refuses_only_its_readers() {
    let (l, engine) = super::tests::engine_for(SRC);
    let why = login_held_unmeasured(false, TestIsolation::Codeunit);
    let mut wrote = CompanyOpenWrites::default();
    wrote.held = held_reads(&held(&l), why, false, &wrote);
    let r = refusals(&l, &engine, &wrote);
    assert!(
        r[0].as_ref()
            .is_some_and(|w| w.ends_with("this test can read table Login Row")),
        "{r:?}"
    );
    assert_eq!(r[1], None);
    assert!(
        r[2].as_ref()
            .is_some_and(|w| w.ends_with("SingleInstance codeunit Memo")),
        "{r:?}"
    );
    assert!(wrote.watched().contains(&(object(&l, "Login Row") as u32)));
}

// On the hub under Codeunit isolation without the held image only the reader of the SingleInstance codeunit is not
// run (its own login set it again); with the image thawed nothing is refused.
#[test]
fn a_hub_codeunit_without_the_held_image_refuses_only_the_single_instance_reader() {
    let (l, engine) = super::tests::engine_for(SRC);
    let mut wrote = CompanyOpenWrites::default();
    wrote.held = held_reads(&held(&l), None, false, &wrote);
    let r = refusals(&l, &engine, &wrote);
    assert_eq!((&r[0], &r[1]), (&None, &None));
    assert!(r[2].as_ref().is_some_and(|w| w.contains("could not be kept")), "{r:?}");
    assert!(held_reads(&held(&l), None, true, &wrote).is_none());
}
