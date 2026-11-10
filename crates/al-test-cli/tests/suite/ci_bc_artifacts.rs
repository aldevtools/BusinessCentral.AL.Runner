//! ADR-013 amendment of 2026-11-10: Microsoft's apps come only from the pinned builds in the artifact cache, so every
//! CI job that runs the corpus or a Rust test that reads a Microsoft file fills the cache first, through one
//! composite action (`.github/actions/bc-artifacts`). The tests never download: without the step they fail.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

const USES: &str = "uses: ./.github/actions/bc-artifacts";

/// The text of job `name` of ci.yml: from its header to the next job's.
fn job(ci: &str, name: &str) -> String {
    let start = ci
        .find(&format!("\n  {name}:\n"))
        .unwrap_or_else(|| panic!("ci.yml has no job `{name}`"));
    let rest = &ci[start + 1..];
    let end = rest
        .match_indices("\n  ")
        .map(|(i, _)| i)
        .find(|&i| {
            let line = rest[i + 1..].lines().next().unwrap_or("");
            i > 0
                && line.starts_with("  ")
                && !line.starts_with("   ")
                && !line.trim_start().starts_with('#')
                && line.trim_end().ends_with(':')
        })
        .unwrap_or(rest.len());
    rest[..end].to_string()
}

fn before(text: &str, first: &str, then: &str) {
    let a = text.find(first).unwrap_or_else(|| panic!("missing `{first}`"));
    let b = text.find(then).unwrap_or_else(|| panic!("missing `{then}`"));
    assert!(a < b, "`{first}` must come before `{then}`");
}

#[test]
fn the_action_names_the_builds_restores_the_cache_and_fetches() {
    let action = read(".github/actions/bc-artifacts/action.yml");
    before(&action, "--print-builds", "actions/cache@v4");
    before(&action, "actions/cache@v4", "--fetch-deps");
    assert!(action.contains("path: ~/.cache/al-runtime/artifacts"));
    // The key names the builds of the selection the job fetches, and other caches can use it.
    assert!(action.contains("key: bc-artifacts-${{ steps.bc.outputs.key }}"));
    assert!(action.contains("value: ${{ steps.bc.outputs.key }}"));
}

/// The debug tests read the BC 27 build only (`al_testfiles::MAJOR`); the step comes before the tests.
#[test]
fn the_test_job_fetches_the_bc_27_build_before_the_tests() {
    let job = job(&read(".github/workflows/ci.yml"), "test");
    before(
        &job,
        "cargo build --workspace --no-default-features --bin al-test-cli",
        USES,
    );
    before(&job, USES, "name: Tests (debug, binaries side by side)");
    assert!(job.contains("binary: ./target/debug/al-test-cli"));
    assert!(job.contains("versions: --bc-version 27"));
}

#[test]
fn the_release_tests_job_fetches_before_its_tests() {
    let job = job(&read(".github/workflows/ci.yml"), "release-tests");
    before(&job, "cargo build --release -p al-test-cli", USES);
    before(&job, USES, "name: Perf budgets");
    before(&job, USES, "name: Release tests");
}

#[test]
fn the_corpus_job_and_the_bump_bot_fetch_through_the_same_action() {
    let corpus = job(&read(".github/workflows/ci.yml"), "corpus");
    before(&corpus, USES, "name: Corpus pass rate");
    let bot = read(".github/workflows/corpus-bump-bot.yml");
    before(&bot, USES, "name: Bump");
    // The three steps are in the action only: no workflow carries its own copy.
    for (name, text) in [
        ("ci.yml", read(".github/workflows/ci.yml")),
        ("corpus-bump-bot.yml", bot),
    ] {
        assert!(
            !text.contains("--print-builds |"),
            "{name} has its own copy of the build list step"
        );
        assert!(
            !text.contains("path: ~/.cache/al-runtime/artifacts"),
            "{name} has its own artifact cache step"
        );
    }
}
