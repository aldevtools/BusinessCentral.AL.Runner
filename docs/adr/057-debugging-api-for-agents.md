# ADR-057: A debugging API for AI agents

- Status: Proposed
- Date: 2026-10-29
- Related: ADR-048 (`al-runner debug`, the `DebugHook` seam), ADR-030 (value renderer, loop passes), ADR-028 (`serve`),
  ADR-029 (incremental compile), ADR-004 (snapshot store), ADR-006 (core purity), ADR-009 (observable equivalence),
  ADR-027 (scope), ADR-031 (VM speed budget), ADR-045 (stepping test clock), ADR-051 (`stubbed` tests),
  ADR-056 (in-memory file system)

## Context

The owner asked (2026-10-29) for an API that lets an AI agent debug AL tests through the runtime. An agent does not
click through a stack view. It asks precise questions: "run test X to line 120 when `Qty > 5`, and give me the stack,
the locals, the `Sales Line` record and the first 20 rows of `Item Ledger Entry`". It also wants to try a fix without
editing code ("set `Qty` to 0 and continue"), and to go back ("what was `Amount` two passes of this loop ago?").

All features below are in scope (owner, 2026-10-29):

1. Stop on error: any error, uncaught errors only, and errors inside a TryFunction or `asserterror` as options.
2. Inspect: expression evaluation without side effects, full record values (all fields, filters, key, marks,
   temporary state), globals, and the database rows of a table at a stop (filters, paging).
3. Change values in memory at a stop: locals, globals, record fields, and (to decide) database rows. A test whose
   state the debugger changed is reported as such, never as a plain pass.
4. Step back: one statement back, reverse continue to a breakpoint, back N loop passes or to pass K.
5. Conditional breakpoints, hit counts, log points.
6. Warm sessions: debugging inside `serve`, with the compile kept and several runs per session.
7. A front end for agents (a JSON request in `serve` and/or an MCP server), and DAP kept for editors.

### What exists today

- **The seam.** `crates/al-vm/src/debug.rs`: `DebugHook::statement(&mut self, &DebugView) -> Resume` is called before
  each statement of the debug form. `Resume` has one variant, `Continue`, and is `#[non_exhaustive]` (ADR-048,
  "decided without owner review" item 4: more variants are additive). `DebugView` gives the position id, the call
  depth, the program and the frames. The innermost frame's locals are read live; outer frames' locals are copies
  rendered when their last statement began (`Debugger::outer`).
- **No cost when off.** The debug form is a third compile of each routine (`ExecProgram::debug_chunk`,
  `crates/al-vm/src/exec/compile.rs` around line 901: one `Op::DbgHit { p }` after each `Op::Pos`).
  `Session::call` (`crates/al-vm/src/lib.rs`, the `match self.form` after the frame push) picks the plain, captured or
  debug chunk. A session without a debugger runs the plain chunk, which has no `DbgHit`. `Session::debug_hit` is
  `#[cold]` and `#[inline(never)]` so the op loop's other arms do not pay for it
  (`crates/al-vm/src/exec/run.rs` around line 285). The VM speed budget (ADR-031) holds this in CI.
- **The DAP front end.** `crates/alrunner-cli/src/debug/` (`mod.rs`, `wire.rs`, `breakpoints.rs`). A reader thread
  answers requests; the test worker blocks inside `Hook::statement` on a condition variable while stopped;
  `stackTrace`, `scopes` and `variables` are answered from a snapshot taken at the stop. `evaluate`, `setVariable`,
  conditional breakpoints and stop-on-error answer `unsupported command` (ADR-048 decision 5, owner 2026-10-21).
  `setExceptionBreakpoints` answers success with no filters. One `Locals` scope; a record shows as its primary key.
- **Values.** `crates/al-vm/src/capture.rs`: the ADR-030 renderer (`Rendered`: null, bool, int, decimal digits,
  text, or `Unrendered(kind)`). A record is rendered as its primary key. Lists, codeunit variables and .NET values are
  `Unrendered`.
