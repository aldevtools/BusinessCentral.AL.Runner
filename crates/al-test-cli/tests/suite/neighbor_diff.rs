//! The table of `tools/neighbor-diff.sh` (tests/neighbors/README.md): `tools/compare_junit.py --entries FILE MAIN.xml
//! HEAD.xml` prints every test whose outcome differs between a main binary and the head binary, with the entry the
//! author wrote for it (PINNED file:line, ASSUMED id, or REFUSED text). An entry counts only when it states the head
//! result and its evidence exists. It exits 1 while a difference has no such entry, or an entry has no difference, so
//! an implementer cannot hand back a neighbor that changed without saying why. The rule: `neighbor_rule.rs`.

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

/// A repository root with one corpus file of 200 lines and one assumption entry, the evidence the entries cite.
fn evidence(dir: &Path) -> PathBuf {
    let root = dir.join("root");
    let corpus = root.join("corpus/tests/al-language/pages");
    std::fs::create_dir_all(&corpus).unwrap();
    std::fs::write(corpus.join("PartRefill.al"), "x\n".repeat(200)).unwrap();
    let registry = root.join("crates/al-builtins/src/assumptions");
    std::fs::create_dir_all(&registry).unwrap();
    std::fs::write(registry.join("left-view-filter.rs"), "").unwrap();
    std::fs::write(registry.join("other-id.rs"), "").unwrap();
    root
}

fn compare(dir: &Path, main: &Path, head: &Path, entries: &str) -> (i32, String) {
    compare_on(dir, main, head, entries, &[])
}

fn compare_on(dir: &Path, main: &Path, head: &Path, entries: &str, extra: &[&str]) -> (i32, String) {
    let e = dir.join("entries.tsv");
    std::fs::write(&e, entries).unwrap();
    let o: Output = std::process::Command::new("python3")
        .arg(root().join("tools/compare_junit.py"))
        .arg("--root")
        .arg(evidence(dir))
        .args(extra)
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
const FLAGGED: &str = "<properties><property name=\"assumptions\" value=\"left-view-filter\" /></properties>";

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
        P::B\tASSUMED left-view-filter (corpus PR 1450)\n\
        P::C\tREFUSED First() on a part whose view is not pinned\n";
    let (code, out) = compare(d.path(), &main, &head, entries);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("| pass | PINNED tests/al-language/pages/PartRefill.al:120 |"),
        "{out}"
    );
    assert!(
        out.contains("| pass, flagged (left-view-filter) | ASSUMED left-view-filter (corpus PR 1450) |"),
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
    assert!(
        out.contains("| P::A | pass | pass, flagged (left-view-filter) | MISSING |"),
        "{out}"
    );
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

#[test]
fn files_without_a_test_fail_instead_of_showing_no_difference() {
    let d = tempfile::tempdir().unwrap();
    let main = junit(d.path(), "m.xml", &[]);
    let head = junit(d.path(), "h.xml", &[]);
    let (code, out) = compare(d.path(), &main, &head, "");
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("neither file has a test"), "{out}");
}

#[test]
fn a_refused_entry_needs_a_head_refusal_that_starts_with_its_text() {
    let d = tempfile::tempdir().unwrap();
    let main = junit(d.path(), "m.xml", &[("A", REFUSED), ("B", ""), ("C", "")]);
    let head = junit(d.path(), "h.xml", &[("A", ""), ("B", REFUSED), ("C", REFUSED)]);
    let entries = "P::A\tREFUSED First() on a part\n\
        P::B\tREFUSED another refusal\n\
        P::C\tREFUSED First() on a part\n";
    let (code, out) = compare(d.path(), &main, &head, entries);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("| P::A | not run: First() on a part whose view is not pinned | pass | MISSING |"),
        "{out}"
    );
    assert!(out.contains("3 of 3 tests differ, 2 without an entry"), "{out}");
}

