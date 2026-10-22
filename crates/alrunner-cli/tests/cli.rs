//! `al-runner test` end to end on small fixture apps. Nothing here is Microsoft's code.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn alrunner(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_al-runner"))
        // Few threads: the machine is shared, and the fixtures are tiny.
        .env("AL_CHECK_THREADS", "2")
        .env("AL_LOWER_THREADS", "2")
        .env("AL_TEST_THREADS", "2")
        // The note about another al-runner on PATH (the machine may have AL Runner v2) stays out of these outputs.
        .env("ALRUNNER_NO_PATH_NOTE", "1")
        .args(args)
        .output()
        .expect("run al-runner")
}

fn text(o: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

fn code(o: &Output) -> i32 {
    o.status.code().expect("exit code")
}

fn write(dir: &Path, rel: &str, content: &str) {
    let p = dir.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, content).unwrap();
}

fn app_json(id: &str, name: &str, deps: &[(&str, &str)]) -> String {
    let deps: Vec<String> = deps
        .iter()
        .map(|(id, name)| format!(r#"{{"id":"{id}","name":"{name}","publisher":"Test","version":"1.0.0.0"}}"#))
        .collect();
    format!(
        r#"{{"id":"{id}","name":"{name}","publisher":"Test","version":"1.0.0.0","dependencies":[{}]}}"#,
        deps.join(",")
    )
}

const MIXED: &str = r#"
codeunit 50100 "Mixed Tests"
{
    Subtype = Test;

    [Test]
    procedure Passes()
    begin
    end;

    [Test]
    procedure Fails()
    begin
        Error('boom %1', 5);
    end;

    [Test]
    procedure Refused()
    begin
        Codeunit.Run(Codeunit::"Not There");
    end;
}

codeunit 50101 "Other Tests"
{
    Subtype = Test;

    [Test]
    procedure AlsoPasses()
    begin
    end;
}
"#;

fn mixed_app(root: &Path) -> PathBuf {
    let dir = root.join("mixed");
    write(
        &dir,
        "app.json",
        &app_json("11111111-0000-0000-0000-000000000001", "Mixed", &[]),
    );
    write(&dir, "src/Tests.al", MIXED);
    dir
}

/// A stored (uncompressed) ZIP with CRC-32s, enough for `al-deps`.
fn zip(entries: &[(&str, &str)]) -> Vec<u8> {
    fn crc32(data: &[u8]) -> u32 {
        let mut c = !0u32;
        for &b in data {
            c ^= u32::from(b);
            for _ in 0..8 {
                c = if c & 1 != 0 { (c >> 1) ^ 0xEDB8_8320 } else { c >> 1 };
            }
        }
        !c
    }
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, data) in entries {
        let data = data.as_bytes();
        let crc = crc32(data);
        let offset = out.len() as u32;
        out.extend(0x0403_4b50u32.to_le_bytes());
        out.extend([20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        out.extend(crc.to_le_bytes());
        out.extend((data.len() as u32).to_le_bytes());
        out.extend((data.len() as u32).to_le_bytes());
        out.extend((name.len() as u16).to_le_bytes());
        out.extend([0, 0]);
        out.extend(name.as_bytes());
        out.extend(data);
        central.extend(0x0201_4b50u32.to_le_bytes());
        central.extend([20, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        central.extend(crc.to_le_bytes());
        central.extend((data.len() as u32).to_le_bytes());
        central.extend((data.len() as u32).to_le_bytes());
        central.extend((name.len() as u16).to_le_bytes());
        central.extend([0; 12]);
        central.extend(offset.to_le_bytes());
        central.extend(name.as_bytes());
    }
    let start = out.len() as u32;
    let size = central.len() as u32;
    out.extend(central);
    out.extend(0x0605_4b50u32.to_le_bytes());
    out.extend([0, 0, 0, 0]);
    out.extend((entries.len() as u16).to_le_bytes());
    out.extend((entries.len() as u16).to_le_bytes());
    out.extend(size.to_le_bytes());
    out.extend(start.to_le_bytes());
    out.extend([0, 0]);
    out
}

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
    // The summary says failures are not to be trusted without test data.
    assert!(t.contains("no test data"), "{t}");
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

/// A test app in a source archive, and a library it depends on in another.
fn archives(root: &Path) -> (PathBuf, PathBuf) {
    let lib_id = "22222222-0000-0000-0000-000000000001";
    let lib = root.join("Lib.Source.zip");
    std::fs::write(
        &lib,
        zip(&[
            ("app.json", &app_json(lib_id, "Lib", &[])),
            (
                "src/Helper.Codeunit.al",
                "codeunit 50300 \"Lib Helper\" { procedure Answer(): Integer begin exit(42); end; }",
            ),
        ]),
    )
    .unwrap();
    let app = root.join("Tests-Lib.Source.zip");
    std::fs::write(
        &app,
        zip(&[
            ("app.json", &app_json("22222222-0000-0000-0000-000000000002", "Tests-Lib", &[(lib_id, "Lib")])),
            (
                "T.Codeunit.al",
                "codeunit 50301 \"Lib Tests\" { Subtype = Test; [Test] procedure UsesTheLibrary() var H: Codeunit \"Lib Helper\"; begin if H.Answer() <> 42 then Error('wrong'); end; }",
            ),
        ]),
    )
    .unwrap();
    (app, lib)
}

#[test]
fn a_source_archive_runs_with_its_dependency_given_by_dep() {
    let d = tempfile::tempdir().unwrap();
    let (app, lib) = archives(d.path());
    let o = alrunner(&[
        "test",
        app.to_str().unwrap(),
        "--dep",
        lib.to_str().unwrap(),
        "--no-download",
        "--verbose",
    ]);
    let t = text(&o);
    assert_eq!(code(&o), 0, "{t}");
    assert!(t.contains("Tests: 1 total, 1 passed"), "{t}");
    assert!(t.contains("dependencies: 1 apps"), "{t}");
    // The same, with the library found in a folder of packages.
    let o = alrunner(&[
        "test",
        app.to_str().unwrap(),
        "--package-cache",
        d.path().to_str().unwrap(),
        "--no-download",
    ]);
    assert_eq!(code(&o), 0, "{}", text(&o));
}

#[test]
fn a_dependency_app_reads_its_own_packaged_resources() {
    // session/W5NavAppResources.al:GetResourceAsText_NameMatching: the name is matched exactly, relative to the
    // resource folder. The library's own code asks; the folder comes from the `resourceFolders` of its app.json.
    let d = tempfile::tempdir().unwrap();
    let lib_id = "22222222-0000-0000-0000-000000000011";
    let lib = d.path().join("Res.Source.zip");
    std::fs::write(
        &lib,
        zip(&[
            (
                "app.json",
                &format!(
                    r#"{{"id":"{lib_id}","name":"Res","publisher":"Test","version":"1.0.0.0","dependencies":[],"resourceFolders":[".resources"]}}"#
                ),
            ),
            (
                "src/Helper.Codeunit.al",
                "codeunit 50310 \"Res Helper\" { procedure Greeting(): Text begin exit(NavApp.GetResourceAsText('sub/hello.txt')); end; }",
            ),
            (".resources/sub/hello.txt", "Hello from a dependency"),
        ]),
    )
    .unwrap();
    let app = d.path().join("Tests-Res.Source.zip");
    std::fs::write(
        &app,
        zip(&[
            ("app.json", &app_json("22222222-0000-0000-0000-000000000012", "Tests-Res", &[(lib_id, "Res")])),
            (
                "T.Codeunit.al",
                "codeunit 50311 \"Res Tests\" { Subtype = Test; [Test] procedure ReadsIt() var H: Codeunit \"Res Helper\"; begin if H.Greeting() <> 'Hello from a dependency' then Error('got %1', H.Greeting()); end; }",
            ),
        ]),
    )
    .unwrap();
    let o = alrunner(&[
        "test",
        app.to_str().unwrap(),
        "--dep",
        lib.to_str().unwrap(),
        "--no-download",
    ]);
    let t = text(&o);
    assert_eq!(code(&o), 0, "{t}");
    assert!(t.contains("Tests: 1 total, 1 passed"), "{t}");
}

#[test]
fn a_missing_dependency_refuses_the_tests_that_need_it_instead_of_failing_the_run() {
    let d = tempfile::tempdir().unwrap();
    let (app, _lib) = archives(d.path());
    let o = alrunner(&["test", app.to_str().unwrap(), "--no-download"]);
    let t = text(&o);
    assert_eq!(code(&o), 7, "{t}");
    assert!(t.contains("1 not run"), "{t}");
    assert!(t.contains("is outside the workspace"), "{t}");
}

#[test]
fn an_app_folder_may_be_given_as_a_dependency_and_a_build_fills_microsoft_variables() {
    let d = tempfile::tempdir().unwrap();
    // The library is a folder; its version is one of Microsoft's build variables.
    let lib = d.path().join("lib");
    write(
        &lib,
        "app.json",
        r#"{"id":"22222222-0000-0000-0000-000000000009","name":"Lib","publisher":"Test","version":"$(app_currentVersion)","dependencies":[]}"#,
    );
    write(
        &lib,
        "H.al",
        "codeunit 50400 \"Lib Helper\" { procedure Answer(): Integer begin exit(7); end; }",
    );
    let app = d.path().join("app");
    write(
        &app,
        "app.json",
        &app_json(
            "22222222-0000-0000-0000-00000000000a",
            "App",
            &[("22222222-0000-0000-0000-000000000009", "Lib")],
        ),
    );
    write(
        &app,
        "T.al",
        "codeunit 50401 \"T\" { Subtype = Test; [Test] procedure Uses() var H: Codeunit \"Lib Helper\"; begin if H.Answer() <> 7 then Error('wrong'); end; }",
    );
    // Without a build the variable cannot be filled: said, not guessed.
    let o = alrunner(&[
        "test",
        app.to_str().unwrap(),
        "--dep",
        lib.to_str().unwrap(),
        "--no-download",
    ]);
    assert!(text(&o).contains("$(app_"), "{}", text(&o));
    // An exact build fills it.
    let o = alrunner(&[
        "test",
        app.to_str().unwrap(),
        "--dep",
        lib.to_str().unwrap(),
        "--no-download",
        "--bc-version",
        "27.5.46862.48827",
    ]);
    assert_eq!(code(&o), 0, "{}", text(&o));
    assert!(text(&o).contains("Tests: 1 total, 1 passed"), "{}", text(&o));
}

const TP_TESTS: &str = r#"
table 50100 "exttest" { fields { field(1; id; Integer) { } field(2; own; Text[50]) { } } keys { key(PK; id) { } } }
table 50101 "stray" { fields { field(1; id; Integer) { } } keys { key(PK; id) { } } }
codeunit 50100 "Data Tests" { Subtype = Test;
  [Test] procedure RestoredRows() var E: Record "exttest"; begin
    if E.Count() <> 3 then Error('count %1', E.Count());
    E.Get(2);
    if E.own <> 'base-two' then Error(E.own);
    if CompanyName() <> 'TP' then Error(CompanyName());
  end;
  [Test] procedure AnEmptyTable() var S: Record "stray"; begin if not S.IsEmpty() then Error('rows'); end; }
"#;

fn typeprobe() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../al-bcdb/tests/fixtures/typeprobe.bak")
}

#[test]
fn test_data_restores_a_company_from_a_backup_and_the_summary_says_what_was_loaded() {
    let d = tempfile::tempdir().unwrap();
    let dir = d.path().join("tp");
    write(
        &dir,
        "app.json",
        &app_json("11111111-0000-0000-0000-000000000002", "TP", &[]),
    );
    write(&dir, "src/Tests.al", TP_TESTS);
    let bak = typeprobe();
    let o = alrunner(&[
        "test",
        dir.to_str().unwrap(),
        "--no-download",
        "--verbose",
        "--test-data",
        bak.to_str().unwrap(),
        "--test-data-company",
        "TP",
    ]);
    let t = text(&o);
    assert_eq!(code(&o), 0, "{t}");
    assert!(t.contains("Tests: 2 total, 2 passed, 0 failed, 0 not run"), "{t}");
    assert!(
        !t.contains("no test data"),
        "a run with test data carries no such warning: {t}"
    );
    assert!(
        t.contains("Test data: 3 rows loaded from 1 tables of company 'TP' in typeprobe.bak"),
        "{t}"
    );
    assert!(
        t.contains("test data: ") && t.contains("typeprobe.bak (company 'TP'"),
        "the header names the source: {t}"
    );
}

#[test]
fn a_company_the_backup_does_not_hold_is_a_start_up_error_that_lists_the_companies() {
    let d = tempfile::tempdir().unwrap();
    let app = mixed_app(d.path());
    let bak = typeprobe();
    let o = alrunner(&[
        "test",
        app.to_str().unwrap(),
        "--no-download",
        "--test-data",
        bak.to_str().unwrap(),
        "--test-data-company",
        "Nope",
    ]);
    assert_eq!(code(&o), 2);
    assert!(
        text(&o).contains("'ProbeCo'") && text(&o).contains("'TP'"),
        "{}",
        text(&o)
    );
}

#[test]
fn top_failures_ranks_the_failure_messages_with_numbers_folded() {
    let d = tempfile::tempdir().unwrap();
    let app = mixed_app(d.path());
    let o = alrunner(&["test", app.to_str().unwrap(), "--no-download", "--top-failures", "5"]);
    let t = text(&o);
    assert!(
        t.contains("Top failures, failed tests: 1 failed, 1 distinct reasons"),
        "{t}"
    );
    assert!(t.contains("boom N"), "{t}");
}

#[test]
fn explain_shows_where_a_failing_test_raised_its_error() {
    let d = tempfile::tempdir().unwrap();
    let app = mixed_app(d.path());
    let o = alrunner(&["test", app.to_str().unwrap(), "--no-download", "--explain", "Fails"]);
    let t = text(&o);
    assert!(t.contains("runtime: FAIL"), "{t}");
    assert!(t.contains("error: boom 5"), "{t}");
    assert!(t.contains("raised in (innermost first):"), "{t}");
    assert!(t.contains("Mixed Tests") && t.contains("Fails"), "{t}");
    assert!(t.contains("src/Tests.al:"), "the source position is named: {t}");
    assert_eq!(code(&o), 1, "{t}");
}

#[test]
fn explain_shows_where_a_refused_test_was_refused() {
    let d = tempfile::tempdir().unwrap();
    let app = mixed_app(d.path());
    let o = alrunner(&["test", app.to_str().unwrap(), "--no-download", "--explain", "Refused"]);
    let t = text(&o);
    assert!(t.contains("runtime: NOT RUN (refused)"), "{t}");
    assert!(t.contains("reason: "), "{t}");
    assert_eq!(code(&o), 7, "{t}");
}

#[test]
fn explain_needs_exactly_one_test() {
    let d = tempfile::tempdir().unwrap();
    let app = mixed_app(d.path());
    let o = alrunner(&["test", app.to_str().unwrap(), "--no-download", "--explain", "Pass"]);
    let t = text(&o);
    assert!(t.contains("matches 2 tests"), "{t}");
    assert_eq!(code(&o), 2, "{t}");
    let o = alrunner(&[
        "test",
        app.to_str().unwrap(),
        "--no-download",
        "--explain",
        "NoSuchTest",
    ]);
    assert_eq!(code(&o), 6, "{}", text(&o));
}

const DEEP: &str = r#"
codeunit 50200 "Deep Tests"
{
    Subtype = Test;

    local procedure Inner()
    begin
        Error('deep boom');
    end;

    local procedure Middle()
    begin
        Inner();
    end;

    [Test]
    procedure DeepFails()
    begin
        Middle();
    end;
}
"#;

fn deep_app(root: &Path) -> PathBuf {
    let dir = root.join("deep");
    write(
        &dir,
        "app.json",
        &app_json("11111111-0000-0000-0000-000000000002", "Deep", &[]),
    );
    write(&dir, "src/Deep.al", DEEP);
    dir
}

#[test]
fn a_failure_shows_the_al_call_stack_in_the_console_and_in_junit() {
    let d = tempfile::tempdir().unwrap();
    let app = deep_app(d.path());
    let junit = d.path().join("junit.xml");
    let o = alrunner(&[
        "test",
        app.to_str().unwrap(),
        "--no-download",
        "--junit",
        junit.to_str().unwrap(),
    ]);
    let t = text(&o);
    assert_eq!(code(&o), 1, "{t}");
    // Innermost frame first: Inner, Middle, then the test itself.
    let frames: Vec<&str> = t
        .lines()
        .filter(|l| l.trim_start().starts_with("\"Deep Tests\"(CodeUnit 50200)"))
        .collect();
    assert_eq!(frames.len(), 3, "{t}");
    assert!(frames[0].contains(".Inner line 2"), "{t}");
    assert!(frames[1].contains(".Middle line 2"), "{t}");
    assert!(frames[2].contains(".DeepFails line 2"), "{t}");
    assert!(
        t.lines().any(|l| l.starts_with("FAIL") && l.contains("deep boom")),
        "{t}"
    );
    let x = std::fs::read_to_string(&junit).unwrap();
    assert!(x.contains("<failure message=\"deep boom\">"), "{x}");
    assert!(x.contains(".Inner line 2"), "{x}");
    assert!(x.contains("</failure>"), "{x}");
}

#[test]
fn quiet_leaves_the_call_stack_out_of_the_console() {
    let d = tempfile::tempdir().unwrap();
    let app = deep_app(d.path());
    let o = alrunner(&["test", app.to_str().unwrap(), "--no-download", "--quiet"]);
    assert!(!text(&o).contains("CodeUnit 50200"), "{}", text(&o));
}

#[test]
fn warm_builds_the_index_of_a_backup_so_that_the_first_run_does_not() {
    let backup = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../al-bcdb/tests/fixtures/typeprobe.bak");
    let cache = tempfile::tempdir().unwrap();
    let args = [
        "warm",
        backup.to_str().unwrap(),
        "--test-data-company",
        "TP",
        "--cache-dir",
        cache.path().to_str().unwrap(),
    ];
    let o = alrunner(&args);
    assert_eq!(code(&o), 0, "{}", text(&o));
    assert!(text(&o).contains("built and saved"), "{}", text(&o));
    let dir = cache.path().join("backup-index");
    let files: Vec<_> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().path()).collect();
    assert_eq!(files.len(), 1, "{files:?}");
    let o = alrunner(&args);
    assert_eq!(code(&o), 0, "{}", text(&o));
    assert!(text(&o).contains("kept map and catalog used"), "{}", text(&o));
}

#[test]
fn warm_without_a_backup_or_a_build_says_what_it_needs() {
    let o = alrunner(&["warm"]);
    assert_eq!(code(&o), 2);
    assert!(
        text(&o).contains("PATH.bak") && text(&o).contains("--bc-version"),
        "{}",
        text(&o)
    );
}

// ---- the interface the v2 tools use (docs/integrations/alchemist-lethal.md, step 1) ----

fn json_of(o: &Output) -> serde_json::Value {
    serde_json::from_slice(&o.stdout).unwrap_or_else(|e| panic!("stdout is not one JSON document: {e}\n{}", text(o)))
}

fn row<'a>(doc: &'a serde_json::Value, name: &str) -> &'a serde_json::Value {
    doc["tests"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == name)
        .unwrap_or_else(|| panic!("no row {name} in {doc}"))
}

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
    // disabled is not pinned: every test is reported as not run.
    let o = run(&["--isolation", "disabled"]);
    assert_eq!(code(&o), 7, "{}", text(&o));
    assert_eq!(json_of(&o)["notRun"], 2);
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
    }
    assert_eq!(row(&j, "Codeunit50410.UnderTheLimit")["status"], "pass");
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

