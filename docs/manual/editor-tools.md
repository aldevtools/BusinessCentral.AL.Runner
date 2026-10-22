---
title: "Editor and tool integration"
weight: 9
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

## Debugging

`al-runner debug` is a Debug Adapter Protocol (DAP) server, and the replacement for
version 2's `--dap`. It compiles the apps, waits for one client, runs the selected
tests once with breakpoints, stepping, the call stack and the locals, and exits with
the exit code of `test`.

```bash
al-runner debug --stdio ./MyApp ./MyApp.Test      # DAP on stdin and stdout
al-runner debug --port 4711 ./MyApp ./MyApp.Test  # DAP on 127.0.0.1:4711
```

- `--port [N]` listens on `127.0.0.1` only (default 4711, `0` picks a free port and
  prints it on stderr). It takes one client.
- With `--stdio`, stdout carries only DAP frames.
- The options of `test` work the same way, among them `--filter`, `--codeunit` and
  `--bc-version`. `--output-json`, `--impact-of`, `--recipe` and `--explain` are refused.
- Breakpoints and steps work in your apps and in dependency apps whose source is compiled,
  Microsoft's apps included. A breakpoint on a statement no selected test reaches comes
  back unverified, with the reason.

The complete protocol is in [docs/debugging.md](../debugging.md).

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

Watch mode is not part of AL Runner 3: it was rejected for 3.0.0, because `serve` already
keeps the compile warm. For re-running only what a change affects, use `--affected` (a
command line option and a `serve` field).
