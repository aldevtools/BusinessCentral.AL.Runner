# ADR-066: Run the test runner codeunit's own AL

- Status: Accepted (owner, 2026-11-05); amendment 2 (P7, SingleInstance across test codeunits) Accepted (owner, 2026-11-07); amendment 3 (narrow the lane to real order dependence) Accepted (owner, 2026-11-07); amendment 4 (order-dependent codeunits run in parallel, serial run only after a failure, `--strict-order`) Accepted (owner, 2026-11-07); amendment 5 (each serial re-run is a session of its own, the sessions run in parallel; a test refused before it starts triggers no re-run) Accepted (owner, 2026-11-07); amendment 7 (a re-run only when a failed test could have seen the changed state; a replay cap only with `--max-replay N`, changed by the owner the same day) Accepted (owner, 2026-11-08); amendment 8 (the carry chain walk keeps the SingleInstance state; chain members are never re-run or flagged) Accepted (owner, 2026-11-08, variant A); amendment 9 (the number of test threads follows the memory budget, `--max-memory`) Accepted (owner, 2026-11-09, as a safety net near the limit; redesign in a follow-up)
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
| The runner's triggers run once for the test codeunit itself with FunctionName `''` (before its OnRun and first method, after its last), also without OnRun; the `''` call after has IsSuccess false when a method failed (P3) | `al-language-isolation-probe/src/RpReader.Codeunit.al:55-69` (runners 61330, 61331; Windows nightly, BC 27.5, 28.5, 29.0) |
| After a failed OnRun, BC 29.0 makes no `''` OnAfterTestRun call; 27.5 and 28.5 make one with IsSuccess false (P5; gate `NoCodeunitAfterTestRunAfterFailedOnRunFrom29`) | `RpReader.Codeunit.al:119-125, 195-202`, `RpReaderB.Codeunit.al:21-30, 41-48` |
| OnBeforeTestRun answering false skips the method body, and OnAfterTestRun is not called for it (P1) | `RpReader.Codeunit.al:20-31` |
| OnAfterTestRun: IsSuccess true for a pass and for an error caught by `asserterror`, false for a failure; GetLastErrorText holds the failure (P2) | `RpReader.Codeunit.al:33-53` |
| A declared OnRun gets both triggers with FunctionName `OnRun` (P3) | `RpReader.Codeunit.al:55-62` |
| Rows written by OnBeforeTestRun, OnAfterTestRun and a subscriber survive the isolation rollback under TestIsolation Codeunit and Function, with and without `Commit()`, also for a failing method; the method sees the row of OnBeforeTestRun; its own row is rolled back (P4) | `RpReader.Codeunit.al:71-107` |
| In the runner, `Ok := Codeunit.Run(test codeunit)` returns false when a method fails and when OnRun fails (the last error is then the OnRun error); the statement form raises nothing (P5, P5b) | `RpReader.Codeunit.al:109-125`, `RpReaderB.Codeunit.al:21-30` |
| TestPermissions passed to OnBeforeTestRun: Restrictive when neither the method nor the codeunit sets it; the codeunit property applies; a method attribute wins over it (P8, P8b) | `RpReader.Codeunit.al:174-191`, `RpReaderB.Codeunit.al:32-38` |

The rows from P1 to P8b replace the registered assumptions `test-runner-trigger-calls` and
`test-runner-trigger-transactions` (removed from the registry). What `Ok := Codeunit.Run(test codeunit)` returns when
every method passed is not measured: al-runner refuses that call.

**Probes.** A runner's triggers cannot be reached from a test (the nesting error above), so each probe needs a
top-level custom runner in the isolation-probe app, read from the Windows nightly (ADR-007). P1-P5 and P8 are pinned
(table above, corpus PR 1221); P6 and P7 are outside that change (P7: amendment 2). The questions as filed:

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



## Amendment 2 (Accepted, owner, 2026-11-07): SingleInstance state from one test codeunit to the next (P7), found and run in one serial lane

- Status: Accepted by the owner on 2026-11-07. Supersedes amendment 1 (carrying the state between sessions, in pull
  request 7247 on branch `rt-p7v2`, which is not merged and stays unmerged). Amendment 1 is superseded: it is not built.

### What BC does (P7 pinned)

Within one test runner run, the globals of a SingleInstance codeunit that test codeunit A left are still there in the
next test codeunit B. Table rows A wrote are rolled back per codeunit.
`al-language/codeunits/TestCuP7ASetter.al:24-58`, `TestCuP7BReader.al:24-37`; identical on BC 27.5, 28.5 and 29.0. This
covers the SingleInstance codeunits of Microsoft's apps too.

### What al-runner does today and why it does not copy state

Each test codeunit runs on a session of its own, in parallel, and starts with fresh globals. Amendment 1 copied the
globals from session to session, with a codec for records, lists and temporary tables. Measured on the BC 29 corpus run
it cost +22.7 % instructions, and most of the second runs were caused by lazy caches and set-and-forget flags, not by
values that differ.

### Decision

1. **No state is copied between sessions.** The parallel run stays as it is.
2. **Find the dependent codeunits.** A test codeunit depends on SingleInstance state when it reads a global before it
   writes it, an earlier test codeunit of the same BC session (a suite, and each leg of the TestRunnerHub) left that
   global with another value, and the codeunit does not end with that same value itself.
3. **Run them in one serial lane.** After the parallel part, the dependent codeunits and, until nothing is added, every
   earlier codeunit that wrote a global one of them read run again one after the other on one thread in BC's order.
   The single instances, their temporary tables and the manual bindings they hold move (are not copied) from codeunit to
   codeunit (`Session::take_single_instances`, `adopt_single_instances`). The dependent codeunits' outcomes replace the
   parallel ones. A writer that does not depend keeps its first outcome.
4. **Flag them.** Every test of a dependent codeunit is marked with the registered assumption
   `single-instance-order` (same family as the flagged passes of unprobed assumptions): "relies on SingleInstance state
   that test codeunit X left (order-dependent test; fix the test)". `--timings` names X for each codeunit. The stability
   mode (ADR-058) lists these tests in a list of its own.
5. **Chain members.** A dependent codeunit that is a member of the carry chain (it hands rows to later codeunits) runs
   in the lane like any other. Its outcome is the lane's; the later codeunits keep the rows of its first run, as the
   chain walk gave them. That the lane's run leaves the same rows is assumed (registered with the flag).

### Cost control

- **Watch only what is used.** The SingleInstance codeunits that have globals are watched in each BC session of two
  test codeunits or more; SingleInstance codeunits without globals are ignored. A session notes only the single
  instances it makes, so a codeunit that uses none pays one test per global access. An app with no SingleInstance
  codeunit that has globals, or a BC session of one test codeunit, watches nothing: no hook runs.
  A static restriction from the reach of each test codeunit (which codeunit can read what an earlier one can write) was
  built and measured: the walk of every test's reach through the Microsoft apps cost +3.3 % instructions on the BC 29
  corpus run, the notes of watching everything +0.3 %. So the reach walk is not used.
- **Lazy caches.** A codeunit that read a flag first (a lazy cache guard, a set-and-forget flag) and then filled the cache
  itself is not dependent when it ends with the very values the earlier codeunit left: one value comparison per global
  at the end of the run.
- **Notes.** A watched session notes, per global, the fingerprint at the first read and at the end. A value without a
  comparable form (a .NET object, a page, a temporary record) never equals anything, so it counts as a difference.

### Consequences and limits

- Whether a dependence is real is decided by values, so a reader of an incomparable value (a DotNet object another
  codeunit wrote) is run in the lane even when both hold the same thing. It costs time and a flag, not a wrong outcome.
- The decision uses the values the first runs left. A dependent writer's own first run may have left a value that
  rests on stale state (a counter it read first and raised); a later reader that saw that very value is then taken as
  independent. Such a reader sees the state it would see in the lane only by chance, and is run in the lane when it
  also needs a writer that is in it.
- State left by a refused or not-run test codeunit is not seen (it left no notes).
- The registered assumption covers what the P7 probe does not pin: whether the Execute permission check of a moved
  single instance restarts (assumed as `Y93WildScope.al:439` shows for one shape).

