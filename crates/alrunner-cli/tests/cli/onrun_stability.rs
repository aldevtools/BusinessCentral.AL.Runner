//! OnRun of test codeunits and the stability mode.

use super::*;

/// Al-runner issue 5748: a test codeunit whose OnRun raises, one whose OnRun does not, one without OnRun.
const ONRUN: &str = r#"
codeunit 50981 "R537 OnRun Tests"
{
    Subtype = Test;

    trigger OnRun()
    begin
        Error('boom 537 onrun');
    end;

    [Test]
    procedure AfterThrowingOnRun()
    begin
    end;

    [Test]
    procedure AfterThrowingOnRunTwo()
    begin
    end;
}

codeunit 50982 "R537 Quiet OnRun Tests"
{
    Subtype = Test;

    trigger OnRun()
    begin
    end;

    [Test]
    procedure QuietPasses()
    begin
    end;
}

codeunit 50983 "R537 No OnRun Tests"
{
    Subtype = Test;

    [Test]
    procedure PlainPasses()
    begin
    end;
}
"#;

fn onrun_app(root: &Path) -> PathBuf {
    one_file_app(
        root,
        "onrun",
        "11111111-0000-0000-0000-000000005748",
        "OnRun",
        "src/Tests.al",
        ONRUN,
    )
}

