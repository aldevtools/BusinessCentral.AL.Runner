//! `al-runner test --profile`: `.alcpuprofile` files the AL extension opens (one per test codeunit, or one merged
//! file), in instrumentation and sampling mode, plus the deprecated `AL_PROFILE` variable.

use super::{app_json, code, text, write};
use serde_json::Value;
use std::path::Path;
use std::process::Command;

const SRC: &str = "table 50690 \"Prof Row\"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Amount; Decimal) { }
    }
    keys { key(PK; Id) { } }
}

codeunit 50691 \"Prof Tests\"
{
    Subtype = Test;

    [Test]
    procedure Work()
    var
        r: Record \"Prof Row\";
        i: Integer;
    begin
        for i := 1 to 200 do begin
            r.Id := i;
            r.Amount := i;
            r.Insert();
        end;
        Spin();
    end;

    local procedure Spin()
    var
        i: Integer;
        d: Decimal;
    begin
        for i := 1 to 20000 do
            d += i / 7;
    end;
}
";

fn app(d: &Path) -> std::path::PathBuf {
    let dir = d.join("prof");
    write(
        &dir,
        "app.json",
        &app_json("11111111-0000-0000-0000-000000006925", "Prof", &[]),
    );
    write(&dir, "src/Tests.al", SRC);
    dir
}

fn run(dir: &Path, extra: &[&str], env: &[(&str, &Path)]) -> std::process::Output {
    let mut c = Command::new(env!("CARGO_BIN_EXE_al-runner"));
    c.args(["test", dir.to_str().unwrap(), "--no-download"])
        .args(extra)
        .env("AL_CHECK_THREADS", "2")
        .env("AL_LOWER_THREADS", "2")
        .env("AL_TEST_THREADS", "2")
        .env("ALRUNNER_NO_PATH_NOTE", "1")
        .env_remove("AL_PROFILE");
    for (k, v) in env {
        c.env(k, v);
    }
    c.output().unwrap()
}

fn read(p: &Path) -> Value {
    let t = std::fs::read_to_string(p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
    serde_json::from_str(&t).unwrap()
}

fn node<'a>(v: &'a Value, name: &str) -> &'a Value {
    v["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["callFrame"]["functionName"] == name)
        .unwrap_or_else(|| panic!("no node {name} in {v}"))
}

/// The fields VS Code reads (phase 1 report of the profiler, sections 1.2 and 1.3).
fn assert_loadable(v: &Value) {
    assert!(v["startTime"].is_u64() && v["endTime"].is_u64());
    let nodes = v["nodes"].as_array().unwrap();
    let ids: Vec<u64> = nodes.iter().map(|n| n["id"].as_u64().unwrap()).collect();
    for n in nodes {
        assert!(n["callFrame"]["functionName"].is_string());
        assert!(n["callFrame"]["url"].is_string());
        assert!(n["callFrame"]["lineNumber"].is_u64() && n["callFrame"]["columnNumber"].is_u64());
        assert!(n["hitCount"].is_u64());
        assert!(n["declaringApplication"]["appName"].is_string());
        assert!(n["applicationDefinition"]["objectType"].is_string());
        for c in n["children"].as_array().unwrap() {
            assert!(ids.contains(&c.as_u64().unwrap()));
        }
    }
    let samples = v["samples"].as_array().unwrap().len();
    assert_eq!(samples, v["timeDeltas"].as_array().unwrap().len());
    if v.get("kind").is_none() {
        assert_eq!(samples, v["sampleExecutionTimes"].as_array().unwrap().len());
    }
    assert_eq!(v["alRunner"]["unit"], "microseconds");
}

