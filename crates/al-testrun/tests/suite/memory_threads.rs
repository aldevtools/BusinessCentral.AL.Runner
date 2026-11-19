//! ADR-066 amendment 10: the run starts its test threads one at a time, only for work it can use and only while memory
//! has room; a thread with no room ends; a hard limit stops the run instead of going over, and the tests it did not
//! reach are not run. Which thread runs a codeunit, and when, never changes an outcome.

use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::runtime::memory::MemoryBudget;
use al_testrun::runtime::{CorpusRuntime, Manifest, RunOptions, SuiteRun};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

const BASE: &str = r#"
codeunit 50271 MemoryThreadsInstaller
{
    Subtype = Install;
    trigger OnInstallAppPerCompany() begin NumberSequence.Insert('SHARED'); end;
}
codeunit 50272 "Memory Threads Count"
{
    SingleInstance = true;
    procedure Bump() begin Count += 1; end;
    procedure Get(): Integer begin exit(Count); end;
    var Count: Integer;
}
"#;

/// Chain members that use SingleInstance state: the OnRun of each takes a number of the install code's sequence (so
/// it is in the carry chain and keeps the walk's single instances, ADR-066 amendment 8; a member whose tests take the
/// number runs with fresh ones, amendment 11) and its test reads and writes a SingleInstance global.
fn single_chain(n: u32) -> String {
    (0..n)
        .map(|i| {
            let id = 60300 + i;
            format!(
                r#"
codeunit {id} "Single Chain {id}"
{{
    Subtype = Test;
    trigger OnRun() var N: BigInteger; begin N := NumberSequence.Next('SHARED'); end;
    [Test] procedure Bumps{id}()
    var I: Integer; W: BigInteger; S: Codeunit "Memory Threads Count";
    begin
        for I := 1 to 30000 do W += I;
        S.Bump();
    end;
}}
"#
            )
        })
        .collect()
}

/// Chain members that take numbers only from a sequence of their own, and codeunits
/// outside the chain (the parallel part), some of whose tests fail.
fn mixed() -> String {
    let mut src = String::new();
    for i in 0..8 {
        let id = 60400 + i;
        src.push_str(&format!(
            r#"
codeunit {id} "Own {id}"
{{
    Subtype = Test;
    trigger OnRun() begin end;
    [Test] procedure TakesOwn{id}()
    var I: Integer; N: BigInteger;
    begin
        for I := 1 to 50000 do N += I;
        NumberSequence.Insert('OWN');
        if NumberSequence.Next('OWN') <> 0 then Error('the sequence of an earlier codeunit was not rolled back');
    end;
}}
"#
        ));
    }
    // A writer of SingleInstance state and a reader that needs it: the reader passes only in its serial re-run after
    // the writer (ADR-066 amendments 4 and 5), so a stopped run that skips that re-run must not report its failure.
    src.push_str(
        r#"
codeunit 60600 "Single Writer"
{
    Subtype = Test;
    [Test] procedure Writes() var S: Codeunit "Memory Threads Count"; begin S.Bump(); end;
}
codeunit 60601 "Single Reader"
{
    Subtype = Test;
    [Test] procedure Reads()
    var S: Codeunit "Memory Threads Count";
    begin
        if S.Get() <> 1 then Error('no earlier writer');
    end;
}
"#,
    );
    for i in 0..12 {
        let id = 60500 + i;
        src.push_str(&format!(
            r#"
codeunit {id} "Plain {id}"
{{
    Subtype = Test;
    [Test] procedure Passes{id}()
    var I: Integer; N: BigInteger;
    begin
        for I := 1 to 20000 do N += I;
    end;
    [Test] procedure Fails{id}()
    begin
        if {i} mod 3 = 0 then Error('fails in %1', {id});
    end;
}}
"#
        ));
    }
    src
}

fn app(id: &str, deps: &[&str], src: &str) -> AppSource {
    AppSource {
        id: id.into(),
        name: id.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![(format!("{id}.al"), src.into())],
    }
}

fn manifest(p: &str) -> Manifest {
    Manifest {
        publisher: p.into(),
        version: "1.0.0.0".into(),
        resources: vec![],
        include_source: false,
        target: String::new(),
        logo: None,
        description: None,
        layout_files: Vec::new(),
        id_ranges: Vec::new(),
        translations: false,
        package_data: false,
        package_data_files: Vec::new(),
        translation_files: Vec::new(),
        translation_source: None,
        runtime: None,
        app_json_keys: Vec::new(),
        platform_internals_visible_to: Vec::new(),
        symbols: vec![],
    }
}

