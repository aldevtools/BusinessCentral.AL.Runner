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

**A test codeunit's `OnRun` trigger** has no row of its own. AL Runner reports it
as Microsoft's Test Runner app does (runner codeunit 130450, used by
BcContainerHelper and AL-Go), on every BC version. When `OnRun` raises an error,
the codeunit is skipped: none of its tests has a row, and the run does not fail
because of it.

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

### Flagged: relies on SingleInstance state another test codeunit left

Business Central runs all test codeunits of a run in one session. The globals of a
`SingleInstance` codeunit that one test codeunit left are still there when the
next test codeunit starts. AL Runner runs test codeunits in parallel, each on a
session of its own, so a test codeunit starts with fresh globals.

A test codeunit is **order dependent** when it reads such a global before it
writes it, and an earlier test codeunit left another value there. On Business
Central it sees that value; in AL Runner's parallel run it sees a fresh one.

AL Runner follows one principle: a test should give the same result whether its
codeunit runs alone or inside the whole suite. A parallel run with fresh state is a
real Business Central result: the one you get when the codeunit runs alone, or
first. So, by default:

1. Order-dependent codeunits run in parallel like all others.
2. When such a codeunit has a failed test or a test not run in its parallel run,
   AL Runner runs it again, together with the codeunits that wrote the state, one
   after the other in Business Central's order, with the state carried over. That
   serial result is the one reported.
3. Every test of an order-dependent codeunit is a flagged pass (or a flagged
   failure) with the assumption `single-instance-order`, whether it passed in
   parallel or after the serial run. It is counted apart from the clean passes.

`--timings` names the codeunit whose state each test relies on. Its
`SingleInstance order:` line gives how many order-dependent codeunits were found, how
many ran again because of a failure, and how long that took.

What the flag means: the result depends on the order in which Business Central
runs the test codeunits. The stability mode (`--stability`) lists these tests apart,
with and without `--strict-order`.

The risk of the default: a test that passes alone but fails on Business Central in
the full order is reported as a flagged pass here. To remove that risk, use
`--strict-order` (in `serve`: the `runTests` field `strictOrder`). Every
order-dependent codeunit then runs again in Business Central's order, and that
result is the one reported. It costs time on suites with many such codeunits:
Microsoft's Tests-SCM takes about 357 s instead of 245 s on BC 29, because the
entry-number buffers of Sequence No. Mgt. make most posting codeunits order dependent.

`AL_TEST_THREADS=1` runs one test codeunit at a time, but each one still starts with
fresh SingleInstance state. It does not reproduce Business Central's order; only
`--strict-order` does.

How to fix such a test:

1. Make the test set up what it reads. Call the setup procedure in the test, or
   in the codeunit's own `Initialize()`, and do not rely on an earlier codeunit having
   done it.
2. Clear the state. Add a procedure to the `SingleInstance` codeunit that clears its
   globals, and call it in `Initialize()` (or at the end of each test).
3. A lazy cache (a flag that guards a fill) is not flagged when the test fills it
   with the same values as the earlier codeunit did.

A test codeunit that also hands rows on to later codeunits (`RequiredTestIsolation =
Disabled`, or an `OnRun` that writes under runner 130450) runs again like the others.
The later codeunits keep the rows its first run left.

## Errors whose text is not pinned

Some errors are known to happen, but the corpus does not pin the exact text
Business Central gives. By default, a test that ends on such an error is **not run**,
with the reason `The test result of an error whose text is not pinned`. A made-up
text would otherwise stand in its result.

Set `ALRUNNER_REPORT_UNPINNED_ERRORS=1` to report such a test as **failed** instead
(opt-in, issue 6487). The test did run and fail; only the wording is not known.

- Its failure message starts with `[error text not pinned]`, in the console,
  `--junit` and `--output-json`.
- The `Tests:` line ends with `(ALRUNNER_REPORT_UNPINNED_ERRORS=1: N failed with an error
  text that is not pinned, reported instead of not run; not a conformance result)`.
- `--junit` has a `report_unpinned_errors` property with value `1` on every `testsuite`.
- `--output-json` has `reportUnpinnedErrors: true` and `unpinnedErrorFailures: N`.
- Any other value, or no variable, keeps the default.

A run with the variable set is not a conformance result. Use it in CI when a failure
is more useful to you than a refusal.

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

## Time per AL routine

`--timings` stops at the test codeunit. To see which AL routines a slow run spends
its time in, set the environment variable `AL_PROFILE` to a file name:

```bash
AL_PROFILE=profile.txt al-runner test ./MyApp ./MyApp.Test --codeunit 50100
```

The run samples the AL call stack every 5 ms. It writes the file every 5 seconds
and once more when the run ends. The first line has the sample count and the interval,
for example `samples 1200 of 5 ms`: 1,200 samples are about 6 seconds of AL code. Then
come three lists of the 60 entries with the most samples:

- `self`: samples where the routine was running its own statements.
- `inclusive`: samples where the routine was anywhere on the call stack.
- `self by statement`: the `self` samples split by the statement that was running,
  as file and line. When the sample landed inside a built-in method, its name follows
  in brackets, for example
  `codeunit No. Series - Impl..GetNoSeriesLine  NoSeriesImpl.Codeunit.al:150  [Record.FindLast]`.
  `(entry)` stands for the time between a call and the routine's first statement.

Each line is the sample count, then the object type, the object name and the routine.
A field trigger also names its field, as in `table Sales Line.Quantity.OnValidate`;
a trigger of a page control, an action or a report data item names its owner.

How to read the counts:

- One process-wide timer samples one thread per tick. When tests run on several
  threads, the samples add up to the time any AL statement ran, not to CPU time
  times the number of threads. Routines that run many statements get a larger share.
  Set `AL_TEST_THREADS=1` for a profile of one thread.
- A sample is taken at the next AL statement or when a built-in method returns.
  Time spent inside a built-in method (for example `FindSet` or `Validate`) counts
  as `self` of the AL routine that called it; `self by statement` names the method
  and the line. A built-in method that runs AL code (a trigger, an event subscriber)
  books that code's time to the AL routines it runs.
Two more variables help with short runs:

- `AL_PROFILE_US=N`: sample every N microseconds instead of 5,000.
- `AL_PROFILE_ONLY=TEXT`: count only samples whose call stack has a frame, written as
  in the file, that contains TEXT. Use one test's name to profile that test.

The profile counts AL statements only, including install code. Loading and compiling
are not in it; `--timings` shows those.

## Report files

- `--junit FILE`: JUnit XML. A test that did not run is `<skipped>` with its reason.
  The AL call stack is the text of each `<failure>`. Every `testsuite` has a `seed` property: the run seed.
- `--output-json [FILE]`: one JSON document in the shape of AL Runner version 2:
  `tests[]` with `name`, `status` (`pass`, `fail`, `error`), `durationMs`, `sleptMs`
  (the time the test's `Sleep` calls asked for, summed; `Sleep` returns at once, so
  `durationMs` does not hold it; absent when the test did not sleep), `message`
  and `stackTrace`, plus `passed`, `failed`, `errors`, `skipped`, `total`, `notRun`,
  `exitCode`, `wallSeconds`, and the run seed as `seed` and `seedSource` (`option`, `env` or `fresh`). A test that did not run is `error` with the message
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
