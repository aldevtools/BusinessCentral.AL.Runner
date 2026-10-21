//! `al-runner serve` (ADR-028): the v2 server protocol on stdin and stdout, in this process. The compile of the
//! last request is kept (`WarmCtx`), a test line is written as each test finishes, and `cancel` stops the run after the
//! current test.

use crate::args::{Command, TestArgs};
use crate::run::{self, WarmCtx};
use serde_json::{Map, Value, json};
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;

/// The request fields of `runTests` and what each becomes.
const RUN_TESTS_FIELDS: &[&str] = &[
    "command",
    "sourcePaths",
    "packagePaths",
    "coverage",
    "perTestCoverage",
    "test",
    "excludeTests",
    "testIsolation",
    "captureValues",
    "iterationTracking",
    "announceStart",
    "serverUrl",
    "tdd",
];
/// The request fields of `execute`.
const EXECUTE_FIELDS: &[&str] = &[
    "command",
    "sourcePaths",
    "code",
    "packagePaths",
    "captureValues",
    "iterationTracking",
    "coverage",
    "perTestCoverage",
    "testIsolation",
    "announceStart",
];
/// Fields v2 knows and v3 defers (ADR-016).
const DEFERRED: &[&str] = &["affectedOnly", "includeFailing", "strictEnvironment"];
/// Fields v2 reads for another command only: accepted, with a warning.
const OTHER_COMMAND: &[&str] = &["code"];

fn out(v: &Value) {
    let mut o = std::io::stdout().lock();
    let _ = writeln!(o, "{v}");
    let _ = o.flush();
}

fn error(message: impl Into<String>) {
    out(&json!({ "error": message.into() }));
}

enum Input {
    Line(String),
    Eof,
}

/// A run is in progress, and a `cancel` or the end of input asks it to stop after the current test.
#[derive(Default)]
struct Flags {
    running: AtomicBool,
    cancel: Arc<AtomicBool>,
    /// The request asked for a `started` line, and it was not written yet.
    announce: AtomicBool,
}

/// Reads stdin on a thread of its own: `cancel` is answered here, at once, also while a run keeps the main thread busy;
/// everything else is queued for the main loop.
fn reader(flags: Arc<Flags>) -> mpsc::Receiver<Input> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            let Ok(line) = line else { break };
            if line.trim().is_empty() {
                continue;
            }
            let command = serde_json::from_str::<Value>(&line)
                .ok()
                .and_then(|v| v["command"].as_str().map(str::to_ascii_lowercase));
            match command.as_deref() {
                Some("cancel") => {
                    let running = flags.running.load(Ordering::SeqCst);
                    if running {
                        flags.cancel.store(true, Ordering::SeqCst);
                    }
                    out(&json!({ "type": "ack", "command": "cancel", "noop": !running }));
                    continue;
                }
                // The run in progress stops, then the loop sees the request.
                Some("shutdown") => flags.cancel.store(true, Ordering::SeqCst),
                _ => {}
            }
            if tx.send(Input::Line(line)).is_err() {
                return;
            }
        }
        flags.cancel.store(true, Ordering::SeqCst);
        let _ = tx.send(Input::Eof);
    });
    rx
}

pub fn serve(daemon: &[String]) -> Result<(), String> {
    let base = daemon_args(daemon)?;
    let flags = Arc::new(Flags::default());
    let rx = reader(flags.clone());
    let mut ctx = WarmCtx {
        cancel: Some(flags.cancel.clone()),
        on_test: Some(Arc::new(|e: al_testrun::events::TestEvent, captured: Option<Value>| {
            let mut row =
                crate::output::json_row(e.codeunit, &e.codeunit_name, &e.test, &e.outcome, &e.call_stack, e.time);
            crate::output::add_stubs(&mut row, &e.stubs);
            let mut line = test_line(&mut row);
            // What the test's code did (`captureValues`, `iterationTracking`): the values and loops are part of the line.
            if let Some(Value::Object(c)) = captured {
                for key in ["capturedValues", "loops", "capturedValuesTruncated"] {
                    if let Some(v) = c.get(key) {
                        line.insert(key.into(), v.clone());
                    }
                }
            }
            out(&Value::Object(line));
        })),
        on_start: Some({
            let flags = flags.clone();
            Arc::new(move |codeunit: u32, test: &str| {
                if flags.announce.swap(false, Ordering::SeqCst) {
                    out(&json!({ "type": "started", "codeunit": codeunit, "test": test }));
                }
            })
        }),
        ..WarmCtx::default()
    };
    crate::run::keep_backups();
    out(&json!({ "ready": true }));
    let mut n = 0u64;
    while let Ok(Input::Line(line)) = rx.recv() {
        let request: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                error(format!("not a JSON object: {e}"));
                continue;
            }
        };
        let Some(fields) = request.as_object() else {
            error("a request is a JSON object");
            continue;
        };
        let command = fields
            .get("command")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        match command.as_str() {
            "shutdown" => {
                out(&json!({ "status": "shutting down" }));
                eprintln!("[server] request {} done", n + 1);
                return Ok(());
            }
            "runtests" => run_tests(&base, fields, &mut ctx, &flags),
            "execute" => execute(&base, fields, &mut ctx, &flags),
            "" => error("a request needs a `command`"),
            other => error(format!("unknown command `{other}`")),
        }
        n += 1;
        eprintln!("[server] request {n} done");
    }
    Ok(())
}

