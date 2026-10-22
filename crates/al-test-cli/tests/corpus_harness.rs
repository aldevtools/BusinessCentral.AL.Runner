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
/// them). No ratchet unless the test brings a baseline: the real one lists cases that need
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
    assert_eq!(r["overall"]["total"], 17708);
    assert_eq!(r["unmapped"].as_array().unwrap().len(), 0, "{}", r["unmapped"]);
    assert_eq!(r["areas"]["json"]["total"], 464);
    assert_eq!(r["areas"]["record"]["total"], 2261);
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
            "381a4ee3d9523f5e75d0a9f205229ce00e981959",
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

#[test]
fn a_baseline_case_that_does_not_pass_fails_the_run() {
    require_corpus();
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("passing.txt");
    std::fs::write(&base, "60002 NoSuchProcedure_UsedByTheRegressionTest\n").unwrap();
    // The ratchet checks the cases of the selection, so selecting the codeunit is enough.
    let (code, _, err) = cli(&["--baseline", base.to_str().unwrap(), "--codeunit", "60002"]);
    assert_eq!(code, 1, "{err}");
    assert!(err.contains("60002 NoSuchProcedure_UsedByTheRegressionTest"), "{err}");
}

/// A repository of its own: a git "corpus" with one test app, and an oracle for BC 27 that says real
/// BC passes both tests of the app. One test passes in the runtime and one fails on purpose, so
/// the run has a failing case whatever the real corpus holds today. The root has the layout the
/// CLI reads (`oracle/oracle.toml`, a git repository for the corpus pin, an app folder).
fn root_with_a_failing_test() -> tempfile::TempDir {
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
baseline = "metrics/27/passing.txt"
"#
        ),
    )
    .unwrap();
    dir
}

#[test]
fn a_failing_case_outside_the_baseline_is_reported_but_does_not_fail_the_run() {
    // ADR-007: only a baseline case that stops passing exits 1. The probe tests the corpus adds
    // for behaviour the runtime does not have yet are failures in the report, not in the gate.
    // The run needs a failing case, and the real corpus may have none (the runtime keeps getting
    // better), so it runs on a fixture whose second test fails on purpose.
    let root = root_with_a_failing_test();
    let run = |baseline: &str| {
        let base = root.path().join("passing.txt");
        std::fs::write(&base, baseline).unwrap();
        let out = root.path().join("r.json");
        let o = Command::new(env!("CARGO_BIN_EXE_al-test-cli"))
            .arg("--root")
            .arg(root.path())
            .args(["--no-deps", "--bc-version", "27", "--baseline"])
            .arg(&base)
            .arg("--json")
            .arg(&out)
            .output()
            .unwrap();
        let err = String::from_utf8_lossy(&o.stderr).to_string();
        let report: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&out).unwrap_or_else(|_| panic!("no report written; stderr:\n{err}")),
        )
        .unwrap();
        (o.status.code().unwrap_or(-1), report, err)
    };
    // The passing case is in the baseline, the failing one is not: reported, exit 0.
    let (code, r, err) = run("60900 Passes\n");
    assert_eq!(code, 0, "{err}");
    assert_eq!(r["overall"]["total"], 2, "{}", r["overall"]);
    assert_eq!(r["overall"]["pass"], 1, "{}", r["overall"]);
    assert_eq!(r["overall"]["fail"], 1, "{}", r["overall"]);
    // The control: the same failure inside the baseline is a regression.
    let (code, _, err) = run("60900 Passes\n60900 FailsOnPurpose\n");
    assert_eq!(code, 1, "{err}");
    assert!(err.contains("60900 FailsOnPurpose"), "{err}");
}

