//! The rule of `tools/neighbor-diff.sh --plan` (tests/neighbors/README.md): a change of a binary input
//! (`tools/binary-inputs.txt`) comes with a folder under tests/neighbors/. The same list decides which older main commit
//! has the same al-runner binary (`--same-inputs`).

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn git(dir: &Path, args: &[&str]) -> String {
    let o = std::process::Command::new("git")
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
    String::from_utf8_lossy(&o.stdout).trim().to_string()
}

fn write(dir: &Path, rel: &str, text: &str) {
    let p = dir.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, text).unwrap();
}

fn commit(dir: &Path, msg: &str) -> String {
    git(dir, &["add", "."]);
    git(dir, &["-c", "commit.gpgsign=false", "commit", "-q", "-m", msg]);
    git(dir, &["rev-parse", "HEAD"])
}

/// Files of the base commit: one runtime file, one test file, and one of each other kind of binary input.
const BASE_FILES: &[&str] = &[
    "crates/x/src/a.rs",
    "crates/x/tests/t.rs",
    "crates/x/build.rs",
    "crates/x/Cargo.toml",
    "Cargo.toml",
    "Cargo.lock",
    "docs/language-catalog.json",
    "tools/dotnet-sidecar/Dispatch.cs",
];

/// A repository whose first commit holds `BASE_FILES`; returns it and that commit.
fn repo() -> (tempfile::TempDir, String) {
    let d = tempfile::tempdir().unwrap();
    git(d.path(), &["init", "-q"]);
    git(d.path(), &["config", "user.email", "t@example.com"]);
    git(d.path(), &["config", "user.name", "t"]);
    for f in BASE_FILES {
        write(d.path(), f, "1\n");
    }
    let base = commit(d.path(), "base");
    (d, base)
}

/// `neighbor-diff.sh MODE --base BASE` run in `dir`.
fn run(dir: &Path, mode: &str, base: &str) -> (i32, String) {
    let o = std::process::Command::new("bash")
        .arg(root().join("tools/neighbor-diff.sh"))
        .args([mode, "--base", base])
        .current_dir(dir)
        .env_remove("GITHUB_OUTPUT")
        .output()
        .expect("bash runs");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    (o.status.code().unwrap_or(-1), text)
}

fn plan(dir: &Path, base: &str) -> (i32, String) {
    run(dir, "--plan", base)
}

#[test]
fn a_runtime_change_without_a_neighbor_folder_fails_the_rule() {
    let (d, base) = repo();
    write(d.path(), "crates/x/src/a.rs", "2\n");
    let (code, out) = plan(d.path(), &base);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("A binary input changed (crates/x/src/a.rs"), "{out}");
}

#[test]
fn a_change_of_any_other_binary_input_fails_the_rule_too() {
    for f in &BASE_FILES[2..] {
        let (d, base) = repo();
        write(d.path(), f, "2\n");
        let (code, out) = plan(d.path(), &base);
        assert_eq!(code, 1, "{f}: {out}");
        assert!(out.contains(&format!("A binary input changed ({f}")), "{out}");
    }
}

#[test]
fn a_none_file_with_its_reason_satisfies_the_rule_and_an_empty_one_does_not() {
    let (d, base) = repo();
    write(d.path(), "crates/x/src/a.rs", "2\n");
    write(d.path(), "tests/neighbors/topic/none.txt", "");
    let (code, out) = plan(d.path(), &base);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("tests/neighbors/topic: none.txt is empty"), "{out}");
    write(d.path(), "tests/neighbors/topic/none.txt", "only a log line changed\n");
    let (code, out) = plan(d.path(), &base);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("tests/neighbors/topic: no neighbor app. Reason: only a log line changed"),
        "{out}"
    );
    assert!(out.contains("neighbor apps: 0"), "{out}");
}

#[test]
fn a_folder_with_both_an_app_and_a_none_file_fails() {
    let (d, base) = repo();
    write(d.path(), "crates/x/src/a.rs", "2\n");
    write(d.path(), "tests/neighbors/topic/none.txt", "comments only\n");
    write(d.path(), "tests/neighbors/topic/app.json", "{}");
    let (code, out) = plan(d.path(), &base);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("tests/neighbors/topic: holds both app.json and none.txt"),
        "{out}"
    );
}

