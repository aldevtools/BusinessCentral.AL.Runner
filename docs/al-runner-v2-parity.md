# AL Runner v2 parity inventory

Repo: al-runtime-rs. Task: ADR-016, "Release: AL Runner v3". State: research,
written 2026-10-13. No code changed.

The project owner's direction: al-runtime-rs becomes **AL Runner version 3**.
It must support everything AL Runner v2 offers today. Commands, flags and calls
may differ. A migration step and a migration guide are acceptable.

This page lists every user-facing v2 feature that could be found. Each row says
where v2 implements it, whether v3 has it today, what blocks it, and the
proposed v3 form.

## How this was made

- Source: the v2 checkout at
  `/home/stefan/Documents/Repos/community/BusinessCentral.AL.Runner`, commit
  `351c2813d` (2026-10-06). Paths below are relative to it.
- Output of the installed v2.12.0 tool: `al-runner --help`, `--guide` and
  `provision --help`. The help text is generated from the code, so it is the
  authority for flags.
- Docs read: `README.md`, `docs/manual/*`, `docs/server-mode.md`,
  `docs/dap-mode.md`, `docs/watch-affected.md`, `docs/run-seed.md`,
  `docs/cli-output-paths.md`, `docs/provisioning.md`, `docs/limitations.md`,
  `docs/scope.md`, `docs/expectations.md`, `docs/jobs-unit-claiming.md`,
  `docs/startup-cost.md`, `docs/v1-to-v2-migration.md`, the workflows in
  `.github/workflows/`.
- I ran the v2 tool once on a two-test project (not committed). Warm run: 14.1 s
  wall clock, 1.2 s of that in tests. Cold dependency caches: 54 to 77 s. v3
  runs the whole corpus (4836 tests) in about 1 s. These are single runs on one
  machine. They show the order of magnitude only.
- Not found in v2: a config file (the `al-runner.json` in the v2 root is a v1
  leftover and no code reads it), an AL-Go integration, an Azure DevOps task,
  telemetry. v2 sends nothing anywhere (`--guide`, "REPORTING A RUNNER GAP").

## Columns

- **v3 today**: `Done`, `Partial`, `No`, or `N/A` (the reason for the feature is
  gone in v3, or v3 replaces it by design).
- **Blocker**: the first thing that stops it. One value per row:
  - `CLI`: nothing but the missing user-facing command line (ADR-016).
    `al-test-cli` is the corpus harness and stays a dev tool.
  - `ADR-013`: dependencies, artifacts, install baseline, platform tables.
  - `ADR-014`: the .NET helper.
  - `ADR-015`: many BC versions in one binary (Proposed).
  - `LANG`: language or runtime surface, tracked by the corpus pass rate.
  - `IR-POS`: needs source positions in the IR. The IR carries them now
    (`docs/architecture.md`, "Source positions"); the row stays until the feature
    that reads them exists.
  - `HOST`: needs an `al-host` capability (clock, randomness, identity).
  - `NEW-ADR`: needs a new ADR before code (a seam or a visible design choice).
  - `PROBE`: behaviour must first be pinned by a corpus test or probe.
  - `CANNOT`: cannot be matched with the v3 design.
  - `-`: no blocker (`Done` or `N/A`).
- **Hard**: marked when matching is hard, with the reason.

