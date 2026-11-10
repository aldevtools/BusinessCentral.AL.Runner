# ADR-070: Order modes: isolated by default, BC's order sequential on request

- Status: Accepted (owner, 2026-11-10)
- Date: 2026-11-10
- Supersedes: ADR-066 amendments 2, 3, 4, 5, 6, 7, 8, 11 and 12 (the order machinery). ADR-066 goes back to its subject:
  running the test runner codeunit's own AL. Amendments 9 and 10 (memory) stay in force until the memory ADR of
  migration step 5 replaces them.
- Amends: ADR-058 amendment 1, decision 3 (how the `isolation.singleInstance` sides are made).
- Source: design review `fable-review/adr066.md` (2026-11-09), accepted by the owner with its eight recommendations on
  2026-11-10.

## Context

BC runs all test codeunits of a run in one session, in order. al-runner runs each test codeunit on its own session, in
parallel. ADR-066 amendments 2 to 11 tried to give BC's in-order result anyway, by speculation: run-ahead helper
threads, a replay lane, fingerprints, guessed start states. Amendment 12 then made the fast isolated run the default and
kept all of that code behind `--strict-order`.

What this cost, from the amendments' own measurements:

- About 7,200 lines of runtime code and 3,100 lines of suite tests, 12 amendments in 5 days, 5 tuning constants, 3
  assumptions, 2 flags.
- Slow runs: a partner suite 2.8 times slower, Tests-SCM 245 s to 357 s, NP Retail 1354 s of 1684 s in re-runs that changed
  0 of 5494 outcomes, 12 threads slower than 1 thread on Tests-ERM-Sales.
- A benefit that showed up only in probes: on Tests-ERM-Purchase, Tests-SCM and NP Retail the machinery changed 0
  outcomes. Only the 7 corpus cases in `metrics/strict-order-files.txt` need it.

And `--strict-order` is not exact. Six gaps are known (numbers-only members with fresh state, the guard-then-refill rule,
an append counted as independent, lane sessions blind to other chains, chain members blind to codeunits outside the chain,
fresh state per member with `--test-suites`). A flag that claims BC's order and is approximate conflicts with hard rule 6.

## Decision

Two modes. No speculation in either.

### The modes

1. **Default: isolated.** As amendment 12 as merged. Each test codeunit starts from the install baseline with fresh
   SingleInstance state and baseline sequence values. All run in parallel. Methods of one codeunit share state in BC's
   order. Disabled isolation and OnRun writers still carry database rows and storage, walked serially (decision 2). The
   run states its mode in the header, in JUnit and in JSON (`orderMode: isolated`).
2. **`--strict-order`: one plain sequential run per BC session.** A BC session is what `single_lane.rs` `streams`
   defines: the suite and hub leg, or each line of `--test-suites`. Its codeunits run one after the other on one thread,
   in BC's order. SingleInstance state, database rows, sequence values and isolated storage move from codeunit to
   codeunit. Different BC sessions run in parallel with each other; that is the only parallelism, as on BC. The result is
   exact by construction (P7, `al-language/codeunits/TestCuP7BReader.al:24-37`). It needs no flag on passes. A later codeunit
   of a session that ran after a refused test, or after a test stopped by the time limit, is marked as a flagged pass
   ("state is not BC's"), because that session's state is no longer BC's. Documented: `--strict-order` runs each BC session
   on one thread. The run states its mode (`orderMode: bc`).

### The eight decisions

1. **How `--strict-order` runs.** Sequential, one thread per BC session, as above. Reached through a first PR that
   deletes run-ahead (migration step 1), not by building it first.
2. **OnRun-write carry in the default mode.** Rows and storage a test codeunit's OnRun trigger writes under runner 130450
   still reach later codeunits, as for Disabled isolation. They are database state that survives on BC across sessions,
   and few codeunits have a writing OnRun. The members of this chain walk serially, with no run-ahead. Re-check if a
   real suite shows a long serial chain from it.
3. **Numbers hint.** The static "number order" list of amendment 12 is dropped: on Microsoft's suites it names nearly
   every codeunit and carries no signal. In its place, a failed test gets a run-time hint when it took a number from a
   sequence that an earlier codeunit of its BC session also took from in this run (`numbers_taken.rs` already records
   what each codeunit took). Same shape as the SingleInstance hint, and it appears only when it can matter.
4. **Passing tests that read carried state.** A default run prints one summary line when the count is above 0 (without
   `--timings`): "N passing tests read SingleInstance state that an earlier codeunit would have left differently on BC.
   `--strict-order` runs BC's order." Each such test gets a JUnit property. This makes the green-here, red-on-BC case
   visible without a flag on each result. `--stability` still lists them.
