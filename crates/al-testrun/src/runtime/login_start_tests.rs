// The scans of production code stop at the first `#[cfg(test)]` of a file; a test module in a file of its own starts with one.
#![cfg(test)]
//! The login start (ADR-073 phase 1, `login_start.rs`): the company-open step of a test session runs once per engine and
//! later sessions with equal inputs take its result; the off switch and the state check.

use super::*;
use crate::login_start::counts;
use al_sema::workspace::AppSource;

/// A Company Triggers subscriber of the app under test keeps a record global (positioned on the last entry) and a
/// counter in a SingleInstance codeunit. Each test codeunit's session must see what a real run of the subscriber left:
/// the record's fields and its position (`Next(-1)` reaches the first entry), and the counter at 1.
const OPEN_SRC: &str = "table 60900 Entry { fields { field(1; No; Integer) { } field(2; D; Date) { } } keys { key(PK; No) { } } }
    codeunit 60910 Seed { trigger OnRun() var E: Record Entry; begin
        E.No := 1; E.D := 20240115D; E.Insert(); E.No := 2; E.D := 20240116D; E.Insert(); end; }
    codeunit 60920 Keeper { SingleInstance = true;
        var Last: Record Entry; Opened: Integer;
        [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Company Triggers\", 'OnCompanyOpenCompleted', '', false, false)]
        local procedure OnOpen() begin Last.FindLast(); Opened += 1; end;
        procedure Seen(): Text begin exit(Format(Last.No) + '|' + Format(Last.D, 0, 9) + '|' + Format(Opened)); end;
        procedure Before(): Integer begin if Last.Next(-1) = 0 then exit(0); exit(Last.No); end; }
    codeunit 60102 T1 { Subtype = Test;
        [Test] procedure SeesTheOpenState() var K: Codeunit Keeper; begin
            if K.Seen() <> '2|2024-01-16|1' then Error('seen %1', K.Seen());
            if K.Before() <> 1 then Error('before %1', K.Before()); end; }
    codeunit 60103 T2 { Subtype = Test;
        [Test] procedure SeesTheOpenState() var K: Codeunit Keeper; begin
            if K.Seen() <> '2|2024-01-16|1' then Error('seen %1', K.Seen());
            if K.Before() <> 1 then Error('before %1', K.Before()); end; }";

fn engine_of(src: &str, mode: al_vm::RecordedRunsMode) -> (Arc<Engine>, Lowered) {
    let (ws, l) = al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), src.into())],
        }],
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().map(|r| r.to_string())).collect())
        .collect();
    let mut engine = Engine::new(l.program.clone(), blocked);
    Arc::get_mut(&mut engine).unwrap().recorded_runs = mode;
    (engine, l)
}

fn object_named(l: &Lowered, name: &str) -> usize {
    l.program.objects.iter().position(|o| o.name == name).unwrap()
}

/// Runs T1, T2 and T1 again from the baseline Seed leaves; every test must pass.
fn run_three(mode: al_vm::RecordedRunsMode) -> crate::login_start::LoginCounts {
    let (engine, l) = engine_of(OPEN_SRC, mode);
    let mut session = al_vm::Session::new(engine.clone());
    let s = object_named(&l, "Seed");
    let on_run = l.program.objects[s]
        .routines
        .iter()
        .position(|r| r.name == "OnRun")
        .unwrap();
    let inst = session.instance(s);
    assert!(session.call(s, on_run, inst, Vec::new()).is_ok());
    let baseline = session.capture_baseline();
    for t in ["T1", "T2", "T1"] {
        let o = run_codeunit_from(&engine, &l, object_named(&l, t), Some(&baseline));
        assert_eq!(o, vec![("SeesTheOpenState".to_string(), Outcome::Pass)], "{t}");
    }
    counts(&engine)
}

#[test]
fn the_company_open_step_runs_once_and_later_sessions_get_its_single_instance_state() {
    let c = run_three(al_vm::RecordedRunsMode::On);
    assert_eq!((c.company_open.recorded, c.company_open.replayed), (1, 2), "{c:?}");
}

