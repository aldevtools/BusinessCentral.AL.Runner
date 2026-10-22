//! `al-runner serve` end to end (ADR-028): the v2 protocol on stdin and stdout.

use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

struct Server {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
}

impl Server {
    fn start(extra: &[&str]) -> Server {
        Server::start_with(extra, &[])
    }

    fn start_with(extra: &[&str], env: &[(&str, &str)]) -> Server {
        let mut child = Command::new(env!("CARGO_BIN_EXE_al-runner"))
            .arg("serve")
            .args(extra)
            .envs(env.iter().copied())
            .env("AL_CHECK_THREADS", "2")
            .env("AL_LOWER_THREADS", "2")
            .env("AL_TEST_THREADS", "2")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("start al-runner serve");
        let stdin = child.stdin.take();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Server { child, stdin, stdout }
    }

    fn send(&mut self, v: Value) {
        let stdin = self.stdin.as_mut().unwrap();
        writeln!(stdin, "{v}").unwrap();
        stdin.flush().unwrap();
    }

    /// The next line, which must be one JSON object.
    fn line(&mut self) -> Value {
        let mut s = String::new();
        assert!(self.stdout.read_line(&mut s).unwrap() > 0, "the server closed stdout");
        serde_json::from_str(&s).unwrap_or_else(|e| panic!("stdout holds more than protocol: {e}: {s}"))
    }

    /// Lines up to and including the first one of `type` summary or an error.
    fn until_summary(&mut self) -> Vec<Value> {
        let mut out = Vec::new();
        loop {
            let l = self.line();
            let done = l["type"] == "summary" || l.get("error").is_some();
            out.push(l);
            if done {
                return out;
            }
        }
    }
}

fn write(dir: &Path, rel: &str, content: &str) {
    let p = dir.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, content).unwrap();
}

fn app(root: &Path, name: &str, al: &str) -> PathBuf {
    let dir = root.join(name);
    write(
        &dir,
        "app.json",
        &format!(
            r#"{{"id":"11111111-0000-0000-0000-0000000000{:02}","name":"{name}","publisher":"Test","version":"1.0.0.0","dependencies":[]}}"#,
            name.len() % 100
        ),
    );
    write(&dir, "src/T.al", al);
    dir
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
}
"#;

#[test]
fn a_run_streams_test_lines_then_a_summary_and_shutdown_ends_the_server() {
    let d = tempfile::tempdir().unwrap();
    let a = app(d.path(), "mixed", MIXED);
    let mut s = Server::start(&["--no-download", "--no-test-data"]);
    assert_eq!(s.line(), json!({"ready": true}));
    // Case-insensitive command names, as in v2.
    s.send(json!({"command": "RunTests", "sourcePaths": [a], "coverage": true, "perTestCoverage": true}));
    let lines = s.until_summary();
    let tests: Vec<&Value> = lines.iter().filter(|l| l["type"] == "test").collect();
    assert_eq!(tests.len(), 2, "{lines:?}");
    let fails = tests.iter().find(|t| t["name"] == "Codeunit50100.Fails").unwrap();
    assert_eq!(
        (fails["status"].as_str(), fails["message"].as_str()),
        (Some("fail"), Some("boom 5"))
    );
    assert!(
        fails["stackFrames"].as_array().is_some_and(|f| !f.is_empty()),
        "{fails}"
    );
    let summary = lines.last().unwrap();
    assert_eq!(summary["type"], "summary");
    assert_eq!(
        (
            summary["exitCode"].as_u64(),
            summary["passed"].as_u64(),
            summary["failed"].as_u64(),
            summary["total"].as_u64()
        ),
        (Some(1), Some(1), Some(1), Some(2))
    );
    assert_eq!(
        (summary["protocolVersion"].as_u64(), summary["cached"].as_bool()),
        (Some(2), Some(false))
    );
    assert!(
        summary["coverage"][0]["file"].as_str().unwrap().ends_with("src/T.al"),
        "{summary}"
    );
    // Only the test that ran a statement has an entry: Passes has an empty body.
    assert_eq!(summary["perTestCoverage"].as_array().unwrap().len(), 1);
    assert_eq!(summary["perTestCoverage"][0]["test"], "Codeunit50100.Fails");
    assert_eq!(summary["cached"], false, "the first request compiles");
    // A second request on the same server: selection and exclusion, on the compile of the first.
    s.send(json!({"command": "runTests", "sourcePaths": [a], "test": "Codeunit50100.Passes", "excludeTests": ["Codeunit50100.Fails"]}));
    let lines = s.until_summary();
    assert_eq!(lines.iter().filter(|l| l["type"] == "test").count(), 1, "{lines:?}");
    assert_eq!(lines.last().unwrap()["exitCode"], 0);
    assert_eq!(
        lines.last().unwrap()["cached"],
        true,
        "nothing changed, nothing is compiled: {lines:?}"
    );
    assert!(lines.last().unwrap().get("coverage").is_none());
    s.send(json!({"command": "shutdown"}));
    assert_eq!(s.line(), json!({"status": "shutting down"}));
    assert!(s.child.wait().unwrap().success());
}

#[test]
fn bad_requests_get_an_error_line_and_the_server_goes_on() {
    let d = tempfile::tempdir().unwrap();
    let a = app(d.path(), "mixed", MIXED);
    let mut s = Server::start(&["--no-download"]);
    s.line();
    s.stdin.as_mut().unwrap().write_all(b"this is not json\n").unwrap();
    assert!(s.line()["error"].as_str().unwrap().contains("JSON"));
    for (request, needle) in [
        (json!({"command": "frobnicate"}), "unknown command"),
        (json!({"nocommand": 1}), "needs a `command`"),
        (json!({"command": "runTests"}), "sourcePaths"),
        (
            json!({"command": "runTests", "sourcePaths": [a], "bogus": 1}),
            "unknown field",
        ),
        (
            json!({"command": "runTests", "sourcePaths": [a], "affectedOnly": true, "test": "Passes"}),
            "cannot be combined",
        ),
        (
            json!({"command": "runTests", "sourcePaths": ["/no/such/app"]}),
            "no such file or folder",
        ),
        (
            json!({"command": "execute", "sourcePaths": ["/no/such/app"]}),
            "no such file or folder",
        ),
    ] {
        s.send(request);
        let l = s.line();
        assert!(l["error"].as_str().is_some_and(|e| e.contains(needle)), "{needle}: {l}");
    }
    // cancel with nothing running is a no-op that says so.
    s.send(json!({"command": "cancel"}));
    assert_eq!(s.line(), json!({"type": "ack", "command": "cancel", "noop": true}));
    // A field of another command only earns a warning.
    s.send(json!({"command": "runTests", "sourcePaths": [a], "code": "x", "test": "Passes"}));
    let lines = s.until_summary();
    assert_eq!(
        lines.last().unwrap()["warnings"].as_array().unwrap().len(),
        1,
        "{lines:?}"
    );
    // End of input ends the server.
    drop(s.stdin.take());
    assert!(s.child.wait().unwrap().success());
}

const SLOW: &str = r#"
codeunit 50200 "Slow Tests"
{
    Subtype = Test;

    [Test]
    procedure Spins()
    var
        i: Integer;
    begin
        for i := 1 to 2000000000 do;
    end;

    [Test]
    procedure SpinsToo()
    var
        i: Integer;
    begin
        for i := 1 to 2000000000 do;
    end;
}
"#;

