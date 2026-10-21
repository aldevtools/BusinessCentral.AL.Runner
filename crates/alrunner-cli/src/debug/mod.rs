//! `al-runner debug` (ADR-048): the tests of `al-runner test` under a Debug Adapter Protocol client, over stdio or TCP,
//! with the requests, capabilities and events of v2's `--dap`, so a client of v2 works unchanged.
//!
//! Threads (decision 3): this thread reads the client's requests and answers them; one worker thread compiles at once
//! and runs the tests one after the other. The worker's debugger ([`Hook`]) stops it on a condition variable and
//! sends `stopped`; `stackTrace`, `scopes` and `variables` are answered from the snapshot it took when it stopped, so
//! this thread never touches the VM.

pub mod breakpoints;
pub mod wire;

use breakpoints::{Index, Request, Resolved};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use wire::Out;

use crate::args::TestArgs;

/// Where the client is (`--stdio` or `--port N`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    Stdio,
    /// `127.0.0.1:N`; 0: a free port, which the listening line on stderr names.
    Tcp(u16),
}

/// The port of `--port` without a number, as v2's `--dap`.
pub const DEFAULT_PORT: u16 = 4711;

/// The one thread a debugged run reports.
const THREAD_ID: i64 = 1;
const THREAD_NAME: &str = "AL Test Thread";

/// A step the client asked for, with the call depth it was asked at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Step {
    #[default]
    None,
    /// `stepIn`: the next statement, at any depth.
    In,
    /// `next`: the next statement at this depth or above.
    Over(usize),
    /// `stepOut`: the next statement above this depth.
    Out(usize),
}

/// A frame of a stop, as the client sees it.
#[derive(Debug, Clone)]
struct Frame {
    name: String,
    /// The file on disk, with line and column (1-based); `None`: code without a file (an app given as a package).
    at: Option<(PathBuf, u32, u32)>,
    locals: Vec<(String, String)>,
}

#[derive(Default)]
enum Compile {
    #[default]
    Pending,
    Ready(Arc<Index>),
    /// The app did not compile, or the run ended before it was compiled: the first diagnostic.
    Failed(String),
}

#[derive(Default)]
struct State {
    initialized: bool,
    lines_from_1: bool,
    columns_from_1: bool,
    compile: Compile,
    /// A `launch` or `attach` waiting for the compile.
    launch: Option<(i64, String)>,
    /// `setBreakpoints` requests waiting for the compile: (seq, path, breakpoints).
    deferred: Vec<(i64, String, Vec<Request>)>,
    configuration_done: bool,
    /// The breakpoints by file (its key), and all of them as position ids.
    by_file: HashMap<String, Vec<u32>>,
    armed: HashSet<u32>,
    step: Step,
    pause: bool,
    /// The frames of the stop, innermost first; `None` while the test runs.
    stopped: Option<Vec<Frame>>,
    /// The depth of the call stack at the stop.
    stopped_depth: usize,
    /// The client let go: nothing stops any more.
    detached: bool,
    /// The run ended with this exit code.
    finished: Option<u8>,
    terminated_sent: bool,
}

struct Shared {
    out: Out,
    state: Mutex<State>,
    wake: Condvar,
    paused: Arc<al_testrun::events::PauseClock>,
    /// The sources of the run's apps, which the run fills in once it has read them.
    sources: Arc<Mutex<Option<breakpoints::Sources>>>,
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn line_out(st: &State, line: u32) -> i64 {
        if st.lines_from_1 || line == 0 {
            i64::from(line)
        } else {
            i64::from(line) - 1
        }
    }

    fn column_out(st: &State, col: u32) -> i64 {
        if st.columns_from_1 || col == 0 {
            i64::from(col)
        } else {
            i64::from(col) - 1
        }
    }

    fn terminate_once(&self, st: &mut State) {
        if st.terminated_sent {
            return;
        }
        st.terminated_sent = true;
        self.out.event("terminated", None);
        self.out
            .event("exited", Some(json!({"exitCode": st.finished.unwrap_or(0)})));
    }

