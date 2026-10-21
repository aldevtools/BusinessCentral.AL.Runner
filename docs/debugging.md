# Debugging AL tests: `al-runner debug`

`al-runner debug` is a [Debug Adapter Protocol](https://microsoft.github.io/debug-adapter-protocol/overview)
(DAP) server. It compiles the apps, waits for one client, runs the selected tests once, and exits. While the
tests run, the client can set breakpoints on AL lines, step, and read the call stack and the local variables.
It speaks the same DAP as AL Runner v2's `--dap`, so a client written for v2 needs only the new command line.
The design is [ADR-048](adr/048-dap-debugging.md).

## Command line

```
al-runner debug --stdio ./MyApp ./MyApp.Test     # DAP on stdin and stdout (a client that starts al-runner)
al-runner debug --port 4711 ./MyApp ./MyApp.Test # DAP on 127.0.0.1:4711 (a client that connects)
al-runner debug ./MyApp ./MyApp.Test             # the same as --port 4711
al-runner debug --port 0 ./MyApp                 # a free port; the port is printed on stderr
```

- The options of `al-runner test` work the same way, among them `--test`, `--codeunit`, `--exclude-test`,
  `--dep`, `--package-cache` and `--bc-version`. v2 always ran every test.
- Refused, with exit code 2: `--output-json`, `--impact-of`, `--recipe`, `--explain`.
- The tests run one after the other, in one worker.
- The TCP port listens on `127.0.0.1` only, and takes one client.
- With `--stdio`, stdout carries only DAP frames. Everything else goes to stderr.
- On stderr: `al-runner debug: listening on 127.0.0.1:PORT` or `al-runner debug: waiting for a client on
  stdin`, and one `[debug] Codeunit.Method: Pass|Fail|NotRun` line per finished test. v2 printed its
  listening line on stdout.

## Session

1. `initialize`: the answer has `supportsConfigurationDoneRequest`, then an `initialized` event follows.
   `linesStartAt1` and `columnsStartAt1` are honored (default 1-based). A second `initialize` is refused.
2. `launch` or `attach`: answered when the compile is done. A compile error answers `success: false` with the
   first error.
3. `setBreakpoints` per source file. It may come before `launch`; it is then answered once the compile is done.
4. `configurationDone` starts the tests.
5. At a stop: a `stopped` event (reason `breakpoint`, `step` or `pause`, with v2's extra `line` field), then
   `threads`, `stackTrace`, `scopes`, `variables`, and `continue`, `next`, `stepIn`, `stepOut`.
6. Each finished test sends an `output` event (category `console`) with its `[debug]` line.
7. At the end: `terminated`, then `exited` with the exit code `al-runner test` would return. Then the process
   exits.

`pause` stops the running test at its next statement (v2 continued instead). `setExceptionBreakpoints` answers
success with no filters. `disconnect` and `terminate` end the session. Every other request, among them
`evaluate`, `setVariable` and `source`, answers `unsupported command: NAME`, as in v2.

## Breakpoints

- A breakpoint binds to the statements that start on its line. Every statement on the line is armed, in every
  object of the file, and the run stops at each one it reaches.
- A `column` narrows this to the statement that starts at the column, else the statement that holds it, else
  the whole line. The answer reports the column it bound.
- A line with no statement is not moved to another line. It comes back `verified: false` with a `message`:
  `the app did not compile`, `no executable AL statement on this line in this file`,
  `the selected tests do not reach this statement` (see dependencies below) or `no source file for this app`.
- A new `setBreakpoints` for a file replaces that file's breakpoints. The old `lines` field works too.
- No conditions, hit counts or log points (as v2).

### Breakpoints in dependencies

Breakpoints and steps work in every app whose source AL Runner compiles: the apps on the command line and the
dependency apps, among them Microsoft's apps, which AL Runner compiles from their source packages.

- **A dependency given as a folder** (`--dep ./MyLib`): its breakpoints are set on its files, as for the apps on the
  command line, and the stack shows those files.
- **A dependency read from a source package** (`*.Source.zip`, or an `.app` that holds its source, for example
  Microsoft's apps from the artifact cache): the package has no file on disk. When the debugger stops in one of its
  files, it writes the file, with the text that was compiled, to
  `<cache>/debug-sources/<publisher>_<name>_<version>/<path in the package>` (the cache folder of `--cache-dir`),
  and the stack shows that file. A breakpoint on that file binds, also in a later session.
- **A checkout of a package's source** (for example a clone of Microsoft's BCApps repository): a breakpoint on a file
  whose path ends with the path of a file in the package binds when the file's text is the text that was compiled.
  The stack then shows the checkout's file. When the text differs, the breakpoint comes back unverified with
  `this file's text is not the text that was compiled for PATH`, because its lines may not match.
- AL Runner compiles a dependency only where the selected tests reach it (ADR-013). A statement no selected test
  reaches cannot stop; its breakpoint comes back unverified with `the selected tests do not reach this statement`.
- `stepIn` enters dependency code. A dependency read from an `.app` without source has no statements to stop at.
## Stepping

`next`, `stepIn` and `stepOut` work on the AL call stack: `next` stops at the next statement of the same
routine or a caller, `stepIn` at the next statement anywhere (dependency code included), `stepOut` at the next
statement of a caller.
Microsoft's install code (the install baseline) is not debugged.

## Stack and variables

- One stack frame per AL routine on the call stack, named `Object.Routine`, with its file, line and column.
  A frame with no source file has no `source` and line 0.
- One scope, `Locals`, with the named local variables and parameters of the frame (as v2).
- Values are written as in v2: a number bare, a text in JSON quotes, a record as its primary key. A value of a
  type that is not rendered yet shows `<not rendered: KIND>`; it is never guessed.
- No globals scope and no expandable records (as v2).

## What does not change

A test gives the same outcome with and without the debugger: the debug run executes the same operations
between two statements, and the test clock steps per read (ADR-045). The per-test time limit does not count the
time a test is stopped. Without `al-runner debug`, a test run pays nothing for the debugger.

A repository test checks this: `every_outcome_is_the_same_with_and_without_the_debugger` in
`crates/alrunner-cli/tests/debug.rs` runs the fixture app `tests/fixtures/debug/outcomes` (passes, failures, a
runtime error, `asserterror`, a TryFunction, records and events, `Codeunit.Run`, handlers, a refused test) once with
`al-runner test` and once with `al-runner debug --stdio`, with a breakpoint on every line of every file. At each stop
the client reads the stack and the locals of every frame and resumes with the next of `continue`, `next`, `stepIn`
and `stepOut`. The two JUnit reports must be the same apart from times, and so must the exit codes. The same check
runs on any app:

```
ALRUNNER_DEBUG_PARITY_APP=corpus/tests/al-language \
ALRUNNER_DEBUG_PARITY_ARGS="--bc-version 29.0 --dep corpus/tests/al-language-internals-fixture --codeunit ID" \
cargo test --release -p alrunner-cli --test debug -- --ignored outcomes_of_a_given_app
```

## Using the debugger from ALchemist

ALchemist (the VS Code extension) starts the runner as a debug adapter and speaks DAP with it over stdin and stdout.
For AL Runner it starts `al-runner debug --stdio` in place of v2's `al-runner --dap stdio`. The DAP messages stay
the same, so the rest of ALchemist's debugger code does not change.

### The command line

ALchemist creates the adapter with `new vscode.DebugAdapterExecutable(path, args)`. The arguments for AL Runner:

```
al-runner debug --stdio APP... [--dep PATH]... [--package-cache DIR]... [--test TEXT | --test-exact NAME]... [--codeunit ID]...
```

1. `debug --stdio` comes first, in place of `--dap stdio`.
2. `APP...` is the launch configuration's `sourcePaths` (or its `bundleDir`): each an app folder with an `app.json`, a
   `*.Source.zip` or an `.app` with source. v2 took one folder; AL Runner takes several, for example the app and its
   test app.
3. A dependency whose breakpoints the user wants to hit goes in as `--dep PATH`: an app folder or a source package. Microsoft's apps need nothing extra: AL Runner compiles them from source and shows their files from
   `<cache>/debug-sources/`.
4. `testFilter` of the launch configuration can now go to the runner: `--test TEXT` (names that contain TEXT),
   `--test-exact NAME` or `--codeunit ID` (both repeatable). v2 ran every test of the app.
5. Options of `al-runner test` work the same: `--package-cache`, `--bc-version`, `--no-test-data`, `--country`.

The exit code of the run (as `al-runner test` would return it) is in the `exited` event. Then the process ends.

### What differs from v2's `--dap`

| | v2 `--dap stdio` | AL Runner `debug --stdio` |
|---|---|---|
| Start | `al-runner --dap stdio DIR` | `al-runner debug --stdio APP...` |
| Tests that run | all tests of the app | the selection (`--test`, `--codeunit`), else all |
| `pause` | continues | stops at the next statement (`stopped`, reason `pause`) |
| `setExceptionBreakpoints` | `unsupported command` | success, no filters |
| Frame names | BC's generated scope class name | `Object.Routine` |
| A value it cannot write | `<unreadable: ...>` | `<not rendered: KIND>` |
| Breakpoints in dependency source | folders | folders and source packages, Microsoft's apps included |
| Per-test lines | `[dap] Codeunit.Method: Outcome` on the console (stderr with stdio), no DAP event | `[debug] Codeunit.Method: Outcome` on stderr and as an `output` event (category `console`) |
| Same outcomes as a test run | not checked | checked by a repository test ("What does not change") |

The rest is the same: `initialize` capabilities, `launch` waiting for the compile, breakpoints before `launch`, the
reasons of unverified breakpoints, `stopped` with the extra `line` field, one thread, the `Locals` scope, and
`unsupported command: NAME` for `evaluate`, `setVariable` and `source`.

### Checking a client

`crates/alrunner-cli/tests/debug.rs` holds scripted DAP sessions in the shapes ALchemist sends. To try a session by
hand, start `al-runner debug --port 0 APP` and connect a client to the port it prints on stderr.
