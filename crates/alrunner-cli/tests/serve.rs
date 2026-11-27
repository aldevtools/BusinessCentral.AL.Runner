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
        Server::start_with_stderr(extra, env, Stdio::null())
    }

    fn start_with_stderr(extra: &[&str], env: &[(&str, &str)], stderr: Stdio) -> Server {
        let mut child = Command::new(env!("CARGO_BIN_EXE_al-runner"))
            .arg("serve")
            .args(extra)
            .env_remove("ALRUNNER_SEED")
            .env_remove("AL_RUNNER_SEED")
            .envs(env.iter().copied())
            .env("AL_CHECK_THREADS", "2")
            .env("AL_LOWER_THREADS", "2")
            .env("AL_TEST_THREADS", "2")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(stderr)
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
        // A codeunit id is given as text (docs/manual/tool-protocol.md); a number is not taken in silence.
        (
            json!({"command": "runTests", "sourcePaths": [a], "testRunner": 130450}),
            "`testRunner`",
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

/// The compile takes the runner codeunits as roots and decides from the runner whether the install baseline runs, so
/// a request with another `testRunner` does not get the compile an earlier request kept (al-runner issue 7610).
#[test]
fn another_test_runner_is_compiled_again() {
    let d = tempfile::tempdir().unwrap();
    let a = app(d.path(), "runners", MIXED);
    let mut s = Server::start(&["--no-download", "--no-test-data"]);
    s.line();
    let run = |s: &mut Server, runner: Option<&str>| {
        let mut request = json!({"command": "runTests", "sourcePaths": [a]});
        if let Some(r) = runner {
            request["testRunner"] = json!(r);
        }
        s.send(request);
        s.until_summary()
    };
    let lines = run(&mut s, Some("builtin"));
    assert_eq!(lines.last().unwrap()["cached"], false, "{lines:?}");
    assert_eq!(run(&mut s, Some("builtin")).last().unwrap()["cached"], true);
    let lines = run(&mut s, None);
    assert_eq!(
        lines.last().unwrap()["cached"],
        false,
        "the default runner after `builtin`: {lines:?}"
    );
    assert_eq!(run(&mut s, None).last().unwrap()["cached"], true);
}

/// The reporter's repro of al-runner issue 7978, with a third codeunit.
const CACHE_PROBE: &str = r#"
codeunit 50120 "Cache Probe A"
{
    Subtype = Test;

    [Test]
    procedure PassesA()
    begin
    end;
}

codeunit 50121 "Cache Probe B"
{
    Subtype = Test;

    [Test]
    procedure PassesB()
    begin
    end;
}

codeunit 50122 "Cache Probe C"
{
    Subtype = Test;

    [Test]
    procedure PassesC()
    begin
    end;
}
"#;

/// The compile depends on the sources, not on the `test` filter: a request that selects another codeunit of the same
/// sources reuses the compile, and runs only what it selects (al-runner issue 7978).
#[test]
fn another_test_filter_on_the_same_sources_reuses_the_compile() {
    let d = tempfile::tempdir().unwrap();
    let a = app(d.path(), "cacheprobe", CACHE_PROBE);
    let mut s = Server::start(&["--no-download", "--no-test-data"]);
    s.line();
    let mut run = |test: &str, coverage: bool| {
        let mut request = json!({"command": "runTests", "sourcePaths": [a], "testIsolation": "test", "test": test});
        if coverage {
            request["coverage"] = json!(true);
            request["perTestCoverage"] = json!(true);
        }
        s.send(request);
        let lines = s.until_summary();
        let ran: Vec<String> = lines
            .iter()
            .filter(|l| l["type"] == "test" && l["status"] == "pass")
            .map(|l| l["name"].as_str().unwrap_or_default().to_string())
            .collect();
        let summary = lines.last().unwrap().clone();
        (summary["cached"].clone(), summary["total"].clone(), ran)
    };
    let one = |name: &str| vec![name.to_string()];
    assert_eq!(
        run("Codeunit50120.", false),
        (json!(false), json!(1), one("Codeunit50120.PassesA")),
        "the first request compiles"
    );
    assert_eq!(
        run("Codeunit50121.", false),
        (json!(true), json!(1), one("Codeunit50121.PassesB"))
    );
    assert_eq!(
        run("Codeunit50121.", false),
        (json!(true), json!(1), one("Codeunit50121.PassesB"))
    );
    assert_eq!(
        run("Codeunit50122.", true),
        (json!(true), json!(1), one("Codeunit50122.PassesC"))
    );
    assert_eq!(
        run("Codeunit50120.", true),
        (json!(true), json!(1), one("Codeunit50120.PassesA"))
    );
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

// Linux only: reads /proc, for the memory test below.
#[cfg(target_os = "linux")]
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

/// Issue 8059: `excludeTestsExact` of a request and of an entry leaves out the test whose method name or qualified name
/// is exactly the text, in any case; `excludeTests` with the same text matches by substring.
#[test]
fn exclude_tests_exact_leaves_out_only_the_test_named() {
    let d = tempfile::tempdir().unwrap();
    let a = app(d.path(), "mixed", MIXED);
    let mut s = Server::start(&["--no-download", "--no-test-data"]);
    s.line();
    let names = |s: &mut Server, req: Value| -> Vec<String> {
        s.send(req);
        let mut n: Vec<String> = s
            .until_summary()
            .iter()
            .filter(|l| l["type"] == "test")
            .map(|l| l["name"].as_str().unwrap().to_string())
            .collect();
        n.sort();
        n
    };
    let all = ["Codeunit50100.Fails", "Codeunit50100.Passes"];
    let req = |extra: Value| {
        let mut r = json!({"command": "runTests", "sourcePaths": [a]});
        r.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        r
    };
    assert_eq!(
        names(&mut s, req(json!({"excludeTestsExact": ["passes"]}))),
        ["Codeunit50100.Fails"]
    );
    assert_eq!(
        names(&mut s, req(json!({"excludeTestsExact": ["Codeunit50100.FAILS"]}))),
        ["Codeunit50100.Passes"]
    );
    assert_eq!(names(&mut s, req(json!({"excludeTestsExact": ["Pass"]}))), all);
    assert_eq!(
        names(&mut s, req(json!({"excludeTests": ["Pass"]}))),
        ["Codeunit50100.Fails"]
    );
    // An entry has its own `excludeTestsExact`.
    s.send(req(json!({"entries": [
        {"id": "e1", "excludeTestsExact": ["Passes"]},
        {"id": "e2", "excludeTestsExact": ["Fails"]},
    ]})));
    let lines = s.until_summary();
    let of = |entry: &str| -> Vec<String> {
        lines
            .iter()
            .filter(|l| l["type"] == "test" && l["entry"] == entry)
            .map(|l| l["name"].as_str().unwrap().to_string())
            .collect()
    };
    assert_eq!(
        (of("e1"), of("e2")),
        (vec![all[0].to_string()], vec![all[1].to_string()]),
        "{lines:?}"
    );
    // Not a list, and not with `entries` or `affectedOnly`, as `excludeTests`.
    for bad in [
        json!({"excludeTestsExact": "Passes"}),
        json!({"excludeTestsExact": ["Passes"], "entries": [{"id": "x"}]}),
        json!({"excludeTestsExact": ["Passes"], "affectedOnly": true}),
    ] {
        s.send(req(bad.clone()));
        assert!(s.line().get("error").is_some(), "{bad}");
    }
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

/// The answer to a `runTests` of [`tdd_app`] without `tdd`: the call of `DoubleIt` is AL0132 (al-runner issue 5917), the
/// run stops with exit 3, no test line, and no stubs.
fn assert_no_tdd_al0132(lines: &[Value]) {
    assert!(lines.iter().all(|l| l["type"] != "test"), "{lines:?}");
    let summary = lines.last().unwrap();
    assert_eq!(summary["exitCode"], 3, "{summary}");
    let errors = summary["compilationErrors"][0]["errors"]
        .as_array()
        .unwrap_or_else(|| panic!("{summary}"));
    assert_eq!(errors.len(), 1, "{summary}");
    assert!(
        errors[0].as_str().unwrap().ends_with(
            r#"Tests.al(12,26): error AL0132: 'Codeunit "Serve Target"' does not contain a definition for 'DoubleIt'"#
        ),
        "{summary}"
    );
    assert!(
        summary.get("stubbed").is_none() && summary.get("generatedStubs").is_none(),
        "{summary}"
    );
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
    // Without `tdd` the same sources do not compile: `tdd` is part of the kept compile's key, and the call of the
    // missing procedure is AL0132 as alc gives it (al-runner issue 5917), so no test runs (exit 3).
    s.send(json!({"command": "runTests", "sourcePaths": [a]}));
    let lines = s.until_summary();
    let summary = lines.last().unwrap();
    assert_eq!(summary["cached"], false, "{summary}");
    assert_no_tdd_al0132(&lines);
    s.send(json!({"command": "shutdown"}));
    s.line();
}

/// The kept compile holds every test codeunit (ADR-028 amendment of 2026-11-13), so with a `test` filter the summary's
/// `generatedStubs` still lists the stubs of the test codeunits the filter leaves out.
#[test]
fn with_tdd_and_a_test_filter_the_summary_lists_the_stubs_of_unselected_test_codeunits() {
    let d = tempfile::tempdir().unwrap();
    let a = tdd_app(d.path());
    write(
        &a,
        "src/Other.al",
        "codeunit 50112 \"Serve Other Tests\"\n{\n    Subtype = Test;\n\n    [Test]\n    procedure Alone()\n    begin\n    end;\n}\n",
    );
    let mut s = Server::start(&["--no-download", "--no-test-data"]);
    s.line();
    s.send(json!({"command": "runTests", "sourcePaths": [a], "tdd": true, "test": "Codeunit50112."}));
    let lines = s.until_summary();
    let tests: Vec<&Value> = lines.iter().filter(|l| l["type"] == "test").collect();
    assert_eq!(tests.len(), 1, "{lines:?}");
    assert_eq!(line_of(&lines, "Alone")["status"], "pass", "{lines:?}");
    let summary = lines.last().unwrap();
    assert_eq!(summary["generatedStubs"], json!([DOUBLE_IT]), "{summary}");
    assert_eq!(
        summary["stubbed"],
        json!({"passed": 0, "failed": 0, "error": 0}),
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
    assert_no_tdd_al0132(&s.until_summary());
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

/// Issue 5798: through `entries`, every test line has `codeunit` and `method`, as the one-shot `--output-json` row does;
/// a method name that holds a dot cannot be split back out of `name`.
#[test]
fn entries_give_every_test_line_its_codeunit_and_method_also_for_a_dotted_name() {
    let d = tempfile::tempdir().unwrap();
    let a = app(
        d.path(),
        "dots",
        "codeunit 50990 \"Dot Name Tests\"
{
    Subtype = Test;

    [Test]
    procedure \"Dot.Step\"()
    begin
    end;

    [Test]
    procedure PlainStep()
    begin
    end;
}",
    );
    let mut s = started();
    for request in [
        json!({"command": "runTests", "sourcePaths": [a], "entries": [{"id": "NONE|1"}]}),
        json!({"command": "runTests", "sourcePaths": [a]}),
    ] {
        s.send(request.clone());
        let lines = s.until_summary();
        let tests: Vec<&Value> = lines.iter().filter(|l| l["type"] == "test").collect();
        assert_eq!(tests.len(), 2, "{request}: {lines:?}");
        for (name, method) in [
            ("Codeunit50990.Dot.Step", "Dot.Step"),
            ("Codeunit50990.PlainStep", "PlainStep"),
        ] {
            let l = tests
                .iter()
                .find(|l| l["name"] == name)
                .unwrap_or_else(|| panic!("{name}: {lines:?}"));
            assert_eq!(
                (&l["codeunit"], &l["method"]),
                (&json!(50990), &json!(method)),
                "{request}: {l}"
            );
        }
    }
}

/// Issue 5793: a `test` line of a test the summed Sleep time stopped says `clock: "virtual"`, as the JSON row does.
#[test]
fn a_test_line_of_a_test_the_sleep_time_stopped_says_the_clock_was_virtual() {
    let d = tempfile::tempdir().unwrap();
    let a = app(
        d.path(),
        "sleepstop",
        "codeunit 50413 \"Sleep Stop\" { Subtype = Test;
            [Test] procedure ShortSleep() begin Sleep(1500); end;
            [Test] procedure LongSleep() begin Sleep(600000); end; }",
    );
    let mut s = Server::start(&["--no-download", "--no-test-data", "--test-timeout", "3"]);
    assert_eq!(s.line(), json!({"ready": true}));
    s.send(json!({"command": "runTests", "sourcePaths": [a]}));
    let lines = s.until_summary();
    let line = |n: &str| lines.iter().find(|l| l["name"] == n).cloned().unwrap();
    let long = line("Codeunit50413.LongSleep");
    assert_eq!(long["message"], "Test exceeded 3s timeout.", "{lines:?}");
    assert_eq!(long["clock"], "virtual", "{lines:?}");
    assert!(line("Codeunit50413.ShortSleep").get("clock").is_none(), "{lines:?}");
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
    s.send(
        json!({"command": "runTests", "sourcePaths": [a], "testIsolation": "test", "entries": [
            {"id": "ok", "test": "Codeunit50661.Passes"},
            {"id": "none", "test": "Codeunit59999."},
        ]}),
    );
    let lines = s.until_summary();
    let entry = |id: &str| {
        lines
            .iter()
            .find(|l| l["type"] == "entrySummary" && l["entry"] == id)
            .unwrap_or_else(|| panic!("{id}: {lines:?}"))
            .clone()
    };
    let ok = entry("ok");
    assert_eq!(
        (ok["total"].as_u64(), ok["exitCode"].as_u64()),
        (Some(1), Some(0)),
        "{ok}"
    );
    assert!(ok.get("warnings").is_none(), "{ok}");
    let none = entry("none");
    assert_eq!(
        (none["total"].as_u64(), none["exitCode"].as_u64()),
        (Some(0), Some(6)),
        "{none}"
    );
    assert!(
        none["warnings"][0]
            .as_str()
            .unwrap_or_default()
            .contains("matches no test"),
        "{none}"
    );
    let summary = lines.last().unwrap();
    assert_eq!(summary["type"], "summary", "{summary}");
    assert_eq!(
        (summary["exitCode"].as_u64(), summary["passed"].as_u64()),
        (Some(6), Some(1)),
        "{summary}"
    );
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
/// codeunits/run-onrun/QuCommitIsolation.al:10-16, 25-39).
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

// ADR-055 (al-runner issue 5859): the stream has no `OnRun` lines, and a codeunit whose OnRun raises has no lines.
#[test]
fn run_tests_streams_no_on_run_rows() {
    let d = tempfile::tempdir().unwrap();
    let a = app(
        d.path(),
        "onrun",
        "codeunit 50981 \"Raises\" { Subtype = Test; trigger OnRun() begin Error('boom 537 onrun'); end; [Test] procedure T1() begin end; }
codeunit 50982 \"Quiet\" { Subtype = Test; trigger OnRun() begin end; [Test] procedure T2() begin end; }",
    );
    let mut s = Server::start(&["--no-download"]);
    assert_eq!(s.line(), json!({"ready": true}));
    s.send(json!({"command": "runTests", "sourcePaths": [a]}));
    let lines = s.until_summary();
    // The codeunits may run on threads of their own: the lines are compared in name order.
    let mut tests: Vec<(String, String)> = lines
        .iter()
        .filter(|l| l["type"] == "test")
        .map(|l| (l["name"].as_str().unwrap().into(), l["status"].as_str().unwrap().into()))
        .collect();
    tests.sort();
    assert_eq!(
        tests,
        [("Codeunit50982.T2".to_string(), "pass".to_string())],
        "{lines:?}"
    );
    assert_eq!(lines.last().unwrap()["exitCode"], 0, "{lines:?}");
    s.send(json!({"command": "shutdown"}));
    assert_eq!(s.line(), json!({"status": "shutting down"}));
}

/// A dependency app (folder `dep`) whose install code leaves one row, and an app under test whose two test codeunits
/// read it and write.
fn installing_apps(root: &Path) -> (PathBuf, PathBuf) {
    let dep = root.join("dep");
    write(
        &dep,
        "app.json",
        r#"{"id":"33333333-0000-0000-0000-000000000001","name":"Seeding Dep","publisher":"Someone","version":"1.0.0.0","dependencies":[]}"#,
    );
    write(
        &dep,
        "src/Dep.al",
        "table 50700 Seeded { fields { field(1; K; Code[20]) { } field(2; N; Integer) { } } keys { key(PK; K) { } } }
codeunit 50701 \"Seeding Install\" { Subtype = Install;
    trigger OnInstallAppPerCompany() var S: Record Seeded; begin S.K := 'INSTALL'; S.N := 10; S.Insert(); end; }",
    );
    let main = root.join("main");
    write(
        &main,
        "app.json",
        r#"{"id":"33333333-0000-0000-0000-000000000002","name":"Seeded Tests","publisher":"Test","version":"1.0.0.0","dependencies":[{"id":"33333333-0000-0000-0000-000000000001","name":"Seeding Dep","publisher":"Someone","version":"1.0.0.0"}]}"#,
    );
    let test = |id: u32, key: &str| {
        format!(
            "codeunit {id} \"Seeded {key}\" {{ Subtype = Test;
    [Test] procedure SeesTheInstallRowOnly() var S: Record Seeded; begin
        if S.Count() <> 1 then Error('Seeded holds %1 rows', S.Count());
        S.Get('INSTALL'); if S.N <> 10 then Error('N is %1', S.N);
        S.N += 1; S.Modify(); S.K := '{key}'; S.Insert(); Commit(); end; }}\n"
        )
    };
    write(&main, "src/T.al", &(test(50710, "A") + &test(50711, "B")));
    (main, dep)
}

/// The parts of `setupMs` in a summary's `timings.setupParts` (al-runner issue 8205).
const SETUP_PARTS: [&str; 10] = [
    "optionsMs",
    "sourcesMs",
    "selectionMs",
    "dependenciesMs",
    "testDataMs",
    "licenseMs",
    "dotnetMs",
    "affectedMs",
    "entriesMs",
    "prepareMs",
];

/// Al-runner issue 5856: every request says where its time went, and a request over a kept compile also keeps the
/// engines and the install baseline, while it still starts from the rows the install code left and none of an earlier
/// request's writes.
#[test]
fn a_summary_says_where_the_time_went_and_a_kept_baseline_does_not_carry_writes() {
    let d = tempfile::tempdir().unwrap();
    let (main, dep) = installing_apps(d.path());
    let dep = dep.to_string_lossy().to_string();
    let cache = d.path().join("cache").to_string_lossy().to_string();
    let mut s = Server::start(&["--no-download", "--no-test-data", "--dep", &dep, "--cache-dir", &cache]);
    s.line();
    let mut summaries = Vec::new();
    for entries in [
        None,
        None,
        Some(json!([{"id": "a", "test": "Codeunit50710."}, {"id": "b"}])),
    ] {
        let mut req = json!({"command": "runTests", "sourcePaths": [main]});
        if let Some(e) = entries {
            req["entries"] = e;
        }
        s.send(req);
        let lines = s.until_summary();
        let failed: Vec<&Value> = lines
            .iter()
            .filter(|l| l["type"] == "test" && l["status"] != "pass")
            .collect();
        assert!(failed.is_empty(), "{lines:?}");
        for e in lines.iter().filter(|l| l["type"] == "entrySummary") {
            assert!(
                e["timings"]["resetMs"].is_number() && e["timings"]["wallMs"].is_number(),
                "{e}"
            );
        }
        summaries.push(lines.last().unwrap().clone());
    }
    for summary in &summaries {
        let t = &summary["timings"];
        for key in [
            "setupMs",
            "compileMs",
            "engineMs",
            "baselineMs",
            "testsMs",
            "coverageMs",
            "otherMs",
        ] {
            assert!(t[key].as_f64().is_some_and(|v| v >= 0.0), "{key}: {summary}");
        }
        // Al-runner issue 8205: `setupMs` split into its parts, which add up to it (each rounded to a tenth).
        let parts = &t["setupParts"];
        let mut sum = 0.0;
        for key in SETUP_PARTS {
            let v = parts[key]
                .as_f64()
                .unwrap_or_else(|| panic!("setupParts.{key}: {summary}"));
            assert!(v >= 0.0, "setupParts.{key}: {summary}");
            sum += v;
        }
        assert_eq!(parts.as_object().map(|o| o.len()), Some(SETUP_PARTS.len()), "{summary}");
        let total = t["setupMs"].as_f64().unwrap();
        assert!((sum - total).abs() <= 0.5, "parts {sum} vs setupMs {total}: {summary}");
    }
    assert_eq!(
        summaries.iter().map(|s| s["passed"].clone()).collect::<Vec<_>>(),
        vec![json!(2), json!(2), json!(3)],
        "{summaries:?}"
    );
    // The first request ran the install code; the later ones used what it kept.
    assert_eq!(summaries[0]["timings"]["baselineSource"], "ran", "{summaries:?}");
    for later in &summaries[1..] {
        assert_eq!(later["cached"], true, "{later}");
        assert_eq!(later["timings"]["baselineSource"], "kept", "{later}");
        assert_eq!(later["timings"]["engineKept"], true, "{later}");
    }
    s.send(json!({"command": "shutdown"}));
    assert_eq!(s.line(), json!({"status": "shutting down"}));
}

/// A dependency app (folder `dep`) with a SingleInstance codeunit, and an app under test with one of its own, a Writer
/// test codeunit that sets session state of each kind and a Reader that fails unless it sees none of it.
fn state_apps(root: &Path) -> (PathBuf, PathBuf) {
    let dep = root.join("dep");
    write(
        &dep,
        "app.json",
        r#"{"id":"33333333-0000-0000-0000-000000000011","name":"State Dep","publisher":"Someone","version":"1.0.0.0","dependencies":[]}"#,
    );
    write(
        &dep,
        "src/Dep.al",
        "codeunit 50720 \"Dep Single\" { SingleInstance = true; var V: Integer;
    procedure SetV(N: Integer) begin V := N; end; procedure GetV(): Integer begin exit(V); end; }",
    );
    let main = root.join("main");
    write(
        &main,
        "app.json",
        r#"{"id":"33333333-0000-0000-0000-000000000012","name":"State Tests","publisher":"Test","version":"1.0.0.0","dependencies":[{"id":"33333333-0000-0000-0000-000000000011","name":"State Dep","publisher":"Someone","version":"1.0.0.0"}]}"#,
    );
    write(
        &main,
        "src/T.al",
        "codeunit 50721 \"Own Single\" { SingleInstance = true; var V: Integer;
    procedure SetV(N: Integer) begin V := N; end; procedure GetV(): Integer begin exit(V); end; }
codeunit 50722 Writer { Subtype = Test;
    [Test] procedure WritesState() var D: Codeunit \"Dep Single\"; O: Codeunit \"Own Single\"; begin
        WorkDate(20010101D); D.SetV(7); O.SetV(8);
        NumberSequence.Insert('STATE5862'); IsolatedStorage.Set('state5862', 'x'); end; }
codeunit 50723 Reader { Subtype = Test;
    [Test] procedure SeesNoState() var D: Codeunit \"Dep Single\"; O: Codeunit \"Own Single\"; begin
        if WorkDate() = 20010101D then Error('WorkDate leaked');
        if D.GetV() <> 0 then Error('dependency SingleInstance leaked');
        if O.GetV() <> 0 then Error('SingleInstance leaked');
        if NumberSequence.Exists('STATE5862') then Error('NumberSequence leaked');
        if IsolatedStorage.Contains('state5862') then Error('IsolatedStorage leaked'); end; }\n",
    );
    (main, dep)
}

/// Al-runner issue 5862: each test codeunit starts from a new session over the post-install database, in every entry of
/// a request and in every request: WorkDate, SingleInstance codeunits (also of a dependency app), NumberSequence and
/// IsolatedStorage that an earlier codeunit set are gone, and a second Writer runs clean.
#[test]
fn every_entry_and_request_starts_without_the_state_an_earlier_one_set() {
    let d = tempfile::tempdir().unwrap();
    let (main, dep) = state_apps(d.path());
    let dep = dep.to_string_lossy().to_string();
    let mut s = Server::start(&["--no-download", "--no-test-data", "--dep", &dep]);
    s.line();
    let w = |id: &str| json!({"id": id, "test": "Codeunit50722."});
    let r = |id: &str| json!({"id": id, "test": "Codeunit50723."});
    let requests = [
        vec![r("r")],
        vec![w("w"), r("r")],
        vec![r("r"), w("w")],
        vec![w("w1"), r("r1"), w("w2"), r("r2"), r("r3")],
        vec![r("r")],
    ];
    for entries in requests {
        let n = entries.len();
        s.send(json!({"command": "runTests", "sourcePaths": [main], "entries": entries}));
        let lines = s.until_summary();
        let tests: Vec<&Value> = lines.iter().filter(|l| l["type"] == "test").collect();
        assert_eq!(tests.len(), n, "{lines:?}");
        assert!(tests.iter().all(|t| t["status"] == "pass"), "{lines:?}");
    }
    s.send(json!({"command": "shutdown"}));
    assert_eq!(s.line(), json!({"status": "shutting down"}));
}

const WORK_DATE_PAIR: &str = r#"
codeunit 50100 "Dates"
{
    Subtype = Test;

    [Test]
    procedure SetsIt()
    begin
        WorkDate(20200315D);
    end;

    [Test]
    procedure ReadsIt()
    begin
        if WorkDate() = 20200315D then
            Error('WorkDate leaked');
    end;
}
"#;

/// Al-runner issue 7207: an entry that names one method starts from the same fresh session as a whole-codeunit entry,
/// so a work date an earlier request or entry set is gone (docs/manual/tool-protocol.md, issue 5862).
#[test]
fn a_single_method_entry_starts_without_the_work_date_an_earlier_one_set() {
    let d = tempfile::tempdir().unwrap();
    let a = app(d.path(), "workdatepair", WORK_DATE_PAIR);
    let mut s = Server::start(&["--no-download", "--no-test-data"]);
    s.line();
    let requests = [
        vec![json!({"id": "s", "test": "Codeunit50100.SetsIt"})],
        vec![json!({"id": "r", "test": "Codeunit50100.ReadsIt"})],
        vec![
            json!({"id": "s", "test": "Codeunit50100.SetsIt"}),
            json!({"id": "r", "test": "Codeunit50100.ReadsIt"}),
        ],
    ];
    for entries in requests {
        s.send(json!({"command": "runTests", "sourcePaths": [a], "entries": entries, "testIsolation": "test"}));
        let lines = s.until_summary();
        let tests: Vec<&Value> = lines.iter().filter(|l| l["type"] == "test").collect();
        assert!(tests.iter().all(|t| t["status"] == "pass"), "{lines:?}");
    }
    s.send(json!({"command": "shutdown"}));
    assert_eq!(s.line(), json!({"status": "shutting down"}));
}

/// Al-runner issue 7207, owner decision 2026-11-06: the plain `test` field of a request that picks exactly one test
/// runs that test alone, as an entry does; a sibling that sets WorkDate does not run before it.
#[test]
fn a_plain_test_field_picking_one_test_runs_it_without_its_siblings() {
    let d = tempfile::tempdir().unwrap();
    let a = app(d.path(), "workdatepair", WORK_DATE_PAIR);
    let mut s = Server::start(&["--no-download", "--no-test-data"]);
    s.line();
    s.send(
        json!({"command": "runTests", "sourcePaths": [a], "test": "Codeunit50100.ReadsIt", "testIsolation": "test"}),
    );
    let lines = s.until_summary();
    let tests: Vec<&Value> = lines.iter().filter(|l| l["type"] == "test").collect();
    assert_eq!(tests.len(), 1, "{lines:?}");
    assert!(tests.iter().all(|t| t["status"] == "pass"), "{lines:?}");
    s.send(json!({"command": "shutdown"}));
    assert_eq!(s.line(), json!({"status": "shutting down"}));
}

const DRAWS: &str = r#"
codeunit 50310 "Draw Tests"
{
    Subtype = Test;

    [Test]
    procedure Draws()
    begin
        Error('%1 %2', Random(1000000), CreateGuid());
    end;
}
"#;

/// ADR-061 section 3: `seed` replays a request's values; a request without one gets a fresh seed; the summary says
/// which. The warm slot is reused across seeds.
#[test]
fn a_request_may_set_the_run_seed_and_the_summary_names_it() {
    let d = tempfile::tempdir().unwrap();
    let a = app(d.path(), "draws", DRAWS);
    let mut s = Server::start_with(&["--no-download", "--no-test-data"], &[]);
    assert_eq!(s.line(), json!({"ready": true}));
    let mut run = |seed: Option<i64>| {
        let mut req = json!({"command": "runTests", "sourcePaths": [a]});
        if let Some(n) = seed {
            req["seed"] = n.into();
        }
        s.send(req);
        let lines = s.until_summary();
        let message = lines.iter().find(|l| l["type"] == "test").unwrap()["message"].clone();
        (message, lines.last().unwrap().clone())
    };
    let (first, summary) = run(Some(4271833));
    assert_eq!(
        (summary["seed"].as_i64(), summary["seedSource"].as_str()),
        (Some(4271833), Some("option"))
    );
    let (again, summary) = run(Some(4271833));
    assert_eq!(again, first);
    assert_eq!(
        summary["cached"], true,
        "the seed is not part of the warm slot: {summary}"
    );
    let (other, _) = run(Some(4271834));
    assert_ne!(other, first);
    let (_, summary) = run(None);
    assert_eq!(summary["seedSource"], "fresh", "{summary}");
}

fn app_needing_missing_lib(root: &Path) -> PathBuf {
    let a = app(root, "needslib", MIXED);
    write(
        &a,
        "app.json",
        r#"{"id":"71390000-0000-0000-0000-000000000002","name":"needslib","publisher":"Test","version":"1.0.0.0","dependencies":[{"id":"71390000-0000-0000-0000-0000000000aa","name":"Contoso Lib","publisher":"Contoso","version":"2.1.0.0"}]}"#,
    );
    a
}

/// al-runner issue 7139: a dependency that is not found under --no-download is an error line by default.
#[test]
fn a_missing_dependency_under_no_download_is_an_error_line() {
    let d = tempfile::tempdir().unwrap();
    let a = app_needing_missing_lib(d.path());
    let cache = d.path().join("cache");
    let mut s = Server::start(&[
        "--no-download",
        "--no-test-data",
        "--cache-dir",
        cache.to_str().unwrap(),
    ]);
    assert_eq!(s.line(), json!({"ready": true}));
    s.send(json!({"command": "runTests", "sourcePaths": [a]}));
    let lines = s.until_summary();
    let e = lines.last().unwrap()["error"].as_str().unwrap_or_default().to_string();
    assert!(
        e.contains("Contoso Lib") && e.contains("--allow-missing-dependencies"),
        "{lines:?}"
    );
}

/// With the flag the run goes on and the summary names the skipped dependency; a run without one has an empty list.
#[test]
fn the_summary_lists_skipped_dependencies() {
    let d = tempfile::tempdir().unwrap();
    let a = app_needing_missing_lib(d.path());
    let ok = app(d.path(), "plain", MIXED);
    let cache = d.path().join("cache");
    let mut s = Server::start(&[
        "--no-download",
        "--no-test-data",
        "--allow-missing-dependencies",
        "--cache-dir",
        cache.to_str().unwrap(),
    ]);
    assert_eq!(s.line(), json!({"ready": true}));
    s.send(json!({"command": "runTests", "sourcePaths": [a]}));
    let summary = s.until_summary().pop().unwrap();
    assert_eq!(summary["type"], "summary", "{summary}");
    let list = summary["skippedDependencies"].as_array().expect("skippedDependencies");
    assert_eq!(list.len(), 1, "{summary}");
    assert_eq!(
        (
            list[0]["id"].as_str(),
            list[0]["name"].as_str(),
            list[0]["version"].as_str()
        ),
        (
            Some("71390000-0000-0000-0000-0000000000aa"),
            Some("Contoso Lib"),
            Some("2.1.0.0")
        )
    );
    s.send(json!({"command": "runTests", "sourcePaths": [ok]}));
    let summary = s.until_summary().pop().unwrap();
    assert_eq!(summary["skippedDependencies"], json!([]), "{summary}");
}

/// Al-runner issue 8125: `testTimeoutSecs` sets the time limit of a request and of an entry (the entry's wins, then the
/// request's, then the server's `--test-timeout`), on the kept compile. The time `Sleep` asks for counts.
#[test]
fn a_request_and_an_entry_set_the_test_time_limit_on_the_kept_compile() {
    let d = tempfile::tempdir().unwrap();
    let a = app(
        d.path(),
        "timelimit",
        "codeunit 50414 \"Time Limit\" { Subtype = Test;
            [Test] procedure SleepsFive() begin Sleep(5000); end; }",
    );
    let mut s = Server::start(&["--no-download", "--no-test-data", "--test-timeout", "30"]);
    assert_eq!(s.line(), json!({"ready": true}));
    let mut run = |request: Value| {
        s.send(request);
        let lines = s.until_summary();
        let summary = lines.last().unwrap().clone();
        assert_eq!(summary["type"], "summary", "{lines:?}");
        let rows: Vec<(String, String)> = lines
            .iter()
            .filter(|l| l["type"] == "test")
            .map(|l| {
                (
                    l["entry"].as_str().unwrap_or("").to_string(),
                    l["message"]
                        .as_str()
                        .unwrap_or(l["status"].as_str().unwrap())
                        .to_string(),
                )
            })
            .collect();
        (rows, summary["cached"].as_bool())
    };
    let pass = |entry: &str| (entry.to_string(), "pass".to_string());
    let stop = |entry: &str, n: u32| (entry.to_string(), format!("Test exceeded {n}s timeout."));
    let (rows, cached) = run(json!({"command": "runTests", "sourcePaths": [a]}));
    assert_eq!((rows, cached), (vec![pass("")], Some(false)), "the server's 30 s");
    let (rows, cached) = run(json!({"command": "runTests", "sourcePaths": [a], "testTimeoutSecs": 2}));
    assert_eq!(
        (rows, cached),
        (vec![stop("", 2)], Some(true)),
        "the request's 2 s, no new compile"
    );
    let (rows, cached) = run(
        json!({"command": "runTests", "sourcePaths": [a], "testTimeoutSecs": 2, "entries": [
            {"id": "own", "test": "Codeunit50414.SleepsFive", "testTimeoutSecs": 4},
            {"id": "none", "test": "Codeunit50414.SleepsFive", "testTimeoutSecs": 0},
            {"id": "request", "test": "Codeunit50414.SleepsFive"},
        ]}),
    );
    assert_eq!(
        (rows, cached),
        (vec![stop("own", 4), pass("none"), stop("request", 2)], Some(true)),
        "an entry's own limit, 0 for none, else the request's"
    );
    let (rows, cached) = run(json!({"command": "runTests", "sourcePaths": [a], "entries": [
        {"id": "server", "test": "Codeunit50414.SleepsFive"},
        {"id": "own", "test": "Codeunit50414.SleepsFive", "testTimeoutSecs": 3},
    ]}));
    assert_eq!(
        (rows, cached),
        (vec![pass("server"), stop("own", 3)], Some(true)),
        "an entry without a limit in a request without one: the server's"
    );
    let (rows, cached) = run(json!({"command": "runTests", "sourcePaths": [a]}));
    assert_eq!(
        (rows, cached),
        (vec![pass("")], Some(true)),
        "a request does not change the server's limit"
    );
    s.send(json!({"command": "runTests", "sourcePaths": [a], "testTimeoutSecs": -1}));
    let lines = s.until_summary();
    assert!(
        lines[0]["error"].as_str().unwrap().contains("`testTimeoutSecs`"),
        "{lines:?}"
    );
}

