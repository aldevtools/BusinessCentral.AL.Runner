// The scans of production code stop at the first `#[cfg(test)]` of a file; a test module in a file of its own starts with one.
#![cfg(test)]
//! The check of the raised login (`login_replay.rs`, al-runner issue 8211) on real runs over the stand-in of Microsoft's
//! login (`login_tests::DEP`): sessions of one key hold the same after the login, and an input the key holds apart (the
//! test codeunit, an entry's resource text) changes what the login leaves, so a key without it would hand a session
//! another input's store.

use super::*;
use crate::login_replay::{Check, first_difference};

/// An app under test whose `OnRegisterManualSetup` subscriber (the issue's shape; the event raises the login, `login.rs`
/// `RAISE_ONLY`) registers a row whose id depends on the text of the app's resource `rpl.txt`, and keeps a copy in a temporary record of
/// a SingleInstance codeunit, shared with a second record (`Copy(x, true)`), as Base Application's "Change Log Management"
/// keeps temporary records after the login on BC 28.5.
const MAIN: &str = r#"
codeunit 77400 "RPL Registration"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Guided Experience", 'OnRegisterManualSetup', '', false, false)]
    local procedure RegisterManualSetup()
    var G: Codeunit "Guided Experience"; Cache: Codeunit "RPL Cache";
    begin
        G.InsertAssistedSetup(77400 + StrLen(NavApp.GetResourceAsText('rpl.txt')));
        Cache.Keep(77400 + StrLen(NavApp.GetResourceAsText('rpl.txt')));
    end;
}
codeunit 77405 "RPL Cache"
{
    SingleInstance = true;
    var Temp: Record "Guided Experience Item" temporary; Shared: Record "Guided Experience Item" temporary;
        Helper: Codeunit "RPL Helper"; Other: Codeunit "RPL Helper"; Same: Codeunit "RPL Helper";
    procedure Keep(Id: Integer)
    begin
        Temp."Object ID" := Id; Temp.Insert(); Shared.Copy(Temp, true);
        Helper.SetX(Id); Same := Helper; Other.SetX(1);
    end;
}
codeunit 77406 "RPL Helper"
{
    var X: Integer; Buffer: Record "Guided Experience Item" temporary;
    procedure SetX(V: Integer) begin X := V; Buffer."Object ID" := V; Buffer.Insert(); end;
}
codeunit 77402 "RPL Tests A"
{
    Subtype = Test;
    [Test]
    procedure SeesTheRow() var I: Record "Guided Experience Item"; begin if I.IsEmpty() then Error('no row'); end;
}
codeunit 77403 "RPL Tests B"
{
    Subtype = Test;
    [Test]
    procedure SeesTheRow() var I: Record "Guided Experience Item"; begin if I.IsEmpty() then Error('no row'); end;
}
"#;

/// The compiled apps (the main app has the resource `rpl.txt` holding `a`) and an engine for them.
fn compiled() -> (Arc<Engine>, al_sema::lower::Lowered) {
    let (ws, mut l) = al_sema::lower::compile_apps(
        [
            ("main", vec!["dep".to_string()], MAIN),
            ("dep", vec![], super::login_tests::DEP),
        ]
        .into_iter()
        .map(|(id, dependencies, src)| al_sema::workspace::AppSource {
            id: id.into(),
            name: id.into(),
            dependencies,
            files: vec![(format!("{id}.al"), src.into())],
        })
        .collect(),
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let main = l.program.apps.iter().position(|a| a.name == "main").unwrap();
    Arc::make_mut(&mut l.program).apps[main].resources = vec![("rpl.txt".into(), b"a".to_vec())];
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().map(|r| r.to_string())).collect())
        .collect();
    (Engine::new(l.program.clone(), blocked), l)
}

/// Runs test codeunit `name` once on `engine` with the check on; every test must pass.
fn run(engine: &Arc<Engine>, l: &al_sema::lower::Lowered, name: &str) {
    run_with(engine, l, name, RunOptions::default());
}