- **Records.** `RecordState` (`crates/al-builtins/src/value.rs` around line 1016) holds the table, `temporary`,
  `temp_id`, `company`, `fields`, `filters` and `filter_group`, `marks` and `marked_only`, `key`, `ascending` and
  more. Everything a full record view needs is there.
- **Catching constructs.** `asserterror` is `op_assert` (`exec/run.rs` around line 458). A TryFunction raises
  `tx.try_depth` in `Session::call`. `[ErrorBehavior(Collect)]` opens `error_scopes`. `Codeunit.Run` with a used
  return value catches in the dispatch code. No single place knows "will this error be caught".
- **Determinism.** The store is a persistent map with O(1) snapshots (ADR-004, ADR-035). The clock is a session
  counter on top of a host instant fixed per run (ADR-045 and its amendment: noon UTC of the day of the first read).
  `CreateGuid` and SystemIds come from `guid_seed` and a counter (`crates/al-vm/src/ctx.rs`, `fresh_guid`). `Random`
  is .NET's seeded generator with a host seed (`crates/al-host/src/random.rs`). AL `File` and .NET file access run on
  an in-memory file system per run (ADR-056). ADR-043 already relies on this determinism ("Why it is exact").
- **Serve.** `crates/alrunner-cli/src/serve.rs`: a reader thread answers `cancel` at once while a run keeps the main
  thread busy (around lines 79-110); other requests queue. The warm slot keeps the compile, the engines and the
  install baseline (ADR-028, ADR-029).
- **Session state.** `Session` (`crates/al-vm/src/lib.rs` around line 610) has more than 100 fields: `Rc`/`RefCell`
  values, record handles, page clients, event state, .NET handles. It is not `Clone`, and making it cloneable is a
  large change.

## Decision

### 1. Stop on error (exception breakpoints)

1. **Filters.** The debugger offers these exception filters. Several can be on at once.

   | Filter id | Stops when an error is raised and |
   |---|---|
   | `uncaught` (default on) | nothing on the AL stack catches it: it will end the test as a failure |
   | `all` | always |
   | `tryFunction` | the nearest catcher is a TryFunction whose result is used |
   | `assertError` | the nearest catcher is `asserterror` |
   | `codeunitRun` | the nearest catcher is `if Codeunit.Run(...)` (a used return value) |
   | `collected` | the error is collected by `[ErrorBehavior(Collect)]` and does not unwind |

2. **Where it stops.** At the statement that raised the error, before the frame unwinds, with the innermost frame's
   locals live. The stop reason is `exception`. The stop carries the error message, error code, the AL call stack and
   the filter that matched.
3. **How "caught" is known.** The debug form keeps a stack of catch scopes: it compiles `asserterror`, TryFunction
   calls, `Codeunit.Run` with a used result and collecting routines with an extra `Op::DbgCatch { kind }` on entry
   and `Op::DbgCatchEnd` on exit. At a raise the innermost catch scope gives the kind; an empty stack means uncaught.
   The plain form gets no new op.
4. **Where it is raised from.** The debug form's op loop calls a new `#[cold]` `Session::debug_error` when an op
   returns `Err(Signal::Error(_))` for the first time (the error carries a "seen" flag, so an error that passes up
   through frames stops once). `DebugHook` gets a new defaulted method `error(&mut self, view, &ErrorInfo) -> Resume`
   (default: `Continue`). A refusal (`Signal::Unsupported`) also stops when the filter `all` is on, with reason
   `refused`, so an agent sees why a test was not run.
5. **After the stop.** `continue` lets the error unwind as normal. The outcome does not change.

### 2. Inspect

