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

/// Like [`junit`], with the suite-level `baseline_assumptions` property of the Runner dialect.
fn junit_baseline(dir: &Path, file: &str, ids: &str, cases: &[(&str, &str)]) -> PathBuf {
    let p = junit(dir, file, cases);
    let s = std::fs::read_to_string(&p).unwrap().replace(
        "<testsuites>",
        &format!("<testsuites>\n  <properties><property name=\"baseline_assumptions\" value=\"{ids}\" /></properties>"),
    );
    std::fs::write(&p, s).unwrap();
    p
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
    // D carries a second id the entry does not name.
    assert!(out.contains("| P::D |"), "{out}");
    assert!(out.contains("4 of 4 tests differ, 4 without an entry"), "{out}");
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

/// A folder the pull request does not change (`--stale-note`, set by `tools/neighbor-diff.sh`) already has its
/// change on main: its stale entries print as a note and exit 0; an unexplained difference still fails.
#[test]
fn a_stale_entry_of_an_unchanged_folder_is_a_note_and_a_changed_folder_fails() {
    let d = tempfile::tempdir().unwrap();
    let main = junit(d.path(), "m.xml", &[("A", "")]);
    let head = junit(d.path(), "h.xml", &[("A", "")]);
    let (code, out) = compare_on(d.path(), &main, &head, "P::A\tREFUSED x\n", &["--stale-note"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("note: stale entry, no difference: P::A"), "{out}");
    let (code, out) = compare(d.path(), &main, &head, "P::A\tREFUSED x\n");
    assert_eq!(code, 1, "{out}");
    let head = junit(d.path(), "h2.xml", &[("A", "<failure message=\"boom\"/>")]);
    let (code, out) = compare_on(d.path(), &main, &head, "", &["--stale-note"]);
    assert_eq!(code, 1, "{out}");
}

/// A third column names the BC version a row applies to: a major (`27`) or a major.minor (`29.1`), matched against the
/// build of the run. A row for another version neither counts nor is stale.
#[test]
fn an_entry_with_a_version_counts_only_on_runs_of_that_version() {
    let d = tempfile::tempdir().unwrap();
    let main = junit(d.path(), "m.xml", &[("A", REFUSED), ("B", "")]);
    let head = junit(d.path(), "h.xml", &[("A", ""), ("B", "")]);
    let same = junit(d.path(), "s.xml", &[("A", REFUSED), ("B", "")]);
    let pinned = "PINNED tests/al-language/pages/PartRefill.al:1";
    for (version, applies, other) in [
        ("27", "27.5.1234.0", "29.0.54011.56262"),
        ("29.1", "29.1.100.0", "29.0.54011.56262"),
        ("29.1", "29.1.100.0", "29.10.1.0"),
    ] {
        let entries = format!("P::A\t{pinned}\t{version}\n");
        let (code, out) = compare_on(d.path(), &main, &head, &entries, &["--build", applies]);
        assert_eq!(code, 0, "{version} on {applies}: {out}");
        // On another version the row neither applies nor is stale: the difference has no entry.
        let (code, out) = compare_on(d.path(), &main, &head, &entries, &["--build", other]);
        assert_eq!(code, 1, "{out}");
        assert!(
            out.contains("| P::A | not run: First() on a part whose view is not pinned | pass | MISSING |"),
            "{out}"
        );
        assert!(!out.contains("stale"), "{out}");
        // Where nothing differs on another version, its row is not stale either.
        let (code, out) = compare_on(d.path(), &main, &same, &entries, &["--build", other]);
        assert_eq!(code, 0, "{out}");
        let (code, out) = compare_on(d.path(), &main, &same, &entries, &["--build", applies]);
        assert_eq!(code, 1, "{out}");
        assert!(
            out.contains(&format!("stale entry, no difference: P::A ({version})")),
            "{out}"
        );
    }
}

#[test]
fn a_side_whose_tests_hit_a_resource_limit_fails_with_the_reason() {
    for reason in [
        "memory limit (--max-memory 3072 MB)",
        "the test used more than the time limit of 60 s",
    ] {
        let d = tempfile::tempdir().unwrap();
        let limit = format!("<skipped message=\"{reason}\" />");
        let main = junit(d.path(), "m.xml", &[("A", ""), ("B", "")]);
        let head = junit(d.path(), "h.xml", &[("A", ""), ("B", &limit)]);
        let entries = "P::B\tREFUSED memory limit\n";
        let (code, out) = compare(d.path(), &main, &head, entries);
        assert_eq!(code, 1, "{out}");
        assert!(
            out.contains(&format!(
                "head: 1 test not run for a resource limit, first P::B: {reason}"
            )),
            "{out}"
        );
    }
}

#[test]
fn a_run_where_no_test_ran_on_either_side_fails() {
    let d = tempfile::tempdir().unwrap();
    let other = "<skipped message=\"codeunit 50100 does not compile\" />";
    let main = junit(d.path(), "m.xml", &[("A", other), ("B", other)]);
    let head = junit(d.path(), "h.xml", &[("A", other), ("B", other)]);
    let (code, out) = compare(d.path(), &main, &head, "");
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("no test ran on either side, first P::A: codeunit 50100 does not compile"),
        "{out}"
    );
}

/// The resource reasons `compare_junit.py` knows are the ones the runtime writes; a changed text fails here.
#[test]
fn the_resource_reasons_match_the_runtime_texts() {
    let text = |p: &str| std::fs::read_to_string(root().join(p)).unwrap();
    let memory = text("crates/al-testrun/src/runtime/memory.rs");
    let run = text("crates/al-testrun/src/runtime/run.rs");
    let script = text("tools/compare_junit.py");
    assert!(memory.contains("pub const REASON: &str = \"memory limit (\";"));
    assert!(run.contains("const TIME_LIMIT_REASON: &str = \"the test used more than the time limit of \";"));
    for r in ["\"memory limit (\"", "\"the test used more than the time limit of \""] {
        assert!(script.contains(r), "{r}");
    }
}

#[test]
fn a_pinned_path_must_stay_under_the_corpus() {
    let d = tempfile::tempdir().unwrap();
    let abs = d.path().join("elsewhere.al");
    std::fs::write(&abs, "x\n").unwrap();
    let main = junit(
        d.path(),
        "m.xml",
        &[("A", REFUSED), ("B", REFUSED), ("C", REFUSED), ("Z", "")],
    );
    let head = junit(d.path(), "h.xml", &[("A", ""), ("B", ""), ("C", ""), ("Z", "")]);
    let entries = format!(
        "P::A\tPINNED {}:1\nP::B\tPINNED /etc/hostname:1\nP::C\tPINNED tests//al-language/pages/PartRefill.al:1\n",
        abs.display()
    );
    let (code, out) = compare(d.path(), &main, &head, &entries);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("3 of 4 tests differ, 3 without an entry"), "{out}");
}

