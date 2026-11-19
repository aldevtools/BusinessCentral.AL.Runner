//! ADR-066 amendment 10, decision 6, with `--strict-order`: a run stopped at its hard memory limit reports a codeunit
//! after one the stop cut short as not run exactly when the lane would have taken its result from a serial re-run that
//! needed the cut codeunit (al-runner issue 8368). Every other codeunit keeps the outcome of its parallel run, as on main.
//!
//! Each run holds the cut codeunit's thread at the end of its first test (`hold`) until the tests the shape needs ended
//! and the run stopped, so the order comes from the test events, never from timing. That needs a second test thread;
//! with one (`AL_TEST_THREADS=1`, one core) the stopped run is only checked against the complete one.

use crate::memory_threads::{budget, outcomes, run_with};
use al_testrun::runtime::{CorpusRuntime, RunOptions};
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

const LIMIT: &str = "memory limit (--max-memory 1000 MB)";

const QUICK: &str = r#"
codeunit 60800 "Quick First"
{
    Subtype = Test;
    [Test] procedure Quick() begin end;
}
"#;

/// A codeunit whose second test writes the SingleInstance global; the stop cuts it after its first test.
const WRITER: &str = r#"
codeunit 60801 "Cut Writer"
{
    Subtype = Test;
    [Test] procedure First() begin end;
    [Test] procedure Writes() var S: Codeunit "Memory Threads Count"; begin S.Bump(); end;
}
"#;

/// A reader that fails in the test that reads: it passes only after the writer.
const READER: &str = r#"
codeunit 60803 "Single Reader"
{
    Subtype = Test;
    [Test] procedure Reads()
    var S: Codeunit "Memory Threads Count";
    begin
        if S.Get() <> 1 then Error('no earlier writer');
    end;
}
"#;

fn strict() -> RunOptions {
    RunOptions {
        strict_order: true,
        ..Default::default()
    }
}

/// The complete run of `src` and the run stopped once every test of `wait_for` ended, with the thread of the test
/// `hold` held at that test's end until the stop, and whether the run had the second thread that order needs.
struct Runs {
    complete: Vec<(u32, String, String)>,
    stopped: Vec<(u32, String, String)>,
    ordered: bool,
}

fn runs(src: &str, opts: &RunOptions, wait_for: &[&str], hold: &'static str) -> Runs {
    let free = run_with(src, opts);
    let ordered = free.phases.memory.asked >= 2;
    let stopped = stopped(src, opts, wait_for, hold, ordered);
    if ordered {
        assert!(stopped.phases.memory.halted.is_some(), "{:?}", stopped.phases.memory);
    }
    let (complete, stopped) = (outcomes(&free), outcomes(&stopped));
    for (c, t, o) in &stopped {
        if !o.contains(LIMIT) {
            assert_eq!(
                Some(o),
                of(&complete, *c, t).as_ref(),
                "a result the stop did not change: {t}"
            );
        }
    }
    assert_eq!(stopped.len(), complete.len(), "every test has an outcome");
    Runs {
        complete,
        stopped,
        ordered,
    }
}

fn stopped(src: &str, opts: &RunOptions, wait_for: &[&str], hold: &'static str, ordered: bool) -> CorpusRuntime {
    let want: BTreeSet<String> = wait_for.iter().map(|s| s.to_string()).collect();
    let ended: Arc<Mutex<BTreeSet<String>>> = Arc::default();
    let (seen, rss) = (ended.clone(), ended);
    let all_ended = move |e: &Mutex<BTreeSet<String>>| want.is_subset(&e.lock().unwrap());
    let all = all_ended.clone();
    run_with(
        src,
        &RunOptions {
            memory: budget(1000, true, move || if all_ended(&rss) { 990 } else { 100 }),
            hooks: al_testrun::events::Hooks {
                on_test: Some(Arc::new(move |e| {
                    seen.lock().unwrap().insert(e.test.clone());
                    if ordered && e.test == hold {
                        // A guard against a hang only: the order itself comes from the events.
                        let since = std::time::Instant::now();
                        while !(all(&seen) && al_testrun::events::halted().is_some()) {
                            assert!(since.elapsed().as_secs() < 120, "the run never stopped");
                            std::thread::sleep(std::time::Duration::from_millis(1));
                        }
                    }
                })),
                ..Default::default()
            },
            ..opts.clone()
        },
    )
}