fn run(src: &str, opts: &RunOptions) -> CorpusRuntime {
    // ADR-066 amendment 12: these runs measure the carry chain and the lane, which run in BC's order only.
    run_with(
        src,
        &RunOptions {
            strict_order: true,
            ..opts.clone()
        },
    )
}

fn run_with(src: &str, opts: &RunOptions) -> CorpusRuntime {
    CorpusRuntime::try_run_suites_with(
        vec![app("main", &["base"], src), app("base", &[], BASE)],
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        &[manifest("Me"), manifest("Microsoft")],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        opts,
    )
    .expect("the run starts")
}

/// A budget of `limit` MB whose resident memory is what `rss` says, in MB, at each read.
fn budget(limit: u64, hard: bool, rss: impl Fn() -> u64 + Send + Sync + 'static) -> Option<MemoryBudget> {
    Some(MemoryBudget {
        limit: limit << 20,
        source: "--max-memory".into(),
        rss: Arc::new(move || Some(rss() << 20)),
        hard,
    })
}

fn outcomes(rt: &CorpusRuntime) -> Vec<(u32, String, String)> {
    let mut v: Vec<_> = rt
        .results()
        .iter()
        .map(|r| (r.codeunit, r.test.clone(), format!("{:?}", r.outcome)))
        .collect();
    v.sort();
    v
}

/// The chain walk runs on one thread: a chain of members that use SingleInstance state starts no second thread.
#[test]
fn a_serial_chain_uses_one_thread() {
    let rt = run(&single_chain(30), &RunOptions::default());
    for r in rt.results() {
        assert_eq!(r.outcome, Outcome::Pass, "{}", r.test);
    }
    let memory = &rt.phases.memory;
    assert!(memory.most <= 1, "the walk's thread only: {memory:?}");
    assert_eq!(memory.at_end, 1, "{memory:?}");
}

/// Decision 7: the same outcome for each test with no budget, with one thread (a budget with never any room), and
/// with a budget tight enough that threads start and end again and again.
#[test]
fn outcomes_are_the_same_with_no_budget_one_thread_and_threads_that_end_and_start_again() {
    let src = mixed();
    let free = run(&src, &RunOptions::default());
    let one = run(
        &src,
        &RunOptions {
            memory: budget(1000, false, || 950),
            ..Default::default()
        },
    );
    assert_eq!(one.phases.memory.most, 1, "{:?}", one.phases.memory);
    let reads = Arc::new(AtomicU64::new(0));
    let flapping = run(
        &src,
        &RunOptions {
            // Every seventh read leaves no room: a thread that reads it after a codeunit ends, and another may start
            // on the next. Rare enough that the cost learned over the codeunits' own windows (a high percentile) stays
            // small, as the resident memory of a real run does.
            memory: budget(1000, false, move || {
                if reads.fetch_add(1, Ordering::SeqCst) % 7 == 0 {
                    950
                } else {
                    100
                }
            }),
            ..Default::default()
        },
    );
    // The original input: every third read leaves no room, so a third of the codeunits' windows show 850 MB of growth.
    // That is a process that spikes often: the learned cost is high and few threads run (the safe side). What the
    // test asserts for it is the outcome, which must not depend on how many threads ran.
    let reads = Arc::new(AtomicU64::new(0));
    let often = run(
        &src,
        &RunOptions {
            memory: budget(1000, false, move || {
                if reads.fetch_add(1, Ordering::SeqCst) % 3 == 0 {
                    950
                } else {
                    100
                }
            }),
            ..Default::default()
        },
    );
    assert_eq!(outcomes(&free), outcomes(&one));
    assert_eq!(outcomes(&free), outcomes(&flapping));
    assert_eq!(outcomes(&free), outcomes(&often));
    assert_eq!(free.results().len(), 8 + 2 + 24);
    // Whether threads start and end again here used to show in `most` and `ended`, because the chain's helper threads
    // started and ended with the flapping reads. The chain walk has no helpers any more (ADR-070, step 1), and the
    // parallel part's few codeunits learn their cost from one early spike, so which threads start is not stable here.
    // `memory.rs` and `threads.rs` unit tests pin when a thread starts and ends; the memory ADR (ADR-070, step 5)
    // re-specifies this run.
}