#[test]
fn a_neighbor_app_is_listed_and_a_folder_without_app_or_reason_fails() {
    let (d, base) = repo();
    write(d.path(), "crates/x/src/a.rs", "2\n");
    write(d.path(), "tests/neighbors/topic/app.json", "{}");
    write(d.path(), "tests/neighbors/topic/src/P.al", "");
    let (code, out) = plan(d.path(), &base);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("neighbor apps: 1"), "{out}");
    write(d.path(), "tests/neighbors/other/differences.tsv", "");
    let (code, out) = plan(d.path(), &base);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("tests/neighbors/other: holds neither app.json nor none.txt"),
        "{out}"
    );
}

#[test]
fn a_pure_rename_or_a_test_change_needs_no_neighbor_folder() {
    let (d, base) = repo();
    git(d.path(), &["mv", "crates/x/src/a.rs", "crates/x/src/b.rs"]);
    write(d.path(), "crates/x/tests/t.rs", "2\n");
    let (code, out) = plan(d.path(), &base);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("neighbor apps: 0"), "{out}");
}

#[test]
fn an_older_main_commit_has_the_same_binary_only_while_no_binary_input_changed() {
    let (d, a) = repo();
    write(d.path(), "crates/x/tests/t.rs", "2\n");
    let b = commit(d.path(), "test only");
    write(d.path(), "tools/dotnet-sidecar/Dispatch.cs", "2\n");
    let c = commit(d.path(), "sidecar only");
    write(d.path(), "crates/x/tests/t.rs", "3\n");
    let e = commit(d.path(), "test only");
    let (code, out) = run(d.path(), "--same-inputs", &e);
    assert_eq!(code, 0, "{out}");
    assert_eq!(out.trim(), format!("{e}\n{c}"), "{out}");
    let (_, out) = run(d.path(), "--same-inputs", &b);
    assert_eq!(out.trim(), format!("{b}\n{a}"), "{out}");
}

/// Every file a crate pulls in with `include_str!`, `include_bytes!` or `include!` is in the list, so a new include
/// that the list misses fails here.
#[test]
fn the_binary_input_list_covers_every_included_file() {
    let root = root();
    let mut specs = vec!["ls-files".to_string(), "--".to_string()];
    for line in std::fs::read_to_string(root.join("tools/binary-inputs.txt"))
        .unwrap()
        .lines()
    {
        if let Some(spec) = line.split_whitespace().next().filter(|s| !s.starts_with('#')) {
            specs.push(spec.to_string());
        }
    }
    let specs: Vec<&str> = specs.iter().map(String::as_str).collect();
    let covered: std::collections::BTreeSet<String> = git(&root, &specs).lines().map(str::to_string).collect();
    let mut included = Vec::new();
    let mut stack = vec![root.join("crates")];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir).unwrap() {
            let p = e.unwrap().path();
            let rel = p.strip_prefix(&root).unwrap().to_string_lossy().replace('\\', "/");
            if p.is_dir() {
                // A crate's tests are not built into a binary; everything else under crates/ may be.
                if !rel.ends_with("/tests") && !rel.ends_with("/target") {
                    stack.push(p);
                }
            } else if rel.ends_with(".rs") {
                let text = std::fs::read_to_string(&p).unwrap();
                for mac in ["include_str!(\"", "include_bytes!(\"", "include!(\""] {
                    for (i, _) in text.match_indices(mac) {
                        let rest = &text[i + mac.len()..];
                        let target = p.parent().unwrap().join(&rest[..rest.find('"').unwrap()]);
                        let target = target.canonicalize().unwrap_or(target);
                        let t = target.strip_prefix(&root).unwrap().to_string_lossy().replace('\\', "/");
                        included.push((rel.clone(), t));
                    }
                }
            }
        }
    }
    assert!(included.len() > 10, "{included:?}");
    let missing: Vec<_> = included.iter().filter(|(_, t)| !covered.contains(t)).collect();
    assert!(missing.is_empty(), "add these to tools/binary-inputs.txt: {missing:?}");
    for f in ["crates/al-builtins/build.rs", "Cargo.lock", "Cargo.toml"] {
        assert!(covered.contains(f), "{f}");
    }
}
