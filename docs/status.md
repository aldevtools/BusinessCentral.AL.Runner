# Status and roadmap

As of `main` on 2026-10-14. Oracles: BC 27.5, 28.5 and 29.0 (corpus CI runs 38534169906,
38536067343 and 38537727198), corpus `6219bf4`.
Regenerate the numbers with
`./target/release/al-test-cli --root . --markdown pass-rate.md`; CI uploads
the same report as a `pass-rate-<date>` artifact. The markdown is one line per BC
version (counts, rate, total time). Per-area counts and phase times are in `--json`.

## BC versions (ADR-015)

One oracle and one ratchet per BC major. `al-test-cli --root .` runs the newest;
`--bc-version N` picks one; `--all-versions` runs all three and compares them.

| BC major | Oracle leg | Baseline | Passing | Failing | Not run | Version gates active |
|---|---|---|---:|---:|---:|---|
| 27 | 27.5 | `metrics/27/passing.txt` | 4913 of 5147 | 0 | 234 | none |
| 28 | 28.5 | `metrics/28/passing.txt` | 4913 of 5147 | 0 | 234 | none |
| 29 | 29.0 | `metrics/29/passing.txt` | 4913 of 5147 | 0 | 234 | Integer sum into BigInteger FlowField; min and max over SystemRowVersion |

The same 5147 cases run on every version, and the same 234 are not run. On BC 28.5 and 29.0 the
corpus compiles Microsoft's apps from the corpus's 27.5 `.alpackages`. Only the test libraries
(Any, System Application Test Library, Permissions Mock) come from the artifact of the matching
version.

### Other test apps (2026-10-14)

Each has its own oracle, ratchet and report line, and is not part of the overall count
(`oracle/README.md`).

| App | Oracle | Pass | Total | Why the rest does not run |
|---|---|---:|---:|---|
| OnPrem (27, 28, 29) | CI run, small artifact | 42 | 64 | 22 are not run. 16 need a platform table with no row provider yet (Object Metadata 12, Object 2, Tenant Profile Page Metadata 2), 4 need Published Application columns that no corpus test pins, 2 compare Blob values. No `DotNet` in the app |
| Isolation probe under a Function runner (28) | Windows nightly, BC 28.4 | 7 | 7 | |
| Isolation probe under a Codeunit runner (28) | same artifact | 6 | 6 | BC fails `T5` here on purpose; so does the runtime |

There is no isolation oracle for BC 27 and 29 (`docs/probes-needed.md`).

## Overall

**4913 / 5147 pass (95.5%), 0 fail, 234 not run**, the same on BC 27.5, 28.5 and 29.0.
Measured on 2026-10-14 on a shared 12-thread machine under load: the corpus run takes 2.9 to
6.2 s with Microsoft's apps (six runs) and 0.4 to 0.9 s with `--no-deps` (five runs). Base
Application and the other Microsoft apps are compiled from source on every run of `al-test-cli`, on demand. `al-runner test` keeps their complete compile in a compile cache (ADR-022, `<cache dir>/compile/`); `al-test-cli --compile-cache` does the same for the corpus job. Numbers with the cache are in ADR-022.

| Area | Total | Pass | Not run | Rate |
|---|---:|---:|---:|---:|
| collections | 84 | 83 | 1 | 98.8% |
| error-handling | 82 | 81 | 1 | 98.8% |
| text | 132 | 130 | 2 | 98.5% |
| types | 392 | 381 | 11 | 97.2% |
| fieldref | 126 | 122 | 4 | 96.8% |
| record | 1076 | 1034 | 42 | 96.1% |
| query | 146 | 140 | 6 | 95.9% |
| handlers | 1171 | 1121 | 50 | 95.7% |
| codeunit | 281 | 268 | 13 | 95.4% |
| streams | 60 | 57 | 3 | 95.0% |
| session | 447 | 410 | 37 | 91.7% |
| errorinfo | 23 | 21 | 2 | 91.3% |
| media | 23 | 21 | 2 | 91.3% |
| pageapplicationarea | 76 | 69 | 7 | 90.8% |
| xmlport | 62 | 55 | 7 | 88.7% |
| profile | 16 | 14 | 2 | 87.5% |
| tableextensiontrigger | 13 | 10 | 3 | 76.9% |
| permissions | 58 | 37 | 21 | 63.8% |
| pageextensioncontrol | 28 | 17 | 11 | 60.7% |
| reportextensiontrigger | 10 | 6 | 4 | 60.0% |
| network | 5 | 0 | 5 | 0.0% |