### Conformance

- `crates/al-testrun/tests/suite/single_instance_lane.rs`: a dependent pair passes and is flagged; a lazy cache with an
  equal value is neither flagged nor run again; a cache with another value is flagged; an app without a dependence has
  no lane.
- Corpus: `P7B_ReadsWhatThePreviousTestCodeunitLeft` and the other P7 cases pass flagged.

## Amendment 3 (Accepted, owner, 2026-11-07): narrow the SingleInstance lane to real order dependence

- Status: Accepted by the owner on 2026-11-07. Amends amendment 2. The owner rejected the further cut "re-run only the
  dependent codeunits that failed": every dependent codeunit still runs again in the lane.
- Supersedes pull request 7270 (branch `rt-lanerefuse`): its rule that a refused writer is no change is kept (R3); its
  "distrust" step, which turned a failure of a later codeunit into not run, is dropped.

### Why

An outside user's suite (al-runner issue 6924) put 716 of its 717 test codeunits into the lane, and the run took 2.8
times as long. Microsoft's Tests-ERM-Purchase on BC 29 went from 131 s to 205 s with 32 of 32 codeunits in the lane,
Tests-SCM from 245 s to 517 s with 75 of 76. The causes:

1. One cache pulled everything in. The lane took every earlier codeunit that changed any global a lane member read, until
   nothing was added. A cache that every codeunit fills (Sequence No. Mgt., Unit of Measure Management, the dimension
   and general ledger setup caches) linked every codeunit to every earlier one.
2. Writes were noted as reads. Only a plain `:=` of a global counted as a write. `GlobalRec := Passed`, `Clear`, `Get`,
   `Find`, `List.Add`, `Dictionary.Set` and a `var` argument went through the read path: the codeunit looked as if it
   read the earlier state.
3. A keyed cache never matched. A codeunit that refilled a cache with its own key ended with its own key, so the "same
   end value" rule of amendment 2 never applied.
4. State left by a test that was refused mid-run moved on. Tests-SCM codeunit 137035 is refused in the middle of a
   print; the row it leaves in a SingleInstance temporary table moved into 137038 in the lane, and 5 of its tests went
   from pass to fail ("Production Order does not exist").

### Decision

- **R1, access kind.** The first access of a global decides whether the codeunit read the state it was given. A write
  in place is a write: `:=` into a record, `Clear`, `Init`, `Get`, `GetBySystemId`, `Copy`, `Find`, `FindFirst`,
  `FindLast`, `FindSet` on a global record; `Add`, `AddRange`, `Insert`, `Remove`, `RemoveAt`, `RemoveRange`, `Set` on a
  global list or dictionary; `Insert`, `Delete`, `DeleteAll` on a global temporary record; a global passed by `var` to a
  procedure, and the write-back of a `var` parameter to the global it stands for. Only a read of the value (a field, a
  comparison, `Count`, `Contains`, `Get(key)` of a dictionary, `IsEmpty`) is a read. A builtin hands its `var`
  receiver back after the call; when it hands back the value the global holds (the same list, dictionary or record),
  that is no write.
- **R1b, a re-validated cache.** The notes also take the fingerprint right after the first write of a global that was
  read first (the refill). When that first write replaces the whole value without using it (`:=` into a record,
  `Clear`, `Init`, `Get`, `Copy`, a `Find`, or
  `G := x` where `x` is made without `G`, such as `WhenGotGLSetup := CurrentDateTime`; not `Log += x` or
  `Flag := not Flag`) and no other watched global was written between the read and it, the read was a
  guard and the write the refill. Such a global is no link when the refill left the value the earlier codeunit left (the
  same state either way, nothing assumed). It is no link either when the refill left another value; that is the
  registered assumption `single-instance-refill` (`Assumption::SingleInstanceRefill`): with the earlier value the guard
  would have refilled all the same or found the key it wants, and the cached row is what the store holds. Every test of
  such a codeunit is flagged and the codeunit keeps its parallel run. Neither applies when the codeunit also left
  another global unlike the earlier codeunit (a cache filled after its key): then it is dependent.
- **R2, only the last writer.** For each global that makes a codeunit dependent, only the last earlier codeunit that
  wrote it joins the lane. That writer's own writers join only when it is dependent itself. The fixpoint over every
  earlier writer is gone.
- **R3, a refused writer is no change.** What a codeunit with a test refused while it ran left is not BC's state (on BC
  the test runs to its end). It makes no later codeunit dependent; a later codeunit that reads it keeps its own run and
  outcome. In the lane such a codeunit hands no single instances on: the next codeunit starts without them.
- **R4, the lane is timed.** `LaneReport::took` and the `--timings` line give the lane's time and the number of
  codeunits flagged under R1b.

### Decided without owner review (2026-11-07)

Measured on Tests-ERM-Purchase (BC 29) while building R1 to R4, two refinements of the lazy-cache rule of amendment 2
were needed to reach real dependence only. Both are recorded in `docs/owner-review.md`.

1. **A refilled global is not a culprit.** A global the codeunit refilled after a guard is left out of the "did it end
   as the earlier codeunit left it" check: its end is the refill's own value. Without this a time stamp
   (`WhenGotGLSetup` of Get Shortcut Dimension Values) kept every reader of the dimension cache dependent.
2. **A flag guards a cache in its own codeunit.** When a flag read first ends as it was left, only the globals the
   codeunit wrote in the same SingleInstance codeunit as that flag are checked. What it wrote in another single instance
   (Application Area Cache filled with its own keys) is not tied to the flag. A flag that guards a cache in another
   codeunit is missed; that test keeps its parallel run unflagged.

What stays a dependence on purpose: Sequence No. Mgt. keeps the rest of an entry-number range it allocated
(`SeqNoBufferFrom`, `SeqNoBufferTo`, `PendingAllocation`) when a posting stopped with an error, and BC's next test
codeunit draws its entry numbers from that rest. A record cache (the general ledger setup in Dimension CaptionClass Mgmt
and Get Shortcut Dimension Values) that an earlier codeunit filled after it changed the setup in its rolled-back
transaction is what BC's next codeunit sees.

### Consequences and limits

- A run without real order dependence has no lane. Real dependence still runs serially and is a flagged pass.
- R1 takes a codeunit whose first access appends to a list or dictionary an earlier codeunit filled as independent;
  a test that then counts the entries sees only its own. Accepted by the owner with the rule.
- `Find` on a global record whose filters an earlier codeunit set is a write under R1: the filters it finds by are not
  compared.
- R1b rests on the assumption above; when a probe pins it, the probe wins.

### Conformance

