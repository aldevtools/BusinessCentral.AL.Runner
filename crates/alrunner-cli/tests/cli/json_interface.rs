//! The interface the v2 tools use (docs/integrations/alchemist-lethal.md, step 1).

use super::*;

#[test]
fn output_json_without_a_file_is_the_only_thing_on_stdout_and_has_a_row_per_test() {
    let d = tempfile::tempdir().unwrap();
    let app = mixed_app(d.path());
    let o = alrunner(&["test", app.to_str().unwrap(), "--no-download", "--output-json"]);
    assert_eq!(code(&o), 1, "{}", text(&o));
    // The report is on stderr, the document alone on stdout, starting with `{` in column 0.
    assert!(String::from_utf8_lossy(&o.stdout).starts_with("{\n"));
    assert!(String::from_utf8_lossy(&o.stderr).contains("Result: FAILED (exit 1)"));
    let v = json_of(&o);
    assert_eq!(
        (
            v["total"].as_u64(),
            v["passed"].as_u64(),
            v["failed"].as_u64(),
            v["errors"].as_u64()
        ),
        (Some(4), Some(2), Some(1), Some(1))
    );
    assert_eq!((v["skipped"].as_u64(), v["exitCode"].as_u64()), (Some(0), Some(1)));
    assert_eq!(row(&v, "Codeunit50100.Passes")["status"], "pass");
    let fails = row(&v, "Codeunit50100.Fails");
    assert_eq!(
        (fails["status"].as_str(), fails["message"].as_str()),
        (Some("fail"), Some("boom 5"))
    );
    assert!(
        fails["stackTrace"].as_str().is_some_and(|s| s.contains("Fails")),
        "{fails}"
    );
    let refused = row(&v, "Codeunit50100.Refused");
    assert_eq!(refused["status"], "error");
    assert!(
        refused["message"]
            .as_str()
            .is_some_and(|m| m.starts_with("NotRun: out-of-scope: ")),
        "{refused}"
    );
    assert_eq!(refused["notRun"], true);
}

#[test]
fn output_json_with_a_file_keeps_the_report_on_stdout() {
    let d = tempfile::tempdir().unwrap();
    let app = mixed_app(d.path());
    let out = d.path().join("r/out.json");
    let o = alrunner(&[
        "test",
        app.to_str().unwrap(),
        "--no-download",
        "--output-json",
        out.to_str().unwrap(),
    ]);
    assert_eq!(code(&o), 1, "{}", text(&o));
    assert!(String::from_utf8_lossy(&o.stdout).contains("Result: FAILED (exit 1)"));
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
    assert_eq!(v["total"], 4);
}

#[test]
fn a_qualified_test_name_selects_one_test_and_exclude_test_removes_tests() {
    let d = tempfile::tempdir().unwrap();
    let app = mixed_app(d.path());
    let a = app.to_str().unwrap();
    let run = |extra: &[&str]| {
        let mut args = vec!["test", a, "--no-download", "--output-json"];
        args.extend_from_slice(extra);
        alrunner(&args)
    };
    let o = run(&["--test", "Codeunit50100.Passes"]);
    assert_eq!(code(&o), 0, "{}", text(&o));
    let v = json_of(&o);
    assert_eq!(v["total"], 1);
    assert_eq!(row(&v, "Codeunit50100.Passes")["status"], "pass");
    // The qualified name is matched in any case; a sibling of another codeunit with the same method name is not picked.
    let o = run(&["--test", "codeunit50101.alsopasses"]);
    assert_eq!(json_of(&o)["total"], 1);
    // Nothing matches: exit 6.
    assert_eq!(code(&run(&["--test", "Codeunit50100.AlsoPasses"])), 6);
    // --test-exact.
    let v = json_of(&run(&["--test-exact", "Passes"]));
    assert_eq!(v["total"], 1);
    // --exclude-test leaves out whole qualified names and wins over the selection.
    let o = run(&[
        "--exclude-test",
        "Codeunit50100.Fails",
        "--exclude-test",
        "Codeunit50100.Refused",
    ]);
    assert_eq!(code(&o), 0, "{}", text(&o));
    assert_eq!(json_of(&o)["total"], 2);
    assert_eq!(
        code(&run(&[
            "--test",
            "Codeunit50100.Passes",
            "--exclude-test",
            "Codeunit50100.Passes"
        ])),
        6
    );
}

const ISO: &str = r#"
table 50300 "Iso Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
codeunit 50300 "Iso Tests"
{
    Subtype = Test;

    [Test]
    procedure A_Inserts()
    var
        R: Record "Iso Row";
    begin
        R.Id := 1;
        R.Insert();
    end;

    [Test]
    procedure B_SeesTheRow()
    var
        R: Record "Iso Row";
    begin
        if R.Count() <> 0 then
            Error('kept');
    end;
}
"#;