#[test]
fn a_pinned_symlink_out_of_the_corpus_is_missing() {
    let d = tempfile::tempdir().unwrap();
    let outside = d.path().join("outside.al");
    std::fs::write(&outside, "x\n").unwrap();
    let main = junit(d.path(), "m.xml", &[("A", REFUSED), ("Z", "")]);
    let head = junit(d.path(), "h.xml", &[("A", ""), ("Z", "")]);
    // evidence() is built by compare(); build the link after it in a second root run.
    let root = evidence(d.path());
    std::os::unix::fs::symlink(&outside, root.join("corpus/link.al")).unwrap();
    let (code, out) = compare(d.path(), &main, &head, "P::A\tPINNED link.al:1\n");
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("1 of 2 tests differ, 1 without an entry"), "{out}");
}

#[test]
fn a_baseline_that_newly_runs_through_an_assumption_is_a_difference() {
    let d = tempfile::tempdir().unwrap();
    let main = junit(d.path(), "m.xml", &[("A", "")]);
    let head = junit_baseline(d.path(), "h.xml", "left-view-filter", &[("A", "")]);
    let (code, out) = compare(d.path(), &main, &head, "");
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("| <baseline> | missing | pass, flagged (left-view-filter) | MISSING |"),
        "{out}"
    );
    let (code, out) = compare(
        d.path(),
        &main,
        &head,
        "<baseline>\tASSUMED left-view-filter (corpus PR 1)\n",
    );
    assert_eq!(code, 0, "{out}");
    let same = junit_baseline(d.path(), "s.xml", "left-view-filter", &[("A", "")]);
    let (code, out) = compare(d.path(), &head, &same, "");
    assert_eq!(code, 0, "{out}");
}

#[test]
fn a_malformed_entry_row_is_an_error() {
    let d = tempfile::tempdir().unwrap();
    let main = junit(d.path(), "m.xml", &[("A", REFUSED)]);
    let head = junit(d.path(), "h.xml", &[("A", "")]);
    for rows in [
        "P::A\tREFUSED x\t29\textra\n",
        "P::A\tREFUSED x\t92x\n",
        "P::A\tREFUSED x\t30\n",
        "P::A\n",
    ] {
        let (code, out) = compare(d.path(), &main, &head, rows);
        assert_eq!(code, 1, "{rows}: {out}");
        assert!(out.contains("error in the entries file"), "{rows}: {out}");
    }
}

