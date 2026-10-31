# ADR-027: Scope: what AL Runner v3 runs, replaces, refuses and defers

- Status: Accepted (owner decisions 2026-10-15; see the end). Amended (owner, 2026-10-29): rows 9, 14, 17 and 18 of section 3. Amended (owner, 2026-10-31): row 15 of section 3 (CRM tables through the `@@test@@` connection), Accepted (owner, 2026-10-31)
- Date: 2026-10-15
- Needs: owner review of section 5 (open decisions) before it is accepted

## Context

AL Runner v2 has a scope manifest (`docs/scope.md` in the v2 repository). It sorts every
surface into four buckets: real code, faithful replacement, out of scope (the runner throws),
and not yet built. Its section 3 is the list users read when a test fails with
`RunnerOutOfScopeException`.

v3 has no such list. Its rules are spread over the hard rules in `CLAUDE.md` (match BC where
observable, refuse explicitly, never fake objects of dependent apps), ADR-009, ADR-013,
ADR-014, ADR-016, ADR-019, ADR-020, and the rows of `docs/probes-needed.md`. A user who hits
"not run" cannot tell whether the feature is missing, refused for good, or waiting for a probe.

This ADR is that list for v3. It records the stance for each v2 section-3 area, what v3 does
today (with the code that does it), and the decisions the owner has not made yet.

It changes no seam. It does not change code. Where it names a default for an open decision,
the default is a recommendation, not a fact.

## How v3 differs from v2 (read this first)

| | v2 | v3 |
|---|---|---|
| Out-of-scope surface | The test **fails** with `RunnerOutOfScopeException` | The test is **not run**, with a reason (exit code 7 unless `--allow-not-run`) |
| Where refusal happens | At run time, in a patched .NET method | Preferably before the run: a lowering reason or an `al-sema` diagnostic. Otherwise `Signal::Unsupported` mid-run |
| Why a surface is refused | "Out of process", "needs a service tier" | Two separate reasons: **by design** (section 3) or **not pinned yet** (section 4, hard rule 3) |
| Error text | Anchors into `scope.md` | The refusal text names the surface. A later change adds an anchor into this ADR |
| Unknown BC behavior | Often guessed or stubbed | Refused until a corpus test or probe pins it |

"Not pinned yet" is the larger group today. It is not a scope decision. It shrinks as probes
land (`docs/probes-needed.md` must shrink, per project policy). This ADR only lists the
refusals that are decisions.

## 1. Real behavior, as BC

v3 executes these as BC does. The corpus oracle (ADR-007) settles them.

| Area | Notes |
|---|---|
| AL language | Lexing, `#if`, parsing, type check, overload resolution, namespaces, extensions |
| Value semantics | Text, Code, Decimal, Date, DateTime, Option, Enum, Guid, BigInteger, Duration; formatting and `Evaluate` (fixed en-US culture) |
| Records | CRUD, filters, ranges, keys, FlowFields, AutoIncrement, SetLoadFields, rename propagation, `TransferFields`, RecordRef and FieldRef |
| Triggers and events | Table, page and codeunit triggers; integration and business events; subscribers, manual binding, order |
| Errors | `asserterror`, `TryFunction`, `GetLastError*`, error text and code, call depth ceiling (992) |
| Collections and data | List, Dictionary, JSON, XML, streams, BLOB, TextBuilder, Query (the pinned shapes), XmlPort import and export over streams |
| Reports (processing) | `Run`, `RunModal`, `Execute`, `RunRequestPage`, data items, `OnPreReport` to `OnPostReport`, reportextensions, dataset as XML (ADR-025) |
| Pages (headless) | The test client of ADR-012: open, `SetValue`, validation, actions, navigation, modal and page handlers |
| Microsoft's apps | Compiled from Microsoft's own AL source (ADR-013). Posting routines, validation, System Application modules run as written |
| `DotNet` | Real .NET code through the helper process (ADR-014), under the policy list `crates/al-vm/src/dotnet/policy.txt`. Needs the .NET 8 runtime |

