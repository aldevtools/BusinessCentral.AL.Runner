//! End-to-end: the CLI against the real, pinned corpus and the real oracle.
//! These need the `corpus` submodule; a missing checkout fails loudly rather
//! than skipping, because a skipped harness test reads as a green one.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn require_corpus() {
    assert!(
        root().join("corpus/tests/al-language/app.json").exists(),
        "corpus submodule not checked out: run `git submodule update --init`"
    );
}

/// Runs the CLI over the corpus on its own: without the Microsoft apps, which would make
/// each run in this unoptimised test build take minutes (the release CI step runs with
/// them). No list check unless the test brings a list: the real lists assume
/// those apps.
fn cli(args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_al-test-cli"));
    cmd.arg("--root").arg(root()).arg("--no-deps");
    if !args.contains(&"--baseline") {
        cmd.arg("--no-ratchet");
    }
    let out = cmd.args(args).output().unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into(),
        String::from_utf8_lossy(&out.stderr).into(),
    )
}

/// A test codeunit with a handful of tests that does not need the Microsoft apps. Runs that only
/// check how a flag is handled select it: a run costs a fixed compile whatever it selects, but
/// only the selected cases are run.
const SMALL_CU: &str = "60184";

/// A test codeunit of the OnPrem app (`corpus/tests/al-language-onprem`), whose cases need the
/// Microsoft apps and are refused in these `--no-deps` runs.
const ONPREM_CU: &str = "61200";

/// The one run over every oracle version that most tests below read their numbers from, so
/// that the corpus is loaded, compiled and run three times in total and not once per assertion.
/// It is `--all-versions` with the three reports written, exactly the CI corpus step minus the
/// Microsoft apps.
struct AllVersions {
    dir: PathBuf,
    code: i32,
    out: String,
    err: String,
}

fn all_versions() -> &'static AllVersions {
    static RUN: OnceLock<AllVersions> = OnceLock::new();
    RUN.get_or_init(|| {
        require_corpus();
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("corpus_harness_all_versions");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = |f: &str| dir.join(f).to_str().unwrap().to_string();
        let (json, md, junit) = (path("pass-rate.json"), path("pass-rate.md"), path("runtime-junit.xml"));
        let (code, out, err) = cli(&["--all-versions", "--json", &json, "--markdown", &md, "--junit", &junit]);
        AllVersions { dir, code, out, err }
    })
}

impl AllVersions {
    fn file(&self, stem: &str, major: &str, ext: &str) -> PathBuf {
        self.dir.join(format!("{stem}-{major}.{ext}"))
    }

    /// The report of one version (`27`, `28` or `29`).
    fn json(&self, major: &str) -> serde_json::Value {
        assert_eq!(self.code, 0, "BC {major}: {}", self.err);
        let f = self.file("pass-rate", major, "json");
        let text =
            std::fs::read_to_string(&f).unwrap_or_else(|_| panic!("no report for BC {major}; stdout:\n{}", self.out));
        serde_json::from_str(&text).unwrap()
    }
}

#[test]
fn every_oracle_case_is_reported_and_attributed_to_an_area() {
    // BC 29: the version a run without `--bc-version` picks.
    let r = all_versions().json("29");
    assert_eq!(r["overall"]["total"], 27579);
    assert_eq!(r["unmapped"].as_array().unwrap().len(), 0, "{}", r["unmapped"]);
    assert_eq!(r["areas"]["json"]["total"], 597); // corpus bump to 833a01ce: the restructure moved JSON tests into json/
    assert_eq!(r["areas"]["record"]["total"], 4258); // corpus bump to 833a01ce: the restructure merged records, filters, calcfields and system-fields into record/
}

#[test]
fn oracle_pinned_to_a_different_corpus_commit_is_refused() {
    require_corpus();
    let dir = tempfile::tempdir().unwrap();
    let t = std::fs::read_to_string(root().join("oracle/oracle.toml")).unwrap();
    let bad = dir.path().join("oracle.toml");
    std::fs::write(
        &bad,
        t.replace(
            "6e3fbaf30db077bd30faab86ed746893fe857230",
            "0000000000000000000000000000000000000000",
        ),
    )
    .unwrap();
    std::fs::create_dir(dir.path().join("27")).unwrap();
    std::fs::copy(root().join("oracle/27/junit.xml"), dir.path().join("27/junit.xml")).unwrap();
    let (code, _, err) = cli(&["--oracle", bad.to_str().unwrap()]);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("corpus pin"), "{err}");
}

/// The real oracle with an older corpus commit recorded for BC 29 (every table is on the pin).
fn oracle_with_29_recorded_for(commit: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let t = std::fs::read_to_string(root().join("oracle/oracle.toml")).unwrap();
    let marker = "bc_version = \"29.0\"\n";
    assert!(t.contains(marker), "BC 29 has a table");
    assert!(
        !t.contains("workflow_run = 38306086475"),
        "BC 29 is recorded for the pin"
    );
    let bad = dir.path().join("oracle.toml");
    std::fs::write(
        &bad,
        t.replace(marker, &format!("{marker}corpus_commit = \"{commit}\"\n")),
    )
    .unwrap();
    for v in ["27", "28", "29"] {
        std::fs::create_dir(dir.path().join(v)).unwrap();
        std::fs::copy(
            root().join(format!("oracle/{v}/junit.xml")),
            dir.path().join(format!("{v}/junit.xml")),
        )
        .unwrap();
    }
    (dir, bad)
}

#[test]
fn an_oracle_recorded_for_a_corpus_commit_that_changed_existing_tests_is_refused() {
    require_corpus();
    // 9df7c23 .. pin changes tests/al-language/record/TestAutoIncrementExplicitKey.al (ADR-007).
    let (_dir, bad) = oracle_with_29_recorded_for("9df7c23c91095f6747ace1cefaebec7dd882410a");
    let (code, _, err) = cli(&["--bc-version", "29", "--oracle", bad.to_str().unwrap()]);
    assert_eq!(code, 2, "{err}");
    // The list shows the first ten changed files; since corpus bump 28 (25 files) TestAutoIncrementExplicitKey.al is past them.
    assert!(
        err.contains("changes or removes existing test files") && err.contains("  M tests/") && err.contains("29"),
        "{err}"
    );
}

#[test]
fn an_oracle_recorded_for_a_commit_missing_from_the_checkout_is_a_setup_error() {
    require_corpus();
    let (_dir, bad) = oracle_with_29_recorded_for("0123456789012345678901234567890123456789");
    let (code, _, err) = cli(&["--bc-version", "29", "--oracle", bad.to_str().unwrap()]);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("fetch"), "{err}");
}

// ADR-065 section 2 point 3: a listed case that is no longer in the oracle is a stale line, and CI asks for its
// deletion.
#[test]
fn a_listed_case_no_longer_in_the_oracle_asks_for_its_deletion() {
    require_corpus();
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("not-passing.txt");
    std::fs::write(&base, "60002 NoSuchProcedure_UsedByTheStaleLineTest\n").unwrap();
    // The rule checks the cases of the selection, so selecting the codeunit is enough.
    let (code, _, err) = cli(&["--baseline", base.to_str().unwrap(), "--codeunit", "60002"]);
    assert_eq!(code, 1, "{err}");
    assert!(
        err.contains("stale line(s)")
            && err.contains("--prune-not-passing")
            && err.contains("60002 NoSuchProcedure_UsedByTheStaleLineTest  [not in the oracle]"),
        "{err}"
    );
}

/// A repository of its own: a git "corpus" with one test app, and an oracle for BC 27 that says real
/// BC passes both tests of the app. One test passes in the runtime and one fails on purpose, so
/// the run has a failing case whatever the real corpus holds today. The root has the layout the
/// CLI reads (`oracle/oracle.toml`, a git repository for the corpus pin, an app folder).
fn root_with_a_failing_test() -> tempfile::TempDir {
    fake_root(|_| {})
}

