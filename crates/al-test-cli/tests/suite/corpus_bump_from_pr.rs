//! `tools/corpus_bump.sh --from-pr` (`tools/corpus_bump_pr.py`, owner decision 2026-11-05): a bump that takes the
//! results of a corpus pull request's own CI run instead of waiting for the master run. Against a fixture (a fake `gh`
//! and a throwaway corpus repository), with no network: the refusals, and the oracle edit (the codeunits the run ran
//! replaced, every other case kept).

use std::path::{Path, PathBuf};
use std::process::Command;

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

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn junit(suites: &[(&str, &[&str])]) -> String {
    let mut s = String::from("<?xml version='1.0' encoding='utf-8'?>\n<testsuites name=\"hybrid\">");
    for (cu, cases) in suites {
        s += &format!("<testsuite name=\"Codeunit {cu}\" tests=\"{}\">", cases.len());
        for c in *cases {
            s += &format!("<testcase classname=\"Codeunit {cu}\" name=\"{c}\" />");
        }
        s += "</testsuite>";
    }
    s + "</testsuites>\n"
}

/// The fixture: a corpus repository whose master has the pin, then the merge of a pull request branch; an al-runtime-rs
/// root with an oracle (codeunits 50001 and 50099 for every major); a fake `gh` for pull request 7 and its run 55.
struct Fx {
    dir: tempfile::TempDir,
    pin: String,
    head: String,
    merge: String,
}

impl Fx {
    fn corpus(&self) -> PathBuf {
        self.dir.path().join("corpus")
    }
    fn root(&self) -> PathBuf {
        self.dir.path().join("root")
    }
}

/// `pr_files`: (path, content) the pull request branch writes; `outside`: (path, content) a master commit writes
/// after the branch left master and before the merge (an unlisted commit when `outside_before_pin` is false).
fn fixture(pr_files: &[(&str, &str)], outside: Option<(&str, &str)>) -> Fx {
    let dir = tempfile::tempdir().unwrap();
    let c = dir.path().join("corpus");
    write(
        &c.join("tests/al-language/a/Cu1.al"),
        "codeunit 50001 \"A\"\n{\n    Subtype = Test;\n}\n",
    );
    write(&c.join("tests/al-language/_fixtures/F.al"), "table 50500 F {}\n");
    git(&c, &["init", "-q", "-b", "master"]);
    git(&c, &["add", "."]);
    git(&c, &["commit", "-qm", "pin"]);
    let pin = git(&c, &["rev-parse", "HEAD"]);
    git(&c, &["checkout", "-qb", "pr"]);
    for (p, t) in pr_files {
        write(&c.join(p), t);
    }
    git(&c, &["add", "."]);
    git(&c, &["commit", "-qm", "pr"]);
    let head = git(&c, &["rev-parse", "HEAD"]);
    git(&c, &["checkout", "-q", "master"]);
    if let Some((p, t)) = outside {
        write(&c.join(p), t);
        git(&c, &["add", "."]);
        git(&c, &["commit", "-qm", "outside"]);
    }
    git(&c, &["merge", "-q", "--no-ff", "-m", "Merge pull request #7", "pr"]);
    let merge = git(&c, &["rev-parse", "HEAD"]);
    git(&c, &["checkout", "-q", &pin]);
    let r = dir.path().join("root");
    for m in ["27", "28", "29"] {
        write(
            &r.join(format!("oracle/{m}/junit.xml")),
            &junit(&[("50001", &["A1"]), ("50099", &["Z1", "Z2"])]),
        );
    }
    write(
        &r.join("oracle/oracle.toml"),
        &format!("corpus_commit = \"{pin}\"\n\n[[version]]\nmajor = 27\n"),
    );
    write(
        &r.join("oracle/README.md"),
        &format!(
            "| Corpus pin | `{pin}` (x) |\n\n| 27 | 27.5 | `{}` (the pin) | 1 | `junit-test-results-27.5` | 2 | 3 |\n",
            &pin[..7]
        ),
    );
    Fx { dir, pin, head, merge }
}

