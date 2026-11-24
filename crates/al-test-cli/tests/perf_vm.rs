//! The VM speed budget (ADR-031): instructions per iteration of eight small programs that run in the interpreter.
//!
//! The programs are the test codeunits of `tests/vm-bench` (our own AL code): an Integer loop, a Decimal loop, a procedure
//! call, text builtins, math builtins, record operations, `RecordRef.FieldIndex` on a table with FlowFields and a byte
//! loop of `OutStream.Write` and `InStream.Read` on a temporary Blob. Each runs
//! on one thread; the test counts the user-space instructions that thread executes while the tests of the codeunit run
//! (`perf_event_open`), subtracts what an empty test codeunit costs (the session and the engine), and divides by the
//! iterations. Instructions, not seconds: they do not move
//! with the load of the machine, so the budget can be tight (10 % over the value of the day it was set) and still not flap.
//!
//! Where the kernel does not allow the counter (a virtual machine without a PMU, `perf_event_paranoid` above 2) the test
//! falls back to the CPU time of the thread, with limits three times as wide, and says so. In that mode an excess only warns (owner, 2026-10-20). A fast path that merges lowers
//! its limit in the same commit. `AL_VM_BENCH_CPU_TIME=1` forces the fallback (to set or check its limits).
//!
//! Ignored by default (a release build is needed):
//!
//! ```text
//! cargo test --release -p al-test-cli --test perf_vm -- --ignored --nocapture
//! ```

use al_sema::AppKind;
#[path = "support/meter.rs"]
mod meter;

use al_test_cli::runtime;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use meter::Meter;
use std::path::{Path, PathBuf};

/// One benchmark: the test codeunit, the iterations of its loop (the unit of the limits; the number in the AL file), the
/// limit in instructions per iteration, and the wall budget in CPU nanoseconds per iteration for the fallback.
struct Bench {
    name: &'static str,
    codeunit: u32,
    iterations: u64,
    limit_instructions: u64,
    limit_cpu_ns: u64,
}

/// Limits: the measured value of the day plus 10 % (instructions), and three times the CPU time of this machine (fallback).
/// Each fast path of ADR-031 that merges lowers them. CallLoop, TextLoop and BuiltinLoop were set on 2026-10-19 from 2295,
/// 5342 and 5697 (spare value vectors for calls, the short path of a builtin whose operands are all values); the others
/// are older and lower than their value plus 10 %. FieldIndex was set on 2026-10-27 from 4331 (al-runner issue 5911: the
/// order of a table's fields worked out once, not on every call; it was 38861 before). StreamByteLoop was set on 2026-11-22 from
/// 4487 (one iteration is a Write and a Read of one byte): it guards the stream byte path of the GZip trailer codeunit 75290,
/// whose wall budget in perf_budget.rs only catches a codeunit several times slower.
const BENCHES: &[Bench] = &[
    Bench {
        name: "IntLoop",
        codeunit: 70002,
        iterations: 300_000,
        limit_instructions: 534,
        limit_cpu_ns: 1000,
    },
    Bench {
        name: "DecimalLoop",
        codeunit: 70003,
        iterations: 100_000,
        limit_instructions: 1821,
        limit_cpu_ns: 1000,
    },
    Bench {
        name: "CallLoop",
        codeunit: 70004,
        iterations: 100_000,
        limit_instructions: 2525,
        limit_cpu_ns: 2300,
    },
    Bench {
        name: "TextLoop",
        codeunit: 70005,
        iterations: 30_000,
        limit_instructions: 5876,
        limit_cpu_ns: 4400,
    },
    Bench {
        name: "BuiltinLoop",
        codeunit: 70006,
        iterations: 50_000,
        limit_instructions: 6267,
        limit_cpu_ns: 6000,
    },
    Bench {
        name: "RecordLoop",
        codeunit: 70007,
        iterations: 15_000,
        limit_instructions: 16261,
        limit_cpu_ns: 9500,
    },
    Bench {
        name: "FieldIndex",
        codeunit: 70008,
        iterations: 24_000,
        limit_instructions: 4764,
        limit_cpu_ns: 1000,
    },
    Bench {
        name: "StreamByteLoop",
        codeunit: 70009,
        iterations: 100_000,
        limit_instructions: 4936,
        limit_cpu_ns: 1100,
    },
];
const EMPTY: u32 = 70001;
/// Runs per benchmark; the least counts (a run that was disturbed counts more, never less).
const RUNS: usize = 3;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// The cost of running the tests of `codeunit` (compile excluded) and whether they passed.
fn cost(meter: &Meter, codeunit: u32) -> (u64, bool) {
    let app = runtime::load_app(&root().join("crates/al-test-cli/tests/vm-bench")).unwrap();
    let manifest = runtime::load_manifest(&root().join("crates/al-test-cli/tests/vm-bench")).unwrap();
    let opts = RunOptions {
        only: Some(vec![(0, codeunit)]),
        ..Default::default()
    };
    let compiled = CorpusRuntime::compile_suites(
        vec![app],
        vec![AppKind::Workspace],
        &[manifest],
        &Default::default(),
        None,
        &[SuiteRun::everything()],
        &opts,
    )
    .unwrap();
    // A single test codeunit runs on the calling thread, which is the one the meter watches.
    let (rt, count) = meter.measure(|| CorpusRuntime::run_compiled(&compiled, None, Default::default(), &opts));
    let passed = !rt.results().is_empty() && rt.results().iter().all(|r| r.outcome == Outcome::Pass);
    (count, passed)
}

fn least(meter: &Meter, codeunit: u32) -> u64 {
    (0..RUNS)
        .map(|_| {
            let (count, passed) = cost(meter, codeunit);
            assert!(passed, "the benchmark codeunit {codeunit} did not pass");
            count
        })
        .min()
        .unwrap()
}

#[test]
#[ignore = "perf budget: run with `cargo test --release -p al-test-cli --test perf_vm -- --ignored --nocapture`"]
fn the_interpreter_stays_within_its_instruction_budget() {
    if cfg!(debug_assertions) {
        panic!("perf budgets are only meaningful in --release; this is a debug build");
    }
    let meter = Meter::new(false);
    let counting = matches!(meter, Meter::Instructions(_));
    meter::print_mode(counting);
    eprintln!("vm budget: counting {}", meter.name());
    let empty = least(&meter, EMPTY);
    let mut over = Vec::new();
    for b in BENCHES {
        let total = least(&meter, b.codeunit);
        let per = total.saturating_sub(empty) / b.iterations;
        let limit = if counting { b.limit_instructions } else { b.limit_cpu_ns };
        eprintln!(
            "vm budget: {:<12} {:>8} per iteration (limit {:>8}), {} for {} iterations, {} for an empty codeunit",
            b.name, per, limit, total, b.iterations, empty
        );
        if limit != 0 && per > limit {
            over.push(format!("{} used {per} per iteration, limit {limit}", b.name));
        }
    }
    meter::enforce(
        counting,
        !over.is_empty(),
        &format!("over the VM budget ({}): {}", meter.name(), over.join("; ")),
    );
}
