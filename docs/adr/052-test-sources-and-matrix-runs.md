# ADR-052: Test sources (`--affected-from`, `al-runner test-microsoft`) and matrix runs over BC versions and countries

- Status: Accepted (owner, 2026-10-21, with the answers recorded under the open questions)
- Date: 2026-10-21
- Related: ADR-013 (dependency apps from source, compiled-only apps refused), ADR-015 (one runtime for every BC
  major), ADR-022 (compile cache), ADR-028 (serve), ADR-043 (impact selection), ADR-046 (install code, companies),
  ADR-050 (`--affected`: own tests against the last run, a different feature)

## Decision in short

1. **Test sources.** `al-runner test APP... --affected-from SOURCE` (repeatable) adds the tests of another test
   source to the run. Only the tests of that source that can reach the user's extension are run, with and without the
   extension (ADR-043). `SOURCE` is `microsoft` (every Microsoft test app of the build, with Microsoft's recipe) or
   the path of an ISV test app (folder, `.app` with source, or `*.Source.zip`). One report, grouped by source.
2. **One-liner.** `al-runner test-microsoft [APP...]` runs Microsoft's tests with no paths to collect. Without an
   APP it runs them as Microsoft's CI does. With an APP it is `test APP --affected-from microsoft` with no own tests.
3. **Matrix runs.** `--bc-version 27.5,28.5,29.0 --country w1,us` runs every combination; `--matrix "27.5:w1,29.0:us"`
   names the cells. One child process per cell, sequential by default, `--matrix-parallel N` within a memory limit.
   The report has a table of cells and the list of tests whose outcome differs between cells. Exit code: the worst
   cell.
4. **Serve** keeps refusing `--recipe`, `--impact-of`, `--affected-from` and matrix options, with a clear message
   (owner answer 5).

The trust rule binds every part: no test is ever reported as passed unless it ran and passed in this run. A test that
was not selected, not run or not reached is counted and named as such, never as a pass.

## Context

The owner's requirements (2026-10-21), in short:

- Microsoft's tests without the download chore: one command that fetches the BC artifact, finds every Microsoft test
  app of that build, applies `--recipe microsoft` (company per codeunit, disabled tests from BCApps) and runs them.
  Default: all test apps; `--app` narrows.
- On the normal `al-runner test MYAPP...`: add the Microsoft tests that MYAPP can affect (impact selection,
  ADR-043), next to the user's own tests, in one report.
- Generalized: any app that ships tests is a test source, not only Microsoft. `--affected-from SOURCE`, repeatable.
  ISV test apps use normal test defaults; their dependencies resolve as today. A test app must carry AL source.
  Per-source settings later, if ISVs need them.
- Matrix runs: a list of BC versions times a list of countries, or explicit cells. Each cell has its own artifact and
  compile cache. Preprocessor symbols per version (the owner's first note said the runner sets them, as the corpus
  CI does; revised by owner answer 4 below). Report: a table per cell,
  the tests whose outcome differs across cells, JUnit per cell, `--output-json` with all cells, exit code of the
  worst cell. Combines with `--affected-from`.
- Ordering: this ADR first, then the one-liner, then `--affected-from`, then matrix runs.

What exists today:

- `al-runner test Tests-ERM.Source.zip` runs one Microsoft test app; the user must find and pass the archive.
  Dependencies, `System.app` and the CRONUS backup come from the artifact feed (`al-deps` `artifact`, `demo`).
- `--recipe microsoft` (`recipe.rs`, `recipe_run.rs`): one run per lane (Legacy, UnitTest; the two Contoso lanes are
  refused with a reason), disabled tests skipped, a by-lane summary, exit code over the lanes
  (`recipe_run::exit_code`).
- `--impact-of APP` (ADR-043, `impact.rs`, `impact_index.rs`): dynamic selection from a cached index, static
  selection with a warning when none is cached, `--impact all`; change classes, `--impact-json`, `--impact-accept`,
  JUnit properties, exit codes. The "without" side holds Microsoft's apps only.
- `--bc-version V` and `--country CC` choose one target per run. The compile cache, the impact index and the demo
  backup are already keyed by build and country.
- `al_version::corpus_symbols(major)` gives the cumulative `BC27PLUS` ... `BC<major>PLUS` set. Only the corpus uses it
  today; a third-party app gets only its own `preprocessorSymbols` and `--define`.
- Serve refuses `--recipe microsoft` and `--impact-of` at start (`args.rs`), because each request is one plain run.
- ADR-050 `--affected` selects the user's own tests by what changed since the last run. It is a different feature.
  `--affected-from` selects another source's tests by what the user's extension can reach.

## Decision

### 1. Test sources: `--affected-from SOURCE`

#### 1.1 Command line

```
al-runner test APP... --affected-from SOURCE [--affected-from SOURCE ...]
               [--microsoft-app NAME ...] [--impact dynamic|static|all]
               [--impact-accept FILE] [--impact-json FILE] [existing flags]
```

- `SOURCE` is one of:
  - `microsoft`: the Microsoft test apps of the run's build and country (1.3), with Microsoft's recipe (1.4).
  - A path: an ISV test app as a folder with `app.json`, a `*.Source.zip`, or a `.app` that carries its AL source.
    A folder of several test apps is one source per app inside it.
- `--microsoft-app NAME` (repeatable) narrows the `microsoft` source to the named test apps, for example
  `Tests-ERM`. The name is the app name in the artifact, matched without case. An unknown name exits 2 and lists the
  names that exist.
- `--impact`, `--impact-accept` and `--impact-json` keep their ADR-043 meaning and apply to every source.
- The APPs on the command line are split into the **extension** (apps without test codeunits) and the **own test
  apps** (apps with at least one test codeunit). An app with both counts as both; its own test codeunits are own
  tests and are not selected as source tests (as ADR-043 step 1).

#### 1.2 What a run does

1. **Own tests.** The own test apps run as a normal `al-runner test` run (`run::test`), with all the usual flags.
   `--filter`, `--codeunit`, `--test-exact`, `--exclude-test`, `--disabled-tests` and ADR-050 `--affected` apply to
   own tests only.
2. **Each source**, in the order given, runs as an ADR-043 impact run: the extension is the "with" part, the
   source's test apps are the test apps. The "without" side is the source's test apps with all their dependencies
   and without the extension. (ADR-043 says the "without" side holds Microsoft's apps only; for an ISV source it
   also holds the ISV's apps. This ADR widens that sentence; the rest of ADR-043 is unchanged.) Selection is
   dynamic when an index is cached for the source, else static with ADR-043's warning, or `--impact all`.
3. **Dependencies** of a source resolve exactly as for any APP today: `--dep`, `--package-cache`, the `.alpackages`
   folder of each APP and of the source folder, then the artifact feed.
4. **Compile.** The extension's compile layer is reused between the own run and every source (ADR-022). Every
   dependency layer is compiled completely, never demand-driven, on both sides (ADR-043 condition 1).

#### 1.3 The `microsoft` source: which test apps

- The test apps are the `Test/*.Source.zip` entries of the artifact of the run's build and country that hold at least
  one test codeunit. Apps with no test codeunit (test libraries such as `Tests-TestLibraries`, `Library Assert`,
  `Any`) are dependencies, not sources. The listing reuses `al-deps` `artifact` (it already reads the artifact's
  central directory by range and lists the `*.Source.zip` names).
- For a country, the same lookup is used that `--country` uses for Base Application today.
- Nothing is downloaded that the run does not compile: with `--microsoft-app`, only those test apps and their
  dependencies.

#### 1.4 Defaults per source

| | `microsoft` | ISV test app |
|---|---|---|
| Companies | Microsoft's recipe: lanes from BCApps, company per test codeunit (as `--recipe microsoft`) | the run's normal test data (CRONUS backup or `--no-test-data`), one company |
| Disabled tests | BCApps `DisabledTests`, skipped (ADR-043 owner decision 2) | none |
| Impact index key | ADR-043 key plus the lane | ADR-043 key (it already hashes the test apps' source) |
| Build | the run's build | the run's build |

For the `microsoft` source, the impact run runs once per lane, and the index is kept per lane (ADR-043 section 4
already names the lane in the key). Codeunits of a lane the runner cannot build yet (the Contoso lanes) are reported
as "not verified: lane not run yet", with the lane's reason.

Per-source settings (a company or a disabled-tests folder for one ISV source) are left for later (owner note).

#### 1.5 What is refused

| Case | Answer |
|---|---|
| A source `.app` without AL source, or a dependency of a source without AL source | exit 2 before the run: "SOURCE is a compiled-only app; AL Runner compiles test apps from their AL source (ADR-013). Ask the vendor for a package with source, or a `*.Source.zip`." |
| A source that depends on an app of the extension | exit 2: "SOURCE depends on APP: it is your own test app; pass it as an APP" |
| `--affected-from` with no extension app among the APPs | exit 2: "nothing to measure: --affected-from needs an app without test codeunits. To run Microsoft's tests, use al-runner test-microsoft" |
| The same source given twice, or a source that is also an APP | exit 2, naming it |
| `--microsoft-app` without `--affected-from microsoft` (in `test`) | exit 2 |
| `--recipe microsoft` together with `--affected-from` | exit 2: the recipe applies to the `microsoft` source by itself; own tests use normal defaults |
| `--no-compile-cache` | exit 2 (as `--impact-of`: both sides must compile the dependencies completely) |
| `--explain`, `--coverage*` | exit 2 with `--affected-from` (as `--impact-of`) |
| The "with" side of a source does not compile | that source ends with exit 2 and the compiler's message; the other parts still run and report |
| A kind of impact point the selection does not cover | as ADR-043: every test codeunit of that source runs, with a warning naming the point |

#### 1.6 Report

Text, one section per part, then a summary table:

```
== own tests: My App Tests 1.0.0.0 ==
  ... the normal report of al-runner test ...
== microsoft: BC 28.5.<build> w1, 34 test apps, dynamic selection ==
  lane Legacy:   ... the ADR-043 impact report (classes, extension routines that ran) ...
  lane UnitTest: ...
  not selected: 38412 tests do not reach your extension (index of <date>)
== Vendor Product Tests 5.2.0.0 (path given) ==
  no impact index: tests were selected by static reach, which misses ... (ADR-043 warning)
  ...
Sources
  part          tests run  ran your code  newly failing  fails differently  newly refused  not verified  exit
  own tests     212        -              -              -                  -              10 not run    1
  microsoft     1840       412            1              0                  3              220           1
  Vendor ...    96         40             0              0                  0              5             0
Result: FAIL (own tests: 2 failed; microsoft: 1 newly failing)
```

- A source is named by its app name and version from `app.json`, never by its path.
- Tests a source did not select are counted on the `not selected` line. They are never listed as passes.
- `--junit FILE`: one file. Own tests keep their test suites as today. Each source adds its "with" run's suites
  (only selected codeunits), with the ADR-043 properties `impact_class`, `impact_without`, `impact_accepted`, and a
  suite property `test_source` (`microsoft` or the source's app name). Not-selected tests are not in the file; each
  source's first suite carries a property `not_selected` with their count.
- `--output-json FILE`: the own tests in AL Runner v2's shape, as today, plus a new top-level `sources` array
  (additive, so v2 readers still work). Each entry: `source` (`microsoft` or the app name), `kind`, `testApps`
  (name, id, version), `mode`, `lanes` (for `microsoft`), `counts` per class, `notSelected`, `exitCode`, and per test
  the ADR-043 `--impact-json` entry.
- `--impact-json FILE`: `format: 2`, `sources: [...]`, each entry the format-1 body of ADR-043 with a `source` field.
- `--impact-accept FILE`: reads format 1 and 2. An entry with a `source` field matches only that source; one without
  matches in every source (format-1 files keep working).

#### 1.7 Exit code

- Own tests: `output::exit_code` as today (0, 1, 6, 7, 8; `--allow-not-run`).
- Each source: ADR-043's rule. 1 for an unaccepted newly failing, fails differently, newly refused or selection
  miss; 2 when the source could not be loaded or compiled; else 0. "Not verified" tests never give 7 or 8. A source
  that selects no test gives 0 (nothing of it reaches the extension), not 6.
- The run: the worst part, by one shared helper also used by matrix runs (3.6): 2 if any part is 2; else 1 if any
  part is 1; else 8; else 7; else 6 only when every part selected nothing; else 0. Own tests that selected nothing
  while a source ran give 6 only with no source tests either.

#### 1.8 Reuse

- `impact.rs` runs each source. Its "without" side generalizes from "Microsoft's apps" to "the source's
  dependencies"; its `run`, `decide`, `decide_static`, `classify` and report stay.
- `recipe_run::lane_args` gives the per-lane arguments of the `microsoft` source.
- `impact_index.rs`: `al-runner impact-index build` accepts `--from microsoft [--microsoft-app NAME ...]` so the
  index for the `microsoft` source is built with one command. ISV test apps are built as today
  (`impact-index build ISV-TEST-APP`). The no-index warning prints the exact command for each source.
- New: `crates/alrunner-cli/src/sources.rs` (the source model: parse `SOURCE`, list Microsoft test apps, split APPs
  into extension and own tests, the refusals of 1.5) and `crates/alrunner-cli/src/sources_report.rs` (the grouped
  report, JSON, JUnit and exit code). `al-deps` gains a pure function that lists the test apps of an artifact.

### 2. The one-liner: `al-runner test-microsoft`

```
al-runner test-microsoft [APP...] [--bc-version V] [--country CC] [--microsoft-app NAME ...]
                         [--impact dynamic|static|all] [existing test flags]
```

- **Name.** `test-microsoft` (owner answer 1). It sorts next to `test` in the help, and reads as what
  it does. `--app NAME` is accepted as a short form of `--microsoft-app NAME` in this command only, as the owner's
  note writes it.
- **Without an APP:** run Microsoft's test apps (1.3) with `--recipe microsoft`. This is `al-runner test
  <every test app> --recipe microsoft` without collecting the paths. `--filter`, `--codeunit` and the other selection
  flags apply to Microsoft's tests. Default BC version: the existing rule with no app (the newest published build of
  the newest verified major); default country `w1`.
- **With APPs** (extensions only): `al-runner test APP... --affected-from microsoft`. An APP with test codeunits is
  refused (exit 2: "use al-runner test APP --affected-from microsoft to also run your own tests").
- **Report and exit code:** without an APP, the recipe run's by-lane report and exit code. With APPs, the `microsoft`
  section of 1.6 and the rule of 1.7.
- **Refused:** `--recipe` (always on here), `--disabled-tests`, `--parity`, `--company-data`, `--test-data*` (the
  recipe decides them, as `--recipe microsoft` refuses them today), `--affected-from` (the source is fixed), and
  ADR-050 `--affected` (no own tests).
- **Reuse:** a thin front over `sources.rs` and `recipe_run::test`. No new run logic.
- Our own development runs keep using `al-runner test Tests-ERM.Source.zip` without the recipe, so disabled tests
  still run there (owner decision of 2026-10-17).

### 3. Matrix runs

#### 3.1 Command line

```
al-runner test APP... --bc-version 27.5,28.5,29.0 [--country w1,us,de]   # every combination
al-runner test APP... --matrix "27.5:w1,29.0:us"                         # the named cells only
              [--matrix-parallel N] [--matrix-memory GB]
```

- A list in `--bc-version` or `--country` makes a matrix. A single value in both is a normal run, as today.
- `--matrix` takes `VERSION:COUNTRY` pairs; `VERSION` alone means `w1`. It cannot be combined with a list in
  `--bc-version` or `--country` (exit 2).
- Each `VERSION` takes the forms `--bc-version` takes (major, major.minor, exact build).
- Works with `al-runner test`, `al-runner test-microsoft` and `--affected-from` (each cell runs every part).

#### 3.2 Pre-flight

Before any cell runs, every cell is resolved: its exact build, the country artifact, and the checks that a single
run makes before it starts (`app.json` `application` and `platform` floors, `--company-data package` only on w1).
Any cell that fails these checks stops the whole run with exit 2 and one line per bad cell. This costs seconds; it
avoids learning after 20 minutes that the last cell could not start. (Owner answer 3.)

#### 3.3 One cell

- Each cell is one child process of the same binary, with the command line of the cell: one `--bc-version` (the
  exact build from the pre-flight), one `--country`, and a hidden `--matrix-cell FILE` that writes the cell's result
  as JSON. The parent prints, aggregates and decides the exit code.
- Why a child process: a cell's memory is returned to the system when it ends; a panic or an out-of-memory kill stays
  in one cell; threads and caches of two cells never mix. The cell's run is exactly a normal run, so a cell result
  can be reproduced by its own command line, which the report prints.
- **Caches:** each cell uses the compile cache, impact index and demo backup of its build and country, which are
  already keyed that way. Countries download their localized apps and backup once; later runs read the cache.
- **ADR-050 stored result:** with a matrix, the key of the stored result also holds the build and country, so cells
  do not overwrite each other. `--affected-store FILE` gets the cell appended to its name (`FILE` ->
  `FILE.27.5-w1`).
- **Threads:** each cell gets `threads / N` worker threads (at least 1) when `N` cells run at once.

#### 3.4 Preprocessor symbols per cell

- AL Runner defines no preprocessor symbols of its own, in a matrix or anywhere else. Symbol names are the app
  author's choice; the runner never invents names such as `BC<n>PLUS` (owner, 2026-10-21).
- Every cell compiles with the app's own `preprocessorSymbols` from `app.json` and `--define`, exactly as a single run
  does today. So a cell gives the same compile as a single run of the same version and country.
- Microsoft's apps and other dependencies keep their own symbols (hard rule 6; the compile cache key already has the
  symbols per layer).
- **Optional later step (step 6 of the build order, sized apart):** per-cell symbols from configuration. A matrix
  file (`--matrix-file FILE`) lists the cells and, for each version or cell, the symbols to add, for example
  `{"cells": [{"bcVersion": "27.5", "country": "w1", "define": ["MYAPP_BC27"]}, ...]}`. The file only adds the
  names the author writes. The report header prints each cell's symbols. Built only if users ask for it.

#### 3.5 Parallel cells and memory

- Default: one cell at a time.
- `--matrix-parallel N`: at most N cells at once. A new cell starts only while the free memory is above the cell's
  expected peak plus a reserve of 2 GB. The expected peak is the peak of the same cell in the last matrix run (kept
  in the cache next to the compile cache), else 3 GB for a run without a source and 10 GB for a cell with the
  `microsoft` source (the measured full-run peak in ADR-043). `--matrix-memory GB` caps the total for all running
  cells.
- A waiting cell waits; it is never killed to make room. A cell that the system kills ends with exit 2 and the
  reason; the other cells finish.
- Free memory is read in `alrunner-cli` (ADR-006: no OS access in core crates).

#### 3.6 Report

Text: one cell's normal report per cell, in order, under a header (`== cell 28.5.<build> us ==`) when `--verbose`;
by default only each cell's failures and `Result:` line. Then:

```
Matrix
  cell              build             pass   flagged  fail  not run  exit
  27.5 w1           27.5.<build>      1180   211      3     40       1
  28.5 w1           28.5.<build>      1183   209      0     40       7
  29.0 us           29.0.<build>      1175   220      2     46       1
Differs between cells (5 tests)
  codeunit  test                         27.5 w1  28.5 w1  29.0 us
  50110     PostWithDiscount             FAIL     PASS     PASS
  50112     ExportVatReport              PASS     PASS     FAIL
  ...
  12 more tests run in some cells and are not run in others (--verbose lists them)
Result: FAIL (worst cell 27.5 w1: exit 1)
```

- A test "differs" when its outcome (pass, fail) differs between cells where it ran, or when it failed in every cell
  with different error texts. Pass versus not run is counted on its own line and listed with `--verbose`: it is a
  gap of the runtime, not a difference of BC.
- A test that exists in some cells only (an object under `#if`, or a test app of another version) shows `absent`.
- A flagged pass shows `PASS*`. A flagged pass and a clean pass are the same outcome for "differs".
- With `--affected-from`, the matrix table has one row per cell and part, and "differs" compares the change class
  of each source test across cells.
- `--junit FILE`: one file per cell, the cell inserted before the extension (`results.xml` -> `results.27.5-w1.xml`).
- `--output-json FILE`: `{"format": 1, "cells": [{"bcVersion", "build", "country", "symbols", "commandLine",
  "exitCode", "peakMemoryMb", "run": <the cell's normal --output-json document>}], "differs": [{"codeunit", "test",
  "outcomes": {"<cell>": "pass|fail|notRun|absent"}, "messages": {...}}], "exitCode"}`. A matrix document is a new
  shape; a single run's document is unchanged.
- **Exit code:** the worst cell, by the shared helper of 1.7.

#### 3.7 What is refused

- `--matrix` with a list in `--bc-version` or `--country`; an empty or repeated cell; `--matrix-parallel 0`.
- `--explain` with a matrix (explain one cell's test with that cell's command line, which the report prints).
- `--test-data PATH.bak` with more than one build or country: one backup cannot fit two cells.
- Any cell that fails pre-flight (3.2).

#### 3.8 Reuse

- The cell runs `run::test`, `recipe_run::test` or the sources run unchanged.
- The parent reads the cells' JSON documents. The "differs" list compares `(codeunit, test)` ids from those
  documents, the same id `al-testrun` `CaseId` uses.
- `al_version::choose_target` resolves each cell's build.
- New: `crates/alrunner-cli/src/matrix.rs` (cells, pre-flight, child processes, scheduling, report).

### 4. Serve

Today serve refuses `--recipe microsoft` and `--impact-of` at start, because each `runTests` request is one plain
run on one warm workspace.

**Decision (owner answer 5): keep refusing**, and add `--affected-from`, `--microsoft-app`, `--matrix`, lists in `--bc-version`
and `--country`, and `--matrix-parallel` to the refused set with the same message. Reasons:

- Serve holds one warm workspace for one target. A matrix needs several targets and several processes; it would make
  serve a scheduler.
- An impact run over Microsoft's tests takes minutes and up to 10 GB. That is not an editor loop, and a warm serve
  process gains little: the dependency layers are already cached on disk.
- No tool sends these fields today (ALchemist, LethAL).

A later ADR can add a `runTests` field `affectedFrom` for ISV sources when a tool asks for it. Nothing in this design
blocks that: the source model in `sources.rs` takes its input from arguments, not from the command line.

## Test plan (red, then green)

No test downloads from Microsoft. The tests use the fake artifact feed of `al-deps` (`Cdn` trait) with small
corpus-style fixture apps: a fake "Base Application", fake Microsoft test apps, an ISV app with its test app, and a
user extension. Nothing of Microsoft's is committed (hard rule 7).

### One-liner

1. `al-runner test-microsoft` exits 2 today (unknown command): red.
2. With the fake artifact: every test app with test codeunits is found; test libraries are loaded as dependencies,
   not run as sources.
3. `--microsoft-app Tests-Fake-A` runs only that app; an unknown name exits 2 and lists the names.
4. The recipe applies: a disabled test is skipped; a `TestType = UnitTest` codeunit runs on the empty company.
5. With an extension APP: the report equals `al-runner test APP --affected-from microsoft`.
6. An APP with test codeunits, `--recipe`, `--disabled-tests` and `--affected-from` are refused.

### Test sources

7. `--affected-from PATH` with an ISV test app whose test reaches the extension's subscriber: the test is selected,
   newly failing, and the run exits 1. A test of the same source that does not reach it is counted under
   `not selected`, never listed as passed.
8. Own tests and two sources in one run: three sections, the summary table, exit code by the shared helper.
9. The "without" side of an ISV source holds the ISV app and not the extension.
10. A compiled-only ISV `.app` and a compiled-only dependency of a source: exit 2 with the ADR-013 message.
11. A source that depends on the extension: exit 2.
12. No index cached for a source: static selection with the warning and the exact `impact-index build` command for
    that source; with `impact-index build --from microsoft` first, dynamic selection.
13. `microsoft` source per lane: the index key holds the lane; a Contoso-lane codeunit is "not verified" with the
    lane's reason.
14. `--impact-json` format 2 and `--impact-accept` with and without `source`; a format-1 file still matches.
15. `--output-json`: the v2 shape is unchanged for own tests (existing tests stay green) and `sources` is present.
16. `--junit`: source suites carry `test_source`, `impact_class`, and `not_selected` counts; no unselected test.
17. `--affected` (ADR-050) narrows own tests only; source selection is unchanged.

### Matrix

18. `--bc-version 27.5,28.5` exits 2 today (one version only): red.
19. Two versions times two countries make four cells; `--matrix "27.5:w1,29.0:us"` makes two; the mixed form exits 2.
20. Pre-flight: a cell below the app's `application` floor stops the run before any cell starts.
21. A fixture app with `#if` on a symbol it does not define: every cell compiles it the same way as a single run
    (no symbol is added by the runner). A fixture whose test outcome differs between two cells by version-specific
    behavior is listed under "differs".
22. A test present only in one cell shows `absent`; pass versus not run is counted, not listed.
23. Exit code is the worst cell; a cell killed (simulated by a child that exits by signal) is exit 2 and the others
    finish.
24. `--matrix-parallel 2` runs two cells at once (timestamps in the cell documents overlap); with a memory cap below
    one cell's expected peak plus reserve, cells run one by one.
25. JUnit file per cell with the cell in the name; `--output-json` holds every cell and `differs`.
26. `--affected-from microsoft` in a matrix: each cell runs its own index; "differs" compares change classes.
27. `--affected` in a matrix: each cell has its own stored result.

### Serve

28. `serve --affected-from microsoft` and `serve --bc-version 27.5,28.5` exit 2 with the refusal message.

## Size

Derived from the pieces above, at the pace of ADR-043's steps and ADR-050's estimate:

| Part | Pieces | Agent-days |
|---|---|---|
| One-liner | artifact test-app listing in `al-deps` (0.5), `sources.rs` for the `microsoft` source without an extension (0.5), `test-microsoft` command, help, refusals (0.5), fixture artifact and tests 1-6 (0.5) | 2 |
| Test sources | ISV source loading and refusals (0.75), "without" side generalized in `impact.rs` (1), per-lane impact runs and index key (1), grouped report, JSON, JUnit, accept file format 2 (1.5), exit helper (0.25), `impact-index build --from microsoft` (0.25), tests 7-17 (1.25) | 6 |
| Matrix | parsing and pre-flight (0.5), child process cells and cell JSON (1), "differs" and the matrix report in text, JSON, JUnit (1), parallel cells with the memory rule (0.75), per-cell stored result (0.15), tests 18-27 (0.75) | 4.15 |
| Per-cell symbols from a matrix file (optional, later) | file format and parsing (0.5), cells and symbols from the file, report header (0.25), tests (0.25) | 1 |
| Serve | refusal list and test 28 | 0.1 |

About 12 agent-days for the first four parts, plus 1 day for the optional matrix file. The matrix part is above the backlog's first guess of 2 to 3 days. The pre-flight, the
memory rule for parallel cells and the "differs" report are what add to it; a matrix without them is about 2.5 days
but either fails late or risks an out-of-memory kill.

## Build order

1. One-liner without an APP (Microsoft's tests by recipe, no paths). It builds the source model and the artifact
   listing that the next steps use.
2. `--affected-from microsoft`, and `test-microsoft APP` on top of it: per-lane impact runs.
3. ISV sources, own tests next to sources, the grouped report, format-2 JSON.
4. Matrix runs.
5. Serve refusals: with step 2 (one line each).
6. Optional, only if users ask: per-cell symbols from a matrix file (3.4).

This is the owner's order; step 1 and 2 split the one-liner so its first half ships before impact work.

## Alternatives considered

- **A flag `--with-microsoft-affected`** (the owner's first wording). Covers Microsoft only; the generalized
  `--affected-from SOURCE` replaces it (owner, same note).
- **Matrix cells in one process.** Saves process start (milliseconds) and lets cells share loaded dependency layers.
  But memory is not returned between cells, a panic or out-of-memory kill takes every cell, and the cell result is not
  reproducible by one command line. Rejected.
- **Runner-defined version symbols (`BC<n>PLUS` per cell).** The first draft. Rejected by the owner: symbol names are
  free for the app author, so the runner must not invent them. Per-cell symbols come from configuration (3.4).
- **Unselected source tests as JUnit `<skipped>`.** Honest, but about 40000 entries per run for the `microsoft`
  source. A count property says the same.

## Consequences

- A partner runs Microsoft's and an ISV's tests against an extension with one command and no paths.
- `impact.rs` loses its assumption that the "without" side is Microsoft's apps only.
- A matrix run costs the sum of its cells. Nothing is cut to save time; the pre-flight only refuses early.
- `--output-json` gains an additive `sources` array. A matrix document is a new shape, documented in the help and the
  migration guide.
- The recipe's lane exit code and the new shared helper differ only when one lane selects nothing and another passes
  (the recipe gives 6, the helper 0). Step 1 moves the recipe to the helper.

## Owner answers (2026-10-21)

1. Command name: `test-microsoft`.
2. Source tests that were not selected: one count property in JUnit (`not_selected`), no `<skipped>` entry each.
3. A matrix cell that cannot start: a pre-flight check of all cells; any bad cell stops the whole run before tests
   start.
4. Preprocessor symbols: AL Runner sets none of its own and never invents names. Without configuration, `app.json`
   `preprocessorSymbols` apply in every cell as today. Per-cell symbols from a configuration file are an optional
   later step with its own size (3.4).
5. Serve: keep refusing sources, recipe, impact and matrix options, with a clear message.
6. Pass in one cell and not run in another: counted; listed with `--verbose`.

## Conformance

Planned: `crates/alrunner-cli/tests/test_microsoft.rs`, `tests/test_sources.rs` and `tests/matrix.rs` (the test plan
above, on the fake artifact feed and fixture apps under `tests/fixtures/sources`), unit tests in `sources.rs` (source
parsing, the extension and own-test split, refusals) and `matrix.rs` (cell lists, "differs", the exit helper), and
the serve refusal test in `args.rs`.

## Decided without owner review (2026-10-22, build step 1)

- **One run per test app.** `test-microsoft` runs each Microsoft test app as its own recipe run, not all of them in
  one run. A test app that cannot be fetched, loaded or compiled is then one row with exit 2 while the others still
  run, and each run's memory is returned before the next. The compile cache keeps the shared dependency layers, so
  later apps load them instead of compiling them.
- **Which entries are candidates.** A `*.Source.zip` under `Applications/` or `Applications.<COUNTRY>/` whose path
  contains `test` (any case): the platform artifact keeps test apps in `Test`, `Tests` and `Test Library` folders, a
  country artifact keeps all apps in one folder where only the file name says it. A candidate is a test app when its
  source holds a `Subtype = Test` codeunit; the others (test libraries, the test framework) are left out with a line.
- **`_Exclude_` apps.** Test apps whose artifact name starts with `_Exclude_` (`_Exclude_APIV2_ Tests`) are left out
  by default, with a line; `--app` runs them. The prefix marks apps Microsoft keeps apart; whether its own CI runs
  them was not checked.
- **The Legacy lane's package.** The run fetches the platform artifact's RapidStart package
  (`ConfigurationPackages/*ENU.EXTENDED*.rapidstart`) into `<cache>/artifacts/<build>/packages/` once and gives it to
  every app's Legacy lane. Before this, `--recipe microsoft` found the package only in an extracted artifact on disk,
  so a run from the artifact feed could not build the Legacy company.
- **Output files.** `--junit FILE` and `--output-json FILE` get the app's name before the extension, then the lane
  (`FILE.Tests-ERM.legacy.xml`). `--output-json` without a FILE is refused: several documents cannot share stdout.


## Decided without owner review (2026-10-22, build step 2)

Step 2 as built: `al-runner test APP... --affected-from microsoft [--microsoft-app NAME ...]`,
`al-runner test-microsoft APP...`, `al-runner impact-index build --from microsoft [--microsoft-app NAME ...]`, and the
serve refusals of `--affected-from` and `--microsoft-app` (section 4). Code: `sources.rs` (`affected`,
`microsoft_source`, `build_index`, `split`), `sources_report.rs` (`PartRow`, `table_lines`), `impact::run_part`,
`recipe_run::plan_lanes`, `output::junit_parts`. Tests: `crates/alrunner-cli/tests/test_microsoft.rs` (plan items 5,
6, 12, 13, 16 for the `microsoft` source).

- **The lane in the index key.** Each lane's run differs in its company options (Legacy: the package company;
  UnitTest: an empty company without test data), and the ADR-043 key already hashes them, so the key of each lane is
  its own without a new field.
- **One impact run per test app and lane.** As step 1's one run per test app: a run that does not load or compile
  gives the source exit 2 and a line, the other runs go on.
- **A lane the runner cannot build** (the Contoso lanes) and codeunits no lane runs are listed in codeunits
  (`Tests-X lane IntegrationTest: 4 codeunit(s) not verified: lane not run yet: <reason>`), not per test: their tests
  are only known after a compile, which the run does not do for them.
- **An APP with both** test codeunits and other objects is both an own test app and part of the extension (as 1.1).
  Objects are counted by parsing the app's source.
- **Not selected** counts every test of a test codeunit the selection left out, the tests Microsoft disables among
  them (they run on neither side).
- **JUnit.** A source that selected no test still writes one empty suite named `microsoft` with `test_source` and
  `not_selected`, so the count is in the file.
- **Not built in step 2, refused with a message:** `--output-json` (the `sources` array), `--impact-json` (format
  2) and `--impact-accept` with a source. A format-1 accept file given to every run would list its entries as "no
  longer matches" in every run of another test app; step 3 builds the accept file per source.
- **The no-index warning** of a source run prints `al-runner impact-index build --from microsoft --microsoft-app
  <app> --bc-version <build>` plus the options that are part of the key (`--country`, `--bcapps`, `--cache-dir`,
  `--dep`, `--package-cache`, `--define`), not the run's own command line.

## Decided without owner review (2026-10-22, build step 3)

Step 3 as built: `al-runner test APP... --affected-from PATH` (ISV test apps), own tests next to any sources, the
grouped report, `--output-json` with `sources`, `--impact-json` format 2, `--impact-accept` format 1 and 2. Code:
`sources.rs` (`resolve`, `isv_source`, `SourceOut`), `impact::Part::{json, stale}`, `impact::Accepts::for_source`,
`TestArgs::accept_source`. Tests: `crates/alrunner-cli/tests/test_sources.rs` (plan items 7-11, 14, 15) and the
format-2 test of the `microsoft` source in `tests/test_microsoft.rs`.

- **Section header.** `== <name> <version> (<path>) ==`: the "(path given)" of 1.6 is read as the path the user gave
  (for a folder of apps, the app's path inside it). The row, `test_source` and `source` use the app.json name only.
- **Checks before the run.** Every source is read before the own tests start. A compiled-only source (`.app` without
  AL source), a non-Microsoft dependency found only without AL source in `--dep`, `--package-cache` or an
  `.alpackages` folder, a source that depends on an extension app, the same app id twice, and a source that is also an
  APP exit 2 with no test run. Only direct dependencies are checked here; a deeper compiled-only dependency fails the
  source's run with the dependency loader's message (exit 2 for that source, the other parts still run).
- **The "without" side** of an ISV source also searches the `.alpackages` folder of each folder APP, so both sides
  resolve the source's dependencies in the same places (ADR-052 1.2 item 3). Those folders are then part of the index
  key and of the printed `impact-index build` command.
- **`--impact-json` format 2** has one entry per impact run, not one per source: the `microsoft` source has one run
  per test app and lane, and each entry adds `testApp` and `lane`. The top level also has the run's `exitCode`.
- **Stale accept entries** are listed once per source: an entry is stale when no run of that source matched it. An
  entry whose `source` names a source the run does not have is listed on one line per source name. Neither changes
  the exit code (as ADR-043).
- **`--output-json`.** The top-level `exitCode` is the whole run's (the worst part), not the own tests' alone, so a v2
  reader sees the run's result. Without own tests the v2 part is empty. `--output-json` without a FILE is refused
  with sources (the text report is on stdout). Each `sources` entry has `mode` (`mixed` when the runs of a source
  differ), `path` for a source given as a path, `lanes` for `microsoft`, `error` when the source could not run.

## Decided without owner review (2026-10-22, build step 4)

Step 4 as built: `--bc-version A,B`, `--country X,Y`, `--matrix "V:CC,..."`, `--matrix-parallel N`,
`--matrix-memory GB` on `al-runner test` (with or without sources) and `al-runner test-microsoft`. Code: `matrix.rs`,
`al_deps::test_apps::choose` (the build choice of the artifact listing, now shared). Tests: `tests/matrix.rs` (plan
items 18-26 except 21's version-specific runtime behavior, see below), unit tests in `matrix.rs` and `args.rs`
(serve refusal, item 28).

- **The floor in the pre-flight.** A single run below the app's floor warns and runs (`BuildQuery::floor_warning`).
  A matrix cell below the floor is a bad cell and stops the run (test plan item 20): a cell the author did not mean
  to support fails late otherwise.
- **`--matrix` with `--bc-version` or `--country`** is refused even when they hold one value: `--matrix` names every
  cell's version and country.
- **What a cell runs.** The run's own command line without `--bc-version`, `--country`, the matrix options,
  `--junit` and `--output-json`, plus the cell's exact build, its country, its JUnit file and the hidden
  `--matrix-cell FILE` (the cell's `--output-json`). The parent reads every JSON document in the cell's folder, so a
  `test-microsoft` cell's documents per test app and lane are all read.
- **A lone cell always starts**, whatever the free memory: waiting for memory with nothing running would never end,
  and one cell at a time is the run without `--matrix-parallel`. The peak of a cell is the child's `VmHWM` (Linux
  only; the .NET helper is not counted); `ALRUNNER_MATRIX_FREE_MB` replaces the free memory for the tests.
- **Flagged passes in the JSON.** The v2 document's test rows get `flagged: true` on a pass that ran through an
  unprobed assumption (additive), so the matrix report can show `PASS*`.
- **"Differs" keys.** A test of the run's own is (codeunit, test); a source test also has the source and, for
  `microsoft`, the test app. Pass versus not run is counted on its own line; failing everywhere with different texts
  differs.
- **Not built:** the ADR-050 stored result per cell (3.3), because `--affected` is still deferred and refused; the
  per-cell symbols file (3.4, optional step 6). Test plan item 21's second half (an outcome that differs by
  version-specific runtime behavior) is covered by two builds whose test apps differ (`Tests-Fake-B` fails in one,
  `Tests-Fake-C` exists in one only); the fixture apps have no behavior that differs by BC version in the runtime.
- `ALRUNNER_MATRIX_TEST_ABORT=<build>` makes a cell of that build abort (tests of a cell the system kills, item 23).
