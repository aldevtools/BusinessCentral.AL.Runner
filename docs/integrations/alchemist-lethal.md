# ALchemist and LethAL against AL Runner v3

Repo: al-runtime-rs. Task: ADR-016 gate R item 4 (`serve` speaks protocol-v2, ALchemist and
LethAL run unchanged). State: research, written 2026-10-15. No runtime code changed.

## Summary

- Both tools drive AL Runner v2 through its command line and a JSON-lines server. v3 had
  `al-runner test` only when this was written; `al-runner serve` and `al-runner debug` exist now
  (`migrating-tools-to-v3.md`).
- LethAL can run on v3 earliest, with one-shot mode only. It needs a v2-shaped
  `--output-json`, `--isolation test`, `--exclude-test`, qualified test names and a few
  accepted-and-ignored flags. No server needed. No conflict with the "server deferred" direction.
- ALchemist needs the server (`runTests`, `execute`, `cancel`, `shutdown`) and, for its
  main feature, per-statement value capture and a statement table. Its debugger needs DAP.
- Three things cannot be matched: .NET exception text in failure messages, v2's human log
  lines on stderr, and compiled-only dependency apps (DP-10).
- Both tools check for v2. LethAL refuses any `--version` that does not match `v2.<digit>`.
  Both tools are third-party code (GitHub user SShadowS). Their changes need their owner.

### Needs the owner

| # | Decision | Why it needs the owner |
|---|---|---|
| 1 | Build `serve` now. | ADR-016 (2026-10-13) defers server mode. The same text exempts interfaces and names "an editor integration is wanted" as a reason to revisit. Confirm that this trigger has fired. |
| 2 | How do the tools reach v3? (a) They call `al-runner serve` and `al-runner debug`. (b) `al-runner` also accepts v2 top-level flags `--server`, `--dap stdio`, `--output-json`. (c) A thin `al-runner` launcher accepts v2 syntax. | The 2026-10-14 decision says: clean CLI, no compatibility launcher for now. Option (b) and (c) go against it. Option (a) needs edits in both tools. |
| 3 | What does `al-runner --version` print? | LethAL's check is `/\bv2\.\d/`. v3 prints `al-runner 3.0.0-alpha.1 (...)`. LethAL must change, or v3 must lie. Recommend: LethAL changes. |
| 4 | In v2-shaped JSON, exit code for "tests not run". | v3 uses 7. LethAL reads rows only on exit 0 and 1 (and a proven exit 3). Exit 7 would hide every row. Recommend: LethAL accepts 7 like 1. |
| 5 | DAP (`al-runner debug`) needs an ADR. | The VM is synchronous (ADR-002). A pause hook is a seam change (hard rule 5). |
| 6 | Value capture (`captureValues`) and loop tracking (`iterationTracking`). | Large VM work and a wire format. They are ALchemist's main feature. Decide if they are in v3.0. |

## Sources

| Item | Where | Commit |
|---|---|---|
| ALchemist 0.6.1 (VS Code extension) | https://github.com/SShadowS/ALchemist | `b71f5a3` |
| LethAL (mutation testing) | https://github.com/SShadowS/LethAL | `7fd06618` |
| AL Runner v2 | `BusinessCentral.AL.Runner` checkout | `351c2813d` (v2.12.0-305) |
| AL Runner v3 | `al-runner 3.0.0-alpha.1 (547d0a601)`; this doc branched from `origin/main` `2fd27ac` | |

All `file:line` below are in the repo named by the table heading, at the commit above.
Nothing was copied from the tool repos. Both were cloned read-only into a scratch folder.

## 1. The v2 contract

### 1.1 Server protocol (v2 `--server`)

Source: `docs/server-mode.md`, `protocol-v2.schema.json`, `AlRunner/ServerProtocol.cs`.

