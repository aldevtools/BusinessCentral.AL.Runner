//! Matrix runs (ADR-052 section 3, build step 4) on two fake builds of the extracted artifact (`support::fake_artifact`:
//! fixture test apps under the paths the real artifact uses, a fake BCApps recipe, no network). Nothing here is
//! Microsoft's code.

mod support;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use support::{ext_only, fake_artifact, impact_fixture, text};

const NEW: &str = "29.0.1.2";
const OLD: &str = "28.5.1.1";

/// Both builds; in the old one `Tests-Fake-B` fails, and only the new one has `Tests-Fake-C`.
fn two_builds(root: &Path) {
    fake_artifact(root, NEW, false);
    fake_artifact(root, OLD, true);
    support::add_test_app(root, NEW, "Tests-Fake-C", 50600, "CRuns");
}

fn alrunner(root: &Path, args: &[&str], env: &[(&str, &str)]) -> Output {
    let cache = root.join("cache");
    let bcapps = root.join("bcapps");
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_al-runner"));
    cmd.env("HOME", root.join("home"))
        .env("AL_CHECK_THREADS", "2")
        .env("AL_LOWER_THREADS", "2")
        .env("AL_TEST_THREADS", "2")
        .env("ALRUNNER_NO_PATH_NOTE", "1")
        .args(args)
        .args(["--no-download", "--cache-dir", cache.to_str().unwrap()]);
    if args.first() == Some(&"test-microsoft") {
        cmd.args(["--bcapps", bcapps.to_str().unwrap()]);
    }
    for (k, v) in env {
        cmd.env(k, v);
    }
    cmd.output().expect("run al-runner")
}

/// A small app with one test that fails if a symbol the app does not define is set.
fn symbol_app(root: &Path) -> PathBuf {
    let dir = root.join("symbol-app");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("app.json"),
        r#"{"id":"43000000-0000-0000-0000-000000000051","name":"Symbol App","publisher":"Test","version":"1.0.0.0","idRanges":[{"from":50700,"to":50709}],"dependencies":[]}"#,
    )
    .unwrap();
    std::fs::write(
        dir.join("T.Codeunit.al"),
        "codeunit 50700 \"Symbol Tests\"\n{\n    Subtype = Test;\n\n    [Test]\n    procedure NoSymbolOfTheRunner()\n    begin\n#if BC29PLUS\n        Error('the runner defined BC29PLUS');\n#endif\n#if BC28PLUS\n        Error('the runner defined BC28PLUS');\n#endif\n    end;\n}\n",
    )
    .unwrap();
    dir
}

fn json(path: &Path) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))).unwrap()
}

#[test]
fn a_cell_below_the_app_floor_stops_the_whole_run_before_any_cell_starts() {
    let dir = tempfile::tempdir().unwrap();
    two_builds(dir.path());
    let app = symbol_app(dir.path());
    let json_path = app.join("app.json");
    let floor = std::fs::read_to_string(&json_path).unwrap().replace(
        r#""version":"1.0.0.0","#,
        r#""version":"1.0.0.0","application":"29.0.0.0","#,
    );
    std::fs::write(&json_path, floor).unwrap();
    let o = alrunner(
        dir.path(),
        &["test", app.to_str().unwrap(), "--bc-version", "28.5,29.0"],
        &[],
    );
    let out = text(&o);
    assert_eq!(o.status.code(), Some(2), "{out}");
    assert!(
        out.contains(&format!("cell 28.5 w1: build {OLD} is below the app's floor 29.0")),
        "{out}"
    );
    assert!(out.contains("no cell was run"), "{out}");
    assert!(!out.contains("NoSymbolOfTheRunner"), "{out}");
}

