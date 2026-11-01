//! ADR-022, "CI step order": the workflow caches the compile cache files, builds or verifies them
//! before the measured run, makes that run require a hit, compares its outcomes with the uncached
//! run, reports the times with the cache on and the time to build it on its own line, and prunes
//! before the cache is saved.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn workflow() -> String {
    std::fs::read_to_string(root().join(".github/workflows/ci.yml")).expect("ci.yml")
}

fn at(text: &str, needle: &str, from: usize) -> usize {
    match text[from..].find(needle) {
        Some(i) => from + i,
        None => panic!("`{needle}` does not follow the previous step in .github/workflows/ci.yml"),
    }
}

#[test]
fn the_steps_come_in_the_order_the_adr_gives() {
    let ci = workflow();
    let mut pos = at(&ci, "Fetch dependency apps", 0);
    pos = at(&ci, "name: Compile cache, restore", pos);
    pos = at(&ci, "actions/cache@v4", pos);
    pos = at(&ci, "name: Compile cache, build or verify", pos);
    pos = at(&ci, "--build-compile-cache", pos);
    // The reference run, then the measured one that requires a hit.
    pos = at(
        &ci,
        "name: Corpus with the compile cache, against the uncached run",
        pos,
    );
    pos = at(&ci, "--require-compile-cache", pos);
    pos = at(&ci, "tools/cache_vs_uncached.sh compare", pos);
    pos = at(&ci, "Time to build the compile cache", pos);
    pos = at(&ci, "GITHUB_STEP_SUMMARY", pos);
    at(&ci, "name: Compile cache, prune", pos);
}

#[test]
fn the_cache_key_names_the_builds_and_has_no_fallback() {
    let ci = workflow();
    let start = at(&ci, "name: Compile cache, restore", 0);
    let step_end = at(&ci, "name: Compile cache, build or verify", start);
    let step = &ci[start..step_end];
    assert!(
        step.contains("steps.bc.outputs.key"),
        "the key must name the BC builds:\n{step}"
    );
    assert!(
        step.contains("hashFiles("),
        "the key must name the sources of the runtime:\n{step}"
    );
    assert!(
        !step.contains("restore-keys"),
        "an entry of another build is never usable:\n{step}"
    );
}

#[test]
fn the_summary_reports_the_times_with_the_cache_on_and_the_build_time_apart() {
    let ci = workflow();
    let start = at(&ci, "name: Corpus with the compile cache, against the uncached run", 0);
    let step = &ci[start..at(&ci, "name: Compile cache, prune", start)];
    assert!(
        step.contains("cached-pass-rate-*.md"),
        "the times of the cached run:\n{step}"
    );
    assert!(
        step.contains("compile cache build time"),
        "the build time on its own line:\n{step}"
    );
}

#[test]
fn the_comparison_script_exists_and_compares_with_the_allow_list() {
    let script = std::fs::read_to_string(root().join("tools/cache_vs_uncached.sh")).expect("the script");
    assert!(script.contains("compare_junit.py"));
    assert!(script.contains("demand-eager-known-differences.txt"));
    assert!(script.contains("runtime-junit-") && script.contains("cached-junit-"));
}

/// The GitHub cache has a limit; a push to main saves new entries, so a job at the end deletes the old ones. It must
/// never fail the build.
#[test]
fn a_cleanup_job_keeps_the_github_cache_under_its_limit_and_cannot_fail_the_build() {
    let ci = workflow();
    let job = &ci[at(&ci, "\n  cache-cleanup:", 0)..];
    for needle in [
        "needs: [test, portable, corpus, release-tests]",
        "github.event_name == 'push' && github.ref == 'refs/heads/main'",
        "actions: write",
        "--key compile-cache-",
        ".[3:][].id",
        "--key v0-rust-",
        ".[1:][].id",
    ] {
        assert!(job.contains(needle), "the cleanup job lacks `{needle}`");
    }
    // The job and both steps are continue-on-error, and a failed delete does not stop the loop.
    assert_eq!(job.matches("continue-on-error: true").count(), 3);
    assert_eq!(job.matches("|| true").count(), 2);
}
