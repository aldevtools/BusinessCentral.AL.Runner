# Corpus writer guide

For an agent that writes probe tests for the corpus without the context of the person who started this. Read it fully once, then work from the checklists.

## What you do

al-runtime-rs may only implement AL behaviour that a corpus test pins (CLAUDE.md, hard rules 3 and 4). A "probe" is a test in the corpus repo that was **measured on real Business Central** and asserts exactly what BC did. You write probes for the open rows of `docs/probes-needed.md`, or for a whole topic (all members of a type with their edge cases and error texts).

**Rules.** Never `pkill` or `killall`, never `git stash`, never `git commit -a` or `git add -A`. Never wait more than 20 minutes on a container or CI: write `RESUME.md` in your scratch folder and hand back WAITING. Commit messages end with the session attribution line your task gives you. Put nothing local (host names, URLs, user names, home paths, partner names) in a commit or pull request. Extend the existing topic: check for duplicates first, add tests to the existing codeunits of the area, never edit existing assertions or inputs. The first line of your report is `[corpus / <task> / DONE | WAITING | BLOCKED]`. Your hand-over names the local object reason line: why each object id and name you added is free (checked as in "Object ids and names").

You never change al-runtime-rs code. You write AL in the corpus repo, measure it on a local BC 29 container, make it green there, and hand the branch over.

## Repos and branches

| What | Where |
|---|---|
| Corpus repo | `https://github.com/aldevtools/BusinessCentral.AL.Language.Tests` (local clone, call it `$CORPUS`) |
| Corpus branch the runtime pins | `master`, the default branch (since 2026-10-19; before that the side branch `al-runtime-rs`). A ruleset protects it: changes only through a pull request with the 10 required checks green, no direct push, no force push, no delete |
| Integration branch | `al-runtime-rs-staging` (the publisher merges green writer branches here; the CI batcher merges writer commits onto it for test runs) |
| Your branch | `al-runtime-rs-staging-<letter>` (local writers) or `al-runtime-rs-writer-<name>` (the laptop), created from the current `al-runtime-rs-staging`; merge staging into it before you hand it over |
| Local BC container | `https://github.com/aldevtools/MsDyn365Bc.On.Linux` (call it `$BCLINUX`) |
| Tools for this guide | `tools/corpus-writer/` in al-runtime-rs |

Never push to `master` or `al-runtime-rs-staging`, and never open a pull request to `master` (the publisher does that, see "How a bump lands on master"). Never edit or delete an existing test (add new ones). Never put secrets in files.

### How a bump lands on master

For the publisher and the orchestrator. Writers only hand over their branch.

1. Make the bump branch from `origin/al-runtime-rs-staging`: `al-runtime-rs-staging-pub<n>`. If `git merge-base --is-ancestor origin/master origin/al-runtime-rs-staging` fails (an outside pull request was merged into `master`), merge `origin/master` into the bump branch first. The pull request run tests the bump branch merged with `master`; the pinned commit must hold every file that run tested, and `tools/corpus_bump.sh` refuses the pin otherwise.
2. Merge the green writer branches into the bump branch and push it.
3. Open the pull request from the bump branch into `master`. Its CI run (event `pull_request`) is the matrix of the bump: legs 27.5, 28.5 and 29.0 of the Cloud, OnPrem and OnPrem DotNet jobs, plus the object-id gate. These ten checks are the required ones. Fix on the bump branch and push until the run is green; do not dispatch a second matrix run for the same commit. Never open the pull request from `al-runtime-rs-staging` itself: the repository deletes the head branch of a merged pull request.
4. When the run is green, merge with a merge commit: `gh pr merge <N> --merge`. Never squash or rebase: both rewrite the commit, and the run's commit would then not be on `master`.
5. Fast-forward staging to `master`, so writers and the CI batcher start from the new state: `git push origin origin/master:refs/heads/al-runtime-rs-staging` (check `git merge-base --is-ancestor origin/al-runtime-rs-staging origin/master` first).
6. In al-runtime-rs, `tools/corpus_bump.sh <run id>` with the id of that pull request run. It checks that the run's commit is on `master` and that the merge commit has the same files as the run's commit.

Outside contributors open pull requests to `master` as before. They merge with a merge commit too; step 1 then brings their commits into the next bump branch.

