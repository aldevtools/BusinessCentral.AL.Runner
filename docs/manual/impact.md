---
title: "Impact analysis"
weight: 9
---

Impact analysis answers one question: what does an extension change in the tests
of other apps? It is meant for Microsoft's own test apps. It runs the selected
tests without the extension, then with it, and lists per test what changed and
which procedures of the extension ran.

```bash
al-runner test Tests-ERM.Source.zip --impact-of ./MyExtension
```

`--impact-of APP` is repeatable. With `--disabled-tests`, the tests Microsoft turns
off are left out.

## Choosing the tests

`--impact MODE` decides which tests run:

| Mode | Behavior |
|---|---|
| `dynamic` | The default when an impact index of the same apps and options is cached. Only the test codeunits that reached one of the extension's impact points in the index's run are run, once, with the extension. The outcomes without it come from the index. |
| `all` | Every selected test runs without and with the extension. |
| `static` | The test codeunits whose tests can reach an impact point over the call graph, on both sides. With no index cached, the run warns and does what `static` does. |

## The impact index

```bash
al-runner impact-index build APP...
al-runner impact-index check-static APP...
```

`build` runs the tests once (the same options as `test`, without `--impact-of`) and
keeps, per test, the events it raised and the tables, pages and reports it used.
It is never built implicitly. `check-static` checks that static reach covers
everything each test recorded in the cached index (exit 1 on a miss), and reports
how much more static reach selects.

## Output

- Each test gets a change class, shown in the report.
- `--junit FILE` writes the run with the extension. Each test has the properties
  `impact_class`, `impact_without` and `impact_accepted`.
- `--output-json FILE` writes that run in version 2's shape.
- `--impact-json FILE` writes the per-test difference: `format`, `mode`,
  `extensions`, `testApps`, `counts`, `accepted`, `notSelected`, `exitCode`, and
  `tests[]` with the outcome without and with the extension, the impact points
  reached, and the routines that ran.

## Accepting changes on purpose

`--impact-accept FILE` lists changes the extension makes on purpose. The file has the
shape `--impact-json` writes, so the file of an earlier run works as it is. Delete
the tests you do not accept. A test is accepted when its codeunit, name, outcome
with the extension and message equal an entry whose class is a change. An accepted
change is listed, marked `(accepted)`, and does not fail the run. An entry that no
longer matches is listed below the report.

## Exit codes of an impact run

| Code | Meaning |
|---|---|
| 0 | No test changed for the worse, or every such change is accepted. |
| 1 | A test is newly failing, fails differently, is newly refused or is a selection miss, and is not accepted. |
| 2 | Bad options, or an unreadable accept file. |
| 6 | A side selected no test. |

Tests not run on a side are the class "not verified". They do not give 7 or 8.
