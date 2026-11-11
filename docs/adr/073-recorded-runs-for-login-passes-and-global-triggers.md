# ADR-073: Run the login, the try-write baseline and the Global Triggers setup once, and replay the result

- Status: Accepted (owner, 2026-11-11): the shared mechanism and Phases 1, 2 and 4. Phase 2 only with the check
  written into it. Phase 3 is not built; the testlibraries pass gets a term in the budget. A temporary budget limit
  that only goes down. The decisions are listed at the end. Amended 2026-11-11 (see "Amendment 1"): Phases 1 and 4
  and the shared mechanism are stopped; Phase 2 is built.
- Date: 2026-11-11
- Amends: ADR-001 (what the sessions of one engine share), ADR-013 decision 5 and ADR-069 decisions 1 and 6 (what a
  test session starts from), ADR-063 as-built point 8 (the try-write suite stops being a pass).
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
step. If the run can be recorded, its result is kept per engine as a **login start**: the rows as a frozen state on
the install baseline (ADR-069 decision 6 already allows frozen states on the same base) and the session image. A later
session with the same key starts from the login start. At most 8 login starts are kept per engine, as `LoginMemo`
keeps 8 runs.

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

Decision (owner, 2026-11-11: yes, but only with the check of point 5):

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
5. **A check that no main result can change.** One function in `al-testrun` says whether an app may share a pass
   with suites that do not have it on BC. It fails when the try-write app:
   - has an event subscriber to an event published outside the app (a platform codeunit, one of Microsoft's apps,
     the main corpus app, an event of another app's table or page);
   - extends or implements an object of another app (a table, page, report, enum or permission set extension, an
     interface implementation; the `relower_set` rule of ADR-063);
   - has an object or routine in the static reach of a test codeunit of another suite of the pass;
   - has install code that, in the trace of the baseline run, wrote a table of another app, a number sequence or
     isolated storage.

   A test in `al-test-cli` runs the function on the pinned corpus and fails with the object and the rule it broke.
   The corpus run calls the same function and fails the same way, so a corpus bump that adds such a subscriber
   shows in the bump. There is no allow list. A failure is fixed by giving the suite its own pass again, never by
   an exception.

What a test can observe, and why it is the same: a try-write test sees install rows, and code behavior under the
setting. Every install step before the first one that consulted the setting did not read it, so it would have run
the same with the setting on. The steps from there on really run with the setting on. The tests run on an engine
with the setting on.

What changes in the test environment: the main app's tests now have the try-write app in their program, and the
try-write tests have the OnPrem suite apps in theirs. On BC neither job has the other's app. The main pass already
does this with the main app and the OnPrem suite apps. With the check of point 5 a main test cannot run code of
the try-write app, and reads no row its install code changed in a table it can name.

What the check cannot rule out, so the outcome differential covers it for today's cases only:

- a test that reads an object listing (AllObj, Field, the installed apps) and counts rows of every app;
- a test that opens a table or runs a codeunit by an id it computes at run time;
- the SystemId and row version counters that the app's install rows moved;
- the other direction: a try-write test that an OnPrem suite app's subscriber or extension reaches. The measuring
  step runs the same function for the OnPrem suite apps against the try-write suite. If it fails there, the owner
  gets the result before Phase 2 is built;
- the try-write app and the OnPrem suite apps do not depend on each other, so the install-order rules of ADR-046
  amendments 6 and 7 apply to one more pair of apps.

What sends the suite back to a pass of its own: the off switch, a failure of the check of point 5, or a difference
in the differential check.

Expected saving: 13 to 15 G of 19.7 G (the measurement estimated 15 G). What stays is the second engine, the
try-write app's own install code and its 33 cases.

### Phase 3: not built. The testlibraries pass gets a term in the budget (cause: 27.6 G)

Owner, 2026-11-11: do not build this phase. The cost of the pass is modeled in the budget (see "The budget").

The testlibraries pass has other dependency apps (Tests-TestLibraries, Data Archive, 178 subscribers), so it keeps
its own program, engine, install baseline and environment, as ADR-063 decided. ADR-063 decision 3 stays as it is.

What was proposed, for the record: the pass would start its install baseline from a checkpoint of the main pass's
baseline run, taken after the last install root both passes share, when everything that run consulted up to there
was the same in the later program (the routines entered, the tables, the subscribers of each event raised, every
read of a platform listing, the settings and host values).

Why it is not built:

- The saving was uncertain. The measurement estimated 10 to 15 G. The baseline was 5.1 G of this pass on
  2026-10-31, and the shared start is only part of the baseline.
- It needed the largest piece of new code: a check across two programs, and a session image of the install
  session in the middle of the dependency apps.
- Passes exist only in the corpus job. A user's `al-runner test` has one pass, so no user run would get faster.

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
saving. With Phase 3 not built, nothing needs a program identity across compiles.

### Order, measuring steps and the gate

Phases 1 and 4 each start with a measuring step: a run that records, counts how many replays would apply and why
the others would not, and still runs everything. The phase is built when that step shows a saving of at least 5 G
on BC 29. The repeat noise of the measurement is under 0.5 % (about 1.7 G); a seam change should pay at least three
times that. Phase 2 starts with the check of its point 5, in both directions.

1. The budget changes (see "The budget"): the temporary limit first, then the term for the testlibraries pass.
2. The shared module (decisions 1 to 8), with `LoginMemo` moved onto it. No saving; the WorkDate replay must still
   apply as often as before.
3. Phase 1, the login.
4. Phase 2, the try-write suite. It needs none of the image code, so it can be built in parallel with steps 2
   and 3.
5. Phase 4, the Global Triggers setup. Last: it sits in the path of every first write, and its key is the widest.

### Savings against the limit

| After | Run (BC 29) | Limit |
|---|---|---|
| PR 7754 | 333.2 G | 280.6 G from the formula; 333.2 G temporary |
| Phase 1 (12 to 14 G) | 319 to 321 G | |
| Phase 2 (13 to 15 G) | 304 to 308 G | |
| Phase 4 (8 to 9 G) | 295 to 300 G | 280.6 G plus the term for the testlibraries pass |

The term is not measured yet. One data point: on 2026-10-31 that pass cost 20.1 G with 41 tests that took 0.05 s
(ADR-063, as built). If the pass with no test still costs about that, the term is about 21 G, the limit about
302 G at today's corpus size, and the run ends 2 to 7 G under it. If Phase 4 does not pass its gate, the run stays
at 304 to 308 G, which is over that limit. The estimates of Phases 1 and 2 must also hold.

### The budget

Facts:

- The formula is a fixed part (82.8 G, the empty-corpus run of 2026-10-19 plus 5 %) plus 4.23 M per KiB of corpus
  source. It was set before any extra pass existed. It has no term for a pass.
- The two extra passes cost 47.3 G; the run is 52.6 G over the limit. A pass costs its compile growth, its
  engine, its install baseline and its listings whatever the number of cases.
- The empty-corpus run costs 68.6 G today. So the fixed part already has 10.8 G more room than "today plus 5 %".
- Without the fixed part the run costs about 5.66 M per KiB today, against 4.23 M in the formula. So a growing
  corpus uses up room unless the cost per test falls.
- The test has failed since 2026-11-06. The step that broke it was unplanned (36 to 40 G), and while it failed
  two more unplanned regressions came in (about 14 G and 29 G). All three were found only by this measurement.
- The instruction limit fails a run only where an instruction counter exists (a local run). CI has none and only
  warns.

Decided by the owner, 2026-11-11:

1. **A term per extra pass.** The limit becomes the fixed part, plus the part per KiB, plus one term per extra
   pass. The term of a pass is the cost of that pass with no test, plus 5 %. It is measured the way the fixed part
   is: the BC 29 full run with the cases of the pass deselected, minus the same run without the pass. Each pass
   has its own named constant in `perf_budget.rs`. A pass without a constant gets 0, so a new pass needs the
   owner. The term is measured again when Microsoft's build pin moves or the apps of the pass change. It goes up
   only with the owner's OK.
   - The testlibraries pass gets a term.
   - The try-write pass gets none: Phase 2 removes the pass. Until then its cost is under the temporary limit.
2. **A temporary limit that only goes down.** While the run is over the formula limit, the budget test passes
   when the run is at or under the temporary limit. It starts at today's value, 333.2 G. Each pull request that
   lowers the run lowers it to the new measured value in the same pull request. It is never raised. When the run
   is under the formula limit, the temporary limit and its code are deleted.

How the temporary limit is stored, my reading of the decision, to confirm in the pull request that adds it: as the
amount the run may be over the formula limit, 52.6 G today (333.2 G minus 280.6 G). The formula limit grows with
the corpus source, so a corpus bump does not need the temporary limit raised. When the term for the testlibraries
pass is added, the amount is lowered by that term in the same pull request, so the sum stays where it was.

What follows from it: the limit has no room at the start. New corpus source gets 4.23 M per KiB from the formula
and costs about 5.66 M per KiB today on average. Until Phase 1 or Phase 2 lands, a corpus bump can put a local run
of the test over the temporary limit. The answer then is to land the phase, not to raise the limit.

Options the owner did not take: keep the formula as it is (then the testlibraries pass would need Phase 3, and the
run would end at the limit with no margin); measure both parts of the formula again (it would accept today's cost
per test, including the causes Phases 1 and 4 remove).

### Options not chosen

- **Ask the Global Triggers setup for every table when the session starts, or start sessions with warm
  SingleInstance state.** BC does neither; a test that changes the setup before its first write would see the
  difference.
- **Replay the Global Triggers answer when the subscribers have no side effects** (static reach only). The answer
  also depends on rows, on reads through `RecordRef` and on globals (PR 7754).
- **One program for all passes, with the runtime hiding the extra apps.** Rejected in ADR-063; the reasons hold.
- **Keep the try-write suite's own program and reuse the main baseline across the two programs.** It keeps both
  environments exactly as today, but needs the check across two programs that Phase 3 would have needed, for a
  suite of 33 cases.
- **Start the testlibraries pass from a checkpoint of the main pass's install baseline** (Phase 3). Not built;
  the reasons are in that section.
- **Keep recorded runs on disk.** Possible later. It needs a key over the program and the runtime build.
- **Bit sets for the reach walks, settled routines.** Measured in PR 7754: no gain.

## Consequences

Seams this ADR changes:

- **What a test session starts from** (ADR-013 decision 5, ADR-069 decisions 1 and 6): the install baseline, or a
  login start on top of it.
- **What sessions of one engine share** (ADR-001): recorded runs, kept on the engine behind a lock. The program
  and the engine's own data stay immutable.
- **`al-vm` and `al-testrun`**: recording, session image and replay live in `al-vm`. `al-testrun` owns the key of
  the login and the check of Phase 2, and hands static reach facts to `al-vm` as data (a new, defaulted engine
  field).
- **Passes** (ADR-063 as-built point 8): a suite that differs only in a server setting is a second engine of a
  pass. ADR-063 decision 3 is unchanged: a pass with other apps reuses no runtime state. `work.json` pass names
  and counts change, and it gets counters for recorded runs, replays and runs not recorded.
- **`Engine::write_in_try_allowed`** becomes private behind an accessor.
- **The full-run budget** (`perf_budget.rs`): a term per extra pass, and a temporary limit that only goes down.
  CLAUDE.md's line on the budgets changes with it.

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
5. **Phase 2 changes two test environments** (see there). The check of its point 5 rules out subscribers,
   extensions and reachable code. A later case could still depend on what the check cannot rule out.
6. **Phase 4 adds work to every first write** (a key lookup and a globals image). The VM budgets (`perf_vm.rs`)
   and the measuring step show whether it pays.
7. **Harder to read.** A session's start is no longer one fixed state. `--explain` and `--timings` must say which
   start a test codeunit took and why a run was not recorded.
8. **Memory.** A login start shares unchanged chunks with the baseline; at most 8 per engine.
9. **The budget has little room.** With the term for the testlibraries pass the run ends near the limit only if
   Phases 1, 2 and 4 save what is estimated (see "Savings against the limit").

## Conformance

- **Outcome check, per phase:** with the phase on and with its off switch, every case of every suite on BC 27, 28
  and 29 has the same outcome and message (JUnit files, `tools/compare_junit.py`, zero differences, no allow
  list). A new tool in the form of `tools/fresh_vs_grown.sh` and `tools/shared_vs_private.sh` runs it; CI runs it
  nightly. For Phases 1 and 4 also on Tests-ERM-Purchase, Tests-ERM-Sales, Tests-SCM and NP Retail, because they
  change user runs.
- **State check:** in a check mode, every replay also runs the code for real in a second session started from the
  same state. The two sessions must then be equal: the rows as bytes (`Baseline::to_bytes`) and the session image.
  A difference fails the run and names the first field that differs. This catches a missing input or a missing
  piece of state that no test reads today. For Phase 2 the same check compares the second engine's baseline with
  one run from the start under the setting.
- **The check of Phase 2, point 5:** the `al-test-cli` test on the pinned corpus, and the same function in the
  corpus run. Unit tests, red first, with one fixture app per rule (a subscriber to an outside event, an
  extension, an object in another suite's reach, install code that writes another app's table).
- **Existing checks stay at zero differences:** `tools/baseline_rule_check.sh run`, `tools/demand_vs_eager.sh
  run`, `tools/fresh_vs_grown.sh` and `tools/shared_vs_private.sh`.
- **`--baseline-diff` on BC 27, 28 and 29:** 0 regressions, 0 stale lines, 0 cases without an outcome.
- **Unit tests, red first:** a test per class of decision 2 and per image shape of decision 3; a miss test per key
  input (the rule of `run_inputs.rs`); a session that notes code never takes a replay; a run that touched a
  "must be untouched" field is not recorded.
- **A rule that stops applying is visible:** the counters for recorded runs, replays and runs not recorded are in
  `work.json`, under the work ratchet.
- **Budget:** each phase reports the BC 29 full run with the command of `perf_budget.rs` before and after, and
  lowers the temporary limit to the new value in the same pull request. `perf_budget.rs` gets unit tests for the
  formula with a pass term and for the temporary limit (a run over it fails; a pass without a constant gets 0).

## Owner decisions (2026-11-11)

1. The shared mechanism (decisions 1 to 8) and Phases 1 and 4, each with its measuring step and gate: accepted.
2. The try-write app compiled and installed in the main pass (Phase 2): yes, but only with a check that fails when
   the try-write app has event subscribers or objects the main tests can reach, so that no main result can change
   (Phase 2, point 5).
3. The testlibraries pass: a per-pass term in the budget, the cost of the pass with no tests plus 5 %. Phase 3 is
   not built.
4. A temporary budget limit at today's 333 G that may only go down, removed once the run is under the real budget:
   yes.

## Amendment 1 (2026-11-11): Phases 1 and 4 stopped, Phase 2 as built

Owner decisions, 2026-11-11, after the measuring step of Phase 1.

### Phases 1 and 4 and the shared mechanism are stopped

The measuring step of Phase 1 ran on BC 29 (full run, `perf stat -e instructions:u`):

| Run | Instructions | Company-open step of the main pass (3,820 sessions) |
|---|---|---|
| Recorded runs off, two runs | 335.19 G, 335.46 G | runs in every session |
| Recorded runs on, as built | 336.64 G | 0 recorded: the session image refuses |
| On, with the image's blockers skipped (a throwaway build), the corpus clock | 379.86 G | 2,575 recorded, 1,244 replayed |
| Same, with every test codeunit's clock at noon UTC of its oracle day | 326.19 G (off: 336.76 G) | 12 recorded, 3,807 replayed |

The gate (a saving of at least 5 G) is not met, and the session image cannot meet it alone:

- The corpus job starts each test codeunit at the second of its suite in the oracle JUnit file (`oracle.rs`
  `parse_clocks`). Microsoft's company-open code reads the clock to the millisecond (the system stamps of the rows it
  writes, `CurrentDateTime` in the database trigger setup), so the clock position is rightly part of the key, and only
  sessions that start in the same second share a recorded run: 1,244 of 3,820. The other 2,575 each record a run, which
  costs more than a replay saves (+44.7 G).
