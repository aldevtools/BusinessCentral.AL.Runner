//! Selection, reports, exit codes and the parity list.

use super::*;

#[test]
fn help_and_version_exit_0_and_say_what_the_command_does() {
    let o = alrunner(&["--help"]);
    assert_eq!(code(&o), 0);
    let t = text(&o);
    assert!(t.contains("al-runner test [OPTIONS] APP..."), "{t}");
    assert!(
        t.contains("al-runner debug [--stdio | --port [N]] [OPTIONS] APP..."),
        "{t}"
    );
    assert!(t.contains("--dep PATH") && t.contains("--package-cache DIR"), "{t}");
    assert!(t.contains("7  some tests were not run"), "{t}");
    assert_eq!(code(&alrunner(&["--version"])), 0);
}

#[test]
fn version_prints_the_crate_version_and_the_commit() {
    let o = alrunner(&["--version"]);
    let t = text(&o);
    let want = format!("al-runner {} (", env!("CARGO_PKG_VERSION"));
    assert!(t.starts_with(&want) && t.trim_end().ends_with(')'), "{t}");
    assert!(env!("CARGO_PKG_VERSION").starts_with("3.0.0-alpha."), "{t}");
}

#[test]
fn a_bad_invocation_exits_2_and_says_why() {
    for (args, why) in [
        (vec!["test"], "at least one APP"),
        (vec!["test", ".", "--nope"], "unknown option"),
        (vec!["test", ".", "--test-data", "--no-test-data"], "cannot be combined"),
        (vec!["test", ".", "--test-data", "/no/such/Demo.bak"], "Demo.bak"),
        (vec!["run"], "not implemented yet"),
        (vec!["debug"], "at least one APP"),
    ] {
        let o = alrunner(&args);
        assert_eq!(code(&o), 2, "{args:?}");
        assert!(text(&o).contains(why), "{args:?}: {}", text(&o));
    }
    let o = alrunner(&["test", "/no/such/app"]);
    assert_eq!(code(&o), 2);
    assert!(text(&o).contains("no such file or folder"), "{}", text(&o));
}

#[test]
fn a_run_reports_pass_fail_and_not_run_and_exits_1_on_a_failure() {
    let d = tempfile::tempdir().unwrap();
    let app = mixed_app(d.path());
    let o = alrunner(&["test", app.to_str().unwrap(), "--no-download", "--verbose"]);
    let t = text(&o);
    assert_eq!(code(&o), 1, "{t}");
    // Each line: the label, the run time in ms with three decimals, then the test.
    let line = |label: &str, rest: &str| {
        t.lines().any(|l| {
            l.strip_prefix(label).is_some_and(|r| {
                let r = r.trim_start();
                r.split_once(" ms ").is_some_and(|(n, tail)| {
                    n.trim().parse::<f64>().is_ok()
                        && n.trim().split('.').nth(1).is_some_and(|f| f.len() == 3)
                        && tail.starts_with(rest)
                })
            })
        })
    };
    assert!(line("PASS", "50100 Mixed Tests.Passes"), "{t}");
    assert!(line("PASS", "50101 Other Tests.AlsoPasses"), "{t}");
    assert!(line("FAIL", "50100 Mixed Tests.Fails: boom 5"), "{t}");
    assert!(line("NOT RUN", "50100 Mixed Tests.Refused: "), "{t}");
    assert!(!t.contains('\x1b'), "piped output is plain: {t}");
    assert!(t.contains("  passed "), "the summary table: {t}");
    assert!(t.contains("Tests: 4 total, 2 passed, 1 failed, 1 not run"), "{t}");
    // The app uses no Microsoft app (no platform or application in app.json), so it needs no CRONUS
    // company: the summary must not say its failures are untrusted for lack of test data.
    assert!(!t.contains("no test data"), "{t}");
    assert!(t.contains("Top refusals"), "{t}");
    assert!(!t.contains("Top failures"), "failures are ranked only on request: {t}");
    assert!(t.contains("Result: FAILED (exit 1)"), "{t}");
    assert!(t.contains("threads: check 2 (AL_CHECK_THREADS)"), "{t}");
    assert!(t.contains("Time: load "), "{t}");
}

