# ADR-074: Which test driver al-runner models, and the discovery pass of page 130455

- Status: Accepted (owner, 2026-11-12, option 1). Default driver: `bc-linux` (no behavior change for a run that does
  not ask for `bccontainerhelper`); a second pull request makes `bccontainerhelper` the default of `al-runner test`.
- Date: 2026-11-12
- Amends: ADR-055 (OnRun rows), ADR-066 decision 2 and owner decision 2 (native rows, discovery "only if
  P6 shows the extra OnRun is observable"), ADR-070 (what a test codeunit starts from).
- Source: corpus PR 1314 (`tests/al-language-isolation-probe/src/SwOnRunRunner.al`), corpus Windows nightly run
  43058375098 (BC 28.4, artifact 13889917561), the pinned Linux oracles `oracle/<major>/junit.xml`, and the Test Runner
  app source of BC 29.0 (`Test Runner.Source.zip` in the artifact cache, not committed).

## Context

Two drivers start BC's tests, and the corpus pins both. They give different results for the same corpus file.

**BcContainerHelper (`Run-TestsInBcContainer`, the way AL-Go runs tests).** It opens page 130455 "Command Line Test
Tool" in one client session. Setting the codeunit range discovers every test codeunit of the range first
(`CommandLineTestTool.Page.al:50-54`, `TestSuiteMgt.Codeunit.al:278-285` and `:409-419`): codeunit 130452 "Test Runner -
Get Methods" runs each test codeunit and answers true for `OnRun` (`TestRunnerGetMethods.Codeunit.al:36-46`). Then
`RunNextTest` runs one codeunit per call through runner 130450. There is no TestRunnerHub. Measured:

- `SwOnRunRunner.al:10-26`, observed on BC 27.5, 28.5 and 29.0 (and in the nightly's `xunit-onrun-runner.xml`, all five
  reader tests pass): each test codeunit's OnRun runs once in the discovery pass, in the test session, without test
  isolation. Its writes stay (row 1 of "LT SW OnRun Writes"), and SingleInstance state it changed is still there when
  the run starts. The run calls OnRun again, inside 130450's codeunit isolation: its row 2 is seen by the test and rolled
  back after the codeunit. An OnRun that raises only in the discovery pass leaves the codeunit with no method rows (no
  result rows at all); one that raises only in the run is Skipped with its methods listed as not executed.
- `codeunits/run-onrun/RqIsolationModes.al` in the same nightly (`xunit-al-language.xml`): `RQ_C02_SecondTest`,
  `RQ_F02_SecondTest` and `RQ_D02_SecondTest` fail with "expected [notes = 6] but was [notes = 8]": the OnRun of the A
  codeunit ran twice in the session. `RQ_C11` and `RQ_F11` see OnRun's committed row (the discovery row).

**The corpus Linux legs (bc-linux's runner; the main oracle).** On BC 28 and later a test codeunit without
`[HandlerFunctions]` and with `TestPermissions = Disabled` runs on the TestRunnerHub, every other one under 130450.
Measured (`RqIsolationModes.al:22-41`, passing in `oracle/27`, `28` and `29/junit.xml`): OnRun runs once in the test
session (`onrun_calls` 1, `notes` 6, `:300-308`). Under 130450 (every codeunit on BC 27) OnRun's writes reach the next
codeunit (`:357-369` `#else`); on the hub they are rolled back.

**What al-runner models today.** The Linux legs, also for its users: the hub leg whenever RequiredTestIsolation is
honored (`al-testrun` `runtime/corpus.rs` `on_hub`, `hub_path`), OnRun once, and under 130450 the run's OnRun treated as
outside the codeunit isolation so its writes reach the later codeunits (`runtime/chain.rs` `onrun_writes`,
`runtime/run.rs`). Measured on main 764d69ecf with `al-runner test corpus/tests/al-language-isolation-probe --codeunit
97010..97018` on BC 29: `Raises_AfterOnRun` passes (BcContainerHelper: Skipped), and the reader
`OnRunWrites_DiscoveryRowAndRollback` fails (`OnRun calls 1, rows 1`; BcContainerHelper: `OnRun calls 2, rows 1,2`). The
SwOnRun codeunits even run on the hub leg there, which BcContainerHelper does not have.

So "implement the discovery pass as BC does it" has no single answer. A discovery pass in the test session makes the
SwOnRun readers pass and makes `RQ_C02`, `RQ_F02` and `RQ_D02` fail on BC 27 (130450 there; `notes` 8 against the
oracle's 6), which hard rule 2 forbids. Leaving it out keeps the Linux oracle green and stays wrong for every AL-Go
user whose test codeunit has an OnRun with side effects.

## Options

1. **Name the driver.** `RunOptions::driver` with two values, one owner module per difference:
   - `bc-linux`: today's model, unchanged (hub leg on BC 28+, OnRun once, its writes carried under 130450).
   - `bccontainerhelper`: no hub leg; before the first unit of a BC session, a discovery pass (new `runtime/discovery.rs`)
     runs the OnRun of every selected test codeunit of that session, in id order, in one session from the install
     baseline, committed after each, under no test isolation; its database state, number sequences, isolated storage and
     SingleInstance state are the start of every unit of the session (in the default order mode the SingleInstance part
     is dropped, as ADR-070 drops carried state); a codeunit whose discovery OnRun raised gets no method rows and
     reports no rows; the run's OnRun runs inside the runner's isolation and carries nothing.
   - The corpus Linux suites in `oracle.toml` name `bc-linux`; the isolation-probe suites (Windows nightly) and a new
     `onrun-runner` suite (codeunits 97010..97019 under 130450, oracle from run 43058375098) name `bccontainerhelper`.
   - Cost: one switch read in two places (the hub decision and the discovery pass). Each value has its own oracle.
2. **One model, BcContainerHelper's**, and ask the corpus to drive the Linux legs through page 130455 in one session,
   then re-record the oracles. Until the re-recorded oracles land, the Linux cases that change would need not-passing
   lines for cases the oracle had (hard rule 2: owner's OK).
3. **Keep the Linux model only.** Record in ADR-055 that AL-Go users can see OnRun twice; register an assumption that
   flags every test of a codeunit with a declared OnRun under 130450 ("BcContainerHelper runs OnRun once more before the
   run"). The SwOnRun readers stay failing and are listed as not passing for the isolation-probe app.

## Recommendation

Option 1, with `bccontainerhelper` as the default of `al-runner test` and `serve` (it is what AL-Go users compare
against, and ADR-055 already names that driver for the rows), and `bc-linux` for the corpus Linux suites. Built in two
pull requests: (1) the switch, `discovery.rs`, the `onrun-runner` oracle suite, the isolation-probe suites on
`bccontainerhelper`; the default stays `bc-linux`; (2) the default flips after a run on the partner apps shows the
changed outcomes and each is explained.

Not pinned under `bccontainerhelper`, refused or kept as today until a probe lands:
- Whether an earlier codeunit sees what a later codeunit's discovery OnRun wrote. The Test Runner source discovers the
  whole range first; no corpus test reads it. Until probed: a test whose reach reads a table, a number sequence or
  isolated storage that a later codeunit's OnRun can write is refused.
- An OnRun that raises in the discovery pass after it changed something: what the pass keeps of it. Refused.
- Discovery across apps (several test apps, one range) and OnRun of test codeunits in dependency apps. Refused.
- `OnGetTestMethods`, `OnBeforeAddTestMethodLine` and `OnAfterAddTestMethodLine` subscribers (130452 raises them per
  method). Refused when a subscriber exists.

## Open questions for the owner

1. Which option?
2. Under option 1: the default driver of `al-runner test` (recommended: `bccontainerhelper`, flipped in the second
   pull request).
3. Under option 1: may the Windows nightly's XUnit files be oracles for the `bccontainerhelper` suites (today only the
   two isolation suites are)?

## Conformance

- `crates/al-test-cli/tests/suite/isolation_probe.rs`: the `onrun-runner` suite against `xunit-onrun-runner.xml`.
- The Linux oracles stay green: `RqIsolationModes.al` `RQ_C02`, `RQ_F02`, `RQ_D02` on BC 27, 28, 29.

## Decision (owner, 2026-11-12)

Option 1. The driver is chosen per run: `al-runner test --test-driver bc-linux|bccontainerhelper`
(`RunOptions::test_driver`); `bc-linux` is the default. The corpus harness picks, per suite, the driver its oracle was
recorded with (`SuiteRun::driver`, from the oracle's format: JUnit from the Linux legs is `bc-linux`, XUnit from the
Windows nightly is `bccontainerhelper`, `al-test-cli` `OracleFormat::driver`), so the two isolation suites run under
`bccontainerhelper` and every Linux suite is unchanged. Open question 3 (more Windows XUnit files as oracles, such as an
`onrun-runner` suite) stays open; the SwOnRun reader is pinned by a Rust test against its own expectation
(`al-test-cli` `tests/suite/isolation_probe.rs`) and by the red/green run of the corpus case.

### What was built (first pull request)

One owner module, `al-testrun` `runtime/discovery.rs`:
- `TestDriver::has_hub`: no TestRunnerHub leg under `bccontainerhelper` (`corpus.rs` `hub_path`); `--isolation required`
  with `bccontainerhelper` is an error in `al-runner`.
- The discovery pass is a link of the carry chain at the front of each BC session (`chain_walk.rs`
  `Link::Discovery`, `ChainWhy::Discovery`): it runs the OnRun trigger of every selected test codeunit, in id order,
  in a test session from the install baseline, committing after each. What it leaves (rows, number sequences,
  isolated storage) is where the codeunits of the session start; a codeunit that reads a table it changed runs again
  from it, as after a Disabled codeunit. Under `--strict-order` the session runs as one walk (`strict_sequential`) and
  the single instances the pass left move into the first codeunit.
- `TestDriver::run_onrun_carries`: the run's call of OnRun carries no rows (SwOnRunRunner.al:21-23); a codeunit whose
  OnRun can change a number sequence or isolated storage, or whose tests take numbers, stays in the chain, starting the
  later codeunits from the state before its OnRun (`discovery::run_onrun_left`).
- A codeunit whose OnRun raised in the pass reports no method rows; the run still calls its OnRun (SwOnRunRunner.al:24-26).

Refused under `bccontainerhelper` until a probe pins them:
- A codeunit whose reach touches a table that a later codeunit's OnRun changed in the pass, or, when its reach opens
  tables through a RecordRef or is partly unknown, one that its run read (measured, `Scratch::read_tables`;
  `discovery::unpinned_reach`); under `--strict-order` also one that reads a SingleInstance global a later codeunit's
  OnRun can write.
- Any codeunit of the session (the writer itself included) whose reach can read a number sequence or isolated storage
  that the pass changed: only the rows and the SingleInstance state the pass leaves are pinned.
- Any runner other than 130450 (`--test-runner ID`, `--isolation test` or `disabled`, `--isolation required`): every
  test of the session (an allowlist, `discovery::unmeasured`). A corpus harness suite whose oracle BcContainerHelper
  recorded runs under exactly the runner of that oracle (the isolation suites, runners 61300 and 61301); the harness
  compares every case with it.
- Without `--strict-order`: a codeunit whose reach reads a SingleInstance global the pass changed (each codeunit starts
  with fresh single instances there, ADR-066 amendment 12). `al-runner` and `serve` never run `bccontainerhelper` that
  way: the pass and the run are one session (SwOnRunRunner.al:18-20), so they always take BC's order
  (`TestDriver::one_session`). Only the harness's isolation suites run it in the default order.
- A pass whose OnRun raised after it changed the database or session state, was refused, stopped by the time limit or
  went through an unprobed assumption: the codeunits that read what it may have written.
- A pass that changed the session language or WorkDate (each codeunit has a session of its own here): every test of
  the session.
- Selected test codeunits of several apps, `--test-suites`, and a subscriber outside the Test Runner app to
  `OnGetTestMethods`, `OnBeforeAddTestMethodLine` or `OnAfterAddTestMethodLine` of codeunit 130452: every test of the
  session. Test codeunits of dependency apps are never selected, so the pass never runs their OnRun.
- The run's OnRun changing a number sequence or isolated storage: the later codeunits that can read them.
