# Microsoft's BaseApp test buckets under `al-runner`

How to run `Tests-*.Source.zip` of a BC artifact with `al-runner test`, and what the first measurement found
(2026-10-14, branch `alrunner-cli`). Nothing here is Microsoft's code; the sources are read from the local
artifact and never copied into this repository.

## Recipe

`A` is `Applications/` of the platform artifact (here BC 27.5, build in the folder name `46862.48827`). The build in
the folder name fills the `$(app_...)` variables of the source archives. Everything comes from one artifact, with
`--no-download`, so the Microsoft apps and the tests are the same build.

```
al-runner test "$A/BaseApp/Test/Tests-SMB.Source.zip" --no-download \
  --dep "$A/BaseApp/Test/Tests-TestLibraries.Source.zip" \
  --dep "$A/TestFramework/TestLibraries/Variable Storage/Library Variable Storage.Source.zip" \
  --dep "$A/TestFramework/TestLibraries/Assert/Library Assert.Source.zip" \
  --dep "$A/TestFramework/TestLibraries/Any/Any.Source.zip" \
  --dep "$A/TestFramework/TestLibraries/permissions mock/Permissions Mock.Source.zip" \
  --dep "$A/System Application/Test/System Application Test Library.Source.zip" \
  --dep "$A/BusinessFoundation/Test/Business Foundation Test Libraries.Source.zip" \
  --dep "$A/Application/Source/Application.Source.zip" \
  --dep "$A/BaseApp/Source/Base Application.Source.zip" \
  --dep "$A/System Application/Source/System Application.Source.zip" \
  --dep "$A/BusinessFoundation/Source/Business Foundation.Source.zip" \
  --dep "$A/../ModernDev/pfiles/microsoft dynamics nav/270/al development environment/System.app"
```

