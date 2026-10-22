# AL Runner v2 parity inventory

This page lists every user-facing feature of AL Runner v2 and says what AL Runner
does with it today. It is the parity page of ADR-016 (gate R, item 1).

- First written 2026-10-13 (ADR-016, "Release: AL Runner v3").
- Refreshed 2026-10-22 against `main` at `4111976b8`. Every row was checked
  against the code, `al-runner --help` and the docs on that commit.

The owner's direction: AL Runner must support everything v2 offers. Commands,
flags and calls may differ. A migration step and a migration guide are
acceptable. So a row is `Done` when the feature exists in another form and the
migration guides say how to move.

## How the v2 side was found

- The v2 source (now the `v2` branch of this repository), commit `351c2813d`
  (2026-10-06). The v2 references below are paths in it.
- The output of the v2.12.0 tool: `al-runner --help`, `--guide` and
  `provision --help`. The help text is generated from the code, so it is the
  authority for flags.
- v2's docs: `README.md`, `docs/manual/*`, `docs/server-mode.md`,
  `docs/dap-mode.md`, `docs/watch-affected.md`, `docs/run-seed.md`,
  `docs/cli-output-paths.md`, `docs/provisioning.md`, `docs/limitations.md`,
  `docs/scope.md`, `docs/expectations.md`, `docs/jobs-unit-claiming.md`,
  `docs/startup-cost.md`, `docs/v1-to-v2-migration.md`, and its workflows.
- Not found in v2: a config file (the `al-runner.json` in the v2 root is a v1
  leftover that no code reads), an AL-Go integration, an Azure DevOps task,
  telemetry.

## Status values

- **Done**: AL Runner has the feature today, maybe with another command or flag.
  The row cites the merged pull request of
  `aldevtools/BusinessCentral.AL.Runner` or the doc that describes it. Features
  built before the repository move (2026-10-21) cite the doc, because their pull
  requests are in the old working repository.
- **Partial**: part of it exists. The row says what is missing.
- **Open**: not built, and no owner decision says it will not be.
- **Not planned**: the owner decided not to build it. The row names the decision.
- **N/A**: the reason for the feature is gone in AL Runner, or the design replaces
  it (for example: v2 compiled AL to DLLs, AL Runner has no DLLs).