- `crates/al-vm/tests/language/single_track_access.rs`: `:=` into a record, `Clear`, `Get`, `FindFirst`, `List.Add`,
  `Dictionary.Set` and a `var` argument as the first access are writes; a field read and `Count` stay reads; a guard
  directly followed by `Get` is guarded with the refill's fingerprint, and so is `if When = 0DT then When :=
  CurrentDateTime`; a write of another global between, an append, a toggle and `ContainsKey` then `Set` on a
  dictionary (the other entries stay: corpus `XhLog.al`, read by `BiTestHandlerCrossApp.al:87-103`) are not guarded.
- `crates/al-testrun/src/runtime/single_lane.rs` unit tests: the last writer only; a dependent writer brings its writer,
  an independent one does not; a refused writer is no change; a guard-then-refill with the same value is no link, with
  another value no link under the assumption, and with another global left otherwise a dependence.
- `crates/al-testrun/tests/suite/single_instance_lane.rs`: a keyed cache every codeunit checks and refills has no lane;
  a global record overwritten before it is read has no lane; the dependent pair of amendment 2 still runs in the lane
  and is flagged. `crates/al-testrun/tests/suite/runner_reset.rs`: an append to a SingleInstance log stays a dependence.

## Amendment 4 (Accepted, owner, 2026-11-07): order-dependent codeunits run in parallel; the serial run only after a failure

- Status: Accepted by the owner on 2026-11-07. Amends amendments 2 and 3: how a dependent codeunit is found (amendment 3)
  is unchanged; when it runs again serially changes.

### Principle (owner, 2026-11-07)

A test should give the same result whether its codeunit runs alone or inside the whole suite. A codeunit whose result
depends on SingleInstance state an earlier codeunit left is an order-dependence problem in the tests, which is what
`--stability` (ADR-058) is for. A parallel run with fresh SingleInstance state is a real BC outcome: the codeunit run
alone, or first.

### Decision

1. **Default: in parallel.** A codeunit that amendment 3 finds dependent runs in parallel like all others. There is no
   serial lane up front.
2. **Serial run only after a failure.** Only when a dependent codeunit has a failed test or a test not run in its
   parallel run does it run again serially in BC's order, with the writers it depends on (the chain of amendments 2
   and 3: for each global that makes it dependent the last writer, and that writer's own writers when it is dependent
   itself). The serial result is reported. A dependent writer that joins the lane only as a writer keeps its parallel
   run.
3. **Every test of a dependent codeunit is flagged.** When it passes, whether in parallel or after the serial run, it
   is a flagged pass, counted apart from clean passes, through the registered assumption `single-instance-order`
   (`Assumption::SingleInstanceOrder`). The id keeps its meaning (the test is order dependent); its text now says
   which run is reported.
4. **`--strict-order` forces amendment 3.** Every dependent codeunit runs again serially in BC's order and that run is
   reported (`RunOptions::strict_order`; `serve`: the `runTests` field `strictOrder`). Nothing else serializes the
   order: `AL_TEST_THREADS=1` runs one codeunit at a time, but each still starts with fresh SingleInstance state, so it
   does not reproduce BC's order.
5. **`--stability` lists these codeunits** (`orderDependent`) in both modes: the flag is set either way.
6. **`--timings`** keeps the lane line: the lane's time, how many dependent codeunits were found (`LaneReport::dependent`)
   and how many ran again because of a failure (`LaneReport::rerun`; under `--strict-order` all of them).

### Why

Amendment 3 ran every dependent codeunit again. On Microsoft's posting suites most dependence is real but harmless:
Sequence No. Mgt. keeps the rest of an entry-number range after a posting that stopped with an error, and the next
codeunit draws its entry numbers from that rest. Those codeunits pass with fresh state too. Running them all again
serially cost Tests-SCM (BC 29, 12 threads) 357 s against 245 s before the lane existed.

### Rejected alternative

Strict serial for every dependent codeunit by default (amendment 3 as merged). Rejected for its cost on posting suites
(Tests-SCM 357 s against 245 s, caused by the entry-number buffers of Sequence No. Mgt.) while the parallel result is a
real BC outcome. It stays available as `--strict-order`.

### Risk: a false pass

A test that passes alone but fails on BC in the full order (it needs another value than the fresh one, but the
earlier codeunit's value breaks it) is reported as a flagged pass by default. The flag and the `--stability` list say
so; `--strict-order` removes the risk. A test that fails alone and passes in BC's order still gets BC's result through
the serial run.

### Measured (BC 29, Microsoft's apps, 12 threads)

Release binary, BC 29.0.54011.55816, cores 0-2 and 5-11, warm compile cache. "Before the lane" is the binary of
7b1a05de7; amendment 3 is main at 5605cb8ba.

| Bucket | Before the lane | Amendment 3 | Amendment 4 (this default) | Dependent / ran again | Outcome changes against before the lane |
|---|---|---|---|---|---|
| Tests-ERM-Purchase | 131.7 s | 103.5 s | 92.0 s (lane 25.9 s) | 9 / 7 | 0 |
| Tests-SCM | 244.9 s | 357 s | 360.6 s (lane 121.6 s) | 31 / 26 | 0 |

Tests-SCM codeunits 137035 and 137038 alone keep their 128 passes.

The cut is small on Microsoft's suites. Of the 31 dependent codeunits of Tests-SCM, 13 have a failed test and 20 a
test not run (mostly refusals of features not built yet) already with fresh state, so 26 run again. Measured for
information with a local build that counts only failures (not committed): 13 ran again, lane 90.5 s, run 341.7 s.
In both runs every serial result equals the parallel one (0 outcome changes). Making the serial run cheaper on such
suites needs another rule; that is an open question for the owner.

### Conformance

- `crates/al-testrun/tests/suite/single_instance_lane.rs`: a dependent codeunit that passes in parallel is not run
  again and is flagged; one that fails in parallel (the reader of amendment 2, the corpus P7B shape) runs again
  serially and passes flagged; `--strict-order` runs every dependent codeunit again.
- `crates/al-testrun/src/runtime/single_lane.rs` unit test: the lane of the chosen dependent codeunits holds only their
  chain.
- `crates/alrunner-cli/tests/cli/onrun_stability.rs`: `--stability` lists the dependent test with and without
  `--strict-order`.
- Corpus: `P7B_ReadsWhatThePreviousTestCodeunitLeft` fails with fresh state, runs again and passes flagged.

## Amendment 5 (Accepted, owner, 2026-11-07): each serial re-run is a session of its own, and the sessions run in parallel

- Status: Accepted by the owner on 2026-11-07. Amends amendment 4: which codeunits are dependent (amendment 3) and
  which of them run again (amendment 4) are unchanged, except decision 2 below. How the serial runs are grouped and
  where they run changes. `--strict-order`, `--stability` (`orderDependent`) and the flagged-pass rule stay as amendment 4
  defined them.

### Decision

1. **One serial session per chain (owner).** A dependent codeunit that runs again gets a serial session of its own:
   its chain (amendment 4, decision 2: for each global that makes it dependent the last writer, and that writer's own
   writers when it is dependent itself; `Found::lane_for`) in BC's order, the single instances moved from codeunit to
   codeunit as before. The sessions run in parallel on the worker threads (as many as the parallel run uses), not one
   after the other on one thread. A codeunit no longer sees the state that codeunits outside its session left.
2. **A writer two chains share runs in each (orchestrator's call, within the owner's decision).** Each chosen
   dependent codeunit keeps a session of its own even when its chain shares a writer with another chain; the shared
   writer runs once per session (`Found::sessions`). So each codeunit sees exactly the state of its own chain, as
   decision 1 says. One exception saves runs without changing what a codeunit sees: a chosen codeunit that is in a later
   chosen codeunit's chain, where every codeunit of that chain up to it is in its own chain, is reported from that
   session and gets none of its own (it starts there with the very state its own session would give it).
   Rejected: merging overlapping chains into one session, so a shared writer runs once. Built first and measured on
   Tests-ERM-Purchase: the chains link through one dependent writer (134804 RED Test Unit for Purch Doc, itself
   dependent on 134398 through the entry-number buffers of Sequence No. Mgt.), so 7 re-runs merged into 2 sessions, one
   of them holding almost every codeunit: longest session 26.7 s, lane wall time 31.4 s, no faster than amendment 4's
   single lane (25.9 s). Merging also lets a codeunit see the state another chain's codeunits left, against decision 1.
3. **A test refused before it starts triggers no re-run (orchestrator's call, within the owner's rule of amendment 4
   "re-run only on failures").** Only a failed test (also a failure judged not trusted) or a test that started and did
   not run to its end (refused while it ran, stopped by the time limit) makes a dependent codeunit run again. A test the
   lowering refused, a diagnostic, or a refusal of the whole codeunit before it started is not caused by the state the
   codeunit started with, and the serial run would refuse it all the same (`Scratch::unfinished_after_start`,
   `needs_serial_run`).
4. **`--strict-order` keeps its meaning** (every dependent codeunit runs again serially in BC's order, with the writers
   it depends on, and that run is reported) and uses the same parallel sessions. This does not contradict the flag's
   documented meaning: each dependent codeunit still runs after its chain in BC's order. What changes under the flag is
   that state from codeunits outside the chain is not carried, as amendment 3 already limited the chain to the last
   writer.
5. **Carried starts as bytes, written when needed.** A codeunit that comes after a carry-chain member
   (RequiredTestIsolation = Disabled, ADR-055) may start from the baseline that member left, which holds `Rc`s and stays
   on the thread that made it. The main thread works out where each codeunit of a session starts (`Step`). The sessions
   run on worker threads, as many as the parallel run uses (`AL_TEST_THREADS`); the main thread writes a carried
   baseline as bytes only when a worker asks for it (`Starts`), and the worker reads it back over the engine's test
   data, as the chain walk's helpers do, and drops the bytes. A worker holds one carried start at a time, for one step.
   A session whose carried baseline cannot be written as bytes runs on the main thread from the baselines themselves,
   as do all sessions when there is one thread or one session.
   **Memory rule (al-runner issue 6924):** at most one carried start per worker thread is alive at once, as bytes or
   read back. As first merged, the main thread wrote every carried start all sessions needed before they started and
   kept them to the lane's end, and each worker kept every start it had read back: a partner suite with about 700
   codeunits in the carry chain and about 65,000 rows of test data ran out of memory. `--timings` prints how many
   starts were written and the most alive at once.
   Built first without this: on Tests-ERM-Purchase every codeunit is in the carry chain, so every session stayed on
   the main thread (6 sessions, longest 13.6 s, lane wall time 45.8 s).
   **Each carried start made once, as rows (al-runner issue 6924, 2026-11-08; an internal change, rule 6).** With the
   memory rule alone, a synthetic suite shaped like the partner's (737 test codeunits, 701 in the carry chain, 36 that
   fail alone, about 65,000 rows) took 527 s against 24 s for amendment 3's one lane: its 36 sessions asked for 6,737
   carried starts, the same carry step once per session, and the main thread wrote each whole (about 78 ms each, one
   after another) while the workers waited. Now (`lane_starts.rs`) the main thread makes each carry's start once, as a
   row delta (`al_vm::RowDelta`): the rows its baseline holds apart from the baseline of the carry before it (from the
   install baseline for the first one, or from an empty baseline in a run without one), found by comparing the
   persistent maps' shared entries by pointer and only the others by their bytes, with every other part of the baseline
   (counters, sequences, storage, marks, origin) as it is. A worker keeps the one start it built last and walks forward
   from it, or from its own copy of the install baseline, putting the deltas between on top; put on a copy of the
   baseline it was taken from, a delta gives the very baseline (`Baseline::row_delta_from`, `with_row_delta`). The main
   thread keeps the deltas for every session that asks, up to 256 MB (`DELTA_BUDGET`); past it it drops them and makes
   one again when asked, from the same baselines, so the bytes are the same. A carry whose delta cannot be made (a row
   that is not a plain value, other company copies) goes whole as before. Which codeunits run, in which session and
   from which state is unchanged; a start now also keeps which tables a DeleteAll emptied, as the one-thread path
   (`run_bound`) already did. The memory rule still holds: a worker holds one carried start at a time.
   `--timings` prints how many starts were made, how many went whole and the most bytes of deltas kept.
   Measured (12 threads, cores 0-2 and 5-11, release; the synthetic suite with and without an install baseline, which
   a run has only when a dependency app has install code; Microsoft's suites on BC 29.0.54011.55816): synthetic, no
   install baseline: 527 s before, 35 s now (amendment 3's one lane: 24 s), 1474 passes each; synthetic with an
   install baseline: 273 s before, 2 s now (amendment 3: 1 s); Tests-ERM-Purchase 90 s before, 88 s now; Tests-SCM
   296 s before, 301 s now. Outcomes equal except `FiveThousandSerialNos`, the CPU-time-limit case named above. Without
   an install baseline every session's codeunit that is no carry member runs the workspace's install code again
   before it reruns from the carried start; that, not the starts, is the rest of the 35 s.
6. **`--timings`**: the lane line adds the number of serial sessions, the time of the longest one and the lane's wall
   time (`LaneReport::chains`, `longest`, `took`).

### Why

Amendment 4 ran all re-runs of a run in one lane on one thread. On Tests-SCM (BC 29) that lane took 121.6 s of the
360.6 s run, with 26 codeunits running again, while the other 11 threads were idle. Most chains are short (a dependent
codeunit and the last codeunit that wrote what it read, most often through the entry-number buffers of Sequence No.
Mgt.), so they can run side by side. A test refused before it starts was one of the reasons a dependent codeunit ran again (20 of the
31 dependent codeunits of Tests-SCM have a test not run, mostly refusals of features not built yet); the serial run
cannot change such a refusal.

### Measured (BC 29, Microsoft's apps, 12 threads)

Release binary, BC 29.0.54011.55816, cores 0-2 and 5-11, warm compile cache, 12 threads. The machine was shared with
other agents (load average between 1.5 and 14 during these runs), so the totals carry noise of 10 to 30 %; the lane's
wall time is the number this amendment changes. "Before the lane" is the binary of 7b1a05de7 (amendment 4's table).
Run 1 is this change alone; run 2 is this change and then the binary amendment 4's table measured, in the same half hour.

| Bucket | Before the lane | Amendment 4 (its table / run 2) | Amendment 5 (run 1 / run 2) | Lane wall time, amendment 4 (run 2) / amendment 5 (run 1 / run 2) | Serial sessions (longest, run 1) | Codeunit runs in the lane, amendment 4 / 5 | Dependent / ran again | Outcome changes against before the lane |
|---|---|---|---|---|---|---|---|---|
| Tests-ERM-Purchase | 131.7 s | 92.0 s / 99.1 s | 85.9 s / 145.0 s | 25.8 s / 16.5 s / 17.7 s | 6 (16.3 s) | 12 / 17 | 9 / 7 | 0 / 0 |
| Tests-SCM | 244.9 s | 360.6 s / 418.3 s | 322.5 s / 313.5 s | 132.8 s / 64.5 s / 56.5 s | 20 (57.6 s) | 49 / 81 | 31 / 26 | 1 / 0, see below |

The 145.0 s of Purchase in run 2 is load: the load average went from 1.8 to 14 during that run, and its lane still
took 17.7 s.

Tests-SCM codeunits 137035 and 137038 alone (`--codeunit 137035 --codeunit 137038`) keep their 128 passes, 0 outcome
changes.

- The lane's wall time is now the time of its longest session. On Tests-SCM that is one long chain (57.6 s); the
  other 19 sessions finish within it. More codeunit runs (writers that two chains share run in each) cost CPU time on
  threads that are idle at that point.
- Decision 3 changed no count on these suites: each of their dependent codeunits with a test not run also has a failed
  test or a test refused while it ran.
- The one outcome change on Tests-SCM in run 1 is `FiveThousandSerialNos` of codeunit 137380 SCM Performance Tests,
  which is not in the lane: it took 20.2 s of CPU time (limit 20 s; also 20.5 s in an earlier run of this branch)
  against 17.9 s before, and passed in run 2. Run alone
  (`--codeunit 137380`) right after each other, it passes with both binaries (7b1a05de7: 17.1 s, this change: 17.7 s).
  The CPU time of that test sits close to the limit and grows with the machine's load; the change does not touch the
  parallel run it ran in.

### Risk

- A codeunit no longer sees state from codeunits outside its session. Where that state matters, amendment 3 says the
  codeunit reads it, so the codeunit that left it is its last writer and in its chain. A missed link (a flag that guards
  a cache in another codeunit, amendment 3) could make the serial result differ from BC's; it was already possible
  under amendment 4 for codeunits not in the lane.
- A dependent codeunit whose parallel run has only tests refused before they start and passes keeps its parallel run,
  flagged. Its refused tests stay not run either way.
- Parallel sessions use more memory at once, as the parallel run does: one session per worker thread.
- A writer two chains share runs once per session: more CPU time in total than one lane, spent on threads that are idle
  at that point of the run.

### Conformance

- `crates/al-testrun/tests/suite/single_instance_lane.rs`: two dependent codeunits with writers of their own run as two
  sessions (`two_dependents_with_their_own_writers_run_as_two_serial_sessions`); two that share a writer run as two
  sessions, the writer in each (`chains_that_share_a_writer_run_as_sessions_of_their_own`); a test refused before it starts
  does not make its codeunit run again (`a_test_refused_before_it_starts_does_not_trigger_a_serial_run`); a test refused
  while it runs does (`a_test_refused_while_it_runs_triggers_a_serial_run`); `--strict-order` still runs every dependent
  codeunit again.
- `crates/al-testrun/src/runtime/single_lane.rs` unit tests: every chain is a session of its own and a shared writer
  runs in each; a chain that is the start of a later chosen codeunit's chain runs inside it, and one that is not keeps
  its own session.
- `crates/al-testrun/tests/suite/single_instance_lane.rs`
  `carried_starts_are_written_when_a_session_needs_them_and_freed_after`: 24 sessions in a carry chain, each with a
  carried start of its own, never hold more starts at once than there are worker threads (issue 6924).
  `a_carried_start_that_sessions_share_is_made_once`: 24 sessions that each run the same two writers from the same
  carried starts make each start once (no more than there are carries) and none whole.
- `crates/al-vm/src/baseline/row_delta.rs` and `crates/al-data/src/memory/row_changes.rs` unit tests: a row delta put
  on a copy of its base gives the same fingerprint, bytes and origin (rows changed, gone and new, a table gone and one
  new, and from an empty baseline); rows two stores share are not compared by value.


## Amendment 6 (Accepted, owner, 2026-11-07): test-suite boundaries are Business Central session boundaries

- Status: Accepted by the owner on 2026-11-07 ("Yes, but treat those as minimum boundaries. If feasible the runner
  should be able to parallelize even further."). Amends amendments 2 to 5: which BC session a test codeunit belongs to
  can now come from a file. Within a session nothing changes.

### Why

Amendments 2 to 5 assume what Microsoft's own test runner does: all test codeunits of a run in one session, in order.
Not every pipeline runs tests that way. An outside user (al-runner issue 6924) runs his reference pipeline with ALOps
as one test-tool run per BC test suite: 73 pipeline steps, each an `ALOpsAppTest` task on test page 130455 that opens
its own client session, runs one suite and disconnects. SingleInstance state therefore never crosses a suite on BC.
al-runner treated the whole run as one session and linked codeunits across suites: codeunit 81697 (suite LCSP) was
made dependent on `ConfigLoaded`, which codeunit 81673 (suite DATACHECK) left. On BC that link cannot happen.

### Decision

1. **`--test-suites FILE`** (`RunOptions::test_suites`; `serve`: the `runTests` field `testSuites`, the file's path).
   The file lists suites, one per line: an optional name ended by the first `:`, then the suite's test codeunit ids in
   BC's run order, separated by spaces, tabs or commas (`DATACHECK: 81659 81660 81673`). The name may hold spaces
   (`suite  1: 93001 93002`, the shape issue 6924 posted); a line without a name is named `line N`. Blank lines and
   lines that start with `#` are skipped; a byte order mark and Windows line ends are accepted. The parser is
   `al_testrun::test_suites` (no OS access); `alrunner-cli/src/test_suites.rs` reads the file (ADR-006).