#[test]
fn isolation_decides_whether_the_database_is_rolled_back_between_the_tests_of_a_codeunit() {
    let d = tempfile::tempdir().unwrap();
    let app = d.path().join("iso");
    write(
        &app,
        "app.json",
        &app_json("11111111-0000-0000-0000-000000000003", "Iso", &[]),
    );
    write(&app, "T.al", ISO);
    let a = app.to_str().unwrap();
    let run = |extra: &[&str]| {
        let mut args = vec!["test", a, "--no-download", "--no-test-data", "--output-json"];
        args.extend_from_slice(extra);
        alrunner(&args)
    };
    // The default, Codeunit isolation: the second test sees the first one's row.
    let o = run(&[]);
    assert_eq!(code(&o), 1, "{}", text(&o));
    assert_eq!(row(&json_of(&o), "Codeunit50300.B_SeesTheRow")["message"], "kept");
    // test (and its v2 and BC spellings): the database goes back after every test.
    for mode in ["test", "function", "method"] {
        let o = run(&["--isolation", mode]);
        assert_eq!(code(&o), 0, "{mode}: {}", text(&o));
        assert_eq!(json_of(&o)["passed"], 2);
    }
    // disabled: the passing test's row stays for the next test, as under a runner with TestIsolation = Disabled
    // (corpus PR 735, IsolProbeWriter.Codeunit.al W3 under runner 61304).
    let o = run(&["--isolation", "disabled"]);
    assert_eq!(code(&o), 1, "{}", text(&o));
    assert_eq!(row(&json_of(&o), "Codeunit50300.B_SeesTheRow")["message"], "kept");
    assert_eq!(row(&json_of(&o), "Codeunit50300.A_Inserts")["status"], "pass");
    assert_eq!(code(&run(&["--isolation", "nope"])), 2);
}

/// A test that reports the time of day it reads, the way `Any.SetDefaultSeed` and `LibraryRandom.Init` seed the random
/// generator from it (`Randomize(Time - 000000T)`).
const CLOCK: &str = r#"
codeunit 50310 "Clock Tests"
{
    Subtype = Test;

    [Test]
    procedure ReportsTheTime()
    begin
        Error('%1', Time - 000000T);
    end;
}
"#;

/// Two runs that start at different moments read the same time of day: the run clock starts at 12:00:00 UTC of the day
/// (ADR-045 amendment), so an outcome that depends on `Time` does not depend on when the run, or the run that kept its
/// install baseline, began. Before, the clock started at the wall time of the first read: `Job Posting.
/// PurchaseOrderFCYWithJob` (Tests-Job, codeunit 136309) seeds `Random` from `Time` in an earlier test of its codeunit
/// and passed or failed depending on which other codeunits were selected (ADR-043 condition 1).
#[test]
fn the_run_clock_starts_at_the_same_time_of_day_in_every_run() {
    let d = tempfile::tempdir().unwrap();
    let app = d.path().join("clock");
    write(
        &app,
        "app.json",
        &app_json("11111111-0000-0000-0000-000000000031", "Clock", &[]),
    );
    write(&app, "T.al", CLOCK);
    let a = app.to_str().unwrap();
    let time = || {
        let o = alrunner(&["test", a, "--no-download", "--no-test-data", "--output-json"]);
        assert_eq!(code(&o), 1, "{}", text(&o));
        row(&json_of(&o), "Codeunit50310.ReportsTheTime")["message"]
            .as_str()
            .expect("a message")
            .to_string()
    };
    let first = time();
    std::thread::sleep(std::time::Duration::from_millis(5));
    let second = time();
    assert_eq!(
        first, second,
        "the time of day a test reads depends on when its run began"
    );
    // The first read of a session is the run clock's start: noon (12 h after midnight, in ms).
    assert_eq!(first.replace(',', ""), "43200000");
}

#[test]
fn define_adds_a_preprocessor_symbol_and_the_v2_flags_are_accepted() {
    let d = tempfile::tempdir().unwrap();
    let app = d.path().join("defs");
    write(
        &app,
        "app.json",
        &app_json("11111111-0000-0000-0000-000000000004", "Defs", &[]),
    );
    write(
        &app,
        "T.al",
        "codeunit 50400 \"Defs\" { Subtype = Test; [Test] procedure Symbol() begin\n#if LETHAL_MUTANT\n Error('mutant');\n#endif\n end; }",
    );
    let a = app.to_str().unwrap();
    assert_eq!(code(&alrunner(&["test", a, "--no-download", "--no-test-data"])), 0);
    let o = alrunner(&[
        "test",
        a,
        "--no-download",
        "--no-test-data",
        "--define",
        "LETHAL_MUTANT",
        "--auto-provision",
        "--test-timeout",
        "30",
    ]);
    assert_eq!(code(&o), 1, "{}", text(&o));
    assert!(text(&o).contains("mutant"), "{}", text(&o));
    // The v2 variable is read; a value that is no number is an error.
    let o = Command::new(env!("CARGO_BIN_EXE_al-runner"))
        .env("ALRUNNER_NO_PATH_NOTE", "1")
        .env("AL_RUNNER_TEST_TIMEOUT_SEC", "soon")
        .args(["test", a, "--no-download", "--no-test-data"])
        .output()
        .unwrap();
    assert_eq!(code(&o), 2, "{}", text(&o));
    assert!(text(&o).contains("AL_RUNNER_TEST_TIMEOUT_SEC"), "{}", text(&o));
}

