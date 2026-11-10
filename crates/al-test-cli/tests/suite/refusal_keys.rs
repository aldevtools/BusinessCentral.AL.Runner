//! The refusal key lists of all crates together (ADR-071 conformance; owner decision 2026-11-10: one list per crate
//! that refuses, checked together here): unique, lower case and dotted, a known area, no entry named `legacy`, one
//! file per key named by it, every key used, and the generated table of `docs/probes-needed.md` current.

use al_builtins::refusal::{AREAS, LEGACY};
use al_test_cli::refusal_keys::{LISTS, all, probes_needed_doc};
use std::path::{Path, PathBuf};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn keys_are_unique_lower_case_and_dotted() {
    let keys = all();
    let mut seen = std::collections::BTreeSet::new();
    for (krate, k) in &keys {
        let key = k.key();
        assert!(seen.insert(key), "{key} is registered twice (last in {krate})");
        let parts: Vec<&str> = key.split('.').collect();
        assert!(parts.len() >= 2, "{key}: an area and at least one more dotted part");
        for p in parts {
            assert!(
                !p.is_empty()
                    && p.bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
                    && !p.starts_with('-')
                    && !p.ends_with('-'),
                "{key}: each part is lower case words joined by hyphens"
            );
        }
    }
}

#[test]
fn every_key_starts_with_a_known_area() {
    for (krate, k) in all() {
        let area = k.key().split('.').next().unwrap_or("");
        assert!(AREAS.contains(&area), "{} ({krate}): unknown area `{area}`", k.key());
    }
}

#[test]
fn no_entry_is_named_legacy() {
    for (krate, k) in all() {
        assert!(
            k.key() != LEGACY && k.key().split('.').next() != Some(LEGACY),
            "{krate}: `legacy` is the key of free text, not an entry"
        );
    }
}

/// The key files of one crate's list; a missing folder is an empty list (git keeps no empty folder).
fn key_files(krate: &str) -> Vec<String> {
    let dir = repo().join("crates").join(krate).join("src/refusals");
    match std::fs::read_dir(&dir) {
        Ok(rd) => rd
            .flatten()
            .filter_map(|e| e.file_name().to_str()?.strip_suffix(".rs").map(String::from))
            .collect(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => panic!("{}: {e}", dir.display()),
    }
}

#[test]
fn every_crate_with_a_key_folder_has_its_list_in_lists() {
    let mut missing = Vec::new();
    for e in std::fs::read_dir(repo().join("crates")).expect("crates/").flatten() {
        let krate = e.file_name().to_string_lossy().into_owned();
        if !key_files(&krate).is_empty() && !LISTS.iter().any(|(c, _)| *c == krate) {
            missing.push(krate);
        }
    }
    assert!(
        missing.is_empty(),
        "crates with src/refusals/ but no row in al-test-cli/src/refusal_keys.rs LISTS: {missing:?}"
    );
}

#[test]
fn every_file_of_a_list_is_named_by_its_key() {
    for (krate, list) in LISTS {
        let mut files = key_files(krate);
        files.sort();
        let mut keys: Vec<String> = list.iter().map(|k| k.key().to_string()).collect();
        keys.sort();
        assert_eq!(files, keys, "{krate}: one file per key, named `<key>.rs`");
    }
}

/// The source files of every crate, without the key lists themselves.
fn sources() -> Vec<(PathBuf, String)> {
    let mut out = Vec::new();
    let mut stack = vec![repo().join("crates")];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if p.is_dir() {
                if !matches!(name, "target" | "tests" | "refusals") {
                    stack.push(p);
                }
            } else if name.ends_with(".rs") && p.to_string_lossy().contains("/src/") {
                if let Ok(t) = std::fs::read_to_string(&p) {
                    out.push((p, t));
                }
            }
        }
    }
    out
}

#[test]
fn every_entry_is_used_somewhere() {
    let files = sources();
    let unused: Vec<&str> = all()
        .into_iter()
        .filter(|(_, k)| {
            let needle = format!("keys::{}", k.info().name);
            !files.iter().any(|(_, t)| t.contains(&needle))
        })
        .map(|(_, k)| k.key())
        .collect();
    assert!(
        unused.is_empty(),
        "registered but never used as `keys::NAME` (delete the file or use the key): {unused:?}"
    );
}

#[test]
fn the_refusal_keys_table_of_probes_needed_is_current() {
    let path = repo().join("docs/probes-needed.md");
    let text = std::fs::read_to_string(&path).expect("docs/probes-needed.md");
    let new = probes_needed_doc(&text).expect("the refusal-keys markers");
    if new != text && std::env::var_os("UPDATE_REFUSALS_DOC").is_some() {
        std::fs::write(&path, &new).expect("write docs/probes-needed.md");
        return;
    }
    assert!(
        new == text,
        "the refusal keys table of docs/probes-needed.md is stale: run al-test-cli --root . --refusals-doc"
    );
}
