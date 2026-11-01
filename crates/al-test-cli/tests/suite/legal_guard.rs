//! `tools/legal_guard.sh` (run by `check.sh`) fails on content that must not be committed (hard
//! rule 7): binaries and packages (`*.dll`, `*.app`, `*.nupkg`, `*.Source.zip`), a Microsoft
//! copyright header, the output of a .NET decompiler, and a licensing claim
//! in the docs. Every pattern has a fixture here that the guard must catch, the allowlist must let a
//! listed path through, and the repository itself must pass.
//!
//! The needles are assembled so that this file does not contain them itself.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn guard(dir: &Path) -> Output {
    Command::new("bash")
        .arg(root().join("tools/legal_guard.sh"))
        .arg(dir)
        .output()
        .expect("bash runs")
}

fn git(dir: &Path, args: &[&str]) {
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
}

/// A fresh repository whose index holds `files` (path, bytes).
fn repo_with(files: &[(&str, &[u8])]) -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    git(d.path(), &["init", "-q"]);
    for (name, bytes) in files {
        let p = d.path().join(name);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, bytes).unwrap();
        git(d.path(), &["add", "--", name]);
    }
    d
}

fn text(o: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

fn assert_caught(name: &str, bytes: &[u8]) {
    let d = repo_with(&[("README.md", b"clean\n"), (name, bytes)]);
    let o = guard(d.path());
    assert!(!o.status.success(), "{name} was not caught: {}", text(&o));
    assert!(text(&o).contains(name), "{name} is not named: {}", text(&o));
    assert!(!text(&o).contains("README.md"), "a clean file is named: {}", text(&o));
}

#[test]
fn a_clean_repository_passes() {
    let d = repo_with(&[
        ("README.md", b"# Tool\n\nRuns tests.\n"),
        ("src/lib.rs", b"fn f() {}\n"),
    ]);
    let o = guard(d.path());
    assert!(o.status.success(), "{}", text(&o));
}

#[test]
fn binaries_and_packages_are_caught() {
    for name in [
        "lib/Some.Assembly.dll",
        "deps/Publisher_App_1.0.0.0.app",
        "pkg/Some.Package.1.0.0.nupkg",
        "src/Base.Application.Source.zip",
        "UPPER/CASE.DLL",
    ] {
        assert_caught(name, b"MZ\x90\x00binary");
    }
}

#[test]
fn a_microsoft_copyright_header_is_caught() {
    let header = ["// Copyright (c) Micro", "soft Corporation. All rights reserved.\n"].concat();
    assert_caught("src/Codeunit.al", header.as_bytes());
    let mit = [
        "// Licensed under the MIT License. See Lic",
        "ense.txt in the project root\n",
    ]
    .concat();
    assert_caught("src/Other.al", mit.as_bytes());
}

#[test]
fn decompiler_markers_are_caught() {
    for (i, marker) in [
        ["Decompiled", " with ", "IL", "Spy"].concat(),
        ["ICSharpCode", ".Decompiler"].concat(),
        ["IL", "Spy"].concat(),
        ["dn", "Spy"].concat(),
        ["dot", "Peek"].concat(),
        ["// decompiled", " with a tool"].concat(),
    ]
    .iter()
    .enumerate()
    {
        assert_caught(
            &format!("src/File{i}.cs"),
            format!("// {marker}\nclass C {{}}\n").as_bytes(),
        );
    }
}

#[test]
fn license_claims_are_caught() {
    for (i, claim) in [
        ["Runs tests without a BC lic", "ense."].concat(),
        ["needs no BC lic", "ense"].concat(),
        ["without a Business Central lic", "ence"].concat(),
        ["SQL Server and without a lic", "ense."].concat(),
        ["no Docker, no SQL Server and no lic", "ense, so"].concat(),
        ["Run without Docker, SQL or a lic", "ense |"].concat(),
    ]
    .iter()
    .enumerate()
    {
        assert_caught(&format!("docs/page{i}.md"), format!("{claim}\n").as_bytes());
    }
}

#[test]
fn text_about_licenses_that_makes_no_claim_passes() {
    let d = repo_with(&[(
        "docs/a.md",
        b"No license state is held.\nTenant License State rows are refused.\nthe licensing terms\n",
    )]);
    let o = guard(d.path());
    assert!(o.status.success(), "{}", text(&o));
}

/// Encoded data (base64 in a fixture or an oracle file) spells short words by chance.
#[test]
fn a_marker_inside_encoded_data_passes() {
    let blob = ["QUJDREVGR0h", "dn", "SpyXk9QUlNU", "VVZ", "dot", "PeekWFla\n"].concat();
    let d = repo_with(&[("oracle/data.xml", blob.as_bytes())]);
    let o = guard(d.path());
    assert!(o.status.success(), "{}", text(&o));
}

#[test]
fn an_allowlisted_path_passes_and_only_for_its_check() {
    let marker = ["dot", "Peek"].concat();
    let d = repo_with(&[
        ("fixtures/app/Test.app", b"binary"),
        ("docs/history.md", format!("{marker}\n").as_bytes()),
        (
            "tools/legal_guard_allowlist.txt",
            b"# check path\nbinary fixtures/app/Test.app\nmarker docs/history.md\n",
        ),
    ]);
    let o = guard(d.path());
    assert!(o.status.success(), "{}", text(&o));

    // An entry for one check does not excuse the same file from another.
    let d = repo_with(&[
        ("docs/history.md", format!("{marker}\n").as_bytes()),
        ("tools/legal_guard_allowlist.txt", b"binary docs/history.md\n"),
    ]);
    assert!(!guard(d.path()).status.success());
}

#[test]
fn an_untracked_file_is_not_looked_at() {
    let d = repo_with(&[("README.md", b"clean\n")]);
    std::fs::write(d.path().join("Local.dll"), b"binary").unwrap();
    let o = guard(d.path());
    assert!(o.status.success(), "{}", text(&o));
}

#[test]
fn this_repository_passes() {
    let o = guard(&root());
    assert!(o.status.success(), "{}", text(&o));
}