#[test]
fn a_duplicate_entry_key_is_an_error() {
    let d = tempfile::tempdir().unwrap();
    let main = junit(d.path(), "m.xml", &[("A", REFUSED)]);
    let head = junit(d.path(), "h.xml", &[("A", "")]);
    let pinned = "PINNED tests/al-language/pages/PartRefill.al:1";
    let rows = format!("P::A\t{pinned}\t29\nP::A\t{pinned}\n");
    let (code, out) = compare_on(d.path(), &main, &head, &rows, &["--build", "29.0.1.0"]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("error in the entries file: duplicate key P::A"), "{out}");
    // Rows for different versions are not duplicates.
    let rows = format!("P::A\t{pinned}\t29\nP::A\t{pinned}\t28\n");
    let (code, out) = compare_on(d.path(), &main, &head, &rows, &["--build", "29.0.1.0"]);
    assert_eq!(code, 0, "{out}");
}

#[test]
fn an_assumed_entry_names_every_new_id_of_the_head() {
    let d = tempfile::tempdir().unwrap();
    let two = "<properties><property name=\"assumptions\" value=\"left-view-filter,other-id\" /></properties>";
    let main = junit(d.path(), "m.xml", &[("A", REFUSED), ("B", FLAGGED), ("Z", "")]);
    let head = junit(d.path(), "h.xml", &[("A", two), ("B", two), ("Z", "")]);
    let (code, out) = compare(
        d.path(),
        &main,
        &head,
        "P::A\tASSUMED left-view-filter\nP::B\tASSUMED other-id\n",
    );
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("2 of 3 tests differ, 1 without an entry"), "{out}");
    let (code, out) = compare(
        d.path(),
        &main,
        &head,
        "P::A\tASSUMED left-view-filter,other-id\nP::B\tASSUMED other-id\n",
    );
    assert_eq!(code, 0, "{out}");
}

#[test]
fn a_run_where_nothing_passed_on_either_side_fails() {
    let d = tempfile::tempdir().unwrap();
    let f = "<failure message=\"the app does not install\" />";
    let main = junit(d.path(), "m.xml", &[("A", f), ("B", f)]);
    let head = junit(d.path(), "h.xml", &[("A", f), ("B", f)]);
    let (code, out) = compare(d.path(), &main, &head, "");
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("nothing passed on either side, first P::A: the app does not install"),
        "{out}"
    );
}

/// `deps/<name>/` of a neighbor app is packed at run time into `<publisher>_<name>_<version>.Source.zip` with
/// `app.json` at the archive root, so no binary is committed (README, "Dependency apps").
#[test]
fn a_dependency_folder_is_packed_into_a_source_package() {
    let d = tempfile::tempdir().unwrap();
    let dep = d.path().join("app/deps/Lib");
    std::fs::create_dir_all(dep.join("src")).unwrap();
    std::fs::write(
        dep.join("app.json"),
        "{\"id\":\"1\",\"name\":\"Lib\",\"publisher\":\"Me Inc\",\"version\":\"1.0.0.0\"}",
    )
    .unwrap();
    std::fs::write(dep.join("src/a.al"), "codeunit 50100 X { }\n").unwrap();
    let out = d.path().join("out");
    let o = std::process::Command::new("python3")
        .arg(root().join("tools/pack_deps.py"))
        .arg(d.path().join("app"))
        .arg(&out)
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert_eq!(String::from_utf8_lossy(&o.stdout).trim(), "1");
    let list = std::process::Command::new("python3")
        .args(["-m", "zipfile", "-l"])
        .arg(out.join("Me_Inc_Lib_1.0.0.0.Source.zip"))
        .output()
        .unwrap();
    let list = String::from_utf8_lossy(&list.stdout);
    assert!(list.contains("app.json") && list.contains("src/a.al"), "{list}");
    let script = std::fs::read_to_string(root().join("tools/neighbor-diff.sh")).unwrap();
    assert!(
        script.contains("--dep \"$out/$name.packages\""),
        "the script passes the packages to both runs"
    );
}

/// `neighbor-diff.sh --stale-mode APP` says "note" only for a folder inside the repository (physical path) that git
/// reports unchanged; an app outside the repository, or one reached through a symlink, is judged by its real path.
#[test]
fn the_stale_note_needs_a_folder_inside_the_repository_that_git_reports_unchanged() {
    let mode = |app: &Path| -> String {
        let o = std::process::Command::new("bash")
            .arg(root().join("tools/neighbor-diff.sh"))
            .args(["--base", "HEAD", "--stale-mode"])
            .arg(app)
            .current_dir(root())
            .output()
            .unwrap();
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        String::from_utf8(o.stdout).unwrap().trim().to_string()
    };
    let d = tempfile::tempdir().unwrap();
    let outside = d.path().join("probe");
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("differences.tsv"), "P::A\tREFUSED x\n").unwrap();
    assert_eq!(mode(&outside), "strict");
    let inside = std::fs::read_dir(root().join("tests/neighbors"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.is_dir())
        .expect("a neighbor folder");
    let link = d.path().join("link");
    std::os::unix::fs::symlink(&inside, &link).unwrap();
    // The symlink resolves to the same folder, so both get the same answer, never a path git cannot place.
    assert_eq!(mode(&link), mode(&inside));
}
