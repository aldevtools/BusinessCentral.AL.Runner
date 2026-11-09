//! The memory budget of a test run (ADR-066 amendments 9 and 10): `--max-memory`, the `--timings` line, and the stop
//! with a clear message and exit code 2 when the run cannot fit under a hard limit.

use super::*;

const TESTS: &str = r#"
codeunit 50310 "Budget Tests"
{
    Subtype = Test;

    [Test]
    procedure Passes()
    begin
    end;

    [Test]
    procedure Fails()
    begin
        Error('as expected');
    end;
}
"#;

fn budget_app(root: &Path) -> PathBuf {
    one_file_app(
        root,
        "budget",
        "11111111-0000-0000-0000-000000000031",
        "Budget",
        "src/Tests.al",
        TESTS,
    )
}

#[test]
fn a_limit_below_what_one_test_thread_needs_stops_the_run_with_the_numbers() {
    let d = tempfile::tempdir().unwrap();
    let app = budget_app(d.path());
    let o = alrunner(&["test", app.to_str().unwrap(), "--no-download", "--max-memory", "1M"]);
    let t = text(&o);
    assert_eq!(code(&o), 2, "{t}");
    assert!(t.contains("not enough memory to run the tests"), "{t}");
    assert!(t.contains("the limit is 1 MB (--max-memory)"), "{t}");
    assert!(!t.contains("PASS"), "no test ran: {t}");
}

/// ADR-066 amendment 10, decision 6: the tests a stopped run did not reach are reported as not run, with the limit as
/// the reason, never as failures.
#[test]
fn the_tests_a_stopped_run_did_not_reach_are_not_run_with_the_limit_as_the_reason() {
    let d = tempfile::tempdir().unwrap();
    let app = budget_app(d.path());
    let junit = d.path().join("out.xml");
    let o = alrunner(&[
        "test",
        app.to_str().unwrap(),
        "--no-download",
        "--max-memory",
        "1M",
        "--junit",
        junit.to_str().unwrap(),
    ]);
    let t = text(&o);
    assert_eq!(code(&o), 2, "{t}");
    let xml = std::fs::read_to_string(&junit).unwrap_or_else(|e| panic!("{e}: {t}"));
    assert_eq!(xml.matches("memory limit (--max-memory 1 MB)").count(), 2, "{xml}");
    assert!(!xml.contains("<failure"), "{xml}");
}

#[test]
fn timings_print_the_budget_its_source_and_the_threads_chosen() {
    let d = tempfile::tempdir().unwrap();
    let app = budget_app(d.path());
    let o = alrunner(&[
        "test",
        app.to_str().unwrap(),
        "--no-download",
        "--max-memory",
        "1500M",
        "--timings",
    ]);
    let t = text(&o);
    assert_eq!(code(&o), 1, "{t}");
    let line = t
        .lines()
        .find(|l| l.starts_with("memory: budget"))
        .unwrap_or_else(|| panic!("{t}"));
    assert!(line.contains("budget 1500 MB (--max-memory)"), "{line}");
    // ADR-066 amendment 10: the most threads that ran and how many ended for memory.
    assert!(line.contains("test threads at most 1 of"), "{line}");
    assert!(line.contains("0 ended for memory"), "{line}");
}

#[test]
fn without_the_flag_the_budget_is_the_free_memory_or_the_cgroup_limit() {
    let d = tempfile::tempdir().unwrap();
    let app = budget_app(d.path());
    let o = alrunner(&["test", app.to_str().unwrap(), "--no-download", "--timings"]);
    let t = text(&o);
    assert_eq!(code(&o), 1, "{t}");
    let line = t
        .lines()
        .find(|l| l.starts_with("memory: budget"))
        .unwrap_or_else(|| panic!("{t}"));
    assert!(
        line.contains("(free memory)") || line.contains("(cgroup memory.max)"),
        "{line}"
    );
}

#[test]
fn a_tight_budget_gives_the_same_outcomes() {
    let d = tempfile::tempdir().unwrap();
    let app = budget_app(d.path());
    let run = |extra: &[&str]| {
        let mut args = vec!["test", app.to_str().unwrap(), "--no-download", "--timings"];
        args.extend_from_slice(extra);
        text(&alrunner(&args))
    };
    let outcomes = |t: &str| {
        let mut v: Vec<String> = t
            .lines()
            .filter(|l| l.starts_with("FAIL") || l.starts_with("Tests:"))
            .map(str::to_string)
            .collect();
        v.sort();
        v
    };
    let free = run(&[]);
    assert_eq!(outcomes(&free).len(), 2, "{free}");
    // A budget a little above what the free run held after the install baseline (the resident memory of a compile
    // varies by about 100 MB between runs, so the thread count is not asserted here; `memory.rs` tests it).
    let held: u64 = free
        .split("after install baseline ")
        .nth(1)
        .and_then(|r| r.split(" MB").next())
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("{free}"));
    let tight = run(&["--max-memory", &format!("{}M", held + 300)]);
    assert_eq!(outcomes(&free), outcomes(&tight), "{tight}");
    assert!(tight.contains("(--max-memory)"), "{tight}");
}

#[test]
fn a_bad_size_is_a_bad_invocation() {
    let d = tempfile::tempdir().unwrap();
    let app = budget_app(d.path());
    let o = alrunner(&["test", app.to_str().unwrap(), "--no-download", "--max-memory", "lots"]);
    let t = text(&o);
    assert_eq!(code(&o), 2, "{t}");
    assert!(t.contains("--max-memory"), "{t}");
}

/// Decision 6 holds while the apps compile too (owner, 2026-11-09): a limit the process passes before its compile ends
/// stops the compile, says so with the numbers, and reports every test as not run (exit code 2).
#[test]
fn a_limit_the_compile_passes_stops_the_compile_and_no_test_runs() {
    let d = tempfile::tempdir().unwrap();
    let app = budget_app(d.path());
    let junit = d.path().join("out.xml");
    let o = alrunner(&[
        "test",
        app.to_str().unwrap(),
        "--no-download",
        "--max-memory",
        "1M",
        "--junit",
        junit.to_str().unwrap(),
    ]);
    let t = text(&o);
    assert_eq!(code(&o), 2, "{t}");
    assert!(t.contains("while the apps were compiled"), "{t}");
    assert!(t.contains("the limit is 1 MB (--max-memory)"), "{t}");
    let xml = std::fs::read_to_string(&junit).unwrap_or_else(|e| panic!("{e}: {t}"));
    assert_eq!(xml.matches("memory limit (--max-memory 1 MB)").count(), 2, "{xml}");
    assert!(xml.contains("Budget Tests"), "the codeunit name is reported: {xml}");
}