Without `--no-download` and without the last five `--dep`s, the feed supplies Application, Base Application, System
Application and Business Foundation (build of the newest published match, which can differ from the tests' build),
and the platform `System.app` (from the `ModernDev/.../al development environment` folder of the platform artifact).
Without `System.app` the platform tables (`Field`, `AllObj`, `Company`) are outside the workspace. A build named
exactly with `--bc-version 27.5.46862.48827` is read from `~/.bcartifacts.cache/sandbox/<build>/platform` when it
is there, without asking the feed (the feed lists only recent builds); any matching build there is also the
fallback when the feed has none or cannot be reached.

## The plain command (zero configuration, 2026-10-14, branch `zero-config`)

`al-runner test "$A/BaseApp/Test/Tests-ERM.Source.zip"` with no option and no environment variable now gives the
same result as the recipe, test by test (`tools/bucket_diff.py` on both JUnit files: every test has the same
outcome). The run does these things for you:

1. Reads the build of the tests from the folder of the archive (`.../<build>/platform/...`) or from its version.
2. Takes the Microsoft apps, the test libraries and the platform `System.app` from that build: from
   `~/.bcartifacts.cache/sandbox/<build>` when it is there, else from the feed by range reads into the artifact cache.
   If the build is gone from the feed (it keeps recent ones) the newest match is used and a `warning:` line names both
   builds.
3. Restores the CRONUS demo company of the same build (ADR-020, owner decision of 2026-10-14: test data is on by
   default). It is looked for on disk, then in the cache, then downloaded once (the `.bak` is 517 MB stored, 900 MB
   inflated) with progress on stderr. A failed download turns test data off with a `note:` line that gives the reason;
   `--no-test-data` turns it off on purpose.

| Bucket (BC 27.5) | Plain: passed / failed / not run | Recipe + `--test-data` | Tests with a different outcome |
|---|---|---|---:|
| Tests-SMB, build 48827 on disk | 12 / 3 / 1015 | 12 / 3 / 1015 | 0 |
| Tests-ERM, build 48827 on disk | 557 / 3 / 8773 | 557 / 3 / 8773 | 0 |

Before: plain Tests-ERM gave 185 passed, 60 failed, 9088 not run (no `System.app`, no test data, the newest feed
build next to tests of an older one).

Times (12 cores, no thread option; the machine was shared, load average 20 to 30, so read them as upper bounds):

| Run | First (downloads) | Second (cached) |
|---|---:|---:|
| Tests-SMB 27.5.46862.56249 from the feed, empty HOME and cache | 20 s wall (load 13.6 s, of which 11 s the demo download) | 5 s |
| Tests-ERM 27.5.46862.56249 from the feed, empty HOME and cache | 24 s wall | 11 s |
| Tests-ERM 48827 on disk (plain) | 18.3 s total | same |
| Tests-ERM 48827 on disk (recipe) | 15.5 s total | same |

The plain run needed 12 threads (one per core) and 3.1 GB at the peak, as the recipe does. The cache after a feed run
holds the `.bak` (902 MB) beside the entries fetched by range.

Several buckets can be named in one run (all 32 together: 40300 tests, one compile).

## The compile cache (ADR-022, branch `compile-cache`, 2026-10-14)

`al-runner test` keeps the complete compile of the dependency apps in `<cache dir>/compile/` (13 MB per set of apps;
`--cache-dir`, `$ALRUNNER_CACHE`, default `~/.cache/al-runtime`). The first run with a new set of apps builds it (about
5 s and 4 GB on 12 cores). The next runs load it. `--no-compile-cache` (or `ALRUNNER_NO_COMPILE_CACHE=1`) compiles only
what the tests reach, as before. `--require-compile-cache` fails when there is no usable entry (CI).

Tests-ERM, BC 27.5, with the CRONUS data, one run each after the entry exists: 8.1 to 8.3 s against 11.7 to 12.0 s
without the cache, 2.1 GB against 3.1 GB. All 32 buckets in one process (measured before the last merges of `main`;
the combined run of current `main` needs more than 18 GB with or without the cache): 35 s against 30 to 33 s (the 91 MB
of test sources and the 19 s of tests dominate), 4.7 GB against 6.3 GB. Outcomes: Tests-ERM has no difference; the 32
buckets differ in 7 of 40300 tests (they are not run with the cache, see `docs/adr/022-compile-cache.md`, "Outcomes").
The measurements are in `docs/adr/022-measurements.txt`.

## Reading the result

Without `--test-data` there is no company data. Microsoft's tests expect a CRONUS company, so most failures are
missing setup data. The not-run counts and their reasons are the measurement; use `--top-refusals N`. The failures
are not.

Test data is on by default now (see "The plain command"); `--no-test-data` gives the figures of "off". With it (ADR-020) the cached demo backup of the same build (`~/.bcartifacts.cache/sandbox/27.5.46862.48827/w1/`)
is restored table by table when a test first touches the table: add `--test-data` to the command above, and
`--top-failures N` to rank the failure messages. Measured on 2026-10-14 (branch `test-data`; 4 threads each for
check, lowering and tests; one run each; under `MemoryMax=4G`):

| Bucket | Test data | Passed | Failed | Not run | Run | Total | Peak memory |
|---|---|---:|---:|---:|---:|---:|---:|
| Tests-SMB | off | 6 | 6 | 1018 | 0.13 s | 2.72 s | 1833 MB |
| Tests-SMB | on | 10 | 3 | 1017 | 0.18 s | 4.29 s | 1826 MB |
| Tests-ERM | off | 227 | 121 | 8985 | 0.24 s | 4.40 s | 2878 MB |
| Tests-ERM | on | 434 | 9 | 8890 | 2.12 s | 6.67 s | 2928 MB |

Those four rows are the morning measurement. Afternoon, main after `fixes-8`, 4 threads, `MemoryMax=4G`: Tests-SMB off 6 passed, 6 failed, 1018 not run (total 4.71 s); Tests-SMB on 12 passed, 3 failed, 1015 not run (4.49 s); Tests-ERM off 241 passed, 149 failed, 8943 not run (6.74 s); Tests-ERM on 487 passed, 5 failed, 8841 not run (13.56 s, 2953 MB).

With test data Tests-SMB loads 2953 rows from 97 tables and Tests-ERM 15352 rows from 167 tables; no table is
refused. The Tests-SMB total includes 1.6 s to open the 860 MB backup on a cold page cache (0.1 to 0.2 s warm). The data costs nothing before a table is touched, which is why the run phase grows only with the tests that
now get far enough to run (ERM: 2.12 s for 434 passing tests instead of 0.24 s).

### Failures with test data

Triage rule (the AL Runner v2 skill): a failure is a data-recipe difference when the runtime did the right
thing for the company it was handed. Of the first nine Tests-ERM failures none were; `fixes-8` (2026-10-14)
dealt with them. Tests-ERM with test data went from 480 passed, 18 failed (main after `fixes-7`) to 484 passed,
5 failed; Tests-SMB is unchanged (11 passed, 3 failed).

| Was | Cause | Now |
|---|---|---|
| 3 tests: `The record in table <Positive Pay Entry / Purch. Comment Line Archive> already exists` | `CreateGuid` had a counter in the last 8 bytes, so `CopyStr(Format(CreateGuid()), 1, 20)` was `{00000000-0000-4000-` for every Guid | Fixed: Guids are version 4 in every byte, from a seeded generator (the host's `random_seed` and a counter) |
| 2 tests: `An error was expected inside an ASSERTERROR statement` (Pos. Pay Test Unit) | Same Guid collision (the setup rows shared one key) | Fixed with the Guids |
| 2 tests: same message (Price List Line UT and Price Worksheet Line UT `T029_ValidateAmountType`) | `fixes-7` and `fixes-8` both fixed it: an overload chosen by arity picked `CopyTo(var Price Asset)` for a `Price Source` argument. `fixes-8` extends the rule to every object a parameter names | Fixed |
| 1 test: `Bal. Account Type must not be   in ...` vs `... must have a value ...`; 6 more once the overload fix ran them (`Product Type must not be (All)`) | `TestField` on an Option or Enum holding its zero member says `must not be <member>`; the corpus does not pin the text | Refused (Unsupported), probe in "Probe batch 6"; 7 tests not run |
| 1 test: `The User Personalization does not exist ... User SID=<name>` | BC creates the row of the session user at sign-in; the corpus does not pin it | Refused, probe in "Probe batch 6" |
| 198 not run: `Format of Record(RefCell { ... })` | A debug dump of the value in the refusal text | The text names the type: `Format of a Record (standard format 0) is not pinned yet`. 213 Tests-ERM tests and 1 more (`FieldRef`) stop there: Base Application compares `Format(Rec) = Format(xRec)` in its Global Triggers subscribers. Probe in "Probe batch 6" |
| 3 (Tests-SMB): `The Office Add-in does not exist` | Unchanged. The test that creates the default add-ins is not run (`Uri.Scheme` is a DotNet value used as an operand), and three later tests of the codeunit rely on its rows. The backup holds no Office Add-in row | ADR-014 question |

`T043_AmountTypeMustBeDiscountForItemDiscGroup` (Price List Line UT and Price Worksheet Line UT) failed because
variables of a table object did not persist per record variable; ADR-021 (`table-globals`) fixes it: Tests-ERM with
test data went from 487 passed, 5 failed to 490 passed, 3 failed.

Still failing in Tests-ERM (3 on 2026-10-14, BC 27.5 build 46862.56320, `al-runner test ... --test-data`; each run with
`--explain TEST`, add `--codeunit ID` when the name is in two codeunits). Triage of issue 5:

| Test | Cause | Kind |
|---|---|---|
| 134979 `AddTextforLanguageInReminderLevelCommunication` ("The TestPage is not open") | Base Application's action "Add New Language" runs `Languages.Editable := false` on a Page variable before `RunModal`. The runtime raised "not open" for any `Page.Editable` on a page that is not running. BC raises no error there, and no corpus test says what it does | Runtime defect, not pinned: now refused (`Page.Editable on a page variable that is not running`), `docs/probes-needed.md` "Page variable Editable". The test is not run until a probe pins it |
| 134994 `AccountScheduleNoIndentationAnyValue` (`Commit` under AutoRollback) | `Initialize` commits once (`IsInitialized`). In BC an earlier test of the codeunit runs it first, under the default transaction model. Here the 9 tests before it are not run (refusals such as `Report.ObjectId`, `DocumentEntries` reportextensions), so this one is the first to run and BC's own error is raised | Not a defect: a consequence of earlier refused tests. It passes once they run |
| 134442 `GLAccountCategoryDelete` (`Income/Balance must be equal to 'Balance Sheet'`) | The library `CreateGLAccountCategory` draws two numbers from `LibraryRandom` (SingleInstance, seed 1 on first use) and the test fails for a draw of 4, 5 or 6 for the account category followed by any additional report definition. On a fresh seed the first two draws of 7 are 2 and 1 (checked), so it passes there. Here several of the tests before it in the codeunit are not run (`Format of a Record`, the `DocumentEntries` reportextensions), so the draws it gets differ from BC's | Not a defect: the random state at the start of the test depends on the tests before it. It cannot be shown to pass or fail in BC without running them |

Without test data Tests-ERM failed 121 tests; the first 30 failures are missing setup data (`General Posting Setup`
must contain records, `Sales & Receivables Setup`, `Tax Setup` and `Purchases & Payables Setup` do not exist, a date
calculation on an undefined date).

## Alone and together (issue 3, 2026-10-14)

The 32 buckets that hold tests, run one at a time and all in one `al-runner test` process, do not give the same
counts. Measured on BC 27.5 build 46862.56320 (the build of the first measurement, 46862.48827, is no longer on
the artifact feed), without test data, `tools/bucket_diff.py` over the JUnit files of both:

| | Passed | Failed | Not run | Tests |
|---|---:|---:|---:|---:|
| Sum of the 32 runs | 1109 | 890 | 38327 | 40326 |
| One process, 32 buckets | 1102 | 852 | 38372 | 40326 |

The same 40326 tests are in both. 39 pass or fail alone and are not run together, 7 pass alone and are not run together, 1 is not run alone
and fails together. Everything else has the same outcome. No test is nondeterministic: the same selection gives the same
result twice.

Every difference has one cause: **what is compiled and reached depends on the selection.** Compilation is demand
driven (ADR-013), and whatever one test reaches is active for all tests of the run: the reportextensions that change a
report, the page extension code of a base page, the subscribers of an event, the install code a baseline root
reaches. A refusal that depends on that set (more than one reportextension, a page control kind, a report property) hits a
test only when some test of the run activates the other part. In BC every installed app is active whatever a test uses,
so the combined run is the closer one, and a test that passes alone only because a part was not active is an accident of the selection.

| Codeunit (bucket) | Tests | Alone | Together | Why |
|---|---:|---|---|---|
| 134485 (Dimension) | 30 | failed (`The General Ledger Setup does not exist`, no test data) | not run | `Document Entries` has two reportextensions that change a run (`Mfg. Document Entries`, `Serv. Document Entries`, both in Base Application). Alone only one is active (`lower.rs`, "more than one reportextension changes the run"). Any second bucket I paired with Dimension made both active, except Tests-SCM-Manufacturing. Which test activates the second one is not pinned down |
| 137101 (SCM-Assembly) | 5 | failed (no setup data) | not run | `Mfg. ItemAvailByLocationLines` (page extension code on base page members, not lowered yet) is active only together |
| 136212 (Marketing) | 3 | failed (no setup data) | not run | column 9 of `Table Relations Metadata` is reached only together |
| 134821 (Cost Accounting) | 1 | failed | not run | page control kind `fixed` of `Apply Employee Entries` is reached only together |
| 134930 | 1 | passed | not run | `Mfg. Adjust Add. Rep. Currency` (reportextension `addafter(Job Ledger Entry)`) is active only together |
| 137462 (Physical Inventory) | 2 | passed | not run | report `Lot No Label` (property `wordmergedataitem`) is active only together |
| 135952 (Upgrade) | 4 | passed | not run | Tests-Upgrade depends on Library Assert only. Alone the baseline runs further (10 of 16 install roots, 49 tables hold rows, 20 unseeded); with any other bucket it is the 5 of 17 roots, 8 tables, 58 unseeded that every other run shows, because `Global Triggers` events are enabled for the tables the install code inserts into (`globaltriggers.rs`; the `GetGlobalTableTriggerMask` subscribers are in Base Application). Alone the Retention Policy install roots reach a different closure, the tables they would fill are not marked unseeded, and an empty table passes where BC holds rows |
| 134562 (Misc) | 1 | not run (`report 'Chart of Accounts' was not compiled`) | failed | the report is compiled only together; it then runs and fails on missing data |

Not found out: which test or bucket activates each part (only the pairing with Dimension was tried), and why alone the
`GetGlobalTableTriggerMask` subscribers are not active in Tests-Upgrade. No code was changed: making activation
independent of the selection (an extended object always activates its extensions at the level of a test that reaches it, as BC
does, or a stable "activate every extension of an activated object") changes which tests of the corpus
pass or are refused, and belongs in its own task with the baselines as the check.

`tools/bucket_diff.py --a <JUnit of each bucket> --b <JUnit of the combined run>` prints the table above for any two
sets of runs. A bucket name with a space needs quoting in the shell.

## All buckets with test data (issue 10, 2026-10-14)

The 32 buckets with tests (every `Tests-*.Source.zip` except `Tests-TestLibraries` and `Tests-Local`) in one
`al-runner test --test-data` process, BC 27.5 build 46862.56320, 4 threads each for check, lowering and tests,
one run. Codeunit 136106 "Service Response Time" is left out (see below): 40315 tests.

| | Passed | Failed | Not run | Peak memory | Total |
|---|---:|---:|---:|---:|---:|
| Issue 10 start (perf-3 merge, with the 40300 tests of the first measurement) | 1862 | 216 | 38222 | 5918 MB | 28.9 s |
| Data-3 merge, 32 buckets (40326 tests) | 2178 | 257 | 37891 | 6566 MB | 28.9 s |
| main at a861bcf | did not finish: over 11 GB, killed (the leak below) | | | | |
| This branch | 2166 | 68 | 38081 | 6419 MB | 36.6 s |

Main had a memory leak that the Tests-SCM-Service codeunit 136106 exposed: it loops over `Calendar Management`,
a codeunit that holds a temporary record and that every call creates anew, and the private table of a temporary
record was kept for the whole session (13 KB per record variable that used it). That one test took 8 GB. Fixed
(`TempId`, `al-vm/tests/temp_tables.rs`). The test itself still does not finish in reasonable time here:
`Library - Service`.`GetFirstWorkingDay` loops over days until a day has a Service Hour row, the restored
CRONUS company has none (`al-bcdb read ... --table "Service Hour"` lists no row), and the loop ends only when
`CalcDate` leaves the date range in the year 9999. In Microsoft's pipeline the rows exist. The figures above
leave the codeunit out by removing its file from a local copy of the bucket archive.

A run no longer hangs on such a test: `al-runner test` stops a test that runs longer than `--test-timeout-secs`
(default 20, 0 = no limit) at its next loop iteration and reports it as not run, with the limit as the reason.
Measured here, the loop does about 7000 days per second (140 microseconds a day, `Calendar Management` builds
temporary records each time), so the 2.7 million days to the year 9999 would take about 6 minutes per test and
the codeunit has five such tests. Real BC would not be quick either (a database round trip per day).

Causes of the failures that remain (68), by message:

| Tests | Message | Cause |
|---:|---|---|
| 17 | `There are no records within the filters specified for table Cost Type` | Data: the restored company holds no Cost Type row (`al-bcdb read ... --table "Cost Type"`), which `Library - Cost Accounting` expects |
| 8 | `The TestPage is not open.` | Skipped by the issue (its own task) |
| 2 | `An error was expected inside an ASSERTERROR statement.` | Microsoft's Tests-Misc T120 to T122 expect a field declared `ObsoleteState = Removed` to be left out of `RecordRef` (FieldCount, FieldExist, Field raises). The runtime lists it. Refusing the calls stopped 252 passing Base Application tests, which call `RecordRef.Field` on such fields and pass in BC, so no refusal is added. Not pinned: `docs/probes-needed.md` |
| 2 | `Tests cannot call the Commit function if TransactionModel property is set to AutoRollback.` | Not a defect: the first test that runs in the codeunit is the one with AutoRollback because the earlier tests are refused (the same cause as in issue 5) |
| 2 | `The payment that you are trying to export is different from the specified Generic Export` | Not looked at |
| 2 | `You cannot create a task of type Meeting because you're not using an on-premises deployment.` | Depends on the deployment type the runtime reports (`docs/probes-needed.md`, environment) |

Not in the list any more, as measured on this main: `Table N already added to backup` (25), `The binding of codeunit
N was unsuccessful` (11), `The Dimension does not exist` (20). Other work fixed them.

## Measured (4 threads each for check, lowering and tests; one run per bucket; under `MemoryMax=3G`; 2026-10-14, afternoon, main after `fixes-8`)

The first measurement of the morning of the same day is in the git history of this file. Since then the platform-table
providers (ADR-019) and `fixes-8` were merged; without test data the sum went from 651 passed, 175 failed, 39474 not run
to the figures below. The machine was shared and loaded, so the Tests/s column varies between runs.

| Bucket | Tests | Passed | Failed | Not run | Compile | Run | Tests/s | Peak memory |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Tests-Bank | 667 | 7 | 1 | 659 | 4.14 s | 0.24 s | 2827 | 1520 MB |
| Tests-Cash Flow | 399 | 26 | 3 | 370 | 4.17 s | 0.24 s | 1678 | 1587 MB |
| Tests-Cost Accounting | 562 | 56 | 26 | 480 | 3.24 s | 0.32 s | 1751 | 1390 MB |
| Tests-CRM integration | 961 | 11 | 6 | 944 | 4.82 s | 0.40 s | 2419 | 1597 MB |
| Tests-Data Exchange | 399 | 27 | 9 | 363 | 4.93 s | 0.89 s | 449 | 1568 MB |
| Tests-Dimension | 976 | 21 | 97 | 858 | 5.48 s | 0.33 s | 2980 | 1946 MB |
| Tests-ERM | 9333 | 241 | 149 | 8943 | 9.22 s | 0.91 s | 10280 | 2903 MB |
| Tests-Fixed Asset | 462 | 6 | 0 | 456 | 4.90 s | 0.26 s | 1810 | 1513 MB |
| Tests-General Journal | 825 | 11 | 24 | 790 | 4.42 s | 0.19 s | 4375 | 1669 MB |
| Tests-Graph | 77 | 4 | 0 | 73 | 4.31 s | 0.24 s | 321 | 1418 MB |
| Tests-Integration | 365 | 10 | 0 | 355 | 5.64 s | 0.35 s | 1051 | 1661 MB |
| Tests-Job | 1289 | 10 | 3 | 1276 | 5.54 s | 0.24 s | 5266 | 1736 MB |
| Tests-Marketing | 737 | 4 | 13 | 720 | 5.34 s | 0.21 s | 3511 | 1655 MB |
| Tests-Misc | 3245 | 358 | 167 | 2720 | 7.70 s | 6.89 s | 471 | 2434 MB |
| Tests-Monitor Sensitive Fields | 19 | 3 | 2 | 14 | 2.96 s | 0.20 s | 97 | 1250 MB |
| Tests-Permissions | 105 | 4 | 2 | 99 | 4.34 s | 0.22 s | 487 | 1664 MB |
| Tests-Physical Inventory | 157 | 35 | 6 | 116 | 3.05 s | 0.15 s | 1041 | 1401 MB |
| Tests-Prepayment | 567 | 0 | 4 | 563 | 3.63 s | 0.17 s | 3430 | 1689 MB |
| Tests-Rapid Start | 385 | 71 | 2 | 312 | 3.56 s | 0.57 s | 672 | 1575 MB |
| Tests-Report | 781 | 3 | 1 | 777 | 3.42 s | 0.22 s | 3568 | 1698 MB |
| Tests-Resource | 346 | 9 | 35 | 302 | 2.36 s | 0.15 s | 2255 | 1513 MB |
| Tests-Reverse | 198 | 0 | 1 | 197 | 1.66 s | 0.10 s | 2063 | 1385 MB |
| Tests-SCM-Assembly | 1764 | 5 | 48 | 1711 | 2.64 s | 0.15 s | 11708 | 1793 MB |
| Tests-SCM-Manufacturing | 1179 | 8 | 61 | 1110 | 3.41 s | 0.16 s | 7240 | 1739 MB |
| Tests-SCM-Service | 1917 | 20 | 47 | 1850 | 3.44 s | 0.16 s | 11956 | 1908 MB |
| Tests-SCM | 8295 | 61 | 123 | 8111 | 5.63 s | 1.10 s | 7518 | 2660 MB |
| Tests-SINGLESERVER | 922 | 29 | 4 | 889 | 5.20 s | 0.71 s | 1290 | 2126 MB |
| Tests-SMB | 1030 | 6 | 6 | 1018 | 4.96 s | 0.30 s | 3391 | 1914 MB |
| Tests-Upgrade | 12 | 11 | 0 | 1 | 2.83 s | 0.36 s | 33 | 1403 MB |
| Tests-User | 73 | 0 | 0 | 73 | 3.89 s | 0.19 s | 391 | 1591 MB |
| Tests-VAT | 1197 | 8 | 51 | 1138 | 2.95 s | 0.10 s | 12240 | 1838 MB |
| Tests-Workflow | 1056 | 25 | 5 | 1026 | 2.49 s | 0.15 s | 7075 | 1801 MB |
| Sum of the 32 buckets | 40300 | 1090 | 896 | 38314 | | | | |

All 32 buckets in one process (8 GB cap, 4 threads; 2026-10-14, afternoon, main after `fixes-8`; three runs, the same counts each time):
40300 tests, **1083 passed, 858 failed, 38359 not run**. Load 0.40 to 0.48 s, compile 12.1 to 12.9 s, run 4.1 to 5.4 s
(6762 to 9885 tests/s), total 16.8 to 19.1 s, peak memory 5.8 to 5.9 GB. The per-bucket sum above (1090, 896, 38314) differs from
this one-process run; the cause of the difference is not investigated. The one-process figures are the ones to quote.
Morning figures of the same day (before the platform-table round): 651 passed, 175 failed, 39474 not run; total 21.17 s.
`Tests-TestLibraries` (a library) and `Tests-Local` (empty in the W1 artifact) are left out.

Threads for Tests-ERM (compile / run / total): 1 thread 14.62 s / 0.86 s / 15.92 s; 4 threads 7.64 s / 0.37 s / 8.48 s;
12 threads 5.69 s / 0.31 s / 6.38 s. Peak memory 2.6 to 2.8 GB at every thread count.

Compile dominates: the demand compile of Base Application for the test codeunits of the bucket. The run is
sub-second except for Tests-Misc (5.4 s: 229 tests really run).

## "Outside the workspace" refusals (branch `bucket-demand`, 2026-10-14)

All 32 buckets in one process, before and after (same artifact, same flags). Before: 2939 tests refused as outside
the workspace (global table 1968, global codeunit 500, table 330, page 71, report 32). After: none. The cause was
never the demand-driven reach. A dependency file that does not parse takes all its objects out of the compile, and
name lookup then says "outside the workspace". The fixes are parser fixes (`Option` return type with no members,
`trigger Var::Event`, `ControlAddIn Name`, control add-in `event`), namespace-qualified data item tables, and the
message for platform tables with a dot in the name. The details are in ADR-013, "Outside the workspace on
Microsoft's test buckets". To find the next one, run a bucket with `AL_EAGER_DEPENDENCIES=1` (parses every file;
Tests-SMB: 3.4 GB peak, 8 s) and read the `parse:` lines; a normal run prints them for the files it reached.

## Where the time of the slow buckets went (2026-10-15, branch `scm-service-time`)

32 buckets one at a time with test data, BC 27.5, `--timings`, load average 9 to 20: sum of the totals 372 s before, 224 s after.

- **Tests-Cost Accounting 17.6 s -> 1.5 s** (run phase 17.1 s -> 0.8 s). One test (`GLTransferWithAmountZero`, refused after its setup) took 17 s alone: every step of a loop over a table (`FindSet`/`Next`) looked at all rows of the table, and each comparison of two integers went through a `Decimal`. `MemStore::next` now steps through the table in primary key order, integer, decimal and text keys compare directly, and the other keys compare without building the key of each row. A walk over 30,000 rows in primary key order: seconds, where it took minutes (`cargo test --release -p al-data -- --ignored`: two walks 442 s -> 49 s, the rest being the secondary key walk, which is still one pass over the table per step).
- **Tests-SCM-Service stays at about 105 s**, of which 102 to 112 s are codeunit 136106 `Service Response Time`: five tests stopped by the 20 s per-test CPU limit (`ResponseTimeInSameWorkingDay`, `ResponseTimeSameWorkingDayFull`, `ResponseTimeTwoSimulWorkingDay`, `ResponseTimeWorkingNonWorking`, `ResponseTimeValidOnHolidays`). `Library - Service.GetFirstWorkingDay` loops over days until a day has a Service Hour row; the restored CRONUS company has none, so the loop runs to the year 9999 (about 2.7 million days). Real BC does the same on this database and would need hours. Not a runtime loop bug: missing demo data. The rest of the bucket takes 3 s.
- Tests-Workflow printed no `Tests:` or `Time:` line to a `grep -I` / ugrep (the shell function in the agent shells) because one failure text holds a NUL (`Expected:<asdf\0>`), which made the whole output a binary file to those tools. Not a crash or an out-of-memory kill: ten reruns finished with exit 1 and all lines. A control character in a failure text is shown as `\x00` now.

## Package mode (`--company-data package`) measured

Tests-ERM (BC 27.5.46862.48827), same binary, backup company against package company:

| Company | Rows read | Reader refusals | Passed | Failed | Not run |
|---|---|---|---|---|---|
| backup | 38904 in 283 tables | 0 | 6980 | 124 | 2229 |
| package, before | 12627 in 300 tables | 2 (`To-do.Duration`, `Send Reminders Setup.MinimumTimeBetweenSending`: XML durations such as `P1DT0H0M0.0S` were read as whole numbers) | 6974 | 161 | 2198 |
| package, after | 12687 in 302 tables | 0 | 6974 | 161 | 2198 |

The package has fewer rows than the backup (it is the DemoTool's Extended data, not the sandbox company), so some tests
that pass on the backup fail on it and the other way round; that is the Legacy lane's data, which is what Microsoft's
Legacy buckets run on. The two tables the reader used to refuse are now read, and no test of this bucket touches them,
so the counts did not change. The largest package-only failure is `The Country/Region does not exist ... Code=''` (40
tests, first: ERM Report Amount in FCY, `AmountInFCYOnDocumentEntriesReport`). Explained (2026-10-18, rerun of the
bucket: still 40, same text): it is not a gap of the package reader. `ERMFinancialReportsIII.Codeunit.al:1966`
(`UpdateIntrastatCountryCode`) does `CompanyInformation.Get()` and then `CountryRegion.Get(CompanyInformation."Country/Region Code")`.
The package (`NAV27.5.W1.ENU.EXTENDED.rapidstart`) holds 68 Country/Region rows but no Company Information table at all, so
the Company Information row is the one `Company-Initialize` writes, with a blank country. The backup company has the
demo tool's Company Information row (a country set), which is why the same tests pass there. A company built from this
package therefore fails these 40 tests on any host that builds it the same way. Whether Microsoft's Legacy lane has a
Company Information country from some other step is not known here; it needs a probe on real BC (build a company
from the package and read Company Information."Country/Region Code"). Until then these 40 stay as package-only failures.

## Microsoft's recipe: `--recipe microsoft` (2026-10-18, branch `ms-recipe-3`)

`al-runner test <Tests-*.Source.zip> ... --recipe microsoft` runs the buckets the way BCApps' CI does.

1. The BCApps files come from `releases/<major>.<minor>` of github.com/microsoft/BCApps (MIT), fetched with `git`
   (shallow, no file contents, only `src/DisabledTests`, `build/projects`, `build/scripts`) into
   `<cache dir>/bcapps/<major>.<minor>/` once. The release is `--bc-version` (major.minor), else the build of the tests.
   `--bcapps DIR` names a checkout instead. `--no-download` reads only the cache. `ALRUNNER_BCAPPS_URL` replaces the
   GitHub address (a mirror).
   Measured 2026-10-18: `releases/27.x` and `releases/28.x` do not have the BaseApp test recipe at all (no `Test Apps W1`
   project, no `TestConfiguration.json`, no `src/DisabledTests`); `releases/29.0` is the first that does. A release
   without it uses `releases/29.0`, and the run prints that. Disabled tests are matched by codeunit id and method
   name, so entries for tests a 27.5 artifact lacks do nothing.
2. The lane of a test codeunit: a test app in `TestConfiguration.json` (`LegacyTests-Bucket1`, `-Bucket2`) is Legacy,
   whatever the codeunit says. For any other app the AL property `TestType` of the codeunit decides:
   `UnitTest` or none, `IntegrationTest`, `Uncategorized`; `AITest` is in no build mode and is left out. A codeunit
   without the property runs in the UnitTest lane: checked 2026-10-22 against Microsoft's CI (BCApps `releases/29.x`,
   run 39519586834), where the Default job ran 242 such codeunits and the UncategorizedTests job none.
3. Company per lane: Legacy runs `--company-data package` (company name from the settings, CRONUS International Ltd.);
   UnitTest runs without demo data (Company-Initialize and the installs only). IntegrationTest and Uncategorized need
   Microsoft's Contoso Demo Tool (`CreateSetupDemoData`, `CreateAllDemoData`, then the tenant data codeunit 5691, which also
   creates the Web Service rows). The runner does not run it yet, so those lanes print `NOT RUN` with the reason and the
   exit code is 7.
   A codeunit with `RequiredTestIsolation = Disabled` runs only in the UnitTest lane: BCApps
   `build/scripts/RunTestsInBcContainer.ps1` runs a second pass with `requiredTestIsolation = Disabled` (test runner
   130451) for `UnitTest` only, and in run 39519586834 the IntegrationTests job ran none of its 13 such codeunits.
   `al-runner test-microsoft --list-lanes` prints the lane of every codeunit without compiling or running anything.