#[test]
fn the_off_switch_runs_the_login_in_every_session() {
    let c = run_three(al_vm::RecordedRunsMode::Off);
    assert_eq!(c, crate::login_start::LoginCounts::default());
}

#[test]
fn the_state_check_runs_the_login_where_a_replay_applies_and_finds_the_same_state() {
    let c = run_three(al_vm::RecordedRunsMode::Check);
    assert_eq!(
        (c.company_open.recorded, c.company_open.replayed, c.company_open.checked),
        (1, 0, 2),
        "{c:?}"
    );
}

// Login code that changes a record global is replayed with it (ADR-073 decision 3): a later session's global holds the
// record the run found (`LastNo`), as in a session that ran the code.
#[test]
fn login_code_that_changes_a_record_global_is_replayed_with_it() {
    let src = super::tests::LOGIN_SRC
        .replace(
            "var LogInWorkDate: Date;",
            "var LogInWorkDate: Date; Last: Record Entry;",
        )
        .replace(
            "if E.FindLast() then begin",
            "if Last.FindLast() then; if E.FindLast() then begin",
        )
        .replace(
            "procedure LoginDate(): Date",
            "procedure LastNo(): Integer begin exit(Last.No); end; procedure LoginDate(): Date",
        )
        .replace(
            "Error('LogInWorkDate %1', L.LoginDate());",
            "Error('LogInWorkDate %1', L.LoginDate()); if L.LastNo() <> 1 then Error('Last %1', L.LastNo());",
        );
    let (engine, l) = super::tests::login_engine(&src);
    let baseline = super::tests::seeded_baseline(&engine, &l, "Seed");
    for t in ["T1", "T2"] {
        let o = run_codeunit_from(&engine, &l, object_named(&l, t), Some(&baseline));
        assert_eq!(o, vec![("SeesTheLoginDate".to_string(), Outcome::Pass)], "{t}");
    }
    assert_eq!(super::tests::login_replays(&engine), 1);
}

/// The WorkDate step leaves LogInManagement's single instance; the company-open step after it is still recorded and
/// replayed, with that instance's image as part of its key, and a later session sees both steps' state.
#[test]
fn the_company_open_step_after_the_work_date_step_is_replayed_with_both_steps_state() {
    let login = "codeunit 40 LogInManagement { SingleInstance = true;
        var LogInWorkDate: Date;
        procedure GetDefaultWorkDate(): Date var E: Record Entry; begin
            if E.FindFirst() then begin LogInWorkDate := E.D; exit(E.D); end; exit(WorkDate()); end;
        procedure LoginDate(): Date begin exit(LogInWorkDate); end; }";
    let check = "if K.Before() <> 1 then Error('before %1', K.Before());";
    let src = format!("{OPEN_SRC}\n{login}").replace(
        check,
        &format!("{check} if WorkDate() <> 20240115D then Error('WorkDate %1', WorkDate()); if L.LoginDate() <> 20240115D then Error('login %1', L.LoginDate());"),
    ).replace("var K: Codeunit Keeper;", "var K: Codeunit Keeper; L: Codeunit LogInManagement;");
    let (engine, l) = engine_of(&src, al_vm::RecordedRunsMode::On);
    let mut session = al_vm::Session::new(engine.clone());
    let s = object_named(&l, "Seed");
    let on_run = l.program.objects[s]
        .routines
        .iter()
        .position(|r| r.name == "OnRun")
        .unwrap();
    let inst = session.instance(s);
    assert!(session.call(s, on_run, inst, Vec::new()).is_ok());
    let baseline = session.capture_baseline();
    for t in ["T1", "T2", "T1"] {
        let o = run_codeunit_from(&engine, &l, object_named(&l, t), Some(&baseline));
        assert_eq!(o, vec![("SeesTheOpenState".to_string(), Outcome::Pass)], "{t}");
    }
    let c = counts(&engine);
    assert_eq!((c.work_date.replayed, c.company_open.replayed), (2, 2), "{c:?}");
}
