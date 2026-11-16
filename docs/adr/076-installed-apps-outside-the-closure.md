# ADR-076: Apps installed on the server that are not dependencies of the app under test

- Status: Proposed
- Date: 2026-11-16
- Owner decision behind it (2026-11-16): users can declare apps that are installed on their BC server but are not
  dependencies of the app under test, as the corpus does with `extra_apps`. The runner loads them from the BC
  artifact, and everything that depends on the installed set sees them. This ADR settles how.
- Amends: [ADR-013](013-external-objects.md) decision 2, layer B, item 2 (the roots of the dependency resolution), and
  its section "The test environment's apps: Microsoft's Test Runner" (whose "Decided without owner review" note asked
  for an amendment if apps that no project declares were ever added)
- Related: [ADR-022](022-compile-cache.md) (cache layers and the baseline key), [ADR-028](028-alrunner-serve.md)
  (serve), [ADR-046](046-install-code-of-the-app-under-test.md) (install order), [ADR-063](063-one-compile-per-version-for-all-passes.md)
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
- **#8170.** The virtual Field table, AllObj, AllObjWithCaption, Table Metadata and the other metadata tables list the
  objects of the loaded apps (`al-vm/src/platform/objects.rs`, `fields.rs`). On BC they list every installed app. A
  scan of Field by `FieldName` without a `TableNo` filter finds a Base Application table first on BC, and the app's own
  table in the runner, when the app does not depend on Base Application.

### What already exists

The corpus has the same need, and the runner already serves it, for the corpus only:

- `oracle/oracle.toml` names `extra_apps` for the testlibraries suite (Tests-TestLibraries and Data Archive), as the
  corpus CI job `test-testlibraries` does through bc-linux's `extra_apps` input.
- `al-test-cli` runs that suite in a pass of its own (`main.rs`, `passes`). `deps::load_corpus` adds each extra app to
  the dependency resolution as a root, looked up by id like a declared dependency. ADR-063 grows the main pass's
  compile with those apps.
- Loading an app does not make it visible by name: an app sees only what it declares (`al_sema::workspace`,
  `visible_from`). The Test Runner app is loaded the same way.
- Once loaded, an extra app is a full member of the program: its objects are AllObj and Field rows, its subscribers are
  bound, its extensions change the objects they extend, its install code runs in the install baseline, and the test
  runner finds 130301 and 130302 by id (`RunnerReset::of`). The testlibraries suite checks all of this against BC.

bc-linux, which runs the corpus on BC, keeps an extra app installed by **name**, case-insensitive, together with its own
dependencies, and fails the job when a name matches no app in the artifact (`scripts/resolve-keep-app-ids.py:361-376`).
The apps it keeps were installed when the database was made, before the corpus apps are published.

So the mechanism exists and is checked. What is missing is a way for a user to ask for it.

## Decision

### 1. One concept: an installed app

An **installed app** is an app the modeled server has installed that no app under test depends on, directly or through
another app. The runner treats it exactly as it treats a dependency app, with two differences that BC has too:

- the apps under test cannot name its objects (they do not declare it; a reference is a compile error, as on BC);
- its test codeunits never run (only the tests of the apps under test run, as for every dependency app).

There is one owner for this in the code: a function in `al-testrun/src/deps.rs` that adds the installed apps of the
environment to the resolution. The corpus `extra_apps`, the Test Runner app and the user's declarations all go through
it. No second copy of the rule is added to `alrunner-cli`.

### 2. What becomes observable, and why it is all of it

Everything the program holds for a loaded app becomes observable. This is not a choice of the ADR: BC has no notion of
"installed for metadata only". A half-installed app would be a guessed environment (hard rule 3).

- **Metadata tables.** AllObj, AllObjWithCaption, CodeUnit Metadata, Table Metadata, Field, Key, the page and report
  metadata tables: rows for every object of the installed app (`platform/objects.rs`, `fields.rs`, `pages.rs`,
  `reports.rs` already list every loaded app).
- **App tables.** NAV App Installed App and Published Application get a row per installed app, with "Published As"
  from the existing rule (`platform/apps.rs`): Global for Microsoft's apps, Dev for others, Global with
  `--dependencies-published-global`.
- **Event subscribers run.** This is the largest consequence. Every subscriber of an installed app is bound and runs
  when its event is raised, in every test, as on BC. A subscriber that the runner cannot run refuses the test that
  reaches it (hard rule 4). Declaring a large app can turn passes into refusals; that is the honest answer. An example
  the corpus pins: Tests-TestLibraries codeunit 130011 "Backup Management" sets all four flags of
  `GetDatabaseTableTriggerSetup` for every table, so the global database triggers run on every write.
- **Extensions apply.** Table extension fields and triggers, page extension controls and triggers, report extensions,
  enum extension values and permission set extensions of the installed app change the objects of the closure they
  extend (the `relower_set` rule of ADR-022 and ADR-063).
- **Install code runs** in the install baseline, step 1, with the dependency apps (`al-testrun/src/baseline.rs`): its
  install codeunits and "Extension Triggers" subscribers, and its subscribers to Company Triggers and
  `Company-Initialize`. On BC that code ran when the app was installed.