/// A fake `gh`: `pr view` answers `pr`, `run list` answers `runs`, the artifact list of run 55 one artifact per
/// (leg, JUnit file) of `arts`, each served as a zip holding that JUnit file.
fn fake_gh(fx: &Fx, pr: &str, runs: &str, arts: &[(&str, &str, String)]) -> PathBuf {
    let d = fx.dir.path().join("gh-data");
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("pr.json"), pr).unwrap();
    std::fs::write(d.join("runs.json"), runs).unwrap();
    let mut lines = String::new();
    for (k, (leg, xml, body)) in arts.iter().enumerate() {
        let id = 900 + k;
        lines += &format!(
            "{{\"id\":{id},\"name\":\"junit-test-results-{leg}\",\"created_at\":\"2026-11-05T00:00:{k:02}Z\"}}\n"
        );
        let src = d.join(format!("{id}-{xml}"));
        std::fs::write(&src, body).unwrap();
        let st = Command::new("python3")
            .args([
                "-c",
                "import sys, zipfile\nwith zipfile.ZipFile(sys.argv[1], 'w') as z: z.write(sys.argv[2], sys.argv[3])",
            ])
            .arg(d.join(format!("{id}.zip")))
            .arg(&src)
            .arg(xml)
            .status()
            .unwrap();
        assert!(st.success());
    }
    std::fs::write(d.join("arts.jsonl"), lines).unwrap();
    let gh = d.join("gh");
    let dd = d.display();
    std::fs::write(
        &gh,
        format!(
            "#!/bin/sh\ncase \"$*\" in\n  *\"pr view\"*) cat {dd}/pr.json;;\n  *\"run list\"*) cat {dd}/runs.json;;\n  */zip*) id=$(echo \"$*\" | sed 's#.*/artifacts/\\([0-9]*\\)/zip.*#\\1#'); cat {dd}/$id.zip;;\n  *runs/55/artifacts*) cat {dd}/arts.jsonl;;\n  *) echo \"unexpected gh $*\" >&2; exit 9;;\nesac\n"
        ),
    )
    .unwrap();
    Command::new("chmod").arg("+x").arg(&gh).status().unwrap();
    gh
}

fn pr_json(fx: &Fx, state: &str) -> String {
    format!(
        r#"{{"number":7,"state":"{state}","mergeCommit":{{"oid":"{}"}},"headRefOid":"{}","baseRefName":"master","title":"t"}}"#,
        fx.merge, fx.head
    )
}

fn runs_json(fx: &Fx, conclusion: &str) -> String {
    format!(
        r#"[{{"databaseId":55,"status":"completed","conclusion":"{conclusion}","createdAt":"2026-11-05T00:00:00Z","headSha":"{}"}}]"#,
        fx.head
    )
}

/// The run's JUnit of tests/al-language for every leg: codeunit 50001 with a new case, the new codeunit 50002.
fn green_arts() -> Vec<(&'static str, &'static str, String)> {
    ["27.5", "28.5", "29.0"]
        .iter()
        .map(|l| {
            (
                *l,
                "junit-al-language.xml",
                junit(&[("50001", &["A1", "A2"]), ("50002", &["B1"])]),
            )
        })
        .collect()
}

const NEW_CU: (&str, &str) = (
    "tests/al-language/a/Cu2.al",
    "codeunit 50002 \"B\"\n{\n    Subtype = Test;\n}\n",
);
const EDIT_CU: (&str, &str) = (
    "tests/al-language/a/Cu1.al",
    "codeunit 50001 \"A\"\n{\n    Subtype = Test;\n    // A2\n}\n",
);