#[test]
fn the_default_output_leaves_setup_details_to_verbose() {
    let d = tempfile::tempdir().unwrap();
    let app = mixed_app(d.path());
    let o = alrunner(&["test", app.to_str().unwrap(), "--no-download"]);
    let t = text(&o);
    assert_eq!(code(&o), 1, "{t}");
    assert!(t.starts_with("al-runner "), "the version line comes first: {t}");
    for kept in [
        "FAIL    50100 Mixed Tests.Fails: boom 5",
        "Top refusals",
        "Tests: 4 total",
        "Result: FAILED (exit 1)",
    ] {
        assert!(t.contains(kept), "{kept}: {t}");
    }
    for hidden in [
        "target: BC",
        "app: ",
        "build chosen by:",
        "dependencies: ",
        "threads: ",
        "isolation: ",
        "server address:",
        "Time: ",
        "dependency note",
        "baseline: ",
    ] {
        assert!(!t.contains(hidden), "{hidden} belongs to --verbose: {t}");
    }
    assert!(!t.contains("PASS    "), "one line per passing test is --verbose: {t}");
    // The default output keeps one compact time line, between the totals table and `Result:`.
    let time = t
        .lines()
        .position(|l| l.starts_with("  time "))
        .unwrap_or_else(|| panic!("no time line: {t}"));
    let l: Vec<&str> = t.lines().collect();
    assert!(
        l[time].contains("(compile ") && l[time].contains(", run ") && l[time].contains(" tests/s)"),
        "{t}"
    );
    assert!(l[time - 1].starts_with("  not run "), "the table comes first: {t}");
    assert!(l[time + 1].starts_with("Result: "), "{t}");
    let o = alrunner(&["test", app.to_str().unwrap(), "--no-download", "--quiet"]);
    let t = text(&o);
    assert!(!t.contains("threads: ") && !t.contains("Time: "), "{t}");
    assert!(t.contains("Tests: 4 total"), "{t}");
}

#[test]
fn not_run_tests_exit_7_unless_allowed() {
    let d = tempfile::tempdir().unwrap();
    let app = d.path().join("refused");
    write(
        &app,
        "app.json",
        &app_json("11111111-0000-0000-0000-000000000002", "Refused", &[]),
    );
    write(
        &app,
        "T.al",
        "codeunit 50200 \"R\" { Subtype = Test; [Test] procedure Ok() begin end; [Test] procedure No() begin Codeunit.Run(Codeunit::\"Missing\"); end; }",
    );
    let o = alrunner(&["test", app.to_str().unwrap(), "--no-download"]);
    assert_eq!(code(&o), 7, "{}", text(&o));
    assert!(text(&o).contains("Result: TESTS NOT RUN (exit 7)"), "{}", text(&o));
    let o = alrunner(&["test", app.to_str().unwrap(), "--no-download", "--allow-not-run"]);
    assert_eq!(code(&o), 0, "{}", text(&o));
}

#[test]
fn selection_by_codeunit_and_filter_runs_codeunits_whole_and_reports_the_selected_tests() {
    let d = tempfile::tempdir().unwrap();
    let app = mixed_app(d.path());
    let a = app.to_str().unwrap();
    let o = alrunner(&["test", a, "--no-download", "--codeunit", "50101", "--verbose"]);
    let t = text(&o);
    assert_eq!(code(&o), 0, "{t}");
    assert!(t.contains("Tests: 1 total, 1 passed"), "{t}");
    assert!(t.contains("selection: 1 test codeunits of 2"), "{t}");
    // A filter picks tests by name, any case; the other tests of the codeunit are not reported.
    let o = alrunner(&["test", a, "--no-download", "--filter", "ALSO", "--filter", "fails"]);
    let t = text(&o);
    assert_eq!(code(&o), 1, "{t}");
    assert!(t.contains("Tests: 2 total, 1 passed, 1 failed, 0 not run"), "{t}");
    // Nothing matches: exit 6.
    let o = alrunner(&["test", a, "--no-download", "--filter", "no such test"]);
    assert_eq!(code(&o), 6, "{}", text(&o));
}

#[test]
fn junit_has_a_suite_per_codeunit_and_skipped_with_the_reason() {
    let d = tempfile::tempdir().unwrap();
    let app = mixed_app(d.path());
    let out = d.path().join("reports/deep/junit.xml");
    let o = alrunner(&[
        "test",
        app.to_str().unwrap(),
        "--no-download",
        "--junit",
        out.to_str().unwrap(),
    ]);
    assert_eq!(code(&o), 1, "{}", text(&o));
    let x = std::fs::read_to_string(&out).unwrap();
    assert!(
        x.contains("<testsuites tests=\"4\" failures=\"1\" errors=\"0\" skipped=\"1\""),
        "{x}"
    );
    assert!(x.contains("<testsuite name=\"Mixed Tests\" tests=\"3\""), "{x}");
    assert!(x.contains("<testsuite name=\"Other Tests\" tests=\"1\""), "{x}");
    assert!(x.contains("<failure message=\"boom 5\""), "{x}");
    assert!(x.contains("<skipped message=\""), "{x}");
}