1. **Full record values.** A record value is shown as an object, not only as its primary key:

   ```json
   {"table": 37, "tableName": "Sales Line", "temporary": false, "company": null,
    "key": ["Document Type", "Document No.", "Line No."], "ascending": true,
    "fields": {"Document Type": "Order", "Document No.": "1001", "Line No.": 10000, "Quantity": 5,
               "Amount": {"flowField": true, "calculated": false, "value": 0}},
    "filters": [{"group": 0, "field": "Document No.", "filter": "1001"}],
    "markedOnly": false, "marks": 0, "position": "Document Type=CONST(Order),Document No.=CONST(1001),Line No.=CONST(10000)",
    "xRec": null}
   ```

   - Every field in field order, with its name. A FlowField shows the value in the buffer and whether
     `CalcFields` ran on it. `calculate: true` in a request computes it on a copy of the record, so the buffer is not
     touched.
   - Filters of every filter group, the current key and order, `MarkedOnly`, the number of marks (the marked keys
     with `marks: "list"`), `temporary` and, for a temporary record, the number of rows in its private table.
   - `xRec` in a trigger frame that has one.
   - A Blob or Media field shows its length and `hasValue`, not its bytes.
2. **Globals.** A `Globals` scope per frame: the globals of the frame's object instance (codeunit, table instance
   per record variable as in ADR-021, page, report). SingleInstance codeunits that were created show under a
   separate `SingleInstance` scope.
3. **More kinds rendered.** The renderer learns List, Dictionary (as JSON arrays and objects, paged when large),
   codeunit and interface variables (the object name), JsonObject and friends (their JSON text), and
   TextBuilder (its text). A kind that is still not rendered stays `<not rendered: KIND>`.
4. **Database rows.** At a stop an agent can read the rows of a table as the stopped session sees them (its open
   transaction included):

   ```json
   {"table": "Item Ledger Entry", "company": null, "filters": {"Item No.": "1000", "Posting Date": "010125..311225"},
    "key": ["Item No.", "Posting Date"], "descending": false, "fields": ["Entry No.", "Quantity"],
    "offset": 0, "limit": 20}
   ```

   The answer has `rows`, `total` (the number of rows that match) and `more`. Filters use AL filter syntax and the
   runtime's own filter parser. A temporary table is read through the record variable that owns it
   (`{"variable": "TempSalesLine"}`). The read goes to the store directly. It does not move the session's SQL
   counters, record caches or the clock. FlowFields are left out unless asked for, and then computed read-only.
5. **Expression evaluation without side effects.**
   - **Compile.** The expression is parsed and checked in the scope of the stopped frame: its locals, its object's
     globals, and the program's objects. It is lowered against the running program into a separate routine that is
     not part of `Program` (the running engine stays immutable, ADR-001). It is cached per (position, text).
   - **Purity, checked before it runs.** A walker over the lowered expression accepts only: reads of variables,
     fields and constants; operators; record methods that only read the buffer (`GetFilter`, `GetFilters`,
     `FieldNo`, `HasFilter`, `IsTemporary`, `GetPosition`, `TableCaption` and the like); store reads that do not
     move a cursor (`Count`, `IsEmpty`) run on a copy of the record; and builtins on an allow-list
     (`PURE_BUILTINS` in `al-builtins`, for example `Format`, `StrLen`, `CopyStr`, `Round`, `Abs`, `CalcDate`).
     Everything else is refused before it runs, with the reason: procedure calls, assignments, `var` arguments,
     store writes, cursor moves (`Find`, `Next`), `Today`, `Time`, `CurrentDateTime`, `CreateGuid`, `Random`,
     `DotNet`, UI and events.
   - **Purity, checked after it runs.** The session takes a fingerprint before and after the evaluation: the store's
     root (a pointer compare, ADR-004), the clock counter, `next_guid`, `random_draws`, `calls_made`, the number of
     messages and the SQL counters. When any of them differs, the evaluation is reported as having changed state
     and the test is marked as changed by the debugger (section 3). This should never happen. The check makes a
     mistake in the allow-list visible instead of silent.
   - **Evaluation with side effects, on a copy.** `sideEffects: "fork"` evaluates any expression (procedure calls
     included) on a copy of the session reached by re-execution (section 4) and throws the copy away. The real
     session is not touched. Refused where re-execution is refused.

### 3. Change values in memory

