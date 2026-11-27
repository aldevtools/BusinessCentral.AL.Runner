---
title: "Tool protocol"
weight: 92
---

This page is for the authors of tools that call AL Runner, such as ALchemist and LethAL. It
describes the command line, the `--output-json` document, the coverage files, the `serve`
protocol and the exit codes. For each part it says what differs from AL Runner version 2.

How to use this page: find your tool's section, apply each "Change", then run the check at
the end of the section.

## Why move to version 3

Version 3 reproduces the observable behavior of the AL language, which Microsoft keeps stable
for partners. Version 2 depends on Business Central's internals, which can change with any
hotfix or release, so a tool built on version 2 can break without a change on its own side.

## 1. The command

| v2 | v3 | Change |
|---|---|---|
| binary `al-runner` | binary `al-runner` (called `alrunner` up to 3.0.0-alpha.2) | Run `al-runner test ...`. v2's dotnet tool has the same name, so a tool that finds the binary by name on the PATH may start v2. Tell them apart by `--version` (section 2), or use a configured path. v3 prints a note on stderr when it finds another copy; `ALRUNNER_NO_PATH_NOTE=1` turns it off. |
| `al-runner [FLAGS] DIR [DIR]` | `al-runner test [FLAGS] APP...` | Put the word `test` first. APP is an app folder (with `app.json`), a `*.Source.zip` or an `.app` that carries source. |
| install: `dotnet tool install -g msdyn365bc.al.runner` | one binary | Do not run `dotnet tool`. The user sets the path. v3 needs no .NET (an optional helper exists for `DotNet` variables). |
| `al-runner --server [--package-cache P]... [--define S]... [--test-timeout N]` | `al-runner serve [--package-cache P]... [--define S]... [--test-timeout N]` | Same options (section 4b). |
| `al-runner --dap stdio DIR` | `al-runner debug --stdio APP...` | Same DAP session (see [Debugging](debugging.md)). |
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
| `--isolation codeunit\|test\|disabled` (also `--test-isolation`) | `codeunit`: the database is rolled back when a test codeunit ends (the default, as in Microsoft's test runner). `test` (aliases `function`, `method`): the database goes back to its state after `OnRun` after every test method, **the codeunit's AL variables are kept** (BC's `TestIsolation = Function`). v2's `test` also reset the codeunit's state; a tool that relied on that must not. `disabled`: nothing is rolled back but a failed test's writes since its last `Commit()`; the test codeunits run in id order and see what the earlier ones left. Under `test` and `disabled` a codeunit's `RequiredTestIsolation` has no effect. `required` (not in v2): each test codeunit runs under its own `RequiredTestIsolation`, as BC 28 and later runs it without a test runner codeunit; on BC 27 as `codeunit`. Use it for suites that set `RequiredTestIsolation = Function` or `Disabled`. |
| `--test TEXT` (also `--filter`) | Case-insensitive substring. TEXT with a dot, such as `Codeunit50011.Foo`, is matched against the qualified name `Codeunit<ID>.<Method>`; TEXT without a dot against the method name. v2 matched the qualified name for both. A bare name that is also a part of the word `Codeunit` is not special. |
| `--test-exact NAME` | New in v3 for the tools: the test whose method name or qualified name is exactly NAME (any case). Repeatable. Given with `--test`, it narrows: only the tests that both match are selected, as in v2. A name is the AL name as written, without quotes: `--test-exact "Codeunit50650.Space Step"`. |
| `--exclude-test TEXT` | Removes the tests TEXT matches (the same matching as `--test`: a substring, any case). A whole qualified name also removes every test whose name contains it (`Codeunit50100.Foo` removes `Codeunit50100.Foo2` too). Wins over every selecting option. Repeatable. |
| `--exclude-test-exact NAME` | Removes the test whose method name or qualified name is exactly NAME (any case), as `--test-exact` selects it. A part of a name matches nothing. Wins over every selecting option. Repeatable. |
| `--define SYMBOL` | Adds a preprocessor symbol (`#if SYMBOL`) for the APPs. Repeatable. `--preprocessor-symbols A,B` adds several. v3 predefines no symbol. v2 defined `CLEANSCHEMA1` to `CLEANSCHEMA25` for every app; the AL compiler defines none of them, and neither does v3. A tool that relied on them passes them with `--define` (or the app lists them in `preprocessorSymbols`). |
| `--package-cache DIR` | Same. Repeatable. |
| `--test-timeout N`, `--test-timeout-secs N` | Seconds. In `serve`: a start option, and the `testTimeoutSecs` field of `runTests` and of an entry sets it for one request or entry. Differences: v3 counts **CPU time** of the test (the load of the machine does not count) and a test that hangs without computing is stopped at 10 times N of wall time. `Sleep` does not wait, but the time it asks for, summed over the test, counts toward N; a test whose Sleep calls go past N gets the same timeout row. Default 20 seconds. `0` means no limit. A test codeunit's `OnRun` trigger has a limit of its own, counted the same way; its time counts toward no test method. An `OnRun` that goes past it is stopped, and every test of the codeunit gets the timeout row (the full reason in `--junit` and `--explain` names the trigger). |
| `--skip-after-timeout` | Not in v2. Once a test is stopped at the time limit, the later tests of its codeunit are not run and end as not run, with a reason that names the stopped test. Default: they run. In `serve`: a start option, and the `skipAfterTimeout` field of `runTests` (true or false) sets it for one request. The summary and each entry have `notRunSeconds`, the time the tests that ended not run took. |
| `--stop-after-first-failure` | Not in v2. Once a test fails, the later tests of its codeunit are not run, and in `serve` neither are the later entries of the request: their tests end as not run (`notRun: true`), with a message that names the failed test and `stopAfterFirstFailure`, so a client can tell them from a refusal. Each test codeunit stops on its own at its first failing test. Every other test codeunit of the same entry, or of a request without `entries`, still runs, also with one test thread, and reports. A selection then runs only the tests it picks, as `--run-selected-only`, so a test the answer does not list cannot stop the run. A failure is a `fail` row; a timeout or a refusal does not stop the run. In `serve`, entries may carry a `stopGroup` that limits which later entries a failure skips (see the `entries` field of `runTests`). Default: everything runs. In `serve`: a start option, and the `stopAfterFirstFailure` field of `runTests` (true or false) sets it for one request. |
| `--stop-after-timeout` | Not in v2. As `--stop-after-first-failure`, for a timeout instead of a failure: once the time limit stops a test (its CPU time, the wall backstop, its Sleep calls, or the test codeunit's `OnRun`), the later tests of its codeunit are not run, and in `serve` neither are the later entries of the request (with `stopGroup`s, of its group; one scope, shared with `stopAfterFirstFailure`). Their tests end as not run (`notRun: true`), with a message that names the stopped test (`Codeunit<id>.OnRun` for an `OnRun`) and `stopAfterTimeout`. The timeout row itself stays `notRun` with its `clock`. A failure, a refusal, a test a stop left out, the memory limit or `cancel` does not trigger it. With `--stop-after-first-failure` too, either stops, and the first row that does is named. With `--skip-after-timeout` too, it includes that option, and the rest of the codeunit carries the one stop text; `--skip-after-timeout` alone is unchanged. The server is kept: the stop only leaves out tests. Default: everything runs. In `serve`: a start option, and the `stopAfterTimeout` field of `runTests` (true or false) sets it for one request. |
| `--auto-provision` | Accepted and ignored. v3 always fetches Microsoft's apps from the artifact feed unless `--no-download`. |
| `--coverage` | Counts the statements of the APPs (not of dependency apps) per test. Writes Cobertura XML to `coverage.cobertura.xml` in the working directory unless `--coverage-out` names a file. |
| `--coverage-out FILE` | Cobertura XML to FILE; implies `--coverage`. Section 4a. |
| `--coverage-json FILE` | New in v3: the statements and what each test ran, as JSON; implies `--coverage`. Section 4a. |
| `--profile PATH`, `--profile-mode MODE`, `--profile-interval US` | Not in v2. AL CPU profiles (`.alcpuprofile`): a folder gets one file per test codeunit and `summary.txt`, a PATH ending in `.alcpuprofile` one merged file. The format is BC's (V8 CPU profile contract); the runner adds an `alRunner` block. See [Profiling AL code](profiling.md). In `serve`: the `profile` field of `runTests`. |
| `--allow-write-in-try-functions` | Not in v2. Runs as a service tier whose setting DisableWriteInsideTryFunctions is off (Business Central online); without it a write inside a TryFunction whose result is used raises TryFunctionWrite, as the setting's default does. With it, such a write runs and is kept, also when an error after it makes the TryFunction return false. In `serve`: the `allowWriteInTryFunctions` field of `runTests`. |
| `--strict-order` | Not in v2 (v2 has no order dependence across test codeunits). Runs the test codeunits in Business Central's order: each Business Central session runs its test codeunits one after the other, and SingleInstance state, rows and NumberSequence numbers carry from codeunit to codeunit; nothing runs twice (ADR-070). Without it (ADR-066 amendment 12) every test codeunit starts with fresh SingleInstance state and the install code's NumberSequence values, nothing runs again, a failed test that read state an earlier codeunit would have set gets `orderHint` (text) on its `--output-json` row (in `serve`: the summary's `orderHints`, `[{"name", "orderHint"}]`, since the `test` lines stream before the run's order analysis), and codeunits that take numbers after another one get the JUnit property `numberSequenceOrder`. In `serve`: the `strictOrder` field of `runTests`. |
| `--test-suites FILE` | Not in v2. Each test suite in FILE is a Business Central session of its own for order dependence, as on a pipeline that runs one test-tool run per suite (ADR-066 amendment 6). In `serve`: the `testSuites` field of `runTests`. |
| `--max-memory SIZE` | Not in v2. The most memory the run may use, a hard limit: test threads start only while there is room for another codeunit, and a run that cannot fit stops before it passes the limit (exit code 2; the tests not reached are not run, with the reason `memory limit (--max-memory X MB)`). Without it the limit is the free memory or the cgroup's `memory.max`. In `serve`: the `maxMemory` field of `runTests`, or `serve --max-memory` for every request. |
| `--dependencies-published-global` | Not in v2. "Published As" of "NAV App Installed App" is Global, not Dev, for a dependency app of a publisher other than Microsoft (a pipeline that publishes dependencies as compiled apps). A setting, not measured: such a pass is flagged `dependency-published-as-global`. In `serve`: the `dependenciesPublishedGlobal` field of `runTests`. |
| `--dev-endpoint-ui` | Not in v2. A UI call that no handler takes gets the answer of the BC 28 and 29 dev-endpoint test runner, for the measured shapes; see [Writing tests](writing-tests.md#ui-calls-without-a-handler). Each answered call is a `warning:` line on stderr, and the test is flagged (`dev-endpoint-ui`). In `serve`: the `devEndpointUi` field of `runTests`. |
| `--no-client` | Not in v2. The test session is a web-service session, as a user measured it on Business Central 28.4: `GuiAllowed()` false on every version; on version 28 only, in a test method, the test's handlers answer the dialogs, an unhandled `Message` or `Hyperlink` fails with `Unhandled UI` and `CurrentClientType()` answers ODataV4, and the measured processing-only report skips its request page. A `TestPage`, `Report.RunRequestPage`, a Session or Active Session table read, the new answers on 27 and 29 and the calls not measured there are not run. Passes through these answers are flagged (`no-client-session`). See [CLI reference](cli-reference.md). |
| `--tdd` | A test that calls a procedure that does not exist yet runs against a stub. Differences from v2: a stub field of type Text or Code is declared with length 2048 and a longer write is refused (`--tdd: the length of the stub field F is unknown`); stub objects take the first free id of the test app's `idRanges`, as in v2; Text and Code return types are allowed (the stub returns `''`); a bare-statement call is allowed; a test is stubbed only when a stub **ran** while it ran (v2 decided it from the call graph, so a stub in a branch that did not run flagged the test in v2 and does not in v3); a site whose return type its use does not tell is refused (its test is not run with a `--tdd: cannot infer ...` reason) instead of failing the test codeunit's compile. Without `--tdd` nothing changes. In `serve`: the `tdd` field of `runTests`, and `serve --tdd` (section 4b). |
| `--affected` | Runs only the test codeunits a change since the last `--affected` run can affect. Differences from v2: no `--watch` needed (each command-line run compares with the stored result of the last one); the stored result is `<cache dir>/affected/<key>.json.gz`, keyed by the ids and versions of the APPs, or `--affected-store FILE` (save and restore it in CI); a change is found per object from its compiled form (an edit that moves no line, such as a comment at the end of a line, changes nothing); always whole test codeunits; an added or removed object, an `app.json` change, another dependency app or BC build, another run option, changed install code or a changed object of a kind the trace does not follow runs everything. The text has `affected: ran X of N   skipped-unaffected A   skipped-failing B`, then `changed: ...` or `full run: <reason>`, then `not re-run, still failing from an earlier run: ...`; the failed count reads `0 failed (+1 still failing, not re-run)`. `--output-json` adds the summary object `selection` (`mode`, `ran`, `skipped`, `skippedFailing`, `changedObjects`, `changedObjectsTruncated` past 50, `forcedFull`, `reason`); `tests[]` holds only the tests that ran. `--junit` lists a skipped test as `<skipped message="not affected by the change; stored outcome: pass">`. The exit code is 1 when a skipped test failed last time. Refused with `--test`, `--test-exact`, `--codeunit`, `--exclude-test`, `--exclude-test-exact` (exit 2). |
| `--affected-store FILE` | New in v3: where `--affected` reads and writes its stored result. Needs `--affected`. |
| `--include-failing` | With `--affected`: also runs every codeunit with a test that did not pass last time. Without `--affected`: exit 2, as v2. |
| `--strict-environment` | With `--affected`: accepted and changes nothing; the strict behavior is always on (another dependency app or BC build runs everything; v3 never diffs two environments). Without `--affected`: exit 2, as v2. |
| `--per-suite`, `--watch` | Refused with exit 2 and the reason (deferred). Do not send them. |
| any other v2 flag | `unknown option`, exit 2. `al-runner test --help` is the list. |

Selection runs whole codeunits (in `serve`, a `test` or an entry that picks exactly one test runs that test alone; a difference from v2, the CLI `--test` is unchanged): tests of one codeunit share state, so `al-runner` runs the codeunit
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
| `tests[].message` | text with a .NET prefix such as `NavNCLDialogException: ` | the BC message only | Do not match .NET type prefixes. Match the message text. |
| `tests[].stackTrace` | .NET stack | the AL call stack, one frame per line, innermost first | Show it as text. |
| `tests[].durationMs` | present | present when the test started (also on a test refused while it ran); absent on a test refused before it started | none |
| `tests[].sleptMs` | absent | the time the test's `Sleep` calls asked for, in milliseconds, summed; absent on a test that did not sleep | New. `Sleep` returns at once in v3, so `durationMs` does not hold this time; the sum counts toward the time limit. Add it to `durationMs` for the time the test would have taken on BC. |
| `tests[].flagged` | absent | `true` on a pass that ran through an unprobed assumption (see `--list-assumptions`); absent otherwise | New. Count a flagged pass apart from a clean pass. |
| `tests[].notRun` | absent | `true` on a test that did not run | New. v3 refuses what it cannot answer instead of running it. Count these separately. |
| not-run tests | no such thing in v2 | a row with `status: "error"`, `notRun: true`, `message: "NotRun: out-of-scope: <reason>"` | The prefix `<Type>: out-of-scope: ` is the form LethAL reads as "the runner refused" (not a killed mutant). Treat it as "unknown", not "killed". |
| timed-out test | row with `Test exceeded <N>s timeout.` | the same sentence, `status: "error"`, `notRun: true`, and `clock`: `cpu` when the test thread's CPU time passed the limit, `wall` when the wall clock did (the backstop at 10 times the limit for a test that does not compute, or the limit itself where the test thread's CPU time cannot be read), `virtual` when the test's `Sleep` time did | `Sleep` does not wait in v3, so a `virtual` stop takes less real time than the limit names; `durationMs` stays the real time. A `virtual` stop is not evidence that the test hangs on BC. |
| `OnRun` rows | absent | absent, on every BC version, as Microsoft's Test Runner app reports them (runner codeunit 130450, the way BcContainerHelper and AL-Go run tests; ADR-055, al-runner issue 5859). A test codeunit's `OnRun` trigger runs before its tests but has no row of its own. When `OnRun` raises an error, the codeunit is skipped: it has no rows at all, nothing in the console, JSON or JUnit names it, and the run is not failed by it. This rests on a measurement, not a corpus test: a pass of a test in a codeunit that declares `OnRun` is a flagged pass (assumption `test-codeunit-onrun-rows`). | Do not expect a row whose method is `OnRun`. A codeunit with no rows may have been skipped because its `OnRun` raised. |
| `passed`, `failed`, `total`, `wallSeconds` | present | present | none |
| `flagged` | absent | of the passes, the flagged ones | New (al-runner issue 7954). |
| `errors` | tests that errored | **the tests not run** (the same number as `notRun`) | Read `notRun` for the meaning. |
| `skipped` | present | always 0 | none |
| `skippedDependencies` | absent | always present: a list (empty when none) of the dependency apps the run did not load, each `{id, name, publisher, version, reason}`; `version` is the declared minimum. Without `--allow-missing-dependencies` and with `--no-download` such a dependency stops the run (exit 2) and the list stays empty; `serve` summaries carry the same field | New (al-runner issue 7139). A tool that compares runs across machines can refuse a run whose list is not empty. |
| `seed` | present | absent | v3 has no random seed in the document. |
| `exitCode` | present | present | See section 5. |
| `warnings` | absent | only when the selection matches no test (exit 6): the document has no rows, all counts 0, and `warnings` says why, as the `serve` summary does | none: v2 wrote the same empty document |
| `compilationErrors` | present when non-empty | present on exit 3: no rows, every count 0, `[{file, errors[]}]` as in v2 | Exit 3 covers the AL compiler errors the corpus has checked against the real compiler, and source of an app under test that AL Runner's parser does not accept (that line has the parser's message and no AL code, `<file>(<line>,<column>): error: <message>`). Other code AL Runner cannot compile becomes `notRun` rows: treat all-rows-`notRun` as "not scorable" too (see LethAL, below). |
| `executionErrors`, `suiteErrors`, `companyInitFailures` | present when non-empty | absent | none |
| `codeunit`, `codeunitName`, `method` | absent | present | New. Use them instead of splitting `name`. |
| `tests[].generatedStubs` | with `--tdd`, on a test that reaches a stub | the same, only on a test that **ran** a stub; plus `tests[].stubbed: true` | The form is v2's: `"<Object>: procedure \"<Name>\"(Arg1: <Type>, ...): <Type>"`, and for declarations `"<Table>: field \"<Name>\": <Type>"`, `"<Enum>: value \"<Name>\""` and `"<kind> \"<Name>\" (id N)"`. `status` is the test's own outcome; a stubbed pass is `pass`. |
| `tests[].uiAnswered` | absent | with `--dev-endpoint-ui`, on a test with UI calls the dev-endpoint answer took: each `<call> <shown>: <answer>`, in order; the test also has `flagged: true` | New. |
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
- `serve`'s `"coverage": "hit"` (section 4b, al-runner issue 8244): `coverage` lists only the statements with `hits` above 0, each
  as the full list shows it, and leaves out a file with none. `perTestCoverage` is the same as with `true`. Its cost grows with the
  statements the tests ran, not with the statements of the APPs.

Cost: with coverage off the VM pays one `Option` check per statement. With it on, one counter bump per statement run.

## 4b. `al-runner serve`

Same wire as v2 (`protocolVersion` 2): one JSON object per line on stdin, answers on stdout, logs on stderr; first
stdout line `{"ready":true}`; command names are case insensitive; `[server] request <n> done` on stderr after each
request except `cancel` and a line that is not a JSON object. Each request also prints the human `Tests:` and `Result:`
lines on stderr. Start options are the request-independent options of `al-runner test`
(`--package-cache`, `--dep`, `--define`, `--preprocessor-symbols`, `--test-timeout`, `--bc-version`, `--cache-dir`,
`--no-download`, `--allow-missing-dependencies`, `--test-data`, `--no-test-data`, `--allow-not-run`, `--server-url`, `--environment` and the like);
`--protocol 2` is accepted. An APP and the options that select tests or write reports (`--filter`, `--test`,
`--codeunit`, `--test-exact`, `--exclude-test`, `--exclude-test-exact`, `--isolation`, `--junit`, `--output-json`, `--coverage*`, `--explain`)
are an error at start-up (exit 2). So are `--recipe microsoft` and `--impact-of`: a request runs the tests once, the
plain way, and these make a run of another kind.

| Command | v3 |
|---|---|
| `runTests` | Fields `sourcePaths` (required), `packagePaths`, `coverage` (true, false, or `"hit"`: only the statements that ran, section 4a; another value answers `{"error": ...}`), `perTestCoverage`, `test` (one text, as in v2; matched like `--test`, section 3: `Codeunit50100.` with the dot selects a codeunit; a list or another value is an `error`, as a non-list `excludeTests` is), `excludeTests` (substring, as `--exclude-test`), `excludeTestsExact` (a list of names, as `--exclude-test-exact`; not in v2), `testIsolation` (`codeunit`, `test`, `disabled`, `required`; the aliases of `--isolation` are refused; `required` is not in v2 and is `--isolation required`), `serverUrl` (as `--server-url`), `license` (not in v2; the path of a `.bclicense` file, as `--license`; the summary has the `license` object of `--output-json`), `companies` (text; not in v2): as `--companies` for this request (company names, comma separated, the session company first, or `all`); absent or `null`, the server's `--companies`. A FindSet on the Company table is refused without it, and with `all` but no test data, as on the command line. A bad value answers `{"error": ...}`. `announceStart` (one `{"type":"started","codeunit":<id>,"test":"<method>"}` line when the first test starts). `tdd` (true or false): as `--tdd` for this request; a server started with `serve --tdd` stubs by default and `"tdd": false` turns it off for one request, as in v2. A stubbed `test` line has `stubbed: true` and `generatedStubs`, and the summary `stubbed` and `generatedStubs` (as `--output-json`, section 4). `tdd` is part of the kept compile's key: switching it compiles again. `runSelectedOnly` (true or false): as `--run-selected-only` for this request. `ignorePermissionMock` (true or false): as `--ignore-permission-mock` for this request. `devEndpointUi` (true or false; not in v2): as `--dev-endpoint-ui` for this request; a `test` line of a test with answered UI calls has `uiAnswered` (the calls and answers), and each call is a `warning:` line on stderr. `noClient` (true or false; not in v2): as `--no-client` for this request. `allowWriteInTryFunctions` (true or false; not in v2): as `--allow-write-in-try-functions` for this request. `strictOrder` (true or false; not in v2): as `--strict-order` for this request. `skipAfterTimeout` (true or false; not in v2): as `--skip-after-timeout` for this request; absent, the server's setting. `stopAfterFirstFailure` (true or false; not in v2): as `--stop-after-first-failure` for this request (each test codeunit stops on its own at its first failing test and the later entries are not run, or with a `stopGroup` on the entries only the later entries of the failing entry's group; the tests it does not run are `notRun` rows); absent, the server's setting. `stopAfterTimeout` (true or false; not in v2): as `--stop-after-timeout` for this request (a test the time limit stops ends the rest of its codeunit and the later entries, or with a `stopGroup` on the entries only the later entries of its group); absent or `null`, the server's setting. `testTimeoutSecs` (a whole number of seconds, `0` for no limit; not in v2): as `--test-timeout-secs` for this request, also for the `OnRun` trigger's limit; absent or `null`, the server's `--test-timeout` or environment variable. The timeout row names the limit that applied (`Test exceeded <N>s timeout.`). The kept compile and install baseline do not depend on it. A negative, fractional or text value answers `{"error": ...}`. `testSuites` (text; not in v2): the path of a test suites file, as `--test-suites FILE` for this request (a relative path is read from the server's working folder; an absolute path is safest); a bad line answers `{"error": ...}`.  `dependenciesPublishedGlobal` (true or false; not in v2): as `--dependencies-published-global` for this request. `testRunner` (a codeunit id as text, or `"builtin"`; not in v2): as `--test-runner` for this request; a request with another runner compiles again. `testDriver` (`"bc-linux"` or `"bccontainerhelper"`; not in v2): as `--test-driver` for this request. `permissions` (`"on"` or `"off"`): as `--permissions` for this request. `affectedOnly` (true or false): as `--affected` for this request: only the test codeunits a change since the stored result can affect run and stream `test` lines; the summary gets `selection` (`mode: "affected"`, `ran`, `skipped`, `skippedFailing`, `changedObjects`, `changedObjectsTruncated` past 50, `forcedFull`, `reason`), and `exitCode` is 1 when a skipped test failed last time. `environmentDrift` is never sent: another environment is a full run. The stored result is the one `--affected` uses, or the file of the start option `serve --affected-store FILE`; a restarted server reads it. A cancelled request leaves it as it was. With `affectedOnly`, `test`, `excludeTests` and `excludeTestsExact` are `{"error": ...}`. `includeFailing`: as `--include-failing`. `strictEnvironment`: accepted, changes nothing (always strict). `maxMemory` (text such as `"6G"`; not in v2): as `--max-memory` for this request; absent, the server's `--max-memory` applies. In `serve` a request that does not fit in it runs with one test thread instead of stopping the server. `profile` (not in v2): `{"path": ..., "mode": "instrumentation"|"sampling", "intervalUs": N}`, as `--profile PATH --profile-mode MODE --profile-interval N` for this request (`path` required; a relative path is resolved from the server's working folder). The summary gets `profile: {"files": [...]}`, the files written. A bad value answers `{"error": ...}`. `seed` (not in v2's protocol; 0 to 2147483647): the run seed of this request, as `--seed`; a request without one gets the server's `--seed` or environment seed, else a fresh one. The summary has `seed` and `seedSource` (`option`, `env` or `fresh`). The kept compile and install baseline do not depend on it. Without `affectedOnly` these two earn a `warnings` entry. `captureValues`, `iterationTracking`: `capturedValues` and `loops` on each `test` line (v2 has them for `execute` only). `code`: accepted, with a `warnings` entry in the summary. `entries` (not in v2): a list of runs over one compile and one install baseline, for example one per mutant of a mutation-testing tool. Each entry is `{"id", "resources", "test", "excludeTests", "excludeTestsExact", "testTimeoutSecs", "stopGroup"}`, all optional: `id` (text or number; default the entry's index) is copied to the entry's lines; two entries with the same id (a text and a number that read alike count as the same, and so does an explicit id equal to another entry's index) are an `error`; `resources` maps a resource file name (as `NavApp.GetResourceAsText` names it) to the text that entry's run reads in place of the file's, in every app under test that has a resource of that name (a name no app has is an `error`; the files on disk are not changed); `test`, `excludeTests` and `excludeTestsExact` select the entry's tests as the fields of the request do. `testTimeoutSecs` is the time limit of the entry's tests, as the request's field; absent or `null`, the request's limit applies. `stopGroup` (text or number; not in v2; `null` counts as absent; any other value answers `{"error": "the `stopGroup` of an entry is a text or a number"}`; a text and a number that read alike are one group, as for `id`) scopes `stopAfterFirstFailure` and `stopAfterTimeout`. If no entry has a `stopGroup`: exactly as above (the whole request). If any entry has a `stopGroup`, the stop is scoped by group: a failure skips the later tests of its codeunit and only the later entries of the same `stopGroup`; entries of other groups run. A request in which some entries have a `stopGroup` and some do not is refused: `{"error": "some entries have a `stopGroup` and some do not: give every entry one (an entry that stops alone gets a group of its own), or none (a failure then stops the whole request)"}`; under `stopAfterTimeout` alone the tail reads `a timeout`, under both `a failure or a timeout`. An opt-out for ungrouped entries has more than one plausible reading, and a refusal can be lifted later without breaking a client, while a chosen meaning cannot. The `notRun` message of a test a group's failure skipped names the group (``stopGroup `g` ``) and the failed test. `stopGroup` with both stops off (each field `false`, or absent on a server started without its option) answers `{"error": "`stopGroup` needs `stopAfterFirstFailure` or `stopAfterTimeout` (or a server started with `--stop-after-first-failure` or `--stop-after-timeout`): without a stop a group changes nothing"}`. Each entry starts from the same post-install database. Its `test` lines carry `"entry": <id>`, and after the last entry one `{"type":"entrySummary","entry":<id>,"exitCode","passed","flagged","failed","errors","notRun","total"}` line per entry, in the order of the entries, comes before the request's `summary`, whose counts are the sums. `exitCode` is the entry's own (section 5); an entry a `cancel` stopped before it ran has none. An entry whose selection matches no test has `exitCode` 6 and the reason in its `warnings`, as a whole request has; the request's `warnings` then name the entry (``entry `<id>`: the selection matches no test``) and the request's `exitCode` is 6, which ranks above 1, 8 and 7. With `entries`, the request's own `test`, `excludeTests`, `excludeTestsExact`, `affectedOnly`, `coverage` and `perTestCoverage` are an `error`. When the install baseline read a resource file, an entry with `resources` would need an install baseline of its own: its tests are not run, with that reason, whether the baseline ran for the request or an earlier one kept it (`baselineSource` `kept`, which a request with `resources` takes as one without: the baseline runs before any entry's texts are set, al-runner issue 8206). When no install baseline runs (no dependency app, or no selected test can observe it), the install code of the apps under test runs once for all entries and test threads of the request, and each entry's test codeunits start from what it left; it runs again for an entry whose texts of the resource files it read differ from those it read (`NavApp.ListResources` counts as reading every file of the app). Install code that leaves more than the database, WorkDate, GlobalLanguage and the clock (for example a media, an encryption key change, a file, a scheduled task, a table connection, a last error, an application area or a reseeded `Random`), and a server started with a files folder, run it again in each test codeunit's session, as before. Any other field: `{"error": "unknown field ..."}`. |
| `execute` | Runs the `OnRun` trigger of one codeunit per app (`sourcePaths`) or of inline `code`; one answer line without `type`: `{"exitCode","tests":[{"name","status","durationMs","message","stackTrace","stackFrames","messages"}],"warnings","coverage","perTestCoverage"}`; a test also carries the other fields of its `--output-json` row when they apply (`sleptMs`, `flagged`, `uiAnswered`, `orderHint`, `stubbed`, `generatedStubs`, `notRun`, `clock`). `captureValues` and `iterationTracking` give `capturedValues` and `loops` on each test and, with `iterationTracking`, the tagged `messages` on the response (`runTests` takes both too: they are on the `test` lines). Names: `Codeunit<ID>.OnRun`, for `code` `<object name>.OnRun`. `code` and `sourcePaths` together are an error. `testIsolation` and the `runTests`-only fields (`tdd` and `affectedOnly` among them) are accepted with a `warnings` entry; `serve --tdd` has no effect on `execute`. An app with no codeunit that has an `OnRun` trigger answers `exitCode` 6, `tests: []` and a warning. |
| `cancel` | Answered at once: `{"type":"ack","command":"cancel","noop":bool}`. During a run it stops **after the current test**: the tests that ran were already streamed, and the summary has `cancelled: true`, `exitCode: 130`, as in v2. A run that `shutdown` or the end of input stops has exit code 130 too. |
| `shutdown` | `{"status":"shutting down"}`, then exit 0. During a run it stops the run after the current test like `cancel`: the summary (`cancelled: true`) comes first, then `{"status":"shutting down"}`. EOF on stdin stops a run the same way and exits 0. |

Differences a client must adapt to:

1. **Test lines stream**, one per test as it finishes, then the summary.
2. The server is **warm and in-process**: compiled dependency apps and test data stay in memory. Same sources as the
   last request: the compiled workspace is reused and `cached` is `true`, whatever the request selects (`test`,
   `excludeTests`, `excludeTestsExact`, `coverage`). The compile holds every test codeunit of the sources, so the first request compiles
   all of them even when it selects one, and with `tdd` the summary's `generatedStubs` also lists the stubs of test
   codeunits the request does not select (a one-shot `al-runner test --tdd --test` compiles only the selected ones). A changed source always shows. When only the
   code of routines of codeunits, tables or table extensions changed, just those objects are compiled again and the
   summary has `incremental: true` (a request takes 0.4 s on the corpus app and 2.2 s on Tests-ERM, of which
   1.8 s is the tests); any other change compiles the workspace again. Resource files (`resourceFolders`) are not
   compiled: each request reads them as they are then, so a changed resource file is seen by the next request and that
   request still reuses the compile (`cached: true`). `cached` says only that the compile was
   reused. Every request starts from the same post-install
   database, so nothing leaks between requests. The first request after a new build of `al-runner` builds the
   dependency cache (a minute with Microsoft's apps, then 3 GB resident); with the cache on disk it takes as long as a
   one-shot run. Repeats take 0.2 to 2 s.
   A panic in a test is reported as an error for that test and the server goes on.
3. `test` lines carry every field of the test's row in `--output-json` (section 4) except `orderHint` (that one is in the
   summary's `orderHints`), plus `type`: `name`, `codeunit`,
   `codeunitName`, `method` (use these instead of splitting `name`: a
   method name may hold a dot), `status` (`pass`, `fail`, `error`), `clock` on a timed-out test, `durationMs`,
   `sleptMs`, `flagged`, `message`, `stackTrace`,
   `stackFrames` (the AL stack split into lines), and `notRun: true` for a test that did not run (status `error`, message
   `NotRun: out-of-scope: <reason>`; see section 4). `errorKind`, `alSourceFile`, `alSourceLine` are not sent.
4. The `summary` has `exitCode` (the exit code of `al-runner test`, section 5), `passed`, `flagged` (as in
   `--output-json`), `failed`, `errors` (= not run),
   `notRun`, `total`, `cached`, `incremental` when only some objects were compiled again, `protocolVersion`, `wallSeconds`,
   `warnings` when there are some, `cancelled` when cancelled, and `coverage` / `perTestCoverage` when asked for (section 4a shapes). `selection` with `affectedOnly`,
   and `compilationErrors` when the apps do not compile (exit 3). `changedFiles`, `companyInitFailures`, `sourceScanFailures` are not sent.
   `timings` (not in v2, al-runner issue 5856) says where the request spent its `wallSeconds`, in milliseconds to a
   tenth: `setupMs` (the options, the apps' sources, the dependency apps, the test data), `compileMs` (finding the kept
   compile, updating it or compiling again), `engineMs` (building the engine from the compiled program, or setting up
   the one an earlier request kept), `baselineMs` (the install baseline: running the install code or reading it),
   `testsMs` (the test codeunits, every entry), `coverageMs` (the coverage document) and `otherMs` (the rest: the report
   and the output). `baselineSource` is `ran` (the install code ran), `cache` (read from the compile cache folder) or
   `kept` (the baseline an earlier request over the same compile read), and is absent when no baseline was needed.
   A request with coverage keeps and reads the baseline as one without: the code of the install baseline is not counted (al-runner issue 8244). Without a baseline (no dependency app) the install code of the apps under test runs in each test codeunit's session and is counted, as before.
   `testDataIndex` is how the open of the test data got the backup's kept page map and catalog: `read` (the kept index
   was used), `built` (this process built it and saved it in the cache folder), `built-unsaved` (this process built it,
   but saving it failed, for example in a read-only cache folder, so the next open builds it again) or `off` (nothing
   is kept: `--no-compile-cache`, or a `.bacpac`);
   absent without test data or for a source with no index. It describes the open this process did, so a daemon repeats
   it on every request. `testDataMs` includes the download of the backup and the building of its index when the cache
   folder did not hold them.
   `setupParts` splits `setupMs` into its parts, which add up to it (al-runner issue 8205): `optionsMs` (the run seed,
   the server address, the output paths), `sourcesMs` (reading the apps' `app.json` and source files), `selectionMs`
   (finding the test codeunits, the disabled tests and test suites, the request's and its entries' selection),
   `dependenciesMs` (the target build and the dependency apps, read or kept), `testDataMs` (opening the test data),
   `licenseMs` (the license: a file, the artifact's, or kept), `dotnetMs` (the .NET helpers: their configuration and
   starting them ahead), `affectedMs` (`affectedOnly`: the stored result and the hashes of the inputs), `entriesMs`
   (building the runs of the entries) and `prepareMs` (the rest before the compile: the dependency notes, the hooks and
   the run's options). `--timings` prints the same parts as its `setup:` line.
   `loginReplays` and `loginStarts` (always present) count, over every request the engine ran, the test sessions that took a kept login start in place of running the login and the login starts kept (al-runner issue 8211). The first test session of a login runs it and keeps a start; later sessions of the same login, company, user and inputs thaw it. The replay is off by default (both 0); `ALRUNNER_LOGIN_REPLAY=1` turns it on. Under `ALRUNNER_LOGIN_REPLAY_CHECK=1` no session takes a start, so `loginReplays` stays 0, but the check keeps the starts it compares later sessions with, so `loginStarts` is not 0.
   `engineKept: true` when the request ran on the engines an earlier request over the same compile kept. On a request
   that compiled, `workspaceCache` says what the workspace layer of the compile cache did: `read` (the compiled
   workspace apps were read from the cache folder, not compiled), `built` (compiled, and kept for a later daemon or run;
   written after the response) or `off` (`ALRUNNER_NO_WORKSPACE_CACHE=1`, or a compile the layer does not cover or
   does not store; `--timings` says why). Without
   `entries`, `resetMs` is the time the test threads took to set up their own copy of the install baseline (the
   longest of them; 0 with one thread). With `entries`, each `entrySummary` has `timings` with that entry's `resetMs`
   and `wallMs` (its wall time, the reset included). The fields are for reading only: a client must not depend on
   their set or their values.
   A server keeps the apps under test it read for its next request: an app is read again when a file under its folder,
   the record next to it (`<path>.json`), or a resource folder or the logo outside its folder changes, comes or goes,
   or when the request is for another build. Where a file's stamp has no change time (off unix today), the apps under
   test are read on every request. For its next request over the same compile, a server also keeps the dependency apps
   it read (read again when the app files, the options or a file in a package folder change), the engines and the
   install baseline: a request that reuses the compile costs the tests and little else. What a request starts from is the same as when nothing is
   kept: the rows the install code left, none of an earlier request's writes.
   The run clock is not kept: each request begins its own, at 12:34:56.789 UTC of the day the request starts, so a
   server kept past midnight answers `Today` with the new date, as a new server would (ADR-045, amendment of
   2026-11-22, Accepted (owner, 2026-11-22)).
   What each test codeunit starts from (in every request and every entry; al-runner issue 5862): a new session over
   the post-install database. Reset: table rows, the system id, row version and AutoIncrement counters,
   IsolatedStorage, NumberSequence, SingleInstance codeunits (also those of dependency apps), event bindings, handlers,
   and session settings such as WorkDate and GlobalLanguage. Not reset: files a `--files-dir` folder holds on disk,
   and the state of the .NET helper processes, which the codeunits of one request share.
   A test the time limit stops ends at the next statement the interpreter checks the limit at; its session is dropped,
   and the next test codeunit, entry or request starts as above, as on a new server (al-runner issue 8334), except under `testIsolation` `disabled`: there the next test codeunit of the same request sees the rows the stopped test committed, as on a new server and on main. A test that
   waits inside a .NET helper call is never stopped inside that call: the limit is read only where the interpreter
   checks it and in `Sleep`, and a helper call is one request and reply on the test thread. So the call either
   returns and the test stops at the next check, or it reaches the helper's deadline, and that helper process is
   killed and never used again. Such a test answers late, at `--dotnet-timeout-secs` (default 60), not at
   `testTimeoutSecs`; a client's own deadline should be above it (al-runner issue 9213).
   An entry whose selection picks exactly one test runs that test alone (the codeunit's `OnRun`, then that test), as
   one runner call per method does in BC, so a sibling that sets WorkDate does not run before it (al-runner issue 7207).
   The plain `test` field of a request does the same when its selection picks exactly one test (owner decision
   2026-11-06). This is a difference from v2, which ran the whole codeunit. Only `serve` does this: the CLI `--test`
   keeps running whole codeunits (section 3).
5. A request that cannot run answers one `{"error": "<reason>"}` line instead of a summary with exit code 2 or 3:
   for example `{"error":"/x: no such file or folder"}`, ``{"error":"unknown field `cobertura` for runTests"}``,
   ``{"error":"`sourcePaths` needs at least one app folder"}``. A `test`, `excludeTests` or `excludeTestsExact` that matches no test is not
   an error: it answers a summary with exit code 6, all counts 0, and the reason in `warnings`. With `entries`, an
   entry that matches no test gives the same signal on its `entrySummary`, and the request's exit code is 6.
6. While a run is in progress `cancel` is answered at once. `shutdown` takes effect after the current test (see the
   table). Other requests are queued and run in order, as in v2.
7. `testIsolation: "test"` means TestIsolation Function (section 3), `"disabled"` rolls nothing back (as `--isolation disabled`). `"required"` (not in v2)
   runs each test codeunit under its own `RequiredTestIsolation`, as `--isolation required` does.

## 5. Exit codes

| Code | v2 | v3 |
|---|---|---|
| 0 | all passed | all selected tests passed |
| 1 | a test failed or errored | a test failed |
| 2 | bundle could not execute, bad flag | bad invocation or the run could not start, also a dependency app that cannot be found under `--no-download` (add `--allow-missing-dependencies` to skip it with a warning) |
| 3 | bundle could not compile, or suite error | the APPs do not compile: an AL compiler error that the corpus has checked against the real compiler, or source of an app under test that AL Runner's parser does not accept (no AL code on that line); no test ran (`compilationErrors` in `--output-json`). Other code AL Runner cannot compile gives `notRun` rows and exit 7. |
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

The command LethAL builds
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
   Exit 3 means the APPs do not compile, as in v2.
4. **Not-run rows** (`al-runner-backend.ts:97`): keep recognizing `<Type>: out-of-scope: ...`; the v3 message is
   `NotRun: out-of-scope: <reason>`. Also read `notRun: true`. A row with `notRun` is "runner refused", never "killed".
5. **Timeouts** (`al-runner-backend.ts:79-93`): unchanged sentence. The limit is CPU time in v3.
6. **Compile failure** (`al-runner-contract.ts` fact `compile-failure-not-scorable`): a mutant that the AL compiler
   rejects gives exit 3 when the corpus has checked that error against the real compiler or AL Runner's parser rejects the source, else rows with `notRun: true`. Score both as "not scorable".
7. **Assertion screen** (`assertion-screen.ts:36-44, 78-90`): do not depend on `NavNCLDialogException: ` or
   `InvalidOperationException: out-of-scope:`. Messages are the BC text.
8. **Stderr** (`al-runner-transport.ts:106-210`): see section 7.
9. **Coverage**: `"coverage": "al-runner"` works with `--coverage --coverage-out FILE` (section 4a). LethAL reads
   `<class ... filename=...>` and `<line number=... hits=...>` by regex: both are as it expects. Labels are relative to the
   working directory of the process. Per-test coverage (`--coverage-json`) is new; use it instead of one run per test.
10. **Selector**: `"selectorMode": "static"`.
11. **`--auto-provision`** may stay in the command; it is accepted.

Check: run LethAL's contract probe against `al-runner`; the six facts (version, unknown flag rejected with a
non-zero exit chosen by the process (2), qualified test name, timeout classified, timeout exit readable,
compile failure not scorable) must hold with the changes above.

## 9. ALchemist

ALchemist needs `al-runner serve`, `execute`, value capture and `al-runner debug`. All four exist; scratch runs, inline values and debugging need the changes below.

1. Binary lookup (`alRunnerManager.ts:47-51, 129-139`): also `which al-runner`; spawn `al-runner serve` instead of
   `al-runner --server` (`serverProcess.ts:125-141`). The version check only warns and accepts `3.0.0` (A3).
2. `runtests` (lowercase) is accepted. ALchemist sends `captureValues` and `iterationTracking` on `runTests`: v3 puts
   `capturedValues` and `loops` on the `test` lines (v2 ignored them there). It never sends `cobertura` or `testFilter`; both would be
   refused as unknown fields. Section 4b lists the other differences (warm state, streaming, queued requests).
3. `coverage` arrives in the summary in the v2 shape (section 4a). ALchemist reads `hitStatements` and `totalStatements`,
   which neither v2 nor v3 sends (a change on the tool's side).
4. Test names are `Codeunit<ID>.<Method>` as in v2; the label match in `testController.ts:493` (A8) must be checked on a run.
5. Debugger: start `al-runner debug --stdio APP...` instead of `al-runner --dap stdio DIR`. Requests, capabilities,
   events and breakpoint answers are v2's. Differences: `pause` stops at the next statement (v2 continued);
   `setExceptionBreakpoints` succeeds with no filters; frame names are `Object.Routine`; a value that is not rendered yet
   shows `<not rendered: KIND>`; `--test` and `--codeunit` select tests (v2 ran all); a file of a dependency read from a
   source package is shown from `<cache>/debug-sources/` (see [Debugging](debugging.md)).

### A note for ALchemist's author

The debugger needs one change in ALchemist: the arguments of the `DebugAdapterExecutable`
(`debugAdapterFactory.ts:56-98`). Pass `['debug', '--stdio', ...sourcePaths]` instead of
`['--dap', 'stdio', ...sourcePaths]`, with `--dep` for a dependency folder whose code should be debugged and the
launch configuration's `testFilter` as `--test`. Nothing else in the DAP code has to change: requests, capabilities,
events and breakpoint answers are v2's. ALchemist's text that says "stepping acts as continue"
(`debugLaunch.ts:61-62`) is out of date: `next`, `stepIn` and `stepOut` step. [Debugging](debugging.md), section "Using the
debugger from ALchemist", has the command line and a table of what differs from v2. AL Runner ships no VS Code
extension of its own.

## 10. What does not change for a tool

The names of the tests (`Codeunit<ID>.<Method>`), the meaning of pass and fail, `durationMs`, the
JUnit file (`--junit`), the position of positional app folders, `--package-cache`.
