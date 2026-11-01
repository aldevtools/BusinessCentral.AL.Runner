//! ADR-013 decision 3 and hard rule 7: Microsoft's source is compiled from a cache outside
//! the repository and never committed. Every file Microsoft ships starts with a copyright
//! header, so no tracked file may carry one. The `corpus` submodule is another repository's
//! content and is not looked at.

use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

#[test]
fn no_tracked_file_carries_the_microsoft_copyright_header() {
    // The needles are assembled so that this file does not contain them itself.
    let needles = [
        ["Copyright (c) Micro", "soft Corporation. All rights reserved."].concat(),
        [
            "Licensed under the MIT License. See Lic",
            "ense.txt in the project root",
        ]
        .concat(),
    ];
    let out = Command::new("git")
        .arg("-C")
        .arg(root())
        .args(["ls-files", "-z"])
        .output()
        .expect("git runs");
    assert!(out.status.success(), "git ls-files failed");
    let names = String::from_utf8_lossy(&out.stdout).into_owned();
    let mut checked = 0;
    let mut hits = Vec::new();
    for name in names.split('\0').filter(|n| !n.is_empty()) {
        if name == "corpus" || name.starts_with("corpus/") {
            continue;
        }
        // A submodule entry is a directory; a deleted-but-tracked file is not there.
        let Ok(bytes) = std::fs::read(root().join(name)) else {
            continue;
        };
        checked += 1;
        let text = String::from_utf8_lossy(&bytes);
        if needles.iter().any(|n| text.contains(n.as_str())) {
            hits.push(name.to_string());
        }
    }
    assert!(checked > 100, "only {checked} tracked files were looked at");
    assert!(
        hits.is_empty(),
        "Microsoft source must not be committed (hard rule 7); found its header in: {hits:?}"
    );
}

/// ADR-022: the compile cache holds the lowered form of Microsoft's apps, made on the user's
/// machine. It is never tracked, and `.gitignore` keeps it out of a commit by accident.
#[test]
fn no_compile_cache_file_is_tracked_and_git_ignores_them() {
    let out = Command::new("git")
        .arg("-C")
        .arg(root())
        .args(["ls-files", "-z"])
        .output()
        .expect("git runs");
    assert!(out.status.success(), "git ls-files failed");
    let names = String::from_utf8_lossy(&out.stdout).into_owned();
    let cache_files: Vec<&str> = names
        .split('\0')
        .filter(|n| n.ends_with(".alcc") || n.contains("/compile/deps-"))
        .collect();
    assert!(
        cache_files.is_empty(),
        "compile cache files are tracked: {cache_files:?}"
    );
    let ignore = std::fs::read_to_string(root().join(".gitignore")).expect(".gitignore");
    assert!(
        ignore.lines().any(|l| l.trim() == "*.alcc"),
        ".gitignore must list *.alcc"
    );
}
