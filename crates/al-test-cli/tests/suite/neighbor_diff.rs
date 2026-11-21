//! `tools/neighbor-diff.sh` (tests/neighbors/README.md). Its rule (`--plan`): a change of runtime code comes with a
//! folder under tests/neighbors/. Its table: `tools/compare_junit.py --entries FILE MAIN.xml HEAD.xml` is the comparison `tools/neighbor-diff.sh` prints: a
//! markdown table of every test whose outcome differs between a main binary and the head binary, with the entry the
//! author wrote for it (PINNED file:line, ASSUMED id, or REFUSED text). It exits 1 while a difference has no valid
//! entry, so an implementer cannot hand back a neighbor that changed without saying why.

use std::path::{Path, PathBuf};
use std::process::Output;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// One JUnit file of `al-runner --junit`, from `(name, body)` test cases (`body` is the inside of `<testcase>`).
fn junit(dir: &Path, file: &str, cases: &[(&str, &str)]) -> PathBuf {
    let mut s = String::from("<testsuites>\n  <testsuite name=\"P\" tests=\"1\">\n");
    for (name, body) in cases {
        s.push_str(&format!(
            "    <testcase classname=\"P\" name=\"{name}\" time=\"0.001\">{body}</testcase>\n"
        ));
    }
    s.push_str("  </testsuite>\n</testsuites>\n");
    let p = dir.join(file);
    std::fs::write(&p, s).unwrap();
    p
}

fn compare(dir: &Path, main: &Path, head: &Path, entries: &str) -> (i32, String) {
    let e = dir.join("entries.tsv");
    std::fs::write(&e, entries).unwrap();
    let o: Output = std::process::Command::new("python3")
        .arg(root().join("tools/compare_junit.py"))
        .arg("--entries")
        .arg(&e)
        .arg(main)
        .arg(head)
        .output()
        .expect("python3 runs");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    (o.status.code().unwrap_or(-1), text)
}

const REFUSED: &str = "<skipped message=\"First() on a part whose view is not pinned\" />";
const FLAGGED: &str = "<properties><property name=\"assumptions\" value=\"A17\" /></properties>";

#[test]
fn a_difference_without_an_entry_fails_and_shows_an_empty_entry() {
    let d = tempfile::tempdir().unwrap();
    let main = junit(d.path(), "m.xml", &[("N1", REFUSED), ("Same", "")]);
    let head = junit(d.path(), "h.xml", &[("N1", ""), ("Same", "")]);
    let (code, out) = compare(d.path(), &main, &head, "");
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("| test | main | head | entry |"), "{out}");
    assert!(
        out.contains("| P::N1 | not run: First() on a part whose view is not pinned | pass | MISSING |"),
        "{out}"
    );
    assert!(!out.contains("P::Same"), "{out}");
    assert!(out.contains("1 of 2 tests differ, 1 without an entry"), "{out}");
}

#[test]
fn each_kind_of_entry_lets_its_difference_through() {
    let d = tempfile::tempdir().unwrap();
    let main = junit(d.path(), "m.xml", &[("A", REFUSED), ("B", REFUSED), ("C", "")]);
    let head = junit(d.path(), "h.xml", &[("A", ""), ("B", FLAGGED), ("C", REFUSED)]);
    let entries = "# a comment\n\
        P::A\tPINNED tests/al-language/pages/PartRefill.al:120\n\
        P::B\tASSUMED A17 (corpus PR 1450)\n\
        P::C\tREFUSED First() on a part whose view is not pinned\n";
    let (code, out) = compare(d.path(), &main, &head, entries);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("| pass | PINNED tests/al-language/pages/PartRefill.al:120 |"),
        "{out}"
    );
    assert!(
        out.contains("| pass, flagged (A17) | ASSUMED A17 (corpus PR 1450) |"),
        "{out}"
    );
    assert!(out.contains("3 of 3 tests differ, 0 without an entry"), "{out}");
}

#[test]
fn an_entry_without_its_evidence_counts_as_missing() {
    let d = tempfile::tempdir().unwrap();
    let main = junit(d.path(), "m.xml", &[("A", REFUSED), ("B", REFUSED), ("C", REFUSED)]);
    let head = junit(d.path(), "h.xml", &[("A", ""), ("B", ""), ("C", "")]);
    // A PINNED entry needs a file:line, ASSUMED an id, REFUSED a text; free words are no entry.
    let entries = "P::A\tPINNED by the corpus\nP::B\tASSUMED\nP::C\tlooks right\n";
    let (code, out) = compare(d.path(), &main, &head, entries);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("3 of 3 tests differ, 3 without an entry"), "{out}");
}

