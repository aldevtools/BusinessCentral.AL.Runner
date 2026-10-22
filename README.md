# BusinessCentral.AL.Runner

Run Business Central AL tests without a BC service tier, without Docker, without SQL Server
and without a BC license.

> **AL Runner v3 replaces v2.** This repository now holds AL Runner v3, written in Rust.
> AL Runner v2 (C#) is frozen on the [`v2` branch](https://github.com/aldevtools/BusinessCentral.AL.Runner/tree/v2) (MIT License).
> Its dotnet tool [`MSDyn365BC.AL.Runner`](https://www.nuget.org/packages/MSDyn365BC.AL.Runner)
> stays on NuGet. v2 gets no further releases; the versions published on NuGet keep working. If you maintain a tool that
> calls AL Runner, see [Migrating tools from AL Runner v2 to v3](docs/integrations/migrating-tools-to-v3.md).
>
> v3 was developed in the repository `StefanMaron/al-runtime-rs`, now archived. Issue and pull
> request numbers below about 5570 in the commit history and in older documents refer to that
> repository.

## What it is

AL Runner is a standalone test runner for Business Central AL code. You point it at your app
and its test app, and it runs the test codeunits and reports each test as passed, failed or
not run.

AL Runner parses and checks AL source with its own front end and runs it in its own VM against an
in-memory database. It needs no BC service tier, no Microsoft DLLs and no .NET. AL code that
uses the `DotNet` type calls an optional .NET helper process (see "DotNet" below). Microsoft's
apps (System Application, Base Application, the test libraries) are compiled from their own
source, so your tests run against Microsoft's real AL code, not against stubs.

It is meant for AL developers who want quick test results on their own machine or in a pull
request build, before the full BC pipeline runs. The full pipeline with a service tier stays
the final check; AL Runner does not replace it. It is an alpha: many tests are still refused,
so do not gate a pipeline on it yet (see "Try the alpha").

What it does not do is just as important. A feature AL Runner does not support is refused:
the test is reported as "not run", with the reason. It never returns a guessed default
instead. See "Can I trust the results?" and "What AL Runner does not do" below.

The goal is the whole AL language. How much of it runs today is in "Status" below and in
`docs/status.md`.

### Two commands named `al-runner`

The v2 dotnet tool also installs a command named `al-runner`. If both are installed, two
commands with that name are on your `PATH`, and the one found first runs. This `al-runner` prints a one-line note on
stderr when it finds the v2 tool (at most once a day; `ALRUNNER_NO_PATH_NOTE=1` turns it off), and
`al-runner --version` lists the other copies it finds. To avoid the clash, uninstall the
dotnet tool:

```
dotnet tool uninstall -g MSDyn365BC.AL.Runner
```


## Install

`al-runner` is built for Linux x64, Windows x64 and macOS arm64.

**With mise (recommended).** [mise](https://mise.jdx.dev) installs the binary from the GitHub
releases of this repository and keeps it up to date.

```
mise use -g github:aldevtools/BusinessCentral.AL.Runner    # install the newest alpha
mise up                                         # update to a newer alpha
```

Each numbered alpha (`3.0.0-alpha.N`) is a normal GitHub release, so mise finds it.

**The edge build, with mise.** `edge` is rebuilt every night from the newest commit of `main`
that passed CI. Pick it if you want every build, not only the numbered alphas. The numbered
alphas are released by hand.

```
mise use -g github:aldevtools/BusinessCentral.AL.Runner@edge
mise install --force github:aldevtools/BusinessCentral.AL.Runner@edge    # update
```

The version name `edge` never changes, so `mise up` does not see a new edge build. Use
`mise install --force` to get the newest one.

**With the install scripts.** The scripts check the SHA-256 of the download.

Linux, macOS:

```
curl -fsSL https://github.com/aldevtools/BusinessCentral.AL.Runner/releases/download/edge/install.sh | sh -s -- edge
```

Windows (PowerShell):

```
& ([scriptblock]::Create((irm https://github.com/aldevtools/BusinessCentral.AL.Runner/releases/download/edge/install.ps1))) -Version edge
```

Replace `edge` with `latest-alpha` or a tag such as `v3.0.0-alpha.4` for a numbered alpha.
The binary goes to `~/.local/bin` (Linux, macOS) or `%LOCALAPPDATA%\alrunner\bin` (Windows).
Add that folder to your `PATH` if the script says so. More about builds and about
signing is in "Try the alpha" below.

## Getting started with your own project

`al-runner` runs the tests of your AL app without a Business Central service tier. It is an
alpha: many tests are refused ("not run", see below) and results may change between builds.
Do not gate a pipeline on it yet.

**1. Install.** See "Install" above.

**2. Run your tests.** Pass the app and its test app:

```
al-runner test ./MyApp ./MyApp.Test
```

Every test codeunit runs in its own session. Your apps are compiled on each run. Each app
must carry its AL source.

**3. Dependencies.** For the apps you name, `al-runner` looks for the apps they depend on
(`app.json`) in this order:

1. `--dep PATH`: a package file (`.app` or `*.Source.zip`), an app folder or a folder of
   packages. You can repeat it.
2. `--package-cache DIR`: a folder of `.app` files. You can repeat it.
3. The `.alpackages` folder of each app you name.
4. Microsoft's artifact feed. It supplies Microsoft's apps, the test libraries and the
   platform `System.app`. `--no-download` turns the feed off.

A Microsoft app in `.alpackages` that is older than the BC version of the run gives way to the
feed's copy of that version. Apps from `--dep` and `--package-cache` are always used as found.

Microsoft's apps are compiled from their own source. A package that carries no AL source
cannot be used, and tests that reach it are not run (the message names the app).
Downloads are kept in a cache folder: `--cache-dir DIR`, else `$ALRUNNER_CACHE`, else
`$XDG_CACHE_HOME/al-runtime`, else `~/.cache/al-runtime`.

**4. Choose the BC version.** `--bc-version V` takes a major (`28`), a major and minor
(`28.5`) or an exact build (four numbers). Without it, `al-runner` uses these in order: the
build of the tests when they are Microsoft's own test apps; the build of the Microsoft packages
in your package folders (`.alpackages`, `--dep`, `--package-cache`), if it is of the target BC
major and at or above the `application` and `platform` versions in `app.json`; then the newest
published build of the target major. The target major is the major of `application` in
`app.json`, else the newest BC major the runtime is verified against (29). `--verbose` prints
which rule chose the build. The corpus is checked against BC 27, 28 and 29.

**5. Test data.** A run that uses Microsoft's apps restores the CRONUS demo company of the
same BC build. It is on by default. The first run downloads about 520 MB into the cache; progress
shows on stderr. If the backup cannot be had, test data is turned off and the run says why.
`--no-test-data` turns it off yourself. Without test data, Microsoft's setup tables are empty.
`--company-data package` builds the company the way Microsoft's Legacy test lane does, from the
artifact's RapidStart package, instead of restoring the backup (opt-in; see the next two flags
for the rest of Microsoft's recipe). For a parity report, `--disabled-tests DIR` reads Microsoft's
`src/DisabledTests` folder from BCApps. A normal run still runs those tests and marks them in
the summary and the JUnit file. `--parity` also skips them.
`--recipe microsoft` does all of this the way Microsoft's CI does: BCApps' own settings decide the
company of each test codeunit (the Legacy test apps get the package company, a codeunit with
`TestType = UnitTest` an empty one), the disabled tests are skipped, and each lane has its own
summary and JUnit file. The BCApps files (MIT) are fetched sparsely with `git` into the cache once;
`--bcapps DIR` names a checkout, `--no-download` uses the cache only. The two Contoso lanes
(IntegrationTest and the codeunits with no TestType) are not run yet: the runner does not build the
Contoso company, and says so. The recipe's time limit per test is 60 s of CPU time (20 s for other
runs); `--test-timeout-secs N` or `ALRUNNER_TEST_TIMEOUT` sets another.
`al-runner test-microsoft` runs all of Microsoft's test apps of a BC build this way, with nothing to
download or collect by hand: it reads the list of test apps from the build's artifact, fetches the
ones it runs, and runs each by the recipe (see "Microsoft's test buckets" below).
`--server-url URL` (default `http://localhost:8080/BC`) sets the address `GetUrl(ClientType::Web)` answers with; a company built from the package needs it for Company-Initialize.

**6. .NET is optional.** Nothing in the core needs .NET. AL code that uses the `DotNet` type
is served by a .NET 8 helper process. A few common types (for example `StringBuilder`,
`byte[]`, `Encoding`, SHA and MD5 hashes, Base64) run inside `al-runner` and need no helper
(ADR-038). Tests that reach any other `DotNet` call need the .NET 8 SDK. The binary carries the
helper sources and builds the helper into the cache on first use. Without .NET those tests are
not run, with a reason, and the exit code is 8. `--require-dotnet` makes a missing helper an
error.

**7. The compile cache.** Microsoft's apps are compiled completely once. The result is kept in
`<cache dir>/compile/` (about 20 MB per set of apps) and loaded on later runs. The first run
that needs it takes longer (about 10 s for Business Central). The cache also holds the state
the install code of Microsoft's apps leaves in the database. Entries nothing used for 30 days
are removed.

- `al-runner cache clear` removes every entry. Use it when a cached result is in doubt.
- `--rebuild-compile-cache` builds the entries this run needs again and overwrites them.
- `--no-compile-cache` turns the cache off for one run.
- `--require-compile-cache` fails when there is no usable entry. Use it in CI.

**8. Exit codes.**

| Code | Meaning |
|---|---|
| 0 | every selected test passed |
| 1 | a test failed |
| 2 | bad invocation, or the run could not start |
| 6 | the selection matched no test |
| 7 | some tests were not run (a refusal) |
| 8 | some tests were not run because the .NET helper could not be used |

When several apply, the order is 2, 6, 1, 8, 7. `--allow-not-run` makes 7 and 8 return 0.

**9. "Not run" and how to see why.** "Not run" means `al-runner` refused the test and gave a
reason, usually a feature that is missing. The test neither passed nor failed. Missing
features are refused, not guessed. See "Can I trust the results?" below for what a pass,
a failure and "not run" each mean.

- The default output has the version line, failures, the top refusals, the `Tests:` line, the totals
  table, one time line (total, compile, run, tests per second) and the `Result:` line. A warning that changes how to read the result is one line, only when it applies
  (for example "baseline incomplete: 16 of 17 install roots ran to their end").
- `--verbose` adds everything else: setup details first (dependency notes, target, apps,
  dependencies, threads, isolation, server address, test data, baseline details), then one line
  per test (with its run time in ms) and the reason for each "NOT RUN", then the test data rows and the full `Time:` line (load, compile, install baseline, run, total, peak memory) in place of the short time line.
  `--quiet` prints the summary only. Errors go to stderr.
- `--color auto|always|never` (default auto) colors PASS green, FAIL red, NOT RUN yellow,
  flagged passes cyan and headings bold. Auto colors only on a terminal and never with `NO_COLOR`.
  The `Tests:` and `Result:` lines keep their text; a small table of counts and shares follows
  the `Tests:` line.
- `--top-refusals [N]` ranks the most common reasons (default 10).
- `--explain TEST` runs one test and prints its outcome, and for a refusal the routine and
  source line it stopped at.
- `--filter TEXT` and `--codeunit ID` select tests. `--junit FILE` writes JUnit XML where a
  test that did not run is `<skipped>` with its reason.

**10. GitHub Actions.** A minimal job for a repository that holds `MyApp` and `MyApp.Test`.
It uses only the install script and flags that `al-runner test --help` lists. This repository's
own CI does not run it.

```yaml
name: AL tests
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    env:
      ALRUNNER_CACHE: ${{ github.workspace }}/.alrunner-cache
    steps:
      - uses: actions/checkout@v4
      - name: Install al-runner
        run: |
          curl -fsSL https://github.com/aldevtools/BusinessCentral.AL.Runner/releases/download/edge/install.sh | sh -s -- edge
          echo "$HOME/.local/bin" >> "$GITHUB_PATH"
      - uses: actions/cache@v4
        with:
          path: .alrunner-cache
          key: alrunner-${{ runner.os }}-${{ hashFiles('**/app.json') }}
      - name: Run tests
        run: al-runner test ./MyApp ./MyApp.Test --junit test-results.xml --allow-not-run
      - uses: actions/upload-artifact@v4
        if: always()
        with:
          name: test-results
          path: test-results.xml
```

`--allow-not-run` keeps refused tests from failing the job while the runtime is an alpha. Drop
it when you want every test to run. Pin a numbered alpha (`v3.0.0-alpha.4`) instead of `edge`
if the result must be repeatable.

**11. Report a wrong result.** A test that passes or fails here and behaves differently on real
Business Central is the most useful report. Open a GitHub issue with the "Alpha feedback"
template. Include `al-runner --version`, the BC version, the test name, what you expected, and the
output of `--explain TEST`.

## Can I trust the results?

A pass or a failure here is meant to match real Business Central. When the runtime cannot be
sure, it reports the test as "not run" and says why. Some false results are still possible.
The known causes are listed under "Known risks" below. For Microsoft's own tests, nobody has
measured yet how often the result here matches real BC.

**A pass** means the test ran to its end without an error. The project rule is that every
behavior the runtime implements is measured on real BC by a test in the
[AL language test corpus](https://github.com/aldevtools/BusinessCentral.AL.Language.Tests),
per BC version. There is one exception. A behavior that Microsoft's own tests need on real BC
may be built before a corpus test measures it. Each such behavior is registered in the runtime
code as an unprobed assumption (`crates/al-builtins/src/assumptions.rs`; the list is also in
`docs/probes-needed.md`). A pass that ran through one is a **flagged pass**, counted apart:

- The `Tests:` line reads `N passed (M flagged: rely on unprobed assumptions)`.
- `--verbose` marks each such test `(flagged)`.
- `--explain TEST` lists each assumption and the probe that would settle it.
- `--junit FILE` lists the assumption ids in an `assumptions` property of the test case.
- `--top-assumptions` ranks the assumptions of a run. `al-runner --list-assumptions` describes all of them.

**"Not run"** means the runtime refused the test and gave a reason. There are three kinds:

1. A feature is missing.
2. A behavior is not measured on real BC yet.
3. The test failed, but its failure is not trusted. An earlier test in the same codeunit did
   not run to its end, and it could have changed what this test sees: a table this test
   reads, the codeunit's shared state, or the sequence of random numbers. When part of what the
   earlier test would have done is not known (it reaches code the runtime refuses), it may have
   changed anything. The reason says "this failure is not trusted" and keeps the original
   error. A test stopped by the time limit counts as such an earlier test.

**A failure** means the test ran and raised an error or failed an assertion, and no earlier
refused test can explain it. Without test data (`--no-test-data`), Microsoft's tests fail mostly
on missing setup data. The `Tests:` line says so.

**How results are checked against real BC**

- The corpus runs on real BC 27.5, 28.5 and 29.0. The JUnit result of each run is the oracle
  for that version (`oracle/`). A corpus test counts as passing only when it passes here and
  passed on real BC.
- `metrics/<major>/passing.txt` lists the corpus tests that must keep passing. CI runs all
  three versions on every pull request and fails when one of them stops passing.
- Must-not-compile cases: 272 small AL files the real AL compiler rejects. The runtime must
  reject each one with the same diagnostic codes, on each version. The cases not matched yet
  are listed as known gaps; any other mismatch fails the tests.
- The corpus CI runs BC in a Linux container, which Microsoft does not support. Results that
  look like a container difference are run on Microsoft's Windows container first. Where the
  two differ, the runtime refuses that case (`docs/probes-needed.md`, "Linux artifacts checked
  on Windows").

**Known risks**

1. Code written before the measurement rule. Some behavior has no corpus test yet. Corpus tests
   are being added for it. Recent examples: a Duration divided by a number raised an error
   where BC returns a Decimal (fixed); `IsPathTemporary` gave the Linux container's answer for
   paths where Windows BC answers differently (now refused); the test clock did not move
   between reads (fix in review).
2. A flagged pass depends on its assumption. When a corpus test measures the behavior and
   disagrees, the runtime is changed to match BC.
3. The Linux container can differ from BC on Windows. Only results someone suspects are run on
   Windows.
4. Microsoft's own tests are not compared with real BC test by test. Their pass rate is the
   result of this runtime, not a measured match with BC.
5. Permissions are not enforced: every access is granted (ADR-027). A test that expects a
   permission error fails here. Code that BC would stop for a missing permission runs here.
6. The "not trusted" rule covers failures only. A test that passes after a refused test stays
   a pass, even if the refused test would have changed its data on BC.

**Report a suspected false result.** Open an issue on this repository with the "Alpha
feedback" template. Give the test name, the BC version, the result here, the result on real
BC, `al-runner --version` and the output of `--explain TEST`.

## Why it is built on the language, not on BC's internals

AL Runner reproduces what AL code can observe: values, errors, data, and the order of
triggers and events. That surface is the public contract of the language. Microsoft cannot
change it without a breaking change for every partner, so it changes slowly and visibly,
behind versions and obsolete markers.

A runner can instead run Microsoft's compiled code on top of BC's internals: the C# runtime,
its metadata and session classes, and patches into them. Microsoft can change those in any
hotfix or release without notice, so a fix there can stop working with the next update, and
such a runner needs one engine per BC minor version. AL Runner is one binary for all supported
versions.

Where the observable behavior does differ between BC versions (an error text, a system
table column), the corpus measures it on every major and pins it per version, so a change
in a new BC release shows up as a failing corpus case with a known cause. The one part that
still loads Microsoft's internals is the optional .NET helper for `DotNet` code, and it is
kept small on purpose.

## What "correct" means here

Correctness is defined by real Business Central, not by documentation. The
[AL language test corpus](https://github.com/aldevtools/BusinessCentral.AL.Language.Tests)
runs its CI against real BC in containers, once per BC major (27, 28 and 29).
The JUnit result of each run is the oracle for that major (`oracle/`). A test
counts as passing only when it passes here **and** passed on real BC.
Observable behavior must match BC. Internals, speed and diagnostics are free to
be better (ADR-009).

## Status

Measured on 2026-10-21 with `al-test-cli --root . --all-versions`, corpus commit `f3fe6c99`.
Each major has its own oracle and its own baseline of cases that must keep passing
(`metrics/<major>/passing.txt`).

| BC version | Passing | Failing | Not run |
|---|---|---|---|
| 27.5 | 13964 / 17050 (81.9%) | 0 | 3086 |
| 28.5 | 14002 / 17140 (81.7%) | 0 | 3138 |
| 29.0 | 14023 / 17234 (81.4%) | 0 | 3211 |

A test counts as passing only when it passes here and passed on real BC. "Failing" is a case
where real BC passed and the runtime gave a different result or an error. No case fails on any version. "Not run" means the runtime refused
the test, with a stated reason: a feature is missing, a behavior is not pinned yet, or a
failure is not trusted. Between 435 and 438 passes on each version run through an unprobed assumption
(435 on 27.5, 438 on 28.5, 437 on 29.0; counted as "flagged" in the summary). See "Can I trust the results?". Per-area numbers and the roadmap are in `docs/status.md`, which is older than these
numbers. The corpus also has a smaller OnPrem test app (121 of 342 on 27.5, 125 of 348 on 28.5,
123 of 356 on 29.0) and two isolation probes, reported separately.

On BC 27.5 the corpus compiles Microsoft's apps from the corpus's own `.alpackages`. The
other versions take them from the artifact of the matching build.

Time: the command above, for all three versions, took 38 s of wall time on the main machine.
`al-test-cli` compiles Base Application, System Application and the others from source on every
run, only what the tests reach. `al-runner test` keeps their complete compile in a compile cache
(ADR-022, `<cache dir>/compile/`) and loads it on later runs. `--no-compile-cache` turns it off,
and `al-test-cli --compile-cache` turns it on for the corpus.

## Working on the runtime

Clone with `--recurse-submodules`; the corpus is a pinned submodule.

```
cargo build --release
./target/release/al-test-cli --root .                  # pass rate for the newest BC version, ratchet check
./target/release/al-test-cli --root . --all-versions   # 27.5, 28.5 and 29.0, then compare
./target/release/al-test-cli --root . --explain TEST   # why one test passed, failed or was not run
./check.sh                                             # fmt, clippy, all tests, corpus ratchet
```

## Try the alpha

`al-runner` is in alpha (`3.0.0-alpha.4`).
Many tests are refused, and results may change between builds. Use it to try
it out and to report what is wrong. Do not gate a pipeline on it.

**Which build?**

- **edge**: rebuilt every night from the newest commit of `main` that passed CI.
  One pre-release named `edge`, replaced on each build. Pick this to test the latest fixes.
- **alpha**: a numbered checkpoint (`3.0.0-alpha.N`), released by hand. Pick this
  for a stable point you can name in a bug report.

**Install** (Linux x64, Windows x64, macOS arm64). One line, pointing at the newest
build (`edge`). The scripts check the SHA-256 of the download. Replace `edge` with
`latest-alpha` or a tag such as `v3.0.0-alpha.4` for a numbered checkpoint. mise works too: see
"Install" near the top.

Linux, macOS:

```
curl -fsSL https://github.com/aldevtools/BusinessCentral.AL.Runner/releases/download/edge/install.sh | sh -s -- edge
```

Windows (PowerShell):

```
& ([scriptblock]::Create((irm https://github.com/aldevtools/BusinessCentral.AL.Runner/releases/download/edge/install.ps1))) -Version edge
```

`irm https://github.com/aldevtools/BusinessCentral.AL.Runner/releases/download/edge/install.ps1 | iex` also works and
installs the latest numbered alpha. (The `edge` release is the stable place for the scripts:
its URL never changes.)
You can also download an archive from the releases page, check it against
`SHA256SUMS`, and unpack it. Each archive holds the binary, `LICENSE` and a short
`README.txt`. `gh attestation verify FILE --repo aldevtools/BusinessCentral.AL.Runner`
checks the build provenance.

**Signing.** The Windows binary is signed (Authenticode, Azure Trusted Signing). The macOS
and Linux binaries are not signed.

- macOS: Gatekeeper blocks the file. Run `xattr -d com.apple.quarantine ./al-runner`
  (the install script does this for you).
- Windows: `Get-AuthenticodeSignature .\al-runner.exe` shows the signature. If SmartScreen
  still warns, click "More info", then "Run anyway", or run `Unblock-File .\al-runner.exe`.

**Run**

```
al-runner --version
al-runner test ./MyApp ./MyApp.Test
```

Pass both the app and its test app. Then your app is compiled fresh on each run,
and Microsoft's apps stay in the cache. The first run that needs Microsoft's apps
downloads them from Microsoft's artifact feed and builds the cache. Later runs
load it.

- **Test data** (the CRONUS demo company) is on by default when a run uses
  Microsoft's apps. The first run downloads about 520 MB. `--no-test-data`
  turns it off.
- **"Not run"** means `al-runner` refused the test and said why, for example a
  feature it does not support yet. Missing features are refused, not guessed. The
  test did not pass and did not fail. `--verbose` lists the reason for each test.
  See "Can I trust the results?".
- **Exit codes**: see "Getting started with your own project" above.

**Known limits**

- Tests that use the `DotNet` type need the .NET 8 SDK. The binary carries the
  helper sources and builds the helper into the cache on first use. Without the
  SDK these tests are not run, with a reason.
- Many features are missing. See `docs/status.md` and `docs/al-runner-v2-parity.md`.
- No watch mode. `al-runner debug` is a Debug Adapter Protocol server for breakpoints, stepping and
  locals (`docs/debugging.md`). `al-runner serve` and `--coverage` (Cobertura XML,
  `--coverage-json`) exist. `serve` speaks AL Runner v2's JSON-lines protocol (`runTests`,
  `execute`, `cancel`, `shutdown`). All requests run in the one server process, which keeps the
  compile of the last request, so a repeated request with the same sources takes 0.2 to 2 s
  (more with Microsoft's apps when many tests run). Each request starts from the same database. Fields and answers are in
  `docs/integrations/migrating-tools-to-v3.md`, section 4b.
- `--tdd` (test-first work): a test that calls a procedure of a codeunit that does not exist
  yet runs against a stub that returns the default value (0, '', false, an empty record)
  instead of failing to compile. Each such test is reported as stubbed, pass or fail, and the
  summary counts stubbed tests apart. Stubs stand for procedures (on codeunit, record, page and
  report variables, `Rec`, `CurrPage`, bare calls, missing overloads), and for fields, enum values
  and missing codeunits, tables and enums, which are declared for the run (ADR-051). `serve` takes it as the `tdd` field of `runTests`, or `serve --tdd`.
- `--affected` (ADR-050): run only the test codeunits a change since the last `--affected`
  run can affect, against a stored result in the cache folder (or `--affected-store FILE`, for
  CI). The other tests keep their stored outcome: they are counted as skipped, never as passed,
  and one that failed last time keeps the exit code at 1. A change the rule cannot follow
  (an added object, `app.json`, a dependency app) runs everything and says why.
  `--include-failing` also runs every test that did not pass last time. In `serve`: the
  `affectedOnly` and `includeFailing` fields of `runTests`.
- Only three targets are built: Linux x64, Windows x64 and macOS arm64.

**Report problems** as a GitHub issue, using the "Alpha feedback" template.
A wrong result is the most useful report: a test that passes or fails here and
behaves differently on real Business Central.

## What AL Runner does not do

Some AL features are refused on purpose. A test that reaches one is reported as
"not run", with the reason. A few are not refused but are limited: for example, every
permission is granted, so a test that expects a permission error fails here. The table
says which.

| Area | What AL Runner does | What to do instead |
|---|---|---|
| Email, outbound HTTP, web service endpoints | Not run. No network, no server | Put the call behind an AL interface and fake it in the test |
| Cloud storage, Key Vault, external data connections | Not run | Same |
| File upload and download dialogs | Not run (needs a browser) | Test with streams |
| Report rendering to PDF, Word or Excel | Not run. The XML dataset works | Assert on the dataset |
| Background jobs, `StartSession` | Tasks are created and never run. A second session is not started | Call the code the task would run |
| Licensing, permission enforcement | Everything is granted. Denial paths cannot be tested | Use a real service tier |
| Apps that ship without AL source | Not run. The message names the app | Provide an `.app` with source |
| Watch mode | Not available | Not available yet |

The full list, with reasons and open questions, is
[ADR-027](docs/adr/027-scope.md) (Proposed).

## Command lines

**`al-test-cli`** runs the corpus and reports the pass rate against the oracle.
Exit code 0 is ok, 1 means a baseline case no longer passes, 2 is a setup error.
Main flags by purpose (`--help` lists all of them):

- Version: `--bc-version N`, `--all-versions`.
- Test selection: `--codeunit ID`, `--test NAME`, `--area FOLDER`, `--file SUFFIX`.
- Diagnosis: `--explain TEST`, `--top-refusals [N]`, `--baseline-diff`, `--timings`.
- Baselines and reports: `--update-baseline`, `--no-ratchet`, `--junit FILE`,
  `--json FILE`, `--markdown FILE`.

**`al-runner`** runs the tests of any app, or of one of Microsoft's
`Tests-*.Source.zip` archives. Every test codeunit runs in its own session.

```
al-runner test ./MyApp                                       # an app folder with app.json
al-runner test ./MyApp --filter Posting --verbose            # selected tests, one line per test
al-runner test Tests-ERM.Source.zip                          # Microsoft's tests: apps, System.app and CRONUS data fetched
al-runner test-microsoft --app Tests-ERM                     # the same app, found in the artifact, run by Microsoft's recipe
al-runner test ./MyApp --no-test-data                        # without the CRONUS company
al-runner debug --stdio ./MyApp ./MyApp.Test                 # debug the tests from an editor (DAP)
```

Exit codes are in "Getting started with your own project" above. `al-runner test --help`
lists every flag.

## Dependencies

Microsoft's apps are compiled from their own source (ADR-013). Nothing of
Microsoft's is committed to this repository.

1. `al-runner` searches `--dep` and `--package-cache` first, then each app's
   `.alpackages` folder. The corpus ships System Application, Base Application
   and Business Foundation in its `.alpackages`, with source.
2. What is still missing comes from Microsoft's artifact feed: the apps, the
   test libraries, the platform `System.app` and the demo data. For Microsoft's
   own test archives the build is the build of the tests when it is still
   available. Downloads are kept in a local cache (`--cache-dir`).
   `--no-download` turns the feed off.
3. Only the objects the selected tests reach are compiled.

`al-test-cli` searches `.alpackages` first, then another folder of `.app`
packages given with `--deps DIR`, then the feed.

## Test data

Test data is on by default. A run that uses Microsoft's apps restores the CRONUS
demo company of the same BC build from its backup (`.bak`); a table is read the
first time a test touches it. The backup is looked for in
`~/.bcartifacts.cache/sandbox/<build>/w1`, then in the artifact cache, then it is
downloaded once (about 520 MB, progress on stderr) from the artifact feed into the
cache. If none can be had, test data is off and the run says why.
`--no-test-data` turns it off; `--test-data PATH.bak` names a backup.
Without test data, Microsoft's tests find an empty database and many fail on
missing setup data. Those failures are not trusted.

`al-bcdb` is the reader behind this: a Rust port of bcdb for BC `.bak` and
`.bacpac` files, with no SQL Server. It is also a command line
(`al-bcdb tables FILE`). See `docs/bcdb.md`.

## DotNet

The core needs no .NET. AL code that uses the `DotNet` type is served by an
optional .NET 8 helper process (ADR-014). A closed set of pure .NET framework members
(`StringBuilder`, `byte[]` access, `Encoding`, SHA and MD5 hashes, Convert Base64) runs inside
`al-runner` and does not call the helper (ADR-038). Without the helper, tests that reach any
other `DotNet` call are not run, with the reason stated, and the exit code is 8.
`--require-dotnet` turns that into an error, for CI.

## Microsoft's test buckets

`al-runner test-microsoft` runs every Microsoft test app of a BC build as Microsoft's CI does, without
paths: the newest build of the newest BC major by default (`--bc-version`, `--country` choose another),
each test app with Microsoft's recipe (`--recipe microsoft`: the company of each codeunit, Microsoft's
disabled tests skipped). `--app NAME` (repeatable) runs only the named test apps, by their name in the
artifact (`Tests-ERM`, `Data Search Tests`). Apps without a test codeunit (test libraries) are loaded as
dependencies, not run; apps whose artifact name starts with `_Exclude_` run only when `--app` names
them. Each test app is a run of its own; the end of the output is one row per test app (passed, failed,
not run, total, codeunits of a lane the runner cannot build yet, exit code) and a total. `--junit FILE`
writes one file per test app and lane (`FILE.Tests-ERM.legacy.xml`). The exit code is the worst test
app's. `--list-lanes` prints, without compiling or running, the lane of Microsoft's CI (Default,
IntegrationTests, UncategorizedTests, LegacyTestsBucket1, ...) each test app's codeunits run in, with the
counts of tests, of tests Microsoft disables and of tests that would run; `--verbose` adds one line per
codeunit.

`al-runner test-microsoft MYEXT` runs only the Microsoft tests your extension can affect: each Microsoft
test app, lane by lane with the recipe, as an impact run of `MYEXT` (with and without it; see
`--impact-of`). Tests that do not reach the extension are counted as not selected, never as passed.
`al-runner test MYEXT MYTESTS --affected-from microsoft` does the same next to your own tests, in one
report with one summary table and one `--junit` file (each source suite has the property
`test_source`, the first one `not_selected`). Without a cached impact index the tests are selected by
static reach, with a warning that prints the command that builds the index:
`al-runner impact-index build --from microsoft --microsoft-app Tests-ERM --bc-version ...`.

An ISV's test app is a test source too: `al-runner test MYEXT --affected-from ../vendor/Product.Tests`
(an app folder, an `.app` with AL source or a `*.Source.zip`; a folder of several test apps is one source
per app; repeatable, also next to `--affected-from microsoft`). It runs with the run's normal test data,
and its dependencies resolve as for any APP. A test app or dependency without AL source is refused before
the run, as is a test app that depends on your extension (pass that one as an APP). `--output-json FILE`
writes your own tests as a plain run does plus a `sources` array; `--impact-json FILE` writes format 2
(one entry per impact run, each with its `source`); `--impact-accept FILE` reads format 1 and 2, and an
entry with a `source` matches only in that source. `al-runner serve` refuses test sources.

Matrix runs: `al-runner test MYEXT --bc-version 27.5,28.5,29.0 --country w1,us` runs every combination of
versions and countries, one cell each; `--matrix "27.5:w1,29.0:us"` names the cells. Every cell is checked
before any runs (its build, the country, the app's floor), and one bad cell stops the run. Each cell runs as
its own process; the report has a table of the cells and the list of tests whose outcome differs between
cells. `--matrix-parallel N` runs up to N cells at once when the free memory allows it. `--junit FILE` writes
one file per cell, `--output-json FILE` one document with every cell. The exit code is the worst cell's. AL
Runner sets no preprocessor symbols of its own in a cell: each compiles with the app's `preprocessorSymbols`
and `--define`, as a single run does.

`al-runner` can run Microsoft's own Base Application test buckets. Most of these tests are
still refused, not run. The full run of 2026-10-16 (main `fae0c8df`, BC 27.5, all buckets
together) held 40300 tests: 16848 passed, 2508 failed and 20944 were not run. It took 298 s on a
12-thread desktop CPU. `docs/alrunner-buckets.md` has the per-bucket numbers, the recipe and
the failure analysis. Speed numbers of such a run cannot be compared with other runners until
many more of these tests run.

## Documentation

- `docs/debugging.md`: `al-runner debug`, the Debug Adapter Protocol server.
- `docs/releasing.md`: how an edge build, a numbered alpha and a hotfix are released.
- `docs/architecture.md`: pipeline, crates, runtime model, budgets.
- `docs/codemap.md`: which module owns what, and where the entry points are.
- `docs/status.md`: current coverage, what is refused, roadmap, known issues.
- `docs/adr/`: architecture decisions. Seams change only through an ADR.
- `docs/alrunner-buckets.md`: how to run Microsoft's test buckets, and the measurements.
- `docs/bcdb.md`: the `.bak` and `.bacpac` reader.
- `docs/al-runner-v2-parity.md`: which features of AL Runner v2 exist in the current version.
- `docs/owner-review.md`: decisions made without the owner, waiting for review.
- `docs/dotnet-examples.md` and `docs/dotnet-generic.md`: how Microsoft's AL uses `DotNet`, and the options.
- `docs/probes-needed.md`: behavior taken from documentation only, waiting for a probe on real BC.
- `docs/corpus-inventory.md`: what the corpus contains.
- `CLAUDE.md`: the rules every contributor and agent follows.
- `docs/agents/brief.md`: the working method for implementation agents (worktree, TDD, gates, memory caps, report).
- `docs/agents/remote-worker.md`: setup and task loop for a session on another machine that takes GitHub issues.
- `oracle/`: the real-BC results the pass rate is measured against.
- `metrics/<major>/passing.txt`: the cases that must keep passing, per BC major.
- `docs/history/`: the original spike brief and report.

## License

AL Runner is licensed under the Business Source License 1.1 (`LICENSE`). You may use it for free,
including commercial work: developing, testing and building Business Central extensions for yourself or
for your customers, on your own machines and in build pipelines. You may not sell it, or offer it or a
product or service built substantially on it to others for a fee. Each version becomes available under
the Apache License 2.0 four years after it is published. Versions published before this change remain
under the MIT License. For other licensing arrangements, contact the author.