/// The `al-runner test` options of the daemon, as the template of every request's.
fn daemon_args(daemon: &[String]) -> Result<TestArgs, String> {
    let mut probe = vec!["test".to_string(), "APP".to_string()];
    probe.extend(daemon.iter().cloned());
    match crate::args::parse(probe)? {
        Command::Test(t) => Ok(*t),
        _ => Err("al-runner serve: options of `al-runner test` only".into()),
    }
}

fn string_list(fields: &Map<String, Value>, name: &str) -> Result<Vec<String>, String> {
    match fields.get(name) {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Array(a)) => a
            .iter()
            .map(|v| {
                v.as_str()
                    .map(String::from)
                    .ok_or_else(|| format!("`{name}` holds only strings"))
            })
            .collect(),
        Some(_) => Err(format!("`{name}` is a list of strings")),
    }
}

fn flag(fields: &Map<String, Value>, name: &str) -> Result<bool, String> {
    match fields.get(name) {
        None | Some(Value::Null) => Ok(false),
        Some(Value::Bool(b)) => Ok(*b),
        Some(_) => Err(format!("`{name}` is true or false")),
    }
}

/// The `al-runner test` arguments for a request, and the warnings the request earns.
fn request_args(base: &TestArgs, fields: &Map<String, Value>) -> Result<(TestArgs, Vec<String>), String> {
    let mut warnings = Vec::new();
    for key in fields.keys() {
        if DEFERRED.contains(&key.as_str()) {
            return Err(format!(
                "`{key}` is deferred in v3 (ADR-016): it only hides run cost and waits until it is measured to matter"
            ));
        }
        if OTHER_COMMAND.contains(&key.as_str()) {
            warnings.push(format!("`{key}` is read by `execute` only; `runTests` ignores it"));
        } else if !RUN_TESTS_FIELDS.contains(&key.as_str()) {
            return Err(format!("unknown field `{key}` for runTests"));
        }
    }
    let sources = string_list(fields, "sourcePaths")?;
    if sources.is_empty() {
        return Err("`sourcePaths` needs at least one app folder".into());
    }
    let mut a = base.clone();
    a.apps = sources.into_iter().map(PathBuf::from).collect();
    a.package_caches
        .extend(string_list(fields, "packagePaths")?.into_iter().map(PathBuf::from));
    if let Some(t) = fields.get("test").and_then(Value::as_str).filter(|t| !t.is_empty()) {
        a.filters.push(t.into());
    }
    a.excludes.extend(string_list(fields, "excludeTests")?);
    if let Some(i) = fields.get("testIsolation").and_then(Value::as_str) {
        a.isolation = Some(match i.to_ascii_lowercase().as_str() {
            "codeunit" => al_testrun::isolation::TestIsolation::Codeunit,
            "test" => al_testrun::isolation::TestIsolation::Function,
            "disabled" => al_testrun::isolation::TestIsolation::Disabled,
            _ => return Err(format!("`testIsolation` is codeunit, test or disabled, not `{i}`")),
        });
    }
    a.coverage = flag(fields, "coverage")? || flag(fields, "perTestCoverage")?;
    a.capture = al_vm::CaptureConfig {
        values: flag(fields, "captureValues")?,
        loops: flag(fields, "iterationTracking")?,
    };
    if let Some(u) = fields
        .get("serverUrl")
        .and_then(Value::as_str)
        .filter(|u| !u.is_empty())
    {
        a.server_url = Some(al_host::check_server_url(u).map_err(|e| format!("`serverUrl`: {e}"))?);
    }
    // ADR-051 decision 1: `tdd` turns stubs on for this request; absent, the server's `--tdd` decides.
    if !matches!(fields.get("tdd"), None | Some(Value::Null)) {
        a.tdd = flag(fields, "tdd")?;
    }
    a.quiet = true;
    Ok((a, warnings))
}

