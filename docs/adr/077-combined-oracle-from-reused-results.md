# ADR-077: The oracle may combine fresh and reused BC results (amends ADR-007)

- Status: Accepted (owner, 2026-11-17)
- Date: 2026-11-17
- Amends: [ADR-007](007-oracle-and-ratchet.md), the oracle source and its amendment of 2026-11-05 (`--from-pr`).

## Context

ADR-007 says the oracle is the corpus CI's `junit-test-results-<version>` artifact "for the exact corpus commit the
`corpus` submodule is pinned to". Every master push of the corpus runs every test today, so that artifact holds one
fresh BC result per test.

The owner decided on 2026-11-17 that corpus master pushes stop running every test. A master run will run the tests its
change can affect (the selection that pull requests already use) and take every other result from the last green
master run on the same BC build. A full run happens when Microsoft's BC build changes, every night, when the selection
says so, and after 20 subset runs in a row. The artifact of a master run then holds results from several runs and
several corpus commits. The corpus switches at once, without a shadow period in which master runs stay full and only
compute the combined file (owner, 2026-11-17).

Two rules of ADR-007 already go in this direction:

- An oracle may be recorded for an older corpus commit when the pin only adds tests (2026-10-13).
- A bump may merge the results of a corpus pull request's own run into the current oracle (`--from-pr`, 2026-11-05).

Facts read from the code on 2026-11-17:

- `al-test-cli/src/oracle.rs`, `parse_junit`, skips XML elements it does not know.
- `parse_clocks` starts each test codeunit's clock from the `timestamp` of its `<testsuite>`.
- `oracle.toml` has `bc_build` per version. It is the build the runner loads. Nothing records the build CI ran.
  The corpus pins the build for 29.0 only; 27.5 and 28.5 take the newest build of the minor, in each shard on its own.

## Decision

1. **The oracle of a version is the combined JUnit of a green corpus master run**: fresh results of that run plus
   reused results of earlier green master runs. A full run is the case with no reused result.
2. **The file is still copied unchanged.** Nobody edits a JUnit file in this repository. The corpus CI is the only
   place that combines results.
3. **Every test codeunit carries its provenance.** Each `<testsuite>` has a `<properties>` child with
   `source_run`, `source_attempt`, `source_commit`, `bc_build` and `app` (`full` or `partial`). A codeunit always
   runs whole, so its tests share one provenance. A reused suite keeps its `timestamp`.
4. **The run carries a manifest** (artifact `results-manifest`): commit, base run, and per version the BC build, the
   mode, the chain length and the nightly comparison. A green run without a manifest is a full run from before this
   change and is accepted as before.
5. **`tools/corpus_bump.sh` refuses the bump** unless all of this holds:
   - every test leg of the run succeeded, and the run is on master;
   - every suite has its properties, and every suite of one file names the manifest's BC build;
   - every `source_commit` is an ancestor of the run's commit;
   - for each reused suite, the files that declare its codeunit are identical between `source_commit` and the new pin;
   - the chain is at most 20 runs;
   - the newest nightly comparison at or before the run's commit does not report a difference;
   - no codeunit of the current oracle is missing, unless the corpus diff deleted or renamed its file.
6. **A nightly full run is a valid source while a comparison difference is open** (owner, 2026-11-17). All its
   results are fresh, so the comparison does not bear on them. The bump accepts it when every test leg succeeded and
   its manifest says `full` for every version, even when its own comparison reports a difference. Only combined runs
   stay blocked. The corpus therefore writes the manifest of a nightly run whether or not its comparison passes.
7. **`oracle.toml` records**, per version: `mode`, the counts of fresh and reused codeunits, the oldest source run,
   and `ci_bc_build`. The bump prints a warning when `ci_bc_build` differs from `bc_build`.
8. **The pin check does not change.** The CLI still exits 2 when the pin and `oracle.toml` disagree. The rule for an
   oracle recorded for an older commit (only added files) stays for suites that have no Linux CI result.
9. **The ratchet does not change** (ADR-065). A reused result counts like a fresh one.
10. **`--from-pr` is retired** (owner, 2026-11-17). It keeps working, unchanged, until corpus master runs reuse
    results. The first runner pull request after that switch deletes `tools/corpus_bump_pr.py`, its tests, the
    2026-11-05 sentence in ADR-007 and its paragraph in `docs/agents/workflow.md`. The corpus then owns the one rule
    that merges results.

## Options considered

- **(a) Take the oracle only from full runs (the nightly).** Simple and always fresh. The pin could then move once a
  day. Today it moves several times a day, and outside issues wait for it. Rejected as the only path; it stays possible
  (decision 6).
- **(b) Combine in the runner** (grow `--from-pr` into the general case). The merge rule would live in two
  repositories. Rejected: one behavior, one place.
- **(c) This decision.** The corpus combines, the runner checks what it can prove locally (ancestry, unchanged test
  files, one BC build, chain length) and trusts the corpus's nightly comparison for the rest.

## Consequences

- A reused pass can be wrong when the corpus selection missed an effect of a later change. The guards are the nightly
  full run with its comparison, the chain cap, and the file check of decision 5. The window is at most one day. This is
  a real cost against hard rule 2 ("never weaken an oracle") and the reason every guard refuses instead of warning.
- Without a shadow period, the first nightly comparisons are the first measurement of the selection on master. A
  difference blocks combined bumps until it is classified; nightly bumps (decision 6) keep the pin moving meanwhile.
- A corpus fixture may now change. That can change the runner's outcome for tests whose own code did not change.
  `--baseline-diff` and the ratchet catch it at the bump; the corpus pull request shows runner parity before the merge.
- The build CI ran becomes visible for the first time, and so does a difference to the build the runner loads.
- `oracle/README.md` no longer says that the files come from one run of one commit. It names the run and points to
  the properties in the file.
- No change to the CLI's reader, to the report formats or to the not-passing lists.

## Conformance

- `crates/al-test-cli/tests/suite/corpus_bump_combined.rs` (new, with a fake `gh` like `corpus_bump_script.rs`): one
  test per refusal of decision 5, each seen red first; a full run without a manifest is accepted; a nightly full run
  whose comparison differs is accepted and a combined run at that commit is refused (decision 6); a combined run is
  accepted and `oracle.toml` gets the fields of decision 7.
- `al-test-cli/src/oracle.rs` unit test: a suite with a `<properties>` child gives the same cases and the same clock
  as the suite without it.
- `corpus_harness.rs`: pin mismatch still exits 2.
