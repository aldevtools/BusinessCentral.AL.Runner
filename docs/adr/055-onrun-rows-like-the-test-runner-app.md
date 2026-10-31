# ADR-055: Report a test codeunit's OnRun as Microsoft's Test Runner app does, on every BC version

- Status: Accepted (owner, 2026-10-28)
- Date: 2026-10-28

## Context

A test codeunit may declare an `OnRun` trigger. BC runs it once, before the
codeunit's test methods. If it raises an error, BC runs no test method of the
codeunit. AL code cannot see how the runner reports this; only the result rows
show it. Rule 6 (ADR-009) still asks that the rows match real BC, because a user
compares them with the results of their own pipeline.

Al-runner issue 5748 (a user's test codeunit whose `OnRun` raises) led to the
gate `TestCodeunitOnRunRowFrom28` and the assumption `test-codeunit-onrun-rows`
(PR 5846). The gate rested on corpus writer RL's probe (corpus commit c69419f6,
`codeunit/lt-outside-onrun-error/RlOnRunError.al`), read from the corpus CI on
BC 27.5, 28.5 and 29.0:

- BC 27.5: no `OnRun` row; a codeunit whose `OnRun` raises has no rows at all.
- BC 28.5 and 29.0: a row `OnRun` of its own, failed with the error's text when
  `OnRun` raises (and then no rows for the test methods), else passed.

Al-runner issue 5859 showed that the split at BC 28 is a split between two
runners, not between two BC versions. The corpus CI calls the
MsDyn365Bc.On.Linux workflow `bc-test-from-source.yml` with `test_runner: auto`.
On BC 28 and later that runs a test codeunit without `[HandlerFunctions]` (the
probe codeunits have none) through the AL tool's `al runtests` against the
service tier's `/dev/TestRunnerHub`, without an AL test runner codeunit. On
BC 27 it runs every codeunit through Microsoft's Test Runner app (runner
codeunit 130450). So the BC 28 and 29 rows of the probe are the rows of
`al runtests`, not of the Test Runner app.

The reporter of issue 5859 ran BC 29.0 the way BcContainerHelper's
`Run-TestsInBcContainer`, and so AL-Go, runs tests: page 130455 "Command Line
Test Tool" with runner codeunit 130450 "Test Runner - Isol. Codeunit". There the
result has no `OnRun` row, and a codeunit whose `OnRun` raises is reported as
Skipped with no rows while the run stays green: the BC 27 shape. Codeunit 130454
"Test Runner - Mgt" treats the function names `''` and `OnRun` as the codeunit
itself, not as a test method. The measurement of issue 5748 (BC 28.4, through
codeunits 130456 and 130450) also found no method rows for a codeunit whose
`OnRun` raises.

Al-runner is its own runner. It does not run AL test runner codeunits; it
already models runner 130450 around each test method (`runner_reset.rs`).

## Decision

1. On every BC version, al-runner reports a test codeunit's `OnRun` as
   Microsoft's Test Runner app does: no `OnRun` row. A codeunit whose `OnRun`
   raises is skipped: it has no rows, and the run stays green.
2. The match is exact: no note in the console, JSON or JUnit names the skipped
   codeunit.
3. There is no option for the `al runtests` shape.
4. The gate `TestCodeunitOnRunRowFrom28` is deleted. The former BC 27 shape
   (`OnRunRows::Bc27`) is the default, named `OnRunRows::TestRunner`.
5. Until corpus writer SW's pins (the Test Runner app on 27.5, 28.5 and 29.0,
   `codeunit/lt-sw-onrun-runner/SwOnRunRunner.al`) land, the shape stays under
   the assumption `test-codeunit-onrun-rows`: a pass of a test in a codeunit that
   declares `OnRun` is a flagged pass. When the pins land they win; the code
   follows them where they disagree.

The corpus harness (`al-test-cli`) is not changed. It runs with
`OnRunRows::PerTest`, and the oracle leaves the runner's `OnRun` row out
(`oracle::RUNNER_PSEUDO_CASE`), so the hub shape the BC 28 and 29 oracle was
recorded under never reached the harness's rows. The TestRunnerHub rules the
harness keeps (`corpus.rs` `on_hub`, the gate
`TestRunnerHubRequiredIsolationFrom28`) are about isolation and what `OnRun`
writes, not about rows.

## Consequences

- Users who run through BcContainerHelper or AL-Go get the same rows from
  al-runner as from BC. A codeunit whose `OnRun` raises no longer fails the run.
- Users who run through `al runtests` see a different shape from BC 28 on. That
  runner is not the one Microsoft's pipelines use.
- A codeunit that has no rows may have been skipped. Nothing in the output says
  so, as in the Test Runner app.
- `--filter` and `--codeunit` no longer pick an `OnRun` row.

## Conformance

- `crates/alrunner-cli/tests/cli/` (onrun_stability.rs)
  `a_test_codeunit_whose_on_run_raises_is_skipped_like_the_test_runner_app_on_every_major`
  (27.5, 28.5, 29.0: no `OnRun` row, exit 0, the skipped codeunit is not named,
  the pass after a declared `OnRun` is flagged).
- `crates/alrunner-cli/tests/serve.rs` `run_tests_streams_no_on_run_rows`.
- The assumption `test-codeunit-onrun-rows` (`al-builtins/src/assumptions.rs`)
  awaits `tests/al-language/codeunit/lt-sw-onrun-runner/SwOnRunRunner.al`;
  `al-test-cli --awaiting-assumptions` fails once that file is in the pinned
  corpus and the entry is still there.
