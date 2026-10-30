//! `tools/regen_bot.sh`: the regen bot on main (`.github/workflows/ci.yml`, job `regen-bot`) copies the
//! generated files that the green CI run of a push wrote into the checkout and commits them. It
//! never removes a baseline case (CLAUDE.md hard rule 2: a removal is a human decision), touches no
//! other path, commits nothing when nothing changed, and marks its commit `[skip ci]` so that the
//! push does not start CI again.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

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
        .args(args)
        .output()
        .expect("git runs");
    assert!(
        o.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    String::from_utf8(o.stdout).unwrap()
}

fn put(dir: &Path, name: &str, text: &str) {
    let p = dir.join(name);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, text).unwrap();
}

const BASE: &str = "# header\n100 A\n100 B\n";

/// A repository with one baseline and one generated doc, committed.
fn repo() -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    git(d.path(), &["init", "-q"]);
    git(d.path(), &["config", "user.name", "t"]);
    git(d.path(), &["config", "user.email", "t@example.com"]);
    git(d.path(), &["config", "commit.gpgsign", "false"]);
    put(d.path(), "metrics/29/passing.txt", BASE);
    put(d.path(), "docs/corpus-coverage.md", "old\n");
    put(d.path(), "src/main.rs", "fn main() {}\n");
    git(d.path(), &["add", "-A"]);
    git(d.path(), &["commit", "-q", "-m", "start"]);
    d
}

fn bot(repo: &Path, regen: &Path) -> Output {
    Command::new("bash")
        .arg(root().join("tools/regen_bot.sh"))
        .arg(repo)
        .arg(regen)
        .arg("0123456789abcdef")
        .output()
        .expect("bash runs")
}

fn commits(repo: &Path) -> usize {
    git(repo, &["rev-list", "--count", "HEAD"]).trim().parse().unwrap()
}

#[test]
fn new_passes_and_doc_changes_become_one_skip_ci_commit() {
    let r = repo();
    let g = tempfile::tempdir().unwrap();
    put(g.path(), "metrics/29/passing.txt", "# header\n100 A\n100 B\n100 C\n");
    put(g.path(), "metrics/29/passing-onprem.txt", "# header\n7 X\n");
    put(g.path(), "docs/corpus-coverage.md", "new\n");
    let o = bot(r.path(), g.path());
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert_eq!(commits(r.path()), 2);
    let msg = git(r.path(), &["log", "-1", "--format=%B"]);
    assert!(msg.contains("[skip ci]"), "{msg}");
    assert!(msg.contains("01234567"), "names the commit it was made from: {msg}");
    let files = git(r.path(), &["show", "--name-only", "--format=", "HEAD"]);
    let mut files: Vec<&str> = files.lines().collect();
    files.sort();
    assert_eq!(
        files,
        [
            "docs/corpus-coverage.md",
            "metrics/29/passing-onprem.txt",
            "metrics/29/passing.txt"
        ]
    );
    assert!(git(r.path(), &["status", "--porcelain"]).is_empty());
}

#[test]
fn a_lost_baseline_case_fails_and_commits_nothing() {
    let r = repo();
    let g = tempfile::tempdir().unwrap();
    put(g.path(), "metrics/29/passing.txt", "# header\n100 A\n100 C\n");
    put(g.path(), "docs/corpus-coverage.md", "new\n");
    let o = bot(r.path(), g.path());
    assert!(!o.status.success());
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(err.contains("100 B"), "names the lost case: {err}");
    assert_eq!(commits(r.path()), 1);
    assert_eq!(
        std::fs::read_to_string(r.path().join("metrics/29/passing.txt")).unwrap(),
        BASE
    );
    assert_eq!(
        std::fs::read_to_string(r.path().join("docs/corpus-coverage.md")).unwrap(),
        "old\n",
        "nothing is copied when a case is lost"
    );
}

#[test]
fn a_path_that_is_not_a_generated_file_is_refused() {
    let r = repo();
    let g = tempfile::tempdir().unwrap();
    put(g.path(), "src/main.rs", "fn main() { evil() }\n");
    let o = bot(r.path(), g.path());
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("src/main.rs"));
    assert_eq!(commits(r.path()), 1);
}

#[test]
fn nothing_changed_commits_nothing() {
    let r = repo();
    let g = tempfile::tempdir().unwrap();
    put(g.path(), "metrics/29/passing.txt", BASE);
    put(g.path(), "docs/corpus-coverage.md", "old\n");
    let o = bot(r.path(), g.path());
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert_eq!(commits(r.path()), 1);
}

/// The workflow wiring: the corpus job writes the baselines in its measured run and uploads the
/// generated files only on a push to main; the bot runs after CI on a push to main, one at a time,
/// fails when CI failed, goes through the script and never force-pushes.
#[test]
fn the_workflow_runs_the_bot_after_a_green_push_to_main() {
    let ci = std::fs::read_to_string(root().join(".github/workflows/ci.yml")).unwrap();
    let run = &ci[ci.find("name: Corpus pass rate, and the demand-driven").unwrap()..];
    let run = &run[..run.find("eager_pids=()").unwrap()];
    assert!(run.contains("update=(--update-baseline)"), "{run}");
    assert!(run.contains("refs/heads/main"), "only on a push to main:\n{run}");
    assert!(ci.contains("--assumptions-doc") && ci.contains("name: regen-${{ github.run_id }}"));
    let bot = std::fs::read_to_string(root().join(".github/workflows/regen-bot.yml")).unwrap();
    assert!(
        bot.contains("workflows: [CI]") && bot.contains("branches: [main]"),
        "{bot}"
    );
    assert!(bot.contains("github.event.workflow_run.event == 'push'"), "{bot}");
    assert!(bot.contains("cancel-in-progress: false"), "one bot run at a time");
    assert!(
        bot.contains("conclusion != 'success'") && bot.contains("exit 1"),
        "fails when CI failed"
    );
    assert!(bot.contains("tools/regen_bot.sh"));
    assert!(!bot.contains("--force"), "the bot never force-pushes");
}

/// Least privilege: the bot pushes over SSH with the deploy key `regen-bot` under a ruleset bypass for
/// deploy keys (GitHub accepts no "GitHub Actions" bypass on this repository), so no workflow token
/// writes contents: the workflow grants nothing at the top, the bot job only reads; ci.yml stays read-only.
#[test]
fn no_workflow_token_gets_contents_write() {
    let bot = std::fs::read_to_string(root().join(".github/workflows/regen-bot.yml")).unwrap();
    assert!(bot.contains("\npermissions: {}\n"), "no workflow-wide grant:\n{bot}");
    let grants = |text: &str| text.lines().filter(|l| l.trim() == "contents: write").count();
    assert_eq!(grants(&bot), 0, "{bot}");
    assert!(
        bot.contains("ssh-key: ${{ secrets.REGEN_BOT_DEPLOY_KEY }}"),
        "the push uses the deploy key:\n{bot}"
    );
    let ci = std::fs::read_to_string(root().join(".github/workflows/ci.yml")).unwrap();
    assert!(ci.contains("\npermissions:\n  contents: read\n"), "ci.yml is read-only");
    assert_eq!(grants(&ci), 0, "no job of ci.yml writes to the repository");
}