#[test]
fn cancel_stops_the_run_after_the_current_test_and_reports_what_ran() {
    let d = tempfile::tempdir().unwrap();
    let a = app(d.path(), "slow", SLOW);
    // The CPU limit ends the first test after 2 s; the second never starts.
    let mut s = Server::start(&["--no-download", "--no-test-data", "--test-timeout", "2"]);
    s.line();
    s.send(json!({"command": "runTests", "sourcePaths": [a], "announceStart": true}));
    // A cancel that arrives while the app still compiles ends the run with no test at all (also correct, but not what
    // this test is about), so wait until the first test starts.
    let started = s.line();
    assert_eq!(started["type"], "started", "{started}");
    s.send(json!({"command": "cancel"}));
    assert_eq!(s.line(), json!({"type": "ack", "command": "cancel", "noop": false}));
    let lines = s.until_summary();
    let tests: Vec<&Value> = lines.iter().filter(|l| l["type"] == "test").collect();
    assert_eq!(
        tests.len(),
        1,
        "the test that was running finished, the next did not start: {lines:?}"
    );
    assert_eq!(tests[0]["name"], "Codeunit50200.Spins");
    assert!(
        tests[0]["message"]
            .as_str()
            .unwrap()
            .starts_with("Test exceeded 2s timeout"),
        "{}",
        tests[0]
    );
    let summary = lines.last().unwrap();
    assert_eq!(
        (summary["cancelled"].as_bool(), summary["total"].as_u64()),
        (Some(true), Some(1)),
        "{summary}"
    );
    // ADR-028 item 7: a cancelled run's summary has exit code 130, as in v2, whatever the tests that ran came to.
    assert_eq!(summary["exitCode"], 130, "{summary}");
    // The server still answers, and the next run is not cancelled.
    s.send(json!({"command": "cancel"}));
    assert_eq!(s.line()["noop"], true);
    s.send(json!({"command": "shutdown"}));
    assert_eq!(s.line(), json!({"status": "shutting down"}));
}

const WRITES: &str = r#"
table 50300 "Serve Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
codeunit 50300 "Serve Writes"
{
    Subtype = Test;

    [Test]
    procedure InsertsAfterCheckingTheTableIsEmpty()
    var
        R: Record "Serve Row";
    begin
        if not R.IsEmpty() then
            Error('the table was not empty: %1 row(s)', R.Count());
        R.Id := 1;
        R.Insert();
        R.Id := 2;
        R.Insert();
        Commit();
    end;
}
"#;

#[test]
fn a_test_that_writes_gives_the_same_answer_in_two_requests() {
    let d = tempfile::tempdir().unwrap();
    let a = app(d.path(), "writes", WRITES);
    let mut s = Server::start(&["--no-download", "--no-test-data"]);
    s.line();
    let mut outcomes = Vec::new();
    for _ in 0..3 {
        s.send(json!({"command": "runTests", "sourcePaths": [a]}));
        let lines = s.until_summary();
        let summary = lines.last().unwrap().clone();
        outcomes.push((
            lines[0]["status"].clone(),
            lines[0]["message"].clone(),
            summary["passed"].clone(),
            summary["cached"].clone(),
        ));
    }
    // Each request starts from the same database: the table is empty every time, though the test commits two rows.
    assert_eq!(outcomes[0].0, "pass", "{outcomes:?}");
    assert!(outcomes.iter().all(|o| o.0 == "pass" && o.2 == 1), "{outcomes:?}");
    // The later requests used the compile of the first.
    assert_eq!(
        (outcomes[0].3.clone(), outcomes[1].3.clone(), outcomes[2].3.clone()),
        (json!(false), json!(true), json!(true))
    );
}

#[test]
fn a_changed_source_is_compiled_again_and_an_unchanged_one_is_not() {
    let d = tempfile::tempdir().unwrap();
    let a = app(d.path(), "changing", MIXED);
    let mut s = Server::start(&["--no-download", "--no-test-data"]);
    s.line();
    let run = |s: &mut Server| {
        s.send(json!({"command": "runTests", "sourcePaths": [a], "test": "Codeunit50100.Fails"}));
        s.until_summary()
    };
    assert_eq!(run(&mut s).last().unwrap()["cached"], false);
    assert_eq!(run(&mut s).last().unwrap()["cached"], true);
    // The test is fixed on disk: the next request sees it.
    write(&a, "src/T.al", &MIXED.replace("Error('boom %1', 5);", ""));
    let lines = run(&mut s);
    assert_eq!(lines.last().unwrap()["cached"], false);
    assert_eq!(lines[0]["status"], "pass", "{lines:?}");
    // A request for a test the first compile did not reach compiles for both.
    s.send(json!({"command": "runTests", "sourcePaths": [a], "test": "Codeunit50100.Passes"}));
    let lines = s.until_summary();
    assert_eq!(lines.last().unwrap()["passed"], 1, "{lines:?}");
}

const TWO_UNITS: &str = r#"
codeunit 50400 "Boom Tests"
{
    Subtype = Test;

    [Test]
    procedure First()
    begin
    end;

    [Test]
    procedure Boom()
    begin
    end;

    [Test]
    procedure Third()
    begin
    end;
}

codeunit 50401 "Fine Tests"
{
    Subtype = Test;

    [Test]
    procedure Fine()
    begin
    end;
}
"#;

#[test]
fn a_panic_in_a_test_is_reported_as_an_error_and_the_server_goes_on() {
    let d = tempfile::tempdir().unwrap();
    let a = app(d.path(), "boom", TWO_UNITS);
    let mut s = Server::start_with(
        &["--no-download", "--no-test-data"],
        &[("ALRUNNER_DEBUG_PANIC_IN", "Boom")],
    );
    s.line();
    s.send(json!({"command": "runTests", "sourcePaths": [a]}));
    let lines = s.until_summary();
    let status = |name: &str| {
        lines
            .iter()
            .find(|l| l["name"] == name)
            .unwrap_or_else(|| panic!("no {name}: {lines:?}"))
            .clone()
    };
    assert_eq!(status("Codeunit50400.First")["status"], "pass");
    let boom = status("Codeunit50400.Boom");
    assert_eq!(boom["status"], "error");
    assert!(boom["message"].as_str().unwrap().contains("runtime panic"), "{boom}");
    // The rest of that codeunit is not run, with the reason; the other codeunit is.
    assert!(
        status("Codeunit50400.Third")["message"]
            .as_str()
            .unwrap()
            .contains("panicked")
    );
    assert_eq!(status("Codeunit50401.Fine")["status"], "pass");
    assert_eq!(lines.last().unwrap()["total"], 4);
    // The server answers the next request.
    s.send(json!({"command": "runTests", "sourcePaths": [a], "test": "Fine"}));
    assert_eq!(s.until_summary().last().unwrap()["passed"], 1);
}

fn rss_mb(pid: u32) -> u64 {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).unwrap();
    let kb: u64 = status
        .lines()
        .find_map(|l| l.strip_prefix("VmRSS:"))
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .parse()
        .unwrap();
    kb / 1024
}

#[test]
#[cfg(target_os = "linux")]
fn memory_stays_bounded_over_fifty_requests_that_change_the_source_every_time() {
    let d = tempfile::tempdir().unwrap();
    let a = app(d.path(), "many", MIXED);
    let mut s = Server::start(&["--no-download", "--no-test-data"]);
    s.line();
    let pid = s.child.id();
    let mut at_10 = 0;
    for i in 0..50 {
        // A changed file every time: the compile is dropped and made again 50 times.
        write(&a, "src/T.al", &format!("{MIXED}\n// request {i}\n"));
        s.send(json!({"command": "runTests", "sourcePaths": [a]}));
        let lines = s.until_summary();
        assert_eq!(lines.last().unwrap()["total"], 2, "{lines:?}");
        if i == 9 {
            at_10 = rss_mb(pid);
        }
    }
    let at_50 = rss_mb(pid);
    assert!(
        at_50 < at_10 + 60,
        "resident memory grew from {at_10} MB after 10 requests to {at_50} MB after 50"
    );
    s.send(json!({"command": "shutdown"}));
    s.line();
}

