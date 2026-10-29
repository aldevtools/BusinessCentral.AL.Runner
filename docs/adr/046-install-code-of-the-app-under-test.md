# ADR-046: The install code of the app under test runs in the install baseline; several companies only as an opt-in

- Status: Accepted; amendment 1 Accepted (owner, 2026-10-19); amendment 2 (step B as built, 2026-10-29) decided without owner review
- Amendment 4 (2026-10-30): the default mode models a database with one company (Accepted, owner, 2026-10-30); what
  "My Company" holds under the opt-in (built in parts, see the section)
- Amendment 3 (2026-10-29): probe Q1 landed. Accepted under this ADR's own rule (A2, "a later amendment replaces the refusals a to d with that order"), for the part the probe pins
- Date: 2026-10-19 (accepted by the owner, 2026-10-19, with the `--companies` flag); amendment 1 2026-10-19
- Amends: ADR-013 decision 5 (items 1 and 2) and its section "Install baseline and Global Triggers as built"
- Amendment 1 (Accepted by the owner 2026-10-19, section "Amendment 1" below) replaces the object-order rule of decision 3 and decision 8 with
  what the corpus probes P1 and P3 to P8 pinned. Where the two disagree, amendment 1 wins.

## Context

Owner decisions (2026-10-19):

- The install baseline runs the install triggers of the app under test, the way real BC runs them when the app is
  installed.
- The runner runs **one company by default**. It never runs the install baseline again for other companies of the
  database, whatever the database holds.
- **Several companies are an opt-in**, for partner tests that rely on several companies. Under the opt-in the app under
  test's install code runs once per company, session company first, then the per-database part once.
- The opt-in is one flag that lists the companies to run, `--companies <NAME>[,<NAME>...]`, or `--companies all` for
  every company in the database. Without the flag the run has one company, the session company.

This ADR says what runs in each mode, how a test that observes another company is handled in the default mode, how
the corpus run gets the opt-in, and what still needs a corpus probe before it is built.

"App under test" means the apps of the workspace (`lowered.dependency[oi]` is false): the corpus apps in a corpus
run, the APPs given to `alrunner test`. In a run of NP Retail's tests, NP Retail itself is a dependency app and only
its test app is under test.

### What real BC does (pinned by the corpus)

All file references are in the corpus submodule at `db082fc`, under `tests/al-language/` unless a path says otherwise.
Both tests below pass on real BC 27, 28 and 29 (`oracle/27|28|29/junit.xml`, codeunit 69852).

1. **An ordinary codeunit that subscribes to "Extension Triggers" runs at install.** Codeunit 69851 "LT V Subs" has no
   `Subtype` (`events/lt-platform-events-v/PeVFixtures.al:222`). It subscribes to `OnInstallAppPerCompany` and
   `OnInstallAppPerDatabase` of codeunit "Extension Triggers" (`PeVFixtures.al:395-407`). Each subscriber writes a row
   into table 69840 "LT V Event Log" (`DataPerCompany = false`, `AutoIncrement` Id: `PeVFixtures.al:9-23`, written by
   `WriteRow` at `:237-246`). A test finds those rows (`events/lt-platform-events-v/PeVTests.al:284-285`).
2. **The per-company part runs once in each of two companies.** There are 2 `OnInstallAppPerCompany` rows
   (`PeVTests.al:284`). Each row holds `CompanyName()` as the subscriber saw it (`PeVFixtures.al:398-399`). Row #1
   holds the company of the test session, row #2 another company (`PeVTests.al:313-314`). So the corpus CI database
   has two companies, and the install code runs in each with `CompanyName()` set to that company.
3. **The per-database part runs once** (`PeVTests.al:285`).
4. **Order: the session company, then the other company, then the per-database part.** `Verify` compares the
   observations as an ordered list and fails on a missing or an extra one (`_fixtures/ProbeObserver.Codeunit.al:156-189`).
   So the two per-company rows have the Ids 1 and 2, and with 3 rows in all (`PeVTests.al:298`) the per-database row
   has Id 3. AutoIncrement gives each insert the next number (`record/TestAutoIncrementExplicitKey.al:50-51`). This is a
   corpus pin, for subscribers of one app, of the order the probe agent measured for install codeunits
   (`docs/probes-needed.md:28`, not a corpus test).
5. **No upgrade, environment-copy or user-initialization event runs at install** (0 rows each, `PeVTests.al:286-296`).
6. **The test session does not see the SingleInstance state of the install run.** The subscribers also add to a list
   in a SingleInstance codeunit (`PeVFixtures.al:133-135`, `:248-252`). In the test session that list holds only the
   two Company Triggers entries (`PeVTests.al:297`).
7. **The test company is "CRONUS International Ltd."** (`session/TestEnvironmentUserTenantContracts.al:253`).
8. **For install codeunits (already pinned):** the per-company trigger runs in the test company and finds its Company row
   (`install/TestInstallEnvVisible_Tests.al:39-58`); the test company holds exactly 2 per-company seed rows and the
   database exactly 1 per-database seed row (`install/TestInstallTriggerSeed_Tests.al:58-59`). Two companies do not
   change these: "Install Seed" is per company and "Install Seed Database" is written only by the per-database trigger.

### What the runtime does today

- `baseline.rs` runs, once per run: each dependency app's install codeunits (all per-company triggers in the session
  company, then all per-database triggers with no company, per app, `baseline.rs:112-127`), then `Company-Initialize`,
  then the workspace apps' `Subtype = Install` codeunits (`baseline.rs:210`, through `run_install_codeunits`,
  `runtime/blocked.rs:8`). Without a baseline (skipped by the rule, `AL_NO_BASELINE`, or no dependency root,
  `baseline.rs:139`) the workspace installs run per test codeunit instead (`runtime/run.rs:229-237`).
- Everything runs in one company. The Company table holds one row. A read of the Company table, or a call that names
  another company, is answered as if the database held only the session company and the companies the run created.
- Nothing raises the events of "Extension Triggers" (2000000010). Its subscribers never run.
- `run_install_codeunits` groups all workspace apps together: every per-company trigger of every workspace app, then
  every per-database trigger. Dependency apps are handled per app.

Result on BC 29 today: `EventLog_InstallRowCompany` fails ("observation 1: expected [install per company #1 info is the
company of the test session = Yes] but was []") and `EventLog_RowsWrittenBeforeTheTestSessionStarted` fails ("expected
[rows OnInstallAppPerCompany = 2] but was [rows OnInstallAppPerCompany = 0]").

## Decision

1. **Install code of an app** is its `Subtype = Install` codeunits' `OnInstallAppPerCompany` and
   `OnInstallAppPerDatabase` triggers, and its subscribers to the same two events of "Extension Triggers" (fact 1). This
   holds for every app. Microsoft's apps have no such subscribers outside install codeunits (see Cost), so for them
   nothing changes. A look-alike procedure on a normal codeunit is still not install code
   (`install/TestInstallTriggerSeed_Tests.al`, `TestInstall_NonInstallCodeunitDidNotAutoRun`).

2. **Two company modes.**
   - **One company (the default).** The run has one company, the session company. The install baseline never runs for
     another company, whatever the database holds; a test-data backup with several companies (the demo backup holds
     "My Company" too, `docs/probes-needed.md:215`) is no exception.
   - **Several companies (opt-in).** One flag, `--companies <NAME>[,<NAME>...]`, on `alrunner test` and `al-test-cli`
     (later `companies` in `alrunner serve`, ADR-028). It declares the complete company list of the database, in run
     order. Two forms:
     - **A list of names.** The first name is the session company. The session company is still chosen as today (the
       company restored from the test data, `--test-data-company`, the recipe lane, or the default name); the first name
       must be that company, or the runner stops with a usage error before anything runs. The other names follow in the
       order given. A name given twice is a usage error. A name with a comma cannot be given (none is known to be
       needed).
     - **`all`.** Every company in the database the run starts from: the session company first, then the others in the
       Company table's key order (the order of the opt-in is probe P2). The database the run starts from is the
       test-data backup; a run without test data (`--no-test-data`, the corpus run) starts from an empty database, so
       there `all` gives only the session company, the same as the default.

     The resolved list is a host fact next to the company name (`al-host` `Identity`, an added method whose default is
     the empty list, meaning one company) and part of the baseline cache key, next to `company_name`
     (`runtime/corpus.rs:1151-1152`).

