# ADR-066: Run the test runner codeunit's own AL

- Status: Accepted (owner, 2026-11-05); amendment 1 (P7, SingleInstance across test codeunits) Proposed (2026-11-06)
- Date: 2026-11-05

## Context

Al-runner is its own test runner. It reads one property of a runner codeunit (`TestIsolation`,
`al-testrun/src/isolation.rs` `runner_isolation`) and never runs the runner's AL. Around each test method it replays
four steps of Microsoft's Test Runner app by hand (`runtime/runner_reset.rs`, ADR-013 "The test runner's steps around
each test method"): codeunits 131006, 130301, 130302 and 132553, plus ClearLastError and ApplicationArea(''). It fakes
the runner's frames on the call stack (`run.rs` `runner_frames`).

The events of codeunit 130454 "Test Runner - Mgt" are never raised. Al-runner issue 6924 (waldo1001, a 70-app ISV
suite on BC 29 BE) shows the cost: the suite subscribes to `OnBeforeCodeunitRun` and `OnAfterCodeunitRun` to switch
stubs on and off. On al-runner the stub stays off, 214 tests fail with the StartSession error, and 593 later tests are
refused because a failed test left a manual binding. 807 tests in one suite.

Owner request (2026-11-05): run the real runner codeunit's AL, Microsoft's (130450 and siblings) and any custom
`Subtype = TestRunner` codeunit, so BC's behavior comes from the AL itself.

### What Microsoft's runner does (Test Runner app source, BC 29.0; 28.5 identical)

Paths are inside `TestFramework/TestRunner/Test Runner.Source.zip` of the artifact (read from the cache, not committed).

1. **Driver.** BcContainerHelper and AL-Go open page 130455 "Command Line Test Tool", set the suite name, the codeunit
   range or extension id, the procedure filter and the runner id, then call the action `RunNextTest` until it answers
   "All tests executed" (`CommandLineTestTool.Page.al:353-375`). Each call runs one test codeunit and returns its
   result as JSON (`TestSuiteMgt.Codeunit.al:97-121` `RunNextTest`, `:148-215` `TestResultsToJSON`).
2. **Rows.** The suite is a row of table "AL Test Suite" (runner id, code coverage settings) and one "Test Method Line"
   per codeunit (Line Type Codeunit) and per test method (Line Type Function, field `Run`). Discovery
   (`SelectTestMethodsByRange`, `GetTestMethods`) inserts the codeunit line and runs codeunit 130452 "Test Runner - Get
   Methods" on it (`TestSuiteMgt.Codeunit.al:609-621`). 130452 is a TestRunner that runs the test codeunit and answers
   `false` from OnBeforeTestRun for every method, after adding its line (`TestRunnerGetMethods.Codeunit.al:107-138`).
   It answers `true` for `OnRun`, so **discovery runs the test codeunit's OnRun trigger**.
3. **Run.** `RunSelectedTests` builds a line filter and calls `Codeunit.Run(ALTestSuite."Test Runner Id", Lines)`
   (`TestSuiteMgt.Codeunit.al:218-253`, `:646-649`). Runner 130450 (`TestRunnerIsolCodeunit.Codeunit.al:16-40`):
   OnRun calls `Test Runner - Mgt`.RunTests, which resets the results, commits, raises `OnRunTestSuite`, and per
   codeunit line raises `OnBeforeCodeunitRun`, calls `Codeunit.Run(test codeunit)`, raises `OnAfterCodeunitRun`, then
   `OnAfterRunTestSuite` (`TestRunnerMgt.Codeunit.al:25-58`).
4. **Per method.** The platform calls the runner's `OnBeforeTestRun` and `OnAfterTestRun`. 130450 forwards them to
   `PlatformBeforeTestRun` (`:83-116`: for `''`/`OnRun` stamps the codeunit line and answers true; else finds the
   method's line, answers false when there is none or `Run` is false, runs 131006, stamps the line, raises
   `OnBeforeTestMethodRun` with `var Skip`) and `PlatformAfterTestRun` (`:118-144`: 131006 again, writes Success or
   Failure with `GetLastErrorText` and call stack into the line and the codeunit line, `Commit()`, `ClearLastError()`,
   raises `OnAfterTestMethodRun`).
