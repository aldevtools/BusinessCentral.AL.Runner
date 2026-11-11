//! `tools/record_windows_oracle.sh`: records the BC 28 isolation oracle (suites isolation-function and
//! isolation-codeunit) from a corpus "Nightly Windows" run. Tested against a fixture (a fake `gh`, a throwaway
//! corpus repository and a copy of `oracle/`), with no network. The script must refuse, changing nothing, when the
//! run is not that workflow, not green, not on an ancestor of the pin, when the isolation probe app changed between
//! the run's commit and the pin, when the artifact lacks a file, or when the download fails. When it records, it copies
//! the two XUnit files byte for byte and rewrites only the two suite tables and their README rows.

use std::path::{Path, PathBuf};
use std::process::Command;

const WORKFLOW: &str = "Nightly Windows (official MS BC container)";
const PROBE: &str = "tests/al-language-isolation-probe";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn git(dir: &Path, args: &[&str]) -> String {
    let o = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .output()
        .unwrap();
    assert!(
        o.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    String::from_utf8_lossy(&o.stdout).trim().to_string()
}

fn commit(corpus: &Path, file: &str, text: &str) -> String {
    let p = corpus.join(file);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, text).unwrap();
    git(corpus, &["add", "."]);
    git(corpus, &["commit", "-qm", file]);
    git(corpus, &["rev-parse", "HEAD"])
}

/// A repository root with a copy of `oracle/` and a corpus with three commits on master: `run_commit` (the
/// Windows run's), `pin` (a later commit that changes a file outside the probe app) and, on a side branch,
/// `changed_probe` (changes a probe file).
struct Fixture {
    dir: tempfile::TempDir,
    run_commit: String,
    pin: String,
    probe_changed: String,
    off_master: String,
}

impl Fixture {
    fn root(&self) -> PathBuf {
        self.dir.path().join("repo")
    }
    fn read(&self, rel: &str) -> String {
        std::fs::read_to_string(self.root().join(rel)).unwrap()
    }
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("repo");
    std::fs::create_dir_all(repo.join("oracle/28")).unwrap();
    for f in [
        "oracle/oracle.toml",
        "oracle/README.md",
        "oracle/28/xunit-isolation-function.xml",
        "oracle/28/xunit-isolation-codeunit.xml",
    ] {
        std::fs::copy(root().join(f), repo.join(f)).unwrap();
    }
    let corpus = repo.join("corpus");
    std::fs::create_dir_all(&corpus).unwrap();
    git(&corpus, &["init", "-q", "-b", "master"]);
    commit(&corpus, &format!("{PROBE}/src/a.al"), "x");
    let run_commit = commit(&corpus, "tests/al-language/b.al", "y");
    let pin = commit(&corpus, "tests/al-language/c.al", "z");
    let probe_changed = commit(&corpus, &format!("{PROBE}/src/a.al"), "changed");
    git(&corpus, &["checkout", "-qb", "side", &run_commit]);
    let off_master = commit(&corpus, "tests/al-language/d.al", "w");
    git(&corpus, &["checkout", "-q", "--detach", &pin]);
    Fixture {
        dir,
        run_commit,
        pin,
        probe_changed,
        off_master,
    }
}

fn xunit(tests: usize, date: &str) -> String {
    let mut s = format!(
        "\u{feff}<?xml version=\"1.0\" encoding=\"UTF-8\"?>\r\n<assemblies>\r\n  <assembly name=\"61302 F\" run-date=\"{date}\" run-time=\"10:00:00\" total=\"{tests}\">\r\n"
    );
    for i in 0..tests {
        s.push_str(&format!(
            "      <test name=\"F:T{i}\" method=\"T{i}\" time=\"0.01\" result=\"Pass\" />\r\n"
        ));
    }
    s.push_str("  </assembly>\r\n</assemblies>\r\n");
    s
}

/// The artifact zip `ID.zip` in the fixture folder, holding `files` (name, content).
fn artifact_zip(f: &Fixture, id: u64, files: &[(&str, &str)]) {
    let src = f.dir.path().join(format!("z{id}"));
    std::fs::create_dir_all(&src).unwrap();
    let mut names = Vec::new();
    for (name, text) in files {
        std::fs::write(src.join(name), text).unwrap();
        names.push(name.to_string());
    }
    let st = Command::new("zip")
        .current_dir(&src)
        .arg("-q")
        .arg(f.dir.path().join(format!("{id}.zip")))
        .args(&names)
        .status()
        .unwrap();
    assert!(st.success());
}

