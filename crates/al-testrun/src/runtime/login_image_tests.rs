// The scans of production code stop at the first `#[cfg(test)]` of a file; a test module in a file of its own starts with one.
#![cfg(test)]
//! The session image on real runs (`login_replay.rs` `Check::round_trip`, al-vm `login_image.rs`, al-runner issue 8211):
//! under the login replay check, each new key's session after the raised login is frozen, thawn into a twin and compared,
//! over the stand-in of Microsoft's login (`login_tests::DEP`) and the sources of `login_replay_tests.rs`.
//!
//! The two conditions of the maintainer stay in `login_replay_tests.rs` and `login_replay.rs` (content or SHA-256 in the
//! key, no 64-bit digest; the test codeunit stays in the key). The image adds no key field: `another_test_codeunit_is_another_key`
//! there still holds with the image on, and the last test here shows the first report line is unchanged by it.

use super::login_replay_tests::{
    GAP, GAP_INSTANCES, GAP_MEDIA, GAP_RANDOM, KINDS_INSIDE, MAIN, OPAQUE_INSIDE, compiled_with, run,
};
use super::*;

/// The two report lines of `engine`'s check: the check line and the image line.
fn report_lines(engine: &Engine) -> (String, String) {
    let report = crate::login_replay::report(engine).expect("a report");
    let (check, image) = report
        .split_once('\n')
        .unwrap_or_else(|| panic!("no image line in: {report}"));
    (check.to_string(), image.to_string())
}

/// Runs "RPL Tests A" once with the check on and returns the report's image line.
fn image_line_of(sources: &[&str]) -> String {
    let (engine, l) = compiled_with(&sources.concat());
    run(&engine, &l, "RPL Tests A");
    report_lines(&engine).1
}

/// The outcomes of test codeunit `name`, with the check on or off.
fn outcomes(engine: &Arc<Engine>, l: &al_sema::lower::Lowered, name: &str, check: bool) -> Vec<(String, Outcome)> {
    let mut scratch = Scratch {
        flags: super::session_flags::SessionFlags::of(&RunOptions {
            login_replay_check: check,
            ..RunOptions::default()
        }),
        ..Default::default()
    };
    let oi = l.program.objects.iter().position(|o| o.name == name).unwrap();
    run_codeunit_core(
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
    )
}

#[test]
fn the_thawn_twin_of_a_raised_login_holds_what_the_session_holds() {
    let line = image_line_of(&[MAIN]);
    assert!(
        line.starts_with("login image: 1 keys, 1 imaged, 0 not imaged; round trips 1, 0 differ"),
        "{line}"
    );
}

#[test]
fn shared_instances_temporary_tables_the_clock_and_random_round_trip() {
    // MAIN keeps temporary records, one shared (`Copy(x, true)`), in a SingleInstance codeunit; GAP_INSTANCES holds
    // instances and a manual binding, GAP_RANDOM reseeds Random, GAP_MEDIA imports a media object.
    for plant in [GAP_INSTANCES, GAP_RANDOM, GAP_MEDIA] {
        let (engine, l) = compiled_with(&format!("{GAP}{plant}"));
        run(&engine, &l, "RPL Gap Tests");
        let line = report_lines(&engine).1;
        assert!(
            line.starts_with("login image: 1 keys, 1 imaged, 0 not imaged; round trips 1, 0 differ"),
            "{line}"
        );
    }
    let line = image_line_of(&[MAIN]);
    assert!(line.contains("round trips 1, 0 differ"), "{line}");
}

#[test]
fn every_kind_of_value_round_trips_or_is_named_as_deferred() {
    let line = image_line_of(&[MAIN, KINDS_INSIDE]);
    assert!(line.contains(", 0 differ"), "{line}");
    // The kinds with no form in step 2 (Json, Xml, Http) refuse; every reason names them as deferred.
    assert!(line.contains("(deferred)"), "{line}");
    for reason in line.split("; ").filter(|p| p.contains("not imaged (")) {
        assert!(reason.contains("(deferred)"), "a reason that is not deferred: {reason}");
    }
}

#[test]
fn a_planted_difference_in_the_twin_is_reported() {
    crate::login_replay::TWIN_PLANT.with(|p| p.set(Some(|twin| twin.set_work_date(1))));
    let line = image_line_of(&[MAIN]);
    crate::login_replay::TWIN_PLANT.with(|p| p.set(None));
    assert!(
        line.contains("round trips 1, 1 differ (session state 1; first: session state 1); per key: "),
        "{line}"
    );
}

#[test]
fn a_session_the_image_refuses_is_counted_and_changes_no_outcome() {
    let (engine, l) = compiled_with(&format!("{MAIN}{OPAQUE_INSIDE}"));
    let with = outcomes(&engine, &l, "RPL Tests A", true);
    let (_, line) = report_lines(&engine);
    assert!(line.contains(", 1 not imaged (1: "), "{line}");
    assert!(line.contains("round trips 0"), "{line}");
    let (engine, l) = compiled_with(&format!("{MAIN}{OPAQUE_INSIDE}"));
    assert_eq!(with, outcomes(&engine, &l, "RPL Tests A", false));
    assert!(crate::login_replay::report(&engine).is_none(), "no check, no report");
}

#[test]
fn the_check_line_keeps_its_shape_and_the_image_line_follows() {
    let (engine, l) = compiled_with(MAIN);
    for _ in 0..3 {
        run(&engine, &l, "RPL Tests A");
    }
    let (check, image) = report_lines(&engine);
    // Step 1's text, exactly (`login_replay_tests.rs`, `sessions_of_one_key_hold_the_same_after_the_raised_login`).
    assert_eq!(
        check,
        "login replay check: 3 sessions over 1 keys (a key seen before: 2 sessions, 66%), 2 compared, 0 differ; across test codeunits: 0 compared, 0 differ"
    );
    // One key, so one image and one round trip, however many sessions came.
    assert!(
        image.starts_with("login image: 1 keys, 1 imaged, 0 not imaged; round trips 1, 0 differ"),
        "{image}"
    );
}
