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
   mode, the chain length and the nightly comparison. A manifest counts only when its `run_attempt` is the run's
   latest attempt and the run's conclusion is success: a later attempt replaces the legs' files but not an earlier
   manifest. A green run without a manifest is accepted as a full run from before this change only when none of its
   suites carries provenance properties.
5. **`tools/corpus_bump.sh` refuses the bump** unless all of this holds:
   - the run is green (conclusion success) and on master;
   - the manifest's `run_attempt` is the run's latest attempt;
   - every suite has its properties, and every suite of one file names the manifest's BC build;
   - every `source_commit` is an ancestor of the run's commit;
   - for each reused suite, the files that declare its codeunit are identical between `source_commit` and the new pin;
   - the chain is at most 20 runs;
   - the newest nightly comparison at or before the run's commit does not report a difference;
   - no codeunit of the current oracle is missing, unless the corpus diff deleted or renamed its file.
6. **The nightly full run is the one exception to decision 5** (owner, 2026-11-17). All its results are fresh, so an
   open comparison difference does not bear on them. The bump accepts a nightly run whose only failed job is the
   comparison, when every test leg succeeded and its manifest says `full` for every version. That run passes the
   first bullet and the comparison bullet of decision 5; every other bullet still applies. The exception names the
   nightly run only: a push run that is full for every version stays blocked while a difference is open. The corpus
   therefore writes the manifest of a nightly run whether or not its comparison passes.

   **A combined run needs the next nightly to have compared it** (owner, 2026-11-18). The first nightly run after a run
   that reuses results is, among the nightly runs whose commit is the run's commit or a descendant of it, the one
   created first. The run is accepted only when that nightly has ended green, has a manifest of its latest attempt,
   and for every BC version of the run reports `identical` and names this run as the run it compared
   (`comparison_detail.compared_run`, corpus PR 1440). So only the run that the nightly compared can be bumped as a
   combined run; a nightly that reports a difference blocks it. While no nightly after the run exists, the run is
   refused until the next nightly ends. A first nightly that is red, cancelled, reports a difference or compared another
   run keeps refusing the run, because the first nightly after the run does not change. Only a re-run of that nightly
   can make the run usable; otherwise the pin moves through a later run or a nightly run.
   A full push run and the nightly run itself keep the rules above.
7. **`oracle.toml` records**, per version: `mode`, the counts of fresh and reused codeunits, the oldest source run,
   and `ci_bc_build`. The bump prints a warning when `ci_bc_build` differs from `bc_build`.
8. **The pin check does not change.** The CLI still exits 2 when the pin and `oracle.toml` disagree. The rule for an
   oracle recorded for an older commit (only added files) stays for suites that have no Linux CI result.
9. **The ratchet does not change** (ADR-065). A reused result counts like a fresh one.
10. **`--from-pr` is retired** (owner, 2026-11-17). It keeps working, unchanged, until corpus master runs reuse
    results. The first runner pull request after that switch deletes every `--from-pr` path:
    `tools/corpus_bump_pr.py` and its tests, the `--from-pr` branch of `tools/corpus_bump.sh`, the PR-results record
    that `tools/corpus_bump_edit.py` removes and `oracle/README.md` holds, the paragraphs in `docs/agents/bump.md` and
    `docs/agents/workflow.md`, the path list entry in `.github/workflows/ci.yml`, and the 2026-11-05 sentence in
    ADR-007. The corpus then owns the one rule that merges results. Until then, the checks of decision 5 apply to a bump
    from a master run only. A `--from-pr` bump keeps its own checks: its suites carry a pull request's merge commit as
    `source_commit`, a commit on no branch, so the ancestry check cannot apply to them.

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
- A combined run is usable only after the next nightly compared it (decision 6, owner 2026-11-18), so combined runs
  move the pin at most once a day, the cost for which option (a) was rejected. Full push runs still move it at once.
  The bump bot lists green nightly runs as well as green push runs (owner, 2026-11-18), so the pin still moves when
  master does not move after a run that stays refused. The bump checks a nightly like any other run, so a green
  nightly is refused when a later nightly on the same commit reports a difference or ended red (decision 5, the newest
  nightly at or before its commit); the pin then waits for a later green nightly. A nightly without a manifest is
  refused (decision 4).
- A corpus fixture may now change. That can change the runner's outcome for tests whose own code did not change.
  `--baseline-diff` and the ratchet catch it at the bump; the corpus pull request shows runner parity before the merge.
- The build CI ran becomes visible for the first time, and so does a difference to the build the runner loads.
- `oracle/README.md` no longer says that the files come from one run of one commit. It names the run and points to
  the properties in the file.
- No change to the CLI's reader, to the report formats or to the not-passing lists.

## Conformance

- `crates/al-test-cli/tests/suite/corpus_bump_combined.rs` (new, with a fake `gh` like `corpus_bump_script.rs`): one
  test per refusal of decision 5, each seen red first; a run whose test legs all succeeded but whose manifest job
  failed is refused; a manifest of an earlier attempt is refused; a full run without a manifest and without provenance
  properties is accepted, and one without a manifest whose suites carry properties is refused (decision 4); a nightly
  full run whose only failed job is the comparison is accepted, while a combined run and a full push run at that
  commit are refused (decision 6); a combined run is accepted and `oracle.toml` gets the fields of decision 7.
- `al-test-cli/src/oracle.rs` unit test: a suite with a `<properties>` child gives the same cases and the same clock
  as the suite without it.
- `corpus_harness.rs`: pin mismatch still exits 2.
- `corpus_bump_combined.rs` (owner decision 2026-11-18): a combined run is refused when no nightly run at or after its
  commit exists, when the first one reports any answer but `identical`, has not ended, ended red or cancelled, has no
  manifest, has a manifest of an earlier attempt, lacks a version, or compared another run, an older, a later or no run,
  for any version, and when the oldest listed nightly is after it; a difference in a later nightly after an identical
  first one does not block it; a nightly at the run's commit counts as the first after it; a full push run keeps its
  answer whatever the nightlies after it say.