/// [`root_with_a_failing_test`], with `extra` called on the corpus folder (`fixture/`) before the corpus commit, to
/// add or overwrite files.
fn fake_root(extra: impl Fn(&Path)) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let at = |p: &str| dir.path().join(p);
    std::fs::create_dir_all(at("fixture/tests/app")).unwrap();
    std::fs::create_dir_all(at("oracle/27")).unwrap();
    // The CLI always loads the corpus's internals fixture app next to the app under test: an empty one.
    std::fs::create_dir_all(at("fixture/tests/al-language-internals-fixture")).unwrap();
    std::fs::write(
        at("fixture/tests/al-language-internals-fixture/app.json"),
        r#"{
  "id": "0a1b2c3d-0000-4000-8000-000000000002",
  "name": "Harness Internals Fixture",
  "publisher": "Test",
  "version": "1.0.0.0",
  "dependencies": [],
  "runtime": "16.0",
  "target": "Cloud"
}
"#,
    )
    .unwrap();
    std::fs::write(
        at("fixture/tests/app/app.json"),
        r#"{
  "id": "0a1b2c3d-0000-4000-8000-000000000001",
  "name": "Harness Fixture",
  "publisher": "Test",
  "version": "1.0.0.0",
  "platform": "27.0.0.0",
  "application": "27.0.0.0",
  "dependencies": [],
  "idRanges": [{ "from": 60900, "to": 60999 }],
  "runtime": "16.0",
  "target": "Cloud"
}
"#,
    )
    .unwrap();
    std::fs::write(
        at("fixture/tests/app/Fixture.al"),
        r#"codeunit 60900 "Harness Fixture"
{
    Subtype = Test;

    [Test]
    procedure Passes()
    begin
    end;

    [Test]
    procedure FailsOnPurpose()
    begin
        Error('the harness fixture fails on purpose');
    end;
}
"#,
    )
    .unwrap();
    std::fs::write(
        at("oracle/27/junit.xml"),
        r#"<?xml version='1.0' encoding='utf-8'?>
<testsuites name="DEFAULT" tests="2" failures="0" errors="0" skipped="0">
  <testsuite name="Codeunit 60900" tests="2" failures="0" errors="0" skipped="0">
    <testcase classname="Codeunit 60900" name="Passes" time="0.001" />
    <testcase classname="Codeunit 60900" name="FailsOnPurpose" time="0.001" />
  </testsuite>
</testsuites>
"#,
    )
    .unwrap();
    // The corpus pin is the head of the "corpus" repository. Signing is off: the machine's own
    // git configuration may ask for a key.
    let git = |args: &[&str]| {
        let out = Command::new("git")
            .arg("-C")
            .arg(at("fixture"))
            .args([
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@example.com",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    };
    extra(&at("fixture"));
    git(&["init", "-q"]);
    git(&["add", "-A"]);
    git(&["commit", "-q", "-m", "fixture"]);
    let head = git(&["rev-parse", "HEAD"]);
    std::fs::write(
        at("oracle/oracle.toml"),
        format!(
            r#"corpus_repo = "fixture"
corpus_app = "fixture/tests/app"
corpus_commit = "{head}"

[[version]]
major = 27
bc_version = "27.5"
junit = "27/junit.xml"
baseline = "metrics/27/not-passing.txt"
"#
        ),
    )
    .unwrap();
    dir
}

/// Runs the CLI on a fixture root against the list `list` (written to `<root>/not-passing.txt` first), with `extra`
/// flags. Returns the exit code, the report and standard error.
fn run_fixture(root: &Path, list: &str, extra: &[&str]) -> (i32, serde_json::Value, String) {
    let base = root.join("not-passing.txt");
    std::fs::write(&base, list).unwrap();
    let out = root.join("r.json");
    let o = Command::new(env!("CARGO_BIN_EXE_al-test-cli"))
        .arg("--root")
        .arg(root)
        .args(["--no-deps", "--bc-version", "27", "--baseline"])
        .arg(&base)
        .arg("--json")
        .arg(&out)
        .args(extra)
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&o.stderr).to_string();
    let report: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&out).unwrap_or_else(|_| panic!("no report written; stderr:\n{err}")),
    )
    .unwrap();
    (o.status.code().unwrap_or(-1), report, err)
}

