# ADR-076: Apps installed on the server that are not dependencies of the app under test

- Status: Accepted (2026-11-17)
- Date: 2026-11-16
- Owner decision behind it (2026-11-16): users can declare apps that are installed on their BC server but are not
  dependencies of the app under test, as the corpus does with `extra_apps`. The runner loads them from the BC
  artifact, and everything that depends on the installed set sees them. This ADR settles how.
- Amends:
  - [ADR-013](013-external-objects.md) decision 2, layer B, item 2 (the roots of the dependency resolution), and its
    section "The test environment's apps: Microsoft's Test Runner" (whose "Decided without owner review" note asked
    for an amendment if apps that no project declares were ever added);
  - ADR-013 Layer A decision 10 and the amendment of 2026-11-10, item 6 ("Offline"): an extra app of a corpus suite
    that is not found is no longer skipped and logged; it stops the suite's pass (section 11);
  - [ADR-028](028-alrunner-serve.md) decision 8 (request fields): `runTests` and `execute` get the field
    `installedApps`. Additive.
- Depends on: [ADR-062](062-permission-enforcement.md) for the denial #8160 reports (section 2, "What this ADR does not
  give")
- Related: [ADR-022](022-compile-cache.md) (cache layers and the baseline key), [ADR-046](046-install-code-of-the-app-under-test.md)
  (install order), [ADR-050](050-affected-test-selection.md) (`--affected`), [ADR-063](063-one-compile-per-version-for-all-passes.md)
  (the corpus passes with `extra_apps`), [ADR-066](066-run-the-test-runner-codeunit.md) (the test runner's steps)
- Issues: al-runner #8160, #8170

## Context

### What the runner models today

A run loads the closure of the app under test: the apps its `app.json` declares, their own dependencies, the platform's
`System.app`, and Microsoft's Test Runner app, which BC's test environment always has (`al-testrun/src/deps.rs`,
`with_test_environment`). Nothing else is installed in the modeled server.

A real BC server often has more. A Windows container made with BcContainerHelper has Tests-TestLibraries and Data
Archive installed (corpus `.github/workflows/ci.yml:523`). A sandbox database from the artifact has every app Microsoft
put in it. Two outside reports show where the difference changes a result:

- **#8160.** Microsoft's Test Runner lowers permissions before each test only through codeunit 130302 "Set State Test
  Run" of Tests-TestLibraries. It looks the codeunit up in AllObj and runs it only if it is there
  (`al-testrun/src/runtime/runner_reset.rs`, `ALTestRunnerResetEnvironment.Codeunit.al:54-62`). The reporter's BC has
  Tests-TestLibraries installed, so a write before any permission set fails there. The reporter's app does not depend
  on Tests-TestLibraries, so the runner models a server without it, and the write passes. The corpus pins both sides:
  without the app the write passes (`pages/run-modal/StaticRunModalFindRecordLookup.al:127`, with `BaTpEnv.al:28`
  showing 130302 is absent); with it BC refuses the write (`al-language-testlibraries/test-framework/testpermissions/BiTpDefault.al:35-45`).
  Two things are missing for the second side, not one: 130302 must run (this ADR), and the permission sets it assigns
  must be enforced (ADR-062; see section 2).
- **#8170.** The virtual Field table, AllObj, AllObjWithCaption, Table Metadata and the other metadata tables list the
  objects of the loaded apps (`al-vm/src/platform/objects.rs`, `fields.rs`). On BC they list every installed app. A
  scan of Field by `FieldName` without a `TableNo` filter finds a Base Application table first on BC, and the app's own
  table in the runner, when the app does not depend on Base Application.

### What already exists

The corpus has the same need, and the runner already serves it, for the corpus only:

- `oracle/oracle.toml` names `extra_apps` for the testlibraries suite (Tests-TestLibraries and Data Archive, each with
  id and name), as the corpus CI job `test-testlibraries` does through bc-linux's `extra_apps` input.
- `al-test-cli` runs that suite in a pass of its own (`main.rs`, `passes`). `deps::load_corpus` adds each extra app to
  the dependency resolution as a root, looked up by id like a declared dependency. ADR-063 grows the main pass's
  compile with those apps.
- Loading an app does not make it visible by name: an app sees only what it declares (`al_sema::workspace`,
  `visible_from`). The Test Runner app is loaded the same way.
- Once loaded, an extra app is a member of the program: its objects are AllObj and Field rows, its subscribers are
  bound, its extensions change the objects they extend, its install code runs in the install baseline, and the test
  runner finds 130301 and 130302 by id (`RunnerReset::of`). The testlibraries suite checks part of this against BC; the
  table in section 2 says which part.

bc-linux, which runs the corpus on BC, keeps an extra app installed by **name**, case-insensitive, together with its own
dependencies, and fails the job when a name matches no app in the artifact (`scripts/resolve-keep-app-ids.py:361-376`).
The apps it keeps were installed when the database was made, before the corpus apps are published.

So the mechanism exists, and part of it is checked. What is missing is a way for a user to ask for it, and pins or
assumptions for the parts the corpus does not check.

## Decision

### 1. One concept: an installed app

An **installed app** is an app the modeled server has installed that no app under test depends on, directly or through
another app. The runner treats it as it treats a dependency app, with two differences that BC has too:

- the apps under test cannot name its objects (they do not declare it; a reference is a compile error, as on BC);
- its test codeunits never run (only the tests of the apps under test run, as for every dependency app).

The apps an installed app depends on are installed too, with every effect of this ADR (they are "pulled in"). An app
that the closure already holds stays a closure app. For Tests-TestLibraries this pulls in Base Application (through its
`application` floor), Permissions Mock (`BiTpEnv.al:31` pins it as installed) and the test libraries it uses. For an
app under test with no Base Application dependency, this adds Base Application's install code, its
`Company-Initialize` data, its login code (WorkDate from G/L entries) and the default demo backup. Section 9 lists the
pulled-in apps in the output.

**One owner in the code.** A function in `al-testrun/src/deps.rs` adds the installed apps of the environment to the
resolution and returns, with the resolved apps, which of them are outside the closure (section 2a). The corpus
`extra_apps`, the Test Runner app and the user's declarations all go through it. It takes the roots and the search list
as input; it does not decide which origins are allowed. The admission gate of step 1 (section 7) lives in the
`al-runner` front end, so the corpus can keep loading a non-Microsoft extra app ("Harness Env Lib" in
`al-test-cli/tests/corpus_harness.rs`).

Two other places add an app that no project declares. They relate to the one function as follows:

- `--company-data demotool` and `--company-data contoso-*` push the DemoTool or the Contoso app into
  `project.dependencies` (`alrunner-cli/src/run.rs`, the `company_data` block). The refactor of conformance step 1
  moves them into the one function as closure roots of their own kind. They stay closure apps, so their outcomes do not
  change. Whether they should be installed apps instead is a follow-up issue, not part of this ADR.
- The corpus `extra_dirs` (xhandler-c and xhandler-d in `oracle.toml`) are corpus folders compiled from source as
  apps under test that no test app declares. They are apps under test, not installed apps, and this ADR does not change
  them.

### 2. What becomes observable

BC has no notion of "installed for metadata only", so the runner does not hide part of an installed app. But a loaded
app does not make every observable pinned. Hard rule 3 applies per observable: each row below runs on a pin, runs
through an assumption entry that cites a probe, or refuses in step 1. "Outside the closure" means an installed app or an
app it pulls in (section 2a decides membership).

One corpus probe pull request in `al-language-testlibraries` (written before the build PR, ADR-007; called "the
installed-app probe" below) measures the assumption rows. It includes an app under test whose closure has no Base
Application, so that it measures the #8170 shape, where Base Application is an app outside the closure. Every suite app
that measures this today depends on Base Application. An assumption covers only the shapes the probe's local BC result
measures; every other shape of that row refuses.

| Observable for an app outside the closure | Pin (file:line) | Step 1 |
|---|---|---|
| AllObj codeunit rows | `testpermissions/BiTpEnv.al:27,30` | runs |
| NAV App Installed App rows, by name | `BiTpEnv.al:28,31,79-97` | runs |
| The test runner's steps find 130301 and 130302 by id and run them (`RunnerReset::of`, `TEST_RUNNER_CODEUNITS`) | `BiTpEnv.al:29,32` (runner frames) | runs. The denial needs ADR-062, see below |
| A subscriber of an installed app runs (Backup Management's four `GetDatabaseTableTriggerSetup` flags) | `trigger-setup/BiTriggerSetup.al:222-260`, `setup-order/Y3SetupOrderTests.al:10,36`, `stale-delete-triggers/WbStaleDeleteTrace.al:25` | runs |
| Order of an installed app's subscribers against another app's subscribers to the same event | the existing rule in `al-vm/src/dispatch.rs` (section 2b) | runs where that rule is pinned; refuses where it refuses today |
| Default test handlers (BC 29, ADR-059) of an app no test app declares | `al-language-xhandler-c`, `-d` (published, undeclared apps under test) | assumption `installed-app-default-handlers` for an artifact app; no step 1 app has one today, so it refuses until the probe measures one |
| AllObj rows of other object types, AllObjWithCaption, Field, Key, Table Metadata, CodeUnit Metadata, the page and report metadata tables, Page Control Field | none (#8170's reporter measured a Field scan) | assumption `installed-app-metadata-rows` (the installed-app probe; evidence also the #8170 report) |
| Published Application rows and their "Published As", Installed Application rows | none | refuses: a read that returns a row of an app outside the closure raises `Signal::Unsupported` |
| All Profile rows | none | refuses, as above |
| Aggregate Permission Set, Metadata Permission Set, Expanded Permission rows | none | refuses, as above |
| Table extension fields and triggers, page extension controls and triggers, report extensions, enum extension values, permission set extensions on closure objects | none | assumption `installed-app-extensions-apply` for the kinds the probe measures; other kinds refuse at compile, with a lowering reason that names the extension and its app |
| Rows its install code leaves, per company and per database (ADR-046 A6) | none | assumption `installed-app-install-rows` |
| Access by id from code of an app under test: `Codeunit.Run(id)`, `Page.Run(id)`, `RecordRef.Open(id)`, `NavApp.GetModuleInfo(id)` | only the Test Runner app's `Codeunit.Run` of 130301 and 130302 (`BiTpEnv.al:29,32`) | assumption `installed-app-reached-by-id` for the forms the probe measures; other forms refuse |
| Login code of a non-Microsoft app outside the closure (`OnAfterLogin`) | none | existing assumption `partner-login-subscriber-in-test-session` (`al-testrun/src/partner_login.rs`), as for a partner dependency app (step 2 only) |

Further facts that hold for every row:

- **Static subscribers are bound.** A subscriber with `EventSubscriberInstance = Manual` is bound only when the app's
  own code binds it, as on BC. A subscriber that the runner cannot run refuses the test that reaches it (hard rule 4).
  Declaring a large app can turn passes into refusals; that is the correct answer until the gap is closed.
- **Install code** runs in the install baseline, step 1, with the dependency apps (`al-testrun/src/baseline.rs`): its
  install codeunits and "Extension Triggers" subscribers, and its subscribers to Company Triggers and
  `Company-Initialize`. Per-company install code runs in every company of the modeled database (ADR-046 A6). Upgrade
  codeunits never run: the app is installed new, as when the artifact's database was made.
- **The refusals and assumptions apply to the corpus too.** The testlibraries pass goes through the same function and
  the same membership. The build PR's `--baseline-diff` on BC 27, 28 and 29 shows whether a testlibraries case changes;
  a case that goes from pass to refused is a regression and blocks the PR until that row has a pin or an assumption.

**What this ADR does not give for #8160.** It makes 130302 run. The denial the reporter sees on BC also needs the
permission sets 130302 assigns to be enforced. Today they are not: `runner_reset.rs:72-75` and the assumption
`runner-permission-sets-grant-all` grant every access. Measured on main (binary of 2026-11-14):
`al-test-cli --root . --bc-version 27 --codeunit 81553 --explain BiTpDefault_Insert` fails with "LT BI inserted", where
BC raises "Sorry, the current permissions prevented the action. (TableData 81550 ...)". The case and nine more
`BiTp*_Insert` and `BiTp*_Permissions` cases are on `metrics/27/not-passing-testlibraries.txt`.

Draft PR 8166 (branch `perm-default`, the enforcement stage of ADR-062, in a fix round) raises the pinned Insert error
for the measured Tests-TestLibraries shapes; `BiTpDefault_Insert` and its siblings pass on that branch. So:

- #8160 needs both this ADR (130302 runs for a user who declares Tests-TestLibraries) and that enforcement (PR 8166,
  ADR-062).
- #8160 closes only when the `BiTpDefault_Insert` line is pruned from the not-passing list on `main`.
- Until then, the reporter gets BC's error for the shapes PR 8166 measures, once both have landed, and a flagged pass
  under `runner-permission-sets-grant-all` for every other shape. The reply to the reporter says so.

### 2a. Where closure membership is kept

Which loaded apps are outside the closure is computed per run, and per pass in the corpus, from the apps under test:
the one function of section 1 returns it with the resolved apps, and the run keeps it as run state next to the program
(the pass's engine context). The virtual table reads and the extension and by-id checks of section 2 read it there.

It is never stored in a compile cache layer and is not part of a layer key. The same layer file serves app X as a
declared dependency of one project and as an installed app of another (ADR-022, "a base layer shared by projects"), and
ADR-063 grows the corpus main pass's compile with the extra apps while that pass is closure-only. A mark in the layer
would be wrong for the second user of that layer: the flag would be lost, or a pinned read would be flagged.

The install baseline is different: it holds the rows install code left, and `installed-app-install-rows` flags them.
So the membership set is an input of the install baseline key (section 6), and a baseline restored from disk reports
the assumptions it relied on, as every baseline already does.

### 2b. Subscriber order across apps

The order of subscribers to one event across apps has one owner, `al-vm/src/dispatch.rs` (the subscriber sort). Its
rule: app by app, each app's dependencies before it (depth first, post-order), apps that do not depend on each other in
ascending app id; inside one app, ascending codeunit id. Its pins:

- dependency order: `events/subscriber-order/BwSubscriberOrderTests.al:52-56` and `:150`;
- apps that do not depend on each other, by app id and not by publish order or name:
  `BwSubscriberOrderTests.al` (`SubscriberOrder_PlatformTableEvent_AppsThatDoNotDependOnEachOther_SixApps`,
  `SubscriberOrder_PlatformTableEvent_DependenciesVisitedInAppIdOrderNotDeclaredOrder` and their install variants) and
  `TtSubscriberOrderTests.al:37-44` (BC walks from every app, not only from the apps no other app depends on, and does
  not sort by publisher; al-runner issue 6924).

The sort reads only the loaded apps and their dependencies; it does not know how an app came to be loaded, and neither
does BC. An installed app is an app that no app under test depends on, which is the shape the second group of pins
measures (one of those apps was published first and still runs second). So the rule applies as it is. Where it refuses
today (`visit_orders`, where the readings it cannot tell apart give different orders and `MEASURED_PAIRS` has no
entry), it refuses for installed apps too. ADR-046 A8 still lists "across apps for one event (probe Q2)" as open; those
pins answered it after A8 was written, and the build PR updates that line.

### 3. Install order

Installed apps take their place in the canonical dependency order of ADR-022 (Microsoft's apps before the others, then
by dependencies, name and id). Their install code runs in that order, one app complete before the next (ADR-046 A3).

For Microsoft's apps this is what BC has: bc-linux and a BC container install the artifact's apps when the database is
made, before the user publishes anything, and the runner puts Microsoft's apps first. For a non-Microsoft installed app
(step 2 below) the order against an unrelated non-Microsoft app is not pinned. Step 2 needs a probe for it before it
runs such a pair (hard rule 3). Until then, a run with two non-Microsoft apps that do not depend on each other, both
with install code, **and at least one of them an installed app outside the closure**, refuses with a reason that names
the two apps. Two unrelated partner dependency apps inside the closure run as they do today.

### 4. How a user names an installed app

On the command line, `--installed-app VALUE`, repeatable, on `al-runner test`, `debug`, `serve` (as the server's
default, see 6), `impact-index build` and `test-microsoft`. VALUE is one of:

1. an app id (a GUID);
2. a name, matched case-insensitively (section 5). A name that matches apps with different ids stops the run and lists
   them;
3. a path to a package file, a source archive or an app folder (step 2 only; in step 1 it exits 2 naming step 2).

There is no version on the command line. A Microsoft app comes in the build of the run, like every other Microsoft app
(section 7 checks this).

The shape for a settings file, when there is one (owner decision 3: none in step 1, so this shape is for a later ADR), is the shape of an `app.json` dependency:
`"installedApps": [{ "id": "...", "name": "...", "publisher": "...", "version": "..." }]`, with `version` a minimum, as
in `app.json`.

### 5. Where an installed app is found

**Step 1 looks only in the artifact of the run's build**: the build's folder in the artifact cache, a local artifact
folder (`bcartifacts_sandbox_from_env`), or the artifact feed. `--dep`, `--package-cache` and `.alpackages` are not
searched for an installed app in step 1; a hit there is handled by the admission gate (section 7). Its own dependencies
are resolved together with the closure: one version per app id, and a conflict is the same error as between two
declared dependencies. ADR-013's rule that a package without source is not found applies (hard rule 7: Microsoft's apps
are compiled from their source).

**How a name is matched in the artifact.** The artifact is a ZIP; its folder names are not app names
(`TestFramework/TestLibraries/permissions mock/`, `BaseApp/Test/Tests-*.Source.zip`). The rule is the one the loader
already uses for a dependency (`al-deps/src/artifact.rs`, `find_entries`, `rank_extensions`, `entry_is`):

1. The central directory of the platform artifact (and of the country artifact, which comes first) is read by range
   and saved in the build's cache folder. This is the same read a run that downloads any dependency makes, so a run
   that already needs the feed pays nothing more for it.
2. The file name is a hint: the entries whose base name is `<name>.Source.zip`, then `Microsoft_<name>.app`, and in a
   country artifact's `Extensions/` folder `Microsoft_<name>[_<version>].app`, all compared without case.
3. The manifest decides: each hinted entry is fetched (one ranged read of the entry, checked against its CRC-32 and
   saved in the build's cache folder, as a dependency entry is today), and its `app.json` must have that name, compared
   without case. Hinted entries whose manifests have different ids are "a name with two ids" and stop the run.
4. A name with no hinted entry, or no hinted entry whose manifest has the name, is "not found". The message lists the
   entries searched. The runner does not read the manifest of every entry to find an app whose file name differs from
   its name; that would be one fetch per `*.Source.zip` entry. Such an app is a gap for an issue, and the user can name
   it as step 2 allows.

**How an id is matched.** An id is matched against the manifests of the build's entries already in the cache folder.
An id that none of them has exits 2 with "the artifact is searched by name: pass the app's name", because the feed has
no index by id. The corpus `extra_apps` give both: the name finds the entry and the id checks the manifest, as today.

**Cost.** The central directory read (a few hundred kilobytes, shared with every other dependency fetch of the run),
plus one ranged fetch per hinted entry, once per build: later runs read the saved files.

**`--no-download`.** The saved central directory and saved entries answer. A name or id whose entry is not on disk
stops the run, naming the app and "not in the cache of build B; run once without `--no-download`".

### 6. Serve, and the keys that must see the installed set

- `al-runner serve --installed-app ...` sets the list for every request.
- The `runTests` and `execute` field `installedApps` (a list of strings in the forms of 4) **adds to** the server's
  list for that request, as `packagePaths` adds to the server's folders (owner decision 5). The apps of the run are the
  union of the server list and the request list; an app named in both is loaded once. A request with no field, or `[]`,
  gets a run with only the server list. A value that is not a list of strings answers `{"error":...}`.
- **A request cannot remove a server entry.** A client that needs a run without an app the server lists must talk to a
  server started without that `--installed-app`. A run with no installed apps at all needs a server started with none.
  The server's list is therefore the floor of every run on that server. `docs/manual/tool-protocol.md` says this in the
  same place as the `packagePaths` rule.

Eight keys decide whether something computed earlier is reused. Each must change when the installed set changes:

1. **Compile cache layers** (ADR-022): the key holds every app of the layer. Covered.
2. **Install baseline key** (`al-testrun/src/runtime/baseline_key.rs`): "dependencies" is the top layer's file name, and apps
   outside layers are hashed. Covered for the apps. The closure membership set of section 2a is added as an input.
3. **Install-once `RunInstalls` key** (`al-testrun/src/runtime/install_once.rs`, `Key.engine`): the engine uid, and the engine is built from
   the program. Covered.
4. **Kept engines** (`al-testrun/src/runtime/corpus.rs`, `kept_engine_key`, plus the program pointer). Covered.
5. **`run::warm_key`** (`alrunner-cli/src/run.rs`): hashes the id and identity of every loaded dependency app. Covered, once the dependencies it
   reads are the right ones (key 7).
6. **`--affected` environment** (`alrunner-cli/src/affected.rs`, `environment`): every dependency app. Covered; a
   changed set is a full run with the existing reason "the dependency apps or the BC build changed".
7. **Serve's kept dependencies** (`alrunner-cli/src/run.rs`, `deps_key`, al-runner issue 5856). Not covered by
   itself: it holds `{project:?}`, and the installed list is the one function's own input, not part of `project`. A
   request with another `installedApps` would reuse the old dependencies, `warm_key` would be computed from them and
   match, and the run would answer for another server. The installed list (the server list plus the request list)
   is added to `deps_key`.
8. **Run inputs** (`alrunner-cli/src/run_inputs.rs`, `RunInputs::of`): the new `TestArgs` field goes in class
   **outcome**, so it is in the options hash of the `--affected` stored result and in the key of the impact index
   (ADR-043). `RunInputs::of` destructures `TestArgs` without `..`, so the field does not compile until it is
   classified.

### 7. What stops the run, and what refuses

**Admission gate, step 1** (in the `al-runner` front end, not in the shared function). An installed app is admitted
only when all of these hold, each checked by equality:

- VALUE is an id or a name;
- the package was read from the artifact of the run's build: the origin the loader records (its cache folder, local
  artifact folder or feed URL for that build), not the publisher text (recurring finding: "Microsoft's app read from
  the publisher string alone", PR 8015);
- its version is that build's: the major and minor of the app equal the build's, and the build is the run's build;
- every app it pulls in that is not in the closure meets the same two facts.

Everything else exits 2 with a reason that names the app, what failed, and that step 2 covers it. In particular:

- a non-Microsoft app whose name is found in `.alpackages`;
- a package with publisher "Microsoft" in `--dep` at another build;
- a name found both in the artifact and in `--dep`, `--package-cache` or `.alpackages`: the message names both places
  (step 1 does not choose between them).

**Setup errors**, before the compile, exit 2 with a message that names the app and the places searched:

- an installed app that is not found, or found only without source;
- a name that matches more than one app id;
- an installed app whose own dependencies cannot be met (not found, below a minimum, or in conflict with the version
  the closure resolved), under ADR-013's existing rules (the advisory minimum of the five platform apps stays);
- an installed app that depends on an app under test (it would have to be compiled after the workspace, which the
  cache layers do not allow; the message says to pass it as an app instead). Step 1 has no such case: no Microsoft app
  depends on a user's app.

`--allow-missing-dependencies` does not apply to installed apps. The user named them; a run without one is not the
server the user described. `--no-download`: see section 5.

No-ops, with a line in the verbose log: an installed app that is already in the closure, one that is an app under
test, and the same app named twice.

**At run time**, code of an installed app refuses as dependency code does, and section 2's table adds the refusals of
its unpinned rows. Section 3 adds the install order refusal for step 2.

### 8. Steps

- **Step 1: Microsoft's apps from the artifact**, named by name or id, behind the admission gate of 7. This lets
  130302 run for #8160 (the denial also needs PR 8166, section 2) and covers #8170 with Base Application and the
  rest, through the assumptions of section 2. It is what the corpus `extra_apps` and bc-linux do.
- **Step 2: other apps and paths** (a partner app installed on the user's server). Needs the install order probe of 3
  and a pin for "Published As" of an app installed on the user's server. Owner question 2 decides whether and when.

### 9. Output

The run's start prints one line per installed app (name, version, where it came from) and one per pulled-in app,
marked "pulled in by NAME". `--output-json` and serve's summary carry `installedApps` (id, name, publisher, version,
origin, `pulledInBy` when pulled in), next to `skippedDependencies`. Both are additive.

### 10. Other features

- **`--affected` (ADR-050)** and **the impact index (ADR-043)**: keys 6 and 8 of section 6.
- **Matrix runs**: `--installed-app` is forwarded to every cell (`matrix.rs`, the list of forwarded flags). An app name
  that exists in one build and not in another stops that cell with exit 2 ("not found"); the other cells run.
- **`test-microsoft` and `--recipe microsoft`**: the declared apps are added to what the recipe loads.
- **Test data** (`--test-data`, ADR-020): the tables of an installed app are filled from the backup by the existing
  rule (by AL name and declaring app id), as a dependency app's are.
- **`execute`** with `installedApps`: allowed; conformance step 2 tests it.

### 11. The corpus

- `oracle.toml` keeps the name `extra_apps`: it mirrors the corpus CI. `al-test-cli` hands the list to the same
  function as `--installed-app`, without the admission gate of 7.
- One behavior change: today an extra app that is not found is skipped with a note (`MissingPolicy::Skip`; ADR-013
  Layer A decision 10 and the 2026-11-10 amendment, item 6). With this ADR it stops the suite's pass (exit 2), also
  under `--no-download` (owner decision 4). A testlibraries pass without Tests-TestLibraries gives outcomes of another
  environment. The harness tests in `al-test-cli/tests/corpus_harness.rs` change in the same PR.
- `al-test-cli --no-deps` loads no app from the artifact, so it loads no extra app either, as today. Nothing stops;
  this is the documented exception, and the run is not a CI-shaped run.
- The main pass stays closure-only. That is the corpus CI's main environment (bc-linux keeps only what the corpus apps
  need), and it is the oracle environment.

### 12. Cost

- **With the compile cache (the default).** An installed Microsoft app outside the base layer is a layer of its own
  (ADR-022, "a base layer shared by projects"), built once per build and set. ADR-022 measured 0.2 to 1.1 s per test
  library layer and about 1.0 to 1.5 ms per object. An installed app that sorts before an existing layer changes the
  keys of the layers above it, which are rebuilt once. The base layer is not rebuilt.
- **Without the cache (demand-driven, ADR-013 decision 6).** The installed app adds roots: its subscribers to events
  the run raises, its install code, its extension triggers on objects the run touches. The corpus shows the size for
  the two extra apps on BC 29 (`metrics/29/work.json`): 15 dependency apps against 9 in the main pass, and 161 more
  dependency objects and 4 182 more routines on top of the main pass's reach. ADR-063 measured that pass at 20.1 G
  instructions grown, of which 5.1 G is its own install baseline.
- **The feed**: section 5, once per build.
- **The corpus run** does not change: same passes, same apps. `metrics/<major>/work.json` stays as it is.
- The build PR measures the user path on a small app with and without `--installed-app Tests-TestLibraries`, cached
  and uncached (wall time, instructions, peak memory), and reports the numbers. No budget is raised.

## Options not chosen

- **Fill the metadata tables from every package in `--package-cache`** (the reporter's suggestion in #8170). A package
  folder is a search list, not an install list. It would also fix only the metadata tables: subscribers, extensions and
  install code of those apps would still be missing, so the tables would describe a server that does not exist.
- **Load an installed app for the metadata tables only** (rows in AllObj and Field, subscribers and install code
  inert). BC has no such state. Every rule that hides part of an app is a place where a result can be wrong.
- **Model a BcContainerHelper container by default** (Tests-TestLibraries and Data Archive always installed, or every
  app of the artifact's database). It changes the corpus oracle environment (closure-only, BaTpEnv.al:28) and the
  result of every user run, and which apps a container has is not pinned. A preset on top of the explicit list is
  owner decision 6 (none now).
- **A key in `app.json`.** `app.json` belongs to the AL compiler and describes the app, not the server it is tested on.
- **Serve: the request list replaces the server list** (the draft's recommendation). Not chosen by the owner
  (decision 5): a request could then drop apps the server was started with, and the server list would not be a floor.
  The cost of adding: a client cannot ask for fewer apps than the server lists.
- **Serve: a server-level list only.** `packagePaths` is already a request field; an editor that switches projects
  needs the installed set per request as well.
- **Search `--dep`, `--package-cache` and `.alpackages` for an installed app in step 1.** A name can match a
  non-Microsoft app there, or a Microsoft-named package of another build. Step 1 takes only the run's artifact.
- **Keep the "outside the closure" mark in the compile cache layer.** Section 2a.

## Consequences

- A user can describe the server the tests run on. For #8170 the runner gives that server's answer for the pinned
  rows, a flagged pass for the assumption rows and a refusal for the rest. For #8160, 130302 runs; BC's denial needs
  ADR-062's enforcement as well (PR 8166), and the issue stays open until `BiTpDefault_Insert` is pruned on `main`.
- The default does not change: without `--installed-app`, a run models the closure, as today and as the corpus main
  environment.
- An installed app's code runs, and the apps it pulls in are installed too. Users who declare large apps get more
  refusals in the first versions; the refusal reasons name the routine and the app.
- The corpus `extra_apps` path and the user path are one code path, so the testlibraries suite checks the user feature.
- Not-found extra apps stop a corpus pass instead of being skipped.
- Five new assumption ids (section 2) and their `docs/probes-needed.md` rows, all citing the installed-app probe.
- `docs/manual/cli-reference.md` and `tool-protocol.md` get the flag and the field in the build PR.

## Conformance

Red/green, in this order (hard rule 1):

1. **Refactor first, no behavior change.** `deps::load_corpus`'s extra apps, `with_test_environment` and the
   `--company-data` roots move into the one function of decision 1. `--baseline-diff` on BC 27, 28 and 29 shows no
   change; `metrics/<major>/work.json` does not change; `tools/fresh_vs_grown.sh run` shows 0 differences.
2. **Rust tests for what the corpus cannot reach** (CLI, protocol and refusal plumbing), each red before the code:
   - `alrunner-cli` args: `--installed-app` with an id, a name, repeated; a path refused in step 1 with a reason that
     names step 2; the serve field `installedApps` accepted and added to the server list, a name in both loaded once, `[]` or no
     field giving the server list alone, a non-list answered with an error; `execute` with `installedApps`.
   - The admission gate, on fixture packages made in the test (no Microsoft content): a non-Microsoft name hit in
     `.alpackages`, a package with publisher "Microsoft" in `--dep` at another build, a name that hits in the artifact
     and in `--dep`, a pulled-in app that fails the gate; each exits 2 naming step 2.
   - `al-testrun` dependency tests on a pool of fixture packages: not found, found without source, a name with two ids,
     an id not in the cache, an unmet dependency, a dependency on an app under test, a name not on disk with
     `--no-download`; each exits 2 with its message. `--allow-missing-dependencies` does not skip one; an app already
     in the closure is a no-op. The corpus path still loads "Harness Env Lib" through `extra_apps`.
   - The feed match of section 5 on a fake feed: a folder name that differs from the app name, a country entry over the
     platform's, two hinted entries with different ids.
   - **Serve keys**: same sources; request 2 changes only `installedApps`, the answer changes and `cached` is false;
     request 3 with `[]` answers as request 1 would without the field (the server list alone); a request cannot
      remove a server entry.
   - **Membership**: one fixture app used as a declared dependency in one run and as an installed app in the next,
     with the same cache folder. The second run flags its assumption rows; a third run, as a dependency again, does not.
   - Section 3's refusal: two unrelated non-Microsoft closure apps with install code still run; one of them as an
     installed app refuses (step 2 code path, so this test comes with step 2).
   - End to end on fixtures: an app with no dependencies and an installed fixture app with a table, a codeunit, an
     install codeunit and a subscriber to an event the test raises. The test sees the codeunit in AllObj and the
     subscriber's effect; a reference by name to the installed app does not compile; the unpinned rows of section 2
     refuse or flag as the table says. Red without the flag, green with it.
3. **The corpus is the oracle for Microsoft's apps.** The pinned rows of section 2 are checked by the testlibraries
   suite. A check runs that suite through `al-runner test` with `--installed-app Tests-TestLibraries --installed-app
   "Data Archive"` (the corpus apps as `--dep` folders) and compares its outcomes with the `al-test-cli` pass: 0
   differences on BC 27, 28 and 29. This check cannot show #8160 closed: both sides fail `BiTpDefault_Insert` the same
   way today. #8160 counts as closed only when that line is pruned from `metrics/27/not-passing-testlibraries.txt` (and
   its siblings for 28 and 29) on `main`, after PR 8166 has landed.
4. **The installed-app probe** pins, for an app outside the closure, every assumption row of section 2:
   `installed-app-metadata-rows` (a Field scan by `FieldName` without `TableNo` that reaches a Base Application, Data
   Archive or Tests-TestLibraries table, from an app under test without Base Application in its closure; the Table
   Metadata, AllObjWithCaption, Key, CodeUnit Metadata, page and report metadata and Page Control Field rows of such an
   object; AllObj rows of other object types), `installed-app-extensions-apply` (one case per extension kind the step 1
   apps have on closure objects; the build PR lists those kinds with `AL_REACH_REPORT`), `installed-app-install-rows`
   (a row the install code of an installed app leaves, in two companies), `installed-app-reached-by-id` (each of the
   four forms) and `installed-app-default-handlers`. Until it is pinned, those rows run through their assumption
   entries, each citing the probe PR (hard rule 3), and the refusal rows stay refused. When a corpus bump pins a row,
   its assumption is deleted.
5. `al-testrun/src/observe.rs` is checked once for a default handler that lives in an installed app: the reach starts
   at the test codeunit's own routines, so a handler outside the closure must still make the baseline observable.
   `tools/baseline_rule_check.sh run` and `tools/demand_vs_eager.sh run` stay at 0 differences.

## Owner decisions (2026-11-17)

1. **Flag name.** `--installed-app`.
2. **Non-Microsoft apps and paths (step 2).** Built only when a user asks, after the install-order probe and the pins
   for subscriber order and "Published As".
3. **Settings file.** None in step 1. The list is given on the command line and to `serve` only.
4. **Corpus `extra_apps` not found.** The pass stops (exit 2), also under `--no-download`. `--no-deps` is the
   documented exception.
5. **Serve field and server list.** The request's `installedApps` adds to the server's start list, like
   `packagePaths`. This is not the draft's recommendation (replace). A client gets a run with only the server list by
   sending none. A request cannot remove a server entry; a client that needs fewer apps uses a server started with a
   shorter list. The draft's reason "a client must be able to ask for none" no longer holds: asking for none now means
   "no additions", and "no installed apps at all" needs a server started without `--installed-app`.
6. **Presets.** None now. A follow-up issue: with `--test-data`, a verbose note when the backup's NAV App Installed App
   names apps the run does not load (a diagnostic, not a guess; it points users at gaps like #8170).
