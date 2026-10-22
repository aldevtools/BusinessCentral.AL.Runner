//! ISV test apps as test sources (ADR-052 section 1, build step 3): `al-runner test APP --affected-from PATH`, own
//! tests next to sources, the grouped report, `--output-json` with `sources`, `--impact-json` format 2 and
//! `--impact-accept` per source. The fixture apps of `tests/fixtures/impact` stand in for an ISV: `lib` is the ISV's
//! app (a publisher), `tests` its test app, the extension (`ext`) subscribes to the publisher. Nothing is downloaded.

mod support;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use support::{ext_only, impact_fixture, source_row, text, zip};

fn alrunner(root: &Path, args: &[&str]) -> Output {
    let cache = root.join("cache");
    Command::new(env!("CARGO_BIN_EXE_al-runner"))
        .env("AL_CHECK_THREADS", "2")
        .env("AL_LOWER_THREADS", "2")
        .env("AL_TEST_THREADS", "2")
        .env("ALRUNNER_NO_PATH_NOTE", "1")
        .args(args)
        .args(["--no-download", "--cache-dir", cache.to_str().unwrap()])
        .output()
        .expect("run al-runner")
}

fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

/// A copy of the fixture's test app under `dir` with another id and name, holding only the codeunit that reaches
/// nothing of the extension.
fn second_test_app(dir: &Path) -> PathBuf {
    let tests = impact_fixture("tests");
    std::fs::create_dir_all(dir).unwrap();
    let json = std::fs::read_to_string(tests.join("app.json"))
        .unwrap()
        .replace(
            "43000000-0000-0000-0000-000000000002",
            "43000000-0000-0000-0000-000000000022",
        )
        .replace("Impact Tests", "Impact Tests B");
    std::fs::write(dir.join("app.json"), json).unwrap();
    let al = std::fs::read_to_string(tests.join("Tests.Codeunit.al")).unwrap();
    let at = al.find("codeunit 50511").unwrap();
    std::fs::write(dir.join("Other.Codeunit.al"), &al[at..]).unwrap();
    dir.to_path_buf()
}

#[test]
fn an_isv_test_app_runs_the_tests_that_reach_the_extension_with_and_without_it() {
    let dir = tempfile::tempdir().unwrap();
    let ext = ext_only(dir.path());
    let (tests, lib) = (impact_fixture("tests"), impact_fixture("lib"));
    let junit = dir.path().join("r.xml");
    let o = alrunner(
        dir.path(),
        &[
            "test",
            &s(&ext),
            "--affected-from",
            &s(&tests),
            "--dep",
            &s(&lib),
            "--junit",
            &s(&junit),
        ],
    );
    let out = text(&o);
    // A source is named by its app.json, never by its path.
    assert!(out.contains("== Impact Tests 1.0.0.0 ("), "{out}");
    // No index: static selection, with the command that builds the index of this source.
    assert!(
        out.contains(&format!("al-runner impact-index build {}", s(&tests))),
        "{out}"
    );
    // The "without" side holds the ISV app (the test passes there), the "with" side adds the extension.
    assert!(
        out.contains("50510 Imp Tests.PostKeepsALargeAmount: pass -> fail: amount 20"),
        "{out}"
    );
    // 50511 reaches nothing of the extension: not selected, counted, never a pass.
    assert!(
        out.contains("Impact Tests: static selection; not selected: 1 test(s) do not reach your extension"),
        "{out}"
    );
    assert!(!out.contains("AlsoPostsNothing"), "{out}");
    assert_eq!(source_row(&out, "Impact Tests"), "4 2 1 0 0 0 1", "{out}");
    assert!(
        out.contains("Result: FAIL (Impact Tests: 1 newly failing) (exit 1)"),
        "{out}"
    );
    assert_eq!(o.status.code(), Some(1), "{out}");
    let x = std::fs::read_to_string(&junit).unwrap();
    assert!(
        x.contains("<property name=\"test_source\" value=\"Impact Tests\" />"),
        "{x}"
    );
    assert!(x.contains("<property name=\"not_selected\" value=\"1\" />"), "{x}");
    assert!(!x.contains("AlsoPostsNothing"), "{x}");
}