// ---- coverage (docs/integrations/alchemist-lethal.md, step 2) ----

const COVERED: &str = r#"
codeunit 50500 "Covered Tests"
{
    Subtype = Test;

    [Test]
    procedure TakesTheThenBranch()
    var
        N: Integer;
    begin
        N := 1;
        if N = 1 then
            N := 2
        else
            N := 3;
    end;

    [Test]
    procedure TakesTheElseBranch()
    var
        N: Integer;
    begin
        N := 5;
        if N = 1 then
            N := 2
        else
            N := 3;
    end;
}
"#;

fn covered_app(root: &Path) -> PathBuf {
    let dir = root.join("covered");
    write(
        &dir,
        "app.json",
        &app_json("11111111-0000-0000-0000-000000000005", "Covered", &[]),
    );
    write(&dir, "src/Covered.al", COVERED);
    dir
}

#[test]
fn coverage_writes_cobertura_with_the_runs_of_each_line_and_json_with_each_tests_statements() {
    let d = tempfile::tempdir().unwrap();
    let app = covered_app(d.path());
    let xml = d.path().join("out/c.xml");
    let js = d.path().join("out/c.json");
    let o = alrunner(&[
        "test",
        app.to_str().unwrap(),
        "--no-download",
        "--no-test-data",
        "--coverage-out",
        xml.to_str().unwrap(),
        "--coverage-json",
        js.to_str().unwrap(),
    ]);
    assert_eq!(code(&o), 0, "{}", text(&o));
    assert!(text(&o).contains("coverage: "), "{}", text(&o));
    // Cobertura: a class for the file with a line per statement line. Test 1: `N := 1;` (11), the `if` (12), the then
    // branch (13) run, the else branch (15) does not. Test 2: `N := 5;` (23), the `if` (24), the else branch (27) run,
    // the then branch (25) does not.
    let x = std::fs::read_to_string(&xml).unwrap();
    assert!(x.contains("filename=\"") && x.contains("Covered.al\""), "{x}");
    assert!(x.contains("<line number=\"11\" hits=\"1\""), "{x}");
    assert!(x.contains("<line number=\"12\" hits=\"1\""), "{x}");
    let line_hits = |n: u32| -> u64 {
        let tag = format!("<line number=\"{n}\" hits=\"");
        let at = x.find(&tag).unwrap_or_else(|| panic!("no line {n} in {x}")) + tag.len();
        x[at..].split('"').next().unwrap().parse().unwrap()
    };
    assert_eq!(
        (line_hits(13), line_hits(15), line_hits(25), line_hits(27)),
        (1, 0, 0, 1)
    );
    // JSON: every statement, and per test the statements it ran.
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&js).unwrap()).unwrap();
    let file = &v["coverage"][0];
    assert!(file["file"].as_str().unwrap().ends_with("/src/Covered.al"), "{file}");
    let statements = file["statements"].as_array().unwrap();
    assert!(
        statements
            .iter()
            .any(|s| s["scope"] == "TakesTheThenBranch" && s["line"] == 13 && s["hits"] == 1)
    );
    let per_test = v["perTestCoverage"].as_array().unwrap();
    let then = per_test
        .iter()
        .find(|t| t["test"] == "Codeunit50500.TakesTheThenBranch")
        .unwrap();
    let ran: Vec<u64> = then["coverage"][0]["statements"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["line"].as_u64().unwrap())
        .collect();
    assert!(ran.contains(&13) && !ran.contains(&15), "{ran:?}");
}