| Part | v2 behavior |
|---|---|
| Start | `al-runner --server [--package-cache P]... [--cache DIR] [--define SYM]... [--preprocessor-symbols A,B] [--test-timeout N]`. Symbols and timeout are daemon-wide. |
| Transport | One JSON object per line. stdin requests, stdout responses. stdout carries only protocol. Logs go to stderr. |
| Ready | First stdout line is `{"ready":true}`. |
| Commands | `runTests`, `execute`, `cancel`, `shutdown`. Names are case-insensitive (`ServerProtocol.cs:349`). |
| `runTests` request | `sourcePaths`, `packagePaths`, `coverage`, `perTestCoverage`, `affectedOnly`, `includeFailing`, `strictEnvironment`, `tdd`, `test`, `excludeTests`, `testIsolation` (`codeunit` default, `test`, `disabled`). |
| `execute` request | `sourcePaths` or `code` (not both), `packagePaths`, `captureValues`, `iterationTracking`, `coverage`, `perTestCoverage`, `affectedOnly`, `testIsolation`. |
| Unknown request fields | Refused with `{"error": "..."}`. A declared field the command does not read runs with a `warnings` entry (`ServerProtocol.cs:320-380`). |
| `runTests` response | Zero or more `{"type":"test","name","status","durationMs","message","errorKind","stackFrames","stackTrace"}` lines, then one `{"type":"summary","exitCode","passed","failed","errors","total","cached","protocolVersion":2,...}`. `status` is `pass`, `fail`, `error`, `skipped`. Name is `Codeunit<ID>.<Method>`. |
| Summary extras | `cancelled`, `changedFiles`, `compilationErrors[{file,errors[]}]`, `coverage[{file,statements[{id,scope,line,column,endLine,endColumn,hits}]}]`, `perTestCoverage[{test,coverage[...]}]`, `warnings`, `companyInitFailures`, `sourceScanFailures`, `selection`, `wallSeconds`. |
| `execute` response | One line, no `type`: `{"exitCode","tests":[...],"capturedValues","messages","loops","coverage"}`. |
| `cancel` | Answered at once, even mid-run: `{"type":"ack","command":"cancel","noop":bool}`. The running test finishes. The next does not start. Summary has `cancelled:true`. |
| `shutdown` | `{"status":"shutting down"}`, then exit 0. EOF on stdin also exits. |
| Request error | `{"error":"..."}`. The server keeps running. |
| stderr marker | `[server] request <n> done` after each request except `cancel`. |
| Versioning | Only `protocolVersion: 2` in the summary. No handshake. A reply with no `type` is read as v1 by ALchemist. |

### 1.2 `--output-json` (v2)

Source: `AlRunner/Reporter.cs:844` (`SerializeJsonOutput`), `docs/cli-output-paths.md`.

One pretty-printed document. Null fields are left out.

| Field | Meaning |
|---|---|
| `tests[]` | `name` (`Codeunit<ID>.<Method>`), `status` (`pass`/`fail`/`error`/`skipped`), `durationMs`, `message`, `stackTrace`, `diagnosis`, `suspect`, `expectation`, `generatedStubs` |
| `passed`, `failed`, `errors`, `skipped`, `total`, `exitCode`, `seed`, `wallSeconds` | totals |
| `compilationErrors[]`, `executionErrors[]`, `suiteErrors[]` | each `{file, errors[]}`. Only when non-empty. |
| `companyInitFailures[]` | only when non-empty |

Banner lines can precede the document on stdout (seen on 2.0.0.0). Both tools allow for that.
Exit codes: 0 pass, 1 test failed or errored, 2 bundle could not execute, 3 bundle could not
compile or a suite error, 6 `--test` matched nothing. Rank order `3 > 2 > 4 > 6 > 1 > 5`.

### 1.3 DAP (v2 `--dap stdio`)

Source: `docs/dap-mode.md`, `AlRunner/Program.cs:7649-7870`.

Content-Length framed DAP over stdin/stdout. Requests handled: `initialize` (answers
`supportsConfigurationDoneRequest` only, then the `initialized` event), `launch`/`attach`
(wait for the compile), `setBreakpoints` (exact line match, optional column, may arrive before
`launch`), `configurationDone`, `threads` (one thread), `stackTrace`, `scopes`, `variables`,
`continue`, `pause` (acts as continue), `next`, `stepIn`, `stepOut`, `disconnect`, `terminate`.
Anything else answers `unsupported command`. `stopped` events carry reason `breakpoint` or
`step`. The client's `linesStartAt1` and `columnsStartAt1` are honored. One module per session.

