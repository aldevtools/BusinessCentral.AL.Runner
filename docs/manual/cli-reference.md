---
title: "Command line reference"
weight: 50
---

`al-runner --help` is the complete list and is generated from the code. When the
two differ, trust `--help`. This page groups the same options by purpose.

```
al-runner test [OPTIONS] APP...
al-runner test-microsoft [OPTIONS] [APP...]
al-runner serve [--protocol 2] [OPTIONS]
al-runner debug [--stdio | --port [N]] [OPTIONS] APP...
al-runner impact-index build|check-static [OPTIONS] APP...
al-runner impact-index build --from microsoft [--microsoft-app NAME ...] [OPTIONS]
al-runner cache clear|prune|info [--cache-dir DIR]
al-runner warm [PATH.bak] [OPTIONS]
al-runner --list-assumptions [--cache-dir DIR]
al-runner --version | --help
```

APP is an app folder (with `app.json`), a source archive (`Tests-SMB.Source.zip`)
or an `.app` that carries its source.

## Dependencies

| Flag | Effect |
|---|---|
| `--dep PATH` | A package file (`.app`, `*.Source.zip`), an app folder or a folder of packages. Repeatable. |
| `--package-cache DIR` | A folder of `.app` files. Repeatable. |
| `--bc-version V` | A Business Central major, major.minor, or an exact build (four numbers). An exact build also fills Microsoft's `$(app_...)` variables in source archives outside a folder named for their build. |
| `--no-download` | Never fetch from the artifact feed (apps, `System.app`, demo data). A dependency app that is then not found (not in `--dep`, `.alpackages` or the cache) stops the run with exit 2; the message names its id, name and declared version. |
| `--allow-missing-dependencies` | With `--no-download`: skip a dependency that cannot be found with a warning on stderr, as before. The run may then differ from one that has the app. `--output-json` and the `serve` summary list it in `skippedDependencies`. |
| `--cache-dir DIR` | Where downloads and the compile cache live. Else `$ALRUNNER_CACHE`, else `$XDG_CACHE_HOME/al-runtime`, else `~/.cache/al-runtime` (on Windows `%USERPROFILE%\.cache\al-runtime`). |
| `--country CODE` | The Business Central country version (`us`, `gb`, `de`, ...). Default `w1`. See [Test data](test-data.md). |
| `--define SYMBOL` | A preprocessor symbol for `#if SYMBOL`. Repeatable. `--preprocessor-symbols A,B` names several. Like the AL compiler, AL Runner predefines no symbol (no `CLEANSCHEMA<n>`, no `BC<n>PLUS`). |

Lookup order: `--dep`, `--package-cache`, each APP's `.alpackages`, then Microsoft's
feed. A build named exactly (four numbers) that is in `~/.bcartifacts.cache/sandbox`
(on Windows, when that folder does not exist, `C:\bcartifacts.cache\sandbox`)
is read from there without the feed, and any matching build there is the fallback
when the feed has none or is offline.

## The compile cache

Microsoft's apps and the other dependency apps are compiled once and kept in
`<cache dir>/compile/`. The key covers the apps, versions, sources, target and the
`al-runner` build. The cache also holds the install baseline: what the install code
of the dependency apps and `Company-Initialize` leave in the database. At the end
of `al-runner test`, at most once a day and for at most 200 ms, entries of other
builds unused for 7 days, entries of any build unused for 30 days, and temporary
files older than a day are removed.

| Flag or command | Effect |
|---|---|
| `--no-compile-cache` | Compile the dependency apps on every run, only what the tests reach. Same as `ALRUNNER_NO_COMPILE_CACHE=1`. |
| `--require-compile-cache` | Fail when there is no usable entry. Use it in CI. |
| `--rebuild-compile-cache` | Build the entries this run needs again and write them over the old ones. |
| `al-runner cache info` | List the entries, their size, and the split by build. |
| `al-runner cache prune [--dry-run] [--all-other-builds]` | Run the cleanup now and print what went. `--all-other-builds` also removes entries of other builds younger than 7 days. |
| `al-runner cache clear` | Remove every entry. Use it when a cached result is in doubt. |

`ALRUNNER_NO_CACHE_PRUNE=1` turns the automatic cleanup off.

## Selecting tests