Terms: a **bundle** is a folder with an `app.json` (v2's name; AL Runner says "app
folder"). The **corpus** is the AL language test corpus, the oracle (ADR-007).
The **migration guides** are `docs/manual/migrating-from-v2.md` (for users) and
`docs/integrations/migrating-tools-to-v3.md` (for tools such as ALchemist and
LethAL).

## 1. Inputs

| ID | Feature | v2 reference | Status | AL Runner today |
|---|---|---|---|---|
| IN-1 | One or more bundle folders as arguments; one combined summary | `Program.cs` arg loop (lines 480-660); `ProgramSupport/CliText.cs` | Done | `al-runner test APP...`. APP is an app folder, a `*.Source.zip` or an `.app` with source (`--help`, USAGE). |
| IN-2 | Bundle root found by climbing from any folder below an `app.json` | `Infrastructure/BundleRootValidation.cs` | Open | A folder without an `app.json` is an error (`crates/alrunner-cli/src/apps.rs`, `load`). |
| IN-3 | A folder holding several apps runs as one bundle per app | `ProgramSupport/Suites.cs`, `docs/server-mode.md` (#5107) | Open | Each APP must be named. Only `--affected-from PATH` splits a folder of test apps (PR 5653). |
| IN-4 | The same folder named twice runs once | `Infrastructure/BundleRootDeduplication.cs` | Open | Not checked for APP arguments. |
| IN-5 | `app.json` read for identity, dependencies, `platform`/`application` floor, `propagateDependencies`, `internalsVisibleTo`, `preprocessorSymbols` | `ProgramSupport/Dependencies.cs:261`, `Infrastructure/InProcessAppPackager.cs:178` | Partial | Identity, dependencies, the floor and `preprocessorSymbols` are read (`apps.rs`). Missing: `propagateDependencies` and `internalsVisibleTo` are not read. |
| IN-6 | The project's own `.alpackages` is a dependency source | `DependencyResolver.cs` | Done | Order: `--dep`, `--package-cache`, `.alpackages`, then the artifact feed (`--help`, DEPENDENCIES). |
| IN-7 | An app sees only its declared dependencies (plus `propagateDependencies`); otherwise `AL0185` | `--guide` (#4096) | Done | `AppSource` has `visible_from` by dependency id. |
| IN-8 | Preprocessor symbols: `--define`, `--preprocessor-symbols`, `app.json`, built-in `CLEANSCHEMA1..25` | `Program.cs:625,640` | Done | `--define`, `--preprocessor-symbols` and `app.json` work (`--help`, OTHER). v2's built-in `CLEANSCHEMA1..25` set is not copied: the AL compiler predefines none of them (ADR-015 3.9; issue 5615, closed). Projects that need them pass `--define`. |
| IN-9 | Run inline AL, or a codeunit's `OnRun`, without tests (`execute`) | `ServerProtocol.cs`, `docs/server-mode.md` | Partial | The `serve` command `execute` works (migrating-tools, section 4b). Missing: a command-line form; `al-runner run` is refused as not built (`args.rs`). |
| IN-10 | Package a folder as `.app` without compiling (`--emit-app`) | `ProgramSupport/SiblingCompile.cs`, `Program.cs:256` | Open | `al-runner package` is refused as not built (`args.rs`). |
| IN-11 | Compile one `.app` to a DLL (`--precompile`) | `Program.cs:249` | N/A | AL Runner has no DLLs. |
| IN-12 | Config file | none (the root `al-runner.json` is not read) | N/A | v2 has none either. |

## 2. Dependencies, artifacts, platform data

| ID | Feature | v2 reference | Status | AL Runner today |
|---|---|---|---|---|
| DP-1 | Download missing BC artifacts by default; `--no-auto-provision`; `--auto-provision` | `Program.cs:491` | Partial | Downloads by default; `--no-download` turns it off; `--auto-provision` is accepted and ignored (`--help`). Missing: v2's `--no-auto-provision` is an unknown option. |
| DP-2 | `provision` command: `--platform-apps`, `--test-apps`, `--service-tier`, `--resolve-version`, `--force` | `ProgramSupport/Provisioning.cs`, `tools/DownloadArtifacts/` | Partial | `al-runner warm` fetches and opens the demo backup ahead of time. Missing: a command that fetches the apps alone, and `--resolve-version`; `al-runner deps` is refused as not built. |
| DP-3 | `--bc-version` exact build or prefix; newest match; error lists the builds | `Infrastructure/BcArtifacts.cs:292` | Done | Same flag (`--help`, DEPENDENCIES; `crates/al-deps/src/build.rs`). |
| DP-4 | Floor from `app.json`; never below it; an explicit lower value runs with a warning | `Infrastructure/BcVersionFloor.cs` | Done | `BuildQuery::admits` and `floor_warning` in `crates/al-deps/src/build.rs`. |
| DP-5 | `--artifact-path DIR` pins one engine directory | `Program.cs:105` | N/A | No engine directory. `--bc-version` and `--dep` cover it. |
| DP-6 | `--country CODE`; per-country cache | `Program.cs:492`, `AlRunner.Provisioning/ArtifactDownloader.cs` | Done | `--country` takes the country's Microsoft apps and demo backup (`--help`, COUNTRY). |
| DP-7 | `--package-cache PATH` (repeatable); default scan of `~/.bcartifacts.cache`, artifacts dir, `.alpackages` | `Program.cs:539`, `Provisioning.cs:197` | Partial | `--package-cache` and `--dep` work. `~/.bcartifacts.cache/sandbox` is read for an exact build and as the fallback. Missing: a general scan of that folder. |
| DP-8 | Resolver rules: app id wins, version is a minimum, highest wins, cycle error, optional dependencies skipped | `DependencyResolver.cs` | Done | `crates/al-deps/src/select.rs`. |
| DP-9 | Symbols-only versus code-bearing packages; `NO IMPLEMENTATION` report in an "Action needed" block | `DependencyResolver.cs`, `--guide` | Partial | Packages with source win; a missing source names the symbol-only packages found (`select.rs`). Missing: v2's "Action needed" block. |
| DP-10 | A dependency that ships only compiled code (no AL source) still runs | `DependencyLoader.cs`, `AppLoader.cs` | Open | Cannot be matched: AL Runner has no CLR. Tests that reach such an app are not run, and the message names the app (`docs/manual/what-works.md`). Needs an owner deferral (ADR-016, open question 8). |
| DP-11 | Download by ranged ZIP reads; only needed entries | `ArtifactDownloader.cs` | Done | `crates/al-deps/src/zip.rs`, `remote.rs`. |
| DP-12 | Network failure classification; IPv4/IPv6 multi-address connect | `NetworkDiagnosis.cs`, `MultiAddressConnector.cs` | Partial | Retries and HTTP error texts (`remote.rs`). Missing: v2's failure classes and multi-address connect. |
| DP-13 | Offline and air-gapped use | `--no-auto-provision` | Done | `--no-download`; a cached build is the fallback when the feed is offline (`--help`). |
| DP-14 | Cache location variables `AL_RUNNER_ARTIFACTS_ROOT`, `_CACHE_ROOT`, `_SYMBOLS_ROOT`, `_ENGINE_CACHE_ROOT` | `Infrastructure/CacheRoots.cs` | Done | One root: `--cache-dir` or `ALRUNNER_CACHE` (`--help`). |
| DP-15 | Microsoft Test Runner app loaded by default; its events raised; `AL_RUNNER_DEFAULT_TEST_TOOL=off` | `docs/limitations.md` | Partial | The per-method steps of `Test Runner - Mgt` and 130453 run natively in Microsoft's order and call Microsoft's codeunits when loaded (`crates/al-testrun/src/runtime/runner_reset.rs`). Missing: the app's events are not raised for other subscribers. |
| DP-16 | Platform system tables: Date, Integer, AllObj, Company, User, Session, permissions, metadata | `Patches/RecordPatches.*VirtualTable.cs` | Partial | Row providers (ADR-019, `crates/al-vm/src/platform/`). A table without a provider is refused. |
| DP-17 | Install triggers of dependency apps, then `Company-Initialize`, then own install triggers | `InstallTriggerRunner.cs`, `CompanyInitializer.cs` | Done | The install baseline (ADR-013 decision 5; `--help`, COMPILE CACHE). |
| DP-18 | Install baseline cached on disk | `Infrastructure/InstallBaselineDiskCache.cs` | Done | Kept in the compile cache (ADR-022). |
| DP-19 | Identity answers: `TESTUSER`, `My Company`, fixed GUIDs, en-US | `BcRuntime.cs:1309,1433,1441` | Done | `docs/manual/what-works.md`, "Replaced by something equivalent". |
| DP-20 | `--test-data`: fill the database from a BC `.bak`, one table at first touch | `TestDataProvisioner.cs` | Done | On by default with Microsoft's apps (ADR-020; `docs/manual/test-data.md`). `.bacpac` works too. |
| DP-21 | `--test-data-company NAME` (prefix match); `--test-data-normalize-company` | same | Partial | `--test-data-company` works. Missing: `--test-data-normalize-company` is refused (`args.rs`). |
| DP-22 | Pinned `bcbak` reader installed on demand; `AL_RUNNER_BCBAK` | `Infrastructure/BackupReaderProvisioning.cs` | Done | The reader is built in (`crates/al-bcdb`). Nothing to install. |
| DP-23 | `[test-data]` note under a failure that names an empty table | `Reporter.cs` | Open | The summary lists the tables loaded and left out, but no note under a failure. |
| DP-24 | `DotNet` interop | `docs/limitations.md` | Partial | A closed set of framework members runs inside `al-runner`; the rest goes to the .NET 8 helper (ADR-014). Missing: a prebuilt helper; the helper is built on first use and needs the .NET 8 SDK (`docs/manual/what-works.md`, "DotNet"). |

## 3. Test selection and execution

| ID | Feature | v2 reference | Status | AL Runner today |
|---|---|---|---|---|
| SEL-1 | `--test`/`--filter PATTERN`: substring of `CodeunitNNNN.Method`, any case | `Program.cs:602` | Done | `--filter`, `--test`, `--test-exact`, `--codeunit` (`--help`, SELECTION; PR 5622). A pattern without a dot matches the method name only (migrating-tools, section 3). |
| SEL-2 | Exit 6 when the pattern selects no test | `Program.cs` | Done | Exit 6, also in `--output-json` (PR 5622). |
| SEL-3 | `--exclude-test` | `Program.cs:520` | Done | `--help`, SELECTION. |
| SEL-4 | `--isolation codeunit` (default) | `TestExecutor.cs` | Done | `--help`, ISOLATION. |
| SEL-5 | `--isolation test` and `disabled`; aliases | `Program.cs:658` | Partial | `test` (aliases `function`, `method`) works. Missing: `disabled` refuses every test (issue 5609), and `Commit()` under `test` is refused (issue 5607). |
| SEL-6 | Per-test timeout (`--test-timeout`, `AL_RUNNER_TEST_TIMEOUT_SEC`, message `Test exceeded {N}s timeout.`) | `Program.cs:603` | Done | Same flag, variable and message. The limit is CPU time, default 20 s; `Sleep` time counts (PR 5640). |
| SEL-7 | Watchdog for hung tests; `--resume-aborts`; `--merge-counts`; `--merge-results` | `Infrastructure/AbortResume.cs` | N/A | The time limit stops one test cleanly. No re-exec. |
| SEL-8 | `--jobs N`: parallel workers | `docs/jobs-unit-claiming.md` | Done | Test codeunits run on threads by default, one per core; `AL_TEST_THREADS` sets the count (`--help`, ENVIRONMENT). There is no `--jobs` flag. |
| SEL-9 | `--seed N`, `AL_RUNNER_SEED`; seed printed; replay | `docs/run-seed.md` | Open | No seed. Needs an owner decision (ADR-016 gate R1). |
| SEL-10 | `--tdd`: stubs for missing symbols, so the test fails red instead of the compile failing | `TddGeneration.cs`, `--guide` "TDD MODE" | Done | ADR-051: procedures, overloads, fields, enum values, missing codeunits, tables and enums, also in `serve` (PRs 5601, 5616, 5630, 5638, 5644, 5657). |
| SEL-11 | Several bundles load together so events cross bundles | `README.md` (#4931) | Done | All APPs compile into one program (`apps.rs`, `combined_project`). |
| SEL-12 | `--per-suite`, `--bundled` | `Program.cs:540` | N/A | v2 pipeline switches. `--per-suite` is refused with a reason. |
| SEL-13 | An object that cannot be emitted is dropped; its tests are skipped; other codeunits still run | `ProgramSupport/ExcludedObjectTriage.cs` | Done | Refusal per routine: a test is "not run" with the reason (`docs/manual/results.md`). |
| SEL-14 | Test-runner semantics: `OnRun` once, commit at test end, `asserterror` rollback, handlers, unhandled UI fails | `TestExecutor.cs` | Done | `crates/al-testrun`, pinned by the corpus. |

## 4. Output and reporting

| ID | Feature | v2 reference | Status | AL Runner today |
|---|---|---|---|---|
| OUT-1 | Run header: tool version, BC build, country, app count | `CliText.cs` `RunHeader` | Partial | The version line by default; the build, apps and dependencies only with `--verbose` (`--help`, OUTPUT). |
| OUT-2 | One `FAIL` line per failing test; PASS lines only with `--show-pass` | `Reporter.cs` | Done | FAIL lines by default; PASS lines with `--verbose` (`--help`, OUTPUT). `--show-pass` is not accepted. |
| OUT-3 | AL call stack under a failure | `StackFrameMapper.cs`, `AlCallStackCapture.cs` | Done | Printed under each failing test and written to `--junit` and `--output-json` (`--help`, OUTPUT). |
| OUT-4 | Summary: `Tests:`, `Seed:`, `Result:`, "Action needed" block | `Reporter.cs` | Partial | `Tests:`, totals, time and `Result:` lines. Missing: the `Seed:` line (see SEL-9) and the "Action needed" block. |
| OUT-5 | `--show-pass`, `--failures-only`/`--quiet`, `--verbose` | `Program.cs` | Done | `--verbose`, `--quiet`; the default shows only failures. The v2 names `--show-pass` and `--failures-only` are unknown options. |
| OUT-6 | `--output-junit PATH`, grouped by codeunit, seed property, skipped | `JUnitReport.cs` | Done | `--junit FILE`: one suite per codeunit, not-run tests are `<skipped>` with the reason. No seed property. |
| OUT-7 | `--output-json`: per-test status, message, stack, duration, summary, exit code | `Reporter.cs` | Done | v2's shape (migrating-tools, section 4). Adds `notRun`, `sleptMs` (PR 5652). |
| OUT-8 | `--coverage`: Cobertura XML, `--coverage-out` | `AlCoverage*.cs` | Done | `--coverage`, `--coverage-out`, `--coverage-json` (migrating-tools, section 4a). |
| OUT-9 | Exit codes 0 to 6 and their order | `README.md` "Exit Codes" | Done | 0, 1, 2, 3, 6, plus 7 and 8 for tests not run (`--help`, EXIT CODES). Exit 3 (the APPs do not compile) came with PR 5623. v2's 4 and 5 belong to dev-only flags (DEV-1, DEV-2). |
| OUT-10 | `--no-strict-exit`: always exit 0 | `Program.cs:601` | Open | Not built. `--allow-not-run` turns only 7 and 8 into 0. |
| OUT-11 | Output paths checked before the run; parent folders created | `Infrastructure/OutputPaths.cs` | Done | `crates/alrunner-cli/src/run.rs` (`write_file`, the check before the run). |
| OUT-12 | Out-of-scope surfaces fail loudly, with a reason and a scope anchor | `docs/scope.md` | Done | A refused test is "not run" with the reason (hard rule 4). The scope list is ADR-027 and `docs/manual/what-works.md`. |
| OUT-13 | `[testpage]` note under a failure (the real error behind "The TestPage is not open.") | `Reporter.cs` | Open | Not built. |
| OUT-14 | `--dump-csharp DIR` | `Program.cs:651` | N/A | No C# stage. |
| OUT-15 | Timing records: `AL_RUNNER_PHASE_LOG` | `PerfTrace.cs` | Done | `--timings` prints the compile phases and the slowest codeunits. |
| OUT-16 | `--version`, `--help`, `--guide` | `CliText.cs` | Partial | `--version` and `--help` work. Missing: `al-runner guide` is refused as not built (`args.rs`). |
| OUT-17 | Log tag filter and trace variables | `Log.cs`, `docs/log-filter.md` | N/A | They describe v2 internals. `--verbose` covers the rest. |
| OUT-18 | `--print-cache-key` | `Program.cs:585` | N/A | v2 cache diagnostic. `al-runner cache info` lists the cache. |

## 5. Speed and caching

| ID | Feature | v2 reference | Status | AL Runner today |
|---|---|---|---|---|
| PERF-1 | `--cache DIR`: cache of compiled AL output; `--no-cache` | `AlCacheWriter.cs` | N/A | The user's own code compiles in well under a second. The dependency cache is PERF-2. |
| PERF-2 | Cache of dependency symbols and compiled dependencies | `DepSymbolCompilerCache.cs` | Done | The compile cache (ADR-022; `--help`, COMPILE CACHE). |
| PERF-3 | Ncl rewrite cache, shadow runtime, re-exec, Win32 shim | `NclShadowRuntime.cs`, `Win32Stubs/` | N/A | Gone with the .NET engine. |
| PERF-4 | `--watch`: rerun on save | `WatchSource.cs` | Not planned | ADR-049, Rejected by the owner on 2026-10-21. `--watch` is refused with a reason. |
| PERF-5 | Live terminal dashboard | `WatchDashboard.cs` | Not planned | Part of watch mode (ADR-049). |
| PERF-6 | `--affected`, `--include-failing`, `--strict-environment` | `Infrastructure/Affected*.cs`, `docs/watch-affected.md` | Done | Without `--watch`: `al-runner test --affected` and `serve` `affectedOnly` (ADR-050; PRs 5632, 5634, 5643, 5649). |
| PERF-7 | Scratch folder sweeping; package-dedup cache; memory census | `Infrastructure/ScratchDirs.cs` | N/A | No scratch folders. The cache cleans itself (`--help`, COMPILE CACHE). |

## 6. Editors, debugging, other tools

The interface points of ALchemist and LethAL are in
[`docs/integrations/alchemist-lethal.md`](integrations/alchemist-lethal.md), and the
steps for each tool in
[`docs/integrations/migrating-tools-to-v3.md`](integrations/migrating-tools-to-v3.md).

| ID | Feature | v2 reference | Status | AL Runner today |
|---|---|---|---|---|
| ED-1 | `--server`: JSON lines; `{"ready":true}` first; `runTests` streams `test` lines, then a `summary` | `ServerProtocol.cs`, `docs/server-mode.md` | Done | `al-runner serve` (ADR-028, still Proposed; migrating-tools, section 4b). |
| ED-2 | `execute`: inline `code` or one `OnRun` codeunit | `docs/server-mode.md` | Done | migrating-tools, section 4b. |
| ED-3 | `cancel`: the running test finishes, the next does not start | `docs/server-mode.md` | Done | migrating-tools, section 4b. |
| ED-4 | Request fields (`sourcePaths`, `test`, `coverage`, `tdd`, `affectedOnly`, `captureValues`, ...); unknown fields refused | same | Done | All of v2's fields; unknown fields answer `{"error": ...}` (migrating-tools, section 4b; PRs 5616, 5649). |
| ED-5 | `[server] request N done` on stderr; warm state across requests | same | Done | Same marker; a request with the same sources reuses the compile, and a code-only change compiles only the changed objects (`--help`, SERVE). |
| ED-6 | `captureValues`: local variable values after each statement | `AlValueCapture.cs` | Done | ADR-030 (still Proposed); migrating-tools, section 4b. |
| ED-7 | `iterationTracking`: per-loop iteration records | `AlIterationTracker.cs` | Done | ADR-030; migrating-tools, section 4b. |
| ED-8 | Per-statement hit counts, per-test coverage | `AlCoverageTracker.cs` | Done | `coverage` and `perTestCoverage` in `serve`; `--coverage-json` (migrating-tools, section 4a). |
| ED-9 | `--dap [PORT]` and `--dap stdio`: breakpoints, stepping, stack, locals | `Infrastructure/AlDapSession.cs`, `docs/dap-mode.md` | Done | `al-runner debug [--stdio \| --port [N]]` (ADR-048; `docs/debugging.md`; PRs 5628, 5633, 5636). Also in dependency apps compiled from source. |
| ED-10 | ALchemist drives `--server` and `--dap stdio` | `docs/server-mode.md` | Partial | AL Runner's side is done (PR 5637; migrating-tools, section 9). Missing: ALchemist must start the new commands, and its owner has not confirmed (ADR-016 gate R4). |
| ED-11 | LethAL drives `--server` and `--output-json`; matches failure text | `LethAL/packages/runner/src/al-runner-transport.ts` | Partial | AL Runner's side is done (migrating-tools, section 8). Missing: LethAL's changes, and its owner's confirmation (gate R4). |
| ED-12 | No telemetry; no network use except downloads | `--guide` | Done | Same. |

## 7. Runtime surface (language and platform)

These rows are the long tail. The corpus tracks them. On 2026-10-21 the corpus
passed 81-82% on BC 27.5, 28.5 and 29.0, with 0 failing; the rest is refused with a
reason (`README.md`, "Status"). Per-area numbers are in `docs/status.md`, which is
older.

| ID | Feature | v2 reference | Status | AL Runner today |
|---|---|---|---|---|
| RT-1 | Records: CRUD, filters, keys, CalcFields/CalcSums, triggers, transactions | `docs/scope.md` sections 1 and 2 | Partial | Runs as BC does (`docs/manual/what-works.md`). Missing: row locks, other sessions, `LockTable(false)`. |
| RT-2 | Events: integration, business, internal; subscribers; lifecycle events | same | Partial | Implemented and pinned. Missing: the Test Runner app's events (DP-15). |
| RT-3 | Handlers: Confirm, Message, ModalPage, Request, Report, Notification | same | Partial | Implemented; refusals remain in the corpus. |
| RT-4 | `TestPage`, `TestPart`, page triggers | `RunnerPageInstance.cs` | Partial | Headless test client (ADR-012). Refusals remain in the corpus. |
| RT-5 | Queries | `docs/limitations.md` "Query" | Partial | The shapes the corpus pins run (`what-works.md`). |
| RT-6 | XmlPorts | same | Partial | Import and export run; refusals remain in the corpus. |
| RT-7 | Reports: handler callbacks fire; no rendering | `docs/scope.md` | Done | Processing, request pages and the XML dataset run; rendering is refused, as in v2 (`what-works.md`). |
| RT-8 | RecordRef/FieldRef, BLOB/streams, JSON, XML, regex, crypto, IsolatedStorage | `docs/scope.md` | Partial | Most run. User scopes of IsolatedStorage are refused. |
| RT-9 | Permissions: all granted; Permissions Mock; permission tables | `docs/scope.md` sections 2, 3.17 | Partial | Everything is granted. Refusals remain in the permissions area of the corpus. |
| RT-10 | `[TransactionModel]`, guarded `Codeunit.Run` | `docs/limitations.md` | Partial | Implemented (`crates/al-vm/src/transactions.rs`). Refusals remain in the corpus. |
| RT-11 | System Application and Base Application run as real code | `docs/scope.md` section 1 | Done | Microsoft's apps compile from source (ADR-013), kept in the compile cache (ADR-022). |
| RT-12 | Microsoft's own test buckets run | `.github/workflows/ms-bucket.yml` | Partial | `al-runner test-microsoft` runs them by Microsoft's recipe (PRs 5646, 5648, 5653). Missing: about half of Microsoft's tests are still not run (`docs/alrunner-buckets.md`). |
| RT-13 | Refused by design: SMTP, outbound HTTP, files, OData/SOAP, printers, scheduler, rendering | `docs/scope.md` section 3 | Done | Same list (`what-works.md`, "Refused by design"). Outbound HTTP works through `[HttpClientHandler]`. |
| RT-14 | BC versions 27.0 to 28.5 (nine minors) | `.github/bc-versions.txt` | Partial | One binary for BC 27, 28 and 29 (ADR-015). Missing: each major is checked against one minor only (27.5, 28.5, 29.0). |
| RT-15 | Fixed culture en-US; Production environment; on-premises topology | `docs/limitations.md` | Done | en-US; `--environment saas` (the default, probed) or `onprem` (`--help`, OTHER). The default differs from v2: a SaaS sandbox, not Production. |

## 8. Distribution and CI use

| ID | Feature | v2 reference | Status | AL Runner today |
|---|---|---|---|---|
| DIST-1 | `dotnet tool install -g MSDyn365BC.AL.Runner` | `README.md` | N/A | A self-contained binary with installers (`README.md`). No .NET needed except for the `DotNet` helper. |
| DIST-2 | Windows, Linux (x64, arm64), macOS | `README.md` | Partial | Linux x64, Windows x64, macOS arm64. Missing: Linux arm64, macOS Intel, Windows arm64. |
| DIST-3 | Linux needs no C compiler (prebuilt Win32 shim) | `README.md` | N/A | No shim. |
| DIST-4 | One engine per BC minor inside the package | `AlRunner.csproj:255-290` | N/A | One binary for all BC versions (ADR-015). |
| DIST-5 | Authenticode-signed binaries | `.github/workflows/publish.yml` (`sign-and-pack`) | Open | No signing and no macOS notarization. Releases carry SHA256SUMS and build provenance attestations. |
| CI-1 | JUnit XML for CI systems | `CHANGELOG.md` 1.0.12 | Done | `--junit` (OUT-6). |
| CI-2 | Cobertura XML | `.github/workflows/coverage-demo.yml` | Done | `--coverage-out` (OUT-8). |
| CI-3 | Exit codes as a gate; `--no-strict-exit` | `README.md` | Partial | Exit codes work (OUT-9). Missing: `--no-strict-exit` (OUT-10). |
| CI-4 | Cache artifacts between CI runs | `README.md` `AL_RUNNER_ARTIFACTS_ROOT` | Done | `--cache-dir` or `ALRUNNER_CACHE`; `--require-compile-cache`; `--affected-store` (`--help`; `docs/manual/ci.md`). |
| CI-5 | AL-Go integration | none in v2 | N/A | v2 has none. |
| CI-6 | Run without Docker, SQL or a license | `README.md` | Done | Microsoft's apps and demo data come from the artifact feed (`docs/manual/getting-started.md`). |

## 9. Environment variables

| ID | Feature | v2 reference | Status | AL Runner today |
|---|---|---|---|---|
| ENV-1 | `AL_RUNNER_VERBOSE`, `AL_RUNNER_SHOW_PASS`, `AL_RUNNER_FAILURES_ONLY` | `CliText.cs` | Partial | The flags exist (OUT-5). Missing: the variables are not read (migrating-tools, section 6). |
| ENV-2 | `AL_RUNNER_TEST_TIMEOUT_SEC`, `AL_RUNNER_SEED` | `Program.cs` | Partial | The timeout variable is read, and `ALRUNNER_TEST_TIMEOUT` too. Missing: the seed (SEL-9). |
| ENV-3 | `AL_RUNNER_ARTIFACTS_ROOT`, `_CACHE_ROOT`, `_SYMBOLS_ROOT`, `_ENGINE_CACHE_ROOT` | `CacheRoots.cs` | Done | See DP-14. |
| ENV-4 | `AL_RUNNER_DEFAULT_TEST_TOOL`, `AL_RUNNER_NO_DEP_COMPANY_CACHE`, `AL_RUNNER_BCBAK`, `AL_RUNNER_WATCH_*` | `README.md` | Partial | `_WATCH_*` is not planned (ADR-049). `_BCBAK` is not needed (DP-22). Missing: `_DEFAULT_TEST_TOOL` and `_NO_DEP_COMPANY_CACHE` are not read. |
| ENV-5 | `AL_RUNNER_EMIT_TIMEOUT_SEC`, `AL_RUNNER_NCL_CACHE`, `AL_RUNNER_TRACE_*`, `AL_RUNNER_JOBS_*` and others | `CliText.cs` | N/A | They describe v2 internals. |

## 10. Dev-only features (stay in `al-test-cli`)

These serve v2's own corpus runs. A user of a project does not need them. AL
Runner does the same job with the oracle and the ratchet (ADR-007).

| ID | Feature | v2 reference | AL Runner equivalent |
|---|---|---|---|
| DEV-1 | `--expectations DIR`, `--expectations-require-match` | `ExpectationManifest.cs`, `docs/expectations.md` | `oracle/` and `metrics/<major>/passing.txt`. A user-facing "known gap" file is an open question. |
| DEV-2 | `--count-baseline`, `--count-baseline-require-all`, `--count-out` | `Infrastructure/CountBaseline.cs` | The ratchet |
| DEV-3 | `--out`, `--classify`: failure classification JSON | `Reporter.cs` | `al-test-cli --json`, `--markdown` |
| DEV-4 | Corpus checkout, MS bucket and mutation tooling | `tools/*.py`, `.github/workflows/ms-bucket*.yml` | `al-test-cli`, `check.sh` |

These four rows are not counted below.

## Headline numbers

118 user-facing features are counted (sections 1 to 9).

| Status | 2026-10-13 | 2026-10-22 |
|---|---:|---:|
| Done | 3 | 57 |
| Partial | 36 | 33 |
| Open (2026-10-13: "No") | 63 | 10 |
| Not planned | 0 | 2 |
| N/A | 16 | 16 |

The counts come from the tables above. Recount them when a row changes.

The 10 Open rows: IN-2, IN-3, IN-4 (how APP arguments are found), IN-10
(`package`), DP-10 (compiled-only dependencies), DP-23 and OUT-13 (notes under a
failure), SEL-9 (`--seed`), OUT-10 (`--no-strict-exit`), DIST-5 (signing).

For gate R1, each Open and Partial row needs to be built or deferred by the owner
in writing. Candidates for a written deferral: DP-10, SEL-9, IN-10, OUT-10, and
the missing platforms (DIST-2).

## What is hard to match, and why

1. **DP-10, compiled-only dependencies.** v2 loads Microsoft's CLR code. AL
   Runner has no CLR, so an app that ships without AL source cannot run. This
   touches ISV and AppSource apps that set `ShowMyCode = false`. Microsoft's own
   apps are not affected: their source is in the artifacts (ADR-013).
2. **RT-14, many BC versions.** v2 ships an engine per minor. In AL Runner each
   version difference is a behavior that needs oracle evidence. The corpus has
   oracles for 27.5, 28.5 and 29.0.
3. **RT-12, Microsoft's test buckets.** About half of Microsoft's tests are still
   refused. Each refusal is a gap in the runtime, closed through the corpus.
4. **SEL-9, seed.** `Randomize(seed)` must give the sequence .NET gives. That is
   a port of a known algorithm, but it is observable.

## Differences users will notice (not features)

- **Failure text.** v2 puts a .NET exception type in front of the BC message,
  for example `NavNCLDialogException: Assert.AreEqual failed...`. AL Runner
  prints the BC message only (ADR-009). Scripts that match the prefix must
  change.
- **Not run versus failed.** v2 often ran unsupported code on guessed behavior.
  AL Runner reports such a test as "not run", with the reason and exit code 7.
- **Isolation `test`.** v2 also reset the codeunit's variables. AL Runner keeps
  them, as BC's `TestIsolation = Function` does.
- **Time limit.** CPU time with a default of 20 s, not 60 s of wall time.
- **Speed.** v2 pays seconds of startup. AL Runner runs the whole corpus in
  seconds, and a project with Microsoft's apps loads them from the compile cache
  after the first run.