The corpus app is `tests/al-language` (Cloud target, BC 27 symbols, runtime 16.0). Two more apps exist: `tests/al-language-onprem` (Target OnPrem: for `Scope = OnPrem` objects such as `Database.SID`, `Report.SaveAsPdf`, `Environment Info Test Library`) and `tests/al-language-onprem-dotnet` (needs AL compiler 18 or newer).

## Object ids and names

Every object id in the corpus app must lie in `idRanges` of `tests/al-language/app.json` and be unique in the whole tree (`.github/scripts/check-object-ids.py` checks both and runs in CI). Ranges per writer:

| Writer | Ids |
|---|---|
| Publisher (existing objects) | 60000-60999, 67000-69999 |
| Laptop writer | **70000-74999** (already in `idRanges` on `al-runtime-rs-staging`) |

Use ids only from your own range. Object **names** must be unique per object type in the app and at most 30 characters: give them a short prefix of your own (for example `LT `) so they cannot clash with the publisher's (`PA `, `W1 ` to `W7 `, `PX `, ...). Both gates fail the build; the object-id gate also catches two branches that use the same id once they are merged.

## Local setup (once)

You need Docker, Python 3, .NET (for the AL compiler) and about **12 GB of free RAM**: the BC container needs about 5-6 GB and the SQL container keeps its database in a 4 GB tmpfs. The first start downloads about 3 GB of artifacts and takes 5-10 minutes; later starts reuse the cache volumes. Linux is the supported host. On macOS use `docker-compose.macos.yml` and `MacOS.md` of bc-linux; on Windows use WSL2 with Docker Desktop and run everything inside WSL (not tried by the publisher).

1. Clone bc-linux and the corpus; check out your branch:
   ```bash
   git clone git@github.com:aldevtools/MsDyn365Bc.On.Linux.git ~/bc-linux
   git clone https://github.com/aldevtools/BusinessCentral.AL.Language.Tests ~/corpus
   cd ~/corpus && git checkout -b al-runtime-rs-writer-<name> origin/al-runtime-rs-staging
   ```
2. Start **BC 29.0.54011.55816** (the build the CI legs pin; same platform as the oracle):
   ```bash
   cd ~/bc-linux
   BC_VERSION=29.0 BC_ARTIFACT_URL=https://bcartifacts-exdbf9fwegejdqak.b02.azurefd.net/sandbox/29.0.54011.55816/w1 docker compose up -d --wait
   ```
   The database is wiped when the containers are recreated (`docker compose down`, then `up`). That is how you get a **fresh install** (see pitfalls).
3. Install the AL compiler and stage the symbols the corpus needs:
   ```bash
   cd ~/bc-linux
   eval "$(bash scripts/install-al-compiler.sh 29.0)"          # prints AL_BIN_DIR; put it on PATH
   export PATH="$AL_BIN_DIR:$PATH"
   ARTIFACTS=<the artifact dir inside the bc volume, or the folder download-artifacts.sh used>
   python3 scripts/stage-symbols.py --app-json ~/corpus/tests/al-language/app.json --artifact-dir "$ARTIFACTS" --out-dir ~/corpus/tests/al-language/.alpackages
   ```
   (`al-compile` from `StefanMaron/al-smart-compile` is an optional wrapper that does the symbol lookup for you; `run-corpus.sh` uses it when it is installed.)
4. Publish the fixture app the corpus depends on (once per fresh database):
   ```bash
   cd ~/corpus/tests/al-language-internals-fixture && AL compile /project:. /packagecachepath:../al-language/.alpackages /out:fixture.app
   cd ~/bc-linux && bash -c '. scripts/publish-app.sh; bc_publish_app ~/corpus/tests/al-language-internals-fixture/fixture.app'
   ```
5. Check the setup with a known test: `export CORPUS=~/corpus BCLINUX=~/bc-linux; SYMS=BC27PLUS,BC28PLUS,BC29PLUS tools/corpus-writer/run-corpus.sh 69100` (in al-runtime-rs). Expect "passed 1 failed 0".

Timings on a laptop-class machine: compiling and publishing the corpus app takes 1 to 2 minutes per run, running one test codeunit a few seconds, a batch of 100 to 150 tests about 5 minutes with observe, verify and fixes.

