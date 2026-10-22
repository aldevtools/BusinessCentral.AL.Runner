//! `al-runner test-microsoft` (ADR-052 section 2) on a fake extracted artifact: small fixture test apps under the
//! paths the real artifact uses, a fake BCApps recipe, no network. Nothing here is Microsoft's code.
//!
//! Build step 2 (`--affected-from microsoft`, `test-microsoft APP`) adds the impact fixture's test app
//! (`tests/fixtures/impact/tests`, a stand-in for a Microsoft test app) to the artifact as `Tests-Imp`, with its
//! publisher app `lib` given by `--dep` and the subscriber `ext` as the extension.

mod support;

use std::path::Path;
use std::process::{Command, Output};
use support::{impact_fixture, source_row, text, zip};

const BUILD: &str = "29.0.1.2";

/// The fake artifact of `BUILD` and a fake BCApps checkout.
fn fixture(root: &Path, fail_b: bool) {
    support::fake_artifact(root, BUILD, fail_b);
}

fn run(root: &Path, extra: &[&str]) -> Output {
    run_cmd(root, &["test-microsoft"], extra)
}

fn run_cmd(root: &Path, cmd: &[&str], extra: &[&str]) -> Output {
    let cache = root.join("cache");
    let bcapps = root.join("bcapps");
    let mut args: Vec<&str> = cmd.to_vec();
    args.extend([
        "--bc-version",
        BUILD,
        "--no-download",
        "--cache-dir",
        cache.to_str().unwrap(),
        "--bcapps",
        bcapps.to_str().unwrap(),
    ]);
    args.extend_from_slice(extra);
    Command::new(env!("CARGO_BIN_EXE_al-runner"))
        .env("HOME", root.join("home"))
        .env("AL_CHECK_THREADS", "2")
        .env("AL_LOWER_THREADS", "2")
        .env("AL_TEST_THREADS", "2")
        .env("ALRUNNER_NO_PATH_NOTE", "1")
        .args(&args)
        .output()
        .expect("run al-runner")
}

