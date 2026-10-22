//! `al-runner test --affected` (ADR-050) end to end: a run stores its result, the next one runs only the test
//! codeunits a change since then can affect, and keeps the stored outcomes of the others without counting them as
//! passed or failed in this run. Each test writes its apps and its stored result under its own temporary folder.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

const APP_JSON: &str = r#"{
  "id": "5a1f0c3e-0000-4000-8000-00000000a001",
  "name": "Affected Fixture",
  "publisher": "AL Runner Tests",
  "version": "1.0.0.0",
  "idRanges": [{ "from": 50100, "to": 50149 }],
  "dependencies": []
}"#;

const LIB_A: &str = r#"codeunit 50100 "Lib A"
{
    procedure A(): Integer
    begin
        exit(1);
    end;
}
"#;

const LIB_B: &str = r#"codeunit 50101 "Lib B"
{
    procedure B(): Integer
    begin
        exit(2);
    end;
}
"#;

const TESTS_A: &str = r#"codeunit 50110 "Tests A"
{
    Subtype = Test;
    [Test]
    procedure UsesA()
    var
        L: Codeunit "Lib A";
    begin
        if L.A() <> 1 then Error('A is not 1');
    end;
}
"#;

const TESTS_B: &str = r#"codeunit 50111 "Tests B"
{
    Subtype = Test;
    [Test]
    procedure UsesB()
    var
        L: Codeunit "Lib B";
    begin
        if L.B() <> 2 then Error('B is not 2');
    end;
}
"#;

const TESTS_RED: &str = r#"codeunit 50112 "Tests Red"
{
    Subtype = Test;
    [Test]
    procedure AlwaysRed()
    begin
        Error('red');
    end;
}
"#;

/// A fresh folder for one test: the app in `app/`, the stored result at `store.json.gz`.
fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("affected").join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("app")).unwrap();
    dir
}

fn write_app(dir: &Path, with_red: bool) {
    let app = dir.join("app");
    std::fs::write(app.join("app.json"), APP_JSON).unwrap();
    std::fs::write(app.join("LibA.Codeunit.al"), LIB_A).unwrap();
    std::fs::write(app.join("LibB.Codeunit.al"), LIB_B).unwrap();
    std::fs::write(app.join("TestsA.Codeunit.al"), TESTS_A).unwrap();
    std::fs::write(app.join("TestsB.Codeunit.al"), TESTS_B).unwrap();
    if with_red {
        std::fs::write(app.join("TestsRed.Codeunit.al"), TESTS_RED).unwrap();
    }
}

fn edit(dir: &Path, file: &str, from: &str, to: &str) {
    let p = dir.join("app").join(file);
    let text = std::fs::read_to_string(&p).unwrap();
    assert!(text.contains(from), "{from} in {text}");
    std::fs::write(&p, text.replacen(from, to, 1)).unwrap();
}

