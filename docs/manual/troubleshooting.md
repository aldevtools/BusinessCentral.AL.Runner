---
title: "Troubleshooting"
weight: 13
---

## Many tests are "not run"

This is expected in an alpha. Each one has a reason:

```bash
al-runner test ./MyApp ./MyApp.Test --top-refusals
al-runner test ./MyApp ./MyApp.Test --explain TestName
```

See [Reading results](results.md) for what the three kinds of "not run" mean. The
run exits with 7. Use `--allow-not-run` to accept that, for example in CI.

## A failure says "this failure is not trusted"

An earlier test in the same codeunit was refused or stopped by the time limit, and it
could have changed what this test saw. The original error is kept in the message.
Fix or exclude the earlier test, or run this test alone with `--test-exact`, to get
a trusted result.

## Tests fail on empty tables

Setup records such as number series, posting groups and company information come
from the demo company. Check that test data was restored: `--verbose` prints the
test data line and its exclusions. Without test data (`--no-test-data`, or no backup
could be had), Microsoft's tests mostly fail on missing setup data. See
[Test data](test-data.md).

## The first run takes a long time

The first run that uses Microsoft's apps downloads them, restores the demo backup
(about 520 MB) and compiles the apps once (about 10 seconds). Progress shows on
stderr. Later runs load the compile cache. In CI, cache the folder named by
`ALRUNNER_CACHE`. See [Continuous integration](ci.md).

## A run is slow every time

Check that the compile cache is used: `--verbose` shows whether the dependency apps
were loaded from it. The cache key includes the `al-runner` build, so each new edge
build builds a new entry. `al-runner cache info` lists the entries, and
`al-runner cache clear` removes them if a cached result is in doubt.

## The run exits with 8

Some tests reach `DotNet` code that needs the .NET 8 helper, and it could not be
used. Install the .NET 8 SDK (the runtime alone is not enough to build the helper),
or accept the result with `--allow-not-run`. See [What works](what-works.md#dotnet).

## "A required dependency is missing" or "no AL source"

Microsoft's apps come from the artifact feed unless `--no-download` is set. An app
from another publisher must be given with `--dep`, `--package-cache`, or the
`.alpackages` folder, and it must carry AL source. A package without source cannot be
used, and the message names it.

## `-v: no such file or folder`

`3.0.0-alpha.3` read the short flags `-v` and `-q` as app paths
([issue 5618](https://github.com/aldevtools/BusinessCentral.AL.Runner/issues/5618)).
Update to `3.0.0-alpha.4` or later, or use `--verbose` and `--quiet`.

## The run exits with 3

The apps do not compile. An error the AL compiler gives, such as AL0118 or AL0666
(a method or property newer than the `runtime` in `app.json`), stopped the run
before any test. The errors are on stderr and in `compilationErrors` of
`--output-json`.

## Two copies of `al-runner`

A note on stderr says another `al-runner` is on your `PATH`, for example the .NET
tool next to the standalone binary. `al-runner --version` lists all copies. If one
of them is an old 2.x .NET tool, it runs the version 2 engine: update it with
`dotnet tool update -g MSDyn365BC.AL.Runner`, or remove it. Set
`ALRUNNER_NO_PATH_NOTE=1` to silence the note.

## The binary is blocked

The macOS and Linux binaries are not signed. The Windows binary is signed. See
[Getting started](getting-started.md#install) for the macOS and Windows steps.

## Something else

If a result differs from real Business Central, open an issue with the "Alpha
feedback" template at
[aldevtools/BusinessCentral.AL.Runner](https://github.com/aldevtools/BusinessCentral.AL.Runner/issues/new/choose).
Include `al-runner --version`, the Business Central version, the test name, what you
expected, and the output of `--explain TEST`.