fn run_tests(base: &TestArgs, fields: &Map<String, Value>, ctx: &mut WarmCtx, flags: &Flags) {
    let (args, warnings) = match request_args(base, fields) {
        Ok(x) => x,
        Err(e) => return error(e),
    };
    let (want_coverage, want_per_test) = (
        flag(fields, "coverage").unwrap_or(false) || flag(fields, "perTestCoverage").unwrap_or(false),
        flag(fields, "perTestCoverage").unwrap_or(false),
    );
    // A cancel that arrived while nothing ran was answered as a no-op: it does not stop the next run.
    flags.cancel.store(false, Ordering::SeqCst);
    flags
        .announce
        .store(flag(fields, "announceStart").unwrap_or(false), Ordering::SeqCst);
    flags.running.store(true, Ordering::SeqCst);
    ctx.report = None;
    ctx.used = None;
    let started = std::time::Instant::now();
    // A panic anywhere in a request ends the request, not the server; what is kept is dropped, as it may be inconsistent.
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run::test_with(&args, Some(ctx))));
    flags.running.store(false, Ordering::SeqCst);
    let cancelled = flags.cancel.swap(false, Ordering::SeqCst);
    match outcome {
        Err(p) => {
            ctx.runtime.clear();
            let msg = p
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_default();
            error(format!("the run panicked (a runtime bug): {msg}"));
        }
        Ok(Err(e)) => error(e),
        Ok(Ok(code)) => {
            let Some(report) = ctx.report.take() else {
                return error("the run produced no report");
            };
            let doc = &report.doc;
            // ADR-028 item 7: a cancelled run (also one that `shutdown` or the end of input stopped) has exit code
            // 130, as in v2, whatever the tests that ran came to.
            let code = if cancelled { 130 } else { code };
            let mut warnings = warnings;
            warnings.extend(report.warnings.iter().cloned());
            let mut summary = json!({
                "type": "summary",
                "exitCode": code,
                "passed": doc["passed"], "failed": doc["failed"], "errors": doc["errors"],
                "notRun": doc["notRun"], "total": doc["total"],
                "cached": ctx.used == Some(al_testrun::runtime::WarmUse::Reused),
                "protocolVersion": 2,
                "wallSeconds": started.elapsed().as_secs_f64(),
            });
            if ctx.used == Some(al_testrun::runtime::WarmUse::Updated) {
                // Only the code of some routines changed: only those objects were compiled again.
                summary["incremental"] = json!(true);
            }
            if cancelled {
                summary["cancelled"] = json!(true);
            }
            // `--tdd` (ADR-051 decision 8): the stubbed counts and every stub of the program, as in `--output-json`.
            for key in ["stubbed", "generatedStubs"] {
                if let Some(v) = doc.get(key) {
                    summary[key] = v.clone();
                }
            }
            if !warnings.is_empty() {
                summary["warnings"] = json!(warnings);
            }
            if let Some(cov) = &report.coverage {
                if want_coverage {
                    summary["coverage"] = cov["coverage"].clone();
                }
                if want_per_test {
                    summary["perTestCoverage"] = cov["perTestCoverage"].clone();
                }
            }
            out(&summary);
        }
    }
}

/// The object keywords of AL: a `code` that starts with one (after white space and comments) is a whole object.
const OBJECT_KEYWORDS: &[&str] = &[
    "codeunit",
    "table",
    "tableextension",
    "page",
    "pageextension",
    "pagecustomization",
    "report",
    "reportextension",
    "query",
    "xmlport",
    "enum",
    "enumextension",
    "interface",
    "permissionset",
    "permissionsetextension",
    "profile",
    "controladdin",
    "entitlement",
    "dotnet",
];

/// Whether `code` is a declaration of an AL object (`// comments` and `/* comments */` first, then an object keyword),
/// as opposed to a list of statements.
fn is_object_declaration(code: &str) -> bool {
    let mut rest = code.trim_start_matches('\u{feff}');
    loop {
        rest = rest.trim_start();
        if let Some(after) = rest.strip_prefix("//") {
            rest = after.split_once('\n').map_or("", |(_, tail)| tail);
        } else if let Some(after) = rest.strip_prefix("/*") {
            rest = after.split_once("*/").map_or("", |(_, tail)| tail);
        } else {
            break;
        }
    }
    let word: String = rest.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
    OBJECT_KEYWORDS.iter().any(|k| word.eq_ignore_ascii_case(k)) && rest[word.len()..].starts_with(char::is_whitespace)
}