/// Decision 6: a hard limit stops the run before the resident memory passes it. Tests that finished keep their
/// outcome; the others are not run with the limit as the reason, never failed (also a test whose result would have
/// come from a serial re-run, and a chain member after one that was not reached); the report says why it stopped.
/// The stop comes at several points of the run.
#[test]
fn a_hard_limit_stops_the_run_and_the_tests_not_reached_are_not_run() {
    let src = mixed();
    let free = run(&src, &RunOptions::default());
    let expected: std::collections::HashMap<_, _> = outcomes(&free).into_iter().map(|(c, t, o)| ((c, t), o)).collect();
    assert_eq!(expected.get(&(60601, "Reads".into())).map(String::as_str), Some("Pass"));
    for room_for in [4, 10, 30, 100, 300] {
        let reads = Arc::new(AtomicU64::new(0));
        let stopped = run(
            &src,
            &RunOptions {
                // Room for the compile, the install baseline and `room_for` reads, then the process passes the limit.
                memory: budget(1000, true, move || {
                    if reads.fetch_add(1, Ordering::SeqCst) < room_for {
                        100
                    } else {
                        990
                    }
                }),
                ..Default::default()
            },
        );
        // A run that ends before the reads run out does not stop; the first ones always do.
        let halted = stopped.phases.memory.halted.clone();
        assert!(
            halted.is_some() || room_for > 10,
            "the run stopped after {room_for} reads"
        );
        assert!(halted.is_none_or(|h| h.contains("1000 MB (--max-memory)")));
        for (c, t, o) in outcomes(&stopped) {
            if !o.contains("memory limit (--max-memory 1000 MB)") {
                assert_eq!(
                    Some(&o),
                    expected.get(&(c, t.clone())),
                    "{t} (stopped after {room_for} reads)"
                );
            }
        }
        assert_eq!(
            stopped.results().len(),
            free.results().len(),
            "every test has an outcome"
        );
    }
}