Environment variables used by the tools: `CORPUS` (corpus clone), `BCLINUX` (bc-linux clone), `WORK` (scratch folder, default `~/.cache/corpus-writer`), `DRAFTS` (folder with your draft AL files, default `$WORK/drafts`), `SYMS` (preprocessor symbols; always `BC27PLUS,BC28PLUS,BC29PLUS` on the BC 29 container so `#if BC29PLUS` code is compiled the way CI compiles it for 29).

## The loop: observe, then assert

You write tests that only **observe**, run them once on the container, and a script turns what BC did into assertions. You never type an expected value by hand.

1. **Draft** the AL in `$DRAFTS/<topic>/` (any sub folders). Every test ends with `Observer.Finish();` and records facts with the helper `"Probe Observer"` (codeunit 69160, in `tests/al-language/_fixtures/ProbeObserver.Codeunit.al`):
   ```al
   [Test]
   procedure Label_Split_OneSeparator()
   var
       Observer: Codeunit "Probe Observer";
   begin
       Observer.Observe('parts', Format(MyLabel.Split(',').Count()));   // a fact: name and value (Text)
       asserterror SomethingThatShouldFail();
       Observer.ObserveError('name');                                   // records text and code of the error, clears it
       Observer.Finish();                                               // ends the observation form
   end;
   ```
   Good examples to copy the style from: `tests/al-language/types/PXEnumRecordRef.al`, `types/PRDecimalToInteger.al`, `types/PUTestFieldErrorInfo.al`, `pages/extensions/PCCurrRecordReads.al` (pages, logs), `queries/PRQueryOpenColumn.al`.
2. **Run the pipeline** (observes, converts to assertions, verifies):
   ```bash
   export CORPUS=~/corpus BCLINUX=~/bc-linux SYMS=BC27PLUS,BC28PLUS,BC29PLUS
   python3 tools/corpus-writer/pipeline.py <topic> <fixes.py|-> "<test codeunit ids, a|b|c>"
   ```
   The test codeunit ids: `python3 tools/corpus-writer/testids.py $DRAFTS/<topic>`. `pipeline.py` copies the drafts into the corpus app, switches the helper to its observing variant, runs every test (they fail on purpose with `OBSERVE:` messages), rewrites each `Observer.Finish();` into `Observer.Expect(...)` lines plus `Observer.Verify();`, switches the helper back, and runs the result. It prints `0 failing` when everything is green and `FAIL ...` lines otherwise. Long observations continue in generated `_More` tests; the script handles it.
3. **Fix what failed** in `fixes.py` (a Python file the pipeline runs after copying; helpers in `tools/corpus-writer/fixes-lib.py`: `drop_test`, `sub`, `noerr`). Typical causes: a statement you thought would raise does not (turn `asserterror X; ObserveError` into `X; Observe('..','no error')`), a test that fails by itself (drop it), a value that varies per run (drop the observation or hide it, see rules). Repeat step 2.
4. **Check the file headers**: each new file starts with a comment block in the style of the existing files (BC Documentation link, Scope, Fixtures used, "Probe: written against ...", the facts in plain words, BC versions).
5. **Run the repo gates** and commit:
   ```bash
   cd ~/corpus
   python3 .github/scripts/check-object-ids.py && python3 .github/scripts/check-singleinstance-fixture-owners.py && python3 .github/scripts/check-install-triggers.py .
   for t in .github/scripts/test-*.py; do python3 $t || break; done
   ```
   Commit small and often, message `test: <what is pinned>`.

Compile-error facts ("does X compile, and which diagnostic") are **must-not-compile cases**, not tests: `tests/must-not-compile/README.md` explains the format; `tools/corpus-writer/mnc-fill.py scratch.al tests/must-not-compile/cases/NNNN-name.al` runs the compiler and writes the case with the real diagnostic (needs `AL` and `SYMBOLS` environment variables: the AL compiler command and the symbol folder). Cases must hold on AL 16, 17, 18 and 30 (`.github/scripts/check-must-not-compile.py`).

## Rules learned the hard way

Each of these broke a published batch.