// ADR-065 section 2 point 1. The run needs a failing case, and the real corpus may have none (the runtime keeps getting
// better), so it runs on a fixture whose second test fails on purpose.
#[test]
fn a_listed_failing_case_passes_the_run_and_an_unlisted_one_is_a_regression() {
    let root = root_with_a_failing_test();
    // The failing case is listed: reported, exit 0.
    let (code, r, err) = run_fixture(root.path(), "60900 FailsOnPurpose\n", &[]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(r["overall"]["total"], 2, "{}", r["overall"]);
    assert_eq!(r["overall"]["pass"], 1, "{}", r["overall"]);
    assert_eq!(r["overall"]["fail"], 1, "{}", r["overall"]);
    // Not listed: a regression, with what it does now.
    let (code, _, err) = run_fixture(root.path(), "", &[]);
    assert_eq!(code, 1, "{err}");
    assert!(
        err.contains("1 regression(s)")
            && err.contains("60900 FailsOnPurpose  [fail: the harness fixture fails on purpose]"),
        "{err}"
    );
}

// ADR-065 section 2 point 2 and owner decision 1: a listed case that passes fails the run until
// `--prune-not-passing` deletes its line; the flag deletes stale lines and adds none.
#[test]
fn a_stale_listed_case_fails_until_prune_not_passing_deletes_it() {
    let root = root_with_a_failing_test();
    let list = "60900 FailsOnPurpose\n60900 Passes\n60900 LeftTheOracle\n";
    let (code, _, err) = run_fixture(root.path(), list, &[]);
    assert_eq!(code, 1, "{err}");
    assert!(
        err.contains("2 stale line(s)")
            && err.contains("60900 Passes  [passes]")
            && err.contains("60900 LeftTheOracle  [not in the oracle]"),
        "{err}"
    );
    let base = root.path().join("not-passing.txt");
    let run_prune = || {
        Command::new(env!("CARGO_BIN_EXE_al-test-cli"))
            .arg("--root")
            .arg(root.path())
            .args(["--no-deps", "--bc-version", "27", "--prune-not-passing", "--baseline"])
            .arg(&base)
            .output()
            .unwrap()
    };
    let o = run_prune();
    assert_eq!(o.status.code(), Some(0), "{}", String::from_utf8_lossy(&o.stderr));
    let now = std::fs::read_to_string(&base).unwrap();
    assert!(now.starts_with("# Corpus cases"), "{now}");
    assert_eq!(
        now.lines().filter(|l| !l.starts_with('#')).collect::<Vec<_>>(),
        ["60900 FailsOnPurpose"]
    );
    // Pruning never adds a line: a regression stays a failure and the list keeps what it had.
    std::fs::write(&base, "").unwrap();
    let o = run_prune();
    assert_eq!(o.status.code(), Some(1), "{}", String::from_utf8_lossy(&o.stderr));
    assert_eq!(std::fs::read_to_string(&base).unwrap(), "");
}

// ADR-065 section 2 point 4: an oracle case the run recorded no outcome for fails the run, listed or not. Here the
// oracle names a procedure the fixture codeunit does not declare, so the runner never runs it.
#[test]
fn an_oracle_case_without_an_outcome_fails_the_run_listed_or_not() {
    let root = root_with_a_failing_test();
    std::fs::write(
        root.path().join("oracle/27/junit.xml"),
        r#"<?xml version='1.0' encoding='utf-8'?>
<testsuites name="DEFAULT" tests="3" failures="0" errors="0" skipped="0">
  <testsuite name="Codeunit 60900" tests="3" failures="0" errors="0" skipped="0">
    <testcase classname="Codeunit 60900" name="Passes" time="0.001" />
    <testcase classname="Codeunit 60900" name="FailsOnPurpose" time="0.001" />
    <testcase classname="Codeunit 60900" name="NeverRuns" time="0.001" />
  </testsuite>
</testsuites>
"#,
    )
    .unwrap();
    for list in ["60900 FailsOnPurpose\n", "60900 FailsOnPurpose\n60900 NeverRuns\n"] {
        let (code, _, err) = run_fixture(root.path(), list, &[]);
        assert_eq!(code, 1, "{list}: {err}");
        assert!(
            err.contains("1 oracle case(s) got no outcome") && err.contains("  60900 NeverRuns"),
            "{list}: {err}"
        );
        assert!(!err.contains("regression"), "{err}");
    }
}

/// Makes `root` a git repository whose one commit holds `oracle/` as it is now (the base branch of
/// `--list-new-cases` and `--list-changes`).
fn commit_oracle(root: &Path) {
    let git = |args: &[&str]| {
        let out = Command::new("git")
            .arg("-C")
            .arg(root)
            .args([
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@example.com",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    };
    git(&["init", "-q"]);
    git(&["add", "oracle"]);
    if root.join("not-passing.txt").exists() {
        git(&["add", "not-passing.txt"]);
    }
    git(&["commit", "-q", "-m", "base"]);
}

const ORACLE_ONLY_PASSES: &str = r#"<?xml version='1.0' encoding='utf-8'?>
<testsuites name="DEFAULT" tests="1" failures="0" errors="0" skipped="0">
  <testsuite name="Codeunit 60900" tests="1" failures="0" errors="0" skipped="0">
    <testcase classname="Codeunit 60900" name="Passes" time="0.001" />
  </testsuite>
</testsuites>
"#;

// ADR-065 section 2 ("New cases from a corpus bump are not regressions"): `--list-new-cases REV` lists a failing case
// that the oracle at REV did not have, and never one REV's oracle had.
#[test]
fn list_new_cases_adds_only_cases_the_base_oracle_lacks() {
    let root = root_with_a_failing_test();
    let junit = root.path().join("oracle/27/junit.xml");
    let bumped = std::fs::read_to_string(&junit).unwrap();
    // The base branch's oracle had only Passes; the bump adds FailsOnPurpose.
    std::fs::write(&junit, ORACLE_ONLY_PASSES).unwrap();
    commit_oracle(root.path());
    std::fs::write(&junit, &bumped).unwrap();
    let (code, _, err) = run_fixture(root.path(), "", &["--list-new-cases", "HEAD"]);
    assert_eq!(code, 0, "{err}");
    let list = std::fs::read_to_string(root.path().join("not-passing.txt")).unwrap();
    assert!(list.lines().any(|l| l == "60900 FailsOnPurpose"), "{list}");
    // The same failure against a base whose oracle already had it is a regression: no line is added.
    let root = root_with_a_failing_test();
    commit_oracle(root.path());
    let (code, _, err) = run_fixture(root.path(), "", &["--list-new-cases", "HEAD"]);
    assert_eq!(code, 1, "{err}");
    assert!(err.contains("60900 FailsOnPurpose"), "{err}");
    assert_eq!(
        std::fs::read_to_string(root.path().join("not-passing.txt")).unwrap(),
        ""
    );
    // A revision that does not exist is a setup error.
    let (code, _, err) = run_fixture(root.path(), "", &["--list-new-cases", "no-such-branch"]);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("no-such-branch"), "{err}");
}

// ADR-065 owner decision 2: a line added for a case the base oracle had is a regression shown for the owner's review;
// a line for a new case is not.
#[test]
fn list_changes_tells_accepted_regressions_from_new_cases() {
    let root = root_with_a_failing_test();
    let toml = root.path().join("oracle/oracle.toml");
    let t = std::fs::read_to_string(&toml)
        .unwrap()
        .replace("metrics/27/not-passing.txt", "not-passing.txt");
    std::fs::write(&toml, t).unwrap();
    std::fs::write(root.path().join("not-passing.txt"), "").unwrap();
    commit_oracle(root.path());
    let changes = |list: &str| {
        std::fs::write(root.path().join("not-passing.txt"), list).unwrap();
        let o = Command::new(env!("CARGO_BIN_EXE_al-test-cli"))
            .arg("--root")
            .arg(root.path())
            .args(["--list-changes", "HEAD"])
            .output()
            .unwrap();
        (
            o.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&o.stdout).to_string(),
        )
    };
    let (code, md) = changes("60900 FailsOnPurpose\n");
    assert_eq!(code, 1, "{md}");
    assert!(
        md.contains("Regressions accepted by this pull request (1)") && md.contains("60900 FailsOnPurpose"),
        "{md}"
    );
    let (code, md) = changes("");
    assert_eq!(code, 0, "{md}");
    assert!(md.contains("No list changed."), "{md}");
}

/// The corpus app depends on corpus apps next to it (corpus `tests/al-language/app.json`: the internals fixture and the
/// cross-app fixtures A, B, C, where B depends on A). The CLI loads each from source before the app under test, in
/// dependency order, whatever order the app declares them in: here the app declares B before A, and the test passes only
/// when B's codeunit reaches A's.
#[test]
fn the_corpus_apps_the_app_under_test_depends_on_are_loaded_from_source() {
    let root = fake_root(|corpus| {
        let app = |name: &str, json: &str, al: &str| {
            let d = corpus.join("tests").join(name);
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join("app.json"), json).unwrap();
            std::fs::write(d.join("Objects.al"), al).unwrap();
        };
        let dep =
            |id: &str, name: &str| format!(r#"{{"id":"{id}","name":"{name}","publisher":"Test","version":"1.0.0.0"}}"#);
        let a_id = "0a1b2c3d-0000-4000-8000-00000000000a";
        let b_id = "0a1b2c3d-0000-4000-8000-00000000000b";
        app(
            "dep-a",
            &format!(
                r#"{{"id":"{a_id}","name":"Harness Dep A","publisher":"Test","version":"1.0.0.0","dependencies":[],"idRanges":[{{"from":61900,"to":61909}}],"runtime":"16.0","target":"Cloud"}}"#
            ),
            "codeunit 61900 \"Harness A\" { procedure Value(): Integer begin exit(7); end; }",
        );
        app(
            "dep-b",
            &format!(
                r#"{{"id":"{b_id}","name":"Harness Dep B","publisher":"Test","version":"1.0.0.0","dependencies":[{}],"idRanges":[{{"from":61910,"to":61919}}],"runtime":"16.0","target":"Cloud"}}"#,
                dep(a_id, "Harness Dep A")
            ),
            "codeunit 61910 \"Harness B\" { procedure Value(): Integer var A: Codeunit \"Harness A\"; begin exit(A.Value() * 2); end; }",
        );
        std::fs::write(
            corpus.join("tests/app/app.json"),
            format!(
                r#"{{"id":"0a1b2c3d-0000-4000-8000-000000000001","name":"Harness Fixture","publisher":"Test","version":"1.0.0.0","platform":"27.0.0.0","application":"27.0.0.0","dependencies":[{},{}],"idRanges":[{{"from":60900,"to":60999}}],"runtime":"16.0","target":"Cloud"}}"#,
                dep(b_id, "Harness Dep B"),
                dep(a_id, "Harness Dep A")
            ),
        )
        .unwrap();
        std::fs::write(
            corpus.join("tests/app/Fixture.al"),
            r#"codeunit 60900 "Harness Fixture"
{
    Subtype = Test;

    [Test]
    procedure Passes()
    var
        B: Codeunit "Harness B";
    begin
        if B.Value() <> 14 then
            Error('B answered %1', B.Value());
    end;

    [Test]
    procedure FailsOnPurpose()
    begin
        Error('the harness fixture fails on purpose');
    end;
}
"#,
        )
        .unwrap();
    });
    let out = root.path().join("r.json");
    let o = Command::new(env!("CARGO_BIN_EXE_al-test-cli"))
        .arg("--root")
        .arg(root.path())
        .args(["--no-deps", "--no-ratchet", "--bc-version", "27", "--json"])
        .arg(&out)
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&o.stderr).to_string();
    let report: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&out).unwrap_or_else(|_| panic!("no report written; stderr:\n{err}")),
    )
    .unwrap();
    assert_eq!(report["overall"]["pass"], 1, "{}\n{err}", report["overall"]);
    assert_eq!(report["overall"]["fail"], 1, "{}\n{err}", report["overall"]);
}

