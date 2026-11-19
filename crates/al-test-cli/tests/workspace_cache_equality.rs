//! ADR-022 amendment 2026-11-15, "Equality requirement" (CI's release tests): the corpus apps compiled over Microsoft's
//! apps of the pinned build of the lowest BC major as dependency layers, then written to the workspace layer and read
//! back by a new compile in a new process. Every compile runs in a process of its own (the test runs itself) over one
//! kept cache folder: fresh, built (writes the entry), read, read with one thread, fresh with one thread
//! (`AL_CHECK_THREADS=1 AL_LOWER_THREADS=1`: the compile reads its thread counts from the environment). The canonical
//! dumps of all five are equal. Needs the Microsoft apps in the artifact cache (`al-test-cli --root . --all-versions
//! --fetch-deps`); a missing app fails, never skips.

use std::hash::{Hash, Hasher};
use std::path::Path;
use std::process::Command;

/// Set in the runs the test starts: the compile to make (`fresh`, `built`, `read`).
const STEP: &str = "WORKSPACE_EQUALITY_STEP";
/// The cache folder the runs share.
const CACHE: &str = "WORKSPACE_EQUALITY_CACHE";
/// Where a run writes what it found: the hash of its dump and the workspace layer's state.
const OUT: &str = "WORKSPACE_EQUALITY_OUT";

const NAME: &str = "a_compile_read_in_a_new_process_equals_fresh_compiles_with_all_threads_and_with_one";

/// One compile, in this process (a run the test started).
fn step(step: &str) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let cache = std::env::var_os(CACHE).unwrap();
    let p = al_test_cli::workspace_equality::prepare(&root, al_version::MIN_BC_MAJOR, None).unwrap();
    let (dump, report, took) =
        al_test_cli::workspace_equality::compile(&p, Path::new(&cache), step != "fresh").unwrap();
    let mut h = std::collections::hash_map::DefaultHasher::new();
    dump.hash(&mut h);
    let state = report.map_or("none".to_string(), |r| r.state.as_str().to_string());
    eprintln!(
        "{step}: {:.2} s, {state}, dump {} bytes",
        took.as_secs_f64(),
        dump.len()
    );
    std::fs::write(std::env::var_os(OUT).unwrap(), format!("{:016x} {state}", h.finish())).unwrap();
}

#[test]
fn a_compile_read_in_a_new_process_equals_fresh_compiles_with_all_threads_and_with_one() {
    if let Some(s) = std::env::var_os(STEP) {
        // On a thread with the stack the command lines give theirs: with one thread the lowering runs on this one.
        let s = s.to_string_lossy().to_string();
        let run = std::thread::Builder::new()
            .stack_size(al_testrun::RUNNER_STACK)
            .spawn(move || step(&s));
        return run.unwrap().join().unwrap();
    }
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join("cache");
    let mut found = Vec::new();
    for (i, (s, one_thread, want)) in [
        ("fresh", false, "none"),
        ("built", false, "built"),
        ("read", false, "read"),
        ("read", true, "read"),
        ("fresh", true, "none"),
    ]
    .into_iter()
    .enumerate()
    {
        let out = dir.path().join(format!("out{i}"));
        let mut run = Command::new(std::env::current_exe().unwrap());
        run.args(["--exact", NAME, "--nocapture"])
            .env(STEP, s)
            .env(CACHE, &cache)
            .env(OUT, &out);
        if one_thread {
            run.env("AL_CHECK_THREADS", "1").env("AL_LOWER_THREADS", "1");
        }
        let status = run.status().unwrap();
        assert!(
            status.success(),
            "the run {s} (one thread: {one_thread}) failed: {status}"
        );
        let text = std::fs::read_to_string(&out).unwrap();
        let (hash, state) = text.split_once(' ').unwrap();
        assert_eq!(state, want, "{s} (one thread: {one_thread})");
        found.push((s, one_thread, hash.to_string()));
    }
    assert!(
        found.iter().all(|f| f.2 == found[0].2),
        "the dumps differ (run the example workspace_cache_equality for the line): {found:?}"
    );
}
