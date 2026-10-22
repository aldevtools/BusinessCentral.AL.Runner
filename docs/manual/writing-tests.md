---
title: "Writing tests"
weight: 3
---

Tests for AL Runner are ordinary Business Central test codeunits. There is no
runner-specific framework and no attribute to add. A test that works here also
works against a real environment, which is the point: a test that only passed
here would say nothing about Business Central.

## Apps

Pass your app and its test app, in that order or any order:

```bash
al-runner test ./MyApp ./MyApp.Test
```

Each is a folder with an `app.json`, a source archive (`*.Source.zip`), or an
`.app` that carries its source. The test app lists your app as a dependency in
`app.json`, as it would for the AL compiler.

Each app sees the AL methods of the `runtime` its own `app.json` declares. A call
of a method newer than that runtime gets the AL compiler's AL0666, and the run stops
with exit 3, as the compiler's would. For an app whose `runtime` is older than 16.0,
AL Runner uses a table measured with the AL compiler itself, so it reports the same
AL0666. Where the compiler reports something other than AL0666 for the call (for
example a method only OnPrem apps may use, in an app older than runtime 4.0), the
routine that makes the call does not run, and a note says so.
The same holds for properties, data types, method attributes and object types: a
property the compiler does not accept at the app's runtime (for example `Subtype` on a
report layout below runtime 18.0, or `ObsoleteState` on a page area from runtime 13.0
on) gets the compiler's AL0666, from a second table measured the same way.
Preprocessor symbols for `#if` come from `--define SYMBOL` (repeatable), or
`--preprocessor-symbols A,B`.

## A test codeunit

```al
codeunit 50100 "My Feature Tests"
{
    Subtype = Test;

    [Test]
    procedure DiscountAppliesToTheLineAmount()
    var
        Assert: Codeunit "Library Assert";
    begin
        // arrange and act here
        Assert.AreEqual(90, ActualAmount, 'A 10% discount should leave 90.');
    end;
}
```

`Subtype = Test` marks the codeunit and `[Test]` marks each test procedure. The
Microsoft test libraries (`Library Assert`, `Any`, and the rest) are real Business
Central code and run as written. Handler attributes work as on a service tier:
`[ConfirmHandler]`, `[MessageHandler]`, `[StrMenuHandler]`, `[ModalPageHandler]`,
`[PageHandler]`, `[RequestPageHandler]`, `[ReportHandler]`, `[SendNotificationHandler]`
and `[HttpClientHandler]`. An unhandled dialog fails the test, as it does on
Business Central. Page and report layout is not rendered; see
[What works](what-works.md).

## Choosing tests

A test is selected when any selecting option matches it:

```bash
al-runner test ./MyApp ./MyApp.Test --filter Posting          # name contains "Posting"
al-runner test ./MyApp ./MyApp.Test --filter Codeunit50100.Foo # qualified name
al-runner test ./MyApp ./MyApp.Test --codeunit 50100           # by object id
al-runner test ./MyApp ./MyApp.Test --test-exact Codeunit50100.Foo
al-runner test ./MyApp ./MyApp.Test --exclude-test Slow
```

- `--filter TEXT` (also `--test TEXT`): tests whose name contains TEXT, any case.
  TEXT with a dot is matched against `Codeunit<ID>.<Method>`; TEXT without a dot
  against the method name.
- `--test-exact NAME`: the test whose method name or `Codeunit<ID>.<Method>` is exactly NAME.
- `--codeunit ID`: a test codeunit by object id.
- `--exclude-test TEXT`: leaves out the tests TEXT matches. It wins over every selecting option.

`--filter` and `--test-exact` together select only the tests both match. All of them can be repeated. A selected test's codeunit runs whole, because the
tests of a codeunit share state. Only the selected tests are reported, and only
the selected codeunits are compiled. If the selection matches no test, the run
exits with 6.

To run only the selected tests of a codeunit, and not the whole codeunit, add
`--run-selected-only`. This is how Business Central's test tool treats enabled test
methods: the other tests do not run, so their globals, rows, time and refusals cannot
reach the selected ones. The codeunit's `OnRun` still runs first.

## Isolation

```bash
al-runner test --isolation codeunit ./MyApp ./MyApp.Test   # default
al-runner test --isolation test     ./MyApp ./MyApp.Test
```

- `codeunit` (default): the database is rolled back when a test codeunit ends.
  This is the default of Microsoft's test runner.
- `test` (also `function`, `method`): the database goes back to its state after
  `OnRun` after every test method. The codeunit's AL variables are kept, as with
  `TestIsolation = Function`.
- `disabled` is not supported yet. Every test is reported as not run.

Under `test`, a test that calls `Commit()` is reported as not run, because whether
Business Central keeps the committed rows for the next test is not pinned yet. Run
that codeunit with `codeunit` isolation.