/// Microsoft's list of disabled tests for the `mixed` app: Fails by name, the whole Other Tests codeunit.
fn disabled_list(root: &Path) -> PathBuf {
    let dir = root.join("DisabledTests").join("Mixed");
    write(
        &dir,
        "a.json",
        r#"[{"codeunitId":50100,"codeunitName":"Mixed Tests","method":"Fails","bug":1},
           {"codeunitId":50101,"codeunitName":"Other Tests","method":"*"}]"#,
    );
    root.join("DisabledTests")
}

#[test]
fn disabled_tests_still_run_in_a_normal_run_and_are_marked() {
    let d = tempfile::tempdir().unwrap();
    let app = mixed_app(d.path());
    let list = disabled_list(d.path());
    let out = d.path().join("junit.xml");
    let o = alrunner(&[
        "test",
        app.to_str().unwrap(),
        "--no-download",
        "--disabled-tests",
        list.to_str().unwrap(),
        "--junit",
        out.to_str().unwrap(),
    ]);
    let t = text(&o);
    assert_eq!(code(&o), 1, "{t}");
    assert!(t.contains("Tests: 4 total, 2 passed, 1 failed, 1 not run"), "{t}");
    assert!(
        t.contains("Disabled by Microsoft: 2 of the tests run (1 passed, 1 failed, 0 not run; 2 entries of 1 files)"),
        "{t}"
    );
    let x = std::fs::read_to_string(&out).unwrap();
    assert_eq!(x.matches("disabled_by_microsoft").count(), 2, "{x}");
}

#[test]
fn parity_leaves_out_the_tests_microsoft_disables_and_says_how_many() {
    let d = tempfile::tempdir().unwrap();
    let app = mixed_app(d.path());
    let list = disabled_list(d.path());
    let o = alrunner(&[
        "test",
        app.to_str().unwrap(),
        "--no-download",
        "--disabled-tests",
        list.to_str().unwrap(),
        "--parity",
        "--verbose",
    ]);
    let t = text(&o);
    assert!(t.contains("Tests: 2 total, 1 passed, 0 failed, 1 not run"), "{t}");
    assert!(!t.contains("Mixed Tests.Fails"), "{t}");
    assert!(!t.contains("AlsoPasses"), "{t}");
    assert!(
        t.contains("Disabled by Microsoft: 2 tests not run (--parity; 2 entries of 1 files)"),
        "{t}"
    );
}

#[test]
fn parity_without_a_list_and_a_list_without_json_are_start_up_errors() {
    let d = tempfile::tempdir().unwrap();
    let app = mixed_app(d.path());
    let a = app.to_str().unwrap();
    let o = alrunner(&["test", a, "--no-download", "--parity"]);
    assert_eq!(code(&o), 2, "{}", text(&o));
    assert!(text(&o).contains("--parity needs --disabled-tests"), "{}", text(&o));
    let empty = d.path().join("empty");
    std::fs::create_dir_all(&empty).unwrap();
    let o = alrunner(&["test", a, "--no-download", "--disabled-tests", empty.to_str().unwrap()]);
    assert_ne!(code(&o), 0, "{}", text(&o));
    assert!(text(&o).contains("no *.json file"), "{}", text(&o));
}

#[test]
fn an_unwritable_junit_path_fails_before_the_run() {
    let d = tempfile::tempdir().unwrap();
    let app = mixed_app(d.path());
    let blocker = d.path().join("file");
    std::fs::write(&blocker, "x").unwrap();
    let bad = blocker.join("junit.xml");
    let o = alrunner(&[
        "test",
        app.to_str().unwrap(),
        "--no-download",
        "--junit",
        bad.to_str().unwrap(),
    ]);
    assert_eq!(code(&o), 2, "{}", text(&o));
    assert!(!text(&o).contains("Tests:"), "{}", text(&o));
}

#[test]
fn cache_help_prints_the_cache_commands_and_exits_0() {
    for args in [
        &["cache", "--help"][..],
        &["cache", "-h"],
        &["cache", "prune", "--help"],
    ] {
        let o = alrunner(args);
        assert_eq!(code(&o), 0, "{args:?}: {}", text(&o));
        let out = String::from_utf8_lossy(&o.stdout);
        for needle in ["al-runner cache prune", "al-runner cache info", "al-runner cache clear"] {
            assert!(out.contains(needle), "{args:?}: {needle}: {out}");
        }
        // Only the cache part of the help, not the whole of it.
        assert!(!out.contains("SELECTION"), "{args:?}: {out}");
    }
}
