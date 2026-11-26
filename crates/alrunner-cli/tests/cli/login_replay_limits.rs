//! The caps of the login replay's starts (al-runner issue 8211): `ALRUNNER_LOGIN_REPLAY_MAX_IMAGES` and
//! `ALRUNNER_LOGIN_REPLAY_MAX_MB`, read next to `ALRUNNER_LOGIN_REPLAY`. A bad value stops the run with an error that
//! names the variable; unset or good, the run goes on as before.

use super::*;

const TESTS: &str = r#"
codeunit 50320 "Replay Cap Tests"
{
    Subtype = Test;

    [Test]
    procedure Passes()
    begin
    end;
}
"#;

fn cap_app(root: &Path) -> PathBuf {
    one_file_app(
        root,
        "caps",
        "11111111-0000-0000-0000-000000000032",
        "Replay Caps",
        "src/Tests.al",
        TESTS,
    )
}

fn run_with(env: &[(&str, &str)]) -> Output {
    let d = tempfile::tempdir().unwrap();
    let app = cap_app(d.path());
    let mut env = env.to_vec();
    env.push(("ALRUNNER_LOGIN_REPLAY", "1"));
    alrunner_with(&["test", app.to_str().unwrap(), "--no-download"], &env)
}

#[test]
fn set_or_unset_caps_run_the_tests() {
    for env in [
        &[][..],
        &[
            ("ALRUNNER_LOGIN_REPLAY_MAX_IMAGES", "32"),
            ("ALRUNNER_LOGIN_REPLAY_MAX_MB", "512"),
        ][..],
    ] {
        let o = run_with(env);
        let t = text(&o);
        assert_eq!(code(&o), 0, "{t}");
        assert!(t.contains("PASS"), "{t}");
    }
}

#[test]
fn a_bad_cap_stops_the_run_with_an_error_that_names_the_variable() {
    for (name, bad) in [
        ("ALRUNNER_LOGIN_REPLAY_MAX_IMAGES", "0"),
        ("ALRUNNER_LOGIN_REPLAY_MAX_IMAGES", "many"),
        ("ALRUNNER_LOGIN_REPLAY_MAX_MB", "-5"),
        ("ALRUNNER_LOGIN_REPLAY_MAX_MB", "99999999999999999999"),
    ] {
        let o = run_with(&[(name, bad)]);
        let t = text(&o);
        assert_ne!(code(&o), 0, "{name}={bad}: {t}");
        assert!(
            t.contains(&format!("{name} takes a whole number from 1 to ")),
            "{name}={bad}: {t}"
        );
        assert!(t.contains(&format!("not `{bad}`")), "{name}={bad}: {t}");
        assert!(!t.contains("PASS"), "no test ran: {t}");
    }
}

/// The start check (al-runner issue 8211): with the replay on and `--max-memory` set, a byte cap that is not below the
/// limit stops the run before it starts, with an error that names both values.
#[test]
fn a_byte_cap_not_below_max_memory_stops_the_run_naming_both_values() {
    let d = tempfile::tempdir().unwrap();
    let app = cap_app(d.path());
    let run = |env: &[(&str, &str)], limit: &str| {
        alrunner_with(
            &["test", app.to_str().unwrap(), "--no-download", "--max-memory", limit],
            env,
        )
    };
    let on = ("ALRUNNER_LOGIN_REPLAY", "1");
    for (env, limit, cap) in [
        (vec![on], "64M", "64 MB"),
        (vec![on], "32M", "64 MB"),
        (vec![on, ("ALRUNNER_LOGIN_REPLAY_MAX_MB", "512")], "300M", "512 MB"),
        (vec![("ALRUNNER_LOGIN_REPLAY_CHECK", "1")], "32M", "64 MB"),
    ] {
        let o = run(&env, limit);
        let t = text(&o);
        let want = format!(
            "ALRUNNER_LOGIN_REPLAY_MAX_MB is {cap}, not below the memory limit of {} MB",
            limit.trim_end_matches('M')
        );
        assert_eq!(code(&o), 2, "{t}");
        assert!(t.contains(&want), "{want}: {t}");
        assert!(!t.contains("PASS"), "no test ran: {t}");
    }
    // A cap below the limit, or no replay: no such stop (the run may still stop for the limit itself).
    for env in [vec![on, ("ALRUNNER_LOGIN_REPLAY_MAX_MB", "16")], vec![]] {
        let t = text(&run(&env, "32M"));
        assert!(!t.contains("not below the memory limit"), "{t}");
    }
}