/// The folder of the inline app a request with `code` runs, written fresh: an `app.json` and one file. Without
/// Microsoft's apps in sight the app has no dependencies (it runs at once); with `--bc-version`, `--dep`,
/// `--package-cache` or `packagePaths` it is built for that BC version and sees the Base Application.
fn inline_app(code: &str, with_microsoft: bool, bc_version: Option<&str>) -> Result<PathBuf, String> {
    let dir = std::env::temp_dir().join(format!("al-runner-execute-{}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut app = json!({
        "id": "6f3f1a4e-3d52-4c1e-9d4b-0a1c2e3f4a5b",
        "name": "AL Runner Inline Execute",
        "publisher": "AL Runner",
        "version": "1.0.0.0",
        "idRanges": [{ "from": 50000, "to": 99999 }],
    });
    if with_microsoft {
        let major = bc_version
            .and_then(|v| v.split('.').next())
            .filter(|m| m.chars().all(|c| c.is_ascii_digit()) && !m.is_empty())
            .unwrap_or("27");
        app["platform"] = json!(format!("{major}.0.0.0"));
        app["application"] = json!(format!("{major}.0.0.0"));
    }
    std::fs::write(dir.join("app.json"), app.to_string()).map_err(|e| e.to_string())?;
    let source = if is_object_declaration(code) {
        code.to_string()
    } else {
        format!(
            "codeunit 50100 \"AL Runner Inline Execute\"\n{{\n    trigger OnRun()\n    begin\n{code}\n    end;\n}}\n"
        )
    };
    std::fs::write(dir.join("Inline.al"), source).map_err(|e| e.to_string())?;
    Ok(dir)
}