/// The app the environment of the `env` suite adds (`extra_apps` in oracle.toml): one codeunit, which the tests run by
/// its id. It has the id of Tests-TestLibraries' "Reset State Before Test Run", which the test runner runs before
/// every test when it is installed (al-testrun `runner_reset.rs`); its OnRun does nothing.
const ENV_APP_ID: &str = "0a1b2c3d-0000-4000-8000-0000000000e1";

/// A fixture root whose main app and suite `env` each test whether codeunit 130301 of the app "Harness Env Lib" is
/// installed. The suite names that app in `extra_apps`; the app itself is in `env-lib/` of the root (outside the
/// corpus), to be passed with `--deps`.
fn root_with_an_environment_suite() -> tempfile::TempDir {
    // Codeunit.Run of an id no installed app has raises an error (corpus codeunits/TestCodeunitRunMissing.al:45).
    let probe = |cu: u32, name: &str, test: &str, installed: bool| {
        let call = if installed { "" } else { "asserterror " };
        format!(
            r#"codeunit {cu} "{name}"
{{
    Subtype = Test;

    [Test]
    procedure {test}()
    begin
        {call}Codeunit.Run(130301);
    end;
}}
"#
        )
    };
    let root = fake_root(|corpus| {
        std::fs::write(
            corpus.join("tests/app/Absent.al"),
            probe(60901, "Harness Env Absent", "EnvAppIsAbsent", false),
        )
        .unwrap();
        let suite = corpus.join("tests/suite-app");
        std::fs::create_dir_all(&suite).unwrap();
        std::fs::write(
            suite.join("app.json"),
            r#"{"id":"0a1b2c3d-0000-4000-8000-0000000000e2","name":"Harness Env Suite","publisher":"Test","version":"1.0.0.0","dependencies":[],"idRanges":[{"from":60960,"to":60969}],"runtime":"16.0","target":"Cloud"}"#,
        )
        .unwrap();
        std::fs::write(
            suite.join("Present.al"),
            probe(60960, "Harness Env Present", "EnvAppIsLoaded", true),
        )
        .unwrap();
    });
    let at = |p: &str| root.path().join(p);
    let lib = at("env-lib");
    std::fs::create_dir_all(&lib).unwrap();
    std::fs::write(
        lib.join("app.json"),
        format!(
            r#"{{"id":"{ENV_APP_ID}","name":"Harness Env Lib","publisher":"Test","version":"1.0.0.0","dependencies":[],"idRanges":[{{"from":130301,"to":130301}}],"runtime":"16.0","target":"Cloud"}}"#
        ),
    )
    .unwrap();
    std::fs::write(
        lib.join("Lib.al"),
        "codeunit 130301 \"Reset State Before Test Run\" { trigger OnRun() begin end; }",
    )
    .unwrap();
    std::fs::write(
        at("oracle/27/junit.xml"),
        r#"<?xml version='1.0' encoding='utf-8'?>
<testsuites name="DEFAULT" tests="3" failures="0" errors="0" skipped="0">
  <testsuite name="Codeunit 60900" tests="2" failures="0" errors="0" skipped="0">
    <testcase classname="Codeunit 60900" name="Passes" time="0.001" />
    <testcase classname="Codeunit 60900" name="FailsOnPurpose" time="0.001" />
  </testsuite>
  <testsuite name="Codeunit 60901" tests="1" failures="0" errors="0" skipped="0">
    <testcase classname="Codeunit 60901" name="EnvAppIsAbsent" time="0.001" />
  </testsuite>
</testsuites>
"#,
    )
    .unwrap();
    std::fs::write(
        at("oracle/27/junit-env.xml"),
        r#"<?xml version='1.0' encoding='utf-8'?>
<testsuites name="DEFAULT" tests="1" failures="0" errors="0" skipped="0">
  <testsuite name="Codeunit 60960" tests="1" failures="0" errors="0" skipped="0">
    <testcase classname="Codeunit 60960" name="EnvAppIsLoaded" time="0.001" />
  </testsuite>
</testsuites>
"#,
    )
    .unwrap();
    let toml = std::fs::read_to_string(at("oracle/oracle.toml")).unwrap();
    std::fs::write(
        at("oracle/oracle.toml"),
        format!(
            r#"{toml}
[[version.suite]]
name = "env"
app = "fixture/tests/suite-app"
oracle = "27/junit-env.xml"
baseline = "metrics/27/not-passing-env.txt"
extra_apps = [{{ id = "{ENV_APP_ID}", name = "Harness Env Lib" }}]
"#
        ),
    )
    .unwrap();
    std::fs::create_dir_all(at("metrics/27")).unwrap();
    std::fs::write(at("metrics/27/not-passing.txt"), "60900 FailsOnPurpose\n").unwrap();
    std::fs::write(at("metrics/27/not-passing-env.txt"), "").unwrap();
    root
}

/// Runs the CLI on `root` with the Microsoft apps off (`--no-download`) and the env app as a package folder.
fn cli_env(root: &Path, args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_al-test-cli"))
        .arg("--root")
        .arg(root)
        .args(["--bc-version", "27", "--no-download", "--deps"])
        .arg(root.join("env-lib"))
        .args(args)
        .output()
        .unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into(),
        String::from_utf8_lossy(&out.stderr).into(),
    )
}

// A suite with `extra_apps` (the corpus testlibraries app: corpus .github/workflows/ci.yml, job test-testlibraries) runs
// in an environment of its own: the apps are installed for its tests and not for the main app's. One report and one
// JUnit file per app, as for any suite.
#[test]
fn a_suite_with_extra_apps_runs_in_its_own_environment() {
    let root = root_with_an_environment_suite();
    let dir = root.path().join("out");
    std::fs::create_dir_all(&dir).unwrap();
    let json = dir.join("pass-rate.json");
    let junit = dir.join("runtime-junit.xml");
    let (code, out, err) = cli_env(
        root.path(),
        &["--json", json.to_str().unwrap(), "--junit", junit.to_str().unwrap()],
    );
    assert_eq!(code, 0, "{out}\n{err}");
    let report: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&json).unwrap_or_else(|_| panic!("no report; stdout:\n{out}\nstderr:\n{err}")),
    )
    .unwrap();
    // The main app: Passes and EnvAppIsAbsent pass (the env app is not installed there), FailsOnPurpose fails.
    assert_eq!(report["overall"]["total"], 3, "{report}");
    assert_eq!(report["overall"]["pass"], 2, "{report}\n{err}");
    let suites = report["suites"].as_array().unwrap();
    assert_eq!(suites.len(), 1, "{report}");
    assert_eq!(suites[0]["name"], "env");
    assert_eq!(suites[0]["counts"]["total"], 1);
    assert_eq!(suites[0]["counts"]["pass"], 1, "{report}\n{err}");
    let main_junit = std::fs::read_to_string(dir.join("runtime-junit.xml")).unwrap();
    assert!(
        main_junit.contains("EnvAppIsAbsent") && !main_junit.contains("EnvAppIsLoaded"),
        "{main_junit}"
    );
    let env_junit = std::fs::read_to_string(dir.join("runtime-junit-env.xml")).unwrap();
    assert!(
        env_junit.contains("EnvAppIsLoaded") && !env_junit.contains("EnvAppIsAbsent"),
        "{env_junit}"
    );
    assert!(out.contains("BC 27.5 env: 1/1 pass"), "{out}");
    // The suite's list (ADR-065): a line of the suite's that passes is stale in its own file, and
    // --prune-not-passing deletes it there while the main app's list keeps its own line.
    let env_list = root.path().join("metrics/27/not-passing-env.txt");
    std::fs::write(&env_list, "60960 EnvAppIsLoaded\n").unwrap();
    let (code, out, err) = cli_env(root.path(), &[]);
    assert_eq!(code, 1, "{out}\n{err}");
    assert!(
        err.contains("BC 27.5 env: 1 stale line(s)") && err.contains("60960 EnvAppIsLoaded  [passes]"),
        "{err}"
    );
    let (code, out, err) = cli_env(root.path(), &["--prune-not-passing"]);
    assert_eq!(code, 0, "{out}\n{err}");
    let env_now = std::fs::read_to_string(&env_list).unwrap();
    assert!(!env_now.contains("60960"), "{env_now}");
    let main_now = std::fs::read_to_string(root.path().join("metrics/27/not-passing.txt")).unwrap();
    assert!(
        main_now.contains("60900 FailsOnPurpose") && !main_now.contains("60900 Passes"),
        "{main_now}"
    );
    // A selection of the suite's codeunit runs only its environment; --baseline-diff lists the suite.
    let (code, out, err) = cli_env(root.path(), &["--codeunit", "60960", "--baseline-diff"]);
    assert_eq!(code, 0, "{out}\n{err}");
    assert!(out.contains("BC 27.5 env: 1/1 pass"), "{out}");
    assert!(out.contains("BC 27.5 env"), "{out}");
    // A selection of the main app's codeunit does not run the suite.
    let (code, out, err) = cli_env(root.path(), &["--codeunit", "60901"]);
    assert_eq!(code, 0, "{out}\n{err}");
    assert!(out.contains("BC 27.5: 1/1 pass"), "{out}");
    // --explain finds a test of the suite.
    let (code, out, err) = cli_env(root.path(), &["--explain", "EnvAppIsLoaded"]);
    assert_eq!(code, 0, "{out}\n{err}");
    assert!(out.contains("env: test EnvAppIsLoaded"), "{out}");
    // A selection or a test name that no pass has is still an error.
    let (code, _, err) = cli_env(root.path(), &["--codeunit", "60999"]);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("matches no test"), "{err}");
    let (code, _, err) = cli_env(root.path(), &["--explain", "NoSuchTest"]);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("no test matches `NoSuchTest`"), "{err}");
}

