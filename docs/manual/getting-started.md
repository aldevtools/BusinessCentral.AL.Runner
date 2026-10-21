---
title: "Getting started"
weight: 2
---

## Install

`al-runner` is built for Linux x64, Windows x64 and macOS arm64.

**With mise** (recommended). mise installs the binary from the GitHub releases
and keeps it current:

```bash
mise use -g github:aldevtools/BusinessCentral.AL.Runner        # newest numbered alpha
mise use -g github:aldevtools/BusinessCentral.AL.Runner@edge   # nightly build of main
```

The name `edge` never changes, so `mise up` does not see a newer edge build. Use
`mise install --force github:aldevtools/BusinessCentral.AL.Runner@edge` to update it.

**With the install script.** The script checks the SHA-256 of the download and
puts the binary in `~/.local/bin` (Linux, macOS) or `%LOCALAPPDATA%\alrunner\bin`
(Windows).

```bash
curl -fsSL https://github.com/aldevtools/BusinessCentral.AL.Runner/releases/download/edge/install.sh | sh -s -- edge
```

```powershell
& ([scriptblock]::Create((irm https://github.com/aldevtools/BusinessCentral.AL.Runner/releases/download/edge/install.ps1))) -Version edge
```

Replace `edge` with `latest-alpha`, or with a tag such as `v3.0.0-alpha.3`, for a
numbered alpha. You can also download an archive from the releases page and check
it against `SHA256SUMS`. `gh attestation verify FILE --repo aldevtools/BusinessCentral.AL.Runner`
checks the build provenance.

**Which build?** `edge` is rebuilt every night from the newest commit of `main`
that passed CI. A numbered alpha (`3.0.0-alpha.N`) is released by hand. Use a
numbered alpha when you need a version you can name in a bug report.

**The binaries are not signed.** On macOS, Gatekeeper blocks the file; run
`xattr -d com.apple.quarantine ./al-runner` (the install script does this). On
Windows, SmartScreen warns; choose "More info", then "Run anyway", or run
`Unblock-File .\al-runner.exe`.

**Two commands named `al-runner`.** Version 2's dotnet tool has the same command
name. If both are installed, the one found first on your `PATH` runs. Version 3
prints a note on stderr when it finds the other one, and `al-runner --version`
lists all copies. Remove version 2 with
`dotnet tool uninstall -g MSDyn365BC.AL.Runner`, or silence the note with
`ALRUNNER_NO_PATH_NOTE=1`.

Check the install:

```bash
al-runner --version
```

## Run your tests

Pass the app and its test app. Each must carry its AL source:

```bash
al-runner test ./MyApp ./MyApp.Test
```

An argument can be an app folder (with `app.json`), a source archive
(`Tests-SMB.Source.zip`), or an `.app` that carries its source. Every
`Subtype = Test` codeunit of every app runs, each in its own session. Your apps
are compiled on every run.

The default output shows the version line, one line per failing test with its AL
call stack, the top reasons tests were not run, a `Tests:` summary with a table of
counts, one time line, and a `Result:` line:

```
FAIL    50100 Demo Calc Tests.Wrong: Expected 80 but got 90
          "Demo Calc Tests"(CodeUnit 50100).Wrong line 5 - Demo.Test by Me version 1.0.0.0
Tests: 2 total, 1 passed, 1 failed, 0 not run
  passed        1   50.0 %
  failed        1   50.0 %
  not run       0    0.0 %
  time        0.81 s   (compile 0.65 s, run 0.00 s, 742 tests/s)
Result: FAILED (exit 1)
```

Add `--verbose` for the setup details and one line per test. Add `--quiet` for the
summary only. Errors go to stderr.

## The first run

The first run that uses Microsoft's apps is slow, and later runs are fast:

1. AL Runner downloads Microsoft's apps, the test libraries and the platform
   `System.app` from Microsoft's artifact feed.
2. It restores the CRONUS demo company for the same build (about 520 MB, with
   progress on stderr). See [Test data](test-data.md).
3. It compiles Microsoft's apps completely once, which takes about 10 seconds for
   Business Central, and keeps the result in the compile cache (about 20 MB per
   set of apps).

Later runs load the cache. A test app with no Microsoft dependencies needs none
of this. Use `--no-download` to forbid network access, and `--no-test-data` to
skip the demo company.

Downloads and the compile cache live in the cache folder: `--cache-dir DIR`, else
`$ALRUNNER_CACHE`, else `$XDG_CACHE_HOME/al-runtime`, else `~/.cache/al-runtime`.
`al-runner cache info` shows what is in it, and entries unused for 30 days are
removed automatically. See [the command line reference](cli-reference.md#the-compile-cache).

## Where dependencies come from

For the apps you name, AL Runner finds the apps they depend on (from `app.json`)
in this order:

1. `--dep PATH`: a package file (`.app` or `*.Source.zip`), an app folder, or a
   folder of packages. Repeatable.
2. `--package-cache DIR`: a folder of `.app` files. Repeatable.
3. The `.alpackages` folder of each app you name.
4. Microsoft's artifact feed.

Microsoft's apps are compiled from their own AL source. A package that carries no
AL source cannot be used, and tests that reach it are not run. The message names
the app. A Microsoft app in `.alpackages` that is older than the Business Central
version of the run gives way to the feed's copy.

## Choosing the Business Central version

`--bc-version V` takes a major (`28`), a major and minor (`28.5`), or an exact
build (four numbers). Without it, AL Runner picks the first rule that applies:

1. The build of the tests, when they are Microsoft's own test apps.
2. The build of the Microsoft packages in your package folders, when it is of the
   target major and not below the `application` and `platform` versions in `app.json`.
3. The newest published build of the target major.

The target major is the major of `application` in `app.json`, else the newest
major AL Runner is verified against (29). `--verbose` prints which rule chose the
build. The corpus is checked against 27, 28 and 29.

## Next

[Writing tests](writing-tests.md) covers how a test project is put together.
[Reading results](results.md) explains "not run".
