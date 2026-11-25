// The scans of production code stop at the first `#[cfg(test)]` of a file; a test module in a file of its own starts with one.
#![cfg(test)]
//! A login start is found by what the login read (ADR-073 amendment 2026-11-25, al-runner issue 8211): the base key,
//! then the start whose texts of the resource files its login read (`al_vm::TextsRead`) are the session's texts of
//! them. A login that read no resource file is replayed under any texts; `NavApp.ListResources` reads every file of the
//! app. Over the stand-in of Microsoft's login and the sources of `login_replay_tests.rs`.

use super::login_replay_tests::{MAIN, compiled_with, run_as};
use super::*;
use crate::login_start::{self, starts};

/// The texts of the entry (a serve entry's `resources`, `Engine::resource_overrides`): per app (`main`, `dep`), its
/// files with their texts.
fn set_texts(engine: &Engine, l: &al_sema::lower::Lowered, texts: &[(&str, &str, &str)]) {
    let mut o = engine.resource_overrides.write().unwrap();
    o.clear();
    for (app, file, text) in texts {
        let index = l.program.apps.iter().position(|a| a.name == *app).unwrap();
        let files = Arc::make_mut(o.entry(index).or_default());
        files.push((file.to_string(), text.as_bytes().to_vec()));
    }
}

/// Runs `RPL Tests A` once with the replay on under the texts `texts`; the test must pass.
fn run_under(engine: &Arc<Engine>, l: &al_sema::lower::Lowered, texts: &[(&str, &str, &str)]) {
    set_texts(engine, l, texts);
    let opts = RunOptions {
        login_replay: true,
        ..Default::default()
    };
    let ran = run_as(engine, l, "RPL Tests A", opts);
    assert_eq!(
        ran.outcomes,
        vec![("SeesTheRow".to_string(), Outcome::Pass)],
        "{texts:?}"
    );
}

/// [`MAIN`] whose login subscriber reads no resource file: the row's id is fixed.
fn reads_none() -> String {
    MAIN.replace("StrLen(NavApp.GetResourceAsText('rpl.txt'))", "1")
}

/// [`MAIN`] whose login subscriber lists the app's resource files (`NavApp.ListResources`) and reads none of them.
fn lists() -> String {
    MAIN.replace(
        "StrLen(NavApp.GetResourceAsText('rpl.txt'))",
        "NavApp.ListResources('*').Count()",
    )
}

// (a) A login that reads no resource file leaves the same for any texts: a session of another entry's texts takes the
// start of the first.
#[test]
fn a_login_that_reads_no_resource_replays_across_entries_with_other_resources() {
    let (engine, l) = compiled_with(&reads_none());
    run_under(&engine, &l, &[("main", "rpl.txt", "a")]);
    run_under(&engine, &l, &[("main", "rpl.txt", "abcd")]);
    run_under(&engine, &l, &[]);
    assert_eq!(starts(&engine).kept(), 1, "one start, found under any texts");
    assert_eq!(starts(&engine).replayed(), 2, "the second and third entries replay it");
}

// (b) A login that reads `rpl.txt` is replayed only to a session whose text of `rpl.txt` is the text it read; another
// file's text does not count.
#[test]
fn a_login_that_reads_one_resource_replays_only_when_that_text_is_equal() {
    let (engine, l) = compiled_with(MAIN);
    run_under(&engine, &l, &[("main", "rpl.txt", "a"), ("main", "other.txt", "x")]);
    run_under(&engine, &l, &[("main", "rpl.txt", "a"), ("main", "other.txt", "y")]);
    assert_eq!(
        starts(&engine).replayed(),
        1,
        "a text the login did not read does not count"
    );
    run_under(&engine, &l, &[("main", "rpl.txt", "abcd"), ("main", "other.txt", "x")]);
    assert_eq!(
        starts(&engine).replayed(),
        1,
        "another text of rpl.txt runs its own login"
    );
    assert_eq!(starts(&engine).kept(), 2, "and keeps a start of its own");
    run_under(&engine, &l, &[("main", "rpl.txt", "abcd"), ("main", "other.txt", "z")]);
    run_under(&engine, &l, &[("main", "rpl.txt", "a")]);
    assert_eq!(
        starts(&engine).replayed(),
        3,
        "each text of rpl.txt finds its own start"
    );
}

