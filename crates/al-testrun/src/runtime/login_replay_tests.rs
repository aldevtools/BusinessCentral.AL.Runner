// The scans of production code stop at the first `#[cfg(test)]` of a file; a test module in a file of its own starts with one.
#![cfg(test)]
//! The check of the raised login (`login_replay.rs`, al-runner issue 8211) on real runs over the stand-in of Microsoft's
//! login (`login_tests::DEP`): sessions of one key hold the same after the login, and an input the key holds apart (the
//! test codeunit, an entry's resource text) changes what the login leaves, so a key without it would hand a session
//! another input's store.

use super::*;
use crate::login_replay::{Check, first_difference};

/// An app under test whose `OnRegisterManualSetup` subscriber (the issue's shape; the event raises the login, `login.rs`
/// `RAISE_ONLY`) registers a row whose id depends on the text of the app's resource `rpl.txt`.
const MAIN: &str = r#"
codeunit 77400 "RPL Registration"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Guided Experience", 'OnRegisterManualSetup', '', false, false)]
    local procedure RegisterManualSetup()
    var G: Codeunit "Guided Experience";
    begin
        G.InsertAssistedSetup(77400 + StrLen(NavApp.GetResourceAsText('rpl.txt')));
    end;
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
    let mut scratch = Scratch {
        flags: super::session_flags::SessionFlags::of(&RunOptions {
            login_replay_check: true,
            ..Default::default()
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
            "login replay check: 3 sessions over 1 keys, 2 compared, 0 differ; across test codeunits: 0 compared, 0 differ"
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
            "login replay check: 2 sessions over 2 keys, 0 compared, 0 differ; across test codeunits: 1 compared, 0 differ"
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
            "login replay check: 3 sessions over 2 keys, 1 compared, 0 differ; across test codeunits: 0 compared, 0 differ"
        )
    );
    let firsts = crate::login_replay::check(&engine).firsts();
    let difference = first_difference(&firsts[0].1, &firsts[1].1);
    assert!(
        difference.as_deref().is_some_and(|d| d.contains("77404")),
        "{difference:?}"
    );
    let planted = Check::default();
    planted.note(firsts[0].0.clone(), firsts[0].1.clone());
    planted.note(firsts[0].0.clone(), firsts[1].1.clone());
    assert!(planted.line().is_some_and(|l| l.contains("1 compared, 1 differ")));
}
