//! `al-runner test --impact-of` (ADR-043 step 1: two runs and a diff) end to end on the small fixture apps in
//! `tests/fixtures/impact`. Nothing here is Microsoft's code: `lib` stands in for the Base App (a publisher),
//! `tests` for a Microsoft test app, `ext` for a partner extension (a subscriber), `unrelated` for an app that
//! touches nothing the tests reach.

use std::path::PathBuf;
use std::process::{Command, Output};

fn fixture(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/impact")
        .join(name)
        .to_string_lossy()
        .into_owned()
}

fn alrunner(args: &[&str]) -> Output {
    let cache = tempfile::tempdir().unwrap();
    Command::new(env!("CARGO_BIN_EXE_al-runner"))
        .env("AL_CHECK_THREADS", "2")
        .env("AL_LOWER_THREADS", "2")
        .env("AL_TEST_THREADS", "2")
        // The note about another al-runner on PATH (the machine may have AL Runner v2) stays out of these outputs.
        .env("ALRUNNER_NO_PATH_NOTE", "1")
        .args(args)
        .args(["--no-download", "--cache-dir", cache.path().to_str().unwrap()])
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

/// The lines of the report from the section headed `head` up to the next blank line.
fn section(t: &str, head: &str) -> String {
    let Some(start) = t.find(&format!("\n{head} (")) else {
        return String::new();
    };
    t[start + 1..].split("\n\n").next().unwrap_or_default().to_string()
}

#[test]
fn an_impact_run_classifies_each_test_and_names_the_extension_routines_that_ran() {
    let (tests, ext, lib) = (fixture("tests"), fixture("ext"), fixture("lib"));
    let o = alrunner(&["test", &tests, "--impact-of", &ext, "--impact", "all", "--dep", &lib]);
    let t = text(&o);
    // A test that passed without the extension and fails with it is a breaking change: exit 1.
    assert_eq!(code(&o), 1, "{t}");
    assert!(
        t.contains(
            "Impact: 5 tests: 1 newly failing, 0 fails differently, 0 newly refused, 0 selection miss, 0 newly passing, 0 newly runs, 1 same (reached), 3 same (not reached), 0 not verified"
        ),
        "{t}"
    );
    let failing = section(&t, "newly failing");
    assert!(
        failing.contains("50510 Imp Tests.PostKeepsALargeAmount: pass -> fail: amount 20"),
        "{t}"
    );
    assert!(
        failing.contains("ran: Imp Ext Subscriber.DoubleLargeAmounts (1 call)"),
        "{t}"
    );
    let reached = section(&t, "same, reached");
    assert!(reached.contains("50510 Imp Tests.PostKeepsASmallAmount: pass"), "{t}");
    assert!(
        reached.contains("ran: Imp Ext Subscriber.DoubleLargeAmounts (2 calls)"),
        "{t}"
    );
    // Unchanged tests that ran no extension code are counted, not listed.
    assert!(!t.contains("Imp Tests.PostsNothing"), "{t}");
    // A failure on both sides with the same text is unchanged.
    assert!(!t.contains("fails differently ("), "{t}");
    // The extension's own test codeunits are not part of the run.
    assert!(
        !t.contains("OwnTest") && !t.contains("the extension's own test ran"),
        "{t}"
    );
}

#[test]
fn outcomes_do_not_depend_on_the_selection_or_on_an_unrelated_app_in_the_compile() {
    // ADR-043 section 2.2, condition 1: the same codeunit alone and in a full run, with and without an
    // unrelated app in the compile, gives the same outcomes.
    let (tests, unrelated, lib) = (fixture("tests"), fixture("unrelated"), fixture("lib"));
    let all = alrunner(&[
        "test",
        &tests,
        "--impact-of",
        &unrelated,
        "--impact",
        "all",
        "--dep",
        &lib,
    ]);
    let t = text(&all);
    assert_eq!(code(&all), 0, "{t}");
    assert!(
        t.contains(
            "Impact: 5 tests: 0 newly failing, 0 fails differently, 0 newly refused, 0 selection miss, 0 newly passing, 0 newly runs, 0 same (reached), 5 same (not reached), 0 not verified"
        ),
        "{t}"
    );
    let one = alrunner(&[
        "test",
        &tests,
        "--impact-of",
        &unrelated,
        "--impact",
        "all",
        "--dep",
        &lib,
        "--codeunit",
        "50510",
    ]);
    let t = text(&one);
    assert_eq!(code(&one), 0, "{t}");
    assert!(
        t.contains(
            "Impact: 4 tests: 0 newly failing, 0 fails differently, 0 newly refused, 0 selection miss, 0 newly passing, 0 newly runs, 0 same (reached), 4 same (not reached), 0 not verified"
        ),
        "{t}"
    );
    // The outcomes of codeunit 50510 alone equal its outcomes in the full run (a plain run, no impact).
    let full = text(&alrunner(&["test", &tests, "--dep", &lib, "--verbose"]));
    let alone = text(&alrunner(&[
        "test",
        &tests,
        "--dep",
        &lib,
        "--verbose",
        "--codeunit",
        "50510",
    ]));
    let of_50510 = |t: &str| -> Vec<String> {
        let mut v: Vec<String> = t
            .lines()
            .filter(|l| l.contains(" 50510 Imp Tests."))
            .map(|l| {
                // Drop the time column: `PASS    12 ms 50510 ...`.
                let (kind, rest) = l.split_once(' ').unwrap_or((l, ""));
                format!("{kind} {}", &rest[rest.find("50510").unwrap_or(0)..])
            })
            .collect();
        v.sort();
        v
    };
    assert_eq!(of_50510(&full).len(), 4, "{full}");
    assert_eq!(of_50510(&full), of_50510(&alone), "{full}\n{alone}");
}

#[test]
fn without_an_index_the_run_selects_by_static_reach_and_says_what_that_misses() {
    let (tests, ext, lib, unrelated) = (fixture("tests"), fixture("ext"), fixture("lib"), fixture("unrelated"));
    // No index cached: static selection (ADR-043 owner decision 1), with a warning before the run and again above
    // the summary that says what static reach misses and how to build the index.
    let o = alrunner(&["test", &tests, "--impact-of", &ext, "--dep", &lib]);
    let t = text(&o);
    assert_eq!(code(&o), 1, "{t}");
    assert_eq!(t.matches("warning: no impact index").count(), 2, "{t}");
    assert!(t.contains("tests were selected by static reach, which misses"), "{t}");
    assert!(t.contains("RecordRef on a table number from a variable"), "{t}");
    assert!(
        t.contains("0 selected tests reach such a site (\"cannot rule out\")"),
        "{t}"
    );
    assert!(
        t.contains("Build the index once with: al-runner impact-index build"),
        "{t}"
    );
    // Only codeunit 50510 calls the publisher the extension subscribes to; both sides run it.
    assert!(t.contains("static selection: selected 1 of 2 test codeunits"), "{t}");
    assert!(t.contains("\nwithout the extension:"), "{t}");
    assert!(
        t.contains(
            "Impact: 4 tests: 1 newly failing, 0 fails differently, 0 newly refused, 0 selection miss, 0 newly passing, 0 newly runs, 1 same (reached), 2 same (not reached), 0 not verified"
        ),
        "{t}"
    );
    assert!(t.contains("not selected: 1 test in 1 codeunit"), "{t}");
    let failing = section(&t, "newly failing");
    assert!(
        failing.contains("50510 Imp Tests.PostKeepsALargeAmount: pass -> fail: amount 20"),
        "{t}"
    );
    assert!(failing.contains("reached: subscriber"), "{t}");
    // `--impact static` asks for the same selection: no warning.
    let o = alrunner(&["test", &tests, "--impact-of", &ext, "--impact", "static", "--dep", &lib]);
    let t = text(&o);
    assert_eq!(code(&o), 1, "{t}");
    assert!(!t.contains("warning: no impact index"), "{t}");
    assert!(t.contains("static selection: selected 1 of 2 test codeunits"), "{t}");
    // An extension that reaches nothing the tests reach selects nothing.
    let o = alrunner(&["test", &tests, "--impact-of", &unrelated, "--dep", &lib]);
    let t = text(&o);
    assert_eq!(code(&o), 0, "{t}");
    assert!(t.contains("static selection: selected 0 of 2 test codeunits"), "{t}");
    // `--impact` without `--impact-of` has nothing to compare.
    let o = alrunner(&["test", &tests, "--impact", "all", "--dep", &lib]);
    assert_eq!(code(&o), 2, "{}", text(&o));
    // Both sides must compile the dependencies the same way: from the compile cache, never on demand.
    let o = alrunner(&[
        "test",
        &tests,
        "--impact-of",
        &ext,
        "--impact",
        "all",
        "--dep",
        &lib,
        "--no-compile-cache",
    ]);
    assert_eq!(code(&o), 2, "{}", text(&o));
    assert!(text(&o).contains("--no-compile-cache"), "{}", text(&o));
    // An index is built without an extension.
    let o = alrunner(&["impact-index", "build", &tests, "--impact-of", &ext, "--dep", &lib]);
    assert_eq!(code(&o), 2, "{}", text(&o));
}

/// Issue 8059 review: the index holds every test of the APPs, so a selection of any kind (the exact exclusion too) is
/// refused before any test runs; a short index would replace the full one.
#[test]
fn an_index_build_refuses_every_kind_of_selection() {
    let (tests, lib) = (fixture("tests"), fixture("lib"));
    for flags in [
        ["--exclude-test-exact", "X"],
        ["--exclude-test", "X"],
        ["--test-exact", "X"],
        ["--test", "X"],
        ["--codeunit", "50510"],
    ] {
        let mut args = vec!["impact-index", "build", tests.as_str(), "--dep", lib.as_str()];
        args.extend(flags);
        let o = alrunner(&args);
        assert_eq!(code(&o), 2, "{flags:?}: {}", text(&o));
        assert!(
            text(&o).contains("runs every test of the APPs"),
            "{flags:?}: {}",
            text(&o)
        );
    }
}

fn alrunner_in(cache: &std::path::Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_al-runner"))
        .env("AL_CHECK_THREADS", "2")
        .env("AL_LOWER_THREADS", "2")
        .env("AL_TEST_THREADS", "2")
        // The note about another al-runner on PATH (the machine may have AL Runner v2) stays out of these outputs.
        .env("ALRUNNER_NO_PATH_NOTE", "1")
        .args(args)
        .args(["--no-download", "--cache-dir", cache.to_str().unwrap()])
        .output()
        .expect("run al-runner")
}

#[test]
fn with_an_index_the_run_selects_the_codeunits_that_reach_the_extension_and_runs_them_once() {
    let (tests, ext, lib, unrelated) = (fixture("tests"), fixture("ext"), fixture("lib"), fixture("unrelated"));
    let cache = tempfile::tempdir().unwrap();
    let o = alrunner_in(cache.path(), &["impact-index", "build", &tests, "--dep", &lib]);
    let t = text(&o);
    assert_eq!(code(&o), 0, "{t}");
    assert!(t.contains("impact index: 5 tests in 2 codeunits"), "{t}");

    // The default with an index is dynamic selection: only codeunit 50510 calls the publisher the extension
    // subscribes to. The "without" side comes from the index.
    let o = alrunner_in(cache.path(), &["test", &tests, "--impact-of", &ext, "--dep", &lib]);
    let t = text(&o);
    assert_eq!(code(&o), 1, "{t}");
    assert!(!t.contains("warning: no impact index"), "{t}");
    assert!(t.contains("selected 1 of 2 test codeunits"), "{t}");
    assert!(!t.contains("\nwithout the extension:"), "{t}");
    // The same classes as `--impact all` for the codeunit that ran, and the rest counted as not selected.
    assert!(
        t.contains(
            "Impact: 4 tests: 1 newly failing, 0 fails differently, 0 newly refused, 0 selection miss, 0 newly passing, 0 newly runs, 1 same (reached), 2 same (not reached), 0 not verified"
        ),
        "{t}"
    );
    assert!(t.contains("not selected: 1 test in 1 codeunit"), "{t}");
    let failing = section(&t, "newly failing");
    assert!(
        failing.contains("50510 Imp Tests.PostKeepsALargeAmount: pass -> fail: amount 20"),
        "{t}"
    );
    assert!(failing.contains("reached: subscriber"), "{t}");

    // An extension that reaches nothing the tests reach selects nothing and runs nothing.
    let o = alrunner_in(
        cache.path(),
        &["test", &tests, "--impact-of", &unrelated, "--dep", &lib],
    );
    let t = text(&o);
    assert_eq!(code(&o), 0, "{t}");
    assert!(t.contains("selected 0 of 2 test codeunits"), "{t}");

    // `--impact all` still runs both sides, index or not.
    let o = alrunner_in(
        cache.path(),
        &["test", &tests, "--impact-of", &ext, "--impact", "all", "--dep", &lib],
    );
    let t = text(&o);
    assert_eq!(code(&o), 1, "{t}");
    assert!(t.contains("\nwithout the extension:"), "{t}");
}

#[test]
fn tests_the_index_run_refused_get_the_static_verdict() {
    let (tests, ext, lib) = (fixture("refused-tests"), fixture("ext"), fixture("lib"));
    let cache = tempfile::tempdir().unwrap();
    let o = alrunner_in(cache.path(), &["impact-index", "build", &tests, "--dep", &lib]);
    assert_eq!(code(&o), 0, "{}", text(&o));
    let o = alrunner_in(cache.path(), &["test", &tests, "--impact-of", &ext, "--dep", &lib]);
    let t = text(&o);
    // Neither test recorded anything in the index's run: dynamic selection leaves their codeunit out.
    assert!(t.contains("selected 0 of 1 test codeunits"), "{t}");
    let refused = section(&t, "refused in the index's run");
    assert!(
        refused.contains("refused in the index's run (2): the index holds nothing for them; static reach: 1 reaches an impact point, 0 cannot rule one out, 1 reaches none"),
        "{t}"
    );
    assert!(
        refused.contains("50520 Imp Refused Tests.PostsAfterARefusal: static: reaches subscriber"),
        "{t}"
    );
    assert!(!refused.contains("RefusesOnly"), "{t}");
}

#[test]
fn static_reach_covers_what_the_index_recorded() {
    let (tests, lib) = (fixture("tests"), fixture("lib"));
    let cache = tempfile::tempdir().unwrap();
    // Without an index there is nothing to check against.
    let o = alrunner_in(cache.path(), &["impact-index", "check-static", &tests, "--dep", &lib]);
    assert_eq!(code(&o), 2, "{}", text(&o));
    assert!(text(&o).contains("no impact index"), "{}", text(&o));
    let o = alrunner_in(cache.path(), &["impact-index", "build", &tests, "--dep", &lib]);
    assert_eq!(code(&o), 0, "{}", text(&o));
    // Every key a test recorded is in its static reach: no miss, exit 0.
    let o = alrunner_in(cache.path(), &["impact-index", "check-static", &tests, "--dep", &lib]);
    let t = text(&o);
    assert_eq!(code(&o), 0, "{t}");
    assert!(t.contains("static reach covers every key of 5 tests"), "{t}");
    assert!(t.contains("missed: 0"), "{t}");
}

#[test]
fn the_json_diff_can_be_given_back_as_the_accept_file_and_then_only_new_differences_fail_the_run() {
    let (tests, ext, lib) = (fixture("tests"), fixture("ext"), fixture("lib"));
    let dir = tempfile::tempdir().unwrap();
    let json = dir.path().join("impact.json");
    let json_s = json.to_str().unwrap();
    let o = alrunner(&[
        "test",
        &tests,
        "--impact-of",
        &ext,
        "--impact",
        "all",
        "--dep",
        &lib,
        "--impact-json",
        json_s,
    ]);
    let t = text(&o);
    assert_eq!(code(&o), 1, "{t}");
    let doc: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&json).unwrap()).unwrap();
    assert_eq!(doc["format"], 1, "{doc}");
    assert_eq!(doc["exitCode"], 1, "{doc}");
    assert_eq!(doc["counts"]["newlyFailing"], 1, "{doc}");
    assert_eq!(doc["counts"]["sameNotReached"], 3, "{doc}");
    let rows = doc["tests"].as_array().unwrap();
    assert_eq!(rows.len(), 5, "{doc}");
    let failing = rows.iter().find(|r| r["test"] == "PostKeepsALargeAmount").unwrap();
    assert_eq!(failing["class"], "newly failing", "{doc}");
    assert_eq!(failing["codeunit"], 50510, "{doc}");
    assert_eq!(failing["without"]["outcome"], "pass", "{doc}");
    assert_eq!(failing["with"]["outcome"], "fail", "{doc}");
    assert_eq!(failing["with"]["message"], "amount 20", "{doc}");
    assert_eq!(
        failing["ran"][0]["routine"], "Imp Ext Subscriber.DoubleLargeAmounts",
        "{doc}"
    );
    assert_eq!(failing["ran"][0]["calls"], 1, "{doc}");
    assert_eq!(failing["accepted"], false, "{doc}");

    // The same file given back accepts that change: the run reports it, marked accepted, and exits 0.
    let o = alrunner(&[
        "test",
        &tests,
        "--impact-of",
        &ext,
        "--impact",
        "all",
        "--dep",
        &lib,
        "--impact-accept",
        json_s,
    ]);
    let t = text(&o);
    assert_eq!(code(&o), 0, "{t}");
    let failing = section(&t, "newly failing");
    assert!(failing.contains("newly failing (1, 1 accepted)"), "{t}");
    assert!(
        failing.contains("50510 Imp Tests.PostKeepsALargeAmount: pass -> fail: amount 20 (accepted)"),
        "{t}"
    );
    assert!(t.contains("1 accepted change"), "{t}");

    // An accepted change whose error text is not the one of this run is a new difference: exit 1, and the report
    // says which entry of the accept file no longer matches.
    let edited = std::fs::read_to_string(&json)
        .unwrap()
        .replace("\"amount 20\"", "\"amount 30\"");
    let accept = dir.path().join("accept.json");
    std::fs::write(&accept, edited).unwrap();
    let o = alrunner(&[
        "test",
        &tests,
        "--impact-of",
        &ext,
        "--impact",
        "all",
        "--dep",
        &lib,
        "--impact-accept",
        accept.to_str().unwrap(),
    ]);
    let t = text(&o);
    assert_eq!(code(&o), 1, "{t}");
    assert!(section(&t, "newly failing").contains("newly failing (1)"), "{t}");
    assert!(t.contains("accept file: 1 accepted change no longer matches"), "{t}");
    assert!(
        t.contains("50510 Imp Tests.PostKeepsALargeAmount: accepted fail: amount 30"),
        "{t}"
    );

    // An accept file that cannot be read stops the run before it starts.
    std::fs::write(&accept, "not json").unwrap();
    let o = alrunner(&[
        "test",
        &tests,
        "--impact-of",
        &ext,
        "--impact",
        "all",
        "--dep",
        &lib,
        "--impact-accept",
        accept.to_str().unwrap(),
    ]);
    assert_eq!(code(&o), 2, "{}", text(&o));
    assert!(text(&o).contains("--impact-accept"), "{}", text(&o));
    // Both flags belong to an impact run.
    let o = alrunner(&["test", &tests, "--dep", &lib, "--impact-json", json_s]);
    assert_eq!(code(&o), 2, "{}", text(&o));
}