- **The test runner's steps** find 131006, 130301, 130302 and 132553 by id when an installed app has them
  (`RunnerReset::of`, `TEST_RUNNER_CODEUNITS`). This closes #8160 for a user who declares Tests-TestLibraries.
- **Default test handlers** (BC 29, ADR-059) of an installed app apply to the test codeunits of other apps, as the
  corpus `xhandler` fixtures pin.
- **Login.** A non-Microsoft installed app's "System Initialization" `OnAfterLogin` subscribers follow the existing
  assumption `partner-login-subscriber-in-test-session` (`al-testrun/src/partner_login.rs`), as a partner dependency
  app's do.

### 3. Install order

Installed apps take their place in the canonical dependency order of ADR-022 (Microsoft's apps before the others, then
by dependencies, name and id). Their install code runs in that order, one app complete before the next (ADR-046 A3).

For Microsoft's apps this is what BC has: bc-linux and a BC container install the artifact's apps when the database is
made, before the user publishes anything, and the runner puts Microsoft's apps first. For a non-Microsoft installed app
(step 2 below) the order against an unrelated non-Microsoft app is not pinned. Step 2 needs a probe for it before it
runs such a pair (hard rule 3); until then a run with two unrelated non-Microsoft apps that both have install code
refuses with a reason that names the two apps.

### 4. How a user names an installed app

On the command line, `--installed-app VALUE`, repeatable, on `al-runner test`, `debug`, `serve` (as the server's
default, see 6), `impact-index build` and `test-microsoft`. VALUE is one of:

1. an app id (a GUID): found by id, as a declared dependency is;
2. a name: matched case-insensitively against the apps of the search list (see 5), as bc-linux does. A name that
   matches apps with different ids stops the run and lists them, so the user can pass the id;
3. a path to a package file, a source archive or an app folder (step 2 only).

There is no version on the command line. A Microsoft app comes in the build of the run, like every other Microsoft app.
A found package is used at the version found, under the same rules as a dependency (ADR-013 layer B, items 3 to 7).

The shape for a settings file, when there is one (open question 3), is the shape of an `app.json` dependency:
`"installedApps": [{ "id": "...", "name": "...", "publisher": "...", "version": "..." }]`, with `version` a minimum, as
in `app.json`.

### 5. Where an installed app is found

