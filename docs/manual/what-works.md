---
title: "What works and what is refused"
weight: 11
---

The goal is the whole AL language. How much runs today is in the status table in
the [README](../../README.md) and in [docs/status.md](../status.md). The scope
decisions are [ADR-027](../adr/027-scope.md).

## The rule

A feature AL Runner does not support is refused. The test is reported as **not
run**, with the reason, and the exit code is 7 unless you pass `--allow-not-run`.
A guessed or partial behavior is never used as a stand-in. Refusal happens before
the run where possible, and mid-run otherwise.

There are two kinds of reason:

- **By design.** The surface needs a process, a network or a service tier that AL
  Runner does not have. It will stay refused.
- **Not measured yet.** AL Runner will not guess behavior that no corpus test pins
  on real Business Central. This group shrinks as tests are added.

## Status

Measured on 2026-10-21 against the AL language test corpus (commit `f3fe6c99`). A
test counts as passing only when it passes here and passed on real Business Central.

| Business Central | Passing | Failing | Not run |
|---|---|---|---|
| 27.5 | 13964 of 17050 (81.9%) | 0 | 3086 |
| 28.5 | 14002 of 17140 (81.7%) | 0 | 3138 |
| 29.0 | 14023 of 17234 (81.4%) | 0 | 3211 |

"Failing" is a case where real Business Central passed and AL Runner gave a
different result or an error. About 435 passes on each version run through an
unprobed assumption and are counted as flagged. Microsoft's own Base Application
test buckets mostly do not run yet: the full run of 2026-10-16 held 40300 tests,
of which 16848 passed, 2508 failed and 20944 were not run
([docs/alrunner-buckets.md](../alrunner-buckets.md)).

## Runs as Business Central does

- The AL language: parsing, `#if`, type checking, overload resolution, namespaces and extensions.
- Records: create, read, update and delete; filters; keys; FlowFields; `CalcSums`;
  `SetLoadFields`; rename propagation; `TransferFields`; `RecordRef` and `FieldRef`.
- Triggers and events: table, page and codeunit triggers; integration and business
  events; subscribers; manual binding; order.
- Errors: `asserterror`, `TryFunction`, `GetLastError*`, error text and code.
- Collections and data: `List`, `Dictionary`, JSON, XML, streams, BLOBs,
  `TextBuilder`, queries (the shapes the corpus pins), XmlPort import and export.
- Reports, processing only: `Run`, `RunModal`, `Execute`, `RunRequestPage`, data
  items, the triggers from `OnPreReport` to `OnPostReport`, and the dataset as XML.
- Pages, headless: the test client opens pages, sets values, validates, runs
  actions, navigates, and drives modal pages and handlers.
- Microsoft's apps, compiled from their own source. Posting routines, validation and
  System Application modules run as written.
- Outbound HTTP, answered by an `[HttpClientHandler]` in the test. A call with no
  handler raises the error Business Central raises under `BlockOutboundRequests`.

## Replaced by something equivalent

| Surface | What AL Runner does | Where it ends |
|---|---|---|
| Tables | An in-memory store | No row locks, no other session, no SQL errors. `LockTable(false)` is refused. |
| Transactions | Snapshots. A passing test commits at its boundary. `asserterror` rolls back. | Commit order across sessions is not modeled. |
| Test data | CRONUS from the artifact, loaded per table on first touch | Only the demo company. |
| Identity | User `TESTUSER`, tenant `default`, company `My Company`, session 1 | Tests that probe real authentication or other users. |
| Environment | `--environment saas` (default) or `onprem` | Server topology is fixed. |
| Clock | The system clock. `Sleep(n)` does not wait. | A test sees a jump, not a delay. |
| Client UI | Handlers and the `TestPage` client | No pixels, no layout, no add-ins. |
| Permissions | Everything is granted | A test that expects "access denied" fails here. |
| Task scheduler | `CanCreateTask` is false. `CreateTask` returns an id and never runs the task. | A task never executes. |
| `StartSession` | Starts nothing during an install; raises Business Central's error in a test run | No second session exists. |
| Isolated storage | One store per app | User scopes are refused. |
| Table connections | Bookkeeping, and the `@@test@@` CRM connection | Any other connection string is refused. |

## Refused by design

| Area | Instead |
|---|---|
| Email, SMTP and Exchange | Put the send behind an AL interface and fake it in the test. |
| Cloud storage, Key Vault, external data connections | Fake the interface. |
| File upload and download dialogs (a browser round trip) | Test with streams. |
| Report rendering to PDF, Word, Excel or HTML. The XML dataset works. | Assert on the dataset. |
| Web service endpoints (OData, SOAP, API) | Call the page or codeunit directly, or use `TestPage`. |
| Background jobs and the job queue running tasks | Call the code the task would run. |
| Licensing and entitlement denial paths | Use a real service tier. |
| Multi-user locking, deadlocks, SQL error behavior | Use a real service tier. |
| Windows-only .NET libraries (`System.Drawing`, imaging) | Fake the image step. |
| Apps that ship without AL source | Provide an `.app` with source. The message names the app. |

## DotNet

The core needs no .NET. AL code that uses the `DotNet` type is served by an optional
.NET 8 helper process. A closed set of pure framework members runs inside
`al-runner` and needs no helper: `StringBuilder`, `byte[]` access, `Encoding`, SHA
and MD5 hashes, and Base64 conversion.

Tests that reach any other `DotNet` call need the .NET 8 SDK. The binary carries the
helper sources and builds the helper into the cache on first use. Without .NET those
tests are not run, with the reason, and the exit code is 8. `--require-dotnet` turns
a missing helper into an error, for CI. Arbitrary .NET that reaches the machine
(`System.IO.File`, `System.Net`, `Process`, the registry) is refused by a policy list.

## Not available yet

- Watch mode. It was rejected for 3.0.0, because `serve` keeps the compile warm and `--affected` skips unaffected tests. See [Editor and tool integration](editor-tools.md).
- The Legacy, IntegrationTest and Uncategorized lanes of `--recipe microsoft`. They need Microsoft's demo company, and building it is experimental and off by default.
- Builds for platforms other than Linux x64, Windows x64 and macOS arm64.

If you hit a "not run" that none of this explains, see [Reading results](results.md).