#[test]
fn a_request_sent_during_a_run_waits_for_it() {
    let d = tempfile::tempdir().unwrap();
    let a = app(d.path(), "slow2", SLOW);
    let b = app(d.path(), "mixed", MIXED);
    let mut s = Server::start(&["--no-download", "--no-test-data", "--test-timeout", "1"]);
    s.line();
    s.send(json!({"command": "runTests", "sourcePaths": [a], "test": "Codeunit50200.SpinsToo"}));
    s.send(json!({"command": "runTests", "sourcePaths": [b], "test": "Codeunit50100.Passes"}));
    let first = s.until_summary();
    assert_eq!(first.last().unwrap()["total"], 1);
    assert_eq!(first[0]["name"], "Codeunit50200.SpinsToo");
    let second = s.until_summary();
    assert_eq!(second[0]["name"], "Codeunit50100.Passes", "{second:?}");
}

// ---- execute ----

const SCRATCH: &str = r#"
codeunit 50200 "Later"
{
    trigger OnRun()
    begin
        Message('later');
    end;
}

codeunit 50150 "Scratch"
{
    trigger OnRun()
    var
        N: Integer;
    begin
        N := 40;
        Message('answer %1', N + 2);
        Message('second');
    end;
}

codeunit 50160 "Helper"
{
    procedure P()
    begin
    end;
}
"#;

/// LethAL: the `test` line carries `sleptMs`, the summed time the test's `Sleep` calls asked for, as `--output-json` does.
#[test]
fn a_test_line_has_the_slept_time_of_the_test() {
    let d = tempfile::tempdir().unwrap();
    let a = app(
        d.path(),
        "sleeps",
        "codeunit 50412 \"Sleeps\" { Subtype = Test;
            [Test] procedure Twice() begin Sleep(300); Sleep(45); end;
            [Test] procedure NoSleep() begin end; }",
    );
    let mut s = started();
    s.send(json!({"command": "runTests", "sourcePaths": [a]}));
    let lines = s.until_summary();
    let line = |n: &str| lines.iter().find(|l| l["name"] == n).cloned().unwrap();
    assert_eq!(line("Codeunit50412.Twice")["sleptMs"], 345, "{lines:?}");
    assert!(line("Codeunit50412.NoSleep").get("sleptMs").is_none(), "{lines:?}");
}

fn started() -> Server {
    let mut s = Server::start(&["--no-download", "--no-test-data"]);
    assert_eq!(s.line(), json!({"ready": true}));
    s
}

#[test]
fn execute_runs_the_on_run_of_the_lowest_codeunit_of_an_app_and_returns_its_messages() {
    let d = tempfile::tempdir().unwrap();
    let a = app(d.path(), "scratch", SCRATCH);
    let mut s = started();
    s.send(json!({"command": "execute", "sourcePaths": [a]}));
    let r = s.line();
    // One line, no `type`, as v2 answers.
    assert!(r.get("type").is_none(), "{r}");
    assert_eq!(r["exitCode"], 0, "{r}");
    let t = &r["tests"][0];
    assert_eq!(t["name"], "Codeunit50150.OnRun", "{r}");
    assert_eq!(t["status"], "pass");
    assert_eq!(t["messages"], json!(["answer 42", "second"]));
    assert_eq!(r["tests"].as_array().unwrap().len(), 1);
    // A change in the code runs again with the new code (the warm compile takes it in).
    write(&a, "src/T.al", &SCRATCH.replace("N := 40;", "N := 50;"));
    s.send(json!({"command": "execute", "sourcePaths": [a]}));
    let r = s.line();
    assert_eq!(r["tests"][0]["messages"], json!(["answer 52", "second"]), "{r}");
    s.send(json!({"command": "shutdown"}));
    s.line();
}

#[test]
fn execute_runs_inline_code_a_statement_list_or_a_whole_object() {
    let mut s = started();
    s.send(json!({"command": "execute", "code": "Message('hi');"}));
    let r = s.line();
    assert_eq!(r["exitCode"], 0, "{r}");
    assert_eq!(r["tests"][0]["name"], "AL Runner Inline Execute.OnRun", "{r}");
    assert_eq!(r["tests"][0]["messages"], json!(["hi"]));
    // A comment first, then an object: used as it is.
    s.send(json!({"command": "execute", "code": "// scratch\ncodeunit 50100 X { trigger OnRun() begin Message('from x'); Error('boom %1', 3); end; }"}));
    let r = s.line();
    assert_eq!(r["exitCode"], 1, "{r}");
    let t = &r["tests"][0];
    assert_eq!(
        (t["name"].as_str(), t["status"].as_str()),
        (Some("X.OnRun"), Some("fail")),
        "{r}"
    );
    assert_eq!(t["message"], "boom 3");
    assert_eq!(t["messages"], json!(["from x"]));
    // The next statement list works after a failure.
    s.send(json!({"command": "execute", "code": "Message(Format(6 * 7));"}));
    assert_eq!(s.line()["tests"][0]["messages"], json!(["42"]));
    s.send(json!({"command": "shutdown"}));
    s.line();
}

#[test]
fn execute_checks_its_request_and_says_what_it_does_not_do_yet() {
    let d = tempfile::tempdir().unwrap();
    let a = app(d.path(), "scratch", SCRATCH);
    let none = app(d.path(), "none", "codeunit 50100 NoRun { procedure P() begin end; }");
    let mut s = started();
    let mut ask = |v: Value| {
        s.send(v);
        s.line()
    };
    assert!(
        ask(json!({"command": "execute"}))["error"]
            .as_str()
            .unwrap()
            .contains("sourcePaths")
    );
    assert!(
        ask(json!({"command": "execute", "code": "x", "sourcePaths": [a]}))["error"]
            .as_str()
            .unwrap()
            .contains("mutually exclusive")
    );
    assert!(
        ask(json!({"command": "execute", "code": "x", "bogus": 1}))["error"]
            .as_str()
            .unwrap()
            .contains("bogus")
    );
    // ADR-050 decision 1: the affected fields earn a warning on execute.
    assert!(
        ask(json!({"command": "execute", "code": "Message('a');", "includeFailing": true}))["warnings"]
            .to_string()
            .contains("includeFailing")
    );
    let r =
        ask(json!({"command": "execute", "code": "Message('a');", "captureValues": true, "iterationTracking": true}));
    assert_eq!(r["tests"][0]["messages"], json!(["a"]), "{r}");
    assert!(r.get("warnings").is_none(), "{r}");
    // A field of runTests: a warning, not an error.
    let r = ask(json!({"command": "execute", "code": "Message('a');", "test": "X"}));
    assert!(r["warnings"][0].as_str().unwrap().contains("runTests"), "{r}");
    // An app without an OnRun: nothing ran.
    let r = ask(json!({"command": "execute", "sourcePaths": [none]}));
    assert_eq!(r["tests"], json!([]), "{r}");
    assert_eq!(r["exitCode"], 6, "{r}");
    assert!(r["warnings"][0].as_str().unwrap().contains("OnRun"), "{r}");
    s.send(json!({"command": "shutdown"}));
    s.line();
}

