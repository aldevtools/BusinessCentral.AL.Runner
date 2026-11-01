//! `crates/al-vm/src/platform/windows_language_rows.rs`: the rows of Windows Language the corpus pins
//! (`al_test_cli::windows_language`), derived from the corpus so that the table cannot drift.
//! Regenerate with `UPDATE_GOLDEN=1` on this test.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The BC majors the repository measures (`metrics/<major>/`).
fn majors() -> Vec<u16> {
    let mut v: Vec<u16> = std::fs::read_dir(root().join("metrics"))
        .unwrap()
        .filter_map(|e| e.ok()?.file_name().to_str()?.parse().ok())
        .collect();
    v.sort();
    v
}

#[test]
fn windows_language_rows_are_current() {
    let dir = root().join("corpus").join(al_test_cli::windows_language::FOLDER);
    let mut files: Vec<(String, String)> = std::fs::read_dir(&dir)
        .map(|d| {
            d.filter_map(|e| {
                let p = e.ok()?.path();
                (p.extension()? == "al").then(|| {
                    let name = format!(
                        "{}/{}",
                        al_test_cli::windows_language::FOLDER,
                        p.file_name().unwrap().to_str().unwrap()
                    );
                    (name, std::fs::read_to_string(&p).unwrap())
                })
            })
            .collect()
        })
        .unwrap_or_default();
    files.sort();
    let majors = majors();
    let tables = al_test_cli::windows_language::tables(&files, &majors).unwrap();
    // Only the files that give a row are named, so a probe file of the folder does not make the header stale.
    let named: Vec<String> = files
        .iter()
        .filter(|f| {
            al_test_cli::windows_language::tables(std::slice::from_ref(f), &majors).is_ok_and(|t| !t.is_empty())
        })
        .map(|(n, _)| n.clone())
        .collect();
    // A held pin generates the empty table (`windows_language::HELD`).
    let text = match al_test_cli::windows_language::HELD {
        Some(_) => al_test_cli::windows_language::to_rust(&Default::default(), &[]),
        None => al_test_cli::windows_language::to_rust(&tables, &named),
    };
    let path = root().join("crates/al-vm/src/platform/windows_language_rows.rs");
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, &text).unwrap();
    }
    let committed = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        committed == text,
        "crates/al-vm/src/platform/windows_language_rows.rs is stale: run UPDATE_GOLDEN=1 cargo test -p al-test-cli --test windows_language_rows"
    );
}