1. **Assert only the asked fact.** No dumps of rows, datasets or metadata, no counts of Microsoft table rows, no generated ids (control, action, data item, column, task, test ids), no GUIDs, no company name or id, no user, session or host values, no dates of today, no app or platform versions, no time-of-day, no runner or identity values (`ClientType`, `UserId`, client computer name, error stack frames of the runner), no BC build texts. Prefer relations (equal, not equal, before, after).
2. **No tests that test the runner**: anything that reads differently when the test host differs (websocket runner on 27, altool on 28 and 29): the object named in "tried to invoke" errors, "UI handlers were not executed", the stack below the test procedure, session state at the start of a test, lock timeouts against a background session, TestPage field ids.
3. **Never write a codeunit with `Subtype = Install` or `Upgrade`.** An install trigger runs when the app is published; one that raises breaks the app on every BC version and turns the whole matrix red. `check-install-triggers.py` rejects them.
4. **`ApplicationArea = All;` on every page that a TestPage opens** (BC 28 and later cannot find controls otherwise).
5. **Version differences are real**: guard them with `#if BC29PLUS` / `#if BC28PLUS` / (`not BC28PLUS` for 27) and assert both variants; or, when only the value of a platform table differs, key it on the version like `record/PFFeatureKeyColumns.al` does (`ApplicationSystemConstants` text is `W1 29.0` - country first).
6. **A test codeunit must contain at least one `[Test]`** (an empty one makes the run report "no test results" and fails the leg). Helper `[Test]`-less codeunits are fine.
7. **A SingleInstance codeunit (a log) must be used by exactly one test codeunit** (`check-singleinstance-fixture-owners.py`). It also counts a codeunit as using everything that appears LATER in the same file: put tables and pages that declare log variables above the first codeunit of the file.
8. **No writes inside a `[TryFunction]`** (the test container sets `DisableWriteInsideTryFunctions`); use `asserterror`. `asserterror` rolls back the writes made inside it and, in a test, the earlier uncommitted ones.
9. **Rows persist between the tests of one codeunit.** Every test starts with its own `DeleteAll()` of the tables it uses. A duplicate-key insert into a table without a BLOB raises only at the end of the test; never leave one pending.
10. **An `asserterror` around a statement that does not raise fails the whole test**; when unsure, observe the plain statement and, if needed, give it its own test. A `[HandlerFunctions]` handler the test does not trigger fails the test.
11. **Cloud target only in `tests/al-language`**: `Scope = OnPrem` objects give AL0296 there (a lot of platform tables, `File`, `TemporaryPath`, ...). Put those probes in the OnPrem app.
12. **Own tables for every probe**; never write to Microsoft tables. Never create or delete companies, never touch the encryption key (the database is shared by every test).
13. A test must restore session state it changed (`WorkDate`, `CurrentTransactionType`, application area, language).
14. Keep one fact family per test, at most about 25 observations; many small tests beat one large one.

## Pitfalls

- **Upgrade versus fresh install.** `run-corpus.sh` bumps the app version on every run, so after the first run the container does an *upgrade*: install triggers do **not** run. A broken install trigger only shows on a fresh database. The gate rejects install codeunits, so you should not have any; if you ever add one on purpose, test it with `docker compose down && docker compose up -d --wait` (wipes the database; republish the fixture) and then one run.
- **A lower or equal app version is silently ignored** by the container. The script keeps the counter in `$WORK/ver.n`; if the container was recreated, delete it.
- **Compile with the compiler of every CI leg before each push.** CI compiles BC 27 with AL 16.x, BC 28 with AL 17.x and BC 29 with AL 18.x, each with its own preprocessor symbols. Code that the 29 compiler accepts can fail on 16.x or 17.x (example: an Integer FlowField over a BigInteger field gives AL0427 on 27 and 28 only). That turns every 27 and 28 leg red at compile time with no test run. `CORPUS=~/corpus BCLINUX=~/bc-linux tools/corpus-writer/compile-legs.sh` installs the three compilers (through `install-al-compiler.sh 27.0`, `28.0`, `29.0`), compiles the Cloud app once per group with the right symbols and fails on any error. Run it after the BC 29 loop and before every push to your branch.
- **Absolute dates from `Today` depend on the run date.** A test that prints or compares a date derived from `Today`, `WorkDate` or `CurrentDateTime` against a literal (for example `Expect(..., '10/15/26')`) passes only on the day it was written. Derive the expected value from `Today` or `WorkDate` instead. The al-runtime-rs harness starts each oracle codeunit's clock at its suite `timestamp` as a safety net, but real BC does not.
- Do not run two compile/publish runs at the same time against one container.
- If the container is out of memory the whole machine suffers: close other heavy programs. `docker stats` shows the use.
- After each run the script restores `app.json` and generated layout files; do not commit `tests/al-language/app.json` changes of version or `preprocessorSymbols`.
- Test pages open slowly the first time after a restart (a first failure such as "page definition has changed" disappears on the second run).