## 2. Faithful replacement

v3 swaps in something AL code cannot tell apart from BC for what it can observe. Each row
states where the faithfulness ends.

| Surface | Real BC | v3 replacement | Boundary |
|---|---|---|---|
| Table storage | SQL Server | In-memory store on persistent maps (ADR-003, 004) | No row locks, no other session, no SQL error. `LockTable(false)` is refused (`al-vm/src/recmeta.rs:301`) |
| Transactions | SQL transactions | Snapshots. A passing test commits at its boundary. `asserterror` and a failing test roll back to the last commit (`docs/architecture.md`, runtime model) | Commit order across sessions is not modelled. `TestIsolation = Disabled` is refused (`al-testrun/src/isolation.rs:10`) |
| Test data | A restored database | CRONUS from the BC artifact (`.bak`), loaded per table on first touch, on by default (ADR-020). `--no-test-data` turns it off | Only the demo company. Tables whose column shapes are unmeasured are refused with the table and column named. The `timestamp` column is left at its default |
| Identity | Live user, tenant, company, session | `TESTUSER`, fixed user security id, tenant `default`, company `My Company`, session id 1 (ADR-013, `al-host`) | Tests that probe real authentication, other users or telemetry identity |
| Environment | Real server | `--environment saas` (default, as probed) or `onprem` (`al-host/src/lib.rs:72`). `Environment Information` stays the real System Application codeunit | Server topology is fixed. No real tenant settings |
| Clock | Wall clock | `Clock` capability. The command line injects the system clock. The deterministic host is frozen at 2024-01-15T12:00:00Z (`docs/probes-needed.md`, "Clock"). `Sleep(n)` does not wait; it moves the session's view of time | No real waiting. A test that needs a real elapsed time sees a jump, not a delay |
| Client UI | Web client | Handlers: `MessageHandler`, `ConfirmHandler`, `StrMenuHandler`, `ModalPageHandler`, `PageHandler`, `RequestPageHandler`, `TestPage` client. An unhandled dialog fails the test, as on BC | No pixels, no layout. Add-ins and `usercontrol` have no client |
| Platform tables | Server-side system tables | Providers keyed by table id (ADR-019). Stored tables are seeded with the session's facts | A table with no provider is refused: `platform table 'X' (id) has no provider`. A column with no pinned value is refused |
| Permissions | Evaluated per user | All access is granted. `Permissions Mock` is accepted and never evaluated. `SetPermissionFilter` filters nothing (`al-builtins/src/native.rs:481`). The session user's effective permission is answered; other users are refused | Any test that expects "access denied" fails here and passes on BC. See open decision 3 |
| Task scheduler | Job queue and task engine | `CanCreateTask` is false (pinned, `native.rs:302`). `CreateTask` returns a new id and never runs the task (`native.rs:306`) | A task never executes. `TaskExists`, `SetTaskReady` and `CancelTask` are refused (`native.rs:308-315`) |
| `StartSession` | A second server session | During an install: starts nothing, answers false. In a test run under `TestIsolation = Codeunit`: the same error BC raises (`native.rs:887`). Anywhere else: refused | No second session exists. See open decision 4 |
| Page background tasks | A worker session | The worker runs in the same session, cannot write to a table, and gets the pinned permission text (ADR-018) | Not parallel. A direct `Commit` in a worker is refused |
| Table connections | Dataverse, external SQL | Bookkeeping plus the CRM test connection `@@test@@` only (`al-vm/src/sessionops.rs:51`). A `TableType = CRM` table reads and writes an in-memory store per registered connection (section 3, row 15) | Any other connection string is refused. `ExternalSQL` registration raises the permission error BC raises for a Cloud extension |
| Isolated storage | Per user, company, module | One store per app, company scope adds the session company, rolled back with the transaction | User scopes are refused |
| Code coverage tables | Platform recording | `CodeCoverageLog` records what the session saw | Statement coverage output is deferred (section 4) |
| Platform `File` | Server file system | A temp area under `File::TEMP_DIR` through the host. A context without a file system refuses (`native.rs:171`) | Browser round trips are not faked (section 3, row 5) |