#[test]
fn execute_and_runtests_share_the_server() {
    let d = tempfile::tempdir().unwrap();
    let a = app(d.path(), "mixed", MIXED);
    let mut s = started();
    s.send(json!({"command": "execute", "code": "Message('x');"}));
    assert_eq!(s.line()["exitCode"], 0);
    s.send(json!({"command": "runTests", "sourcePaths": [a]}));
    let lines = s.until_summary();
    assert_eq!(lines.last().unwrap()["total"], 2, "{lines:?}");
    s.send(json!({"command": "execute", "code": "Message('y');"}));
    assert_eq!(s.line()["tests"][0]["messages"], json!(["y"]));
    s.send(json!({"command": "shutdown"}));
    s.line();
}

#[test]
fn cancel_stops_an_execute_that_does_not_end() {
    let mut s = started();
    s.send(json!({"command": "execute", "code": "Message('start'); while true do ;", "announceStart": true}));
    // A cancel during the compile gives cancelled, exit code 6 and no error: also correct, but not what this test is
    // about. Wait until the code starts to run.
    let started = s.line();
    assert_eq!(started["type"], "started", "{started}");
    s.send(json!({"command": "cancel"}));
    // The acknowledgement comes at once, and then the answer of the stopped run.
    let ack = s.line();
    assert_eq!(ack, json!({"type": "ack", "command": "cancel", "noop": false}));
    let r = s.line();
    assert_eq!(r["cancelled"], true, "{r}");
    assert_eq!(r["tests"][0]["status"], "error", "{r}");
    assert!(r["tests"][0]["message"].as_str().unwrap().contains("cancelled"), "{r}");
    // The server goes on.
    s.send(json!({"command": "execute", "code": "Message('after');"}));
    assert_eq!(s.line()["tests"][0]["messages"], json!(["after"]));
    s.send(json!({"command": "shutdown"}));
    s.line();
}

// ---- captureValues and iterationTracking ----

#[test]
fn execute_returns_the_values_and_the_loops_as_v2_does() {
    let mut s = started();
    s.send(json!({
        "command": "execute", "captureValues": true, "iterationTracking": true, "coverage": true,
        "code": "codeunit 50102 X3 { trigger OnRun() var i: Integer; total: Integer; begin total := 0; for i := 1 to 3 do begin total := total + i; Message(Format(total)); end; end; }"
    }));
    let r = s.line();
    assert_eq!(r["exitCode"], 0, "{r}");
    let t = &r["tests"][0];
    assert_eq!(t["name"], "X3.OnRun");
    let cv = t["capturedValues"].as_array().unwrap();
    let shape: Vec<(String, i64, Option<i64>, Option<i64>)> = cv
        .iter()
        .map(|v| {
            (
                v["variableName"].as_str().unwrap().to_string(),
                v["value"].as_i64().unwrap(),
                v["loop"].as_i64(),
                v["iteration"].as_i64(),
            )
        })
        .collect();
    assert_eq!(
        shape,
        vec![
            ("total".to_string(), 0, None, None),
            ("i".to_string(), 1, Some(0), Some(1)),
            ("total".to_string(), 1, Some(0), Some(1)),
            ("i".to_string(), 2, Some(0), Some(2)),
            ("total".to_string(), 3, Some(0), Some(2)),
            ("i".to_string(), 3, Some(0), Some(3)),
            ("total".to_string(), 6, Some(0), Some(3)),
        ]
    );
    // The loop: where it is, how many passes, how it ended, and the statements of each pass.
    let l = &t["loops"][0];
    assert_eq!(
        (l["id"].as_i64(), l["scope"].as_str(), l["iterationCount"].as_i64()),
        (Some(0), Some("OnRun"), Some(3))
    );
    assert_eq!(l["closedBy"], "exit");
    assert!(l["file"].as_str().unwrap().ends_with("/Inline.al"), "{l}");
    assert_eq!(l["iterations"].as_array().unwrap().len(), 3);
    assert_eq!(l["iterations"][0]["index"], 1);
    // The ids of statements are the ids of the coverage table (same scope).
    let ids: Vec<i64> = cv.iter().map(|v| v["statementId"].as_i64().unwrap()).collect();
    let covered: Vec<i64> = r["coverage"][0]["statements"]
        .as_array()
        .unwrap()
        .iter()
        .map(|st| st["id"].as_i64().unwrap())
        .collect();
    assert!(ids.iter().all(|i| covered.contains(i)), "{ids:?} not in {covered:?}");
    // The tagged messages are the response's.
    let m = r["messages"].as_array().unwrap();
    assert_eq!(m.len(), 3, "{r}");
    assert_eq!(
        (m[1]["text"].as_str(), m[1]["loop"].as_i64(), m[1]["iteration"].as_i64()),
        (Some("3"), Some(0), Some(2))
    );
    // Without the flags there is nothing of it.
    s.send(json!({"command": "execute", "code": "Message('x');"}));
    let r = s.line();
    assert!(
        r["tests"][0].get("capturedValues").is_none() && r["tests"][0].get("loops").is_none(),
        "{r}"
    );
    assert!(r.get("messages").is_none(), "{r}");
    s.send(json!({"command": "shutdown"}));
    s.line();
}

const CAPTURE_TESTS: &str = r#"
codeunit 50300 "Capture Tests"
{
    Subtype = Test;

    [Test]
    procedure Loops()
    var
        i: Integer;
        Sum: Integer;
    begin
        for i := 1 to 2 do
            Sum += i;
    end;
}
"#;