    /// Answers a `setBreakpoints` once the compile is known, and arms what it verified (replacing the file's earlier
    /// breakpoints).
    fn answer_breakpoints(&self, st: &mut State, seq: i64, path: &str, requests: &[Request]) {
        let resolved: Vec<Resolved> = match &st.compile {
            Compile::Ready(ix) => ix.resolve(path, requests),
            _ => breakpoints::unverified(requests, breakpoints::NOT_COMPILED),
        };
        let file = breakpoints::key(Path::new(path));
        if let Some(old) = st.by_file.remove(&file) {
            for p in old {
                st.armed.remove(&p);
            }
        }
        let targets: Vec<u32> = resolved.iter().flat_map(|r| r.targets.iter().copied()).collect();
        st.armed.extend(targets.iter().copied());
        st.by_file.insert(file, targets);
        let list: Vec<Value> = resolved
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let mut b = serde_json::Map::new();
                b.insert("id".into(), json!(i));
                b.insert("verified".into(), json!(r.verified));
                b.insert("line".into(), json!(Shared::line_out(st, r.line)));
                if let Some(c) = r.column.filter(|_| r.verified) {
                    b.insert("column".into(), json!(Shared::column_out(st, c)));
                }
                if let Some(m) = &r.message {
                    b.insert("message".into(), json!(m));
                }
                Value::Object(b)
            })
            .collect();
        self.out.ok(seq, "setBreakpoints", Some(json!({ "breakpoints": list })));
    }

    /// The compile is known: answer what waited for it.
    fn compile_known(&self, st: &mut State) {
        for (seq, path, requests) in std::mem::take(&mut st.deferred) {
            self.answer_breakpoints(st, seq, &path, &requests);
        }
        if let Some((seq, command)) = st.launch.take() {
            match &st.compile {
                Compile::Failed(why) => self.out.fail(seq, &command, why),
                _ => self.out.ok(seq, &command, None),
            }
        }
    }
}

/// The debugger of a test session: stops at breakpoints, steps and pauses.
struct Hook {
    shared: Arc<Shared>,
    index: Arc<Index>,
}

impl al_vm::DebugHook for Hook {
    fn statement(&mut self, view: &al_vm::DebugView<'_>) -> al_vm::Resume {
        let pos = view.pos();
        let depth = view.depth();
        let mut st = self.shared.lock();
        if st.detached {
            return al_vm::Resume::Continue;
        }
        let breakpoint = st.armed.contains(&pos);
        // Steps and pauses stop only in code with a source (decision 7): an app folder or a source package.
        let here = || self.index.has_source(pos);
        let step = match st.step {
            Step::None => false,
            Step::In => here(),
            Step::Over(d) => depth <= d && here(),
            Step::Out(d) => depth < d && here(),
        };
        let pause = st.pause && here();
        if !breakpoint && !step && !pause {
            return al_vm::Resume::Continue;
        }
        // A step that lands on a breakpoint reports the breakpoint (v2).
        let reason = if breakpoint {
            "breakpoint"
        } else if step {
            "step"
        } else {
            "pause"
        };
        st.step = Step::None;
        st.pause = false;
        let program = view.program();
        let frames: Vec<Frame> = view
            .frames()
            .into_iter()
            .map(|f| {
                let o = &program.objects[f.object];
                let name = o
                    .routines
                    .get(f.routine)
                    .map_or_else(|| o.name.clone(), |r| format!("{}.{}", o.name, r.name));
                let at = self.index.location(f.pos);
                let locals = f.locals.iter().map(|(n, v)| (n.clone(), wire_value(v))).collect();
                Frame { name, at, locals }
            })
            .collect();
        let line = frames
            .first()
            .and_then(|f| f.at.as_ref())
            .map(|(_, l, _)| Shared::line_out(&st, *l));
        st.stopped = Some(frames);
        st.stopped_depth = depth;
        self.shared.paused.stop();
        let mut body = json!({"reason": reason, "threadId": THREAD_ID, "allThreadsStopped": true});
        if let Some(l) = line {
            body["line"] = json!(l);
        }
        self.shared.out.event("stopped", Some(body));
        while st.stopped.is_some() && !st.detached {
            st = self.shared.wake.wait(st).unwrap_or_else(|e| e.into_inner());
        }
        self.shared.paused.resume();
        al_vm::Resume::Continue
    }
}