2. **Each suite is a BC session of its own for order dependence.** The streams of `single_lane.rs` are the suites of
   the file (per suite, app and hub leg), each in the file's order, not in id order. No SingleInstance link, serial
   re-run or writer chain crosses a suite. The watch (amendment 2, cost control) follows: a suite of one codeunit
   watches nothing.
3. **Minimum boundaries.** Within a suite amendments 3 to 5 apply unchanged: every codeunit runs in parallel with fresh
   state, a dependent codeunit is flagged (`single-instance-order`), and only one with a failed test or a test refused
   while it ran (or every one, under `--strict-order`) runs again in a serial session of its own chain, the sessions in
   parallel. The suites themselves add no serial step: they only remove links.
4. **A codeunit in no suite is a session of its own** (orchestrator's call). On a per-suite pipeline such a codeunit
   is not run by BC at all, so it has no BC order to copy and no earlier codeunit whose state it could see. Running it
   as if alone is the parallel run al-runner gives anyway, and it is never linked or flagged. Rejected: one default
   session for all unlisted codeunits, as today. It would invent links between codeunits that never share a session
   on BC.
5. **A codeunit listed in more than one suite** (issue 6924: suites 5 and 6, and 2 and 25, list the same codeunits;
   BC runs such a codeunit once per suite) runs once in the parallel run and is reported once (orchestrator's call).
   It is a member of each suite's session: it counts as a writer for the codeunits after it in every one of them (what
   it left is its parallel run's end state, the same each time). It is dependent when it is dependent in any suite it
   is listed in; the first such suite in the file's order gives its note and, when it runs again, its serial session.
   It is counted once in `--timings`. Rejected: reporting it once per suite, which would change the result format
   (one result per test) for a rare case whose suites are, in the case seen, identical.
6. **What the file may hold.** A token that is not a whole number from 1 to 4294967295, or a codeunit listed twice in
   one suite (a BC test suite holds a codeunit once), stops the run with exit code 2 and names the line. A listed id
   that is no test codeunit of the apps under test is a `warning:` on stderr per suite and is ignored: a pipeline's
   file may name codeunits of other apps or ones a selection leaves out. A suite without ids is accepted and holds
   nothing.
7. **Suites cut SingleInstance state only (owner, 2026-11-07).** The compile, the database load, the install code and
   the install baseline (ADR-013) are made once per run and shared by every suite and every session, exactly as
   without the option: a suite pays no compile, database load or install code of its own. This holds by construction:
   the suites only change how `single_lane.rs` groups the codeunits it already analyses after the parallel run; the
   parallel run and the serial sessions start from the run's one install baseline (a worker thread reads its copy from
   the snapshot written once per run, amendment 5). The work ratchet counts (`RunWork`) of a run with suites equal
   those of the same run without them.
8. **Without the option nothing changes**: the whole run is one session (Microsoft's default), as in amendments 2
   to 5.
9. **`--stability`** lists as `orderDependent` only codeunits that depend on a codeunit of their own suite (it reads
   the same flag).
10. **`--affected`**: the file's text is part of the environment hash, so another file is a full run.

### Consequences and limits

- The database carry chain of `RequiredTestIsolation = Disabled` (ADR-055) is not changed: on BC committed rows stay
  in the database across sessions, so suites do not cut it. It keeps the id order it had. Whether the order of the
  suites themselves matters for such rows on a per-suite pipeline is not modelled.
- A codeunit listed in two suites whose dependence differs per suite is reported with one result. In the shape seen
  (identical suites) both give the same.
- The file describes the pipeline; al-runner cannot check that it matches what BC ran. Issue 6924's reporter generates
  it from the same suite definitions the pipeline uses.

### Conformance

- `crates/al-testrun/src/test_suites.rs` unit tests: named and unnamed lines, a name with spaces and the order kept,
  comments, blank lines, byte order mark and Windows line ends, an empty name and an empty suite, a bad token and a
  codeunit twice in one suite as errors naming the line, a codeunit in several suites.
- `crates/al-testrun/tests/suite/single_instance_lane.rs`: a writer in one suite and a reader in another are not
  linked; a writer and a reader in one suite are still linked (the reader runs again and passes flagged); the order
  within a suite is the file's; a codeunit in no suite is a session of its own; a codeunit in two suites is reported
  and counted once, and is dependent through either suite. The tests without the option are unchanged. Suites share one compile and one install baseline:
  two readers of a row the install code wrote, in different suites, both see it, and `RunWork` and the number of
  passes equal those of the run without suites (`suites_share_one_compile_and_one_install_baseline`).
- `crates/alrunner-cli/src/test_suites.rs` and `serve.rs` unit tests: the option takes a path, `serve` refuses it as a
  server option and takes `testSuites` per request, an unknown id is a warning per suite, a missing file is an error.
- `crates/alrunner-cli/tests/cli/onrun_stability.rs`: `--stability` lists a reader as order dependent only when its
  writer is in its suite; a bad line exits 2.

## Amendment 7 (Accepted, owner, 2026-11-08): skip a serial re-run that cannot change the outcome

- Status: Accepted by the owner on 2026-11-08: option 1 plus option 2 below. Amends amendment 4, decision 2 (which
  dependent codeunits run again) and amendment 5, decision 1 (each re-run replays its whole chain). How a dependent
  codeunit is found (amendment 3), the flag on its tests, `--strict-order` and `--test-suites` are unchanged.

### Why

On a partner suite (NP Retail, 336 test codeunits, 5494 tests, BC 29, 12 threads) the serial re-runs are most of the
run, and they change no outcome. Measured on main 7c3c37006 with a local trace (not committed):

| | |
|---|---|
| Whole run | 1684 s (compile 13 s) |
| Serial re-runs (lane wall time) | 1354 s |
| Dependent codeunits / re-run because a test failed or did not finish | 308 / 26 |
| Codeunit runs in the lane | 3025 in 21 serial sessions (4 to 274 codeunits each, longest session 956 s) |
| Re-runs whose outcome differs from the parallel run | **0 of 26**: no test went from pass to fail, fail to pass or to another error |
| Tests whose result changed at all | 1: `GivenTwoRunsDifferingOnlyInDepartmentCount_...` of 85340 went from one not-run reason to another (refused in parallel; in the lane it hit the 20 s CPU limit under load) |

All 25 failures of the run fail the same way with fresh SingleInstance state and in BC's order (Assert texts about an
on-prem service tier, a missing Magento Setup, a missing VAT account). Tests-ERM-Purchase and Tests-SCM showed the
same in amendments 4 and 5: 0 outcome changes from the serial runs there either.

Why each re-run is so long: almost every NP codeunit reads SingleInstance state the codeunit before it left
(`_Images` of NPR Item Before Image, the entry-number buffer of Sequence No. Mgt.), so the chain of last writers
(amendment 3, R2) of a codeunit is nearly the whole prefix of the run before it. 26 re-runs replay 3025 codeunits.

What cannot be done within amendments 4 and 5 (checked):

- **Share the prefixes.** The 21 sessions have only 309 distinct prefix steps (against 3025 runs), so one walk with a
  copy of the single instances at each branch would run about 10 % of the codeunits. But after the first NP codeunit the
  single instances hold 21,024 RecordRef values, HttpClient, JsonObject, JsonArray, SecretText and internal .NET
  values. Copying them so that each branch sees exactly what a replay would give is a new seam on every library type,
  with the risk of a wrong outcome; not done.
- **Schedule the sessions better.** Taking the costliest session first (an internal change, on branch
  `rt-lane-reruns`, not merged) can at best bring the lane's wall time down to its longest session (956 s of 1354 s
  above). Measured once on NP with load average 7 to 14 from other work: lane 1675 s, longest session 1420 s, 0 outcome
  changes; the load hides any gain. It does not remove replays.