4. Microsoft's disabled tests are skipped (`--parity`) and counted per lane. Each test app reads only its own folder,
   `src/DisabledTests/<app name with _ for spaces>/*.json` (`Get-DisabledTests -AppName` in the same script): the
   folder of E-Document Core Tests disables codeunit 139235 with `*`, and Microsoft still runs its 105 tests in PEPPOL
   Tests. Each lane has its own summary and, with
   `--junit FILE`, its own `FILE.<lane>.xml`.

Time limit per test (2026-10-22): Microsoft's recipe sets no time limit per test, so `--recipe microsoft` (and
`test-microsoft`, `--affected-from microsoft`) keeps AL Runner's default of 20 s (`--test-timeout-secs`). Checked in the
BCApps files the recipe fetches (`releases/29.0`): `build/scripts/RunTestsInBcContainer.ps1` passes no timeout to
BcContainerHelper's `Run-TestsInBcContainer`; the only timeouts in `build/scripts` are for waiting on a free tenant
(`ParallelTestExecution.psm1`, `Wait-ForFreeTenant`, 7200 s), web requests and the client session of the test data
import (`ImportTestDataInBcContainer.ps1`, 2 hours); `build/projects` and `TestConfiguration.json` have none. The test
runner of the artifact (`TestFramework/TestRunner`) has one limit, the client transaction timeout of 10 minutes in
`Internal/ALTestRunnerInternal.psm1` (`DefaultTransactionTimeout`), and it applies to one `RunNextTest` call, which
runs a whole test codeunit (`TestSuiteMgt.Codeunit.al`, `RunNextTest`), not one test. A test whose `Sleep` calls ask
for more than 20 s is therefore a timeout row under the recipe too: Tests-Dimension `TestGetShortcutDimCached`
(codeunit 134482, `Sleep(60500)`) passes on BC and is `Test exceeded 20s timeout.` here. Pass `--test-timeout 120`
(or `0`, no limit) to run it; measured 2026-10-22 on BC 29.0.54011.55816: it passes with `--test-timeout 120`.

