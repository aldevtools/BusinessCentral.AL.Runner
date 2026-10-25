# Corpus inventory

Corpus commit `6219bf4` (the `corpus` submodule pin). Oracle: CI run
38534169906, BC 27.5 — **5147 cases, 0 failures**. BC 28.5 and 29.0 have their own oracle
files with the same 5147 cases. Regenerate the per-area numbers with
`python3 tools/inventory.py corpus/tests/al-language oracle/27/junit.xml`.

## Apps in the corpus

| App | `.al` files | Role |
|---|---:|---|
| `tests/al-language` (Target = Cloud) | 1370 | The test app. Everything below refers to it. |
| `tests/al-language-internals-fixture` | 8 | A dependency of the test app (CI publishes it first). Must be loaded as a second app: some `codeunit/` tests call its objects across the app boundary (`ALT Internal Codeunit`, `ALT Cross Compute`). |
| `tests/al-language-xapp-a`, `-b`, `-c` | 3, 4, 2 | Cross-app fixtures (no tests) the test app depends on: B depends on the internals fixture and A, C on A and B. The tests in `events/lt-bw-subscriber-order` and `tableextension/lt-bw-cross-app` use them. |

The runner loads every corpus app the test app depends on (directly or through another of them) from source, in
dependency order, before the test app: `al_testrun::deps::corpus_source_dirs` reads the `app.json` files of the folders
next to the test app. No list of folders is kept in the runtime.
| `tests/al-language-onprem` (Target = OnPrem) | 10 | Not in the main oracle (a separate CI job and artifact). Run as the `onprem` suite against its own oracle (`oracle/README.md`). |
| `tests/al-language-isolation-probe` | 5 | Only in the Windows nightly, under two custom test runners. Run as the `isolation-function` and `isolation-codeunit` suites (BC 28 only). |

The original brief's "519 files / 273 test codeunits" is out of date: the test app
now has 1370 files (121 of them in `_fixtures`) and 723 test codeunits.

## First-task answers

**Assert codeunit.** The corpus ships its own `codeunit 60021 Assert`
(`_fixtures/Assert.al`), modelled on Microsoft's Library Assert but written in
plain AL. No native shim is needed: the runtime executes it like any other
codeunit. It does lean on `Variant` type inspection, `RecordRef`, `GetLastErrorText`,
`GetLastErrorCode` and `StrPos`, so those are on the M2 critical path.

**Handler attributes** (count of attribute lines, whole test app; a line such as
`[ModalPageHandler]` counts once):

| Attribute | Uses | Attribute | Uses |
|---|---:|---|---:|
| `HandlerFunctions` | 360 | `ConfirmHandler` | 10 |
| `ModalPageHandler` | 112 | `MessageHandler` | 7 |
| `RequestPageHandler` | 75 | `SendNotificationHandler` | 2 |
| `PageHandler` | 40 | `StrMenuHandler` | 2 |
| | | `SessionSettingsHandler` | 1 |

Other attributes the VM must understand: `Test` (5149), `EventSubscriber` (137),
`IntegrationEvent` (42), `TryFunction` (36), `TransactionModel` (33),
`CommitBehavior` (7), `ErrorBehavior` (5), `BusinessEvent` (2),
`NonDebuggable` (1), `InternalEvent` (1).

In the M2–M4 areas handlers are rare (`codeunit` 8, `record` 13, `session` 5,
`types` 0).

**Preprocessor symbols.** `BC29PLUS` (8× `#if`, 1× `#if not`), one `BC27PLUS` and one
`BC143PLUS`. No `BC28PLUS` guard is currently used. Emulating BC 27 means
`BC27PLUS` defined, everything else undefined; BC 28 and 29 add `BC28PLUS` and
`BC29PLUS` (ADR-015, `al_version::corpus_symbols`).

**Fixtures per area** (top references): every area uses `ALTFixtureCleanup`.
`record` is dominated by `ALT Universal` (454), then `ALT Trigger Log` and
`ALT Triggered` (72 each), `ALT Parent` (41), `ALT Keyed` (40) and `ALT Link Host` (36). `codeunit` uses `ALT Universal`, `ALT Event Publisher`, `ALT Trigger Log`,
`ALT Manual Event Sub`, `IALTCompute`/`ALTDouble`. `streams` uses `ALT Blob`. `json`, `xml`, `text`
use no table fixtures beyond cleanup.

## Cases per area (oracle, BC 27.5)

| Milestone | Areas | Cases |
|---|---|---:|
| M2 pure semantics | types 392, text 132, collections 84, codeunit 281, error-handling 82 | 971 |
| M3 record engine | record 1076, recordref 118, fieldref 126 | 1320 |
| M4 stretch | json 129, xml 105, streams 60, session 447 | 741 |
| Other areas | handlers 1171, query 146, xmlport 62, profile 16, page*/testpage*/test* and the rest | 2115 |

`handlers/` alone is 23% of the corpus (1171 of 5147). The runtime passes 4913 of 5147
overall (`docs/status.md` has every area).

## Dependencies outside the corpus

The test app declares dependencies on Base Application, System Application,
System Application Test Library and Permissions Mock. Microsoft's apps are compiled from their
own source (ADR-013) and the platform tables are served by row providers (ADR-019). Declared
variables referring to objects the corpus does not define, in the main areas:

| Area | External objects referenced |
|---|---|
| types | `Codeunit "Temp Blob"` (System App, 11), `Enum "Printer Paper Kind"` (8) |
| codeunit | `Record "Code Coverage"` (9) and `Code Coverage Tests Run` (6), `Record "CodeUnit Metadata"` (3) |
| error-handling | `Codeunit "Temp Blob"` (1) |
| record | system / virtual tables: `CodeUnit Metadata` 31, `Record Link` 26, `Date` 25, `Field` 23, `AllObjWithCaption` 21, `AllObj` 20, `Page Control Field` 19, `Table Metadata` 12, `Page Metadata` 12, `Integer` 10 |
| recordref | `Customer Bank Account` and `Item` (1 each) |
| fieldref | Base App tables: `Dimension Correction` 7, `ALLanguage` 6, `Customer` 2, `Analysis Line` 2, `G/L Entry`, `Item Register`, `Inventory Setup` and `Requisition Line` (1 each) |
| session | `Session` 12, `Active Session` 11, `Environment Information` 9, `Application Area Mgmt. Facade` 5 |
| streams | `Codeunit "Temp Blob"` (11), `Base64 Convert` (1) |
| text, collections, json, xml | none |

Cases that need something the runtime does not have yet are refused with a stated reason and
listed as not run in the report. They are not dropped silently (hard rule 4).
