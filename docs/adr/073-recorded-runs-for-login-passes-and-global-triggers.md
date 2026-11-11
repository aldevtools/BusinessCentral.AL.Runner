# ADR-073: Run the login, the pass baselines and the Global Triggers setup once, and replay the result

- Status: Proposed
- Date: 2026-11-11
- Amends, when accepted: ADR-001 (what the sessions of one engine share), ADR-013 decision 5 and ADR-069 decisions 1
  and 6 (what a test session starts from), ADR-063 decision 3 and as-built point 8 (what a later pass reuses).
- Source: the measurement `perf-1111` (2026-11-11, BC 29 full run, `perf stat -e instructions:u`) and the code-only
  fixes of the pull request "Full-run budget: four code-only performance fixes" (PR 7754).

## Context

The full corpus run with Microsoft's apps is over its instruction budget (`perf_budget.rs`) since 2026-11-06.

| | Instructions |
|---|---|
| main before the code-only fixes | 371.3 G |
| after the code-only fixes (PR 7754, corpus pin `f2d5ea3`) | 333.2 G |
| limit at that corpus size (82.8 G + 4 230 000 per KiB x 46 770 KiB) | 280.6 G |
| still over | 52.6 G |

No code-only fix is left that the measurement found. What remains repeats the same work with the same inputs:

| Cause (measured at main `46819ce1d`) | Cost | Repeats per |
|---|---|---|
| The login raised in AL before the first test (`login.rs`, `install::test_session_company_open`) | 16.4 G | test session |
| The testlibraries pass (129 cases) | 27.6 G | run |
| The try-write pass (suite `trywrite-allowed`, 33 cases) | 19.7 G | run |
| Global Triggers: the setup asked (9.1 G) and the database events raised (5.2 G) | 14.3 G | session and table |
| Platform listings built once per pass program | 10.6 G | pass |
| The login decision's own reach walk (`login::decide`) | about 10 G | test codeunit; not covered here |

Wall time of the two extra passes at main (`--timings`; a rough guide only, tests run on several threads):
testlibraries 0.32 s load, 1.02 s compile, 1.39 s install baseline, 0.96 s tests; try-write 0.35 s, 0.86 s, 1.06 s,
0.62 s. On 2026-10-31 the install baseline was 5.1 G of the 20.1 G the testlibraries pass cost then (ADR-063, as
built).

Each fix needs state to cross a line that an accepted ADR draws, so it needs this ADR (hard rule 5):

- A test session starts from the install baseline and nothing else (ADR-013 decision 5, ADR-069 decision 1).
- A later pass reuses the compile and no runtime state (ADR-063 decision 3).
- Sessions of one engine share only immutable data (ADR-001). One exception exists: `install::LoginMemo` records the
  login code that sets WorkDate once per engine and replays it.

What the code already has, and this ADR builds on:

- `LoginMemo` (`al-testrun/src/install.rs`): one real run under the impact trace, a key (baseline origin, test data,
  company, WorkDate), a check that the run can be replayed (`replayable`), and a replay that sets the WorkDate, the
  changed globals and the assumption marks.
- `SharedBaseline` (ADR-069): rows frozen once and shared by all threads; a session's writes never reach them.
- Table stamps (`Session::table_stamp`): equal stamps mean the same rows, from one counter for the whole process.
- The impact trace (`al_vm::impact::Touches`): the events raised, the tables read and written, the pages, reports and
  enums used, whether .NET ran, and the routines entered.
- The static reach facts (`al-testrun/src/runtime/written.rs`, `ReachFacts`): the session state and the codeunit
  globals a set of routines can read and write.
- The kept compile (ADR-063): a later pass knows which objects it took unchanged from the main pass.
- `SharedListings` (`al-vm/src/platform/mod.rs`): engines of one program share their platform listings.

Two facts I checked for this ADR, because the estimates depend on them:

- Base Application answers the database trigger setup from one subscriber (codeunit 49), which asks the CRM
  integration, the API webhooks, an integration event and the change log. The change log codeunit is SingleInstance
  and keeps the two setup records it read in record globals. So the first ask of a session writes codeunit globals of
  type Record. A replay that can only restore Date globals, as `LoginMemo` does, would never apply here.
- A pass has one program, one engine and one install baseline for all its suites. The OnPrem suite apps are already
  compiled and installed in the main pass, next to the main app.