fn names(doc: &serde_json::Value) -> Vec<String> {
    doc["tests"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect()
}

// ADR-055 (al-runner issue 5859): the rows of Microsoft's Test Runner app (page 130455, runner codeunit 130450, as
// BcContainerHelper and AL-Go run it) on every major: no `OnRun` row; a codeunit whose OnRun raises is Skipped with no
// rows and the run stays green. Measured by the reporter on BC 29.0 (issue 5859) and by corpus writer RL on BC 27.5
// (probe commit c69419f6); the passes rest on the assumption `test-codeunit-onrun-rows` until the corpus pins land.
#[test]
fn a_test_codeunit_whose_on_run_raises_is_skipped_like_the_test_runner_app_on_every_major() {
    let d = tempfile::tempdir().unwrap();
    let app = onrun_app(d.path());
    let junit = d.path().join("junit.xml");
    for version in ["27.5.46862.48827", "28.5.0.0", "29.0.0.0"] {
        let o = alrunner(&[
            "test",
            app.to_str().unwrap(),
            "--no-download",
            "--bc-version",
            version,
            "--output-json",
            "--junit",
            junit.to_str().unwrap(),
        ]);
        assert_eq!(code(&o), 0, "{version}: {}", text(&o));
        let v = json_of(&o);
        assert_eq!(
            names(&v),
            ["Codeunit50982.QuietPasses", "Codeunit50983.PlainPasses"],
            "{version}: {v}"
        );
        // Exactly the Test Runner app's shape: nothing in the outputs names the skipped codeunit.
        let all = format!("{}{v}", text(&o));
        assert!(!all.contains("50981") && !all.contains("boom 537"), "{version}: {all}");
        let x = std::fs::read_to_string(&junit).unwrap();
        assert!(
            !x.contains("name=\"OnRun\"") && !x.contains("50981") && !x.contains("boom 537"),
            "{version}: {x}"
        );
        // The pass after a declared OnRun rests on the unprobed shape.
        assert!(
            String::from_utf8_lossy(&o.stderr).contains("Tests: 2 total, 2 passed (1 flagged"),
            "{version}: {}",
            text(&o)
        );
        assert!(x.contains("test-codeunit-onrun-rows"), "{version}: {x}");
    }
    // A selection of one test of the skipped codeunit reports nothing and stays green.
    let o = alrunner(&[
        "test",
        app.to_str().unwrap(),
        "--no-download",
        "--output-json",
        "--filter",
        "AfterThrowingOnRunTwo",
    ]);
    assert!(names(&json_of(&o)).is_empty(), "{}", text(&o));
}

/// The compile errors of al-runner issues 5854, 5852 and 5853 stop the run with exit 3 like any other: two codeunits
/// with one id (AL0264; before, the link step panicked with exit 2), exit(value) in a procedure without a return value
/// (AL0139; before, the test passed) and a call with too few arguments on the path the test calls (AL0135; before,
/// exit 7 with the test not run).
#[test]
fn duplicate_ids_exit_values_and_missing_arguments_stop_the_run() {
    let d = tempfile::tempdir().unwrap();
    let app = d.path().join("app");
    write(
        &app,
        "app.json",
        &app_json("11111111-0000-0000-0000-000000005854", "Logic", &[]),
    );
    let tests = d.path().join("tests");
    write(
        &tests,
        "app.json",
        &app_json(
            "11111111-0000-0000-0000-000000005855",
            "Logic Tests",
            &[("11111111-0000-0000-0000-000000005854", "Logic")],
        ),
    );
    write(
        &tests,
        "src/Tests.al",
        "codeunit 50650 \"Probe Tests\" { Subtype = Test; [Test] procedure Passes() var L: Codeunit \"Probe Logic\"; begin if L.Add(2, 3) <> 5 then Error('bad'); end; }",
    );
    let logic = "codeunit 50600 \"Probe Logic\" {\n    procedure Add(A: Integer; B: Integer): Integer begin exit(A + B); end;\n}\n";
    let run = || {
        alrunner(&[
            "test",
            app.to_str().unwrap(),
            tests.to_str().unwrap(),
            "--no-download",
            "--codeunit",
            "50650",
        ])
    };
    for (extra, want) in [
        (
            "codeunit 50600 \"Probe Dup\" { }",
            "error AL0264: An application object of type 'Codeunit' with ID '50600' is already declared by the extension",
        ),
        (
            "codeunit 50601 \"Probe Bump\" { procedure Bump() begin exit(5); end; }",
            "error AL0139: Since 'Bump' doesn't have a return value, EXIT cannot be called with a value",
        ),
    ] {
        write(&app, "src/Logic.al", logic);
        write(&app, "src/Extra.al", extra);
        let o = run();
        assert_eq!(code(&o), 3, "{}", text(&o));
        assert!(text(&o).contains(want), "{}", text(&o));
    }
    std::fs::remove_file(app.join("src/Extra.al")).unwrap();
    write(
        &app,
        "src/Logic.al",
        "codeunit 50600 \"Probe Logic\" {\n    procedure Add(A: Integer; B: Integer): Integer begin exit(Twice(A) - A + B); end;\n    procedure Twice(A: Integer): Integer begin exit(Add(A)); end;\n}\n",
    );
    let o = run();
    assert_eq!(code(&o), 3, "{}", text(&o));
    assert!(
        text(&o).contains(
            "error AL0135: There is no argument given that corresponds to the required formal parameter 'B' of 'Add(Integer, Integer)'"
        ),
        "{}",
        text(&o)
    );
}

/// ADR-058: `--stability microsoft` runs the tests once per variant over one compile and reports the tests whose
/// outcome changes: an order-dependent test flips under "each test alone" and "reverse order", a test that needs the
/// work date to be today flips under "work date plus one year". A flip is exit 1; `--stability-allow-flips` keeps 0.
#[test]
fn stability_mode_reports_order_and_date_dependent_tests() {
    let d = tempfile::tempdir().unwrap();
    let dir = d.path().join("stab");
    write(
        &dir,
        "app.json",
        &app_json("11111111-0000-0000-0000-000000005800", "Stability", &[]),
    );
    write(
        &dir,
        "src/Tests.al",
        "codeunit 50800 \"Stability Tests\"\n{\n    Subtype = Test;\n\n    var\n        Counter: Integer;\n\n    [Test]\n    procedure First()\n    begin\n        Counter += 1;\n    end;\n\n    [Test]\n    procedure SecondNeedsFirst()\n    begin\n        if Counter <> 1 then\n            Error('counter=%1', Counter);\n    end;\n\n    [Test]\n    procedure WorkDateIsToday()\n    begin\n        if WorkDate() <> Today() then\n            Error('work date %1', WorkDate());\n    end;\n\n    [Test]\n    procedure NotOnLeapDay()\n    begin\n        if (Date2DMY(Today(), 1) = 29) and (Date2DMY(Today(), 2) = 2) then\n            Error('leap day');\n    end;\n}\n",
    );
    let a = dir.to_str().unwrap();
    let json = d.path().join("r.json");
    let junit = d.path().join("r.xml");
    let o = alrunner(&[
        "test",
        a,
        "--no-download",
        "--stability",
        "microsoft",
        "--output-json",
        json.to_str().unwrap(),
        "--junit",
        junit.to_str().unwrap(),
    ]);
    let t = text(&o);
    assert_eq!(code(&o), 1, "{t}");
    assert!(t.contains("Codeunit50800.SecondNeedsFirst  (base: pass)"), "{t}");
    assert!(t.contains("isolation.alone=true"), "{t}");
    assert!(t.contains("order.methods=reverse"), "{t}");
    assert!(
        t.contains("pass -> fail under") && t.contains("workDate.offset=+1Y"),
        "{t}"
    );
    assert!(t.contains("4 tests, 2 stable, 2 flipped"), "{t}");
    let doc: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&json).unwrap()).unwrap();
    assert_eq!(doc["stability"]["summary"]["variants"], 5, "{doc}");
    assert_eq!(doc["stability"]["summary"]["flipped"], 2, "{doc}");
    assert_eq!(doc["stability"]["flips"].as_array().unwrap().len(), 3, "{doc}");
    let x = std::fs::read_to_string(&junit).unwrap();
    assert_eq!(x.matches("<testsuite ").count(), 5, "{x}");
    assert!(x.contains("outcome changed from pass under"), "{x}");
    // A report-only run.
    let o = alrunner(&[
        "test",
        a,
        "--no-download",
        "--stability",
        "microsoft",
        "--stability-allow-flips",
    ]);
    assert_eq!(code(&o), 0, "{}", text(&o));
    // One variant again, as a normal run, from its printed values.
    let o = alrunner(&[
        "test",
        a,
        "--no-download",
        "--stability-replay",
        "order.methods=reverse",
    ]);
    assert_eq!(code(&o), 1, "{}", text(&o));
    assert!(text(&o).contains("counter=0"), "{}", text(&o));
    // A config file: the run clock on a leap day.
    let cfg = d.path().join("leap.json");
    std::fs::write(&cfg, r#"{"format":1,"dimensions":{"clock.date":["2028-02-29"]}}"#).unwrap();
    let o = alrunner(&["test", a, "--no-download", "--stability", cfg.to_str().unwrap()]);
    let t = text(&o);
    assert_eq!(code(&o), 1, "{t}");
    assert!(t.contains("Codeunit50800.NotOnLeapDay  (base: pass)"), "{t}");
    assert!(t.contains("4 tests, 3 stable, 1 flipped"), "{t}");
}

