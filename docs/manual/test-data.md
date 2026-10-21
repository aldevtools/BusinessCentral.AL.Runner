---
title: "Test data and Microsoft's tests"
weight: 6
---

## The demo company

Microsoft's tests expect the CRONUS demo company, and many of them fail on missing
setup data without it. A run that uses Microsoft's apps therefore restores the
CRONUS company of the same build by default. A table is read the first time a
test touches it. The rest is never read.

AL Runner looks for the backup in this order:

1. `~/.bcartifacts.cache/sandbox/<build>/w1`
2. The artifact cache.
3. Microsoft's artifact feed, once (about 520 MB, progress on stderr).
4. A backup of another build of the same Business Central version. The output says which.

If none can be had, test data is turned off and the run says why. The summary
lists what was loaded and every exclusion: tables the runtime could not rebuild,
tables the reader failed on (with its text), and columns left out.

| Flag | Effect |
|---|---|
| `--no-test-data` | Run with an empty database. |
| `--test-data [PATH]` | Use a specific `.bak` or `.bacpac`, or insist on test data. A missing backup then stops the run. |
| `--test-data-company NAME` | Which company to restore. Exact name or prefix. |
| `al-runner warm [PATH.bak]` | Open the backup once and keep its page map and catalog in the cache. The first open verifies the whole file (2 to 3 seconds on a 900 MB backup); later opens take milliseconds. Run it before a demo on a new machine. |

The reader behind this is `al-bcdb`, a Rust reader for `.bak` and `.bacpac` files
that needs no SQL Server. See [docs/bcdb.md](../bcdb.md).

## Other countries

`--country CODE` selects the country version (`us`, `gb`, `de`, and the other names
of the artifact feed). The default is `w1`. Only this flag selects it. `app.json`
and AL-Go settings are not read. The country's own Microsoft apps replace the w1
ones, and the demo data is that country's backup (about 600 MB the first time).
It cannot be combined with `--company-data package`.

## The RapidStart package company

```bash
al-runner test Tests-ERM.Source.zip --company-data package
```

`--company-data package` builds the company the way Microsoft's Legacy test lane
does: a fresh company, `Company-Initialize`, then the RapidStart package of the
artifact. `--package PATH` names a different `.rapidstart` file. A company built
from the package needs `--server-url` for `Company-Initialize`.

## Running Microsoft's test apps

```bash
al-runner test Tests-ERM.Source.zip
```

Microsoft's test archives run directly. The apps, `System.app` and the CRONUS data
are fetched, and the build is the build of the tests when it is still available.

For a parity report against Microsoft's own list of disabled tests:

```bash
al-runner test Tests-ERM.Source.zip --disabled-tests path/to/BCApps/src/DisabledTests
```

Those tests still run and are marked `disabled_by_microsoft` in the summary and the
JUnit file. Add `--parity` to skip them.

`--recipe microsoft` does all of this the way Microsoft's CI does. BCApps' own
settings decide the company of each test codeunit, which they call a lane:

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

For the measurements of Microsoft's Base Application test buckets, see
[docs/alrunner-buckets.md](../alrunner-buckets.md).
