//! `tools/corpus_bump.sh` (issue 44): the refusal paths, against a fixture (a fake `gh` and a throwaway corpus
//! repository), with no network. The script must refuse before it changes anything when the run is not green,
//! when its head commit is not on the corpus branch `master`, or when a leg does not have exactly the two
//! artifacts (the oracle and the OnPrem file).

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

/// A corpus repository with a commit on `master` and one on a side branch only.
struct Fixture {
    dir: tempfile::TempDir,
    on_branch: String,
    off_branch: String,
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let corpus = dir.path().join("corpus");
    std::fs::create_dir_all(corpus.join("tests")).unwrap();
    git(&corpus, &["init", "-q", "-b", "master"]);
    std::fs::write(corpus.join("tests/a.al"), "x").unwrap();
    git(&corpus, &["add", "."]);
    git(&corpus, &["commit", "-qm", "a"]);
    let on_branch = git(&corpus, &["rev-parse", "HEAD"]);
    git(&corpus, &["checkout", "-qb", "side"]);
    std::fs::write(corpus.join("tests/b.al"), "y").unwrap();
    git(&corpus, &["add", "."]);
    git(&corpus, &["commit", "-qm", "b"]);
    let off_branch = git(&corpus, &["rev-parse", "HEAD"]);
    git(&corpus, &["checkout", "-q", "master"]);
    Fixture {
        dir,
        on_branch,
        off_branch,
    }
}

/// A fake `gh` that answers `run view` with `run` and `api .../artifacts` with `artifacts`.
fn fake_gh(dir: &Path, run: &str, artifacts: &str) -> PathBuf {
    let gh = dir.join("gh");
    std::fs::write(dir.join("run.json"), run).unwrap();
    std::fs::write(dir.join("artifacts.json"), artifacts).unwrap();
    let d = dir.display();
    std::fs::write(
        &gh,
        format!(
            "#!/bin/sh\ncase \"$*\" in\n  *\"run view\"*) cat {d}/run.json;;\n  *artifacts*) cat {d}/artifacts.json;;\n  *) echo \"unexpected gh $*\" >&2; exit 9;;\nesac\n"
        ),
    )
    .unwrap();
    Command::new("chmod").arg("+x").arg(&gh).status().unwrap();
    gh
}

fn bump(f: &Fixture, gh: &Path, args: &[&str]) -> (i32, String) {
    let o = Command::new("bash")
        .arg(root().join("tools/corpus_bump.sh"))
        .args(args)
        .env("CORPUS_BUMP_GH", gh)
        .env("CORPUS_BUMP_CORPUS", f.dir.path().join("corpus"))
        .env("CORPUS_BUMP_NO_FETCH", "1")
        .output()
        .unwrap();
    let all = format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    (o.status.code().unwrap_or(-1), all)
}

