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