// al-runner issue 7775: the reporter's repro, a FindSet on the Company table.
const COMPANY_FINDSET: &str = r#"
codeunit 50420 "Company FindSet"
{
    Subtype = Test;

    [Test]
    procedure FindSetAll()
    var
        C: Record Company;
    begin
        if C.FindSet() then;
    end;
}
"#;

/// al-runner issue 7775: `companies` is `--companies` for one request. Without it (and without the server's flag) the
/// FindSet is refused, and the refusal names the flag and the field; a request without it does not keep the last one's.
#[test]
fn the_companies_field_is_the_companies_flag_for_one_request() {
    let d = tempfile::tempdir().unwrap();
    let a = app(d.path(), "companies", COMPANY_FINDSET);
    // The Company table is a table of the platform's System app, which the app names through `platform`.
    let manifest = std::fs::read_to_string(a.join("app.json")).unwrap();
    write(
        &a,
        "app.json",
        &manifest.replace(r#""dependencies""#, r#""platform":"27.0.0.0","dependencies""#),
    );
    let packages = d.path().join("packages");
    std::fs::create_dir_all(&packages).unwrap();
    std::fs::copy(al_testfiles::system_app(), packages.join("System.app")).unwrap();
    let run = |s: &mut Server, req: Value| -> (Value, Value) {
        s.send(req);
        let lines = s.until_summary();
        let t = lines
            .iter()
            .find(|l| l["type"] == "test")
            .unwrap_or_else(|| panic!("{lines:?}"));
        (t["status"].clone(), t["message"].clone())
    };
    let mut s = Server::start(&["--no-download", "--no-test-data"]);
    s.line();
    let refused = |o: &(Value, Value)| {
        let m = o.1.as_str().unwrap_or_default();
        o.0 != "pass" && m.contains("--companies") && m.contains("`companies`")
    };
    let o = run(
        &mut s,
        json!({"command": "runTests", "sourcePaths": [a], "packagePaths": [packages]}),
    );
    assert!(refused(&o), "{o:?}");
    let o = run(
        &mut s,
        json!({"command": "runTests", "sourcePaths": [a], "packagePaths": [packages], "companies": "CRONUS International Ltd."}),
    );
    assert_eq!(o.0, "pass", "{o:?}");
    let o = run(
        &mut s,
        json!({"command": "runTests", "sourcePaths": [a], "packagePaths": [packages]}),
    );
    assert!(refused(&o), "the next request without the field: {o:?}");
    s.send(json!({"command": "runTests", "sourcePaths": [a], "packagePaths": [packages], "companies": ["CRONUS International Ltd."]}));
    let lines = s.until_summary();
    assert!(lines[0]["error"].as_str().unwrap().contains("`companies`"), "{lines:?}");
    s.send(json!({"command": "runTests", "sourcePaths": [a], "packagePaths": [packages], "companies": "Other,CRONUS International Ltd."}));
    let lines = s.until_summary();
    assert_eq!(
        lines[0]["error"].as_str(),
        Some("`companies`: the first company must be the session company `CRONUS International Ltd.`, not `Other`"),
        "the session company comes first: {lines:?}"
    );
    // The server's `--companies` holds for a request without the field, as the CLI flag does.
    let mut s = Server::start(&[
        "--no-download",
        "--no-test-data",
        "--companies",
        "CRONUS International Ltd.",
    ]);
    s.line();
    let o = run(
        &mut s,
        json!({"command": "runTests", "sourcePaths": [a], "packagePaths": [packages]}),
    );
    assert_eq!(o.0, "pass", "{o:?}");
}

/// Al-runner issue 8206: a dependency with install code (a row `INSTALL`), and an app under test with a resource file
/// `flag.txt` whose test fails with the flag text and the row count in its message. With `install_reads_resource`, the
/// app's own install code (which the install baseline runs too) adds a row holding the text of `flag.txt`.
fn resource_baseline_apps(root: &Path, install_reads_resource: bool) -> (PathBuf, PathBuf) {
    let dep = root.join("dep");
    write(
        &dep,
        "app.json",
        r#"{"id":"82060000-0000-0000-0000-000000000001","name":"Install Dep","publisher":"Someone","version":"1.0.0.0","dependencies":[]}"#,
    );
    write(
        &dep,
        "src/Dep.al",
        "table 50830 Installed { fields { field(1; K; Code[20]) { } } keys { key(PK; K) { } } }
codeunit 50831 \"Dep Install\" { Subtype = Install;
    trigger OnInstallAppPerCompany() var S: Record Installed; begin S.K := 'INSTALL'; S.Insert(); end; }",
    );
    let main = root.join("main");
    write(
        &main,
        "app.json",
        r#"{"id":"82060000-0000-0000-0000-000000000002","name":"Resource Tests","publisher":"Test","version":"1.0.0.0","resourceFolders":["res"],"dependencies":[{"id":"82060000-0000-0000-0000-000000000001","name":"Install Dep","publisher":"Someone","version":"1.0.0.0"}]}"#,
    );
    write(&main, "res/flag.txt", "off");
    let install = if install_reads_resource {
        "codeunit 50833 \"Reads Flag\" { Subtype = Install;
    trigger OnInstallAppPerCompany() var S: Record Installed; begin
        S.K := CopyStr(NavApp.GetResourceAsText('flag.txt'), 1, 20); S.Insert(); end; }\n"
    } else {
        ""
    };
    write(
        &main,
        "src/T.al",
        &format!(
            "codeunit 50832 \"Resource Tests\" {{ Subtype = Test;
    [Test] procedure Flag() var S: Record Installed; begin
        Error('flag=%1 rows=%2', NavApp.GetResourceAsText('flag.txt'), S.Count()); end; }}\n{install}"
        ),
    );
    (main, dep)
}