## 2. ALchemist (repo `ALchemist`, `b71f5a3`)

Needs AL Runner >= 2.11.0 (`src/runner/alRunnerManager.ts:11`).

| # | Interface point | Where | v3 today | Gap |
|---|---|---|---|---|
| A1 | Finds the binary: setting `alchemist.alRunnerPath`, env `ALCHEMIST_RUNNER_PATH`, then `which`/`where al-runner` | `alRunnerManager.ts:47-51, 129-139`; `package.json:269` | Name is `al-runner`, so the PATH lookup misses. The setting works. | None for v3. Tool could also look for `al-runner`. |
| A2 | Installs v2: `dotnet tool install -g msdyn365bc.al.runner`, update check via `dotnet tool list -g` and `tool search` | `alRunnerManager.ts:142-184, 186-220` | Not applicable. v3 is not a dotnet tool. | Cannot match. User sets the path. Update nag is skipped when the path is set (`:187`). |
| A3 | Version check: `<path> --version`, first `\d+.\d+.\d+` compared with 2.11.0. Only a warning. | `alRunnerManager.ts:28-32, 100-127` | `3.0.0-alpha.1` parses and is above 2.11.0. No warning. | None. |
| A4 | Spawns `<path> --server` with no other args, cwd inherited | `serverProcess.ts:125-141`; `extension.ts:207-215` | `al-runner --server` is "unknown command". `al-runner serve` is "not implemented". | Server, plus owner decision 2 for the spelling. |
| A5 | Waits for `{"ready":true}` on stdout. Skips non-JSON lines before it. | `serverProcess.ts:186-192` | No | Part of the server. |
| A6 | Sends `runtests` (lowercase) with `sourcePaths`, `captureValues` (default true), optional `iterationTracking`, `coverage`, `cobertura`, `testFilter` | `serverExecutionEngine.ts:23-33`; callers `testController.ts:403, 431`, `extension.ts:596, 801` | No | Server must accept lowercase command and tolerate `captureValues` and `iterationTracking` on `runTests` (v2 runs and adds a warning). `cobertura` and `testFilter` are never set by current callers. v2 would refuse them. |
| A7 | Reads streamed `test` lines: `name`, `status` (pass/fail/error), `durationMs`, `message`, `stackTrace`, `stackFrames`, `errorKind`, `alSourceFile`, `alSourceLine`, `messages`, `capturedValues` | `protocolV2Types.ts:89-103`; `serverExecutionEngine.ts:194-209`; `testController.ts:485-560` | No | `alSourceFile`, `alSourceLine`, `capturedValues`, `messages` are not emitted by v2 `runTests` either (schema-only). Without them ALchemist shows pass/fail and stack frames only. |
| A8 | Maps a test event to a Test Explorer item. It compares the item label (the bare procedure name, `:282-285`) with `event.name`. | `testController.ts:109-130, 493` | Name format decides | v2 sends `Codeunit<ID>.<Method>` (`ServerProtocol.cs:452`). ALchemist's own tests use bare names (`test/suite/serverProcess.streaming.test.ts:157`). Either ALchemist drops v2 results or I misread it. Not run live. Check on a v2 run first. v3 should send exactly what v2 sends. |
| A9 | Reads the `summary`: `passed`, `failed`, `errors`, `total`, `exitCode`, `cached`, `cancelled`, `protocolVersion`, `coverage` | `serverExecutionEngine.ts:57-118`; `protocolV2Types.ts:138-153` | No | A `type:"summary"` with `protocolVersion: 2` is the switch that selects the v2 path (`:61`). |
| A10 | Coverage: `coverage[].statements[]` turned into VS Code coverage. It also reads `hitStatements` and `totalStatements`. | `coverageAdapter.ts:58-100`; `testController.ts:578` | No (OUT-8, ED-8) | v2 never emits `hitStatements`/`totalStatements` (`grep` of `AlRunner/` finds only a comment). So file totals are likely `undefined` on v2 too. Needs a check on a live run. Tool-side issue, not a v3 gap. |
| A11 | Scratch run: `execute` with `code` or `sourcePaths`, `captureValues: true`, `iterationTracking: true` | `serverExecutionEngine.ts:121-130`; `extension.ts:435-455, 567-587` | No (IN-9, ED-2) | Server `execute`. |
| A12 | Reads `execute` response: `tests`, `capturedValues` (with `loop`, `iteration`, `captureError`), `messages`, `loops`, `unresolvedScopes`, `coverage` | `upstreamExecuteNormalizer.ts:1-80`; `serverExecutionEngine.ts:147-192` | No (ED-6, ED-7) | Value capture and loop tracking. Hard. Without them the inline-value feature is empty. Runs, but shows nothing. |
| A13 | `cancel`: writes `{"command":"cancel"}` when the user cancels | `serverProcess.ts:89-96`; `testController.ts:381-392` | No (ED-3) | Needs a cancel flag the VM checks between tests. |
| A14 | `shutdown`: writes it, then SIGTERM, SIGKILL after 2 s | `serverProcess.ts:290-310` | No | Part of the server. |
| A15 | Crash handling: if the process dies before any `test` line, respawn and resend once. After a `test` line, fail the request. | `serverProcess.ts:234-266` | n/a | Client logic. v3 must exit non-zero on a crash. |
| A16 | Source path format in responses: absolute, forward slashes. Older relative forms are still matched. | `decorations.ts:595-610`; `serverProcess.ts:15-26` | n/a | v3 must emit absolute forward-slash paths in `coverage[].file` and `loops[].file`. |
| A17 | Ignores stderr (does not read it) | `serverProcess.ts:138` (stdout only) | n/a | None. The `[server] request n done` marker is not needed. |
| A18 | Debugger: `DebugAdapterExecutable(path, ['--dap','stdio', ...sourcePaths])`. Debug type `alchemist`. Launch config fields `bundleDir`, `sourcePaths`, `testFilter` are ALchemist's own. They are not sent to the runner. | `debugAdapterFactory.ts:56-98`; `debugLaunch.ts:31-47`; `package.json:162-212` | Partial (ED-9) | `al-runner debug --stdio APP...` (`docs/debugging.md`). See 1.3 for the DAP surface. |
| A19 | Says "stepping acts as continue" | `debugLaunch.ts:61-62` | n/a | Out of date: v2 now implements step. v3 should implement step. Tool text is the tool owner's to fix. |
| A20 | Environment: `ALCHEMIST_RUNNER_PATH` (its own). Sends no `AL_RUNNER_*` variable. | `alRunnerManager.ts:49` | n/a | None. |

