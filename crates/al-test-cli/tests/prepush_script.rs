//! `tools/prepush.sh`: the fast checks a runtime agent runs before every push. Its `--dry-run`
//! prints the commands it would run, one per line, with the changed files read from stdin
//! (`--files -`) instead of git, so the choice of crates for clippy is tested without a build.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// The commands `tools/prepush.sh --dry-run` prints for these changed files.
fn plan(files: &[&str]) -> Vec<String> {
    let mut child = Command::new("bash")
        .arg(root().join("tools/prepush.sh"))
        .args(["--dry-run", "--files", "-"])
        .current_dir(root())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(files.join("\n").as_bytes()).unwrap();
    drop(stdin);
    let o = child.wait_with_output().unwrap();
    assert!(o.status.success(), "prepush.sh: {}", String::from_utf8_lossy(&o.stderr));
    String::from_utf8_lossy(&o.stdout).lines().map(str::to_string).collect()
}

const FIXED: [&str; 4] = [
    "cargo fmt --all --check",
    "cargo test -p al-version --test no_version_literals --test gate_completeness -q",
    "cargo test -p al-builtins --lib -q assumptions",
    "cargo test -p al-builtins --test builtin_ids_current -q",
];

#[test]
fn clippy_covers_only_the_changed_crates() {
    let p = plan(&[
        "crates/al-vm/src/runtime.rs",
        "crates/al-sema/src/lower.rs",
        "crates/al-vm/tests/store/main.rs",
        "docs/status.md",
    ]);
    let mut want: Vec<String> = FIXED.iter().map(|s| s.to_string()).collect();
    want.push("cargo clippy -p al-sema -p al-vm --all-targets -- -D warnings".into());
    assert_eq!(p, want);
}

#[test]
fn no_changed_crate_means_no_clippy() {
    let p = plan(&["docs/status.md", "tools/inventory.py"]);
    assert_eq!(p, FIXED.iter().map(|s| s.to_string()).collect::<Vec<_>>());
}

#[test]
fn a_workspace_file_runs_clippy_on_the_whole_workspace() {
    let p = plan(&["Cargo.lock", "crates/al-vm/src/runtime.rs"]);
    assert_eq!(
        p.last().unwrap(),
        "cargo clippy --workspace --all-targets -- -D warnings"
    );
}

#[test]
fn a_deleted_crate_is_not_passed_to_clippy() {
    let p = plan(&["crates/no-such-crate/src/lib.rs"]);
    assert_eq!(p, FIXED.iter().map(|s| s.to_string()).collect::<Vec<_>>());
}
