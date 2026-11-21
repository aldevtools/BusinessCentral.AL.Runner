//! `tools/machine_cleanup.sh`: lists, and with `--apply` deletes, the worktrees of finished pull requests, idle cargo
//! target folders, old compile cache entries and old scratch folders of a shared machine. These tests build a small
//! repository with worktrees and a cache folder in a temporary folder, feed the pull request states with `--prs`, and
//! check what goes and, above all, what stays: uncommitted or unpushed work, open pull requests, folders a running
//! process uses, recent folders and the artifact cache.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// git without the user's own configuration (no signing, no hooks).
fn git(dir: &Path, args: &[&str]) {
    let o = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .expect("git runs");
    assert!(
        o.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&o.stderr)
    );
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// Sets the modification time of everything below `dir` (and `dir` itself) to ten days ago.
fn age(dir: &Path) {
    let o = Command::new("find")
        .arg(dir)
        .args(["-exec", "touch", "-h", "-d", "10 days ago", "{}", "+"])
        .output()
        .unwrap();
    assert!(o.status.success());
}

struct Machine {
    _tmp: tempfile::TempDir,
    repo: PathBuf,
    cache: PathBuf,
    prs: PathBuf,
}

impl Machine {
    fn wt(&self, name: &str) -> PathBuf {
        self.repo.join(".worktrees").join(name)
    }

    fn run(&self, extra: &[&str]) -> String {
        let o: Output = Command::new("bash")
            .arg(root().join("tools/machine_cleanup.sh"))
            .arg("--prs")
            .arg(&self.prs)
            .args(extra)
            .current_dir(&self.repo)
            .env("ALRUNNER_CACHE", &self.cache)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output()
            .expect("bash runs");
        let out = String::from_utf8_lossy(&o.stdout).to_string();
        assert!(
            o.status.success(),
            "machine_cleanup.sh: {out}{}",
            String::from_utf8_lossy(&o.stderr)
        );
        out
    }
}

/// The repository, its worktrees and the cache folder; everything is ten days old except `target-fresh` and
/// `compile/new.alcc`.
fn machine() -> Machine {
    let tmp = tempfile::tempdir().unwrap();
    let origin = tmp.path().join("origin.git");
    let repo = tmp.path().join("repo");
    let cache = tmp.path().join("cache");
    std::fs::create_dir_all(&origin).unwrap();
    git(&origin, &["init", "-q", "--bare", "-b", "main"]);
    git(tmp.path(), &["clone", "-q", origin.to_str().unwrap(), "repo"]);
    write(&repo.join("README"), "x\n");
    git(&repo, &["add", "README"]);
    git(&repo, &["commit", "-q", "-m", "first"]);
    git(&repo, &["push", "-q", "origin", "HEAD:main"]);
    git(&repo, &["fetch", "-q", "origin"]);
    for name in ["merged", "dirty", "own", "open", "nopr-own", "nopr-main"] {
        let path = format!(".worktrees/{name}");
        git(&repo, &["worktree", "add", "-q", &path, "-b", name, "origin/main"]);
    }
    // `own` and `nopr-own` hold a commit that is on no remote branch.
    for name in ["own", "nopr-own"] {
        let wt = repo.join(".worktrees").join(name);
        write(&wt.join("work.txt"), "work\n");
        git(&wt, &["add", "work.txt"]);
        git(&wt, &["commit", "-q", "-m", "work"]);
    }
    write(&repo.join(".worktrees/dirty/notes.txt"), "not committed\n");
    write(&repo.join(".worktrees/open/target/CACHEDIR.TAG"), "tag\n");
    for d in ["target-idle", "target-busy", "target-fresh", "artifacts/x"] {
        write(&cache.join(d).join("CACHEDIR.TAG"), "tag\n");
    }
    write(&cache.join("compile/old.alcc"), "old\n");
    write(&cache.join("compile/new.alcc"), "new\n");
    write(&cache.join("scratch/old-task/log.txt"), "old\n");
    write(&cache.join("scratch/new-task/log.txt"), "new\n");
    let prs = tmp.path().join("prs.txt");
    std::fs::write(&prs, "merged MERGED\ndirty MERGED\nown CLOSED\nopen OPEN\n").unwrap();
    age(&repo.join(".worktrees"));
    for d in [
        "target-idle",
        "target-busy",
        "artifacts",
        "compile/old.alcc",
        "scratch/old-task",
    ] {
        age(&cache.join(d));
    }
    Machine {
        _tmp: tmp,
        repo,
        cache,
        prs,
    }
}

/// A process whose working folder is `dir`, killed on drop.
struct Busy(Child);
impl Busy {
    fn in_dir(dir: &Path) -> Busy {
        Busy(Command::new("sleep").arg("60").current_dir(dir).spawn().unwrap())
    }
}
impl Drop for Busy {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn a_dry_run_lists_but_deletes_nothing() {
    let m = machine();
    let out = m.run(&[]);
    assert!(out.contains("would delete  worktree merged"), "{out}");
    let idle = format!("would delete  target {}", m.cache.join("target-idle").display());
    assert!(out.contains(&idle), "{out}");
    assert!(out.contains("would delete 1 entries"), "{out}");
    assert!(out.contains("dry run: nothing deleted"), "{out}");
    assert!(m.wt("merged").exists());
    assert!(m.cache.join("target-idle").exists());
    assert!(m.cache.join("compile/old.alcc").exists());
    assert!(m.cache.join("scratch/old-task").exists());
}

#[test]
fn apply_deletes_only_finished_and_idle_folders() {
    let m = machine();
    let _busy = Busy::in_dir(&m.cache.join("target-busy"));
    let out = m.run(&["--apply"]);
    // Gone: the merged clean worktree, a worktree without commits of its own, idle targets, old entries.
    assert!(!m.wt("merged").exists(), "{out}");
    assert!(!m.wt("nopr-main").exists(), "{out}");
    assert!(!m.wt("open").join("target").exists(), "{out}");
    assert!(!m.cache.join("target-idle").exists(), "{out}");
    assert!(!m.cache.join("compile/old.alcc").exists(), "{out}");
    assert!(!m.cache.join("scratch/old-task").exists(), "{out}");
    // Kept: uncommitted work, unpushed commits, an open pull request, own commits without one.
    for name in ["dirty", "own", "open", "nopr-own"] {
        assert!(m.wt(name).exists(), "{name} was removed: {out}");
    }
    assert!(out.contains("worktree dirty: uncommitted work"), "{out}");
    assert!(
        out.contains("worktree own: commits that are on neither main nor its remote branch"),
        "{out}"
    );
    assert!(out.contains("worktree open: pull request OPEN"), "{out}");
    assert!(
        out.contains("worktree nopr-own: no pull request and commits not on main"),
        "{out}"
    );
    // Kept: a target a process uses, a recent target, the artifact cache, a recent entry and scratch folder.
    assert!(m.cache.join("target-busy").exists(), "{out}");
    assert!(m.cache.join("target-fresh").exists(), "{out}");
    assert!(m.cache.join("artifacts/x/CACHEDIR.TAG").exists(), "{out}");
    assert!(m.cache.join("compile/new.alcc").exists(), "{out}");
    assert!(m.cache.join("scratch/new-task").exists(), "{out}");
}

#[test]
fn without_pull_requests_no_worktree_goes() {
    let m = machine();
    std::fs::write(&m.prs, "").unwrap();
    let out = m.run(&["--apply"]);
    assert!(m.wt("merged").exists(), "{out}");
    assert!(m.wt("nopr-main").exists(), "{out}");
    assert!(out.contains("no pull requests read"), "{out}");
}