- The image also fails on what Microsoft's login leaves: temporary record globals of SingleInstance codeunits, filled
  lists, untouched codeunit variables, and an internal .NET object.

Phase 4 asks through the same database trigger setup, which reads the clock, and through the same temporary record
globals; by its own rule ("did not read the clock") no ask would be recorded. So Phase 4 is stopped too, and with
both phases stopped the shared mechanism (decisions 1 to 8 of "The rule every phase follows") has no user. The work is
kept on the branch `adr073-phase1` and is not merged. `install::LoginMemo` stays as it is.

### Phase 2 as built

The try-write suite runs in the main pass. What was built, against the decision of Phase 2:

1. **Passes** (`al-test-cli` `main.rs` `passes_with`). A suite that differs from a pass only in its server settings
   runs in that pass. The off switch `AL_SERVER_SETTINGS_OWN_PASS=1` gives such a suite a pass of its own again, as
   before. `work.json` has no try-write pass any more.
2. **Engines** (`al-testrun` `runtime/setting_lanes.rs`). The run's own engine runs every suite with the run's settings
   (lane 0). The suites whose job sets DisableWriteInsideTryFunctions = false (`RunOptions::write_in_try_suites`) run on
   a sibling engine with the setting (lane 1): it shares the program and the compiled code. Each lane runs its test
   codeunits through the same walks over its own engine and install baseline. Both lanes' tests engines get the unseeded
   tables, the demo tables and the company refusal, also with one test thread and when the install baseline is skipped.