/// Where the sources of a run's apps come from: the APPs of the command line, then the dependency apps (the caller's
/// app order of `al-runner test`). Every app read from a folder or a source package has a source the debugger shows.
pub fn sources_of(
    inputs: &[crate::apps::AppInput],
    deps: &al_testrun::deps::Dependencies,
    written: PathBuf,
) -> breakpoints::Sources {
    use breakpoints::{Origin, PackageText};
    let label = |publisher: &str, name: &str, version: &str| format!("{publisher}_{name}_{version}");
    let by_path = |files: &[(String, String)]| -> HashMap<String, usize> {
        files
            .iter()
            .enumerate()
            .map(|(i, (p, _))| (p.replace('\\', "/"), i))
            .collect()
    };
    let eager = |app: &al_sema::workspace::AppSource| -> PackageText {
        let texts: HashMap<String, String> = app
            .files
            .iter()
            .map(|(p, t)| (p.replace('\\', "/"), t.clone()))
            .collect();
        Arc::new(move |p: &str| texts.get(p).cloned())
    };
    let mut origins = Vec::with_capacity(inputs.len() + deps.apps.len());
    for i in inputs {
        origins.push(Some(if i.path.is_dir() {
            Origin::Folder {
                dir: i.path.clone(),
                prefix: String::new(),
            }
        } else {
            Origin::Package {
                label: label(&i.manifest.publisher, &i.source.name, &i.manifest.version),
                prefix: String::new(),
                text: eager(&i.source),
            }
        }));
    }
    for (j, app) in deps.apps.iter().enumerate() {
        let path = deps.paths.get(j);
        let manifest = deps.manifests.get(j);
        // The program names a file of a dependency `<app name>/<path in the app>` (`al_deps::load`).
        let prefix = format!("{}/", app.name);
        let origin = if path.is_some_and(|p| p.is_dir()) {
            Origin::Folder {
                dir: path.cloned().unwrap_or_default(),
                prefix,
            }
        } else {
            let text = match deps.lazy.get(j).and_then(Option::as_ref) {
                Some(lazy) => {
                    let index = by_path(&app.files);
                    let loader = lazy.loader.clone();
                    Arc::new(move |p: &str| index.get(p).map(|&i| loader.text(i))) as PackageText
                }
                None => eager(app),
            };
            Origin::Package {
                label: label(
                    manifest.map_or("", |m| m.publisher.as_str()),
                    &app.name,
                    manifest.map_or("", |m| m.version.as_str()),
                ),
                prefix,
                text,
            }
        };
        origins.push(Some(origin));
    }
    breakpoints::Sources {
        origins,
        given: inputs.len(),
        written,
    }
}

/// A value as v2 wrote it (`AlValueWireFormat`): a number bare, a text in JSON quotes.
fn wire_value(v: &al_vm::Rendered) -> String {
    use al_vm::Rendered as R;
    match v {
        R::Null => "null".into(),
        R::Bool(b) => b.to_string(),
        R::Int(i) => i.to_string(),
        R::Num(n) => n.clone(),
        R::Text(t) => serde_json::to_string(t).unwrap_or_default(),
        R::Unrendered(kind) => format!("<not rendered: {kind}>"),
    }
}

/// The outcome of a test as the `[debug]` line names it.
fn outcome_word(o: &al_testrun::Outcome) -> &'static str {
    match o {
        al_testrun::Outcome::Pass => "Pass",
        al_testrun::Outcome::Fail(_) => "Fail",
        al_testrun::Outcome::NotRun(_) => "NotRun",
    }
}

