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
| `--no-download` | Never fetch from the artifact feed (apps, `System.app`, demo data). |
| `--cache-dir DIR` | Where downloads and the compile cache live. Else `$ALRUNNER_CACHE`, else `$XDG_CACHE_HOME/al-runtime`, else `~/.cache/al-runtime` (on Windows `%USERPROFILE%\.cache\al-runtime`). |
| `--country CODE` | The Business Central country version (`us`, `gb`, `de`, ...). Default `w1`. See [Test data](test-data.md). |
| `--define SYMBOL` | A preprocessor symbol for `#if SYMBOL`. Repeatable. `--preprocessor-symbols A,B` names several. Like the AL compiler, AL Runner predefines no symbol (no `CLEANSCHEMA<n>`, no `BC<n>PLUS`). |

Lookup order: `--dep`, `--package-cache`, each APP's `.alpackages`, then Microsoft's
feed. A build named exactly (four numbers) that is in `~/.bcartifacts.cache/sandbox`
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
| `--ignore-permission-mock` | Run the tests that lower their own permissions (`Permissions Mock`, `Library - Lower Permissions`) with every access granted. Without it, such a test is not run, because AL Runner does not enforce permissions. **Risk:** permissions are not checked. A test that passes here can fail on Business Central with a permission error, and a test that expects a permission error fails here. Each such test is a flagged pass with the assumption `permissions-not-enforced`, so it is never counted as a clean pass. Use it to run the rest of the test's logic, not to check permissions. In `serve`: the `runTests` field `ignorePermissionMock`. See [Writing tests](writing-tests.md#permissions). |
| `--permissions on\|off` | Whether AL Runner checks permissions where one of its permission-check stages is on (ADR-062). Default `on`. No stage is on yet, so `on` and `off` give the same result today; `off` will keep a run as it is now once stages turn on. In `serve`: the `runTests` field `permissions` (`"on"` or `"off"`). |
| `--stability FILE\|microsoft` | Stability mode (ADR-058): run the selected tests once per variant of a JSON config over one compile and list the tests whose outcome changes against the base variant. The preset `microsoft` is Microsoft's four: another `Library - Random` seed, each test alone, reverse method order, work date plus one year. Dimensions built: `random.librarySeed`, `isolation.alone`, `order.methods`, `workDate.offset`, `clock.date`, `order.codeunits` (changes outcomes only with `--isolation disabled`), `random.hostSeed`, `guid.seed`, `clock.timeOfDay`, `clock.stepMs` (positive), `identity.userId` (letters and digits), `identity.userSecurityId`, `identity.sessionId`, `identity.serviceInstanceId`, `identity.tenantId`, `company.name` (letters, digits and spaces, up to 30); the integer ones also take a range; strategies `oneAtATime`, `matrix`, `sample`. The config's `matrix` names the axes `bcVersion`, `country`, `companies` (each value `null` for one company, `"all"`, or a list of names) and `environment` (`saas`, `onprem`): every cell runs every variant and is compared with its own base variant. Version, country and companies cells are child processes, as with `--matrix`; the environments of a cell share its compile. A version or country list on the command line forms the cells the same way when the config names neither axis. A config that names a "never" or "pin first" dimension, a dimension not built yet, or an unknown matrix axis is refused with exit 2, and so is an axis the config names together with its command-line option (`--bc-version`, `--country`, `--matrix`, `--environment`, `--companies`). Exit 1 when a test changes between pass and fail; a change to or from not run is listed apart. `--junit` holds one testsuite per variant, `--output-json` a `stability` object. In a matrix, each cell writes its own JUnit file and its document holds its `stability` object; the matrix report adds a table of the cells and the tests that changed within each. Not with `--affected`, `--recipe`, `--explain`, `serve`, or `ALRUNNER_REAL_CLOCK=1`. |
| `--seed N` | The run seed, 0 to 2147483647 (ADR-061). Every run has one: `--seed`, else `ALRUNNER_SEED` or `AL_RUNNER_SEED`, else a fresh one. It is printed at the start and in the summary (`Seed: N`, plus a `Replay:` command for the first failure), and written to `--output-json` (`seed`, `seedSource`), `--junit` (a `seed` property on every testsuite) and the `serve` summary. Per test method it derives the seed of `Random` (version 2's formula; `Randomize(n)` still carries over to the next test, as in Business Central) and the Guids and SystemIds. The same seed, apps and selection replay the same values. It does not change the clock, the work date, the test order or what install code draws. A value out of range is exit 2. |
| `--seed-library` | Also hand `Library - Random` a seed derived from the run seed, after the reset step's `SetSeed(1)`. Off by default, as Business Central's test runner keeps 1. A stability run always does it. |
| `--stability-replay ID\|VALUES` | Run one variant as a normal run: its id (`29.0-w1/3@4271833`, which also sets the run seed; with matrix axes in the config, also the cell, such as `28.5-w1-companies2-onprem/1@4271833`) with `--stability`, or its printed values (`"order.methods=reverse workDate.offset=+1Y"`) without. |
| `--stability-max N` | The most variants the `matrix` strategy forms (default 1000). |
| `--stability-allow-flips` | Exit 0 although a test changed outcome. |
| `--stability-print-preset microsoft` | Print the preset, as a starting point for a config file. |
| `--explain TEST` | Run one test and print its outcome and, for a failure or refusal, the routines and source lines. |

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
| `--test-timeout-secs N`, `--test-timeout N` | CPU-time limit per test, default 20; 60 with `--recipe microsoft`, `test-microsoft` and `--affected-from microsoft`. `0` is no limit. A hung test is stopped at 10 times N of wall time. `Sleep` does not wait, but the time it asks for counts toward N. |
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
| `--timings` | Compile phases and the slowest test codeunits. |
| `--list-assumptions` | Print the registry of unprobed assumptions with the counts of the last run. Use it alone. |

## Coverage

| Flag | Effect |
|---|---|
| `--coverage` | Count how often each statement of the APPs runs, per test. Writes Cobertura XML to `coverage.cobertura.xml`. With coverage off, a run pays one branch per statement. |
| `--coverage-out FILE` | Write the Cobertura XML to FILE. Implies `--coverage`. |
| `--coverage-json FILE` | Write JSON with `coverage` and `perTestCoverage`. Implies `--coverage`. |

See [Editor and tool integration](editor-tools.md) for the formats.

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
| `--server-url URL` | The base address that `GetUrl(ClientType::Web)` answers with. Default `http://localhost:8080/BC`, else `$ALRUNNER_SERVER_URL`. |
| `--environment saas\|onprem` | The server that `Environment Information` describes. Default `saas`. The on-prem values are an unprobed assumption. |
| `--companies NAMES` | The companies of the database, comma separated, the session company first, or `all` (every company of the restored test data, the session company first). The per-company install code of your apps runs once in each company, then the per-database part runs once. Microsoft's apps stay in the session company: a test that reads one of their per-company tables in another company is not run. Without the flag the run has one company. See [Writing tests](writing-tests.md#companies). |
| `--files-dir DIR` | Let the `File` data type read and write real files under DIR. |
| `--license FILE` | The BC license file (`.bclicense`) of the run: a developer, partner or customer license. Default: the license in the BC artifacts of the run's build. License Permission answers from it. The file is read for this run only, never copied or cached; a file that cannot be read stops the run. See [BC license](license.md). |
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
See [What works](what-works.md#dotnet).

## Test-microsoft

`al-runner test-microsoft [--app NAME ...] [--bc-version V] [--country CODE] [OPTIONS] [APP...]`
runs Microsoft's test apps of a build with `--recipe microsoft`. `--app NAME` (also
`--microsoft-app NAME`) names a test app. See [Running Microsoft's tests](microsoft-tests.md).

## Debugging

`al-runner debug [--stdio | --port [N]] [OPTIONS] APP...` starts a Debug Adapter
Protocol server. See [Editor and tool integration](editor-tools.md#debugging).

## Exit codes

| Code | Meaning |
|---|---|
| 0 | Every selected test passed. |
| 1 | A test failed. |
| 2 | Bad invocation, or the run could not start. |
| 3 | The apps do not compile: an error the AL compiler gives (its code, such as AL0118). No test ran. |
| 6 | The selection matched no test. |
| 7 | Some tests were not run (a refusal). `--allow-not-run` makes this 0. |
| 8 | Some tests were not run because the .NET helper could not be used (not found, or no .NET 8 runtime). `--allow-not-run` makes this 0. |

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
| `AL_CHECK_THREADS`, `AL_LOWER_THREADS`, `AL_TEST_THREADS` | Worker threads of the check, the lowering and the test codeunits. Default: one per core. |
| `NO_COLOR` | No color with `--color auto`. |