Every test passes in 19 more areas (836 tests): access-control, autoformat, dateformulacontrol, extensionevents, filterpage, install, json, pageextensionpart, pageextensiontrigger, pagefindrecord, pagesave, pageupdate, recordref, tableextension, testfilter, testpageclose, testpageteardown, testpart, xml.

## Must-not-compile (2026-10-14)

The corpus repo holds AL files the real compiler rejects (`tests/must-not-compile`, branch `master`; the
submodule does not carry them yet). Each records the AL codes (`// @expect AL0161 | text`). The front end must
report the same codes. Run: `al-test-cli --must-not-compile DIR [--bc-version N]`, or
`AL_MNC_DIR=DIR cargo test -p al-test-cli --test must_not_compile`; the test skips with a message when the folder is absent.
Our own diagnostics keep their `AR` codes; the AL ones are `alcodes.rs` and `check/infer.rs` (`al_unresolved_name`).

| BC major | Cases | Match | Mismatch | Not checked |
|---|---:|---:|---:|---:|
| 27, 28, 29 | 16 | 15 | 0 | 1 |

Match: AL0161 (2 cases), AL0440, AL0162, AL0118, AL0296 (2), AL0124, AL0365, AL0132 (4), AL0385, AL0197.
AL0122 (cases 0069, 0070, 0074, 0075): an Integer variable into a Text, a text literal into an Integer, and an
enum into an enum of another type whose target does not state `AssignmentCompatibility = true`.
Not checked: 0008 (AL0175, `=` between two DotNet class objects): an enum is comparable and a class is not, and the
front end has no assembly metadata. Message text is compared as a note only (the corpus README: the text is a
reference). A case whose code the front end does not implement is "not checked" with that reason, never a pass.
Each check rejects only the shape its case pins; the wider rules are in `docs/probes-needed.md`.
A run of the workspace's code prints `compile: error ALnnnn ...` on stderr and goes on; nothing is refused by it yet.
Guard against rejecting code BC accepts: the corpus (27, 28, 29), Tests-ERM and NP Retail give no such line and the same counts.

## Platform tables (ADR-019)

The platform `System` app loads with every app. Providers answer Integer, Date, Session, Active Session, AllObj,
AllObjWithCaption, CodeUnit Metadata, Table Metadata, Page Metadata, Page Control Field, Page Action, Field, Key,
Time Zone, Windows Language, NAV App Installed App, Published Application, the permission tables, Report Layout
List, Report Metadata, Report Data Items, XmlPort Metadata, Query Metadata, Event Subscription (compiled
subscribers) and Table Relations Metadata. Company, User, Access Control, All Profile, Record Link, the code
coverage tables and a few tenant tables are stored (the session seeds the first four). Everything else in the
System app lowers, works as a temporary record, and has its persistent rows refused with `platform table 'X' (id) has no provider`. Still open: Tenant License
State, MCP Configuration and Event Subscription rows for uncompiled codeunits. The three code coverage tables hold what a
`CodeCoverageLog` recording saw (hits per statement, the tests that started, the methods that ran in them; `docs/probes-needed.md`,
"Code coverage rows").

## Microsoft's demo company builds: experimental, off by default (owner, 2026-10-23)

The builds of Microsoft's demo company (the DemoTool company of the Legacy lane, the Contoso companies of the
IntegrationTest and Uncategorized lanes) are on hold and will be redesigned. The code stays, behind
`--experimental-company-builds`. Without it, `--company-data demotool|contoso-setup|contoso-all` is refused, and
`--recipe microsoft` / `al-runner test-microsoft` report those three lanes as not run, with the reason. The general
fixes that came with that work stay on. Details: [alrunner-buckets.md](alrunner-buckets.md) (recipe, step 3).

## What is implemented