/// The test lines of a request (name, status, message) and its summary's `baselineSource`.
fn outcomes_and_source(s: &mut Server, req: Value) -> (Vec<(String, String, String)>, Value) {
    s.send(req);
    let lines = s.until_summary();
    let tests = lines
        .iter()
        .filter(|l| l["type"] == "test")
        .map(|l| {
            let text = |k: &str| l[k].as_str().unwrap_or_default().to_string();
            (text("name"), text("status"), text("message"))
        })
        .collect();
    let summary = lines.last().unwrap();
    assert!(summary["type"] == "summary", "{lines:?}");
    (tests, summary["timings"]["baselineSource"].clone())
}

/// Al-runner issue 8206: a kept daemon takes the kept install baseline for entries with `resources`, and every request
/// gives what a fresh daemon gives for it: the entries' texts (the same or another), a resource file changed on disk,
/// and install code that reads a resource (where a fresh run refuses an entry with resources of its own).
#[test]
fn entries_with_resources_take_the_kept_baseline_and_give_what_a_fresh_daemon_gives() {
    for install_reads_resource in [false, true] {
        let d = tempfile::tempdir().unwrap();
        let (main, dep) = resource_baseline_apps(d.path(), install_reads_resource);
        let dep = dep.to_string_lossy().to_string();
        let start = |cache: &Path| {
            let cache = cache.to_string_lossy().to_string();
            let mut s = Server::start(&["--no-download", "--no-test-data", "--dep", &dep, "--cache-dir", &cache]);
            s.line();
            s
        };
        let request = |text: Option<&str>| {
            let mut req = json!({"command": "runTests", "sourcePaths": [main], "testIsolation": "test"});
            if let Some(t) = text {
                req["entries"] = json!([{"id": "e0", "resources": {"flag.txt": t}, "test": "Codeunit50832."}]);
            }
            req
        };
        let mut kept = start(&d.path().join("kept-cache"));
        let steps: [(Option<&str>, Option<&str>); 5] = [
            (Some("on"), None),
            (Some("on"), None),
            (Some("other"), None),
            (None, None),
            // The app's own resource file changes on disk: a new compile and a new baseline.
            (Some("on"), Some("changed")),
        ];
        let mut sources = Vec::new();
        for (n, (text, on_disk)) in steps.into_iter().enumerate() {
            if let Some(t) = on_disk {
                write(&main, "res/flag.txt", t);
            }
            let (got, source) = outcomes_and_source(&mut kept, request(text));
            let mut fresh = start(&d.path().join(format!("fresh-cache-{n}")));
            let (want, fresh_source) = outcomes_and_source(&mut fresh, request(text));
            assert_eq!(fresh_source, "ran", "a fresh daemon runs the install code");
            assert_eq!(
                got, want,
                "step {n} (install reads a resource: {install_reads_resource})"
            );
            // What a fresh run gives: the entry's text and the install row, or, when the install code read a resource
            // and the entry has resources of its own, the test not run (issue 5635).
            let message = &want[0].2;
            let expected = match (install_reads_resource, text) {
                (true, Some(_)) => "the install code read a resource file".to_string(),
                (false, Some(t)) => format!("flag={t} rows=1"),
                (reads, None) => format!("flag=off rows={}", 1 + usize::from(reads)),
            };
            assert!(message.contains(&expected), "step {n}: {message}");
            sources.push(source);
        }
        assert_eq!(
            sources,
            [json!("ran"), json!("kept"), json!("kept"), json!("kept"), json!("ran")],
            "install reads a resource: {install_reads_resource}"
        );
    }
}

/// `timings.testDataIndex` says how the open of the test data got the backup's kept index (al-runner issue 8479).
#[test]
fn a_summary_says_how_the_test_data_index_was_obtained() {
    let d = tempfile::tempdir().unwrap();
    let a = app(d.path(), "idx", MIXED);
    let cache = d.path().join("cache");
    let bak = Path::new(env!("CARGO_MANIFEST_DIR")).join("../al-bcdb/tests/fixtures/typeprobe.bak");
    let (cache, bak) = (cache.to_str().unwrap(), bak.to_str().unwrap());
    let mut states = Vec::new();
    for _ in 0..2 {
        let mut s = Server::start(&[
            "--no-download",
            "--cache-dir",
            cache,
            "--test-data",
            bak,
            "--test-data-company",
            "TP",
        ]);
        assert_eq!(s.line(), json!({"ready": true}));
        // The first request opens the backup; the second, on the same daemon, repeats what the open did.
        for _ in 0..2 {
            s.send(json!({"command": "runTests", "sourcePaths": [a]}));
            let summary = s.until_summary().pop().unwrap();
            states.push(summary["timings"]["testDataIndex"].clone());
        }
    }
    assert_eq!(
        states,
        [json!("built"), json!("built"), json!("read"), json!("read")],
        "{states:?}"
    );
    let mut s = Server::start(&["--no-download", "--no-test-data"]);
    s.line();
    s.send(json!({"command": "runTests", "sourcePaths": [a]}));
    let summary = s.until_summary().pop().unwrap();
    assert!(summary["timings"].get("testDataIndex").is_none(), "{summary}");
}

/// Two apps whose test needs the complete compile, so the dependency app is compiled into a layer of the compile cache.
#[cfg(target_os = "linux")] // its only user, the deleted-executable test below, is Linux only
fn layered_apps(root: &Path) -> (PathBuf, PathBuf) {
    let dep = root.join("dep");
    write(
        &dep,
        "app.json",
        r#"{"id":"82080000-0000-0000-0000-000000000001","name":"Literal Dep","publisher":"Probe","version":"1.0.0.0","dependencies":[]}"#,
    );
    write(
        &dep,
        "src/D.al",
        "table 50952 \"Literal Row\" { fields { field(1; K; Integer) { } } keys { key(PK; K) { } } }
codeunit 50950 \"Literal Source\" { procedure Value(): Integer begin exit(41); end; }\n",
    );
    let main = root.join("main");
    write(
        &main,
        "app.json",
        r#"{"id":"82080000-0000-0000-0000-000000000002","name":"Literal Tests","publisher":"Probe","version":"1.0.0.0","dependencies":[{"id":"82080000-0000-0000-0000-000000000001","name":"Literal Dep","publisher":"Probe","version":"1.0.0.0"}]}"#,
    );
    write(
        &main,
        "src/T.al",
        "codeunit 50951 \"Literal Tests\" { Subtype = Test;
    [Test] procedure ShowValue() var S: Codeunit \"Literal Source\"; R: Record \"Literal Row\";
    begin R.K := 1; R.Insert(); Error('v=%1 rows=%2', S.Value(), R.Count()); end; }\n",
    );
    (main, dep)
}

/// The build tags (the first 16 hex of a cache file's name) of the dependency layers in `cache`.
#[cfg(target_os = "linux")] // its only user, the deleted-executable test below, is Linux only
fn layer_tags(cache: &Path) -> Vec<String> {
    let mut tags: Vec<String> = std::fs::read_dir(cache.join("compile"))
        .map(|rd| {
            rd.filter_map(|e| e.ok()?.file_name().into_string().ok())
                .filter(|n| n.ends_with(".alcc"))
                .map(|n| n[..16].to_string())
                .collect()
        })
        .unwrap_or_default();
    tags.sort();
    tags.dedup();
    tags
}

/// The compile cache's key holds a hash of the running build (`compile_cache::runtime_build`). A server whose executable
/// was deleted or replaced after it started (an upgrade, a rebuild) still knows its build: it files its entries under the
/// tag a normal run of the same build uses, not under a value two builds could share (two builds that both fell back read
/// each other's entries: a program lowered by another build).
#[cfg(target_os = "linux")]
#[test]
fn a_server_whose_executable_was_deleted_files_its_cache_entries_under_its_own_build() {
    let d = tempfile::tempdir().unwrap();
    let (main, dep) = layered_apps(d.path());
    let dep = dep.to_string_lossy().to_string();
    // A normal run of this build: the tag its layers carry.
    let normal = d.path().join("normal-cache");
    let o = Command::new(env!("CARGO_BIN_EXE_al-runner"))
        .args([
            "test",
            main.to_str().unwrap(),
            "--dep",
            &dep,
            "--no-download",
            "--no-test-data",
        ])
        .env("ALRUNNER_CACHE", &normal)
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&o.stderr).to_string() + &String::from_utf8_lossy(&o.stdout);
    assert!(text.contains("v=41"), "{text}");
    let own = layer_tags(&normal);
    assert_eq!(own.len(), 1, "one layer: {own:?}");
    // The same build, started from a copy that is deleted before the first request.
    let exe = d.path().join("al-runner-copy");
    std::fs::copy(env!("CARGO_BIN_EXE_al-runner"), &exe).unwrap();
    let cache = d.path().join("cache");
    let mut server = Command::new(&exe);
    server
        .args(["serve", "--dep", &dep, "--no-download", "--no-test-data"])
        .env("ALRUNNER_CACHE", &cache)
        .env_remove("ALRUNNER_SEED")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    // A copy just written can be busy for a moment ("Text file busy"): another thread of this test binary that forks
    // while the copy's write handle is open keeps it open in the child until that child execs.
    let mut tries = 0;
    let mut child = loop {
        match server.spawn() {
            Err(e) if e.kind() == std::io::ErrorKind::ExecutableFileBusy && tries < 50 => {
                tries += 1;
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            spawned => break spawned.unwrap(),
        }
    };
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();
    assert!(line.contains("ready"), "{line}");
    std::fs::remove_file(&exe).unwrap();
    let mut stdin = child.stdin.take().unwrap();
    writeln!(stdin, "{}", json!({"command": "runTests", "sourcePaths": [main]})).unwrap();
    stdin.flush().unwrap();
    let mut answers = String::new();
    loop {
        line.clear();
        assert!(
            stdout.read_line(&mut line).unwrap() > 0,
            "the server closed stdout: {answers}"
        );
        answers.push_str(&line);
        if line.contains("\"type\":\"summary\"") {
            break;
        }
    }
    drop(stdin);
    child.wait().unwrap();
    assert!(answers.contains("v=41"), "{answers}");
    assert_eq!(
        layer_tags(&cache),
        own,
        "the deleted executable's server filed its layer under another build tag"
    );
}

/// Al-runner issue 8244: `coverage: "hit"` lists in `coverage` only the statements the tests ran, each as the full list
/// shows it, and `perTestCoverage` as without it. A coverage request keeps the install baseline of the server, and its
/// coverage is the same as over a baseline it ran.
#[test]
fn coverage_hit_lists_the_statements_that_ran_and_a_coverage_request_keeps_the_baseline() {
    let d = tempfile::tempdir().unwrap();
    let (main, dep) = installing_apps(d.path());
    let dep = dep.to_string_lossy().to_string();
    let cache = d.path().join("cache").to_string_lossy().to_string();
    let mut s = Server::start(&["--no-download", "--no-test-data", "--dep", &dep, "--cache-dir", &cache]);
    s.line();
    let mut run = |coverage: Value| {
        s.send(
            json!({"command": "runTests", "sourcePaths": [main], "test": "Codeunit50710.",
            "coverage": coverage, "perTestCoverage": true}),
        );
        let lines = s.until_summary();
        let summary = lines.last().unwrap().clone();
        assert_eq!(
            (summary["type"].as_str(), summary["passed"].as_u64()),
            (Some("summary"), Some(1)),
            "{lines:?}"
        );
        summary
    };
    let full = run(json!(true));
    let hit = run(json!("hit"));
    let again = run(json!(true));
    assert_eq!(full["timings"]["baselineSource"], "ran", "{full}");
    assert_eq!(hit["timings"]["baselineSource"], "kept", "{hit}");
    assert_eq!(again["timings"]["baselineSource"], "kept", "{again}");
    assert_eq!(again["coverage"], full["coverage"]);
    assert_eq!(again["perTestCoverage"], full["perTestCoverage"]);
    assert_eq!(hit["perTestCoverage"], full["perTestCoverage"]);
    // The full list less its statements without a run, and less the files left with none.
    let ran: Vec<Value> = full["coverage"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|f| {
            let statements: Vec<Value> = f["statements"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|s| s["hits"] != 0)
                .cloned()
                .collect();
            (!statements.is_empty()).then(|| json!({"file": f["file"], "statements": statements}))
        })
        .collect();
    let all = |v: &Value| {
        v.as_array()
            .unwrap()
            .iter()
            .map(|f| f["statements"].as_array().unwrap().len())
            .sum::<usize>()
    };
    assert!(
        all(&json!(ran)) > 0 && all(&json!(ran)) < all(&full["coverage"]),
        "{full}"
    );
    assert_eq!(hit["coverage"], json!(ran), "{hit}");
    s.send(json!({"command": "runTests", "sourcePaths": [main], "coverage": "all"}));
    let lines = s.until_summary();
    assert!(
        lines.last().unwrap()["error"]
            .as_str()
            .is_some_and(|e| e.contains("`coverage`")),
        "{lines:?}"
    );
    s.send(json!({"command": "shutdown"}));
    assert_eq!(s.line(), json!({"status": "shutting down"}));
}