1. **What can be changed.** At a stop:
   - locals and parameters of any frame on the stack;
   - globals of any object instance on the stack, and of SingleInstance codeunits;
   - fields of a record variable (the buffer only: no `Validate`, no trigger);
   - filters, key and `MarkedOnly` of a record variable;
   - rows of the database (recommended, see open question 2): insert, modify and delete of one row, raw, with no
     trigger and no event, inside the session's open transaction, so they roll back with it.
2. **Values** are given as JSON in the form the renderer writes (section 2) and converted with the AL conversion
   rules of the target type. A value that does not convert is refused with the conversion error.
3. **Outer frames.** Today outer frames are copies rendered at each statement (`Debugger::outer`), which cannot be
   changed and cost a render per statement. The debug form will keep the locals of every frame reachable from the
   session while a debugger is attached (a stack of frame references kept only by the debug form). Reads and
   changes are then live in every frame, and the per-statement render goes away.
4. **How the change reaches the VM.** The hook stays the only place that blocks. It returns a new
   `Resume::Act(DebugAction)` (`SetValue`, `Evaluate`, `ReadRows`, `Rewind`, ...). The session performs the action
   with `&mut` access, calls the new defaulted `DebugHook::result(&mut self, view, outcome)`, and then calls
   `statement` again at the same position without running anything. So the client stays stopped and sees the new
   state.
5. **Reported as changed.** A test in which the debugger changed state (any `SetValue`, a row change, or an
   evaluation whose fingerprint differed) is marked `changedByDebugger: true`:
   - the `test` line in serve and `--output-json` get `changedByDebugger: true` and the list of changes
     (`{step, target, from, to}`);
   - the outcome text is `pass (state changed by the debugger)` or `fail (state changed by the debugger)`, in the DAP
     `[debug]` line too;
   - the summary counts these tests apart (`changedByDebugger`), as ADR-051 counts `stubbed` tests;
   - the exit code treats a changed pass as not passed (exit code 1), so no script mistakes it for a green run;
   - the per-test JUnit case gets a `<property name="changedByDebugger" value="true"/>`.
   A change on a timeline that a later step back dropped (section 4) does not count.

### 4. Step back: deterministic re-execution

Three options were compared:

- **(a) Re-execution from the start of the test codeunit** to step N. The runtime is deterministic, so the same
  inputs give the same steps. Memory: a few counters. Time: one run of the prefix per step back (two for reverse
  continue). No change to how `Session` holds its state.
- **(b) Snapshots at intervals plus re-execution between them.** The store snapshot is O(1), but the rest of the
  session is not: `Session` holds `Rc`/`RefCell` values, record handles, page clients, .NET handles and event state.
  A snapshot needs a deep copy of all of that, kept in step with every new field. Large and easy to get wrong.
- **(c) Recording** every write (locals, fields, store) to undo them. The cost is paid on every statement of every
  debugged run, memory grows with the run, and every builtin that changes state needs an undo record. Largest.

**Decision: (a).** Later, if measurements show re-execution is too slow for long tests, (b) can be added for the
store alone (re-execution still needed for the session), behind the same interface.

1. **Step numbers.** The debugger counts every `DbgHit` from the start of the test codeunit's session: step 1 is the
   first statement. Every stop reports its `step`. A step number with its timeline (below) names a point exactly.
2. **Where re-execution starts.** Each test codeunit runs on a new session over the post-install database
   (`docs/manual/tool-protocol.md` section 4b, item 4). So re-execution starts a new session for the codeunit and runs
   from its start: `OnRun`, earlier test methods and handlers included, without stopping, to the target step. A
   test that ran under `--isolation test` still restarts at the codeunit start, because globals and SingleInstance
   state carry across its test methods.
3. **Requests.**
   - `stepBack`: to the last step before this one at this call depth or above (the reverse of `next`).
   - `stepBackInto`: to step N-1 (the reverse of `stepIn`).
   - `reverseContinue`: to the last step before this one where an armed breakpoint (with its condition and hit
     count) or an exception filter held.
   - `rewindLoop {back: N}` or `rewindLoop {pass: K}`: to the first statement of pass `current - N`, or of pass K, of
     the innermost loop that holds the current statement (`loop` names an outer loop by its loop id).
   - `restartFrame`: to the first statement of the chosen frame.
   - `runToStep {step: S}`: any earlier or later step.
