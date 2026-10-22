---
title: "Reading results"
weight: 4
---

A pass or a failure is meant to match real Business Central. When AL Runner cannot
be sure, it reports the test as **not run** and gives the reason. It never
guesses.

## Pass, fail, not run

**A pass** means the test ran to its end without an error.

**A failure** means the test ran and raised an error or failed an assertion, and
no earlier refused test can explain it. Without test data, Microsoft's tests fail
mostly on missing setup data, and the `Tests:` line says so.

**Not run** means AL Runner refused the test. The test neither passed nor failed.
There are three kinds:

1. A feature is missing.
2. A behavior is not measured on real Business Central yet.
3. The test failed, but the failure is not trusted. An earlier test in the same
   codeunit did not run to its end, and it could have changed what this test sees:
   a table it reads, the codeunit's shared state, or the sequence of random
   numbers. The reason says "this failure is not trusted" and keeps the original
   error. A test stopped by the time limit counts as such an earlier test.

## Flagged passes

Some behavior is built before a test in the corpus measures it on real Business
Central. This happens only when Microsoft's own tests need that behavior. Each
such behavior is registered as an unprobed assumption. A pass that ran through one
is a **flagged pass**, counted apart:

- The `Tests:` line reads `N passed (M flagged: rely on unprobed assumptions)`.
- `--verbose` marks each such test `(flagged)`.
- `--explain TEST` lists the assumptions and the probe that would settle each.
- `--junit` lists the assumption ids in an `assumptions` property.
- `--top-assumptions [N]` ranks the assumptions a run went through.
- `al-runner --list-assumptions` describes all of them, with the counts of the last run.

## Finding out why

```bash
al-runner test ./MyApp ./MyApp.Test --top-refusals          # most common not-run reasons
al-runner test ./MyApp ./MyApp.Test --top-failures          # most common failure messages
al-runner test ./MyApp ./MyApp.Test --refusal-detail TEXT   # reasons that contain TEXT
al-runner test ./MyApp ./MyApp.Test --explain TestName      # one test in detail
al-runner test ./MyApp ./MyApp.Test --timings               # where the time went
```

`--explain` runs one test (its name, or a part of it that only one test has) and
prints its outcome. For a failure or a refusal it also prints the routines and
source lines the test left from:

```
BC 27.5.46862.56249: test Wrong in codeunit 50100 (Demo Calc Tests)
runtime: FAIL
error: Expected 80 but got 90
code: Dialog
AL call stack (GetLastErrorCallStack form):
  "Demo Calc Tests"(CodeUnit 50100).Wrong line 5 - Demo.Test by Me version 1.0.0.0
raised in (innermost first):
  codeunit 50100 "Demo Calc Tests".Wrong
    at Demo.Test: src/CalcTests.Codeunit.al:20:13
```

## Report files

- `--junit FILE`: JUnit XML. A test that did not run is `<skipped>` with its reason.
  The AL call stack is the text of each `<failure>`.
- `--output-json [FILE]`: one JSON document in the shape of AL Runner version 2:
  `tests[]` with `name`, `status` (`pass`, `fail`, `error`), `durationMs`, `sleptMs`
  (the time the test's `Sleep` calls asked for, summed; `Sleep` returns at once, so
  `durationMs` does not hold it; absent when the test did not sleep), `message`
  and `stackTrace`, plus `passed`, `failed`, `errors`, `skipped`, `total`, `notRun`,
  `exitCode` and `wallSeconds`. A test that did not run is `error` with the message
  `NotRun: out-of-scope: <reason>`. A test the time limit stopped is `error` with
  `Test exceeded <N>s timeout.` Without FILE, the document is the only thing on
  stdout and the report goes to stderr.
- `--coverage`, `--coverage-out FILE`, `--coverage-json FILE`: see [Editor and tool integration](editor-tools.md).

## Exit codes

| Code | Meaning |
|---|---|
| 0 | Every selected test passed. |
| 1 | A test failed. |
| 2 | Bad invocation, or the run could not start. |
| 3 | The apps do not compile (an AL compiler error such as AL0118). No test ran. |
| 6 | The selection matched no test. |
| 7 | Some tests were not run (a refusal). |
| 8 | Some tests were not run because the .NET helper could not be used. |

When several apply, the order is 2, 6, 1, 8, 7. `--allow-not-run` makes 7 and 8
return 0. Use it while the runtime is an alpha, so refused tests do not fail a job.

## Known risks

1. Some behavior was written before the measurement rule and has no corpus test
   yet. Corpus tests are being added for it.
2. A flagged pass depends on its assumption. When a corpus test measures the
   behavior and disagrees, the runtime is changed to match Business Central.
3. The corpus runs Business Central in a Linux container, which Microsoft does not
   support. Results that look like a container difference are checked on
   Microsoft's Windows container first. Where the two differ, the runtime refuses the case.
4. Microsoft's own tests are not compared with real Business Central test by test.
   Their pass rate is the result of this runtime.
5. Permissions are not enforced. Every access is granted, so a test that expects a
   permission error fails here.
6. The "not trusted" rule covers failures only. A test that passes after a refused
   test stays a pass, even if the refused test would have changed its data on
   Business Central.

## Reporting a wrong result

A test that passes or fails here and behaves differently on real Business Central
is the most useful report. Open a GitHub issue with the "Alpha feedback" template.
Include `al-runner --version`, the Business Central version, the test name, what
you expected, and the output of `--explain TEST`.
