//! `al-runner debug` (ADR-048) end to end: scripted Debug Adapter Protocol sessions over TCP and stdio against the
//! fixture apps in `tests/fixtures/debug`, in the shapes v2's `--dap` tests use (requests, capabilities, events and
//! answers), so a client of v2 works unchanged. The fixtures are written for this runner.

use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use serde_json::{Value, json};

fn fixture(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/debug")
        .join(name)
        .to_string_lossy()
        .into_owned()
}

fn fixture_file(app: &str, file: &str) -> String {
    PathBuf::from(fixture(app)).join(file).to_string_lossy().into_owned()
}

const BREAKPOINT_FILE: &str = "BreakpointTests.Codeunit.al";
/// `Counter := 2;`: Counter is 1 and Name is 'abc' before it runs.
const SECOND_COUNTER_LINE: i64 = 13;
const STEPPING_FILE: &str = "SteppingTests.Codeunit.al";
/// `Result := Twice(Result);`, `Result := Result + 10;`, and Twice's `Y := X * 2;` and `exit(Y);`.
const CALL_LINE: i64 = 11;
const AFTER_CALL_LINE: i64 = 12;
const TWICE_FIRST_LINE: i64 = 21;
const TWICE_EXIT_LINE: i64 = 22;

const WAIT: Duration = Duration::from_secs(60);

fn command(args: &[&str]) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_al-runner"));
    c.env("AL_CHECK_THREADS", "2")
        .env("AL_LOWER_THREADS", "2")
        .env("ALRUNNER_NO_PATH_NOTE", "1")
        .args(args);
    c
}

/// Reads one `Content-Length` frame; any other byte on the stream fails the test.
fn read_frame(r: &mut impl BufRead) -> Option<Value> {
    let mut length = None;
    loop {
        let mut line = String::new();
        if r.read_line(&mut line).ok()? == 0 {
            return None;
        }
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            break;
        }
        let (name, value) = line
            .split_once(':')
            .unwrap_or_else(|| panic!("a byte on the wire that is not a frame header: {line:?}"));
        assert_eq!(name, "Content-Length", "header {line:?}");
        length = Some(value.trim().parse::<usize>().expect("length"));
    }
    let mut body = vec![0; length.expect("Content-Length")];
    r.read_exact(&mut body).ok()?;
    Some(serde_json::from_slice(&body).unwrap_or_else(|e| panic!("a frame that is not JSON: {e}")))
}

/// A small DAP client: requests go out numbered, responses and events come back through a reader thread.
struct Client {
    child: Child,
    to: Box<dyn Write>,
    from: mpsc::Receiver<Value>,
    seq: i64,
    /// Messages read while waiting for another one.
    backlog: Vec<Value>,
    stderr: mpsc::Receiver<String>,
}

fn reader(r: impl Read + Send + 'static) -> mpsc::Receiver<Value> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut r = BufReader::new(r);
        while let Some(m) = read_frame(&mut r) {
            if tx.send(m).is_err() {
                break;
            }
        }
    });
    rx
}

fn stderr_lines(r: impl Read + Send + 'static) -> mpsc::Receiver<String> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(r).lines().map_while(Result::ok) {
            if tx.send(line).is_err() {
                break;
            }
        }
    });
    rx
}