#[test]
fn markdown_summary_is_written() {
    require_corpus();
    // The whole-corpus summary of BC 29 (the default version) ...
    let shared = all_versions();
    assert_eq!(shared.code, 0, "{}", shared.err);
    let md = std::fs::read_to_string(shared.file("pass-rate", "29", "md")).unwrap();
    assert!(md.starts_with("- BC 29.0: ") && md.contains(" of 27579 ("), "{md}");
    assert_eq!(md.lines().count(), 1, "one line per version: {md}");
    // ... and `--markdown` of a single-version run writes the file it is given.
    let dir = tempfile::tempdir().unwrap();
    let one = dir.path().join("s.md");
    let (code, _, err) = cli(&["--markdown", one.to_str().unwrap(), "--codeunit", SMALL_CU]);
    assert_eq!(code, 0, "{err}");
    assert!(std::fs::read_to_string(one).unwrap().contains(" passed, "));
}

#[test]
fn the_first_line_names_the_target() {
    require_corpus();
    // A run over every version starts with the oldest one's target and names the others.
    let shared = all_versions();
    assert_eq!(shared.code, 0, "{}", shared.err);
    assert_eq!(
        shared.out.lines().next(),
        Some("target: BC 27.5.46862.56249, runtime 16.1"),
        "{}",
        shared.out
    );
    for line in [
        "target: BC 28.5.54151.56267, runtime 17.1",
        "target: BC 29.0.54011.56262, runtime 18.0",
    ] {
        assert!(shared.out.lines().any(|l| l == line), "{line} in {}", shared.out);
    }
    // `--bc-version` picks one. A selection puts its own line first, so the target line follows it.
    let target_line = |v: &str| {
        let (code, out, err) = cli(&["--bc-version", v, "--codeunit", SMALL_CU]);
        assert_eq!(code, 0, "{err}");
        assert_eq!(
            out.lines().next().map(|l| l.starts_with("selection: ")),
            Some(true),
            "{out}"
        );
        out.lines().nth(1).unwrap_or_default().to_string()
    };
    assert_eq!(target_line("27"), "target: BC 27.5.46862.56249, runtime 16.1");
    assert_eq!(target_line("28"), "target: BC 28.5.54151.56267, runtime 17.1");
}

#[test]
fn an_unknown_bc_version_exits_2_and_lists_the_known_ones() {
    require_corpus();
    let (code, _, err) = cli(&["--bc-version", "99"]);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("no oracle for BC 99") && err.contains("27.5"), "{err}");
}

#[test]
fn bc_26_is_not_supported() {
    require_corpus();
    let (code, _, err) = cli(&["--bc-version", "26"]);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("27 and later only"), "{err}");
}

#[test]
fn every_bc_major_has_an_oracle_with_the_same_cases() {
    require_corpus();
    // 28 and 29 come from the hybrid runner: its OnRun pseudo-cases must be skipped, or the
    // run stops with exit 2 (ADR-015 1.6).
    // All three are recorded for the pinned corpus commit.
    for (v, total) in [("27", 27293), ("28", 27436), ("29", 27579)] {
        let r = all_versions().json(v);
        assert_eq!(r["overall"]["total"], total, "BC {v}");
        assert_eq!(r["unmapped"].as_array().unwrap().len(), 0, "BC {v}");
    }
}

#[test]
fn the_default_is_the_newest_version_with_an_oracle() {
    require_corpus();
    let (code, out, err) = cli(&["--codeunit", SMALL_CU]);
    assert_eq!(code, 0, "{err}");
    // The selection's own line comes first.
    assert_eq!(
        out.lines().nth(1),
        Some("target: BC 29.0.54011.56262, runtime 18.0"),
        "{out}"
    );
}