#[test]
fn own_tests_and_a_folder_of_two_test_apps_make_three_parts_and_the_worst_part_sets_the_exit_code() {
    let dir = tempfile::tempdir().unwrap();
    let vendor = dir.path().join("vendor");
    // A folder of several test apps is one source per app inside it.
    let copy = vendor.join("a");
    std::fs::create_dir_all(&copy).unwrap();
    for f in ["app.json", "Tests.Codeunit.al"] {
        std::fs::copy(impact_fixture("tests").join(f), copy.join(f)).unwrap();
    }
    second_test_app(&vendor.join("b"));
    let (ext, lib) = (impact_fixture("ext"), impact_fixture("lib"));
    let o = alrunner(
        dir.path(),
        &["test", &s(&ext), "--affected-from", &s(&vendor), "--dep", &s(&lib)],
    );
    let out = text(&o);
    let own = out.find("== own tests: Impact Ext 1.0.0.0 ==").expect(&out);
    let a = out.find("== Impact Tests 1.0.0.0 (").expect(&out);
    let b = out.find("== Impact Tests B 1.0.0.0 (").expect(&out);
    assert!(own < a && a < b, "{out}");
    assert_eq!(source_row(&out, "own tests"), "1 - - - - 0 not run 1", "{out}");
    assert_eq!(source_row(&out, "Impact Tests"), "4 2 1 0 0 0 1", "{out}");
    // The second app selects nothing: 0, not 6, and nothing is counted as run.
    assert_eq!(source_row(&out, "Impact Tests B"), "0 0 0 0 0 0 0", "{out}");
    assert!(
        out.contains("Result: FAIL (own tests: 1 failed; Impact Tests: 1 newly failing) (exit 1)"),
        "{out}"
    );
    assert_eq!(o.status.code(), Some(1), "{out}");
}

/// A `.app` with a manifest and no AL source, as a vendor ships it compiled.
fn compiled_only(path: &Path, id: &str, name: &str) {
    let manifest = format!(
        r#"<Package><App Id="{id}" Name="{name}" Publisher="Test" Version="1.0.0.0" /><Dependencies/></Package>"#
    );
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, zip(&[("NavxManifest.xml", manifest)])).unwrap();
}

#[test]
fn a_compiled_only_source_or_dependency_of_a_source_exits_2_before_the_run() {
    let dir = tempfile::tempdir().unwrap();
    let ext = ext_only(dir.path());
    let lib = impact_fixture("lib");
    let app = dir.path().join("vendor/Vendor Tests.app");
    compiled_only(&app, "43000000-0000-0000-0000-000000000031", "Vendor Tests");
    let o = alrunner(
        dir.path(),
        &["test", &s(&ext), "--affected-from", &s(&app), "--dep", &s(&lib)],
    );
    let out = text(&o);
    assert_eq!(o.status.code(), Some(2), "{out}");
    assert!(
        out.contains("is a compiled-only app; AL Runner compiles test apps from their AL source (ADR-013). Ask the vendor for a package with source, or a `*.Source.zip`."),
        "{out}"
    );
    assert!(!out.contains("== own tests"), "nothing ran: {out}");

    // The source's dependency (the ISV app) only as a compiled package.
    let packages = dir.path().join("packages");
    compiled_only(
        &packages.join("Impact Lib.app"),
        "43000000-0000-0000-0000-000000000001",
        "Impact Lib",
    );
    let tests = impact_fixture("tests");
    let o = alrunner(
        dir.path(),
        &[
            "test",
            &s(&ext),
            "--affected-from",
            &s(&tests),
            "--package-cache",
            &s(&packages),
        ],
    );
    let out = text(&o);
    assert_eq!(o.status.code(), Some(2), "{out}");
    assert!(
        out.contains("Impact Lib 1.0.0.0 (a dependency of Impact Tests, ")
            && out.contains(
                "Impact Lib.app) is a compiled-only app; AL Runner compiles test apps from their AL source (ADR-013)"
            ),
        "{out}"
    );
}

#[test]
fn a_source_that_depends_on_the_extension_or_is_given_twice_exits_2() {
    let dir = tempfile::tempdir().unwrap();
    let ext = ext_only(dir.path());
    let lib = impact_fixture("lib");
    // A test app of the extension itself, given as a source.
    let own = dir.path().join("own-tests");
    std::fs::create_dir_all(&own).unwrap();
    std::fs::write(
        own.join("app.json"),
        r#"{"id":"43000000-0000-0000-0000-000000000041","name":"Ext Tests","publisher":"Test","version":"1.0.0.0","dependencies":[{"id":"43000000-0000-0000-0000-000000000003","name":"Impact Ext","publisher":"Test","version":"1.0.0.0"}]}"#,
    )
    .unwrap();
    std::fs::write(
        own.join("T.Codeunit.al"),
        "codeunit 50530 \"Ext T\"\n{\n    Subtype = Test;\n\n    [Test]\n    procedure T()\n    begin\n    end;\n}\n",
    )
    .unwrap();
    let o = alrunner(
        dir.path(),
        &["test", &s(&ext), "--affected-from", &s(&own), "--dep", &s(&lib)],
    );
    let out = text(&o);
    assert_eq!(o.status.code(), Some(2), "{out}");
    assert!(
        out.contains("Ext Tests 1.0.0.0 depends on Impact Ext: it is your own test app; pass it as an APP"),
        "{out}"
    );

    let tests = impact_fixture("tests");
    for extra in [vec![s(&tests), s(&tests)], vec![s(&ext)]] {
        let mut args = vec!["test".to_string(), s(&ext)];
        for e in &extra {
            args.extend(["--affected-from".to_string(), e.clone()]);
        }
        args.extend(["--dep".to_string(), s(&lib)]);
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let o = alrunner(dir.path(), &args);
        let out = text(&o);
        assert_eq!(o.status.code(), Some(2), "{out}");
        assert!(
            out.contains("is given twice") || out.contains("is also an APP"),
            "{out}"
        );
    }
}

