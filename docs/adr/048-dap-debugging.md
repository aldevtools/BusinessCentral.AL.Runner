# ADR-048: `alrunner debug`: Debug Adapter Protocol over stdio or TCP

- Status: Proposed
- Date: 2026-10-21
- Related: ADR-016 (gate R item 5, step 6 of the owner decision of 2026-10-15, owner decision 2026-10-21 that 3.0.0
  needs v2's features), ADR-002 (synchronous VM), ADR-006 (core purity), ADR-030 (value rendering), ADR-034
  (executable form), ADR-041 (positions in dependency IR), ADR-045 (stepping test clock)
- Parity rows: ED-9 (`--dap`), ED-10 (ALchemist)

## Context

ADR-016 names `alrunner debug [--stdio | --port N] PATH` as the replacement of v2's `--dap`. Gate R item 5 says
`debug` passes the same DAP script that v2's tests use. Today `alrunner debug` answers "not implemented yet"
(`args.rs`).

### What v2 does

All references are to the `v2` branch.

- **Start.** `--dap [PORT]` listens on `127.0.0.1:PORT` (default 4711) for one client. `--dap stdio` speaks on stdin
  and stdout and moves console output to stderr. Exactly one app path. `--dap` with `--server` or `--tdd` is refused
  (exit 2). `Program.cs` around lines 150-199, 803, 2675-2688.
- **One session, one run.** `RunDapLoop` (`Program.cs` around 7306-7965) compiles at once, waits for
  `configurationDone`, then runs **all tests** of the app. There is no test filter. After each test it prints
  `[dap] Codeunit.Method: Outcome`. It exits when the run ends.
- **Requests** (switch around `Program.cs` 7728-7948):
  - `initialize`: reads `linesStartAt1` and `columnsStartAt1` (default true). Capabilities:
    `{supportsConfigurationDoneRequest: true}`. Sends `initialized`. A second `initialize` is refused.
  - `launch` and `attach`: the same; arguments ignored. The answer waits for the compile. A compile error answers
    `success: false` with the first diagnostic.
  - `setBreakpoints`: `source.path` and `breakpoints[{line, column?}]` or the old `lines[]`. Answer
    `breakpoints[{id, verified, line, column?, message?}]`.
  - `configurationDone`: starts the tests.
  - `threads`: one thread `{id: 1, name: "AL Test Thread"}`.
  - `stackTrace`: frames `{id, name, source{path, name}, line, column: 1}`, `totalFrames`. "not stopped" when running.
  - `scopes`: one scope, `Locals`. `variables`: `{name, value, variablesReference: 0}`. Handles reset at every stop.
  - `continue`, `next`, `stepIn`, `stepOut`. `pause` only continues.
  - `disconnect`, `terminate`: release a paused test, cancel, send `terminated` and `exited`.
  - Anything else (`setExceptionBreakpoints`, `evaluate`, `source`, `setVariable`): `unsupported command: X`.
- **Events.** `initialized`; `stopped {reason: "breakpoint" | "step", threadId: 1, allThreadsStopped: true, line}`
  (`line` is a v2 extension); `output` on stderr category only when the stack walk fails; `terminated`, then
  `exited {exitCode}`.
- **Breakpoints** (`Infrastructure/DapBreakpointResolver.cs`, `DapUnverifiedReason.cs`): the line must be the line of
  an executable statement, no nearest-line fallback. Every statement on the line is armed, in every object of the
  file. A column narrows it: a statement that starts there, then one that contains it, then the whole line.
  Unverified breakpoints carry a reason. Breakpoints sent before `launch` are held and answered when the compile is
  ready. Replacement is per file. Paths compare case-sensitive on Linux, case-insensitive on Windows and macOS.
  Breakpoints in source dependencies work.
- **Stepping** by call depth: `stepIn` stops at the next statement, `next` at depth <= D, `stepOut` at depth < D.
  A step that lands on a breakpoint reports `breakpoint`.
- **Values** (`AlValueWireFormat.ToWireValue`): primitives as they are, anything else by `ToString()`, then JSON
  encoded, so a number shows as `1` and a text as `"abc"` with quotes. Unreadable: `<unreadable: ...>`.
- **Wire** (`Infrastructure/DapTransport.cs`): `Content-Length` framing, UTF-8 JSON, nulls left out, one write lock.
  On stdio, stdout carries only frames.
- **A paused test does not time out** (`DapServerTests`: a pause longer than the watchdog).
- **Tests.** `DapServerTests`, `DapStdioServerTests`, `DapBreakpointReplacementTests`, `DapContainerTests`,
  `DapDependencySourceBreakpointTests`, `DapLineBaseTests`, `DapMultiObjectFileTests`, `DapMultiTargetLineTests`,
  `DapPreLaunchBreakpointTests`, `DapStackScopesWireTests`, `DapRawFrameValidatorTests`, fixtures `DapBreakpoint`,
  `DapStepping`, `DapMultiTarget`, `DapTwoObjects`. There is no script file: each test sends a fixed sequence, for
  example `initialize`, `launch`, `setBreakpoints`, `configurationDone`, wait `stopped`, `stackTrace`, `scopes`,
  `variables` (assert `Counter` is `1`), `continue`, wait `exited` with exit code 0.
- **Tools.** `docs/dap-mode.md` is the user doc. No evidence that ALchemist drives DAP today; it uses the server.
  So the main client is VS Code through a small debugger contribution.

### What AL Runner has already

- Every statement has a position marker (`Stmt::Pos`) with file, line and column. Coverage and value capture use
  them (ADR-030).
- With capture on, the VM compiles a separate executable form with extra ops per statement (`Op::CapHit`,
  `Op::CapBegin`, `Op::CapEnd` in `al-vm/src/exec/compile.rs`), so a run without capture pays nothing.
- The VM keeps an AL call stack with `file:line:col` per frame (`callstack.rs`, `faultsite.rs`).
- The value renderer of ADR-030 turns a value into JSON without guessing.
- Dependency routines carry source positions (ADR-041).
- The test clock steps per read, not by wall time (ADR-045), so a pause does not change what a test sees.

## Decision

1. **Surface.** `alrunner debug [--stdio | --port N] PATH... [test options]`. `--port` without a number uses 4711,
   bound to `127.0.0.1` only. The selection options of `alrunner test` (`--test`, `--codeunit`, `--exclude-tests`)
   are accepted, an improvement on v2, which always runs every test. All other run options mean what they mean on
   `alrunner test`. The migration guide maps `--dap stdio` and `--dap N`.
2. **Where it lives.**
   - `al-vm` gets an additive debug hook: `Session::debug: Option<Box<dyn DebugHook>>`. The trait has one method,
     called before each statement in the debug executable form: it gets the position id and a read-only view of the
     frames (routine name, position, named locals) and returns `Continue` or blocks until the client resumes. It
     does no OS access; blocking is a channel receive inside the implementation (ADR-006 holds).
   - The debug executable form is a third compile of a routine, like the capture form: one `Op::DbgHit { p }` per
     statement. A normal run never sees it and pays nothing.
   - `alrunner-cli/src/debug/` (new) owns the wire (framing, stdio, TCP), the session state machine, breakpoint
     resolution against the program's position table, and the hook implementation.
3. **Threads.** The reader thread reads frames and answers requests. One worker thread runs the tests one after the
   other (a debug run is always one worker). The hook blocks the worker on a channel when it stops and sends the
   `stopped` event. `stackTrace`, `scopes` and `variables` are answered from a snapshot the worker takes when it
   stops, so the reader never touches the VM.
4. **Lifecycle as v2.** The compile starts at once. `launch` and `attach` answer when it is done (a compile error:
   `success: false` with the first diagnostic). Breakpoints sent before the compile is done are held and answered
   then. `configurationDone` starts the run. Each finished test prints `[debug] Codeunit.Method: Outcome` as an
   `output` event (category `console`) and on stderr. At the end: `terminated`, then `exited {exitCode}` with the
   exit code `alrunner test` would give. Then the process exits.
5. **Requests.** The v2 set with the same answers, plus:
   - `pause` really pauses at the next statement (`stopped` with reason `pause`). v2 continued instead.
   - `setExceptionBreakpoints` answers success with no filters, so VS Code does not log an error.
   - `stopped` keeps v2's extra `line` field.
   - `evaluate`, `setVariable`, conditional breakpoints, `source` and stop-on-error stay `unsupported command: X`,
     as v2. See open question 2.
6. **Breakpoints as v2.** Exact line of a statement position, every statement on the line in every object of the
   file, column narrowing, a reason when unverified ("the app did not compile", "no executable AL statement on this
   line in this file", "no source file for this app"), replacement per file, path comparison by OS. Breakpoints in a
   dependency given as source work through its positions (ADR-041). A dependency read from an `.app` has no file on
   disk: its breakpoints are unverified with the reason above.
7. **Stepping as v2**, on the depth of the AL call stack. Statements of dependency apps are stepped into only when
   the dependency is given as source; otherwise `stepIn` steps over them. The install baseline is not debugged.
8. **Scopes and values.** One scope `Locals` with the named locals and parameters of the frame, as v2. Values come
   from the ADR-030 renderer and are written in v2's form: a number bare, a text in JSON quotes, a record as its
   primary key. A value the renderer does not render yet shows `<not rendered: KIND>`, never a made-up value.
   See open question 1 for globals and expandable records.
9. **Time limits.** The per-test time limit (`--test-timeout-secs`) stops counting while the test is paused.
10. **Values the test sees do not change** when it is debugged: the test clock steps per read (ADR-045), and the
    debug form runs the same ops between hits. A test gives the same outcome with and without the debugger
    (conformance test 9).

## What is refused

- `debug` together with `serve`, `--watch`, `--affected`, `--output-json`, `--jobs` above 1 and `--impact-of`
  (exit 2, with the reason).
- More than one client per process (a second TCP connection is closed).
- The WebAssembly hosts: no TCP and no blocking thread (ADR-016 already lists it).
- `debug` with `--tdd` is allowed: a stub (ADR-051) has no statements, so stepping over it is all there is. v2
  refused the pair because of how it built stubs.

## Test plan (red, then green)

All in `crates/alrunner-cli/tests/debug.rs`, with a small DAP client in the test crate (TCP and stdio). The four v2
fixtures (`DapBreakpoint`, `DapStepping`, `DapMultiTarget`, `DapTwoObjects`) are re-created as AL fixtures in the
test crate. Each v2 test sequence becomes one Rust test:

1. Breakpoint, stack, `Locals` with `Counter` = `1`, continue, `exited` 0 (`DapServerTests`). Red: today "not
   implemented yet".
2. The same over stdio, with every stdout byte a valid frame (`DapStdioServerTests`, `DapRawFrameValidatorTests`).
3. `next`, `stepIn`, `stepOut` stop where v2 stops.
4. 0-based lines and columns, a second `initialize` refused, the old `lines[]` field (`DapLineBaseTests`).
5. Breakpoints before `launch`, and `disconnect` while the compile runs (`DapPreLaunchBreakpointTests`).
6. Replacement per file (`DapBreakpointReplacementTests`).
7. Two objects in one file, several statements on one line, column breakpoints (`DapMultiObjectFileTests`,
   `DapMultiTargetLineTests`).
8. A dependency given as source (`DapDependencySourceBreakpointTests`); a dependency from an `.app` gives an
   unverified breakpoint with the reason.
9. A pause longer than the test time limit does not fail the test; outcomes with and without the debugger are the
   same for every test of a corpus area.
10. `pause` stops; `setExceptionBreakpoints` succeeds; `evaluate` answers `unsupported command`.
11. A compile error answers `launch` with `success: false`.
12. Refused combinations exit 2.
13. Unit tests: framing (split headers, wrong length), the breakpoint resolver on a position table, the value form.
14. Performance: a run without `debug` executes the same ops as before (the VM speed budget of ADR-031 holds).

## Size

About 7 agent-days: 1.5 for the hook and the debug executable form in `al-vm`; 1 for the wire and the session; 1.5
for breakpoints and stepping; 1 for stack, scopes and values; 2 for the v2 test sequences, fixtures and docs (user
doc, migration guide, parity rows ED-9 and ED-10). A small VS Code debugger contribution for ALchemist is not in
this estimate.

## Consequences

- One additive seam: `DebugHook` in `al-vm`. It is opt-in and costs nothing when off.
- Gate R item 5 can be checked by the ported v2 sequences.
- Because the debug run uses the stepping test clock, a test behaves the same under the debugger. v2 could not
  promise that.

## Open questions for the owner

1. Variables: `Locals` only as v2 (recommended for 3.0) or also a `Globals` scope and expandable record fields
   (about 1.5 more days).
2. `evaluate` and conditional breakpoints: not in 3.0 as v2 (recommended) or in 3.0 (needs an expression compile at
   a stop, about 3 days).
3. Who ships the VS Code side: ALchemist adds a debugger contribution that starts `alrunner debug --stdio`
   (recommended) or AL Runner ships its own small extension.
4. One session runs all selected tests and exits, as v2 (recommended), or the process stays for several launches.

## Conformance

`crates/alrunner-cli/tests/debug.rs` (the ported v2 sequences above) and the unit tests in
`crates/alrunner-cli/src/debug/`.