#[test]
fn coverage_off_writes_nothing_and_a_selection_counts_only_the_selected_tests() {
    let d = tempfile::tempdir().unwrap();
    let app = covered_app(d.path());
    let xml = d.path().join("c.xml");
    let o = alrunner(&["test", app.to_str().unwrap(), "--no-download", "--no-test-data"]);
    assert_eq!(code(&o), 0, "{}", text(&o));
    assert!(!text(&o).contains("coverage:"), "{}", text(&o));
    let o = alrunner(&[
        "test",
        app.to_str().unwrap(),
        "--no-download",
        "--no-test-data",
        "--test",
        "Codeunit50500.TakesTheElseBranch",
        "--coverage-out",
        xml.to_str().unwrap(),
    ]);
    assert_eq!(code(&o), 0, "{}", text(&o));
    let x = std::fs::read_to_string(&xml).unwrap();
    // Only the second test counts: its else branch ran, the first test's then branch did not.
    assert!(x.contains("<line number=\"27\" hits=\"1\""), "{x}");
    assert!(x.contains("<line number=\"13\" hits=\"0\""), "{x}");
}

#[test]
fn server_url_is_checked_printed_and_taken_from_the_variable() {
    let d = tempfile::tempdir().unwrap();
    let app = mixed_app(d.path());
    let a = app.to_str().unwrap();
    let o = alrunner(&["test", a, "--no-download", "--server-url", "bc.example"]);
    assert_eq!(code(&o), 2, "{}", text(&o));
    assert!(text(&o).contains("--server-url"), "{}", text(&o));
    let o = alrunner(&[
        "test",
        a,
        "--no-download",
        "--verbose",
        "--server-url",
        "https://bc.example:7048/BC1/",
    ]);
    assert!(
        text(&o).contains("server address: https://bc.example:7048/BC1/ (--server-url)"),
        "{}",
        text(&o)
    );
    let o = alrunner(&["test", a, "--no-download", "--verbose"]);
    assert!(
        text(&o).contains("server address: http://localhost:8080/BC (default)"),
        "{}",
        text(&o)
    );
    let o = std::process::Command::new(env!("CARGO_BIN_EXE_al-runner"))
        .env("ALRUNNER_NO_PATH_NOTE", "1")
        .env("ALRUNNER_SERVER_URL", "http://env.example/BC/")
        .args(["test", a, "--no-download", "--verbose"])
        .output()
        .unwrap();
    assert!(
        text(&o).contains("server address: http://env.example/BC/ (ALRUNNER_SERVER_URL)"),
        "{}",
        text(&o)
    );
}