/// The numbers of a row of the summary table.
fn row(out: &str, name: &str) -> String {
    let line = out
        .lines()
        .find(|l| l.trim_start().starts_with(&format!("{name} ")))
        .unwrap_or_else(|| panic!("no row {name} in:\n{out}"));
    line.trim_start()[name.len()..]
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn every_test_app_of_the_build_runs_by_the_recipe_and_libraries_are_left_out() {
    let dir = tempfile::tempdir().unwrap();
    fixture(dir.path(), false);
    let o = run(dir.path(), &[]);
    let out = text(&o);
    assert!(
        out.contains(&format!("Microsoft tests: BC {BUILD} w1, 3 test app(s)")),
        "{out}"
    );
    assert!(out.contains("left out: Fake Test Library: no test codeunit"), "{out}");
    // Microsoft's CI runs the `_Exclude_` test apps, so the default does too.
    assert!(!out.contains("left out: _Exclude_Fake Tests"), "{out}");
    assert!(!out.contains("== test app Fake Test Library"), "{out}");
    // passed failed not-run total codeunits-left exit. The disabled test is skipped by the recipe, not run.
    assert_eq!(row(&out, "Tests-Fake-A"), "1 0 0 1 0 0", "{out}");
    assert_eq!(row(&out, "Tests-Fake-B"), "1 0 0 1 0 0", "{out}");
    assert_eq!(row(&out, "_Exclude_Fake"), "Tests 0 1 0 1 0 1", "{out}");
    assert_eq!(row(&out, "Total"), "2 1 0 3 0 1", "{out}");
    assert_eq!(o.status.code(), Some(1), "{out}");
}

#[test]
fn app_narrows_to_the_named_test_apps_and_the_worst_app_sets_the_exit_code() {
    let dir = tempfile::tempdir().unwrap();
    fixture(dir.path(), true);
    let junit = dir.path().join("out/r.xml");
    let o = run(
        dir.path(),
        &["--app", "tests-fake-b", "--junit", junit.to_str().unwrap()],
    );
    let out = text(&o);
    assert!(out.contains("1 test app(s)"), "{out}");
    assert!(!out.contains("Tests-Fake-A"), "{out}");
    assert_eq!(row(&out, "Tests-Fake-B"), "0 1 0 1 0 1", "{out}");
    assert_eq!(o.status.code(), Some(1), "{out}");
    assert!(dir.path().join("out/r.Tests-Fake-B.unittest.xml").is_file(), "{out}");
}

#[test]
fn an_unknown_app_name_or_a_library_exits_2_before_any_test_runs() {
    let dir = tempfile::tempdir().unwrap();
    fixture(dir.path(), false);
    let o = run(dir.path(), &["--app", "Tests-Nope"]);
    let out = text(&o);
    assert_eq!(o.status.code(), Some(2), "{out}");
    assert!(
        out.contains("Tests-Nope") && out.contains("Tests-Fake-A, Tests-Fake-B"),
        "{out}"
    );
    let o = run(dir.path(), &["--app", "Fake Test Library"]);
    let out = text(&o);
    assert_eq!(o.status.code(), Some(2), "{out}");
    assert!(out.contains("holds no test codeunit"), "{out}");
    // An excluded app runs when named.
    let o = run(dir.path(), &["--app", "_exclude_fake tests"]);
    let out = text(&o);
    assert_eq!(row(&out, "_Exclude_Fake"), "Tests 0 1 0 1 0 1", "{out}");
}

/// The step 1 artifact plus `Tests-Imp` (the impact fixture's test app), and the extension without its own tests
/// under `root/ext-only`. Returns (ext-only, ext with own tests, lib).
fn source_fixture(root: &Path) -> (String, String, String) {
    fixture(root, false);
    let tests = impact_fixture("tests");
    let read = |f: &str| std::fs::read_to_string(tests.join(f)).unwrap();
    let test = root.join(format!(
        "home/.bcartifacts.cache/sandbox/{BUILD}/platform/Applications/Fake/Test"
    ));
    std::fs::write(
        test.join("Tests-Imp.Source.zip"),
        // The recipe's UnitTest lane (an empty company); without a TestType the codeunit is in the Uncategorized lane.
        zip(&[
            ("app.json", read("app.json")),
            (
                "src/Tests.Codeunit.al",
                read("Tests.Codeunit.al").replace("Subtype = Test;", "Subtype = Test;\n    TestType = UnitTest;"),
            ),
        ]),
    )
    .unwrap();
    let only = support::ext_only(root);
    let ext = impact_fixture("ext");
    (
        only.to_string_lossy().into_owned(),
        ext.to_string_lossy().into_owned(),
        impact_fixture("lib").to_string_lossy().into_owned(),
    )
}

#[test]
fn test_microsoft_with_an_extension_runs_the_microsoft_tests_that_reach_it_with_and_without_it() {
    let dir = tempfile::tempdir().unwrap();
    let (ext, _, lib) = source_fixture(dir.path());
    let junit = dir.path().join("out/r.xml");
    let o = run(dir.path(), &[&ext, "--dep", &lib, "--junit", junit.to_str().unwrap()]);
    let out = text(&o);
    assert!(
        out.contains(&format!("== microsoft: BC {BUILD} w1, 4 test app(s)")),
        "{out}"
    );
    // No index is cached: static selection, with the exact command that builds the index of that test app.
    assert!(
        out.contains(&format!(
            "al-runner impact-index build --from microsoft --microsoft-app Tests-Imp --bc-version {BUILD}"
        )),
        "{out}"
    );
    assert!(
        out.contains("50510 Imp Tests.PostKeepsALargeAmount: pass -> fail: amount 20"),
        "{out}"
    );
    // tests run, ran your code, newly failing, fails differently, newly refused, not verified, exit. Tests-Fake-A and
    // -B, `_Exclude_Fake Tests` and the second codeunit of Tests-Imp reach nothing of the extension: they are not
    // selected, never counted as passes (the disabled test of Tests-Fake-A is among them).
    let r = source_row(&out, "microsoft");
    assert!(r.ends_with(" 1 0 0 0 1"), "{r}\n{out}");
    assert!(
        out.contains("not selected: 5 test(s) do not reach your extension"),
        "{out}"
    );
    assert!(
        out.contains("Result: FAIL (microsoft: 1 newly failing) (exit 1)"),
        "{out}"
    );
    assert_eq!(o.status.code(), Some(1), "{out}");
    let x = std::fs::read_to_string(&junit).unwrap();
    assert!(
        x.contains("<property name=\"test_source\" value=\"microsoft\" />"),
        "{x}"
    );
    assert_eq!(
        x.matches("<property name=\"not_selected\" value=\"5\" />").count(),
        1,
        "{x}"
    );
    assert!(
        x.contains("<property name=\"impact_class\" value=\"newly failing\" />"),
        "{x}"
    );
    assert!(
        !x.contains("APasses") && !x.contains("BRuns"),
        "not selected tests are not in the file: {x}"
    );
}

#[test]
fn own_tests_and_the_microsoft_source_make_one_report_and_the_worst_part_sets_the_exit_code() {
    let dir = tempfile::tempdir().unwrap();
    let (_, ext, lib) = source_fixture(dir.path());
    let junit = dir.path().join("r.xml");
    let o = run_cmd(
        dir.path(),
        &["test"],
        &[
            &ext,
            "--affected-from",
            "microsoft",
            "--microsoft-app",
            "Tests-Imp",
            "--dep",
            &lib,
            "--junit",
            junit.to_str().unwrap(),
        ],
    );
    let out = text(&o);
    // The extension holds a subscriber and a test codeunit: it is the extension and an own test app.
    assert!(out.contains("== own tests: Impact Ext 1.0.0.0 =="), "{out}");
    assert!(
        out.contains(&format!("== microsoft: BC {BUILD} w1, 1 test app(s)")),
        "{out}"
    );
    assert_eq!(source_row(&out, "own tests"), "1 - - - - 0 not run 1", "{out}");
    assert!(source_row(&out, "microsoft").ends_with(" 1 0 0 0 1"), "{out}");
    assert!(
        out.contains("Result: FAIL (own tests: 1 failed; microsoft: 1 newly failing) (exit 1)"),
        "{out}"
    );
    assert_eq!(o.status.code(), Some(1), "{out}");
    let x = std::fs::read_to_string(&junit).unwrap();
    assert!(x.contains("OwnTest") && x.contains("PostKeepsALargeAmount"), "{x}");
}

#[test]
fn an_index_built_from_microsoft_turns_on_dynamic_selection() {
    let dir = tempfile::tempdir().unwrap();
    let (ext, _, lib) = source_fixture(dir.path());
    let o = run_cmd(
        dir.path(),
        &["impact-index", "build"],
        &["--from", "microsoft", "--microsoft-app", "Tests-Imp", "--dep", &lib],
    );
    let out = text(&o);
    assert_eq!(o.status.code(), Some(0), "{out}");
    assert!(
        out.contains("Tests-Imp lane UnitTest:") && out.contains("impact index: "),
        "{out}"
    );
    let o = run(dir.path(), &[&ext, "--app", "Tests-Imp", "--dep", &lib]);
    let out = text(&o);
    assert!(out.contains("dynamic selection in 1 run(s)"), "{out}");
    assert!(
        !out.contains("impact-index build --from"),
        "no warning with an index: {out}"
    );
    assert!(
        out.contains("50510 Imp Tests.PostKeepsALargeAmount: pass -> fail: amount 20"),
        "{out}"
    );
    assert_eq!(o.status.code(), Some(1), "{out}");
}

#[test]
fn an_app_with_tests_in_test_microsoft_or_no_extension_at_all_exits_2() {
    let dir = tempfile::tempdir().unwrap();
    let (_, ext, lib) = source_fixture(dir.path());
    let o = run(dir.path(), &[&ext, "--dep", &lib]);
    let out = text(&o);
    assert_eq!(o.status.code(), Some(2), "{out}");
    assert!(
        out.contains("Impact Ext 1.0.0.0 holds test codeunits") && out.contains("--affected-from microsoft"),
        "{out}"
    );
    let tests = impact_fixture("tests");
    let o = run_cmd(
        dir.path(),
        &["test"],
        &[tests.to_str().unwrap(), "--affected-from", "microsoft", "--dep", &lib],
    );
    let out = text(&o);
    assert_eq!(o.status.code(), Some(2), "{out}");
    assert!(out.contains("nothing to measure"), "{out}");
}

#[test]
fn a_codeunit_of_a_lane_the_runner_cannot_build_is_not_verified_with_the_lane_reason() {
    let dir = tempfile::tempdir().unwrap();
    let (ext, _, lib) = source_fixture(dir.path());
    let tests = impact_fixture("tests");
    let read = |f: &str| std::fs::read_to_string(tests.join(f)).unwrap();
    let test = dir.path().join(format!(
        "home/.bcartifacts.cache/sandbox/{BUILD}/platform/Applications/Fake/Test"
    ));
    // The same test app without a TestType: Microsoft's recipe puts it in the Uncategorized lane (Contoso data).
    std::fs::write(
        test.join("Tests-Imp-Uncategorized.Source.zip"),
        zip(&[
            (
                "app.json",
                read("app.json")
                    .replace(
                        "43000000-0000-0000-0000-000000000002",
                        "43000000-0000-0000-0000-000000000012",
                    )
                    .replace("Impact Tests", "Impact Tests U"),
            ),
            ("src/Tests.Codeunit.al", read("Tests.Codeunit.al")),
        ]),
    )
    .unwrap();
    let o = run(dir.path(), &[&ext, "--app", "Tests-Imp-Uncategorized", "--dep", &lib]);
    let out = text(&o);
    assert!(
        out.contains("Tests-Imp-Uncategorized lane Uncategorized: 2 codeunit(s) not verified: lane not run yet: the Uncategorized company is built by Microsoft's Contoso Demo Tool"),
        "{out}"
    );
    assert!(out.contains("microsoft: no impact run;"), "{out}");
    // Nothing ran, nothing reached the extension's code: a source that selects nothing gives 0, not a pass count.
    assert_eq!(source_row(&out, "microsoft"), "0 0 0 0 0 0 0", "{out}");
    assert_eq!(o.status.code(), Some(0), "{out}");
}

#[test]
fn the_microsoft_source_writes_one_impact_json_entry_per_test_app_and_lane_and_reads_it_back_as_accept_file() {
    let dir = tempfile::tempdir().unwrap();
    let (ext, _, lib) = source_fixture(dir.path());
    let diff = dir.path().join("diff.json");
    let o = run(
        dir.path(),
        &[&ext, "--dep", &lib, "--impact-json", diff.to_str().unwrap()],
    );
    assert_eq!(o.status.code(), Some(1), "{}", text(&o));
    let doc: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&diff).unwrap()).unwrap();
    assert_eq!(doc["format"], 2, "{doc}");
    let imp: Vec<&serde_json::Value> = doc["sources"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s["testApp"] == "Tests-Imp")
        .collect();
    assert_eq!(imp.len(), 1, "{doc}");
    assert_eq!(imp[0]["source"], "microsoft", "{doc}");
    assert_eq!(imp[0]["lane"], "UnitTest", "{doc}");
    assert_eq!(imp[0]["counts"]["newlyFailing"], 1, "{doc}");
    // Given back, the change is accepted in its run, and no entry is stale in the runs of the other test apps.
    let o = run(
        dir.path(),
        &[&ext, "--dep", &lib, "--impact-accept", diff.to_str().unwrap()],
    );
    let out = text(&o);
    assert_eq!(o.status.code(), Some(0), "{out}");
    assert!(!out.contains("no longer match"), "{out}");
}