fn bump_pr(fx: &Fx, gh: &Path, extra: &[&str]) -> (i32, String) {
    for _ in 0..20 {
        let o = Command::new("python3")
            .arg(root().join("tools/corpus_bump_pr.py"))
            .arg("--root")
            .arg(fx.root())
            .args(["--prs", "7"])
            .args(extra)
            .env("CORPUS_BUMP_GH", gh)
            .env("CORPUS_BUMP_CORPUS", fx.corpus())
            .env("CORPUS_BUMP_NO_FETCH", "1")
            .env("CORPUS_BUMP_PIN", &fx.pin)
            .output()
            .unwrap();
        let all = format!(
            "{}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        );
        // ETXTBSY from another test thread's fork while the fake gh was open for writing: the harness, not the tool.
        if !all.contains("Text file busy") {
            return (o.status.code().unwrap_or(-1), all);
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    panic!("the fake gh stayed busy");
}

#[test]
fn the_run_codeunits_replace_the_oracle_cases_and_the_rest_is_kept() {
    let fx = fixture(&[NEW_CU, EDIT_CU], None);
    let gh = fake_gh(&fx, &pr_json(&fx, "MERGED"), &runs_json(&fx, "success"), &green_arts());
    let body = fx.dir.path().join("body.md");
    let (code, out) = bump_pr(&fx, &gh, &["--body", body.to_str().unwrap()]);
    assert_eq!(code, 0, "{out}");
    for m in ["27", "28", "29"] {
        let j = std::fs::read_to_string(fx.root().join(format!("oracle/{m}/junit.xml"))).unwrap();
        for case in ["\"A1\"", "\"A2\"", "\"B1\"", "\"Z1\"", "\"Z2\""] {
            assert!(j.contains(case), "BC {m} lacks {case}: {j}");
        }
        assert_eq!(j.matches("<testsuite name=\"Codeunit 50001\"").count(), 1, "{j}");
    }
    let toml = std::fs::read_to_string(fx.root().join("oracle/oracle.toml")).unwrap();
    assert!(toml.contains(&format!("corpus_commit = \"{}\"", fx.merge)), "{toml}");
    let readme = std::fs::read_to_string(fx.root().join("oracle/README.md")).unwrap();
    assert!(
        readme.contains("pr-results-bump:start") && readme.contains("run 55"),
        "{readme}"
    );
    assert!(
        readme.contains("(full run; PR results on top, see below) | 1 | `junit-test-results-27.5` | 2 | 5 |"),
        "{readme}"
    );
    assert_eq!(git(&fx.corpus(), &["rev-parse", "HEAD"]), fx.merge);
    let b = std::fs::read_to_string(&body).unwrap();
    assert!(
        b.contains("PR-results corpus bump") && b.contains("actions/runs/55"),
        "{b}"
    );
    assert!(b.contains("oracle/27/junit.xml: 3 -> 5"), "{b}");
}

#[test]
fn a_dry_run_changes_nothing() {
    let fx = fixture(&[NEW_CU], None);
    let gh = fake_gh(&fx, &pr_json(&fx, "MERGED"), &runs_json(&fx, "success"), &green_arts());
    let before = std::fs::read_to_string(fx.root().join("oracle/27/junit.xml")).unwrap();
    let (code, out) = bump_pr(&fx, &gh, &["--dry-run"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("dry run: nothing changed"), "{out}");
    assert_eq!(
        before,
        std::fs::read_to_string(fx.root().join("oracle/27/junit.xml")).unwrap()
    );
    assert_eq!(git(&fx.corpus(), &["rev-parse", "HEAD"]), fx.pin);
}

fn refused(fx: &Fx, gh: &Path, needle: &str) {
    let (code, out) = bump_pr(fx, gh, &["--dry-run"]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("refused") && out.contains(needle), "{out}");
}

#[test]
fn a_pull_request_that_is_not_merged_is_refused() {
    let fx = fixture(&[NEW_CU], None);
    let gh = fake_gh(&fx, &pr_json(&fx, "OPEN"), &runs_json(&fx, "success"), &green_arts());
    refused(&fx, &gh, "is not merged");
}

#[test]
fn a_last_run_that_is_not_green_is_refused() {
    let fx = fixture(&[NEW_CU], None);
    let gh = fake_gh(&fx, &pr_json(&fx, "MERGED"), &runs_json(&fx, "failure"), &green_arts());
    refused(&fx, &gh, "is not green");
}

#[test]
fn a_missing_bc_version_is_refused() {
    let fx = fixture(&[NEW_CU], None);
    let mut arts = green_arts();
    arts.pop();
    let gh = fake_gh(&fx, &pr_json(&fx, "MERGED"), &runs_json(&fx, "success"), &arts);
    refused(&fx, &gh, "a BC version is missing");
}

#[test]
fn a_change_to_a_shared_fixture_is_refused() {
    let fx = fixture(
        &[NEW_CU, ("tests/al-language/_fixtures/F.al", "table 50500 F { }\n")],
        None,
    );
    let gh = fake_gh(&fx, &pr_json(&fx, "MERGED"), &runs_json(&fx, "success"), &green_arts());
    refused(&fx, &gh, "shared fixture");
}

#[test]
fn an_unlisted_master_commit_between_the_pin_and_the_merge_is_refused() {
    let fx = fixture(
        &[NEW_CU],
        Some(("tests/al-language/b/Cu3.al", "codeunit 50003 \"C\" {}\n")),
    );
    let gh = fake_gh(&fx, &pr_json(&fx, "MERGED"), &runs_json(&fx, "success"), &green_arts());
    refused(&fx, &gh, "is not one of the listed pull requests");
}

#[test]
fn a_changed_codeunit_with_oracle_cases_the_run_did_not_run_is_refused() {
    let fx = fixture(&[NEW_CU, EDIT_CU], None);
    let arts: Vec<_> = ["27.5", "28.5", "29.0"]
        .iter()
        .map(|l| (*l, "junit-al-language.xml", junit(&[("50002", &["B1"])])))
        .collect();
    let gh = fake_gh(&fx, &pr_json(&fx, "MERGED"), &runs_json(&fx, "success"), &arts);
    refused(&fx, &gh, "codeunit 50001");
}

#[test]
fn an_oracle_that_would_lose_cases_is_refused() {
    let fx = fixture(&[EDIT_CU], None);
    let arts: Vec<_> = ["27.5", "28.5", "29.0"]
        .iter()
        .map(|l| (*l, "junit-al-language.xml", junit(&[("50001", &[])])))
        .collect();
    let gh = fake_gh(&fx, &pr_json(&fx, "MERGED"), &runs_json(&fx, "success"), &arts);
    refused(&fx, &gh, "never shrinks");
}

/// The shell entry point passes --from-pr on and checks its argument.
#[test]
fn the_shell_script_takes_from_pr_and_checks_its_argument() {
    let o = Command::new("bash")
        .arg(root().join("tools/corpus_bump.sh"))
        .args(["--dry-run", "--from-pr", "7,x"])
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&o.stderr).contains("--from-pr N[,N...]"));
}

/// A bump from a master run replaces every oracle file, so it removes the record of PR-results bumps.
#[test]
fn a_normal_bump_removes_the_record_of_pr_results_bumps() {
    let dir = tempfile::tempdir().unwrap();
    let o = dir.path().join("oracle");
    std::fs::create_dir_all(&o).unwrap();
    std::fs::copy(root().join("oracle/oracle.toml"), o.join("oracle.toml")).unwrap();
    let readme = std::fs::read_to_string(root().join("oracle/README.md")).unwrap();
    std::fs::write(
        o.join("README.md"),
        format!("{readme}\n<!-- pr-results-bump:start -->\n## x\n- pin\n<!-- pr-results-bump:end -->\n"),
    )
    .unwrap();
    let st = Command::new("python3")
        .arg(root().join("tools/corpus_bump_edit.py"))
        .arg(dir.path())
        .args([&"a".repeat(40), "1"])
        .status()
        .unwrap();
    assert!(st.success());
    let after = std::fs::read_to_string(o.join("README.md")).unwrap();
    assert!(!after.contains("pr-results-bump"), "{after}");
}
