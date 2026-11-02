# Status and roadmap

This page holds what implementation work needs: what is built, the open decisions and the known debt. It keeps
no pass counts, because they go stale within a day. For the current numbers:

- `./target/release/al-test-cli --root . --all-versions` runs the corpus on BC 27.5, 28.5 and 29.0 and compares
  them; `--markdown pass-rate.md` writes the one-line-per-version report that CI uploads as a `pass-rate-<date>`
  artifact, and `--json` adds per-area counts and phase times.
- `--top-refusals 15` ranks what is refused (add `--top-refusals-area AREA` for one folder).
- `al-test-cli --must-not-compile DIR [--bc-version N]` checks the must-not-compile cases.
- The dated user-facing summary is the "Status" table in `docs/manual/what-works.md`.
- Microsoft's test buckets: `docs/alrunner-buckets.md`.

## BC versions (ADR-015)

One oracle and one ratchet per BC major (`metrics/<major>/passing.txt`). `al-test-cli --root .` runs the newest;
`--bc-version N` picks one; `--all-versions` runs all three and compares them. On BC 27.5 the corpus compiles
Microsoft's apps from the corpus's own `.alpackages`; the other versions take them from the artifact of the
matching build. The OnPrem app and the isolation probe each have their own oracle, ratchet and report line, and
are not part of the overall count (`oracle/README.md`). There is no isolation oracle for BC 27 and 29
(`docs/probes-needed.md`).

## Must-not-compile

The corpus repo holds AL files the real compiler rejects (`tests/must-not-compile`). Each records the AL codes
(`// @expect AL0161 | text`). The front end must report the same codes. Run: `al-test-cli --must-not-compile DIR
[--bc-version N]`, or `AL_MNC_DIR=DIR cargo test -p al-test-cli --test must_not_compile`; the test skips with a
message when the folder is absent. Our own diagnostics keep their `AR` codes; the AL ones are `alcodes.rs` and
`check/infer.rs` (`al_unresolved_name`). Message text is compared as a note only (the corpus README: the text is a
reference). A case whose code the front end does not implement is "not checked" with that reason, never a pass.
Each check rejects only the shape its case pins; the wider rules are in `docs/probes-needed.md`.

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

- **Company modes (ADR-046).** One company by default; `--companies LIST|all` runs the per-company install
  code in each declared company and models BC with several companies. User text: `docs/manual/writing-tests.md`.
- **Test framework, BC 29 (ADR-059, Accepted).** Data-driven tests (`[TestDataSource]`) run with one result per
  case, `Procedure[identifier]` (`al-testrun` `runtime/data_driven.rs`). `TestHandlers` are refused; the
  `TestHandlerContext` and `DataSourceContext` members work on a fresh variable. A skipped outcome waits for a pin.
- **License-backed permissions (ADR-060).** `--license FILE` and the license in the BC artifact answer License
  Permission (`docs/manual/license.md`). The license is not yet a limit on the permission checks (ADR-062 stage 5).
- **Permission enforcement (ADR-062, Accepted).** Stage 1 is on by default: table data writes in a session a test
  lowered are checked (`al-vm/src/permissions.rs`); other denials refuse the test. Reads, `Execute` and the runner
  sets (stages 2 to 4) wait for probes. `--permissions off` restores the refusal.
- **Stability mode (ADR-058, Accepted).** `--stability` runs variants over one compile and reports flips
  (`docs/manual/stability.md`). **Run seed (ADR-061):** `--seed N`, printed and replayable.
- **Install order.** Apps install one at a time in dependency order; two apps with no dependency path between them have
  no known order, and what one could see of the other is refused (`al-testrun/src/install.rs`).
- **`--dev-endpoint-ui`** answers unhandled UI as BC 28 and 29's dev-endpoint test runner does (flagged pass).

## Largest remaining blockers

Run `al-test-cli --root . --top-refusals 15` for the current list. Each test is counted under the first thing that
stops it.

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

## Roadmap: deferred until users ask (owner, 2026-10-25)

The user-facing roadmap is `docs/manual/roadmap.md`. A deferred or experimental item goes on both pages.

- **App translations** (issue 5772, ADR-053): the `Translations/*.xlf` files of an app folder are applied under another `GlobalLanguage` (labels, enum captions, field captions). The files inside a package and the reads not built yet are still refused.

## Next targets (owner, 2026-10-13; state on 2026-10-14)

1. **Microsoft's own Base App test buckets.** Started. `al-runner` runs them
   (`docs/alrunner-buckets.md`, which holds the dated measurements). The refusal reasons are
   ranked with `--top-refusals`.
2. **CRONUS demo data in native Rust.** Done: `al-bcdb` is the Rust port of bcdb, and
   `al-runner test --test-data` loads CRONUS into the store, table by table (ADR-020).