3. **Which apps each lane has, built differently from point 4 of the decision.** An app is not installed for a lane when
   only the other lane's suite apps need it: the dependency closure of the other lane's suite apps less the closure of
   its own. Lane 0 leaves the try-write app uninstalled; lane 1 leaves the OnPrem suite apps uninstalled, so its tenant
   has the apps of the try-write suite's own pass (the main corpus app, the try-write app and their dependencies). One
   owner says whether an app is installed: `al_vm::Engine::app_installed`. Its install code does not run; its event
   subscribers (manual ones too) and table extension triggers do not run; the listings (AllObj, AllObjWithCaption,
   Field, Key, the metadata tables, the permission sets, the profiles, the installed and published apps) have no row
   of it; `NavApp.GetModuleInfo` does not find it; and a lookup by an id known at run time (`Codeunit.Run`, `Page.Run`,
   `Report.Run`, an XmlPort or Query by id, `RecordRef.Open`) does not find its objects. The platform listings are kept
   per set of uninstalled apps. This answers the reverse direction of "What the check cannot rule out": the OnPrem
   suite apps have manual subscribers on table User (TestUserDeleteCascadeOrder.al:134), so the check of point 5 run
   for them against the try-write tests would fail rule 1; lane 1 does not install them, and none of their code runs
   there. Lane 1 runs the whole install baseline once more, under its own setting. The capture of point 4 is not
   built: lane 1's baseline is the reference that a capture would have to equal, and it costs about 2 s of wall time.
   Point 3 (a private `Engine::write_in_try_allowed` that notes when a session reads it) is not built, because only the
   capture needed that note. Lane 1's baseline notes, assumptions, install orders, rows (in `work.json`) and what its
   engines met that the compile left out are merged into the run's.