Company name per lane (2026-10-18, `ms-recipe-4`): a lane without a restored company sets the run's company name
(`TestArgs.company_name`, `al_testrun::host::set_company_name`), so `CompanyName()` answers `Empty Company` in the UnitTest lane
(and `My Company` once the Contoso lanes run). The name is part of the baseline cache key. Legacy answers the company of
the package.

### Contoso lanes: spike result (2026-10-18)

The Contoso Coffee Demo Dataset app (`Microsoft_Contoso Coffee Demo Dataset_<build>.app`, source zip in the artifact)
resolves as a dependency of an ordinary test app, and its code compiles without a static refusal. A scratch test app with
one test per module and level (16 modules x Setup/Master/Transactional/Historical, calling each module codeunit's
`Create<Level>Data`) shows where the run stops. Ranked:

1. `DotNet new System.Drawing.Bitmap` (refused by policy: needs the Windows drawing stack), reached from `Foundation Module`
   `CreateSetupData` -> `Create Company Information`.CreateSetupCompany (CreateCompanyInformation.Codeunit.al:76) ->
   Base App `O365 Brand Color`.CreateDefaultBrandColors/MakePicture (O365BrandColor.Table.al:124). This is the first step of
   `CreateSetupDemoData`, so it blocks both lanes and hides what follows (13 further tests fail only because the Foundation
   data is missing).