#[test]
fn color_always_paints_the_labels_and_never_or_no_color_keeps_them_plain() {
    let d = tempfile::tempdir().unwrap();
    let app = mixed_app(d.path());
    let a = app.to_str().unwrap();
    let t = text(&alrunner(&[
        "test",
        a,
        "--no-download",
        "--verbose",
        "--color",
        "always",
    ]));
    assert!(t.contains("\x1b[32mPASS"), "{t}");
    assert!(t.contains("\x1b[31mFAIL"), "{t}");
    assert!(t.contains("\x1b[33mNOT RUN"), "{t}");
    assert!(t.contains("\x1b[1m"), "headings are bold: {t}");
    let t = text(&alrunner(&["test", a, "--no-download", "--color", "never"]));
    assert!(!t.contains('\x1b'), "{t}");
    let o = std::process::Command::new(env!("CARGO_BIN_EXE_al-runner"))
        .env("ALRUNNER_NO_PATH_NOTE", "1")
        .env("NO_COLOR", "1")
        .args(["test", a, "--no-download"])
        .output()
        .unwrap();
    assert!(!text(&o).contains('\x1b'));
    // A JUnit file is plain even with --color always.
    let xml = d.path().join("r.xml");
    alrunner(&[
        "test",
        a,
        "--no-download",
        "--color",
        "always",
        "--junit",
        xml.to_str().unwrap(),
    ]);
    assert!(!std::fs::read_to_string(&xml).unwrap().contains('\x1b'));
}

// The note about another `al-runner` on PATH (ADR-016, amendment of 2026-10-21). AL Runner v2's dotnet
// tool installs a command of the same name; whichever comes first on PATH runs.

/// A fake other `al-runner`: a shell script that runs `body`.
#[cfg(unix)]
fn fake_al_runner(dir: &Path, body: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    std::fs::create_dir_all(dir).unwrap();
    let p = dir.join("al-runner");
    std::fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    p
}

/// Runs this al-runner with a PATH, a home folder and a cache folder of the test's own, the note switched on.
#[cfg(unix)]
fn alrunner_env(path: &Path, home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_al-runner"))
        .env_remove("ALRUNNER_NO_PATH_NOTE")
        .env("PATH", path)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("ALRUNNER_CACHE", home.join("cache"))
        .args(args)
        .output()
        .expect("run al-runner")
}

#[cfg(unix)]
const NOTE: &str = "note: another al-runner is on PATH at ";

