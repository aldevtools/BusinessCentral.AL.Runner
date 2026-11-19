//! ADR-022 amendment 2026-11-15, "Equality requirement" (CI's release tests): the corpus apps compiled over Microsoft's
//! apps of the pinned build of the lowest BC major as dependency layers, then written to the workspace layer and read
//! back. The read compile's canonical dump equals the fresh one's, with all threads and with one thread (the test runs
//! itself, once per thread count, the second time with `AL_CHECK_THREADS=1 AL_LOWER_THREADS=1`: the lowering reads its thread count from the
//! environment, which a test must not change in its own process). Needs the Microsoft apps in the artifact cache
//! (`al-test-cli --root . --all-versions --fetch-deps`); a missing app fails, never skips.

use std::path::Path;
use std::process::Command;

/// Set in the runs the test starts: the thread counts to check with.
const THREADS: &str = "WORKSPACE_EQUALITY_THREADS";

fn check(threads: &str) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let cache = tempfile::tempdir().unwrap();
    let major = al_version::MIN_BC_MAJOR;
    let outcome = al_test_cli::workspace_equality::run(&root, major, cache.path(), None, &mut |_, _| {}).unwrap();
    for s in &outcome.steps {
        eprintln!(
            "{threads}: {} {:.2} s {:?}",
            s.name,
            s.took.as_secs_f64(),
            s.report.as_ref().map(|r| r.state)
        );
    }
    assert!(
        outcome.difference.is_none(),
        "{threads}, BC {major}: {:?}",
        outcome.difference
    );
    assert!(outcome.ok(), "{threads}, BC {major}: {:?}", outcome.steps);
}

#[test]
fn a_read_compile_of_the_corpus_equals_a_fresh_one_with_all_threads_and_with_one() {
    if let Some(threads) = std::env::var_os(THREADS) {
        // On a thread with the stack the command lines give theirs: with one thread the lowering runs on this one.
        let threads = threads.to_string_lossy().to_string();
        let run = std::thread::Builder::new()
            .stack_size(al_testrun::RUNNER_STACK)
            .spawn(move || check(&threads));
        return run.unwrap().join().unwrap();
    }
    // Each check in a process of its own: one compile of the corpus and its dumps take several GB, which a second check
    // in the same process would add to.
    for (threads, count) in [("all threads", None), ("one thread", Some("1"))] {
        let mut run = Command::new(std::env::current_exe().unwrap());
        run.args([
            "--exact",
            "a_read_compile_of_the_corpus_equals_a_fresh_one_with_all_threads_and_with_one",
            "--nocapture",
        ])
        .env(THREADS, threads);
        if let Some(n) = count {
            run.env("AL_CHECK_THREADS", n).env("AL_LOWER_THREADS", n);
        }
        let status = run.status().unwrap();
        assert!(status.success(), "the run with {threads} failed: {status}");
    }
}
