---
title: "Test data"
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

## The DemoTool company

```bash
al-runner test Tests-SCM-Warehouse.Source.zip --company-data demotool
```

Microsoft's CI builds the Legacy lane's company with the DemoTool on every BC
version (BCApps `Invoke-LegacyDemoDataTool`): it installs only the base apps,
creates a new company (`Company-Initialize`), runs the action "Create Demo Data
from Config" of page 101900 "Demonstration Data Tool", and installs the other
apps after that. The container is on-prem. `--company-data demotool` does the
same without a page: it runs that action's codeunit 101899 in the install
baseline, after `Company-Initialize`, on an on-prem server. Only the apps of
BCApps' groups `Base`, `TestFramework` and `LocalBaseExtensions` (from
`build/groups.json`) and the DemoTool are installed before it; the install code
of the other apps runs after it, and their event subscribers do not run before
that. The tests themselves keep `--environment`.

The DemoTool is not in the artifact. Its AL source and its W1 resource files
(`DemoDataConfig.xml`, pictures, text files) come from Microsoft's BCApps
repository, branch `releases/<major>.<minor>`. They are fetched once with git into
`<cache dir>/bcapps-demotool/<major>.<minor>/`. `--demotool DIR` names a BCApps
checkout instead. The app is compiled with the Microsoft apps. The codeunit reads
the resource files through the `File` type, matched without case as on Windows.
A path that climbs out of `TemporaryPath()` (`TemporaryPath() + '..\X'`) names
the folder of the resource files, as BCApps copies them into the folder above the
server's temporary folder.

No backup is restored. If the codeunit does not run to its end, no test runs: each
test is not run, with the place where the codeunit stopped. The run also lists
every refused routine the codeunit can reach (`--verbose`). The company is kept
with the install baseline in the compile cache, so the next run of the same
program reads it. w1 only.

## Microsoft's test apps

To run Microsoft's own tests, with the right company for each test codeunit and
Microsoft's disabled tests skipped, see
[Running Microsoft's tests](microsoft-tests.md).