/// Al-runner issue 8210, the reporter's repro: with `stopAfterFirstFailure` each test codeunit stops at its first failing test. The
/// rest of that entry and every later entry is not run, each with a `notRun` row whose message names the failed test.
/// Without the field every test of every entry runs, as before.
#[test]
fn stop_after_first_failure_ends_the_request_at_its_first_failing_test() {
    let d = tempfile::tempdir().unwrap();
    let main = app(
        d.path(),
        "stopafterfail",
        "codeunit 50110 \"Stop After Fail Tests\" { Subtype = Test;
            [Test] procedure FailsFirst() begin Error('the first test fails'); end;
            [Test] procedure SlowTwo() begin Burn(); end;
            [Test] procedure SlowThree() begin Burn(); end;
            [Test] procedure SlowFour() begin Burn(); end;
            local procedure Burn() var i: Integer; x: Integer; begin for i := 1 to 3000 do x := (x + i) mod 7919; end; }",
    );
    let mut s = Server::start(&["--no-download", "--no-test-data"]);
    s.line();
    let entry = |id: &str| json!({"id": id, "test": "Codeunit50110."});
    let mut rows = |stop: Option<bool>, entries: Vec<Value>| {
        let mut req = json!({"command": "runTests", "sourcePaths": [main], "entries": entries});
        if let Some(stop) = stop {
            req["stopAfterFirstFailure"] = json!(stop);
        }
        s.send(req);
        let lines = s.until_summary();
        let tests: Vec<Value> = lines.iter().filter(|l| l["type"] == "test").cloned().collect();
        (tests, lines)
    };
    let stopped = |t: &Value| {
        t["notRun"] == true
            && t["message"]
                .as_str()
                .is_some_and(|m| m.contains("Codeunit50110.FailsFirst") && m.contains("stopAfterFirstFailure"))
    };
    let (one, lines) = rows(Some(true), vec![entry("a")]);
    assert_eq!(one.len(), 4, "{lines:?}");
    assert_eq!(one[0]["status"], "fail", "{lines:?}");
    assert!(one[1..].iter().all(stopped), "{lines:?}");
    let (three, lines) = rows(Some(true), vec![entry("a"), entry("b"), entry("c")]);
    assert_eq!(three.len(), 12, "{lines:?}");
    assert_eq!(three[0]["status"], "fail", "{lines:?}");
    assert!(three[1..].iter().all(stopped), "{lines:?}");
    for stop in [None, Some(false)] {
        let (all, lines) = rows(stop, vec![entry("a"), entry("b"), entry("c")]);
        assert_eq!(all.len(), 12, "{lines:?}");
        assert_eq!(all.iter().filter(|t| t["status"] == "pass").count(), 9, "{lines:?}");
    }
    s.send(json!({"command": "shutdown"}));
    assert_eq!(s.line(), json!({"status": "shutting down"}));
}

/// Al-runner issue 8210: under `stopAfterFirstFailure` a selection runs only the tests it picks, as `runSelectedOnly`
/// does, so a failing test the answer does not list cannot stop the run.
#[test]
fn stop_after_first_failure_runs_only_the_picked_tests() {
    let d = tempfile::tempdir().unwrap();
    let main = app(
        d.path(),
        "stoppicked",
        "codeunit 50111 \"Stop Picked\" { Subtype = Test;
            [Test] procedure A_FailsUnpicked() begin Error('not picked'); end;
            [Test] procedure B_Picked() begin end;
            [Test] procedure C_Picked() begin end; }",
    );
    let mut s = Server::start(&["--no-download", "--no-test-data"]);
    s.line();
    s.send(
        json!({"command": "runTests", "sourcePaths": [main], "stopAfterFirstFailure": true,
        "entries": [{"id": "e", "test": "Codeunit50111.", "excludeTests": ["FailsUnpicked"]},
                    {"id": "f", "test": "Codeunit50111.", "excludeTests": ["FailsUnpicked"]}]}),
    );
    let lines = s.until_summary();
    let tests: Vec<&Value> = lines.iter().filter(|l| l["type"] == "test").collect();
    assert_eq!(tests.len(), 4, "{lines:?}");
    assert!(tests.iter().all(|t| t["status"] == "pass"), "{lines:?}");
    s.send(json!({"command": "shutdown"}));
    assert_eq!(s.line(), json!({"status": "shutting down"}));
}

/// Al-runner issue 8334: a test the time limit stops in the middle of its work leaves nothing for the next test codeunit,
/// the next entry or the next request: committed rows, WorkDate, a SingleInstance codeunit, NumberSequence and
/// IsolatedStorage it set are gone, and the reader's answers equal those of a new daemon.
#[test]
fn a_daemon_after_a_test_timeout_answers_as_a_new_daemon() {
    let d = tempfile::tempdir().unwrap();
    let main = app(
        d.path(),
        "timeoutstate",
        "table 50730 \"Timeout Row\" { fields { field(1; No; Integer) { } } keys { key(PK; No) { } } }
codeunit 50731 \"Timeout Single\" { SingleInstance = true; var V: Integer;
    procedure SetV(N: Integer) begin V := N; end; procedure GetV(): Integer begin exit(V); end; }
codeunit 50732 Hanger { Subtype = Test;
    [Test] procedure WritesThenHangs() var R: Record \"Timeout Row\"; S: Codeunit \"Timeout Single\"; i: Integer; begin
        R.No := 1; R.Insert(); Commit(); R.No := 2; R.Insert();
        WorkDate(20010101D); S.SetV(7);
        NumberSequence.Insert('STATE8334'); IsolatedStorage.Set('state8334', 'x');
        while true do i := (i + 1) mod 7919; end; }
codeunit 50733 Reader { Subtype = Test;
    [Test] procedure SeesNoState() var R: Record \"Timeout Row\"; S: Codeunit \"Timeout Single\"; begin
        if not R.IsEmpty() then Error('rows leaked');
        if WorkDate() = 20010101D then Error('WorkDate leaked');
        if S.GetV() <> 0 then Error('SingleInstance leaked');
        if NumberSequence.Exists('STATE8334') then Error('NumberSequence leaked');
        if IsolatedStorage.Contains('state8334') then Error('IsolatedStorage leaked'); end; }\n",
    );
    let reader = |s: &mut Server| {
        s.send(json!({"command": "runTests", "sourcePaths": [main], "test": "Codeunit50733."}));
        let lines = s.until_summary();
        let tests: Vec<Value> = lines
            .iter()
            .filter(|l| l["type"] == "test")
            .map(|t| t["status"].clone())
            .collect();
        let summary = lines.last().unwrap().clone();
        (
            tests,
            [
                summary["exitCode"].clone(),
                summary["passed"].clone(),
                summary["total"].clone(),
            ],
        )
    };
    let mut fresh = Server::start(&["--no-download", "--no-test-data"]);
    fresh.line();
    let expected = reader(&mut fresh);
    assert_eq!(expected.0, vec![json!("pass")], "{expected:?}");
    fresh.send(json!({"command": "shutdown"}));
    let mut s = Server::start(&["--no-download", "--no-test-data"]);
    s.line();
    s.send(
        json!({"command": "runTests", "sourcePaths": [main], "testTimeoutSecs": 1,
        "entries": [{"id": "h", "test": "Codeunit50732."}, {"id": "r", "test": "Codeunit50733."}]}),
    );
    let lines = s.until_summary();
    let tests: Vec<&Value> = lines.iter().filter(|l| l["type"] == "test").collect();
    assert_eq!(tests.len(), 2, "{lines:?}");
    assert!(
        tests[0]["notRun"] == true && tests[0]["message"].as_str().is_some_and(|m| m.contains("timeout")),
        "{lines:?}"
    );
    assert_eq!(tests[1]["status"], "pass", "{lines:?}");
    assert_eq!(reader(&mut s), expected);
    s.send(json!({"command": "shutdown"}));
    assert_eq!(s.line(), json!({"status": "shutting down"}));
}

/// Al-runner issue 8492: `stopAfterFirstFailure` over an app with a dependency whose install code writes a row. A later
/// request keeps the install baseline and the engines; every request starts from the install row and none of the writes
/// of an earlier one, with or without the stop.
#[test]
fn stop_after_first_failure_over_a_dependency_keeps_the_baseline_and_carries_no_writes() {
    let d = tempfile::tempdir().unwrap();
    let (main, dep) = installing_apps(d.path());
    write(
        &main,
        "src/T.al",
        "codeunit 50712 \"Seeded Stop\" { Subtype = Test;
    [Test] procedure A_Fails() begin Error('fails first'); end;
    [Test] procedure B_SeesTheInstallRowOnly() var S: Record Seeded; begin
        if S.Count() <> 1 then Error('Seeded holds %1 rows', S.Count());
        S.K := 'B'; S.Insert(); Commit(); end; }\n",
    );
    let dep = dep.to_string_lossy().to_string();
    let cache = d.path().join("cache").to_string_lossy().to_string();
    let mut s = Server::start(&["--no-download", "--no-test-data", "--dep", &dep, "--cache-dir", &cache]);
    s.line();
    let mut run = |stop: bool| {
        s.send(json!({"command": "runTests", "sourcePaths": [main], "stopAfterFirstFailure": stop}));
        let lines = s.until_summary();
        let of = |name: &str| {
            lines
                .iter()
                .find(|l| l["type"] == "test" && l["name"].as_str().is_some_and(|n| n.ends_with(name)))
                .unwrap_or_else(|| panic!("{name}: {lines:?}"))
                .clone()
        };
        let summary = lines.last().unwrap().clone();
        (of("A_Fails"), of("B_SeesTheInstallRowOnly"), summary)
    };
    let stopped = |b: &Value| {
        b["notRun"] == true
            && b["message"]
                .as_str()
                .is_some_and(|m| m.contains("A_Fails") && m.contains("stopAfterFirstFailure"))
    };
    for (stop, source) in [(true, "ran"), (true, "kept"), (false, "kept"), (true, "kept")] {
        let (a, b, summary) = run(stop);
        assert_eq!(a["status"], "fail", "{stop}: {a}");
        if stop {
            assert!(stopped(&b), "{stop}: {b}");
        } else {
            assert_eq!(b["status"], "pass", "{b}");
        }
        assert_eq!(summary["timings"]["baselineSource"], source, "{stop}: {summary}");
    }
    s.send(json!({"command": "shutdown"}));
    assert_eq!(s.line(), json!({"status": "shutting down"}));
}

/// Al-runner issue 8492: a coverage request after one without coverage gives the coverage a server's first request gives.
#[test]
fn a_coverage_request_after_one_without_coverage_gives_the_coverage_of_a_first_request() {
    let d = tempfile::tempdir().unwrap();
    let (main, dep) = installing_apps(d.path());
    let dep = dep.to_string_lossy().to_string();
    let cache = d.path().join("cache").to_string_lossy().to_string();
    let start = || {
        let mut s = Server::start(&["--no-download", "--no-test-data", "--dep", &dep, "--cache-dir", &cache]);
        s.line();
        s
    };
    let run = |s: &mut Server, coverage: Option<bool>| {
        let mut req = json!({"command": "runTests", "sourcePaths": [main], "test": "Codeunit50710."});
        if let Some(c) = coverage {
            req["coverage"] = json!(c);
            req["perTestCoverage"] = json!(c);
        }
        s.send(req);
        let lines = s.until_summary();
        let summary = lines.last().unwrap().clone();
        assert_eq!(summary["passed"], 1, "{lines:?}");
        summary
    };
    let mut first = start();
    let want = run(&mut first, Some(true));
    first.send(json!({"command": "shutdown"}));
    let mut s = start();
    let without = run(&mut s, None);
    assert!(without.get("coverage").is_none(), "{without}");
    let got = run(&mut s, Some(true));
    assert_eq!(got["coverage"], want["coverage"], "{got}");
    assert_eq!(got["perTestCoverage"], want["perTestCoverage"], "{got}");
    s.send(json!({"command": "shutdown"}));
    assert_eq!(s.line(), json!({"status": "shutting down"}));
}

/// Removes what differs between two runs of the same request on purpose, at any depth (the test lines of an `execute`
/// answer are nested): the timings, the wall time, `cached` and the fresh seed of each daemon.
fn without_times(lines: &[Value]) -> Vec<Value> {
    fn strip(v: &mut Value) {
        match v {
            Value::Object(o) => {
                for k in ["timings", "wallSeconds", "durationMs", "cached", "elapsedMs", "seed"] {
                    o.remove(k);
                }
                o.values_mut().for_each(strip);
            }
            Value::Array(a) => a.iter_mut().for_each(strip),
            _ => {}
        }
    }
    lines
        .iter()
        .map(|l| {
            let mut l = l.clone();
            strip(&mut l);
            l
        })
        .collect()
}

/// `request` sent alone to a new daemon over the cache folder `cache`: its answer, to the summary (or the one line of an
/// `execute`, whose answer has no timings), and what its `--timings` line says the workspace layer did ("read",
/// "built", "off" or "none" without the line).
fn answer_of_a_new_daemon(cache: &Path, request: &Value) -> (Vec<Value>, String) {
    let log = cache.with_extension("stderr.txt");
    let dir = cache.to_string_lossy().to_string();
    let err = std::fs::File::create(&log).unwrap();
    let mut s = Server::start_with_stderr(
        &["--no-download", "--no-test-data", "--timings", "--cache-dir", &dir],
        &[],
        Stdio::from(err),
    );
    assert_eq!(s.line(), json!({"ready": true}));
    s.send(request.clone());
    let lines = if request["command"] == "execute" {
        vec![s.line()]
    } else {
        s.until_summary()
    };
    s.send(json!({"command": "shutdown"}));
    s.line();
    s.child.wait().unwrap();
    let said = std::fs::read_to_string(&log).unwrap();
    let state = said
        .lines()
        .find_map(|l| l.strip_prefix("compile cache: workspace layer "))
        .and_then(|rest| rest.split(|c: char| !c.is_alphabetic()).next())
        .unwrap_or("none")
        .to_string();
    if let Some(json_state) = lines.last().unwrap()["timings"].get("workspaceCache") {
        assert_eq!(json_state, &json!(state), "the JSON and the --timings line: {said}");
    }
    (lines, state)
}

/// The apps of two `sourcePaths`: an app and its test app (issue 8143's shape).
fn lib_and_tests(root: &Path) -> (PathBuf, PathBuf) {
    let lib = root.join("lib");
    write(
        &lib,
        "app.json",
        r#"{"id":"22222222-0000-0000-0000-000000000001","name":"Lib","publisher":"Test","version":"1.0.0.0","dependencies":[]}"#,
    );
    write(
        &lib,
        "src/Lib.al",
        "codeunit 50300 Lib { procedure Twice(X: Integer): Integer begin exit(X * 2); end; }\n",
    );
    let tests = root.join("tests");
    write(
        &tests,
        "app.json",
        r#"{"id":"22222222-0000-0000-0000-000000000002","name":"LibTests","publisher":"Test","version":"1.0.0.0","dependencies":[{"id":"22222222-0000-0000-0000-000000000001","name":"Lib","publisher":"Test","version":"1.0.0.0"}]}"#,
    );
    write(
        &tests,
        "src/T.al",
        &MIXED.replace("Error('boom %1', 5);", "if 0 = 1 then; Error('boom %1', 5);"),
    );
    (lib, tests)
}

/// ADR-022 amendment 2026-11-15, "Equality requirement", serve suite (issue 8143): each request shape of this suite
/// that compiles sources, sent to a fresh daemon (no entry yet) and then to a new daemon over the same cache folder. The
/// second daemon reads the entry the first one built (`workspaceCache`: `built`, then `read`; `cached` stays false, as
/// it means this daemon's kept compile), and the two answers are the same but for the timings.
#[test]
fn each_request_shape_answers_the_same_on_a_fresh_and_on_a_reading_daemon() {
    let d = tempfile::tempdir().unwrap();
    let mixed = app(d.path(), "mixed", MIXED);
    let (lib, tests) = lib_and_tests(d.path());
    let tdd = tdd_app(d.path());
    let scratch = app(d.path(), "scratch", SCRATCH);
    let broken = app(
        d.path(),
        "broken",
        "codeunit 50990 Broken { Subtype = Test; [Test] procedure P() begin X := ; end; }\n",
    );
    let run = |more: Value| {
        let mut r = json!({"command": "runTests", "sourcePaths": [mixed]});
        for (k, v) in more.as_object().unwrap() {
            r[k] = v.clone();
        }
        r
    };
    let requests = [
        run(json!({})),
        run(json!({"test": "Codeunit50100.Passes"})),
        run(json!({"test": "Codeunit50100.Passes", "excludeTests": ["Codeunit50100.Fails"]})),
        run(json!({"testIsolation": "test"})),
        run(json!({"testRunner": "130450"})),
        run(json!({"stopAfterFirstFailure": true})),
        run(json!({"coverage": true, "perTestCoverage": true})),
        run(json!({"entries": [
            {"id": "e1", "excludeTestsExact": ["Passes"]},
            {"id": "e2", "excludeTestsExact": ["Fails"]},
        ]})),
        json!({"command": "runTests", "sourcePaths": [lib, tests]}),
        json!({"command": "runTests", "sourcePaths": [tdd], "tdd": true}),
        json!({"command": "runTests", "sourcePaths": [broken]}),
        json!({"command": "execute", "sourcePaths": [scratch]}),
    ];
    let mut states = Vec::new();
    for (n, request) in requests.iter().enumerate() {
        let cache = d.path().join(format!("cache-{n}"));
        let (fresh, built) = answer_of_a_new_daemon(&cache, request);
        let (read, reading) = answer_of_a_new_daemon(&cache, request);
        states.push((built, reading));
        assert_ne!(read.last().unwrap()["cached"], json!(true), "{request}");
        assert_eq!(without_times(&fresh), without_times(&read), "{request}");
    }
    let built_then_read = ("built".to_string(), "read".to_string());
    assert_eq!(
        states,
        vec![built_then_read; requests.len()],
        "per request: {requests:?}"
    );
}

