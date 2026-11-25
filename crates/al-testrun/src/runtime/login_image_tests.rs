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
    GAP, GAP_INSTANCES, GAP_MEDIA, GAP_RANDOM, KINDS_INSIDE, MAIN, OPAQUE_INSIDE, Ran, compiled_with, run, run_as,
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
        ..Scratch::new(crate::host::RunClock::begin())
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
    // The kinds the image defers (Json, Xml, Http) refuse; every reason names them as deferred.
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
    // The check line's text, exactly (`login_replay_tests.rs`, `sessions_of_one_key_hold_the_same_after_the_raised_login`).
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

// A twin that is not a new session (here: its runner permission step is on) does not take the image, and the thaw says so instead of
// panicking on the thread that runs it: the round trip is counted as not thawn, nothing is reported as a difference, and
// the session ran its own login, so the outcomes are those of a run with the check off (issue 8211).
#[test]
fn a_twin_the_thaw_refuses_is_counted_and_changes_no_outcome() {
    let (engine, l) = compiled_with(MAIN);
    crate::login_replay::TWIN_TARGET.with(|p| p.set(Some(|twin| twin.runner_permission_step = true)));
    let with = outcomes(&engine, &l, "RPL Tests A", true);
    crate::login_replay::TWIN_TARGET.with(|p| p.set(None));
    let (_, line) = report_lines(&engine);
    assert!(
        line.starts_with(
            "login image: 1 keys, 1 imaged, 0 not imaged; round trips 0, 0 differ, 1 not thawed (1: the target is not a \
             new session: runner_permission_step); per key: "
        ),
        "{line}"
    );
    let (engine, l) = compiled_with(MAIN);
    assert_eq!(with, outcomes(&engine, &l, "RPL Tests A", false));
}

/// A raised login that leaves state its tests read: a row, a SingleInstance codeunit's globals (an Integer, a Text, a
/// List, a Dictionary, a temporary record with two rows, an instance of another codeunit), a seeded Random sequence, a
/// work date and a manual binding. Each test but one fails on purpose with what it read, so its outcome holds the values.
const TWIN_READS: &str = r#"
codeunit 77470 "RPL Twin Registration"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Guided Experience", 'OnRegisterManualSetup', '', false, false)]
    local procedure RegisterManualSetup()
    var G: Codeunit "Guided Experience"; Keep: Codeunit "RPL Twin Keep";
    begin
        G.InsertAssistedSetup(77470);
        Keep.Fill();
    end;
}
codeunit 77471 "RPL Twin Keep"
{
    SingleInstance = true;
    var N: Integer; T: Text; Items: List of [Integer]; Names: Dictionary of [Integer, Text];
        Temp: Record "Guided Experience Item" temporary; Peer: Codeunit "RPL Twin Peer"; Sub: Codeunit "RPL Twin Sub";
    procedure Fill()
    begin
        N := 7; T := 'kept'; Items.Add(3); Items.Add(5); Names.Add(1, 'one');
        Temp."Object ID" := 1; Temp.Insert(); Temp."Object ID" := 2; Temp.Insert();
        Peer.SetX(11); Randomize(42); BindSubscription(Sub); WorkDate(20300105D);
    end;
    procedure Describe(): Text
    begin
        exit(StrSubstNo('%1 %2 %3 %4 %5 %6', N, T, Items.Get(2), Names.Get(1), Temp.Count(), Peer.GetX()));
    end;
    procedure Bump() begin N += 1; end;
    procedure Ping() begin OnPing(); end;
    [IntegrationEvent(false, false)]
    local procedure OnPing() begin end;
}
codeunit 77472 "RPL Twin Peer"
{
    var X: Integer;
    procedure SetX(V: Integer) begin X := V; end;
    procedure GetX(): Integer begin exit(X); end;
}
codeunit 77473 "RPL Twin Sub"
{
    EventSubscriberInstance = Manual;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RPL Twin Keep", 'OnPing', '', false, false)]
    local procedure HandlePing() var Keep: Codeunit "RPL Twin Keep"; begin Keep.Bump(); end;
}
codeunit 77474 "RPL Twin Tests"
{
    Subtype = Test;
    [Test]
    procedure SeesTheRow() var I: Record "Guided Experience Item"; begin if I.IsEmpty() then Error('no row'); end;
    [Test]
    procedure ReadsTheKeptState() var Keep: Codeunit "RPL Twin Keep"; begin Error('kept: %1', Keep.Describe()); end;
    [Test]
    procedure DrawsFromTheSeededSequence() begin Error('draws: %1 %2 %3', Random(1000), Random(1000), CreateGuid()); end;
    [Test]
    procedure ReadsTheWorkDate() begin Error('work date: %1', WorkDate()); end;
    [Test]
    procedure TheBindingStillHolds() var Keep: Codeunit "RPL Twin Keep"; begin Keep.Ping(); Error('pinged: %1', Keep.Describe()); end;
}
"#;

/// Runs `name` with the check on, on the session that ran its login or (`twin`) on a twin thawn from its image.
fn ran_on(source: &str, name: &str, twin: bool) -> Ran {
    let (engine, l) = compiled_with(source);
    crate::login_replay::SERVE_TWIN.with(|s| s.set((twin, 0)));
    let ran = run_as(
        &engine,
        &l,
        name,
        RunOptions {
            login_replay_check: true,
            ..RunOptions::default()
        },
    );
    let (_, served) = crate::login_replay::SERVE_TWIN.with(std::cell::Cell::get);
    crate::login_replay::SERVE_TWIN.with(|s| s.set((false, 0)));
    assert_eq!(served, u32::from(twin), "{name}: the sessions a twin replaced");
    ran
}

// The evidence at the level of test results the maintainer asked for before a replay serves tests (issue 8211): each
// codeunit, run on a twin thawn from its session's image in place of that session, gives the outcomes (the values its
// failing tests read included), the error stacks and the assumption marks of the session that ran its own login.
#[test]
fn a_codeunit_on_a_thawn_twin_gives_the_results_and_marks_of_a_fresh_login() {
    let gap_instances = format!("{GAP}{GAP_INSTANCES}");
    let gap_random = format!("{GAP}{GAP_RANDOM}");
    let gap_media = format!("{GAP}{GAP_MEDIA}");
    let reads = format!("{MAIN}{TWIN_READS}");
    let runs = [
        (MAIN, "RPL Tests A"),
        (gap_instances.as_str(), "RPL Gap Tests"),
        (gap_random.as_str(), "RPL Gap Tests"),
        (gap_media.as_str(), "RPL Gap Tests"),
        (reads.as_str(), "RPL Twin Tests"),
    ];
    for (source, name) in runs {
        let fresh = ran_on(source, name, false);
        assert!(!fresh.outcomes.is_empty(), "{name}: no test ran");
        assert_eq!(ran_on(source, name, true), fresh, "{name}");
    }
    // The tests read what the login left: the outcomes hold the values, not a refusal.
    let fresh = ran_on(&reads, "RPL Twin Tests", false);
    let text = format!("{:?}", fresh.outcomes);
    for want in [
        "kept: 7 kept 5 one 2 11",
        "pinged: 8 kept 5 one 2 11",
        "work date: 01/05/30",
    ] {
        assert!(text.contains(want), "{want}: {text}");
    }
}