#[test]
fn all_versions_runs_each_oracle_writes_per_version_files_and_compares() {
    let AllVersions { dir, code, out, err } = all_versions();
    assert_eq!(*code, 0, "{err}");
    for v in ["27", "28", "29"] {
        let f = dir.join(format!("pass-rate-{v}.json"));
        assert!(f.exists(), "no report for BC {v}; stdout:\n{out}");
    }
    assert!(
        !dir.join("pass-rate.json").exists(),
        "all-versions writes only per-version files"
    );
    assert!(out.contains("Version comparison"), "{out}");
    // Every oracle is recorded for the pin. Sixty-three cases existed on some majors only before bump 26, which made it 105 (corpus bump 11 added sixteen guarded probes-needed tests, bump 12 five more, bump 14 four more, bump 17 seven more, bump 20 two more, bump 22 eleven more, bump 23 one more, bump 24 one more: Guid_Plain of the FieldRef TestField group is the new case; bump 27 eighteen more: the JsonArray and JsonObject SelectTokens tests of json/json/SelectTokensList.al, guarded by `#if BC28PLUS`, which make it 123): FeatureKey_Columns_AndRows_OfThe27Platform
    // (27, guarded by `#if not BC28PLUS`) and thirty-one guarded by `#if BC28PLUS` or `#if BC29PLUS` (corpus bump 10: the
    // BigInteger FlowField and query aggregate tests, the Base64 and conversion error texts that differ on 27.0 and 27.3,
    // and the interface collection codeunit).
    // Corpus bump 28 adds 31 more (the BC28PLUS and BC29PLUS guards that al-version/src/gates.rs cites for bump 28), which make it 154.
    // Corpus bump 29 adds 10 more (the BC28PLUS and BC29PLUS guards that al-version/src/gates.rs cites for bump 29), which make it 164.
    // Corpus bump 30 adds 22 more (the BC28PLUS and BC29PLUS guards that al-version/src/gates.rs cites for bump 30), which make it 186.
    // Corpus bumps 31 and 32 add 12 more: the PYD_IsDirty tests of codeunit 77504 in record/ACRecordIsDirty.al, whose
    // `#if BC29PLUS` guard the gate RecordIsDirtyFrom29 cites (Record.IsDirty exists from BC 29), which make it 198.
    // Corpus bumps 33 and 34 make it 195. Six leave: the BC 27 only tests of 85660 and 85661 in
    // http/client/LtDaSendFailure.al are now compiled under `#if BC28PLUS and LT_HELD_BCLINUX_165`, which the
    // Linux CI does not define, so no major has them. Three come: PW_RaisedEvents_OtherFormats and PW_RaisedEvents_RunRunModal
    // of 25077 (events/platform-triggers/PeVTests.al:750, the RDLC holds lifted to `#if BC28PLUS`) and 85312
    // CodeunitRun_ByQualifiedName (session-system/isolated-storage/CuWriteTxTests.al, hold LT_HELD_BCLINUX_159 lifted inside its BC28PLUS
    // guard). Corpus bump 42 makes it 196: PW_RaisedEvents_Execute of 25077 (PeVTests.al:725, hold LT_HELD_BCLINUX_158 lifted
    // inside the BC28PLUS guard) now runs on BC 28 and 29. Corpus bump 43 makes it 202: the six tests of 85660 and 85661 in
    // LtDaSendFailure.al come back under `#if BC28PLUS` (hold LT_HELD_BCLINUX_165 lifted), on BC 28 and 29 only.
    // The multi-app corpus bump (corpus PR 710) makes it 203: 78603 QV_Header_CompanyLogo_PictureVariants
    // (reports/dataset/AhDatasetHeader.al:220, inside the file's `#if BC29PLUS`) runs on BC 29 only.
    // Corpus bump 55 makes it 209: the six RB_Header_AddressFormat_* tests of 25077 (AhDatasetHeader.al:309-430), BC 29 only.
    // Corpus bump to master 2db2dbd5 makes it 211: the two tests of 99830 in collections/SsCallerModulesTestRunner.al run on
    // BC 27 only (`#if not BC28PLUS`, the Test Runner app path of the CI; gate TestRunnerHubRequiredIsolationFrom28).
    // Corpus bump to master 28de3f39 makes it 214: the three tests of 90645 in metadata/WcFieldRowCopy.al run on BC 29
    // only (`#if BC29PLUS`; gate PlatformMetadataColumnsFrom29).
    // Corpus bump to master 09855762 makes it 224: the nine Y8_IsDirty_* tests of 91415 in record/Y8RecordIsDirtyShapes.al run
    // on BC 29 only (`#if BC29PLUS`; gate RecordIsDirtyFrom29), and 90775 WD_BareNameExt_ValueReadError in
    // testpage/WdBareQuotedField.al on BC 28 and 29 only (`#if BC28PLUS`; gate ExtControlBareQuotedSourceNotFoundFrom28).
    // Corpus bump to master 92031e20 makes it 238: five tests of 25077 (Y17_TenantReportLayoutCfg_InsertModifyDelete, _Shapes,
    // Y40_TenantReportLayoutOverride_InsertModifyDelete, _SeenByReportLayoutList, _TypedRecord) and nine BigInteger and Enum
    // conversions of the Y31 conversion matrix (types/operators-expressions/, `#if BC29PLUS`; gate
    // BigIntegerToEnumVariableFrom29) run on BC 29 only. Corpus bump to master 7f7cdc2a makes it 239:
    // Y47_SelectTokens_ArrayFilter (json/DbJPathLeads.al, `#if BC28PLUS`; gate JsonSelectTokensListFrom28)
    // runs on BC 28 and 29 only. Corpus bump to master 4b6013d0 makes it 244: the new BC28PLUS and BC29PLUS guards of writers
    // Y64, Y65, Y68 and Y70 that the probed-values gates cite. Corpus bump to master 34ca226f makes it 249: the BC28PLUS and
    // BC29PLUS guards of writers Y71 and Y66 (gates FieldRefTextNumberEvaluateErrorFrom28, McpConfigurationFrom29,
    // PlatformMetadataColumnsFrom29).
    // Corpus bump to master 941cd813 makes it 283: the bump to 9cce33bd (writers Y76 to Y81, which did not update this count)
    // and the Y79 fix and Y85 pins after it add BC28PLUS and BC29PLUS guards that the probed-values gates cite.
    // Corpus bump to master c8bb6694 makes it 298: the Y90 BC28PLUS guards (gate PermissionShapeProbesFrom28) and the other new cases.
    // Corpus bump to master eccb1330 makes it 300: the two tests of codeunits/interfaces/R6924dThisInDefaultBody.al (`#if BC29PLUS`;
    // gate DefaultInterfaceMethodsFrom29) run on BC 29 only.
    // Corpus bump to master f2d5ea33 makes it 302: Card_LeftOpenAtTestEnd_ClosePageConfirmUnhandled_Write and _Read of
    // pages/close-errors/CemTestPageCloseTests.al (`#if BC28PLUS`; gate ProbedValuesDifferFrom28) run on BC 28 and 29 only.
    assert!(out.contains("oracle: 302 case(s) differ between versions"), "{out}");
    // The known BC 29 differences are gated (ADR-015 3.4), so the runtime passes the same cases on every version, among the
    // cases every oracle has. One difference is known and open (corpus bump 10, backlog): the runtime passes
    // 71500 Get_Uses_Primary_Key_Not_Current_Key on BC 27 only. A new difference would be listed here.
    // Corpus bump 28 added four, all new cases, none in a baseline; one remains: 81959
    // BM_CalcFormula_IntegerFilterFromBigInteger is refused on BC 29 (a BigInteger above the Integer range in an Integer filter).
    // 79926 Byte10 and Byte200 pass on every version since branch lane-bump28-failures, 72786
    // BN_With_ImplicitWithInTableNoCodeunit since branch lane-record-isdirty (Record.IsDirty).
    // Corpus bump 29 adds one, a new case: 84275 Expr_BigIntegerLiteral_Enabled (testpage/CcPageLeads.al:801) passes
    // from BC 28; on BC 27 real BC fails to compile the dynamic control expression `0L <> Big`, and the runtime refuses the
    // page there (gate BigIntegerLiteralInControlExpressionFrom28), so the case is not run on 27.
    // Corpus bump 30 adds three new cases that pass on BC 27 only: 79926 PX_ByteValues and PX_ByteValues_High
    // (record/testfield/AsTestFieldTexts.al:153: from BC 28 the TestField of a Byte has an escaped form that is not pinned,
    // so they are refused there), and 85180 List_OfRecordRef (collections/CsCollections.al:594: from BC 28 the
    // test reads more of a global that an earlier refused test of the codeunit could change, so it is not run there).
    // Corpus bump 64 adds 13, all of codeunits/run-onrun/RqIsolationModes.al, which pass on BC 27 only: there
    // runner 130450 runs every codeunit and StartSession fails as pinned. On BC 28 and 29 the hub runs 88510 under Disabled,
    // where BC starts the session (:573-600) and the runtime does not run a second session, so RQ_D03 is refused and what
    // the Disabled codeunits leave is not known: RQ_D04, RQ_D11 and the 130450 codeunits after them (88512-88515) are not run.
    // Branch refuse-types makes it 18: 81959 BM_CalcFormula_IntegerFilterFromBigInteger now passes on BC 29 too (an Integer
    // parameter takes a BigInteger argument by value).
    // Branch issue-5607 makes it 15: StartSession under Disabled runs (al-runner issue 5609), so RQ_D03, RQ_D04 and RQ_D11
    // pass on every version. 88512-88515 stay not run on BC 28 and 29: they have handlers, and whether BC runs them before
    // or after the Disabled codeunit 88510 that wrote their table is not pinned.
    // Branch addins makes it 19: a control add-in call raises its not-instantiated error from BC 28 (gate
    // ControlAddInNotInstantiatedErrorFrom28) and refuses on BC 27, so 77540 AC_ControlAddIn_ProcedureCallFromFieldTrigger,
    // 79590 AddinProcedureFromAction, 67122 Addin_PageOpened_OnOpenPageCallsProcedure and Addin_PageVariableNeverOpened_Calls
    // pass on BC 28 and 29 only.
    // Corpus bump to 21a41286 makes it 27: eight new TZ_* cases of 25077 (control add-in calls from page triggers and
    // actions) pass on BC 28 and 29 only, behind the same gate ControlAddInNotInstantiatedErrorFrom28.
    // Branch fix-left makes it 17: the numbers a Required-isolation codeunit takes are carried over, so the ten
    // remaining RqIsolationModes.al cases (88512-88515 and the ones after them) now run and pass on BC 28 and 29 too.
    // Corpus bump to master 4b6013d0 makes it 19: 93335 Y64_UserNameFields_Exist passes on BC 29 only (RecordRef.FieldExist of
    // 2000000005 is refused before BC 29), and 70303 Y64_ModifyAll_PartialRecord_UnloadedField on BC 27 only (on 28 and 29 an
    // earlier test of the codeunit is refused: CalcFields on a Media field).
    // Corpus bump to master 34ca226f keeps it at 19 once FieldRef.Name of SystemCreatedByUserName is served (Y66).
    // Train 1102q makes it 18: 93335 Y64_UserNameFields_Exist (RecordRef.FieldExist of 2000000005 to 2000000008) now
    // passes on BC 27 and 28 too, so it no longer differs between versions.
    // Corpus bump to master b2a88adc makes it 32: the 14 Y89_Old_Property_* and Y89_Old_Setter_* cases of 81070
    // (reports/runtime/BdRegionTests.al:1521) pass on BC 29 only; on 27 and 28 the format with a FormatRegion
    // other than the default is not pinned and refused (the same refusal as BdProperty_*_DatasetAndFormat).
    // Train 1104e makes it 33: 95042 StoredSize_SmallText_Result (record/flowfields/BlobStoredSizeSign.al:224, al-runner
    // issue 6896) passes on BC 29 only; on 27 and 28 the stored size of 1 byte in a compressed field is not pinned (gate
    // ProbedValuesDifferFrom29) and the case is refused.
    // Corpus bump to master 92da8280 makes it 35: 95439 OtherRecordOption_OneRow_Values and
    // OtherRecordOption_TwoRows_AfterNext_Values (pages/controls/OtherRecordOptionMembers.al:146, :194, al-runner issue 7005)
    // pass on BC 29 only, where the control shows the OptionCaption (gate OtherRecordOptionCaptionFrom29); before BC 29 a
    // control bound to another record's Option field is refused.
    // Train 1105f makes it 36: the runtime pull request for issue 7005 shows the member names on BC 28 (confirmed on Windows)
    // and a blank control on a list without rows, so the three 95439 OtherRecordOption_* cases pass on BC 28 and 29; BC 27 is a
    // Linux suspect and stays refused.
    // Branch fix-6975-record-validate-variant makes it 37: 60670 TemporalSetValue_ValidateRefusesTextCarriedInAVariant
    // (testpage/TestPageTemporalSetValue_Tests.al:303) passes on BC 27 only: Record.Validate with a Variant now converts as
    // FieldRef.Validate does, whose error on 27 names the type for any text; from 28 a text into a Date field is not pinned.
    // This branch makes it 38: 67605 N582_Evaluate_FieldRef_Guid_ResultUsedAndFormat9 (recordref-fieldref/N582EvalFieldRefUnused.al,
    // al-runner issue 6924) passes on BC 28 and 29; on 27 the last error stays set when Evaluate into a Guid FieldRef
    // answers false (the `#else` arms), which the runtime does not do, so it stays listed there.
    // Corpus bump to master 3f82e513 makes it 39: 69340 EnvironmentInformation_PreviewRings (types/PVEnvironment.al:49-68)
    // passes on BC 27 only in this run without Microsoft's apps: there its body is guarded out (gate
    // EnvironmentPreviewRingsFrom28); on 28 and 29 it calls Environment Information of the System Application.
    // Branch rt-notrun-7 (FormatRegion after Language) leaves it at 39 in this run: CI on the merge train reported 39, the
    // Y89_Old_*/BdProperty_*/BdSetter_* cases of 81070 already counted the same on every version here.
    // Merge train l (profiler memory, ADR-066 amendment 12, SetFilter quote, login chain, shared baseline 3, AccessByPermission
    // actionref) makes it 49 in the local run of the train: ten more cases pass on some BC versions only. The count was
    // not measured on main alone; CI on the train is the check.
    // Exact runner frames by routing (CallstackRunnerFrames.al, corpus pin c4efa9eb) add the eight
    // CallstackRunnerFrames_Plain_* cases of 69950: a codeunit without handlers has no runner frames from BC 28 on.
    assert!(out.contains("runtime: 57 case(s) differ between versions"), "{out}");
    assert!(out.contains("71500 Get_Uses_Primary_Key_Not_Current_Key: 27"), "{out}");
}