3. **The steps of the install baseline**, once per run, on one fresh session:

   | Step | What runs | One company (default) | Several companies (opt-in) |
   |---|---|---|---|
   | 1 | Seed the platform rows (ADR-013 decision 5 item 2): the Company rows, the User, the SUPER Access Control row | one Company row | one Company row per declared company |
   | 2 | Each dependency app, in dependency order: its per-company install code, then its per-database install code (unchanged) | session company; per-database with no company | the same: session company only (decision 5) |
   | 3 | `Company-Initialize` (unchanged) | session company | the same: session company only |
   | 4a | Each app under test, in dependency order, one app complete before the next: its per-company install code | session company | once per declared company, in list order (session company first) |
   | 4b | The same app's per-database install code, after 4a | once, with no company | once, with no company |
   | 5 | Capture the baseline (ADR-004), as today | | |

   `CompanyName()` is the company being installed in 2, 3 and 4a, and empty in the per-database part. Within 4a and 4b
   the routines run in object order, as today (the order between an install codeunit's trigger and a subscriber is
   probe P6; **replaced by amendment 1, A2**: the order is not object order, and a test that can see it is refused).
   Each routine commits when it ends (as today). Step 4 changes today's grouping of all workspace apps into
   one per app.

4. **In the default mode, a test that observes another company is refused at run time** (`Signal::Unsupported`, with
   the reason), never answered. "Known companies" are the session company and the companies the run itself inserted,
   renamed or deleted in the Company table (they are handled as today, `docs/probes-needed.md:201`). Three things
   show another company:
   - **a. The Company table.** A `Get` of a known company, and a read whose filters select one known name (a single-value
     filter on `Name`), are answered. Every other read is refused: a `Get` of another name, `Find*`, `Next`, `Count`,
     `IsEmpty`, `CalcSums`, and a FlowField or `TableRelation` check that reads the table. The database may hold more
     companies than the run knows.
   - **b. A call that names a company that is not known.** `Record.ChangeCompany`, `RecordRef.ChangeCompany`,
     `RecordRef.Open` with a company, and every other builtin whose answer depends on whether a named company exists.
     Today such a company is answered as missing. That answer is a guess, because the database may hold it.
   - **c. A shared table that per-company install code of an app under test writes.** On BC it holds one write per
     company of the database, here one. The tables are found before anything runs: the static closure of the per-company
     roots of step 4a (`tables_written`), tables with `DataPerCompany = false` only. They join `Engine::unseeded`, so a
     read whose answer depends on the rows is refused at run time (the existing rule: `Find*`, `Next`, `Count`, `IsEmpty`,
     `CalcSums`, a FlowField, a `TableRelation` miss, a `Get` that finds nothing; a `Get` that finds its row is
     answered). In the corpus this is "LT V Event Log" only, so the two PeV tests are not run in the default mode.

   Why at run time, not statically: the table list of item c is static, but the read decides. A static refusal of every
   test that reaches the Company table or one of those tables would also refuse correct work, such as
   `Company.Get(CompanyName())` (`session/lt-company-record/CompanyRecord.al:20`) or an insert into a shared table. The
   runner reports a refused test as not run, with the reason.

   Under the opt-in the declared list is complete: items a and b are answered from it (a name that is not on it does
   not exist), and item c does not apply, because step 4a ran once per declared company.

   **Not decided:** dependency apps' per-company install code also writes shared tables. By the reasoning of item c it
   shows another company too. Step A counts these tables (`tables_written` over the dependency roots, shared tables
   only) and the Microsoft tests a refusal would stop, and reports both numbers to the owner, who decides.

5. **Under the opt-in, dependency apps stay in the session company.** What another company holds from the dependency
   apps and `Company-Initialize` depends on how and when that company was created, which no corpus test pins (probe
   P3). So in a declared company other than the session company:
   - a per-company table of an app under test is answered normally. It is new at that app's install, so its rows are
     exactly what install code and tests wrote there (consistent with the exact counts at
     `install/TestInstallTriggerSeed_Tests.al:58-59`);
   - a read or write of any other per-company table (dependency tables, per-company platform tables, and the rows a
     test-data backup holds for that company) raises `Signal::Unsupported` naming the table and the company;
   - shared tables behave as today.

6. **A root that stops** is rolled back, as today. Under the opt-in, a stop in another company touches only that
   company's rows and the shared tables. The tables the root would have written (`tables_written`) become unseeded per
   company: a per-company table only in the company where the root stopped, a shared table everywhere.
   `Engine::unseeded` gets a company in its key (an internal change).

7. **One function runs step 4** for both paths: the baseline (after step 3) and the run without a baseline
   (`runtime/run.rs:229-237`). A skipped baseline and a run baseline give the same rows. This is already true for the
   install codeunits; the subscribers and the opt-in companies join that function.

8. **One session for every root**, as today; the test session does not see its SingleInstance state (fact 6). Whether
   BC uses one session per company is probe P8. **Replaced by amendment 1, A4**: one session per app, and its
   SingleInstance state starts again in each company and in the per-database part.

### How the corpus run gets the opt-in

The corpus CI database has two companies (facts 2 and 4). It is a fact of the oracle's run, like the Microsoft build
of each major. So `oracle/oracle.toml` gets a top-level `companies` key, set at the corpus bump that brings probe P1
and citing it, in the same way as `bc_build`. It takes the same values as the flag: a list of names, session company
first (`companies = ["CRONUS International Ltd.", "My Company"]` if P1 confirms those names), or `"all"`. Because the
corpus run starts from an empty database, `"all"` gives the corpus only the session company; the corpus needs the
list of names. A `[[version]]` table may override the key if P1 shows that the majors differ. `al-test-cli` runs every
suite of the corpus job with that value as the opt-in; `--companies` on the command line overrides it. The OnPrem
suites run in separate CI jobs of the same artifact (`.github/workflows/ci.yml` in the corpus, jobs `test-onprem` and
`test-onprem-dotnet`); P1 checks their databases too.

Until P1 lands, `companies` is absent and the corpus runs in the default mode: the two PeV tests are not run (item 4c).

The default mode also needs coverage. A new check, `tools/company_mode_check.sh run`, runs the corpus with the opt-in
(the `companies` value of `oracle.toml`) and again with one company (the run ignores `companies`), and compares the JUnit files (like `tools/baseline_rule_check.sh`). A case may differ only by
being not run in the default mode. A case that passes or fails differently is an error. This shows that the default
mode refuses what it cannot know and never answers it differently.

### Build order

Each step is built only after the probes it names have landed and the submodule and oracle are bumped (owner,
2026-10-19).

- **Step A** (waits for P4, P6, P7): decision 1 for every app, step 4 per app in the default mode, decision 7, item
  4c, the two consumers below (demand roots, impact points), and the measurement under "Not decided". Expected: no
  baseline case lost; the two PeV tests become not run.
- **Step B** (waits for P1 and P8): the opt-in (`--companies`, the `Identity` method, `companies` in
  `oracle.toml`), step 4a per declared company, decisions 5 and 6, the default-mode refusals 4a and 4b, and
  `tools/company_mode_check.sh`. Items 4a and 4b come only together with the corpus opt-in, so the corpus cases that
  read the Company table or name a company keep passing. Two baseline cases of that kind: `Company_Names` counts the
  Company table (`session/TestProbeSessionMiscContracts.al:705`), and `TestInstall_CompanyGetConsultsTheKeyDuringInstall`
  reads what the install code's `Comp.Get('NO SUCH COMPANY')` answered (`install/TestInstallSeeder.al:72`,
  `install/TestInstallEnvVisible_Tests.al:64`). Expected: `EventLog_InstallRowCompany` passes on 27, 28 and 29.
- **Interim, until step A (2026-10-20):** replaced by step A as built (section "Step A as built" below).
- **Not in this ADR:** `EventLog_RowsWrittenBeforeTheTestSessionStarted` also needs Company Triggers `OnCompanyOpen`
  and `OnCompanyOpenCompleted` raised at the start of the test session (`PeVTests.al:297`). No code raises them today.
  That is an additive change with its own pins.

## Fit with the install-baseline skip rule

- **The rule needs no new case.** `observe.rs` skips the baseline only when no selected test can read the data. All new
  data (rows of the apps under test, the Company rows of the opt-in, the AutoIncrement counter of a shared table, the
  unseeded set) and every refusal of decision 4 is reached only through a record, `RecordRef`, query, page, report or an
  event publisher. Each of these already says "may observe". `CompanyName()` still answers the session company. The
  module comment ("the data a session holds after the dependency apps' install code ran") is updated to name the apps
  under test and the opt-in companies.
- **The skip path stays correct** because of decision 7: when the baseline is skipped, step 4 runs per test codeunit
  through the same function.
- **Checks after each build step:** `tools/baseline_rule_check.sh run` shows 0 differences (CLAUDE.md),
  `tools/demand_vs_eager.sh run` shows no difference, and after step B `tools/company_mode_check.sh run` passes.
