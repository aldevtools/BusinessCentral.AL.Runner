# ADR-063: The passes of a corpus version share one compile

- Status: Proposed
- Date: 2026-10-31

## Context

Since the testlibraries suite (main merge 40ed93f0c, commit 7a7d7b266), `al-test-cli` runs each BC version in
passes: the main pass (main corpus app, without Tests-TestLibraries and Data Archive), then one pass per set of
`extra_apps` in `oracle/oracle.toml`. Each pass is a compile from scratch. The testlibraries app depends on the main
corpus app, so the extra pass parses, checks and lowers the main corpus app again, and the demand-driven compile of
the Microsoft apps (ADR-013) reaches about 100 000 dependency routines again: its activation roots (page triggers,
event subscribers, table triggers of every dependency app) are the same in both passes.

Measured on 2026-10-31 (BC 29, instructions of the whole process, `perf stat -e instructions:u`):

| run | instructions |
|---|---|
| full BC 29 run before the suite (bf1c11884, older corpus pin) | 205.0 G |
| full BC 29 run on main 7a8575d79 | 347.5 G |
| the testlibraries pass alone (`--area` on its 5 folders), no compile cache | 122.0 G |
| the testlibraries pass alone, with the compile cache | 53.6 G |
| `perf_budget` limit of the full run at this corpus size | 215.0 G |

The pass runs 39 to 41 tests (BC 27/28/29) and spends 0.05 s in them. Nearly all of its cost is the second compile:
about 80 G for the Microsoft apps (the fixed part of `perf_budget`, an empty corpus with the Microsoft apps, is
79 G) and about 40 G for the main corpus app. No change inside one pass can remove that; only reusing the main
pass's compile can. Reusing it changes how a pass gets its program, so it needs an ADR (hard rule 5).

What must stay as it is: each pass runs in its own environment. Tests-TestLibraries and Data Archive are observable
(Backup Management sets the database trigger flags, codeunits 130301 and 130302 change permissions, one enum
extension, a permission set extension, 178 event subscribers), so the main pass must not see them.

## Decision

Proposed: the extra pass of a version extends the compiled program of the main pass. It does not compile again
from source.

1. The main pass compiles as today. Its checked and lowered program, the demand state (what was reached) and the
   parsed sources are kept until the version's last pass ends. No state is kept across runs (ADR-016 and ADR-022 are
   unchanged: this is not a cache, the program is used by the next pass of the same process and then dropped).
2. The extra pass adds its apps (the `extra_apps` and the suite apps) to that program:
   - new apps are parsed, checked and lowered;
   - objects of the main program that a new app extends or implements (table, page, enum and report extensions,
     interface implementations) are lowered again, with the same rule the compile cache uses for a workspace that
     extends a cached object (`al_sema::provided::relower_set`);
   - the demand continues from the new roots (the new apps' tests, install code, activations and what they reach)
     on top of the main pass's reach;
   - every other object keeps its lowered form. This is sound for the same reason the compile cache layers are: an
     app sees only what it declares (`al_sema::workspace`), so the lowered form of an object depends only on the
     apps below it and on the extensions and implementers it embeds (`DepFacts::embeds`).
3. The extra pass gets a fresh engine, store, install baseline and sessions built from the extended program. No
   runtime state of the main pass is reused. The runtime seam does not change.
4. The extended program must equal a compile from scratch of the same apps. A differential test enforces it (see
   Conformance). Where they can differ (test codeunits of the main app are lowered in the main pass and are stubs in
   a from-scratch extra pass), the extra pass refuses to run them; it never runs them.
5. `AL_FRESH_PASS_COMPILE=1` turns the reuse off (each pass compiles from scratch, as today), for the differential
   test and for diagnosis.

Expected effect: the extra pass costs the lowering of Tests-TestLibraries, Data Archive and the suite app, the
objects they extend, the added demand and its own install baseline: an estimated 5 to 15 G instead of 122 G, which
brings the full run back to the level before the suite (about 225 G at today's corpus size, against 347.5 G).

### Options not chosen

- **One program for all apps, with the runtime hiding the extra apps in the main pass** (subscribers not bound,
  AllObj and installed-app rows, enum extension values, permission set extensions, install code, table extension
  fields). Saves the most, but every hiding rule is a runtime seam and a missed rule leaks into the main pass's
  results, where no differential test can see it.
- **Put the extra apps into the compile cache layers and require the cache for the extra pass.** The cache already
  shares the Microsoft layer between passes; the cached extra pass still costs 53.6 G (the main corpus app is
  compiled again), and the uncached normal run in CI would no longer measure a compile from source.
- **Leave out the main corpus app's compile-error check in the extra pass** (the check of what a selection leaves
  out, al-runner issue 5602). Measured: 122.0 G to 109.8 G uncached, 53.6 G to 43.9 G cached. Small, and it drops
  a check; with this ADR the main corpus app is checked once and the question goes away.

## Consequences

- The full corpus run is back within its `perf_budget` instruction limit, and the extra pass of each version takes
  about the time of its own apps.
- Memory: the main pass's program is kept while the extra pass runs (about the size of one pass's compile; the
  corpus run stays under the 3 GB cap, to be measured).
- `al_sema` gets an API to extend a compiled program with apps. The compile cache's relowering rule is shared by both
  users.
- A future pass with other `extra_apps` uses the same mechanism.

## Conformance

- A differential test in CI: for BC 27, 28 and 29, the testlibraries suite run with the reuse and with
  `AL_FRESH_PASS_COMPILE=1` gives the same JUnit outcomes, refusal reasons and install baseline, and
  `--example compile_dump` of the two programs is equal except for the main app's test codeunits.
- `tools/demand_vs_eager.sh run` and `tools/baseline_rule_check.sh run` stay at 0 differences.
- `perf_budget` (`a_full_corpus_run_with_the_microsoft_apps_stays_within_its_budget`) passes with the instruction
  counter, with no change to its limit.
