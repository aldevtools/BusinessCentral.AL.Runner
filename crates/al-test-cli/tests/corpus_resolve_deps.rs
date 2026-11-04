//! The corpus as a workspace with its dependency apps (ADR-013): every reference that
//! still does not resolve is in `crates/al-sema/tests/expected-unresolved.txt`, the backlog
//! of the ADR. The list shrinks as dependencies, platform tables and the rest arrive; it
//! never grows without a decision.
//!
//! Needs the `corpus` submodule, including its `.alpackages`. Ignored in a debug build (about 45 s
//! there); CI runs it in the release step:
//! `cargo test --release -p al-test-cli --test corpus_resolve_deps -- --ignored`.

/// Same files as KNOWN_UNPARSED in al-syntax/tests/corpus_parse.rs.
const KNOWN_UNPARSED: &[&str] = &["types/variant/VaTTextConst.al", "types/variant/LTP89TextConst.al"];

use al_test_cli::{deps, runtime};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

#[test]
#[ignore = "slow unoptimised: cargo test --release -p al-test-cli --test corpus_resolve_deps -- --ignored"]
fn unresolved_references_are_exactly_what_no_dependency_supplies() {
    let tests = root().join("corpus/tests");
    assert!(
        tests.join("al-language/app.json").exists(),
        "corpus submodule not checked out"
    );
    let corpus_deps = deps::corpus_publish_dirs(&tests.join("al-language")).unwrap();
    let mut apps: Vec<_> = corpus_deps.iter().map(|d| runtime::load_app(d).unwrap()).collect();
    apps.push(runtime::load_app(&tests.join("al-language")).unwrap());
    let mut kinds = vec![al_sema::AppKind::Workspace; apps.len()];
    let dependencies = deps::load(&tests.join("al-language"), &corpus_deps, &[]).unwrap();
    assert!(
        dependencies.apps.iter().any(|a| a.name == "Base Application"),
        "the corpus's .alpackages supplies Base Application: {:?}",
        dependencies.log
    );
    kinds.extend(dependencies.apps.iter().map(|_| al_sema::AppKind::Dependency));
    apps.extend(dependencies.apps);
    let ws = al_sema::Workspace::load_apps(apps, kinds, &["BC27PLUS".to_string()].into());
    let members = al_sema::members::build(&ws);
    let checked = al_sema::check::check(&ws, &members, &al_builtins::catalog::Catalog::load());
    // Only the corpus's own files: what Microsoft's code references is Microsoft's business.
    let own: std::collections::HashSet<&str> = ws
        .files
        .iter()
        .filter(|f| ws.kinds[f.app] == al_sema::AppKind::Workspace)
        .map(|f| f.path.as_str())
        .collect();
    let mut unresolved: BTreeMap<String, usize> = BTreeMap::new();
    let mut other = Vec::new();
    for d in ws
        .diagnostics
        .iter()
        .chain(&checked.diagnostics)
        .filter(|d| own.contains(d.file.as_str()) && !d.code.starts_with("AR1"))
        // Files the parser does not read yet; listed in al-syntax/tests/corpus_parse.rs KNOWN_UNPARSED.
        .filter(|d| !KNOWN_UNPARSED.contains(&d.file.as_str()))
    {
        if d.code == "AR0002" {
            *unresolved.entry(d.message.clone()).or_default() += 1;
        } else {
            other.push(format!("{} {}: {}", d.code, d.file, d.message));
        }
    }
    assert!(other.is_empty(), "{}", other.join("\n"));
    let actual: String = unresolved.iter().map(|(m, n)| format!("{n:4} {m}\n")).collect();
    let golden_path = root().join("crates/al-sema/tests/expected-unresolved.txt");
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(&golden_path, &actual).unwrap();
    }
    let golden = std::fs::read_to_string(&golden_path).unwrap_or_default();
    assert_eq!(
        actual, golden,
        "unresolved references changed; review and rerun with UPDATE_GOLDEN=1"
    );
}

/// The corpus HEAD commit, which the coverage maps name.
fn corpus_head() -> String {
    String::from_utf8(
        std::process::Command::new("git")
            .arg("-C")
            .arg(root().join("corpus"))
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim()
    .to_string()
}

/// `docs/corpus-coverage.md` and `.json`: which builtin overloads some corpus test calls. ADR-065: they are not
/// committed; CI on main uploads them as an artifact, and `al-test-cli --root . --coverage-map` writes them locally.
/// This test checks that the map can be built and says something.
#[test]
#[ignore = "slow unoptimised: cargo test --release -p al-test-cli --test corpus_resolve_deps -- --ignored"]
fn corpus_coverage_map_builds() {
    let map = al_test_cli::coverage::corpus_coverage(&root(), &corpus_head()).unwrap();
    let md = al_test_cli::coverage::to_markdown(&map);
    let json = al_test_cli::coverage::to_json(&map);
    assert!(md.lines().count() > 10, "{md}");
    serde_json::from_str::<serde_json::Value>(&json).expect("the JSON map parses");
}

/// `docs/corpus-behavior-coverage.md` and `.json`: which triggers, properties, platform events, syntax kinds and
/// runtime error sites some corpus test pins. Not committed, like the map above (`--behavior-coverage-map`).
#[test]
#[ignore = "slow unoptimised: cargo test --release -p al-test-cli --test corpus_resolve_deps -- --ignored"]
fn corpus_behavior_coverage_builds() {
    let map = al_test_cli::behavior_coverage::corpus_behavior_coverage(&root(), &corpus_head()).unwrap();
    let md = al_test_cli::behavior_coverage::to_markdown(&map);
    let json = al_test_cli::behavior_coverage::to_json(&map);
    assert!(md.lines().count() > 10, "{md}");
    serde_json::from_str::<serde_json::Value>(&json).expect("the JSON map parses");
}

/// `crates/al-builtins/src/pinned_columns.rs`: the columns of platform tables the corpus pins
/// (`al_builtins::platform::PINNED_COLUMNS`), derived from the corpus so that the list cannot drift.
/// Regenerate with `UPDATE_GOLDEN=1` on this test.
#[test]
#[ignore = "slow unoptimised: cargo test --release -p al-test-cli --test corpus_resolve_deps -- --ignored"]
fn pinned_columns_are_current() {
    let commit = String::from_utf8(
        std::process::Command::new("git")
            .arg("-C")
            .arg(root().join("corpus"))
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim()
    .to_string();
    let ws = al_test_cli::coverage::corpus_workspace(&root()).unwrap();
    let members = al_sema::members::build(&ws);
    let checked = al_sema::check::check(&ws, &members, &al_builtins::catalog::Catalog::load());
    let text =
        al_test_cli::platform_columns::to_rust(&al_test_cli::platform_columns::pinned_columns(&ws, &checked), &commit);
    let path = root().join("crates/al-builtins/src/pinned_columns.rs");
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, &text).unwrap();
    }
    let committed = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        committed == text,
        "crates/al-builtins/src/pinned_columns.rs is stale: run UPDATE_GOLDEN=1 cargo test --release -p al-test-cli --test corpus_resolve_deps -- --ignored pinned_columns"
    );
}