// (c) `NavApp.ListResources` reads every file of the running app (`Session::resource_reads`): any text of that app
// counts, and a text of another app does not.
#[test]
fn list_resources_makes_every_text_of_the_app_count() {
    let (engine, l) = compiled_with(&lists());
    run_under(&engine, &l, &[("main", "rpl.txt", "a"), ("main", "other.txt", "x")]);
    run_under(&engine, &l, &[("main", "rpl.txt", "a"), ("main", "other.txt", "y")]);
    assert_eq!(
        starts(&engine).replayed(),
        0,
        "the login listed other.txt, so its text counts"
    );
    run_under(
        &engine,
        &l,
        &[
            ("main", "rpl.txt", "a"),
            ("main", "other.txt", "x"),
            ("dep", "d.txt", "1"),
        ],
    );
    assert_eq!(starts(&engine).replayed(), 1, "a text of another app does not count");
}

/// [`MAIN`] whose login subscriber takes the row's id from `RplLen()`, a procedure of its codeunit with the body `body`
/// (locals `T: Text`, `InS: InStream`, `J: JsonObject`) that reads `rpl.txt` through one path.
fn reading_by(body: &str) -> String {
    let len = format!(
        "local procedure RplLen(): Integer var T: Text; InS: InStream; J: JsonObject; begin {body} end;\n    \
         [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Guided Experience\", 'OnRegisterManualSetup'"
    );
    MAIN.replace("StrLen(NavApp.GetResourceAsText('rpl.txt'))", "RplLen()")
        .replacen(
            "[EventSubscriber(ObjectType::Codeunit, Codeunit::\"Guided Experience\", 'OnRegisterManualSetup'",
            &len,
            1,
        )
}

/// Runs the login of `src` under `rpl.txt` = `first`, then under `second`: the second session runs its own login (no
/// replay, a start of its own) and holds the row its own text gives (`77400 + ids.1`), not the first's (`77400 + ids.0`).
fn another_text_runs_its_own_login(src: &str, (first, second): (&str, &str), ids: (u32, u32)) {
    let (engine, l) = compiled_with(src);
    login_start::OPENED.with(|o| *o.borrow_mut() = Some(Vec::new()));
    run_under(&engine, &l, &[("main", "rpl.txt", first)]);
    run_under(&engine, &l, &[("main", "rpl.txt", second)]);
    let states = login_start::OPENED.with(|o| o.borrow_mut().take()).unwrap();
    assert_eq!(
        starts(&engine).replayed(),
        0,
        "another text of rpl.txt is never replayed"
    );
    assert_eq!(starts(&engine).kept(), 2, "each text keeps its own start");
    let holds = |s: usize, id: u32| {
        states[s]
            .rows()
            .iter()
            .any(|r| r.holds(&format!("Integer({})", 77400 + id)))
    };
    assert!(holds(0, ids.0), "the first session reads its text");
    assert!(
        holds(1, ids.1) && !holds(1, ids.0),
        "the second session reads its own text"
    );
}

// One test per read path of a resource file (al-runner issue 8211): each path's read is recorded, so the texts the
// login read hold the second session apart from the first's start. The three GetResource paths share one recording
// site (al-builtins `resource_bytes`, al-vm `running_app_resource`); ListResources has its own (`running_app_resources`).
#[test]
fn a_login_that_reads_by_get_resource_is_not_replayed_under_another_text() {
    let src = reading_by("NavApp.GetResource('rpl.txt', InS); InS.ReadText(T); exit(StrLen(T));");
    another_text_runs_its_own_login(&src, ("a", "abcd"), (1, 4));
}

#[test]
fn a_login_that_reads_by_get_resource_as_text_is_not_replayed_under_another_text() {
    let src = reading_by("exit(StrLen(NavApp.GetResourceAsText('rpl.txt')));");
    another_text_runs_its_own_login(&src, ("a", "abcd"), (1, 4));
}

#[test]
fn a_login_that_reads_by_get_resource_as_json_is_not_replayed_under_another_text() {
    let src = reading_by("J := NavApp.GetResourceAsJson('rpl.txt'); J.WriteTo(T); exit(StrLen(T));");
    another_text_runs_its_own_login(&src, (r#"{"a":1}"#, r#"{"a":12345}"#), (7, 11));
}

// ListResources reads the names of the app's files, not their texts, so both sessions hold the same row; that it counts
// as reading every file (`Session::resource_reads`) keeps the second session from the first's start.
#[test]
fn a_login_that_lists_the_resources_is_not_replayed_under_another_text() {
    let (engine, l) = compiled_with(&reading_by("exit(NavApp.ListResources('*').Count());"));
    run_under(&engine, &l, &[("main", "rpl.txt", "a")]);
    run_under(&engine, &l, &[("main", "rpl.txt", "abcd")]);
    assert_eq!(
        starts(&engine).replayed(),
        0,
        "another text of a listed file is never replayed"
    );
    assert_eq!(starts(&engine).kept(), 2, "each text keeps its own start");
}
