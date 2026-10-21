//! `catalog/signatures.txt` must be exactly what the pinned corpus's docs
//! scrape generates. Regenerate with `UPDATE_GOLDEN=1 cargo test -p al-builtins`.

use std::path::Path;

#[test]
fn signatures_file_is_up_to_date_with_the_pinned_corpus() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let json = std::fs::read_to_string(root.join("../../corpus/scripts/al-surface.json"))
        .expect("corpus submodule not checked out: run `git submodule update --init`");
    let lines = al_builtins::catalog::generate(&json).unwrap_or_else(|e| panic!("unparseable syntax lines:\n{e}"));
    let generated = format!(
        "# GENERATED from corpus/scripts/al-surface.json by al-builtins (ADR-008). Do not edit; use overrides.txt.\n{}\n",
        lines.join("\n")
    );
    let path = root.join("catalog/signatures.txt");
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, &generated).unwrap();
    }
    assert_eq!(
        std::fs::read_to_string(&path).unwrap_or_default(),
        generated,
        "stale; rerun with UPDATE_GOLDEN=1"
    );
}

/// `catalog/docs-runtimes.txt` (the docs runtime of each overload, al-runner issue 5573) is what the same scrape
/// generates. Regenerate with `UPDATE_GOLDEN=1 cargo test -p al-builtins`.
#[test]
fn docs_runtimes_file_is_up_to_date_with_the_pinned_corpus() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let json = std::fs::read_to_string(root.join("../../corpus/scripts/al-surface.json"))
        .expect("corpus submodule not checked out: run `git submodule update --init`");
    let lines = al_builtins::catalog::generate_docs_runtimes(&json).unwrap_or_else(|e| panic!("{e}"));
    let generated = format!(
        "# GENERATED from corpus/scripts/al-surface.json by al-builtins (ADR-008). Do not edit.\n\
         # <catalog line>\\t<runtime>: the runtime Microsoft's docs give for the overload (\"Available or changed with\n\
         # runtime version\"), for every overload above 1.0; `?` when the docs give none. Other generated lines: 1.0.\n{}\n",
        lines.join("\n")
    );
    let path = root.join("catalog/docs-runtimes.txt");
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, &generated).unwrap();
    }
    assert_eq!(
        std::fs::read_to_string(&path).unwrap_or_default(),
        generated,
        "stale; rerun with UPDATE_GOLDEN=1"
    );
}