2. `builtin System.DWY2Date#1 is not implemented`: 7 tests (for example `Bank Module` `CreateHistoricalData`). Implemented
   on 2026-10-18 as pinned by types/lt-datetime/DateFuncs.al:311-399; the spike no longer shows it.
3. `DotNet new System.Net.Mail.MailAddress` (refused by policy: touches the network): `CRM Module` `CreateMasterData`.

Behind the Bitmap refusal (2026-10-18, scratch run with the `MakePicture` body cut out of the Base App source, not committed;
ADR-044 is the proposal for Bitmap itself). 64 tests: 52 pass, 7 fail, 5 not run. In order:

1. `NavApp.GetResource` of the Contoso app finds nothing: 5 tests (`PostingExchangeDefinitions/OCRCREDITMEMO.xml`, `CRONUS.jpg`,
   `Images/Item/1896-S.jpg`, `Images/Person/*.jpg`, `IncomingDocuments/*.PDF`). The `.app` holds them under `resources/` with
   percent-encoded names (`%20`); resources of dependency apps are only loaded for corpus apps (`app.resources` is set in
   `al-testrun/src/runtime/corpus.rs`). A runtime task: load and decode `resources/` of dependency packages.
2. `platform table 'License Permission' (2000000043) has no provider`: `Sales Module` `CreateHistoricalData`.
3. `DotNet new System.Net.Mail.MailAddress` (policy, network): `HR Module` `CreateMasterData`. Needs an owner decision.
4. `Unhandled UI: Message Shipment Date 03/29/26 is before work date 10/18/26`: `Sales Module` `CreateTransactionalData`. The
   demo data dates are relative to a work date; check what the recipe sets as WorkDate before treating it as a bug.