#[test]
fn all_versions_cannot_be_combined_with_one_version() {
    require_corpus();
    let (code, _, err) = cli(&["--all-versions", "--bc-version", "28"]);
    assert_eq!(code, 2, "{err}");
}

#[test]
fn require_dotnet_turns_an_unusable_dotnet_into_a_setup_error() {
    require_corpus();
    // ADR-014 section 11: CI passes --require-dotnet so that "no .NET" is never mistaken for a pass.
    let (code, _, err) = cli(&[
        "--require-dotnet",
        "--dotnet-sidecar",
        "/nonexistent/AlDotNetSidecar.dll",
    ]);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("--require-dotnet") && err.contains("DotNet"), "{err}");
    // Without the flag the same machine still runs the corpus (a codeunit of it: the flag decides
    // whether the run starts, not what it selects).
    let (code, _, err) = cli(&[
        "--dotnet-sidecar",
        "/nonexistent/AlDotNetSidecar.dll",
        "--codeunit",
        SMALL_CU,
    ]);
    assert_eq!(code, 0, "{err}");
}

// BC 28 holds the main app and three more oracles (oracle/oracle.toml `[[version.suite]]`): the
// OnPrem app and the isolation probe under its two runners. Their cases stay out of `overall`.
#[test]
fn the_other_test_apps_are_reported_apart_from_the_main_one() {
    let r = all_versions().json("28");
    assert_eq!(r["overall"]["total"], 27436);
    let suites = r["suites"].as_array().unwrap();
    let get = |n: &str| {
        suites
            .iter()
            .find(|s| s["name"] == n)
            .unwrap_or_else(|| panic!("no suite {n}"))
    };
    assert_eq!(get("onprem")["counts"]["total"], 487);
    // Real BC fails T8 and T10 (a deferred duplicate-key Insert at the end of the test) under both runners,
    // and the runtime fails them too. The nightly of 2026-11-09 adds the DJ and SD isolation codeunits
    // (61306-61318), many of which fail on BC on purpose (readers of rows a runner keeps or rolls back).
    let function = get("isolation-function");
    assert_eq!(function["counts"]["total"], 29);
    assert_eq!(function["counts"]["pass"], 29);
    assert_eq!(function["bc_failed"], 20);
    assert_eq!(function["bc_failed_reproduced"], 20);
    // The control: real BC also fails T5 and T11 under Codeunit isolation. Whether the runtime fails every case BC
    // fails here is asserted in `the_isolation_control_fails_what_bc_fails`.
    let control = get("isolation-codeunit");
    assert_eq!(control["counts"]["pass"], 31);
    assert_eq!(control["bc_failed"], 18);
}

// Under TestIsolation = Codeunit real BC fails 18 cases of the isolation probe, and the runtime must fail them all
// (oracle/28/xunit-isolation-codeunit.xml, nightly run 42604559766). Among them codeunit 61309 "LT DJ Isol After
// Rollback", S8_NumberSequenceStillExists and S9_NumberSequenceNotReset: the failed S4 takes back the
// NumberSequence.Insert of S3 (al-vm `sequence_start.rs`).
#[test]
fn the_isolation_control_fails_what_bc_fails() {
    let r = all_versions().json("28");
    let control = r["suites"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["name"] == "isolation-codeunit")
        .expect("no suite isolation-codeunit");
    assert_eq!(control["bc_failed"], 18);
    assert_eq!(control["bc_failed_reproduced"], 18);
}

