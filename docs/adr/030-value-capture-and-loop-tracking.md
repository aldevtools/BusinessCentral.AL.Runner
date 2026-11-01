# ADR-030: Value capture and loop tracking (`captureValues`, `iterationTracking`)

- Status: Accepted (2026-10-15, orchestrator; tool-lane scope decided by the owner in ADR-016) (the owner decided in ADR-016 that both are in v3.0; this ADR fixes the design)
- Date: 2026-10-15
- Related: ADR-016 (tools), ADR-028 (`alrunner serve`), ADR-029 (incremental compile), `docs/integrations/alchemist-lethal.md`
  (A12: what ALchemist reads)

## Context

ALchemist shows the value of a local next to the statement that set it, and steps through the passes of a loop. AL Runner
v2 gets both from a rewrite of the compiled C# (hooks on `StmtHit`) and has to guess the passes of a loop from the hit
stream. v3 interprets the IR, so the interpreter itself knows when a statement starts and ends, when a loop starts, when a
pass starts and how the loop ended. The same data serves live testing as you type (`docs/ideas.md`): the values of a test
that just ran, next to the code that was just typed.

## Decision

1. **Opt in, per request.** `captureValues` and `iterationTracking` are fields of `execute` and `runTests`. Off, a run
   pays nothing: `Session::capture` is `None`, a block runs in the loop it always had, and the loop statements make one
   test of that option per pass. (Measured: a nested loop of 1.5 million statement executions, five runs each, alternating the build before this step and this build: medians 455 ms and 425 ms, then 321 ms and 333 ms. The difference is the noise of a shared machine.)
2. **What a statement is.** The lowering marks each statement with a `Stmt::Pos(id)`. With capture on, `exec_block` runs a
   block through `exec_block_captured`, which treats a marker and the statement after it as one statement. `id` is the
   position id that `coverage` uses (`coverage[].statements[].id`), so a `statementId` finds its place there.
3. **Values** (`captureValues`). After each statement of a routine of an app of the run (never a dependency's) that ends
   without an error, every named local of the routine is rendered and compared with what was last seen. A record
   `{scopeName, variableName, value, statementId}` is made for a local that changed, and for a local the statement
   writes by its shape: the target of `:=`, the receiver of a method called as a statement (`Rec.Insert()`,
   `List.Add(1)`), a `var` argument. So `x := 5; x := 5;` is two records. A parameter's value at entry is the baseline and
   no record; a local never written has none. A `for` or `foreach` variable is recorded for the loop statement when each
   pass starts.
4. **Values as JSON.** Integer, BigInteger, Byte, Char: number. Decimal: number. Boolean: boolean. Text, Code, DateFormula: string.
   Date, Time, DateTime, Duration, Guid: string in the XML form (`Format(x, 0, 9)`). Option, Enum: the member name.
   Record: its primary key, the values joined by `, `. Anything else (a List, a codeunit variable, a DotNet value, ...):
   `value: null` and `captureError: "<kind> is not rendered yet"`, never a made-up value. Globals are not captured.
5. **Loops** (`iterationTracking`). Each execution of a loop statement is a loop instance with an `id` (unique in the
   response), `scope`, `file`, `line`, `column`, `endLine`, `iterationCount` (the passes whose body began), `closedBy`
   (`exit`: the loop ended, also by `break`; `scopeExit`: the routine returned from inside it; `unfinished`: an error went
   through it), `parentLoop` and `parentIteration` (the loop and pass active when it started, across calls), and
   `iterations[]` of `{index, statements, lines}` (the statements of the loop's own routine that the pass ran, nested loops'
   included). No loop is "unsegmentable": the interpreter counts the passes. A loop over a DotNet collection is not
   tracked yet. `endColumn` is not known to v3 and not sent.
6. **Tags.** A value record or a `Message` made inside a loop carries `loop` and `iteration`: the innermost loop active
   across calls, and its pass. With `iterationTracking` the response also has the tagged `messages`
   `{text, scopeName, statementId, loop, iteration}` of an `execute`, as v2.
7. **Where it comes back.** `execute`: `capturedValues` and `loops` on each test of the response, `messages` on the
   response. `runTests` (a v3 extension of v2, which has it for `execute` only): `capturedValues` and `loops` on each `test`
   line, so a client that runs tests as you type gets the values of the test that just ran. Loop ids are unique within a
   request.
8. **Bounded.** A capture keeps at most 200,000 value records and 200,000 messages per test (`MAX_RECORDS`); past that it
   drops them and the test has `capturedValuesTruncated: true`. The run itself goes on.
9. **Cost when on.** Each statement renders every local of its routine: a loop of 1.5 million passes with two locals takes
   5.7 s instead of 0.4 s, and stops recording at the bound. Scratch code and tests are small; the bound keeps a mistake
   from filling the memory.

## Consequences

- No Cecil-style rewrite and no heuristics: the passes are exact, also for loops v2 cannot segment.
- Tags follow the dynamic loop, so a value produced in a procedure called from pass 2 is tagged with that pass.
- The statement ids are the coverage ids of the current program. In live testing the ids move when the program is renumbered
  (ADR-029 renumbers exactly as a complete compile does), so a client reads `coverage` of the same response.
- A later step can keep the render cheap (compare without rendering) if capture on a big test becomes slow.

## Conformance

`crates/al-testrun/tests/suite/capture.rs` (values, parameters, callee scopes, loop passes and tags, nesting across calls and
`closedBy`, an error through a loop, each flag alone, the bound); `crates/alrunner-cli/src/capture.rs` (the JSON shapes);
`crates/alrunner-cli/tests/serve.rs` (`execute` with both flags against the example of v2's documentation, and `runTests`
lines).