fn of(v: &[(u32, String, String)], id: u32, test: &str) -> Option<String> {
    v.iter()
        .find(|(c, t, _)| *c == id && t == test)
        .map(|(.., o)| o.clone())
}

impl Runs {
    fn stopped(&self, id: u32, test: &str) -> String {
        of(&self.stopped, id, test).unwrap_or_else(|| panic!("no outcome for {test}"))
    }

    fn complete(&self, id: u32, test: &str) -> String {
        of(&self.complete, id, test).unwrap_or_else(|| panic!("no outcome for {test}"))
    }

    /// The shape holds: the stop cut `test` of codeunit `id` short.
    fn cut(&self, id: u32, test: &str) {
        if self.ordered {
            assert!(
                self.stopped(id, test).contains(LIMIT),
                "{test} was not cut short: {:?}",
                self.stopped
            );
        }
    }
}

/// The pinned shape: the reader failed in its parallel run, and a complete run takes its result from its serial re-run
/// after the writer, where it passes. The stopped run reached neither the write nor the re-run, so the reader is not
/// run, never failed (decision 6). A plain failure after the writer that reads nothing keeps its outcome.
#[test]
fn a_hard_limit_before_an_earlier_writer_ran_does_not_report_the_reader_failed() {
    let plain = r#"
codeunit 60802 "Plain Fail"
{
    Subtype = Test;
    [Test] procedure FailsAlone() begin Error('fails alone'); end;
}
"#;
    let src = format!("{QUICK}{WRITER}{plain}{READER}");
    let r = runs(&src, &strict(), &["Quick", "FailsAlone", "Reads"], "First");
    assert_eq!(r.complete(60803, "Reads"), "Pass");
    r.cut(60801, "Writes");
    if r.ordered {
        assert!(r.stopped(60803, "Reads").contains(LIMIT), "{:?}", r.stopped);
    }
    assert_eq!(r.stopped(60802, "FailsAlone"), "Fail(\"fails alone\")");
    // Without BC's order nothing runs again: a complete run keeps the reader's parallel failure, and so does the
    // stopped run.
    let r = runs(&src, &RunOptions::default(), &["Quick", "FailsAlone", "Reads"], "First");
    assert_eq!(r.complete(60803, "Reads"), "Fail(\"no earlier writer\")");
    assert_eq!(r.stopped(60803, "Reads"), r.complete(60803, "Reads"));
}

/// The failed test reads nothing; a later test of the codeunit reads the global and passes. A complete run keeps the
/// parallel result (amendment 7, option 1: neither the failed test nor one before it read the global), so the stopped
/// run keeps it too.
#[test]
fn a_reader_whose_failed_test_read_nothing_keeps_its_outcome_after_a_cut_writer() {
    let reader = r#"
codeunit 60803 "Late Reader"
{
    Subtype = Test;
    [Test] procedure AFailsAlone() begin Error('plain failure'); end;
    [Test] procedure BReads() var S: Codeunit "Memory Threads Count"; begin if S.Get() < 0 then Error('never'); end;
}
"#;
    let src = format!("{QUICK}{WRITER}{reader}");
    let r = runs(&src, &strict(), &["Quick", "AFailsAlone", "BReads"], "First");
    r.cut(60801, "Writes");
    assert_eq!(r.stopped(60803, "AFailsAlone"), "Fail(\"plain failure\")");
    assert_eq!(r.stopped(60803, "BReads"), "Pass");
}