In the search list of the dependencies, in its order: `--dep`, `--package-cache`, each app's `.alpackages`, then the
artifact feed of the run's build. Its own dependencies are resolved together with the closure: one version per app id,
and a conflict is the same error as between two declared dependencies. ADR-013's rule that a package without source is
not found applies (hard rule 7: Microsoft's apps are compiled from their source).

### 6. Serve

- `al-runner serve --installed-app ...` sets the list for every request.
- The `runTests` and `execute` field `installedApps` (a list of strings in the forms of 4) **replaces** the server's
  list for that request when present. `[]` means none. A value that is not a list of strings answers `{"error":...}`.
- The warm slot's reuse key already holds the dependency identities (ADR-028 decision 3). Installed apps are part of
  the dependency set, so a request with another set compiles again; the same set reuses the warm compile.

### 7. What stops the run, and what refuses

Setup errors, before the compile, exit 2 with a message that names the app and the places searched:

- an installed app that is not found, or found only without source;
- a name that matches more than one app id;
- an installed app whose own dependencies cannot be met (not found, below a minimum, or in conflict with the version
  the closure resolved), under ADR-013's existing rules (the advisory minimum of the five platform apps stays);
- an installed app that depends on an app under test (it would have to be compiled after the workspace, which the
  cache layers do not allow; the message says to pass it as an app instead). Step 1 has no such case: no Microsoft app
  depends on a user's app.

`--allow-missing-dependencies` does not apply to installed apps. The user named them; a run without one is not the
server the user described. With `--no-download`, an installed app that is not on disk stops the run like any missing
dependency.

No-ops, with a line in the verbose log: an installed app that is already in the closure, one that is an app under
test, and the same app named twice.

At run time nothing new refuses: code of an installed app is dependency code and refuses as dependency code does.

### 8. Steps

- **Step 1: Microsoft's apps from the artifact**, named by name or id. This covers both issues (Tests-TestLibraries
  for #8160, Base Application and the rest for #8170) and is exactly what the corpus `extra_apps` and bc-linux do.
- **Step 2: other apps and paths** (a partner app installed on the user's server). Needs the install order probe of 3.
  Owner question 2 decides whether and when.

### 9. Output

The run's start prints one line per installed app (name, version, where it came from). `--output-json` and serve's
summary carry `installedApps` (id, name, publisher, version, origin), next to `skippedDependencies`. Both are additive.

### 10. Other features

- **`--affected` (ADR-050)**: the stored environment holds every dependency app (`alrunner-cli/src/affected.rs`,
  `environment`), so a changed installed set is a full run with the existing reason "the dependency apps or the BC
  build changed".
- **Matrix runs**: `--installed-app` is forwarded to every cell (`matrix.rs`, the list of forwarded flags).
- **`test-microsoft` and `--recipe microsoft`**: the declared apps are added to what the recipe loads.
- **Test data** (`--test-data`, ADR-020): the tables of an installed app are filled from the backup by the existing
  rule (by AL name and declaring app id), as a dependency app's are.

### 11. The corpus

- `oracle.toml` keeps the name `extra_apps`: it mirrors the corpus CI. `al-test-cli` hands the list to the same
  function as `--installed-app`.
- One behavior change: today an extra app that is not found is skipped with a note (`MissingPolicy::Skip`). With this
  ADR it stops the pass (exit 2), as for a user (owner question 4). A testlibraries pass without Tests-TestLibraries
  gives outcomes of another environment.
- `al-test-cli --no-deps` loads no app from the artifact, so it loads no extra app either, as today. Nothing stops;
  the run is not a CI-shaped run.
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
  open question 6.
- **A key in `app.json`.** `app.json` belongs to the AL compiler and describes the app, not the server it is tested on.
- **Serve: a server-level list only.** `packagePaths` is already a request field; an editor that switches projects
  needs the installed set per request as well.

## Consequences

- A user can describe the server the tests run on, and the runner gives that server's answer for #8160 and #8170.
- The default does not change: without `--installed-app`, a run models the closure, as today and as the corpus main
  environment.
- An installed app's code runs. Users who declare large apps get more refusals in the first versions; the refusal
  reasons name the routine and the app.
- The corpus `extra_apps` path and the user path are one code path, so the testlibraries suite checks the user feature.
- Not-found extra apps stop a corpus pass instead of being skipped.
- `docs/manual/cli-reference.md` and `tool-protocol.md` get the flag and the field in the build PR.

## Conformance

Red/green, in this order (hard rule 1):

1. **Refactor first, no behavior change.** `deps::load_corpus`'s extra apps and `with_test_environment` move into the
   one function of decision 1. `--baseline-diff` on BC 27, 28 and 29 shows no change; `metrics/<major>/work.json` does
   not change; `tools/fresh_vs_grown.sh run` shows 0 differences.
2. **Rust tests for what the corpus cannot reach** (CLI, protocol and refusal plumbing), each red before the code:
   - `alrunner-cli` args: `--installed-app` with an id, a name, repeated; a path refused in step 1 with a reason;
     the serve field `installedApps` accepted, replacing the server list, `[]` clearing it, a non-list answered with
     an error.
   - `al-testrun` dependency tests on a pool of fixture packages made in the test (no Microsoft content): not found,
     found without source, a name with two ids, an unmet dependency, a dependency on an app under test, each exits 2
     with its message; `--allow-missing-dependencies` does not skip one; an app already in the closure is a no-op.
   - End to end on fixtures: an app with no dependencies and an installed fixture app with a table, a codeunit, an
     install codeunit and a subscriber to an event the test raises. The test sees the codeunit in AllObj, the table's
     fields in Field, the install rows and the subscriber's effect; a reference by name to the installed app does not
     compile. Red without the flag, green with it.
3. **The corpus is the oracle for Microsoft's apps.** The testlibraries suite pins AllObj rows of an installed
   non-dependency app (`BiTpEnv.al:27-30`), the #8160 shape (`BiTpDefault.al:35-45`) and Backup Management's trigger
   flags. A check runs that suite through `al-runner test` with `--installed-app Tests-TestLibraries --installed-app
   "Data Archive"` (the corpus apps as `--dep` folders) and compares its outcomes with the `al-test-cli` pass: 0
   differences on BC 27, 28 and 29.
4. **A probe for #8170's Field part.** Only AllObj is pinned for an installed non-dependency app. A corpus probe in
   `al-language-testlibraries` pins a Field scan by `FieldName` without `TableNo` that reaches a table of Data Archive
   or Tests-TestLibraries, and that table's Table Metadata and AllObjWithCaption rows. Until it is pinned, those rows
   for an app loaded as installed run through an assumption entry that cites the probe PR and the reporter's
   measurement in #8170 (hard rule 3).
5. `tools/baseline_rule_check.sh run` and `tools/demand_vs_eager.sh run` stay at 0 differences.

## Open questions for the owner

1. **Flag name.** `--installed-app` (recommended: it says what the app is on the server) or `--extra-app` (the corpus
   and bc-linux word).
2. **Non-Microsoft apps and paths (step 2).** Build only after a user asks, with the install order probe first
   (recommended); build together with step 1; or never.
3. **Settings file.** None in step 1, CLI and serve only (recommended; the settings file is a separate decision,
   `docs/ideas.md` "Build pin"); a new `alrunner.json` next to `app.json` now; or a key in `app.json`.
4. **Corpus `extra_apps` not found.** Stop the pass (recommended: the outcomes would be of another environment); or
   keep today's skip with a note.
5. **Serve field and server list.** The request list replaces the server's list (recommended: a client can then also
   ask for none); or it adds to it, as `packagePaths` does.
6. **Presets.** No preset now (recommended); later, after a probe measures it, a preset for the apps a BcContainerHelper
   container has installed, or for the installed apps of the artifact's demo database (its NAV App Installed App
   table, read with `al-bcdb`).
