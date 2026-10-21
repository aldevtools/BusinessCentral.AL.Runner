# Migrating tools from AL Runner v2 to v3

For the authors of tools that call AL Runner (ALchemist, LethAL, and others), and for the
AI-assisted maintainers who change them. Decision: ADR-016, "v3 replaces v2" (2026-10-15). Background
and the file:line list of what each tool uses: `docs/integrations/alchemist-lethal.md`.

How to use this page: find your tool's section, apply each "Change" literally, then run the
check at the end of the section. Every difference below is real in the code that is in this
repository. Where a row says "not yet", the step number is the step of the plan in ADR-016 (this
page grows with each step).

## Why move to v3

v3 reproduces the observable behavior of the AL language, which Microsoft keeps stable for
partners. v2 depends on BC's internals, which can change with any hotfix or release, so a tool
built on v2 can break without a change on its own side. See the README section "Why it is
built on the language, not on BC's internals".

## Status by step

| Step | What | State |
|---|---|---|
| 1 | `al-runner test` flags LethAL uses, v2-shaped `--output-json` | done (this page, sections 1 to 5) |
| 2 | Cobertura coverage (`--coverage`, `--coverage-out`), per-test statements (`--coverage-json`) | done (sections 3 and 4a) |
| 3 | `al-runner serve` (`runTests`, `cancel`, `shutdown`) | done (section 4b; ADR-028, Proposed) |
| 4 | `execute`, `cancel`, `coverage`, `perTestCoverage` in `serve` | done (section 4b; ADR-028) |
| 5 | `captureValues`, `iterationTracking` | done (section 4b; ADR-030, Proposed) |
| 6 | `al-runner debug --stdio` (DAP) | done, also in dependency apps compiled from source (`docs/debugging.md`; ADR-048) |

## 1. The command

| v2 | v3 | Change |
|---|---|---|
| binary `al-runner` | binary `al-runner` (called `alrunner` up to 3.0.0-alpha.2) | Run `al-runner test ...`. v2's dotnet tool has the same name, so a tool that finds the binary by name on the PATH may start v2. Tell them apart by `--version` (section 2), or use a configured path. v3 prints a note on stderr when it finds another copy; `ALRUNNER_NO_PATH_NOTE=1` turns it off. |
| `al-runner [FLAGS] DIR [DIR]` | `al-runner test [FLAGS] APP...` | Put the word `test` first. APP is an app folder (with `app.json`), a `*.Source.zip` or an `.app` that carries source. |
| install: `dotnet tool install -g msdyn365bc.al.runner` | one binary | Do not run `dotnet tool`. The user sets the path. v3 needs no .NET (an optional helper exists for `DotNet` variables). |
| `al-runner --server [--package-cache P]... [--define S]... [--test-timeout N]` | `al-runner serve [--package-cache P]... [--define S]... [--test-timeout N]` | Same options (section 4b). |
| `al-runner --dap stdio DIR` | `al-runner debug --stdio APP...` | Same DAP session (`docs/debugging.md`). |
| `al-runner --dap [PORT] DIR` | `al-runner debug [--port [PORT]] APP...` | Default port 4711 as v2. The listening line is on stderr (`al-runner debug: listening on 127.0.0.1:PORT`); v2 printed `[dap] listening on ...` on stdout. |

## 2. `--version`

v2 prints `... v2.12.0 ...` somewhere in its output. v3 prints

```
al-runner 3.0.0-alpha.2 (<commit>)
```

Change: do not match `/\bv2\.\d/`. Parse the first `\d+\.\d+\.\d+` (as ALchemist already does).
Read only the first line: when other copies of `al-runner` are on the PATH, v3 adds one
`also on PATH: <path>: <its version line>` line for each. Both versions start with `al-runner `
(v2 prints `al-runner v2.12.0`), so a tool that must tell them apart checks for `al-runner 3.`.

## 3. Flags of `al-runner test`

Accepted with the v2 meaning unless the "Difference" column says otherwise.