## Test-first work with `--tdd`

`--tdd` lets you write the test before the code. A test that calls a procedure
that does not exist yet runs against a stub instead of failing to
compile. The stub returns the default value of the type the call site asks for
(`0`, `''`, `false`, an empty record), writes nothing and never raises. `var`
arguments stay unchanged.

```bash
al-runner test ./MyApp ./MyApp.Test --tdd
```

- Every test that ran a stub is reported as stubbed, whether it passed or failed.
  The output names the stubs, and the `Tests:` line counts stubbed passes and
  failures separately.
- A stubbed pass counts as a pass for the exit code.
- `--output-json` adds `stubbed` and `generatedStubs` per test and in the summary,
  and `--junit` adds a `stubbed` property.
- In test apps, these become stubs: a procedure that does not exist on a codeunit,
  record, page or report variable, on `Rec`, `CurrPage` or `this`; a bare call of a
  procedure the object does not declare; a call whose argument count no overload
  takes; a missing field; a missing enum value; and a missing codeunit, table or enum,
  which is declared for the run (ADR-051). A call on a TestPage, an interface or a
  built-in type stays an error.
  A call site whose type the use does not reveal is refused, and its test is not run.
- `serve` takes it as the `tdd` field of `runTests`, or `serve --tdd`.

## Running only what a change affects

`--affected` runs only the test codeunits a change since the last `--affected` run
can affect. A codeunit runs when its tests entered a changed object, used a changed
table, page, report or enum, or raised an event a changed subscriber binds. Codeunits
that are new, or did not run to their end last time, run too.

```bash
al-runner test ./MyApp ./MyApp.Test --affected
al-runner test ./MyApp ./MyApp.Test --affected --include-failing
```

- The other codeunits are not run and keep their stored outcome. They are counted as
  **skipped**, never as passed. A skipped test that failed last time keeps the exit
  code at 1.
- A change the rule cannot follow runs everything and says why: an added or removed
  object, `app.json`, a dependency app, the Business Central build, a run option, or
  install code.
- Every run with `--affected` writes the stored result for the next one. It lives in
  `<cache dir>/affected/`, keyed by the ids and versions of the apps. In CI, use
  `--affected-store FILE` and save and restore that file between builds.
- `--include-failing` also runs every codeunit with a test that did not pass last time.
- It cannot be combined with `--test`, `--filter`, `--test-exact`, `--codeunit` or
  `--exclude-test`.
- In `serve`, the `runTests` fields `affectedOnly` and `includeFailing` do the same.

## Several versions and countries

A list in `--bc-version` or `--country` runs every combination, one cell each:

```bash
al-runner test ./MyApp ./MyApp.Test --bc-version 27.5,28.5,29.0 --country w1,us
al-runner test ./MyApp ./MyApp.Test --matrix "27.5:w1,29.0:us"
```

`--matrix` names the cells directly, and a version alone means `w1`. It cannot be
combined with `--bc-version` or `--country`. Every cell is checked before any runs
(its build, the country on the feed, the `app.json` floor, and `--company-data
package` only on `w1`). One bad cell stops the run with exit 2.

- Each cell runs as its own process, with its own command line, which the report prints.
- `--matrix-parallel N` runs up to N cells at once. A cell starts only while the free
  memory is above its expected peak plus 2 GB: its peak in the last matrix run, else
  3 GB, or 10 GB with the Microsoft source. Each cell gets 1/N of the worker threads.
  `--matrix-memory GB` keeps the expected peaks of the running cells within GB.
- The report has each cell's failures and `Result:` line (`--verbose` prints its whole
  report), a table of the cells, and the tests whose outcome differs between cells.
- `--junit FILE` writes one file per cell (`results.28.5-w1.xml`). `--output-json FILE`
  writes one document with every cell and the differences. The exit code is the worst
  cell's.
- AL Runner defines no preprocessor symbol of its own in a cell. Each compiles with the
  app's `preprocessorSymbols` and `--define`.
- Refused with a matrix: `--explain`, `--test-data PATH.bak`, `--output-json` without a
  FILE, and `serve`.

## Setup data

Many Business Central tests expect setup records that an empty database does not
have. A run that uses Microsoft's apps restores the CRONUS demo company by
default. See [Test data](test-data.md).

## Time limit

A test that uses more than 20 seconds of CPU time is stopped at its next loop
iteration and reported as not run, with the limit as the reason. Load on your
machine does not count. A test that hangs without computing is stopped at ten
times the limit in wall time. `--test-timeout-secs N` changes the limit, and `0`
removes it. Later failures in the same codeunit are then not trusted; see
[Reading results](results.md).

## Examples

The [AL language test corpus](https://github.com/aldevtools/BusinessCentral.AL.Language.Tests)
holds thousands of tests of exactly this shape. Each was validated against a real
Business Central service tier.
