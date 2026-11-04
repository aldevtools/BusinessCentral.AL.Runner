# ADR-007: Oracle source, emulated version, and the passing-set ratchet

- Status: Accepted. Amended by ADR-015: one oracle and one ratchet per BC major, under `oracle/<major>/` and `metrics/<major>/`. Amended below: an oracle may be recorded for an older corpus commit when the pin only adds tests. Amended by ADR-065 (2026-11-04): the ratchet is the list of the cases the runtime does not pass yet, `metrics/<major>/not-passing*.txt`.
- Date: 2026-10-13

## Context
The runtime is measured against real BC. The corpus repository already runs
every test on real BC service tiers (BC 27.0 through 29.0) in GitHub Actions
and uploads a JUnit report per version. No BC container is available to agents
in this project's sessions.

## Decision
- **Oracle**: the corpus CI's `junit-test-results-27.5` artifact for the exact
  corpus commit the `corpus` submodule is pinned to, stored in `oracle/`. The
  CLI refuses to run (exit 2) if the pin and `oracle/oracle.toml` disagree.
- **Emulated version**: BC 27.5, the lowest version in the original spike
  matrix. Preprocessor symbols are those the corpus CI defines for 27.5:
  `BC27PLUS`.
- **Probes**: when behaviour is unclear, write the probe as a corpus test on a
  branch of the corpus repo and read its result from that branch's CI run.
  Once merged there, bump the submodule and the oracle together.
- **Ratchet** (ADR-065 since 2026-11-04): `metrics/<major>/not-passing*.txt` lists, per BC major and test
  app, the oracle cases real BC passes and the runtime does not pass yet. CI fails when a case that is not
  listed does not pass, when a listed case passes or left the oracle (`al-test-cli --prune-not-passing`
  deletes the line), and when an oracle case gets no outcome. Adding a line for a case the oracle already
  had needs the owner's OK. Failures still gate only on regressions: a case the runtime never passed is
  listed, not a failure. (Until 2026-11-04 the ratchet was `metrics/<major>/passing.txt`, the cases the
  runtime passes, grown by `--update-baseline`.)

## Consequences
- Oracle refresh is a two-file change (submodule + `oracle/`), easy to review.
- A probe round-trip costs one corpus CI run (~20 min) instead of a local run.

## Conformance
`al-test-cli/tests/corpus_harness.rs`: pin mismatch exits 2; a case not in the not-passing list that does
not pass exits 1 (the rest of the rule: ADR-065, Conformance).

## Decided without owner review (2026-10-13/14)

Three decisions came up when the corpus pin moved from `cf6e546` to `cdf315a`, which adds
about 150 probe tests (223 cases) and changes no existing test file.

1. **An oracle may be recorded for an older corpus commit.** A `[[version]]` table in
   `oracle/oracle.toml` can carry its own `corpus_commit`. The CLI (`pin::check`) accepts it when
   `git diff --name-status --no-renames <recorded> <pin> -- tests/` shows only added files. Any
   modified, deleted or renamed file under `tests/` refuses the run (exit 2) and names the files.
   A recorded commit missing from the checkout is a setup error that says to fetch the history
   (CI runs `git fetch --unshallow` in the corpus checkout). Cases that exist in the corpus but
   not in a version's oracle do not count for that version: they are not in its report, its
   ratchet or its totals.
   - Why now: the 29.0 run on the pin is unusable. The corpus workflow's dispatch path builds the
     preprocessor symbols with `for v in 27 28` and never adds `BC29PLUS`, so four existing
     `BC29PLUS`-guarded tests failed on real BC. BC 29 stays on `cf6e546` until the workflow is
     fixed and a 29.0 run on the pin exists (`docs/owner-review.md`).
   - Options: (a) wait for the workflow fix and keep all versions on one commit (stalls the bump);
     (b) take the unusable 29.0 run as the oracle (it would mark four tests as failing on BC when
     they pass; a weakened oracle, refused by hard rule 2); (c) this rule.
   - Why sound: an added file cannot change what an existing test does, so the old oracle still
     describes every case it lists. The runtime also compiles the new tests for BC 29, so a new
     test that breaks an old one would show as a regression of the ratchet.
2. **Corpus failures gate only on regressions.** `al-test-cli` exits 1 only when a case in
   `metrics/<major>/passing.txt` stops passing. A case real BC passes and the runtime fails (or
   refuses) is counted as `fail` or `not run` in the report and does not turn CI red. This was
   already how the gate worked; it is now stated, and covered by a harness test. The new probe
   tests the runtime does not pass yet are the work list for the next fix rounds. They are never
   added to the baseline, so nothing wrong is marked as passing and nothing is hidden.
   - Options: (a) this; (b) make `fail > 0` red (CI would stay red for a long time after every
     probe batch, and a red CI hides real regressions); (c) an allowlist of known failures (a
     second file to keep in step with the baseline, with no more safety than (a)).
3. **The version comparison covers shared cases only.** The runtime table of `--all-versions`
   (ADR-015 3.6) compares the cases every version's oracle has. A case one oracle lacks cannot
   show a version difference. The oracle table still lists such cases, so the gap stays visible.

### Decided without owner review (2026-10-14): other test apps

4. **More test apps, each with its own oracle and ratchet.** `oracle.toml` takes
   `[[version.suite]]` tables: the OnPrem app (the small artifact of the same CI run) and the
   isolation probe. Each suite has its own oracle file, `metrics/<major>/passing-<name>.txt` and
   report line; none counts toward `overall`. `--update-baseline` writes all of them and refuses
   while any regresses.
   - The isolation probe has no Linux CI result. Its oracle is the Windows nightly's XUnit
     (BC 28.4, corpus `master` at `cf6e546`, copied unchanged), accepted under the same rule as an
     older oracle: the corpus diff since `cf6e546` adds files and changes none. Only BC 28 has one.
   - A suite is a runner run: the runner codeunit's `TestIsolation` property selects how it is
     run. The same test codeunit can be in two suites (the probe under both runners).
   - A control case real BC fails on purpose (the probe's `T5` under Codeunit isolation) is
     excluded from the pass count, and the report says whether the runtime fails it too.
   - Options: (a) this; (b) put the extra cases into the main oracle (the codeunit id ranges do not
     overlap, but one codeunit under two runners has two answers, and the main file is a CI
     artifact that must stay byte for byte); (c) leave the apps out (their behaviour would stay
     unmeasured).

## Conformance (added)
`corpus_harness.rs`: an older recorded commit with changed tests is refused; an unknown one is a
setup error; a failing case outside the baseline exits 0. `pin.rs` unit tests cover add, modify,
delete, rename and non-`tests/` changes against throwaway git repositories.
