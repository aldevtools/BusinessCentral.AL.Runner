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

/// ADR-066 amendment 4, decision 5: a test codeunit that relies on SingleInstance state an earlier one left is listed as
/// order dependent by `--stability`, whether it keeps its parallel run (default) or runs again serially
/// (`--strict-order`).
#[test]
fn stability_lists_order_dependent_codeunits_with_and_without_strict_order() {
    let d = tempfile::tempdir().unwrap();
    let src = r#"
codeunit 50870 "Order State"
{
    SingleInstance = true;
    procedure SetFlag(V: Text) begin Flag := V; end;
    procedure GetFlag(): Text begin exit(Flag); end;
    var Flag: Text;
}
codeunit 50871 "Order Setter"
{
    Subtype = Test;
    [Test] procedure SetsTheFlag() var S: Codeunit "Order State"; begin S.SetFlag('left'); end;
}
codeunit 50872 "Order Lenient Reader"
{
    Subtype = Test;
    [Test] procedure ReadsAnyFlag() var S: Codeunit "Order State"; begin if StrLen(S.GetFlag()) > 10 then Error('long'); end;
}
"#;
    let app = one_file_app(
        d.path(),
        "order",
        "11111111-0000-0000-0000-000000006924",
        "Order",
        "src/Tests.al",
        src,
    );
    let a = app.to_str().unwrap();
    for strict in [false, true] {
        let json = d.path().join(format!("r{strict}.json"));
        let mut args = vec![
            "test",
            a,
            "--no-download",
            "--stability",
            "microsoft",
            "--stability-allow-flips",
            "--output-json",
            json.to_str().unwrap(),
        ];
        if strict {
            args.push("--strict-order");
        }
        let o = alrunner(&args);
        assert_eq!(code(&o), 0, "{}", text(&o));
        let doc: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&json).unwrap()).unwrap();
        let listed: Vec<&str> = doc["stability"]["orderDependent"]
            .as_array()
            .unwrap_or_else(|| panic!("{doc}"))
            .iter()
            .map(|t| t["test"].as_str().unwrap())
            .collect();
        assert_eq!(listed, vec!["Codeunit50872.ReadsAnyFlag"], "strict {strict}: {doc}");
    }
}

/// ADR-066 amendment 6 (al-runner issue 6924): with `--test-suites` a codeunit is order dependent only on what an
/// earlier codeunit of its own suite left. A listed id that is no test codeunit is a warning; a bad line stops the run.
#[test]
fn stability_lists_order_dependence_within_a_suite_only() {
    let d = tempfile::tempdir().unwrap();
    let src = r#"
codeunit 50875 "Suite State"
{
    SingleInstance = true;
    procedure SetFlag(V: Text) begin Flag := V; end;
    procedure GetFlag(): Text begin exit(Flag); end;
    var Flag: Text;
}
codeunit 50876 "Suite Setter"
{
    Subtype = Test;
    [Test] procedure SetsTheFlag() var S: Codeunit "Suite State"; begin S.SetFlag('left'); end;
}
codeunit 50877 "Suite Lenient Reader"
{
    Subtype = Test;
    [Test] procedure ReadsAnyFlag() var S: Codeunit "Suite State"; begin if StrLen(S.GetFlag()) > 10 then Error('long'); end;
}
"#;
    let app = one_file_app(
        d.path(),
        "suites",
        "11111111-0000-0000-0000-000000006925",
        "Suites",
        "src/Tests.al",
        src,
    );
    let a = app.to_str().unwrap();
    for (name, file, expected) in [
        ("split", "BASE01: 50876\nDATACHECK: 50877 99\n", vec![]),
        (
            "together",
            "# one suite\nsuite  1: 50876 50877\n",
            vec!["Codeunit50877.ReadsAnyFlag"],
        ),
    ] {
        let suites = d.path().join(format!("{name}.txt"));
        std::fs::write(&suites, file).unwrap();
        let json = d.path().join(format!("{name}.json"));
        let o = alrunner(&[
            "test",
            a,
            "--no-download",
            "--stability",
            "microsoft",
            "--stability-allow-flips",
            "--test-suites",
            suites.to_str().unwrap(),
            "--output-json",
            json.to_str().unwrap(),
        ]);
        assert_eq!(code(&o), 0, "{}", text(&o));
        assert_eq!(
            text(&o).contains("suite `DATACHECK` (line 2) lists 99 that is no test codeunit"),
            name == "split",
            "{}",
            text(&o)
        );
        let doc: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&json).unwrap()).unwrap();
        let listed: Vec<&str> = doc["stability"]["orderDependent"]
            .as_array()
            .unwrap_or_else(|| panic!("{doc}"))
            .iter()
            .map(|t| t["test"].as_str().unwrap())
            .collect();
        assert_eq!(listed, expected, "{name}: {doc}");
    }
    let bad = d.path().join("bad.txt");
    std::fs::write(&bad, "A: 50876 x\n").unwrap();
    let o = alrunner(&["test", a, "--no-download", "--test-suites", bad.to_str().unwrap()]);
    assert_eq!(code(&o), 2, "{}", text(&o));
    assert!(text(&o).contains("line 1: `x` is not a codeunit id"), "{}", text(&o));
}