5. **`metrics/strict-order-files.txt` stays honest.** Keep the list. Add a nightly job that runs the whole corpus with
   `--strict-order`: every case must pass there, and a case that passes in both modes may not be on the list. The corpus
   is the oracle for BC, and BC is strict order, so the full strict run must stay green somewhere.
6. **Memory.** Amendments 9 and 10 stay as they are until ADR-069 is built. Then re-measure and reduce to: a budget, a
   learned cost per codeunit, admission before a codeunit starts, and a hard stop with exit code 2. The hard limit
   (`--max-memory`) and exit code 2 stay in every case. This goes into its own ADR (step 5), not here.
7. **Stability dimension `isolation.singleInstance`.** Two plain runs: `fresh` is the default isolated run, `carried` is
   the sequential strict run. Diff them and attach the reason. The reason text names the globals read, says "took numbers
   after codeunit X" when numbers may be the cause, and says "cause not known" when neither applies. The "run the
   suspect again from its exact start" mechanism (`single_fresh.rs` `rerun_fresh`) goes.
8. **Where this is written.** This ADR, not amendment 13 on ADR-066.

### Rule for future changes

A design decision about order or threads needs one run at a time on a quiet machine, or instruction counts, not wall
times from a loaded machine. No new mechanism for BC's order unless a real suite shows a changed outcome, not only a
probe.

## Migration

Small PRs, each red/green. Outcomes do not change in steps 1 to 3 except where this ADR names a gap it closes.

1. **Delete run-ahead.** `chain_runs.rs`, the helper half of `chain_walk.rs` (the walk is always serial), `al-vm`
   `baseline/delta.rs`, `MISSES`, `RESTARTS`, `TRIES`, `WINDOW_PER_HELPER`, amendment 10 decision 4 (helper usability),
   amendment 11 (numbers-only run-ahead), `RunOptions::chain_wait_for_runs_ahead`, and the suite tests of run-ahead.
   Outcomes must not change in either mode (`--baseline-diff` on BC 27, 28 and 29, and the corpus strict files).
2. **Sequential strict run behind an internal option.** The walk follows `streams` order, every codeunit is a member,
   BC sessions run in parallel. Compare with the lane-based strict path on the corpus strict files, the stress suite
   and NP Retail. Differences are expected only at the six gaps.
3. **Switch `--strict-order` to it.** Delete lane re-runs (`Found::lane_for`, `Found::sessions`, `run_chains`),
   `lane_starts.rs`, `baseline/row_delta.rs`, `al-data` `row_changes.rs`, `--max-replay` and `maxReplay`,
   `replay_every_dependent`, and the assumptions `single-instance-order`, `single-instance-refill`,
   `single-instance-rerun-skipped`. Removing `--max-replay` and changing what `--strict-order` runs is a seam change;
   this ADR is the decision.
4. **Reports.** The run-time numbers hint (decision 3), the summary line and JUnit property (decision 4), `orderMode`
   in the header, JUnit and JSON, the flagged pass after a refused or time-stopped test of a strict session, two plain
   runs for stability (decision 7), the nightly full-corpus `--strict-order` job (decision 5), and manual and `--help`
   text (`--test-suites` only changes hints in the default mode).
5. **Memory ADR** after the ADR-069 re-measure (decision 6).

## Consequences

- `--strict-order` becomes exact and its code becomes the simplest in this area. About 4,100 lines of runtime code (about
  4,600 with the memory cut), about 1,700 lines of tests, 3 assumptions, 2 flags and 5 tuning constants go away over the
  steps (estimates from the review).
- Under `--strict-order` a BC session uses one core. Where run-ahead worked best it gained about 25 % (Tests-ERM-Purchase
  100 s with 12 threads, 135 s with 1). That is given up, in the opt-in mode only. The measurements in the review say the
  sequential run is as fast or faster on Microsoft's suites (Tests-ERM-Sales 152 s on one thread against 238 s on 12).
- The stability run gets slower: its carried variant runs in BC's order. Variants may run in parallel while memory allows.
- Hard rule 6 for the default mode: the default deviates from BC on purpose (owner, 2026-11-09). A test that passes on BC
  only because another codeunit ran first can fail here, and the hint says so. A test that fails on BC in full order can pass
  here, and the summary line of decision 4 says so.
- Correctness of the strict mode rests on the serial walk alone. The corpus strict files and the P7 cases pin it.

## Conformance

- `metrics/strict-order-files.txt` cases pass with `--strict-order` (each pass runs them once more, alone).
- `crates/al-testrun/tests/suite/order_default.rs` (default order, hint, no re-run) and the serial-walk tests of the
  carry chain.
- Step 2 adds a differential test of the sequential strict run against the lane-based path. The nightly job of decision 5
  runs the whole corpus with `--strict-order`.