#[test]
fn test_microsoft_over_two_builds_lists_the_tests_that_differ_and_exits_with_the_worst_cell() {
    let dir = tempfile::tempdir().unwrap();
    two_builds(dir.path());
    let out_dir = dir.path().join("out");
    let junit = out_dir.join("r.xml");
    let doc = out_dir.join("m.json");
    let o = alrunner(
        dir.path(),
        &[
            "test-microsoft",
            "--bc-version",
            &format!("28.5,{NEW}"),
            "--junit",
            junit.to_str().unwrap(),
            "--output-json",
            doc.to_str().unwrap(),
        ],
        &[],
    );
    let out = text(&o);
    assert!(out.contains("\nMatrix\n"), "{out}");
    let row = |label: &str| {
        out.lines()
            .find(|l| l.trim_start().starts_with(label) && l.contains(" w1 "))
            .unwrap_or_else(|| panic!("no row {label}:\n{out}"))
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    };
    // cell, build, pass, flagged, fail, not run, exit
    assert_eq!(row("28.5 w1"), format!("28.5 w1 {OLD} 1 0 1 0 1"), "{out}");
    assert_eq!(row(&format!("{NEW} w1")), format!("{NEW} w1 {NEW} 3 0 0 0 0"), "{out}");
    assert!(out.contains("Differs between cells (2 tests)"), "{out}");
    let differs = &out[out.find("Differs between cells").expect(&out)..];
    let line = |t: &str| {
        differs
            .lines()
            .find(|l| l.split_whitespace().nth(2) == Some(t))
            .unwrap_or_else(|| panic!("no line {t}:\n{out}"))
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    };
    assert_eq!(line("BRuns"), "- 50200 BRuns FAIL PASS", "{out}");
    assert_eq!(line("CRuns"), "- 50600 CRuns absent PASS", "{out}");
    assert!(out.contains("Result: FAIL (worst cell 28.5 w1: exit 1)"), "{out}");
    assert_eq!(o.status.code(), Some(1), "{out}");
    // One JUnit file per cell (the recipe then adds the test app and lane); one document with every cell.
    let names: Vec<String> = std::fs::read_dir(&out_dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    for tag in ["28.5-w1", &format!("{NEW}-w1")] {
        assert!(
            names
                .iter()
                .any(|n| n.starts_with(&format!("r.{tag}.")) && n.ends_with(".xml")),
            "{tag}: {names:?}"
        );
    }
    let m = json(&doc);
    assert_eq!(m["format"], 1, "{m}");
    assert_eq!(m["cells"].as_array().unwrap().len(), 2, "{m}");
    assert_eq!(m["cells"][0]["build"], OLD, "{m}");
    assert_eq!(m["cells"][0]["exitCode"], 1, "{m}");
    assert!(
        m["cells"][0]["commandLine"]
            .as_str()
            .unwrap()
            .contains(&format!("--bc-version {OLD} --country w1")),
        "{m}"
    );
    let b = m["differs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["test"] == "BRuns")
        .expect("BRuns differs");
    assert_eq!(b["outcomes"]["28.5 w1"], "fail", "{m}");
    assert_eq!(b["outcomes"][format!("{NEW} w1")], "pass", "{m}");
    assert_eq!(m["exitCode"], 1, "{m}");
}

#[test]
fn a_killed_cell_is_exit_2_and_the_other_cells_finish() {
    let dir = tempfile::tempdir().unwrap();
    two_builds(dir.path());
    let app = symbol_app(dir.path());
    let o = alrunner(
        dir.path(),
        &["test", app.to_str().unwrap(), "--bc-version", &format!("{OLD},{NEW}")],
        &[("ALRUNNER_MATRIX_TEST_ABORT", OLD)],
    );
    let out = text(&o);
    assert_eq!(o.status.code(), Some(2), "{out}");
    assert!(
        out.contains(&format!("== cell {OLD} w1 ({OLD}): exit 2, killed by signal")),
        "{out}"
    );
    assert!(out.contains(&format!("== cell {NEW} w1 ({NEW}): exit 0 ==")), "{out}");
    assert!(
        out.contains(&format!("Result: ERROR (worst cell {OLD} w1: exit 2)")),
        "{out}"
    );
}

#[test]
fn cells_run_side_by_side_within_the_memory_rule_and_one_by_one_when_memory_is_short() {
    let dir = tempfile::tempdir().unwrap();
    two_builds(dir.path());
    let app = symbol_app(dir.path());
    let doc = dir.path().join("m.json");
    let overlap = |m: &serde_json::Value| {
        let c = m["cells"].as_array().unwrap();
        let (s0, e0) = (c[0]["startedMs"].as_u64().unwrap(), c[0]["endedMs"].as_u64().unwrap());
        let (s1, e1) = (c[1]["startedMs"].as_u64().unwrap(), c[1]["endedMs"].as_u64().unwrap());
        s0 < e1 && s1 < e0
    };
    let versions = format!("{OLD},{NEW}");
    let run = |extra: &[&str], free: &str| {
        let mut args = vec!["test", app.to_str().unwrap(), "--bc-version", &versions];
        args.extend(["--output-json", doc.to_str().unwrap(), "--matrix-parallel", "2"]);
        args.extend_from_slice(extra);
        alrunner(dir.path(), &args, &[("ALRUNNER_MATRIX_FREE_MB", free)])
    };
    let o = run(&[], "100000");
    let out = text(&o);
    // No symbol of the runner's own: the test passes in every cell, as in a single run.
    assert_eq!(o.status.code(), Some(0), "{out}");
    assert!(!out.contains("Differs between cells"), "{out}");
    let m = json(&doc);
    assert!(overlap(&m), "{m}");
    assert_eq!(m["cells"][0]["symbols"], serde_json::json!([]), "{m}");
    // Less free memory than a cell's expected peak plus the reserve: one cell at a time (the cap of --matrix-memory
    // is the unit test of `may_start`).
    let o = run(&["--matrix-memory", "64"], "100");
    assert_eq!(o.status.code(), Some(0), "{}", text(&o));
    assert!(!overlap(&json(&doc)), "{}", json(&doc));
}

#[test]
fn a_test_source_runs_in_every_cell_with_a_row_per_cell_and_part() {
    let dir = tempfile::tempdir().unwrap();
    two_builds(dir.path());
    let ext = ext_only(dir.path());
    let (tests, lib) = (impact_fixture("tests"), impact_fixture("lib"));
    let o = alrunner(
        dir.path(),
        &[
            "test",
            ext.to_str().unwrap(),
            "--affected-from",
            tests.to_str().unwrap(),
            "--dep",
            lib.to_str().unwrap(),
            "--bc-version",
            &format!("{OLD},{NEW}"),
        ],
        &[],
    );
    let out = text(&o);
    assert!(out.contains("\nSources per cell\n"), "{out}");
    for cell in [OLD, NEW] {
        let row = out
            .lines()
            .find(|l| l.trim_start().starts_with(&format!("{cell} w1  Impact Tests")))
            .unwrap_or_else(|| panic!("no source row for {cell}:\n{out}"));
        assert!(
            row.split_whitespace()
                .collect::<Vec<_>>()
                .ends_with(&["4", "1", "0", "0", "0", "1"]),
            "{row}"
        );
    }
    assert!(
        !out.contains("Differs between cells"),
        "the same change in both cells: {out}"
    );
    assert_eq!(o.status.code(), Some(1), "{out}");
}

#[test]
fn affected_in_a_matrix_keeps_a_stored_result_per_cell() {
    let dir = tempfile::tempdir().unwrap();
    two_builds(dir.path());
    let app = symbol_app(dir.path());
    let store = dir.path().join("store");
    let versions = format!("{OLD},{NEW}");
    let run = || {
        alrunner(
            dir.path(),
            &[
                "test",
                app.to_str().unwrap(),
                "--bc-version",
                &versions,
                "--affected",
                "--affected-store",
                store.to_str().unwrap(),
            ],
            &[],
        )
    };
    let o = run();
    assert_eq!(o.status.code(), Some(0), "{}", text(&o));
    for b in [OLD, NEW] {
        assert!(dir.path().join(format!("store.{b}-w1")).is_file(), "{b}: {}", text(&o));
    }
    // Without --affected-store, each cell's key holds its build and country: the first run stores both, the second
    // finds both (one shared key would leave one cell without its result).
    let default = || {
        alrunner(
            dir.path(),
            &[
                "test",
                app.to_str().unwrap(),
                "--bc-version",
                &versions,
                "--affected",
                "--verbose",
            ],
            &[],
        )
    };
    let first = text(&default());
    assert_eq!(first.matches("full run: no stored result").count(), 2, "{first}");
    let o = default();
    let out = text(&o);
    assert_eq!(o.status.code(), Some(0), "{out}");
    assert!(!out.contains("no stored result"), "{out}");
    assert_eq!(out.matches("affected: ran ").count(), 2, "{out}");
}
