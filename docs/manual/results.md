---
title: "Reading results"
weight: 40
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

## All outcome labels

| Label | Meaning | Counts for the exit code as |
|---|---|---|
| passed | The test ran to its end without an error. | a pass |
| flagged | A pass that relies on an unprobed assumption (see below). | a pass |
| stubbed | With `--tdd`: the test ran a stub for a procedure that does not exist yet. It also passed or failed. | its own pass or failure |
| failed | The test raised an error or failed an assertion. | 1 |
| not run | AL Runner refused the test and gives the reason. | 7 (8 when the .NET helper could not be used) |
| not trusted | A failure that an earlier refused test could explain. It is reported as not run. | 7 |
| timeout | The time limit stopped the test. It is reported as not run, with `Test exceeded <N>s timeout.` | 7 |
| skipped | With `--affected`: the change cannot affect the test, so it did not run and keeps its stored outcome. | 1 when it failed last time, else nothing |
| not selected | With `test-microsoft`: the test does not reach your extension, so it did not run. | nothing |
| `disabled_by_microsoft` | A mark, not an outcome: Microsoft's list of disabled tests names the test. It still runs. `--parity` skips it. | its own outcome |

In `--output-json`, a timed-out test has a `clock` field: `cpu` when its CPU time
passed the limit, `virtual` when the time its `Sleep` calls asked for did. `Sleep`
does not wait, so a `virtual` stop takes less real time than the limit.

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
  `Test exceeded <N>s timeout.` and a `clock` field (`cpu` or `virtual`). Without FILE, the document is the only thing on
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

[How results are checked](how-it-works.md) explains how results are compared with real
Business Central, and lists the known risks.

## Reporting a wrong result

A test that passes or fails here and behaves differently on real Business Central
is the most useful report. [Reporting a problem](feedback.md) says what to include.