/// The `--timings` line of the workspace layer is said for a request that compiled, as its JSON `workspaceCache` is: a
/// request on the kept compile says none (it did not compile, so the layer did nothing for it).
#[test]
fn a_request_on_the_kept_compile_says_no_workspace_layer_line() {
    let d = tempfile::tempdir().unwrap();
    let a = app(d.path(), "kept", MIXED);
    let cache = d.path().join("cache").to_string_lossy().to_string();
    let log = d.path().join("stderr.txt");
    let err = std::fs::File::create(&log).unwrap();
    let mut s = Server::start_with_stderr(
        &["--no-download", "--no-test-data", "--timings", "--cache-dir", &cache],
        &[],
        Stdio::from(err),
    );
    assert_eq!(s.line(), json!({"ready": true}));
    let request = json!({"command": "runTests", "sourcePaths": [a]});
    let mut said = Vec::new();
    for _ in 0..2 {
        s.send(request.clone());
        let summary = s.until_summary().last().unwrap().clone();
        let lines = std::fs::read_to_string(&log).unwrap();
        let count = lines
            .lines()
            .filter(|l| l.starts_with("compile cache: workspace layer"))
            .count();
        said.push((
            summary["cached"].clone(),
            summary["timings"].get("workspaceCache").cloned(),
            count,
        ));
    }
    s.send(json!({"command": "shutdown"}));
    s.line();
    s.child.wait().unwrap();
    assert_eq!(
        said,
        vec![(json!(false), Some(json!("built")), 1), (json!(true), None, 1)],
        "(cached, workspaceCache, workspace layer lines on stderr so far) per request"
    );
}

/// ADR-022 amendment 2026-11-15: a one-shot `al-runner test` and a daemon over the same workspace and cache folder share
/// the workspace layer. The first one-shot run builds the entry (written before the process ends), the second reads it,
/// and so does a daemon started after them; `ALRUNNER_NO_WORKSPACE_CACHE=1` turns the layer off.
#[test]
fn a_one_shot_run_and_a_daemon_share_the_workspace_layer() {
    let d = tempfile::tempdir().unwrap();
    let a = app(d.path(), "mixed", MIXED);
    let cache = d.path().join("cache").to_string_lossy().to_string();
    let one_shot = |env: &[(&str, &str)]| {
        let out = Command::new(env!("CARGO_BIN_EXE_al-runner"))
            .args([
                "test",
                a.to_str().unwrap(),
                "--no-download",
                "--no-test-data",
                "--timings",
            ])
            .args(["--cache-dir", &cache])
            .envs(env.iter().copied())
            .output()
            .unwrap();
        let err = String::from_utf8_lossy(&out.stderr).to_string();
        let line = err
            .lines()
            .find(|l| l.starts_with("compile cache: workspace layer"))
            .unwrap_or_default()
            .to_string();
        (out.status.code(), line)
    };
    let (code, first) = one_shot(&[]);
    assert!(first.contains("workspace layer built"), "{first}");
    let (code2, second) = one_shot(&[]);
    assert_eq!(code, code2);
    assert!(second.contains("workspace layer read"), "{second}");
    let (_, off) = one_shot(&[("ALRUNNER_NO_WORKSPACE_CACHE", "1")]);
    assert!(off.contains("workspace layer off"), "{off}");
    let mut s = Server::start(&["--no-download", "--no-test-data", "--cache-dir", &cache]);
    assert_eq!(s.line(), json!({"ready": true}));
    s.send(json!({"command": "runTests", "sourcePaths": [a]}));
    let summary = s.until_summary().last().unwrap().clone();
    assert_eq!(summary["timings"]["workspaceCache"], json!("read"), "{summary}");
    // A request on the kept compile used no cache: no `workspaceCache`.
    s.send(json!({"command": "runTests", "sourcePaths": [a]}));
    let summary = s.until_summary().last().unwrap().clone();
    assert_eq!(summary["cached"], json!(true), "{summary}");
    assert!(summary["timings"].get("workspaceCache").is_none(), "{summary}");
}

/// The page of review probe app9 (HIGH 1): a pageextension sets the base page's protected variable. Whether that runs
/// depends on `AL_UNPINNED_BASE_ACCESS`, which an empty value sets to "pin nothing".
const BASE_ACCESS: &str = r#"
page 50500 "P9 Base"
{
    PageType = Card;
    layout { area(Content) { field(SharedCtl; Shared) { } } }
    protected var
        Shared: Integer;
}

pageextension 50501 "P9 Ext" extends "P9 Base"
{
    trigger OnOpenPage()
    begin
        Shared := 7;
    end;
}

codeunit 50510 "P9 Tests"
{
    Subtype = Test;

    [Test]
    procedure ExtensionSetsTheBaseVariable()
    var
        P: TestPage "P9 Base";
    begin
        P.OpenView();
        if P.SharedCtl.AsInteger() <> 7 then
            Error('shared is %1', P.SharedCtl.AsInteger());
    end;
}
"#;

/// One-shot `al-runner test` of `app` over the cache folder `cache` with `env`: (exit code, the `--timings` line of the
/// workspace layer, stderr).
fn one_shot_run(app: &Path, cache: &str, env: &[(&str, &str)]) -> (Option<i32>, String, String) {
    one_shot_run_with(app, cache, env, &[])
}

/// [`one_shot_run`] with more arguments.
fn one_shot_run_with(app: &Path, cache: &str, env: &[(&str, &str)], args: &[&str]) -> (Option<i32>, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_al-runner"))
        .args([
            "test",
            app.to_str().unwrap(),
            "--no-download",
            "--no-test-data",
            "--timings",
        ])
        .args(["--cache-dir", cache])
        .args(args)
        .env_remove("AL_UNPINNED_BASE_ACCESS")
        .envs(env.iter().copied())
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&out.stderr).to_string();
    let line = err
        .lines()
        .find(|l| l.starts_with("compile cache: workspace layer"))
        .unwrap_or_default()
        .to_string();
    (out.status.code(), line, err)
}

/// Review HIGH 1: a run with `AL_UNPINNED_BASE_ACCESS` set but empty and a normal run never read each other's entry
/// (without dependency layers nothing else in the key tells them apart).
#[test]
fn a_switch_set_but_empty_and_unset_do_not_share_a_workspace_entry() {
    for first_empty in [false, true] {
        let d = tempfile::tempdir().unwrap();
        let a = app(d.path(), "p9", BASE_ACCESS);
        let cache = d.path().join("cache").to_string_lossy().to_string();
        let empty: &[(&str, &str)] = &[("AL_UNPINNED_BASE_ACCESS", "")];
        let (first, second) = if first_empty {
            (empty, &[][..])
        } else {
            (&[][..], empty)
        };
        let (code1, line1, _) = one_shot_run(&a, &cache, first);
        let (code2, line2, _) = one_shot_run(&a, &cache, second);
        let fresh = d.path().join("fresh").to_string_lossy().to_string();
        let (code2_fresh, _, _) = one_shot_run(&a, &fresh, second);
        assert!(line1.contains("built"), "{line1}");
        assert!(
            line2.contains("built"),
            "empty first {first_empty}: the second run read the first's entry: {line2}"
        );
        assert_eq!(code2, code2_fresh, "empty first {first_empty}: {code1:?}");
    }
}

/// ADR-022 amendment decision 10: a damaged entry is removed and the run compiles fresh; the reason goes to stderr with
/// `--verbose` only (the `--timings` line of the layer names it in its notes either way).
#[test]
fn a_damaged_workspace_entry_is_said_on_stderr_with_verbose_only() {
    let d = tempfile::tempdir().unwrap();
    let a = app(d.path(), "damaged", MIXED);
    let cache = d.path().join("cache").to_string_lossy().to_string();
    let damage = || {
        let dir = d.path().join("cache").join("compile");
        let entry = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .find(|p| p.extension().is_some_and(|x| x == "alcw"))
            .expect("an entry was written");
        let mut bytes = std::fs::read(&entry).unwrap();
        let at = bytes.len() / 2;
        bytes[at] ^= 0x41;
        std::fs::write(&entry, bytes).unwrap();
    };
    let said = |err: &str| {
        err.lines()
            .any(|l| l.starts_with("workspace cache: ") && l.contains("ignored and removed"))
    };
    let (code, line, _) = one_shot_run(&a, &cache, &[]);
    assert!(line.contains("built"), "{line}");
    for verbose in [false, true] {
        damage();
        let args: &[&str] = if verbose { &["--verbose"] } else { &[] };
        let (again, line, err) = one_shot_run_with(&a, &cache, &[], args);
        assert_eq!(again, code, "verbose {verbose}: {err}");
        assert!(
            line.contains("built") && line.contains("ignored and removed"),
            "verbose {verbose}: {line}"
        );
        assert_eq!(said(&err), verbose, "verbose {verbose}: {err}");
    }
}

/// Review HIGH 2 (probe app8): a dependency file that does not parse is said on stderr (`parse: ...`) by a run that reads
/// the workspace layer as by the run that built it, for the compile on the dependency layers and for the compile of
/// selected tests free of data.
#[test]
fn a_read_says_which_dependency_files_did_not_parse() {
    let d = tempfile::tempdir().unwrap();
    let dep = d.path().join("dep");
    write(
        &dep,
        "app.json",
        r#"{"id":"88888888-0000-0000-0000-000000000001","name":"P8Dep","publisher":"T","version":"1.0.0.0","dependencies":[]}"#,
    );
    write(
        &dep,
        "src/good.al",
        "codeunit 50900 \"P8 Dep Good\"\n{\n    procedure One(): Integer\n    begin\n        exit(1);\n    end;\n}\n",
    );
    write(
        &dep,
        "src/bad.al",
        "codeunit 50901 \"P8 Dep Bad\"\n{\n    procedure Two(): Integer\n    begin\n        exit(2 +);\n    end\n}\n",
    );
    let ws = d.path().join("ws");
    write(
        &ws,
        "app.json",
        r#"{"id":"88888888-0000-0000-0000-000000000002","name":"P8Ws","publisher":"T","version":"1.0.0.0","dependencies":[{"id":"88888888-0000-0000-0000-000000000001","name":"P8Dep","publisher":"T","version":"1.0.0.0"}]}"#,
    );
    write(
        &ws,
        "src/t.al",
        "codeunit 50910 \"P8 Tests\"\n{\n    Subtype = Test;\n\n    [Test]\n    procedure GoodWorks()\n    var\n        G: Codeunit \"P8 Dep Good\";\n    begin\n        if G.One() <> 1 then\n            Error('wrong');\n    end;\n\n    [Test]\n    procedure BadIsRefused()\n    var\n        B: Codeunit \"P8 Dep Bad\";\n    begin\n        if B.Two() <> 2 then\n            Error('wrong');\n    end;\n}\n",
    );
    for extra in [
        &[][..],
        &["--test-runner", "builtin"][..],
        &["--test-runner", "builtin", "--codeunit", "50910"][..],
    ] {
        let cache = d
            .path()
            .join(format!("cache{}", extra.len()))
            .to_string_lossy()
            .to_string();
        let run = || {
            let out = Command::new(env!("CARGO_BIN_EXE_al-runner"))
                .args(["test", ws.to_str().unwrap(), "--dep", dep.to_str().unwrap()])
                .args(["--no-download", "--no-test-data", "--timings", "--cache-dir", &cache])
                .args(extra)
                .output()
                .unwrap();
            let err = String::from_utf8_lossy(&out.stderr).to_string();
            let parse: Vec<String> = err
                .lines()
                .filter(|l| l.starts_with("parse:"))
                .map(String::from)
                .collect();
            let state = err
                .lines()
                .find(|l| l.starts_with("compile cache: workspace layer"))
                .unwrap_or_default()
                .to_string();
            (out.status.code(), parse, state)
        };
        let (code1, parse1, state1) = run();
        let (code2, parse2, state2) = run();
        assert!(
            state1.contains("built") && state2.contains("read"),
            "{extra:?}: {state1} / {state2}"
        );
        assert!(parse1.iter().any(|l| l.contains("bad.al:5")), "{extra:?}: {parse1:?}");
        assert_eq!((code1, &parse1), (code2, &parse2), "{extra:?}");
    }
}

/// Second review MED A: `ALRUNNER_CHECK_INCREMENTAL=1` checks an incremental compile against a complete compile, which
/// must be a fresh compile: it neither reads the workspace layer nor writes an entry.
#[test]
fn the_check_of_an_incremental_compile_does_not_use_the_workspace_layer() {
    let d = tempfile::tempdir().unwrap();
    let dep = d.path().join("dep");
    write(
        &dep,
        "app.json",
        r#"{"id":"66666666-0000-0000-0000-000000000001","name":"ChkDep","publisher":"T","version":"1.0.0.0","dependencies":[]}"#,
    );
    write(
        &dep,
        "src/dep.al",
        "table 50601 \"Chk T\" { fields { field(1; K; Integer) { } } keys { key(PK; K) { } } }\ncodeunit 50600 \"Chk Dep\" { procedure One(): Integer begin exit(1); end; }\n",
    );
    let ws = d.path().join("ws");
    write(
        &ws,
        "app.json",
        r#"{"id":"66666666-0000-0000-0000-000000000002","name":"ChkWs","publisher":"T","version":"1.0.0.0","dependencies":[{"id":"66666666-0000-0000-0000-000000000001","name":"ChkDep","publisher":"T","version":"1.0.0.0"}]}"#,
    );
    let lib = "codeunit 50610 \"Chk Lib\" { procedure Twice(X: Integer): Integer var D: Codeunit \"Chk Dep\"; begin exit(X * 2 * D.One()); end; }\n";
    write(&ws, "src/lib.al", lib);
    write(
        &ws,
        "src/tests.al",
        "codeunit 50620 \"Chk Tests\" { Subtype = Test; [Test] procedure Twice() var L: Codeunit \"Chk Lib\"; R: Record \"Chk T\"; begin R.DeleteAll(); if L.Twice(2) <> 4 then Error('twice'); end; }\n",
    );
    let cache = d.path().join("cache");
    let dep_arg = dep.to_string_lossy().to_string();
    let cache_arg = cache.to_string_lossy().to_string();
    let mut s = Server::start_with(
        &[
            "--no-download",
            "--no-test-data",
            "--dep",
            &dep_arg,
            "--cache-dir",
            &cache_arg,
        ],
        &[("ALRUNNER_CHECK_INCREMENTAL", "1")],
    );
    assert_eq!(s.line(), json!({"ready": true}));
    let request = json!({"command": "runTests", "sourcePaths": [ws]});
    s.send(request.clone());
    let first = s.until_summary().last().unwrap().clone();
    assert_eq!(first["timings"]["workspaceCache"], json!("built"), "{first}");
    write(
        &ws,
        "src/lib.al",
        &lib.replace("exit(X * 2 * D.One())", "exit(X * 3 * D.One())"),
    );
    s.send(request);
    let edited = s.until_summary().last().unwrap().clone();
    assert_eq!(edited["incremental"], json!(true), "{edited}");
    s.send(json!({"command": "shutdown"}));
    s.line();
    s.child.wait().unwrap();
    let entries: Vec<_> = std::fs::read_dir(cache.join("compile"))
        .unwrap()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "alcw"))
        .map(|e| e.file_name())
        .collect();
    assert_eq!(
        entries.len(),
        1,
        "the check's complete compile wrote an entry: {entries:?}"
    );
}

/// A stand-in of Microsoft's login in a dependency app, in the shape of its code (al-testrun `runtime/login_tests.rs`):
/// Company Triggers `OnCompanyOpenCompleted` raises `OnAfterLogin`, whose subscriber raises the registration events.
const LOGIN_STAND_IN: &str = r#"
table 1990 "Guided Experience Item" { fields { field(1; "Object ID"; Integer) { } } keys { key(PK; "Object ID") { Clustered = true; } } }
table 1996 "Checklist Setup" { fields { field(1; K; Code[10]) { } } keys { key(PK; K) { Clustered = true; } } }
codeunit 150 "System Initialization" { [IntegrationEvent(false, false, true)] internal procedure OnAfterLogin() begin end; }
codeunit 151 "System Initialization Impl."
{
    SingleInstance = true;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Company Triggers", 'OnCompanyOpenCompleted', '', false, false)]
    local procedure InitCompany() var SystemInitialization: Codeunit "System Initialization"; begin SystemInitialization.OnAfterLogin(); end;
}
codeunit 1990 "Guided Experience"
{
    procedure InsertAssistedSetup(ObjectID: Integer) var G: Record "Guided Experience Item"; begin G."Object ID" := ObjectID; G.Insert(); end;
    procedure Register() begin OnRegisterManualSetup(); end;
    [IntegrationEvent(false, false)] local procedure OnRegisterManualSetup() begin end;
}
codeunit 1997 "Checklist Events"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure OnAfterLogIn() var S: Record "Checklist Setup"; G: Codeunit "Guided Experience";
    begin
        if S.Get('') then exit;
        G.Register();
        S.Insert();
    end;
}
"#;

/// The login stand-in app and an app under test whose login subscriber registers a row that depends on its resource
/// `rpl.txt`: the apps of the login replay check's serve tests. Returns the stand-in's folder and the app's folder.
fn login_probe(d: &Path) -> (PathBuf, PathBuf) {
    let dep = d.join("login");
    write(
        &dep,
        "app.json",
        r#"{"id":"82110000-0000-0000-0000-000000000001","name":"Login Stand-in","publisher":"Someone","version":"1.0.0.0","dependencies":[]}"#,
    );
    write(&dep, "src/Login.al", LOGIN_STAND_IN);
    let main = d.join("main");
    write(
        &main,
        "app.json",
        r#"{"id":"82110000-0000-0000-0000-000000000002","name":"Login Probe","publisher":"Test","version":"1.0.0.0","resourceFolders":["Res"],"dependencies":[{"id":"82110000-0000-0000-0000-000000000001","name":"Login Stand-in","publisher":"Someone","version":"1.0.0.0"}]}"#,
    );
    write(&main, "Res/rpl.txt", "a");
    write(
        &main,
        "src/T.al",
        r#"codeunit 50820 "Probe Registration"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Guided Experience", 'OnRegisterManualSetup', '', false, false)]
    local procedure RegisterManualSetup() var G: Codeunit "Guided Experience";
    begin G.InsertAssistedSetup(50800 + StrLen(NavApp.GetResourceAsText('rpl.txt'))); end;
}
codeunit 50821 "Probe Login Tests"
{
    Subtype = Test;
    [Test] procedure SeesTheRow() var I: Record "Guided Experience Item"; begin if I.IsEmpty() then Error('no row'); end;
}
"#,
    );
    (dep, main)
}