/// A writer and a reader of SingleInstance state on two threads, the writer's codeunit slow before its write: the run
/// stops at its hard limit once the reader's test ended, while the writer's codeunit still runs (al-runner issue 8368,
/// where this order came from a busy machine). The reader failed in its parallel run, and a complete run gives it its
/// serial re-run after the writer, where it passes; the stopped run reached neither the write nor the re-run, so the
/// reader is not run, never failed (decision 6).
#[test]
fn a_hard_limit_before_an_earlier_writer_ran_does_not_report_the_reader_failed() {
    let src = r#"
codeunit 60800 "Quick First"
{
    Subtype = Test;
    [Test] procedure Quick() begin end;
}
codeunit 60801 "Slow Writer"
{
    Subtype = Test;
    [Test] procedure SlowFirst()
    var I: Integer; N: BigInteger;
    begin
        for I := 1 to 3000000 do N += I;
    end;
    [Test] procedure Writes() var S: Codeunit "Memory Threads Count"; begin S.Bump(); end;
}
codeunit 60802 "Plain Fail"
{
    Subtype = Test;
    [Test] procedure FailsAlone() begin Error('fails alone'); end;
}
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
    // The run stops once the reader's test ended: the writer's codeunit is then still in its slow test.
    let stopped_after_reads = |strict_order: bool| {
        let read = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let (seen, rss) = (read.clone(), read);
        let rt = run_with(
            src,
            &RunOptions {
                strict_order,
                memory: budget(1000, true, move || if rss.load(Ordering::SeqCst) { 990 } else { 100 }),
                hooks: al_testrun::events::Hooks {
                    on_test: Some(Arc::new(move |e| {
                        if e.test == "Reads" {
                            seen.store(true, Ordering::SeqCst);
                        }
                    })),
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        assert!(rt.phases.memory.halted.is_some(), "{:?}", rt.phases.memory);
        outcomes(&rt)
    };
    let of = |v: &[(u32, String, String)], id: u32| v.iter().find(|(c, ..)| *c == id).map(|(.., o)| o.clone());
    let free = outcomes(&run(src, &RunOptions::default()));
    assert_eq!(of(&free, 60803).as_deref(), Some("Pass"));
    let stopped = stopped_after_reads(true);
    for (c, t, o) in &stopped {
        if !o.contains("memory limit (--max-memory 1000 MB)") {
            assert_eq!(Some(o), of(&free, *c).as_ref(), "{t}");
        }
    }
    // A failure after the stopped writer that read no SingleInstance global keeps its outcome.
    assert_eq!(of(&stopped, 60802).as_deref(), Some("Fail(\"fails alone\")"));
    // Without BC's order nothing runs again: a complete run keeps the reader's parallel failure, and so does the
    // stopped run.
    let free = outcomes(&run_with(src, &RunOptions::default()));
    assert_eq!(of(&free, 60803).as_deref(), Some("Fail(\"no earlier writer\")"));
    assert_eq!(of(&stopped_after_reads(false), 60803), of(&free, 60803));
}

/// Decision 6 holds for the whole run, the compile included (owner, 2026-11-09: "if I say maximum, then that has to be
/// the maximum I can spend on this task"): a run whose resident memory passes the stop line while the apps compile
/// stops there, before the compile ends. Every test is not run with the limit as the reason, never failed, and the
/// message says that it stopped while compiling.
#[test]
fn a_hard_limit_stops_the_compile_and_every_test_is_not_run() {
    let src = mixed();
    let free = run(&src, &RunOptions::default());
    let stopped = run(
        &src,
        &RunOptions {
            memory: budget(1000, true, || 990),
            ..Default::default()
        },
    );
    let memory = &stopped.phases.memory;
    let halted = memory.halted.clone().expect("the run stopped");
    assert!(halted.contains("while the apps were compiled"), "{halted}");
    assert!(
        halted.contains("990 MB") && halted.contains("1000 MB (--max-memory)"),
        "{halted}"
    );
    assert_eq!(memory.after_compile, None, "the compile did not end: {memory:?}");
    assert_eq!(
        stopped.results().len(),
        free.results().len(),
        "every test has an outcome"
    );
    for (_, t, o) in outcomes(&stopped) {
        assert_eq!(o, "NotRun(\"memory limit (--max-memory 1000 MB)\")", "{t}");
    }
    // The tests are the ones a complete run reports, codeunit names included.
    let names = |rt: &CorpusRuntime| {
        let mut v: Vec<_> = rt
            .results()
            .iter()
            .map(|r| (r.app, r.codeunit, r.codeunit_name.clone(), r.test.clone()))
            .collect();
        v.sort();
        v
    };
    assert_eq!(names(&stopped), names(&free));
}

/// A soft limit (`serve`, `debug`) never stops the compile.
#[test]
fn a_soft_limit_does_not_stop_the_compile() {
    let src = mixed();
    let soft = run(
        &src,
        &RunOptions {
            memory: budget(1000, false, || 990),
            ..Default::default()
        },
    );
    assert!(soft.phases.memory.halted.is_none(), "{:?}", soft.phases.memory);
    assert!(soft.phases.memory.after_compile.is_some());
}

/// The compile under a hard limit runs in a rayon pool of its own (`compile_halt.rs`). Its threads get the stack of
/// the main thread the compile ran on before: the checker and the lowerer recurse once per operand of a chain of
/// binary operators, and Base Application 28 holds a text built from 1,374 `+` operands
/// (`IntelligentInfoManifest.Codeunit.al`, `DefaultManifestText`), which overflowed a pool thread's default 2 MB
/// stack while the compile cache was built.
#[test]
fn a_hard_limit_compiles_a_long_operator_chain() {
    let operands: Vec<String> = (0..2000).map(|i| format!("'p{i}'")).collect();
    let src = format!(
        r#"
codeunit 60700 "Long Chain"
{{
    Subtype = Test;
    [Test] procedure Chain()
    var T: Text;
    begin
        T := {};
        if StrLen(T) = 0 then Error('empty');
    end;
}}
"#,
        operands.join(" +\n            ")
    );
    // On a thread with the front end's stack, as `al-runner` runs it: only the compile's pool threads are under test.
    let outcomes = std::thread::Builder::new()
        .stack_size(al_testrun::RUNNER_STACK)
        .spawn(move || {
            let rt = run(
                &src,
                &RunOptions {
                    memory: budget(1000, true, || 100),
                    ..Default::default()
                },
            );
            assert!(rt.phases.memory.halted.is_none(), "{:?}", rt.phases.memory);
            outcomes(&rt)
        })
        .expect("spawn the runner thread")
        .join()
        .expect("the run ends");
    assert_eq!(outcomes, vec![(60700, "Chain".into(), "Pass".into())]);
}
