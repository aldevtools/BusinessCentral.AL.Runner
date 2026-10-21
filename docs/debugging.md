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
  `the app did not compile`, `no executable AL statement on this line in this file` or
  `no source file for this app`.
- A new `setBreakpoints` for a file replaces that file's breakpoints. The old `lines` field works too.
- Breakpoints work in the apps given on the command line. Breakpoints in a dependency are not supported yet:
  they come back unverified.
- No conditions, hit counts or log points (as v2).

## Stepping

`next`, `stepIn` and `stepOut` work on the AL call stack: `next` stops at the next statement of the same
routine or a caller, `stepIn` at the next statement anywhere, `stepOut` at the next statement of a caller.
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