/// ADR-058 section 3: a config that names a "never" or a "pin first" dimension is refused with exit 2 before anything
/// compiles; the preset prints.
#[test]
fn stability_configs_with_forbidden_dimensions_are_refused() {
    let d = tempfile::tempdir().unwrap();
    let never = d.path().join("never.json");
    std::fs::write(
        &never,
        r#"{"format":1,"dimensions":{"events.subscriberOrder":["reverse"]}}"#,
    )
    .unwrap();
    let o = alrunner(&["test", "nothing-here", "--stability", never.to_str().unwrap()]);
    assert_eq!(code(&o), 2, "{}", text(&o));
    assert!(text(&o).contains("may never be changed"), "{}", text(&o));
    let pin = d.path().join("pin.json");
    std::fs::write(
        &pin,
        r#"{"format":1,"dimensions":{"session.timeZone":["Europe/Berlin"]}}"#,
    )
    .unwrap();
    let o = alrunner(&["test", "nothing-here", "--stability", pin.to_str().unwrap()]);
    assert_eq!(code(&o), 2, "{}", text(&o));
    assert!(text(&o).contains("needs a corpus pin"), "{}", text(&o));
    let o = alrunner(&["test", "--stability-print-preset", "microsoft"]);
    assert_eq!(code(&o), 0, "{}", text(&o));
    assert!(text(&o).contains("\"random.librarySeed\""), "{}", text(&o));
}

/// ADR-058 phase 2: host-value dimensions (identity, company, clock) flip the tests that read them, a value with spaces
/// replays from its printed form, and `order.codeunits` without `--isolation disabled` says it cannot change anything.
#[test]
fn stability_mode_moves_host_values() {
    let d = tempfile::tempdir().unwrap();
    let dir = d.path().join("stab2");
    write(
        &dir,
        "app.json",
        &app_json("11111111-0000-0000-0000-000000005801", "Stability2", &[]),
    );
    write(
        &dir,
        "src/Tests.al",
        "codeunit 50810 \"Stability Host Tests\"\n{\n    Subtype = Test;\n\n    [Test]\n    procedure UserIsTestUser()\n    begin\n        if UserId() <> 'TESTUSER' then\n            Error('user %1', UserId());\n    end;\n\n    [Test]\n    procedure CompanyIsCronus()\n    begin\n        if CompanyName() <> 'CRONUS International Ltd.' then\n            Error('company %1', CompanyName());\n    end;\n\n    [Test]\n    procedure BeforeEvening()\n    begin\n        if Time() > 180000T then\n            Error('evening');\n    end;\n}\n",
    );
    let a = dir.to_str().unwrap();
    let cfg = d.path().join("host.json");
    std::fs::write(
        &cfg,
        r#"{"format":1,"dimensions":{"identity.userId":["OTHER"],"company.name":["Other Company"],"clock.timeOfDay":["23:00:00.000"],"order.codeunits":["reverse"]}}"#,
    )
    .unwrap();
    let o = alrunner(&["test", a, "--no-download", "--stability", cfg.to_str().unwrap()]);
    let t = text(&o);
    assert_eq!(code(&o), 1, "{t}");
    assert!(t.contains("3 tests, 0 stable, 3 flipped"), "{t}");
    assert!(t.contains("company.name=Other Company"), "{t}");
    assert!(
        t.contains("order.codeunits cannot change an outcome in this run"),
        "{t}"
    );
    let o = alrunner(&[
        "test",
        a,
        "--no-download",
        "--stability-replay",
        "company.name=Other Company",
    ]);
    assert_eq!(code(&o), 1, "{}", text(&o));
    assert!(text(&o).contains("company Other Company"), "{}", text(&o));
}