fn run_json(conclusion: &str, sha: &str) -> String {
    format!(r#"{{"status":"completed","conclusion":"{conclusion}","headSha":"{sha}"}}"#)
}

fn artifacts(legs: &[&str], per_leg: usize) -> String {
    let mut a = Vec::new();
    let mut id = 100;
    for leg in legs {
        for k in 0..per_leg {
            id += 1;
            a.push(format!(
                r#"{{"id":{id},"name":"junit-test-results-{leg}","size_in_bytes":{}}}"#,
                1000 * (k + 1)
            ));
        }
    }
    format!(r#"{{"artifacts":[{}]}}"#, a.join(","))
}

#[test]
fn a_run_that_is_not_green_is_refused() {
    let f = fixture();
    let gh = fake_gh(
        f.dir.path(),
        &run_json("failure", &f.on_branch),
        &artifacts(&["27.5", "28.5", "29.0"], 2),
    );
    let (code, out) = bump(&f, &gh, &["--dry-run", "123"]);
    assert_ne!(code, 0, "{out}");
    assert!(out.contains("not green"), "{out}");
}

#[test]
fn a_run_whose_commit_is_not_on_the_corpus_branch_is_refused() {
    let f = fixture();
    let gh = fake_gh(
        f.dir.path(),
        &run_json("success", &f.off_branch),
        &artifacts(&["27.5", "28.5", "29.0"], 2),
    );
    let (code, out) = bump(&f, &gh, &["--dry-run", "123"]);
    assert_ne!(code, 0, "{out}");
    assert!(out.contains("not on the corpus branch master"), "{out}");
}

#[test]
fn a_leg_without_its_two_artifacts_is_refused() {
    let f = fixture();
    let gh = fake_gh(
        f.dir.path(),
        &run_json("success", &f.on_branch),
        &artifacts(&["27.5", "28.5", "29.0"], 1),
    );
    let (code, out) = bump(&f, &gh, &["--dry-run", "123"]);
    assert_ne!(code, 0, "{out}");
    assert!(
        out.contains("junit-test-results-27.5") && out.contains("expected 2"),
        "{out}"
    );
}

#[test]
fn a_green_run_on_the_branch_passes_the_checks_in_a_dry_run() {
    let f = fixture();
    let gh = fake_gh(
        f.dir.path(),
        &run_json("success", &f.on_branch),
        &artifacts(&["27.5", "28.5", "29.0"], 2),
    );
    let (code, out) = bump(&f, &gh, &["--dry-run", "123"]);
    assert_eq!(code, 0, "{out}");
    // The oracle is the larger artifact of a leg, the OnPrem file the smaller.
    assert!(out.contains("27.5: oracle artifact 102, onprem artifact 101"), "{out}");
    assert!(out.contains("dry run: nothing changed"), "{out}");
}

/// A staging commit merged into `master` with a merge commit, the way a bump pull request lands
/// (ruleset of the corpus repo: master takes pull requests only). With `outside`, master first gets a
/// commit staging does not have (an outside contributor's merge), so the merge commit's files differ
/// from the staging commit. Returns the staging commit.
fn merged_by_pull_request(f: &Fixture, outside: bool) -> String {
    let corpus = f.dir.path().join("corpus");
    git(&corpus, &["checkout", "-qb", "staging", "master"]);
    std::fs::write(corpus.join("tests/c.al"), "z").unwrap();
    git(&corpus, &["add", "."]);
    git(&corpus, &["commit", "-qm", "c"]);
    let staging = git(&corpus, &["rev-parse", "HEAD"]);
    git(&corpus, &["checkout", "-q", "master"]);
    if outside {
        std::fs::write(corpus.join("tests/d.al"), "w").unwrap();
        git(&corpus, &["add", "."]);
        git(&corpus, &["commit", "-qm", "d"]);
    }
    git(&corpus, &["merge", "-q", "--no-ff", "-m", "bump", "staging"]);
    staging
}

#[test]
fn a_staging_commit_merged_into_master_by_pull_request_passes_the_checks_in_a_dry_run() {
    let f = fixture();
    let staging = merged_by_pull_request(&f, false);
    let gh = fake_gh(
        f.dir.path(),
        &run_json("success", &staging),
        &artifacts(&["27.5", "28.5", "29.0"], 2),
    );
    let (code, out) = bump(&f, &gh, &["--dry-run", "123"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("dry run: nothing changed"), "{out}");
}

#[test]
fn a_staging_commit_whose_merge_brought_other_master_changes_is_refused() {
    // The pull request run tests staging merged with master. When master had commits staging did not
    // have, the run's JUnit covers files the pinned staging commit does not hold.
    let f = fixture();
    let staging = merged_by_pull_request(&f, true);
    let gh = fake_gh(
        f.dir.path(),
        &run_json("success", &staging),
        &artifacts(&["27.5", "28.5", "29.0"], 2),
    );
    let (code, out) = bump(&f, &gh, &["--dry-run", "123"]);
    assert_ne!(code, 0, "{out}");
    assert!(out.contains("merge origin/master into"), "{out}");
}

/// A job rerun of the bump pull request (corpus run 39303760055, attempt 2: BC 27.5 and OnPrem DotNet 29.0
/// re-ran) leaves the first attempt's artifact next to the new one under the same name. The role of each
/// artifact is the XML file it holds; per role the newest artifact is the one of the green attempt.
#[test]
fn a_rerun_job_leaves_two_artifacts_of_a_role_and_the_newest_wins() {
    let f = fixture();
    let d = f.dir.path();
    let mut a = Vec::new();
    let mut id = 200;
    for leg in ["27.5", "28.5", "29.0"] {
        let mut roles = vec![
            ("junit-al-language.xml", 250_000, "2026-10-19T19:00:00Z"),
            ("junit-al-language-onprem.xml", 6_000, "2026-10-19T18:40:00Z"),
            ("junit-al-language-onprem-dotnet.xml", 3_000, "2026-10-19T18:41:00Z"),
        ];
        if leg == "27.5" {
            // The rerun oracle: smaller than the first attempt's, created later.
            roles.push(("junit-al-language.xml", 249_000, "2026-10-19T20:08:00Z"));
        }
        for (xml, size, at) in roles {
            id += 1;
            let src = d.join(format!("z{id}"));
            std::fs::create_dir_all(&src).unwrap();
            std::fs::write(src.join(xml), "<testsuites/>").unwrap();
            let st = Command::new("zip")
                .current_dir(&src)
                .args(["-q", &format!("{}/{id}.zip", d.display()), xml])
                .status()
                .unwrap();
            assert!(st.success());
            a.push(format!(
                r#"{{"id":{id},"name":"junit-test-results-{leg}","size_in_bytes":{size},"created_at":"{at}"}}"#
            ));
        }
    }
    let gh = fake_gh(
        d,
        &run_json("success", &f.on_branch),
        &format!(r#"{{"artifacts":[{}]}}"#, a.join(",")),
    );
    // Serve each artifact's zip: `api repos/.../actions/artifacts/ID/zip`.
    let dd = d.display();
    std::fs::write(
        &gh,
        format!(
            "#!/bin/sh\ncase \"$*\" in\n  *\"run view\"*) cat {dd}/run.json;;\n  */zip*) id=$(echo \"$*\" | sed 's#.*/artifacts/\\([0-9]*\\)/zip.*#\\1#'); cat {dd}/$id.zip;;\n  *artifacts*) cat {dd}/artifacts.json;;\n  *) echo \"unexpected gh $*\" >&2; exit 9;;\nesac\n"
        ),
    )
    .unwrap();
    let (code, out) = bump(&f, &gh, &["--dry-run", "123"]);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("27.5: oracle artifact 204, onprem artifact 202, dotnet artifact 203"),
        "{out}"
    );
    assert!(
        out.contains("28.5: oracle artifact 205, onprem artifact 206, dotnet artifact 207"),
        "{out}"
    );
}

/// A leg has a fourth artifact since the corpus added the TestLibraries job: it is not part of the oracle.
#[test]
fn the_test_libraries_artifact_of_a_leg_is_left_out() {
    let f = fixture();
    let d = f.dir.path();
    let mut a = Vec::new();
    let mut id = 200;
    for leg in ["27.5", "28.5", "29.0"] {
        let mut roles = vec![
            ("junit-al-language.xml", 250_000, "2026-10-19T19:00:00Z"),
            ("junit-al-language-onprem.xml", 6_000, "2026-10-19T18:40:00Z"),
            ("junit-al-language-onprem-dotnet.xml", 3_000, "2026-10-19T18:41:00Z"),
        ];
        // The TestLibraries job (corpus tests/al-language-testlibraries) runs with Microsoft's Tests-TestLibraries
        // installed, an environment the runtime does not model: its results are not an oracle (orchestrator, 2026-10-22).
        roles.push(("junit-al-language-testlibraries.xml", 900, "2026-10-19T18:42:00Z"));
        for (xml, size, at) in roles {
            id += 1;
            let src = d.join(format!("z{id}"));
            std::fs::create_dir_all(&src).unwrap();
            std::fs::write(src.join(xml), "<testsuites/>").unwrap();
            let st = Command::new("zip")
                .current_dir(&src)
                .args(["-q", &format!("{}/{id}.zip", d.display()), xml])
                .status()
                .unwrap();
            assert!(st.success());
            a.push(format!(
                r#"{{"id":{id},"name":"junit-test-results-{leg}","size_in_bytes":{size},"created_at":"{at}"}}"#
            ));
        }
    }
    let gh = fake_gh(
        d,
        &run_json("success", &f.on_branch),
        &format!(r#"{{"artifacts":[{}]}}"#, a.join(",")),
    );
    // Serve each artifact's zip: `api repos/.../actions/artifacts/ID/zip`.
    let dd = d.display();
    std::fs::write(
        &gh,
        format!(
            "#!/bin/sh\ncase \"$*\" in\n  *\"run view\"*) cat {dd}/run.json;;\n  */zip*) id=$(echo \"$*\" | sed 's#.*/artifacts/\\([0-9]*\\)/zip.*#\\1#'); cat {dd}/$id.zip;;\n  *artifacts*) cat {dd}/artifacts.json;;\n  *) echo \"unexpected gh $*\" >&2; exit 9;;\nesac\n"
        ),
    )
    .unwrap();
    let (code, out) = bump(&f, &gh, &["--dry-run", "123"]);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("27.5: oracle artifact 201, onprem artifact 202, dotnet artifact 203"),
        "{out}"
    );
    assert!(
        out.contains("28.5: oracle artifact 205, onprem artifact 206, dotnet artifact 207"),
        "{out}"
    );
}

#[test]
fn usage_without_a_run_id() {
    let f = fixture();
    let gh = fake_gh(f.dir.path(), "{}", "{}");
    let (code, out) = bump(&f, &gh, &[]);
    assert_eq!(code, 2, "{out}");
    assert!(out.contains("usage"), "{out}");
}

/// Runs the oracle edit of the bump on a copy of the real `oracle/` with the given legs.
fn edit_copy(sha: &str, run: &str, legs: &[[&str; 5]]) -> (tempfile::TempDir, String, String) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("oracle")).unwrap();
    for f in ["oracle.toml", "README.md"] {
        std::fs::copy(root().join("oracle").join(f), dir.path().join("oracle").join(f)).unwrap();
    }
    let mut c = Command::new("python3");
    c.arg(root().join("tools/corpus_bump_edit.py"))
        .arg(dir.path())
        .arg(sha)
        .arg(run);
    for l in legs {
        c.args(l);
    }
    let o = c.output().unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let toml = std::fs::read_to_string(dir.path().join("oracle/oracle.toml")).unwrap();
    let readme = std::fs::read_to_string(dir.path().join("oracle/README.md")).unwrap();
    (dir, toml, readme)
}

/// The values of the current pin, read from the real oracle.toml.
fn current() -> (String, String, Vec<[String; 5]>) {
    let t = std::fs::read_to_string(root().join("oracle/oracle.toml")).unwrap();
    let v: toml::Value = toml::from_str(&t).unwrap();
    let sha = v["corpus_commit"].as_str().unwrap().to_string();
    let mut run = String::new();
    let mut legs = Vec::new();
    for ver in v["version"].as_array().unwrap() {
        run = ver["workflow_run"].as_integer().unwrap().to_string();
        let onprem = ver["suite"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["name"].as_str() == Some("onprem"))
            .unwrap()["artifact_id"]
            .as_integer()
            .unwrap();
        legs.push([
            ver["major"].as_integer().unwrap().to_string(),
            ver["artifact_id"].as_integer().unwrap().to_string(),
            onprem.to_string(),
            "0".into(),
            "0".into(),
        ]);
    }
    (sha, run, legs)
}

#[test]
fn re_applying_the_current_pin_changes_neither_oracle_toml_nor_the_pin_and_ids_of_the_readme() {
    let (sha, run, legs) = current();
    let legs: Vec<[&str; 5]> = legs
        .iter()
        .map(|l| {
            [
                l[0].as_str(),
                l[1].as_str(),
                l[2].as_str(),
                l[3].as_str(),
                l[4].as_str(),
            ]
        })
        .collect();
    let (_d, toml, _readme) = edit_copy(&sha, &run, &legs);
    assert_eq!(
        toml,
        std::fs::read_to_string(root().join("oracle/oracle.toml")).unwrap()
    );
}

#[test]
fn a_new_pin_moves_the_legs_and_leaves_suites_of_other_runs_alone() {
    let (old_sha, _, _) = current();
    let sha = "0123456789abcdef0123456789abcdef01234567";
    let legs = [
        ["27", "1", "2", "5600", "72"],
        ["28", "3", "4", "5598", "72"],
        ["29", "5", "6", "5598", "72"],
    ];
    let (_d, toml, readme) = edit_copy(sha, "999", &legs);
    let v: toml::Value = toml::from_str(&toml).unwrap();
    assert_eq!(v["corpus_commit"].as_str(), Some(sha));
    for (ver, l) in v["version"].as_array().unwrap().iter().zip(&legs) {
        assert_eq!(ver["workflow_run"].as_integer(), Some(999));
        assert_eq!(ver["artifact_id"].as_integer().unwrap().to_string(), l[1]);
        for s in ver["suite"].as_array().unwrap() {
            if s["name"].as_str() == Some("onprem") {
                assert_eq!(s["artifact_id"].as_integer().unwrap().to_string(), l[2]);
                assert_eq!(s["workflow_run"].as_integer(), Some(999));
            } else {
                // The isolation probe comes from another run and keeps its own commit and run.
                assert_ne!(s["workflow_run"].as_integer(), Some(999), "{s:?}");
                assert_ne!(s.get("corpus_commit").and_then(|c| c.as_str()), Some(sha), "{s:?}");
            }
        }
    }
    assert!(readme.contains(&format!("| Corpus pin | `{sha}`")), "{readme}");
    assert!(!readme.contains(&format!("| Corpus pin | `{old_sha}`")));
    assert!(
        readme.contains("| 27 | 27.5 | `0123456` (the pin) | 999 | `junit-test-results-27.5` | 1 | 5600 |"),
        "{readme}"
    );
}
