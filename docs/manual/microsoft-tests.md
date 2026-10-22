---
title: "Running Microsoft's tests"
weight: 7
---

Microsoft's own test apps are the best check of how closely AL Runner matches
Business Central. They are also the most useful check for your extension: they show
which of Microsoft's tests your code changes. Three commands cover this:

| You want to | Use |
|---|---|
| Run all of Microsoft's test apps of a build | `al-runner test-microsoft` |
| Run the Microsoft tests your extension can affect | `al-runner test-microsoft MYEXT` |
| Run those next to your own tests, in one report | `al-runner test MYEXT MYTESTS --affected-from microsoft` |

All three use Microsoft's recipe, which decides the company and the disabled tests
for each test codeunit. Nothing needs to be downloaded or collected by hand.

## Run Microsoft's test apps: `test-microsoft`

```bash
al-runner test-microsoft                  # every test app of the newest build
al-runner test-microsoft --app Tests-ERM  # only the named test app
al-runner test-microsoft --bc-version 28.5 --country us
```

`test-microsoft` reads the list of test apps from the build's artifact (the
`Test/*.Source.zip` entries, and the country's own), fetches each app when it runs,
and runs it with `--recipe microsoft`.

- **Build and country.** The default is the newest build of the newest Business
  Central major, taken from the cache first, as for `test`. `--bc-version` and
  `--country` choose another. The default country is `w1`.
- **Which apps.** `--app NAME` (also `--microsoft-app NAME`) runs only the named test
  app, by its name in the artifact (`Tests-ERM`, `Data Search Tests`), in any case.
  It is repeatable. An unknown name exits with 2 and lists the names.
- **Test libraries.** Apps without a test codeunit are loaded as dependencies and not
  run. Apps whose artifact name starts with `_Exclude_` run only when `--app` names them.
- **Report.** Each test app is a run of its own. The output ends with one row per
  test app (passed, failed, not run, total, the codeunits of a lane that cannot be
  built yet, and the exit code) and a total.
- **Files.** `--junit FILE` writes one file per test app and lane, for example
  `FILE.Tests-ERM.legacy.xml`. `--output-json FILE` writes one document per test app
  and lane, and FILE is required here.
- **Exit code.** The worst test app's: 2 if an app could not be loaded or compiled,
  else 1, 8 or 7. It is 6 only when nothing ran.

The recipe decides the company and the disabled tests, so `--recipe`,
`--disabled-tests`, `--parity`, `--company-data`, `--package`, `--test-data*`,
`--no-test-data` and `--explain` are refused with `test-microsoft`.

## The recipe

`--recipe microsoft` runs the way Microsoft's CI does. You can use it on its own with
`al-runner test`. BCApps' own settings decide the company of each test codeunit,
which they call a lane:

| Test codeunit | Company |
|---|---|
| Legacy test apps | The RapidStart package company |
| `TestType = UnitTest` | An empty company |
| `IntegrationTest` | The Contoso setup company (not run yet) |
| The rest | The Contoso company with all demo data (not run yet) |

Each lane has its own summary and, with `--junit FILE`, its own file
(`FILE.<lane>.xml`). Microsoft's disabled tests are skipped and counted. The BCApps
files (MIT) of the release are fetched sparsely with `git` into
`<cache dir>/bcapps/<major>.<minor>/` once. `--bcapps DIR` names a checkout, and
`--no-download` uses the cache only. The two Contoso lanes are not run yet, because
AL Runner does not build the Contoso company, and the output says so. The recipe
cannot be combined with the company and disabled-test options.

**Time limit.** The recipe's limit is 60 seconds of CPU time per test, and other runs
keep 20. `--test-timeout-secs N` sets another, and `0` removes it. `Sleep` does not
wait, but the time it asks for counts toward the limit. For example, Tests-Dimension
`TestGetShortcutDimCached` sleeps 60.5 seconds and passes on Business Central, so it
is a timeout row at the default. Run it with `--test-timeout 120`.

## Only the tests your extension can affect

Running every Microsoft test on every change is slow, and most of them never touch
your code. Give `test-microsoft` your extension, and it runs Microsoft's tests as an
impact run (see [Impact analysis](impact.md)): each test app, lane by lane, with and
without your extension. Only the tests that reach your extension run.

```bash
al-runner test-microsoft ./MyExt
```

Tests that do not reach the extension are counted as **not selected**, never as
passed. Pass an app without test codeunits here. An app with test codeunits is refused.

To run your own tests and Microsoft's together, in one report with one summary table
and one `--junit` file:

```bash
al-runner test ./MyExt ./MyExt.Test --affected-from microsoft
```

The apps without a test codeunit are the extension, and the apps with one are your own
tests. Your own tests run first, as a normal run. In the JUnit file, each source suite
has the property `test_source`, and the first one has `not_selected`.

**Without an index.** Without a cached impact index, tests are selected by static
reach, which selects more tests. The run warns and prints the command that builds the
index:

```bash
al-runner impact-index build --from microsoft --microsoft-app Tests-ERM --bc-version 29.0
```

`impact-index build --from microsoft` builds the index of every Microsoft test app of
the build, one per lane, or only those named with `--microsoft-app`. Give it the
`--bc-version`, `--country`, `--dep` and `--define` of the runs that read it.

## Other vendors' tests

An ISV's test app is a test source too:

```bash
al-runner test ./MyExt --affected-from ../vendor/Product.Tests
```

The source is an app folder, an `.app` with AL source, or a `*.Source.zip`. A folder
of several test apps is one source per app. The option is repeatable, also next to
`--affected-from microsoft`, and the sources run in the order given. The source runs
with the run's normal test data, and its dependencies resolve as for any APP.

These cases are refused with exit 2, before any test runs:

- a test app or dependency without AL source
- a test app that depends on your extension (pass that one as an APP)
- the same source given twice

`--filter`, `--codeunit`, `--test-exact` and `--exclude-test` apply to your own
tests only. `--recipe`, `--impact-of`, `--no-compile-cache`, `--explain`, `--coverage*`,
`--tdd` and `--output-json` without a FILE are refused with it, and so is `serve`.

**Reports.** The report ends with a table: own tests, then per source the tests run,
how many ran your code, newly failing, fails differently, newly refused, not verified,
and the exit code. `--output-json FILE` writes your own tests as a plain run does, plus
a `sources` array. `--impact-json FILE` writes format 2, with one entry per impact run,
each with its `source`. `--impact-accept FILE` reads format 1 and 2, and an entry with a
`source` matches only in that source. The exit code is the worst part's: 2, then 1, 8,
7. A source that selects nothing gives 0.

## Several versions and countries

A list in `--bc-version` or `--country` runs every combination, one cell each. This
works with `test`, `test-microsoft` and `--affected-from`.

```bash
al-runner test ./MyExt ./MyExt.Test --bc-version 27.5,28.5,29.0 --country w1,us
al-runner test ./MyExt ./MyExt.Test --matrix "27.5:w1,29.0:us"
```

See [Writing tests](writing-tests.md#several-versions-and-countries) for the details.

## Parity with Microsoft's disabled tests

To check a single test archive against Microsoft's list of disabled tests without the
full recipe:

```bash
al-runner test Tests-ERM.Source.zip --disabled-tests path/to/BCApps/src/DisabledTests
```

Those tests still run and are marked `disabled_by_microsoft` in the summary and the
JUnit file. Add `--parity` to skip them. For the measurements of Microsoft's Base
Application test buckets, see [docs/alrunner-buckets.md](../alrunner-buckets.md).
