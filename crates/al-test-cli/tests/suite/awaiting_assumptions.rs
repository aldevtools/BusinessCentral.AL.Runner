//! `--awaiting-assumptions`: an assumption that waits for a corpus pin fails the check once the pin is in the
//! pinned corpus. The first test is the check itself over the real registry and the pinned corpus (cheap while no
//! awaited file has landed: nothing is compiled); the others inject an entry through `AL_AWAITING_EXTRA`.

use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// A corpus file of a small test codeunit (60184) that needs no Microsoft app, and one that does not exist.
const PRESENT: &str = "tests/al-language/types/TestDurationContracts.al";
const MISSING: &str = "tests/al-language/types/NoSuchAwaitedFile.al";

fn cli(args: &[&str], extra: Option<&str>) -> (i32, String, String) {
    assert!(
        root().join("corpus").join(PRESENT).exists(),
        "corpus submodule not checked out: run `git submodule update --init`"
    );
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_al-test-cli"));
    cmd.arg("--root").arg(root()).arg("--no-deps").args(args);
    match extra {
        Some(e) => cmd.env("AL_AWAITING_EXTRA", e),
        None => cmd.env_remove("AL_AWAITING_EXTRA"),
    };
    let out = cmd.output().unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into(),
        String::from_utf8_lossy(&out.stderr).into(),
    )
}

/// The check over the real registry: no assumption may outlive the corpus pin that replaces it.
#[test]
fn no_assumption_outlives_its_corpus_pin() {
    let (code, out, err) = cli(&["--awaiting-assumptions"], None);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn a_landed_pin_that_passes_says_remove_the_assumption() {
    let extra = format!("code-coverage-snapshot-while-logging;5806;2026-10-20;{PRESENT}");
    let (code, out, err) = cli(&["--awaiting-assumptions"], Some(&extra));
    assert_eq!(code, 1, "{out}{err}");
    assert!(
        out.contains("assumption `code-coverage-snapshot-while-logging` (al-runner #5806, waiting since 2026-10-20): the pin landed: remove the assumption and cite the pin"),
        "{out}{err}"
    );
}

#[test]
fn a_partly_landed_pin_is_a_warning() {
    let extra = format!("code-coverage-snapshot-while-logging;5806;2026-10-20;{PRESENT},{MISSING}");
    let (code, out, err) = cli(&["--awaiting-assumptions"], Some(&extra));
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        err.contains("warning: assumption `code-coverage-snapshot-while-logging`"),
        "{out}{err}"
    );
    assert!(err.contains(&format!("still missing: {MISSING}")), "{out}{err}");
}

#[test]
fn the_summary_is_one_line_oldest_first() {
    // Dates older than any real entry, so that real entries do not come between them.
    let extra = format!(
        "code-coverage-snapshot-while-logging;5806;2020-01-02;{MISSING}|query-closed-after-last-row;5807;2020-01-01;{PRESENT}"
    );
    let (code, out, err) = cli(&["--awaiting-assumptions", "--summary"], Some(&extra));
    assert_eq!(code, 0, "{out}{err}");
    let line = out
        .lines()
        .find(|l| l.starts_with("awaiting assumptions: "))
        .unwrap_or_default();
    assert!(
        line.contains("oldest first: query-closed-after-last-row (al-runner #5807, since 2020-01-01, pin landed, act now); code-coverage-snapshot-while-logging (al-runner #5806, since 2020-01-02, waiting)"),
        "{out}{err}"
    );
}

/// An awaited test in a file that is already in the corpus (`file::Test`): the pin lands when the test is there
/// (al-runner issue 5788, whose pin adds tests to an existing isolation probe file).
#[test]
fn an_awaited_test_of_a_present_file_waits_until_the_test_is_there() {
    let extra = format!("code-coverage-snapshot-while-logging;5806;2020-01-02;{PRESENT}::Duration_NoSuchAwaitedTest");
    let (code, out, err) = cli(&["--awaiting-assumptions", "--summary"], Some(&extra));
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("code-coverage-snapshot-while-logging (al-runner #5806, since 2020-01-02, waiting)"),
        "{out}{err}"
    );
    let extra = format!("code-coverage-snapshot-while-logging;5806;2020-01-02;{PRESENT}::duration_oneDayIs86400000Ms");
    let (code, out, err) = cli(&["--awaiting-assumptions"], Some(&extra));
    assert_eq!(code, 1, "{out}{err}");
    assert!(
        out.contains("assumption `code-coverage-snapshot-while-logging` (al-runner #5806, waiting since 2020-01-02): the pin landed: remove the assumption and cite the pin"),
        "{out}{err}"
    );
    assert!(out.contains("1 awaited test(s) pass as on real BC"), "{out}{err}");
}
