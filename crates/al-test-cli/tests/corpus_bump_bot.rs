//! The corpus bump bot (`.github/workflows/corpus-bump-bot.yml`, `tools/corpus_bump_bot.sh`,
//! `tools/corpus_bump_report.py`): on a schedule it looks for a green corpus CI run newer than the pin, makes
//! the mechanical part of the bump (pin, oracle, harness counts) on a branch `corpus-bump/<commit>` and opens a
//! pull request whose body lists what is left for a person. Its safety properties:
//!
//! - it never merges, never turns on auto-merge and never force-pushes; it pushes only its own branches;
//! - it never writes a baseline (`metrics/`): the regen bot adds new passes after the merge, and a removal stays
//!   a human decision (CLAUDE.md hard rule 2);
//! - no workflow token gets `contents: write`: the push uses the deploy key of the regen bot;
//! - what needs judgment (a lost baseline case, a new failure, a landed awaiting assumption, a corpus guard
//!   without a gate, a harness count) is listed in the pull request, never decided by the bot.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap()
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
            "-c",
            "protocol.file.allow=always",
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

fn put(dir: &Path, name: &str, text: &str) {
    let p = dir.join(name);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, text).unwrap();
}

fn text(o: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

/// Runs a script; a fake `gh` written by another test thread can make the exec fail with ETXTBSY for a moment
/// (see `corpus_bump_script.rs`), so that one error is retried.
fn run(cmd: impl Fn() -> Command) -> Output {
    for _ in 0..20 {
        let o = cmd().output().unwrap();
        if !text(&o).contains("Text file busy") {
            return o;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    panic!("stayed busy");
}

// ---------------------------------------------------------------------------------------------------
// The workflow.

fn workflow() -> String {
    read(".github/workflows/corpus-bump-bot.yml")
}

/// The lines that are not YAML comments, so that the header may name what the bot does not do.
fn code(text: &str) -> String {
    text.lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn the_workflow_never_merges_and_never_force_pushes() {
    let w = code(&workflow());
    for bad in [
        "gh pr merge",
        "--auto",
        "--force",
        "push -f",
        "--admin",
        "gh pr review",
        "enablePullRequestAutoMerge",
        "refs/heads/main",
    ] {
        assert!(!w.contains(bad), "the bot must not use `{bad}`:\n{w}");
    }
    let pushes: Vec<&str> = w.lines().filter(|l| l.contains("git push")).collect();
    assert!(!pushes.is_empty(), "the bot pushes its branch");
    for p in pushes {
        assert!(
            p.contains("HEAD:refs/heads/corpus-bump/"),
            "the bot pushes only its own corpus-bump/ branch: {p}"
        );
    }
    assert!(w.contains("gh pr create"), "the bot opens a pull request");
    assert!(w.contains("--base main"), "{w}");
}

/// Least privilege, as for the regen bot: nothing is granted at the top, the job reads contents and writes only
/// pull requests, and the branch is pushed over SSH with the regen bot's deploy key. No other secret is used.
#[test]
fn no_workflow_token_gets_contents_write() {
    let w = workflow();
    assert!(w.contains("\npermissions: {}\n"), "no workflow-wide grant:\n{w}");
    let grants: Vec<&str> = w
        .lines()
        .map(str::trim)
        .filter(|l| l.ends_with(": write") || l.ends_with(": read"))
        .collect();
    assert_eq!(grants, ["contents: read", "pull-requests: write"], "{w}");
    assert!(
        w.contains("ssh-key: ${{ secrets.REGEN_BOT_DEPLOY_KEY }}"),
        "the push uses the deploy key:\n{w}"
    );
    assert_eq!(w.matches("secrets.").count(), 1, "no other secret:\n{w}");
}

#[test]
fn the_workflow_runs_on_a_schedule_one_at_a_time_and_leaves_existing_work_alone() {
    let w = workflow();
    assert!(w.contains("schedule:") && w.contains("cron:"), "{w}");
    assert!(w.contains("workflow_dispatch:"), "{w}");
    assert!(w.contains("cancel-in-progress: false"), "one bot run at a time");
    assert!(w.contains("tools/corpus_bump_bot.sh find"), "{w}");
    assert!(w.contains("tools/corpus_bump_bot.sh bump"), "{w}");
    // One open bot pull request at a time, and a branch that exists is never pushed again.
    assert!(w.contains("gh pr list") && w.contains("corpus-bump/"), "{w}");
    assert!(w.contains("git ls-remote"), "{w}");
}

/// A pull request opened with the workflow token starts no `pull_request` workflow, so CI runs on the push of
/// the bot's branch. The required checks of the ruleset `main` match by name, which a push run satisfies.
#[test]
fn ci_runs_on_the_bot_branches() {
    let ci = read(".github/workflows/ci.yml");
    let on = &ci[ci.find("\non:\n").unwrap()..ci.find("\npermissions:").unwrap()];
    assert!(on.contains("'corpus-bump/**'"), "{on}");
}

// ---------------------------------------------------------------------------------------------------
// `tools/corpus_bump_bot.sh find`.

/// A corpus repository: `master` has the pin and two later commits, `side` a commit master lacks.
struct Corpus {
    dir: tempfile::TempDir,
    pin: String,
    b: String,
    c: String,
    side: String,
}

fn corpus() -> Corpus {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path().join("corpus");
    std::fs::create_dir_all(r.join("tests")).unwrap();
    git(&r, &["init", "-q", "-b", "master"]);
    let commit = |name: &str| {
        std::fs::write(r.join("tests").join(name), name).unwrap();
        git(&r, &["add", "."]);
        git(&r, &["commit", "-qm", name]);
        git(&r, &["rev-parse", "HEAD"])
    };
    let pin = commit("a.al");
    let b = commit("b.al");
    let c = commit("c.al");
    git(&r, &["checkout", "-qb", "side"]);
    std::fs::write(r.join("tests/s.al"), "s").unwrap();
    git(&r, &["add", "."]);
    git(&r, &["commit", "-qm", "s"]);
    let side = git(&r, &["rev-parse", "HEAD"]);
    git(&r, &["checkout", "-q", "master"]);
    Corpus { dir, pin, b, c, side }
}

/// Artifacts of a complete run: the oracle and the OnPrem file per leg.
fn full_artifacts() -> String {
    let mut a = Vec::new();
    let mut id = 100;
    for leg in ["27.5", "28.5", "29.0"] {
        for size in [1000, 2000] {
            id += 1;
            a.push(format!(
                r#"{{"id":{id},"name":"junit-test-results-{leg}","size_in_bytes":{size}}}"#
            ));
        }
    }
    format!(r#"{{"artifacts":[{}]}}"#, a.join(","))
}

/// A fake `gh`: `run list` answers the green runs (newest first), `run view ID` and `api .../runs/ID/artifacts`
/// answer per run.
fn fake_gh(dir: &Path, runs: &[(u64, &str, bool)]) -> PathBuf {
    let list: Vec<String> = runs
        .iter()
        .map(|(id, sha, _)| format!(r#"{{"databaseId":{id},"headSha":"{sha}"}}"#))
        .collect();
    std::fs::write(dir.join("list.json"), format!("[{}]", list.join(","))).unwrap();
    for (id, sha, complete) in runs {
        std::fs::write(
            dir.join(format!("view-{id}.json")),
            format!(r#"{{"status":"completed","conclusion":"success","headSha":"{sha}"}}"#),
        )
        .unwrap();
        let arts = if *complete {
            full_artifacts()
        } else {
            r#"{"artifacts":[]}"#.to_string()
        };
        std::fs::write(dir.join(format!("arts-{id}.json")), arts).unwrap();
    }
    let d = dir.display();
    let gh = dir.join("gh");
    std::fs::write(
        &gh,
        format!(
            r#"#!/bin/bash
case "$*" in
  *"run list"*) cat {d}/list.json ;;
  *"run view"*) cat {d}/view-$3.json ;;
  *"/artifacts?"*) id=${{2#*runs/}}; cat {d}/arts-${{id%%/*}}.json ;;
  *) echo "unexpected gh $*" >&2; exit 9 ;;
esac
"#
        ),
    )
    .unwrap();
    Command::new("chmod").arg("+x").arg(&gh).status().unwrap();
    gh
}

fn find(c: &Corpus, gh: &Path) -> Output {
    run(|| {
        let mut cmd = Command::new("bash");
        cmd.arg(root().join("tools/corpus_bump_bot.sh"))
            .arg("find")
            .env("CORPUS_BUMP_GH", gh)
            .env("CORPUS_BUMP_CORPUS", c.dir.path().join("corpus"))
            .env("CORPUS_BUMP_NO_FETCH", "1")
            .env("CORPUS_BUMP_BOT_PIN", &c.pin);
        cmd
    })
}

#[test]
fn find_picks_the_newest_green_run_after_the_pin() {
    let c = corpus();
    let g = tempfile::tempdir().unwrap();
    let gh = fake_gh(g.path(), &[(3, &c.c, true), (2, &c.b, true), (1, &c.pin, true)]);
    let o = find(&c, &gh);
    assert!(o.status.success(), "{}", text(&o));
    let out = String::from_utf8_lossy(&o.stdout);
    assert_eq!(out, format!("run=3\nsha={}\n", c.c), "{}", text(&o));
}

#[test]
fn find_skips_a_run_that_fails_the_bump_checks_and_one_off_master() {
    let c = corpus();
    let g = tempfile::tempdir().unwrap();
    // Run 4 tested a commit master lacks; run 3 lacks its artifacts.
    let gh = fake_gh(g.path(), &[(4, &c.side, true), (3, &c.c, false), (2, &c.b, true)]);
    let o = find(&c, &gh);
    assert!(o.status.success(), "{}", text(&o));
    assert_eq!(
        String::from_utf8_lossy(&o.stdout),
        format!("run=2\nsha={}\n", c.b),
        "{}",
        text(&o)
    );
}

#[test]
fn find_prints_nothing_when_no_green_run_is_newer_than_the_pin() {
    let c = corpus();
    let g = tempfile::tempdir().unwrap();
    let gh = fake_gh(g.path(), &[(1, &c.pin, true)]);
    let o = find(&c, &gh);
    assert!(o.status.success(), "{}", text(&o));
    assert_eq!(String::from_utf8_lossy(&o.stdout), "", "{}", text(&o));
}

// ---------------------------------------------------------------------------------------------------
// `tools/corpus_bump_bot.sh commit REPO SHA RUN`.

/// A runner repository with a corpus submodule at the old commit, an oracle, a baseline, the harness and code.
struct Runner {
    dir: tempfile::TempDir,
    corpus: Corpus,
}

const BASE: &str = "# header\n100 A\n100 B\n";

fn runner() -> Runner {
    let c = corpus();
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    git(r, &["init", "-q", "-b", "main"]);
    let src = c.dir.path().join("corpus");
    git(r, &["submodule", "add", "-q", src.to_str().unwrap(), "corpus"]);
    git(&r.join("corpus"), &["checkout", "-q", &c.pin]);
    put(r, "oracle/oracle.toml", &format!("corpus_commit = \"{}\"\n", c.pin));
    put(r, "oracle/29/junit.xml", "<testsuites/>\n");
    put(r, "metrics/29/passing.txt", BASE);
    put(r, "crates/al-test-cli/tests/corpus_harness.rs", "// 10\n");
    put(r, "src/main.rs", "fn main() {}\n");
    git(r, &["add", "-A"]);
    git(r, &["commit", "-qm", "start"]);
    Runner { dir, corpus: c }
}

/// The mechanical bump to commit `c`: the submodule, the oracle and the harness counts.
fn bump_files(r: &Runner) {
    let p = r.dir.path();
    git(&p.join("corpus"), &["checkout", "-q", &r.corpus.c]);
    put(
        p,
        "oracle/oracle.toml",
        &format!("corpus_commit = \"{}\"\n", r.corpus.c),
    );
    put(p, "oracle/29/junit.xml", "<testsuites tests=\"1\"/>\n");
    put(p, "crates/al-test-cli/tests/corpus_harness.rs", "// 11\n");
}

fn commit(r: &Runner, sha: &str) -> Output {
    run(|| {
        let mut cmd = Command::new("bash");
        cmd.arg(root().join("tools/corpus_bump_bot.sh"))
            .args(["commit"])
            .arg(r.dir.path())
            .args([sha, "77"])
            .env("CORPUS_BUMP_BOT_NAME", "t")
            .env("CORPUS_BUMP_BOT_EMAIL", "t@t")
            .env("GIT_CONFIG_COUNT", "1")
            .env("GIT_CONFIG_KEY_0", "commit.gpgsign")
            .env("GIT_CONFIG_VALUE_0", "false");
        cmd
    })
}

fn commits(r: &Runner) -> usize {
    git(r.dir.path(), &["rev-list", "--count", "HEAD"]).parse().unwrap()
}

#[test]
fn commit_makes_one_commit_of_the_pin_oracle_and_harness() {
    let r = runner();
    bump_files(&r);
    let sha = r.corpus.c.clone();
    let o = commit(&r, &sha);
    assert!(o.status.success(), "{}", text(&o));
    assert_eq!(commits(&r), 2);
    let msg = git(r.dir.path(), &["log", "-1", "--format=%B"]);
    assert!(msg.contains(&sha[..12]), "{msg}");
    assert!(!msg.contains("[skip ci]"), "CI must run on the bump: {msg}");
    let files = git(r.dir.path(), &["show", "--name-only", "--format=", "HEAD"]);
    let mut files: Vec<&str> = files.lines().collect();
    files.sort();
    assert_eq!(
        files,
        [
            "corpus",
            "crates/al-test-cli/tests/corpus_harness.rs",
            "oracle/29/junit.xml",
            "oracle/oracle.toml"
        ]
    );
    assert!(git(r.dir.path(), &["status", "--porcelain"]).is_empty());
}

#[test]
fn commit_refuses_a_removed_baseline_line() {
    let r = runner();
    bump_files(&r);
    put(r.dir.path(), "metrics/29/passing.txt", "# header\n100 A\n");
    let o = commit(&r, &r.corpus.c.clone());
    assert!(!o.status.success());
    let t = text(&o);
    assert!(t.contains("metrics/29/passing.txt"), "{t}");
    assert_eq!(commits(&r), 1, "nothing committed");
}

/// The regen bot adds new passes after the merge (CLAUDE.md, Workflow): the bump bot writes no baseline at all.
#[test]
fn commit_refuses_any_baseline_change() {
    let r = runner();
    bump_files(&r);
    put(
        r.dir.path(),
        "metrics/29/passing.txt",
        "# header\n100 A\n100 B\n100 C\n",
    );
    let o = commit(&r, &r.corpus.c.clone());
    assert!(!o.status.success());
    assert!(text(&o).contains("metrics/29/passing.txt"), "{}", text(&o));
    assert_eq!(commits(&r), 1);
}

#[test]
fn commit_refuses_a_path_outside_the_bump() {
    let r = runner();
    bump_files(&r);
    put(r.dir.path(), "src/main.rs", "fn main() { evil() }\n");
    let o = commit(&r, &r.corpus.c.clone());
    assert!(!o.status.success());
    assert!(text(&o).contains("src/main.rs"), "{}", text(&o));
    assert_eq!(commits(&r), 1);
}

#[test]
fn commit_refuses_a_pin_that_is_not_the_target() {
    let r = runner();
    bump_files(&r);
    let o = commit(&r, &r.corpus.b.clone());
    assert!(!o.status.success());
    assert!(text(&o).contains("corpus"), "{}", text(&o));
    assert_eq!(commits(&r), 1);
}

// ---------------------------------------------------------------------------------------------------
// `tools/corpus_bump_report.py DIR`: the pull request body.

fn junit(cases: &[(&str, &str, &str)]) -> String {
    let mut s = String::from("<testsuites>\n");
    for (cls, name, outcome) in cases {
        let inner = match *outcome {
            "fail" => "<failure message=\"boom\"/>",
            "skip" => "<skipped message=\"refused\"/>",
            _ => "",
        };
        s.push_str(&format!(
            "<testsuite name=\"{cls}\"><testcase classname=\"{cls}\" name=\"{name}\">{inner}</testcase></testsuite>\n"
        ));
    }
    s.push_str("</testsuites>\n");
    s
}

/// The inputs of a bump with nothing left for a person.
fn clean_inputs() -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    let p = d.path();
    put(
        p,
        "meta.json",
        r#"{"sha":"1111111111111111111111111111111111111111","old":"2222222222222222222222222222222222222222","run":"77","repo":"o/corpus"}"#,
    );
    put(
        p,
        "bump.log",
        "oracle cases per leg, before -> after (OnPrem in brackets):\n  29.0: 10 -> 12 (2 more) [1 -> 1]\n",
    );
    put(p, "corpus-log.txt", "abc1234 Merge pull request #5 from o/staging\n");
    put(
        p,
        "diff.txt",
        "Baseline diff, BC 29: 2 newly pass, 1 fail and are not in the baseline, 0 baseline cases no longer pass\n",
    );
    put(p, "diff.code", "0\n");
    put(
        p,
        "awaiting.txt",
        "awaiting assumptions: 3, none with its corpus pin landed\n",
    );
    put(p, "awaiting.code", "0\n");
    put(p, "gates.txt", "test result: ok.\n");
    put(p, "gates.code", "0\n");
    put(p, "harness.txt", "test result: ok.\n");
    put(p, "harness.code", "0\n");
    put(p, "golden.txt", "test result: ok.\n");
    put(p, "golden.code", "0\n");
    put(
        p,
        "before/runtime-junit-29.xml",
        &junit(&[("Codeunit 100", "Old", "fail"), ("Codeunit 100", "A", "pass")]),
    );
    put(
        p,
        "after/runtime-junit-29.xml",
        &junit(&[("Codeunit 100", "Old", "fail"), ("Codeunit 100", "A", "pass")]),
    );
    d
}

fn report(dir: &Path) -> String {
    let o = run(|| {
        let mut cmd = Command::new("python3");
        cmd.arg("-I").arg(root().join("tools/corpus_bump_report.py")).arg(dir);
        cmd
    });
    assert!(o.status.success(), "{}", text(&o));
    String::from_utf8_lossy(&o.stdout).into_owned()
}

#[test]
fn report_with_nothing_left_says_so_and_that_the_bot_does_not_merge() {
    let d = clean_inputs();
    let body = report(d.path());
    assert!(body.contains("Nothing is left for a person"), "{body}");
    assert!(body.contains("does not merge"), "{body}");
    assert!(body.contains("111111111111"), "names the target: {body}");
    assert!(
        body.contains("Merge pull request #5"),
        "names the corpus changes: {body}"
    );
    assert!(body.contains("29.0: 10 -> 12"), "oracle counts: {body}");
    assert!(
        !body.contains("Codeunit 100 Old"),
        "a failure on the old pin too is not new: {body}"
    );
}

#[test]
fn report_lists_each_judgment_item_left_for_a_person() {
    let d = clean_inputs();
    let p = d.path();
    put(
        p,
        "diff.txt",
        "Baseline diff, BC 29: 0 newly pass, 0 fail and are not in the baseline, 1 baseline cases no longer pass\n\nIn the baseline but no longer passing (regressions) (1):\n  record (1):\n    100 B  [fail: boom]\n",
    );
    put(p, "diff.code", "1\n");
    put(
        p,
        "after/runtime-junit-29.xml",
        &junit(&[
            ("Codeunit 100", "Old", "fail"),
            ("Codeunit 100", "A", "pass"),
            ("Codeunit 200", "NewOne", "fail"),
        ]),
    );
    put(
        p,
        "awaiting.txt",
        "assumption some-id: its corpus pin landed (tests/x.al); remove it\n",
    );
    put(p, "awaiting.code", "1\n");
    put(
        p,
        "gates.txt",
        "corpus guards with no gate in al-version/src/gates.rs:\n  tests/y.al BC29PLUS\n",
    );
    put(p, "gates.code", "101\n");
    put(
        p,
        "harness.txt",
        "assertion `left == right` failed\n  left: 5\n right: 6\n",
    );
    put(p, "harness.code", "101\n");
    put(p, "golden.txt", "pinned_columns.rs is stale; rerun with UPDATE_GOLDEN=1\n");
    put(p, "golden.code", "101\n");
    let body = report(p);
    assert!(!body.contains("Nothing is left"), "{body}");
    assert!(body.contains("Left for a person"), "{body}");
    assert!(body.contains("100 B"), "the lost baseline case: {body}");
    assert!(body.contains("Codeunit 200 NewOne"), "the new failure: {body}");
    assert!(body.contains("some-id"), "the landed assumption: {body}");
    assert!(body.contains("tests/y.al BC29PLUS"), "the guard without a gate: {body}");
    assert!(body.contains("left: 5"), "the harness count: {body}");
    assert!(body.contains("pinned_columns.rs is stale"), "the golden files: {body}");
    assert!(
        body.contains("never removes"),
        "says the bot removed no baseline line: {body}"
    );
}

#[test]
fn report_stays_under_the_pull_request_body_limit() {
    let d = clean_inputs();
    let p = d.path();
    let many: Vec<(String, String, String)> = (0..5000)
        .map(|i| {
            (
                "Codeunit 300".to_string(),
                format!("Fails_{i:05}_with_a_long_name"),
                "fail".into(),
            )
        })
        .collect();
    let refs: Vec<(&str, &str, &str)> = many
        .iter()
        .map(|(a, b, c)| (a.as_str(), b.as_str(), c.as_str()))
        .collect();
    put(p, "after/runtime-junit-29.xml", &junit(&refs));
    let body = report(p);
    assert!(body.len() < 60_000, "{}", body.len());
    assert!(
        body.contains("5000"),
        "says how many there are: {}",
        &body[..2000.min(body.len())]
    );
}