### Options for the owner

1. **Re-run only when a failed test could have seen the difference.** The watch (amendment 2) already notes, per
   codeunit, the globals read first. Note it per test instead: a dependent codeunit runs again only when a test that
   failed or did not finish, or a test before it in the same codeunit, read first a global whose value the chain left
   differently from the fresh value. Otherwise the parallel run stands (flagged, as today). This keeps the reason for the
   re-run (a test that may fail only because it ran alone, corpus P7B) and drops re-runs whose failures never touch
   the state. Cost: one note per test of a watched codeunit. Effect on NP not measured yet; it needs the per-test notes
   first.
2. **Cap the replay.** A chain longer than a limit (for example 50 codeunits, or an expected cost of 60 s from the
   parallel run times) is not replayed: the codeunit keeps its parallel run and is flagged with a text that says the
   serial re-run was skipped and why. `--strict-order` always replays. `--stability` lists such codeunits. Simple and
   predictable; on NP a limit of 50 codeunits would skip 18 of the 21 sessions.
3. **No re-run by default.** Every dependent codeunit keeps its parallel run, flagged; `--strict-order` gives the
   serial result. This follows amendment 4's principle (the parallel result is a real BC outcome: the codeunit run alone),
   and the measured re-runs changed nothing. It loses the serial result for a test that fails alone and passes in BC's
   order (corpus P7B) unless the user asks for `--strict-order`.