#[test]
fn profile_writes_one_instrumentation_file_per_test_codeunit() {
    let d = tempfile::tempdir().unwrap();
    let dir = app(d.path());
    let out = d.path().join("out");
    let o = run(&dir, &["--profile", out.to_str().unwrap()], &[]);
    assert_eq!(code(&o), 0, "{}", text(&o));
    let file = out.join("50691-Prof Tests.alcpuprofile");
    let v = read(&file);
    assert_loadable(&v);
    assert!(
        v.get("kind").is_none(),
        "instrumentation is written as the AL extension writes it"
    );
    let work = node(&v, "Work");
    assert_eq!(work["hitCount"], 1);
    assert_eq!(work["applicationDefinition"]["objectType"], "CodeUnit");
    assert_eq!(work["applicationDefinition"]["objectId"], 50691);
    assert_eq!(work["declaringApplication"]["appName"], "Prof");
    // The first statement of Work (`for i := 1 to 200 do begin`) is line 21, column 9 in the editor: 0-based 20, 8.
    assert_eq!(work["callFrame"]["lineNumber"], 20);
    assert_eq!(work["callFrame"]["columnNumber"], 8);
    let url = work["callFrame"]["url"].as_str().unwrap();
    assert!(url.starts_with("file://") && url.ends_with("src/Tests.al"), "{url}");
    assert!(work["positionTicks"].as_array().is_some_and(|p| !p.is_empty()));
    let spin = node(&v, "Spin");
    assert!(work["children"].as_array().unwrap().contains(&spin["id"]));
    let insert = node(&v, "Insert \"Prof Row\"");
    assert_eq!(insert["hitCount"], 200);
    let db = node(&v, "Database (in memory)");
    assert!(work["children"].as_array().unwrap().contains(&db["id"]));
    assert_eq!(v["alRunner"]["codeunits"][0]["tests"][0]["name"], "Work");
    assert!(
        v["nodes"][0]["callFrame"]["functionName"]
            .as_str()
            .unwrap()
            .starts_with("Thread ")
    );
    let summary = std::fs::read_to_string(out.join("summary.txt")).unwrap();
    assert!(summary.starts_with("instrumentation on 1 threads"), "{summary}");
    assert!(summary.contains("Insert \"Prof Row\""), "{summary}");
}

#[test]
fn profile_mode_sampling_into_one_merged_file() {
    let d = tempfile::tempdir().unwrap();
    let dir = app(d.path());
    let file = d.path().join("all.alcpuprofile");
    let o = run(
        &dir,
        &[
            "--profile",
            file.to_str().unwrap(),
            "--profile-mode",
            "sampling",
            "--profile-interval",
            "100",
        ],
        &[],
    );
    assert_eq!(code(&o), 0, "{}", text(&o));
    let v = read(&file);
    assert_loadable(&v);
    assert_eq!(v["kind"], 1);
    assert_eq!(v["nodes"][0]["callFrame"]["functionName"], "al-runner run");
    node(&v, "Codeunit 50691 \"Prof Tests\"");
    assert_eq!(v["alRunner"]["intervalUs"], 100);
    assert!(v["alRunner"]["samples"].as_u64().unwrap() > 0, "{}", v["alRunner"]);
}

#[test]
fn profile_mode_without_profile_is_an_error() {
    let d = tempfile::tempdir().unwrap();
    let dir = app(d.path());
    let o = run(&dir, &["--profile-mode", "sampling"], &[]);
    assert_ne!(code(&o), 0);
    assert!(text(&o).contains("--profile-mode needs --profile"), "{}", text(&o));
    let o = run(&dir, &["--profile", "x", "--profile-mode", "trace"], &[]);
    assert!(text(&o).contains("instrumentation or sampling"), "{}", text(&o));
}

#[test]
fn the_old_al_profile_variable_still_writes_its_text_and_a_profile() {
    let d = tempfile::tempdir().unwrap();
    let dir = app(d.path());
    let file = d.path().join("profile.txt");
    let o = run(&dir, &[], &[("AL_PROFILE", &file)]);
    assert_eq!(code(&o), 0, "{}", text(&o));
    assert!(text(&o).contains("AL_PROFILE is deprecated"), "{}", text(&o));
    let t = std::fs::read_to_string(&file).unwrap();
    assert!(t.starts_with("samples "), "{t}");
    assert_loadable(&read(&d.path().join("profile.txt.alcpuprofile")));
}