#[cfg(unix)]
#[test]
fn another_al_runner_on_path_gets_one_note_on_stderr_at_most_once_a_day() {
    let d = tempfile::tempdir().unwrap();
    let bin = d.path().join("v2bin");
    let other = fake_al_runner(&bin, "echo 'AL Runner 1.0.99'");
    let home = d.path().join("home");
    let cache_dir = d.path().join("c");
    let args = ["cache", "info", "--cache-dir", cache_dir.to_str().unwrap()];
    let first = alrunner_env(&bin, &home, &args);
    assert_eq!(code(&first), 0, "{}", text(&first));
    let err = String::from_utf8_lossy(&first.stderr);
    assert!(err.contains(&format!("{NOTE}{}", other.display())), "{err}");
    assert!(
        err.contains("ALRUNNER_NO_PATH_NOTE=1"),
        "the note says how to turn it off: {err}"
    );
    assert_eq!(err.matches(NOTE).count(), 1, "one line: {err}");
    assert!(
        !String::from_utf8_lossy(&first.stdout).contains("al-runner is on PATH"),
        "never on stdout"
    );
    let second = alrunner_env(&bin, &home, &args);
    assert!(!text(&second).contains(NOTE), "once a day: {}", text(&second));
    assert_eq!(first.stdout, second.stdout, "the note does not change stdout");
}