4. **Re-run only the failing test methods.** Rejected: the cost is the replay of the writers before the codeunit, not
   the codeunit itself.

Recommendation: option 1, with option 2 as the bound for chains that option 1 still selects. Both keep
`--strict-order` as the exact BC order, and both need a corpus case for the flag text before they are built.

### Decision (owner, 2026-11-08: option 1 plus option 2)

1. **Per-test rule (option 1).** A watched session numbers the test methods it begins (`Session::single_test_begins`,
   from 1; a read before the first test, in `OnRun`, is test 0) and notes for each global read first the test it was
   read in (`SingleOut::read_in`). The analysis notes, per dependent codeunit, the globals it read first that the last
   writers left with another value (`Found::changed`). A dependent codeunit with a failed test or a test that started
   and did not run to its end (amendment 5, decision 3) runs again only when such a test, or a test before it in the
   same codeunit, read one of those globals (`failure_saw_change`). Otherwise it keeps its parallel run, flagged as
   before (`LaneReport::kept` counts these). A failure the test loop did not number counts as the last test, so it
   always sees every read. Corpus P7B (`TestCuP7BReader.al:24-37`): the reader test reads what codeunit P7A left and
   fails with fresh state, so it still runs again and passes flagged.
2. **Cap (option 2), opt-in since the owner's change below.** With `--max-replay N` (`serve`: `maxReplay`), a
   codeunit that the per-test rule still selects is not replayed when its own chain (`Found::lane_for` of it alone)
   holds more than `N` test codeunits (`Found::within_limit`). Without the flag there is no cap. It keeps its parallel
   run. Its tests are flagged with the registered assumption `single-instance-rerun-skipped`
   (besides `single-instance-order`), the run output prints one line per such codeunit (with `--max-replay 50`:
   "test codeunit 50999 Chain Checker: re-run skipped (chain of 51 codeunits over the limit of 50); its parallel result
   is kept (flagged); --strict-order replays it"), with or without `--timings`, and each of its tests carries the JUnit
   property `singleInstanceRerun` with the text "re-run skipped (chain of M codeunits over the limit of N)"
   (`LaneReport::skipped`). `--stability` lists such tests (`rerunSkipped` in the JSON, a list of its own on the
   console). The cap is a cost bound of the runner, not a behavior of BC, so no corpus probe pins it (ADR-009: tooling).
3. **`--strict-order`** (`serve`: `strictOrder`) replays every dependent codeunit's chain whatever its length and
   whatever its tests read, as amendments 4 and 5 defined it. It wins over `--max-replay`.
4. **`--timings`** adds to the lane line how many codeunits kept their parallel run under the per-test rule and, with
   `--max-replay`, how many re-runs were skipped for the cap.
5. **No state across runs.** Nothing this amendment adds is kept from one run to the next: the per-test notes, the
   changed globals and the choice of re-runs are made inside the run and dropped at its end. A run never depends on an
   earlier run. The only thing kept across runs stays the compile cache (ADR-022).
6. **Costliest session first.** The worker threads take the serial sessions in order of the summed parallel run times
   of their codeunits. Which codeunits run, and from which state, does not change; only the wall time does.

### Owner's change (2026-11-08, before merge): the cap is opt-in

The first build had the cap on by default at 50 codeunits (`RERUN_CHAIN_LIMIT`). The owner changed that the same day:

- **Why.** On the synthetic stress suite the default cap turned 28 order-dependent passes into flagged failures: tests
  that pass in BC's full order were reported as failures because their chains were over 50. A default run must give
  the exact result. A cost bound that changes outcomes may only be something the user asks for.
- **Decision.** By default every re-run the per-test rule asks for is replayed in full, however long its chain. The
  cap is the flag `--max-replay N` (`serve`: `maxReplay`); without it there is no cap. With it, chains longer than `N`
  are skipped and flagged as in decision 2. `--strict-order` still replays everything and wins over the flag.
- **Kept on by default:** the per-test rule (decision 1), which works within one run and does not change an outcome
  (a failure that read nothing changed cannot change in the replay), and the costliest-session-first order
  (decision 6).
- **Runs never depend on earlier runs** (decision 5): no saved result changes a later run, beyond the compile cache.
- **Measured** (NP Retail, 5494 tests, BC 29, 12 threads, load average 5 to 13 from other work). Default: 2092 s,
  peak RSS 13.8 GB; all 26 re-runs replayed in full (3025 codeunit runs in 21 sessions, lane 1686 s); the per-test
  rule kept none, since every failing NP test comes after a test that read the changed state. `--max-replay 50`:
  494 s, peak RSS 4.0 GB; 3 re-runs replayed, 23 skipped and flagged (lane 172 s). The two runs give the same outcome
  for all 5494 tests. The default costs about what a run cost before this amendment (1831 s, 13.0 GB on main
  7c3c37006). Stress suite with the default: 0 outcome changes against main, no re-run skipped.

### Conformance

- `crates/al-vm/tests/language/single_track_access.rs`: a first read notes the test it came in.
- `crates/al-testrun/src/runtime/single_lane.rs` unit tests: the changed globals are noted per dependent codeunit; a
  failure sees a changed global only when it or an earlier test read it (a read in `OnRun` counts for all); a chain over
  the limit is left out with its length.
- `crates/al-testrun/tests/suite/single_instance_lane.rs`: a failed test that read no changed global triggers no serial
  run (`--strict-order` still runs it); without `--max-replay` a chain of 101 codeunits is replayed in full; with
  `--max-replay 50` a chain of exactly 50 codeunits is replayed and a chain of 51 is skipped and flagged, and
  `--strict-order` (with `--max-replay 50`) replays it and the test passes. The amendment 4 and 5 cases (P7B shape, a test refused
  while it runs after the reading test) still run again.
- `crates/alrunner-cli/tests/cli/onrun_stability.rs`: the default run replays the chain of 51 and passes; with
  `--max-replay 50` the skip text in the run output and the JUnit property, `--stability` lists the test under
  `rerunSkipped`, `--strict-order` passes it.
- `crates/alrunner-cli/src/args.rs` and `serve.rs` unit tests: `--max-replay` and `maxReplay` are off unless given.

## Amendment 8 (Accepted, owner, 2026-11-08): the carry chain walk keeps the SingleInstance state

- Status: Accepted by the owner on 2026-11-08 as "variant A as is". Amends amendments 2, 4, 5 and 7 for the members of
  the carry chain only. How a dependent codeunit outside the chain is found, flagged and run again is unchanged.
  Variant B (keep run ahead and replay one prefix of the session) was prototyped and not taken.

### What the code did before (checked)

- **A chain member runs once, in the walk, not in the parallel run.** `corpus.rs` builds the chain (every codeunit
  with RequiredTestIsolation = Disabled, and every codeunit runner 130450 runs whose OnRun writes or whose tests take
  numbers, `Unit::carry`), walks it (`ChainWalk::walk`) and then runs only the other codeunits in parallel. The
  "parallel result" of a chain member is the walk's result.
- **The walk runs in BC's order.** It sorts by (suite, leg, id): the TestRunnerHub's codeunits, then the codeunits
  runner 130450 runs. A carry member is never on the hub (`carry` requires `!on_hub`), so this is the order of the BC
  sessions `single_lane.rs` `streams` uses (suite, hub leg, id).
- **Run ahead.** Helper threads run members ahead from a predicted database state. The walk takes such a run only when
  its start has the fingerprint of the walk's own state. Each run, in the walk or ahead, was `run_unit` on a new
  `Session`: fresh single instances for every member. Only the lane moved single instances (`run.rs` `LANE_SINGLES`).
- **The lane replayed chain members.** A chosen dependent chain member ran again in a serial session of its chain
  (amendment 5), the single instances moved along. Its result replaced the walk's.

### Decision (owner, 2026-11-08: variant A)

1. **The walk keeps the single instances** from member to member of a BC session, moved as the lane moves them
   (`ChainWalk::keep_singles`, `KeptSingles`, `run::LANE_SINGLES`). The result of each chain member is then its run in
   BC's order (P7, `TestCuP7BReader.al:24-37`): no lane re-run, no cap, no per-test rule for chain members.
2. **Boundaries.** A new BC session (the suite or the hub leg changes) starts with fresh single instances. A chain
   member that is not run (a refused Disabled codeunit) and a member with a test refused while it ran hand nothing on:
   the next member starts fresh (R3 of amendment 3, as the lane does). A failed test hands its state on, as on BC
   (SingleInstance state is not rolled back). `chain_walk.rs` `starts_fresh`.
3. **Run ahead.** A run ahead (fresh single instances, another thread) is taken only when it read and wrote no watched
   global, was not refused, and the walk's single instances hold no manual binding (`ahead_ignores_singles`). Then its
   run cannot differ from the walk's. Otherwise the member runs on the walk's thread.
4. **Codeunits outside the chain** keep amendments 4, 5 and 7: they run in parallel with fresh state, are flagged when
   dependent, and run again in the lane when chosen. A chain member is a writer for them as before.
5. **Flags.** A chain member's result rests on P7, which is pinned, so it is not flagged `single-instance-order`, and
   `--stability` does not list it as `orderDependent` (`Lane::kept_chain`). `--strict-order` and `--max-replay` do not
   change a chain member.
6. **`--test-suites`**: the walk goes in id order, the suites in the file's order, so with `--test-suites` the walk
   keeps today's fresh state per member and the lane handles the members as before. The same when nothing is watched
   (no session of two codeunits or more).

No switch: variant A is the behavior. The prototype's environment variable (`AL_CHAIN_SI`) is gone, so core crates
still do no OS access (ADR-006).

### Accepted cost

Microsoft's suites lose run ahead. On Tests-ERM-Purchase every member uses Sequence No. Mgt. or another watched
single instance, so rule 3 sends every member to the walk's thread and the walk becomes serial. The owner accepted this
until run ahead is improved later (for example the extended rule of risk 1 below).

### Measured

Prototype (branch `rt-chain-si`, release, 12 threads, cores 0-2 and 5-11; load average 13 to 25 during the bucket
runs, so wall times carry large noise). Outcomes are per test (pass, failed, not run, and the message of each failure).

| Suite | Main default | Main `--strict-order` | Prototype | Outcome changes against strict / default | Peak RSS main / prototype |
|---|---|---|---|---|---|
| stress (868 codeunits, 821 in the chain) | 43.4 s | 45.2 s | 16.1 s | 0 / 0 | 1.87 / 1.90 GB |
| Corpus (BC 29, all buckets, `al-test-cli`) | same | not run | same | 0 (0 regressions, 0 new passes) | not measured |
| Tests-ERM-Purchase (32 codeunits, all in the chain) | 180 s (load 18) | not run | 314 s (load 13, includes a 15 s compile cache build) | - / 0 | 4.73 / 5.52 GB |
| NP Retail (336 codeunits, 333 in the chain, warm cache, load 5) | 2188 s (load 13) | killed by the 14 GB memory cap after 2315 s | 468 s (load 5) | - / 0 | 13.4 / 3.8 GB |

- **stress:** the lane goes from 8202 codeunit runs in 47 sessions to 106 runs in 12 sessions (only the 47 codeunits
  outside the chain); flagged tests 1445 to 83.
- **NP Retail:** the lane runs nothing (main: 3025 codeunit runs in 21 sessions). Same 5437 passes, 25 failures with
  the same messages and 32 not run; flagged tests 5224 to 0.

The port on branch `rt-chain-si-a8` (main plus this amendment, no switch) is measured in its pull request.

### Risks kept

1. **Run ahead lost** on suites that use it (the accepted cost above). A later improvement: a run ahead counts when
   every global it read first held the walk's current value and every global it wrote is a plain value the walk can
   set on its own instances (Sequence No. Mgt. holds integers). New code on the single instance seam; not built.
2. **State from codeunits outside the chain** is not seen by later members (99 members on stress, 225 on NP read such
   a global; `LaneReport::chain_missed` counts them). BC sees it. The outcomes matched in every measured suite.
3. **A test stopped by the time limit** hands on the state it had when it stopped; BC would have run it to its end.
   The lane has the same gap.
4. **Order-dependence information.** Chain members are no longer flagged, so `--stability` no longer lists them.

### What it replaces

For chain members: amendment 4, decision 2 (re-run after a failure), amendment 5, decision 1 (one serial session per
chain), amendment 7 (the per-test rule and `--max-replay`), and the `single-instance-order` flag.

### Conformance

- `crates/al-testrun/tests/suite/single_instance_lane.rs`:
  `a_chain_member_sees_the_single_instance_state_the_member_before_it_left` (passes in the walk, no re-run);
  `a_failed_chain_member_hands_its_single_instance_state_on`; `a_chain_member_of_another_hub_leg_starts_with_fresh_single_instances`;
  `a_chain_member_of_the_next_suite_starts_with_fresh_single_instances`;
  `the_chain_member_after_a_test_refused_while_it_ran_starts_with_fresh_single_instances`. The two carried-start tests of
  amendment 5 (`carried_starts_are_written_when_a_session_needs_them_and_freed_after`,
  `a_carried_start_that_sessions_share_is_made_once`) now use readers outside the chain (RequiredTestIsolation =
  Function, on the same hub session), since chain members no longer run again.
- `crates/al-testrun/src/runtime/chain_walk.rs` unit test: `starts_fresh` (suite and hub leg boundaries, a refused link).

## Amendment 9 (Accepted, owner 2026-11-09): the number of test threads follows the memory budget

- Status: Proposed. Owner request (2026-11-08): a run must fit on any machine, by running more of its work one after
  the other when memory is short, and a user can cap the memory with a flag. Amendments 7 and 8 are other changes; this
  one changes only how many test codeunits run at once. Which codeunits run, from which state, and every re-run of
  amendments 4, 5 and 7 are unchanged.

### Why

The parallel run (amendment 4) starts one test thread per core. Each running codeunit holds its own session, so the
peak grows with the thread count. On a 12-core machine Tests-SCM peaks at about 9 GB and a partner suite (NP Retail,
amendment 7) at 13.8 GB. A machine or CI container with less memory kills the process (out of memory) and the user gets
no result at all, although the same tests run fine on fewer threads.

### Decision

1. **Parallel by default, unchanged.** Without a limit that bites, the run uses one thread per core (or
   `AL_TEST_THREADS`), as before.
2. **The budget.** The front end works out the limit as the smallest of the machine's free memory (`MemAvailable`), the
   `memory.max` of the process's cgroup and its parents (a container or a systemd scope), and `--max-memory SIZE`
   (`serve`: the `runTests` field `maxMemory`, or `serve --max-memory` for every request). Reading these is OS access,
   so it lives in `alrunner-cli` (`memory_budget.rs`, ADR-006); `al-testrun` gets the number and a way to read the
   resident memory (`runtime/memory.rs` `MemoryBudget`).
3. **The thread count follows the budget.** After the install baseline the resident memory is the fixed part. The run
   starts as many threads as fit in nine tenths of the limit beside it at 200 MB each (`PER_WORKER_START`, measured
   below), at least one, at most the threads asked for.
4. **Admission per codeunit.** Before a thread starts the next test codeunit (the parallel run, the serial sessions of
   the lane, the chain walk's own runs and its helpers), it checks that one more fits: the larger of the resident memory and
   fixed part + running codeunits x cost, plus one more cost, must stay under nine tenths of the limit. The cost is what
   the process grew beyond the fixed part per running codeunit, at least 200 MB. Otherwise the thread waits until a
   running codeunit ends. One codeunit always runs, so a run always moves on. When a thread had to wait, the lane drops
   its cache of carried-state deltas (they are made again on demand).
5. **Stop with exit code 2** when not even one test thread fits: after the compile, the resident memory plus 64 MB
   (`WORKER_FLOOR`) is over the limit. The message gives the numbers and names `--max-memory`. `serve` and `debug` do not
   stop: they go on with one thread.
6. **`--timings`** prints a `memory:` line: the budget and its source, resident memory after the compile and after the
   install baseline, the cost per thread, the threads chosen of those asked for, how often a thread waited and how often
   a cache was dropped.
7. **Outcomes do not change.** Each codeunit still runs on a session of its own from the same baseline (the rule "one
   build, one result" of `docs/agents/brief.md`). Only the wall time changes. The budget is scheduling, an internal of
   the runner (ADR-009), so no corpus probe pins it.

### Measured

BC 29, Microsoft's apps, release build, 12 cores, one run at a time, peak = highest resident memory sampled every
0.25 s. Tests-ERM-Purchase (1724 tests) on the first build of this change with a budget that did not bite; all four
runs give the same outcomes (1472 passed, 6 failed, 246 not run):

| Test threads | Peak MB | Wall s |
|---|---|---|
| 12 (cold compile cache) | 4887 | 100.6 |
| 4 | 3314 | 90.3 |
| 2 | 3019 | 122.8 |
| 1 | 2285 | 134.9 |

- Resident memory after the compile and the install baseline (the fixed part): about 750 to 900 MB. A cold compile
  peaks at about 3.3 GB before it.
- Each thread from 2 to 4 added about 150 MB to the peak, each from 4 to 12 about 195 MB: hence 200 MB as the start
  cost.
- With one thread the resident memory still climbs from 1.3 to 2.3 GB: the carry chain of this suite keeps its state
  for the whole run, whatever the thread count.

Tests-SCM (2777 tests), after the merge with amendment 8, each run in a 10 GB cgroup (`memory.max`), 12 cores, shared
machine (load 2 to 6). "main" is the build without the budget, "branch" the build with it. Every finished run gives the
same outcomes (2294 passed, 103 failed, 380 not run; 0 differences between the JUnit files), except main at 4 threads,
where one performance test passed under the CPU time limit instead of being stopped:

| Build | Test threads | Budget | Peak MB | Wall s | Result |
|---|---|---|---|---|---|
| main | 12 | none | 10070 | 311 | killed at the 10 GB cgroup limit |
| main | 4 | none | 7478 | 363 | finished |
| main | 1 | none | 4203 | 294 | finished |
| branch | 12 asked | 10240 MB (cgroup) | 9303 | 832 | finished; waited for memory 150 times |
| branch | 12 asked | 4096 MB (`--max-memory 4G`) | 9746 | 1049 | finished; waited for memory 243 times |

A stress app without Microsoft's apps (1474 tests) peaks at about 250 MB with 1 and 4 threads; the budget does not
bite there.

What this shows:

- Where main is killed by a 10 GB limit, the branch finishes under it with the same outcomes.
- The budget does not hold a limit below what the run needs: with `--max-memory 4G` the peak is still 9.7 GB, above
  even the one-thread peak of 4.2 GB. The resident memory grows with run time, not with the number of running
  codeunits: 12 threads that each keep their own state (their copy of the install baseline, the carried starts they
  hold, freed memory the allocator keeps per thread) grow while most of them wait. The gate limits how many codeunits
  run, not what an idle thread holds. The learned cost per thread (6.5 to 7 GB) then lets about one codeunit run at a
  time, so the run takes 2.8 to 3.6 times as long as main with one thread, which is the fastest run of this suite.
- The start cost of 200 MB per thread, measured on Tests-ERM-Purchase, is far too low for Tests-SCM (about 1.1 GB per
  thread from 1 to 4 threads), so the run still starts all 12 threads.
- Open: the thread count must drop when memory is short (fewer threads, not only fewer running codeunits), and the start
  cost must come from the suite or be learned before all threads start. Until then the budget is a safety net against
  an out-of-memory kill near the limit, not a way to run in much less memory.

### Limits

- The budget cannot cut what does not depend on threads: the carried state of a long carry chain grows for the whole
  run (Tests-ERM-Purchase: from 1.3 to 2.3 GB with one thread). A limit below that still stops or runs slowly; it never
  changes an outcome.
- The compile phase is not gated: a cold compile cache peaks at about 3.3 GB for Microsoft's apps before the test threads
  start.
- Resident memory is read on Linux only (`/proc/self/status`). Elsewhere the run is not gated, as before.

### Conformance

- `crates/al-testrun/src/runtime/memory.rs` unit tests: the thread count from limit, fixed part and cost; the stop
  message; the gate waits when one more does not fit, always lets one codeunit in, and learns the cost from what it sees.
- `crates/alrunner-cli/src/memory_budget.rs` unit tests: sizes, the smallest limit wins, `MemAvailable` and cgroup
  `memory.max` (also `max`) are read; `args.rs` and `serve.rs` tests for `--max-memory` and `maxMemory`.
- `crates/alrunner-cli/tests/cli/memory.rs`: `--max-memory 1M` stops with exit code 2 and the numbers; the `memory:`
  line of `--timings`; the default source; the same outcomes under a tight budget; a bad size exits with code 2.
