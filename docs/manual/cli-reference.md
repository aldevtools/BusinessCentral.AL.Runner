---
title: "Command line reference"
weight: 5
---

`al-runner --help` is the complete list and is generated from the code. When the
two differ, trust `--help`. This page groups the same options by purpose.

```
al-runner test [OPTIONS] APP...
al-runner serve [--protocol 2] [OPTIONS]
al-runner impact-index build|check-static [OPTIONS] APP...
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
| `--cache-dir DIR` | Where downloads and the compile cache live. Else `$ALRUNNER_CACHE`, else `$XDG_CACHE_HOME/al-runtime`, else `~/.cache/al-runtime`. |
| `--country CODE` | The Business Central country version (`us`, `gb`, `de`, ...). Default `w1`. See [Test data](test-data.md). |
| `--define SYMBOL` | A preprocessor symbol for `#if SYMBOL`. Repeatable. `--preprocessor-symbols A,B` names several. |

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
| `--explain TEST` | Run one test and print its outcome and, for a failure or refusal, the routines and source lines. |

## Isolation and limits

| Flag | Effect |
|---|---|
| `--isolation MODE`, `--test-isolation MODE` | `codeunit` (default), or `test` (also `function`, `method`). `disabled` is not supported: every test is reported as not run. |
| `--test-timeout-secs N`, `--test-timeout N` | CPU-time limit per test, default 20; 60 with `--recipe microsoft`, `test-microsoft` and `--affected-from microsoft`. `0` is no limit. A hung test is stopped at 10 times N of wall time. |
| `--allow-not-run` | Exit 0 instead of 7 or 8. |

## Output

| Flag | Effect |
|---|---|
| `--verbose` | Setup details first, then one line per test (PASS, FAIL, NOT RUN with reason) with its run time, the test data rows line, and the full `Time:` line. |
| `--quiet` | Summary only. |
| `--color auto\|always\|never` | PASS green, FAIL red, NOT RUN yellow, flagged passes cyan. `auto` colors only on a terminal and never when `NO_COLOR` is set. JUnit and JSON are never colored. |
| `--top-refusals [N]` | The N most common reasons tests were not run (default 10). |
| `--top-failures [N]` | The N most common failure messages (default 10), numbers and quoted values folded. |
| `--top-assumptions [N]` | The N unprobed assumptions most passes ran through (default 10). |
| `--refusal-detail TEXT` | Not-run reasons that contain TEXT, most common first. |
| `--junit FILE` | JUnit XML. |
| `--output-json [FILE]`, `--output-json=FILE` | JSON in the shape of AL Runner version 2. See [Reading results](results.md). |
| `--timings` | Compile phases and the slowest test codeunits. |
| `--list-assumptions` | Print the registry of unprobed assumptions with the counts of the last run. Use it alone. |

The short forms `-v` and `-q` appear in `--help` but are not accepted in
`3.0.0-alpha.3`: they are read as app paths. Use `--verbose` and `--quiet`.

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
| `--company-data bak\|package` | `bak` (default) restores the CRONUS company of the backup. `package` builds the company from the RapidStart package, the way Microsoft's Legacy test lane does. |
| `--package PATH` | The `.rapidstart` file to use. Implies `--company-data package`. |
| `--disabled-tests DIR` | Microsoft's list of disabled tests (BCApps `src/DisabledTests`, or one app's folder). Those tests still run and are marked `disabled_by_microsoft`. |
| `--parity` | With `--disabled-tests`: do not run those tests. |
| `--recipe microsoft` | Run the way Microsoft's CI does. See [Test data](test-data.md). |
| `--bcapps DIR` | With `--recipe`: use this BCApps checkout instead of the cache. |
| `al-runner warm [PATH.bak]` | Open the demo backup and keep its page map and catalog in the cache. |

## Environment and host

| Flag | Effect |
|---|---|
| `--server-url URL` | The base address that `GetUrl(ClientType::Web)` answers with. Default `http://localhost:8080/BC`, else `$ALRUNNER_SERVER_URL`. |
| `--environment saas\|onprem` | The server that `Environment Information` describes. Default `saas`. The on-prem values are an unprobed assumption. |
| `--files-dir DIR` | Let the `File` data type read and write real files under DIR. |
| `--auto-provision` | Accepted and ignored. Version 3 always downloads unless `--no-download`. |

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

## Exit codes

| Code | Meaning |
|---|---|
| 0 | Every selected test passed. |
| 1 | A test failed. |
| 2 | Bad invocation, or the run could not start. |
| 6 | The selection matched no test. |
| 7 | Some tests were not run (a refusal). `--allow-not-run` makes this 0. |
| 8 | Some tests were not run because the .NET helper could not be used (not found, or no .NET 8 runtime). `--allow-not-run` makes this 0. |

Order when several apply: 2, 6, 1, 8, 7. An impact run has its own codes; see
[Impact analysis](impact.md).

## Environment variables

| Variable | Effect |
|---|---|
| `ALRUNNER_CACHE` | The cache folder when `--cache-dir` is not given. |
| `ALRUNNER_SERVER_URL` | The address `--server-url` sets, when it is not given. |
| `ALRUNNER_NO_COMPILE_CACHE=1` | Same as `--no-compile-cache`. |
| `ALRUNNER_NO_CACHE_PRUNE=1` | Turn off the automatic cache cleanup. |
| `ALRUNNER_BCAPPS_URL` | The address `--recipe` fetches BCApps from. |
| `ALRUNNER_NO_PATH_NOTE=1` | No note about another `al-runner` on the `PATH`, and no list of them under `--version`. |
| `ALRUNNER_TEST_TIMEOUT`, `AL_RUNNER_TEST_TIMEOUT_SEC` | The test time limit in seconds when `--test-timeout-secs` is not given (also for Microsoft's recipe). The second is version 2's name. |
| `AL_CHECK_THREADS`, `AL_LOWER_THREADS`, `AL_TEST_THREADS` | Worker threads of the check, the lowering and the test codeunits. Default: one per core. |
| `NO_COLOR` | No color with `--color auto`. |