## Handing a branch over

When everything is green locally on BC 29 and the gates pass:

1. `git fetch && git merge origin/al-runtime-rs-staging` into your branch; re-run the gates and your new tests once.
2. Push your branch.
3. Tell the publisher (SendMessage or the coordinator): branch, last commit, number of tests, rows or topic pinned, and anything you dropped and why.

The publisher merges your branch into staging, runs the matrix (the bump pull request to `master`, see above) and answers with failures. A 27 or 28 difference found there is fixed with a version guard, not by dropping the probe, unless it is a runner artefact (rule 2).

## CI

You never dispatch a workflow (`gh workflow run`, `gh run rerun`, API dispatch) on any repo. Observe BC behavior locally with the orchestrator's observe tool. CI is only the run of your own pull request. A cell that depends on the handler routing of Microsoft's test runner (page 130455) cannot be observed locally: it needs the PR CI run, so list it in your handover under "needs CI observation". Look at failures with `python3 tools/corpus-writer/ci-failures.py <run id>`.

## Where the facts go

Put the result in the test file header, not only in the commit. The runtime reads the header and the assertions. When a row of `docs/probes-needed.md` is settled by your tests, say which row in the handover message; the publisher updates the document.

## Corpus layout

The corpus must stay ordered and free of duplicates. Rules for every writer and for the publisher:

1. **One folder per topic**, named for the topic and not for the writer or the batch. New tests go into the folder that owns their topic (table below). A new topic gets a new folder under `tests/al-language/`; the name is a lower-case singular or plural noun like the existing ones.
2. **File and codeunit names say what is tested**: `<Topic><Aspect>.al`, codeunit `"<Topic> <Aspect>"` (for example `text/lt-text-functions/CaseConv.al`, codeunit `"LT TxB CaseConv"`). Never a writer or batch prefix in a new name (`PA`, `PX`, `W4`, `batch3`, ...). Test procedures are named `Member_Case_Result`.
3. **Adding a procedure to an existing topic codeunit is allowed**; changing or removing an existing test procedure is not (the oracle uses their names). If the codeunit has too many tests (over about 25 observations in one procedure, or one codeunit mixing topics), start a new codeunit in the same folder.
4. **Do not move or rename existing files or tests.** Test names are part of the oracle. A later one-time consolidation is listed below; the owner decides when.
5. Object ids stay inside your writer range (laptop 70000-74999; the publisher uses 61200-61299 for OnPrem and 69000-69999 for the rest). `check-object-ids.py` enforces uniqueness.
6. **A new writer range must be added to the CI `codeunit_range`** (`.github/workflows/ci.yml`, input `codeunit_range` of the Cloud job and of the OnPrem job) in the same commit that starts using it. CI only runs the test codeunits inside that range; a range that is not listed compiles but runs nothing, and the run is still green. Current ranges: Cloud `60000..60999|67000..69999|70000..74999`, OnPrem `61200..61299`.
7. **The hand-over check is the junit, not the green run.** After the CI run of your branch, download the junit artifacts and count your tests (`python3 tools/corpus-writer/ci-failures.py <run id>` prints failures; count the `testcase` elements whose codeunit id is in your range with a one-line script) and compare the number with the tests you wrote. Fewer than you wrote means a range or a gate problem, whatever the run's colour.

Folder map (`tests/al-language/` unless stated):