fn alrunner(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_al-runner"))
        .env("AL_CHECK_THREADS", "2")
        .env("AL_LOWER_THREADS", "2")
        .env("AL_TEST_THREADS", "2")
        .env("ALRUNNER_NO_PATH_NOTE", "1")
        .env("NO_COLOR", "1")
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

fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

/// A text run of the app in `dir` with `--affected` and the store in `dir`.
fn run(dir: &Path, extra: &[&str]) -> Output {
    let app = s(&dir.join("app"));
    let store = s(&dir.join("store.json.gz"));
    let mut args = vec![
        "test",
        app.as_str(),
        "--no-download",
        "--no-test-data",
        "--affected",
        "--affected-store",
        store.as_str(),
    ];
    args.extend_from_slice(extra);
    alrunner(&args)
}

/// The `--output-json` document of [`run`].
fn json_run(dir: &Path, extra: &[&str]) -> (Value, Output) {
    let mut args = vec!["--output-json"];
    args.extend_from_slice(extra);
    let o = run(dir, &args);
    let doc: Value = serde_json::from_slice(&o.stdout).unwrap_or_else(|e| panic!("{e}: {}", text(&o)));
    (doc, o)
}

fn methods(doc: &Value) -> Vec<String> {
    let mut m: Vec<String> = doc["tests"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["method"].as_str().unwrap().to_string())
        .collect();
    m.sort();
    m
}

#[test]
fn the_first_run_is_full_and_the_second_runs_only_what_a_change_reaches() {
    // Test plan 1, 2 and 12.
    let dir = scratch("second_run_narrowed");
    write_app(&dir, false);
    let (doc, o) = json_run(&dir, &[]);
    assert_eq!(code(&o), 0, "{}", text(&o));
    let sel = &doc["selection"];
    assert_eq!(sel["mode"], "affected", "{doc:#}");
    assert_eq!(sel["forcedFull"], true, "{doc:#}");
    assert_eq!(sel["reason"], "no stored result", "{doc:#}");
    assert_eq!(sel["ran"], 2, "{doc:#}");
    assert_eq!(sel["skipped"], 0, "{doc:#}");
    assert!(dir.join("store.json.gz").is_file());

    edit(&dir, "LibA.Codeunit.al", "exit(1);", "exit(0 + 1);");
    let (doc, o) = json_run(&dir, &[]);
    assert_eq!(code(&o), 0, "{}", text(&o));
    let sel = &doc["selection"];
    assert_eq!(sel["forcedFull"], false, "{doc:#}");
    assert_eq!(sel["ran"], 1, "{doc:#}");
    assert_eq!(sel["skipped"], 1, "{doc:#}");
    assert_eq!(sel["skippedFailing"], 0, "{doc:#}");
    assert_eq!(
        sel["changedObjects"],
        serde_json::json!(["codeunit 50100 Lib A"]),
        "{doc:#}"
    );
    assert_eq!(methods(&doc), vec!["UsesA"], "{doc:#}");
    assert_eq!(doc["passed"], 1, "{doc:#}");
    assert_eq!(doc["total"], 1, "{doc:#}");
}

#[test]
fn nothing_changed_runs_nothing_and_exits_zero() {
    // Test plan 6: an edit that moves no line changes no object.
    let dir = scratch("nothing_changed");
    write_app(&dir, false);
    assert_eq!(code(&run(&dir, &[])), 0);
    edit(&dir, "LibB.Codeunit.al", "exit(2);", "exit(2); // two");
    let (doc, o) = json_run(&dir, &[]);
    assert_eq!(code(&o), 0, "{}", text(&o));
    assert_eq!(doc["selection"]["ran"], 0, "{doc:#}");
    assert_eq!(doc["selection"]["skipped"], 2, "{doc:#}");
    assert_eq!(doc["selection"]["changedObjects"], serde_json::json!([]), "{doc:#}");
}

#[test]
fn a_line_shift_above_an_object_changes_it() {
    // Test plan 5.
    let dir = scratch("line_shift");
    write_app(&dir, false);
    assert_eq!(code(&run(&dir, &[])), 0);
    edit(&dir, "LibB.Codeunit.al", "codeunit 50101", "\ncodeunit 50101");
    let (doc, _) = json_run(&dir, &[]);
    assert_eq!(methods(&doc), vec!["UsesB"], "{doc:#}");
}

#[test]
fn an_added_object_is_a_full_run_and_an_added_test_codeunit_runs_alone() {
    // Test plan 7.
    let dir = scratch("added");
    write_app(&dir, false);
    assert_eq!(code(&run(&dir, &[])), 0);
    std::fs::write(
        dir.join("app/LibC.Codeunit.al"),
        "codeunit 50102 \"Lib C\" { procedure C() begin end; }\n",
    )
    .unwrap();
    let (doc, _) = json_run(&dir, &[]);
    assert_eq!(doc["selection"]["forcedFull"], true, "{doc:#}");
    assert_eq!(
        doc["selection"]["reason"], "an object was added: codeunit 50102 Lib C",
        "{doc:#}"
    );
    std::fs::write(dir.join("app/TestsRed.Codeunit.al"), TESTS_RED).unwrap();
    let (doc, o) = json_run(&dir, &[]);
    assert_eq!(doc["selection"]["forcedFull"], false, "{doc:#}");
    assert_eq!(methods(&doc), vec!["AlwaysRed"], "{doc:#}");
    assert_eq!(code(&o), 1, "{}", text(&o));
}

#[test]
fn a_skipped_failure_keeps_the_run_red_and_is_skipped_in_junit() {
    // Test plan 14 and 15.
    let dir = scratch("skipped_failing");
    write_app(&dir, true);
    assert_eq!(code(&run(&dir, &[])), 1);
    edit(&dir, "LibA.Codeunit.al", "exit(1);", "exit(0 + 1);");
    let junit = s(&dir.join("junit.xml"));
    let o = run(&dir, &["--junit", junit.as_str()]);
    let out = text(&o);
    assert_eq!(code(&o), 1, "{out}");
    assert!(
        out.contains("affected: ran 1 of 3   skipped-unaffected 1   skipped-failing 1"),
        "{out}"
    );
    assert!(out.contains("changed: codeunit 50100 Lib A"), "{out}");
    assert!(
        out.contains("not re-run, still failing from an earlier run: Tests Red::AlwaysRed"),
        "{out}"
    );
    assert!(out.contains("0 failed (+1 still failing, not re-run)"), "{out}");
    let xml = std::fs::read_to_string(&junit).unwrap();
    assert!(
        xml.contains(r#"<skipped message="not affected by the change; stored outcome: fail" />"#),
        "{xml}"
    );
    assert!(
        xml.contains(r#"<skipped message="not affected by the change; stored outcome: pass" />"#),
        "{xml}"
    );
    let (doc, _) = json_run(&dir, &[]);
    assert_eq!(doc["selection"]["skippedFailing"], 1, "{doc:#}");
    assert_eq!(doc["failed"], 0, "{doc:#}");
    assert_eq!(doc["exitCode"], 1, "{doc:#}");
}

#[test]
fn include_failing_runs_a_failing_codeunit_the_change_does_not_reach() {
    // Test plan 9.
    let dir = scratch("include_failing");
    write_app(&dir, true);
    run(&dir, &[]);
    edit(&dir, "LibA.Codeunit.al", "exit(1);", "exit(0 + 1);");
    let (doc, _) = json_run(&dir, &["--include-failing"]);
    assert_eq!(methods(&doc), vec!["AlwaysRed", "UsesA"], "{doc:#}");
    assert_eq!(doc["selection"]["skippedFailing"], 0, "{doc:#}");
}

#[test]
fn a_changed_app_json_is_a_full_run_with_or_without_strict_environment() {
    // Test plan 8.
    let dir = scratch("app_json");
    write_app(&dir, false);
    run(&dir, &[]);
    for (i, extra) in [&[][..], &["--strict-environment"][..]].into_iter().enumerate() {
        // Each run stores the new app.json: change it again for the next one.
        edit(
            &dir,
            "app.json",
            "\"dependencies\": []",
            &format!("\"dependencies\": [], \"brief\": \"{i}\""),
        );
        let (doc, _) = json_run(&dir, extra);
        assert_eq!(doc["selection"]["forcedFull"], true, "{doc:#}");
        assert_eq!(doc["selection"]["reason"], "app.json changed", "{doc:#}");
    }
}

#[test]
fn a_stored_result_from_another_checkout_of_the_same_app_is_used() {
    // Test plan 13.
    let one = scratch("checkout_one");
    let two = scratch("checkout_two");
    write_app(&one, false);
    write_app(&two, false);
    run(&one, &[]);
    std::fs::copy(one.join("store.json.gz"), two.join("store.json.gz")).unwrap();
    let (doc, _) = json_run(&two, &[]);
    assert_eq!(doc["selection"]["forcedFull"], false, "{doc:#}");
    assert_eq!(doc["selection"]["ran"], 0, "{doc:#}");
}

#[test]
fn a_failed_compile_leaves_the_stored_result_unchanged() {
    // Test plan 17.
    let dir = scratch("compile_failed");
    write_app(&dir, false);
    run(&dir, &[]);
    let before = std::fs::read(dir.join("store.json.gz")).unwrap();
    edit(&dir, "LibA.Codeunit.al", "exit(1);", "exit(Missing);");
    let o = run(&dir, &[]);
    assert_eq!(code(&o), 3, "{}", text(&o));
    assert_eq!(std::fs::read(dir.join("store.json.gz")).unwrap(), before);
}

#[test]
fn the_default_store_is_in_the_cache_folder() {
    let dir = scratch("default_store");
    write_app(&dir, false);
    let app = s(&dir.join("app"));
    let cache = s(&dir.join("cache"));
    let args = [
        "test",
        app.as_str(),
        "--no-download",
        "--no-test-data",
        "--no-compile-cache",
        "--cache-dir",
        cache.as_str(),
        "--affected",
    ];
    assert_eq!(code(&alrunner(&args)), 0);
    let o = alrunner(&args);
    let out = text(&o);
    assert!(
        out.contains("affected: ran 0 of 2   skipped-unaffected 2   skipped-failing 0"),
        "{out}"
    );
    let stored: Vec<_> = std::fs::read_dir(dir.join("cache/affected")).unwrap().collect();
    assert_eq!(stored.len(), 1);
}

#[test]
fn flags_that_cannot_go_with_affected_are_refused() {
    let dir = scratch("refused");
    write_app(&dir, false);
    let app = s(&dir.join("app"));
    for args in [
        vec!["test", app.as_str(), "--affected", "--test", "UsesA"],
        vec!["test", app.as_str(), "--affected", "--codeunit", "50110"],
        vec!["test", app.as_str(), "--affected", "--exclude-test", "UsesA"],
        vec!["test", app.as_str(), "--include-failing"],
        vec!["test", app.as_str(), "--strict-environment"],
        vec!["test", app.as_str(), "--affected-store", "x.json.gz"],
    ] {
        let o = alrunner(&args);
        assert_eq!(code(&o), 2, "{args:?}: {}", text(&o));
    }
}

// ---- serve: the request fields `affectedOnly`, `includeFailing` and `strictEnvironment` (ADR-050 part 4) ----

struct Server {
    child: std::process::Child,
    stdin: Option<std::process::ChildStdin>,
    stdout: std::io::BufReader<std::process::ChildStdout>,
}

impl Server {
    fn start(extra: &[&str]) -> Server {
        use std::process::Stdio;
        let mut child = Command::new(env!("CARGO_BIN_EXE_al-runner"))
            .arg("serve")
            .args(extra)
            .env("AL_CHECK_THREADS", "2")
            .env("AL_LOWER_THREADS", "2")
            .env("AL_TEST_THREADS", "2")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("start al-runner serve");
        let stdin = child.stdin.take();
        let stdout = std::io::BufReader::new(child.stdout.take().unwrap());
        let mut s = Server { child, stdin, stdout };
        assert_eq!(s.line()["ready"], true);
        s
    }

    fn send(&mut self, v: Value) {
        use std::io::Write;
        let stdin = self.stdin.as_mut().unwrap();
        writeln!(stdin, "{v}").unwrap();
        stdin.flush().unwrap();
    }

    fn line(&mut self) -> Value {
        use std::io::BufRead;
        let mut s = String::new();
        assert!(self.stdout.read_line(&mut s).unwrap() > 0, "the server closed stdout");
        serde_json::from_str(&s).unwrap_or_else(|e| panic!("{e}: {s}"))
    }

    /// The `test` lines and the summary (or the error) of the next request.
    fn request(&mut self, v: Value) -> (Vec<Value>, Value) {
        self.send(v);
        let mut tests = Vec::new();
        loop {
            let l = self.line();
            if l["type"] == "summary" || l.get("error").is_some() {
                return (tests, l);
            }
            if l["type"] == "test" {
                tests.push(l);
            }
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        drop(self.stdin.take());
        let _ = self.child.wait();
    }
}

fn names(tests: &[Value]) -> Vec<String> {
    let mut n: Vec<String> = tests
        .iter()
        .map(|t| t["name"].as_str().unwrap_or_default().to_string())
        .collect();
    n.sort();
    n
}

fn serve_store(dir: &Path) -> Vec<String> {
    vec![
        "--no-download".into(),
        "--no-test-data".into(),
        "--affected-store".into(),
        s(&dir.join("store.json.gz")),
    ]
}

#[test]
fn serve_selects_by_the_stored_result_and_a_restarted_server_reads_it() {
    // Test plan 1, 2 and 12 in serve.
    let dir = scratch("serve_narrowed");
    write_app(&dir, false);
    let app = s(&dir.join("app"));
    let opts = serve_store(&dir);
    let opts: Vec<&str> = opts.iter().map(String::as_str).collect();
    let mut server = Server::start(&opts);
    let req = serde_json::json!({"command": "runTests", "sourcePaths": [app], "affectedOnly": true});
    let (tests, summary) = server.request(req.clone());
    assert_eq!(tests.len(), 2, "{summary}");
    assert_eq!(summary["selection"]["forcedFull"], true, "{summary}");
    assert_eq!(summary["selection"]["reason"], "no stored result", "{summary}");
    edit(&dir, "LibA.Codeunit.al", "exit(1);", "exit(0 + 1);");
    let (tests, summary) = server.request(req.clone());
    assert_eq!(names(&tests), vec!["Codeunit50110.UsesA"], "{summary}");
    assert_eq!(summary["selection"]["mode"], "affected", "{summary}");
    assert_eq!(summary["selection"]["ran"], 1, "{summary}");
    assert_eq!(summary["selection"]["skipped"], 1, "{summary}");
    assert_eq!(
        summary["selection"]["changedObjects"],
        serde_json::json!(["codeunit 50100 Lib A"]),
        "{summary}"
    );
    assert_eq!(summary["exitCode"], 0, "{summary}");
    // A request without affectedOnly has no selection.
    let (_, summary) = server.request(serde_json::json!({"command": "runTests", "sourcePaths": [app]}));
    assert!(summary.get("selection").is_none(), "{summary}");
    drop(server);

    edit(&dir, "LibB.Codeunit.al", "exit(2);", "exit(1 + 1);");
    let mut server = Server::start(&opts);
    let (tests, summary) = server.request(req);
    assert_eq!(names(&tests), vec!["Codeunit50111.UsesB"], "{summary}");
    assert_eq!(summary["selection"]["forcedFull"], false, "{summary}");
}

#[test]
fn serve_keeps_a_skipped_failure_red_and_include_failing_reruns_it() {
    // Test plan 9 and 14 in serve.
    let dir = scratch("serve_failing");
    write_app(&dir, true);
    let app = s(&dir.join("app"));
    let opts = serve_store(&dir);
    let opts: Vec<&str> = opts.iter().map(String::as_str).collect();
    let mut server = Server::start(&opts);
    let req = serde_json::json!({"command": "runTests", "sourcePaths": [app], "affectedOnly": true});
    server.request(req.clone());
    edit(&dir, "LibA.Codeunit.al", "exit(1);", "exit(0 + 1);");
    let (tests, summary) = server.request(req);
    assert_eq!(names(&tests), vec!["Codeunit50110.UsesA"], "{summary}");
    assert_eq!(summary["selection"]["skippedFailing"], 1, "{summary}");
    assert_eq!(summary["exitCode"], 1, "{summary}");
    let (tests, summary) = server.request(serde_json::json!({
        "command": "runTests", "sourcePaths": [app], "affectedOnly": true,
        "includeFailing": true, "strictEnvironment": true
    }));
    assert_eq!(names(&tests), vec!["Codeunit50112.AlwaysRed"], "{summary}");
    assert_eq!(summary["selection"]["skippedFailing"], 0, "{summary}");
}

#[test]
fn serve_refuses_test_and_exclude_tests_with_affected_only_and_warns_on_execute() {
    let dir = scratch("serve_refused");
    write_app(&dir, false);
    let app = s(&dir.join("app"));
    let opts = serve_store(&dir);
    let opts: Vec<&str> = opts.iter().map(String::as_str).collect();
    let mut server = Server::start(&opts);
    for extra in [
        serde_json::json!({"test": "UsesA"}),
        serde_json::json!({"excludeTests": ["UsesA"]}),
    ] {
        let mut req = serde_json::json!({"command": "runTests", "sourcePaths": [app], "affectedOnly": true});
        for (k, v) in extra.as_object().unwrap() {
            req[k] = v.clone();
        }
        let (_, l) = server.request(req);
        assert!(
            l["error"].as_str().is_some_and(|e| e.contains("cannot be combined")),
            "{l}"
        );
    }
    // Without affectedOnly, includeFailing changes nothing and says so.
    let (_, summary) = server.request(serde_json::json!({
        "command": "runTests", "sourcePaths": [app], "includeFailing": true
    }));
    assert!(summary.get("selection").is_none(), "{summary}");
    assert!(summary["warnings"].to_string().contains("includeFailing"), "{summary}");
    server.send(serde_json::json!({"command": "execute", "code": "Message('x');", "affectedOnly": true}));
    let l = server.line();
    assert!(l["warnings"].to_string().contains("affectedOnly"), "{l}");
}

#[test]
fn a_serve_store_option_alone_is_accepted_and_the_flags_are_not() {
    let dir = scratch("serve_options");
    for flag in ["--affected", "--include-failing", "--strict-environment"] {
        let o = alrunner(&["serve", flag]);
        assert_eq!(code(&o), 2, "{flag}: {}", text(&o));
    }
    let store = s(&dir.join("store.json.gz"));
    let o = alrunner(&["serve", "--affected-store"]);
    assert_eq!(code(&o), 2, "{}", text(&o));
    // With the store option the server starts (and ends with its input).
    let o = Command::new(env!("CARGO_BIN_EXE_al-runner"))
        .args(["serve", "--no-download", "--affected-store", store.as_str()])
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    assert_eq!(code(&o), 0, "{}", text(&o));
    assert!(String::from_utf8_lossy(&o.stdout).contains("\"ready\":true"));
}