/// ADR-066 amendment 7, option 2, opt-in since the owner's change of 2026-11-08: with `--strict-order` the chain of 51
/// codeunits is replayed in full and the checker passes. With `--max-replay 50` too the dependent codeunit keeps its
/// parallel run; the run output and the JUnit file say why, and `--stability` (BC's order) lists it. ADR-066 amendment
/// 12: without `--strict-order` nothing is replayed (the checker fails with an order hint) and `--max-replay` has no
/// effect.
#[test]
fn a_rerun_over_max_replay_is_reported_in_the_output_junit_and_stability() {
    let d = tempfile::tempdir().unwrap();
    let mut src = String::from(
        "codeunit 50880 \"Chain Counter\" { SingleInstance = true; procedure Bump() begin Count += 1; end; procedure Get(): Integer begin exit(Count); end; var Count: Integer; }\n",
    );
    // Each raises the counter by a number of its own, so each depends on the one before it.
    for k in 0..50 {
        src.push_str(&format!(
            "codeunit {} \"Chain Step {k}\" {{ Subtype = Test; [Test] procedure Step{k}() var C: Codeunit \"Chain Counter\"; I: Integer; begin for I := 0 to {k} do C.Bump(); end; }}\n",
            50900 + k
        ));
    }
    src.push_str(
        "codeunit 50999 \"Chain Checker\" { Subtype = Test; [Test] procedure NeedsTheChain() var C: Codeunit \"Chain Counter\"; begin if C.Get() = 0 then Error('alone'); end; }\n",
    );
    let app = one_file_app(
        d.path(),
        "chain",
        "11111111-0000-0000-0000-000000006926",
        "Chain",
        "src/Tests.al",
        &src,
    );
    let a = app.to_str().unwrap();
    let text_of = "re-run skipped (chain of 51 codeunits over the limit of 50)";
    // Default (amendment 12): fresh state, nothing replayed, the checker fails with a hint; `--max-replay` changes nothing.
    for extra in [&[][..], &["--max-replay", "50"][..]] {
        let o = alrunner(&[&["test", a, "--no-download"][..], extra].concat());
        assert_eq!(code(&o), 1, "{}", text(&o));
        assert!(
            text(&o).contains("order hint: ") && !text(&o).contains("re-run skipped"),
            "{}",
            text(&o)
        );
    }
    // `--strict-order`: no cap, the whole chain is replayed.
    let o = alrunner(&["test", a, "--no-download", "--strict-order"]);
    assert_eq!(code(&o), 0, "{}", text(&o));
    assert!(!text(&o).contains("re-run skipped"), "{}", text(&o));
    let o = alrunner(&["test", a, "--no-download", "--max-replay", "x"]);
    assert_eq!(code(&o), 2, "{}", text(&o));
    assert!(
        text(&o).contains("--max-replay needs a number, got `x`"),
        "{}",
        text(&o)
    );
    let junit = d.path().join("r.xml");
    let o = alrunner(&[
        "test",
        a,
        "--no-download",
        "--strict-order",
        "--max-replay",
        "50",
        "--junit",
        junit.to_str().unwrap(),
    ]);
    assert_eq!(code(&o), 1, "{}", text(&o));
    assert!(
        text(&o).contains(&format!("test codeunit 50999 Chain Checker: {text_of}")),
        "{}",
        text(&o)
    );
    let x = std::fs::read_to_string(&junit).unwrap();
    let case = x
        .lines()
        .find(|l| l.contains("name=\"NeedsTheChain\""))
        .unwrap_or_else(|| panic!("{x}"));
    assert!(
        case.contains(&format!(
            "<property name=\"singleInstanceRerun\" value=\"{text_of}\" />"
        )),
        "{case}"
    );
    assert!(case.contains("single-instance-rerun-skipped"), "{case}");
    assert!(
        !x.lines()
            .any(|l| l.contains("name=\"Step3\"") && l.contains("singleInstanceRerun")),
        "{x}"
    );
    // `--stability` lists the codeunit whose re-run was skipped.
    let json = d.path().join("s.json");
    let o = alrunner(&[
        "test",
        a,
        "--no-download",
        "--max-replay",
        "50",
        "--stability",
        "microsoft",
        "--stability-allow-flips",
        "--output-json",
        json.to_str().unwrap(),
    ]);
    assert!(text(&o).contains("re-run skipped"), "{}", text(&o));
    let doc: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&json).unwrap()).unwrap();
    let listed: Vec<&str> = doc["stability"]["rerunSkipped"]
        .as_array()
        .unwrap_or_else(|| panic!("{doc}"))
        .iter()
        .map(|t| t["test"].as_str().unwrap())
        .collect();
    assert!(listed.contains(&"Codeunit50999.NeedsTheChain"), "{doc}");
}