#[cfg(unix)]
#[test]
fn the_path_note_is_off_with_alrunner_no_path_note() {
    let d = tempfile::tempdir().unwrap();
    let bin = d.path().join("v2bin");
    fake_al_runner(&bin, "echo 'AL Runner 1.0.99'");
    let home = d.path().join("home");
    let o = Command::new(env!("CARGO_BIN_EXE_al-runner"))
        .env("ALRUNNER_NO_PATH_NOTE", "1")
        .env("PATH", &bin)
        .env("HOME", &home)
        .env("ALRUNNER_CACHE", home.join("cache"))
        .args(["cache", "info", "--cache-dir", d.path().join("c").to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(code(&o), 0, "{}", text(&o));
    assert!(!text(&o).contains(NOTE), "{}", text(&o));
}

#[cfg(unix)]
#[test]
fn the_dotnet_tools_folder_is_checked_even_when_it_is_not_on_path() {
    let d = tempfile::tempdir().unwrap();
    let home = d.path().join("home");
    let other = fake_al_runner(&home.join(".dotnet").join("tools"), "echo 'AL Runner 1.0.99'");
    let empty = d.path().join("empty");
    std::fs::create_dir_all(&empty).unwrap();
    let o = alrunner_env(
        &empty,
        &home,
        &["cache", "info", "--cache-dir", d.path().join("c").to_str().unwrap()],
    );
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(err.contains(&format!("{NOTE}{}", other.display())), "{err}");
    assert!(err.contains("AL Runner v2, a dotnet tool"), "{err}");
    assert!(err.contains("dotnet tool uninstall -g MSDyn365BC.AL.Runner"), "{err}");
}

#[cfg(unix)]
#[test]
fn this_binary_itself_and_a_shim_of_another_tool_are_not_another_al_runner() {
    let d = tempfile::tempdir().unwrap();
    let home = d.path().join("home");
    // A link to this very binary.
    let me = d.path().join("me");
    std::fs::create_dir_all(&me).unwrap();
    std::os::unix::fs::symlink(env!("CARGO_BIN_EXE_al-runner"), me.join("al-runner")).unwrap();
    // A version manager's shim: `al-runner` is a link to a program of another name.
    let shims = d.path().join("shims");
    std::fs::create_dir_all(&shims).unwrap();
    let mgr = fake_al_runner(&d.path().join("mgr"), "echo manager");
    let mise = mgr.with_file_name("mise");
    std::fs::rename(&mgr, &mise).unwrap();
    std::os::unix::fs::symlink(&mise, shims.join("al-runner")).unwrap();
    let path = std::env::join_paths([&me, &shims]).unwrap();
    let o = alrunner_env(
        Path::new(&path),
        &home,
        &["cache", "info", "--cache-dir", d.path().join("c").to_str().unwrap()],
    );
    assert_eq!(code(&o), 0, "{}", text(&o));
    assert!(!text(&o).contains(NOTE), "{}", text(&o));
}

#[cfg(unix)]
#[test]
fn version_lists_each_other_al_runner_with_the_first_line_of_its_version() {
    let d = tempfile::tempdir().unwrap();
    let bin = d.path().join("v2bin");
    let other = fake_al_runner(&bin, "echo 'AL Runner 1.0.99'; echo 'second line'");
    let home = d.path().join("home");
    let o = alrunner_env(&bin, &home, &["--version"]);
    assert_eq!(code(&o), 0, "{}", text(&o));
    let out = String::from_utf8_lossy(&o.stdout);
    let lines: Vec<&str> = out.lines().collect();
    assert!(
        lines[0].starts_with(&format!("al-runner {} (", env!("CARGO_PKG_VERSION"))),
        "{out}"
    );
    assert_eq!(lines.len(), 2, "{out}");
    assert!(
        lines[1].contains(&other.display().to_string()) && lines[1].contains("AL Runner 1.0.99"),
        "{out}"
    );
    assert!(!lines[1].contains("second line"), "{out}");
    assert!(
        !String::from_utf8_lossy(&o.stderr).contains(NOTE),
        "--version lists instead of the note"
    );
}

#[cfg(unix)]
#[test]
fn version_waits_at_most_2_seconds_for_another_al_runner() {
    let d = tempfile::tempdir().unwrap();
    let bin = d.path().join("v2bin");
    fake_al_runner(&bin, "exec /bin/sleep 20");
    let home = d.path().join("home");
    let start = std::time::Instant::now();
    let o = alrunner_env(&bin, &home, &["--version"]);
    assert!(
        start.elapsed() < std::time::Duration::from_secs(10),
        "{:?}",
        start.elapsed()
    );
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(out.contains("no answer within 2 s"), "{out}");
}

#[cfg(unix)]
#[test]
fn version_read_by_a_tool_that_takes_only_the_first_line_still_exits_0() {
    use std::io::BufRead;
    let d = tempfile::tempdir().unwrap();
    let bin = d.path().join("v2bin");
    // The other copy answers after the reader has gone (`al-runner --version | head -1`).
    fake_al_runner(&bin, "/bin/sleep 1; echo 'AL Runner 1.0.99'");
    let home = d.path().join("home");
    let mut child = Command::new(env!("CARGO_BIN_EXE_al-runner"))
        .env_remove("ALRUNNER_NO_PATH_NOTE")
        .env("PATH", &bin)
        .env("HOME", &home)
        .env("ALRUNNER_CACHE", home.join("cache"))
        .arg("--version")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut first = String::new();
    std::io::BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut first)
        .unwrap();
    assert!(first.starts_with("al-runner "), "{first}");
    let o = child.wait_with_output().unwrap();
    assert_eq!(code(&o), 0, "{}", String::from_utf8_lossy(&o.stderr));
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

/// al-runner issue 5573: an app that declares runtime 13.0 calls NavApp.GetResourceAsText, which the AL compiler
/// rejects below runtime 14.0 (AL0666). The run ends as the compiler's does: a compile error, exit 3.
#[test]
fn a_call_newer_than_the_apps_declared_runtime_is_a_compile_error() {
    let d = tempfile::tempdir().unwrap();
    let app = d.path().join("app");
    let manifest = |id: &str, name: &str, deps: &[(&str, &str)]| {
        app_json(id, name, deps).replacen('{', r#"{"runtime":"13.0","#, 1)
    };
    write(
        &app,
        "app.json",
        &manifest("6f1d2f8e-5b0a-4c1e-9a43-0d6b8a2c5e11", "Resource Repro App", &[]),
    );
    write(&app, "res/value.txt", "HELLO");
    write(
        &app,
        "src/Reader.Codeunit.al",
        "codeunit 50100 \"Resource Reader\"\n{\n    procedure Value(): Text\n    begin\n        exit(NavApp.GetResourceAsText('value.txt', TextEncoding::UTF8));\n    end;\n\n    procedure Plain(): Text\n    begin\n        exit(UpperCase('hello'));\n    end;\n}\n",
    );
    let test = d.path().join("test");
    write(
        &test,
        "app.json",
        &manifest(
            "6f1d2f8e-5b0a-4c1e-9a43-0d6b8a2c5e12",
            "Resource Repro Tests",
            &[("6f1d2f8e-5b0a-4c1e-9a43-0d6b8a2c5e11", "Resource Repro App")],
        ),
    );
    write(
        &test,
        "src/ReaderTest.Codeunit.al",
        "codeunit 50150 \"Resource Reader Test\"\n{\n    Subtype = Test;\n\n    [Test]\n    procedure ReadsResource()\n    var\n        Reader: Codeunit \"Resource Reader\";\n    begin\n        if Reader.Value() <> 'HELLO' then\n            Error('expected HELLO, got %1', Reader.Value());\n    end;\n\n    [Test]\n    procedure ReadsPlain()\n    var\n        Reader: Codeunit \"Resource Reader\";\n    begin\n        if Reader.Plain() <> 'HELLO' then\n            Error('expected HELLO');\n    end;\n}\n",
    );
    let o = alrunner(&["test", app.to_str().unwrap(), test.to_str().unwrap(), "--no-download"]);
    // As the AL compiler does (catalog/alc-runtimes.txt, measured with AL 30.0.42.60748): AL0666, no test runs, exit 3.
    assert_eq!(code(&o), 3, "{}", text(&o));
    assert!(
        text(&o).contains(
            "src/Reader.Codeunit.al(5,14): error AL0666: 'GetResourceAsText' is not available in runtime version '13.0'. The supported runtime versions are: '14.0' or greater."
        ),
        "{}",
        text(&o)
    );
    assert!(text(&o).contains("Result: COMPILE ERRORS (exit 3)"), "{}", text(&o));
}

/// Code the AL compiler rejects is not run (al-runner issue 5602): an AL0118 in a procedure no test calls, an error in a
/// test codeunit the selection leaves out each stop the run with exit 3, the
/// errors on stderr in the AL compiler's form, and `compilationErrors` in the JSON document. Before, the selected test
/// passed with exit 0.
#[test]
fn code_that_does_not_compile_runs_no_test_and_exits_3() {
    let d = tempfile::tempdir().unwrap();
    let app = d.path().join("app");
    write(
        &app,
        "app.json",
        &app_json("11111111-0000-0000-0000-000000005602", "Logic", &[]),
    );
    let logic = "codeunit 50600 \"Probe Logic\" {\n    procedure Add(A: Integer; B: Integer): Integer begin exit(A + B); end;\n    procedure Other(): Integer var I: Integer; begin I := NoSuchProc(); exit(I); end;\n}\n";
    write(&app, "src/Logic.al", logic);
    let tests = d.path().join("tests");
    write(
        &tests,
        "app.json",
        &app_json(
            "11111111-0000-0000-0000-000000005603",
            "Logic Tests",
            &[("11111111-0000-0000-0000-000000005602", "Logic")],
        ),
    );
    write(
        &tests,
        "src/Tests.al",
        "codeunit 50650 \"Probe Tests\" { Subtype = Test; [Test] procedure Passes() var L: Codeunit \"Probe Logic\"; begin if L.Add(2, 3) <> 5 then Error('bad'); end; }",
    );
    let json = d.path().join("out.json");
    let run = |extra: &[&str]| {
        let mut args = vec!["test", app.to_str().unwrap(), tests.to_str().unwrap(), "--no-download"];
        args.extend_from_slice(extra);
        alrunner(&args)
    };
    let o = run(&[
        "--test-exact",
        "Codeunit50650.Passes",
        "--output-json",
        json.to_str().unwrap(),
    ]);
    assert_eq!(code(&o), 3, "{}", text(&o));
    assert!(
        text(&o)
            .contains("src/Logic.al(3,59): error AL0118: The name 'NoSuchProc' does not exist in the current context."),
        "{}",
        text(&o)
    );
    assert!(text(&o).contains("Result: COMPILE ERRORS (exit 3)"), "{}", text(&o));
    let doc: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&json).unwrap()).unwrap();
    assert_eq!(doc["exitCode"], 3);
    assert_eq!(doc["total"], 0);
    let groups = doc["compilationErrors"].as_array().expect("compilationErrors");
    assert_eq!(groups.len(), 1, "{doc}");
    assert!(
        groups[0]["file"].as_str().unwrap().ends_with("app/src/Logic.al"),
        "{doc}"
    );
    assert_eq!(groups[0]["errors"].as_array().unwrap().len(), 1, "{doc}");

    // Fixed, the test runs and passes.
    write(&app, "src/Logic.al", &logic.replace("NoSuchProc()", "1"));
    let o = run(&["--test-exact", "Codeunit50650.Passes"]);
    assert_eq!(code(&o), 0, "{}", text(&o));

    // An error in a test codeunit the selection does not pick.
    write(
        &tests,
        "src/Other.al",
        "codeunit 50651 \"Other Tests\" { Subtype = Test; [Test] procedure Q() begin NoSuchProc(); end; }",
    );
    let o = run(&["--codeunit", "50650"]);
    assert_eq!(code(&o), 3, "{}", text(&o));
    assert!(text(&o).contains("src/Other.al(1,75): error AL0118"), "{}", text(&o));
    std::fs::remove_file(tests.join("src/Other.al")).unwrap();

    // Source our parser does not accept is not a compile error: the parser can reject valid AL (al-runner issue 5576).
    // A test that does not need the file runs; one that needs it is not run (exit 7), with the parse error as the reason.
    write(
        &app,
        "src/Broken.al",
        "codeunit 50601 \"Broken\" {\n    procedure Ok(): Integer begin exit(1); end;\n    procedure Spin(: Integer\n    begin\n    end;\n}\n",
    );
    let o = run(&["--test-exact", "Codeunit50650.Passes"]);
    assert_eq!(code(&o), 0, "{}", text(&o));
    assert!(text(&o).contains("src/Broken.al(3,20): "), "{}", text(&o));
    write(
        &tests,
        "src/NeedsBroken.al",
        "codeunit 50652 \"Needs Broken\" { Subtype = Test; [Test] procedure UsesIt() var B: Codeunit \"Broken\"; begin if B.Ok() <> 1 then Error('bad'); end; }",
    );
    let o = run(&["--codeunit", "50652"]);
    assert_eq!(code(&o), 7, "{}", text(&o));
    assert!(text(&o).contains("the parser did not accept"), "{}", text(&o));
}

/// Issue 5613: with `--output-json`, a selection that matches no test still writes the document (exit 6, no rows,
/// the reason in `warnings`), on stdout and to a file.
#[test]
fn output_json_writes_the_document_when_the_selection_matches_no_test() {
    let d = tempfile::tempdir().unwrap();
    let app = mixed_app(d.path());
    let a = app.to_str().unwrap();
    let o = alrunner(&[
        "test",
        a,
        "--no-download",
        "--output-json",
        "--test",
        "Codeunit0.NoSuchTest",
    ]);
    assert_eq!(code(&o), 6, "{}", text(&o));
    let doc = json_of(&o);
    assert_eq!(doc["exitCode"], 6, "{doc}");
    assert_eq!(doc["total"], 0, "{doc}");
    assert_eq!(doc["tests"].as_array().map(Vec::len), Some(0), "{doc}");
    assert!(
        doc["warnings"][0].as_str().unwrap().contains("matches no test"),
        "{doc}"
    );
    let file = d.path().join("out.json");
    let o = alrunner(&[
        "test",
        a,
        "--no-download",
        "--output-json",
        file.to_str().unwrap(),
        "--test",
        "NoSuchTest",
    ]);
    assert_eq!(code(&o), 6, "{}", text(&o));
    let doc: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert_eq!(doc["exitCode"], 6, "{doc}");
}

/// Issue 5572: a test whose AL name is a quoted identifier is selected by that name, with --test-exact, --test
/// and --explain; issue 5612: --test with --test-exact selects only the tests both match.
#[test]
fn a_quoted_test_name_is_selected_by_its_al_name() {
    let d = tempfile::tempdir().unwrap();
    let dir = d.path().join("quoted");
    write(
        &dir,
        "app.json",
        &app_json("11111111-0000-0000-0000-000000000072", "Quoted", &[]),
    );
    write(
        &dir,
        "src/Tests.al",
        "codeunit 50650 \"Probe Tests\"\n{\n    Subtype = Test;\n\n    [Test]\n    procedure \"Space Step\"()\n    begin\n    end;\n\n    [Test]\n    procedure \"Dot.Name\"()\n    begin\n    end;\n\n    [Test]\n    procedure Passes()\n    begin\n    end;\n\n    [Test]\n    procedure PassesToo()\n    begin\n    end;\n}\n",
    );
    let a = dir.to_str().unwrap();
    let names = |args: &[&str]| -> Vec<String> {
        let mut all = vec!["test", a, "--no-download", "--output-json"];
        all.extend_from_slice(args);
        let o = alrunner(&all);
        assert_eq!(code(&o), 0, "{args:?}: {}", text(&o));
        let doc = json_of(&o);
        let mut n: Vec<String> = doc["tests"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap().to_string())
            .collect();
        n.sort();
        n
    };
    let space = vec!["Codeunit50650.Space Step".to_string()];
    assert_eq!(names(&["--test-exact", "Codeunit50650.Space Step"]), space);
    assert_eq!(names(&["--test-exact", "space step"]), space);
    assert_eq!(names(&["--test", "Space Step"]), space);
    assert_eq!(
        names(&["--test-exact", "Codeunit50650.Dot.Name"]),
        vec!["Codeunit50650.Dot.Name".to_string()]
    );
    assert_eq!(names(&["--test", "Dot"]), vec!["Codeunit50650.Dot.Name".to_string()]);
    assert_eq!(
        names(&["--test", "Codeunit50650.Passes", "--test-exact", "Codeunit50650.Passes"]),
        vec!["Codeunit50650.Passes".to_string()]
    );
    let o = alrunner(&["test", a, "--no-download", "--explain", "Space Step"]);
    assert_eq!(code(&o), 0, "{}", text(&o));
    assert!(text(&o).contains("Space Step"), "{}", text(&o));
}

/// Issues 5609 and 5607: the two isolation refusals say why (no corpus test pins BC's behavior) and what to use
/// instead.
#[test]
fn isolation_refusals_say_why_and_what_to_use_instead() {
    let d = tempfile::tempdir().unwrap();
    let dir = d.path().join("commit");
    write(
        &dir,
        "app.json",
        &app_json("11111111-0000-0000-0000-000000005607", "Commit", &[]),
    );
    write(
        &dir,
        "src/Tests.al",
        "codeunit 50660 \"Commit Tests\"\n{\n    Subtype = Test;\n\n    [Test]\n    procedure CommitCall()\n    begin\n        Commit();\n    end;\n}\n",
    );
    let a = dir.to_str().unwrap();
    let message = |isolation: &str| -> String {
        let o = alrunner(&["test", a, "--no-download", "--output-json", "--isolation", isolation]);
        assert_eq!(code(&o), 7, "{}", text(&o));
        json_of(&o)["tests"][0]["message"].as_str().unwrap().to_string()
    };
    let m = message("disabled");
    assert!(
        m.contains("no corpus test pins") && m.contains("--isolation codeunit"),
        "{m}"
    );
    let m = message("test");
    assert!(
        m.contains("no corpus test pins") && m.contains("--isolation codeunit"),
        "{m}"
    );
    let o = alrunner(&["test", a, "--no-download", "--isolation", "codeunit"]);
    assert_eq!(code(&o), 0, "{}", text(&o));
}

/// Issues 5607 and 5609 with RequiredTestIsolation on the test codeunit: `--isolation required` runs each test codeunit
/// under its own RequiredTestIsolation, as BC's TestRunnerHub does on BC 28 and later (corpus
/// codeunit/lt-qu-commit-isolation/QuCommitIsolation.al:10-16, 25-39).
const REQUIRED: &str = r#"
table 50670 "Req Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
codeunit 50670 "Req Function"
{
    Subtype = Test;
    RequiredTestIsolation = Function;

    [Test]
    procedure A_CommitCall()
    var
        R: Record "Req Row";
    begin
        R.Id := 1;
        R.Insert();
        Commit();
    end;

    [Test]
    procedure B_NextTestSeesNothing()
    var
        R: Record "Req Row";
    begin
        if R.Count() <> 0 then
            Error('kept');
    end;
}
codeunit 50671 "Req Disabled A"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;

    [Test]
    procedure Passes()
    var
        R: Record "Req Row";
    begin
        R.Id := 2;
        R.Insert();
    end;
}
codeunit 50672 "Req Disabled B"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;

    [Test]
    procedure SeesTheRow()
    var
        R: Record "Req Row";
    begin
        if not R.Get(2) then
            Error('gone');
    end;
}
"#;