/// Al-runner issue 8211: `ALRUNNER_LOGIN_REPLAY_CHECK=1` compares what each test session holds after the raised login with
/// the first session of the same inputs, across the entries of a request (their `resources`, which the login reads here)
/// and the requests of one kept daemon (another `seed`). No session of one key holds anything else.
#[test]
fn the_login_replay_check_finds_no_difference_across_entries_and_requests() {
    let d = tempfile::tempdir().unwrap();
    let (dep, main) = login_probe(d.path());
    let log = d.path().join("stderr.txt");
    let dep_arg = dep.to_string_lossy().to_string();
    let mut s = Server::start_with_stderr(
        &["--no-download", "--no-test-data", "--dep", &dep_arg],
        &[("ALRUNNER_LOGIN_REPLAY_CHECK", "1")],
        Stdio::from(std::fs::File::create(&log).unwrap()),
    );
    s.line();
    let entries = json!([
        {"id": "a", "resources": {"rpl.txt": "a"}, "test": "Codeunit50821."},
        {"id": "b", "resources": {"rpl.txt": "abcd"}, "test": "Codeunit50821."},
        {"id": "c", "resources": {"rpl.txt": "a"}, "test": "Codeunit50821."},
    ]);
    for seed in [1, 2] {
        s.send(json!({"command": "runTests", "sourcePaths": [main], "entries": entries, "seed": seed}));
        let lines = s.until_summary();
        assert_eq!(lines.last().unwrap()["passed"], json!(3), "{lines:?}");
    }
    drop(s.stdin.take());
    s.child.wait().unwrap();
    let text = std::fs::read_to_string(&log).unwrap();
    let checks: Vec<&str> = text.lines().filter(|l| l.starts_with("login replay check: ")).collect();
    assert_eq!(checks.len(), 2, "one line per request: {text}");
    // Per request: entries a and c are one key (c compared with a), entry b (`abcd`) another. The second request's seed
    // makes two more keys: the seed moves the clock the login reads (without the seed in the key: "3 differ", the clock).
    assert_eq!(
        checks[0],
        "login replay check: 3 sessions over 2 keys (a key seen before: 1 sessions, 33%), 1 compared, 0 differ; across test codeunits: 0 compared, 0 differ"
    );
    assert_eq!(
        checks[1],
        "login replay check: 6 sessions over 4 keys (a key seen before: 2 sessions, 33%), 2 compared, 0 differ; across test codeunits: 0 compared, 0 differ"
    );
}

/// Al-runner issue 8211: under the same check each new key's session is frozen into a session image, thawn into a
/// twin and compared (`login image:` line, after the check line). Over the same entries and requests as the check's test
/// above: every key is imaged, every round trip equals, and an image is made once per key, so a key seen before adds none.
#[test]
fn the_login_image_round_trips_across_entries_and_requests() {
    let d = tempfile::tempdir().unwrap();
    let (dep, main) = login_probe(d.path());
    let log = d.path().join("stderr.txt");
    let dep_arg = dep.to_string_lossy().to_string();
    let mut s = Server::start_with_stderr(
        &["--no-download", "--no-test-data", "--dep", &dep_arg],
        &[("ALRUNNER_LOGIN_REPLAY_CHECK", "1")],
        Stdio::from(std::fs::File::create(&log).unwrap()),
    );
    s.line();
    let entries = json!([
        {"id": "a", "resources": {"rpl.txt": "a"}, "test": "Codeunit50821."},
        {"id": "b", "resources": {"rpl.txt": "abcd"}, "test": "Codeunit50821."},
        {"id": "c", "resources": {"rpl.txt": "a"}, "test": "Codeunit50821."},
    ]);
    for seed in [1, 2] {
        s.send(json!({"command": "runTests", "sourcePaths": [main], "entries": entries, "seed": seed}));
        let lines = s.until_summary();
        assert_eq!(lines.last().unwrap()["passed"], json!(3), "{lines:?}");
    }
    // Stdin stays open until the last answer.
    drop(s.stdin.take());
    s.child.wait().unwrap();
    let text = std::fs::read_to_string(&log).unwrap();
    let images: Vec<&str> = text.lines().filter(|l| l.starts_with("login image: ")).collect();
    assert_eq!(images.len(), 2, "one line per request: {text}");
    assert!(
        images[0].starts_with("login image: 2 keys, 2 imaged, 0 not imaged; round trips 2, 0 differ"),
        "{}",
        images[0]
    );
    assert!(
        images[1].starts_with("login image: 4 keys, 4 imaged, 0 not imaged; round trips 4, 0 differ"),
        "{}",
        images[1]
    );
}

/// What a login start must not change about a test line: the entry, the test and its verdict (not the timings).
fn verdicts(lines: &[Value]) -> Vec<(Value, Value, Value, Value)> {
    let mut v: Vec<_> = (lines.iter().filter(|l| l["type"] == "test"))
        .map(|t| {
            (
                t["id"].clone(),
                t["name"].clone(),
                t["status"].clone(),
                t["message"].clone(),
            )
        })
        .collect();
    v.sort_by_key(|t| format!("{t:?}"));
    v
}

/// The `loginReplays` of a request's summary (sessions that took a login start in place of their login), if it has one.
fn login_replays(lines: &[Value]) -> Option<u64> {
    lines.last().and_then(|s| s["timings"]["loginReplays"].as_u64())
}

/// Sends the entries of the login probe `rounds` times to one daemon (one seed, so one key per entry whatever the round)
/// started with `env`; each round's lines up to its summary. `extra`: more entries of the first round only.
fn login_probe_rounds(
    env: &[(&str, &str)],
    rounds: usize,
    first_round_extra: &[Value],
    timeout: bool,
) -> Vec<Vec<Value>> {
    let d = tempfile::tempdir().unwrap();
    let (dep, main) = login_probe(d.path());
    write(
        &main,
        "src/Hang.al",
        r#"codeunit 50822 "Probe Hanger"
{
    Subtype = Test;
    [Test] procedure Hangs() var i: Integer; begin while true do i := (i + 1) mod 7919; end;
}
"#,
    );
    let dep_arg = dep.to_string_lossy().to_string();
    let mut s = Server::start_with(&["--no-download", "--no-test-data", "--dep", &dep_arg], env);
    s.line();
    let probe = json!([
        {"id": "a", "resources": {"rpl.txt": "a"}, "test": "Codeunit50821."},
        {"id": "b", "resources": {"rpl.txt": "abcd"}, "test": "Codeunit50821."},
        {"id": "c", "resources": {"rpl.txt": "a"}, "test": "Codeunit50821."},
    ]);
    let mut out = Vec::new();
    for round in 0..rounds {
        let mut entries = probe.as_array().unwrap().clone();
        if round == 0 {
            entries.extend(first_round_extra.iter().cloned());
        }
        let mut request = json!({"command": "runTests", "sourcePaths": [main], "entries": entries, "seed": 7});
        if timeout && round == 0 {
            request["testTimeoutSecs"] = json!(1);
        }
        s.send(request);
        out.push(s.until_summary());
    }
    s.send(json!({"command": "shutdown"}));
    assert_eq!(s.line(), json!({"status": "shutting down"}));
    out
}

/// Al-runner issue 8211: a session whose login inputs another session of the daemon had takes what that login left
/// in place of running it. The second request (the same seed, so the same keys) takes a start for each of its three
/// entries (`timings.loginReplays`), and every test line equals those of the same requests with the replay off.
#[test]
fn a_second_request_takes_the_login_starts_of_the_first() {
    let on = login_probe_rounds(&[("ALRUNNER_LOGIN_REPLAY", "1")], 2, &[], false);
    let off = login_probe_rounds(&[("ALRUNNER_LOGIN_REPLAY", "0")], 2, &[], false);
    for round in 0..2 {
        assert_eq!(on[round].last().unwrap()["passed"], json!(3), "{:?}", on[round]);
        assert_eq!(verdicts(&on[round]), verdicts(&off[round]), "round {round}");
    }
    assert!(
        login_replays(&on[1]).is_some_and(|n| n >= 3),
        "the second request takes a start per entry: {:?}",
        on[1].last()
    );
}

/// Al-runner issue 8211, the start check per request: with the replay on, a request whose `maxMemory` is not above
/// `ALRUNNER_LOGIN_REPLAY_MAX_MB` is an error that names both values, and runs nothing. The kept engine and its starts
/// stay as they were (none is cut for the smaller limit): the next request that may run takes them.
#[test]
fn a_request_whose_max_memory_is_not_above_the_byte_cap_is_an_error_and_the_kept_starts_stay() {
    let d = tempfile::tempdir().unwrap();
    let (dep, main) = login_probe(d.path());
    let dep_arg = dep.to_string_lossy().to_string();
    let env = [("ALRUNNER_LOGIN_REPLAY", "1"), ("ALRUNNER_LOGIN_REPLAY_MAX_MB", "16")];
    let mut s = Server::start_with(&["--no-download", "--no-test-data", "--dep", &dep_arg], &env);
    s.line();
    let request = |max_memory: Option<&str>| {
        let entries = json!([
            {"id": "a", "resources": {"rpl.txt": "a"}, "test": "Codeunit50821."},
            {"id": "b", "resources": {"rpl.txt": "abcd"}, "test": "Codeunit50821."},
        ]);
        let mut r = json!({"command": "runTests", "sourcePaths": [main], "entries": entries, "seed": 7});
        if let Some(m) = max_memory {
            r["maxMemory"] = json!(m);
        }
        r
    };
    s.send(request(None));
    let first = s.until_summary();
    assert_eq!(first.last().unwrap()["passed"], json!(2), "{first:?}");
    let before = login_replays(&first).expect("loginReplays");
    for limit in ["16M", "8M"] {
        s.send(request(Some(limit)));
        let refused = s.until_summary();
        let error = refused.last().unwrap()["error"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        assert!(
            error.contains(&format!(
                "ALRUNNER_LOGIN_REPLAY_MAX_MB is 16 MB, not below the memory limit of {} MB",
                limit.trim_end_matches('M')
            )),
            "{refused:?}"
        );
        assert!(!refused.iter().any(|l| l["type"] == "test"), "nothing ran: {refused:?}");
    }
    s.send(request(Some("6G")));
    let after = s.until_summary();
    assert_eq!(after.last().unwrap()["passed"], json!(2), "{after:?}");
    assert!(
        login_replays(&after).is_some_and(|n| n >= before + 2),
        "the starts of the first request were kept: {before} then {:?}",
        after.last()
    );
    s.send(json!({"command": "shutdown"}));
    assert_eq!(s.line(), json!({"status": "shutting down"}));
}

/// Al-runner issue 9174: a bad `ALRUNNER_LOGIN_REPLAY_MAX_IMAGES` or `ALRUNNER_LOGIN_REPLAY_MAX_MB` stops the server
/// when it starts, before its `ready` line, with the error of a one-shot run and exit code 2, also with the replay off.
/// The environment does not change while the server runs, so no request would get past it.
#[test]
fn a_bad_login_replay_cap_stops_the_server_at_start() {
    for env in [
        &[("ALRUNNER_LOGIN_REPLAY", "1"), ("ALRUNNER_LOGIN_REPLAY_MAX_MB", "abc")][..],
        &[("ALRUNNER_LOGIN_REPLAY_MAX_IMAGES", "0")][..],
    ] {
        let o = Command::new(env!("CARGO_BIN_EXE_al-runner"))
            .args(["serve", "--no-download"])
            .envs(env.iter().copied())
            .stdin(Stdio::null())
            .output()
            .expect("start al-runner serve");
        let (stdout, stderr) = (String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
        let (name, bad) = env.last().unwrap();
        assert_eq!(o.status.code(), Some(2), "{env:?}: {stdout}{stderr}");
        assert!(
            stderr.contains(&format!("error: {name} takes a whole number from 1 to "))
                && stderr.contains(&format!("not `{bad}`")),
            "{env:?}: {stderr}"
        );
        assert!(!stdout.contains("ready"), "the server never starts: {stdout}");
    }
}

/// Al-runner issue 8211: `ALRUNNER_LOGIN_REPLAY=0` turns the replay off: no session takes a start, and the field
/// says so (0, not absent).
#[test]
fn with_the_replay_off_no_session_takes_a_start() {
    let off = login_probe_rounds(&[("ALRUNNER_LOGIN_REPLAY", "0")], 2, &[], false);
    for (round, lines) in off.iter().enumerate() {
        assert_eq!(lines.last().unwrap()["passed"], json!(3), "{lines:?}");
        assert_eq!(login_replays(lines), Some(0), "round {round}: {:?}", lines.last());
    }
}

/// The replay is off unless `ALRUNNER_LOGIN_REPLAY=1` turns it on. On a store a session does not start
/// from a shared snapshot of (install code that runs in every session), the key's digest of every row costs more than the
/// Login it saves, so it is opt-in until that cost is gone.
#[test]
fn the_replay_is_off_unless_turned_on() {
    let default = login_probe_rounds(&[], 2, &[], false);
    for (round, lines) in default.iter().enumerate() {
        assert_eq!(login_replays(lines), Some(0), "round {round}: {:?}", lines.last());
    }
}

/// Al-runner issue 8211: a kept daemon whose first request had a test the time limit stopped answers the next
/// request as a new daemon does (the same verdicts), and takes the starts of the first request (the stopped test's session
/// recorded none that could change the answer).
#[test]
fn a_kept_daemon_with_login_starts_answers_as_a_fresh_one() {
    let hanger = json!({"id": "h", "test": "Codeunit50822."});
    let kept = login_probe_rounds(&[("ALRUNNER_LOGIN_REPLAY", "1")], 2, &[hanger], true);
    let h = kept[0]
        .iter()
        .find(|l| l["type"] == "test" && l["name"].as_str().is_some_and(|n| n.contains("Hangs")));
    assert!(
        h.is_some_and(|t| t["notRun"] == true),
        "the hanger is stopped by the time limit: {:?}",
        kept[0]
    );
    let fresh = login_probe_rounds(&[("ALRUNNER_LOGIN_REPLAY", "1")], 1, &[], false);
    assert_eq!(verdicts(&kept[1]), verdicts(&fresh[0]));
    assert_eq!(kept[1].last().unwrap()["passed"], json!(3), "{:?}", kept[1]);
    assert!(
        login_replays(&kept[1]).is_some_and(|n| n >= 1),
        "the kept daemon replays: {:?}",
        kept[1].last()
    );
}

/// The requests `requests` (each a `runTests` request without `command` and `sourcePaths`) over the login stand-in and an
/// app under test whose sources are `src`, on one daemon started with `env`: each request's lines up to its summary.
fn login_app_requests(src: &str, requests: &[Value], env: &[(&str, &str)]) -> Vec<Vec<Value>> {
    let d = tempfile::tempdir().unwrap();
    let (dep, main) = login_probe(d.path());
    write(&main, "src/T.al", src);
    let dep_arg = dep.to_string_lossy().to_string();
    let mut s = Server::start_with(&["--no-download", "--no-test-data", "--dep", &dep_arg], env);
    s.line();
    let mut out = Vec::new();
    for r in requests {
        let mut request = r.clone();
        request["command"] = json!("runTests");
        request["sourcePaths"] = json!([main]);
        s.send(request);
        out.push(s.until_summary());
    }
    s.send(json!({"command": "shutdown"}));
    assert_eq!(s.line(), json!({"status": "shutting down"}));
    out
}

/// Runs `requests` with the replay on (`ALRUNNER_LOGIN_REPLAY=1`) and off (the default) and asserts every request gives
/// the same verdicts (entry, test, status, message); returns the replay-on lines.
fn same_with_and_without_the_replay(src: &str, requests: &[Value]) -> Vec<Vec<Value>> {
    let on = login_app_requests(src, requests, &[("ALRUNNER_LOGIN_REPLAY", "1")]);
    let off = login_app_requests(src, requests, &[]);
    for (n, (a, b)) in on.iter().zip(&off).enumerate() {
        assert_eq!(verdicts(a), verdicts(b), "request {n}: the replay changed a verdict");
    }
    on
}

/// Al-runner issue 8211: the login reads the request's `serverUrl` (`GetUrl`), so a session of another
/// URL never takes the start of a login that read another one. Same seed, so every other key field is equal.
#[test]
fn a_request_with_another_server_url_takes_no_start_of_the_first() {
    let src = r#"table 50830 "Probe Url" { fields { field(1; K; Integer) { } field(2; Url; Text[250]) { } } keys { key(PK; K) { } } }
codeunit 50820 "Probe Registration"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Guided Experience", 'OnRegisterManualSetup', '', false, false)]
    local procedure RegisterManualSetup() var U: Record "Probe Url";
    begin U.K := 1; U.Url := CopyStr(GetUrl(ClientType::Web), 1, 250); U.Insert(); end;
}
codeunit 50821 "Probe Login Tests"
{
    Subtype = Test;
    [Test] procedure SeesTheUrl() var U: Record "Probe Url"; begin U.Get(1); Error('URL=%1', U.Url); end;
}
"#;
    let other = "https://other.example/BC280/";
    let on = same_with_and_without_the_replay(
        src,
        &[
            json!({"seed": 7}),
            json!({"seed": 7}),
            json!({"seed": 7, "serverUrl": other}),
        ],
    );
    let message = |lines: &[Value]| {
        (lines.iter())
            .find(|l| l["type"] == "test")
            .map(|t| t["message"].to_string())
            .unwrap_or_default()
    };
    assert!(message(&on[2]).contains(other), "{:?}", on[2]);
    assert!(
        login_replays(&on[1]).is_some_and(|n| n >= 1),
        "the same URL replays: {:?}",
        on[1].last()
    );
}

/// The app of two probes: a login subscriber adds a row both test codeunits read,
/// and both take NumberSequence numbers, so under TestIsolation = Function in BC's order the second is refused when the
/// numbers an earlier codeunit took are not known (al-runner issue 8152, `Scratch::numbers_unmeasured`).
const NUMBERS_APP: &str = r#"table 50830 "RG Login Row" { fields { field(1; Code; Code[20]) { } field(2; Note; Text[50]) { } } keys { key(PK; Code) { Clustered = true; } } }
codeunit 50820 "Probe Registration"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Guided Experience", 'OnRegisterManualSetup', '', false, false)]
    local procedure RegisterManualSetup() var Row: Record "RG Login Row";
    begin Row.Code := 'L'; Row.Note := 'from login'; if Row.Insert() then; end;
}
codeunit 50821 "RG Tests W"
{
    Subtype = Test;
    [Test]
    procedure TakesNumbers() var Row: Record "RG Login Row"; N: BigInteger;
    begin
        if not NumberSequence.Exists('RGSEQ') then NumberSequence.Insert('RGSEQ', 1, 1);
        N := NumberSequence.Next('RGSEQ');
        Error('W rows=%1 next=%2', Row.Count(), N);
    end;
}
codeunit 50822 "RG Tests K"
{
    Subtype = Test;
    [Test]
    procedure ReadsNumbers() var Row: Record "RG Login Row"; N: BigInteger;
    begin
        if not NumberSequence.Exists('RGSEQ') then NumberSequence.Insert('RGSEQ', 1, 1);
        N := NumberSequence.Next('RGSEQ');
        Error('K rows=%1 next=%2', Row.Count(), N);
    end;
    [Test] procedure Plain() var Row: Record "RG Login Row"; begin Error('K plain rows=%1', Row.Count()); end;
}
"#;