/// A fake `gh`: `run view` prints `run.json`, `api .../artifacts/ID/zip` prints `ID.zip` (or fails when the file
/// `fail-zip` exists), `api .../artifacts` prints `artifacts.json`.
fn fake_gh(f: &Fixture, run: &str, artifacts: &str) -> PathBuf {
    let d = f.dir.path();
    std::fs::write(d.join("run.json"), run).unwrap();
    std::fs::write(d.join("artifacts.json"), artifacts).unwrap();
    let gh = d.join("gh");
    let dd = d.display();
    std::fs::write(
        &gh,
        format!(
            "#!/bin/sh\ncase \"$*\" in\n  *\"run view\"*) cat {dd}/run.json;;\n  */zip*) if [ -e {dd}/fail-zip ]; then echo 'gh: Server Error (HTTP 502)' >&2; exit 1; fi; id=$(echo \"$*\" | sed 's#.*/artifacts/\\([0-9]*\\)/zip.*#\\1#'); cat {dd}/$id.zip;;\n  *artifacts*) cat {dd}/artifacts.json;;\n  *) echo \"unexpected gh $*\" >&2; exit 9;;\nesac\n"
        ),
    )
    .unwrap();
    Command::new("chmod").arg("+x").arg(&gh).status().unwrap();
    gh
}