/// A codeunit the TestRunnerHub mode still refuses: RequiredTestIsolation = Function with handlers.
const REQUIRED_HANDLERS: &str = r#"
codeunit 50675 "Req Handlers"
{
    Subtype = Test;
    RequiredTestIsolation = Function;

    [Test]
    [HandlerFunctions('Msg')]
    procedure Shows()
    begin
        Message('x');
    end;

    [MessageHandler]
    procedure Msg(Text: Text[1024])
    begin
    end;
}
"#;

#[test]
fn isolation_required_runs_each_codeunit_under_its_required_test_isolation() {
    let d = tempfile::tempdir().unwrap();
    let app = d.path().join("req");
    write(
        &app,
        "app.json",
        &app_json("11111111-0000-0000-0000-000000005670", "Req", &[]),
    );
    write(&app, "T.al", REQUIRED);
    let a = app.to_str().unwrap();
    let run = |extra: &[&str]| {
        let mut args = vec!["test", a, "--no-download", "--no-test-data", "--output-json"];
        args.extend_from_slice(extra);
        alrunner(&args)
    };
    // BC 28 and later: Function returns the next test to its start, the Commit() included (QuCommitIsolation.al:466-488);
    // Disabled keeps the row for the next codeunit (QuCommitIsolation.al:695-718).
    let o = run(&["--isolation", "required"]);
    assert_eq!(code(&o), 0, "{}", text(&o));
    assert_eq!(json_of(&o)["passed"], 4, "{}", text(&o));
    // The default is unchanged: Codeunit isolation for every codeunit, so the second test sees the first one's row.
    let o = run(&[]);
    assert_eq!(code(&o), 1, "{}", text(&o));
    assert_eq!(
        row(&json_of(&o), "Codeunit50670.B_NextTestSeesNothing")["message"],
        "kept"
    );
    assert_eq!(row(&json_of(&o), "Codeunit50672.SeesTheRow")["message"], "gone");
    // BC 27: every codeunit runs under Codeunit isolation (QuCommitIsolation.al:36-40, the BC28PLUS guards), and the
    // report says so.
    let o = run(&["--isolation", "required", "--bc-version", "27.5.46862.48827"]);
    assert_eq!(code(&o), 1, "{}", text(&o));
    assert_eq!(
        row(&json_of(&o), "Codeunit50670.B_NextTestSeesNothing")["message"],
        "kept"
    );
    assert!(text(&o).contains("BC 27"), "{}", text(&o));
    // The old values stay refused: a Commit() under --isolation test.
    let o = run(&["--isolation", "test"]);
    let m = row(&json_of(&o), "Codeunit50670.A_CommitCall")["message"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(m.contains("Commit inside a test under TestIsolation = Function"), "{m}");
    // What the mode does not cover is refused per test, with the reason.
    let h = d.path().join("handlers");
    write(
        &h,
        "app.json",
        &app_json("11111111-0000-0000-0000-000000005675", "ReqHandlers", &[]),
    );
    write(&h, "T.al", REQUIRED_HANDLERS);
    let o = alrunner(&[
        "test",
        h.to_str().unwrap(),
        "--no-download",
        "--no-test-data",
        "--output-json",
        "--isolation",
        "required",
    ]);
    assert_eq!(code(&o), 7, "{}", text(&o));
    let m = row(&json_of(&o), "Codeunit50675.Shows")["message"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(
        m.contains("RequiredTestIsolation = Function on a test codeunit with handlers"),
        "{m}"
    );
}
/// Issue 5608: an outbound request no handler answers fails with BC's pinned error where BC fails it without a
/// connection (an internal address: PyHttpPolicy.al:82-90) and under BlockOutboundRequests (AyHttpHandler.al:121); a
/// request BC sends for real is refused with a reason that says why and what to do instead.
#[test]
fn an_unhandled_outbound_request_fails_as_bc_does_or_says_why_it_is_refused() {
    let d = tempfile::tempdir().unwrap();
    let dir = d.path().join("http");
    write(
        &dir,
        "app.json",
        &app_json("11111111-0000-0000-0000-000000005608", "Http", &[]),
    );
    let unit = |id: u32, policy: &str| {
        format!(
            "codeunit {id} \"Http Tests {id}\"\n{{\n    Subtype = Test;\n{policy}\n    [Test]\n    procedure Local()\n    var\n        Client: HttpClient;\n        Response: HttpResponseMessage;\n    begin\n        Client.Get('http://localhost:1/x', Response);\n    end;\n\n    [Test]\n    procedure Public()\n    var\n        Client: HttpClient;\n        Response: HttpResponseMessage;\n    begin\n        Client.Get('http://example.com/x', Response);\n    end;\n}}\n"
        )
    };
    write(&dir, "src/Default.al", &unit(50670, ""));
    write(
        &dir,
        "src/Block.al",
        &unit(50671, "    TestHttpRequestPolicy = BlockOutboundRequests;"),
    );
    let o = alrunner(&["test", dir.to_str().unwrap(), "--no-download", "--output-json"]);
    let doc = json_of(&o);
    let message = |name: &str| row(&doc, name)["message"].as_str().unwrap().to_string();
    assert!(
        message("Codeunit50670.Local").contains("violates security requirements"),
        "{doc}"
    );
    assert!(
        message("Codeunit50671.Public").starts_with("Unhandled outbound HTTP requests are not permitted"),
        "{doc}"
    );
    let m = message("Codeunit50670.Public");
    assert!(
        m.contains("[HttpClientHandler]") && m.contains("BlockOutboundRequests"),
        "{m}"
    );
}

/// Issue 5611: --run-selected-only runs only the selected tests of a codeunit, as BC's test tool runs only the test
/// methods that are enabled: a sibling that is not selected does not run, so its global does not carry over.
#[test]
fn run_selected_only_leaves_the_other_tests_of_the_codeunit_out() {
    let d = tempfile::tempdir().unwrap();
    let dir = d.path().join("persist");
    write(
        &dir,
        "app.json",
        &app_json("11111111-0000-0000-0000-000000005611", "Persist", &[]),
    );
    write(
        &dir,
        "src/Tests.al",
        "codeunit 50680 \"Persist Tests\"\n{\n    Subtype = Test;\n\n    var\n        Counter: Integer;\n\n    [Test]\n    procedure PersistA()\n    begin\n        Counter += 1;\n    end;\n\n    [Test]\n    procedure PersistB()\n    begin\n        if Counter <> 0 then\n            Error('counter=%1', Counter);\n    end;\n}\n",
    );
    let a = dir.to_str().unwrap();
    let o = alrunner(&["test", a, "--no-download", "--test-exact", "Codeunit50680.PersistB"]);
    assert_eq!(code(&o), 1, "the whole codeunit runs by default: {}", text(&o));
    assert!(text(&o).contains("counter=1"), "{}", text(&o));
    let o = alrunner(&[
        "test",
        a,
        "--no-download",
        "--test-exact",
        "Codeunit50680.PersistB",
        "--run-selected-only",
    ]);
    assert_eq!(code(&o), 0, "{}", text(&o));
    assert!(text(&o).contains("Tests: 1 total, 1 passed"), "{}", text(&o));
}