#[test]
fn run_tests_puts_the_capture_on_the_test_line() {
    let d = tempfile::tempdir().unwrap();
    let a = app(d.path(), "cap", CAPTURE_TESTS);
    let mut s = started();
    s.send(json!({"command": "runTests", "sourcePaths": [a], "captureValues": true, "iterationTracking": true}));
    let lines = s.until_summary();
    let t = lines.iter().find(|l| l["type"] == "test").unwrap();
    assert_eq!(t["status"], "pass", "{t}");
    let values: Vec<(String, i64)> = t["capturedValues"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| {
            (
                v["variableName"].as_str().unwrap().to_string(),
                v["value"].as_i64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        values,
        vec![
            ("i".to_string(), 1),
            ("Sum".to_string(), 1),
            ("i".to_string(), 2),
            ("Sum".to_string(), 3)
        ]
    );
    assert_eq!(t["loops"][0]["iterationCount"], 2);
    // Another request without the flags: the lines carry nothing of it.
    s.send(json!({"command": "runTests", "sourcePaths": [a]}));
    let lines = s.until_summary();
    let t = lines.iter().find(|l| l["type"] == "test").unwrap();
    assert!(t.get("capturedValues").is_none() && t.get("loops").is_none(), "{t}");
    s.send(json!({"command": "shutdown"}));
    s.line();
}

#[test]
fn announce_start_writes_one_started_line_before_the_first_test() {
    let d = tempfile::tempdir().unwrap();
    let a = app(d.path(), "mixed", MIXED);
    let mut s = started();
    // Without the field no `started` line is written.
    s.send(json!({"command": "runTests", "sourcePaths": [a]}));
    let lines = s.until_summary();
    assert!(lines.iter().all(|l| l["type"] != "started"), "{lines:?}");
    s.send(json!({"command": "runTests", "sourcePaths": [a], "announceStart": true}));
    let lines = s.until_summary();
    let started: Vec<usize> = (0..lines.len()).filter(|&i| lines[i]["type"] == "started").collect();
    assert_eq!(started, vec![0], "{lines:?}");
    s.send(json!({"command": "shutdown"}));
    s.line();
}

#[test]
fn a_selection_that_matches_no_test_answers_a_summary_with_exit_code_6() {
    let d = tempfile::tempdir().unwrap();
    let a = app(d.path(), "mixed", MIXED);
    let mut s = started();
    // As `al-runner test` exits 6 when the selection matches nothing, the summary has exit code 6 and no test.
    s.send(json!({"command": "runTests", "sourcePaths": [a], "test": "Codeunit50100.NoSuchTest"}));
    let lines = s.until_summary();
    let summary = lines.last().unwrap();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(summary["type"], "summary", "{summary}");
    assert_eq!(
        (
            summary["exitCode"].as_u64(),
            summary["passed"].as_u64(),
            summary["failed"].as_u64(),
            summary["errors"].as_u64(),
            summary["notRun"].as_u64(),
            summary["total"].as_u64()
        ),
        (Some(6), Some(0), Some(0), Some(0), Some(0), Some(0)),
        "{summary}"
    );
    assert_eq!(summary["protocolVersion"], 2);
    // The reason is in the answer, not only on stderr.
    assert!(
        summary["warnings"][0].as_str().unwrap().contains("matches no test"),
        "{summary}"
    );
    // An exclusion that leaves nothing gives the same answer, and the server goes on.
    s.send(json!({"command": "runTests", "sourcePaths": [a], "excludeTests": ["Codeunit50100.Passes", "Codeunit50100.Fails"]}));
    assert_eq!(s.until_summary().last().unwrap()["exitCode"], 6);
    s.send(json!({"command": "runTests", "sourcePaths": [a]}));
    assert_eq!(s.until_summary().last().unwrap()["total"], 2);
    s.send(json!({"command": "shutdown"}));
    s.line();
}

#[test]
fn execute_with_announce_start_writes_the_started_line_and_no_warning() {
    let mut s = started();
    // ADR-028 item 8: `announceStart` works on `execute` too, so it earns no warning.
    s.send(json!({"command": "execute", "code": "Message('hi');", "announceStart": true}));
    let started = s.line();
    assert_eq!(started["type"], "started", "{started}");
    let r = s.line();
    assert_eq!(r["tests"][0]["messages"], json!(["hi"]), "{r}");
    assert!(r.get("warnings").is_none(), "{r}");
    s.send(json!({"command": "shutdown"}));
    s.line();
}

// `--tdd` on the server (ADR-051 decision 1 and 9, test plan 12 and 13).

const TDD_TARGET: &str = r#"
codeunit 50110 "Serve Target"
{
    procedure Existing(): Integer
    begin
        exit(1);
    end;
}
"#;

const TDD_TESTS: &str = r#"
codeunit 50111 "Serve Tdd Tests"
{
    Subtype = Test;

    [Test]
    procedure CallsMissing()
    var
        Target: Codeunit "Serve Target";
        Result: Integer;
    begin
        Result := Target.DoubleIt(2);
        if Result <> 4 then
            Error('expected 4, got %1', Result);
    end;

    [Test]
    procedure Plain()
    begin
    end;
}
"#;

/// An app of two files: the codeunit under test and the tests that call a procedure it does not have yet.
fn tdd_app(root: &Path) -> PathBuf {
    let dir = root.join("tdd");
    write(
        &dir,
        "app.json",
        r#"{"id":"11111111-0000-0000-0000-0000000000aa","name":"tdd","publisher":"Test","version":"1.0.0.0","idRanges":[{"from":50110,"to":50149}],"dependencies":[]}"#,
    );
    write(&dir, "src/Target.al", TDD_TARGET);
    write(&dir, "src/Tests.al", TDD_TESTS);
    dir
}

fn line_of<'a>(lines: &'a [Value], test: &str) -> &'a Value {
    lines
        .iter()
        .find(|l| l["type"] == "test" && l["name"].as_str().is_some_and(|n| n.ends_with(&format!(".{test}"))))
        .unwrap_or_else(|| panic!("no line for {test}: {lines:?}"))
}

const DOUBLE_IT: &str = r#"Serve Target: procedure "DoubleIt"(Arg1: Integer): Integer"#;

#[test]
fn tdd_true_on_run_tests_runs_against_stubs_and_flags_the_line_and_the_summary() {
    let d = tempfile::tempdir().unwrap();
    let a = tdd_app(d.path());
    let mut s = Server::start(&["--no-download", "--no-test-data"]);
    s.line();
    s.send(json!({"command": "runTests", "sourcePaths": [a], "tdd": true}));
    let lines = s.until_summary();
    let t = line_of(&lines, "CallsMissing");
    assert_eq!(t["status"], "fail", "{lines:?}");
    assert_eq!(t["stubbed"], true, "{t}");
    assert_eq!(t["generatedStubs"], json!([DOUBLE_IT]), "{t}");
    let p = line_of(&lines, "Plain");
    assert_eq!(p["status"], "pass");
    assert!(p.get("stubbed").is_none() && p.get("generatedStubs").is_none(), "{p}");
    let summary = lines.last().unwrap();
    assert_eq!(
        summary["stubbed"],
        json!({"passed": 0, "failed": 1, "error": 0}),
        "{summary}"
    );
    assert_eq!(summary["generatedStubs"], json!([DOUBLE_IT]), "{summary}");
    assert_eq!(summary["exitCode"], 1, "{summary}");
    // Without `tdd` the same sources do not compile as before: `tdd` is part of the kept compile's key.
    s.send(json!({"command": "runTests", "sourcePaths": [a]}));
    let lines = s.until_summary();
    let summary = lines.last().unwrap();
    assert_eq!(summary["cached"], false, "{summary}");
    let t = line_of(&lines, "CallsMissing");
    assert_eq!(t["notRun"], true, "{t}");
    assert!(t.get("stubbed").is_none(), "{t}");
    assert!(
        summary.get("stubbed").is_none() && summary.get("generatedStubs").is_none(),
        "{summary}"
    );
    s.send(json!({"command": "shutdown"}));
    s.line();
}

#[test]
fn a_server_started_with_tdd_stubs_by_default_and_tdd_false_turns_it_off_for_one_request() {
    let d = tempfile::tempdir().unwrap();
    let a = tdd_app(d.path());
    let mut s = Server::start(&["--no-download", "--no-test-data", "--tdd"]);
    s.line();
    s.send(json!({"command": "runTests", "sourcePaths": [a]}));
    let lines = s.until_summary();
    assert_eq!(line_of(&lines, "CallsMissing")["stubbed"], true, "{lines:?}");
    s.send(json!({"command": "runTests", "sourcePaths": [a], "tdd": false}));
    let lines = s.until_summary();
    assert_eq!(line_of(&lines, "CallsMissing")["notRun"], true, "{lines:?}");
    assert!(lines.last().unwrap().get("generatedStubs").is_none(), "{lines:?}");
    s.send(json!({"command": "runTests", "sourcePaths": [a], "tdd": "yes"}));
    assert!(s.line()["error"].as_str().unwrap().contains("`tdd`"));
    s.send(json!({"command": "shutdown"}));
    s.line();
}