/// `al-runner debug`: serves one client, runs the selected tests once, and returns the exit code of the run.
pub fn run(args: &TestArgs, transport: Transport) -> Result<u8, String> {
    let (reader, writer): (Box<dyn Read + Send>, Box<dyn Write + Send>) = match transport {
        Transport::Stdio => {
            let out = stdio_frames_only()?;
            eprintln!("al-runner debug: waiting for a client on stdin");
            (Box::new(std::io::stdin()), out)
        }
        Transport::Tcp(port) => {
            let listener = std::net::TcpListener::bind(("127.0.0.1", port))
                .map_err(|e| format!("al-runner debug: cannot listen on 127.0.0.1:{port}: {e}"))?;
            let port = listener.local_addr().map(|a| a.port()).unwrap_or(port);
            eprintln!("al-runner debug: listening on 127.0.0.1:{port}");
            let (stream, _) = listener.accept().map_err(|e| format!("al-runner debug: accept: {e}"))?;
            // One client per process: later connections are refused.
            drop(listener);
            let _ = stream.set_nodelay(true);
            let w = stream.try_clone().map_err(|e| format!("al-runner debug: {e}"))?;
            (Box::new(stream), Box::new(w))
        }
    };
    let shared = Arc::new(Shared {
        out: Out::new(writer),
        state: Mutex::new(State {
            lines_from_1: true,
            columns_from_1: true,
            step: Step::None,
            ..Default::default()
        }),
        wake: Condvar::new(),
        paused: Arc::new(Default::default()),
        sources: Arc::new(Mutex::new(None)),
    });
    start_worker(args.clone(), shared.clone())?;
    let code = serve(shared.clone(), BufReader::new(reader));
    Ok(code)
}

/// On stdio, stdout carries only frames: the frames get a stream of their own, and everything else the process writes
/// to its standard output goes to stderr.
#[cfg(unix)]
fn stdio_frames_only() -> Result<Box<dyn Write + Send>, String> {
    use std::io::Write as _;
    use std::os::fd::FromRawFd;
    let _ = std::io::stdout().flush();
    // SAFETY: plain descriptor calls on the process's own standard descriptors; the new descriptor is owned by the
    // returned file alone.
    unsafe {
        let frames = libc::dup(1);
        if frames < 0 {
            return Err("al-runner debug: cannot duplicate stdout".into());
        }
        if libc::dup2(2, 1) < 0 {
            return Err("al-runner debug: cannot point stdout at stderr".into());
        }
        Ok(Box::new(std::fs::File::from_raw_fd(frames)))
    }
}

/// Elsewhere the report of the run is sent to stderr (`run::test_debug`) and the frames use stdout.
#[cfg(not(unix))]
fn stdio_frames_only() -> Result<Box<dyn Write + Send>, String> {
    Ok(Box::new(std::io::stdout()))
}

/// Starts the run: compile at once, wait for `configurationDone`, then the tests.
fn start_worker(args: TestArgs, shared: Arc<Shared>) -> Result<(), String> {
    let ready_shared = shared.clone();
    let hook_index: Arc<Mutex<Option<Arc<Index>>>> = Arc::new(Mutex::new(None));
    let ready_index = hook_index.clone();
    let ready = al_testrun::runtime::DebugReadyHook(Arc::new(move |r: &al_testrun::runtime::DebugReady| {
        let s = &ready_shared;
        let mut st = s.lock();
        st.compile = match &r.first_error {
            Some(e) => Compile::Failed(e.clone()),
            None => {
                let sources = s
                    .sources
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .clone()
                    .unwrap_or_default();
                let ix = Arc::new(Index::build(r.program.clone(), r.order, &sources));
                *ready_index.lock().unwrap_or_else(|e| e.into_inner()) = Some(ix.clone());
                Compile::Ready(ix)
            }
        };
        s.compile_known(&mut st);
        // The tests start when the client says so; a compile error runs nothing (the client ends the session).
        let failed = matches!(st.compile, Compile::Failed(_));
        while (!st.configuration_done || failed) && !st.detached {
            st = s.wake.wait(st).unwrap_or_else(|e| e.into_inner());
        }
    }));
    let hook_shared = shared.clone();
    let debugger: al_testrun::events::MakeDebugger = Arc::new(move || {
        let index = hook_index
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
            .expect("the debugger is made after the compile");
        Box::new(Hook {
            shared: hook_shared.clone(),
            index,
        }) as Box<dyn al_vm::DebugHook>
    });
    let test_shared = shared.clone();
    let on_test = Arc::new(move |e: al_testrun::events::TestEvent| {
        let line = format!("[debug] {}.{}: {}", e.codeunit_name, e.test, outcome_word(&e.outcome));
        eprintln!("{line}");
        test_shared.out.event(
            "output",
            Some(json!({"category": "console", "output": format!("{line}\n")})),
        );
        // A step that found no statement before its test ended does not carry over to the next test.
        test_shared.lock().step = Step::None;
    });
    let setup = crate::run::DebugSetup {
        ready,
        debugger,
        paused: shared.paused.clone(),
        on_test,
        sources: shared.sources.clone(),
    };
    std::thread::Builder::new()
        .name("al-runner debug".into())
        .stack_size(512 << 20)
        .spawn(move || {
            let result = crate::run::test_debug(&args, setup);
            let mut st = shared.lock();
            let code = match result {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("error: {e}");
                    if matches!(st.compile, Compile::Pending) {
                        st.compile = Compile::Failed(e);
                        shared.compile_known(&mut st);
                    }
                    2
                }
            };
            st.finished = Some(code);
            shared.terminate_once(&mut st);
        })
        .map_err(|e| format!("al-runner debug: cannot start the run: {e}"))?;
    Ok(())
}