#[test]
fn an_assumed_entry_needs_its_id_in_the_head_flags_and_in_the_registry() {
    let d = tempfile::tempdir().unwrap();
    let flagged = |id: &str| format!("<properties><property name=\"assumptions\" value=\"{id}\" /></properties>");
    let (nowhere, two) = (flagged("nowhere-id"), flagged("other-id,left-view-filter"));
    let main = junit(
        d.path(),
        "m.xml",
        &[("A", REFUSED), ("B", REFUSED), ("C", REFUSED), ("D", REFUSED)],
    );
    let head = junit(
        d.path(),
        "h.xml",
        &[("A", ""), ("B", FLAGGED), ("C", &nowhere), ("D", &two)],
    );
    let entries = "P::A\tASSUMED left-view-filter\n\
        P::B\tASSUMED other-id\n\
        P::C\tASSUMED nowhere-id\n\
        P::D\tASSUMED left-view-filter\n";
    let (code, out) = compare(d.path(), &main, &head, entries);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("| P::D |"), "{out}");
    assert!(out.contains("4 of 4 tests differ, 3 without an entry"), "{out}");
}

#[test]
fn a_pinned_entry_needs_a_clean_head_result_and_an_existing_corpus_line() {
    let d = tempfile::tempdir().unwrap();
    let main = junit(
        d.path(),
        "m.xml",
        &[
            ("A", REFUSED),
            ("B", REFUSED),
            ("C", REFUSED),
            ("D", ""),
            ("E", REFUSED),
        ],
    );
    let head = junit(
        d.path(),
        "h.xml",
        &[("A", FLAGGED), ("B", ""), ("C", ""), ("D", REFUSED), ("E", "")],
    );
    let entries = "P::A\tPINNED tests/al-language/pages/PartRefill.al:1\n\
        P::B\tPINNED nowhere.al:1\n\
        P::C\tPINNED tests/al-language/pages/PartRefill.al:201\n\
        P::D\tPINNED tests/al-language/pages/PartRefill.al:1\n\
        P::E\tPINNED tests/al-language/pages/PartRefill.al:200\n";
    let (code, out) = compare(d.path(), &main, &head, entries);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("| P::E | not run"), "{out}");
    assert!(out.contains("5 of 5 tests differ, 4 without an entry"), "{out}");
}

#[test]
fn an_entry_whose_test_shows_no_difference_is_stale_and_fails() {
    let d = tempfile::tempdir().unwrap();
    let main = junit(d.path(), "m.xml", &[("A", "")]);
    let head = junit(d.path(), "h.xml", &[("A", "")]);
    let (code, out) = compare(d.path(), &main, &head, "P::A\tREFUSED x\nP::Gone\tREFUSED y\n");
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("stale entry, no difference: P::A"), "{out}");
    assert!(out.contains("stale entry, no difference: P::Gone"), "{out}");
}

#[test]
fn an_entry_for_one_bc_major_counts_only_on_that_major() {
    let d = tempfile::tempdir().unwrap();
    let main = junit(d.path(), "m.xml", &[("A", REFUSED)]);
    let head = junit(d.path(), "h.xml", &[("A", "")]);
    let entries = "P::A @27\tPINNED tests/al-language/pages/PartRefill.al:1\n";
    let (code, out) = compare_on(d.path(), &main, &head, entries, &["--major", "27"]);
    assert_eq!(code, 0, "{out}");
    // On 29 the entry neither applies nor is stale: the difference has no entry.
    let (code, out) = compare_on(d.path(), &main, &head, entries, &["--major", "29"]);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("| P::A | not run: First() on a part whose view is not pinned | pass | MISSING |"),
        "{out}"
    );
    assert!(!out.contains("stale"), "{out}");
    let same = junit(d.path(), "s.xml", &[("A", REFUSED)]);
    let (code, out) = compare_on(d.path(), &main, &same, entries, &["--major", "27"]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("stale entry, no difference: P::A @27"), "{out}");
}