#[test]
fn execute_ignores_tdd_with_a_warning() {
    let mut s = started();
    s.send(json!({"command": "execute", "code": "Message('hi');", "tdd": true}));
    let r = s.line();
    assert_eq!(r["tests"][0]["messages"], json!(["hi"]), "{r}");
    assert!(
        r["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w.as_str().unwrap().contains("`tdd`")),
        "{r}"
    );
    s.send(json!({"command": "shutdown"}));
    s.line();
}

#[test]
fn a_kept_tdd_compile_follows_the_sources_and_the_real_procedure_ends_the_stub() {
    let d = tempfile::tempdir().unwrap();
    let a = tdd_app(d.path());
    let mut s = Server::start(&["--no-download", "--no-test-data"]);
    s.line();
    let run = |s: &mut Server| {
        s.send(json!({"command": "runTests", "sourcePaths": [a], "tdd": true}));
        s.until_summary()
    };
    run(&mut s);
    // A change of code only in the tests: the new stub is in the list. (Without dependency apps the server compiles
    // everything again; `al-testrun/tests/incremental.rs` checks the incremental compile of stubs.)
    write(
        &a,
        "src/Tests.al",
        &TDD_TESTS.replace(
            "    procedure Plain()\n    begin\n",
            "    procedure Plain()\n    var\n        Target: Codeunit \"Serve Target\";\n    begin\n        if Target.IsReady() then;\n",
        ),
    );
    let lines = run(&mut s);
    let summary = lines.last().unwrap();
    let ready = r#"Serve Target: procedure "IsReady"(): Boolean"#;
    assert_eq!(summary["generatedStubs"], json!([DOUBLE_IT, ready]), "{summary}");
    assert_eq!(line_of(&lines, "Plain")["generatedStubs"], json!([ready]), "{lines:?}");
    // The real procedures are added: the next request is no longer stubbed.
    write(
        &a,
        "src/Target.al",
        &TDD_TARGET.replace(
            "    procedure Existing",
            "    procedure DoubleIt(N: Integer): Integer\n    begin\n        exit(N * 2);\n    end;\n\n    procedure IsReady(): Boolean\n    begin\n    end;\n\n    procedure Existing",
        ),
    );
    let lines = run(&mut s);
    let t = line_of(&lines, "CallsMissing");
    assert_eq!(t["status"], "pass", "{lines:?}");
    assert!(t.get("stubbed").is_none(), "{t}");
    let summary = lines.last().unwrap();
    assert_eq!(summary["generatedStubs"], json!([]), "{summary}");
    assert_eq!(
        summary["stubbed"],
        json!({"passed": 0, "failed": 0, "error": 0}),
        "{summary}"
    );
    s.send(json!({"command": "shutdown"}));
    s.line();
}

const TDD_TABLE: &str = r#"table 50150 "Serve Tdd Item"
{
    fields
    {
        field(1; "No."; Code[20]) { }
    }
}
"#;

const TDD_FIELD_TESTS: &str = r#"codeunit 50151 "Serve Tdd Field Tests"
{
    Subtype = Test;

    [Test]
    procedure WritesAField()
    var
        Item: Record "Serve Tdd Item";
    begin
        Item.Weight := 7;
        if Item.Weight <> 7 then
            Error('Weight is %1', Item.Weight);
    end;
}
"#;

#[test]
fn a_stub_field_is_declared_on_each_request_and_the_real_field_ends_it() {
    let d = tempfile::tempdir().unwrap();
    let a = d.path().join("tddfield");
    write(
        &a,
        "app.json",
        r#"{"id":"11111111-0000-0000-0000-0000000000ab","name":"tddfield","publisher":"Test","version":"1.0.0.0","idRanges":[{"from":50150,"to":50159}],"dependencies":[]}"#,
    );
    write(&a, "src/Item.al", TDD_TABLE);
    write(&a, "src/Tests.al", TDD_FIELD_TESTS);
    let mut s = Server::start(&["--no-download", "--no-test-data"]);
    s.line();
    let run = |s: &mut Server| {
        s.send(json!({"command": "runTests", "sourcePaths": [a], "tdd": true}));
        s.until_summary()
    };
    let weight = r#"Serve Tdd Item: field "Weight": Integer"#;
    for _ in 0..2 {
        let lines = run(&mut s);
        let t = line_of(&lines, "WritesAField");
        assert_eq!(t["status"], "pass", "{lines:?}");
        assert_eq!(t["generatedStubs"], json!([weight]), "{t}");
    }
    // The real field is added: no stub, and no clash with the stub of the request before.
    write(
        &a,
        "src/Item.al",
        &TDD_TABLE.replace("{ }\n    }", "{ }\n        field(2; Weight; Integer) { }\n    }"),
    );
    let lines = run(&mut s);
    let t = line_of(&lines, "WritesAField");
    assert_eq!(t["status"], "pass", "{lines:?}");
    assert!(t.get("stubbed").is_none(), "{t}");
    assert_eq!(lines.last().unwrap()["generatedStubs"], json!([]), "{lines:?}");
    s.send(json!({"command": "shutdown"}));
    s.line();
}

/// Issue 5604: a changed resource file is seen by the next request. Issue 5635: resources are run-time data, so the
/// request reuses the earlier compile (`cached`).
#[test]
fn a_changed_resource_file_is_seen_by_the_next_request() {
    let d = tempfile::tempdir().unwrap();
    let a = d.path().join("res");
    write(
        &a,
        "app.json",
        r#"{"id":"11111111-0000-0000-0000-000000005604","name":"Res","publisher":"Test","version":"1.0.0.0","dependencies":[],"resourceFolders":["ProbeResources"]}"#,
    );
    write(&a, "ProbeResources/active.txt", "M0001");
    write(
        &a,
        "src/T.al",
        "codeunit 50650 \"Res Tests\" { Subtype = Test; [Test] procedure ResourceIs() begin Error('res=%1', NavApp.GetResourceAsText('active.txt')); end; }",
    );
    let mut s = Server::start(&["--no-download", "--no-test-data"]);
    s.line();
    let run = |s: &mut Server| {
        s.send(json!({"command": "runTests", "sourcePaths": [a], "test": "Codeunit50650.ResourceIs"}));
        s.until_summary()
    };
    let lines = run(&mut s);
    assert!(lines[0]["message"].as_str().unwrap().contains("res=M0001"), "{lines:?}");
    write(&a, "ProbeResources/active.txt", "M0002");
    let lines = run(&mut s);
    assert!(lines[0]["message"].as_str().unwrap().contains("res=M0002"), "{lines:?}");
    assert_eq!(lines.last().unwrap()["cached"], true, "{lines:?}");
}

/// Issue 5635: the install code of a request that reuses the compile reads the resource files as they are now.
#[test]
fn install_code_reads_the_changed_resource_file_of_a_reused_compile() {
    let d = tempfile::tempdir().unwrap();
    let a = d.path().join("res");
    write(
        &a,
        "app.json",
        r#"{"id":"11111111-0000-0000-0000-000000005635","name":"ResInstall","publisher":"Test","version":"1.0.0.0","dependencies":[],"resourceFolders":["ProbeResources"]}"#,
    );
    write(&a, "ProbeResources/active.txt", "M0001");
    write(
        &a,
        "src/T.al",
        "table 50651 \"Res Row\" { fields { field(1; PK; Code[10]) { } field(2; Val; Text[30]) { } } keys { key(PK; PK) { } } }
codeunit 50652 \"Res Install\" { Subtype = Install; trigger OnInstallAppPerCompany() var R: Record \"Res Row\"; begin R.PK := 'A'; R.Val := NavApp.GetResourceAsText('active.txt'); R.Insert(); end; }
codeunit 50653 \"Res Install Tests\" { Subtype = Test; [Test] procedure InstalledIs() var R: Record \"Res Row\"; begin R.Get('A'); Error('inst=%1', R.Val); end; }",
    );
    let mut s = Server::start(&["--no-download", "--no-test-data"]);
    s.line();
    let run = |s: &mut Server| {
        s.send(json!({"command": "runTests", "sourcePaths": [a], "test": "Codeunit50653.InstalledIs"}));
        s.until_summary()
    };
    let lines = run(&mut s);
    assert!(
        lines[0]["message"].as_str().unwrap().contains("inst=M0001"),
        "{lines:?}"
    );
    write(&a, "ProbeResources/active.txt", "M0002");
    let lines = run(&mut s);
    assert!(
        lines[0]["message"].as_str().unwrap().contains("inst=M0002"),
        "{lines:?}"
    );
    assert_eq!(lines.last().unwrap()["cached"], true, "{lines:?}");
}

/// An app whose code reads its active mutant from a resource file (issue 5635), with a test that writes a row.
fn mutant_app(d: &Path) -> std::path::PathBuf {
    let a = d.join("mut");
    write(
        &a,
        "app.json",
        r#"{"id":"11111111-0000-0000-0000-000000056351","name":"Mut","publisher":"Test","version":"1.0.0.0","dependencies":[],"resourceFolders":["ProbeResources"]}"#,
    );
    write(&a, "ProbeResources/active.txt", "M0000");
    write(&a, "ProbeResources/other.txt", "unchanged");
    write(
        &a,
        "src/T.al",
        "table 50660 \"Mut Row\" { fields { field(1; PK; Code[10]) { } } keys { key(PK; PK) { } } }
codeunit 50661 \"Mut Tests\" { Subtype = Test;
[Test] procedure Active() begin Error('active=%1 other=%2', NavApp.GetResourceAsText('active.txt'), NavApp.GetResourceAsText('other.txt')); end;
[Test] procedure WritesARow() var R: Record \"Mut Row\"; begin if not R.IsEmpty() then Error('a row of another entry'); R.PK := 'A'; R.Insert(); end;
[Test] procedure Passes() begin end; }",
    );
    a
}