/// `execute`: the `OnRun` trigger of one codeunit per app (the lowest object id, one that declares no test first), or of
/// the inline `code`, run in the warm process. One answer line without a `type`, like v2's.
fn execute(base: &TestArgs, fields: &Map<String, Value>, ctx: &mut WarmCtx, flags: &Flags) {
    let mut warnings = Vec::new();
    for key in fields.keys() {
        if DEFERRED.contains(&key.as_str()) {
            return error(format!(
                "`{key}` is deferred in v3 (ADR-016): it only hides run cost and waits until it is measured to matter"
            ));
        }
        if !EXECUTE_FIELDS.contains(&key.as_str()) {
            if RUN_TESTS_FIELDS.contains(&key.as_str()) {
                warnings.push(format!("`{key}` is read by `runTests` only; `execute` ignores it"));
            } else {
                return error(format!("unknown field `{key}` for execute"));
            }
        }
    }
    let code = match fields.get("code") {
        None | Some(Value::Null) => None,
        Some(Value::String(c)) => Some(c.clone()),
        Some(_) => return error("`code` is a string"),
    };
    let sources = match string_list(fields, "sourcePaths") {
        Ok(s) => s,
        Err(e) => return error(e),
    };
    if code.is_some() && !sources.is_empty() {
        return error("`code` and `sourcePaths` are mutually exclusive");
    }
    if code.is_none() && sources.is_empty() {
        return error("`execute` needs `sourcePaths` or `code`");
    }
    let mut a = base.clone();
    match string_list(fields, "packagePaths") {
        Ok(p) => a.package_caches.extend(p.into_iter().map(PathBuf::from)),
        Err(e) => return error(e),
    }
    let inline = code.is_some();
    a.apps = match &code {
        Some(code) => {
            let with_microsoft = a.bc_version.is_some() || !a.deps.is_empty() || !a.package_caches.is_empty();
            match inline_app(code, with_microsoft, a.bc_version.as_deref()) {
                Ok(dir) => vec![dir],
                Err(e) => return error(e),
            }
        }
        None => sources.into_iter().map(PathBuf::from).collect(),
    };
    if let Some(i) = fields.get("testIsolation").and_then(Value::as_str) {
        if !["codeunit", "test", "disabled"].contains(&i.to_ascii_lowercase().as_str()) {
            return error(format!("`testIsolation` is codeunit, test or disabled, not `{i}`"));
        }
        warnings.push("`testIsolation` has no effect on `execute`: one OnRun trigger runs".into());
    }
    let (want_coverage, want_per_test) = match (flag(fields, "coverage"), flag(fields, "perTestCoverage")) {
        (Ok(c), Ok(p)) => (c || p, p),
        (Err(e), _) | (_, Err(e)) => return error(e),
    };
    a.coverage = want_coverage;
    a.capture = al_vm::CaptureConfig {
        values: flag(fields, "captureValues").unwrap_or(false),
        loops: flag(fields, "iterationTracking").unwrap_or(false),
    };
    a.quiet = true;
    a.execute = true;
    // `execute` runs no tests, so `tdd` (of the request, with a warning above, or of the server) does nothing here.
    a.tdd = false;
    flags.cancel.store(false, Ordering::SeqCst);
    flags
        .announce
        .store(flag(fields, "announceStart").unwrap_or(false), Ordering::SeqCst);
    flags.running.store(true, Ordering::SeqCst);
    ctx.report = None;
    ctx.used = None;
    // The answer is one line: no `test` line is streamed.
    let streaming = ctx.on_test.take();
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run::test_with(&a, Some(ctx))));
    ctx.on_test = streaming;
    flags.running.store(false, Ordering::SeqCst);
    let cancelled = flags.cancel.swap(false, Ordering::SeqCst);
    match outcome {
        Err(p) => {
            ctx.runtime.clear();
            let msg = p
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_default();
            error(format!("the run panicked (a runtime bug): {msg}"));
        }
        Ok(Err(e)) => error(e),
        Ok(Ok(code_)) => {
            let Some(report) = ctx.report.take() else {
                return error("the run produced no report");
            };
            let mut tests = Vec::new();
            let mut tagged_messages: Vec<Value> = Vec::new();
            for (i, row) in report.doc["tests"].as_array().into_iter().flatten().enumerate() {
                let mut row = row.as_object().cloned().unwrap_or_default();
                let mut line = test_line(&mut row);
                line.remove("type");
                // An inline object is named by its own name: `X.OnRun`.
                if inline {
                    if let Some(name) = report.object_names.get(i) {
                        line.insert("name".into(), json!(format!("{name}.OnRun")));
                    }
                }
                if let Some(m) = report.messages.get(i).filter(|m| !m.is_empty()) {
                    line.insert("messages".into(), json!(m));
                }
                // What the code did. The tagged messages are the response's, not the test's.
                if let Some(Some(Value::Object(c))) = report.captured.get(i) {
                    for key in ["capturedValues", "loops", "capturedValuesTruncated"] {
                        if let Some(v) = c.get(key) {
                            line.insert(key.into(), v.clone());
                        }
                    }
                    if let Some(Value::Array(m)) = c.get("messages") {
                        tagged_messages.extend(m.iter().cloned());
                    }
                }
                tests.push(Value::Object(line));
            }
            if tests.is_empty() {
                warnings.push("no codeunit with an OnRun trigger in the app".into());
            }
            let mut response = json!({ "exitCode": code_, "tests": tests });
            if cancelled {
                response["cancelled"] = json!(true);
            }
            if a.capture.loops {
                response["messages"] = json!(tagged_messages);
            }
            if !warnings.is_empty() {
                response["warnings"] = json!(warnings);
            }
            if let Some(cov) = &report.coverage {
                if want_coverage {
                    response["coverage"] = cov["coverage"].clone();
                }
                if want_per_test {
                    response["perTestCoverage"] = cov["perTestCoverage"].clone();
                }
            }
            out(&response);
        }
    }
}