fn run_json(workflow: &str, conclusion: &str, sha: &str) -> String {
    format!(r#"{{"status":"completed","conclusion":"{conclusion}","headSha":"{sha}","workflowName":"{workflow}"}}"#)
}

fn artifacts_json(id: u64, name: &str) -> String {
    format!(r#"{{"artifacts":[{{"id":{id},"name":"{name}","expired":false}}]}}"#)
}

fn record(f: &Fixture, gh: &Path, args: &[&str]) -> (i32, String) {
    // ETXTBSY: see corpus_bump_script.rs (a child of another test thread may hold the fake gh open).
    for _ in 0..20 {
        let o = Command::new("bash")
            .arg(root().join("tools/record_windows_oracle.sh"))
            .args(args)
            .env("RECORD_WINDOWS_ORACLE_GH", gh)
            .env("RECORD_WINDOWS_ORACLE_ROOT", f.root())
            .env("RECORD_WINDOWS_ORACLE_NO_FETCH", "1")
            .output()
            .unwrap();
        let all = format!(
            "{}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        );
        if !all.contains("Text file busy") {
            return (o.status.code().unwrap_or(-1), all);
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    panic!("{} stayed busy", gh.display());
}

/// The files the script may change, to show a refusal changed nothing.
fn snapshot(f: &Fixture) -> Vec<String> {
    [
        "oracle/oracle.toml",
        "oracle/README.md",
        "oracle/28/xunit-isolation-function.xml",
        "oracle/28/xunit-isolation-codeunit.xml",
    ]
    .iter()
    .map(|p| f.read(p))
    .collect()
}

fn good_artifact(f: &Fixture) -> (String, String) {
    let func = xunit(3, "2026-11-11");
    let cu = xunit(2, "2026-11-11");
    artifact_zip(
        f,
        777,
        &[
            ("xunit-isolation-function.xml", &func),
            ("xunit-isolation-codeunit.xml", &cu),
            ("xunit-al-language.xml", "<assemblies/>"),
        ],
    );
    (func, cu)
}

fn refused(f: &Fixture, run: &str, artifacts: &str, want: &str) {
    let before = snapshot(f);
    let gh = fake_gh(f, run, artifacts);
    let (code, out) = record(f, &gh, &["555"]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("refused"), "{out}");
    assert!(out.contains(want), "{out}");
    assert_eq!(before, snapshot(f), "a refusal changed a file: {out}");
}

#[test]
fn a_run_of_another_workflow_is_refused() {
    let f = fixture();
    good_artifact(&f);
    refused(
        &f,
        &run_json("CI", "success", &f.run_commit),
        &artifacts_json(777, "nightly-windows-bc28.4-555"),
        "not the workflow",
    );
}

#[test]
fn a_run_that_is_not_green_is_refused() {
    let f = fixture();
    good_artifact(&f);
    refused(
        &f,
        &run_json(WORKFLOW, "failure", &f.run_commit),
        &artifacts_json(777, "nightly-windows-bc28.4-555"),
        "not green",
    );
}

#[test]
fn a_run_whose_commit_is_not_an_ancestor_of_the_pin_is_refused() {
    let f = fixture();
    good_artifact(&f);
    refused(
        &f,
        &run_json(WORKFLOW, "success", &f.off_master),
        &artifacts_json(777, "nightly-windows-bc28.4-555"),
        "not an ancestor of the pin",
    );
}

#[test]
fn a_run_before_a_change_to_the_probe_app_is_refused() {
    let f = fixture();
    good_artifact(&f);
    git(
        &f.root().join("corpus"),
        &["checkout", "-q", "--detach", &f.probe_changed],
    );
    refused(
        &f,
        &run_json(WORKFLOW, "success", &f.run_commit),
        &artifacts_json(777, "nightly-windows-bc28.4-555"),
        "src/a.al",
    );
}

#[test]
fn an_artifact_without_the_codeunit_file_is_refused() {
    let f = fixture();
    artifact_zip(&f, 777, &[("xunit-isolation-function.xml", &xunit(3, "2026-11-11"))]);
    refused(
        &f,
        &run_json(WORKFLOW, "success", &f.run_commit),
        &artifacts_json(777, "nightly-windows-bc28.4-555"),
        "xunit-isolation-codeunit.xml",
    );
}

#[test]
fn a_failed_download_is_reported_as_one() {
    let f = fixture();
    good_artifact(&f);
    std::fs::write(f.dir.path().join("fail-zip"), "").unwrap();
    refused(
        &f,
        &run_json(WORKFLOW, "success", &f.run_commit),
        &artifacts_json(777, "nightly-windows-bc28.4-555"),
        "HTTP 502",
    );
}

#[test]
fn a_dry_run_prints_the_edits_and_changes_nothing() {
    let f = fixture();
    good_artifact(&f);
    let before = snapshot(&f);
    let gh = fake_gh(
        &f,
        &run_json(WORKFLOW, "success", &f.run_commit),
        &artifacts_json(777, "nightly-windows-bc28.4-555"),
    );
    let (code, out) = record(&f, &gh, &["--dry-run", "555"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("workflow_run = 555"), "{out}");
    assert!(out.contains(&format!("corpus_commit = \"{}\"", f.run_commit)), "{out}");
    assert!(out.contains("dry run: nothing changed"), "{out}");
    assert_eq!(before, snapshot(&f));
}

#[test]
fn a_good_run_is_recorded_into_the_two_suites_only() {
    let f = fixture();
    let (func, cu) = good_artifact(&f);
    let toml_before = f.read("oracle/oracle.toml");
    let gh = fake_gh(
        &f,
        &run_json(WORKFLOW, "success", &f.run_commit),
        &artifacts_json(777, "nightly-windows-bc28.4-555"),
    );
    let (code, out) = record(&f, &gh, &["--pin", &f.pin, "555"]);
    assert_eq!(code, 0, "{out}");
    // Byte for byte, the BOM and CRLF included.
    assert_eq!(f.read("oracle/28/xunit-isolation-function.xml"), func);
    assert_eq!(f.read("oracle/28/xunit-isolation-codeunit.xml"), cu);

    let toml = f.read("oracle/oracle.toml");
    for name in ["isolation-function", "isolation-codeunit"] {
        let start = toml.find(&format!("name = \"{name}\"")).unwrap();
        let block = &toml[start..toml[start..].find("\n\n").unwrap() + start];
        assert!(
            block.contains(&format!("corpus_commit = \"{}\"", f.run_commit)),
            "{block}"
        );
        assert!(block.contains("workflow_run = 555"), "{block}");
        assert!(block.contains("artifact_id = 777"), "{block}");
        assert!(block.contains("bc_version = \"28.4\""), "{block}");
    }
    // Every other line of oracle.toml is unchanged.
    let changed: Vec<_> = toml_before.lines().zip(toml.lines()).filter(|(a, b)| a != b).collect();
    assert!(changed.len() <= 8, "{changed:?}");
    assert_eq!(toml_before.lines().count(), toml.lines().count());
    for (a, _) in &changed {
        assert!(
            a.starts_with("corpus_commit")
                || a.starts_with("workflow_run")
                || a.starts_with("artifact_id")
                || a.starts_with("bc_version"),
            "{a}"
        );
    }

    let readme = f.read("oracle/README.md");
    let row = readme
        .lines()
        .find(|l| l.starts_with("| `isolation-function`"))
        .unwrap();
    assert!(row.contains("run 555, artifact 777"), "{row}");
    assert!(row.contains(&format!("corpus master {}", &f.run_commit[..8])), "{row}");
    assert!(row.ends_with("| 3 |"), "{row}");
    let row = readme
        .lines()
        .find(|l| l.starts_with("| `isolation-codeunit`"))
        .unwrap();
    assert!(row.contains("| 2 (T5 fails on BC, on purpose) |"), "{row}");
}

#[test]
fn usage_without_a_run_id() {
    let f = fixture();
    let gh = fake_gh(&f, "{}", "{}");
    let (code, out) = record(&f, &gh, &[]);
    assert_eq!(code, 2, "{out}");
    assert!(out.contains("usage"), "{out}");
}
