//! ADR-065 amendment 2: every corpus citation by name in `crates/` and `docs/` resolves at the pinned corpus (the
//! `corpus` submodule): the object of its kind and name exists once, it has the member, and the member holds the cell.
//! A pending citation names a corpus pull request of an assumption entry. The old `File.al:line` form is not read yet
//! (step 3 of the corpus-detach plan converts it).

use std::path::Path;

#[test]
fn every_citation_by_name_resolves_at_the_pinned_corpus() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let oracle = std::fs::read_to_string(root.join("oracle/oracle.toml")).unwrap();
    let cfg = al_test_cli::config::OracleFile::parse(&oracle).unwrap();
    let corpus = root.join(&cfg.corpus_repo);
    assert!(
        corpus.join("tests").is_dir(),
        "the corpus submodule is not checked out: run `git submodule update --init`"
    );
    let (errors, read) = al_test_cli::citation_scan::check(&root, &corpus).unwrap();
    assert!(
        errors.is_empty(),
        "{} citation(s) do not resolve at the pinned corpus (ADR-065 amendment 2; at a pin move run \
         `al-test-cli --root . --rewrite-citations <old corpus rev>`):\n{}",
        errors.len(),
        errors.join("\n")
    );
    // The converted modules (al-test-cli, al-testrun discover.rs) cite by name: a check that reads nothing is broken.
    assert!(read >= 4, "only {read} citations read");
}
