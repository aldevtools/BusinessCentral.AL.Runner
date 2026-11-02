//! `al-runner test` end to end on small fixture apps. Nothing here is Microsoft's code.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

mod compile_and_select;
mod coverage;
mod dependencies;
mod diagnostics;
mod isolation;
mod json_interface;
mod onrun_stability;
mod package_target_resources;
mod path_note;
mod run;
mod seed;

fn alrunner(args: &[&str]) -> Output {
    alrunner_with(args, &[])
}

fn alrunner_with(args: &[&str], env: &[(&str, &str)]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_al-runner"))
        // The run seed (ADR-061) comes from the test, not from the machine's environment.
        .env_remove("ALRUNNER_SEED")
        .env_remove("AL_RUNNER_SEED")
        .envs(env.iter().copied())
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

/// An app folder `root/dir` with a manifest (no dependencies) and one source file.
fn one_file_app(root: &Path, dir: &str, id: &str, name: &str, file: &str, source: &str) -> PathBuf {
    let dir = root.join(dir);
    write(&dir, "app.json", &app_json(id, name, &[]));
    write(&dir, file, source);
    dir
}

fn mixed_app(root: &Path) -> PathBuf {
    one_file_app(
        root,
        "mixed",
        "11111111-0000-0000-0000-000000000001",
        "Mixed",
        "src/Tests.al",
        MIXED,
    )
}

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
