# Hand-off: area-test-isolation-state-1117 (after round 2)

## Done
- #8239 (round 1): partner login SingleInstance refusal narrowed to readers (`partner_login.rs`).
- #8152 follow-ups (round 2, commit "List NumberSequence readers..."):
  - Default run: `order_hint.rs` `numbers_order` also lists a codeunit whose tests or OnRun only read a sequence
    (`chain.rs` `reads_numbers`) after a codeunit whose tests take numbers (`NumbersOrder::reads_only`).
  - `--strict-order` with TestIsolation = Function: `order_hint.rs` `refuse_unmeasured_numbers` refuses a later
    codeunit that can read a sequence (or whose reach is unknown) after a Function-isolation chain member for
    `ChainWhy::TestsTakeNumbers`. Tests: `tests/suite/order_default.rs` (two new).

## Next, in order
1. #8238 (PROMISED). Prepared on LOCAL branch `area-test-isolation-state-1117-8238` (commit "WIP (not for the PR
   yet)", in this worktree, not pushed): `runtime/table_reads.rs` (`reads_only`, `READS`, `TABLE_EVENTS`, 4 tests),
   `blocked.rs` `subscribers_reached_reading` (the walk with a skip predicate; `subscribers_reached_from` calls it
   with `false`), `written.rs` `root` / `stmt_places` moved to module level. Still to do once the probe PR is open
   with local results (corpus branch al-runtime-rs-probe-test-isolation-1117; none existed at 20:22 on 2026-11-17):
   - Assumption file `crates/al-builtins/src/assumptions/table-event-not-raised-by-read.rs` with `awaiting`
     (issue 8238, the probe file). Narrow `READS` to the shapes the probe measures.
   - `install.rs` `touched`: compute the reach with `subscribers_reached_reading(.., &|o, r, t|
     table_reads::reads_only(lowered, o, r, t))` and with `subscribers_reached`; the tables only the full set holds go
     to a new set kept next to `skipped`. A test whose hits meet that set calls `assume(...)` (pattern:
     `CompanyOpenWrites::watch_hit` and `login-own-registration-reads-alone`). Flag every later test too.
   - Red test: the reporter's repro (issue body) as a Rust test in `install_unknown_tests.rs` style: stand-in table
     with an OnAfterDeleteEvent subscriber that writes through a RecordRef of unknown table; install stops; helper
     reads the table with IsEmpty. ReadsTheLoginRowOnly runs (flagged), ReadsTheInstallRow refused.
     Neighbors: Get + Modify(false) and Copy Company keep main's refusal.
   - Regenerate `docs/probes-needed.md` (`al-test-cli --root . --assumptions-doc`). Merge the branch into the PR branch.
2. The rest of the area: 8218, 8192, 8115, 8104, 8063, 8062, 8056, 7640, 7622, 7620, 7619, 7611, 7609 (none started).

## Open neighbors (not changed, main's behavior)
- `--strict-order --isolation required`: a hub-leg codeunit (Codeunit or Function) whose tests take numbers has no
  carry (`corpus.rs` `on_hub`), so a later reader gets the baseline value. Not measured. Probe wanted.
- Within one codeunit under `--isolation test` (builtin runner) the numbers stay after a test (`isolation.rs`
  `Checkpoint::restore` says "not measured there"). Probe wanted.
- A Function-isolation taker whose OnRun also writes a sequence (`ChainWhy::OnRunWritesState`) keeps main's result.

## Probes needed
- P1 (#8238): table events on reads (spec in the round-1 hand-back).
- P2 (#8239): cross-app memo state after a partner login.
- P3 (#8152): runner with TestIsolation = Function; codeunit A's test calls NumberSequence.Next twice on a sequence the
  install code made (seed 100); codeunit B (higher id) reads Current. Same on the TestRunnerHub with
  RequiredTestIsolation = Function and = Codeunit. Also two tests of one codeunit under Function.

## Traps
- `run_codeunit_inner` and several al-testrun files are near their length caps.
- Own target dir: CARGO_TARGET_DIR=<worktree>/target.