#[test]
fn impact_json_format_2_names_the_source_and_the_accept_file_matches_per_source() {
    let dir = tempfile::tempdir().unwrap();
    let ext = ext_only(dir.path());
    let (tests, lib) = (impact_fixture("tests"), impact_fixture("lib"));
    let diff = dir.path().join("diff.json");
    let run = |extra: &[&str]| {
        let mut args = vec![
            "test",
            ext.to_str().unwrap(),
            "--affected-from",
            tests.to_str().unwrap(),
        ];
        args.extend(["--dep", lib.to_str().unwrap()]);
        args.extend_from_slice(extra);
        alrunner(dir.path(), &args)
    };
    let o = run(&["--impact-json", diff.to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(1), "{}", text(&o));
    let doc: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&diff).unwrap()).unwrap();
    assert_eq!(doc["format"], 2, "{doc}");
    let source = &doc["sources"][0];
    assert_eq!(source["source"], "Impact Tests", "{doc}");
    assert_eq!(source["format"], 1, "{doc}");
    assert_eq!(source["counts"]["newlyFailing"], 1, "{doc}");
    assert_eq!(source["notSelected"]["tests"], 1, "{doc}");

    // The diff given back as the accept file: the change is accepted, exit 0.
    let o = run(&["--impact-accept", diff.to_str().unwrap()]);
    let out = text(&o);
    assert_eq!(o.status.code(), Some(0), "{out}");
    assert!(out.contains("1 accepted change"), "{out}");

    // An entry of another source does not match here: exit 1, and the entry is named as stale.
    let mut other = doc.clone();
    other["sources"][0]["source"] = "Other Tests".into();
    let other_file = dir.path().join("other.json");
    std::fs::write(&other_file, other.to_string()).unwrap();
    let o = run(&["--impact-accept", other_file.to_str().unwrap()]);
    let out = text(&o);
    assert_eq!(o.status.code(), Some(1), "{out}");
    assert!(out.contains("Other Tests"), "{out}");

    // A format-1 file (no source) matches in every source.
    let one = dir.path().join("one.json");
    std::fs::write(&one, doc["sources"][0].to_string()).unwrap();
    let o = run(&["--impact-accept", one.to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(0), "{}", text(&o));
}

#[test]
fn output_json_keeps_the_own_tests_in_the_v2_shape_and_adds_the_sources() {
    let dir = tempfile::tempdir().unwrap();
    let (ext, tests, lib) = (impact_fixture("ext"), impact_fixture("tests"), impact_fixture("lib"));
    let file = dir.path().join("out.json");
    let o = alrunner(
        dir.path(),
        &[
            "test",
            &s(&ext),
            "--affected-from",
            &s(&tests),
            "--dep",
            &s(&lib),
            "--output-json",
            &s(&file),
        ],
    );
    assert_eq!(o.status.code(), Some(1), "{}", text(&o));
    let doc: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    // The own tests, as a plain run writes them.
    assert_eq!(doc["failed"], 1, "{doc}");
    assert_eq!(doc["total"], 1, "{doc}");
    assert_eq!(doc["tests"][0]["method"], "OwnTest", "{doc}");
    assert_eq!(doc["exitCode"], 1, "{doc}");
    let source = &doc["sources"][0];
    assert_eq!(source["source"], "Impact Tests", "{doc}");
    assert_eq!(source["kind"], "path", "{doc}");
    assert_eq!(source["testApps"][0]["name"], "Impact Tests", "{doc}");
    assert_eq!(source["testApps"][0]["version"], "1.0.0.0", "{doc}");
    assert_eq!(source["mode"], "static", "{doc}");
    assert_eq!(source["counts"]["newlyFailing"], 1, "{doc}");
    assert_eq!(source["notSelected"], 1, "{doc}");
    assert_eq!(source["exitCode"], 1, "{doc}");
    let failing: Vec<&serde_json::Value> = source["tests"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|t| t["class"] == "newly failing")
        .collect();
    assert_eq!(failing.len(), 1, "{doc}");
    assert_eq!(failing[0]["test"], "PostKeepsALargeAmount", "{doc}");
}