/// [`run`] with the session flags of `opts` (`SessionFlags::of`).
fn run_with(engine: &Arc<Engine>, l: &al_sema::lower::Lowered, name: &str, opts: RunOptions) {
    let mut scratch = Scratch {
        flags: super::session_flags::SessionFlags::of(&RunOptions {
            login_replay_check: true,
            ..opts
        }),
        ..Default::default()
    };
    let oi = l.program.objects.iter().position(|o| o.name == name).unwrap();
    let o = run_codeunit_core(
        engine,
        l,
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
    assert_eq!(o, vec![("SeesTheRow".to_string(), Outcome::Pass)]);
}

/// The text `rpl.txt` of the main app reads as in this pass (a serve entry's `resources`, `Engine::resource_overrides`).
fn set_resource(engine: &Engine, l: &al_sema::lower::Lowered, text: Option<&str>) {
    let main = l.program.apps.iter().position(|a| a.name == "main").unwrap();
    let mut o = engine.resource_overrides.write().unwrap();
    o.clear();
    if let Some(t) = text {
        o.insert(main, Arc::new(vec![("rpl.txt".to_string(), t.as_bytes().to_vec())]));
    }
}

#[test]
fn sessions_of_one_key_hold_the_same_after_the_raised_login() {
    let (engine, l) = compiled();
    for _ in 0..3 {
        run(&engine, &l, "RPL Tests A");
    }
    assert_eq!(
        crate::login_replay::report(&engine).as_deref(),
        Some(
            "login replay check: 3 sessions over 1 keys (a key seen before: 2 sessions, 66%), 2 compared, 0 differ; across test codeunits: 0 compared, 0 differ"
        )
    );
    // What the login wrote is in the state compared: the subscriber's row (77400 + 1).
    let firsts = crate::login_replay::check(&engine).firsts();
    assert!(
        firsts[0].1.rows().iter().any(|r| r.contains("Integer(77401)")),
        "{:?}",
        firsts[0].1.rows()
    );
}

#[test]
fn another_test_codeunit_is_another_key() {
    let (engine, l) = compiled();
    run(&engine, &l, "RPL Tests A");
    run(&engine, &l, "RPL Tests B");
    assert_eq!(
        crate::login_replay::report(&engine).as_deref(),
        Some(
            "login replay check: 2 sessions over 2 keys (a key seen before: 0 sessions, 0%), 0 compared, 0 differ; across test codeunits: 1 compared, 0 differ"
        )
    );
}

// Two serve entries with other `resources`: the login reads the text, so what it leaves differs. The key holds the two
// apart; under one key the comparator reports the difference (the planted case).
#[test]
fn an_entrys_resource_text_is_part_of_the_key_and_changes_what_the_login_leaves() {
    let (engine, l) = compiled();
    run(&engine, &l, "RPL Tests A");
    set_resource(&engine, &l, Some("abcd"));
    run(&engine, &l, "RPL Tests A");
    set_resource(&engine, &l, None);
    run(&engine, &l, "RPL Tests A");
    assert_eq!(
        crate::login_replay::report(&engine).as_deref(),
        Some(
            "login replay check: 3 sessions over 2 keys (a key seen before: 1 sessions, 33%), 1 compared, 0 differ; across test codeunits: 0 compared, 0 differ"
        )
    );
    let firsts = crate::login_replay::check(&engine).firsts();
    let difference = first_difference(&firsts[0].1, &firsts[1].1);
    assert!(
        difference.as_deref().is_some_and(|d| d.contains("77404")),
        "{difference:?}"
    );
    // The temporary table of the SingleInstance cache holds the other row too.
    let temp_row = |k: usize, id: &str| {
        (firsts[k].1.state_lines().lines.iter()).any(|l| l.starts_with("temp table 0 row ") && l.contains(id))
    };
    assert!(temp_row(0, "Integer(77401)") && temp_row(1, "Integer(77404)"));
    let planted = Check::default();
    planted.note(firsts[0].0.clone(), firsts[0].1.clone());
    planted.note(firsts[0].0.clone(), firsts[1].1.clone());
    assert!(planted.line().is_some_and(|l| l.contains("1 compared, 1 differ")));
}

// What the login leaves in a SingleInstance codeunit's temporary records ("Change Log Management" on BC 28.5) compares by
// the table's rows and everything each record carries: equal sessions are equal, not "not comparable"; two records that
// share one table (`Copy(x, true)`) name it once, as one table.
#[test]
fn temporary_records_the_login_leaves_compare_by_their_rows_and_view_and_a_shared_table_is_one_table() {
    let (engine, l) = compiled();
    run(&engine, &l, "RPL Tests A");
    run(&engine, &l, "RPL Tests A");
    let line = crate::login_replay::report(&engine).unwrap();
    assert!(line.contains(", 1 compared, 0 differ;"), "{line}");
    let firsts = crate::login_replay::check(&engine).firsts();
    let state = firsts[0].1.state_lines();
    assert!(state.opaque.is_empty(), "{:?}", state.opaque);
    let rows: Vec<&String> = state
        .lines
        .iter()
        .filter(|l| l.starts_with("temp table 0 row "))
        .collect();
    assert_eq!(rows.len(), 1, "{:?}", state.lines);
    assert!(rows[0].contains("Integer(77401)"), "{rows:?}");
    let records = state
        .lines
        .iter()
        .filter(|l| l.contains(" temp record of table 0: "))
        .count();
    assert_eq!(records, 2, "{:?}", state.lines);
}

// A serve request's session flags and license reach the login code (a permission error, an unhandled UI call) while the
// engine and its check outlive the request: on one engine, a session with another flag or another license has a key of
// its own, it is not compared with the sessions before.
#[test]
fn another_session_flag_or_license_is_another_key() {
    let (mut engine, l) = compiled();
    run(&engine, &l, "RPL Tests A");
    let flag = |o: RunOptions| run_with(&engine, &l, "RPL Tests A", o);
    flag(RunOptions {
        ignore_permission_mock: true,
        ..Default::default()
    });
    flag(RunOptions {
        dev_endpoint_ui: true,
        ..Default::default()
    });
    let everything: &[(i64, i64, u32)] = &[(0, 2_000_000_000, 0b1_1111)];
    let license = al_host::license::parse(&al_host::license::synthetic::file(
        "X",
        "1/1/2040",
        &[
            ("TableData", everything),
            ("Codeunit", everything),
            ("Page", everything),
        ],
    ))
    .unwrap();
    Arc::get_mut(&mut engine).expect("no session holds the engine").license = Some(Arc::new(license));
    run(&engine, &l, "RPL Tests A");
    run(&engine, &l, "RPL Tests A");
    assert_eq!(
        crate::login_replay::report(&engine).as_deref(),
        Some(
            "login replay check: 5 sessions over 4 keys (a key seen before: 1 sessions, 20%), 1 compared, 0 differ; \
             across test codeunits: 0 compared, 0 differ"
        )
    );
}

// Codeunit instances a SingleInstance codeunit holds after the login (on a real app one global of a SingleInstance
// codeunit holds one): compared by identity (one number per instance, by first sight: two globals that hold one
// instance name the same number, another instance another) and by their own state (the codeunit and its globals).
#[test]
fn codeunit_instances_the_login_leaves_compare_by_identity_and_their_state() {
    let (engine, l) = compiled();
    run(&engine, &l, "RPL Tests A");
    run(&engine, &l, "RPL Tests A");
    let line = crate::login_replay::report(&engine).unwrap();
    assert!(line.contains(", 1 compared, 0 differ"), "{line}");
    let firsts = crate::login_replay::check(&engine).firsts();
    let state = firsts[0].1.state_lines();
    assert!(state.opaque.is_empty(), "{:?}", state.opaque);
    let global = |g: usize| {
        let prefix = format!("single 77405 global {g} ");
        let line = state.lines.iter().find(|l| l.starts_with(&prefix));
        line.unwrap_or_else(|| panic!("{:?}", state.lines)).clone()
    };
    // Helper and Same hold one instance, Other another.
    assert!(global(2).ends_with(" instance 0"), "{}", global(2));
    assert!(global(4).ends_with(" instance 0"), "{}", global(4));
    assert!(global(3).ends_with(" instance 1"), "{}", global(3));
    for n in 0..2 {
        let own = format!("instance {n} codeunit 77406 globals [");
        assert!(state.lines.iter().any(|l| l.starts_with(&own)), "{:?}", state.lines);
    }
    // The resource text the login read reaches the instance's own global: another text, another instance state.
    set_resource(&engine, &l, Some("abcd"));
    run(&engine, &l, "RPL Tests A");
    let firsts = crate::login_replay::check(&engine).firsts();
    let own = |k: usize| {
        let lines = &firsts[k].1.state_lines().lines;
        lines
            .iter()
            .find(|l| l.starts_with("instance 0 codeunit 77406"))
            .cloned()
            .unwrap()
    };
    assert_ne!(own(0), own(1));
}
