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

All of them can be repeated. A selected test's codeunit runs whole, because the
tests of a codeunit share state. Only the selected tests are reported, and only
the selected codeunits are compiled. If the selection matches no test, the run
exits with 6.

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
- `disabled` is not supported. Every test is reported as not run.

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
- In test apps, these calls become stubs so far: a procedure that does not exist on a
  codeunit, record, page or report variable, on `Rec`, `CurrPage` or `this`; a bare call
  of a procedure the object does not declare; and a call whose argument count no
  overload takes. A call on a TestPage, an interface or a built-in type stays an error.
  A call site whose type the use does not reveal is refused, and its test is not run.
- `serve` takes it as the `tdd` field of `runTests`, or `serve --tdd`.

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
