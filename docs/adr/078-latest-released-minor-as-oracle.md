# ADR-078: The oracle is the latest released minor of each major; minor-level preprocessor symbols

- Status: Accepted (owner, 2026-11-23, with the answers recorded in the decision)
- Date: 2026-11-23
- Related: [ADR-007](007-oracle-and-ratchet.md) (oracle and ratchet), [ADR-015](015-version-independent-runtime.md)
  (one runtime for every BC major: Q1 "newest minor", 3.4 gates, 3.9 `corpus_symbols`),
  [ADR-052](052-test-sources-and-matrix-runs.md) (matrix runs, Microsoft's tests),
  [ADR-065](065-not-passing-lists.md) (not-passing lists), [ADR-077](077-combined-oracle-from-reused-results.md)
  (combined oracle, build provenance)
- Amends: ADR-015 section 3.9 and the drift check of its Q1 answer; ADR-052 and ADR-077 by the notes at the end.

## Context

The corpus CI runs three legs: 27.5, 28.5 and 29.0. Each leg's JUnit is the oracle of its major (ADR-015 Q1). The
rule so far was to move a leg to a newer minor only when the newer minor gives identical results.

BC 29.1 was released. The corpus ran on it in a draft pull request (corpus PR 1528, run 45069150353). 63 tests passed on
29.0 and failed on 29.1. A rerun gave the same 63, so none is flaky. They fall into three classes:

| Class | Tests | What changed |
|---|---|---|
| 1. The probe pins a value it does not measure | 22 | A whole-table count, a feature caption, the Test Runner app's version prefix. |
| 2. Version data | 34 | Tenant Web Service has one more row (Master Data Management registers `MDMCrossEnvSource`); Aspose writes a longer version string into HTML. |
| 3. Behavior change | 7 | Page part refresh after an action (back to the 27/28 shape); GoToRecord after an external refresh; Word layouts with an external `attachedTemplate` are refused again (Microsoft KB 5135161, entry 654475). |

29.0 was the outlier: in several of these, 29.1 went back to the BC 27/28 behavior. Holding the oracle on 29.0 hid
this. Under the "identical" rule the 29 leg would never move.

The corpus can only tell majors apart. The CI defines `BC27PLUS`, `BC28PLUS` and `BC29PLUS` cumulatively
(corpus `.github/workflows/ci.yml`, job `prepare`). A probe cannot say "29.0 does this, 29.1 does that". The runner
copies this convention in `al_version::corpus_symbols(major)` (ADR-015 3.9). It calls it in `al-test-cli` (`main.rs`,
`mnc.rs`, `windows_language.rs`, `coverage.rs`). `tools/corpus-writer/compile-legs.sh` types the symbols per leg
(`"29.0 BC27PLUS,BC28PLUS,BC29PLUS -"`). `al-sema`'s `Workspace` reads one gate from the symbol set
(`USER_NAME_FIELDS_GATE`).

Code that already tells minors apart today does it without the preprocessor: `al-version` `awaited.rs`
(`bulk_commit_ends_cached_row` keys on `(29, 0)` and `(29, 1)`) and `gates.rs` (Feature Key rows). The
`gate_completeness` test only recognizes `BC<n>PLUS`.

## Decision

The owner decided points 1 and 2 on 2026-11-23 and accepted points 3 to 5 the same day, with four answers: minor
symbols come from one list of released minors (point 3); each oracle leg pins one exact build, and the sweep runs
the newest builds (points 2 and 5); a minor bump waits until the runner matches every behavior change (consequence
item 3); the runner does not run the sweep minors for now (point 2).

### 1. The oracle leg of a major is its latest released minor

- The corpus CI legs, and so the oracle, are the latest released minor of every supported major. Today that is
  27.5, 28.5 and 29.1.
- When Microsoft releases a new minor (29.2 is expected in early December 2026), the leg moves to it. The move is one
  corpus pull request, the minor bump pull request. It:
  1. changes the leg's version in the CI matrix and the pinned build (point 5);
  2. adds a version guard (point 3) for every test whose result differs, or rewrites the test so it no longer pins a
     value it does not measure (class 1);
  3. adds one entry per difference to the corpus docs of version changes: what changed, between which versions, and
     whether Microsoft documents it.
- The rule "switch only if identical" is gone. A difference is a reason for a guard, never a reason to stay on the old
  minor.
- A release candidate or insider build is never an oracle leg. "Released" means Microsoft publishes it as a sandbox
  artifact for production use.

### 2. A weekly sweep over every minor, for information only

- Once a week the corpus runs on every minor from 27.0 upward, plus insider builds of the next minor and the next
  major.
- The sweep is not the oracle. It never blocks a merge, a bump or a pull request. Its results are not copied into
  `oracle/`.
- Its results may be published in the public corpus repo, insider results included (owner, 2026-11-23).
- It always runs the newest build of every minor and of each insider build (owner: "the point of the weekly is to
  always run the latest"). The oracle legs pin one build each (point 5); the sweep does not.
- It replaces the drift check of ADR-015 Q1 (the other minors compared with the oracle leg of their major).
- The runner does not run the sweep's minors for now (owner, 2026-11-23). The corpus sweep is the only check of
  older minors.
- It shows the next minor's differences before the release, so the minor bump pull request can be prepared early.

### 3. Minor-level preprocessor symbols: `BC<major>_<minor>PLUS`

**Name form.** `BC29_1PLUS` means "BC 29.1 or later". It keeps the `BC…PLUS` shape that corpus authors know. The
underscore separates major and minor, so `BC2_91PLUS` and `BC29_1PLUS` cannot be confused. The AL compiler accepts it:
compiled on 2026-11-23 with AL compiler 18.1.43.24897 (BC 29), an app with `"preprocessorSymbols": ["BC29PLUS",
"BC29_1PLUS"]` and `#if BC29_1PLUS and not BC29_2PLUS` / `#elif BC29PLUS` builds, and removing the symbol switches the
branch. The runner's preprocessor (`al-syntax` `preprocessor.rs`, `eval`) splits on spaces and parentheses only, so it
reads the name as one symbol.

**No `_0` form.** `BC29PLUS` already means 29.0 or later. One name per version.

**One list of released minors** (owner, 2026-11-23). The corpus keeps one data file that lists every released
minor from 27.0 (today: 27.0 to 27.5, 28.0 to 28.5, 29.0 and 29.1). The minor bump pull request adds the new minor
to it. Nothing else lists minors: the CI matrix, the sweep and the symbols are all derived from it.

**Which symbols a version gets.** All symbols of versions at or below it, in plain version order:

- `BC<n>PLUS` for every major `n` from 27 to the version's major (unchanged);
- `BC<n>_<m>PLUS` for every released minor `n.m` with `m` of at least 1 that is at or below the version;
- for a version not in the list (an insider build), also its own `BC<n>_<m>PLUS` when its minor is at least 1.

Examples, with today's list:

| Version | Symbols |
|---|---|
| 27.0 | `BC27PLUS` |
| 27.5 | `BC27PLUS`, `BC27_1PLUS` … `BC27_5PLUS` |
| 29.0 | `BC27PLUS` … `BC29PLUS`, `BC27_1PLUS` … `BC27_5PLUS`, `BC28_1PLUS` … `BC28_5PLUS` |
| 29.1 | as 29.0, plus `BC29_1PLUS` |
| 29.2 (insider) | as 29.1, plus `BC29_2PLUS` |
| 30.0 (insider) | as 29.1, plus `BC30PLUS` |

Why version order across majors: 30.0 is later than 29.1, so `BC29_1PLUS` must hold on 30.0. Otherwise the common
shape "29.0 changed it, 29.1 reverted it", written `#if BC29PLUS and not BC29_1PLUS`, would select the 29.0 values on
30.0. An older major that gets a later minor (a 28.6 released after 29.0) adds `BC28_6PLUS` to the list, and from
then on every 29 and 30 version gets it too.

**Where the rule lives.**

- `al_version::corpus_symbols` takes a `BcVersion` (major and minor) and the list of released minors instead of a
  major. It stays the one runner function for the rule (ADR-015 3.9) and stays pure (ADR-006). Every caller passes
  the run's target version.
- The runner keeps no list of its own. The corpus results manifest (ADR-077 decision 4) records the list. The bump
  copies it into `oracle/oracle.toml` (`released_minors`), and `al-test-cli` passes it to `corpus_symbols`. A third-party app still gets
  only its own `preprocessorSymbols` and `--define`; AL Runner defines no symbol for it (ADR-052 3.4).
- The corpus computes the symbols in one script that both `ci.yml` (job `prepare`, both the default matrix and the
  `bc_version` override, which today loops over 27 to 29) and `nightly-windows.yml` call. Today three places type or
  loop the list.
- `tools/corpus-writer/compile-legs.sh` asks `al-test-cli` for the symbols of each leg (a `--print-symbols VERSION`
  option) instead of typing them. `tools/tests/test_compile_legs.py` follows.
- The corpus results manifest (ADR-077 decision 4) records the symbols of each version. `tools/corpus_bump.sh` refuses
  a bump when they differ from `corpus_symbols` of that version. This is the check that keeps the two copies of the
  rule (corpus CI and runner) equal.

**Gates keyed on a minor.** `Gate::since_bc()` returns a major today, and `Target::has` compares majors. It becomes a
version `(major, minor)`, compared as `(major, minor)`. A behavior that one minor introduced and a later minor
reverted is two gates, or a gate with an end version. `gate_completeness` learns `BC<n>_<m>PLUS`: every guard on a
minor symbol must be cited by a gate of that minor. Today it skips such a name without failing, because it expects
`PLUS` right after the digits. The `no_version_literals` test learns the new form too.

### 4. Version-neutral required check names in the corpus

The corpus master ruleset requires the checks "BC 29.0 / test", "BC OnPrem 29.0 / test" and the other legs by their
minor (corpus `bc-leg.yml` header). Every minor bump would then need a ruleset change by the owner at merge time.

Decision: name the CI jobs by major ("BC 29 / test", "BC OnPrem 29 / test", "BC OnPrem DotNet 29 / test",
"BC TestLibraries 29 / test", "BC TryWrites 29 / test"). The job name uses a `major` field of the matrix entry; the
minor stays in the matrix and in the job's log and artifact. The ruleset changes once. A minor bump then needs no
ruleset change.

The artifact name `junit-test-results-<bc_version>` is fixed in bc-linux and keeps the minor. `tools/corpus_bump.sh`
therefore takes each leg's minor from the run's manifest, not from a list in the script.

### 5. One pinned build per oracle leg

ADR-077 found that the corpus pins the build for 29.0 only, and 27.5 and 28.5 take the newest build of the minor.
Decision (owner, 2026-11-23): each oracle leg pins one exact build (`provenance.py`, `PINNED`, one entry per leg).
Only a bump pull request moves a pin: the minor bump pull request, or a corpus pull request that moves a leg to a
newer build of the same minor. The oracle then changes only on purpose. The weekly sweep always runs the newest
build (point 2), so it shows when a newer build differs from the pinned one.

## Runner-side consequences (named, not built here)

Each item is its own pull request. Nothing changes before the corpus switch.

1. **`tools/corpus_bump.sh`**: `legs="27.5 28.5 29.0"` (line 46), the `*_id[29.0]` lines near 408-411 and the header
   (line 8) go. The legs come from the manifest by major (point 4). `tools/corpus_bump_pr.py` `LEGS` (line 32) and
   `tools/tests/test_corpus_bump_harness.py` follow, unless ADR-077 decision 10 has deleted `--from-pr` by then.
2. **`oracle/oracle.toml`**: the 29 block's `bc_version`, `bc_build` and `ci_bc_build` move to 29.1. The bump does
   this, never a hand edit. A bump that changes a leg's minor also moves `bc_build` to a build of the new minor; the
   bump refuses when `bc_build` and the manifest name different minors, so the runner never loads 29.0 apps against a
   29.1 oracle. `oracle/29/*.xml` come from the bump unchanged (ADR-077 decision 2).
3. **Metrics re-baseline under ADR-065 and hard rule 2**: `metrics/29/not-passing*.txt` and `work.json` are
   re-recorded by the bump. A case that passed against the 29.0 oracle must not be added to a not-passing list because
   29.1 changed it. For class 1 the corpus rewrite removes the pinned value, so the runner needs no change. For classes
   2 and 3 the runner change that serves the 29.1 result lands before or with the bump. Until the runner matches every
   behavior change, the minor bump waits. There are no not-passing exceptions for a minor bump (owner, 2026-11-23;
   hard rule 2 stays whole). The sweep's insider results give the lead time to do the runner work before the
   release.
4. **`crates/al-version`**:
   - `corpus_symbols(major)` becomes `corpus_symbols(&BcVersion)`; callers in `al-test-cli` (`main.rs:1104`,
     `mnc.rs`, `windows_language.rs`, `coverage.rs`), the examples and the `al-sema` tests follow.
   - `awaited.rs` functions keyed on "29 means the CI's 29.0" are measured again on 29.1.
     `journal_temp_part_reads_after_refresh` is class 3, cause "page part refresh": it answers `Some(true)` for all of
     29 today, and 29.1 has the 27/28 shape. `bulk_commit_ends_cached_row` says 29.1 is a local container that differs
     from the CI; after the switch, the CI is 29.1.
   - `gates.rs`: gates that mean "since 29" and that 29.1 reverted get an end version (point 3). The Feature Key rows
     and `test_runner_root_scope` already tell 29.0 and 29.1 apart and are checked against the new oracle.
   - The default with no target ("the newest behavior") means the newest oracle minor, 29.1.
   - This changes `Gate::since_bc()` and `corpus_symbols`, both public functions of `al-version` used by other crates.
     This ADR is the ADR that hard rule 5 asks for.
5. **Microsoft's test recipe artifact**: ADR-052 records that Microsoft's CI run is 29.1 while the runner's artifact
   is 29.0 (458 tests missing). With the 29 leg on 29.1, the recipe runs use the 29.1 artifact, and that gap closes.
6. **Docs**: `docs/status.md` line 6 (`--all-versions` text), `oracle/README.md`, `oracle/oracle.toml` header,
   `docs/agents/bump.md` (minor bump steps), the comment in `tools/dotnet-sidecar/Dispatch.cs:140`.
7. **The local observation host** that serves probes for major 29 runs 29.1, so new probes measure the oracle minor.

The order of the switch: (1) the 63 corpus tests are fixed with guards or rewrites, as corpus pull requests;
(2) the runner changes of item 3 land; (3) the corpus switches `ci.yml` to 29.1 and renames
the required checks (point 4); (4) the runner bumps to the first green 29.1 master run. ADR-077 needs no change for
this: the first master run on a new build is a full run, so no result of 29.0 is reused for 29.1.

## Options considered

- **(a) Keep the "identical" rule.** The 29 leg stays on 29.0 while users run 29.1. The runner then matches a minor
  nobody deploys. Rejected by the owner.
- **(b) One oracle per minor** (ADR-015 Q1, the third option). Many more oracle files and not-passing lists for the
  same code. Rejected in ADR-015 and again here; the sweep gives the other minors as information.
- **(c) Minor symbols only within the version's own major** (29.1 gets `BC29_1PLUS`, 30.0 does not). Fewer symbols,
  but `#if BC29PLUS and not BC29_1PLUS` then selects 29.0 values on 30.0. Rejected.
- **(d) Other name forms.** `BC29.1PLUS` is not a valid symbol (a dot). `BC291PLUS` is ambiguous once a minor reaches
  10 and the gate scanner cannot split it. `BC29M1PLUS` works but is harder to read. Chosen: `BC29_1PLUS`.
- **(e) This decision.**

## Consequences

- The runner follows the BC version users run, one minor after Microsoft releases it.
- Every minor release costs one corpus pull request and the runner work for its class 2 and 3 differences. The weekly
  sweep's insider results let that work start before the release.
- The corpus pull request that adds a released minor to the list changes the symbol set of every later version. The
  bump check of the manifest catches a runner that has not taken the new list.
- Behavior of an older minor (for example 29.0's page part refresh) stays in the corpus as the guarded branch, cited
  by a gate, but no oracle checks it after the switch. The weekly sweep is the only check. A runner user who runs
  `--bc-version 29.0` gets that behavior from a pin that the oracle no longer checks.
- The symbol set grows by one name per released minor. 29.1 gets 14 symbols. The compile time cost is a set lookup per
  `#if`.
- `corpus_symbols` and `Gate` change their signature. Every caller changes in one pull request.
- A minor bump with an unfinished runner change blocks the switch (consequence item 3). This is the price of keeping
  hard rule 2 whole.

## Conformance

- `al-version` unit tests: `corpus_symbols` with today's list gives, for 27.0, 27.5, 29.0, 29.1, 29.2 and 30.0, the
  sets in the table of point 3; a 28.6 added to the list appears on 29.0 and 30.0.
- `gate_completeness.rs`: a fixture with `#if BC29_1PLUS` and no gate of 29.1 fails; with the gate it passes.
- `corpus_bump_combined.rs` (or a new `corpus_bump_minor.rs`, with the fake `gh`): a manifest whose symbols differ from
  `corpus_symbols` is refused; a bump whose manifest names 29.1 while `bc_build` stays 29.0 is refused; the legs are
  read from the manifest.
- `tools/tests/test_compile_legs.py`: the symbols come from `al-test-cli --print-symbols`.

## Amendment notes for other ADRs

Each line is added to the named ADR in a follow-up pull request.

- **ADR-052**: "Amended by ADR-078 (2026-11-23): the matrix examples name 29.1 for the 29 leg, and Microsoft's run
  and the recipe artifact are both 29.1 after the switch."
- **ADR-077**: "Amended by ADR-078 (2026-11-23): the corpus pins one build per oracle leg (27.5, 28.5, 29.1), not 29.0
  only, and the results manifest records the preprocessor symbols of each version and the list of released minors."
- **ADR-015**: "Amended by ADR-078 (2026-11-23): 3.9 `corpus_symbols` takes a version and adds `BC<n>_<m>PLUS`; 3.4
  gates may key on a minor; the drift check of Q1 is the corpus's weekly sweep."