| Topic | Folder |
|---|---|
| Text, Code, Label, TextBuilder, string builtins | `text/` (`lt-text-functions`, `lt-text-instance`, `lt-label-textbuilder`); older: `types/` |
| JSON types | `json/` |
| XML types, XmlPort | `xml/`, `xmlport/` |
| Streams, Blob, Media, MediaSet, Base64 | `streams/`; older: `media/`, `types/` |
| List, Dictionary, Array, collections | `collections/` |
| Date, Time, DateTime, Duration, Decimal, Integer, Boolean, Guid, Option, Enum, Variant | `types/` |
| Record (fields, keys, filters, find, modify, triggers, validate, relations, CalcFields, FlowFilters) | `record/`, `records/`, `calcfields/`, `filters/`, `system-fields/`, `tableglobals/` |
| RecordRef, FieldRef, KeyRef | `recordref/`, `fieldref/` |
| Table and table extension triggers | `tableextension/`, `tableextensiontrigger/` |
| Pages, TestPage, page extensions, page events | `pages/`, `pageext*/`, `testpart/`, `controls/`, `pagefindrecord/`, `pageupdate/`, `pagesave/`, `pageapplicationarea/`, `appareas/` |
| Reports, report extensions, datasets, request pages | `report/`, `reportext/`, `reportextensiontrigger/`, `filterpage/` |
| Queries | `query/` |
| Codeunits, interfaces, events, subscribers | `codeunit/`, `events/`, `extensionevents/` |
| Errors, ErrorInfo, error handling | `errors/`, `errorinfo/`, `error-handling/` |
| Session, user, company, permissions, access control, profiles | `session/`, `permissions/`, `access-control/`, `profile/` |
| Platform tables and metadata (Field, Key, AllObj, Feature Key, system tables) | `metadata/`, `platform/`, `misc/` |
| HttpClient and network | `network/`; handler tests in the OnPrem app |
| Install and upgrade | `install/` (no new install codeunits, see rules) |
| Shared fixtures | `_fixtures/` (and `fixtures/`) |
| OnPrem scope objects | `tests/al-language-onprem/<same topic folder>` |
| Must-not-compile | `tests/must-not-compile/cases/NNNN-short-name.al` |

Older files that carry a writer or batch prefix (`PA*`, `PC*`, `PE*`, `PF*`, `PO*`, `PR*`, `PS*`, `PT*`, `PU*`, `PV*`, `PW*`, `PX*`, `PY*`, `PZ*`, `W1*` to `W7*`, and the `handlers/` folder with about 500 files) stay where they are for now. A later one-time consolidation would move them by topic into the folders above: `W1*` into `pages/`, `W2*` into `appareas/`, `bgtasks/`, `controls/`, `misc/`, `W3*` into `report/`, `reportext/`, `W4*` into `calcfields/`, `events/`, `records/`, `recordref/`, `W5*` into `session/`, `errors/`, `W6*` into `metadata/`, `platform/`, `session/`, `W7*` into `types/`, `query/`, `streams/`, `xmlport/`, `PW*`/`PX*`/`PY*`/`PZ*`/`PU*`/`PV*` into `types/` and `session/`, and the files in `handlers/` into the folder of what they test (`report/`, `pageupdate/`, `session/`, ...). Moving renames the oracle test names, so it needs the owner's decision and a regenerated baseline.

## Check before you write

Before you write a test for a member, in this order:

1. Look the member up in `docs/corpus-coverage.md` (in al-runtime-rs; generated, not committed: `./target/release/al-test-cli --root . --coverage-map` writes it, or take the artifact `coverage-maps-<run id>` of main's newest CI run): it lists, per type, which overloads the corpus already calls. A pinned overload is not new work unless you add cases it misses.
2. `grep -rn "<MemberName>(" tests/al-language/<topic folder> tests/al-language-onprem` in the corpus. Read the matching tests. Add only the cases that are missing (other inputs, edge values, error texts, other types of the argument).
3. Write your tests. Before you hand the branch over run the duplicate finder:

```bash
python3 tools/corpus-writer/dupfinder.py --corpus ~/corpus --new-git origin/al-runtime-rs-staging --show
```

It lists the new tests whose facts (member plus literal inputs) already exist in other files, with the place they exist. Remove or reduce a test that only repeats an existing fact. A listed test that also checks other inputs or another context stays; say so in the handover message. The publisher runs the same command on your branch before merging.

## Which BC is the reference

The corpus CI and the local container run BC on Linux (MsDyn365Bc.On.Linux). It matches
Business Central on Windows to about 99.9% and is much faster, but Microsoft does not support
it. The Windows nightly run of the corpus (official Microsoft BC container) is the final
reference. When the nightly flags a test that the Linux CI passed, the difference is an
upstream issue for the BC on Linux repository, not a reason to change the probe.