## 3. Deliberately not built

Refused with an explicit reason. "Until" means: until the owner decides otherwise, not
"until a probe". The last column compares with v2.

Reason codes: **OOP** out of process. **NET** needs a network or an external server.
**ND** non-deterministic. **SRV** needs a real service tier. **LIC** licensing or entitlements.
**CLR** needs the .NET engine v3 does not have.

| # | AL surface | v3 today (file:line) | Why | Instead | v2 |
|---|---|---|---|---|---|
| 1 | Email: `Email.Send`, `Email.Enqueue`, `Email.OpenInEditor`, SMTP, `Mail Management` | No special code. The Email module compiles from Microsoft's source and stops where it needs a connector: `HttpClient` (not implemented, row 2) or an SMTP/Exchange assembly (`dotnet/policy.txt:21-36`). The test is not run. **Not measured by a run for this ADR** | OOP, NET | Put the send behind an AL interface and fake it in the test. Or capture mail with a test library codeunit | Throws `external-smtp`. Same stance |
| 2 | Outbound HTTP to a real network: `HttpClient.Get`, `Post`, `Send`, `Put`, `Delete`, `Patch` | Built (owner decision 1, 2026-10-15): a call is answered by the `[HttpClientHandler]` of the running test (`al-vm/src/http.rs`, `al-builtins/src/http.rs`). A call with no handler raises BC's policy error under `BlockOutboundRequests` and `AllowOutboundFromHandler` (code `TestCodeunitUnhandledHttpRequest`) and is refused otherwise ("never opens a connection"). A handler that answers true would send the request for real: BC first asks whether to allow it (StrMenu), so with no StrMenu handler the call answers false with the unhandled-UI error as the last error (al-language-onprem/network/lt-ay-http-handler/AyHttpHandler.al). Before BC 29 a handler is not called under BlockOutboundRequests and AllowOutboundFromHandler; a handler under AllowAllOutboundRequests or no policy is refused (not probed). `HttpRequestMessage`, `HttpResponseMessage`, `HttpContent`, `HttpHeaders` and the handler's `TestHttpRequestMessage` / `TestHttpResponseMessage` are modelled | NET, ND | A `[HttpClientHandler]` mock | Throws `external-http`, but serves `[HttpClientHandler]` mocks in process |
| 3 | `TestHttpRequestPolicy = BlockOutboundRequests` and the unhandled-request error | Built: the error text and code are the probe's (session/PZHttpBlock.al on the corpus staging branch) | NET | Same as row 2 | Runs on BC's own dispatcher |
| 4 | Web service publishing: OData, SOAP, API endpoints served from pages and codeunits | No web server exists. `WebServiceActionContext` is modelled in process (`al-builtins/src/wsac.rs`). `Web Service` rows are plain table data | OOP, NET | Call the page or codeunit directly. Test API pages through `TestPage` | Throws `web-services` |
| 5 | Browser file round trips: `File.Upload`, `UploadIntoStream`, `Download`, `DownloadFromStream`, `View` | Refused: "needs a client (a browser round trip), which a headless run has not" (`al-builtins/src/file.rs:324-358`) | OOP | Use a stream (`TempBlob`, `InStream`/`OutStream`) in the logic and test that. The static `XmlPort.Export`/`Import` stream forms run | Throws. Same |
| 6 | Cloud blob and file storage: Azure Blob, Azure Files, `File Management` against real disks | The Azure SDK is on the `DotNet` deny list (`dotnet/policy.txt:29-30`). Real disk paths through `File` are not reachable beyond the temp area | NET | Fake the storage interface | Throws. Same |
| 7 | Printing and layout rendering: `Report.Print`, `SaveAsPdf`, `SaveAsExcel`, `SaveAsWord`, `SaveAsHtml` to a real file | `SaveAs` with a stream writes the XML dataset only. Other formats: `only the Xml dataset is produced (a rendered format needs a layout renderer ...)` (`al-vm/src/reports.rs:1240`). `Report.SaveAsPdf#2 is not implemented` (corpus case 61292). A run that reaches output is refused | OOP. Bytes would differ from BC's renderer, so a test would assert against the wrong thing | Assert on the dataset (`SaveAsXml`, `Library - Report Dataset`). Request page and data item logic run | Throws `out-of-scope: NavReport.SaveAs*`. v2 also serves layout selection rows |
| 8 | Background jobs: job queue execution, `TaskScheduler` running tasks | See section 2. The task never runs | ND, SRV | Call the codeunit the task would run. Test the job queue entry's `OnRun` directly | Throws or skips. Same |
| 9 | External crypto and key stores: Key Vault, certificate validation against a store | Key Vault and the Azure SDK are denied (`dotnet/policy.txt:29-30`). In-process hashing and `Encrypt`/`Decrypt` work (`sessionops.rs:542-575`); decrypting text another session made is refused. **Amended 2026-10-29:** Key Vault will answer what BC answers in a test session with no vault configured. Refused until that probe lands (corpus writer TY) | NET, LIC | Test with in-process keys. Fake the key provider behind an interface | Throws. Same |
| 10 | Licensing: license file checks, entitlements, `IsLicensed` denial paths | Not modelled. No license state is held. Tenant License State rows are refused (`docs/status.md`) | LIC | Do not assert on denial | Returns "all granted", throws on denial probes |
| 11 | Parallel sessions: `StartSession` with its own state, `IsSessionActive`, cross-session timing | See section 2: install starts nothing, test run raises BC's error, anything else is refused | SRV, ND | Call the code the session would run | Runs inline for logic, not for contracts |
| 12 | Multi-user locking and SQL error behavior: lock timeouts, deadlocks, `LockTable(false)` | `LockTable(false)` refused (`recmeta.rs:301`). Locks never conflict | SRV | Not testable here. Use a service tier | `Commit` no-op, no locks |
| 13 | Debugger AL surface: `Debugger.Attach`, `Break`, step calls | No such builtins. The platform "Debugger *" tables exist as names only (`al-sema/src/alcodes.rs:148`) | SRV | The planned debug adapter (section 4) is a tool, not an AL API | Throws. v2 has a DAP server |
| 14 | .NET Windows-only libraries (`System.Drawing`, imaging) | **Amended 2026-10-29:** on Windows, `System.Drawing` runs for real through the .NET sidecar (GDI+, the same bytes as BC). On Linux and macOS it stays refused with the reason in `dotnet/policy.txt` (`deny-type-off-windows System.Drawing.`). The front end that starts the sidecar picks the rules (`al-testrun/src/dotnet.rs`, `sidecar_config`). On Windows the ADR-044 registry (`dotnet/drawing.rs`) claims no System.Drawing type, so a Bitmap and a Graphics are always real .NET objects; elsewhere it still serves its eight members first. A .NET exception from the helper is `Unsupported` with its type and message | CLR, platform | Run on Windows, or fake the image step. v3 will not substitute an imaging library (bytes would differ from GDI+) | `dotnet-platform-unsupported` |
| 15 | Live external table connections: `TableType = CRM` with a real connection, `ExternalSQL`, `Exchange`, `MicrosoftGraph` | `RegisterTableConnection` with any string except `@@test@@` is refused (`sessionops.rs:100`). **Amendment Accepted (owner, 2026-10-31):** a `TableType = CRM` table through the `@@test@@` connection is modelled as BC's in-memory test provider: one store per registered connection name, used through the default connection (`al-vm/src/crm.rs`). Pinned by tests/al-language-onprem/session/Y16CrmTableConnectionOps.al:143-318 and session/TestDatabaseTableConnectionCrm.al:128-170: Get, FindFirst, FindLast, FindSet, Next, Count, IsEmpty, filters, Insert (a null Guid key gets a new Guid), Modify, Delete, DeleteAll, Rename, `DB:RecordNotFound` and `DB:RecordExists`, the "must be registered" error unless a connection is the default, Unregister drops the connection's rows. Methods that touch neither rows nor connection (captions, filters read back, TestField, Validate, TransferFields, RecordId) act as on any table. Refused: every other method that reads or writes rows (ModifyAll, CalcFields, LockTable, Find, SetLoadFields and others), a write the table's or an extension's triggers, a table event subscriber or a database trigger would see, a Rename other fields relate to, a key other than one Guid field, `RecordRef.Open` of the table, and the rows after a rollback that came after a write. The CRM assembly and any real Dataverse connection stay refused | NET | Use the `@@test@@` connection, or abstract the data source | Refused by name on first access |
| 16 | Compiled-only dependency apps without AL source (DP-10) | The dependency is not found; tests that reach it are not run and the message names the app (ADR-013, amendment 2026-10-15) | CLR. v3 cannot run compiled C# | Provide an `.app` with AL source (`ShowMyCode`), or fake the dependency's interface | v2 loads the compiled code. **A real difference**: v3 cannot do this |
| 17 | Arbitrary `.NET` that reaches the environment: `System.IO.File`, `System.Net`, `Process`, `Environment`, `Reflection.Assembly.Load`, registry | **Amended 2026-10-29:** `System.IO` file access runs against an in-memory file system, one per run (its own ADR, on branch `np-fs`). The rest is refused by the policy list, with the rule shown (`dotnet/policy.txt:39-58`) | OOP, ND | Use AL `File`, streams and interfaces | Runs inside the one process |
| 18 | Page rendering and client add-ins: `controladdin`, `usercontrol`, profiles, role centers as pixels | Not built. A page's control tree is headless (ADR-012). **Amended 2026-10-29:** pages with add-ins will work in `TestPage` tests: add-in calls and events behave as BC's test client does. Probed by corpus writer TZ (lt-ac/ACControlAddIn.al TZ_*): BC never creates the add-in in a test session, every call of an add-in procedure raises 'The control add-in on control <ctl> on page <page> has not been instantiated.' (DotNetControlAddInNotCreated, wrapped by the caller's context), and add-in event triggers never run; the page itself works. Built that way from BC 28; BC 27 refuses the call when it runs. Nothing is rendered | SRV | Test the page's logic through `TestPage` | Throws. Same |
| 19 | Telemetry and audit logs as readable data: `LogMessage`, `LogSecurityAudit` | Accepted, kept nowhere readable (`native.rs:257-260`, ADR-009) | Not observable from AL | Subscribe to your own event | Accepted |

Rows 10, 12 and 19 are limits to know rather than per-call refusals: v3 answers, and the
answer is not BC's. Permission enforcement is the same kind of limit (section 2, open decision 3).

## 4. Deferred

Not refused forever. Each is waiting for a decision, a probe or an interface.

| Item | State | Waits for |
|---|---|---|
| Server mode (`--server`, JSON lines) | Deferred (owner, 2026-10-13, ADR-016) | Warm start with Base Application measured slower than a test run, or an editor integration is wanted (ALchemist, LethAL) |
| Watch mode, `--affected` | Deferred (ADR-016) | Same trigger |
| `--jobs` worker parallelism, persistent caches beyond ADR-022 | Deferred (ADR-016) | Same trigger |
| Debugger (DAP, breakpoints, step) | Deferred. The IR carries source positions (`docs/architecture.md`); the hook is the `S::Pos` arm of `Session::exec` | An ADR for the protocol (parity ED-9) |
| Statement coverage output, `captureValues` | Deferred. Same hook | Same ADR; Cobertura format (parity OUT-8) |
| `--tdd` stub generation | Deferred | Its own ADR (parity SEL-10) |
| Compiled-only apps (row 16) | Not planned | Nothing in v3 can do it. Stays refused |
| `TestIsolation = Disabled` | Refused until a probe | A fixture that shows what stays between codeunits |
| Query `SaveAsCsv`, aggregate edge shapes | Not implemented (probes pending) | Corpus probes |
| Platform tables with no provider | Refused per table | ADR-019 providers, one pin each |
| Multiple BC majors beyond the three built | Proposed (ADR-015) | Owner acceptance |

## 5. Open decisions for the owner

Each item: the question, why v3 could differ from v2, and the recommended default.

| # | Question | Recommended default |
|---|---|---|
| 1 | **HTTP.** Build `HttpClient` with `[HttpClientHandler]` mocks served in process? v2 does (its docs call this the way to test an HTTP boundary). The corpus app cannot compile the attribute (scope OnPrem). Probes live in the OnPrem app | Yes, handler-served only. The default policy blocks unhandled sends with BC's own error. Never open a socket |
| 2 | **HTTP to localhost** (a test server in the same CI job)? | No. A socket is outside the process and breaks the "no network" promise. Use handler mocks |
| 3 | **Permission enforcement.** v2 made "grant everything" a permanent, deliberate divergence. v3 could model `Permissions Mock` and permission sets | Keep "grant everything" for 3.0. Document it. Revisit after Base Application's own permission tests pass, behind an opt-in flag (ADR-009 limits) |
| 4 | **Background sessions** (`StartSession`, `TaskScheduler.CreateTask`): run the target inline after the test commits? | Not in 3.0. Keep refusal (`StartSession`) and "never runs" (`CreateTask`). A later opt-in `--run-sessions-inline` can be proposed with probes |
| 5 | **Fake clock.** Offer `--now` or `--freeze-time` on the command line? The deterministic host exists in `al-host` | Yes, as a flag. Default stays the system clock, as v2 |
| 6 | **Email capture.** Provide a built-in "test mail outbox" so `Email.Send` is recorded, not refused? | No. Needs a connector interface BC does not expose to tests. Keep refusal. Document the interface pattern |
| 7 | **Report layout rendering** (PDF, Word, Excel). Never, or optional? | Never in the core (bytes differ from BC). Layout selection rows already work. Revisit as an optional helper if users ask |
| 8 | **Exit code for "not run"** (v2 fails the test; v3 exits 7). Does CI treat 7 as failure? | Keep 7, fail by default in the GitHub Action, `--allow-not-run` opts out. This is ADR-016's open question |
| 9 | **Compiled-only dependencies** (DP-10): refuse, or ship an optional .NET path? | Refuse. Name the app and say "provide an `.app` with source" |
| 10 | **Windows-only .NET libraries.** Same stance as v2 (refuse, run on Windows works)? | Same as v2. The helper reports the exception; no imaging substitute |
| 11 | **`DotNet` network and file access** (policy list). Allow an opt-in? | No. Keep the deny list; an allow flag needs an ADR |
| 12 | **User-visible anchors.** Should refusal text link to this ADR's rows (`scope-adr#row-N`)? | Yes. Add a stable reason token per row 3 entry (for example `external-http`) in the refusal text, as v2 does. Code change, separate branch |
| 13 | **Telemetry/audit capture** for tests (readable log)? | No. Not observable on BC either |
| 14 | **`TestIsolation = Disabled`.** Build it or keep refusing? | Keep refusing until a probe shows what persists. Then build |
| 15 | **Multi-user permission and effective-permission queries for other users** | Keep refused. Model only after decision 3 |

## Consequences

- Users get one page that says which "not run" results are by design and which are gaps.
- Each row of section 3 has a reason code, so a refusal text can carry a stable token.
- No change to seams or behavior. Section 5 may lead to later ADRs.
- This page can drift. The check is `al-test-cli --top-refusals`: every top reason must be
  either a gap tracked in `docs/probes-needed.md` or a row here.

## Evidence used

- v2: `docs/scope.md` section 3, `docs/limitations.md`, `docs/al-runner-v2-parity.md`.
- v3 refusals read in `crates/al-builtins/src/{native,http,file}.rs`, `crates/al-vm/src/{sessionops,recmeta,reports}.rs`,
  `crates/al-vm/src/dotnet/policy.txt`, `crates/al-testrun/src/isolation.rs`.
- `al-test-cli --top-refusals 60` on the corpus (BC 27.5, 28.5 and 29.0 sections), 2026-10-15.
- Not verified by a run: rows 1, 8 (job queue), 9, 10, 12. Their v3 behavior is inferred from
  the code paths named. Check these before the ADR is accepted.

## Owner decisions (2026-10-15)

1. **HTTP:** build `HttpClient` with in-process `[HttpClientHandler]` mocks. A send that no
   handler catches gets BC's own blocked-request error. v3 never opens a network connection;
   no localhost exception.
2. **Permissions:** all permissions granted in 3.0, documented. A permission check mode can
   come later as an opt-in flag (docs/ideas.md, idea 12).
3. **Not-run tests in CI:** exit code 7 fails CI by default; `--allow-not-run` opts out.
4. **Background sessions (revised the same day after the probe):** the default matches BC in
   a test: `TaskScheduler.CreateTask` returns an id, `TaskExists`/`CancelTask` work, and the task
   never runs (platform/W6TaskScheduler.al); `StartSession` raises BC's error unless
   TestIsolation = Disabled (session/TestStartSessionRecord.al). An opt-in flag
   (`--run-background-tasks`) runs the task codeunit inline, as a `Codeunit.Run`, and every test
   that passes through it is flagged as differing from BC.

The other open decisions take the recommended defaults listed above (decided by the
orchestrator, recorded in docs/owner-review.md): no localhost, no mail outbox, no report
PDF/Word/Excel rendering in the core, compiled-only dependency apps refused by name,
Windows-only .NET libraries as in v2, no network or file access from DotNet, stable reason
tokens in refusal texts, no telemetry, `TestIsolation = Disabled` refused until probed,
effective-permission queries for other users refused, `--now` fake clock flag yes.

## Owner decisions (2026-10-29)

Amendments to section 3. The table format stays; the changed cells start with "Amended 2026-10-29".

1. **Row 9, Key Vault:** answer what BC answers in a test session with no vault configured. The corpus writer TY
   probes it; until the probe lands, Key Vault stays refused.
2. **Row 14, Windows-only .NET:** `System.Drawing` runs for real through the .NET sidecar on Windows (GDI+, the same
   bytes as BC). On Linux it stays refused with the current reason. On Windows the ADR-044 registry for O365 Brand
   Color.MakePicture claims no System.Drawing type, so every type and member goes to .NET; on Linux it is still asked
   first (orchestrator, 2026-10-29). Decision 10 of section 5 is replaced by this row.
3. **Row 17, `System.IO` file access:** runs against an in-memory file system, one per run, so tests repeat. It has
   its own ADR (branch `np-fs`). The rest of row 17 is unchanged; decision 11 of section 5 is narrowed to it.
4. **Row 18, control add-ins:** pages with add-ins work in `TestPage` tests; add-in calls and events behave as
   BC's test client does. The corpus writer TZ probes it; until the probe lands, they stay refused. No rendering.
   Probe result (corpus PR 792): add-in event triggers never fire in a test session, and every add-in call raises the
   not-instantiated error. Row 18 says what BC does; the runtime does the same from BC 28.

## Owner decisions (2026-10-31)

Accepted (owner, 2026-10-31).

5. **Row 15, CRM tables through the `@@test@@` connection:** a `TableType = CRM` table is no longer refused once a
   CRM connection is the default. It reads and writes an in-memory store per registered connection name, as BC's test
   provider does (corpus PR 912, Y16CrmTableConnectionOps.al:143-318, BC 27.5, 28.5 and 29.0 OnPrem). What the corpus
   does not pin stays refused (row 15). Microsoft's CRM integration tests register this connection through
   `Library - Mock CRM Connection`; before this change about 650 of them were not run for this reason alone.
