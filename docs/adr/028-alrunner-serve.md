# ADR-028: `alrunner serve`: a warm in-process server speaking the v2 wire

- Status: Proposed (the owner decided on 2026-10-15 for a warm server instead of a child process per request; awaiting formal acceptance)
- Date: 2026-10-15

## Context

ADR-016 puts server mode in scope: v3 replaces v2, and ALchemist and LethAL start `al-runner --server` and
speak JSON lines on stdin and stdout (`docs/integrations/alchemist-lethal.md`, sections 1.1, 3.2). An editor loop
(`docs/ideas.md`, live testing as you type) needs the second request to be much cheaper than a fresh
`alrunner test`. A first draft of this ADR ran a child process per request: simple and isolated, but nothing warm.
The owner chose the warm design.

## Decision

1. **The wire is protocol-v2**, unchanged: first stdout line `{"ready":true}`; one JSON object per line; stdout
   carries the protocol only; command names are case insensitive; `runTests` ends with one
   `{"type":"summary",... "protocolVersion":2}`; `shutdown` answers `{"status":"shutting down"}` and exits 0, EOF
   exits too; a bad request answers `{"error":"..."}` and the server goes on; `[server] request <n> done` on stderr.
2. **The run is in-process.** `alrunner serve` calls the same `run::test_with` as `alrunner test`, with a warm
   slot (`al_testrun::runtime::Warm`). `try_run_suites_with` is split into `compile_suites` (produces `Compiled`)
   and `run_compiled`. The server keeps the last `Compiled`: the compiled dependency apps, the lowered workspace,
   the loaded test-data tables and the post-install database snapshot source.
3. **Reuse rule.** A key hashes the workspace sources, the manifests of the workspace apps (`app.json`, the files of the resource folders and the layout files; issue 5604), the dependency identities and the symbols. Same key:
   the compiled workspace is reused (`cached:true` in the summary). Changed source: the workspace and the
   dependencies are compiled again (see Limits). A request that selects tests outside the compiled selection
   recompiles for the union of both selections, so later requests of either kind hit.
4. **No state leaks.** Every run starts from a fresh database built from the same post-install snapshot, and the
   opened backup is kept read-only between requests. Test: a test that writes runs in three requests with the
   same result.
5. **Crash containment.** Each test runs under `catch_unwind`. A panic marks that test as an error, the rest of
   its codeunit as not run ("an earlier test of this codeunit panicked (a runtime bug)"), and the server goes on.
   A panic outside a test clears the warm state and answers an `{"error":...}` line. A hang is stopped by the
   per-test CPU limit (`--test-timeout`).
6. **Streaming.** `al-testrun` has an event hook (`events::Hooks`): one `TestEvent` per test as it finishes.
   The server writes one `test` line per event, then the summary.
7. **`cancel`** is answered at once from a reader thread (`{"type":"ack","command":"cancel","noop":bool}`).
   During a run it sets a flag; the run stops after the current test, and the summary has `cancelled:true`,
   `exitCode` 130, with the tests that ran already streamed. A request that arrives during a run is queued.
8. **Daemon options** and **request fields** are as in the first draft: daemon options are the request-independent
   `alrunner test` options; request fields are `sourcePaths` (required), `packagePaths`, `coverage`,
   `perTestCoverage`, `test`, `excludeTests`, `testIsolation`. Deferred fields (`affectedOnly`, `includeFailing`,
   `strictEnvironment`, `tdd`) answer an error; fields of other v2 commands (`code`, `captureValues`,
   `iterationTracking`) earn a `warnings` entry; others answer `unknown field`.
   Additive, opt-in (2026-10-17): `announceStart: true` on `runTests` or `execute` writes one `{"type":"started","codeunit","test"}` line when the first test is about to run. A client that cancels at a known point waits for it (a cancel during the compile ends the run with `cancelled` and no test; a `runTests` summary then has exit code 130, item 7).