/// Issue 5635: one runTests request with several entries, each with its own resource override and tests. The test lines
/// carry the entry, each entry has its own summary, each starts from the same database, and one compile serves all.
#[test]
fn entries_run_their_own_tests_with_their_own_resources_on_one_compile() {
    let d = tempfile::tempdir().unwrap();
    let a = mutant_app(d.path());
    let mut s = Server::start(&["--no-download", "--no-test-data"]);
    s.line();
    s.send(json!({"command": "runTests", "sourcePaths": [a], "entries": [
        {"id": "m1", "resources": {"active.txt": "M0001"}, "test": "Codeunit50661.Active"},
        {"id": "m2", "resources": {"active.txt": "M0002"}, "test": "Codeunit50661.", "excludeTests": ["Codeunit50661.Passes"]},
        {"id": "plain", "test": "Codeunit50661.WritesARow"},
    ]}));
    let lines = s.until_summary();
    let tests: Vec<(String, String, String, String)> = lines
        .iter()
        .filter(|l| l["type"] == "test")
        .map(|l| {
            (
                l["entry"].as_str().unwrap_or("?").to_string(),
                l["name"].as_str().unwrap().to_string(),
                l["status"].as_str().unwrap().to_string(),
                l["message"].as_str().unwrap_or("").to_string(),
            )
        })
        .collect();
    let t = |entry: &str, name: &str| {
        tests
            .iter()
            .find(|x| x.0 == entry && x.1 == name)
            .unwrap_or_else(|| panic!("{entry} {name}: {lines:?}"))
            .clone()
    };
    assert!(
        t("m1", "Codeunit50661.Active")
            .3
            .contains("active=M0001 other=unchanged"),
        "{lines:?}"
    );
    assert!(
        t("m2", "Codeunit50661.Active")
            .3
            .contains("active=M0002 other=unchanged"),
        "{lines:?}"
    );
    assert_eq!(t("m2", "Codeunit50661.WritesARow").2, "pass", "{lines:?}");
    assert_eq!(
        t("plain", "Codeunit50661.WritesARow").2,
        "pass",
        "a new database for each entry: {lines:?}"
    );
    assert_eq!(tests.len(), 4, "only the selected tests of each entry: {lines:?}");
    let entry_summaries: Vec<&Value> = lines.iter().filter(|l| l["type"] == "entrySummary").collect();
    assert_eq!(entry_summaries.len(), 3, "{lines:?}");
    assert_eq!(entry_summaries[1]["entry"], "m2");
    assert_eq!(entry_summaries[1]["total"], 2);
    assert_eq!(entry_summaries[1]["failed"], 1);
    assert_eq!(entry_summaries[1]["passed"], 1);
    let summary = lines.last().unwrap();
    assert_eq!(summary["type"], "summary");
    assert_eq!(summary["total"], 4, "{summary}");
    assert_eq!(summary["passed"], 2, "{summary}");
    assert_eq!(summary["failed"], 2, "{summary}");
    // The resource files on disk are as they were: a second request sees them, on the same compile.
    s.send(json!({"command": "runTests", "sourcePaths": [a], "test": "Codeunit50661.Active"}));
    let lines = s.until_summary();
    assert!(
        lines[0]["message"].as_str().unwrap().contains("active=M0000"),
        "{lines:?}"
    );
    assert_eq!(lines.last().unwrap()["cached"], true, "{lines:?}");
}

/// Issue 5635: what an entry may not say is an error of the request.
#[test]
fn entries_that_cannot_run_are_an_error_of_the_request() {
    let d = tempfile::tempdir().unwrap();
    let a = mutant_app(d.path());
    let mut s = Server::start(&["--no-download", "--no-test-data"]);
    s.line();
    for (request, says) in [
        (json!({"entries": [{"resources": {"missing.txt": "x"}}]}), "missing.txt"),
        (json!({"entries": [{"tests": "x"}]}), "unknown field `tests`"),
        (json!({"entries": [{"resources": {"active.txt": 1}}]}), "resources"),
        (json!({"entries": [], "test": "Codeunit50661."}), "`test`"),
        (json!({"entries": [{}], "coverage": true}), "coverage"),
        (json!({"entries": [{}], "affectedOnly": true}), "affectedOnly"),
        (json!({"entries": "x"}), "entries"),
    ] {
        let mut r = request.as_object().unwrap().clone();
        r.insert("command".into(), json!("runTests"));
        r.insert("sourcePaths".into(), json!([a]));
        s.send(Value::Object(r));
        let lines = s.until_summary();
        let e = lines.last().unwrap()["error"].as_str().unwrap_or_default().to_string();
        assert!(e.contains(says), "{request}: {lines:?}");
    }
}

/// Issue 5702: `test` is one text, as in v2's protocol (`string? Test`). A list, in the request or in an entry, is an error
/// of the request, as a non-list `excludeTests` is; it was ignored and every test ran.
#[test]
fn a_test_field_that_is_not_a_text_is_an_error_of_the_request() {
    let d = tempfile::tempdir().unwrap();
    let a = mutant_app(d.path());
    let mut s = Server::start(&["--no-download", "--no-test-data"]);
    s.line();
    for request in [
        json!({"test": ["Codeunit50661."]}),
        json!({"test": 50661}),
        json!({"entries": [{"id": "A", "test": ["Codeunit50661.", "Codeunit50662."]}]}),
    ] {
        let mut r = request.as_object().unwrap().clone();
        r.insert("command".into(), json!("runTests"));
        r.insert("sourcePaths".into(), json!([a]));
        s.send(Value::Object(r));
        let lines = s.until_summary();
        assert_eq!(lines.len(), 1, "{request}: no test runs: {lines:?}");
        let e = lines[0]["error"].as_str().unwrap_or_default();
        assert!(e.contains("`test` is a text"), "{request}: {lines:?}");
    }
}