Terms: a **bundle** is a folder with an `app.json` (v2 name; v3 says "project
folder"). **Corpus** is the AL language test corpus used as the oracle (ADR-007).

## 1. Inputs

| ID | Feature | v2 reference | v3 today | Blocker | Proposed v3 | Hard |
|---|---|---|---|---|---|---|
| IN-1 | One or more bundle folders as arguments; run in order; one combined summary | `Program.cs` arg loop (lines 480-660); `ProgramSupport/CliText.cs` | Partial | CLI | `al-runner test PATH...` Built (2026-10-14): `al-runner test APP...`, APP a folder, `*.Source.zip` or `.app`. | - |
| IN-2 | Bundle root found by climbing from any folder below an `app.json` | `Infrastructure/BundleRootValidation.cs` | No | CLI | Same rule | - |
| IN-3 | A folder holding several apps is run as one bundle per app; loose `.al` files beside them are ignored | `ProgramSupport/Suites.cs`, `docs/server-mode.md` (#5107) | No | CLI | Same rule | - |
| IN-4 | The same folder named twice runs once (real path, symlinks) | `Infrastructure/BundleRootDeduplication.cs` | No | CLI | Same rule, print which argument was dropped | - |
| IN-5 | `app.json` read for identity, dependencies, `platform`/`application` floor, `propagateDependencies`, `internalsVisibleTo`, `preprocessorSymbols` | `ProgramSupport/Dependencies.cs:261`, `Infrastructure/InProcessAppPackager.cs:178` | Partial | ADR-013 | `al-test-cli` `load_app` reads identity and dependencies. The floor and the rest arrive with `al-deps` | - |
| IN-6 | The project's own `.alpackages` is a dependency source | `DependencyResolver.cs` | No | ADR-013 | ADR-013 pool order: `.alpackages`, then `--deps`, then cache | - |
| IN-7 | An app sees only its declared dependencies (plus `propagateDependencies`); otherwise `AL0185` | `--guide` (#4096) | Done | - | `AppSource` has `visible_from` by dependency id | - |
| IN-8 | AL preprocessor symbols: `--define`, `--preprocessor-symbols`, `app.json`, built-in `CLEANSCHEMA1..25` | `Program.cs:625,640` | Partial | ADR-015 | Preprocessor exists. Today only the corpus harness passes `BC27PLUS`. ADR-015 3.9: other projects get symbols from `app.json` and `--define` only, never invented `BC<n>PLUS`. v2's built-in `CLEANSCHEMA1..25` set is not carried over: the AL compiler predefines no `CLEANSCHEMA<n>` symbol (no DLL of the AL extension 18.1.2842859 holds the text `CLEANSCHEMA`; checked 2026-10-21 for issue 5615), so a project that needs them names them in `app.json` `preprocessorSymbols` or with `--define` | - |
| IN-9 | Run inline AL, or a codeunit's `OnRun`, without tests (`execute`) | `ServerProtocol.cs`, `docs/server-mode.md` | No | CLI | `al-runner run [PATH] [--code TEXT]` and the server `execute` command | - |
| IN-10 | Package a folder as `.app` without compiling (`--emit-app`) | `ProgramSupport/SiblingCompile.cs`, `Program.cs:256` | No | CLI | `al-runner package PATH OUT.app` (zip plus `NavxManifest.xml`); low priority | - |
| IN-11 | Compile one `.app` to a DLL (`--precompile`) | `Program.cs:249` | N/A | - | v3 has no DLLs. Nothing to precompile | - |
| IN-12 | Config file | none (the root `al-runner.json` is not read) | N/A | - | Not parity. A later `al-runner.toml` is optional | - |

## 2. Dependencies, artifacts, platform data

| ID | Feature | v2 reference | v3 today | Blocker | Proposed v3 | Hard |
|---|---|---|---|---|---|---|
| DP-1 | Download missing BC artifacts by default; `--no-auto-provision`; `--auto-provision` | `Program.cs:491` | No | ADR-013 | Download by default. `--no-download` (ADR-013). Keep `--no-auto-provision` as an alias | - |
| DP-2 | `provision` command: for a project, `--platform-apps`, `--test-apps`, `--service-tier`, `--resolve-version`, `--force` | `ProgramSupport/Provisioning.cs`, `tools/DownloadArtifacts/` | No | ADR-013 | `al-runner deps fetch [PATH]`, `deps resolve-version PREFIX`, `--force`. No service-tier set (no DLLs) | - |
| DP-3 | `--bc-version` exact build or prefix; newest match; error lists the builds | `Infrastructure/BcArtifacts.cs:292` | Partial | ADR-013 | Selection logic exists in `al-deps` on branch `deps` (not merged). Same flag | - |
| DP-4 | Floor from `app.json`; never select below it; explicit lower value runs with a warning | `Infrastructure/BcVersionFloor.cs` | Partial | ADR-013 | Same (ADR-013 decision 2) | - |
| DP-5 | `--artifact-path DIR` pins one engine directory | `Program.cs:105` | N/A | - | No engine directory in v3. `--bc-version` plus `--deps` cover it | - |
| DP-6 | `--country CODE`; per-country `platform-apps-<cc>` cache | `Program.cs:492`, `AlRunner.Provisioning/ArtifactDownloader.cs` | No | ADR-013 | `--country`. ADR-013 surveyed only the `platform` artifact. Country artifact source layout is not measured | Hard: country artifact layout unmeasured (needs a survey) |
| DP-7 | `--package-cache PATH` (repeatable); default scan of `~/.bcartifacts.cache`, artifacts dir, `.alpackages` | `Program.cs:539`, `Provisioning.cs:197` | Partial | ADR-013 | `--deps DIR` (ADR-013), `--package-cache` kept as alias. Read v2's artifact folders and the VS Code cache read-only Built: `--package-cache` (repeatable) and `--dep` (package file, app folder or folder). `~/.bcartifacts.cache/sandbox` is read for an exact `--bc-version` build and as the fallback when the feed has no match; there is no general scan. | - |
| DP-8 | Resolver rules: app id authoritative, version is a minimum, highest wins, post-order, cycle error, optional deps skipped | `DependencyResolver.cs` | Partial | ADR-013 | `al-deps` `select.rs` on branch `deps` | - |
| DP-9 | Symbols-only versus code-bearing packages; `NO IMPLEMENTATION` report in an "Action needed" block | `DependencyResolver.cs`, `--guide` | No | ADR-013 | v3 ranks packages with AL source above symbols-only. Same report shape | - |
| DP-10 | A dependency that ships only compiled code (ReadyToRun DLL, no AL source) still runs | `DependencyLoader.cs`, `AppLoader.cs` | No | CANNOT | v3 has no CLR and cannot run compiled C#. Refuse the tests that reach it, naming the app. Advice: keep using v2 for those projects | Hard: not matchable. Affects AppSource and ISV apps shipped without source (`ShowMyCode=false`) |
| DP-11 | Download by ranged ZIP reads; only needed entries | `ArtifactDownloader.cs` | Partial | ADR-013 | ZIP reader with CRC checks exists on branch `deps`. Network client not yet | - |
| DP-12 | Network failure classification; IPv4/IPv6 multi-address connect; an unanswered probe holds the chosen version | `NetworkDiagnosis.cs`, `MultiAddressConnector.cs`, `docs/provisioning-tiers.md` | No | ADR-013 | Port the rules and messages | - |
| DP-13 | Offline and air-gapped use | `--no-auto-provision` | No | ADR-013 | `--no-download`, `--offline` | - |
| DP-14 | Cache location variables `AL_RUNNER_ARTIFACTS_ROOT`, `AL_RUNNER_CACHE_ROOT`, `AL_RUNNER_SYMBOLS_ROOT`, `AL_RUNNER_ENGINE_CACHE_ROOT` | `Infrastructure/CacheRoots.cs` | No | ADR-013 | `ALRUNNER_CACHE` (one root). Symbols and engine roots drop out | - |
| DP-15 | Microsoft Test Runner app loaded by default; its events (`OnBeforeTestMethodRun` and others) raised; `AL_RUNNER_DEFAULT_TEST_TOOL=off` | `docs/limitations.md` "Microsoft's Test Runner - Mgt. events" | Partial | ADR-013 | Corpus test-runner semantics exist in `al-test-cli`. The per-method steps of `Test Runner - Mgt` and 130453 "ALTestRunner Reset Environment" run natively in Microsoft's order and run Microsoft's own codeunits 131006, 130301, 130302 and 132553 when they are loaded (`al-testrun/src/runtime/runner_reset.rs`, ADR-013 "The test runner's steps around each test method"). The app's events are not raised, so other subscribers to them do not run | - |
| DP-16 | Platform system tables served: Date, Integer, AllObj, Company, User, Session, permissions, metadata tables and more | `Patches/RecordPatches.*VirtualTable.cs` | No | NEW-ADR | ADR-013 decision 7 (native providers by table id). ADR-013 says the table-provider seam needs its own ADR first | Hard: about 47 objects, each needs a corpus pin or probe |
| DP-17 | Install triggers of dependency apps, then `Company-Initialize`, then own install triggers, before the first test | `InstallTriggerRunner.cs`, `CompanyInitializer.cs` | No | ADR-013 | ADR-013 decision 5 | - |
| DP-18 | Install baseline cached on disk | `Infrastructure/InstallBaselineDiskCache.cs` | No | ADR-013 | Optional optimisation, only if the warm start needs it | - |
| DP-19 | Identity answers: `TESTUSER`, `My Company`, fixed GUIDs, en-US, Production environment | `BcRuntime.cs:1309,1433,1441` | No | HOST | ADR-013 decision 9 table, as `al-host` defaults | - |
| DP-20 | `--test-data`: fill the in-memory database from a BC `.bak`, one table at first touch | `TestDataProvisioner.cs`, `Infrastructure/BackupReaderServe.cs` | No | NEW-ADR | `--test-data`. Reuse the pinned `bcbak` reader process from the CLI layer. Rows reach the VM through a host capability | Hard: `.bak` format needs the external reader; about 1 GB download; BLOB and media tables refused by v2 too |
| DP-21 | `--test-data-company NAME` (prefix match, prompt on a terminal); `--test-data-normalize-company` | same | No | NEW-ADR | Same flags | - |
| DP-22 | Pinned `bcbak` reader installed on demand, checksum verified; `AL_RUNNER_BCBAK` | `Infrastructure/BackupReaderProvisioning.cs` | No | NEW-ADR | Same; `ALRUNNER_BCBAK` | - |
| DP-23 | `[test-data]` note under a failure that names an empty table | `Reporter.cs` | No | CLI | Same note; the VM must report the table behind a failure | - |
| DP-24 | `DotNet` interop (standard library, community DLLs, Microsoft server DLLs) | `docs/limitations.md` "DotNet interop" | No | ADR-014 | .NET helper process. Needs the .NET 8 runtime | Hard: second runtime; Win32 shim needed on Linux |

## 3. Test selection and execution

| ID | Feature | v2 reference | v3 today | Blocker | Proposed v3 | Hard |
|---|---|---|---|---|---|---|
| SEL-1 | `--test`/`--filter PATTERN`: case-insensitive substring of `CodeunitNNNN.Method` | `Program.cs:602` | Partial | CLI | `--filter`, `--test` alias. Also match by AL names Built: `--filter`, `--test` alias; matches test names (not `CodeunitNNNN.Method`). | - |
| SEL-2 | Exit 6 when the pattern selects no test | `Program.cs` | Partial | CLI | Same. Built: exit 6. | - |
| SEL-3 | `--exclude-test` (whole codeunit or whole qualified name) | `Program.cs:520` | No | CLI | `--exclude-test` | - |
| SEL-4 | `--isolation codeunit` (default; `TestIsolation = Codeunit`) | `TestExecutor.cs`, `docs/limitations.md` | Partial | CLI | Semantics exist and are pinned by the corpus. Not selectable by the user | - |
| SEL-5 | `--isolation test` (= `Function`) and `disabled`; aliases `--test-isolation`, `method` | `Program.cs:658` | Partial | CLI | `Function` is implemented and pinned by the isolation probe (`al-test-cli/src/isolation.rs`; oracle `oracle/28/xunit-isolation-*.xml`). `Disabled` is refused (not pinned). The flag itself is not built: a runner codeunit's `TestIsolation` property selects it for the corpus suites | - |
| SEL-6 | Per-test timeout (`--test-timeout`, `AL_RUNNER_TEST_TIMEOUT_SEC`, default 60 s, message `Test exceeded {N}s timeout.`) | `Program.cs:603` | No | HOST | The VM checks a deadline from the host monotonic clock at statement boundaries. Same message | - |
| SEL-7 | Watchdog for hung tests; `--resume-aborts`; `--merge-counts`; `--merge-results` | `Infrastructure/AbortResume.cs` | N/A | - | The v3 interpreter is cooperative. A timeout ends the one test cleanly. No re-exec needed. Native builtins need a step budget | - |
| SEL-8 | `--jobs N`: worker processes, balanced by file count, memory bound, shared-bundle claiming | `docs/jobs-unit-claiming.md`, `Infrastructure/JobsMemory.cs` | No | CLI | `--jobs N` with threads (one Session per thread, ADR-002). Test codeunits are independent. Not available in WASM | - |
| SEL-9 | `--seed N`, `AL_RUNNER_SEED`; per-test RNG derived from seed, codeunit, method; seed printed and written to JSON and JUnit; replay command | `docs/run-seed.md` | No | HOST | Same flag and derivation. The VM needs a host random source. `Randomize(seed)` must give BC's sequence, so port .NET's legacy `System.Random(seed)` | Hard: sequence must equal .NET's; `CreateGuid` stays random |
| SEL-10 | `--tdd`: generate stubs for missing symbols so the test fails red instead of the compile failing | `TddGeneration.cs`, `TddCallGraph*.cs`, `--guide` "TDD MODE" | Partial (ADR-051: procedures on codeunit, record, page and report variables, `Rec`, `CurrPage`, `this`, bare calls and missing overloads, fields, enum values and missing codeunits, tables and enums, also in `serve`) | ADR-051 | ADR-009 says an optional stricter or looser mode needs an ADR. v3's error-tolerant parser and per-routine refusal make the basic form (report the test as failed, naming the missing symbol) natural | Hard: stub generation infers types from use |
| SEL-11 | Several bundles load together so events cross bundles; `AL_RUNNER_SEQUENTIAL_BUNDLES` | `README.md` (#4931) | No | CLI | Load all, then run | - |
| SEL-12 | `--per-suite`, `--bundled` | `Program.cs:540` | N/A | - | v2 pipeline switches. No equivalent | - |
| SEL-13 | An object that cannot be emitted is dropped; its tests are SKIPPED; other codeunits still run | `ProgramSupport/ExcludedObjectTriage.cs` | Partial | CLI | v3 refuses per routine and reports "not run" with the reason. Finer than v2 | - |
| SEL-14 | Test-runner semantics: `OnRun` once, commit at test end, `asserterror` rollback, handler attributes bound per test, unhandled UI fails | `TestExecutor.cs` | Done | - | In `al-test-cli/src/runtime.rs`. Must move to a library crate both CLIs use | - |

## 4. Output and reporting

| ID | Feature | v2 reference | v3 today | Blocker | Proposed v3 | Hard |
|---|---|---|---|---|---|---|
| OUT-1 | Run header: tool version, BC build, country, app count | `CliText.cs` `RunHeader` | No | CLI | Same line | - |
| OUT-2 | One `FAIL`/`ERROR` line per failing test with BC's message; PASS lines only with `--show-pass` | `Reporter.cs` | Partial | CLI | Same. Built: FAIL lines (first 30, `--verbose` all), NOT RUN reasons ranked; no call stack yet. | - |
| OUT-3 | AL call stack under a failure: object, procedure, line, app, publisher, version | `StackFrameMapper.cs`, `AlCallStackCapture.cs` | Partial | CLI | `GetLastErrorCallStack` returns v2's frame format for codeunits and tables (`al-vm/src/callstack.rs`). The runner does not yet print the stack under a failing test | - |
| OUT-4 | Summary: `Tests:` line, `Seed:` line with replay command, `Result:` line, "Action needed" block | `Reporter.cs` | Partial | CLI | Same shape. Built: `Tests:` and `Result:` lines and a timing line; no `Seed:` line. | - |
| OUT-5 | `--show-pass`, `--failures-only`/`--quiet`, `--verbose` | `Program.cs` | No | CLI | Same flags | - |
| OUT-6 | `--output-junit PATH`, grouped by codeunit, seed property, skipped | `JUnitReport.cs` | Partial | CLI | Writer exists in `al-test-cli/src/output.rs` in the oracle shape (suite name `Codeunit N`; v2 uses `Codeunit50000`). Needs the user-run shape. Not-run tests become `<skipped>` with the reason Built: `--junit`, one suite per codeunit (name), not-run tests `<skipped>` with the reason; no seed property. | - |
| OUT-7 | `--output-json`: per-test status, message, stack, duration, summary, seed, exit code | `Reporter.cs` | No | CLI | Same field names. `al-test-cli --json` is a pass-rate report, a different shape | - |
| OUT-8 | `--coverage`: statement coverage, Cobertura XML, console table, `--coverage-out` | `AlCoverage*.cs`, `scripts/coverage-gen.js` | No | IR-POS | Same flags. v3 can add branch coverage later | Hard: needs statement spans in the IR |
| OUT-9 | Exit codes 0 to 6 and the rank order `3 > 2 > 4 > 6 > 1 > 5` | `README.md` "Exit Codes" | Partial | CLI | Keep 0, 1, 2, 3, 6. Add one new code for "tests not run" (open question). `al-test-cli` has 0, 1, 2 Built: 0, 1, 2, 6, 7 (7 = tests not run, `--allow-not-run`), rank 2, 6, 1, 7. No 3, 4, 5. | - |
| OUT-10 | `--no-strict-exit`: always exit 0 | `Program.cs:601` | No | CLI | Same | - |
| OUT-11 | Output file paths checked before the run; parent folders created; exit 2 if a write fails | `Infrastructure/OutputPaths.cs`, `docs/cli-output-paths.md` | No | CLI | Same | - |
| OUT-12 | Out-of-scope surfaces fail loudly: `RunnerOutOfScopeException` with API, reason, `scope.md` anchor | `docs/scope.md` | Partial | CLI | v3 reports "not run" with a reason (hard rule 4). Keep a `docs/scope.md` equivalent with anchors | - |
| OUT-13 | Notes under a failure: `[testpage]` (real error behind "The TestPage is not open.") | `Reporter.cs` | No | CLI | Same, if the VM can report the hidden error | - |
| OUT-14 | `--dump-csharp DIR` | `Program.cs:651` | N/A | - | No C# stage. Offer `al-runner debug dump-ir` | - |
| OUT-15 | Timing records: `AL_RUNNER_PHASE_LOG` JSONL per phase | `PerfTrace.cs`, `scripts/phase-log-report.py` | No | CLI | `--timings FILE` | - |
| OUT-16 | `--version`, `--help`, `--guide` (manual for automated callers) | `CliText.cs` | No | CLI | `al-runner --version`, `--help`, `al-runner guide` | - |
| OUT-17 | Log tag filter (`[bc]`, `[dep]`, `[warn]`) and trace variables (`AL_RUNNER_TRACE_*`, `AL_RUNNER_DIAG_*`, hook traces) | `Log.cs`, `docs/log-filter.md` | N/A | - | Replaced by `-v` levels and `ALRUNNER_LOG`. Most trace variables describe v2 internals | - |
| OUT-18 | `--print-cache-key` | `Program.cs:585` | N/A | - | Diagnostic of the v2 cache | - |

## 5. Speed and caching

| ID | Feature | v2 reference | v3 today | Blocker | Proposed v3 | Hard |
|---|---|---|---|---|---|---|
| PERF-1 | `--cache DIR`: cache of compiled AL output; `--no-cache` | `AlCacheWriter.cs`, `Program.cs:561,584` | N/A | - | Cold compile of the corpus (1769 objects) is about 0.5 s. No user-code cache. `--cache-dir` moves the dependency cache (ADR-013) | - |
| PERF-2 | Cache of dependency symbols and compiled dependencies | `DepSymbolCompilerCache.cs`, `compiled-deps` | No | ADR-013 | Persistent parse/index cache per dependency, only if the warm start needs it (ADR-013 decision 6) | - |
| PERF-3 | Ncl rewrite cache, shadow runtime, re-exec, Win32 shim, JIT and GC tuning | `NclShadowRuntime.cs`, `Win32Stubs/` | N/A | - | Gone with the .NET engine | - |
| PERF-4 | `--watch`: stay resident; rerun on `.al` save; wait for quiet (250 ms, max 10 s) | `WatchSource.cs`, `AL_RUNNER_WATCH_QUIET_MS`, `AL_RUNNER_WATCH_MAX_WAIT_MS` | No | CLI | `al-runner test --watch`. File watcher in the CLI layer. A full rerun is fast enough | - |
| PERF-5 | Live terminal dashboard; plain lines when not a terminal | `WatchDashboard.cs` | No | CLI | Same behaviour | - |
| PERF-6 | `--watch --affected`, `--include-failing`, `--strict-environment`: rerun only tests an edit can affect | `Infrastructure/Affected*.cs`, `docs/watch-affected.md` | No | NEW-ADR | Deferred. v3 may not need it. Revisit if a full run with Base Application is slow | Hard: per-test dependency tracking, baselines across environments |
| PERF-7 | Scratch folder sweeping; package-dedup cache; memory census | `Infrastructure/ScratchDirs.cs`, `PkgDedupCache.cs` | N/A | - | v3 writes no scratch folders | - |

## 6. Editors, debugging, other tools

The exact interface points of ALchemist and LethAL (file:line, v3 status, gaps, plan) are in
[`docs/integrations/alchemist-lethal.md`](integrations/alchemist-lethal.md).

| ID | Feature | v2 reference | v3 today | Blocker | Proposed v3 | Hard |
|---|---|---|---|---|---|---|
| ED-1 | `--server`: JSON lines on stdin/stdout; `{"ready":true}` first; `runTests` streams one `test` line per test then a `summary` | `ServerProtocol.cs`, `docs/server-mode.md`, `protocol-v2.schema.json` | No | NEW-ADR | `al-runner serve`, speaking protocol-v2 unchanged so ALchemist and LethAL keep working | - |
| ED-2 | `execute` command: inline `code` or one `OnRun` codeunit; one response line | `docs/server-mode.md` | No | NEW-ADR | Same | - |
| ED-3 | `cancel` side channel; the running test finishes, the next does not start | `docs/server-mode.md` | No | NEW-ADR | Needs a cancel flag the VM checks between tests | - |
| ED-4 | Request fields: `sourcePaths`, `packagePaths`, `test`, `excludeTests`, `testIsolation`, `coverage`, `perTestCoverage`, `affectedOnly`, `includeFailing`, `strictEnvironment`, `tdd`, `captureValues`; warnings for ignored fields; refusal of unknown fields | same | No | NEW-ADR | Same set; fields for deferred features answer with a clear refusal | - |
| ED-5 | `stderr` marker `[server] request N done`; warm state across requests; same-bundle reload | same | No | NEW-ADR | Same marker. Reload is a recompile (about 0.5 s) | - |
| ED-6 | `captureValues`: local variable values after each statement | `AlValueCapture.cs` | No | IR-POS | Same response shape | Hard: VM hook per statement plus wire format |
| ED-7 | `iterationTracking`: per-loop iteration records | `AlIterationTracker.cs` | No | IR-POS | Same response shape | Hard: loop model |
| ED-8 | Per-statement hit counts, position table, per-test coverage | `AlCoverageTracker.cs` | No | IR-POS | Same response shape | - |
| ED-9 | `--dap [PORT]` and `--dap stdio`: breakpoints, next/stepIn/stepOut, stack, locals; `linesStartAt1`/`columnsStartAt1` | `Infrastructure/AlDapSession.cs`, `docs/dap-mode.md` | Partial | - | `al-runner debug [--stdio \| --port [N]]` with the same DAP surface (ADR-048, `docs/debugging.md`). Done for the apps on the command line; breakpoints in a dependency not yet | Hard: VM is synchronous (ADR-002), so pausing needs its own thread; not in browser WASM without threads |
| ED-10 | VS Code extension ALchemist (external repo, `SShadowS/ALchemist`) drives `--server` and `--dap stdio` | `docs/server-mode.md` | Partial | - | Same protocols: `al-runner serve` and `al-runner debug --stdio` exist. ALchemist must start the new commands (migration guide, section 9) and add a VS Code debugger contribution (ADR-048) | - |
| ED-11 | LethAL (mutation testing, in `community/LethAL`) drives `--server` and `--output-json`; matches failure text | `LethAL/packages/runner/src/al-runner-transport.ts` | No | NEW-ADR | Works if ED-1 and OUT-7 match. Failure text differs (see below) | - |
| ED-12 | No telemetry; no network use except downloads | `--guide` | Done | - | Keep. State it in the README | - |

## 7. Runtime surface (language and platform)

These rows are the long tail. The corpus pass rate tracks them
(`docs/status.md`). They are listed so parity is not silently narrower than v2.

| ID | Feature | v2 reference | v3 today | Blocker | Proposed v3 | Hard |
|---|---|---|---|---|---|---|
| RT-1 | Records: CRUD, filters, keys, CalcFields/CalcSums, triggers, transactions | `docs/scope.md` section 1 and 2 | Partial | LANG | record 52%, codeunit 61%, types 67% of corpus (status.md) | - |
| RT-2 | Events: integration, business, internal; subscribers; BC lifecycle events | same | Partial | LANG | Implemented; Global Triggers events arrive with ADR-013 | - |
| RT-3 | Handlers: Confirm, Message, ModalPage, Request, Report, Notification | same | Partial | LANG | handlers 45% of corpus. Request and filter pages: 96 tests not started | - |
| RT-4 | `TestPage`, `TestPart`, page triggers | `RunnerPageInstance.cs` | Partial | LANG | ADR-012. Page extensions (94 tests) and RunObject actions (73) not done | - |
| RT-5 | Queries (joins, aggregates, dataset export) | `docs/limitations.md` "Query" | No | LANG | 129 corpus tests, none started | Hard: new object kind |
| RT-6 | XmlPorts | same | Partial | LANG | 25 of 40 pass; rest need dependency apps | - |
| RT-7 | Reports: handler callbacks fire; rendering and layouts do not | `docs/scope.md` | No | LANG | Same line as v2: no rendering | - |
| RT-8 | RecordRef/FieldRef, BLOB/streams, JSON, XML, regex, crypto, IsolatedStorage | `docs/scope.md` | Partial | LANG | Mostly 85-98% in their areas | - |
| RT-9 | Permissions: all granted; Permissions Mock; permission tables | `docs/scope.md` section 2, 3.17 | No | LANG | 58 corpus tests, none passing | - |
| RT-10 | `[TransactionModel]`, guarded `Codeunit.Run` | `docs/limitations.md` | No | LANG | 58 and 43 corpus tests | - |
| RT-11 | System Application and Base Application code runs as real code | `docs/scope.md` section 1 | No | ADR-013 | Compile Microsoft's source on demand | Hard: Base Application is 113 MB of AL; compile 16.5 s measured (ADR-013) |
| RT-12 | Microsoft's own test buckets (Tests-Misc about 3,200 tests, and others) run with `--jobs` | `.claude` skill `running-ms-test-buckets`, `.github/workflows/ms-bucket.yml` | No | ADR-013 | Real-project gate (section "Readiness gates" in ADR-016) | Hard: needs most of the above |
| RT-13 | Refused by design: SMTP, outbound HTTP, external files, OData/SOAP, printers, scheduler, rendering | `docs/scope.md` section 3 | Partial | LANG | Refusal framework exists. Keep the list and anchors. Host-visible HTTP is better in v3 (ADR-009) | - |
| RT-14 | BC versions 27.0 to 28.5 (nine minors) | `.github/bc-versions.txt` | Partial | ADR-015 | v3 emulates BC 27.5 only today. ADR-015 (Proposed): one `Target` per run, BC 27 and later, oracles per major | Hard: version differences are behaviour, each needs oracle data |
| RT-15 | Fixed culture en-US; Production environment; on-premises topology | `docs/limitations.md` | Partial | HOST | ADR-013 decision 9 | - |

## 8. Distribution and CI use

| ID | Feature | v2 reference | v3 today | Blocker | Proposed v3 | Hard |
|---|---|---|---|---|---|---|
| DIST-1 | `dotnet tool install -g MSDyn365BC.AL.Runner`; needs the .NET SDK 9 or 10 | `README.md`, `AlRunner/AlRunner.csproj` | N/A | - | Self-contained binary. No SDK. Helper needs .NET 8 runtime only for `DotNet` | - |
| DIST-2 | Windows, Linux (x64, arm64), macOS | `README.md` | No | CLI | Release matrix (ADR-016) | - |
| DIST-3 | Linux needs no C compiler on x64 and arm64 (prebuilt Win32 shim) | `README.md` | N/A | - | No shim for core. Helper ships one per Linux RID | - |
| DIST-4 | One engine variant per BC minor inside the package | `AlRunner.csproj:255-290` | N/A | - | One binary for all BC versions (ADR-015) | - |
| DIST-5 | Authenticode-signed assemblies (Azure Artifact Signing, OIDC) | `.github/workflows/publish.yml` (job `sign-and-pack`) | No | CLI | Sign the Windows exe the same way; notarize macOS | - |
| CI-1 | JUnit XML for GitHub Actions, Azure DevOps, GitLab | `CHANGELOG.md` 1.0.12 | Partial | CLI | See OUT-6 | - |
| CI-2 | Cobertura XML for coverage tabs | `.github/workflows/coverage-demo.yml` | No | IR-POS | See OUT-8 | - |
| CI-3 | Exit codes as a gate; `--no-strict-exit` to parse output | `README.md` | Partial | CLI | See OUT-9, OUT-10 | - |
| CI-4 | Cache artifacts between CI runs by pointing the artifact root at a mounted folder | `README.md` `AL_RUNNER_ARTIFACTS_ROOT` | No | ADR-013 | `ALRUNNER_CACHE`; cache key = BC build | - |
| CI-5 | AL-Go integration | none in v2 | N/A | - | Not parity. Optional: a `setup-alrunner` GitHub Action | - |
| CI-6 | Run the whole job without Docker, SQL or a license | `README.md` | Partial | ADR-013 | True for the corpus-only path today | - |

## 9. Environment variables

| ID | Feature | v2 reference | v3 today | Blocker | Proposed v3 | Hard |
|---|---|---|---|---|---|---|
| ENV-1 | `AL_RUNNER_VERBOSE`, `AL_RUNNER_SHOW_PASS`, `AL_RUNNER_FAILURES_ONLY` | `CliText.cs` | No | CLI | `ALRUNNER_VERBOSE` and the same two | - |
| ENV-2 | `AL_RUNNER_TEST_TIMEOUT_SEC`, `AL_RUNNER_SEED` | `Program.cs` | No | CLI | `ALRUNNER_TEST_TIMEOUT`, `ALRUNNER_SEED` | - |
| ENV-3 | `AL_RUNNER_ARTIFACTS_ROOT`, `_CACHE_ROOT`, `_SYMBOLS_ROOT`, `_ENGINE_CACHE_ROOT` | `CacheRoots.cs` | No | ADR-013 | See DP-14 | - |
| ENV-4 | `AL_RUNNER_DEFAULT_TEST_TOOL`, `AL_RUNNER_NO_DEP_COMPANY_CACHE`, `AL_RUNNER_BCBAK`, `AL_RUNNER_WATCH_*` | `README.md` | No | ADR-013 | Matching `ALRUNNER_*` names where the feature exists | - |
| ENV-5 | `AL_RUNNER_EMIT_TIMEOUT_SEC`, `AL_RUNNER_NCL_CACHE`, `AL_RUNNER_WIN32_STUBS_SO`, `AL_RUNNER_TRACE_*`, `AL_RUNNER_DIAG_*`, `AL_RUNNER_HOOK_*`, `DOTNET_GCDynamicAdaptationMode`, `AL_RUNNER_JOBS_*` | `CliText.cs` | N/A | - | Describe v2 internals | - |

## 10. Dev-only features (stay in `al-test-cli`)

These serve the v2 repository's own corpus runs. A user of a project does not
need them. v3 already has the same job done by the oracle and the ratchet
(ADR-007).

| ID | Feature | v2 reference | v3 equivalent |
|---|---|---|---|
| DEV-1 | `--expectations DIR`, `--expectations-require-match` (declare out-of-scope and known-gap tests) | `ExpectationManifest.cs`, `docs/expectations.md` | `oracle/` plus `metrics/<major>/passing.txt`. A user-facing "known gap" file is optional later (open question) |
| DEV-2 | `--count-baseline`, `--count-baseline-require-all`, `--count-out` | `Infrastructure/CountBaseline.cs` | The ratchet |
| DEV-3 | `--out`, `--classify`: failure classification JSON | `Reporter.cs` | `al-test-cli --json`, `--markdown` |
| DEV-4 | Corpus checkout, MS bucket and mutation tooling | `tools/*.py`, `.github/workflows/ms-bucket*.yml` | `al-test-cli`, `check.sh` |

These ten dev rows are not counted in the headline.

## Headline numbers

118 user-facing features are counted (sections 1 to 9). The 4 dev-only rows are not.

| v3 today | Features |
|---|---:|
| Done | 3 |
| Partial | 24 |
| No | 75 |
| N/A (gone or replaced by design) | 16 |

The 99 features that are Partial or No, by first blocker:

| Blocker | Features | Note |
|---|---:|---|
| CLI (only the user command line is missing) | 35 | Work in a new crate, ADR-016 |
| ADR-013 (dependencies, artifacts, baseline) | 24 | ADR-013 is accepted; `al-deps` is on branch `deps` |
| NEW-ADR (a new ADR first) | 14 | Server protocol, debugger, table providers, test data, `--tdd`, `--affected` |
| LANG (language and platform surface) | 11 | Tracked by the corpus pass rate |
| IR-POS (source positions in the IR) | 6 | Coverage, call stacks, value capture, loop tracking |
| HOST (`al-host` capability) | 4 | Clock, randomness, identity |
| ADR-015 (many BC versions) | 2 | In progress |
| ADR-014 (.NET helper) | 1 | Proposed |
| PROBE (needs a corpus pin first) | 1 | `--isolation disabled` |
| CANNOT (v3 design cannot match) | 1 | Compiled-only dependency apps (DP-10) |

17 of the 118 are marked hard. 3 of the 118 are done today. Honest reading:
v3 is a runtime without a product around it. The 35 CLI-only rows are
mechanical. The 24 dependency rows decide whether real projects run at all.

The counts come from the tables above. Regenerate them if a row changes.

## What is hard to match, and why

1. **DP-10, compiled-only dependencies.** v2 loads Microsoft's CLR code. v3 has
   no CLR. An app that ships without AL source cannot run. This is the one
   feature that v3 cannot match. It touches ISV and AppSource apps that set
   `ShowMyCode = false`. Microsoft's own apps are not affected, because their
   source is in the artifacts (ADR-013).
2. **RT-14 and ADR-015, many BC versions.** v2 ships an engine per minor.
   In v3 each version difference is a behaviour that needs oracle evidence.
   The corpus CI has results for BC 27.0 to 29.0 (ADR-007), so the evidence
   exists, but each difference still needs work.
3. **DP-16 and RT-11, platform tables and Base Application.** About 47 system
   tables need native row providers, each with a pin. Base Application source
   takes 16.5 s to compile today (ADR-013), which is 16 times the corpus budget.
4. **DP-20, `--test-data`.** It needs a `.bak` reader. v2 uses a separate pinned
   tool. v3 can run the same tool, but then `--test-data` is not available
   where processes cannot run (WASM).
5. **ED-9 and ED-6 to ED-8, debugging and coverage.** All need source positions
   in the IR. The debugger also needs the synchronous VM to pause.
6. **SEL-9, seed.** `Randomize(seed)` must produce the sequence .NET makes.
   That is a port of a known algorithm, but it is observable.
7. **SEL-10, `--tdd`.** Stub generation infers types from use. It is a design
   choice that ADR-009 says needs its own ADR.
8. **PERF-6, `--affected`.** Per-test dependency tracking. Probably not worth it
   at v3 speed.
9. **RT-5, RT-9, RT-10.** Queries, permissions and transaction models are large
   areas with no passing test yet.

## Differences users will notice (not features)

- **Failure text.** v2 prefixes the BC message with a .NET exception type, for
  example `NavNCLDialogException: Assert.AreEqual failed...`. v3 has no CLR.
  It will print BC's message without the prefix (ADR-009: the message is the
  observable part). Scripts and tools that match the prefix must change.
  LethAL matches failure text and needs a check.
- **Test names in JUnit.** v2: suite `Codeunit50000`, case `Passes`. v3's oracle
  shape uses `Codeunit 50000`. The user-run report should copy v2's names.
- **Not run versus failed.** v2 turns an unsupported surface into a failing
  test. v3 reports "not run". Exit code policy is an open question in ADR-016.
- **Speed.** v2 pays seconds of startup. v3 is about 1 s for 4836 tests with no
  dependency apps. With Base Application it is not measured yet.