## Decision

### The rule every phase follows

AL code that the VM runs is a function of what it reads. If one real run is kept with everything it read (its
inputs) and everything it left (its result), then a later session whose inputs are all equal would leave the same
result by running the same code. Giving that session the kept result has the same observable outcome (ADR-009):
the same values, errors, database state, and the same triggers and subscribers as seen through what they left.

Terms, used the same way in every phase:

- **Recorded run**: one real run of a piece of AL code, kept with its inputs and its result.
- **Replay**: a later session takes the result of a recorded run and does not run the code.
- **Session image**: the state of a session outside the database rows, as plain data that can go to another thread.

A replay is exact only when the inputs and the result are complete. The decisions below make that a property of the
code structure, not of a reviewer's attention.

1. **One owner.** A new module in `al-vm` owns recording, the session image and replay. `LoginMemo` moves onto it in
   Phase 1 and its special code is deleted. No phase gets its own copy of the rule.
2. **Every `Session` field is classified once.** The function that makes a session image destructures `Session`
   without `..` (the pattern of `RunInputs::of` and `manifest_digest`). A new field does not compile until it has a
   class:
   - *imaged*: written into the image and restored from it (WorkDate, language, counters, isolated storage, number
     sequences, the `Random` position, the clock position, the last error, the Global Triggers setups already asked,
     the SQL counter state, the SingleInstance codeunits and their globals);
   - *must be untouched*: a run that changed it is not recorded (open pages, UI handlers, bound manual subscribers,
     temporary tables held outside globals, files, .NET objects, background tasks, HTTP state, a pending platform
     table write);
   - *not state*: caches that can be rebuilt and tooling data.
3. **Codeunit globals have a closed image.** A global is imaged when it holds a plain value (the values
   `value_codec` already writes), a list or dictionary of plain values that nothing else shares, or a record
   variable that is not temporary (its fields, filters, key and position, by an exhaustive destructuring of the
   record state). Anything else (a temporary record, an interface, a codeunit or page variable, a shared collection)
   means the run is not recorded. A global that still holds its initial value is left out.
4. **Inputs come from the run, not from a prediction.** The impact trace gives the tables, events and routines of
   the run. It is extended by: the platform listings read, the engine settings consulted, clock reads and `Random`
   draws. The session state and globals the run read are the static facts of the routines it entered (as
   `LoginMemo` does today), which can only name too much.
5. **A run that is not understood is not recorded.** A run that used a page, a report, a file, HTTP, a .NET member
   that does not answer from its arguments alone, a routine with unknown reach, or that ended in an error or a
   refusal, runs in every session as today. The engine counts why (`--timings`, `work.json`).
6. **A session that notes the code it runs never takes a replay** (coverage, the impact trace, value capture, the
   debugger; `Session::notes_code_run`). Those features must see the code run.
7. **Nothing is kept across processes** by this ADR. The existing file of login records stays as it is.
8. **Each phase has an off switch**, an environment variable read in the front end (ADR-006), for the differential
   checks and for diagnosis.

### Phase 1: the login runs once per engine (cause: 16.4 G)

Today each test session raises the login in AL before its first test: Microsoft's `OnCompanyOpenCompleted`
subscribers (`Login::Raise`), or the two login events to the apps' own subscribers (`Login::Own`).

Decision: the first session with a given key runs the login for real, under the trace. The recorded run covers both
steps a session does when it opens the company: the WorkDate step (`install::login_work_date`) and the company-open
step. If the run can be recorded, its result is kept per engine as a **login start**: the rows as a frozen state on the install baseline (ADR-069
decision 6 already allows frozen states on the same base) and the session image. A later session with the same key
starts from the login start. At most 8 login starts are kept per engine, as `LoginMemo` keeps 8 runs.

What a test can observe of the login, and where the replay gets it from:

| A test can observe | Source in a replay |
|---|---|
| Rows the login wrote, with SystemId, row version, SystemCreatedAt and SystemModifiedAt | The frozen rows. The key holds the counters and the clock position the run started from, so the real run would write the same values |
| AutoIncrement values, number sequences, isolated storage | Session image |
| SingleInstance state Microsoft's login code and the apps' subscribers left | Session image (decision 3) |
| WorkDate, language, `Random` position, clock position, last error text and call stack | Session image |
| Whether a later first write to a table asks the Global Triggers setup again | Session image (the setups already asked) |
| `SqlStatementsExecuted` and `SqlRowsRead` | Session image |
| That the subscribers ran once, in order (`LgnLoginChecklistTests.al:131-148`, `:159-163`) | Only through what they left: rows and globals, both restored |
| A flagged pass (assumption marks of the login) | Replayed marks, as `LoginMemo` does |
| The tables the apps' subscribers wrote (the watch of `CompanyOpenWrites`) | Kept with the login start |

The key: the baseline origin with no table changed since; the test data; the company; WorkDate; the login decision
(`Raise` or `Own`, the events, the subscribers in order, the watch reason); the client kind; the clock position; the
`Random` state; the host values of a stability variant; no SingleInstance codeunit exists yet; no manual subscriber
is bound.

What makes a session run the login itself: any part of the key differs. That covers a carry chain member that starts
from carried rows, `--strict-order` (single instances move in), Disabled isolation with carried state, and a
session that notes code. A stale replay cannot happen: baseline origins and table stamps are never reused within a
process, and the key is compared on every use.

Not changed: `login::decide` still chooses per test codeunit. Its reach walk (about 10 G) stays.