const SI_PAIR: &str = r#"
codeunit 50890 "Si Order State"
{
    SingleInstance = true;
    procedure SetFlag(V: Text) begin Flag := V; end;
    procedure GetFlag(): Text begin exit(Flag); end;
    var Flag: Text;
}
codeunit 50891 "Si Order Setter"
{
    Subtype = Test;
    [Test] procedure SiSetsTheFlag() var S: Codeunit "Si Order State"; begin S.SetFlag('left'); end;
}
codeunit 50892 "Si Order Reader"
{
    Subtype = Test;
    [Test] procedure SiNeedsTheFlag() var S: Codeunit "Si Order State"; begin if S.GetFlag() <> 'left' then Error('no flag [%1]', S.GetFlag()); end;
}
codeunit 50893 "Si Order Lenient"
{
    Subtype = Test;
    [Test] procedure SiReadsAnyFlag() var S: Codeunit "Si Order State"; begin if StrLen(S.GetFlag()) > 10 then Error('long'); end;
}
"#;

/// ADR-058 amendment 1: a test that passes only after an earlier codeunit left SingleInstance state flips under
/// `isolation.singleInstance = fresh`, with a reason that names the global and its last writer; a codeunit that reads
/// carried state with an unchanged outcome is listed in `readsCarried` (checked) and gets the JUnit property.
#[test]
fn stability_reports_order_dependence_through_single_instance_state_with_a_reason() {
    let d = tempfile::tempdir().unwrap();
    let app = one_file_app(
        d.path(),
        "siorder",
        "11111111-0000-0000-0000-000000006930",
        "SiOrder",
        "src/Tests.al",
        SI_PAIR,
    );
    let cfg = d.path().join("si.json");
    std::fs::write(
        &cfg,
        r#"{"format":1,"dimensions":{"isolation.singleInstance":["carried","fresh"]}}"#,
    )
    .unwrap();
    let (json, junit) = (d.path().join("out.json"), d.path().join("out.xml"));
    let o = alrunner(&[
        "test",
        app.to_str().unwrap(),
        "--no-download",
        "--stability",
        cfg.to_str().unwrap(),
        "--output-json",
        json.to_str().unwrap(),
        "--junit",
        junit.to_str().unwrap(),
    ]);
    let out = text(&o);
    assert_eq!(code(&o), 1, "a flip is exit 1: {out}");
    assert!(out.contains("Order dependence (SingleInstance state):"), "{out}");
    assert!(out.contains("Breaks with fresh SingleInstance state:"), "{out}");
    assert!(
        out.contains("SiNeedsTheFlag: passes only after earlier codeunits"),
        "{out}"
    );
    assert!(
        out.contains("Reads carried state, outcome unchanged (checked):"),
        "{out}"
    );
    let doc: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&json).unwrap()).unwrap();
    let st = &doc["stability"];
    let flip = st["flips"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["test"] == "Codeunit50892.SiNeedsTheFlag")
        .unwrap_or_else(|| panic!("{st}"));
    assert_eq!(flip["reason"]["kind"], "passesOnlyAfterEarlierCodeunits", "{flip}");
    let read = &flip["reason"]["reads"][0];
    assert_eq!(read["global"], "Flag", "{flip}");
    assert_eq!(read["singleInstance"]["name"], "Si Order State");
    assert_eq!(read["singleInstance"]["dependencyApp"], false);
    assert_eq!(read["lastWriter"]["id"], 50891);
    let carried = st["readsCarried"].as_array().unwrap();
    assert!(
        carried
            .iter()
            .any(|c| c["codeunit"]["id"] == 50893 && c["checked"] == true && c["outcome"] == "pass"),
        "{st}"
    );
    assert!(
        !carried.iter().any(|c| c["codeunit"]["id"] == 50892),
        "a flip is not listed again: {st}"
    );
    let x = std::fs::read_to_string(&junit).unwrap();
    assert!(x.contains("singleInstanceCarried"), "{x}");
    assert!(x.contains("reads Flag of Si Order State"), "{x}");
}

/// ADR-058 amendment 1, decision 7 (owner, 2026-11-09): a bare `--stability` runs the preset `all`.
#[test]
fn a_bare_stability_runs_the_preset_all() {
    let o = alrunner(&["test", "--stability-print-preset", "all"]);
    assert_eq!(code(&o), 0, "{}", text(&o));
    assert!(text(&o).contains("\"isolation.singleInstance\""), "{}", text(&o));
    let d = tempfile::tempdir().unwrap();
    let app = one_file_app(
        d.path(),
        "siall",
        "11111111-0000-0000-0000-000000006931",
        "SiAll",
        "src/Tests.al",
        SI_PAIR,
    );
    let json = d.path().join("all.json");
    let o = alrunner(&[
        "test",
        "--no-download",
        "--stability",
        app.to_str().unwrap(),
        "--stability-allow-flips",
        "--output-json",
        json.to_str().unwrap(),
    ]);
    assert_eq!(code(&o), 0, "{}", text(&o));
    let doc: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&json).unwrap()).unwrap();
    let variants = doc["stability"]["variants"].as_array().unwrap();
    assert_eq!(variants.len(), 18, "base and one per safe dimension: {variants:?}");
    assert!(
        variants
            .iter()
            .any(|v| v["values"]["isolation.singleInstance"] == "fresh")
    );
}