#[test]
fn markdown_summary_is_written() {
    require_corpus();
    // The whole-corpus summary of BC 29 (the default version) ...
    let shared = all_versions();
    assert_eq!(shared.code, 0, "{}", shared.err);
    let md = std::fs::read_to_string(shared.file("pass-rate", "29", "md")).unwrap();
    assert!(md.starts_with("- BC 29.0: ") && md.contains(" of 17708 ("), "{md}");
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
    for (v, total) in [("27", 17518), ("28", 17614), ("29", 17708)] {
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
    // Every oracle is recorded for the pin. Sixty-three cases existed on some majors only before bump 26, which made it 105 (corpus bump 11 added sixteen guarded probes-needed tests, bump 12 five more, bump 14 four more, bump 17 seven more, bump 20 two more, bump 22 eleven more, bump 23 one more, bump 24 one more: Guid_Plain of the FieldRef TestField group is the new case; bump 27 eighteen more: the JsonArray and JsonObject SelectTokens tests of json/lt-json-selecttokens/SelectTokensList.al, guarded by `#if BC28PLUS`, which make it 123): FeatureKey_Columns_AndRows_OfThe27Platform
    // (27, guarded by `#if not BC28PLUS`) and thirty-one guarded by `#if BC28PLUS` or `#if BC29PLUS` (corpus bump 10: the
    // BigInteger FlowField and query aggregate tests, the Base64 and conversion error texts that differ on 27.0 and 27.3,
    // and the interface collection codeunit).
    // Corpus bump 28 adds 31 more (the BC28PLUS and BC29PLUS guards that al-version/src/gates.rs cites for bump 28), which make it 154.
    // Corpus bump 29 adds 10 more (the BC28PLUS and BC29PLUS guards that al-version/src/gates.rs cites for bump 29), which make it 164.
    // Corpus bump 30 adds 22 more (the BC28PLUS and BC29PLUS guards that al-version/src/gates.rs cites for bump 30), which make it 186.
    // Corpus bumps 31 and 32 add 12 more: the PYD_IsDirty tests of codeunit 77504 in lt-ac/ACRecordIsDirty.al, whose
    // `#if BC29PLUS` guard the gate RecordIsDirtyFrom29 cites (Record.IsDirty exists from BC 29), which make it 198.
    // Corpus bumps 33 and 34 make it 195. Six leave: the BC 27 only tests of 85660 and 85661 in
    // network/lt-http-send-failure/LtDaSendFailure.al are now compiled under `#if BC28PLUS and LT_HELD_BCLINUX_165`, which the
    // Linux CI does not define, so no major has them. Three come: PW_RaisedEvents_OtherFormats and PW_RaisedEvents_RunRunModal
    // of 17708 (events/lt-platform-events-v/PeVTests.al:750, the RDLC holds lifted to `#if BC28PLUS`) and 85312
    // CodeunitRun_ByQualifiedName (misc/lt-cu-odd-leads/CuWriteTxTests.al, hold LT_HELD_BCLINUX_159 lifted inside its BC28PLUS
    // guard). Corpus bump 42 makes it 196: PW_RaisedEvents_Execute of 17708 (PeVTests.al:725, hold LT_HELD_BCLINUX_158 lifted
    // inside the BC28PLUS guard) now runs on BC 28 and 29. Corpus bump 43 makes it 202: the six tests of 85660 and 85661 in
    // LtDaSendFailure.al come back under `#if BC28PLUS` (hold LT_HELD_BCLINUX_165 lifted), on BC 28 and 29 only.
    assert!(out.contains("oracle: 202 case(s) differ between versions"), "{out}");
    // The known BC 29 differences are gated (ADR-015 3.4), so the runtime passes the same cases on every version, among the
    // cases every oracle has. One difference is known and open (corpus bump 10, backlog): the runtime passes
    // 71500 Get_Uses_Primary_Key_Not_Current_Key on BC 27 only. A new difference would be listed here.
    // Corpus bump 28 added four, all new cases, none in a baseline; one remains: 81959
    // BM_CalcFormula_IntegerFilterFromBigInteger is refused on BC 29 (a BigInteger above the Integer range in an Integer filter).
    // 79926 Byte10 and Byte200 pass on every version since branch lane-bump28-failures, 72786
    // BN_With_ImplicitWithInTableNoCodeunit since branch lane-record-isdirty (Record.IsDirty).
    // Corpus bump 29 adds one, a new case: 84275 Expr_BigIntegerLiteral_Enabled (pages/lt-cc-page-leads/CcPageLeads.al:801) passes
    // from BC 28; on BC 27 real BC fails to compile the dynamic control expression `0L <> Big`, and the runtime refuses the
    // page there (gate BigIntegerLiteralInControlExpressionFrom28), so the case is not run on 27.
    // Corpus bump 30 adds three new cases that pass on BC 27 only: 79926 PX_ByteValues and PX_ByteValues_High
    // (record/lt-as-records/AsTestFieldTexts.al:153: from BC 28 the TestField of a Byte has an escaped form that is not pinned,
    // so they are refused there), and 85180 List_OfRecordRef (collections/lt-cs-odd-leads/CsCollections.al:594: from BC 28 the
    // test reads more of a global that an earlier refused test of the codeunit could change, so it is not run there).
    assert!(out.contains("runtime: 6 case(s) differ between versions"), "{out}");
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
    assert_eq!(r["overall"]["total"], 17614);
    let suites = r["suites"].as_array().unwrap();
    let get = |n: &str| {
        suites
            .iter()
            .find(|s| s["name"] == n)
            .unwrap_or_else(|| panic!("no suite {n}"))
    };
    assert_eq!(get("onprem")["counts"]["total"], 349);
    assert_eq!(get("isolation-function")["counts"]["total"], 7);
    assert_eq!(get("isolation-function")["counts"]["pass"], 7);
    // The control: real BC fails T5 under Codeunit isolation, and the runtime fails it too.
    let control = get("isolation-codeunit");
    assert_eq!(control["counts"]["pass"], 6);
    assert_eq!(control["bc_failed"], 1);
    assert_eq!(control["bc_failed_reproduced"], 1);
}

// BC 27 and 29 have no isolation oracle (the probe only ran in the BC 28.4 nightly).
#[test]
fn a_version_without_an_isolation_oracle_lists_only_the_onprem_apps() {
    let r = all_versions().json("27");
    let names: Vec<_> = r["suites"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, vec!["onprem", "onprem-dotnet"]);
}

// `--baseline FILE` is the main app's ratchet alone. The extra apps keep their own files in
// `oracle.toml`, which may need the dependency apps this `--no-deps` run does not load.
#[test]
fn an_explicit_baseline_does_not_ratchet_the_extra_apps() {
    require_corpus();
    // Not a regression: onprem's own baseline lists cases that need the dependency apps.
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("passing.txt");
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
    let base = dir.path().join("passing.txt");
    std::fs::write(&base, "").unwrap();
    // A case the OnPrem app cannot pass without its platform tables and dependency apps.
    let suite = dir.path().join("onprem.txt");
    std::fs::write(
        &suite,
        "61200 ObjectMetadata_Find_ApplicationDatabaseTableId_ReturnsARow\n",
    )
    .unwrap();
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
    assert!(
        err.contains("27.5 onprem") && err.contains("ObjectMetadata_Find_ApplicationDatabaseTableId"),
        "{err}"
    );
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
        3,
        "the main app and the two OnPrem suites: {:?}",
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
