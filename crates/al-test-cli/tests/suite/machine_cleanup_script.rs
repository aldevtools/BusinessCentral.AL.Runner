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

    /// The script with the cache chosen by `ALRUNNER_CACHE`; `envs` adds or replaces variables, `unset` removes them.
    fn output(&self, extra: &[&str], envs: &[(&str, &str)], unset: &[&str]) -> Output {
        let mut c = Command::new("bash");
        c.arg(root().join("tools/machine_cleanup.sh"))
            .arg("--prs")
            .arg(&self.prs)
            .args(extra)
            .current_dir(&self.repo)
            .env("ALRUNNER_CACHE", &self.cache)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1");
        for k in unset {
            c.env_remove(k);
        }
        c.envs(envs.iter().copied());
        c.output().expect("bash runs")
    }

    fn run(&self, extra: &[&str]) -> String {
        let o = self.output(extra, &[], &[]);
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
    write(&repo.join(".gitignore"), "target/\n");
    git(&repo, &["add", "README", ".gitignore"]);
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
    write(&repo.join(".worktrees/dirty/target/CACHEDIR.TAG"), "tag\n");
    write(&repo.join(".worktrees/merged/target/CACHEDIR.TAG"), "tag\n");
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
    // The target of a worktree that is kept goes when its pull request is finished; with an open pull request it stays.
    assert!(!m.wt("dirty").join("target").exists(), "{out}");
    assert!(m.wt("open").join("target/CACHEDIR.TAG").exists(), "{out}");
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

#[test]
fn a_process_in_the_worktree_root_protects_its_target() {
    // A cargo build in a worktree has the worktree root as working folder, not the target folder.
    let m = machine();
    let _busy = Busy::in_dir(&m.wt("dirty"));
    let out = m.run(&["--apply", "--keep-hours", "0"]);
    assert!(m.wt("dirty").join("target/CACHEDIR.TAG").exists(), "{out}");
    assert!(out.contains("a running process uses it"), "{out}");
}

#[test]
fn the_target_of_an_open_pull_request_stays() {
    let m = machine();
    let out = m.run(&["--apply", "--keep-hours", "0"]);
    assert!(m.wt("open").join("target/CACHEDIR.TAG").exists(), "{out}");
    assert!(out.contains("pull request OPEN"), "{out}");
}

#[test]
fn without_a_read_pull_request_state_no_worktree_target_goes() {
    // No pull request state read (an empty list, as when `gh` fails): an aged target of an open pull request stays.
    let m = machine();
    std::fs::write(&m.prs, "").unwrap();
    let o = m.output(&["--apply", "--keep-hours", "0", "--no-gh"], &[], &[]);
    let out = String::from_utf8_lossy(&o.stdout).to_string();
    assert!(m.wt("open").join("target/CACHEDIR.TAG").exists(), "{out}");
    assert!(m.wt("merged").join("target/CACHEDIR.TAG").exists(), "{out}");
    assert!(out.contains("its pull request is not known to be finished"), "{out}");
    // Cache targets are not worktree targets: they still go.
    assert!(!m.cache.join("target-idle").exists(), "{out}");
}

#[test]
fn the_target_of_a_locked_worktree_stays() {
    let m = machine();
    git(&m.repo, &["worktree", "lock", m.wt("merged").to_str().unwrap()]);
    let out = m.run(&["--apply", "--keep-hours", "0"]);
    assert!(m.wt("merged").join("target/CACHEDIR.TAG").exists(), "{out}");
}

/// Adds a worktree `sub` whose `corpus` submodule has a local commit on a branch nobody pushed, and is checked out
/// again at the pinned commit (parent and submodule status are clean).
fn add_submodule_worktree(m: &Machine, stash: bool) {
    add_submodule_worktree_at(m, stash, false)
}

/// With `deep`, the unpushed commit is in a submodule of the submodule (`corpus/deep`) instead of in `corpus`.
fn add_submodule_worktree_at(m: &Machine, stash: bool, deep: bool) {
    let tmp = m._tmp.path();
    let sub = tmp.join("sub.git");
    std::fs::create_dir_all(&sub).unwrap();
    git(&sub, &["init", "-q", "--bare", "-b", "main"]);
    git(tmp, &["clone", "-q", sub.to_str().unwrap(), "subwork"]);
    write(&tmp.join("subwork/a.txt"), "a\n");
    git(&tmp.join("subwork"), &["add", "a.txt"]);
    git(&tmp.join("subwork"), &["commit", "-q", "-m", "a"]);
    git(&tmp.join("subwork"), &["push", "-q", "origin", "HEAD:main"]);
    let allow = ["-c", "protocol.file.allow=always"];
    if deep {
        let inner = tmp.join("inner.git");
        std::fs::create_dir_all(&inner).unwrap();
        git(&inner, &["init", "-q", "--bare", "-b", "main"]);
        git(tmp, &["clone", "-q", inner.to_str().unwrap(), "innerwork"]);
        write(&tmp.join("innerwork/i.txt"), "i\n");
        git(&tmp.join("innerwork"), &["add", "i.txt"]);
        git(&tmp.join("innerwork"), &["commit", "-q", "-m", "i"]);
        git(&tmp.join("innerwork"), &["push", "-q", "origin", "HEAD:main"]);
        let mut add = allow.to_vec();
        add.extend(["submodule", "add", "-q", inner.to_str().unwrap(), "deep"]);
        git(&tmp.join("subwork"), &add);
        git(&tmp.join("subwork"), &["commit", "-q", "-m", "deep"]);
        git(&tmp.join("subwork"), &["push", "-q", "origin", "HEAD:main"]);
    }
    let mut add = allow.to_vec();
    add.extend(["submodule", "add", "-q", sub.to_str().unwrap(), "corpus"]);
    git(&m.repo, &add);
    git(&m.repo, &["commit", "-q", "-m", "submodule"]);
    git(&m.repo, &["push", "-q", "origin", "HEAD:main"]);
    git(&m.repo, &["fetch", "-q", "origin"]);
    git(
        &m.repo,
        &["worktree", "add", "-q", ".worktrees/sub", "-b", "sub", "origin/main"],
    );
    let wt = m.wt("sub");
    let mut upd = allow.to_vec();
    upd.extend(["submodule", "update", "--init", "--recursive", "-q"]);
    git(&wt, &upd);
    let c = if deep {
        wt.join("corpus/deep")
    } else {
        wt.join("corpus")
    };
    if stash {
        write(&c.join("a.txt"), "changed\n");
        git(&c, &["stash", "push", "-q"]);
    } else {
        git(&c, &["checkout", "-q", "-b", "probe"]);
        write(&c.join("probe.txt"), "probe\n");
        git(&c, &["add", "probe.txt"]);
        git(&c, &["commit", "-q", "-m", "probe"]);
        git(&c, &["checkout", "-q", "--detach", "origin/main"]);
    }
    std::fs::write(
        &m.prs,
        format!("{}sub MERGED\n", std::fs::read_to_string(&m.prs).unwrap()),
    )
    .unwrap();
    age(&wt);
}

#[test]
fn unpushed_commits_in_a_submodule_keep_the_worktree() {
    let m = machine();
    add_submodule_worktree(&m, false);
    let out = m.run(&["--apply"]);
    assert!(m.wt("sub").exists(), "{out}");
    assert!(
        out.contains("worktree sub: uncommitted or unpushed work in a submodule"),
        "{out}"
    );
}

#[test]
fn unpushed_commits_in_a_nested_submodule_keep_the_worktree() {
    let m = machine();
    add_submodule_worktree_at(&m, false, true);
    let out = m.run(&["--apply"]);
    assert!(m.wt("sub").exists(), "{out}");
    assert!(
        out.contains("worktree sub: uncommitted or unpushed work in a submodule"),
        "{out}"
    );
}

#[test]
fn a_submodule_check_that_fails_keeps_the_worktree() {
    // The submodule's git folder is unreadable: the check cannot say the submodule is clean, so the worktree stays.
    let m = machine();
    add_submodule_worktree_at(&m, true, false);
    git(&m.wt("sub").join("corpus"), &["stash", "drop", "-q"]);
    let gitdir = std::fs::read_to_string(m.wt("sub").join("corpus/.git")).unwrap();
    let gitdir = m
        .wt("sub")
        .join("corpus")
        .join(gitdir.trim().trim_start_matches("gitdir: "));
    std::fs::write(gitdir.join("HEAD"), "garbage\n").unwrap();
    let o = m.output(&["--apply"], &[], &[]);
    let out = String::from_utf8_lossy(&o.stdout).to_string();
    assert!(m.wt("sub").exists(), "{out}");
    assert!(out.contains("worktree sub: submodules could not be checked"), "{out}");
}

#[test]
fn a_stash_in_a_submodule_keeps_the_worktree() {
    let m = machine();
    add_submodule_worktree(&m, true);
    let out = m.run(&["--apply"]);
    assert!(m.wt("sub").exists(), "{out}");
    assert!(
        out.contains("worktree sub: uncommitted or unpushed work in a submodule"),
        "{out}"
    );
}

#[test]
fn untracked_files_count_whatever_the_status_setting() {
    let m = machine();
    git(&m.repo, &["config", "status.showUntrackedFiles", "no"]);
    let out = m.run(&["--apply"]);
    assert!(m.wt("dirty").join("notes.txt").exists(), "{out}");
    assert!(out.contains("worktree dirty: uncommitted work"), "{out}");
}

#[test]
fn a_locked_worktree_is_left_alone() {
    let m = machine();
    git(&m.repo, &["worktree", "lock", ".worktrees/merged"]);
    let out = m.run(&["--apply"]);
    assert!(m.wt("merged").exists(), "{out}");
    assert!(out.contains("worktree merged: locked"), "{out}");
}

#[test]
fn a_second_run_at_the_same_time_does_nothing() {
    use std::io::{BufRead, BufReader};
    use std::process::Stdio;
    let m = machine();
    std::fs::create_dir_all(&m.cache).unwrap();
    let lock = m.cache.join(".machine_cleanup.lock");
    let mut holder = Command::new("bash")
        .arg("-c")
        .arg(format!(
            "exec 9>'{}'; flock -n 9 && echo held && sleep 60",
            lock.display()
        ))
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(holder.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    assert_eq!(line.trim(), "held");
    let out = m.run(&["--apply"]);
    let _ = holder.kill();
    let _ = holder.wait();
    assert!(out.contains("already running"), "{out}");
    assert!(m.wt("merged").exists(), "{out}");
}

#[test]
fn an_entry_unused_for_three_and_a_half_days_goes_with_three_days() {
    let m = machine();
    write(&m.cache.join("compile/mid.alcc"), "mid\n");
    let o = Command::new("touch")
        .args(["-d", "84 hours ago"])
        .arg(m.cache.join("compile/mid.alcc"))
        .output()
        .unwrap();
    assert!(o.status.success());
    let out = m.run(&["--apply"]);
    assert!(!m.cache.join("compile/mid.alcc").exists(), "{out}");
    assert!(m.cache.join("compile/new.alcc").exists(), "{out}");
}

#[test]
fn the_cache_root_follows_the_runtime_order() {
    // Whitespace-only ALRUNNER_CACHE counts as unset; AL_RUNTIME_CACHE comes next (crates/al-deps/src/cache.rs).
    let m = machine();
    let other = m._tmp.path().join("other");
    write(&other.join("scratch/old-task/log.txt"), "old\n");
    age(&other.join("scratch"));
    let o = m.output(
        &["--apply"],
        &[("ALRUNNER_CACHE", "  "), ("AL_RUNTIME_CACHE", other.to_str().unwrap())],
        &[],
    );
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(o.status.success(), "{out}");
    assert!(!other.join("scratch/old-task").exists(), "{out}");
    assert!(m.cache.join("scratch/old-task").exists(), "{out}");
    // AL_RUNNER_CACHE_ROOT has /v3 appended.
    let base = m._tmp.path().join("base");
    write(&base.join("v3/scratch/old-task/log.txt"), "old\n");
    age(&base.join("v3/scratch"));
    let o = m.output(
        &["--apply"],
        &[("AL_RUNNER_CACHE_ROOT", base.to_str().unwrap())],
        &["ALRUNNER_CACHE", "AL_RUNTIME_CACHE"],
    );
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(o.status.success(), "{out}");
    assert!(!base.join("v3/scratch/old-task").exists(), "{out}");
}

#[test]
fn a_cache_root_that_is_home_or_slash_is_refused() {
    let m = machine();
    let home = m._tmp.path().join("home");
    write(&home.join("work/target/CACHEDIR.TAG"), "tag\n");
    age(&home.join("work"));
    for bad in [home.to_str().unwrap(), "/"] {
        let o = m.output(
            &["--apply"],
            &[("ALRUNNER_CACHE", bad), ("HOME", home.to_str().unwrap())],
            &[],
        );
        assert!(!o.status.success(), "{bad}: {}", String::from_utf8_lossy(&o.stdout));
        assert!(home.join("work/target/CACHEDIR.TAG").exists());
    }
}

fn disk_floor(args: &[&str], path_prefix: Option<&Path>) -> Output {
    let mut c = Command::new("bash");
    c.arg(root().join("tools/disk_floor.sh")).args(args).current_dir(root());
    let tmp = tempfile::tempdir().unwrap();
    c.env("ALRUNNER_CACHE", tmp.path().join("cache"));
    if let Some(p) = path_prefix {
        c.env("PATH", format!("{}:{}", p.display(), std::env::var("PATH").unwrap()));
    }
    c.output().unwrap()
}

#[test]
fn disk_floor_passes_and_fails_on_the_numbers() {
    assert!(disk_floor(&["0"], None).status.success());
    assert!(!disk_floor(&["999999999"], None).status.success());
}

#[test]
fn disk_floor_fails_when_the_floor_is_not_a_number() {
    assert!(!disk_floor(&["lots"], None).status.success());
}

#[test]
fn disk_floor_fails_when_df_fails() {
    let bin = tempfile::tempdir().unwrap();
    let df = bin.path().join("df");
    std::fs::write(&df, "#!/bin/sh\nexit 1\n").unwrap();
    Command::new("chmod").arg("+x").arg(&df).status().unwrap();
    let o = disk_floor(&["0"], Some(bin.path()));
    assert!(!o.status.success(), "{}", String::from_utf8_lossy(&o.stdout));
}