/// A plain request, then one in BC's order under TestIsolation = Function, where the numbers mark
/// refuses `ReadsNumbers`. A start the plain request recorded without the mark must not run it.
#[test]
fn a_start_recorded_without_the_numbers_mark_does_not_run_a_refused_test() {
    let strict = json!({"seed": 1, "testIsolation": "test", "strictOrder": true});
    same_with_and_without_the_replay(
        NUMBERS_APP,
        &[json!({"seed": 1}), strict.clone(), json!({"seed": 1}), strict],
    );
}

/// The strict request first, so a start is recorded with the mark; a later request without it must
/// not refuse the test.
#[test]
fn a_start_recorded_with_the_numbers_mark_does_not_refuse_a_later_test() {
    let strict = json!({"seed": 1, "testIsolation": "test", "strictOrder": true});
    same_with_and_without_the_replay(
        NUMBERS_APP,
        &[
            strict.clone(),
            json!({"seed": 1, "testIsolation": "test"}),
            json!({"seed": 1}),
            strict,
        ],
    );
}

/// A guard (green before and after the login key hashes each row's bytes): an app whose install codeunit leaves a wide
/// table of 200 rows, in two shapes. With `login_row`, a subscriber to the login's setup event adds a row of its own, so
/// its sessions' logins are raised and write a row and the key's rows matter; without it, the login writes no row. Six
/// test codeunits, one failing, in three requests of one seed (one key per codeunit).
fn large_install_base_source(login_row: bool) -> String {
    let mut src = String::from(
        r#"table 50110 "Probe Wide"
{
    fields
    {
        field(1; "No."; Integer) { }
        field(2; F2; Text[100]) { }
        field(6; F6; Decimal) { }
        field(7; F7; Date) { }
        field(9; F9; Boolean) { }
        field(10; F10; Text[250]) { }
    }
    keys { key(PK; "No.") { Clustered = true; } }
}
table 50112 "Probe Login Row" { fields { field(1; Code; Code[20]) { } } keys { key(PK; Code) { Clustered = true; } } }
codeunit 50111 "Probe Install"
{
    Subtype = Install;
    trigger OnInstallAppPerCompany()
    var W: Record "Probe Wide"; i: Integer;
    begin
        for i := 1 to 200 do begin
            W.Init(); W."No." := i; W.F2 := 'Some descriptive text for row number ' + Format(i);
            W.F6 := i * 1.5; W.F7 := 20260101D; W.F9 := (i mod 2) = 0; W.F10 := 'A wide text field'; W.Insert();
        end;
    end;
}
codeunit 50100 "Probe Logic" { procedure Twice(X: Integer): Integer begin exit(X * 2); end; }
"#,
    );
    if login_row {
        src.push_str(
            r#"codeunit 50101 "Probe Registration Row"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Guided Experience", 'OnRegisterManualSetup', '', false, false)]
    local procedure RegisterManualSetup() var R: Record "Probe Login Row";
    begin R.Code := 'LOGIN'; if R.Insert() then; end;
}
"#,
        );
    }
    for n in 50..=55 {
        let fails = if n == 55 {
            "Error('rows=%1 login=%2', W.Count(), R.Count());"
        } else {
            "if Logic.Twice(1) <> 2 then Error('T');"
        };
        src.push_str(&format!(
            "codeunit 501{n} \"Probe Tests {n}\"\n{{\n    Subtype = Test;\n    var Logic: Codeunit \"Probe Logic\";\n    [Test]\n    procedure T1() var W: Record \"Probe Wide\"; R: Record \"Probe Login Row\";\n    begin\n        {fails}\n    end;\n}}\n"
        ));
    }
    src
}

/// Three requests of [`large_install_base_source`]: the six codeunits as entries, seed 1.
fn large_install_base_requests() -> Vec<Value> {
    let entries: Vec<Value> = (0..6)
        .map(|i| json!({"id": format!("e{i}"), "test": format!("Codeunit501{}.", 50 + i)}))
        .collect();
    vec![json!({"seed": 1, "entries": entries}); 3]
}

/// The lines of the replay check on `stderr` of a daemon that ran `requests` of `src` under `ALRUNNER_LOGIN_REPLAY_CHECK=1`.
fn check_lines(src: &str, requests: &[Value]) -> (Vec<Vec<Value>>, Vec<String>) {
    let d = tempfile::tempdir().unwrap();
    let (dep, main) = login_probe(d.path());
    write(&main, "src/T.al", src);
    let log = d.path().join("stderr.txt");
    let dep_arg = dep.to_string_lossy().to_string();
    let mut s = Server::start_with_stderr(
        &["--no-download", "--no-test-data", "--dep", &dep_arg],
        &[("ALRUNNER_LOGIN_REPLAY_CHECK", "1")],
        Stdio::from(std::fs::File::create(&log).unwrap()),
    );
    s.line();
    let mut out = Vec::new();
    for r in requests {
        let mut request = r.clone();
        request["command"] = json!("runTests");
        request["sourcePaths"] = json!([main]);
        s.send(request);
        out.push(s.until_summary());
    }
    drop(s.stdin.take());
    s.child.wait().unwrap();
    let text = std::fs::read_to_string(&log).unwrap();
    let checks = (text.lines().filter(|l| l.starts_with("login replay check: ")))
        .map(String::from)
        .collect();
    (out, checks)
}

/// A guard: the same verdicts with the replay on, off (`ALRUNNER_LOGIN_REPLAY=0`) and under the check, and the check
/// finds 0 that differ, for both shapes.
#[test]
fn a_large_install_base_gives_one_verdict_with_the_replay_on_off_and_under_the_check() {
    for (shape, login_row) in [("a login that writes a row", true), ("no login row", false)] {
        let src = large_install_base_source(login_row);
        let requests = large_install_base_requests();
        let on = login_app_requests(&src, &requests, &[("ALRUNNER_LOGIN_REPLAY", "1")]);
        let off = login_app_requests(&src, &requests, &[("ALRUNNER_LOGIN_REPLAY", "0")]);
        let (checked, check) = check_lines(&src, &requests);
        for n in 0..requests.len() {
            assert_eq!(
                verdicts(&on[n]),
                verdicts(&off[n]),
                "{shape} request {n}: on against off"
            );
            assert_eq!(
                verdicts(&checked[n]),
                verdicts(&off[n]),
                "{shape} request {n}: check against off"
            );
        }
        let failing: Vec<_> = verdicts(&off[0]).into_iter().filter(|v| v.2 != json!("pass")).collect();
        assert_eq!(failing.len(), 1, "{shape}: one failing test: {failing:?}");
        // Without the login row the login is not raised, so it has no key and no check line (nothing to compare); with it,
        // one per request.
        let lines = if login_row { requests.len() } else { 0 };
        assert_eq!(check.len(), lines, "{shape}: check lines: {check:?}");
        for line in &check {
            assert!(line.contains(", 0 differ;"), "{shape}: {line}");
        }
        let replays = login_replays(&on[2]).unwrap_or(0);
        assert_eq!(
            replays >= 1,
            login_row,
            "{shape}: a later request replays: {:?}",
            on[2].last()
        );
    }
}

// ---- Issue 8210: per-entry `stopGroup` for `stopAfterFirstFailure` ----

/// #8484's reason for a test after a failure when no entry has a `stopGroup`, byte for byte.
const UPSTREAM_STOP_TEXT: &str = "the test Codeunit50110.FailsFirst failed before it, and stopAfterFirstFailure (--stop-after-first-failure) runs no test of its codeunit or of a later entry after a failure";

/// 50110: the first test fails, three pass after it. 50140 (the echo): a failing test, then one that must pass.
fn group_app(root: &Path) -> PathBuf {
    app(
        root,
        "groupstop",
        "codeunit 50110 \"Stop Fixture\" { Subtype = Test;
    [Test] procedure FailsFirst() begin Error('killed'); end;
    [Test] procedure SlowTwo() begin end;
    [Test] procedure SlowThree() begin end;
    [Test] procedure SlowFour() begin end; }
codeunit 50140 \"Echo Fixture\" { Subtype = Test;
    [Test] procedure E1_Fails() begin Error('echo reports through Error'); end;
    [Test] procedure E2_Passes() begin end; }",
    )
}

/// A server with one test thread, so that the order of the tests is the declaration order.
fn one_thread_server(extra: &[&str]) -> Server {
    let mut args = vec!["--no-download", "--no-test-data"];
    args.extend_from_slice(extra);
    let mut s = Server::start_with(&args, &[("AL_TEST_THREADS", "1")]);
    s.line();
    s
}

fn test_lines(lines: &[Value]) -> Vec<&Value> {
    lines.iter().filter(|l| l["type"] == "test").collect()
}

fn run_request(s: &mut Server, a: &Path, extra: Value) -> Vec<Value> {
    let mut r = json!({"command": "runTests", "sourcePaths": [a]});
    for (k, v) in extra.as_object().unwrap() {
        r[k] = v.clone();
    }
    s.send(r);
    s.until_summary()
}

fn shut_down(mut s: Server) {
    s.send(json!({"command": "shutdown"}));
    s.line();
}

/// A row of a test the stop did not run, named for the failed `FailsFirst`; `group` is the text the reason must name.
fn is_stopped(l: &Value, group: Option<&str>) -> bool {
    l["notRun"] == true
        && l["message"].as_str().is_some_and(|m| {
            m.contains("Codeunit50110.FailsFirst")
                && m.contains("stopAfterFirstFailure")
                && group.is_none_or(|g| m.contains(&format!("stopGroup `{g}`")))
        })
}

fn entry_exit_codes(lines: &[Value]) -> Vec<Value> {
    lines
        .iter()
        .filter(|l| l["type"] == "entrySummary")
        .map(|l| l["exitCode"].clone())
        .collect()
}

/// S1: a failure stops the later entries of its own `stopGroup` only.
#[test]
fn stop_group_stops_only_the_later_entries_of_its_group() {
    let d = tempfile::tempdir().unwrap();
    let a = group_app(d.path());
    let mut s = one_thread_server(&[]);
    let t = "Codeunit50110.";
    let lines = run_request(
        &mut s,
        &a,
        json!({"stopAfterFirstFailure": true, "entries": [
            {"id": "a", "test": t, "stopGroup": "m1"}, {"id": "b", "test": t, "stopGroup": "m1"},
            {"id": "c", "test": t, "stopGroup": "m2"}, {"id": "d", "test": t, "stopGroup": "m2"}]}),
    );
    let tests = test_lines(&lines);
    assert_eq!(tests.len(), 16, "{lines:?}");
    for (k, (fails, group)) in [(true, "m1"), (false, "m1"), (true, "m2"), (false, "m2")]
        .into_iter()
        .enumerate()
    {
        let rows = &tests[4 * k..4 * k + 4];
        let rest = if fails {
            assert_eq!(rows[0]["status"], "fail", "{lines:?}");
            &rows[1..]
        } else {
            rows
        };
        for r in rest {
            assert!(is_stopped(r, Some(group)), "{r}");
        }
    }
    assert_eq!(
        entry_exit_codes(&lines),
        [json!(1), json!(7), json!(1), json!(7)],
        "{lines:?}"
    );
    let summary = lines.last().unwrap();
    assert_eq!(
        (&summary["failed"], &summary["notRun"], &summary["exitCode"]),
        (&json!(2), &json!(14), &json!(1)),
        "{summary}"
    );
    shut_down(s);
}

/// S2: the values and combinations of `stopGroup`; the daemon answers a plain request after each refusal.
#[test]
fn stop_group_values_and_combinations() {
    let d = tempfile::tempdir().unwrap();
    let a = group_app(d.path());
    let mut s = one_thread_server(&[]);
    let t = "Codeunit50110.";
    for (extra, says) in [
        (
            json!({"stopAfterFirstFailure": true, "entries": [{"test": t, "stopGroup": true}]}),
            "the `stopGroup` of an entry is a text or a number",
        ),
        (
            json!({"stopAfterFirstFailure": true, "entries": [{"test": t, "stopGroup": "m1"}, {"test": t}]}),
            STOP_GROUP_MIX,
        ),
        (
            json!({"stopAfterFirstFailure": false, "entries": [{"test": t, "stopGroup": "m1"}]}),
            STOP_GROUP_NEEDS_THE_STOP,
        ),
        (
            json!({"entries": [{"test": t, "stopGroup": "m1"}]}),
            STOP_GROUP_NEEDS_THE_STOP,
        ),
    ] {
        let lines = run_request(&mut s, &a, extra.clone());
        assert_eq!(lines.last().unwrap()["error"], says, "{extra}: {lines:?}");
        let plain = run_request(&mut s, &a, json!({"test": "Codeunit50140."}));
        assert_eq!(test_lines(&plain).len(), 2, "{plain:?}");
    }
    shut_down(s);
    // A server started with the flag takes groups without the request field.
    let mut s = one_thread_server(&["--stop-after-first-failure"]);
    let lines = run_request(
        &mut s,
        &a,
        json!({"entries": [{"id": "a", "test": t, "stopGroup": "m1"}, {"id": "b", "test": t, "stopGroup": "m1"}]}),
    );
    let tests = test_lines(&lines);
    assert_eq!(tests.len(), 8, "{lines:?}");
    assert_eq!(tests[0]["status"], "fail", "{lines:?}");
    assert!(tests[1..].iter().all(|r| is_stopped(r, Some("m1"))), "{lines:?}");
    shut_down(s);
}

const STOP_GROUP_MIX: &str = "some entries have a `stopGroup` and some do not: give every entry one (an entry that stops alone gets a group of its own), or none (a failure then stops the whole request)";

const STOP_GROUP_NEEDS_THE_STOP: &str = "`stopGroup` needs `stopAfterFirstFailure` or `stopAfterTimeout` (or a server started with `--stop-after-first-failure` or `--stop-after-timeout`): without a stop a group changes nothing";

/// E3, kept on its own so that it is easy to flip if a client needs `stopGroup` with the stop off to be ignored.
#[test]
fn stop_group_with_the_stop_off_is_refused() {
    let d = tempfile::tempdir().unwrap();
    let a = group_app(d.path());
    let mut s = one_thread_server(&[]);
    let lines = run_request(
        &mut s,
        &a,
        json!({"stopAfterFirstFailure": false, "entries": [{"test": "Codeunit50110.", "stopGroup": "m1"}]}),
    );
    assert_eq!(lines.last().unwrap()["error"], STOP_GROUP_NEEDS_THE_STOP, "{lines:?}");
    assert!(test_lines(&lines).is_empty(), "{lines:?}");
    shut_down(s);
}

/// S3: under the stop with groups a selection runs only the tests it picks.
#[test]
fn stop_group_runs_only_the_picked_tests() {
    let d = tempfile::tempdir().unwrap();
    let main = app(
        d.path(),
        "groupspicked",
        "codeunit 50111 \"Stop Picked\" { Subtype = Test;
            [Test] procedure A_FailsUnpicked() begin Error('not picked'); end;
            [Test] procedure B_Picked() begin end;
            [Test] procedure C_Picked() begin end; }",
    );
    let mut s = Server::start(&["--no-download", "--no-test-data"]);
    s.line();
    s.send(
        json!({"command": "runTests", "sourcePaths": [main], "stopAfterFirstFailure": true,
        "entries": [{"id": "e", "test": "Codeunit50111.", "excludeTests": ["FailsUnpicked"], "stopGroup": "e"},
                    {"id": "f", "test": "Codeunit50111.", "excludeTests": ["FailsUnpicked"], "stopGroup": "f"}]}),
    );
    let lines = s.until_summary();
    let tests = test_lines(&lines);
    assert_eq!(tests.len(), 4, "{lines:?}");
    assert!(tests.iter().all(|t| t["status"] == "pass"), "{lines:?}");
    shut_down(s);
}