5. `Vendor Item No. already exists` (`Inventory` transactional), `Status must not be Certified` (`Manufacturing`) and
   `Opportunity Entry already exists` (`CRM`): may follow from the earlier failures in the same codeunit; unverified.

After ADR-044 option 3 (2026-10-18, branch bitmap-noop): the `System.Drawing.Bitmap` refusal is gone. `MakePicture` runs, the
picture is not drawn, and a read of it is refused. The same spike (64 tests, BC 27.5) gives 52 passed (5 flagged by the
assumption `bitmap-picture-not-drawn`), 7 failed, 5 not run, the numbers of the scratch run above. The first refusals now are
`platform table 'License Permission' (2000000043) has no provider` and `DotNet new System.Net.Mail.MailAddress`, then the
`NavApp.GetResource` of the Contoso app.

More refusals may sit behind the first one. Each is a runtime task (a corpus pin is needed first for DWY2Date; the two
DotNet types need an owner decision on the policy or a native replacement). The two lanes stay refused with the first
reason in their line. The company-setup roots in `baseline.rs` were not added: they would run a refused step on every run.

### Contoso lanes: second spike (2026-10-18, `contoso-2`)

Changed since the first spike: `NavApp.GetResource` of a dependency app now finds its packaged resources (a compiled `.app` holds
them under `resources/` with `%20` names, decoded once like the `.al` entry names; a source archive uses the `resourceFolders`
of its `app.json`); an xmlport field bound as `"Data Exch. Def".Code` (a dot inside the quoted table name) now binds; platform
table `License Permission` answers its two pinned rows.