- **Demand roots.** Workspace objects are checked anyway. A dependency app's "Extension Triggers" subscribers become
  roots in `reach_baseline` (`al-sema/src/check/demand.rs:351-377`) next to its install triggers. No Microsoft app has
  one, so the Microsoft compile does not change.
- **Impact selection (ADR-043).** `impact_static.rs:885` and `impact_trace::impact_points` find install code by trigger
  name in install codeunits. They must also list "Extension Triggers" subscribers, because a partner extension can
  install through them.

## Cost

**Microsoft app runs (default mode).** Measured on 2026-10-19 with a scan of every `*.Source.zip` (206 apps) under
`Applications/` of the BC 27.5.46862.48827 sandbox artifact:

- No Microsoft app subscribes to "Extension Triggers" outside an install codeunit. Decision 1 adds no root to a Microsoft app.
- The 35 BaseApp test apps (`Applications/BaseApp/Test/Tests-*`) have no install code. In those bucket runs step 4 runs
  nothing: no compile cost, no added baseline time. Today the baseline takes 0.08 to 0.15 s per run in the bucket logs
  (for example "16 of 17 install roots ran to their end; 14 tables hold rows, 1 are unseeded, 0.08 s"), against about
  300 s of tests.
- Five Microsoft test apps have one per-company install trigger each (Bank Account Reconciliation With AI Tests,
  Business Foundation Tests, E-Document Core Tests, Sales Lines Suggestions Tests, Sustainability Copilot Tests). In the
  default mode they run in the session company, as today.
- The refusals 4a and 4b can turn Microsoft tests that scan the Company table or name another company into not run.
  The buckets are run before and after step B, and the pull request lists every changed outcome. A recipe lane can use
  the opt-in (`--companies` with a list, or `all` on a lane that restores a backup) once its companies are known (P9).

**Partner runs.** NP Retail's 14 install codeunits are dependency roots in its run: unchanged. Its test app (1 install
codeunit) runs as today in the default mode. Under the opt-in, each per-company routine of the apps under test runs once
more per declared company; dependency apps do not.

**Corpus (opt-in from step B).** The corpus app has six per-company install routines (five install codeunit triggers
and the PeV subscriber); each runs once more, in the second company. The baseline cache (ADR-022) keeps the result per
key. `tools/company_mode_check.sh` adds one corpus run in the default mode.

## Not pinned yet: probe questions

Corpus rule for all install probes: install code must never fail, or the app does not install on any BC version
(`.github/scripts/check-install-triggers.py:1-7`). A probe writes with `if Insert() then;` into a `DataPerCompany = false`
log and a test reads the log, as `PeVFixtures.al:237-252` does. New probes use "Extension Triggers" subscribers; a new
`Subtype = Install` codeunit needs a change of the `ALLOWED` list (`check-install-triggers.py:13`). Corpus issue 647
items 1 to 3 ask related questions for install codeunits.

- **P1. Which companies does the corpus CI database have?** Their number, `Name` and `Display Name`, in the Company
  table's key order, on BC 27, 28 and 29, in the main job and the two OnPrem jobs; check the Windows nightly too.
  Evidence so far, not a pin: the probe agent measured "CRONUS International Ltd." then "My Company"
  (`docs/probes-needed.md:29`), and the demo backup holds a second company "My Company" (`docs/probes-needed.md:215`).
  Decides: the list of names in `companies` in `oracle.toml`. Blocks step B.
- **P2. Under the opt-in, in which order do the companies get the per-company install code: the session company first,
  or the Company table's key order?** In the corpus CI the session company also sorts first by name, so
  `PeVTests.al:313-314` cannot tell the two apart. The corpus CI cannot vary its companies, so this needs another CI setup
  (a test company that does not sort first), or stays a documented assumption. Does not block A or B.
- **P3. What does the second company hold before the corpus app's install code runs?** From a test: `ChangeCompany` (or
  `RecordRef.Open(.., false, Company)`) to the other company and count the rows of Company Information (79), General
  Ledger Setup (98) and one per-company table that System Application's install code fills. Inside the per-company
  subscriber: the same counts, logged when it runs in the second company. Decides whether decision 5 can answer
  dependency tables in other companies (Options, item 2).
- **P4. Does an "Extension Triggers" subscriber run only at the install of its own app, or also when a later app is
  installed?** Put subscribers into `al-language-internals-fixture` (published before `al-language`, corpus `ci.yml:207-208`)
  that log into a table of that app; a test in `al-language` counts the rows. 2 + 1 means its own install only. Blocks
  step A.
- **P5. Order across apps:** do all companies and the per-database part of the fixture app's install end before
  `al-language`'s install begins? The same probe as P4, with both apps writing one shared AutoIncrement log (al-language
  depends on the fixture app, so it can write the fixture's table). Confirms step 4's "one app complete before the
  next".
- **P6. Order within one app** between an install codeunit's `OnInstallAppPerCompany` trigger and an "Extension
  Triggers" subscriber, and among two subscriber codeunits (object id order or another order). Both write one shared
  AutoIncrement log. Blocks step A.
- **P7. What does `CompanyName()` answer inside an "Extension Triggers" `OnInstallAppPerDatabase` subscriber, and what
  does a per-company table access there do?** `PeVFixtures.al:405-406` writes a constant, not `CompanyName()`. For install
  codeunits it was measured (empty; `docs/probes-needed.md:28`), not pinned; corpus issue 647 item 1 asks for the error
  text. Log `CompanyName()` into `Info`; wrap the table access in a TryFunction and log `GetLastErrorText` and
  `GetLastErrorCode`. Blocks step A.
- **P8. Session facts of the install run.** Do the three rows carry the same `SessionId()`? The column is written
  (`PeVFixtures.al:244`) but no test asserts it; `RowsOfSession` (`PeVTests.al:250-257`) is not used. Does a SingleInstance
  codeunit keep its state from the session company's run into the second company's run and the per-database run (log
  `Log.Early()` into `Info`)? What does `Session.GetExecutionContext()` answer inside a subscriber (pinned only for install
  codeunit triggers, `install/TestInstallEnvVisible_Tests.al`)? Blocks step B.
- **P9. Which companies exist in each lane of Microsoft's CI** (Legacy, UnitTest, IntegrationTest, Uncategorized)? Not
  a corpus probe: read it from BCApps' AL-Go setup or a run on a Windows container. Until then the recipe lanes run in
  the default mode.

## Amendment 1 (Accepted, 2026-10-19): what the probes pinned

Owner decisions, 2026-10-19:

1. Amendment 1 is accepted.
2. Company Triggers in the install session (A4 item 4): raise `OnCompanyOpen` and `OnCompanyOpenCompleted` to the
   subscribers of EVERY installed app now. This is a flagged assumption (CLAUDE.md rule 3), registered in the runtime
   assumption registry when built, id `install_company_triggers_all_apps`. Probe Q3 confirms it later; if Q3
   disagrees, Q3 wins and the code changes.
3. Dependency apps (Microsoft's apps) get the same rule as the app under test (A2): the order is unknown, so a test
   that can observe it is refused, whatever this costs. Step A still reports the count of affected routine pairs and
   Microsoft tests.

Corpus writer BF answered P1 and P3 to P8 (corpus issue 661). The tests pass on real BC 27.5, 28.5 and 29.0, 7 of 7
per leg (corpus branch `al-runtime-rs-staging-bf` at `3a91496`, CI runs 39227416374, 39227450187, 39227481098). The
results are the same on all three majors, except two Base Application row counts (`#if BC28PLUS`). The submodule and
oracle bump that brings these files is a precondition of every build step below.

File names used in this section, all in the corpus:

- `BfTests` = `tests/al-language/install/lt-bf-install-probes/BfInstallTests.al` (codeunit 81257)
- `BfFix` = `tests/al-language/install/lt-bf-install-probes/BfInstallFixtures.al`
- `BfDep` = `tests/al-language-internals-fixture/LTBFInstallSubs.al` (in the fixture app, which the main app depends on)
- `BfOpen` = `tests/al-language/events/lt-bf-company-open/BfCompanyOpenTests.al` (fixtures in `BfCompanyOpenFixtures.al`)

"Install routine" means one `OnInstallAppPerCompany` or `OnInstallAppPerDatabase` trigger of an install codeunit, or
one subscriber to those events of "Extension Triggers". "Part" means the per-company code in one company, or the
per-database code.

The probe app: three install routines of the main app log every call into the shared AutoIncrement table "LT BF
Install Log" (`BfFix:16-36`): "LT BF Z Subs" (81253, subscriber, `BfFix:149-165`), "LT BF Install Late" (81254, install
codeunit, `BfFix:167-184`) and "LT BF A Subs" (81255, subscriber, `BfFix:186-202`). Each row holds `CompanyName()`,
`SessionId()`, the execution context, a call counter of a SingleInstance codeunit, the fixture app's SingleInstance
counter, and how many Company Triggers calls the session saw (`BfFix:66-86`). The fixture app has one subscriber
codeunit that logs the same way into its own table (`BfDep:28-55`). `BfTests:196-241` sorts the rows that all install
code wrote by `SystemRowVersion`, one counter for the whole database, which gives the run order across tables.