5. **Subscribers in the app itself.** 130453 "ALTestRunner Reset Environment" (SingleInstance): before each method
   ClearLastError, ApplicationArea(''), 130301, 130302 unless TestPermissions is Disabled, 132553; after each codeunit
   WorkDate back to its value at the start (`ALTestRunnerResetEnvironment.Codeunit.al:23-40`). The code coverage
   subscribers (off unless the suite asks) and the data-driven test input codeunits.
6. **Versions differ.** In BC 27.5 `OnBeforeTestMethodRun` has no `Skip` parameter; the rest of 130454 is the same.
   Running each version's own AL gives the difference for free.

### What the platform does for a Subtype = TestRunner codeunit, and what pins it

Corpus pin `92da8280`.

| Behavior | Pinned by |
|---|---|
| A test codeunit run from inside a test raises `TestCodeUnitNestedInvocation`, also through a custom runner | `al-language/events/platform-triggers/PTestRunnerTriggers.al:161-207` |
| A declared OnRun runs once before the first method, on the same instance, and is committed; 130454 answers true for `OnRun` | `al-language/codeunits/TestTestCodeunitOnRun.al:5-13` (cites the platform's `NavTestCodeunit.DoRunAsync`) |
| `TestIsolation` Codeunit, Function, Disabled and no property (= Disabled); Function returns to the state after OnRun | `al-language-isolation-probe` runners 61300, 61301, 61304, 61305 (Windows nightly, `-testRunnerCodeunitId`); `isolation.rs` cites the cases |
| Commit inside a test under each isolation | `al-language/codeunits/run-onrun/QuCommitIsolation.al:24-39` |
| ApplicationArea('') from `OnBeforeTestMethodRun` before each test | `al-language/session-system/TestSessionApplicationAreaTestBoundary.al:13` |
| 130301 and 130302 run per method, 130302 only when TestPermissions is not Disabled | `al-language-testlibraries/test-framework/testpermissions/BiTp*.al` |
| Runner frames 130450 and 130454 on the call stack; the Test Runner app among the caller modules; `RunTests` as the outermost scope in an error text | `BiTpDefault.al:29`, `al-language/collections/SsCallerModulesTestRunner.al:42`, `al-language/errors/CuCommitTests.al:563-564` |
| The runner holds its own "Test Method Line" row in an open transaction while a test runs | `al-language/types/variant/OSmallBuiltins.al:88-90` (comment, from the CopyCompany error) |
| A codeunit whose OnRun raises: no rows, run stays green | ADR-055 (issue 5859 measurement, not a corpus assertion yet; assumption `test-codeunit-onrun-rows`) |

**Not pinned (probes needed).** A runner's triggers cannot be reached from a test (the nesting error above), so each
probe needs a top-level custom runner in the isolation-probe app, read from the Windows nightly (ADR-007).

- **P1** OnBeforeTestRun answers false: is the body skipped, is OnAfterTestRun still called, with what IsSuccess.
- **P2** OnAfterTestRun: IsSuccess for pass, fail, and a failure caught by `asserterror`; is GetLastErrorText set.
- **P3** The OnRun trigger: FunctionName `OnRun` when declared; `''` or no call when not declared; OnAfterTestRun for it.
- **P4** Transaction scope of the triggers: do writes of OnBeforeTestRun, OnAfterTestRun and their subscribers survive
  the isolation rollback (Codeunit, Function)? What does their `Commit()` do inside the isolation? Every
  BcContainerHelper run implies the result rows survive, but no corpus test asserts it.
- **P5** What `Codeunit.Run(test codeunit)` returns or raises in the runner when a test fails or OnRun raises.
- **P6** Discovery through 130452 runs OnRun (point 2 above): are its writes kept for the real run?
- **P7** SingleInstance codeunits and committed rows across test codeunits of one runner session.
- **P8** The TestPermissions value passed when neither the method nor the codeunit sets one (today: Restrictive, from AL
  Runner v2, `runner_reset.rs` `test_permissions`).

## Decision

1. **The platform runs test codeunits for a runner.** `Codeunit.Run` of a `Subtype = Test` codeunit, when a
   `Subtype = TestRunner` codeunit is the root of the call and no test codeunit is running, becomes the platform's test
   codeunit run (a new routine in `al-testrun`, called through a `Ctx` hook from `al-vm`'s `Codeunit.Run`): OnRun with
   OnBeforeTestRun/OnAfterTestRun around it and its commit, then per method: the static refusal checks of today
   (`blocked.rs`, handlers, install reach), OnBeforeTestRun on the runner's instance (false skips the method),
   handlers bound, the method, OnAfterTestRun with IsSuccess and the last error left set; isolation from the runner's
   `TestIsolation` (`isolation.rs` keeps `TestIsolation` and `Checkpoint`). The outcome of each method goes to the
   run's result list directly, with refusals, flags, timings, coverage and capture as today.
2. **Microsoft's runner is driven like BcContainerHelper drives it.** After the install baseline, al-runner writes the
   "AL Test Suite" row and the "Test Method Line" rows for the selection natively, in the shape `AddTestMethod` writes
   them, into the baseline (so every fork has them, O(1)). Each unit (one test codeunit) then calls Microsoft's own
   `Test Suite Mgt.`.RunSelectedTests with that codeunit's lines, as `RunNextTest` does. That runs 130450, 130454 and
   130453 as Microsoft wrote them. OnBeforeCodeunitRun and the other events reach every subscriber.
3. **Results.** Outcomes come from the platform hook (decision 1), not from parsing the rows: the rows cannot hold
   refusals or flags. A differential test checks that the rows agree (Success = pass, Failure = fail, untouched =
   not run). A method the runner skips (OnBeforeTestRun false, or `OnBeforeTestMethodRun` sets `Skip`) is ADR-059's
   `Skipped`. ADR-055's reporting rule for OnRun stays.
4. **Selection maps onto the rows.** `--codeunit`, `--area`, `--file`, impact and `--affected` selection pick which
   codeunit lines exist. `--test` and `--run-selected-only` keep all method lines of a selected codeunit and set
   `Run = false` on the others, as `DisableTestMethod` and the procedure filter of page 130455 do: the runner then
   answers false for them, as on BC. Parallel threads, carry chains (Disabled isolation, OnRun carry) and the distrust
   rules (`written.rs`, ADR-064) stay per unit; the runner's routines join the reach facts of each test. A test whose own code
   reads "Test Method Line" or "AL Test Suite" rows of another codeunit is refused when it reads them (store watch):
   on BC those rows hold the earlier codeunits' results, which a fork does not have.
5. **Choosing the runner.** New flag `--test-runner <id|builtin>` (serve: `testRunner`).
   - Default: 130450 when the Test Runner app is loaded (ADR-013 adds it to every run), else `builtin` (today's path).
   - `--isolation Codeunit` selects 130450, `--isolation Disabled` selects 130451; `Function` has no Microsoft runner
     and stays the built-in override.
   - A custom runner with `TableNo = "Test Method Line"` (Microsoft's shape, like the isolation-probe runners) is driven
     as in decision 2: parallel units, every selection flag.
   - Any other custom runner decides in its OnRun which codeunits run. Al-runner runs its OnRun once, in one session,
     sequentially, exactly as declared, and reports what the platform hook saw. Selection flags are refused with such a
     runner (a clear message); `--affected` falls back to a full run with a reason (ADR-050).
6. **Corpus harness.** `al-test-cli` keeps the built-in path at first. The hub path (BC 28+, no handlers, ADR-055) stays
   built-in for good: BC runs no AL runner there. The BC 27 leg, the codeunits with handlers on 28+, and the
   isolation-probe app move to the AL runner in a later step, after a zero-difference run.

### Replaced or deleted

- `runtime/runner_reset.rs` `RunnerReset::before/after`: deleted on the AL path; 131006, 130301, 130302, 132553 now run
  from 130454 and 130453. `library_seed` stays (stability and `--seed-library`), hooked after 130301 returns.
- `al-sema` `check/demand.rs` `TEST_RUNNER_CODEUNITS` roots: replaced by the runner's roots (its OnRun, OnBeforeTestRun,
  OnAfterTestRun, `Test Suite Mgt.`.RunSelectedTests).
- `run.rs` `runner_frames` and `Session::set_runner_frames`: the real frames appear by themselves.
- `session.start_test_method`'s ApplicationArea and ClearLastError, and the WorkDate reset per codeunit: from 130453.
- `isolation.rs` `runner_isolation`: read when the runner's `Codeunit.Run` of a test codeunit starts, not before the run.
- ADR-013 sections "The test runner's steps around each test method" and the "Unchanged" note of "The test
  environment's apps": amended to point here. ADR-055's context line "Al-runner does not run AL test runner
  codeunits" is amended; its decision stands.
- **Stays platform (Rust):** the test codeunit run of decision 1, isolation rollback, TestPermissions value, handler
  binding and unhandled UI, TransactionModel, the nesting error, time limits, panics, refusals and the assumption
  registry, the hub path, and the built-in path when no runner app is loaded.

## Consequences

- **Costs per test.** The runner's AL adds per method about 30 AL statements and 8 store operations (2 FindFirst, 2
  IsEmpty, 2 Modify, 1 Commit, the AllObj check for 131006 that runs today too) and two event raises. A failure writes
  two blobs. Once per run: the suite rows (one Insert per line). Nothing is measured yet; PR 3 measures it with
  `--timings` and the perf budget tests on the corpus (BC 27) and one Microsoft bucket before the default changes.
- **Work ratchet.** `dependency_routines_lowered` and `objects_lowered` grow by the reached Test Runner routines (the
  app is about 6,900 lines; the reached part is far less). The PR that switches the default runs `--update-work` and
  commits `metrics/<major>/work.json`. `tests_run` does not change.
- **Perf budgets.** The Microsoft-run budget (83.2 G plus per KiB) may need the owner if the measured cost does not fit.
  Never raised without the owner.
- **Gains.** Issue 6924's 807 tests. Every partner pattern on 130454's events. Version differences of the runner for
  free. The four hand-written steps and the fake frames go away. The isolation-probe runners run as written.

### Migration (small PRs, each red/green)

1. Platform test codeunit run (decision 1) behind the `Ctx` hook; red tests: a tiny runner with triggers in an
   `al-testrun` unit test, and the isolation-probe runners 61300-61305 against their oracle. No user change.
2. Native suite rows (decision 2) and the selection mapping (decision 4); unit tests over the rows.
3. `--test-runner` with opt-in `130450`; differential tool `tools/runner_vs_builtin.sh` (corpus BC 27 leg, two
   Microsoft buckets, NP Retail): zero differences except tests that subscribe to 130454's events. Measure cost.
4. Default switch; delete the replaced code above; `--update-work`; ADR-013 and ADR-055 amendments; README row.
5. Custom runners of the second kind (decision 5); docs agent for the manual and CLI pages.
6. Harness legs (decision 6).

Probes P1-P8 go to the corpus the same day as PR 1, all at once. Each unprobed platform step registers an assumption
in `al-builtins/src/assumptions.rs` (hard rule 3 exception: Microsoft's CI runs every test through 130450, so its
tests need these steps) and a row in `docs/probes-needed.md`. When a probe lands, it wins.

### Risks

- **All-or-nothing.** Every test now passes through Microsoft's runner AL. One refused builtin or unprobed step in
  130450, 130454, 130453 or `Test Suite Mgt.` refuses every test of a run. Mitigation: the built-in path stays as
  fallback and differential oracle until PR 4, and PR 3 lists every builtin the runner reaches.
- **Transaction scope (P4).** If the trigger writes are scoped wrong, either the result rows vanish with the isolation
  rollback or a test sees a runner write it should not. Only rows read by tests can show it; refuse those reads until
  P4 lands.
- **One session per codeunit (P7).** BC runs all codeunits of a `RunNextTest` loop in one client session. SingleInstance
  state a subscriber leaves set after `OnAfterCodeunitRun` would reach the next codeunit on BC but not on a fork. This
  is today's assumption too; the AL runner makes it more visible.
- **Cost** (above), unknown until PR 3.

## Open questions for the owner

1. **Build before the probes land?** Recommendation: yes. PRs 1-3 under registered assumptions (rule 3 exception),
   probes P1-P8 filed the same day. Waiting blocks issue 6924 for one nightly cycle per probe round.
2. **Suite rows: native or through 130452 discovery?** Discovery runs each test codeunit's OnRun a second time (P6).
   Recommendation: native rows first; switch to discovery only if P6 shows the extra OnRun is observable.
3. **Default runner.** Recommendation: opt-in for one edge build, then default 130450 once the differential shows zero
   unexplained differences.
4. **Custom runner with its own OnRun plus selection flags.** Recommendation: refuse the flags with a message; running
   everything and filtering the report would hide that the runner ran other code.
5. **`--isolation Function`.** No Microsoft runner has it. Recommendation: keep it as the built-in override and say so
   in the help text.
6. **ADR-055.** Recommendation: keep its decision, amend only its context line.

## Conformance

- PR 1: `al-testrun` unit tests for a runner's triggers (skip, IsSuccess, OnRun) under the assumption ids; the
  isolation-probe runners' oracle rows.
- PR 3: `tools/runner_vs_builtin.sh` in CI on push to main.
- `crates/alrunner-cli/tests/cli/`: a test app subscribing to `OnBeforeCodeunitRun` that enables a stub (the issue 6924
  shape) passes with the default runner.

## Owner decisions (2026-11-05)

1. **Build before the probes land:** yes. PRs 1-3 run unprobed steps under registered assumptions (hard rule 3
   exception); probes P1-P8 go to the corpus the same day. When a probe lands, it wins.
2. **Suite rows:** native rows (orchestrator's call, following the recommendation); discovery through 130452 only if P6
   shows the extra OnRun is observable.
3. **Default runner: 130450 is the default right away** (owner), not opt-in first. The built-in path stays reachable
   with `--test-runner builtin` and stays the differential oracle (`tools/runner_vs_builtin.sh`) until PR 4 deletes
   the replaced code; PR 4 still needs zero unexplained differences.
4. **Custom runner that picks its own codeunits, with selection flags:** refuse the flags with a clear message.
5. **`--isolation Function`:** stays the built-in override (orchestrator's call, following the recommendation).
6. **ADR-055:** decision kept, context line amended (orchestrator's call, following the recommendation).


## Amendment 1 (Accepted, 2026-11-06): SingleInstance state from one test codeunit to the next (P7)

- Status: Accepted by the owner on 2026-11-06: option A below, with a serial check mode that CI runs (see "Decision").

### What BC does (P7 pinned)

Corpus PR 1161 (corpus master `d05042df`; not yet in this repository's pin): within one test runner run, the globals
of a SingleInstance codeunit that test codeunit A set, in a test and in A's OnRun, are still there in the next test
codeunit B; table rows A wrote, committed or not, are rolled back per codeunit; inside one codeunit nothing is rolled
back between tests. `al-language/codeunits/TestCuP7ASetter.al:24-58`, `TestCuP7BReader.al:24-37`, `TestCuP7State.al`;
identical on BC 27.5, 28.5 and 29.0. This covers every SingleInstance codeunit of the session, those of Microsoft's
apps too (the Test Runner's 130453, `Library - Random` 130440, `Reset State Before Test Run` 130301, `Backup
Management`, System Application's `Server Setting`, Base Application's `Change Log Management`, ...).

### What al-runner does today

Each test codeunit runs on a new `Session` (`run.rs` `run_codeunit_inner`); the install `Baseline` has no single
instances. B starts with fresh globals. This is issue 6924 (waldo: a SingleInstance codeunit holding a price set by an
earlier test codeunit).

### What was tried on branch `rt-p7` (built, not merged)

1. al-vm notes, per session, the first use of each global of a single instance: read first (with the value then) or
   written first (`single_carry.rs`; hooks next to `alias_access` in `read`, `write` and `Op::LoadGlobal`). Additive.
2. A walk after all codeunits ran (`single_walk.rs`) goes through them in run order per BC session (suite, hub leg)
   and keeps the serial state. A run counts when every global it read first held the serial value; otherwise the
   codeunit runs again from the serial state. Exact by construction for what it can carry, and the parallel runs and
   runs ahead stay (a codeunit that writes before it reads, as 130301 does with `Library - Random`, never runs again).
3. Carried across sessions: simple values, codeunit values (with their globals), .NET enums and the .NET objects the
   runtime serves itself. Anything else is refused where a later codeunit reads it first.

Measured (Tests-Bank, BC 29, seed 12345, 679 tests): main 568 pass; the branch 16 pass, 663 not run.
- 123 tests: `Backup Management` (Microsoft's test library) holds a **temporary record** (`BackupStorage`) that every
  codeunit fills and the next one reads, through the runner's row writing.
- 540 tests: under runner 130450 every codeunit is a carry-chain member (its OnRun rows); a member run again for
  SingleInstance state must leave the same rows as its first run, and the comparison included the runner's clock
  columns (fixable: compare with `fingerprint_apart(suite_rows::tables)`).
- The small P7 app (the three corpus files) passes with `--test-runner builtin`; under 130450 B is refused for
  `Change Log Management.ChangeLogSetup` (a **record**). Earlier rounds hit `Server Setting.ServerSettingImpl` (a
  codeunit value) and `Server Setting Impl.ALConfigSettings` (a runtime .NET object); both are carried now.

So carrying only what is cheap to carry refuses nearly every Microsoft test after the first codeunit of a bucket.
Records, temporary records (with their rows) and lists must be carried too.

### The decision to take (one of two)

**A. Carry handles across sessions (keeps one session per codeunit and the parallel runs).** Extend the carry form of
`single_carry.rs` to records (field values, filters, key, marks, load fields; the session-relative parts:
`row_origin` versions and commit counts relative to the session's counters, `globals` (the table instance),
`run_page_link`), temporary records with their private table's rows (`Session::temps`), lists and dictionaries. A
.NET object of the session's .NET process stays refused (no form outside its process). Seams: additive (new public
al-vm functions; `Instance::single`). Risk: every session-relative field of `RecordState` needs a pinned meaning in a
new session; a missed one is a silent difference (mitigation: a round-trip test per field, and a debug check that
carrying and not carrying give the same outcome on the corpus).

**B. One session per BC session (serial).** Run the codeunits of one suite (and hub leg) on one `Session`, putting
the install baseline's rows back between codeunits but keeping the single instances and their temporary tables, as BC
does. Exact without a codec. Changes this ADR's consequence "One session per codeunit (P7)" and the parallel model of
`corpus.rs` and `chain_walk.rs`: a suite runs on one thread (runs ahead with predicted single-instance state are not
possible: handles cannot leave the thread). Cost: a Microsoft bucket's wall time becomes about its CPU time.

Recommendation: **A**, with the validation walk of the branch (it keeps the parallel runs where the serial state does
not differ, and it already handles selection: with `--codeunit`/`--test` the state comes only from the codeunits that
ran before in that run, as on BC). B as the fallback if a record field turns out not to have a meaning outside its
session.

Also to decide: a SingleInstance codeunit still bound to manual subscribers at the end of a codeunit
(`BindSubscription` without `UnbindSubscription`): whether BC keeps the binding for the next codeunit is not pinned
(probe wanted). The branch treats it as "may have changed anything" for later failures.

### Decision (owner, 2026-11-06)

**A.** One session per test codeunit and the parallel runs stay. The single instances' globals are carried to the
next session in run order, records, temporary records with their rows and lists included (`al-vm/src/single_codec.rs`).
The walk of the branch checks every run against the serial state and runs a codeunit again when it read another
value (`al-testrun/src/runtime/single_walk.rs`). A value that is not carried never counts as equal to anything, so a
run that read one is never taken as the serial run.

What each session-relative part of a record means in the next session (each with a test in
`crates/al-testrun/tests/suite/single_instance_carry.rs` or `single_codec.rs`):

| Part of `RecordState` | Meaning in the next session |
|---|---|
| `temp_id` | A new private table with the same rows (BC rolls back no temporary table). Refused: a table two carried handles share, a table with an AutoIncrement field that has rows or whose counter the session moved (the counter is the session's), a Code key stored as a number. |
| Row version (SystemRowVersion in the buffer, `row_origin.version`, `failed_insert`, `last_modify`) of a table that is not temporary | Kept when at or below the committed row version at the codeunit's start: every session of the run starts from the same committed rows. A higher one names a row BC rolled back with the earlier codeunit, while the next session hands out the same numbers again. |
| A buffer of such a rolled-back row (a higher row version, or `row_origin` with `own_write`) | BC rolls back rows, not variables: the fields are carried; origin, versions and the IsDirty base are dropped and the record is marked (`records::ROLLED_BACK_ROW`). Field reads and writes, TestField, FieldCaption, FieldName and the view methods run; a Get or Find that finds a row fills it again and drops the mark; every other method is refused. A partly loaded buffer is refused. |
| `row_origin.commits` | `RowOrigin::EARLIER_SESSION`: the origin's transaction ended with the earlier codeunit; no Commit-count rule of the stale-buffer code holds for it. |
| `company` | Carried (every session has the same companies at its start). |
| `crm`, `run_page_link` | Refused (the CRM connection and the page are the session's). |
| `globals` (table and `xRec` instances) | Carried as codeunit instances. |
| Fields, filters, filter group, marks, key, ad hoc key, order, position, auto-calc fields, load state, IsDirty snapshots, flags | Values of the variable: carried as they are. |

Also carried: dictionaries, arrays, codeunit values (a SingleInstance codeunit's value stays its one instance), unopened
and open RecordRefs, RecordIds, TextBuilders. The globals of one single instance are carried as one group: a handle two of them share
(one list in two globals, a JsonToken inside another global's JSON tree as in the Test Runner's "Test Output") is
written once and named at its other places, so the copies share it as the originals do. When any global of a group
changes, the whole group is handed on, and when it shares handles, what its unread globals held at the start counts
as read (the walk checks it). A handle two single instances share, a record two places hold (a FieldRef into a
carried RecordRef's buffer) and a temporary table two records share are refused. A .NET object of the
session's .NET process, streams and the other library values are refused where a later codeunit reads them first.

**Predicted starts (speed).** The carry chain's walk (`chain_walk.rs`) goes through its members in run order, so it
keeps the SingleInstance state of each BC session as it goes and runs a member on its own thread from that state. The
helper threads run members ahead from a predicted state: the walk's state with what the members between wrote in their
last runs applied (`chain_runs.rs` `Runs::guess_singles`). The walk takes a run ahead only when every global it read
first held the walk's value (`single_walk::fits`; a global the walk's state does not hold must not have been given to
the run either). A helper does not start a member while a member before it runs on the walk's thread (its change is
not known), and the walk waits for an idle helper instead of running the next member itself. `single_walk.rs` still
checks every run against the serial state, so a wrong prediction costs time, never an outcome.

**Chain members run again.** A chain member that runs again from the serial state and changes a subset of the
tables its first run changed (it skips a setup, as an `isInitialized` flag makes Microsoft's tests do) relays its rows:
every later codeunit that reads one of them read one of the first run's tables too and runs again from the new rows.
A table only the run again changed stays refused.

Temporary rows keep their row version numbers. A temporary record's row version is no database row version
(`stale_verdict` treats a temporary record as fresh, and a temporary record of a table with SQLTimestamp never stamps,
record/temporary/QrTempTimestamp.al); what other numbers BC gives temporary rows is not pinned, within one session as
before this amendment.

**Serial check (owner requirement).** `AL_SINGLE_SERIAL_CHECK=1` (or `RunOptions::single_serial_check`): after the
run, the codeunits of each BC session (a suite, and a TestRunnerHub leg) in which a codeunit read state an earlier one
left run once more, one after the other on one thread, each session taking over the very single instances and
temporary tables of the one before (`Session::take_single_instances` / `adopt_single_instances`), nothing copied.
Every test the run passed or failed must come out the same; a difference fails the run (al-test-cli exit 1, lines
"SingleInstance serial check: ..."). Tests the run refused are not compared. CI sets it on the normal corpus run of
every pull request (all three majors): only the sessions that depend on the carrying run again, so it costs the job
little.

**Bound subscriptions stay refused.** A single instance still bound to manual subscribers at the end of a codeunit
makes the later failures not trusted, as on the branch. Probe wanted: corpus codeunit A binds a SingleInstance
subscriber codeunit (`EventSubscriberInstance = Manual`) and does not unbind it; codeunit B raises the event and
asserts whether the subscriber ran, on all three majors.

**Bug fixed with it.** The check that a carry-chain member run again left the same rows compared the Test Runner's
rows, whose clock columns differ from run to run. It now compares them apart (`fingerprint_apart` of
`suite_rows::tables`) and accepts different runner rows only when the run again did not see them
(`suite_rows::take_unseen`).