#[test]
fn junit_and_the_json_document_of_an_impact_run_hold_the_run_with_the_extension() {
    let (tests, ext, lib) = (fixture("tests"), fixture("ext"), fixture("lib"));
    let dir = tempfile::tempdir().unwrap();
    let junit = dir.path().join("impact.xml");
    let out = dir.path().join("out.json");
    let o = alrunner(&[
        "test",
        &tests,
        "--impact-of",
        &ext,
        "--impact",
        "all",
        "--dep",
        &lib,
        "--junit",
        junit.to_str().unwrap(),
        "--output-json",
        out.to_str().unwrap(),
    ]);
    let t = text(&o);
    assert_eq!(code(&o), 1, "{t}");
    let x = std::fs::read_to_string(&junit).unwrap();
    // One file, for the "with" run, with the change class of each test as a property.
    assert_eq!(x.matches("<testcase ").count(), 5, "{x}");
    assert!(x.contains("<testsuites tests=\"5\" failures=\"2\""), "{x}");
    let case = |name: &str| -> String {
        let start = x.find(&format!("name=\"{name}\"")).unwrap();
        x[start..].split("</testcase>").next().unwrap().to_string()
    };
    assert!(
        case("PostKeepsALargeAmount").contains("<property name=\"impact_class\" value=\"newly failing\" />"),
        "{x}"
    );
    assert!(
        case("PostKeepsALargeAmount").contains("<property name=\"impact_without\" value=\"pass\" />"),
        "{x}"
    );
    assert!(
        case("PostKeepsASmallAmount").contains("<property name=\"impact_class\" value=\"same, reached\" />"),
        "{x}"
    );
    assert!(
        case("PostsNothing").contains("<property name=\"impact_class\" value=\"same, not reached\" />"),
        "{x}"
    );
    // The JSON document of AL Runner v2's shape: the "with" run, with the exit code of the impact run.
    let doc: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
    assert_eq!(doc["total"], 5, "{doc}");
    assert_eq!(doc["failed"], 2, "{doc}");
    assert_eq!(doc["exitCode"], 1, "{doc}");
    // The JSON document on stdout would mix with the impact report.
    let o = alrunner(&[
        "test",
        &tests,
        "--impact-of",
        &ext,
        "--impact",
        "all",
        "--dep",
        &lib,
        "--output-json",
    ]);
    assert_eq!(code(&o), 2, "{}", text(&o));
}