// BC 27 and 29 have no isolation oracle (the probe only ran in the BC 28.4 nightly). The testlibraries suite runs in
// a pass of its own (`extra_apps`), and its report joins the version's, as does the TryWrites suite (its own pass, server
// settings).
#[test]
fn a_version_without_an_isolation_oracle_lists_only_the_onprem_testlibraries_and_trywrite_apps() {
    let r = all_versions().json("27");
    let names: Vec<_> = r["suites"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        vec!["onprem", "onprem-dotnet", "testlibraries", "trywrite-allowed"]
    );
}

// `--baseline FILE` is the main app's list alone. The extra apps keep their own files in
// `oracle.toml`, which may need the dependency apps this `--no-deps` run does not load.
#[test]
fn an_explicit_baseline_does_not_ratchet_the_extra_apps() {
    require_corpus();
    // Not a regression: onprem's own list assumes the dependency apps.
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("not-passing.txt");
    std::fs::write(&base, "").unwrap();
    // Codeunit 61200 is the OnPrem app's: its cases are refused without the dependency apps.
    let (code, _, err) = cli(&[
        "--bc-version",
        "27",
        "--baseline",
        base.to_str().unwrap(),
        "--codeunit",
        ONPREM_CU,
    ]);
    assert_eq!(code, 0, "{err}");
}

#[test]
fn a_suite_baseline_ratchets_that_extra_app_and_no_ratchet_ignores_it() {
    require_corpus();
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("not-passing.txt");
    std::fs::write(&base, "").unwrap();
    // An empty list for the OnPrem app, whose cases cannot pass without their platform tables and dependency apps.
    let suite = dir.path().join("onprem.txt");
    std::fs::write(&suite, "").unwrap();
    let args = |extra: &[&str]| {
        let mut a = vec![
            "--bc-version",
            "27",
            "--baseline",
            base.to_str().unwrap(),
            "--codeunit",
            ONPREM_CU,
        ];
        a.extend_from_slice(&["--suite-baseline"]);
        let named = format!("onprem={}", suite.display());
        let mut v: Vec<String> = a.iter().map(|s| s.to_string()).collect();
        v.push(named);
        v.extend(extra.iter().map(|s| s.to_string()));
        v
    };
    let run = |a: Vec<String>| cli(&a.iter().map(String::as_str).collect::<Vec<_>>());
    let (code, _, err) = run(args(&[]));
    assert_eq!(code, 1, "{err}");
    assert!(err.contains("BC 27.5 onprem") && err.contains("regression(s)"), "{err}");
    // --no-ratchet reports only, for every app.
    let mut a = args(&["--no-ratchet"]);
    a.retain(|x| x != base.to_str().unwrap() && x != "--baseline");
    let (code, _, err) = run(a);
    assert_eq!(code, 0, "{err}");
    let (_, _, err) = cli(&["--bc-version", "27", "--suite-baseline", "nope=x"]);
    assert!(err.contains("no suite named `nope`"), "{err}");
}

/// The JUnit files of a run (the main app's and the extra suites'), with the per-test times
/// removed: everything else (outcome, reason, flagged assumptions) must be the same between two
/// runs of one build on one corpus.
fn outcomes_of(dir: &Path, stem: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        let name = p.file_name().unwrap().to_string_lossy().into_owned();
        if !(name.starts_with(stem) && name.ends_with(".xml")) {
            continue;
        }
        let xml = std::fs::read_to_string(&p).unwrap();
        let mut kept = String::with_capacity(xml.len());
        let mut rest = xml.as_str();
        // Drops ` time="..."` and ` timestamp="..."` attributes.
        while let Some(i) = rest.find([' ']) {
            kept.push_str(&rest[..i]);
            rest = &rest[i..];
            let attr = [" time=\"", " timestamp=\""].into_iter().find(|a| rest.starts_with(a));
            match attr {
                Some(a) => {
                    let end = rest[a.len()..].find('"').unwrap();
                    rest = &rest[a.len() + end + 1..];
                }
                None => {
                    kept.push(' ');
                    rest = &rest[1..];
                }
            }
        }
        kept.push_str(rest);
        out.push((name.replacen(stem, "", 1), kept));
    }
    out.sort();
    out
}

/// One build on one corpus gives every case one outcome, whatever the thread counts and whatever
/// HOME holds. Two causes of different results were seen on 2026-10-20: 99100
/// `AL_JsonTextWriter_SimpleObject` (OnPrem DotNet) passed only where `~/.bcartifacts.cache`
/// held a BC service tier with Newtonsoft.Json, and was refused everywhere else (CI). The second
/// run swaps both: one worker thread instead of four, and a HOME with that folder (the machine's
/// own when it has one) instead of an empty one. BC 27, because the OnPrem DotNet cases and the
/// local service tiers of BC 27 are where the HOME difference showed.
#[test]
#[cfg(unix)]
fn outcomes_do_not_depend_on_threads_or_home() {
    require_corpus();
    let work = tempfile::tempdir().unwrap();
    let empty_home = work.path().join("home-empty");
    let artifacts_home = work.path().join("home-artifacts");
    std::fs::create_dir_all(&empty_home).unwrap();
    std::fs::create_dir_all(&artifacts_home).unwrap();
    let real = std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".bcartifacts.cache"));
    match real.filter(|r| r.join("sandbox").is_dir()) {
        Some(r) => std::os::unix::fs::symlink(r, artifacts_home.join(".bcartifacts.cache")).unwrap(),
        // A machine without local artifacts (CI): a service-tier folder of the shape the local
        // artifacts have, so that a run which looks there finds one.
        None => std::fs::create_dir_all(artifacts_home.join(
            ".bcartifacts.cache/sandbox/27.5.0.0/platform/ServiceTier/PFiles64/Microsoft Dynamics NAV/270/Service",
        ))
        .unwrap(),
    }
    let run = |home: &Path, threads: &str, stem: &str| {
        let junit = work.path().join(format!("{stem}.xml"));
        let out = Command::new(env!("CARGO_BIN_EXE_al-test-cli"))
            .arg("--root")
            .arg(root())
            .args(["--no-deps", "--no-ratchet", "--bc-version", "27", "--junit"])
            .arg(&junit)
            .env("HOME", home)
            .env_remove("XDG_CACHE_HOME")
            .env("AL_TEST_THREADS", threads)
            .env("AL_LOWER_THREADS", threads)
            .env("AL_CHECK_THREADS", threads)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(0), "{}", String::from_utf8_lossy(&out.stderr));
        outcomes_of(work.path(), stem)
    };
    let a = run(&empty_home, "4", "run-a");
    let b = run(&artifacts_home, "1", "run-b");
    assert_eq!(
        a.len(),
        5,
        "the main app, the two OnPrem suites, the testlibraries suite and the TryWrites suite: {:?}",
        a.iter().map(|f| &f.0).collect::<Vec<_>>()
    );
    assert_eq!(
        a.iter().map(|f| &f.0).collect::<Vec<_>>(),
        b.iter().map(|f| &f.0).collect::<Vec<_>>()
    );
    for ((file, xa), (_, xb)) in a.iter().zip(&b) {
        if xa != xb {
            // The first differing test case, else the first differing line (a suite's counts).
            let pairs = || xa.lines().zip(xb.lines()).filter(|(l, r)| l != r);
            let first = pairs()
                .find(|(l, _)| l.contains("<testcase"))
                .or_else(|| pairs().next());
            panic!(
                "run{file}: the two runs differ, first at\n  4 threads, empty HOME:   {:?}\n  1 thread, artifacts HOME: {:?}",
                first.map(|f| f.0),
                first.map(|f| f.1)
            );
        }
    }
}