/// Issue 5703: two entries with the same `id` cannot be told apart in the answer: an error of the request.
#[test]
fn entries_with_the_same_id_are_an_error_of_the_request() {
    let d = tempfile::tempdir().unwrap();
    let a = mutant_app(d.path());
    let mut s = Server::start(&["--no-download", "--no-test-data"]);
    s.line();
    for entries in [
        json!([{"id": "D", "resources": {"active.txt": "M1"}, "test": "Codeunit50661."},
               {"id": "D", "resources": {"active.txt": "M2"}, "test": "Codeunit50661."}]),
        // An explicit id equal to the default id (the index) of another entry.
        json!([{"test": "Codeunit50661."}, {"id": 0, "test": "Codeunit50661."}]),
        // A text and a number that read alike: a client that keys its answers by text would merge them.
        json!([{"id": "1", "test": "Codeunit50661."}, {"id": 1, "test": "Codeunit50661."}]),
    ] {
        s.send(json!({"command": "runTests", "sourcePaths": [a], "testIsolation": "test", "entries": entries}));
        let lines = s.until_summary();
        assert_eq!(lines.len(), 1, "{entries}: no test runs: {lines:?}");
        let e = lines[0]["error"].as_str().unwrap_or_default();
        assert!(e.contains("more than one entry has the `id`"), "{entries}: {lines:?}");
    }
    // Different ids run.
    s.send(json!({"command": "runTests", "sourcePaths": [a], "entries": [
        {"id": "D", "test": "Codeunit50661.Passes"}, {"id": "E", "test": "Codeunit50661.Passes"}]}));
    assert_eq!(s.until_summary().last().unwrap()["total"], 2);
}

/// Issue 5704: an entry whose selection matches no test gets the signal a whole request gets (exit code 6 and the reason
/// in `warnings`) on its `entrySummary`, and the request's exit code is 6 (the rank of section 5: 2, 6, 1, 8, 7).
#[test]
fn an_entry_whose_selection_matches_no_test_has_exit_code_6_and_so_has_the_request() {
    let d = tempfile::tempdir().unwrap();
    let a = mutant_app(d.path());
    let mut s = Server::start(&["--no-download", "--no-test-data"]);
    s.line();
    s.send(json!({"command": "runTests", "sourcePaths": [a], "testIsolation": "test", "entries": [
        {"id": "ok", "test": "Codeunit50661.Passes"},
        {"id": "none", "test": "Codeunit59999."},
    ]}));
    let lines = s.until_summary();
    let entry = |id: &str| {
        lines
            .iter()
            .find(|l| l["type"] == "entrySummary" && l["entry"] == id)
            .unwrap_or_else(|| panic!("{id}: {lines:?}"))
            .clone()
    };
    let ok = entry("ok");
    assert_eq!((ok["total"].as_u64(), ok["exitCode"].as_u64()), (Some(1), Some(0)), "{ok}");
    assert!(ok.get("warnings").is_none(), "{ok}");
    let none = entry("none");
    assert_eq!((none["total"].as_u64(), none["exitCode"].as_u64()), (Some(0), Some(6)), "{none}");
    assert!(
        none["warnings"][0].as_str().unwrap_or_default().contains("matches no test"),
        "{none}"
    );
    let summary = lines.last().unwrap();
    assert_eq!(summary["type"], "summary", "{summary}");
    assert_eq!((summary["exitCode"].as_u64(), summary["passed"].as_u64()), (Some(6), Some(1)), "{summary}");
    let w = summary["warnings"].to_string();
    assert!(w.contains("`none`") && w.contains("matches no test"), "{summary}");
}

/// Issue 5635: with no dependency app the install code runs in each test codeunit's session, so it reads the entry's
/// resource texts (an install baseline that ran once is `al-testrun`'s `entries` test).
#[test]
fn the_install_code_of_an_app_without_dependencies_reads_the_resources_of_the_entry() {
    let d = tempfile::tempdir().unwrap();
    let a = d.path().join("res");
    write(
        &a,
        "app.json",
        r#"{"id":"11111111-0000-0000-0000-000000056352","name":"ResEntries","publisher":"Test","version":"1.0.0.0","dependencies":[],"resourceFolders":["ProbeResources"]}"#,
    );
    write(&a, "ProbeResources/active.txt", "M0001");
    write(
        &a,
        "src/T.al",
        "table 50670 \"Res Row\" { fields { field(1; PK; Code[10]) { } field(2; Val; Text[30]) { } } keys { key(PK; PK) { } } }
codeunit 50671 \"Res Install\" { Subtype = Install; trigger OnInstallAppPerCompany() var R: Record \"Res Row\"; begin R.PK := 'A'; R.Val := NavApp.GetResourceAsText('active.txt'); R.Insert(); end; }
codeunit 50672 \"Res Install Tests\" { Subtype = Test; [Test] procedure InstalledIs() var R: Record \"Res Row\"; begin R.Get('A'); Error('inst=%1', R.Val); end; }",
    );
    let mut s = Server::start(&["--no-download", "--no-test-data"]);
    s.line();
    s.send(json!({"command": "runTests", "sourcePaths": [a], "entries": [
        {"id": "own", "resources": {"active.txt": "M0002"}},
        {"id": "plain"},
    ]}));
    let lines = s.until_summary();
    let test = |entry: &str| {
        lines
            .iter()
            .find(|l| l["type"] == "test" && l["entry"] == entry)
            .unwrap_or_else(|| panic!("{entry}: {lines:?}"))
            .clone()
    };
    assert!(
        test("own")["message"].as_str().unwrap().contains("inst=M0002"),
        "{lines:?}"
    );
    assert!(
        test("plain")["message"].as_str().unwrap().contains("inst=M0001"),
        "{lines:?}"
    );
}

/// Issues 5607 and 5609 with RequiredTestIsolation on the test codeunit: `testIsolation: "required"` runs each test
/// codeunit under its own RequiredTestIsolation, as BC's TestRunnerHub does on BC 28 and later (corpus
/// codeunit/lt-qu-commit-isolation/QuCommitIsolation.al:10-16, 25-39).
const REQUIRED: &str = r#"
table 50680 "Req Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
codeunit 50680 "Req Function"
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
codeunit 50681 "Req Disabled A"
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
codeunit 50682 "Req Disabled B"
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

#[test]
fn test_isolation_required_runs_each_codeunit_under_its_required_test_isolation() {
    let d = tempfile::tempdir().unwrap();
    let a = app(d.path(), "required", REQUIRED);
    let mut s = Server::start(&["--no-download", "--no-test-data"]);
    s.line();
    s.send(json!({"command": "runTests", "sourcePaths": [a], "testIsolation": "required"}));
    let lines = s.until_summary();
    let summary = lines.last().unwrap();
    assert_eq!(
        (summary["passed"].as_u64(), summary["exitCode"].as_u64()),
        (Some(4), Some(0)),
        "{lines:?}"
    );
    // The default request is unchanged: Codeunit isolation for every codeunit.
    s.send(json!({"command": "runTests", "sourcePaths": [a]}));
    let lines = s.until_summary();
    assert_eq!(lines.last().unwrap()["failed"].as_u64(), Some(2), "{lines:?}");
    // A value it does not know names the values it does.
    s.send(json!({"command": "runTests", "sourcePaths": [a], "testIsolation": "nope"}));
    let l = s.line();
    assert!(l["error"].as_str().is_some_and(|e| e.contains("required")), "{l}");
}
