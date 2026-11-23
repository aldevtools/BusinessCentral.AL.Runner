//! The corpus preprocessor symbols as al-test-cli hands them out (ADR-078 point 3): `--print-symbols` reads the released
//! minors of `--released-minors` or of the oracle table, and `--must-not-compile` compiles every case with the one
//! symbol set of its oracle script, whatever list of released minors the oracle table records.

use std::path::{Path, PathBuf};
use std::process::Command;

const ONE_LEG: &str = "corpus_repo = \"corpus\"\ncorpus_app = \"corpus/tests/al-language\"\ncorpus_commit = \"abc\"\n\n\
                       [[version]]\nmajor = 29\nbc_version = \"29.1\"\njunit = \"29/junit.xml\"\nbaseline = \"metrics/29/not-passing.txt\"\n";

/// A throwaway root with `oracle/oracle.toml` of `toml`.
fn root_with(toml: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("oracle")).unwrap();
    std::fs::write(dir.path().join("oracle/oracle.toml"), toml).unwrap();
    dir
}

/// Runs al-test-cli under `root` with `args`: exit code, stdout, stderr.
fn cli(root: &Path, args: &[&str]) -> (i32, String, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_al-test-cli"))
        .arg("--root")
        .arg(root)
        .args(args)
        .output()
        .unwrap();
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).trim().to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

#[test]
fn print_symbols_reads_the_list_of_the_oracle_table() {
    let with = root_with(&format!(
        "released_minors = [\"27.0\", \"27.5\", \"29.0\", \"29.1\"]\n{ONE_LEG}"
    ));
    let (code, out, err) = cli(with.path(), &["--print-symbols", "29.1"]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(out, "BC27PLUS,BC27_5PLUS,BC28PLUS,BC29PLUS,BC29_1PLUS");
    let without = root_with(ONE_LEG);
    let (code, out, err) = cli(without.path(), &["--print-symbols", "29.1"]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(out, "BC27PLUS,BC28PLUS,BC29PLUS");
}

#[test]
fn print_symbols_refuses_a_version_text_that_is_no_version() {
    let root = root_with(ONE_LEG);
    let (code, out, err) = cli(root.path(), &["--print-symbols", "twenty-nine"]);
    assert_ne!(code, 0);
    assert!(out.is_empty(), "{out}");
    assert!(err.contains("--print-symbols `twenty-nine`"), "{err}");
}

#[test]
fn an_empty_list_of_released_minors_is_refused() {
    let root = root_with(ONE_LEG);
    let (code, _, err) = cli(root.path(), &["--print-symbols", "29.1", "--released-minors", ""]);
    assert_ne!(code, 0);
    assert!(err.contains("--released-minors"), "{err}");
    let empty = root_with(&format!("released_minors = []\n{ONE_LEG}"));
    let (code, _, err) = cli(empty.path(), &["--print-symbols", "29.1"]);
    assert_ne!(code, 0);
    assert!(err.contains("released_minors"), "{err}");
}

#[test]
fn released_minors_without_print_symbols_is_refused() {
    let root = root_with(ONE_LEG);
    let cases = cases_dir(root.path());
    for args in [
        vec!["--released-minors", "29.1"],
        vec![
            "--must-not-compile",
            cases.to_str().unwrap(),
            "--released-minors",
            "29.1",
        ],
    ] {
        let (code, _, err) = cli(root.path(), &args);
        assert_ne!(code, 0, "{args:?}");
        assert!(err.contains("--released-minors"), "{args:?}: {err}");
    }
}

/// A must-not-compile folder with one case whose error the oracle script sees only without `BC29_1PLUS`
/// (check-must-not-compile.py defines `BC<major>PLUS` alone).
fn cases_dir(root: &Path) -> PathBuf {
    let cases = root.join("must-not-compile/cases");
    std::fs::create_dir_all(&cases).unwrap();
    std::fs::write(
        cases.join("9001-x.al"),
        "// @expect AL0197 | An application object of type 'Codeunit' with name 'C' is already declared by the extension\n\
         // @expect AL0197 | An application object of type 'Codeunit' with name 'C' is already declared by the extension\n\
         codeunit 50000 C { }\n#if not BC29_1PLUS\ncodeunit 50001 C { }\n#endif\n",
    )
    .unwrap();
    cases
}

#[test]
fn must_not_compile_never_gets_a_minor_symbol_of_the_list() {
    let root = root_with(&format!("released_minors = [\"27.0\", \"29.0\", \"29.1\"]\n{ONE_LEG}"));
    let cases = cases_dir(root.path());
    for v in ["29.1", "29"] {
        let (code, out, err) = cli(
            root.path(),
            &["--must-not-compile", cases.to_str().unwrap(), "--bc-version", v],
        );
        assert_eq!(code, 0, "--bc-version {v}: {out}\n{err}");
        assert!(out.contains("1 of 1 cases match"), "--bc-version {v}: {out}");
    }
}