## 3. LethAL (repo `LethAL`, `7fd06618`)

Files under `packages/runner/src/`. Config is `lethal.config.json`, section `alRunner`
(`cli.ts:2203-2260`): `alRunnerPath` (required), `packagesDir`, `serverMode` (default true),
`selectorMode`, `coverage` (default `"al-runner"`).

### 3.1 One-shot mode (`OneShotTransport`)

One process per mutant per test. The command line, built in one place
(`al-runner-transport.ts:641-741`):

```
<al-runner> --output-json --isolation test --test <CodeunitN.Method>
  [--test-exact <same>] [--exclude-test <name>]... [--auto-provision]
  [--coverage --coverage-out <file>] <sourceDir> [<testDir>]
  [--package-cache <dir>]... [--define <SYM>]...
```

Environment: `AL_RUNNER_TEST_TIMEOUT_SEC=<n>` (`:754`). `AL_RUNNER_VERBOSE=1` on the one
provisioning call (`al-runner-backend.ts:815`).

| # | Interface point | Where | v3 today | Gap |
|---|---|---|---|---|
| L1 | Binary path comes only from `alRunner.alRunnerPath` | `cli.ts:2237, 2610` | n/a | None. |
| L2 | Version check: `<path> --version` exit 0 and stdout matches `/\bv2\.\d/`. Otherwise "not a v2 build", session refused. Same check again in the contract probe. | `al-runner-backend.ts:165, 1092-1115`; `al-runner-contract.ts:439-452` | `al-runner 3.0.0-alpha.1 (...)` fails the match. | Decision 3. LethAL change. |
| L3 | Contract probe also sends an impossible flag (`--lethal-wire-contract-probe`) and needs a non-zero exit chosen by the process (v2: 2). | `al-runner-contract.ts:170, 454-478` | v3 answers `unknown option`, exit 2. Matches. | None. |
| L4 | `--output-json` document on stdout: the last line that starts with `{` in column 0 to the end is parsed. Needs `tests[]` with `name`, `status`, `durationMs?`, `message?`. | `al-runner-transport.ts:497-535` | `unknown option` | v2-shaped JSON. See 1.2. Print the document pretty-printed so the last column-0 `{` starts it. |
| L5 | `--isolation test` (fresh state per test). LethAL claims "full-reset". | `al-runner-transport.ts:668-673`; `al-runner-backend.ts` capabilities | Not built. `Function` isolation exists but is only reachable through a runner codeunit (SEL-5). | Add `--isolation`. |
| L6 | `--test <CodeunitN.Method>`: case-insensitive substring of the qualified name | `al-runner-transport.ts:674-675, 467` | `--test` is an alias of `--filter` and matches the test name only. `Codeunit50011.Foo` matches nothing, so exit 6. | Match the qualified name. |
| L7 | `--test-exact NAME`: probed once per session. A build that rejects it with exit 2 is treated as "substring only". Then LethAL sends `--exclude-test` for each sibling whose name contains the wanted name. | `al-runner-backend.ts:636-690`; `al-runner-transport.ts:38-51` | Neither flag exists. | Add `--exclude-test` (whole qualified names). `--test-exact` is optional. If absent, LethAL falls back correctly. |
| L8 | `--auto-provision`: sent on every call unless a platform-apps directory is pinned | `al-runner-transport.ts:703` | `unknown option`, exit 2 on every mutant | Accept and ignore, or LethAL stops sending it. Blocker. |
| L9 | `--package-cache DIR` (repeatable), `--define SYM` (repeatable), positional app folders (1 or 2) | `al-runner-transport.ts:716-739` | `--package-cache` yes. App folders yes. `--define` no. | Add `--define` (IN-8). |
| L10 | Env `AL_RUNNER_TEST_TIMEOUT_SEC`. Flag `--test-timeout` is not used one-shot. | `al-runner-transport.ts:754` | v3 reads no such variable. Flag is `--test-timeout-secs`, CPU-time based, default 20. | Accept the v2 variable name (or planned `ALRUNNER_TEST_TIMEOUT`, ENV-2). Without it LethAL's per-mutant time budget does nothing. |
| L11 | Timeout recognition. A row with status not pass/fail and `message` equal to `Test exceeded <N>s timeout.` or `TIMEOUT after <N>s`, or the OnRun sentence, is a timeout. | `al-runner-backend.ts:79-93, 121-130` | v3 reports a timed-out test as "not run" with the limit as reason. | In JSON, emit a row with `status:"error"` and exactly `Test exceeded <N>s timeout.` (SEL-6). |
| L12 | Exit codes: 0 and 1 read rows. 3 read only if the envelope proves a `TEST-TIMEOUT-ABORT` suite error that names a non-pass/fail row. Everything else is an error ("measured nothing"). Code >= 128 is a kill. | `al-runner-transport.ts:576-625, 821-845, 268` | v3: 0, 1, 2, 6, 7. | Decision 4. Exit 7 would be an error for LethAL. |
| L13 | Empty or missing `tests[]` is an error, never "nothing failed". A compile failure must not look clean. | `al-runner-transport.ts:497-509`; `al-runner-contract.ts` (fact `compile-failure-not-scorable`) | v3 has no exit 3. A refused routine is "not run". | In v2-shaped JSON a not-run test needs a row (`status:"skipped"` or `"error"`) so the totals are honest. |
| L14 | Refusal recognition. A fail or error message of the form `<Type>: out-of-scope: ...` is "runner refused", not a kill. | `al-runner-backend.ts:97` | v3 has refusals as "not run" with a reason. | Emit rows with a message starting `out-of-scope: ` (any prefix). Reason text is v3's. |
| L15 | Assertion screen reads `NavNCLDialogException: ` and `InvalidOperationException: out-of-scope:` prefixes in failure messages. | `assertion-screen.ts:36-44, 78-90` | v3 has no .NET type prefix (ADR-009; parity page "Differences users will notice"). | Cannot match. Needs a check in LethAL: its rule must not depend on the prefix. |
| L16 | Reads human log lines from stderr and stdout: `[bc] selected BC <v>`, `[bc] warning: ... not the selected <v>`, banner `al-runner <v> · BC <v> · N apps`, `[dep] <app> resolved to a package with NO IMPLEMENTATION`, `[provision] Downloaded N app(s) ... platform-apps`, `[provision] platform apps already complete at ...`, `[pkg-cache] <dir>` | `al-runner-transport.ts:106-210, 303-358, 390-460` | v3 prints none of these. | Cannot match (log text). Effect: the report has no BC build and no platform-apps pin. Both degrade to "unknown". The pin only saves time (`--auto-provision` cost). Confirm that LethAL does not refuse on "unknown". |
| L17 | Provisioning call: `--test Codeunit0.__lethal_provision_only__`, selects nothing, once per session, with `AL_RUNNER_VERBOSE=1` | `al-runner-backend.ts:173, 442-455, 803-830` | v3 would exit 6 (matched nothing) after compiling. Harmless. `AL_RUNNER_VERBOSE` ignored. | None needed. |
| L18 | Predefined-symbol probe: a test that checks `#if CLEANSCHEMA1..40` and the count of symbols v2 defines (set recorded as `AL_RUNNER_PREDEFINED_SYMBOLS_V2_12_0`). | `al-runner-predefined-probe.ts:30-60, 190-260` | v3 defines other symbols (IN-8, ADR-015). | LethAL records the set per build. Check that a different set is not an error. |
| L19 | Coverage, one-shot: `--coverage --coverage-out FILE`. Reads Cobertura with a regex on `<class ... filename="...">` and `<line number="N" hits="M">`. Line numbers are file-relative. Labels are relative to the process cwd. | `al-runner-transport.ts:711`; `al-runner-coverage.ts:86-95, 645` | `unknown option` (OUT-8) | Cobertura writer. If `coverage` is off, LethAL scores unreached mutants `survived` instead of `no-coverage`. Not wrong, only less exact. |
| L20 | Frame probe: a layout with two apps of the same app id, a multi-object file and a table trigger. It checks that coverage lines land in the instrumented text, not the source. | `al-runner-frame-probe.ts:1-60` | n/a | If v3 reports positions of the compiled file it was given, it passes. Test it. |
| L21 | Selector mode `static`: LethAL rewrites one AL file per mutant. | `schemata/src/selector.ts` | Works (compile per run). | None. |