### A1. Pinned facts

1. **Companies (P1).** The corpus CI database has two companies, in key order "CRONUS International Ltd." (the
   session company, `Evaluation Company` = Yes) and "My Company" (`Evaluation Company` = No). `Display Name` is empty for
   both (`BfTests:47-64`). Checked in the main job only; the OnPrem jobs were not probed.
2. **What "My Company" holds (P3).** No Company Information (79), General Ledger Setup (98) or Source Code Setup (242)
   row; 2 Report Selections (77) rows; No. Series (308) 0 rows on 27 and 1 row on 28 and 29. The session company holds
   1, 1, 1, 70 or 71, 118 or 119 (`BfTests:74-95`). The counts inside the app's own install code are the same
   (`BfTests:125-135`, the `Info` column). So `Company-Initialize` did not run in "My Company", but some dependency code
   did write rows there.
3. **Subscribers run only at their own app's install (P4).** The fixture app's subscriber wrote 3 rows: session company,
   other company, per-database (`BfTests:148-170`). It did not run again when the main app was installed.
4. **One app's install ends before the next app's begins (P5).** All 3 fixture rows come before every main app row in
   `SystemRowVersion` order (`BfTests:239`).
5. **Order inside one app (P6).** In each company the main app's install routines ran in this order (`BfTests:239`,
   rows of one company also `BfTests:125-137`):

   | Position | Routine | Kind | Object id |
   |---|---|---|---|
   | 1 | "LT V Subs" | subscriber | 69851 |
   | 2 | "LT BF Z Subs" | subscriber | 81253 |
   | 3 | "LT BF Install Late" | install codeunit | 81254 |
   | 4 | "LT BF A Subs" | subscriber | 81255 |
   | 5 | "Install StartSession Probe" | install codeunit | 60445 |
   | 6 | "Install Seeder" | install codeunit | 60618 |
   | 7 | "BEI Installer" | install codeunit | 60894 |

   The same order repeats in the other company. The per-database part runs the routines that have one in the same
   order: "LT V Subs", "LT BF Z Subs", "LT BF Install Late", "LT BF A Subs", "Install Seeder" (`BfTests:138-140`,
   `:239`). One company finishes before the next starts; the per-database part runs last. Two install codeunits are
   not in the table because they write nothing: "ALT Install Probe" (60838) and "ALT Quoted Install Probe" (60828),
   both with empty triggers (`tests/al-language/_fixtures/ALTInstallProbe.al`, `ALTQuotedInstallProbe.al`).
6. **The per-database part (P7).** `CompanyName()` is empty there, in subscribers and in install codeunit triggers
   (`BfTests:138-140`, the "blank" tag). `IsEmpty` and `FindFirst` of a per-company table fail with `You must choose a
   company before you can access the "LT BF Per Company Row" table.` and `GetLastErrorCode` `DB:Ok` (`BfTests:138-139`,
   reads in `BfFix:89-146`). `DB:Ok` is an odd code for an error. By the owner's rule for results that may be a bug of
   BC on Linux containers, it is checked on the Windows nightly before any code relies on it; the error text is used
   either way.
7. **Sessions and execution context (P8).** All 9 rows of the main app's install carry one `SessionId()`; it is not the
   test session; the fixture app's install ran in another session, also one for all its rows (`BfTests:142-144`,
   `:168`). The execution context is `Install` in every row, subscribers included (`BfTests:125-140`, `:164-166`).
8. **SingleInstance state (P8).** In the install session a SingleInstance codeunit starts again from its initial state
   in each company and in the per-database part: the counter is 1, 2, 3 in each company and 1, 2, 3 in the per-database
   part (`BfTests:125-140`; codeunit `BfFix:51-63`). Within one company the state carries from one install routine to
   the next. The fixture app's SingleInstance counter reads 0 inside the main app's install (`BfTests:125-140`, the
   `fixture counter` column), consistent with a separate session.
9. **Company Triggers in the install session (P8).** The main app's subscribers to `OnCompanyOpen` and
   `OnCompanyOpenCompleted` (`BfCompanyOpenFixtures.al:51-71`) ran in the install session: every per-company row of the
   main app sees 2 calls, and the per-database rows see 0 (`BfTests:125-140`, the `company open calls` column). Since
   the counter restarts per company (fact 8), both events are raised once in each company, before the logged routines
   run there, and not in the per-database part.
10. **Company Triggers in the test session.** `OnCompanyOpen` then `OnCompanyOpenCompleted`, once each, in the test
    session, before the first test of the codeunit; inside the subscribers `CompanyName()` is the session company, the
    execution context `Normal`, the client type `Web` and `GuiAllowed` Yes (`BfOpen:19-38`). A second test of the same
    codeunit does not raise them again (`BfOpen:40-50`). This agrees with `PeVTests.al:297`.

### A2. The order inside one app is not known: tests that can see it are refused

This replaces "in object order" in decision 3 and the reading of fact 4 as an object-id order.

**Verdict on the rule.** Fact 5 is not ascending object id: 60445, 60618 and 60894 run after 81255. Each candidate
was checked against the 7 per-company and 5 per-database rows:

| Candidate rule | Fits fact 5? |
|---|---|
| Ascending object id; also the pinned order of ordinary events (below) | No: 60445 runs after 81255 |
| Descending object id | No: 69851 runs before 81253 |
| Ascending object name | No: "LT BF Z Subs" runs before "LT BF A Subs" |
| Grouped by kind (all subscribers first, or all install codeunits first) | No: 81254 runs between 81253 and 81255 |
| File order in the compiled app (`src/` entries of the `.app`) | No: that order is Seeder, StartSession, BEI, "LT V Subs", then the three BF codeunits |
| Codeunit order in `SymbolReference.json` of the `.app` | No: the same order as the file order |
| Ascending object id, starting at 69851 and wrapping around | Fits, but the start point is chosen from the data. Not a rule |
| Descending object name (ordinal or ignoring case) | Fits all 12 rows |

The `.app` checked is writer BF's local compile with AL 30 beta (`~/.cache/corpus-writer/bf/t29.app`), not the CI
artifact. The CI compile may order files differently.

Descending object name is the only fixed rule found that fits. It is not adopted, for three reasons:

- It was found after looking at the data, among about eight candidates, with 7 routines.
- It disagrees with the order of ordinary events, which the corpus pins as ascending codeunit id within one app:
  `record/TestTableEventSequenceContracts.al:456-471` (68154, 68155, 68156), `events/TestProbeEventContracts.al:668-680`
  (69806 "PA Ev Subs" before 69807 "PA Ev Subs Other Id": descending name would put 69807 first),
  `misc/lt-probes-needed-3/PnC01Events.al:143-166` (71073 to 71076, manual codeunits in id order) and
  `handlers/ProbeEventSubscriberOrder.al:146-164`. So install code would need its own rule. That is possible, but
  nothing pins it.
- Hard rule 3: a rule nobody pinned is a guess.