| Flag | Difference |
|---|---|
| `--output-json [FILE]` | See section 4. With no FILE, stdout holds only the JSON document. v2 mixed banner lines before it. |
| `--output-json=FILE` | Same, with a FILE of any name. Without the `=`, a file name is taken only if it ends in `.json`. |
| `--isolation codeunit\|test\|disabled` (also `--test-isolation`) | `codeunit`: the database is rolled back when a test codeunit ends (the default, as in Microsoft's test runner). `test` (aliases `function`, `method`): the database goes back to its state after `OnRun` after every test method, **the codeunit's AL variables are kept** (BC's `TestIsolation = Function`). v2's `test` also reset the codeunit's state; a tool that relied on that must not. `disabled`: not pinned, so every test is reported as not run (exit 7). |
| `--test TEXT` (also `--filter`) | Case-insensitive substring. TEXT with a dot, such as `Codeunit50011.Foo`, is matched against the qualified name `Codeunit<ID>.<Method>`; TEXT without a dot against the method name. v2 matched the qualified name for both. A bare name that is also a part of the word `Codeunit` is not special. |
| `--test-exact NAME` | New in v3 for the tools: the test whose method name or qualified name is exactly NAME (any case). Repeatable. Given with `--test`, it narrows: only the tests that both match are selected, as in v2. A name is the AL name as written, without quotes: `--test-exact "Codeunit50650.Space Step"`. |
| `--exclude-test TEXT` | Removes the tests TEXT matches (the same matching as `--test`). Wins over every selecting option. Repeatable. Use whole qualified names to exclude exactly one test. |
| `--define SYMBOL` | Adds a preprocessor symbol (`#if SYMBOL`) for the APPs. Repeatable. `--preprocessor-symbols A,B` adds several. v3 defines its own predefined symbols (ADR-015): they are not v2's list. v2 defined `CLEANSCHEMA1` to `CLEANSCHEMA25` for every app; the AL compiler defines none of them, and neither does v3. A tool that relied on them passes them with `--define` (or the app lists them in `preprocessorSymbols`). |
| `--package-cache DIR` | Same. Repeatable. |
| `--test-timeout N`, `--test-timeout-secs N` | Seconds. Differences: v3 counts **CPU time** of the test (the load of the machine does not count) and a test that hangs without computing is stopped at 10 times N of wall time. Default 20 seconds. `0` means no limit. |
| `--auto-provision` | Accepted and ignored. v3 always fetches Microsoft's apps from the artifact feed unless `--no-download`. |
| `--coverage` | Counts the statements of the APPs (not of dependency apps) per test. Writes Cobertura XML to `coverage.cobertura.xml` in the working directory unless `--coverage-out` names a file. |
| `--coverage-out FILE` | Cobertura XML to FILE; implies `--coverage`. Section 4a. |
| `--coverage-json FILE` | New in v3: the statements and what each test ran, as JSON; implies `--coverage`. Section 4a. |
| `--tdd` | A test that calls a procedure that does not exist yet runs against a stub (ADR-051). Differences from v2: v3 stubs only calls on a codeunit variable so far (no fields, enum values, overloads or objects yet); Text and Code return types are allowed (the stub returns `''`); a bare-statement call is allowed; a test is stubbed only when a stub **ran** while it ran (v2 decided it from the call graph, so a stub in a branch that did not run flagged the test in v2 and does not in v3); a site whose return type its use does not tell is refused (its test is not run with a `--tdd: cannot infer ...` reason) instead of failing the test codeunit's compile. Without `--tdd` nothing changes. In `serve`: the `tdd` field of `runTests`, and `serve --tdd` (section 4b). |
| `--affected`, `--include-failing`, `--strict-environment`, `--per-suite`, `--watch` | Refused with exit 2 and the reason (deferred). Do not send them. |
| any other v2 flag | `unknown option`, exit 2. `al-runner test --help` is the list. |

Selection runs whole codeunits: tests of one codeunit share state, so `al-runner` runs the codeunit
of every selected test and reports only the selected tests. A tool that selects one test and
excludes its siblings sees the same rows as in v2; it does not save the time of the siblings.
With `--run-selected-only` (in `serve`: `"runSelectedOnly": true`) only the selected tests run, as in
v2 and as BC's test tool runs only the test methods that are enabled: the codeunit's `OnRun` runs,
then the selected tests in declaration order. Globals, rows, time limits and refusals of the other
tests then do not reach them.

## 4. The `--output-json` document

v2's shape, pretty printed (the document starts with `{` in column 0, so the rule "parse from the
last `{` in column 0 to the end" still works). Null and empty fields are left out.

```json
{
  "tests": [
    { "name": "Codeunit50100.Passes", "codeunit": 50100, "codeunitName": "Mixed Tests", "method": "Passes",
      "status": "pass", "durationMs": 12 },
    { "name": "Codeunit50100.Fails", "status": "fail", "message": "boom 5",
      "stackTrace": "\"Mixed Tests\"(CodeUnit 50100).Fails line 3", "durationMs": 4 },
    { "name": "Codeunit50100.Refused", "status": "error", "notRun": true,
      "message": "NotRun: out-of-scope: <reason>" }
  ],
  "passed": 1, "failed": 1, "errors": 1, "skipped": 0, "notRun": 1, "total": 3,
  "exitCode": 1, "wallSeconds": 1.234
}
```

| Field | v2 | v3 | Change for a tool |
|---|---|---|---|
| `tests[].name` | `Codeunit<ID>.<Method>` | same | none |
| `tests[].status` | `pass`, `fail`, `error`, `skipped` | `pass`, `fail`, `error` | `fail` is **every** failing test. v2 used `error` for an unexpected .NET exception; v3 has no such split. `skipped` is never sent. A tool that counts `error` as "the runner broke" must look at `notRun` first. |
| `tests[].message` | text with a .NET prefix such as `NavNCLDialogException: ` | the BC message only (ADR-009) | Do not match .NET type prefixes. Match the message text. |
| `tests[].stackTrace` | .NET stack | the AL call stack, one frame per line, innermost first | Show it as text. |
| `tests[].durationMs` | present | present when the test started (also on a test refused while it ran); absent on a test refused before it started | none |
| `tests[].notRun` | absent | `true` on a test that did not run | New. v3 refuses what it cannot answer instead of running it (hard rule 4). Count these separately. |
| not-run tests | no such thing in v2 | a row with `status: "error"`, `notRun: true`, `message: "NotRun: out-of-scope: <reason>"` | The prefix `<Type>: out-of-scope: ` is the form LethAL reads as "the runner refused" (not a killed mutant). Treat it as "unknown", not "killed". |
| timed-out test | row with `Test exceeded <N>s timeout.` | the same sentence, `status: "error"`, `notRun: true` | none |
| `passed`, `failed`, `total`, `wallSeconds` | present | present | none |
| `errors` | tests that errored | **the tests not run** (the same number as `notRun`) | Read `notRun` for the meaning. |
| `skipped` | present | always 0 | none |
| `seed` | present | absent | v3 has no random seed in the document. |
| `exitCode` | present | present | See section 5. |
| `warnings` | absent | only when the selection matches no test (exit 6): the document has no rows, all counts 0, and `warnings` says why, as the `serve` summary does | none: v2 wrote the same empty document |
| `compilationErrors`, `executionErrors`, `suiteErrors`, `companyInitFailures` | present when non-empty | absent | v3 has no compile stage: what it cannot compile becomes `notRun` rows. A tool that treats "compile failed" as exit 3 must treat all-rows-`notRun` instead (see LethAL, below). |
| `codeunit`, `codeunitName`, `method` | absent | present | New. Use them instead of splitting `name`. |
| `tests[].generatedStubs` | with `--tdd`, on a test that reaches a stub | the same, only on a test that **ran** a stub; plus `tests[].stubbed: true` | The form is v2's: `"<Object>: procedure \"<Name>\"(Arg1: <Type>, ...): <Type>"`. `status` is the test's own outcome; a stubbed pass is `pass`. |
| `stubbed`, `generatedStubs` (summary) | absent | with `--tdd` only: `stubbed: {"passed", "failed", "error"}` counts the stubbed tests per outcome, `generatedStubs` lists every stub of the program | New. The exit code follows the outcomes, as in v2: a stubbed pass counts as a pass. |

When the run cannot start (a bad flag, an unreadable app, a missing dependency that stops the
load), no document is written, the message is on stderr and the exit code is 2.

## 4a. Coverage

`--coverage-out FILE` writes Cobertura XML:

- `<coverage ... lines-valid lines-covered>`, `<sources><source>` the working directory, one `<package>` per app, one
  `<class name=<file name> filename=<path>>` per source file, and `<lines><line number="N" hits="M" branch="false" />`.
- `filename` is relative to the working directory when the file lies below it, else absolute; always forward slashes. A
  file inside a package (`.app`, `*.Source.zip`) is `<package path>!/<path inside>`. Line numbers are lines of the file as
  it is on disk (v2 numbered the instrumented text; v3 does not instrument).
- A source line has one entry when a statement starts on it. `hits` is the sum of the runs of the statements that
  start on that line, over the selected tests. A line with no run has `hits="0"` (v2 left statements nothing reached out
  in some cases; a tool that scores "no coverage" from a missing line must also treat `hits="0"` that way).
- There is no `<methods>` content, no branch data (`branch-rate="0"`).
- Dependency apps' code is not in the file, only the APPs given on the command line.

`--coverage-json FILE` writes `{ "coverage": [...], "perTestCoverage": [...] }` in the shapes of the v2 server summary:

```json
{ "coverage": [ { "file": "/abs/path/src/T.al",
                  "statements": [ { "id": 17, "scope": "TakesTheThenBranch", "line": 13, "column": 13, "endLine": 13, "hits": 1 } ] } ],
  "perTestCoverage": [ { "test": "Codeunit50500.TakesTheThenBranch",
                         "coverage": [ { "file": "/abs/path/src/T.al", "statements": [ { "id": 17, "hits": 1, "..." : "..." } ] } ] } ] }
```

- `coverage` has every statement of the APPs with its runs (0 when nothing ran it). With `--codeunit` or `--test`, the test codeunits
  left out are not compiled, so their statements are missing; the other objects of the APPs are listed. `perTestCoverage` has, for each test
  that ran, only the statements it ran, with its own `hits`. A test refused while it ran has an entry with the statements it ran
  before; a test refused before it started has none.
- `id` is stable inside one run only. `scope` is the procedure or trigger name. `column` is a byte column, from 1.
- v2 also sent `endColumn`; v3 does not know it.
- Selected tests only: a codeunit runs whole, but the tests that `--test` and `--exclude-test` leave out do not count.
- What the test ran includes the code its helpers and events ran, in the APPs.

Cost: with coverage off the VM pays one `Option` check per statement. With it on, one counter bump per statement run.

## 4b. `al-runner serve`

Same wire as v2 (`protocolVersion` 2): one JSON object per line on stdin, answers on stdout, logs on stderr; first
stdout line `{"ready":true}`; command names are case insensitive; `[server] request <n> done` on stderr after each
request except `cancel` and a line that is not a JSON object. Each request also prints the human `Tests:` and `Result:`
lines on stderr. Start options are the request-independent options of `al-runner test`
(`--package-cache`, `--dep`, `--define`, `--preprocessor-symbols`, `--test-timeout`, `--bc-version`, `--cache-dir`,
`--no-download`, `--test-data`, `--no-test-data`, `--allow-not-run`, `--server-url`, `--environment` and the like);
`--protocol 2` is accepted. An APP and the options that select tests or write reports (`--filter`, `--test`,
`--codeunit`, `--test-exact`, `--exclude-test`, `--isolation`, `--junit`, `--output-json`, `--coverage*`, `--explain`)
are an error at start-up (exit 2). So are `--recipe microsoft` and `--impact-of`: a request runs the tests once, the
plain way, and these make a run of another kind.

| Command | v3 |
|---|---|
| `runTests` | Fields `sourcePaths` (required), `packagePaths`, `coverage`, `perTestCoverage`, `test` (matched like `--test`, section 3: `Codeunit50100.` with the dot selects a codeunit), `excludeTests`, `testIsolation` (`codeunit`, `test`, `disabled`; the aliases of `--isolation` are refused), `serverUrl` (as `--server-url`), `announceStart` (one `{"type":"started","codeunit":<id>,"test":"<method>"}` line when the first test starts; ADR-028). `tdd` (true or false): as `--tdd` for this request; a server started with `serve --tdd` stubs by default and `"tdd": false` turns it off for one request, as in v2. A stubbed `test` line has `stubbed: true` and `generatedStubs`, and the summary `stubbed` and `generatedStubs` (as `--output-json`, section 4; ADR-051). `tdd` is part of the kept compile's key: switching it compiles again. `runSelectedOnly` (true or false): as `--run-selected-only` for this request. `affectedOnly`, `includeFailing`, `strictEnvironment`: `{"error": ...}` (deferred). `captureValues`, `iterationTracking`: `capturedValues` and `loops` on each `test` line (ADR-030; v2 has them for `execute` only). `code`: accepted, with a `warnings` entry in the summary. Any other field: `{"error": "unknown field ..."}`. |
| `execute` | Runs the `OnRun` trigger of one codeunit per app (`sourcePaths`) or of inline `code`; one answer line without `type`: `{"exitCode","tests":[{"name","status","durationMs","message","stackTrace","stackFrames","messages"}],"warnings","coverage","perTestCoverage"}`. `captureValues` and `iterationTracking` give `capturedValues` and `loops` on each test and, with `iterationTracking`, the tagged `messages` on the response (ADR-030; `runTests` takes both too: they are on the `test` lines). Names: `Codeunit<ID>.OnRun`, for `code` `<object name>.OnRun`. `code` and `sourcePaths` together are an error. `testIsolation` and the `runTests`-only fields (`tdd` among them) are accepted with a `warnings` entry; `serve --tdd` has no effect on `execute`. An app with no codeunit that has an `OnRun` trigger answers `exitCode` 6, `tests: []` and a warning. Details in ADR-028, item 9. |
| `cancel` | Answered at once: `{"type":"ack","command":"cancel","noop":bool}`. During a run it stops **after the current test**: the tests that ran were already streamed, and the summary has `cancelled: true`, `exitCode: 130`, as in v2. A run that `shutdown` or the end of input stops has exit code 130 too. |
| `shutdown` | `{"status":"shutting down"}`, then exit 0. During a run it stops the run after the current test like `cancel`: the summary (`cancelled: true`) comes first, then `{"status":"shutting down"}`. EOF on stdin stops a run the same way and exits 0. |

Differences a client must adapt to:

1. **Test lines stream**, one per test as it finishes, then the summary.
2. The server is **warm and in-process**: compiled dependency apps and test data stay in memory. Same sources and the
   same selected test codeunits as the last request: the compiled workspace is reused and `cached` is `true` (a request
   that selects another codeunit compiles again and has `cached: false`). A changed source always shows. When only the
   code of routines of codeunits, tables or table extensions changed, just those objects are compiled again and the
   summary has `incremental: true` (ADR-029; a request takes 0.4 s on the corpus app and 2.2 s on Tests-ERM, of which
   1.8 s is the tests); any other change compiles the workspace again. Every request starts from the same post-install
   database, so nothing leaks between requests. The first request after a new build of `al-runner` builds the
   dependency cache (a minute with Microsoft's apps, then 3 GB resident); with the cache on disk it takes as long as a
   one-shot run. Repeats take 0.2 to 2 s.
   A panic in a test is reported as an error for that test and the server goes on.
3. `test` lines have `type`, `name`, `status` (`pass`, `fail`, `error`), `durationMs`, `message`, `stackTrace`,
   `stackFrames` (the AL stack split into lines), and `notRun: true` for a test that did not run (status `error`, message
   `NotRun: out-of-scope: <reason>`; see section 4). `errorKind`, `alSourceFile`, `alSourceLine` are not sent.
4. The `summary` has `exitCode` (the exit code of `al-runner test`, section 5), `passed`, `failed`, `errors` (= not run),
   `notRun`, `total`, `cached`, `incremental` when only some objects were compiled again, `protocolVersion`, `wallSeconds`,
   `warnings` when there are some, `cancelled` when cancelled, and `coverage` / `perTestCoverage` when asked for (section 4a shapes). `compilationErrors`,
   `changedFiles`, `selection`, `companyInitFailures`, `sourceScanFailures` are not sent.
5. A request that cannot run answers one `{"error": "<reason>"}` line instead of a summary with exit code 2 or 3:
   for example `{"error":"/x: no such file or folder"}`, ``{"error":"unknown field `cobertura` for runTests"}``,
   ``{"error":"`sourcePaths` needs at least one app folder"}``. A `test` or `excludeTests` that matches no test is not
   an error: it answers a summary with exit code 6, all counts 0, and the reason in `warnings`.
6. While a run is in progress `cancel` is answered at once. `shutdown` takes effect after the current test (see the
   table). Other requests are queued and run in order, as in v2.
7. `testIsolation: "test"` means TestIsolation Function (section 3), `"disabled"` runs nothing.

## 5. Exit codes

| Code | v2 | v3 |
|---|---|---|
| 0 | all passed | all selected tests passed |
| 1 | a test failed or errored | a test failed |
| 2 | bundle could not execute, bad flag | bad invocation or the run could not start |
| 3 | bundle could not compile, or suite error | **never**. v3 has no compile stage. |
| 6 | `--test` matched nothing | the selection matched nothing |
| 7 | (none) | some tests were not run. `--allow-not-run` makes it 0. |
| 8 | (none) | tests were not run because the .NET helper could not be used (a problem of the machine). |

Rank when several apply: 2, 6, 1, 8, 7. A tool that reads rows only on exit 0 and 1 must also read
rows on 7 (and 8); otherwise every run with a refused test hides all its rows.

## 6. Environment variables

| v2 | v3 | Change |
|---|---|---|
| `AL_RUNNER_TEST_TIMEOUT_SEC` | `ALRUNNER_TEST_TIMEOUT` (and the v2 name, still read) | `--test-timeout-secs` wins over both. A value that is not a number is an error (exit 2). |
| `AL_RUNNER_VERBOSE` | not read | Use `--verbose`. |
| `ALCHEMIST_RUNNER_PATH` | n/a (the tool's own) | none |

## 7. Output on stderr and stdout

v2 printed machine-readable and human lines mixed (`[bc] selected BC 27.5`, `al-runner ... BC ... N apps`,
`[dep] ...`, `[provision] ...`, `[pkg-cache] ...`). v3 prints none of them. By default it prints the
version line (`al-runner <version> (<commit>)`), the failures, the top refusals, the `Tests:` line and the
`Result:` line. The setup lines (`target: BC <v>, runtime <r>, Microsoft apps from build <b>`,
`build chosen by: ...`, `dependencies: N apps`) are printed only with `--verbose`. With `--output-json` and no
file, all of it goes to stderr.

Change for a tool that read the `[bc]` or `[dep]` lines: treat the BC build and the
platform-apps pin as "unknown", or add `--verbose` and parse `target: BC <v>, ... build <b>` from stderr.

## 8. LethAL (one-shot mode, `serverMode: false`)

Source: `docs/integrations/alchemist-lethal.md`, section 3. The command LethAL builds
(`al-runner-transport.ts:641-741`) maps to v3 with one rename:

```
al-runner test --output-json --isolation test --test Codeunit50011.Foo [--exclude-test Codeunit50011.Foo2]...
  [--auto-provision] <sourceDir> [<testDir>] [--package-cache <dir>]... [--define <SYM>]...
```

Changes in LethAL:

1. **Binary**: command name `al-runner`, first argument `test`. Config key `alRunner.alRunnerPath` stays.
2. **Version check** (`al-runner-backend.ts:165, 1092-1115`; `al-runner-contract.ts:439-452`): replace
   `/\bv2\.\d/` with a check that stdout starts with `al-runner ` and carries `\d+\.\d+\.\d+`.
3. **Exit codes** (`al-runner-transport.ts:576-625, 821-845`): read the rows on exit 7 and 8 as on 1.
   Exit 3 never happens.
4. **Not-run rows** (`al-runner-backend.ts:97`): keep recognizing `<Type>: out-of-scope: ...`; the v3 message is
   `NotRun: out-of-scope: <reason>`. Also read `notRun: true`. A row with `notRun` is "runner refused", never "killed".
5. **Timeouts** (`al-runner-backend.ts:79-93`): unchanged sentence. The limit is CPU time in v3.
6. **Compile failure** (`al-runner-contract.ts` fact `compile-failure-not-scorable`): v3 has no compile
   exit code. A mutant that does not compile gives rows with `notRun: true`. Score that as "not scorable".
7. **Assertion screen** (`assertion-screen.ts:36-44, 78-90`): do not depend on `NavNCLDialogException: ` or
   `InvalidOperationException: out-of-scope:`. Messages are the BC text.
8. **Stderr** (`al-runner-transport.ts:106-210`): see section 7.
9. **Coverage**: `"coverage": "al-runner"` works with `--coverage --coverage-out FILE` (section 4a). LethAL reads
   `<class ... filename=...>` and `<line number=... hits=...>` by regex: both are as it expects. Labels are relative to the
   working directory of the process. Per-test coverage (`--coverage-json`) is new; use it instead of one run per test.
10. **Selector**: `"selectorMode": "static"`. Server mode: not before step 3.
11. **`--auto-provision`** may stay in the command; it is accepted.

Check: run LethAL's contract probe against `al-runner`; the six facts (version, unknown flag rejected with a
non-zero exit chosen by the process (2), qualified test name, timeout classified, timeout exit readable,
compile failure not scorable) must hold with the changes above.

## 9. ALchemist

ALchemist needs `al-runner serve` (step 3, done), `execute` (step 4), value capture (step 5) and `al-runner debug`
(step 6). All four exist; scratch runs, inline values and debugging need the changes below.

1. Binary lookup (`alRunnerManager.ts:47-51, 129-139`): also `which al-runner`; spawn `al-runner serve` instead of
   `al-runner --server` (`serverProcess.ts:125-141`). The version check only warns and accepts `3.0.0` (A3).
2. `runtests` (lowercase) is accepted. ALchemist sends `captureValues` and `iterationTracking` on `runTests`: v3 puts
   `capturedValues` and `loops` on the `test` lines (v2 ignored them there). It never sends `cobertura` or `testFilter`; both would be
   refused as unknown fields. Section 4b lists the other differences (warm state, streaming, queued requests).
3. `coverage` arrives in the summary in the v2 shape (section 4a). ALchemist reads `hitStatements` and `totalStatements`,
   which neither v2 nor v3 sends (a tool-side issue noted in `alchemist-lethal.md`, section 6).
4. Test names are `Codeunit<ID>.<Method>` as in v2; the label match in `testController.ts:493` (A8) must be checked on a run.
5. Debugger: start `al-runner debug --stdio APP...` instead of `al-runner --dap stdio DIR` (step 6). Requests, capabilities,
   events and breakpoint answers are v2's. Differences: `pause` stops at the next statement (v2 continued);
   `setExceptionBreakpoints` succeeds with no filters; frame names are `Object.Routine`; a value that is not rendered yet
   shows `<not rendered: KIND>`; `--test` and `--codeunit` select tests (v2 ran all); a file of a dependency read from a
   source package is shown from `<cache>/debug-sources/` (`docs/debugging.md`).

### A note for ALchemist's author

The debugger needs one change in ALchemist: the arguments of the `DebugAdapterExecutable`
(`debugAdapterFactory.ts:56-98`). Pass `['debug', '--stdio', ...sourcePaths]` instead of
`['--dap', 'stdio', ...sourcePaths]`, with `--dep` for a dependency folder whose code should be debugged and the
launch configuration's `testFilter` as `--test`. Nothing else in the DAP code has to change: requests, capabilities,
events and breakpoint answers are v2's. ALchemist's text that says "stepping acts as continue"
(`debugLaunch.ts:61-62`) is out of date: `next`, `stepIn` and `stepOut` step. `docs/debugging.md`, section "Using the
debugger from ALchemist", has the command line and a table of what differs from v2. AL Runner ships no VS Code
extension of its own (ADR-048, owner decision 3).

## 10. What does not change for a tool

The names of the tests (`Codeunit<ID>.<Method>`), the meaning of pass and fail, `durationMs`, the
JUnit file (`--junit`), the position of positional app folders, `--package-cache`.