4. **The check of point 5** (`al-testrun` `shared_pass.rs` `check`). Rule 1: an event subscriber to an event published
   outside the app. Rule 2: an extension of another app's object (table, page, report, enum, permission set, profile),
   or an object of a kind the program does not model (a page customization). Rule 3: a routine or table of the app in
   the joined static reach of every object of the other suites' apps (test codeunits, their helpers, install code),
   which follows interface calls, so an implementation of an outside interface counts here. Rule 4 (install code that
   writes another app's table, a number sequence or isolated storage) has nothing to check: the app's install code never
   runs on lane 0. The reach the walk cannot follow (code of a dependency app that was not compiled, an object run by an
   id known only at run time; today the first is `Forward Link Mgt..OnLoadHelpArticleCodes`, not compiled) is printed,
   not taken as a breach: such code reaches the app only through one of its subscribers or by an id known at run time,
   and lane 0's engine runs none of its subscribers and finds none of its objects by such an id (point 3). The corpus
   run calls the check after the install baseline and prints a line per suite; a breach marks the suite's tests not run
   with the reason and fails the run (exit 2). `corpus_harness.rs`
   `the_try_write_suite_shares_the_main_pass_on_every_version` asserts the line on BC 27, 28 and 29. There is no allow
   list.
5. **The listing difference the check cannot rule out happened once.** Before the installed-apps listing left out the
   uninstalled app, `R7169_Test_ReadsPublishedAsOfDependency` (record/lt-r7142-installed-app/R7169DepInstallPublishedAs.al)
   counted the apps of the publisher "AL Language" and got one more (a failing case either way; its message changed).
   With every listing reading `Engine::app_installed`, the JUnit differential is zero.
6. **The differential as a tool** (`tools/shared_vs_own_pass.sh`, in CI's nightly set): one run per version with
   `AL_SERVER_SETTINGS_OWN_PASS=1` against the normal run, every suite, outcomes and messages.

Measured on BC 29 (full run, `perf stat -e instructions:u`, corpus `f2d5ea3`): main 336.62 G, with Phase 2 322.87 G
and 323.10 G. Saving: 13.75 G of the 19.7 G the pass cost. The conformance checks of Phase 2: the JUnit files of
every suite on BC 29 equal main's, with messages (`tools/compare_junit.py --reasons`, 0 differences);
`--baseline-diff` on BC 27, 28 and 29 shows no regression, stale line or case without an outcome;
`tools/baseline_rule_check.sh run` and `tools/demand_vs_eager.sh run` show no difference.

### The budget as built

- The testlibraries pass has its term, 13.97 G (the pass with no test cost 13.30 G, plus 5 %). A pass without a
  constant gets 0. The try-write suite has no pass and no term.
- The temporary allowance over the formula limit is stored as the amount over it, as proposed under "The budget". It
  started at 52.6 G, became 38.63 G when the testlibraries term was taken off it, and is 27.92 G after Phase 2 (the run
  at 322.87 G minus the formula limit at 294.96 G, rounded up to 10 M).

### Open: the cost of the per-lane listings (fix round 2, 2026-11-11)

With every listing reading `Engine::app_installed` (point 3), the platform listings are kept per set of uninstalled
apps, so lane 1 builds again every listing its install code and tests read. Measured on BC 29 (three runs,
`perf stat -e instructions:u`): 325.73 G, 326.60 G, 326.68 G (noise 0.95 G). The same build with the listings shared
across lanes as before: 322.87 G. The run is 30.77 G over the formula limit, above the 27.92 G allowance, which stays
as it is (it only goes down). Not merged until this is solved: a listing built once over every app, with the rows of an
app the engine does not have left out when a lane reads it, would keep both the sharing and the one owner. Rule 3's
wider roots (every object of the other suites' apps) cost nothing measurable (326.64 G with only test codeunits).

### Savings against the limit, now

With Phases 1 and 4 stopped, the run stays about 28 G over the formula limit (322.9 G against 295.0 G). The next
saving has to come from other work: the login decision's reach walk (about 10 G), the platform listings of the
testlibraries pass, or a capture for lane 1's install baseline.

