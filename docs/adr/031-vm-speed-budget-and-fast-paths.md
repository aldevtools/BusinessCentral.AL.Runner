# ADR-031: VM speed budget in CI and interpreter fast paths

- Status: Accepted (2026-10-15, decided by the orchestrator on the owner's delegation; see docs/owner-review.md)
- Date: 2026-10-15
- Related: ADR-009 (internals are free), ADR-011 decision 4 (bytecode only when a budget needs it), review
  `docs/reviews/2026-10-compile-ir-review.md` (R3, R4), plan `docs/reviews/2026-10-rework-plan.md`

## Context

CI has budgets for the compile and the parser only (`docs/architecture.md`, Budgets). Nothing fails when the
interpreter gets slower. ADR-011 decision 4 ties a faster executable form to budgets that do not exist for
compute-heavy code.

The review measured six benchmarks (Integer loop, Decimal loop, calls, text builtins, math builtins, record
operations). One iteration of `s := (s + i) mod N` costs about 2 700 instructions. The causes are overhead, not AL
semantics:

- Integer arithmetic and Integer to Integer conversion go through `rust_decimal` (`al-builtins/src/ops.rs`).
- `native::lookup` scans about 750 string ids in order on every builtin call (`al-builtins/src/native.rs:555`).
- `Session::eval` matches builtin ids against literal strings and prefixes before the lookup
  (`al-vm/src/lib.rs`, the `Expr::Builtin` arm).
- Constants are rebuilt on every evaluation: `Decimal` parsed from a string, a new `Arc` per text and Option
  constant (`al-vm/src/lib.rs`, `constant`).
- Each call looks up the publisher table, scans attributes for `TryFunction`, `CommitBehavior` and
  `ErrorBehavior` by string, and scans table properties for `TableType` for each record local.
- `Result<Value, Signal>` is 72 bytes, because `Signal::Error` holds an `AlError` with three `String`s inline.
- `Catalog::load()` parses the catalog on every call (about 34 call sites).

A scratch experiment with three of the fixes (Integer fast path, identity conversion, hashed lookup) cut
instructions by 30 % over the six benchmarks.

## Decision

1. **A VM speed budget in CI.** The benchmark app (review appendix A) moves into the repository as a test app
   (`crates/al-test-cli/tests/vm-bench/`, our own code, no Microsoft source). An `#[ignore]`d release test
   (`crates/al-test-cli/tests/perf_vm.rs`) runs each benchmark and counts user-space instructions per iteration
   (`perf_event_open` through the `perf-event` crate where the kernel allows it; wall time under
   `flock /tmp/al-perf.lock` otherwise). Limits are set at today's value plus 10 % when the budget lands, and
   lowered by each fast path that merges. CI runs it in the existing release-tests job.
2. **Fast paths are internals (ADR-009) and need no further ADR**, one per commit, each with a red/green unit
   test, citing the corpus test that pins the behaviour it must keep:
   1. Integer and Integer arithmetic and comparison in native `i32` with the same overflow and division-by-zero
      errors (`session-system/TestProbeSessionMiscContracts.al:Arithmetic_ErrorTexts`).
   2. Identity conversions return the value; lowering emits no `Convert` when source and target `RtType` are
      equal.
   3. Builtin lookup by a `HashMap` built once; the VM's own builtins (`Codeunit.Run`, `Report.*`,
      `Session.BindSubscription`, ...) registered in the same table, so `eval` has one path.
   4. Constants built once per program (a side table built in `Engine::with_config`, indexed by node; ADR-034
      moves it into the executable form).
   5. Routine facts computed once per program: is a publisher, is a `TryFunction`, `CommitBehavior`, collects
      errors. (Table facts such as temporary, CRM and an unevaluable `InitValue` belong to ADR-035 decision 4, so the
      two lanes do not edit the same code.)
   6. `for` over an Integer counter without the `BigInteger -> convert -> write -> read` round trip; no `RtType`
      clone per iteration.
   7. `Catalog::load()` behind a `OnceLock`; the hash of the executable computed once per process.
3. **One seam change:** `Signal::Error(Box<AlError>)`. Every `Err(Signal::Error(e))` pattern changes once. The
   variant names and the meaning stay.

## What changes at which seam

- `al-builtins::native::Signal`: `Error` holds a `Box<AlError>` (decision 3). Mechanical change in `al-builtins`,
  `al-vm`, `al-testrun`.
- `al-ir`: no change. Item 2.2 only drops redundant `Convert` nodes; the program differs in those nodes, which is
  why it is proven by outcomes, not by program equality.
- Everything else is inside `al-builtins` (`ops.rs`, `native.rs`, `catalog.rs`) and `al-vm` (`lib.rs`).

## Proof of zero behaviour change

- Corpus: every BC major, outcomes identical to the run before (`--junit`, compared with
  `tools/compare_junit.py`), not only the ratchet.
- Two Microsoft buckets (Tests-ERM and Tests-SCM) with test data: outcomes identical.
- Items 2.1 and 2.2: a property test that compares the fast path with the `Decimal` path for random `i32`
  operands, including `i32::MIN`, `-1` and `0`.
- `tools/demand_vs_eager.sh` and `tools/cache_vs_uncached.sh`: 0 differences (item 2.2 changes lowering).

## Migration (each step merges green)

1. The benchmark app and `perf_vm.rs` with today's limits.
2. Items 2.1 to 2.3 (the measured 30 %), one commit each; tighten the limits.
3. Items 2.4 to 2.7 and decision 3, one commit each; tighten the limits.

## Consequences

- Unblocks: ADR-034 (an executable form needs this budget as its trigger and its measure).
- Blocks nothing.
- Expected gain: 2 to 3 times fewer instructions on compute-heavy code (30 % measured, the rest estimated from
  profile shares).

## Conformance

`perf_vm.rs` (release, CI). The property tests of decision 2. The corpus and bucket outcome comparison is part of
each merge.