#[test]
fn with_an_index_the_json_diff_counts_the_tests_left_out() {
    let (tests, ext, lib) = (fixture("tests"), fixture("ext"), fixture("lib"));
    let cache = tempfile::tempdir().unwrap();
    let o = alrunner_in(cache.path(), &["impact-index", "build", &tests, "--dep", &lib]);
    assert_eq!(code(&o), 0, "{}", text(&o));
    let json = cache.path().join("impact.json");
    let junit = cache.path().join("impact.xml");
    let o = alrunner_in(
        cache.path(),
        &[
            "test",
            &tests,
            "--impact-of",
            &ext,
            "--dep",
            &lib,
            "--impact-json",
            json.to_str().unwrap(),
            "--junit",
            junit.to_str().unwrap(),
        ],
    );
    let t = text(&o);
    assert_eq!(code(&o), 1, "{t}");
    let doc: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&json).unwrap()).unwrap();
    assert_eq!(doc["mode"], "dynamic", "{doc}");
    assert_eq!(doc["notSelected"]["tests"], 1, "{doc}");
    assert_eq!(doc["notSelected"]["codeunits"], 1, "{doc}");
    let failing = doc["tests"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["test"] == "PostKeepsALargeAmount")
        .unwrap();
    assert!(
        failing["reached"][0].as_str().unwrap().starts_with("subscriber"),
        "{doc}"
    );
    // JUnit holds the codeunit that ran with the extension.
    let x = std::fs::read_to_string(&junit).unwrap();
    assert_eq!(x.matches("<testcase ").count(), 4, "{x}");
    assert!(x.contains("value=\"newly failing\""), "{x}");
}