/// A `test` line of v2: the row of the JSON document, with `stackFrames` split out of the stack text.
fn test_line(row: &mut Map<String, Value>) -> Map<String, Value> {
    let mut line = Map::new();
    line.insert("type".into(), json!("test"));
    for key in [
        "name",
        "status",
        "durationMs",
        "message",
        "stackTrace",
        "notRun",
        "stubbed",
        "generatedStubs",
    ] {
        if let Some(v) = row.remove(key) {
            line.insert(key.into(), v);
        }
    }
    if let Some(stack) = line.get("stackTrace").and_then(Value::as_str) {
        let frames: Vec<&str> = stack.lines().collect();
        line.insert("stackFrames".into(), json!(frames));
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fields(v: Value) -> Map<String, Value> {
        v.as_object().unwrap().clone()
    }

    fn base() -> TestArgs {
        daemon_args(&["--define".into(), "S".into()]).unwrap()
    }

    #[test]
    fn code_that_starts_with_an_object_keyword_is_an_object_and_anything_else_is_statements() {
        for object in [
            "codeunit 50100 X { }",
            "  \n// a comment\n/* another\n one */\nTable 50101 T { }",
            "\u{feff}pageextension 50102 P extends \"Customer Card\" { }",
            "interface IFoo { }",
        ] {
            assert!(is_object_declaration(object), "{object}");
        }
        for statements in [
            "Message('hi');",
            "codeunitCount := 1;",
            "tableName := 'x';",
            "",
            "// only a comment",
            "var x: Integer; begin end;",
        ] {
            assert!(!is_object_declaration(statements), "{statements}");
        }
    }

    #[test]
    fn a_request_may_set_the_server_address_and_a_bad_one_is_an_error() {
        let (a, _) = request_args(
            &base(),
            &fields(json!({"command": "runTests", "sourcePaths": ["/a"], "serverUrl": "https://bc.example/BC/"})),
        )
        .unwrap();
        assert_eq!(a.server_url.as_deref(), Some("https://bc.example/BC/"));
        let e = request_args(
            &base(),
            &fields(json!({"command": "runTests", "sourcePaths": ["/a"], "serverUrl": "nope"})),
        )
        .unwrap_err();
        assert!(e.contains("serverUrl"), "{e}");
        let (a, _) = request_args(&base(), &fields(json!({"command": "runTests", "sourcePaths": ["/a"]}))).unwrap();
        assert!(a.server_url.is_none());
    }

    #[test]
    fn a_run_request_becomes_the_arguments_of_a_test_run() {
        let f = fields(json!({
            "command": "runTests", "sourcePaths": ["/a", "/b"], "packagePaths": ["/p"], "test": "Codeunit1.X",
            "excludeTests": ["Codeunit1.Y"], "testIsolation": "test", "coverage": true,
        }));
        let (a, w) = request_args(&base(), &f).unwrap();
        assert_eq!(a.apps, [PathBuf::from("/a"), PathBuf::from("/b")]);
        assert_eq!(a.package_caches, [PathBuf::from("/p")]);
        assert_eq!(a.defines, ["S"]);
        assert_eq!(
            (a.filters.as_slice(), a.excludes.as_slice()),
            (&["Codeunit1.X".to_string()][..], &["Codeunit1.Y".to_string()][..])
        );
        assert_eq!(a.isolation, Some(al_testrun::isolation::TestIsolation::Function));
        assert!(a.coverage && a.quiet);
        assert!(w.is_empty());
    }

    #[test]
    fn fields_are_checked_deferred_ones_refused_and_the_other_commands_warned_about() {
        let run = |v: Value| request_args(&base(), &fields(v));
        assert!(run(json!({"command": "runTests"})).unwrap_err().contains("sourcePaths"));
        assert!(
            run(json!({"sourcePaths": ["/a"], "includeFailing": true}))
                .unwrap_err()
                .contains("deferred")
        );
        assert!(run(json!({"sourcePaths": ["/a"], "tdd": true})).unwrap().0.tdd);
        assert!(run(json!({"sourcePaths": ["/a"], "tdd": 1})).is_err());
        assert!(
            run(json!({"sourcePaths": ["/a"], "frobnicate": 1}))
                .unwrap_err()
                .contains("unknown field")
        );
        assert!(run(json!({"sourcePaths": ["/a"], "testIsolation": "x"})).is_err());
        assert!(run(json!({"sourcePaths": "/a"})).is_err());
        let (_, w) = run(json!({"sourcePaths": ["/a"], "code": "x"})).unwrap();
        assert_eq!(w.len(), 1);
        let (a, w) = run(json!({"sourcePaths": ["/a"], "captureValues": true, "iterationTracking": true})).unwrap();
        assert!(w.is_empty() && a.capture.values && a.capture.loops);
    }

    #[test]
    fn a_test_line_carries_the_stack_as_frames() {
        let mut row = crate::output::json_row(
            1,
            "T",
            "X",
            &al_testrun::Outcome::Fail("m".into()),
            "a\nb",
            std::time::Duration::from_millis(3),
        );
        let l = test_line(&mut row);
        assert_eq!(l["type"], "test");
        assert_eq!(l["name"], "Codeunit1.X");
        assert_eq!(l["stackFrames"], json!(["a", "b"]));
        assert_eq!(l["durationMs"], 3);
    }
}