impl Client {
    fn tcp(app: &str, extra: &[&str]) -> Client {
        let mut args = vec!["debug", "--port", "0", app, "--no-download", "--no-test-data"];
        args.extend_from_slice(extra);
        let mut child = command(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("start al-runner debug");
        let stderr = stderr_lines(child.stderr.take().unwrap());
        let port = loop {
            let line = stderr.recv_timeout(WAIT).expect("the listening line on stderr");
            if let Some(rest) = line.split("listening on 127.0.0.1:").nth(1) {
                break rest.trim().parse::<u16>().expect("port");
            }
        };
        let stream = std::net::TcpStream::connect(("127.0.0.1", port)).expect("connect");
        let from = reader(stream.try_clone().unwrap());
        Client {
            child,
            to: Box::new(stream),
            from,
            seq: 0,
            backlog: Vec::new(),
            stderr,
        }
    }

    fn stdio(app: &str) -> Client {
        let mut child = command(&["debug", "--stdio", app, "--no-download", "--no-test-data"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("start al-runner debug");
        let stderr = stderr_lines(child.stderr.take().unwrap());
        let from = reader(child.stdout.take().unwrap());
        Client {
            to: Box::new(child.stdin.take().unwrap()),
            child,
            from,
            seq: 0,
            backlog: Vec::new(),
            stderr,
        }
    }

    fn send(&mut self, command: &str, arguments: Value) -> i64 {
        self.seq += 1;
        let body = serde_json::to_vec(&json!({
            "seq": self.seq, "type": "request", "command": command, "arguments": arguments
        }))
        .unwrap();
        write!(self.to, "Content-Length: {}\r\n\r\n", body.len()).unwrap();
        self.to.write_all(&body).unwrap();
        self.to.flush().unwrap();
        self.seq
    }

    fn next_matching(&mut self, what: &str, pick: impl Fn(&Value) -> bool) -> Value {
        if let Some(i) = self.backlog.iter().position(&pick) {
            return self.backlog.remove(i);
        }
        loop {
            let m = self
                .from
                .recv_timeout(WAIT)
                .unwrap_or_else(|_| panic!("no {what}; read so far: {:#?}; stderr: {}", self.backlog, self.stderr()));
            if pick(&m) {
                return m;
            }
            self.backlog.push(m);
        }
    }

    fn response(&mut self, seq: i64) -> Value {
        self.next_matching(&format!("response to {seq}"), |m| {
            m["type"] == "response" && m["request_seq"] == seq
        })
    }

    fn request(&mut self, command: &str, arguments: Value) -> Value {
        let seq = self.send(command, arguments);
        self.response(seq)
    }

    fn event(&mut self, name: &str) -> Value {
        self.next_matching(&format!("event {name}"), |m| m["type"] == "event" && m["event"] == name)
    }

    fn stderr(&self) -> String {
        self.stderr.try_iter().collect::<Vec<_>>().join("\n")
    }

    /// `initialize` and `launch`, both answered with success.
    fn start(&mut self, initialize: Value) {
        let r = self.request("initialize", initialize);
        assert_eq!(r["success"], true, "{r}");
        assert_eq!(r["body"]["supportsConfigurationDoneRequest"], true, "{r}");
        self.event("initialized");
        let r = self.request("launch", json!({}));
        assert_eq!(r["success"], true, "{r}");
    }

    fn set_breakpoints(&mut self, file: &str, lines: &[i64]) -> Value {
        let bps: Vec<Value> = lines.iter().map(|l| json!({"line": l})).collect();
        let r = self.request("setBreakpoints", json!({"source": {"path": file}, "breakpoints": bps}));
        assert_eq!(r["success"], true, "{r}");
        r
    }

    fn top_frame(&mut self) -> Value {
        let r = self.request("stackTrace", json!({"threadId": 1}));
        assert_eq!(r["success"], true, "{r}");
        r["body"]["stackFrames"][0].clone()
    }

    /// The locals of frame `frame` by name.
    fn locals(&mut self, frame: i64) -> std::collections::HashMap<String, String> {
        let r = self.request("scopes", json!({"frameId": frame}));
        assert_eq!(r["success"], true, "{r}");
        let scopes = r["body"]["scopes"].as_array().unwrap().clone();
        assert_eq!(scopes.len(), 1, "{r}");
        assert_eq!(scopes[0]["name"], "Locals", "{r}");
        let r = self.request(
            "variables",
            json!({"variablesReference": scopes[0]["variablesReference"]}),
        );
        assert_eq!(r["success"], true, "{r}");
        r["body"]["variables"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| {
                assert_eq!(v["variablesReference"], 0, "{v}");
                (
                    v["name"].as_str().unwrap().to_string(),
                    v["value"].as_str().unwrap().to_string(),
                )
            })
            .collect()
    }

    fn stopped(&mut self) -> Value {
        let e = self.event("stopped");
        assert_eq!(e["body"]["threadId"], 1, "{e}");
        assert_eq!(e["body"]["allThreadsStopped"], true, "{e}");
        e["body"].clone()
    }

    /// Waits for `exited` and the process's end; the exit code the event names.
    fn finish(mut self) -> i64 {
        self.event("terminated");
        let e = self.event("exited");
        let code = e["body"]["exitCode"].as_i64().unwrap();
        let r = self.request("disconnect", json!({}));
        assert_eq!(r["success"], true, "{r}");
        let status = wait(&mut self.child);
        assert_eq!(status.code().map(i64::from), Some(code), "{}", self.stderr());
        code
    }
}

fn wait(child: &mut Child) -> std::process::ExitStatus {
    let end = std::time::Instant::now() + WAIT;
    loop {
        if let Some(s) = child.try_wait().unwrap() {
            return s;
        }
        if std::time::Instant::now() > end {
            let _ = child.kill();
            panic!("al-runner debug did not exit");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// v2's `DapServerTests` main sequence: a breakpoint, the stack, `Locals`, continue, `exited` 0.
fn breakpoint_session(mut c: Client) {
    c.start(json!({"clientID": "test", "linesStartAt1": true, "columnsStartAt1": true}));
    let file = fixture_file("breakpoint", BREAKPOINT_FILE);
    let r = c.set_breakpoints(&file, &[SECOND_COUNTER_LINE]);
    let bps = r["body"]["breakpoints"].as_array().unwrap();
    assert_eq!(bps.len(), 1, "{r}");
    assert_eq!(bps[0]["verified"], true, "{r}");
    assert_eq!(bps[0]["line"], SECOND_COUNTER_LINE, "{r}");
    assert_eq!(bps[0]["column"], 9, "{r}");
    let r = c.request("configurationDone", json!({}));
    assert_eq!(r["success"], true, "{r}");
    let s = c.stopped();
    assert_eq!(s["reason"], "breakpoint", "{s}");
    assert_eq!(s["line"], SECOND_COUNTER_LINE, "{s}");
    let r = c.request("threads", json!({}));
    assert_eq!(
        r["body"]["threads"],
        json!([{"id": 1, "name": "AL Test Thread"}]),
        "{r}"
    );
    let top = c.top_frame();
    assert_eq!(top["line"], SECOND_COUNTER_LINE, "{top}");
    assert_eq!(top["column"], 1, "{top}");
    assert_eq!(top["source"]["name"], BREAKPOINT_FILE, "{top}");
    assert!(top["name"].as_str().unwrap().contains("ThreeAssignments"), "{top}");
    let vars = c.locals(top["id"].as_i64().unwrap());
    assert_eq!(vars["Counter"], "1", "{vars:?}");
    assert_eq!(vars["Name"], "\"abc\"", "{vars:?}");
    let r = c.request("continue", json!({"threadId": 1}));
    assert_eq!(r["success"], true, "{r}");
    assert_eq!(r["body"]["allThreadsContinued"], true, "{r}");
    let out = c.event("output");
    assert_eq!(out["body"]["category"], "console", "{out}");
    assert_eq!(
        out["body"]["output"], "[debug] Debug Breakpoint Tests.ThreeAssignments: Pass\n",
        "{out}"
    );
    assert_eq!(c.finish(), 0);
}

#[test]
fn a_breakpoint_stops_before_its_statement_with_the_stack_and_the_locals_over_tcp() {
    breakpoint_session(Client::tcp(&fixture("breakpoint"), &[]));
}

#[test]
fn the_same_session_over_stdio_writes_only_frames_on_stdout() {
    // `read_frame` fails the test on any byte of stdout that is not part of a frame.
    breakpoint_session(Client::stdio(&fixture("breakpoint")));
}

#[test]
fn without_breakpoints_the_tests_run_to_the_end_and_nothing_stops() {
    let mut c = Client::tcp(&fixture("breakpoint"), &[]);
    c.start(json!({}));
    c.request("configurationDone", json!({}));
    assert_eq!(c.finish(), 0);
    // The run's backlog holds no stop.
}

/// Stops at `line` of the stepping fixture.
fn stepping_at(line: i64) -> Client {
    let mut c = Client::tcp(&fixture("stepping"), &[]);
    c.start(json!({}));
    c.set_breakpoints(&fixture_file("stepping", STEPPING_FILE), &[line]);
    c.request("configurationDone", json!({}));
    let s = c.stopped();
    assert_eq!(
        (s["reason"].as_str(), s["line"].as_i64()),
        (Some("breakpoint"), Some(line)),
        "{s}"
    );
    c
}

#[test]
fn next_steps_over_the_call() {
    let mut c = stepping_at(CALL_LINE);
    let r = c.request("next", json!({"threadId": 1}));
    assert_eq!(r["success"], true, "{r}");
    let s = c.stopped();
    assert_eq!(
        (s["reason"].as_str(), s["line"].as_i64()),
        (Some("step"), Some(AFTER_CALL_LINE)),
        "{s}"
    );
    let top = c.top_frame();
    assert_eq!(c.locals(top["id"].as_i64().unwrap())["Result"], "2");
    c.request("continue", json!({"threadId": 1}));
    assert_eq!(c.finish(), 0);
}

#[test]
fn step_in_stops_at_the_first_statement_of_the_callee() {
    let mut c = stepping_at(CALL_LINE);
    c.request("stepIn", json!({"threadId": 1}));
    let s = c.stopped();
    assert_eq!(
        (s["reason"].as_str(), s["line"].as_i64()),
        (Some("step"), Some(TWICE_FIRST_LINE)),
        "{s}"
    );
    let r = c.request("stackTrace", json!({"threadId": 1}));
    let frames = r["body"]["stackFrames"].as_array().unwrap().clone();
    assert_eq!(r["body"]["totalFrames"], 2, "{r}");
    assert!(frames[0]["name"].as_str().unwrap().contains("Twice"), "{r}");
    assert!(frames[1]["name"].as_str().unwrap().contains("CallsAHelper"), "{r}");
    assert_eq!(frames[1]["line"], CALL_LINE, "{r}");
    let inner = c.locals(frames[0]["id"].as_i64().unwrap());
    assert_eq!((inner["X"].as_str(), inner["Y"].as_str()), ("1", "0"), "{inner:?}");
    // The caller's frame shows its locals as they were when it made the call.
    let outer = c.locals(frames[1]["id"].as_i64().unwrap());
    assert_eq!(outer["Result"], "1", "{outer:?}");
    c.request("continue", json!({"threadId": 1}));
    assert_eq!(c.finish(), 0);
}

#[test]
fn step_out_returns_to_the_caller_after_the_call() {
    let mut c = stepping_at(TWICE_EXIT_LINE);
    let top = c.top_frame();
    assert_eq!(c.locals(top["id"].as_i64().unwrap())["Y"], "2");
    c.request("stepOut", json!({"threadId": 1}));
    let s = c.stopped();
    assert_eq!(
        (s["reason"].as_str(), s["line"].as_i64()),
        (Some("step"), Some(AFTER_CALL_LINE)),
        "{s}"
    );
    let top = c.top_frame();
    assert_eq!(c.locals(top["id"].as_i64().unwrap())["Result"], "2");
    c.request("continue", json!({"threadId": 1}));
    assert_eq!(c.finish(), 0);
}

#[test]
fn zero_based_lines_and_columns_are_answered_in_the_base_the_client_asked_for() {
    let mut c = Client::tcp(&fixture("breakpoint"), &[]);
    c.start(json!({"linesStartAt1": false, "columnsStartAt1": false}));
    let r = c.set_breakpoints(&fixture_file("breakpoint", BREAKPOINT_FILE), &[SECOND_COUNTER_LINE - 1]);
    let bp = &r["body"]["breakpoints"][0];
    assert_eq!(
        (bp["verified"].as_bool(), bp["line"].as_i64()),
        (Some(true), Some(SECOND_COUNTER_LINE - 1)),
        "{r}"
    );
    assert_eq!(bp["column"], 8, "{r}");
    c.request("configurationDone", json!({}));
    let s = c.stopped();
    assert_eq!(s["line"], SECOND_COUNTER_LINE - 1, "{s}");
    let top = c.top_frame();
    assert_eq!(
        (top["line"].as_i64(), top["column"].as_i64()),
        (Some(SECOND_COUNTER_LINE - 1), Some(0)),
        "{top}"
    );
    c.request("continue", json!({"threadId": 1}));
    assert_eq!(c.finish(), 0);
}

#[test]
fn a_second_initialize_is_refused() {
    let mut c = Client::tcp(&fixture("breakpoint"), &[]);
    c.start(json!({}));
    let r = c.request("initialize", json!({}));
    assert_eq!(r["success"], false, "{r}");
    assert!(r["message"].as_str().unwrap().contains("already initialized"), "{r}");
}

#[test]
fn the_old_lines_field_sets_breakpoints_too() {
    let mut c = Client::tcp(&fixture("breakpoint"), &[]);
    c.start(json!({}));
    let file = fixture_file("breakpoint", BREAKPOINT_FILE);
    let r = c.request(
        "setBreakpoints",
        json!({"source": {"path": file}, "lines": [SECOND_COUNTER_LINE]}),
    );
    assert_eq!(r["body"]["breakpoints"][0]["verified"], true, "{r}");
    c.request("configurationDone", json!({}));
    assert_eq!(c.stopped()["line"], SECOND_COUNTER_LINE);
    c.request("continue", json!({"threadId": 1}));
    assert_eq!(c.finish(), 0);
}

#[test]
fn breakpoints_sent_before_launch_are_answered_once_the_app_is_compiled() {
    let mut c = Client::tcp(&fixture("breakpoint"), &[]);
    let r = c.request("initialize", json!({}));
    assert_eq!(r["success"], true, "{r}");
    let file = fixture_file("breakpoint", BREAKPOINT_FILE);
    let bp = c.send(
        "setBreakpoints",
        json!({"source": {"path": file}, "breakpoints": [{"line": SECOND_COUNTER_LINE}]}),
    );
    let launch = c.send("launch", json!({}));
    let r = c.response(bp);
    assert_eq!(r["body"]["breakpoints"][0]["verified"], true, "{r}");
    assert_eq!(c.response(launch)["success"], true);
    c.request("configurationDone", json!({}));
    assert_eq!(c.stopped()["line"], SECOND_COUNTER_LINE);
    c.request("continue", json!({"threadId": 1}));
    assert_eq!(c.finish(), 0);
}

#[test]
fn a_line_without_a_statement_is_not_verified_and_replacing_a_file_drops_its_old_breakpoints() {
    let mut c = Client::tcp(&fixture("breakpoint"), &[]);
    c.start(json!({}));
    let file = fixture_file("breakpoint", BREAKPOINT_FILE);
    let r = c.set_breakpoints(&file, &[11, 8]);
    let bps = r["body"]["breakpoints"].as_array().unwrap();
    assert_eq!(bps[0]["verified"], true, "{r}");
    assert_eq!(bps[1]["verified"], false, "{r}");
    assert_eq!(bps[1]["line"], 8, "{r}");
    assert_eq!(
        bps[1]["message"], "no executable AL statement on this line in this file",
        "{r}"
    );
    assert!(bps[1].get("column").is_none(), "{r}");
    // The second request for the file replaces the first: line 11 no longer stops.
    c.set_breakpoints(&file, &[SECOND_COUNTER_LINE]);
    c.request("configurationDone", json!({}));
    assert_eq!(c.stopped()["line"], SECOND_COUNTER_LINE);
    c.request("continue", json!({"threadId": 1}));
    assert_eq!(c.finish(), 0);
}

#[test]
fn requests_outside_a_stop_and_requests_that_are_not_supported() {
    let mut c = Client::tcp(&fixture("breakpoint"), &[]);
    c.start(json!({}));
    let r = c.request("stackTrace", json!({"threadId": 1}));
    assert_eq!(
        (r["success"].as_bool(), r["message"].as_str()),
        (Some(false), Some("not stopped")),
        "{r}"
    );
    let r = c.request("setExceptionBreakpoints", json!({"filters": []}));
    assert_eq!(r["success"], true, "{r}");
    for cmd in ["evaluate", "setVariable", "source"] {
        let r = c.request(cmd, json!({}));
        assert_eq!(r["success"], false, "{r}");
        assert_eq!(r["message"], format!("unsupported command: {cmd}"), "{r}");
    }
    let r = c.request("scopes", json!({"frameId": 0}));
    assert_eq!(r["message"], "unknown frameId 0", "{r}");
}

#[test]
fn an_app_that_does_not_compile_answers_launch_with_its_first_error() {
    let mut c = Client::tcp(&fixture("broken"), &[]);
    let r = c.request("initialize", json!({}));
    assert_eq!(r["success"], true, "{r}");
    let r = c.request("launch", json!({}));
    assert_eq!(r["success"], false, "{r}");
    let m = r["message"].as_str().unwrap();
    assert!(m.contains("AL0118") && m.contains("NoSuchProcedure"), "{r}");
    let r = c.set_breakpoints(&fixture_file("broken", "BrokenTests.Codeunit.al"), &[10]);
    assert_eq!(r["body"]["breakpoints"][0]["verified"], false, "{r}");
    assert_eq!(r["body"]["breakpoints"][0]["message"], "the app did not compile", "{r}");
    let r = c.request("disconnect", json!({}));
    assert_eq!(r["success"], true, "{r}");
    c.event("terminated");
    wait(&mut c.child);
}

#[test]
fn pause_stops_a_running_test_at_its_next_statement() {
    let mut c = Client::tcp(&fixture("pause"), &["--test-timeout", "600"]);
    c.start(json!({}));
    c.request("configurationDone", json!({}));
    // The test loops until it is stopped; a pause stops it wherever it is.
    std::thread::sleep(Duration::from_millis(200));
    let r = c.request("pause", json!({"threadId": 1}));
    assert_eq!(r["success"], true, "{r}");
    let s = c.stopped();
    assert_eq!(s["reason"], "pause", "{s}");
    let top = c.top_frame();
    assert!(top["name"].as_str().unwrap().contains("LoopsForever"), "{top}");
    let r = c.request("disconnect", json!({}));
    assert_eq!(r["success"], true, "{r}");
    c.event("terminated");
    wait(&mut c.child);
}

#[test]
fn combinations_debug_does_not_take_are_refused_with_exit_code_2() {
    for extra in [vec!["--output-json"], vec!["--impact-of", "x"], vec!["--explain", "T"]] {
        let mut args = vec!["debug", "--port", "0", "app"];
        args.extend(extra.iter().copied());
        let o = command(&args).output().unwrap();
        assert_eq!(o.status.code(), Some(2), "{extra:?}");
        let err = String::from_utf8_lossy(&o.stderr);
        assert!(err.contains("al-runner debug cannot be combined with"), "{err}");
    }
}