| Flag | Effect |
|---|---|
| `--codeunit ID` | A test codeunit by object id. Repeatable. |
| `--filter TEXT`, `--test TEXT` | Tests whose name contains TEXT, any case. With a dot, matched against `Codeunit<ID>.<Method>`; without, against the method name. Repeatable. |
| `--test-exact NAME` | The test whose method name or `Codeunit<ID>.<Method>` is exactly NAME. Repeatable. |
| `--exclude-test TEXT` | Leaves out the tests TEXT matches. Wins over every selecting option. Repeatable. |
| `--run-selected-only` | Run only the selected tests of each codeunit, as Business Central's test tool runs only the enabled test methods. The other tests do not run, so their globals, rows, time and refusals do not reach the selected ones. The codeunit's `OnRun` still runs first. In `serve`: the `runTests` field `runSelectedOnly`. |
| `--ignore-permission-mock` | Run the tests that lower their own permissions (`Permissions Mock`, `Library - Lower Permissions`) with every access granted. Without it, such a test is checked as described under `--permissions`, and not run where a denial is not pinned. **Risk:** permissions are not checked. A test that passes here can fail on Business Central with a permission error, and a test that expects a permission error fails here. Each such test is a flagged pass with the assumption `permissions-not-enforced`, so it is never counted as a clean pass. Use it to run the rest of the test's logic, not to check permissions. In `serve`: the `runTests` field `ignorePermissionMock`. See [Writing tests](writing-tests.md#permissions). |
| `--dev-endpoint-ui` | Answer a UI call that no handler of the test takes (`Confirm`, `StrMenu`, `Message`, `Hyperlink`, `Page.Run`, `Page.RunModal`, the request page of `Report.RunModal`) as the dev-endpoint test runner of BC 28 and 29 does (the runner VS Code uses), instead of failing with `Unhandled UI` as Microsoft's Test Runner app does. Only the call shapes measured on BC 28.5 and 29 (SaaS) are answered; the others still fail. On BC 27 it changes nothing (that runner also fails); on other versions it is refused. **Risk:** a test that passes here can fail under Microsoft's Test Runner app. Each answered call prints a `warning:` line with the test, the call and the answer, and the test is a flagged pass with the assumption `dev-endpoint-ui` (`--junit` property `dev_endpoint_ui`, `--output-json` field `uiAnswered`). In `serve`: the `runTests` field `devEndpointUi`. See [Writing tests](writing-tests.md#ui-calls-without-a-handler). |
| `--no-client` | Run the tests as a session without a client, as a web-service session is. Use it to compare a test's result (for example a mutation-testing run) with a run in a web-service session on BC. Every dialog answers as it does in a page background task worker, the session without a client that a test on BC can reach, and no handler of the test sees the call: `GuiAllowed()` answers false; `Message`, `Dialog.Open`, `Update` and `Close`, `Hyperlink` and `SessionSettings.RequestSessionUpdate(false)` return; `Notification.Send` returns true; `FilterPageBuilder.RunModal` and `UploadIntoStream` return false; `Confirm` (whatever its default), `StrMenu`, `Page.Run(Id)`, `Page.RunModal(Id)`, `RunModal` on a Page variable and a static `Report.Run(Id)` or `Report.RunModal(Id)` whose request page would open raise the `CallbackNotAllowed` error, with the text Business Central gives in that worker. A report run with the request page off is not touched. **Risk:** Business Central is measured only for the worker (corpus `pages/background-tasks/BjNoGuiDialogs.al` and `BjNoClientDialogs.al`, BC 27.5, 28.5 and 29.0); that a web-service session answers the same is assumed, so a test that passes through one of these answers is a flagged pass with the assumption `no-client-session`. Two more passes are flagged the same way: a test that opens a `TestPage` (it works as in a session with a client, which is not measured), and a test that reads a `WorkDate` no code set when Base Application's login code would have set it (that code runs only when `GuiAllowed()` is true, so it does not run here). A call that is not measured in the worker is not run, with the reason: `CurrentClientType()` (the option does not say which kind of session it is: OData, SOAP, API), `Page.Run` on a Page variable, a page run with a record, a page with triggers, extensions or a Caption, a page that an action or a lookup shows, a report run on a variable whose request page would open, a report with an `OnInitReport` trigger, a report extension or no `requestpage` section, a report run for which the test bound a `RequestPageHandler` or `ReportHandler`, the request window of an `Xmlport.Run`, `Report.RunRequestPage`, file download, view and the other upload forms, `Notification.Recall` and `RequestSessionUpdate(true)`. In `serve`: the `runTests` field `noClient`. |
| `--allow-write-in-try-functions` | Run as a Business Central service tier whose setting `DisableWriteInsideTryFunctions` is off, as on Business Central online. Without it, AL Runner follows the setting's default, as a BC container and on-premises: a database write inside a TryFunction whose result is used raises `Call to the function 'MODIFY' is not allowed inside the call to '...' when it is used as a TryFunction.` (code `TryFunctionWrite`). With this option such a write runs: the TryFunction returns true, or false when an error follows the write, and the write is kept in both cases, as on Business Central online. A write inside a `Codeunit.Run` that the TryFunction started is still reported as not run (not measured yet). In `serve`: the `runTests` field `allowWriteInTryFunctions`. |
| `--strict-order` | Run the test codeunits in Business Central's order (ADR-066 amendment 12). Business Central runs all test codeunits in one session, so the globals of `SingleInstance` codeunits and the `NumberSequence` numbers a test took carry from one test codeunit to the next. By default AL Runner gives every test codeunit fresh SingleInstance state and the NumberSequence values the install code left, runs the codeunits in parallel and runs nothing again; a failed test that read a global an earlier codeunit would have set gets an order hint (console, JUnit property `orderHint`, JSON `orderHint`), and test codeunits that take numbers after another one are listed (`number order:` lines, JUnit property `numberSequenceOrder`). With this option the state and the numbers carry: the codeunits that hand state on (`RequiredTestIsolation = Disabled`, an `OnRun` that writes data, tests that take numbers) run one after the other in one chain with the SingleInstance state kept (ADR-066 amendment 8), and an order-dependent codeunit outside the chain keeps its parallel result, flagged `single-instance-order`, and runs again in order with the codeunits it depends on when a test of it failed or was refused while it ran and that test, or one before it, read a value an earlier codeunit left otherwise (amendments 4, 5 and 7). Within one test codeunit the order is always Business Central's. `--stability` implies it. See "Order hints" in [results](results.md). In `serve`: the `runTests` field `strictOrder`. |
| `--max-replay N` | Skip the serial re-run of an order-dependent test codeunit (with `--strict-order`) when its chain holds more than `N` test codeunits. The codeunit keeps its parallel result, a line `re-run skipped (chain of M codeunits over the limit of N)` is printed for it, and its tests get the flag `single-instance-rerun-skipped` and the JUnit property `singleInstanceRerun`. A test that passes only in Business Central's full order then fails here, flagged. Default: no limit; every re-run is replayed in full and the result is exact. Use it when long chains cost too much time (on NP Retail, BC 29, see ADR-066 amendment 7). Without `--strict-order` it has no effect. Nothing is kept between runs. In `serve`: the `runTests` field `maxReplay`. |
| `--test-suites FILE` | Treat each test suite listed in FILE as a Business Central session of its own, as a pipeline does that runs one test-tool run per suite (ALOps). A test codeunit is then order dependent only on a codeunit listed before it in its own suite, in the file's order; within a suite `--strict-order` and the default rules apply unchanged, and everything still runs in parallel. A codeunit in no suite runs as a session of its own; one listed in several suites runs once and has one result. Without this option the whole run is one session. Format and how to make the file: [Test suites file](#test-suites-file). In `serve`: the `runTests` field `testSuites`. |
| `--max-memory SIZE` | The most memory the run may use, such as `6G` or `800M` (`K`, `M`, `G`, `T`, powers of 1024). This is a hard limit. Without it, the limit is the free memory of the machine (`MemAvailable`) or the memory limit of the container or cgroup (`memory.max`), whichever is smaller; with it, the smallest of the three. AL Runner starts with one test thread and adds another only when there is work for it and the memory one test codeunit took (learned from the first codeunit) still fits under nine tenths of the limit. A thread that finds no room after a codeunit stops and frees its memory. The limit holds for the whole run, the compile and the compile cache included. When the run cannot fit (while it compiles its apps or builds or reads the compile cache, while it runs the install baseline, after the first codeunit, or while it runs with one thread left), it stops before the memory passes the limit: exit code 2, a message with the memory the run needed and the limit, and the tests not reached are reported as not run with the reason `memory limit (--max-memory X MB)`, never as failures. Outcomes do not depend on the thread count. `--timings` prints the limit, where it comes from, the memory held after the compile and after the install baseline, the memory per codeunit, the most threads that ran at once and how many stopped for memory. In `serve` and `debug` the limit only sets the threads; the run does not stop. In `serve`: the `runTests` field `maxMemory` (or `serve --max-memory` for every request). |
| `--dependencies-published-global` | Show the column "Published As" of the table "NAV App Installed App" as Global (0), not Dev (2), for a dependency app of a publisher other than Microsoft. Without it such an app is Dev, as when it is published through the development endpoint. Use it when your pipeline publishes dependencies as compiled apps and your install code filters on Published As. Global is a setting, not a value measured on Business Central: a test that reads it is a flagged pass with the assumption `dependency-published-as-global`. Microsoft's apps are always Global, and the apps under test always Dev. In `serve`: the `runTests` field `dependenciesPublishedGlobal`. |
| `--permissions on\|off` | Whether AL Runner checks permissions where one of its permission-check stages is on (ADR-062). Default `on`. Stage 1 is on: in a session that a test lowered, table data writes are checked and a denied write raises Business Central's error. Other denials are not run. `off` checks nothing, and a test that lowers its permissions is not run. In `serve`: the `runTests` field `permissions` (`"on"` or `"off"`). |
| `--stability [FILE\|microsoft\|all]` | Stability mode (ADR-058): run the selected tests once per variant of a JSON config over one compile and list the tests whose outcome changes against the base variant. The value is a preset name or an existing file of any name (a folder never is; a missing `.json` file is an error, "config file not found"). Without a value, the preset `all`: every safe dimension, one at a time. The preset `microsoft` is Microsoft's four: another `Library - Random` seed, each test alone, reverse method order, work date plus one year. Dimensions built: `random.librarySeed`, `isolation.alone`, `isolation.singleInstance` (`carried`, `fresh`: each test codeunit that read SingleInstance state an earlier codeunit left is judged with fresh state; flips name the SingleInstance codeunit, its app, the global and its last writer; with it the base variant replays every order-dependent codeunit in Business Central's order), `order.methods`, `workDate.offset`, `clock.date`, `order.codeunits` (changes outcomes only with `--isolation disabled`), `random.hostSeed`, `guid.seed`, `clock.timeOfDay`, `clock.stepMs` (positive), `identity.userId` (letters and digits), `identity.userSecurityId`, `identity.sessionId`, `identity.serviceInstanceId`, `identity.tenantId`, `company.name` (letters, digits and spaces, up to 30); the integer ones also take a range; strategies `oneAtATime`, `matrix`, `sample`. The config's `matrix` names the axes `bcVersion`, `country`, `companies` (each value `null` for one company, `"all"`, or a list of names) and `environment` (`saas`, `onprem`): every cell runs every variant and is compared with its own base variant. Version, country and companies cells are child processes, as with `--matrix`; the environments of a cell share its compile. A version or country list on the command line forms the cells the same way when the config names neither axis. A config that names a "never" or "pin first" dimension, a dimension not built yet, or an unknown matrix axis is refused with exit 2, and so is an axis the config names together with its command-line option (`--bc-version`, `--country`, `--matrix`, `--environment`, `--companies`). Exit 1 when a test changes between pass and fail; a change to or from not run is listed apart. Every stability run lists the codeunits that read carried SingleInstance state with an unchanged outcome (console section "Order dependence (SingleInstance state)", at most 50 lines; `readsCarried` in the JSON; JUnit property `singleInstanceCarried`). `--junit` holds one testsuite per variant, `--output-json` a `stability` object. In a matrix, each cell writes its own JUnit file and its document holds its `stability` object; the matrix report adds a table of the cells and the tests that changed within each. Not with `--affected`, `--recipe`, `--explain`, `serve`, or `ALRUNNER_REAL_CLOCK=1`. |
| `--seed N` | The run seed, 0 to 2147483647 (ADR-061). Every run has one: `--seed`, else `ALRUNNER_SEED` or `AL_RUNNER_SEED`, else a fresh one. It is printed at the start and in the summary (`Seed: N`, plus a `Replay:` command for the first failure), and written to `--output-json` (`seed`, `seedSource`), `--junit` (a `seed` property on every testsuite) and the `serve` summary. Per test method it derives the seed of `Random` (version 2's formula; `Randomize(n)` still carries over to the next test, as in Business Central) and the Guids and SystemIds. The same seed, apps and selection replay the same values. It also draws how far past a record's write stamp the test clock lands (0, 1 or 2 ms). It does not change the work date, the test order or what install code draws. A value out of range is exit 2. |
| `--seed-library` | Also hand `Library - Random` a seed derived from the run seed, after the reset step's `SetSeed(1)`. Off by default, as Business Central's test runner keeps 1. A stability run always does it. |
| `--stability-replay ID\|VALUES` | Run one variant as a normal run: its id (`29.0-w1/3@4271833`, which also sets the run seed; with matrix axes in the config, also the cell, such as `28.5-w1-companies2-onprem/1@4271833`) with `--stability`, or its printed values (`"order.methods=reverse workDate.offset=+1Y"`) without. |
| `--stability-max N` | The most variants the `matrix` strategy forms (default 1000). |
| `--stability-allow-flips` | Exit 0 although a test changed outcome. |
| `--stability-print-preset microsoft\|all` | Print the preset, as a starting point for a config file. |
| `--explain TEST` | Run one test and print its outcome and, for a failure or refusal, the routines and source lines. TEST is selected like `--filter`: a method name or `Codeunit<ID>.<Method>`. A bare name that matches several tests lists them in the `Codeunit<ID>.<Method>` form. |

`--filter` together with `--test-exact` selects only the tests both match. A test
name is its AL name as written, without quotes (`Codeunit50100.Space Step`).

## Affected tests

| Flag | Effect |
|---|---|
| `--affected` | Run only the test codeunits a change since the last `--affected` run can affect. The others keep their stored outcome and are counted as skipped. See [Writing tests](writing-tests.md#running-only-what-a-change-affects). |
| `--affected-store FILE` | Read and write the stored result at FILE (for CI). Default `<cache dir>/affected/<key>.json.gz`. |
| `--include-failing` | With `--affected`: also run every codeunit with a test that did not pass last time. |
| `--strict-environment` | With `--affected`: accepted for version 2's tools. The strict behavior is always on. |

## Test sources and matrix runs

| Flag | Effect |
|---|---|
| `--affected-from SOURCE` | `microsoft`, or the path of an ISV test app. Also run the tests of that source that your extension can affect. Repeatable. See [Running Microsoft's tests](microsoft-tests.md). |
| `--bc-version A,B`, `--country X,Y` | A list runs every combination, one cell each. |
| `--matrix 28.5:w1,29.0:us` | Run only the named cells. A version alone means `w1`. Not with `--bc-version` or `--country`. |
| `--matrix-parallel N` | At most N cells at once. Default 1. |
| `--matrix-memory GB` | Keep the expected peaks of the cells running at once within GB. |

See [Writing tests](writing-tests.md#several-versions-and-countries).
## Isolation and limits

| Flag | Effect |
|---|---|
| `--isolation MODE`, `--test-isolation MODE` | `codeunit` (default), or `test` (also `function`, `method`). Under `test`, a `Commit()` inside a test is rolled back with the rest of the test. `disabled`: nothing is rolled back but a failed test's writes since its last `Commit()`, and the test codeunits run in id order and see what the earlier ones left (`StartSession` runs the session's codeunit to its end inside the call and commits what it wrote). Under both, `RequiredTestIsolation` has no effect. `required`: each test codeunit runs under its own `RequiredTestIsolation` (`Function`, `Disabled`, or `Codeunit`), as Business Central 28 and later runs a test codeunit without a test runner codeunit. On BC 27 every codeunit runs under `codeunit` isolation, with a note. What this mode does not cover is reported as not run, with the reason ([Writing tests](writing-tests.md#--isolation-required)). |
| `--test-driver DRIVER` | Whose way of starting the tests the run models (ADR-074). `bc-linux` (default): each test codeunit's `OnRun` runs once, before its tests, as the corpus Linux legs run them. `bccontainerhelper`: as BcContainerHelper's `Run-TestsInBcContainer` and AL-Go run tests. A discovery pass first runs the `OnRun` of every test codeunit, in id order, without test isolation, and what it writes stays for the run. Then each codeunit's `OnRun` runs again before its tests, and what it writes is rolled back with the codeunit. A codeunit whose `OnRun` fails in the discovery pass reports no tests. The whole run is one Business Central session, so the run takes Business Central's order, as `--strict-order` does. Refused (the tests are not run, with the reason): any runner other than 130450 "Test Runner - Isol. Codeunit" (`--test-runner ID`, `--isolation test` or `disabled`), several apps with test codeunits, `--test-suites`, a subscriber to the events of codeunit 130452 outside the Test Runner app, an earlier codeunit that can read what a later codeunit's `OnRun` wrote in the discovery pass, and any test that can read a number sequence or isolated storage the discovery pass changed. Not with `--isolation required` (an error). In `serve`: the `runTests` field `testDriver`. |
| `--test-timeout-secs N`, `--test-timeout N` | CPU-time limit per test, default 20; 60 with `--recipe microsoft`, `test-microsoft` and `--affected-from microsoft`. `0` is no limit. A hung test is stopped at 10 times N of wall time. `Sleep` does not wait, but the time it asks for counts toward N. |
| `--skip-after-timeout` | Once a test is stopped at the time limit, the rest of its codeunit is not run. Each later test ends as not run, with a reason that names the stopped test. Default: the later tests run, because a later pass is still a result. Works with `--affected` and as a `serve` start option. The summary table shows the time spent on tests that ended not run (from one second on), and the `--output-json` document has it as `notRunSeconds`. |
| `--allow-not-run` | Exit 0 instead of 7 or 8. |

## Output

| Flag | Effect |
|---|---|
| `--verbose`, `-v` | Setup details first, then one line per test (PASS, FAIL, NOT RUN with reason) with its run time, the test data rows line, and the full `Time:` line. |
| `--quiet`, `-q` | Summary only. |
| `--color auto\|always\|never` | PASS green, FAIL red, NOT RUN yellow, flagged passes cyan. `auto` colors only on a terminal and never when `NO_COLOR` is set. JUnit and JSON are never colored. |
| `--top-refusals [N]` | The N most common reasons tests were not run (default 10). |
| `--top-failures [N]` | The N most common failure messages (default 10), numbers and quoted values folded. |
| `--top-assumptions [N]` | The N unprobed assumptions most passes ran through (default 10). |
| `--refusal-detail TEXT` | Not-run reasons that contain TEXT, most common first. |
| `--junit FILE` | JUnit XML. |
| `--output-json [FILE]`, `--output-json=FILE` | JSON in the shape of AL Runner version 2. See [Reading results](results.md). |
| `--timings` | Compile phases and the slowest test codeunits. For the time per AL routine, use `--profile` (see [Profiling AL code](profiling.md)). |
| `--list-assumptions` | Print the registry of unprobed assumptions with the counts of the last run. Use it alone. |

## Coverage

| Flag | Effect |
|---|---|
| `--coverage` | Count how often each statement of the APPs runs, per test. Writes Cobertura XML to `coverage.cobertura.xml`. With coverage off, a run pays one branch per statement. |
| `--coverage-out FILE` | Write the Cobertura XML to FILE. Implies `--coverage`. |
| `--coverage-json FILE` | Write JSON with `coverage` and `perTestCoverage`. Implies `--coverage`. |

See [Editor and tool integration](editor-tools.md) for the formats.

## Profiling

| Flag | Effect |
|---|---|
| `--profile PATH` | Write AL CPU profiles (`.alcpuprofile`) that the AL extension in VS Code opens. A folder gets one file per test codeunit, named `<id>-<name>.alcpuprofile`, plus `summary.txt`. A PATH that ends in `.alcpuprofile` gets one merged file for the whole run. With the profiler off, a run pays one branch per statement and call. |
| `--profile-mode MODE` | `instrumentation` (default): exact times, call counts and statement times. `sampling`: a sample of every thread at each interval; the file can also be loaded on BC's Performance Profiler page. Needs `--profile`. |
| `--profile-interval US` | Microseconds between samples in sampling mode. Default 1000, at least 50. Needs `--profile`. |

`AL_PROFILE=FILE` still works for this release and is deprecated. See [Profiling AL code](profiling.md#the-old-al_profile-variable).

## Test data

| Flag | Effect |
|---|---|
| `--no-test-data` | Run without company data. |
| `--test-data [PATH]` | Name a backup (`.bak`, `.bacpac`), or insist on test data: a missing backup then stops the run. Write `--test-data=PATH`, or end the name in `.bak` or `.bacpac`, because a bare name after `--test-data` is an APP. |
| `--test-data-company NAME` | The company to restore (exact name or prefix). Default: the only company, else `CRONUS International Ltd.`. |
| `--company-data bak\|package` | `bak` (default) restores the CRONUS company of the backup. `package` builds the company from the RapidStart package, the way Microsoft's Legacy test lane does. The other values (`demotool`, `contoso-setup`, `contoso-all`) build Microsoft's demo company; they are experimental and need `--experimental-company-builds`. |
| `--package PATH` | The `.rapidstart` file to use. Implies `--company-data package`. |
| `--disabled-tests DIR` | Microsoft's list of disabled tests (BCApps `src/DisabledTests`, or one app's folder). Those tests still run and are marked `disabled_by_microsoft`. |
| `--parity` | With `--disabled-tests`: do not run those tests. |
| `--recipe microsoft` | Run the way Microsoft's CI does. See [Running Microsoft's tests](microsoft-tests.md). |
| `--bcapps DIR` | With `--recipe`: use this BCApps checkout instead of the cache. |
| `al-runner warm [PATH.bak]` | Open the demo backup and keep its page map and catalog in the cache. |

## Environment and host

| Flag | Effect |
|---|---|
| `--server-url URL` | The base address that `GetUrl(ClientType::Web)` answers with. Default `http://localhost:8080/BC`, else `$ALRUNNER_SERVER_URL`. The other client types build on it: SOAP with port 7047, OData, ODataV4 and Api with port 7048, Default with `dynamicsnav://`. They need an `http` address; with `https` only Web, Current, Background, Management and NAS answer. |
| `--environment saas\|onprem` | The server that `Environment Information` describes. Default `saas`. The on-prem values are an unprobed assumption. Comparing against a BC Docker container? Use `--environment onprem`. |
| `--companies NAMES` | The companies of the database, comma separated, the session company first, or `all` (every company of the restored test data, the session company first). The per-company install code of your apps runs once in each company, then the per-database part runs once. Microsoft's apps stay in the session company: a test that reads one of their per-company tables in another company is not run. Without the flag the run has one company. See [Writing tests](writing-tests.md#companies). |
| `--files-dir DIR` | Let the `File` data type read and write real files under DIR. |
| `--license FILE` | The BC license file (`.bclicense`) of the run: a developer, partner or customer license. Default: the license in the BC artifacts of the run's build. License Permission answers from it, and it limits table data access: a table the license grants only indirectly needs the `Permissions` property of the code's object. The file is read for this run only, never copied or cached; a file that cannot be read stops the run. See [BC license](license.md). |
| `--auto-provision` | Accepted and ignored. AL Runner always downloads unless `--no-download`. |

## Test first

| Flag | Effect |
|---|---|
| `--tdd` | Stub calls to procedures that do not exist yet. See [Writing tests](writing-tests.md). |

## Impact analysis

`--impact-of APP`, `--impact MODE`, `--impact-json FILE`, `--impact-accept FILE`,
and `al-runner impact-index`. See [Impact analysis](impact.md).

## DotNet helper

`--dotnet-sidecar PATH`, `--dotnet-assembly-dir DIR`, `--dotnet-allow-roll-forward`,
`--dotnet-timeout-secs N` and `--require-dotnet` control the optional .NET helper.
`--dotnet-assembly-dir` folders are searched before the BC service-tier DLLs that AL Runner
finds or downloads for the build in use.
See [What works](what-works.md#dotnet).

## Test-microsoft

`al-runner test-microsoft [--app NAME ...] [--bc-version V] [--country CODE] [OPTIONS] [APP...]`
runs Microsoft's test apps of a build with `--recipe microsoft`. `--list-lanes` prints, without compiling or running, the lane of Microsoft's CI that each test codeunit runs in, with totals per lane; `--verbose` adds a line per codeunit. `--app NAME` (also
`--microsoft-app NAME`) names a test app. See [Running Microsoft's tests](microsoft-tests.md).

## Test suites file

`--test-suites FILE` reads a plain text file (UTF-8) that lists Business Central test
suites, one per line (ADR-066 amendment 6):

```
# one line per suite: an optional name and a colon, then the test codeunit ids in run order
DATACHECK: 81659 81660 81665 81667 81673
LCSP: 81697 81699 81700 81704
suite  3: 81045, 81046, 81047
82002 82004 82005
```

- **Ids**: the suite's test codeunit ids, in the order Business Central runs them.
  Separate them with spaces, tabs or commas.
- **Name** (optional): the text before the first `:`. It may hold spaces. It only
  appears in messages. A line without a name, or with an empty one, is named `line N`.
- **Skipped**: blank lines, and lines whose first non-blank character is `#`. A UTF-8
  byte order mark and Windows line ends are fine.
- **Errors** (exit code 2, with the line number): a token that is not a whole number
  from 1 to 4294967295, and a codeunit listed twice in one suite.
- **Warnings** (on stderr; the run goes on): an id that is no test codeunit of the
  apps under test. It is ignored, so one file can serve several apps.
- A suite without ids (`ENV:`) is accepted and holds nothing.
- A codeunit may be listed in more than one suite. It runs once and has one result.

What the suites change is described in [results](results.md), "One session per test
suite".

### Making the file from your test suites

The order inside a suite matters, so take it from where your pipeline gets it:

1. **From the pipeline's suite definitions.** If the pipeline loops over a list of
   suites (an ALOps template with one `ALOpsAppTest` task per `testsuite`), write
   one line per suite from that same list, in the same order as the tasks run.
2. **From the test tool in Business Central.** The suites are rows of table 130451
   "AL Test Suite". Their codeunits are rows of table 130450 "Test Method Line" with
   "Line Type" = Codeunit, ordered by "Test Suite", "Line No." (the table's key and the
   order the test tool runs them in). Write one line per "Test Suite" value: the suite
   name, a colon, then the "Test Codeunit" values in "Line No." order.
3. **From the results of a run.** If each suite's run writes its own result file
   (for example one JUnit or XUnit file per pipeline step), the codeunits in each file,
   in the order they appear, give that suite's line.

Codeunits you leave out run as sessions of their own, so a partial file is safe: it
only removes links, never adds them.

## Debugging

`al-runner debug [--stdio | --port [N]] [OPTIONS] APP...` starts a Debug Adapter
Protocol server. See [Editor and tool integration](editor-tools.md#debugging).

## Exit codes

| Code | Meaning |
|---|---|
| 0 | Every selected test passed. |
| 1 | A test failed. |
| 2 | Bad invocation, or the run did not fit under its memory limit (see `--max-memory`): the tests not reached are not run. |
| 3 | The apps do not compile: an error the AL compiler gives (its code, such as AL0118). No test ran. |
| 6 | The selection matched no test. |
| 7 | Some tests were not run (a refusal). `--allow-not-run` makes this 0. |
| 8 | Some tests were not run because the .NET helper could not be used (not found, or no .NET runtime of the major the BC version needs: 8 for BC 27-28, 10 for BC 29). `--allow-not-run` makes this 0. |

Order when several apply: 2, 6, 1, 8, 7. Exit code 3 means no test ran; the errors are on stderr and in `compilationErrors` of `--output-json`. The check covers the AL compiler errors that the corpus has checked against the real compiler, not all of them. Source of an app under test that the parser does not accept also exits 3; that line has the parser's message and no AL code. A dependency app's file the parser does not accept is not judged: the tests that need it are not run (exit 7). An impact run has its own codes; see
[Impact analysis](impact.md).

## Environment variables

| Variable | Effect |
|---|---|
| `ALRUNNER_CACHE` | The cache folder when `--cache-dir` is not given. |
| `ALRUNNER_SERVER_URL` | The address `--server-url` sets, when it is not given. |
| `ALRUNNER_SEED`, `AL_RUNNER_SEED` | The run seed when `--seed` is not given. The second is version 2's name. |
| `ALRUNNER_NO_COMPILE_CACHE=1` | Same as `--no-compile-cache`. |
| `ALRUNNER_NO_CACHE_PRUNE=1` | Turn off the automatic cache cleanup. |
| `ALRUNNER_BCAPPS_URL` | The address `--recipe` fetches BCApps from. |
| `ALRUNNER_REPORT_UNPINNED_ERRORS=1` | Opt-in: a test that ends on an error whose text is not pinned is failed, its message starting with `[error text not pinned]`, instead of not run. The summary line, `--junit` and `--output-json` say the switch is on. Not a conformance result. See [Reading results](results.md#errors-whose-text-is-not-pinned). |
| `ALRUNNER_NO_PATH_NOTE=1` | No note about another `al-runner` on the `PATH`, and no list of them under `--version`. |
| `ALRUNNER_TEST_TIMEOUT`, `AL_RUNNER_TEST_TIMEOUT_SEC` | The test time limit in seconds when `--test-timeout-secs` is not given (also for Microsoft's recipe). The second is version 2's name. |
| `AL_CHECK_THREADS`, `AL_LOWER_THREADS`, `AL_TEST_THREADS` | Worker threads of the check, the lowering and the test codeunits. Default: one per core. The memory limit (`--max-memory`) can lower the test threads further, never raise them; a run whose work cannot use more threads uses fewer. |
| `NO_COLOR` | No color with `--color auto`. |