4. **How a target is found.** A target that is a fixed step (`stepBackInto`, `runToStep`) needs one re-execution.
   A target defined by "the last step before N where P held" (`stepBack`, `reverseContinue`) needs two: the first
   runs to N and remembers the last step where P held; the second runs to that step. The debug form marks loop passes
   with `Op::DbgPass { loop }` (the ADR-030 loop ids), so the debugger knows the step where each pass of an active
   loop began; `rewindLoop` and `restartFrame` read the target step from that, one re-execution.
5. **Fast forward.** During re-execution the hook only counts steps and checks the target. It renders nothing
   (section 3.3 removes the per-statement render). Expected cost: the prefix time of the test in the debug form,
   measured in phase 4 and given in the stop as `rewindMs`.
6. **Divergence check.** The debugger keeps a running 64-bit hash of the position ids of every step, and records
   (step, position, hash) at every stop the client saw. During re-execution it compares at each recorded step. A
   difference stops the re-execution and refuses the step back with `re-execution diverged at step S` (a runtime bug
   to report). This catches non-determinism the list below missed.
7. **What makes step back refuse** (checked before it starts, with the reason):
   - A call into the .NET helper process (ADR-014 sidecar) since the codeunit started. The helper keeps state across
     calls and across codeunits, so a second run of the call may answer differently. Calls served in process
     (ADR-038, ADR-039) are deterministic and do not refuse.
   - `ALRUNNER_REAL_CLOCK=1`.
   - `--files-dir DIR` and a file access since the codeunit started (real files on disk). The in-memory file system
     (ADR-056) is copied at each codeunit start while a debugger is attached, and re-execution starts from that copy.
   - A test data provider that reads outside the process after the start (none today: `--test-data` loads from a
     file into memory, deterministically).
   - Changed code. Under `serve`, a recompile between the stop and the step back ends the timeline.
   The clock is kept equal: re-execution uses the host instant of the original run (ADR-045 amendment), so a run
   across midnight does not change the date.
8. **Changed values fork the timeline.** A timeline is the codeunit plus the ordered list of changes made at given
   steps. Re-execution to step S applies every change of the timeline with a step before S, at its step, before that
   statement runs, so the state after a change is reproduced. Stepping back to a step at or before a change drops that
   change and every later one. The answer lists them (`droppedChanges`). The test is marked as changed by the debugger
   only when its final timeline has a change.
9. **Time limits.** The per-test time limit does not count re-execution time, as it does not count stopped time
   (ADR-048 decision 9).

### 5. Conditional breakpoints, hit counts, log points

1. **Condition**: an expression checked as in section 2.5 (pure only). A condition that does not compile or is not
   pure comes back `verified: false` with the reason. A condition that raises an error at run time stops with reason
   `conditionError` and the error, rather than being skipped silently.
2. **Hit count**: `N` or `=N` (stop at the Nth hit), `>N`, `>=N`, `%N` (every Nth hit). Counted per breakpoint, per
   timeline (re-execution recounts).
3. **Log point**: a message with `{expression}` parts, each pure. It writes one `output` event (DAP) or one `log`
   line (serve) with the step and position, and does not stop.
4. **Data breakpoints** (later phase): watch a local, global or record field; stop after the statement that changed
   its value. Checked in the debug form only, after each statement of a frame that can reach the target.

### 6. Warm debug sessions in `serve`

1. A `debug` request in `serve` starts a debug session over the kept compile, engines and install baseline (ADR-028).
   It takes the fields of `runTests` that select tests (`sourcePaths`, `test`, `excludeTests`, `testIsolation`) plus
   debug fields. It answers with `{"type": "stop", ...}` lines while the run is stopped, `test` lines as tests finish,
   and a `summary`.
2. While a debug session is stopped, the reader thread answers its commands at once, as it answers `cancel` today.
   Other requests queue until the session ends.
