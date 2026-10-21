# ADR-050: `--affected`, `--include-failing`, `--strict-environment`: rerun only the tests a change can affect

- Status: Proposed
- Date: 2026-10-21
- Related: ADR-016 (owner decision 2026-10-21 that 3.0.0 needs v2's features), ADR-028 (serve), ADR-029 and ADR-033
  (incremental compile, symbolic IR with stable keys), ADR-043 (per-test trace and static reach for impact
  selection), ADR-049 (`--watch`), ADR-051 (`--tdd`)
- Parity row: PERF-6 (`--watch --affected`, `--include-failing`, `--strict-environment`), ED-4 (serve fields)

## Context

Today `--affected`, `--include-failing` and `--strict-environment` exit 2, and the serve fields `affectedOnly`,
`includeFailing` and `strictEnvironment` answer an error (`args.rs`, `serve.rs`). The owner decided on 2026-10-21
that 3.0.0 needs them.

The danger of this feature is a false pass: a test that the change breaks, not rerun, reported with its old pass.
The project rule is "never a false pass". So the design selects with exact recorded reach, widens where reach is not
recorded, and runs everything when it cannot vouch for the selection.

### What v2 does

All references are to the `v2` branch: `Program.cs` (flags around 591-593 and 697-712, the cycle around 3185-3205,
`RunTestsWithSelection` around 8170-8430, baseline around 8080-8160), `Infrastructure/Affected*.cs`,
`AffectedObjectId.cs`, `ServerProtocol.cs` (around 194-232 and 600-620), `docs/watch-affected.md`,
`docs/server-mode.md`.

- **Flags.** `--affected` only with `--watch`. `--include-failing` and `--strict-environment` only with
  `--affected`. All three are refused with `--tdd`, `--per-suite`, `--test`, `--filter`, `--test-exact`. Serve:
  `affectedOnly`, `includeFailing`, `strictEnvironment` on `runTests`, default false.
- **Changes.** From the incremental compiler: added, modified, removed and renamed objects, keyed `Kind|id:N` or
  `Kind|name:X`. A per-app snapshot of a SHA-256 per `.al` file, `app.json`, defines, the dependency set, and other
  files the compile read. A file with several objects forces a full run when it changes.
- **Recorded reach per test.** Statement coverage; objects built and routines entered; events raised (all
  publishers seeded so every raise is seen); tables used (`tbl|`), table triggers (`trig|`); dependency objects
  (`dep|`); report extensions (`rext|`); session state read and written (`st|r|`, `st|w|`: WorkDate, number
  sequences, SingleInstance codeunits, LastError, DotNet statics).
- **Selection.** Tests whose keys meet the changed keys, plus unknown tests (new, unrecorded, timed out). A failing
  test is selected by its reach like any other, unless `includeFailing` is set: then every test whose last result
  was not a pass runs.
- **Widening.** Codeunit isolation: the whole codeunit. Test isolation: the whole codeunit only if it has globals or
  `OnRun`. Isolation disabled: the whole app. Session state: readers after the first selected test, writers before
  the last one. A changed object also keys the metadata virtual tables that list its kind (AllObj, Field, Key, ...).
- **Full run** (`forcedFull: true` with a `reason`): no usable baseline; `app.json`, dependency set, removed or
  unclassifiable files; another isolation; another environment, only with `strictEnvironment`; a changed subscriber
  whose event raises were not seen; a page extension change; a changed object nothing built; a compile failure.
- **Environment drift.** Without `strictEnvironment`, a baseline from another environment (other BC version or
  dependency set) is diffed object by object through package and per-object hashes, with a warning on stderr.
- **Baseline store.** `<cache>/affected-baseline/<sha256 of the sorted source paths>.json`, schema 9, written
  atomically, shared by watch and server.
- **Output.** Serve summary: `selection: {mode: "affected", ran, skipped, skippedFailing, changedObjects[],
  forcedFull, reason, environmentDrift?}`. Watch: `[watch] affected: ran X of N   skipped-unaffected A
  skipped-failing B`, then `changed: ...` or `full run — <reason>`, and `not re-run, still failing from an earlier
  cycle: ...`. The failed count reads `failed 0 (+1 still failing, not re-run)`. No `selection` in `--output-json`.
- **Tests.** `Affected*Tests` (7 files), `WatchAffectedReportTests`, `WatchAffectedSelectionTests`, 25
  `ServerAffectedSelection*Tests`, `ServerAffectedSelectionIncludeFailingTests`.
- **Tools.** Neither ALchemist nor LethAL sends these fields today.

### What AL Runner has already

- **Per-test trace** (ADR-043, `al-vm/src/impact.rs`, `al-testrun/src/impact_trace.rs`): events raised (also with no
  subscriber), tables used and written, pages and reports used, keyed by object kind and id, never by program index.
  Off, each site costs one branch.
- **Per-test coverage** (`perTestCoverage`): the statements each test ran, so the routines and objects it entered.
- **Stable keys and per-object symbolic IR** (ADR-033): each object of a compile has a key and a symbolic form that
  does not depend on program order.
- **`RunOptions::narrow`** (ADR-043): a callback between compile and run that picks the codeunits to run.
- **The warm slot** (ADR-028) for serve and watch (ADR-049).

So the recording and the narrowing exist. What is new is the change set, the selection rule and the baseline file.

## Decision

1. **Surface as v2.**
   - CLI: `alrunner test --watch --affected [--include-failing] [--strict-environment]`. `--affected` without
     `--watch`, and the other two without `--affected`, exit 2 as v2.
   - Serve: `affectedOnly`, `includeFailing`, `strictEnvironment` on `runTests`. On `execute` they earn a `warnings`
     entry, as v2.
   - Refused with `--affected` as v2 (exit 2, or `{"error"}` in serve): `--test`, `--codeunit`, `--exclude-tests`
     and the serve fields `test` and `excludeTests`. Not refused, unlike v2: `--tdd` (see decision 8).
2. **Where it lives.** The change set and the selection rule are pure code in `al-testrun/src/affected.rs` (new):
   a baseline value in, a list of codeunits to run and a `Selection` report out. Reading and writing the baseline
   file is in `alrunner-cli` (ADR-006).
3. **The change set: compare per-object symbolic IR.** For each object of the workspace apps the baseline holds a
   hash of its symbolic IR, source positions included. An object is changed when the hash differs. This catches a
   change in its own text, a line shift from an edit above it in the same file (call stacks with lines are visible
   to AL code), and a declaration it reads that changed elsewhere (ADR-033 read lists). A comment-only edit that
   moves no line changes nothing and selects nothing. No file-level rule is needed, so a file with several objects
   is handled exactly, unlike v2.
4. **Selection rule.** A test codeunit is run when one of these holds:
   - a test of it, or what ran before its first test (OnRun, the codeunit's setup), entered a routine of a changed
     object (from the recorded coverage);
   - a changed object is a table or table extension whose table the codeunit used (trace key `table`); a page,
     page extension, report or report extension whose base the codeunit used (`page`, `report`); an enum or enum
     extension the codeunit used (`enum`);
   - a changed object holds an event subscriber (added, removed or changed) to an event the codeunit raised (trace
     key `event`); a `Global Triggers` subscriber selects every codeunit that wrote any table;
   - the codeunit read a metadata virtual table (AllObj, Field, Key, the metadata tables of ADR-019) and any object
     changed;
   - the codeunit is new, was not recorded, or its last run ended in a time limit, a panic, a cancel or a refusal;
   - with `includeFailing`, a test of it did not pass last time.
5. **Widening.** Always the whole test codeunit, whatever the isolation: test codeunit globals live across its tests
   in BC, and one rule is easier to trust than v2's three. With isolation disabled, the whole app.
6. **Full run** (`forcedFull: true` and a `reason`, the reasons named like v2's):
   - no baseline, an unreadable one, another schema, or one written by another runtime build;
   - `app.json`, defines, `--country`, `--bc-version`, isolation or any run option in the key changed;
   - a dependency app changed (see decision 7);
   - an object was added or removed, except a new test codeunit (it just runs);
   - an install codeunit or an "Extension Triggers" subscriber changed (the install baseline, ADR-046);
   - a changed object of a kind the trace does not cover: interface, permission set, query, xmlport, profile,
     report layout, control add-in, entitlement, and anything else not listed in decision 4;
   - a test reached a `DotNet` value or a .NET static in the baseline run (state the trace does not key);
   - the compile failed (nothing runs; the last baseline is kept).
7. **Environment.** A changed dependency set, BC version or dependency app always forces a full run. AL Runner does
   not diff dependency apps object by object as v2 does: a full run with Microsoft's apps takes seconds, and the diff
   is the part of v2 that can miss a change. So `--strict-environment` and `strictEnvironment` are accepted and change
   nothing; the help and the migration guide say that the strict behavior is always on. See open question 1.
8. **With `--tdd`.** Allowed. Stubs (ADR-051) live in the workspace IR, so a change that adds the real procedure
   changes the objects that called the stub. On top of decision 4, every codeunit with a stubbed test runs on every
   cycle: those tests are the ones being written.
9. **Skipped codeunits** keep their last outcome in the baseline. They are not streamed as `test` lines and are not
   counted as passed or failed in this request. They are counted in `selection`.
10. **Baseline file.** `<cache>/affected/<key>.json.gz`, `format: 1`. The key hashes the canonical source paths, the
    runtime build and the run options that change outcomes (the same set ADR-043 hashes). It holds: per object the
    key and IR hash; per test codeunit its trace keys and coverage objects (delta coded, as the ADR-043 index); per
    test its last outcome and message. It is written atomically after each run, full or partial (a partial run
    updates the codeunits it ran). Watch and serve share it, so the first cycle of a new watch is already narrowed.
    `--no-cache` reads and writes nothing (every request is then a full run until the warm slot has a baseline in
    memory).
11. **Output as v2.**
    - Serve summary: `selection: {mode: "affected", ran, skipped, skippedFailing, changedObjects[], forcedFull,
      reason}`. `ran` and `skipped` count tests. `changedObjects` lists `Kind id Name`, at most 50, with a
      `changedObjectsTruncated` count past that. `environmentDrift` is never sent (decision 7).
    - Watch: v2's lines (`[watch] affected: ran X of N   skipped-unaffected A   skipped-failing B`, `changed: ...` or
      `full run: <reason>`, `not re-run, still failing from an earlier cycle: ...`) and the failed count
      `failed 0 (+1 still failing, not re-run)`.
12. **The safety check.** A differential test is the oracle of this feature: for every corpus app and a set of
    edits (one per routine: change a literal, add a statement, move a line; one per object kind), the affected
    selection plus the kept outcomes of the skipped codeunits must equal a full run, test by test. Any difference is
    a selection miss and fails the test. It also runs on one Microsoft test app with the Base Application in CI.

## What is refused

- The flag combinations of decision 1 (exit 2 or `{"error"}`).
- `--affected` without `--watch` on the command line, as v2. A one-shot run has no earlier cycle in the process; the
  stored baseline would make it possible, but v2 does not offer it. See open question 2.
- Nothing is "refused" inside a run: every case the rule cannot vouch for is a full run with a reason, never a
  narrower one.

## Test plan (red, then green)

1. Serve: `affectedOnly` today answers an error (red). First request: `forcedFull`, reason "no baseline".
2. Change a routine body: only the codeunits whose tests entered it run; `changedObjects` names the object.
3. Change a table's trigger: the codeunits that used the table run.
4. Add a subscriber in an existing codeunit: the codeunits that raised the event run (also an event nobody
   subscribed to before).
5. An edit above an object in the same file shifts its lines: the codeunits that entered it run.
6. A comment-only edit inside a line: nothing runs, `ran: 0`.
7. Add an object: full run with the reason. Add a test codeunit: it runs, nothing else.
8. Change `app.json` and a dependency app: full run with the reason. `strictEnvironment` true and false give the
   same answer.
9. `includeFailing`: a failing test the change does not reach runs; without it it is in `skippedFailing`.
10. A test that timed out or panicked last time runs again.
11. A metadata table reader runs on any change.
12. Restart serve and watch: the first request is narrowed from the stored baseline; another runtime build forces
    a full run.
13. Watch lines and the failed count text.
14. With `--tdd`: stubbed tests run every cycle.
15. The differential test of decision 12 over the corpus (and, in CI, one Microsoft test app).

## Size

About 7 agent-days: 1 for the per-object IR hash and the change set; 2 for the selection rule on top of the ADR-043
trace and coverage; 1 for the baseline file and its key; 1 for serve, watch and output; 2 for the differential test
and its edit generator. It needs ADR-049 (`--watch`) for the CLI side; the serve side can be built first.

## Consequences

- No new seam: the trace and `RunOptions::narrow` exist; the selection is a new module.
- Recording is on for every test in an affected request; ADR-043 measured it as one branch per site when off and
  cheap when on.
- Full runs are more frequent than in v2 (added objects, dependency changes, uncovered kinds). That is the price of
  never reporting a stale pass, and a full run is fast.

## Open questions for the owner

1. Environment change: always a full run, `--strict-environment` accepted but always on (recommended), or v2's
   object-by-object diff of dependency apps (about 3 more days, and a place where a change can be missed).
2. `--affected` without `--watch` on the command line: refused as v2 (recommended), or allowed against the stored
   baseline for CI.
3. Widening: always the whole test codeunit (recommended) or v2's rule (one test when the codeunit has no globals and
   no `OnRun` under test isolation).
4. Added objects: a full run (recommended for 3.0) or a rule per kind that keeps the run narrow.

## Conformance

`al-testrun/src/affected.rs` unit tests (change set and rule on hand-made baselines),
`crates/alrunner-cli/tests/affected.rs` (serve and watch, test plan above) and the differential test of decision 12.