9. **`execute`** (step 4) runs the `OnRun` trigger of one codeunit per app, or of inline `code`, in the same warm process,
   and answers with one line without a `type`: `{"exitCode","tests":[{"name","status","durationMs","message","stackTrace",
   "stackFrames","messages"}],"warnings","coverage","perTestCoverage"}`. Fields: `sourcePaths` or `code` (not both),
   `packagePaths`, `coverage`, `perTestCoverage`, `testIsolation` (checked, no effect), `captureValues` and
   `iterationTracking` (ADR-030: `capturedValues` and `loops` on each test, tagged `messages` on the response). Which codeunit: of the codeunits with an `OnRun` trigger the lowest object id, one that declares no test
   first. `code` that starts (after comments) with an object keyword is a whole object; anything else is a statement list
   in the `OnRun` of codeunit "AL Runner Inline Execute". The name of a result is `Codeunit<ID>.OnRun` for `sourcePaths` and
   `<object name>.OnRun` for `code`. An app without an `OnRun` gives no test, `exitCode` 6 and a warning. The text of every
   `Message` call comes back in `messages`: outside a test method BC's behaviour is not pinned, so this is a tool
   feature of scratch runs only (`docs/probes-needed.md`); `Confirm` and `StrMenu` are still refused. Inline `code` sees
   the Base Application only when the daemon has `--bc-version`, `--dep` or `--package-cache`, or the request has
   `packagePaths`. No `test` line is streamed. `cancel` stops an `execute` while it runs (the one routine is stopped, the answer has
   `cancelled: true` and the test is `error` with the message `NotRun: out-of-scope: cancelled`); a test run lets the running test
   finish.
10. Value capture and loop tracking: ADR-030. Incremental compile: ADR-029.
11. Resources and entries (additive, al-runner issue 5635, 2026-10-22). Resource files are run-time data, not part of
   the kept compile's key: each request reads them as they are, and a request after a resource change reuses the compile.
   `runTests` takes `entries`: several runs over one compile and one install baseline, each with its own `id`,
   `resources` (name -> text in place of the file's), `test` and `excludeTests`; each starts from the same post-install
   database (item 4). The `test` lines carry `entry`, and one `entrySummary` line per entry comes before the summary.
   When the install baseline read a resource file, an entry with `resources` is not run (it would need a baseline of its
   own). Fields and answers: `docs/integrations/migrating-tools-to-v3.md`, section 4b.

## Measurements (release, 4 threads, 2026-10-15)

| Input | one-shot `alrunner test` (compile cache hot) | serve, first request | serve, repeat (unchanged) |
|---|---|---|---|
| corpus app, 2 tests (Pageworks-sized app) | 1.3 s, peak 0.9 GB | 1.36 s (cache hit; 7.6 s on a miss) | 0.17 s; changed source 0.37 s with ADR-029 (0.67 s without) |
| Tests-ERM, codeunit 134000, 32 tests | 4.9 s, peak 1.3 GB (17.7 s without cache) | 4.25 s (cache hit; 18.8 s on a miss) | 2.1 s (first test line after 0.3 s) |

The compile cache is on disk and keyed by the build of the executable, so the first request after a new build pays
the miss (the same as a one-shot run). After a first request the server holds the compiled dependency apps in memory.
Fifty requests with a changed source each grow resident memory by less than 60 MB (tested).

## Limits (honest)

- The baseline reruns per request (0.1 to 0.8 s); `Engine` is rebuilt per request.
- A changed source compiles only the changed objects when the change is in the code of codeunits and tables
  (ADR-029); any other change compiles the workspace again. The dependency apps are never compiled or read again.

## Consequences

- Editor loops get sub-second repeats; ALchemist sees `cached:true` and per-test streaming as in v2.
- The server holds memory (about 3 GB with Microsoft's apps) while idle.
- A runtime bug in one test cannot kill the server, but a bug that corrupts shared state would; the isolation
  test and the 50-request test guard against it.

## Conformance

`crates/alrunner-cli/tests/serve.rs`: streaming and shutdown, bad requests, cancel after the current test, a
writing test with the same answer in three requests, reuse and recompile rules, a panic contained
(`ALRUNNER_DEBUG_PANIC_IN`), 50 requests with bounded memory, queued requests.