WorkDate: the recipe does not set one. BCApps `Invoke-ContosoDemoTool` runs Company-Initialize (codeunit 2), then
`CreateSetupDemoData` / `CreateAllDemoData` (codeunit 5193) and the tenant data (5691) through OData, so WorkDate is today, as in
`al-runner` (session/TestSessionFunctions.al:53-65). The demo dates are relative to the Starting Year (today's year minus one), so
a sales shipment dated in the past gives the message "Shipment Date ... is before work date ..." on real BC as well. The OData
call ignores the message; a test method does not. The `Unhandled UI` failure of the first spike was a spike artifact: a
`[MessageHandler]` on the test makes `SalesTransactional` pass. The lanes need a message handler (or UI off) around the roots.

The first spike ran on the CRONUS company (the default). That is the wrong base: CRONUS already holds item attributes, vendors and
item references, so `Color already exists` and `Vendor Item No. already exists` are collisions with CRONUS data, not runtime bugs.
The Contoso lanes run on an empty company (`--no-test-data`, as BCApps does). Run that way, with the scratch cuts listed below,
the real order (all Setup, then Master, Transactional, Historical) stops at, in order:

1. `Windows Language`: the Contoso language setup filters the table by "Abbreviated Name"; only 1031, 1033 and 2057 are pinned.
   Needs a probe that lists the whole table on real BC.
2. XmlPort `Imp / Exp Data Exch Def & Map` (1225), imported for 7 data exchange definitions from the Contoso resources:
   `Encoding = UTF8` (export is pinned by xmlport/W7XmlPortTests.al:345-364, import is not), `MinOccurs` on a `tableelement`,
   `Occurrence = Optional` on 53 field attributes (no corpus test uses `Occurrence`), `FieldValidate`, `SourceTableView`, and an
   empty `requestpage` section (refused even when empty). Without the definitions the Bank module fails on a missing
   `SEPA CAMT` row.
3. `License Permission` `Codeunit` 410 (above), `Language Selection` (2000000050; PtCloudTables.al pins it, no provider yet),
   `Media Resources` (2000000182, tenant data), `System.Net.Mail.MailAddress` (policy).