/// The same reader after a complete writer, with a cut codeunit between them that never uses the single instance: the
/// stopped run keeps the reader's outcome.
#[test]
fn a_reader_after_a_cut_codeunit_between_it_and_a_complete_writer_keeps_its_outcome() {
    let src = r#"
codeunit 60800 "Quick First"
{
    Subtype = Test;
    [Test] procedure Quick() begin end;
}
codeunit 60801 "Whole Writer"
{
    Subtype = Test;
    [Test] procedure Writes() var S: Codeunit "Memory Threads Count"; begin S.Bump(); end;
}
codeunit 60802 "Cut Plain"
{
    Subtype = Test;
    [Test] procedure First() begin end;
    [Test] procedure Later() begin end;
}
codeunit 60803 "Late Reader"
{
    Subtype = Test;
    [Test] procedure AFailsAlone() begin Error('plain failure'); end;
    [Test] procedure BReads() var S: Codeunit "Memory Threads Count"; begin if S.Get() < 0 then Error('never'); end;
}
"#;
    let r = runs(src, &strict(), &["Quick", "Writes", "AFailsAlone", "BReads"], "First");
    r.cut(60802, "Later");
    assert_eq!(r.stopped(60801, "Writes"), "Pass");
    assert_eq!(r.stopped(60803, "AFailsAlone"), "Fail(\"plain failure\")");
    assert_eq!(r.stopped(60803, "BReads"), "Pass");
}

/// The reader is a member of the carry chain that keeps the walk's single instances (its OnRun takes a number of the
/// install code's sequence, ADR-066 amendment 8): the lane never runs it again, so its failure stays.
#[test]
fn a_chain_member_reader_keeps_its_failure_after_a_cut_writer() {
    let reader = r#"
codeunit 60803 "Member Reader"
{
    Subtype = Test;
    trigger OnRun() var N: BigInteger; begin N := NumberSequence.Next('SHARED'); end;
    [Test] procedure Reads()
    var S: Codeunit "Memory Threads Count";
    begin
        if S.Get() <> 1 then Error('no earlier writer');
    end;
}
"#;
    let src = format!("{QUICK}{WRITER}{reader}");
    let r = runs(&src, &strict(), &["Reads"], "First");
    assert_eq!(r.complete(60803, "Reads"), "Fail(\"no earlier writer\")");
    r.cut(60801, "Writes");
    assert_eq!(r.stopped(60803, "Reads"), "Fail(\"no earlier writer\")");
}

/// With `--max-replay 1` a complete run skips every re-run (a chain holds the reader and a writer at least) and keeps
/// the reader's failure, so the stopped run keeps it too.
#[test]
fn a_max_replay_of_one_keeps_the_reader_failure_after_a_cut_writer() {
    let opts = RunOptions {
        max_replay: Some(1),
        ..strict()
    };
    let src = format!("{QUICK}{WRITER}{READER}");
    let r = runs(&src, &opts, &["Quick", "Reads"], "First");
    assert_eq!(r.complete(60803, "Reads"), "Fail(\"no earlier writer\")");
    r.cut(60801, "Writes");
    assert_eq!(r.stopped(60803, "Reads"), "Fail(\"no earlier writer\")");
}

/// `replay_every_dependent` (the stability run's base variant): every dependent codeunit takes its result from its
/// serial re-run, a passing one too, so a passing reader after a cut writer is not run.
#[test]
fn replaying_every_dependent_reports_a_passing_reader_after_a_cut_writer_not_run() {
    let reader = r#"
codeunit 60803 "Pass Reader"
{
    Subtype = Test;
    [Test] procedure Reads() var S: Codeunit "Memory Threads Count"; begin if S.Get() > 5 then Error('never'); end;
}
"#;
    let opts = RunOptions {
        replay_every_dependent: true,
        ..strict()
    };
    let src = format!("{QUICK}{WRITER}{reader}");
    let r = runs(&src, &opts, &["Quick", "Reads"], "First");
    assert_eq!(r.complete(60803, "Reads"), "Pass");
    r.cut(60801, "Writes");
    if r.ordered {
        assert!(r.stopped(60803, "Reads").contains(LIMIT), "{:?}", r.stopped);
    }
}