- **Front end.** Full lossless parse of the corpus. Namespaces and `using`s,
  extensions, preprocessor symbols, overload resolution, implicit
  conversions.
- **Language.** All statement forms. TryFunction. Labels. Interfaces through
  codeunit variables. List and Dictionary. Option and enum. Date, Time,
  DateTime and Duration arithmetic. Format and Evaluate for the pinned forms.
- **Records.** CRUD, triggers, filters (SetRange, SetFilter, filter groups,
  marks), keys, Validate with TableRelation checks, Rename with propagation to
  related tables, FlowFields and FlowFilters (all CalcFormula methods),
  CalcSums, SetAutoCalcFields, AutoIncrement, Truncate, temporary records,
  SystemId and SystemRowVersion, transactions (Commit, `asserterror`
  rollback, test-boundary commit).
- **RecordRef, FieldRef, KeyRef and RecordId.**
- **Events.** Integration, business and internal publishers. Static and Manual
  subscribers, BindSubscription, table events in BC's pinned order, table
  extension triggers, Isolated events.
- **Data formats.** JSON, XML DOM with simple XPath, Blob streams, text
  functions, TextBuilder, BigText, SecretText.
- **XmlPorts (Format = Xml).** `Import` and `Export` (instance and static),
  `SetSource`, `SetDestination`, `SetTableView`; table, field, text elements and
  attributes, nested table elements with `LinkFields`, `AutoSave`, `AutoUpdate`,
  `AutoReplace`, `UseTemporary`, and the triggers the corpus uses. 55 of the 62 xmlport tests pass; the
  other 7 are not run.
- **Test runner.** Install codeunits. `[HandlerFunctions]` with Message,
  Confirm, StrMenu, page and modal page handlers.
- **Pages (ADR-012).** TestPage on Card and List pages: lifecycle,
  navigation, SetValue and validation, actions, page triggers,
  `CurrPage.Update`, `Page.Run` and `RunModal`, lookup mode, SaveValues,
  control-text formatting. Parts and `TestPart` (SubPageLink, SubPageView,
  Provider, navigation, new rows with AutoSplitKey, `CurrPage.<Part>.Page`
  calls), `TestFilter`, `Page.SetRecord`, SourceTableTemporary pages.
- **Queries, page extensions, request pages, reports.** 140 of 146 query tests pass. Page
  extensions: 44 of 55 tests in the three `pageextension*` areas pass. All 32 filter page tests
  pass. 6 of 10 report extension trigger tests pass. `Report.SaveAs` writes the XML dataset only.
- **Dependency apps (ADR-013).** Microsoft's System Application, Business Foundation and Base
  Application are compiled from source. `al-test-cli` compiles on demand; `al-runner test` loads the complete compile of the dependency apps from a compile cache (ADR-022), built on the first run. Platform tables are served by
  row providers (ADR-019). `DotNet` calls go to an optional .NET 8 helper process (ADR-014); the
  server classes that need a service tier are provided by the runtime.
- **Other command lines.** `al-runner` runs the tests of any app and of Microsoft's test buckets
  (`docs/alrunner-buckets.md`). `al-bcdb` reads `.bak` and `.bacpac` files, and
  `al-runner test --test-data` restores CRONUS from one (ADR-020, `docs/bcdb.md`).

## Largest remaining blockers

234 tests are not run on each BC version, for 107 distinct reasons
(`al-test-cli --root . --top-refusals 15`, BC 29.0). Each test is counted under the first thing
that stops it. The 15 most common reasons:

| Tests | What stops it |
|---:|---|
| 16 | Permissions of tenant sets: the platform keeps the last composed set per setup version, which is not modelled (`TestMetadataExpandedPermission.al:196-207`) |
| 13 | `RecordId.GetRecord` is not implemented |
| 11 | `foreach` over a `DotNet` value is refused until a corpus test pins it (ADR-014) |
| 9 | A table with `TableType = CRM` is served by a table connection, which the runtime does not model |
| 6 | `OnQueryClosePage` in a page extension is not supported |
| 5 | `DotNet` class `Microsoft.Dynamics.Nav.Runtime.PermissionTestHelper` is neither registered nor classified pure (ADR-014) |
| 5 | A part page with a control of kind `cuegroup` is not supported |
| 5 | A `DotNet` value passed to `Format`: its type is only known at run time |
| 5 | Reading Code Coverage Tests Run after a coverage recording: the runtime records no coverage |
| 5 | Reading Code Coverage after a coverage recording: same |
| 4 | Page extension code on members of the base page is not lowered yet |
| 4 | `Report.DefaultLayout` is not implemented |
| 4 | The User Personalization row of the session user: BC creates it at client sign-in and no corpus test pins its content (`docs/probes-needed.md`) |
| 3 | `Evaluate` into DateTime with a format that no corpus test pins |
| 3 | `GetRangeMin` without a filter: the error text is not pinned |

The other 92 reasons account for 136 tests.

## Open decisions for the project owner

The open items are listed in `docs/owner-review.md`. ADR-013 (dependency apps), ADR-014
(`DotNet`) and ADR-020 (`al-bcdb` and test data) are accepted and built. ADR-015 (BC versions)
and ADR-016 (release) are still Proposed (`docs/adr/README.md`).

Answered on 2026-10-13 and recorded in ADR-013 (and, for `DotNet`, on 2026-10-13 in ADR-014):

- Licensing: download Microsoft's app source into a local cache and a CI cache,
  compile it, commit nothing.
- Native layer: AL that exists is taken as AL and never shimmed. Platform
  tables with no AL body get native row providers.
- Version selection: driven by the project's `app.json`, using the same
  mechanism as AL Runner. Not corpus-specific. The corpus harness passes the
  oracle's BC minor.
- Budget: demand-driven compilation. A persistent cache comes last, after the remaining gaps and
  the performance work (`docs/owner-review.md`, 2026-10-14). It is built (ADR-022): one entry per set of
  dependency apps, on by default in `al-runner`.
- Install triggers and Company-Initialize: mirror AL Runner.

## Known issues and debt

- `GetLastErrorCallStack` and `SessionInformation.Callstack` list real AL frames, innermost first with a
  backslash after each, the test runner's frames below the test (pinned: session/TestSessionInformationCallstackContracts.al).
  Frames of pages, reports, queries, xmlports and extensions are still refused, and so is `Callstack` inside an event
  with no publisher routine. `ErrorInfo.Callstack()` is not implemented.
- Red/green discipline slipped in places during the parallel build-out. A
  mutation-testing pass (cargo-mutants, 2026-10-14) covered event dispatch,
  FlowFields and CalcFormula, transactions, XmlPorts (runtime and sema), queries,
  reports, host values, the TestPage parts, and the Format, date, math and
  ErrorInfo builtins. Every surviving mutant was either killed by a new test or
  recorded as equivalent or unpinned. What is left: `al-vm/src/pages.rs` (the
  TestPage core, 520 mutants) and the rest of `datefns.rs` were not swept; of the 47
  `pages/*` survivors only 14 were rechecked against the corpus ratchet (9 caught, 5
  survive both: `pages/areas.rs` check_areas_pinned, `pages/parts.rs`
  resync_parts_linked_by, `pages/runobject.rs` run_object_codeunit).
- About 400 assumptions in `docs/probes-needed.md` wait for probes on real BC.
  They include the exact "Unhandled UI" text, JSON and XML `WriteTo`
  formatting, and the `Format` of Time and DateTime.
- `al-host` holds the capability traits the core crates use: the clock, identity and session
  defaults, the random seed, `DotNet` and the test-data tables. The network is not there yet.

## Next targets (owner, 2026-10-13; state on 2026-10-14)

1. **Microsoft's own Base App test buckets.** Started. `al-runner` runs them
   (`docs/alrunner-buckets.md`). All 32 buckets together hold 40300 tests; without test data
   1083 pass, 858 fail and 38359 are not run. With `--test-data`, Tests-ERM has 487 passing
   and 5 failing. The refusal reasons are ranked with `--top-refusals`.
2. **CRONUS demo data in native Rust.** Done: `al-bcdb` is the Rust port of bcdb, and
   `al-runner test --test-data` loads CRONUS into the store, table by table (ADR-020).