/// Reads and answers requests until the client disconnects or goes away. Returns the exit code.
fn serve(shared: Arc<Shared>, mut reader: impl BufRead) -> u8 {
    loop {
        let msg = match wire::read_message(&mut reader) {
            Ok(Some(m)) => m,
            Ok(None) => break,
            Err(e) => {
                eprintln!("al-runner debug: {e}");
                break;
            }
        };
        if msg["type"] != "request" {
            continue;
        }
        let seq = msg["seq"].as_i64().unwrap_or(0);
        let command = msg["command"].as_str().unwrap_or_default().to_string();
        let args = &msg["arguments"];
        if handle(&shared, seq, &command, args) {
            break;
        }
    }
    let mut st = shared.lock();
    st.detached = true;
    shared.wake.notify_all();
    shared.terminate_once(&mut st);
    st.finished.unwrap_or(0)
}

/// Answers one request; true when the session ends.
fn handle(shared: &Shared, seq: i64, command: &str, args: &Value) -> bool {
    let out = &shared.out;
    let mut st = shared.lock();
    match command {
        "initialize" => {
            if st.initialized {
                out.fail(
                    seq,
                    command,
                    "initialize: this session is already initialized; initialize is allowed once, as the first request",
                );
                return false;
            }
            st.initialized = true;
            st.lines_from_1 = args["linesStartAt1"] != false;
            st.columns_from_1 = args["columnsStartAt1"] != false;
            out.ok(seq, command, Some(json!({"supportsConfigurationDoneRequest": true})));
            out.event("initialized", None);
        }
        "launch" | "attach" => {
            st.launch = Some((seq, command.to_string()));
            if !matches!(st.compile, Compile::Pending) {
                shared.compile_known(&mut st);
            }
        }
        "setBreakpoints" => {
            let Some(path) = args["source"]["path"].as_str() else {
                out.fail(seq, command, "setBreakpoints: missing source.path");
                return false;
            };
            let line_in = |l: i64| if st.lines_from_1 { l } else { l + 1 };
            let column_in = |c: i64| if st.columns_from_1 { c } else { c + 1 };
            let to_u32 = |v: i64| u32::try_from(v).unwrap_or(0);
            let mut requests = Vec::new();
            if let Some(list) = args["breakpoints"].as_array() {
                for b in list {
                    if let Some(l) = b["line"].as_i64() {
                        requests.push(Request {
                            line: to_u32(line_in(l)),
                            column: b["column"].as_i64().map(|c| to_u32(column_in(c))),
                        });
                    }
                }
            } else if let Some(list) = args["lines"].as_array() {
                for l in list.iter().filter_map(Value::as_i64) {
                    requests.push(Request {
                        line: to_u32(line_in(l)),
                        column: None,
                    });
                }
            }
            // Before the compile is done, the answer waits for it; so does a later request for a file that waits.
            let file = breakpoints::key(Path::new(path));
            let waits_for_file = st
                .deferred
                .iter()
                .any(|(_, p, _)| breakpoints::key(Path::new(p)) == file);
            if (matches!(st.compile, Compile::Pending) && !requests.is_empty()) || waits_for_file {
                st.deferred.push((seq, path.to_string(), requests));
            } else {
                shared.answer_breakpoints(&mut st, seq, path, &requests);
            }
        }
        "setExceptionBreakpoints" => {
            // No exception filters; answered so a client does not log an error (decision 5).
            out.ok(seq, command, Some(json!({"breakpoints": []})));
        }
        "configurationDone" => {
            st.configuration_done = true;
            out.ok(seq, command, None);
            shared.wake.notify_all();
        }
        "threads" => {
            out.ok(
                seq,
                command,
                Some(json!({"threads": [{"id": THREAD_ID, "name": THREAD_NAME}]})),
            );
        }
        "stackTrace" => {
            let Some(frames) = &st.stopped else {
                out.fail(seq, command, "not stopped");
                return false;
            };
            let list: Vec<Value> = frames
                .iter()
                .enumerate()
                .map(|(id, f)| match &f.at {
                    Some((path, line, _)) => json!({
                        "id": id,
                        "name": f.name,
                        "source": {
                            "path": path.to_string_lossy(),
                            "name": path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
                        },
                        "line": Shared::line_out(&st, *line),
                        "column": Shared::column_out(&st, 1),
                    }),
                    None => json!({"id": id, "name": f.name, "line": 0, "column": 0}),
                })
                .collect();
            let total = list.len();
            out.ok(seq, command, Some(json!({"stackFrames": list, "totalFrames": total})));
        }
        "scopes" => {
            let frame = args["frameId"].as_i64().unwrap_or(-1);
            let known = st
                .stopped
                .as_ref()
                .is_some_and(|f| usize::try_from(frame).is_ok_and(|i| i < f.len()));
            if !known {
                out.fail(seq, command, &format!("unknown frameId {frame}"));
                return false;
            }
            // The handle of a frame's locals: its id plus one (0 means "no children" in the protocol).
            out.ok(
                seq,
                command,
                Some(json!({"scopes": [{"name": "Locals", "variablesReference": frame + 1, "expensive": false}]})),
            );
        }
        "variables" => {
            let handle = args["variablesReference"].as_i64().unwrap_or(-1);
            let frame = st
                .stopped
                .as_ref()
                .and_then(|f| usize::try_from(handle - 1).ok().and_then(|i| f.get(i)));
            let Some(frame) = frame else {
                out.fail(seq, command, &format!("unknown variablesReference {handle}"));
                return false;
            };
            let list: Vec<Value> = frame
                .locals
                .iter()
                .map(|(n, v)| json!({"name": n, "value": v, "variablesReference": 0}))
                .collect();
            out.ok(seq, command, Some(json!({"variables": list})));
        }
        "continue" => {
            st.step = Step::None;
            st.stopped = None;
            shared.wake.notify_all();
            out.ok(seq, command, Some(json!({"allThreadsContinued": true})));
        }
        "next" | "stepIn" | "stepOut" => {
            if st.stopped.is_some() {
                let d = st.stopped_depth;
                st.step = match command {
                    "next" => Step::Over(d),
                    "stepIn" => Step::In,
                    _ => Step::Out(d),
                };
                st.stopped = None;
                shared.wake.notify_all();
            }
            out.ok(seq, command, None);
        }
        "pause" => {
            // Stops at the next statement (decision 5; v2 continued instead).
            if st.stopped.is_none() {
                st.pause = true;
            }
            out.ok(seq, command, None);
        }
        "disconnect" | "terminate" => {
            st.detached = true;
            st.stopped = None;
            shared.wake.notify_all();
            out.ok(seq, command, None);
            shared.terminate_once(&mut st);
            return true;
        }
        _ => out.fail(seq, command, &format!("unsupported command: {command}")),
    }
    false
}
