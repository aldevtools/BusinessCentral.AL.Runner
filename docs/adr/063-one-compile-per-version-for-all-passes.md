# ADR-063: The passes of a corpus version share one compile

- Status: Accepted (owner, 2026-10-31)
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

Accepted by the owner on 2026-10-31 (this option, not the options below). The extra pass of a version extends the
compiled program of the main pass. It does not compile again
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

## As built (2026-10-31)

Built as decided, with these points the proposal did not foresee. None changes a seam the decision did not name.

1. **The main pass's own suite apps leave the kept compile.** The main pass also compiles the OnPrem suites, which the
   testlibraries pass does not have. The main pass puts the apps a later pass also has first in its program and its own
   suite apps after them (the program order is not observable; the compile cache reorders the same way). After the main
   pass ran, the kept compile is cut back to the shared apps (`Workspace::truncate_apps`, `lower::Growable::truncate`).
   An object whose lowered form embeds what a cut app added (an extension, an implementer: the `relower_set` rule) is
   lowered again when the compile grows.
2. **The check continues the kept one** (`check::check_grown`): the kept objects are not checked again; their types,
   resolutions, findings and the demand state are kept, the new apps' objects and roots are added, and every demand rule
   that reads the lists of the workspace (extensions, subscribers, implementers, enum values, platform event
   subscribers, the objects a catalog table lists) runs again (`Checker::refire`), so what the new apps add to those
   lists is reached. The member table of the kept objects is kept, except for objects a new or cut app extends.
3. **Unchanged objects come from the kept program** the way a compile cache supplies them (ADR-022, ADR-054): the main
   pass exports the facts of every object (`lower::Export::Facts`, the program is the one a plain compile lowers), and
   an object whose demand level, reached routines and selection state are the same and that no new or cut app changes
   is supplied from the kept program and decoded on first touch. The rest is lowered from source.
4. **The grown compile may hold more than a fresh one** (decision 4 broadened): what the main pass's roots reached
   stays reached, so routines only the main app's tests reach are compiled in the grown pass and stubs in a fresh one.
   No test of the extra pass reaches them; a compile of more than the demand needs runs the same, as the eager compile
   does (ADR-013 decision 6, which CI checks). The main app's test codeunits are left out of the extra pass as before.
5. **Compile errors of the kept apps are kept as their own compile found them.** A fresh compile of all apps can lose
   one: AL0132 does not judge a table that any extension extends, also one of an app the code does not see. BC compiles
   the main app without the extra apps and reports it, as the grown compile does. Diagnostics are printed and never
   acted on in the corpus run.
6. **When a pass compiles from source anyway:** `AL_FRESH_PASS_COMPILE=1`; a main pass with a selection, the compile
   cache, `AL_EAGER_DEPENDENCIES`, `--tdd` or `--affected` (nothing is kept); a kept app that differs in the later pass
   (sources, kind, dependencies, symbols, target) or that would see a new app (a note on stderr says why); a third pass
   of a version (only the first pass after the main one grows the kept compile).
7. **Conformance as built:** `tools/fresh_vs_grown.sh` (CI, after the eager runs): for BC 27, 28 and 29, every suite's
   JUnit outcomes and reasons and every pass's install baseline equal those of runs with `AL_FRESH_PASS_COMPILE=1`. A
   `compile_dump` comparison is not exact because of point 4; the program equality is checked by the al-sema tests
   `lower::extend` (a grown compile equals a fresh compile of all its apps when the roots are the same, also after a cut).

Measured on 2026-10-31 (BC 29, `perf stat -e instructions:u` of the whole process; the split by pass from `perf record`
samples):

| run | instructions |
|---|---|
| full BC 29 run before (origin/main b56edae5b) | 347.8 G |
| full BC 29 run after | 244.0 G (`perf_budget`, best of 3: 246.5 G; main 348.1 G) |
| main pass, every pass from source / grown | 219.7 G / 222.3 G (keeping costs about 2.6 G) |
| testlibraries pass, from source / grown | 120.7 G / 20.1 G (of which its own install baseline 5.1 G) |

The full run stays above the `perf_budget` limit (215 G at this corpus size): the main pass alone is about 220 G.