#[test]
fn a_clean_pass_that_becomes_flagged_is_a_difference() {
    let d = tempfile::tempdir().unwrap();
    let main = junit(d.path(), "m.xml", &[("A", "")]);
    let head = junit(d.path(), "h.xml", &[("A", FLAGGED)]);
    let (code, out) = compare(d.path(), &main, &head, "");
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("| P::A | pass | pass, flagged (A17) | MISSING |"), "{out}");
}

#[test]
fn a_new_failure_text_differs_but_a_new_refusal_text_does_not() {
    let d = tempfile::tempdir().unwrap();
    let fail = |m: &str| format!("<failure message=\"{m}\" />");
    let (f1, f2) = (fail("Expected 1"), fail("Expected 2"));
    let main = junit(d.path(), "m.xml", &[("F", &f1), ("R", REFUSED)]);
    let head = junit(
        d.path(),
        "h.xml",
        &[("F", &f2), ("R", "<skipped message=\"another refusal\" />")],
    );
    let (code, out) = compare(d.path(), &main, &head, "");
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("| P::F | fail: Expected 1 | fail: Expected 2 | MISSING |"),
        "{out}"
    );
    assert!(!out.contains("P::R"), "{out}");
}

#[test]
fn a_test_on_one_side_only_is_a_difference() {
    let d = tempfile::tempdir().unwrap();
    let main = junit(d.path(), "m.xml", &[("A", "")]);
    let head = junit(d.path(), "h.xml", &[("A", ""), ("New", "")]);
    let (code, out) = compare(d.path(), &main, &head, "");
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("| P::New | missing | pass | MISSING |"), "{out}");
}

#[test]
fn a_bar_in_a_reason_does_not_break_the_table() {
    let d = tempfile::tempdir().unwrap();
    let main = junit(d.path(), "m.xml", &[("A", "<skipped message=\"a | b\" />")]);
    let head = junit(d.path(), "h.xml", &[("A", "")]);
    let (_, out) = compare(d.path(), &main, &head, "");
    assert!(out.contains("| not run: a \\| b |"), "{out}");
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

/// A repository whose first commit holds one runtime file and one test file; returns it and that commit.
fn repo() -> (tempfile::TempDir, String) {
    let d = tempfile::tempdir().unwrap();
    git(d.path(), &["init", "-q"]);
    git(d.path(), &["config", "user.email", "t@example.com"]);
    git(d.path(), &["config", "user.name", "t"]);
    write(d.path(), "crates/x/src/a.rs", "fn a() {}\n");
    write(d.path(), "crates/x/tests/t.rs", "fn t() {}\n");
    git(d.path(), &["add", "."]);
    git(d.path(), &["-c", "commit.gpgsign=false", "commit", "-q", "-m", "base"]);
    let base = git(d.path(), &["rev-parse", "HEAD"]);
    (d, base)
}

/// `neighbor-diff.sh --plan --base BASE` run in `dir`.
fn plan(dir: &Path, base: &str) -> (i32, String) {
    let o = std::process::Command::new("bash")
        .arg(root().join("tools/neighbor-diff.sh"))
        .args(["--plan", "--base", base])
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

#[test]
fn a_runtime_change_without_a_neighbor_folder_fails_the_rule() {
    let (d, base) = repo();
    write(d.path(), "crates/x/src/a.rs", "fn a() { let _ = 1; }\n");
    let (code, out) = plan(d.path(), &base);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("Runtime code changed (crates/x/src/a.rs"), "{out}");
}

#[test]
fn a_none_file_with_its_reason_satisfies_the_rule_and_an_empty_one_does_not() {
    let (d, base) = repo();
    write(d.path(), "crates/x/src/a.rs", "fn a() { let _ = 1; }\n");
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
fn a_neighbor_app_is_listed_and_a_folder_without_app_or_reason_fails() {
    let (d, base) = repo();
    write(d.path(), "crates/x/src/a.rs", "fn a() { let _ = 1; }\n");
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
    write(d.path(), "crates/x/tests/t.rs", "fn t() { let _ = 2; }\n");
    let (code, out) = plan(d.path(), &base);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("neighbor apps: 0"), "{out}");
}

#[test]
fn files_without_a_test_fail_instead_of_showing_no_difference() {
    let d = tempfile::tempdir().unwrap();
    let main = junit(d.path(), "m.xml", &[]);
    let head = junit(d.path(), "h.xml", &[]);
    let (code, out) = compare(d.path(), &main, &head, "");
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("neither file has a test"), "{out}");
}