/// Al-runner issue 5605: `Sleep` does not wait, but the time it asks for, summed over the test, counts toward
/// `--test-timeout`. A test whose Sleeps go past the limit gets the row of a test the CPU limit stopped
/// (`Test exceeded <N>s timeout.`, `notRun: true`, exit code 7); Sleeps that stay under it pass.
#[test]
fn sleep_time_past_the_test_timeout_is_reported_as_a_timeout() {
    let d = tempfile::tempdir().unwrap();
    let app = d.path().join("sleepy");
    write(
        &app,
        "app.json",
        &app_json("11111111-0000-0000-0000-000000000041", "Sleepy", &[]),
    );
    write(
        &app,
        "T.al",
        "codeunit 50410 \"Sleepy\" { Subtype = Test;
            [Test] procedure PollsForEver() var Done: Boolean; begin repeat Sleep(500); until Done; end;
            [Test] procedure LongSleep() begin Sleep(600000); Error('after the sleep'); end;
            [Test] procedure UnderTheLimit() begin Sleep(2000); Sleep(999); end; }",
    );
    let a = app.to_str().unwrap();
    let started = std::time::Instant::now();
    let o = alrunner(&[
        "test",
        a,
        "--no-download",
        "--no-test-data",
        "--test-timeout",
        "3",
        "--output-json",
    ]);
    assert!(started.elapsed() < std::time::Duration::from_secs(3), "Sleep waited");
    assert_eq!(code(&o), 7, "{}", text(&o));
    let j = json_of(&o);
    for t in ["PollsForEver", "LongSleep"] {
        let r = row(&j, &format!("Codeunit50410.{t}"));
        assert_eq!(r["message"], "Test exceeded 3s timeout.", "{r}");
        assert_eq!(r["notRun"], true, "{r}");
        // Issue 5793: the Sleep time stopped it, not the real clock. The row says so, and `durationMs` stays the real
        // time the test took (far below the 3 s it names).
        assert_eq!(r["clock"], "virtual", "{r}");
        assert!(r.get("durationMs").map_or(0, |d| d.as_u64().unwrap()) < 3000, "{r}");
    }
    let under = row(&j, "Codeunit50410.UnderTheLimit");
    assert_eq!(under["status"], "pass");
    assert!(under.get("clock").is_none(), "only a stopped test has `clock`: {under}");
}

/// LethAL asked for the summed `Sleep` time of a test as its own field: `sleptMs` on the row of `--output-json` is the
/// time the test's `Sleep` calls asked for (they return at once, so `durationMs` does not hold it). A test without a
/// `Sleep` has no `sleptMs`; a test the Sleep time stopped has the sum that stopped it.
#[test]
fn output_json_has_the_slept_time_of_each_test() {
    let d = tempfile::tempdir().unwrap();
    let app = d.path().join("sleeps");
    write(
        &app,
        "app.json",
        &app_json("11111111-0000-0000-0000-000000000042", "Sleeps", &[]),
    );
    write(
        &app,
        "T.al",
        "codeunit 50411 \"Sleeps\" { Subtype = Test;
            [Test] procedure Twice() begin Sleep(2000); Sleep(999); end;
            [Test] procedure NoSleep() begin end;
            [Test] procedure SleepsThenFails() begin Sleep(40); Error('after the sleep'); end;
            [Test] procedure PastTheLimit() begin Sleep(2500); Sleep(2500); end; }",
    );
    let a = app.to_str().unwrap();
    let o = alrunner(&[
        "test",
        a,
        "--no-download",
        "--no-test-data",
        "--test-timeout",
        "3",
        "--output-json",
    ]);
    let j = json_of(&o);
    assert_eq!(row(&j, "Codeunit50411.Twice")["sleptMs"], 2999, "{j}");
    assert!(row(&j, "Codeunit50411.NoSleep").get("sleptMs").is_none(), "{j}");
    assert_eq!(row(&j, "Codeunit50411.SleepsThenFails")["sleptMs"], 40, "{j}");
    assert_eq!(row(&j, "Codeunit50411.PastTheLimit")["sleptMs"], 5000, "{j}");
}
