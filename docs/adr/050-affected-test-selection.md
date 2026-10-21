# ADR-050: `--affected`, `--include-failing`, `--strict-environment`: run only the tests a change since the last run can affect

- Status: Accepted (owner, 2026-10-21; revised: works without watch mode, on the command line and in serve)
- Date: 2026-10-21
- Related: ADR-016 (owner decision 2026-10-21 that 3.0.0 needs v2's features), ADR-028 (serve), ADR-029 and ADR-033
  (incremental compile, symbolic IR with stable keys), ADR-043 (per-test trace and static reach for impact
  selection), ADR-049 (`--watch`, rejected), ADR-051 (`--tdd`)
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
- **The warm slot** (ADR-028) for serve.

So the recording and the narrowing exist. What is new is the change set, the selection rule and the stored result
of the last run.

The owner rejected watch mode for 3.0.0 (ADR-049) and decided on 2026-10-21 that `--affected` works without it: on
the command line and in serve, against the stored result of the last run, so CI can use it too. That is a change
from v2, where the command line needs `--watch`.

## Decision

1. **Surface.**
   - CLI: `alrunner test --affected [--include-failing] [--strict-environment] PATH...`. No `--watch` needed (owner,
     2026-10-21). `--include-failing` and `--strict-environment` without `--affected` exit 2, as v2.
     `--affected-store FILE` names the stored result explicitly (for CI); without it the file is in the cache
     (decision 10).
   - Serve: `affectedOnly`, `includeFailing`, `strictEnvironment` on `runTests`. On `execute` they earn a `warnings`
     entry, as v2.
   - Refused with `--affected` as v2 (exit 2, or `{"error"}` in serve): `--test`, `--codeunit`, `--exclude-tests`
     and the serve fields `test` and `excludeTests`. Not refused, unlike v2: `--tdd` (see decision 8).
2. **Where it lives.** The change set and the selection rule are pure code in `al-testrun/src/affected.rs` (new):
   the stored result in, a list of codeunits to run and a `Selection` report out. Reading and writing the stored result
   file is in `alrunner-cli` (ADR-006).
3. **The change set: compare per-object symbolic IR.** For each object of the workspace apps the stored result holds a
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
   - the stored result is missing or invalid (decision 10);
   - `app.json`, defines, `--country`, `--bc-version`, isolation or any run option in the key changed;
   - a dependency app changed (see decision 7);
   - an object was added or removed, except a new test codeunit (it just runs);
   - an install codeunit or an "Extension Triggers" subscriber changed (the install baseline, ADR-046);
   - a changed object of a kind the trace does not cover: interface, permission set, query, xmlport, profile,
     report layout, control add-in, entitlement, and anything else not listed in decision 4;
   - a test reached a `DotNet` value or a .NET static in the stored run (state the trace does not key);
   - the compile failed (nothing runs; the stored result is kept).
7. **Environment (owner, 2026-10-21).** A changed dependency set, BC version, country or dependency app always
   forces a full run. AL Runner does not diff dependency apps object by object as v2 does. So
   `--strict-environment` and `strictEnvironment` are accepted and change nothing; the help and the migration guide
   say that the strict behavior is always on.
8. **With `--tdd`.** Allowed. Stubs (ADR-051) live in the workspace IR, so a change that adds the real procedure
   changes the objects that called the stub. On top of decision 4, every codeunit with a stubbed test runs on every
   run: those tests are the ones being written.
9. **Skipped codeunits** keep their outcome from the stored result. They are not run, not streamed as serve `test`
   lines, and not counted as passed or failed in this run; they are counted in `selection`. A skipped test whose
   stored outcome is not a pass keeps the run red: the exit code is 1 when any test failed in this run or is
   "still failing, not re-run". So a skipped failure never turns a run green, also in CI.
10. **The stored result.** One file per workspace, written by every run with `--affected` (and every serve request
    with `affectedOnly`), read by the next one.
    - **Where.** `<cache>/affected/<key>.json.gz` by default; `--affected-store FILE` (CLI) or the daemon option of
      the same name (serve) puts it anywhere, so CI can save and restore it as a build artifact or cache entry.
    - **Key.** The ids and versions of the workspace apps (from `app.json`), never their paths, so two checkouts of
      one repository on two machines share a file.
    - **What it holds** (`format: 1`): the runtime build; the run options that change outcomes (the set ADR-043
      hashes: `--country`, `--bc-version`, company and test-data options, isolation, `--define`, environment options,
      disabled-test files, `--tdd`; a named backup, package or disabled-tests folder counts by its file name and
      its content, never its folder); the environment (each dependency app's id, version and content hash, and the BC
      build); per workspace object its key and symbolic IR hash, positions relative to the app root; per test
      codeunit its trace keys and the objects its tests entered (delta coded, as the ADR-043 index); per test its
      outcome and message.
    - **Updating.** A full run replaces the content. A narrowed run replaces the codeunits it ran and keeps the
      rest; the object hashes become the current ones. Written atomically (temporary file and rename); the last
      writer wins. Nothing is written when the compile failed or the run was cancelled.
    - **When it is invalid, a full run** with the reason as `reason`: no file; unreadable or another `format`;
      another runtime build; a different run option; a different environment (decision 7); different workspace
      app ids or versions; any `app.json` or define change. A file written on another machine with the same key is
      valid: nothing in it depends on the machine.
    - `--no-cache` without `--affected-store` reads and writes nothing, so every `--affected` run is full (serve then
      keeps the result in memory between requests).
11. **Output.**
    - Serve summary as v2: `selection: {mode: "affected", ran, skipped, skippedFailing, changedObjects[],
      forcedFull, reason}`. `ran` and `skipped` count tests. `changedObjects` lists `Kind id Name`, at most 50, with
      a `changedObjectsTruncated` count past that. `environmentDrift` is never sent (decision 7).
    - CLI `--output-json`: the same `selection` object in the summary (new; v2's command line had no `--affected`
      without watch).
    - CLI text: `affected: ran X of N   skipped-unaffected A   skipped-failing B`, then `changed: ...` or
      `full run: <reason>`, then `not re-run, still failing from an earlier run: ...`; the failed count reads
      `failed 0 (+1 still failing, not re-run)`, as v2's watch lines.
    - JUnit: a skipped test is a `<skipped>` element with the message `not affected by the change; stored outcome:
      <outcome>`, so a CI report never shows it as a pass of this run.
12. **The safety check.** A differential test is the oracle of this feature: for every corpus app and a set of
    edits (one per routine: change a literal, add a statement, move a line; one per object kind), the affected
    selection plus the kept outcomes of the skipped codeunits must equal a full run, test by test. Any difference is
    a selection miss and fails the test. It also runs on one Microsoft test app with the Base Application in CI.

## What is refused

- The flag combinations of decision 1 (exit 2 or `{"error"}`).
- Nothing is refused inside a run: every case the rule cannot vouch for is a full run with a reason, never a
  narrower one.

## Test plan (red, then green)

1. Serve: `affectedOnly` today answers an error (red). First request: `forcedFull`, reason "no stored result".
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
12. CLI: two runs in a row with an edit between; the second is narrowed. Restart serve: the first request is
    narrowed from the stored result. Another runtime build, option or environment forces a full run with the reason.
13. `--affected-store FILE`: a result written in one checkout is used by a second checkout of the same apps at
    another path.
14. A skipped test that failed in the stored result keeps the exit code at 1; JUnit shows it as skipped.
15. Text lines, `--output-json` `selection`, the failed count text.
16. With `--tdd`: stubbed tests run every time.
17. A cancelled run and a failed compile leave the stored result unchanged.
18. The differential test of decision 12 over the corpus (and, in CI, one Microsoft test app).

## Size

About 8 agent-days: 1 for the per-object IR hash and the change set; 2 for the selection rule on top of the ADR-043
trace and coverage; 1.5 for the stored result (key, validity, `--affected-store`, merge of narrowed runs); 1.5 for
the CLI and serve surfaces and output (text, JSON, JUnit, exit code); 2 for the differential test and its edit
generator. It needs no other ADR of this set.

## Consequences

- No new seam: the trace and `RunOptions::narrow` exist; the selection is a new module.
- Recording is on for every test in an affected request; ADR-043 measured it as one branch per site when off and
  cheap when on.
- Full runs are more frequent than in v2 (added objects, dependency changes, uncovered kinds). That is the price of
  never reporting a stale pass, and a full run is fast.

## Owner decisions (2026-10-21)

1. Works without watch mode, on the command line and in serve, against the stored result of the last run (also in
   CI).
2. Any dependency or environment change is a full run; `--strict-environment` is always on.
3. Widening: always the whole test codeunit.
4. Added objects: a full run (a new test codeunit just runs).

## Conformance

`al-testrun/src/affected.rs` unit tests (change set and rule on hand-made stored results),
`crates/alrunner-cli/tests/affected.rs` (CLI and serve, test plan above) and the differential test of decision 12.
The differential test is `crates/al-test-cli/tests/affected_differential.rs`: a deterministic sample of routine edits
(an `Error` call at the start of the body, a changed integer literal, a statement that changes nothing when it runs, a
blank line above the object; `AL_AFFECTED_DIFF_EDITS`, default 8) and a blank line above the first object of each
kind, over the corpus app without dependency apps, in CI's release tests. Per edit it runs the narrowed run and the
full run, and compares the selected codeunits with the narrowed run and the skipped ones with their stored outcomes.
Tests whose outcome differs between two runs of the unedited app are not compared and are counted. A small app test
checks that the harness reports a miss for a rule that runs nothing. The Microsoft test app with the Base Application
is not part of it yet.
