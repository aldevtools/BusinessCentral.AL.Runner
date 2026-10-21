---
title: "Editor and tool integration"
weight: 8
---

## `al-runner serve`

`serve` is a long-running process for editors and other tools. It speaks AL Runner
version 2's protocol (`protocolVersion` 2), so tools written for version 2, such as
ALchemist and LethAL, keep working. The protocol is one JSON object per line on
stdin, answers on stdout, and logs on stderr. The first stdout line is
`{"ready":true}`.

```bash
al-runner serve [--protocol 2] [OPTIONS]
```

Commands (not case sensitive): `runTests`, `execute`, `cancel`, `shutdown`.

- **`runTests`** writes one `test` line per test as it finishes, then a `summary`.
- **`execute`** runs one `OnRun` trigger per app, or inline `code`, and answers
  with one line.
- **`cancel`** is answered at once and stops the run after the current test. The
  summary has `cancelled: true` and `exitCode: 130`.
- **`shutdown`** stops a running request the same way and exits.

A `runTests` request carries `sourcePaths`, `packagePaths`, `test`, `excludeTests`,
`testIsolation`, `coverage`, `perTestCoverage`, `captureValues`, `iterationTracking`,
`announceStart`, `serverUrl` and `tdd`. `serve --tdd` makes `tdd` the default.

### Warm state

Requests run in the server process, one at a time. The compile of the last request
is kept:

- The same sources and test codeunits reuse it (`cached: true`).
- A change to only routine code of codeunits, tables or table extensions compiles
  just those objects again (`incremental: true`).
- Every request starts from the same database, so nothing leaks between requests.
- A repeated request takes 0.2 to 2 seconds. The first request after a new build
  builds the dependency cache, which takes about a minute with Microsoft's apps.

### Start options

The start options are those of `test` that do not depend on a request:
`--package-cache`, `--dep`, `--define`, `--test-timeout`, `--bc-version`,
`--cache-dir`, `--no-download`, `--test-data`, `--server-url`, `--environment` and
the like. Options that select tests or write reports (`--filter`, `--junit`,
`--coverage`, and so on) belong in the request, and give an error at start. So do
`--recipe` and `--impact-of`.

The complete field list and the answers are in
[Migrating tools to version 3](../integrations/migrating-tools-to-v3.md), section 4b.

## Coverage output

`--coverage` counts how often each statement of your apps runs, per test.
Dependency apps are not included.

- `--coverage-out FILE` writes Cobertura XML: one `<package>` per app, one `<class>`
  per source file, and a `<line>` with its `hits` for each line where a statement
  starts. File names are relative to the working directory when they lie below it,
  with forward slashes. A file inside a package is `<package path>!/<path inside>`.
  There is no branch data.
- `--coverage-json FILE` writes `{ "coverage": [...], "perTestCoverage": [...] }`:
  every statement with its id, scope, line, column, end line and hits, and for each
  test the statements it ran. File names are absolute.

## JSON and JUnit

See [Reading results](results.md#report-files) for the shapes. Both mark a test that
did not run, so a tool can tell "refused" from "failed".

## Not available in version 3

Watch mode and the debugger (DAP) are not in `3.0.0-alpha.3`. A debug adapter is
planned for 3.0.0, and `--affected` test selection is planned as a command line option
and a `serve` field. Watch mode was rejected for 3.0.0, because `serve` already keeps
the compile warm.
