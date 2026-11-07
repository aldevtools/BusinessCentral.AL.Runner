# ADR-066: Run the test runner codeunit's own AL

- Status: Accepted (owner, 2026-11-05); amendment 2 (P7, SingleInstance across test codeunits) Proposed (2026-11-07)
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



## Amendment 2 (Proposed, 2026-11-07): SingleInstance state from one test codeunit to the next (P7), found and run in one serial lane

- Status: Proposed. Supersedes amendment 1 (carrying the state between sessions, in pull request 7247 on branch
  `rt-p7v2`, which is not merged and stays unmerged). Owner decision of 2026-11-07, written down here for acceptance.

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