Expected saving: 12 to 14 G (the measurement's estimate). It depends on how often the login leaves only state the
image can hold; step 1 of the phase measures that before anything is built.

Follow-up, not decided here: once Microsoft's login is paid once per engine, every session could take the `Raise`
start. That is closer to BC (it always runs the whole login) and would remove the decision walk. It changes
outcomes, so it needs its own check against the oracle.

### Phase 2: the try-write suite runs in the main pass on a second engine (cause: 19.7 G)

The try-write suite is a pass of its own only because its CI job sets `DisableWriteInsideTryFunctions=false`. Its
app depends on the main corpus app, like the OnPrem suite apps.

Decision:

1. The try-write app is compiled and installed in the main pass, as the OnPrem suite apps are. A suite with
   `server_settings` no longer makes a pass when its apps are otherwise those of another pass.
2. Its suite runs on a second engine made from the same program, with the setting on. Engines of one program
   already share their platform listings, so the listings of this suite cost nothing more.
3. `Engine::write_in_try_allowed` becomes private. The one accessor notes on the session that the setting was
   consulted.
4. The baseline run captures the data before each app under test (a pointer copy; SingleInstance state starts
   again there, ADR-046 amendment 1, A4 item 2). The second engine's baseline starts from the capture before the
   first install step that consulted the setting and runs the remaining steps under its own setting. If no step
   consulted it, both engines share one frozen baseline.

What a test can observe, and why it is the same: a try-write test sees install rows, and code behavior under the
setting. Every install step before the first one that consulted the setting did not read it, so it would have run
the same with the setting on. The steps from there on really run with the setting on. The tests run on an engine
with the setting on.

What changes in the test environment, and this is the cost of the phase: the main app's tests now have the
try-write app in their program (its objects in the object listings, its subscribers, its install rows written
under the default setting), and the try-write tests have the OnPrem suite apps in theirs. On BC neither job has the
other's app. The main pass already makes this compromise between the main app and the OnPrem suite apps. The
try-write app and the OnPrem suite apps do not depend on each other, so the install-order rules of ADR-046
amendments 6 and 7 apply to one more pair of apps. The differential check shows whether any of today's cases can
tell; it cannot show it for cases written later.

What sends the suite back to a pass of its own: the off switch, or a difference in the differential check.

Expected saving: 13 to 15 G of 19.7 G (the measurement estimated 15 G). What stays is the second engine, the
try-write app's own install code and its 33 cases.

### Phase 3: the testlibraries pass reuses the start of the main pass's install baseline (cause: 27.6 G)

This pass has other dependency apps (Tests-TestLibraries, Data Archive, 178 subscribers), so it keeps its own
program, engine and environment (ADR-063). What can be shared is the part of the install baseline that runs before
the first app that differs.

Decision, to be built only if its measuring step passes the gate below:

1. The main pass's baseline run keeps a **checkpoint** after the last install root both passes share: the rows
   (frozen) and the session image of the install session.
2. The later pass starts its baseline from the checkpoint when everything the main pass's run consulted up to there
   is the same in its program:
   - every routine entered belongs to an object taken unchanged from the kept compile, at the same object index;
   - every table read or written is such an object too (so its fields, also from extensions, are the same);
   - every event raised has the same subscribers that may run at that step (the dispatcher's own rule, with the
     apps not installed yet and the apps whose install order is unknown);
   - every read of a platform listing gives the same rows in the later program (the reads are kept and run again);
   - the engine settings, the host values, the test data and the company list are the same;
   - .NET ran only members that answer from their arguments (the rule of the on-disk baseline, `dotnet/pure.rs`).
3. `baseline_key.rs` stays the one list of what a baseline depends on. The checkpoint check reads that list; it
   does not get a second one.
4. Otherwise the pass runs its whole baseline, as today.

What a test can observe is the install baseline as a whole: rows, counters, the tables marked as not seeded, the
install facts, the notes and assumption marks. The checkpoint holds the state the later pass's own run would have
reached at that point, because it would have run the same code on the same inputs.

Honest limits:

- The saving is uncertain. The measurement estimated 10 to 15 G. The baseline was 5.1 G of this pass on
  2026-10-31, and the shared start is only part of the baseline. I expect 5 to 15 G and cannot narrow it without
  the measuring step.
- The install session keeps SingleInstance state across dependency apps, so the checkpoint depends on decision 3
  (record globals).
- Passes exist only in the corpus job. A user's `al-runner test` has one pass. This phase adds code that only
  makes the project's own measurement cheaper. The budget section offers another way to handle this cost.

### Phase 4: the Global Triggers setup answer is replayed (cause: 9.1 G of 14.3 G)

Today the first write to a table in a session raises `GetGlobalTableTriggerMask` and
`GetDatabaseTableTriggerSetup` to the subscribers (`Session::global_setup`). The database events themselves (5.2 G)
have effects by design (change log entries) and are not part of this phase.

Decision, to be built only if its measuring step passes the gate below:

1. A recorded run is one ask. Its result is the answer (`GlobalSetup`), the image of the globals it changed and its
   assumption marks. It is recorded only if it wrote no row, raised no event outside the routines it entered, did
   not read the clock, drew no `Random` number and raised no error.
2. Its key: the table, the company, the session values the run read (execution context, client kind), the image of
   the globals the run could read, and the stamp of every table the run read.
3. Before an ask runs, `al-vm` looks for a recorded run of the same table whose globals image and table stamps
   equal the session's now, and whose events have the same subscribers bound now. If one matches, the session
   takes the answer and the changed globals. If none matches, the ask runs and is recorded.
4. The boundary between `al-vm` and `al-testrun`: `al-testrun` puts, once per engine and before the first session,
   the static facts of the routines the Global Triggers subscribers can reach on the engine, as plain data.
   `al-vm` does the recording and the replay with them. `al-vm` does not call back into `al-testrun` and does not
   depend on `al-sema`. A routine the facts do not cover means the ask is not recorded.

What a test can observe of an ask: the answer (which database events follow), the rows the subscribers wrote (none,
or the run is not recorded), the globals they changed (restored), and that a later write to the same table does
not ask again (unchanged: the session still keeps its own setup per table). A test that changes the Change Log
Setup, binds a subscriber or changes a global the subscribers read gets another key and a real run.

Expected saving: 8 to 9 G (the pull request's estimate). Without decision 3 (record globals) it is close to 0 on
Microsoft's apps.

### Platform listings across passes (10.6 G)

No separate mechanism. The try-write suite shares the main program's listings through Phase 2. The testlibraries
pass has other objects, so nearly every listing differs; sharing them would need a rule per listing kind for a small
saving. A program identity across compiles is needed only by Phase 3, where it is "the object was taken unchanged
from the kept compile at the same index".

### Order, measuring steps and the gate

Each phase starts with a measuring step: a run that records, counts how many replays would apply and why the others
would not, and still runs everything. A phase is built when that step shows a saving of at least 5 G on BC 29. The
repeat noise of the measurement is under 0.5 % (about 1.7 G); a seam change should pay at least three times that.

1. The shared module (decisions 1 to 8), with `LoginMemo` moved onto it. No saving; the WorkDate replay must still
   apply as often as before.
2. Phase 1, the login.
3. Phase 2, the try-write suite. It needs none of the image code, so it can be built in parallel with step 2.
4. Phase 3, the testlibraries pass, if measured and if the owner chooses it (budget section).
5. Phase 4, the Global Triggers setup. Last: it sits in the path of every first write, and its key is the widest.

### Savings against the limit

| After | Run (BC 29) | Limit |
|---|---|---|
| PR 7754 | 333.2 G | 280.6 G |
| Phase 1 (12 to 14 G) | 319 to 321 G | |
| Phase 2 (13 to 15 G) | 304 to 308 G | |
| Phase 3 (5 to 15 G) | 289 to 303 G | |
| Phase 4 (8 to 9 G) | 280 to 295 G | 280.6 G |

With the measurement's own estimates for every cause (48 to 57 G in all, 3 G of it for shared listings) the run
ends at 276 to 285 G. Either way, all four phases together end at the limit with no margin, and the limit is
reached only if every estimate holds.

### The budget formula: options for the owner

This section decides nothing. Changing a budget is the owner's decision.

Facts:

- The formula is a fixed part (82.8 G, the empty-corpus run of 2026-10-19 plus 5 %) plus 4.23 M per KiB of corpus
  source. It was set before any extra pass existed. It has no term for a pass.
- The two extra passes cost 47.3 G; the run is 52.6 G over the limit. A pass costs its compile growth, its
  engine, its install baseline and its listings whatever the number of cases.
- The empty-corpus run costs 68.6 G today. So the fixed part already has 10.8 G more room than "today plus 5 %".
- Without the fixed part the run costs about 5.66 M per KiB today, against 4.23 M in the formula. After all four
  phases it would be 4.5 to 4.8 M per KiB. So a growing corpus goes over the limit again unless the cost per test
  falls further.
- The test has failed since 2026-11-06. The step that broke it was unplanned (36 to 40 G), and while it failed
  two more unplanned regressions came in (about 14 G and 29 G). All three were found only by this measurement.
- The instruction limit fails a run only where an instruction counter exists (a local run). CI has none and only
  warns.

Options:

1. **Keep the formula.** All four phases are needed, and the run ends at the limit with no margin.
2. **Add a term per extra pass**, measured the way the fixed part is: the pass with no test selected, plus 5 %,
   and named per pass in the test, so a new pass needs the owner. Measured after Phase 2, so it covers only the
   testlibraries pass. Then Phase 3 is not needed, and its code is not written. This models a cost of how the
   corpus is measured; it hides no cost a user pays.
3. **Measure both parts again** (fixed part down to about 72 G, the per-KiB part up to today's value). This
   accepts today's cost per test, including the causes of Phases 1 and 4. I advise against it before those phases.
4. **A temporary second limit that only goes down**: the test also passes below a recorded value (the last
   measurement plus 1 %), and each performance pull request lowers it, until the formula holds again. This is the
   rule the file-size and function-length ratchets already use. A local run of the test then passes or fails
   again, so a new regression shows.

My recommendation: option 4 now, Phases 1 and 2, then option 2 with the measured cost of the testlibraries pass,
then Phase 4 if its measuring step passes the gate. Reason: Phases 1 and 4 remove cost every user's run pays.
Phase 2 is small because it uses what the main pass already does for the OnPrem suites. Phase 3 is the largest
piece of new code for the least certain saving, and only the corpus job gains from it.

### Options not chosen

- **Ask the Global Triggers setup for every table when the session starts, or start sessions with warm
  SingleInstance state.** BC does neither; a test that changes the setup before its first write would see the
  difference.
- **Replay the Global Triggers answer when the subscribers have no side effects** (static reach only). The answer
  also depends on rows, on reads through `RecordRef` and on globals (PR 7754).
- **One program for all passes, with the runtime hiding the extra apps.** Rejected in ADR-063; the reasons hold.
- **Keep the try-write suite's own program and reuse the main baseline across the two programs.** It keeps both
  environments exactly as today, but needs the whole check of Phase 3 for a suite of 33 cases. It is the fallback
  if the owner does not want the try-write app in the main pass.
- **Keep recorded runs on disk.** Possible later. It needs a key over the program and the runtime build.
- **Bit sets for the reach walks, settled routines.** Measured in PR 7754: no gain.

## Consequences

Seams this ADR changes:

- **What a test session starts from** (ADR-013 decision 5, ADR-069 decisions 1 and 6): the install baseline, or a
  login start on top of it.
- **What sessions of one engine share** (ADR-001): recorded runs, kept on the engine behind a lock. The program
  and the engine's own data stay immutable.
- **`al-vm` and `al-testrun`**: recording, session image and replay live in `al-vm`. `al-testrun` owns the keys of
  the login and the checkpoint check, and hands static reach facts to `al-vm` as data (a new, defaulted engine
  field).
- **Passes** (ADR-063 decision 3, as-built point 8): a suite that differs only in a server setting is a second
  engine of a pass; a later pass may start its baseline from a checkpoint of the main pass. `work.json` pass names
  and counts change, and it gets counters for recorded runs, replays and runs not recorded.
- **`Engine::write_in_try_allowed`** becomes private behind an accessor.

Risks:

1. **An input is missing from a key.** A session then gets a result its own run would not have produced, and
   maybe no test of today shows it. Countered by decisions 2 to 5 and by the state check below, which compares the
   whole state and does not depend on a test reading it.
2. **A piece of state is missing from the image.** Same effect, same counter.
3. **The record variable image** (decision 3) is new code over a state with many parts (filters, keys, marks,
   table globals of ADR-021). An exhaustive destructuring and one unit test per part are required; any part not
   covered means "not recorded".
4. **Fewer replays apply than estimated**, because Microsoft's code leaves state the image cannot hold. The
   measuring steps show this before a phase is built.
5. **Phase 2 changes two test environments** (see there). A later case could depend on it.
6. **Phase 4 adds work to every first write** (a key lookup and a globals image). The VM budgets (`perf_vm.rs`)
   and the measuring step show whether it pays.
7. **Harder to read.** A session's start is no longer one fixed state. `--explain` and `--timings` must say which
   start a test codeunit took and why a run was not recorded.
8. **Memory.** A login start shares unchanged chunks with the baseline; at most 8 per engine.

## Conformance

- **Outcome check, per phase:** with the phase on and with its off switch, every case of every suite on BC 27, 28
  and 29 has the same outcome and message (JUnit files, `tools/compare_junit.py`, zero differences, no allow
  list). A new tool in the form of `tools/fresh_vs_grown.sh` and `tools/shared_vs_private.sh` runs it; CI runs it
  nightly. For Phases 1 and 4 also on Tests-ERM-Purchase, Tests-ERM-Sales, Tests-SCM and NP Retail, because they
  change user runs.
- **State check:** in a check mode, every replay also runs the code for real in a second session started from the
  same state. The two sessions must then be equal: the rows as bytes (`Baseline::to_bytes`) and the session image.
  A difference fails the run and names the first field that differs. This catches a missing input or a missing
  piece of state that no test reads today. For Phases 2 and 3 the same check compares the reused baseline with one
  run from the start.
- **Existing checks stay at zero differences:** `tools/baseline_rule_check.sh run`, `tools/demand_vs_eager.sh
  run`, `tools/fresh_vs_grown.sh` (its `baseline:` lines must come out the same from a reused start) and
  `tools/shared_vs_private.sh`.
- **`--baseline-diff` on BC 27, 28 and 29:** 0 regressions, 0 stale lines, 0 cases without an outcome.
- **Unit tests, red first:** a test per class of decision 2 and per image shape of decision 3; a miss test per key
  input (the rule of `run_inputs.rs`); a session that notes code never takes a replay; a run that touched a
  "must be untouched" field is not recorded.
- **A rule that stops applying is visible:** the counters for recorded runs, replays and runs not recorded are in
  `work.json`, under the work ratchet.
- **Budget:** each phase reports the BC 29 full run with the command of `perf_budget.rs` before and after.

## Decisions the owner must make

1. Accept the shared mechanism (decisions 1 to 8) and Phases 1 and 4 with their gate?
2. May the try-write app be compiled and installed in the main pass, as the OnPrem suite apps are (Phase 2)? If
   not, the suite keeps its own pass and its 19.7 G, or uses the cross-program check of Phase 3.
3. The testlibraries pass: build Phase 3, or add a per-pass term to the budget formula (option 2), or decide after
   the measuring step?
4. Add the temporary second limit that only goes down (option 4), so the budget test catches regressions again
   while this work is under way?