3. **Several runs per session.** `debugRestart` runs the selection again over the same compile, keeping
   breakpoints. When the sources changed, the next `debugRestart` compiles incrementally (ADR-029) and resolves the
   breakpoints again by file and line; the old timeline ends.
4. One debug session at a time per server. It ends with `debugEnd`, `cancel`, or the end of the run.

### 7. Front ends

1. **Serve JSON (the agent front end, built first).** Each command names the session and gets one answer line.

   One-shot inspection, for an agent that wants a snapshot and nothing else:

   ```json
   {"command": "debug", "sourcePaths": ["./App", "./App.Test"], "test": "Codeunit50100.PostOrder_",
    "until": {"file": "src/SalesPost.Codeunit.al", "line": 120, "condition": "Qty > 5", "hit": ">=2"},
    "snapshot": {"frames": 3, "locals": true, "globals": true, "records": "full",
                 "rows": [{"table": "Item Ledger Entry", "filters": {"Item No.": "1000"}, "limit": 20}]},
    "then": "end"}
   ```

   `until` also takes `{"error": "uncaught"}` (or another filter id), `{"routine": "Sales-Post.PostLines"}`,
   `{"step": 4810}` and a list of those (the first that holds). `then` is `end` (stop the run, the default for a
   one-shot), `continue` or `stay` (keep the session stopped for more commands).

   The stop:

   ```json
   {"type": "stop", "session": 1, "reason": "breakpoint", "step": 4810, "timeline": 0,
    "location": {"file": "src/SalesPost.Codeunit.al", "line": 120, "column": 9, "routine": "Sales-Post.PostLines"},
    "test": {"codeunit": 50100, "method": "PostOrder_PostsItemLedger"},
    "stack": [{"frame": 0, "routine": "Sales-Post.PostLines", "file": "src/SalesPost.Codeunit.al", "line": 120,
               "locals": {"Qty": 6, "SalesLine": {"table": 37, "fields": {"...": "..."}}},
               "globals": {"GLSetupRead": true}}],
    "rows": [{"table": "Item Ledger Entry", "rows": [{"Entry No.": 1, "Quantity": -5}], "total": 1, "more": false}],
    "loops": [{"loop": 7, "file": "src/SalesPost.Codeunit.al", "line": 112, "pass": 3}],
    "error": null, "changedByDebugger": false}
   ```

   Commands on a stopped session (`"command": "debugStep"` and so on, each with `"session": 1`):

   | Command | Fields | Answer |
   |---|---|---|
   | `debugContinue` | `until` (optional), `snapshot` | the next `stop`, or the session's `test` lines and `summary` |
   | `debugStep` | `kind`: `over`, `in`, `out`; `snapshot` | `stop` |
   | `debugStepBack` | `kind`: `over` (default), `in`; `snapshot` | `stop` with `rewindMs`, or `error` with the refusal |
   | `debugReverseContinue` | `snapshot` | `stop` |
   | `debugRewindLoop` | `back` or `pass`, `loop` (optional) | `stop` |
   | `debugRunToStep` | `step` | `stop` |
   | `debugEvaluate` | `expression`, `frame`, `sideEffects`: `none` (default) or `fork` | `{"value": ...}` or `error` |
   | `debugSet` | `target` (`"Qty"`, `"SalesLine.Quantity"`, `"global:GLSetupRead"`), `frame`, `value` | `{"from", "to", "step"}` |
   | `debugRows` | as `rows` in `snapshot` | `{"rows", "total", "more"}` |
   | `debugSetRow` | `table`, `op`: `insert`, `modify`, `delete`; `key`; `values` | `{"done": true, "step"}` |
   | `debugBreakpoints` | `set`: list of `{file, line, column?, condition?, hit?, log?}`; `exceptions`: filter ids | the resolved list, as DAP |
   | `debugRestart` | | `stop` or `test` lines |
   | `debugEnd` | | `summary` |

   Every error is `{"error": "<reason>"}` and leaves the session stopped where it was.