### 3.2 Server mode (`AlRunnerServer`, default on in LethAL config)

Source: `al-runner-server.ts`.

| # | Interface point | Where | v3 today | Gap |
|---|---|---|---|---|
| S1 | Spawns `<path> --server [--package-cache P]... [--define SYM]... [--test-timeout N]`. One daemon per worker. | `al-runner-server.ts:259-266` | No | Server. `--test-timeout` is required: LethAL refuses to start a daemon without it (`al-runner-backend.ts:138-145`). |
| S2 | First stdout line must be JSON with `ready: true`. Any other first line refuses the session. | `:271-295` | No | Part of the server. |
| S3 | `runTests` request: `sourcePaths`, `packagePaths?`, `coverage?`, `perTestCoverage?`, `testIsolation` (always `"test"`) | `:339-348` | No | Server. `testIsolation:"test"` must really reset state per test. |
| S4 | Response: reads `type:"test"` lines (`name`, `status`, `durationMs`, `message`). Reads `type:"summary"` (`exitCode`, `total`, `perTestCoverage`). Skips unknown types. An unparseable stdout line aborts the session. | `:350-399` | No | stdout must hold protocol only. |
| S5 | Whole suite runs every time. LethAL measured "no per-test selection" on 2.11.0. It does not send `test` (v2 has had it since #5183). | `:6-34` | n/a | LethAL could send `test`/`excludeTests` later. Not needed. |
| S6 | `perTestCoverage[{test,coverage[{file,statements[{scope,line,hits}]}]}]`. `scope` names the procedure. Absolute file labels. | `:104-121`; `al-runner-coverage.ts:707-770` | No (ED-8) | Per-test statement table. |
| S7 | No `cancel`, no `affectedOnly`, no `execute`. | `:53-64` | n/a | None. |
| S8 | `shutdown`: writes it, waits up to 500 ms for a reply, then kills. | `:402-440` | No | Part of the server. |
| S9 | Keeps the last 64 KB of stderr for the `[bc]` and `[dep]` lines. | `:207, 298-324` | n/a | Same as L16. |
| S10 | Warm state must not give stale answers. LethAL changes one AL file per mutant and measured a verdict flip on the same process. | `:36-52` | n/a | v3 recompiles every request, so correct by construction. |
| S11 | Resource selector: bundle compiled once. The active mutant id is written to `LethALResources/active-mutant.txt` (declared in `app.json` `resourceFolders`). AL reads it with `NavApp.GetResourceAsText` on every call. | `al-runner-backend.ts:1245-1290`; `schemata/src/selector.ts:115-176` | v3 packs resources into the IR at compile time (`al-ir/src/lib.rs:119`). | Only a problem if `serve` keeps compiled state. With a recompile per request it works. Set `selectorMode: "static"` until then. |

### 3.3 LethAL on v3 with no server (config that works first)

```json
{ "alRunner": { "alRunnerPath": "<path to a wrapper>", "serverMode": false,
                "selectorMode": "static", "coverage": "none" } }
```

This uses only section 3.1 rows L2 to L13, minus coverage. It also needs the version change
(L2). The path must point at an executable that LethAL can start with v2 arguments.

## 4. What cannot be matched

| Item | Why | Effect on the tools |
|---|---|---|
| `NavNCLDialogException: ` and other .NET type prefixes in messages | v3 has no CLR. ADR-009 makes the BC message the observable part. | LethAL's assertion screen (L15). Needs a check by its owner. |
| v2 stderr log lines (`[bc]`, `[dep]`, `[provision]`, `[pkg-cache]`, banner) | They describe v2 internals (provisioning, engine variants). | LethAL reports no BC build and does not pin platform apps (L16). |
| Install by `dotnet tool` | v3 is a single binary. | ALchemist auto-install and update nag (A2). The user sets the path. |
| `CS####` compile errors and exit 3 | v3 has no C# stage. It refuses per routine. | LethAL's contract fact `compile-failure-not-scorable` (L13) needs a row-based answer. |
| Dependency apps that ship only compiled code (DP-10) | No CLR. | Projects that depend on such apps stay on v2. |
| Browser WASM debugging | The debugger needs a pause thread (ED-9). | No effect on these tools. |
| `affectedOnly`, `includeFailing`, `strictEnvironment` | Supported since ADR-050 (owner, 2026-10-21), as v2 without `environmentDrift`. | Neither tool sends them today. |
| `tdd`, warm reload (ED-5) | `tdd`: ADR-051. Warm reload: deferred by the owner (2026-10-13). | Neither tool sends them. |

## 5. Plan

Sizes are my rough estimates in engineer-days, including red/green tests (hard rule 1).
Each step is useful alone. Order is smallest first.

| Step | What | Unblocks | Size | Needs owner |
|---|---|---|---|---|
| 1 (done, branch tools-step-1; see migrating-tools-to-v3.md) | `al-runner test` accepts `--output-json` (v2 shape, section 1.2), `--isolation test|codeunit|disabled`, `--exclude-test`, qualified-name `--test`, `--define`, `--auto-provision` (ignored), and reads `AL_RUNNER_TEST_TIMEOUT_SEC`. Not-run tests become rows. Timeouts use v2's sentence. | LethAL one-shot (section 3.3) | 3 to 5 | #3 (version), #4 (exit 7), and #2 only for spelling. Flags that v3 already plans (`--output-json`, `--isolation`, `--exclude-test`) do not conflict. |
| 2 | `--coverage --coverage-out` (Cobertura) in the positions the compiled input has. Per-test file is one run per test already. | LethAL coverage (L19), gate R item 5 | 4 to 6 | none (needs IR positions, which exist) |
| 3 | `al-runner serve`: ready line, `runTests` stream, `summary` with `protocolVersion:2`, `shutdown`, `{"error"}`, field warnings and refusals, `--package-cache`, `--define`, `--test-timeout` start flags, stderr request marker. No warm state: compile per request. | LethAL server mode (S1 to S10). ALchemist Test Explorer without inline values. | 5 to 7, plus a short ADR for the wire protocol | #1, #2 |
| 4 (done, ADR-028 item 9) | `serve` adds `execute` (inline `code` or one `OnRun` codeunit), `cancel` side channel (flag checked between tests), `coverage` and `perTestCoverage` tables on both commands. | ALchemist scratch runs (A11), cancel (A13), coverage (A10); LethAL `perTestCoverage` (S6) | 5 to 8 | none beyond #1 |
| 5 (done, ADR-030) | `captureValues` and `iterationTracking`: per-statement local-value records, loop records, tags. | ALchemist inline values and Iteration Table (A12) | 15 to 20 | #6 |
| 6 | `al-runner debug --stdio`: DAP surface of section 1.3. VM pause hook at statement boundaries on its own thread. | ALchemist debugging (A18) | 15 to 20, plus an ADR | #5 |
| 7 | Tool-side edits, by their owner: LethAL accepts a `v3` version string and exit 7; ALchemist looks for `al-runner`; ALchemist fixes the `hitStatements` read and the stale stepping text. | Clean "unchanged" claim for gate R item 4 | small, theirs | #2, #3 |

Why this order: step 1 is the smallest and gives one tool a working path with no server. Step 3
is the smallest server. Steps 5 and 6 are large and need owner scope decisions first.
"Unchanged" in gate R item 4 holds only if the owner picks spelling option (b) or (c) of
decision 2 and prints a v2-looking version string. Otherwise step 7 is required, and the gate
text should say "run with small edits".

### Check before step 1

1. Confirm LethAL does not refuse when the BC build and platform-apps pin are unknown (L16).
2. Run LethAL's contract probe against a v3 build with a test wrapper. Six facts: version,
   unknown flag rejected, qualified test name, timeout classified, timeout exit readable,
   compile failure not scorable (`al-runner-contract.ts:88-96`).
3. Run ALchemist's v2 fixtures against the step 3 server and diff the lines.

## 6. Findings about the tools, for their owner

| Finding | Where |
|---|---|
| ALchemist reads `hitStatements` and `totalStatements`, which v2 never sends. | `coverageAdapter.ts:61` |
| ALchemist says stepping acts as continue. v2 implements step. | `debugLaunch.ts:61-62` |
| ALchemist sends `captureValues` and `iterationTracking` on `runTests`. v2 reads them only on `execute`, so each run returns a `warnings` entry. | `serverExecutionEngine.ts:28-30` |
| ALchemist sends `testFilter` and `cobertura` if a caller sets them. v2 refuses unknown fields. No caller sets them today. | `serverExecutionEngine.ts:32-33` |
| LethAL source comments say v2 has no per-test selection on `--server`. v2 added `test` and `excludeTests` after 2.11.0. | `al-runner-server.ts:10-14` |

## Later step: mutants applied inside v3 (owner discussion 2026-10-15)

LethAL already compiles all mutants into one program behind switches (mutant schemata) and
runs them through AL Runner. That keeps working with v3 unchanged: the switched code is plain AL.

The step beyond is to let v3 apply the mutation to its compiled program instead of LethAL
rewriting source:

1. LethAL keeps its mutation catalog and reports. It sends v3 a list of mutants as source
   positions plus a change, for example `{id, file, line, column, kind: "relational", from: "<",
   to: "<="}`, or `{kind: "remove-statement", file, line}`.
2. v3 compiles the app once. It maps each position to the node in its compiled program (it keeps
   source positions) and patches that node in memory per mutant. No source rewrite, no recompile.
3. v3 runs only the tests whose coverage reaches the mutated node, and answers per mutant:
   killed (which test), survived, not covered, or unknown (a covering test does not run on v3).
4. A mutation v3 cannot express in its compiled program falls back to LethAL's switched source.

Protocol: a `runMutants` request on `al-runner serve`, after the serve basics. Gain to measure with
a prototype: no per-mutant compile, and only covering tests run.