The runtime today follows the pinned order for ordinary events: `al-vm/src/events.rs:304-318` sorts the subscribers of
one event by codeunit, and within a codeunit by procedure name; `al-vm/src/dispatch.rs:366-368` then sorts the targets
by codeunit id, with no app in the key. Install codeunits run in program order (`runtime/blocked.rs:8-38`). Probe Q1
asks the question that would prove the install rule; probe Q2 asks the order across apps (owner's hypothesis: app id).

**Decision.** Within one app and one part, the runtime runs install routines in a fixed order of its own (program
order, as today) and refuses every test that can see that order. A test sees it through:

- **a. A table that two or more install routines of the same app and part write.** AutoIncrement ids, which insert
  of a key wins, and what a later routine finds all depend on the order. The table joins `Engine::unseeded`, so a read
  whose answer depends on its rows is refused at run time (the rule of item 4c).
- **b. A table that one install routine reads and another of the same app and part writes.** Every table the reading
  routine writes joins `Engine::unseeded`.
- **c. A SingleInstance codeunit that two or more install routines of the same app and part use.** Its state carries
  between them within one company (fact 8). Every table those routines write joins `Engine::unseeded`.
- **d. `SystemRowVersion` of a row that an install routine wrote,** in an app with two or more install routines in
  that part. It orders rows across tables (`BfTests:8-9`). A read of `SystemRowVersion`, `SystemCreatedAt` or
  `SystemModifiedAt` of such a row raises `Signal::Unsupported`.

Items a to c are found before anything runs, from the static closures of the routines (`tables_written`, the tables
read, and the SingleInstance codeunits reached), the same analysis as item 4c. They apply to the apps under test.
Today's corpus has no case they refuse: the install routines of the main app write disjoint tables, and "ALT Install
Probe" and "ALT Quoted Install Probe" write nothing. Item d also refuses `InstallCodeunit_PerDatabaseAndPerCompany_OrderAndCount`
(corpus branch `al-runtime-rs-staging-av`, `tests/al-language/install/lt-av-install-order/AvInstallOrder.al:16-69`)
once it lands, although that test only compares per-company rows with the per-database row, an order that is pinned
(fact 5). This is accepted: the runtime cannot tell which rows a test compares. That case is not in the baseline.

**Decided (owner, 2026-10-19):** dependency apps (Microsoft's apps) get the same refusals a to d, whatever they cost.
Step A still counts the pairs of install routines of one dependency app that a to c flag, and the Microsoft tests the
refusal stops, and reports both numbers.

When probe Q1 pins a rule, a later amendment replaces the refusals a to d with that order.

### A3. Across apps (P4, P5)

- One app's whole install (every company, then the per-database part) ends before the next app's install begins
  (fact 4). This confirms step 4's "one app complete before the next".
- Apps are installed in dependency order. The corpus pins this only for one dependency pair (the fixture app before
  the main app). The CI also publishes them in that order (corpus `ci.yml:207-208`), so the corpus cannot tell
  dependency order from publish order. Both give the same order for every run the runner makes.
- An app's "Extension Triggers" subscribers run only at its own install (fact 3). Option 5 stays rejected.

### A4. Sessions, SingleInstance state and Company Triggers of the install run

This replaces decision 8.

1. **One install session per app**, apart from the test session (fact 7). Each app under test gets its own session for
   its install. Its `SessionId()` differs from the test session's and from every other app's install session.
2. **SingleInstance state starts again** at the start of each company and at the start of the per-database part
   (fact 8), in the default mode as well (the per-database part no longer sees the per-company state). Within one
   company it carries from one install routine to the next.
3. **Execution context `Install`** in subscribers and in install codeunit triggers (fact 7). This is already true for
   install codeunit triggers (`install/TestInstallEnvVisible_Tests.al`).
4. **Company Triggers in the install session.** The install session raises `OnCompanyOpen` and then
   `OnCompanyOpenCompleted` in each company, before that company's install routines, and not in the per-database part
   (fact 9). Pinned only for subscribers of the app being installed. Whether a dependency app's subscribers run there
   is probe Q3. **Decided (owner, 2026-10-19):** raise the two events to every subscriber of every installed app,
   Microsoft's apps included, as a flagged assumption with id `install_company_triggers_all_apps` in the runtime
   assumption registry. A test whose baseline passes through it is reported as a flagged pass. When probe Q3 lands,
   it wins; the code changes if it disagrees. Step A still reports how many dependency-app subscribers to the two
   events exist and which tables they write.
5. **Company Triggers in the test session** (fact 10) stay a separate additive change. It is no longer waiting on a
   probe. Not pinned: whether they are raised once per test session or once per test codeunit (the corpus runs the
   tests of one codeunit in one session).

### A5. Per-database code (P7)

`CompanyName()` is empty there (decision 3 already says so; now pinned by fact 6). A read of a per-company table there
fails with the text of fact 6. The runtime raises that error with that text, so `GetLastErrorText` is answered. The
code is answered `DB:Ok` only after the Windows check of fact 6; until then a read of `GetLastErrorCode` after this
error is refused. A write to a per-company table in per-database code is not pinned: an error there cannot be caught,
so a probe app would not install (`BfTests:24-25`). It stays refused (`Signal::Unsupported`).

### A6. Companies under the opt-in (P1, P3)

- Step B sets `companies = ["CRONUS International Ltd.", "My Company"]` in `oracle/oracle.toml`, citing
  `BfTests:60-62`. It applies to the main corpus job. The OnPrem jobs were not probed; their suites run in the default
  mode until a probe shows their companies.
- The seeded Company rows (step 1) hold the pinned field values: `Evaluation Company` Yes for the session company, No
  for "My Company", `Display Name` empty for both (`BfTests:60-61`). Other fields of the Company table stay refused
  until pinned.
- Decision 5 stays: under the opt-in, a dependency app's per-company table in another company is refused. P3 now says
  what "My Company" holds: the rows of fact 2, without `Company-Initialize`. A later amendment may answer those
  tables if the runtime runs the dependency apps' per-company install code in "My Company" (without
  `Company-Initialize`) and gets exactly the counts of fact 2. That is a measurement for after step B, not part of
  this amendment.
- P2 (session company first, or key order) is still open: in this CI the session company also sorts first.

### A7. Build order and expected outcomes

- **Step A** no longer waits for a probe (P4, P6 and P7 are answered), only for the submodule and oracle bump. It
  adds A2 (refusals a to d), A4 items 1 to 3, A4 item 4 (all apps' subscribers, flagged assumption `install_company_triggers_all_apps`), and A5.
- **Step B** no longer waits for a probe (P1 and P8 are answered), only for the bump. It adds A6.
- Expected outcomes of `BfTests`:

  | Test | Default mode | Opt-in (step B) |
  |---|---|---|
  | `Companies_OfTheDatabase_NamesInKeyOrder` | not run (item 4a) | passes |
  | `Companies_BaseTableRows_ReadFromATest` | not run (item 4a) | not run (decision 5) |
  | `InstallLog_RowsOfThisApp_InIdOrder` | not run (A2 a) | not run (A2 a) |
  | `FixtureLog_RowsOfTheDependencyApp_InIdOrder` | not run (item 4c) | passes (A4 items 1 and 2) |
  | `InstallOrder_AllInstallRows_InSystemRowVersionOrder` | not run (A2 d) | not run (A2 d) |

  The two `BfOpen` tests need the Company Triggers change of A4 item 5.

### A8. Still not pinned

- The order rule inside one app (probe Q1) and across apps for one event (probe Q2).
- Whether dependency apps' Company Triggers subscribers run in the install session (probe Q3).
- P2 (order of companies: session company first, or key order) and P9 (companies in Microsoft's CI lanes).
- The companies of the OnPrem jobs' databases, and the Windows nightly for P1.
- `GetLastErrorCode` `DB:Ok` of fact 6 on Windows.
- A write to a per-company table in per-database code.
- Company Triggers in the test session: once per session or once per test codeunit.

### A9. Probe questions for a corpus writer: the subscriber and install order

Install code must never fail (`check-install-triggers.py`). Each probe logs with `if Insert() then;` into a
`DataPerCompany = false` AutoIncrement table, as `BfFix` does, and orders rows across tables by `SystemRowVersion`.
New subscribers need no allow-list change; a new install codeunit does.

- **Q1. The rule inside one app, for install events and for an ordinary event.** Add four or five subscriber codeunits
  to the main app. Each subscribes to `OnInstallAppPerCompany`, `OnInstallAppPerDatabase` and one ordinary
  integration event that a test raises. Choose ids, names, namespaces and files so that every candidate predicts a
  different order. For example (the writer picks free ids):

  | Codeunit | Id | Name | Namespace | File, position in file |
  |---|---|---|---|---|
  | P | lowest | "LT BG M Subs" | none | folder `b-order`, 2nd |
  | Q | 2nd | "LT BG b Subs" (lower-case b) | none | folder `a-order`, 1st |
  | R | 3rd | "LT BG C Subs" | none | folder `b-order`, 1st |
  | S | 4th | "LT BG A Subs" | `Zzz.Probe` | folder `a-order`, 2nd |
  | T | highest | "LT BG Y Subs" | none | folder `c-order`, 1st |

  Predictions: ascending id P Q R S T; descending id T S R Q P; descending name, ordinal: Q T P R S (lower-case
  sorts after upper-case); descending name ignoring case: T P R Q S; descending full name with namespace: S first;
  file order: Q S R P T (or another order, read from the CI `.app`). The ordinary event shows whether ordinary events
  and install events use the same rule (pinned: ascending id for ordinary events). The names "LT BG …" sort after
  "LT BF …" and the existing rows of fact 5 give more points for free. Also save the CI-built `.app` of the run, so
  its file and symbol order can be compared.
- **Q2. The order across apps for one event (owner's hypothesis: app id).** One ordinary integration event,
  published in the fixture app, with subscribers in the fixture app and in the main app. Choose codeunit ids so that
  codeunit id order and app order disagree: a main app subscriber with an id above 61099 (the fixture app's range is
  61000 to 61099) and one below 61000. The two existing app ids already separate two ways of comparing a GUID: as text,
  `a1b2c3d4-…` (main) sorts before `f1e2d3c4-…` (fixture); SQL Server `uniqueidentifier` order compares the last group
  first, and `ba9876543210` (fixture) sorts before `ef1234567890` (main); so does the byte order of `Guid.ToByteArray`
  (`c4…` before `d4…`). Dependency order also puts the fixture app first. So two apps can show "text order" against
  "the other three". Separating byte order, SQL order and dependency order needs a third app whose id is chosen for
  it, for example a new small corpus app that depends on the fixture app, with an id that sorts first as text and
  last as `uniqueidentifier`. The OnPrem app (`3f8c1a52-…-5ea70b93d418`) cannot separate text order from reverse
  dependency order (both put it first). It also runs in separate jobs.
- **Q3. Company Triggers of a dependency app in the install session.** A subscriber to `OnCompanyOpen` in the fixture
  app increments a SingleInstance counter of the fixture app; the main app's install routines log that counter (as
  `BfFix:82` logs the fixture counter). A test also counts how often the fixture app's subscriber ran during the
  fixture app's own install. This decides A4 item 4.

## Options considered

1. **Several companies by default** (the first draft of this ADR). Rejected by the owner (2026-10-19): the runner runs
   one company by default; several companies are an opt-in for the tests that need them.
2. **Under the opt-in, dependency apps and `Company-Initialize` also run in every company.** This would give the other
   companies setup data, so the app under test's install code could read it there. Not chosen now: what another company
   holds on real BC depends on when it was created (P3), and running `Company-Initialize` there would guess at it. Cost:
   the per-company part of the baseline about doubles (BC 27.5: 3 install triggers in Base Application, 5 in System
   Application, 1 in System Application Test Library, plus `Company-Initialize`, the longest root; the whole baseline of
   a corpus run on BC 29 took 2.68 s today, 18 of 18 roots to their end). Return to it after P3.
3. **In the default mode, answer other-company reads as a database with one company.** That is what the runtime does
   today. Rejected: the database a user's CI runs on may hold more companies (the demo backup holds two), so the answer
   would be a guess (hard rule 4).
4. **A placeholder name for the second company now.** Any name other than the session company would make
   `EventLog_InstallRowCompany` pass. Rejected: the Company table and `CompanyName()` inside install code show the
   name, and nothing pins it (hard rule 3).
5. **Raise "Extension Triggers" to every subscriber of every app at every install.** Not chosen until P4: it would run
   a dependency app's subscribers again at each later install.

## Consequences

- In the default mode the app under test's "Extension Triggers" subscribers now run, in the session company. A partner
  extension that installs through them gets its install data.
- In the default mode, tests that scan the Company table, name a company the run does not know, or read a shared table
  that per-company install code fills are not run, with the reason. Today they are answered. Partner tests that need
  them use the opt-in: `--companies` with their CI's companies, or `--companies all` when the test data holds them.
- With the corpus opt-in, `EventLog_InstallRowCompany` can pass after step B. `EventLog_RowsWrittenBeforeTheTestSessionStarted`
  also needs the Company Triggers change.
- Two workspace apps with install code now run one after the other, not interleaved. No corpus app other than
  `al-language` has install code, so no corpus outcome changes from this.
- `docs/probes-needed.md` rows 28 and 29 change when the probes land: the order for subscribers of one app is now
  pinned (fact 4).

## Conformance

- Corpus: after step A, `EventLog_InstallRowCompany` and `EventLog_RowsWrittenBeforeTheTestSessionStarted` are not run;
  after step B, with the corpus opt-in, `EventLog_InstallRowCompany` passes on BC 27, 28 and 29. The install tests stay
  passing: `install/TestInstallTriggerSeed_Tests.al`, `TestInstallEnvVisible_Tests.al`,
  `TestInstallEventDispatch_Tests.al`, `TestInstallStartSession_Tests.al`. `--baseline-diff` shows no baseline case lost.
- `tools/company_mode_check.sh run` (step B): a corpus case differs between the two modes only by being not run in the
  default mode.
- Runtime tests in `al-testrun/src/runtime/tests.rs`, next to the install order tests: an "Extension Triggers"
  subscriber of a workspace app runs per company, then per database; a look-alike procedure on a normal codeunit does
  not run; two workspace apps install one after the other; the baseline path and the path without a baseline give equal
  rows. Default mode: one Company row; `Company.Get(CompanyName())` is answered, `Company.Count()` and
  `ChangeCompany` to an unknown name are refused, a read of a shared table written by per-company install code is
  refused and an insert into it is not. Opt-in: each declared company holds its per-company rows; a shared AutoIncrement
  table counts 1, 2, 3 in the order of fact 4; `ChangeCompany` to a name not on the list answers that it does not exist;
  a dependency per-company table is refused in another company; a root that stops in another company rolls back only
  there and marks its tables unseeded in that company.
- `tools/baseline_rule_check.sh run`: 0 differences. `tools/demand_vs_eager.sh run`: no difference.
- Microsoft buckets before and after step B (at least Tests-Bank and one of the five test apps with install code): the
  changed outcomes are listed in the pull request.

## Step A as built (2026-10-20, branch `lane-adr046-install`)

- `al-testrun/src/install.rs` lists the install code of every app per part (`plan`): install codeunit triggers and
  "Extension Triggers" subscribers, in program order (the runtime's own order, A2). The dependency apps' lists are the
  baseline's step 2; the apps under test run through `run_apps_under_test` (decision 7: the baseline and the run without a
  baseline call the same function), one app after the other, each with its own install session id (A4 item 1: any
  positive id other than the test session's), SingleInstance state dropped at the start of each part and at the end
  (A4 item 2, fact 6), and the app's own Company Triggers `OnCompanyOpen`/`OnCompanyOpenCompleted` subscribers run before
  the per-company part (A4 item 4, the pinned half). PR 334's interim refusal is gone.
- `install::facts` (once per engine, from the sources only) gives `al_vm::InstallFacts`: the tables of decision 4c and of
  A2 items a to c for the apps under test, whose reads outside install code are refused with the reason, as for
  `Engine::unseeded`; and the tables of A2 item d, whose `SystemRowVersion`, `SystemCreatedAt` and `SystemModifiedAt` are
  refused for rows at or below the row version install code ended at (`Session::mark_install_end`). `AL_INSTALL_FACTS=1`
  prints them, the same lists for the dependency apps (not applied, below), and the shared tables the dependency apps'
  per-company install code writes ("Not decided": counted, not refused; BC 29: Onboarding Signal, Upgrade Tags).
- A5: `IsEmpty` and `FindFirst` of a per-company table without a Caption fail in the per-database part with the pinned
  text and an unpinned code (a read of `GetLastErrorCode` is refused); every other access stays refused.
- Demand: a dependency app's "Extension Triggers" install subscribers are baseline roots; the dependency apps' Company
  Triggers `OnCompanyOpen`/`OnCompanyOpenCompleted` subscribers are roots when an app under test has install code (so the
  refusal below reads complete closures in both compile modes). Impact selection lists an extension's "Extension
  Triggers" install subscribers as install code.
- Not built: A4 item 5 (Company Triggers in the test session) and step B.

### Decided without owner review (2026-10-20)

1. **A4 item 4, other apps' subscribers: not raised.** Raising Company Triggers to every installed app's subscribers
   runs System Application's "System Initialization" (`Init`, `InitCompany`), which reaches a refused DotNet call
   (`NavUserAccountHelper`); every per-company install routine of the corpus app then could not run, and 25 baseline cases
   on BC 29 stopped passing. So only the installed app's own subscribers run (pinned, fact 9). A per-company install
   routine that reads a table those other subscribers write, or shares a SingleInstance codeunit with them, has its
   tables refused. The assumption `install_company_triggers_all_apps` is not registered. Probe Q3 still decides.
2. **A2 for dependency apps: worked out, not applied.** Applied, it loses the baseline case 60293
   `RetenPolAllowedTables_IsAllowedTable_UnregisteredTable_IsFalse` (System Application's "Email Installer" and
   "Retention Policy Installer" both write Retention Policy Allowed Table; also Upgrade Tags, read and written by every
   installer). Counts, tables of System Application and Base Application: BC 29 13 (A2 a to c) and 14 (A2 d); BC 27 10
   and 12.

### A4 item 4 as built (2026-10-28, al-runner issue 6046)

This replaces item 1 of "Decided without owner review (2026-10-20)".

- The install session of an app under test raises Company Triggers `OnCompanyOpen`, then `OnCompanyOpenCompleted`, to the
  subscribers of every installed app: the dependency apps, the apps under test before it in dependency order, and the app
  itself, each event in ascending codeunit id. This is the registered assumption `install-company-triggers-all-apps`
  (registry ids use hyphens; the owner decision named it `install_company_triggers_all_apps`). A test that touches a table
  the other apps' subscribers write, or a table a per-company routine writes that reads such a table or shares a
  SingleInstance codeunit with them, is a flagged pass (`InstallFacts::assumed`, marked at the record operation).
- A subscriber of any installed app that cannot run in this runtime stops the per-company install routines of the app,
  as one of the app's own subscribers does: what the subscribers did is rolled back, and a test that touches a table one
  of the routines or one of the subscribers can write is not run. System Application's "System Initialization" runs here
  since `NavUserAccountHelper.IsDelegatedUser`, `IsUserDelegatedAdmin` and `IsUserDelegatedHelpdesk` answer false
  (`al-language-onprem-dotnet/lt-al-dotnet/AlUserAccountHelper.al:61-63`); a first build kept a fallback to the app's own
  subscribers while it could not, and that fallback is gone (al-runner issue 6046).

## Amendment 2: step B as built (2026-10-29, branch `companies`)

Status line: Amendment 2, pins P1/P2 landed. Decided without owner review (2026-10-29) where marked; nothing the owner
decided is changed.

### The pins, checked against the corpus at the pin

- P1 (companies): two companies in key order, "CRONUS International Ltd." (the session company) then "My Company"
  (`install/lt-bf-install-probes/BfInstallTests.al:47-64`, header `:10`), on 27.5, 28.5 and 29.0.
- Per-company install code runs once in each company, the session company first, before the per-database part:
  `BfInstallTests.al:125-140` (rows #1 to #6, then #7 to #9) and `:239`; `install/lt-av-install-order/AvInstallOrder.al:16-69`
  (`first per-company run` = session company, `per-company after per-database` = No in both companies).
- `CompanyName()` is that company in the per-company part (`AvInstallOrder.al:43-44`, `:60`, `:64`) and empty in the
  per-database part (`BfInstallTests.al:138-140`, `blank`).
- P2 (session company first, or key order) stays open: in the corpus CI the session company also sorts first. The runtime
  runs the declared list in the order given, which is the session company first; nothing here contradicts the ADR.

### What is built

- `--companies NAME[,NAME...]` and `--companies all` on `al-runner test` and `al-test-cli` (a name given twice, an empty
  name, or a list whose first name is not the session company is a usage error before anything runs). `al-test-cli
  --one-company` ignores `companies` of `oracle.toml`. `al_host::Identity::companies` (default: one company) carries the
  setting; `all` is resolved by the session against the test data (`TableProvider::companies`, the backup's Company
  table), and without test data it is the session company alone, the same as the default. The setting is part of the
  baseline cache key.
- Step 1: one Company row per declared company. Step 4a: each app under test's per-company part once per declared
  company, in list order, with its own SingleInstance reset and Company Triggers; step 4b once. Steps 2 and 3 are
  unchanged (session company only).
- Decision 4: items a (Company table reads) and c (shared tables of per-company install code) apply only in the default
  mode; under the opt-in the declared list is complete and both are answered.
- Decision 5: under the opt-in, a per-company table that is not a table of an app under test is refused in a declared
  company other than the session company (`Session::cx_for`), inside install code and in tests.
- Decision 6: a routine that stops in another company makes the tables it would have written (`tables_written` of the
  routine and of the subscribers it reaches) unknown: a per-company table only in that company (`Baseline::company_unknown`,
  refused at run time), a shared table everywhere (not run, as today).
- `tools/company_mode_check.sh run` compares an opt-in run with a `--one-company` run.

### Decided without owner review (2026-10-29)

1. **Company Triggers in another company.** A4 item 4 raises `OnCompanyOpen`/`OnCompanyOpenCompleted` to every installed
   app's subscribers. In a company other than the session company only the subscribers of the apps under test run there:
   decision 5 keeps the dependency apps in the session company, and System Application's "System Initialization" reads
   its per-company tables (Job Queue Entry) there, which decision 5 refuses; run, it stopped every per-company routine of
   the corpus app in "My Company" and 1,104 baseline cases on BC 29 were lost.
2. **Evaluation Company.** The corpus database's session company is an evaluation company (`BfInstallTests.al:61`), but
   a user's database need not be, and Company-Initialize reads the flag: with it set, Experience Tier Setup came out
   Basic where BC has Essential (`session/lt-company-setup-rows/CompanySetupRows.al:50-66`, a failure). So the session
   company's row stays the row of the default mode; another company's row has `Evaluation Company` No and an empty
   `Display Name` (pinned for "My Company").
3. **`companies` is not set in `oracle/oracle.toml` yet.** See the next section.

### Why the corpus run does not use the opt-in yet

Measured with `tools/company_mode_check.sh run` (`--companies "CRONUS International Ltd.,My Company"` against
`--one-company`): no case passes or fails differently, and no case is not run in the default mode only; but 35 (BC 27),
34 (BC 28) and 31 (BC 29) cases that pass in the default mode are not run under the opt-in, and neither PeV test passes
in either mode. The OnPrem and isolation suites show no difference. Cause: in "My Company" every insert of install
code reaches Base Application's database trigger subscribers, which read per-company setup tables of Base Application
(CDS Connection Setup) there. Decision 5 refuses that read, so almost every per-company install routine of the corpus app
stops in "My Company"; by decision 6 the shared tables they would have written (LT V Event Log, LT BF Install Log, and,
through the static closure of "Install Seeder", Company, Upgrade Tags and Feature Data Update Status) are unknown
everywhere. The owner's gate (no baseline case lost) rules the key out until one of these lands:

- the measurement A6 describes: run the dependency apps' per-company install code in "My Company" without
  Company-Initialize and match the counts of fact 2 (P3), then answer the dependency tables there;
- a probe of what CDS Connection Setup (and the other tables the database trigger setup reads) holds in "My Company".

The default mode is unchanged: `--baseline-diff` on 27, 28 and 29 loses no case with this branch.

### NP Retail with the opt-in (2026-10-29)

`al-runner test <NP>/Application <NP>/Test` against the same with `--companies all` (the backup's companies, session
company first): without the flag 1,553 pass, 13 fail, 3,810 not run; with it 1,267 pass, 12 fail, 4,097 not run. The
about 1,070 refusals of decision 4c ("per-company install code of NP Retail writes this DataPerCompany = false table")
are gone, and 2,817 refusals of decision 6 take their place: every per-company install routine of NP Retail stops in
"My Company" at the same read of CDS Connection Setup (Base Application's `Global Triggers` `GetDatabaseTableTriggerSetup`
subscriber calls `CRMIntegrationManagement.GetDatabaseTableTriggerSetup`, which reads that per-company table).

## Amendment 3 (2026-10-29): probe Q1 landed

Accepted under A2's own rule: "When probe Q1 pins a rule, a later amendment replaces the refusals a to d with that
order." The rule is replaced only where the pins reach. Branch `install-order`.

### What the probe pins

Corpus PR 807 (writer UM, branch `al-runtime-rs-staging-pub-um`, commit `e3b2eedf`, not yet in the submodule pin),
`tests/al-language/install/lt-um-install-order/UmInstallOrder.al`. Five install codeunits of the main app, 90431 to
90435, named so that name order and id order differ, in five files of four folders. Each appends its name to one shared
SingleInstance codeunit and logs a row. The result, the same on the three supported majors (`:120-125`):

- In each company and in the per-database part they run in ascending object id: `Mu;b;Cu;Ay;Zz;`. Not name order, not
  file order.
- The SingleInstance codeunit's state carries from one install codeunit to the next, and starts again in each company
  and in the per-database part (`state as each row saw it`). This agrees with A1 fact 8.
- All rows carry one session id (A4 item 1).

### The pins disagree with "ascending object id" across 65536

A1 fact 5 (`install/lt-bf-install-probes/BfInstallTests.al:239`, on master) has install codeunits of the same app run
in this order: 81254, then 60445, 60618, 60894. That is not ascending id. So "install codeunits of one app run in
ascending object id" holds for the UM codeunits but not for the app as a whole.

Both data sets fit one finding: inside one block of 65536 ids (`id / 65536` equal), routines run in ascending id,
install codeunits and Extension Triggers subscribers in one sequence (fact 5: 69851, 81253, 81254, 81255 in block 1;
60445, 60618, 60894 in block 0; UM: 90431 to 90435 in block 1). Across blocks, block 1 ran before block 0. Two rules
explain that, and they disagree for other ids:

| Rule | Fact 5 | UM | NP Retail's 13 per-company install codeunits (blocks 91 to 95) |
|---|---|---|---|
| Ascending id | No | Yes | Ascending id |
| Ascending `id mod 65536` | Yes | Yes | 6248195, 6184801, 6185124, 6059909, 6059987, 6014438 to 6014599, 6151434 to 6151514 |
| Descending block, ascending id inside a block | Yes | Yes | 6248195, 6184801, 6185124, 6151434 to 6151514, 6059909, 6059987, 6014438 to 6014599 |

The main corpus app's id ranges end at 99999, so it only has blocks 0 and 1. There the two rules that fit always agree.
A probe that tells them apart needs ids above 99999 in block 1, or another app.

### Decision

1. The runtime runs the install routines of one app and part in ascending object id (`install.rs` `plan`). That order
   is the runtime's own wherever it is not pinned.
2. **The order of two routines is pinned when their object ids lie in one block of 65536 ids** and they are not two
   subscribers of one codeunit (`install.rs` `order_pinned`). Refusals a, b and c of A2 no longer apply to such a pair.
   SingleInstance state carries between them within one company or part and starts again for each (A4 item 2, as
   built).
3. Refusals a, b and c stay for a pair whose ids lie in different blocks, or two subscribers of one codeunit. The reason
   now names the two routines.
4. **Refusal d stays** for a part that has such an unpinned pair: `SystemRowVersion` orders rows across tables, and the
   order across blocks is not known. In a part whose routines are all pairwise pinned, d no longer applies: the pins
   cover the run order, and the runtime's row versions follow the run order (BfTests:8-9).
5. Until `UmInstallOrder.al` is in the pinned corpus, a test that reads a table a pinned pair makes order-dependent (the
   tables a to c and d would have refused) is a flagged pass, assumption `install-order-ascending-id` (awaiting that
   file). When the bump lands, the entry goes and the code cites the file.

### Decided without owner review (2026-10-29)

- The orchestrator's task said "ascending object id" for the whole app. Fact 5 contradicts that across 65536, so it is
  applied only inside one block. Applying it across blocks would put 60445 before 81254, which BfTests:239 pins the
  other way.

### Effect

- Corpus: no change on 27, 28 and 29 (0 newly pass, 0 lost). No corpus app has a pair that a to c refused inside one
  block.
- NP Retail, owner form `al-runner test Application Test`: no change (1,592 pass, 13 fail, 3,771 not run, 0 outcome
  differences). Every pair that a to c flag in NP Retail crosses a block: "NPR Job Queue Install" (6014438, block 91)
  with "NPR TM Calendar Upgrade" (6059909, block 92), "NPR Feature Management Install" (6151434, block 93) and "NPR
  Feature Flags Install" (6151514, block 93); "NPR Webservice Install" (6014471) with "NPR Spfy App Install" (6184801).
  The shared SingleInstance codeunit in all 20 c refusals is System Application's "Environment Information".

### Still not pinned (probes wanted)

- **Q1b. The order across blocks of 65536 ids.** One app with three install codeunits in two blocks, for example
  60100, 70000 and 130000 (`id mod 65536`: 60100, 4464, 64464), sharing a log table and a SingleInstance codeunit as in
  UM. Ascending id gives 60100, 70000, 130000; ascending `id mod 65536` gives 70000, 60100, 130000; descending block
  gives 70000, 130000, 60100. Needs an id range above 99999 in a corpus app. This unblocks NP Retail.
- **Q1c. Ordinary events across blocks.** Fact 5 interleaves install codeunits with Extension Triggers subscribers in
  one sequence, so BC may order ordinary event subscribers the same way. Every pin of ordinary event order has its
  subscribers in one block (for example `handlers/ProbeEventSubscriberOrder.al`, 69461 to 69467). The runtime sorts
  subscribers by plain codeunit id (`al-vm/src/events.rs`). Probe: one event of the main app with subscribers 60100 and
  70000.


## Amendment 4 (2026-10-30): one company by default, "My Company" under the opt-in

### Owner decision (Accepted, owner, 2026-10-30)

"Many code partners write per-company code because they must support customers with more than one company. That
doesn't mean we can't run the per-company triggers in the one default company the runner runs. But if the caller
specifies multiple companies or `all`, it has to behave like Microsoft would in a multiple-company environment."

1. **The default mode models a database with one company.** Per-company install code runs once, in that company. A
   `DataPerCompany = false` table that per-company install code writes holds that one write, and is answered. This
   replaces decision 4c. Items a and b of decision 4 stay: a test that looks for other companies in the default mode
   still sees only the one company.
2. **Under `--companies <list>` or `all` the run behaves like BC with several companies.**
3. **The corpus CI database has two companies** (A1 fact 1), so the corpus run uses the opt-in through the `companies`
   key of `oracle/oracle.toml`, once adding it loses no case.

### What "My Company" holds (built, branch `mycompany`)

Corpus PR 804 (writer UO, on corpus master, not yet in the submodule pin) pinned more of "My Company" in
`install/lt-bf-install-probes/BfInstallTests.al`, the same on 27.5, 28.5 and 29.0:
`SecondCompany_CdsConnectionSetup_ComparedWithSessionCompany` (no CDS Connection Setup row in either company) and
`SecondCompany_SetupTablesReadByCompanyInitialize_ComparedWithSessionCompany` (no General Ledger Setup, Sales &
Receivables Setup or Company Information row in "My Company"; one Application Area Setup row, a shared table; one Job
Queue Entry). A1 fact 2 adds Source Code Setup (none), Report Selections (2) and No. Series (0 or 1).

Decision 5 changes for the per-company tables of dependency apps and the platform in a declared company other than the
session company. Such a table is answered, starting empty, when the database the run models holds no row of it there:

- **With test data:** the restored database says (`TableProvider::holds_rows_in`, read from the backup's tables of that
  company). BC has what that database holds. A table that holds rows there stays refused: the runtime serves only the
  session company's rows.
- **Without test data:** the run models the corpus CI database. Its "My Company" holds no row of Company Information,
  General Ledger Setup, Source Code Setup, Sales & Receivables Setup and CDS Connection Setup (Base Application, the
  pins above). Every other table there stays refused.

Checked against Microsoft's demo database of 29.0 (`al-bcdb read`): its "My Company" holds 0 rows of those five tables,
2 Report Selections, 1 No. Series and 1 Job Queue Entry, the counts the corpus CI database shows. So the CI database's
"My Company" is the demo database's, unchanged by the corpus apps.

**The session company is an evaluation company under the opt-in without test data** (`BfInstallTests.al:61`). Its row
gets `Evaluation Company` Yes after the dependency apps' install code and Company-Initialize, before the apps under test
install: Company-Initialize keeps the row of the default mode, which gives Experience Tier Essential as pinned
(`session/lt-company-setup-rows/CompanySetupRows.al:50-66`). This replaces amendment 2's decision 2 for runs without
test data. With test data the row stays as amendment 2 says.

### Decided without owner review (2026-10-30)

1. The evaluation flag is set after Company-Initialize, not before it: both pins hold only so.
2. A run without test data that declares "My Company" is taken to be the corpus CI database. A run that declares
   another name gets decision 5 as before.

### Still open: CDS is not the last table

With the five tables answered, Base Application's database trigger setup in "My Company" next reads CRM Connection
Setup (`CRMIntegrationManagement.GetDatabaseTableTriggerSetup`). No corpus test pins it there, so it stays refused, and
the corpus with `companies` still loses 31 (BC 29) to 35 (BC 27) cases, all through "Install Seeder" stopping in "My
Company". Measured with CRM Connection Setup answered as empty (a local experiment; the demo database holds no row of it
in either company): BC 27, 28 and 29 each 9 newly pass, 0 fail, 0 lost. Probe wanted: the count of CRM Connection Setup
in "My Company" (corpus issue 811, docs/probes-needed.md). Once pinned, the table joins the list and the key goes into `oracle.toml`.

Item 1 of the owner decision (decision 4c gone) waits for the key: without it, four corpus cases that see the corpus
database's two companies run in the default mode and fail (69852 `EventLog_RowsWrittenBeforeTheTestSessionStarted`,
`EventLog_InstallRowCompany`; 83950 `InstallOrder_ThisApp_FiveSubscribersPerCompanyAndPerDatabase`,
`InstallOrder_DependencyAppA_TwoSubscribersAndCompanyOpenCalls`). It is built on its own branch.