/// S4: a grouped stop over a dependency keeps the install baseline and carries no writes into the next request.
#[test]
fn stop_group_over_a_dependency_keeps_the_baseline_and_carries_no_writes() {
    let d = tempfile::tempdir().unwrap();
    let (main, dep) = installing_apps(d.path());
    write(
        &main,
        "src/T.al",
        "codeunit 50712 \"Seeded Stop\" { Subtype = Test;
    [Test] procedure A_Fails() begin Error('fails first'); end;
    [Test] procedure B_SeesTheInstallRowOnly() var S: Record Seeded; begin
        if S.Count() <> 1 then Error('Seeded holds %1 rows', S.Count());
        S.K := 'B'; S.Insert(); Commit(); end; }\n",
    );
    let dep = dep.to_string_lossy().to_string();
    let cache = d.path().join("cache").to_string_lossy().to_string();
    let mut s = Server::start(&["--no-download", "--no-test-data", "--dep", &dep, "--cache-dir", &cache]);
    s.line();
    let mut run = |stop: bool, groups: bool| {
        let entry = |id: &str, g: &str| {
            let mut e = json!({"id": id, "test": "Codeunit50712."});
            if groups {
                e["stopGroup"] = json!(g);
            }
            e
        };
        s.send(
            json!({"command": "runTests", "sourcePaths": [main], "stopAfterFirstFailure": stop,
            "entries": [entry("a", "m1"), entry("b", "m2")]}),
        );
        let lines = s.until_summary();
        let of = |entry: &str, name: &str| {
            lines
                .iter()
                .find(|l| {
                    l["type"] == "test" && l["entry"] == entry && l["name"].as_str().is_some_and(|n| n.ends_with(name))
                })
                .unwrap_or_else(|| panic!("{entry} {name}: {lines:?}"))
                .clone()
        };
        let summary = lines.last().unwrap().clone();
        (
            of("a", "A_Fails"),
            of("a", "B_SeesTheInstallRowOnly"),
            of("b", "A_Fails"),
            of("b", "B_SeesTheInstallRowOnly"),
            summary,
        )
    };
    let stopped = |b: &Value| {
        b["notRun"] == true
            && b["message"]
                .as_str()
                .is_some_and(|m| m.contains("A_Fails") && m.contains("stopAfterFirstFailure"))
    };
    for (stop, groups, source) in [
        (true, true, "ran"),
        (true, true, "kept"),
        (false, false, "kept"),
        (true, true, "kept"),
    ] {
        let (a_fails, a_b, b_fails, b_b, summary) = run(stop, groups);
        assert_eq!(a_fails["status"], "fail", "{stop}: {a_fails}");
        assert_eq!(b_fails["status"], "fail", "{stop}: {b_fails}");
        if stop {
            assert!(stopped(&a_b) && stopped(&b_b), "{a_b} {b_b}");
        } else {
            assert_eq!(
                (&a_b["status"], &b_b["status"]),
                (&json!("pass"), &json!("pass")),
                "{a_b} {b_b}"
            );
        }
        assert_eq!(summary["timings"]["baselineSource"], source, "{stop}: {summary}");
    }
    shut_down(s);
}

/// A fixture whose tests write a row, a SingleInstance text and WorkDate before they fail or pass, and a reader that
/// reports what it sees (S5).
fn state_stop_app(root: &Path) -> PathBuf {
    app(
        root,
        "statestop",
        "table 50130 \"Stop Row\" { fields { field(1; PK; Code[10]) { } } keys { key(PK; PK) { } } }
codeunit 50131 \"Stop Single\" { SingleInstance = true; var Txt: Text;
    procedure SetTxt(T: Text) begin Txt := T; end; procedure GetTxt(): Text begin exit(Txt); end; }
codeunit 50132 \"Stop Writers\" { Subtype = Test;
    [Test] procedure FailsAfterWriting() var R: Record \"Stop Row\"; S: Codeunit \"Stop Single\";
    begin R.PK := 'A'; R.Insert(); S.SetTxt('written'); WorkDate(20300101D); Error('killed'); end;
    [Test] procedure PassesAfterWriting() var R: Record \"Stop Row\"; S: Codeunit \"Stop Single\";
    begin R.PK := 'B'; R.Insert(); S.SetTxt('written too'); WorkDate(20310101D); end; }
codeunit 50133 \"Stop Reader\" { Subtype = Test;
    [Test] procedure Reports() var R: Record \"Stop Row\"; S: Codeunit \"Stop Single\";
    begin Error('rows=%1 txt=%2 date=%3', R.Count(), S.GetTxt(), WorkDate()); end; }",
    )
}

/// The `test` lines of a request without their time, and its counts.
fn answer_of(lines: &[Value]) -> (Vec<Value>, Vec<Value>) {
    let tests = lines
        .iter()
        .filter(|l| l["type"] == "test")
        .map(|l| {
            let mut l = l.clone();
            l.as_object_mut().unwrap().remove("durationMs");
            l
        })
        .collect();
    let s = lines.last().unwrap();
    let counts = ["total", "passed", "failed", "notRun", "exitCode"]
        .iter()
        .map(|k| s[*k].clone())
        .collect();
    (tests, counts)
}

/// S5: a request after a grouped stop answers as it does on a fresh daemon.
#[test]
fn the_request_after_a_grouped_stop_answers_as_a_fresh_daemon_does() {
    let d = tempfile::tempdir().unwrap();
    let a = state_stop_app(d.path());
    let mut stopped = one_thread_server(&[]);
    let first = run_request(
        &mut stopped,
        &a,
        json!({"stopAfterFirstFailure": true, "entries": [
            {"id": "w", "test": "Codeunit50132.", "stopGroup": "m1"},
            {"id": "r", "test": "Codeunit50133.", "stopGroup": "m2"}]}),
    );
    assert!(
        test_lines(&first).iter().any(|t| t["notRun"] == true),
        "the request did not stop: {first:?}"
    );
    let after = answer_of(&run_request(&mut stopped, &a, json!({})));
    let mut fresh = one_thread_server(&[]);
    let want = answer_of(&run_request(&mut fresh, &a, json!({})));
    assert_eq!(after, want);
    shut_down(stopped);
    shut_down(fresh);
}

/// S7: the echo shape. An entry that fails on purpose alone in its own group stops no other group; a plain request
/// answers after it.
#[test]
fn an_echo_entry_in_its_own_group_leaves_every_other_group_running() {
    let d = tempfile::tempdir().unwrap();
    let a = group_app(d.path());
    let mut s = one_thread_server(&[]);
    let t = "Codeunit50110.";
    let lines = run_request(
        &mut s,
        &a,
        json!({"stopAfterFirstFailure": true, "entries": [
            {"id": "echo", "test": "Codeunit50140.", "stopGroup": "echo"},
            {"id": "a", "test": t, "stopGroup": "m1"}, {"id": "b", "test": t, "stopGroup": "m2"}]}),
    );
    let tests = test_lines(&lines);
    assert_eq!(tests.len(), 10, "{lines:?}");
    assert_eq!(tests[0]["status"], "fail", "{lines:?}");
    assert_eq!(tests[1]["notRun"], true, "{lines:?}");
    for (k, group) in [(2, "m1"), (6, "m2")] {
        assert_eq!(tests[k]["status"], "fail", "{lines:?}");
        assert!(
            tests[k + 1..k + 4].iter().all(|r| is_stopped(r, Some(group))),
            "{lines:?}"
        );
    }
    let plain = run_request(&mut s, &a, json!({"test": "Codeunit50140."}));
    assert_eq!(test_lines(&plain).len(), 2, "{plain:?}");
    shut_down(s);
}

/// With no `stopGroup` on any entry the reason is #8484's, byte for byte, and the stop is the whole request's.
#[test]
fn without_stop_groups_the_reason_is_upstreams_text() {
    let d = tempfile::tempdir().unwrap();
    let a = group_app(d.path());
    let mut s = one_thread_server(&[]);
    let t = "Codeunit50110.";
    let lines = run_request(
        &mut s,
        &a,
        json!({"stopAfterFirstFailure": true, "entries": [{"id": "a", "test": t}, {"id": "b", "test": t}]}),
    );
    let tests = test_lines(&lines);
    assert_eq!(tests.len(), 8, "{lines:?}");
    assert_eq!(tests[0]["status"], "fail", "{lines:?}");
    for r in &tests[1..] {
        assert_eq!(r["notRun"], true, "{r}");
        let m = r["message"].as_str().unwrap();
        assert_eq!(m, format!("NotRun: out-of-scope: {UPSTREAM_STOP_TEXT}"));
    }
    shut_down(s);
}

// ---- Issue 9213: `stopAfterTimeout`, a timeout row stops the request (or its `stopGroup`) and the daemon is kept ----

/// 50150: a test the Sleep budget stops between two that pass. 50151: one that passes.
fn timeout_app(root: &Path) -> PathBuf {
    app(
        root,
        "timeoutstop",
        "codeunit 50150 \"Timeout Fixture\" { Subtype = Test;
    [Test] procedure A_Passes() begin end;
    [Test] procedure B_TimesOut() begin Sleep(5000); end;
    [Test] procedure C_Passes() begin end; }
codeunit 50151 \"After Fixture\" { Subtype = Test;
    [Test] procedure D_Passes() begin end; }",
    )
}

/// The `notRun` message of a test the stop did not run after `B_TimesOut`, with `later` the entries it names.
fn timeout_stop_message(later: &str) -> String {
    format!(
        "NotRun: out-of-scope: the test Codeunit50150.B_TimesOut was stopped at the time limit before it, and stopAfterTimeout (--stop-after-timeout) runs no test of its codeunit or of {later} after a timeout"
    )
}

fn is_timeout_row(l: &Value) -> bool {
    l["notRun"] == true && l["message"] == "Test exceeded 1s timeout."
}

/// The status of each `test` line, or the message of a `notRun` one.
fn statuses(lines: &[Value]) -> Vec<String> {
    test_lines(lines)
        .iter()
        .map(|l| match l["notRun"] == true {
            true => l["message"].as_str().unwrap_or_default().to_string(),
            false => l["status"].as_str().unwrap_or_default().to_string(),
        })
        .collect()
}

/// Issue 9213: "After a timeout row, the rest of its codeunit and the later entries are not run"; with `stopGroup`s
/// "only the later entries of its own group"; a timeout row stays `notRun` with its clock.
#[test]
fn stop_after_timeout_ends_the_request_or_its_group_at_the_first_timeout_row() {
    let d = tempfile::tempdir().unwrap();
    let a = timeout_app(d.path());
    let mut s = one_thread_server(&[]);
    let (t, after) = ("Codeunit50150.", "Codeunit50151.");
    let lines = run_request(
        &mut s,
        &a,
        json!({"stopAfterTimeout": true, "testTimeoutSecs": 1,
            "entries": [{"id": "a", "test": t}, {"id": "b", "test": after}]}),
    );
    let tests = test_lines(&lines);
    assert_eq!(tests.len(), 4, "{lines:?}");
    assert_eq!(tests[0]["status"], "pass", "{lines:?}");
    assert!(is_timeout_row(tests[1]) && tests[1]["clock"] == "virtual", "{lines:?}");
    for r in &tests[2..] {
        assert_eq!(r["notRun"], true, "{r}");
        assert_eq!(r["message"], timeout_stop_message("a later entry"), "{r}");
    }
    assert_eq!(entry_exit_codes(&lines), [json!(7), json!(7)], "{lines:?}");
    let grouped = run_request(
        &mut s,
        &a,
        json!({"stopAfterTimeout": true, "testTimeoutSecs": 1, "entries": [
            {"id": "a", "test": t, "stopGroup": "m1"}, {"id": "b", "test": after, "stopGroup": "m2"},
            {"id": "c", "test": after, "stopGroup": "m1"}]}),
    );
    let in_m1 = timeout_stop_message("a later entry of its stopGroup `m1`");
    assert_eq!(
        statuses(&grouped),
        ["pass", "Test exceeded 1s timeout.", &in_m1, "pass", &in_m1],
        "{grouped:?}"
    );
    shut_down(s);
}

/// Issue 9213: off by default; a failure does not trigger it, a timeout does not trigger `stopAfterFirstFailure`, and
/// `skipAfterTimeout` alone is unchanged. With both stops the first triggering row is named; with `skipAfterTimeout`
/// too, the codeunit's rows carry the one stop text.
#[test]
fn stop_after_timeout_alone_and_with_the_other_options() {
    let d = tempfile::tempdir().unwrap();
    let a = timeout_app(d.path());
    let mut s = one_thread_server(&[]);
    let entries = json!([{"id": "a", "test": "Codeunit50150."}, {"id": "b", "test": "Codeunit50151."}]);
    let with = |extra: Value| {
        let mut r = json!({"testTimeoutSecs": 1, "entries": entries.clone()});
        for (k, v) in extra.as_object().unwrap() {
            r[k] = v.clone();
        }
        r
    };
    let timeout = "Test exceeded 1s timeout.";
    for extra in [
        json!({}),
        json!({"stopAfterTimeout": false}),
        json!({"stopAfterFirstFailure": true}),
    ] {
        let lines = run_request(&mut s, &a, with(extra.clone()));
        assert_eq!(
            statuses(&lines),
            ["pass", timeout, "pass", "pass"],
            "{extra}: {lines:?}"
        );
    }
    let skip = run_request(&mut s, &a, with(json!({"skipAfterTimeout": true})));
    let skipped = "NotRun: out-of-scope: an earlier test of this codeunit (B_TimesOut) was stopped at the time limit, and --skip-after-timeout does not run the rest of the codeunit";
    assert_eq!(statuses(&skip), ["pass", timeout, skipped, "pass"], "{skip:?}");
    let stop = timeout_stop_message("a later entry");
    for extra in [
        json!({"stopAfterTimeout": true, "skipAfterTimeout": true}),
        json!({"stopAfterTimeout": true, "stopAfterFirstFailure": true}),
    ] {
        let lines = run_request(&mut s, &a, with(extra.clone()));
        assert_eq!(statuses(&lines), ["pass", timeout, &stop, &stop], "{extra}: {lines:?}");
    }
    let lines = run_request(&mut s, &a, with(json!({"stopAfterTimeout": "yes"})));
    assert_eq!(
        lines.last().unwrap()["error"],
        "`stopAfterTimeout` is true or false",
        "{lines:?}"
    );
    shut_down(s);
    // A server started with the flag stops without the request field, and takes groups.
    let mut s = one_thread_server(&["--stop-after-timeout"]);
    let lines = run_request(
        &mut s,
        &a,
        json!({"testTimeoutSecs": 1, "entries": [
            {"id": "a", "test": "Codeunit50150.", "stopGroup": "m1"}, {"id": "b", "test": "Codeunit50151.", "stopGroup": "m1"}]}),
    );
    let in_m1 = timeout_stop_message("a later entry of its stopGroup `m1`");
    assert_eq!(statuses(&lines), ["pass", timeout, &in_m1, &in_m1], "{lines:?}");
    shut_down(s);
}

/// Issue 9213, "Daemon cleanliness": the stop only removes work. A test that writes rows, WorkDate, a SingleInstance
/// value, a NumberSequence and an IsolatedStorage key and then spins until the time limit stops it (issue 8334's cut)
/// stops the reader after it; the next request on the same daemon answers as a new daemon does.
#[test]
fn a_daemon_after_a_stop_after_timeout_answers_as_a_new_daemon() {
    let d = tempfile::tempdir().unwrap();
    let a = app(
        d.path(),
        "timeoutstopstate",
        "table 50152 \"Stop Timeout Row\" { fields { field(1; No; Integer) { } } keys { key(PK; No) { } } }
codeunit 50153 \"Stop Timeout Single\" { SingleInstance = true; var V: Integer;
    procedure SetV(N: Integer) begin V := N; end; procedure GetV(): Integer begin exit(V); end; }
codeunit 50154 \"Stop Hanger\" { Subtype = Test;
    [Test] procedure WritesThenHangs() var R: Record \"Stop Timeout Row\"; S: Codeunit \"Stop Timeout Single\"; i: Integer; begin
        R.No := 1; R.Insert(); Commit(); R.No := 2; R.Insert();
        WorkDate(20010101D); S.SetV(7);
        NumberSequence.Insert('STATE9213'); IsolatedStorage.Set('state9213', 'x');
        while true do i := (i + 1) mod 7919; end; }
codeunit 50155 \"Stop Reader\" { Subtype = Test;
    [Test] procedure Reports() var R: Record \"Stop Timeout Row\"; S: Codeunit \"Stop Timeout Single\"; begin
        Error('rows=%1 date=%2 single=%3 sequence=%4 storage=%5', R.Count(), WorkDate(), S.GetV(),
            NumberSequence.Exists('STATE9213'), IsolatedStorage.Contains('state9213')); end; }",
    );
    let mut stopped = one_thread_server(&[]);
    let first = run_request(
        &mut stopped,
        &a,
        json!({"stopAfterTimeout": true, "testTimeoutSecs": 1, "entries": [
            {"id": "h", "test": "Codeunit50154."}, {"id": "r", "test": "Codeunit50155."}]}),
    );
    let tests = test_lines(&first);
    assert!(is_timeout_row(tests[0]), "{first:?}");
    assert!(
        tests[1]["message"]
            .as_str()
            .is_some_and(|m| m.contains("Codeunit50154.WritesThenHangs") && m.contains("stopAfterTimeout")),
        "{first:?}"
    );
    let reader = json!({"test": "Codeunit50155."});
    let after = answer_of(&run_request(&mut stopped, &a, reader.clone()));
    let mut fresh = one_thread_server(&[]);
    let want = answer_of(&run_request(&mut fresh, &a, reader));
    assert_eq!(after, want);
    shut_down(stopped);
    shut_down(fresh);
}

/// Issue 9213: the refusal of a mix of grouped and ungrouped entries names the stop that would stop the whole request:
/// a failure under `stopAfterFirstFailure` (the text of issue 8210, unchanged), a timeout under `stopAfterTimeout`
/// alone, either under both.
#[test]
fn the_mixed_group_refusal_names_the_stops_that_are_on() {
    let d = tempfile::tempdir().unwrap();
    let a = timeout_app(d.path());
    let mut s = one_thread_server(&[]);
    let entries = json!([{"test": "Codeunit50151.", "stopGroup": "m1"}, {"test": "Codeunit50151."}]);
    let mix = |then: &str| {
        format!(
            "some entries have a `stopGroup` and some do not: give every entry one (an entry that stops alone gets a group of its own), or none ({then} then stops the whole request)"
        )
    };
    for (stops, then) in [
        (json!({"stopAfterFirstFailure": true}), "a failure"),
        (json!({"stopAfterTimeout": true}), "a timeout"),
        (
            json!({"stopAfterFirstFailure": true, "stopAfterTimeout": true}),
            "a failure or a timeout",
        ),
    ] {
        let mut r = stops.clone();
        r["entries"] = entries.clone();
        let lines = run_request(&mut s, &a, r);
        assert_eq!(lines.last().unwrap()["error"], mix(then), "{stops}: {lines:?}");
        assert_eq!(STOP_GROUP_MIX, mix("a failure"), "issue 8210's text is unchanged");
    }
    shut_down(s);
}