2. **MCP server (recommended, after serve).** `al-runner mcp` speaks the Model Context Protocol on stdin and stdout.
   Each serve debug command is one MCP tool with the same fields (`debug_run_to`, `debug_step`, `debug_step_back`,
   `debug_evaluate`, `debug_set`, `debug_rows`, `debug_continue`, `debug_end`), plus `run_tests`. It is a thin
   adapter over the same handler as serve, with a small JSON-RPC layer written in the repository (no new
   dependency). See open question 3.
3. **DAP stays for editors.** Each feature maps to DAP where DAP has a request:

   | Feature | DAP |
   |---|---|
   | Exception filters | capability `exceptionBreakpointFilters` (the ids of section 1), `setExceptionBreakpoints`, `stopped` reason `exception`, `exceptionInfo` (`supportsExceptionInfoRequest`) |
   | Expression evaluation | `evaluate` (`watch`, `hover`, `repl`; `repl` with a leading `!` uses `sideEffects: fork`) |
   | Change a local or global | `setVariable` (`supportsSetVariable`) |
   | Change a record field | `setExpression` (`supportsSetExpression`) and `setVariable` on an expanded record |
   | Globals, full records | `scopes` (`Locals`, `Globals`, `SingleInstance`), `variables` with `variablesReference` > 0 |
   | Database rows | a `Database` scope: one entry per table the codeunit wrote, rows paged through `variables` `start` and `count` (`indexedVariables`); other tables through `evaluate` in `repl` with `:rows TABLE where FILTERS` |
   | Step back, reverse continue | `stepBack`, `reverseContinue` (`supportsStepBack`) |
   | Restart a frame | `restartFrame` (`supportsRestartFrame`) |
   | Conditions, hit counts, log points | `supportsConditionalBreakpoints`, `supportsHitConditionalBreakpoints`, `supportsLogPoints` |
   | Data breakpoints | `dataBreakpointInfo`, `setDataBreakpoints` (`supportsDataBreakpoints`) |
   | Loop passes, run to step | no DAP request: custom requests `alRunner/rewindLoop` and `alRunner/runToStep` |
   | Changed by the debugger | the `[debug]` line and the `output` event |

   v2's requests and answers do not change. ADR-048's owner decision ("no `evaluate` or conditional breakpoints in
   3.0") is about release 3.0. This ADR adds them after it.

### 8. Constraints

1. **ADR-009.** A debugged run without changes gives the same outcome as a normal run: the debug form runs the same
   ops between hits; inspection and pure evaluation change nothing (checked by the fingerprint); re-execution is
   deterministic (checked by the divergence hash). The repository test of ADR-048
   (`every_outcome_is_the_same_with_and_without_the_debugger`) is extended: at every stop it evaluates every local as
   an expression, reads the full records, reads the rows of every table the codeunit wrote, and steps back once and
   forward again. The JUnit reports must still match. Only a change (section 3) or an evaluation that changed state
   may give another outcome, and that test is marked.
2. **ADR-006.** All new VM code is in `al-vm` and does no OS access. Blocking stays inside the hook implementation in
   `alrunner-cli`. Re-execution is driven from `al-testrun` and the front ends.
3. **Scope.** AL Runner is a testing tool (ADR-027). The debugger runs test codeunits in the runtime's own session
   over its in-memory store. It does not attach to a BC server, and a change made by the debugger never leaves the
   process. The install baseline is not debugged (as ADR-048).
4. **No cost when not debugging.** Every new op (`DbgCatch`, `DbgCatchEnd`, `DbgPass`, the error call) is emitted in
   the debug form only. The plain form and the captured form do not change. The ADR-031 VM speed budget is the check.

### 9. Seams

All changes to the `DebugHook` seam are additive: new `Resume` variants (the enum is `#[non_exhaustive]`), new
defaulted trait methods (`error`, `result`), new `DebugView` methods (globals, records, rows, step). The DAP and
serve wires gain requests; nothing existing changes meaning. This ADR records them because they are many.

## Phases and order

| Phase | Content | Size (agent-days) |
|---|---|---|
| 1 | Stop on error (catch scopes in the debug form, `DebugHook::error`, the filters, `exceptionInfo`); full record values; `Globals` and `SingleInstance` scopes; more kinds rendered; live access to outer frames | 4 |
| 2 | Serve debug sessions and the one-shot `debug` request with `until` and `snapshot`; database rows; pure expression evaluation (compile at a stop, allow-list, fingerprint); conditions, hit counts, log points in serve and DAP | 7 |
| 3 | Change values: `Resume::Act`, `debugSet`, `setVariable`, `setExpression`, row changes, `changedByDebugger` in every report and the exit code | 3 |
| 4 | Step back: step numbers, re-execution from the codeunit start, divergence hash, refusals, loop passes, `stepBack`, `reverseContinue`, `rewindLoop`, `restartFrame`, `runToStep`, timelines with changes, evaluation on a fork; measure `rewindMs` on the corpus and on a Microsoft test bucket | 7 |
| 5 | MCP adapter; data breakpoints | 3 |

Total about 24 agent-days. Order: 1, 2, 3, 4, 5. Each phase is useful alone, and each later phase builds on the one
before (changes need the expression and target resolution of phase 2; timelines need the changes of phase 3).

## What is refused

- Evaluation (without `fork`) of anything outside the purity allow-list, with the reason (section 2.5).
- Changing a value of a kind the renderer cannot write back (for example a .NET value, an InStream).
- `Validate` or triggers through `debugSet` (use `debugEvaluate` with `fork` to try them).
- Step back after a call into the .NET helper process, with `ALRUNNER_REAL_CLOCK=1`, or with `--files-dir` and a file
  access, or after a recompile (section 4.7). Step back that diverges.
- A second debug session in one server while one is open. Debugging with `--jobs` above 1, `--affected`,
  `--impact-of`, `--recipe` (as ADR-048).
- The WebAssembly hosts (no blocking thread), as ADR-048.

## Consequences

- An agent gets a complete answer to "what was the state here" in one request, and can test a hypothesis by changing
  a value or by going back, without editing code.
- Step back costs time, not memory: one or two runs of the codeunit's prefix per step back.
- The debug form gets four more ops and live frame access. The plain form does not change.
- A list of pure builtins (`PURE_BUILTINS`) must be kept up to date. A new builtin is not pure until listed.
- A changed test can never look like a clean pass.

## Conformance

- `crates/al-vm/tests/debug_*.rs`: exception filters on each catching construct (asserterror, TryFunction,
  `Codeunit.Run`, collected errors, uncaught); the purity walker refuses each listed side effect; the fingerprint
  catches a deliberately impure builtin in a test build; re-execution reaches the same step with the same position
  hash; a change at step E survives a step back to a later step and is dropped by a step back before E.
- `crates/alrunner-cli/tests/debug.rs`: the new DAP requests and capabilities.
- `crates/alrunner-cli/tests/serve.rs`: the `debug` request and every `debug*` command, one-shot and stay.
- The extended equivalence test of section 8.1, on the fixture app and (ignored) on any app.
- The ADR-031 VM speed budget for the plain form.

## Open questions for the owner

1. **Step back by re-execution (a).** Accept re-execution from the codeunit start as the only mechanism, with store
   snapshots (b) only if `rewindMs` measured in phase 4 is too slow? Recommended: yes.
2. **Database rows.** May the debugger insert, modify and delete rows (raw, no triggers, rolled back with the
   transaction, test marked as changed)? Recommended: yes. The other choice is read-only rows.
3. **MCP.** Ship `al-runner mcp` as a thin adapter over the serve commands (recommended, phase 5), or serve JSON only?
4. **Exit code of a changed test.** A pass changed by the debugger gives exit code 1 (recommended), or a new exit
   code of its own?
5. **The .NET helper and step back.** Refuse step back after any call into the .NET helper process (recommended), or
   allow it when every such call is on a list of members known to answer the same each time?
6. **Order.** Phases as listed (stop on error and records, then serve and evaluation, then changes, then step back,
   then MCP)? Recommended: yes.
