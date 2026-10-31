# ADR-013: Dependency apps and platform objects (objects outside the workspace)

- Status: Accepted (2026-10-13)
- Date: 2026-10-13
- Revised: 2026-10-13, to record the owner's answers to the five open questions and the acceptance

## Context

The corpus references System Application, Base Application, test-library and
platform objects whose source is not in the corpus (its manifest names System
Application, Base Application, System Application Test Library and
Permissions Mock as dependencies). Today the runtime treats every such
reference the same way:

- `al-sema/src/workspace.rs` (`resolve_references`) reports
  `AR0002 unresolved <kind> '<name>'`. `crates/al-sema/tests/expected-unresolved.txt`
  is the golden list: 202 distinct objects, 1062 references (table 112
  objects / 736 references, page 43 / 102, report 18 / 68, codeunit 12 / 81,
  enum 11 / 58, xmlport 4 / 12, query 2 / 5). `corpus_resolve.rs` fails if it
  changes, so it is already the backlog of this ADR.
- `al-sema/src/check.rs` does not guess: fields of an unresolved table have
  `Ty::Unknown`.
- `al-sema/src/lower.rs` (`resolve_obj`, `rt`) refuses a variable of an
  unresolved type, so the test is reported not run with
  `` `Var`: <kind> `X` is outside the workspace``.
- The CLI (`al-test-cli/src/main.rs`) loads exactly two apps, the corpus
  internals fixture and the corpus app (`runtime::load_app` reads `app.json`
  and `*.al`). The workspace model already has what dependencies need:
  `AppSource { id, name, dependencies, files }` and per-app visibility by
  dependency id (`visible_from`).

### Decision of the project owner (2026-10-13)

Fake AL objects of dependent apps are rejected, and so is hand-writing
Business Central / System Application objects. The direction, in the owner's
words: enable a way to either point the runtime at, or have it automatically
download, the dependent artifacts from the Microsoft feeds, extract the AL and
recompile it with the Rust compiler, and execute that compilation. The runtime
is still in spike mode and is not being built as a standalone tool like the
original AL Runner. Partial tables are out: real schemas come from the real
apps' source. Environment, identity, session, company and user answers mirror
what AL Runner (github.com/StefanMaron/BusinessCentral.AL.Runner) does.

The same day the owner answered the five open questions that this ADR first
raised. The answers are recorded verbatim under "Answered by the project owner"
near the end. The decision section reflects them. One question remains, on
`DotNet` types.

## Survey: what is outside the workspace

`python3 tools/external_survey.py . <junit>` (added with this ADR) groups the
skip reasons of a run (HEAD c30ea3d, 1308/4836 pass) by first blocker. 516
tests (10.7 % of the corpus) stop at "outside the workspace": 453 on tables,
55 on codeunits, 8 on enums. A test is counted once, under the first thing that
stops it.

| Class | Objects | Tests | Examples (tests) |
|---|---:|---:|---|
| A. Program-metadata virtual tables | 19 | 203 | CodeUnit Metadata 23, Field 20, Page Control Field 19, AllObjWithCaption 18, AllObj 15, Table Metadata 12, Page Metadata 12, Report Layout List 12 |
| B. Permission tables | 7 | 59 | Expanded Permission 18, Metadata Permission Set 13, Permission Set 10, Aggregate Permission Set 7 |
| C. Generator / reference tables | 4 | 44 | Date 22, Integer 11, Time Zone 6, Windows Language 5 |
| D. Host, identity, platform bookkeeping tables | 12 | 72 | Record Link 18, All Profile 11, Session 10, Company 10, Active Session 9 |
| E. Base App business tables | 34 | 75 | Customer 9, Job 7, Dimension Correction 7, VAT Rate Change Setup 5, Location 5 |
| Codeunits | 9 | 55 | Temp Blob 28, Permissions Mock 6, Reten. Pol. Allowed Tables 5, Environment Information 4, Application Area Mgmt. Facade 4, User Permissions 3 |
| Enums | 3 | 8 | Printer Paper Kind 4, Job Queue Report Output Type 3 |

Members the corpus calls on the codeunits: `Temp Blob` `CreateOutStream` (33
calls), `CreateInStream` (25), `Length` (3); `Permissions Mock` `IsStarted`,
`Start`, `Stop`, `Set`, `ClearAssignments`, `Assign`; `Environment
Information` the six `Is*` predicates (the tests assert that the answers agree
with each other, not fixed values); `User Permissions` `IsSuper`; `Record Link
Management` `CopyLinks`. The survey script also prints the table members and
fields touched through declared variables.

### What the Microsoft artifacts contain (measured, BC 27.5.46862.56249)

All requests were made from this container. Only the entries named were
downloaded, and everything was extracted into `/tmp`, outside the repository.

| Fact | Evidence |
|---|---|
| Feed | `https://bcartifacts-exdbf9fwegejdqak.b02.azurefd.net/sandbox/<build>/<w1\|platform\|<country>>`, a ZIP served with `Accept-Ranges: bytes`. Version list `.../sandbox/indexes/w1.json` (90 KB, 1315 builds, `{Version, CreationTime}`). Reachable here: HEAD and ranged GET answered 200/206. The w1 artifact is 900 MB, the platform artifact 1.3 GB. |
| Other hosts | `api.nuget.org` answers 200. `bcartifacts.azureedge.net` (an older host) is refused by the proxy (502); AL Runner does not use it. |
| Range reads work | The ZIP central directory (125 entries for w1, 6917 for platform) was read with two ranged GETs; single entries with a 30-byte local-header GET plus one ranged GET, inflated locally. |
| The `platform` artifact has plain source | `Applications/<Name>/Source/<Name>.Source.zip`, a repository-layout zip with `app.json` and the `.al` files. Base Application 27 MB (8990 entries, 7868 `.al`, 100 MB of AL), System Application 9.6 MB (1264 `.al`, 6.5 MB), Business Foundation 0.4 MB (96 `.al`), plus `Application`, `System Application Test Library` (0.17 MB, depends on System Application and `Any`; holds `Azure AD Plan Test Library`), `TestFramework/TestLibraries/{Any,Assert,permissions mock}`, `TestFramework/TestRunner` and `BaseApp/Test/Tests-*.Source.zip`. What the corpus needs is about 40 MB. |
| The `w1` artifact has precompiled apps | `Extensions/Microsoft_*.app`: a NAVX header, then a ZIP holding ReadyToRun DLLs and a nested inner `.app`, which holds `src/*.al` (7868 files for Base Application; the outer wrapper holds none), `SymbolReference.json` (59 MB for Base Application) and `NavxManifest.xml`. Same source as the `Source.zip`. |
| Platform system tables have AL declarations | `platform` artifact, `ModernDev/pfiles/microsoft dynamics nav/270/al development environment/System.app` (0.5 MB, NAVX header then ZIP). 338 `.al` files declaring 217 tables (73 under `Virtual Tables`, 90 `Tenant Database Tables`, 28 `Application Database Tables`, 7 `Temporary Tables`) and 24 codeunits (the `*Triggers` event publishers, agent and feature helpers). It contains every system table the corpus misses (Date, Integer, AllObj, AllObjWithCaption, Session, Active Session, Company, User, Field, Key, Record Link, Permission Set, Code Coverage, CodeUnit Metadata, Time Zone, Windows Language, Expanded Permission, Aggregate Permission Set, Page Control Field). Only 28 of the 338 files contain a code body: declaring a virtual table and providing its rows are different things. |
| `DotNet` interop is real code in the apps | System Application declares 545 `DotNet` types and 134 of its files use them; Base Application 220 files. Examples: `Array`, `MemoryStream`, `Convert`, `Encoding`, `UserInfo`, and the platform helpers `NavUserAccountHelper` and `NavTenantSettingsHelper` (`Environment Information Impl.` reads the environment type through the latter). They have no AL source: they are .NET types in Microsoft's assemblies. |
| License signals | Most `.al` files start with a "Licensed under the MIT License" header (6748 of 7868 in Base Application, all but 7 in System Application, every platform `System.app` file looked at). The manifests say `ShowMyCode="False"`, `Target="OnPrem"`. The owner accepted downloading and compiling it with nothing committed (answer 1). |

### What AL Runner does (source: the checkout at `/home/stefan/Documents/Repos/community/BusinessCentral.AL.Runner`, commit 351c2813d, 2026-10-06; paths below are relative to it)

AL Runner is a C# tool that loads Microsoft's own `Microsoft.Dynamics.Nav.*`
DLLs and runs the precompiled application code. It does not recompile
Microsoft's AL. Where this ADR differs, the owner's direction has priority.

1. **Obtaining dependency apps.**
   - Feed and ranged ZIP reads: `AlRunner.Provisioning/ArtifactDownloader.cs`
     (`CdnBase` line 30; `BuildArtifactUrl`; `TryHeadContentLength`,
     `TryReadCentralDirectory` and `PlatformApps` read the central directory
     and fetch only the wanted entries). Version list `indexes/w1.json`
     (line 920). Network failures are classified in
     `AlRunner.Provisioning/NetworkDiagnosis.cs`; an unanswered probe holds the
     chosen version instead of falling back (`docs/provisioning-tiers.md`).
   - What it takes: from the `w1` artifact, `Extensions/` entries matching a
     curated five-app list (`W1PlatformAppPrefixes`, lines 646-659: Base
     Application, System Application, Business Foundation, Application,
     Application Test Library); from the `platform` artifact, the roughly 55
     service-tier DLLs and the test apps (`TestApps`). The AL compiler comes
     from `api.nuget.org` (`AlCompiler`).
   - Layout and cache: `~/.local/share/al-runner/artifacts/<build>/`
     (`AL_RUNNER_ARTIFACTS_ROOT`), `~/.cache/al-runner/` for compiled output,
     `ncl-cecil` and `install-baseline` (`AL_RUNNER_CACHE_ROOT`, `--cache`),
     `~/.bcartifacts.cache/sandbox/<build>/` (the VS Code AL cache, read for
     reuse), `--package-cache DIR` for local `.app` folders. Completeness
     predicates: `docs/provisioning.md`. Selection by app id and minimum
     version: `AlRunner/DependencyResolver.cs`.
2. **Source versus symbols versus DLL.** `docs/where-metadata-comes-from.md`
   and `docs/dependency-metadata-from-bc.md`: a dependency is served at one of
   four tiers (a committed precompiled sidecar, a ReadyToRun DLL inside the
   `.app`, a service-tier DLL, or, for packages that ship only source, a compile
   of `src/*.al`; `AppLoader.HasAlSource`, `AlRunner/AppLoader.cs:819`).
   Microsoft apps are served precompiled; their source is deliberately not
   compiled again because loading the Base Application floor costs about 70 s
   cold and 6 s warm, and compiling it measured 257 s and 8.8 GiB (that page).
   Table metadata for them comes from `SymbolReference.json`. Reading `.app`
   containers, including the `.NEA` runtime-package variant:
   `docs/runtime-packages.md`.
3. **Identity, environment, session, company, user.** The runner builds a
   skeleton `NavSession` by field-poking (`AlRunner/BcRuntime.cs`):
   - `UserId()` is `"TESTUSER"` (lines 1427-1458; `docs/scope.md` section 2);
     `UserSecurityId()` is `{C0A1BDFA-0000-0000-0000-545553545553}` (line 1441);
     the User table gets one matching row
     (`Patches/RecordPatches.UserSystemTable.cs`).
   - `CompanyName()` is `"My Company"` (line 1309; `docs/scope.md`); its company
     id is `{C0A1BDFA-0000-0000-0000-43524F4E5553}`; one Company row is seeded
     (`Patches/RecordPatches.CompanySystemTable.cs`).
   - `SessionId()` is positive (1; first written as 0, corrected below); the Session table has exactly one row, the reading
     session, as BC's own provider returns, and Active Session one row
     (`Patches/RecordPatches.SessionVirtualTable.cs`,
     `RecordPatches.ActiveSessionSystemTable.cs`).
   - Culture is en-US (LCID 1033, around `BcRuntime.cs:1490`); the tenant id is
     `"default"` and the file-tenant sentinels are (empty Guid, `"STANDALONE"`)
     (around `BcRuntime.cs:2516`); a populated tenant is not implemented
     because it broke about 466 tests.
   - Environment: the tenant carries BC's default `EnvironmentType = Production`
     (`Patches/MetadataPatches.cs` lines 781-835), so `Environment Information`
     answers `IsProduction() = true`, `IsSandbox() = false`, `IsSaaS() = false`;
     `IsOnPrem()` is `ApplicationIdentifier() = 'NAV'`; service topology is
     always on-premises (`ProgramSupport/CliText.cs` lines 311-322,
     `docs/limitations.md#environment-type`). A test can switch the tenant to
     sandbox through the Microsoft test libraries, as on a service tier.
   - Permissions: everything is granted (a SUPER position for the session
     user); permission enforcement is a deliberate, permanent divergence, and
     asking about another user is refused with `RunnerOutOfScopeException`
     (`docs/scope.md` sections 2 and 3.17).
   - The corpus CI runs its tenant as a sandbox, so a corpus test cannot pin
     the runner's default (`docs/limitations.md` line 1150). The corpus tests
     that touch these answers (`session/`) assert agreement (`CompanyName()`
     equals `CurrentCompany()`, `IsProduction()` is `not IsSandbox()`), not
     fixed values, so the AL Runner defaults are compatible.
4. **Objects with no AL source.** `docs/scope.md` has four buckets: real code,
   faithful replacement, out of scope (`RunnerOutOfScopeException` naming the API
   and a section anchor) and in scope but not implemented. System tables are
   served two ways:
   - by Microsoft's own providers inside `Ncl.dll`: Date (2000000007) and
     Integer (2000000026) (`Infrastructure/NclCecilRewrite.Runtime.cs` around
     lines 962 and 1216; `docs/limitations.md` line 1273);
   - by the runner, one file per table under `AlRunner/Patches/`:
     `RecordPatches.AllObjVirtualTable.cs` and
     `RecordPatches.AllObjWithCaptionVirtualTable.cs` (rows from one shared
     inventory of the compiled objects; schema read from the platform
     `System.app` source, `docs/virtual-tables-allobj.md`),
     `RecordPatches.CodeunitMetadataVirtualTable.cs`,
     `...TableMetadataVirtualTable.cs`, `...PageMetadataVirtualTable.cs`,
     `...FieldVirtualTable.cs`, `...KeyVirtualTable.cs`,
     `...EventSubscriptionVirtualTable.cs`, `...PermissionSetSystemTable.cs`,
     `...RecordLinkTable.cs`, `...TimeZoneVirtualTable.cs`,
     `...WindowsLanguageVirtualTable.cs`, and so on. Each states what real BC
     answers (measured by decompiling the provider) and which columns it cannot
     fill; a shape nobody provided is refused, not emptied
     (`RecordPatches.VirtualTableShapeGap.cs`).
   - Platform behaviour that is .NET (the `NavTenantSettingsHelper` example
     above) is handled by patching the engine, or runs as the real BCL type.

5. **Dependency version selection.** Two layers.
   - *Which BC build.* `--bc-version X` (an exact build or a prefix) or
     `--artifact-path DIR`; they are mutually exclusive (`AlRunner/Program.cs`
     lines 105 and 396-401). `BcArtifacts.SelectArtifactVersionDir`
     (`AlRunner/Infrastructure/BcArtifacts.cs:292`) lists the
     version-named directories under the artifacts root and takes the highest, or
     the highest whose dot-separated name starts with the prefix (`27.5` matches
     `27.5.x`, not `27.50`). No match is an exception that names the builds
     available and the download command; there is no silent fallback. With
     neither flag the default is bound to the engine the binary was built for
     (`Program.cs` lines 1068-1230): the exact build, then its minor, then its
     major. The cache is asked first, then the CDN when auto-provision is on (the
     default; `--no-auto-provision` opts out) (`ResolveProvisionTargetCore`,
     `BcArtifacts.cs:504`). A CDN probe that gets no answer holds its tier and
     does not fall to a looser one (`docs/provisioning-tiers.md`). The comment at
     `Program.cs:1068` gives the reason: the binary can only faithfully run its
     own major, so a stray download of another major must never become the
     default. The project's `app.json` is read as a floor: the higher of
     `application` and `platform` (`TryDeriveBcFloorFromProject`,
     `AlRunner/ProgramSupport/Provisioning.cs:449`; `ReadMinimumBcVersion`,
     `AlRunner/Infrastructure/InProcessAppPackager.cs:178`). The default never
     selects below it. An explicit `--bc-version` below it runs, with a warning
     (`AlRunner/Infrastructure/BcVersionFloor.cs`).
   - *Which package satisfies each dependency.* `AlRunner/DependencyResolver.cs`.
     Inputs: the roots are the `dependencies` of the project's `app.json`
     (`ReadDependencies`, `AlRunner/ProgramSupport/Dependencies.cs:261`) plus a
     default test-tool app; every chosen package then contributes its own
     `<Dependencies>` from `NavxManifest.xml` and its Platform and Application
     floors as extra roots (`AppLoader.ImplicitRoots`, line 233). The pool is
     every `.app` file under the search directories, indexed by app id and by
     (name, publisher), all versions kept (`EnsureIndexed`, line 583). The
     directories are the bundle's `.alpackages`, then the package-cache
     directories: `--package-cache`, or by default the VS Code AL symbol cache
     `~/.bcartifacts.cache/sandbox/<build>` and a curated symbols directory
     (`DefaultPackageCacheDirs`, `Provisioning.cs:197`), plus the runner's own
     `<artifacts>/<build>/platform-apps` and `test-apps`, which provisioning adds
     to the set (`Program.cs` lines 2203 and 2247). Rules (`TryFind`,
     line 322; `SelectBestVersion`, line 343): a lookup by app id is
     authoritative, and if candidates exist for the id but none meets the minimum
     it does not fall through to name and publisher; name and publisher are used
     only when the id is empty or unknown. A candidate below the declared version
     is dropped, because the version is a minimum. Among the rest, in order: a
     package built from source in this run, then the executable tier (R2R DLLs,
     then AL source or a sidecar DLL, then nothing), then the highest version
     (lines 453-479). For the five Microsoft platform apps the minimum is advisory
     when the winner cannot execute and a below-minimum package can (lines
     496-512). No match: an optional dependency or a Microsoft platform app is
     skipped with a printed line (lines 159-175); candidates that are all below the
     minimum raise `DependencyVersionMismatchException` listing the versions; no
     candidate raises `MissingDependencyException` naming the directories searched
     (lines 176-197). A cycle raises `InvalidOperationException` (line 202).
     The output is a post-order depth-first walk, so dependencies come before
     dependents (line 242). Caches: the candidate index lives in memory for one
     resolver and is never written to disk. Downloaded builds live under
     `~/.local/share/al-runner/artifacts/<build>/` (`AL_RUNNER_ARTIFACTS_ROOT`).
     Other caches sit under `~/.cache/al-runner/<name>` (`CacheRoots.cs`).
   - The corpus `app.json` declares platform, application and the Microsoft
     dependencies as `27.0.0.0`. To AL Runner that is a floor and nothing more. The
     build that runs is the one it selects in the first layer.
6. **Install baseline.** The code is `AlRunner/InstallTriggerRunner.cs`,
   `AlRunner/CompanyInitializer.cs`, `AlRunner/TestExecutor.cs` (lines 540-870) and
   `AlRunner/Patches/RecordPatches.InstallBaseline.cs`.
   - *Which apps, in what order.* Every loaded dependency assembly in dependency
     order (`DependencyLoader.LoadAll` returns them dependencies first), then the
     project's own test assembly last (`InstallTriggerRunner.RunAll`, line 99).
     Within an app, every codeunit that carries a `OnInstallAppPerCompany` or
     `OnInstallAppPerDatabase` subscriber to the system codeunit 2000000010
     "Extension Triggers" (`Scan` and `InstallTriggerMethod`). For each codeunit,
     per-company runs first, then per-database (`FireAll`, line 144).
   - *Install, Upgrade, Company-Initialize.* Only install triggers run. A search
     of `InstallTriggerRunner.cs` and `TestExecutor.cs` finds no call to an upgrade
     trigger. Base Application codeunit 2 `Company-Initialize` runs once, after the
     dependency install triggers and before the dependency snapshot, and only if
     that codeunit exists in the loaded apps (`CompanyInitializer.EnsureCompanyInitialized`,
     line 128). If it aborts, the rows it wrote are kept, and the abort is a
     run-level result with exit code 2 (`docs/partial-company-initialization.md`).
   - *When it runs.* Once per app group (one project and its dependency closure),
     before the first test (`TestExecutor.Run`).
   - *Steps in `TestExecutor.Run`.* Reset per-test state. Seed the Published
     Application rows of the dependency apps, the session User row, the Company
     row, and the Access Control row that makes the user SUPER, because dependency
     install code reads them. Run the dependency install triggers
     (`RunDependenciesOnly`). Run `Company-Initialize`. Capture the "dependency plus
     company" snapshot. Then seed the project's own Published Application row and
     the Active Session row, run the project's own install triggers
     (`RunTestAssemblyOnly`), and capture the baseline.
   - *Snapshot.* The rows of the in-memory table providers are deep-copied, plus
     isolated storage, record links and the auto-increment counters
     (`InstallBaselineSnapshot`). The self-populating virtual tables (AllObj,
     Field, the metadata tables) are not snapshotted; they are derived again on
     every access. The dependency-plus-company snapshot is cached in memory per
     key, and on disk at `~/.cache/al-runner/install-baseline/<sha256>.bin`
     (`InstallBaselineDiskCache.cs`). The key is the dependency assembly set, the
     app symbol state, the subscriber assemblies and the test-data options
     (`CurrentInstallBaselineCacheKey`, line 460), plus the runner build for the
     disk copy. A disk copy is not written when `Company-Initialize` aborted.
   - *Restore.* Under the default `TestIsolation = Codeunit`, the baseline is
     restored at every test-codeunit boundary (`RestoreInstallBaseline`,
     `TestExecutor.cs:939`). Under `Test` isolation it is restored before every
     test. The code comment explains why: real BC rolls each test back but keeps
     committed install data.

### Experiments: what real source buys (measured, no code committed)

A temporary patch to `al-test-cli/src/main.rs` (reverted, not part of this
commit) loaded extra app directories ahead of the corpus apps; each was
extracted from the artifacts above into `/tmp`. Counts are the corpus run
against the BC 27.5 oracle.

| Run | Pass | Fail | Not run | Compile | Note |
|---|---:|---:|---:|---:|---|
| Baseline (main) | 1308 | 0 | 3528 | about 0.5 s | |
| + System Application, Business Foundation, Permissions Mock source (12.6 MB of AL in all) | 1312 | 0 | 3524 | 1.2 s (run 1.8 s) | Temp Blob resolves and its 3 direct tests pass |
| + Base Application source (113 MB of AL, 10 966 objects) | 1312 | 0 | 3524 | 16.5 s (run 80 s) | parse 1.9 s, workspace load 2.8 s, members 0.7 s, check 8.9 s (129 853 diagnostics, our gaps), lower 4.0 s; the rest of the 80 s is test execution, not profiled |
| + platform `System.app` declarations as ordinary tables | 1331 | **160** | 3345 | 1.2 s | virtual tables become empty stored tables: silently wrong answers |
| all of the above | 1331 | **157** | 3348 | 16.5 s (run 110 s) | 10 previously passing tests stop passing (not investigated) |

Re-estimate of the 516 "outside the workspace" tests:

- With real System Application, Business Foundation, Permissions Mock and Base
  Application source, 125 of the 516 move past that blocker and the 391 that
  remain are all system tables (47 objects, classes A to D, plus `Azure AD Plan
  Test Library`, which is in the System Application Test Library source, not
  loaded in this run). Every other codeunit, enum and Base App business table
  in the survey resolves.
- With the platform `System.app` declarations as well, 507 of 516 (98 %) pass
  that blocker; the other 9 are the test-library codeunit.
- Realised passes are small because the next blockers are independent. With
  source alone only the 3 `Temp Blob` tests pass (+4 in total). With the
  platform declarations 31 of the 516 pass, 157 fail (empty virtual tables) and
  about 320 stop at an unimplemented builtin or construct, most often
  `Database.CompanyName`, `NavApp.GetCurrentModuleInfo`, `Record.FieldNo`,
  `System.DMY2Date`, `Record.AddLink`, `Database.ServiceInstanceId`,
  `Report.SaveAs`, `RecordRef.GetTable` and `TestPage`. The 157 failures are the
  tests that become passes once the platform tables get row providers.

Two findings drive the decision:

1. The failures prove that a declared schema without behaviour is worse than
   refusal: a platform table whose provider does not exist must be refused at
   compile time, never stored and returned empty (hard rule 4, ADR-009).
2. Compiling Base Application costs 16.5 s here, far over the 1 s budget the
   corpus has, but the tests reach a tiny part of it. Refusals must stay
   explicit and only reachable code may matter, which the static
   `blocked_reason` call-graph check (`al-test-cli/src/runtime.rs`) already does
   for execution; the compile cost needs the same treatment.

## Decision (revised 2026-10-13 after the owner's answers; accepted)

1. **Dependency apps are Microsoft's source, downloaded into a cache and
   compiled by our front end.** Licensing is settled by the owner's answer: the
   source is downloaded into a local cache and a CI cache, compiled, and nothing
   is committed.
   - *Local path.* `--deps <dir>` (repeatable): a directory holding any of
     `*.Source.zip`, `*.app` (plain or R2R-wrapped; the nested inner `.app` is
     unwrapped), `System.app`, or already extracted source trees. This is the
     offline and CI-cache route. It plays the role of AL Runner's
     `--package-cache`.
   - *Automatic download.* When no package in the search set satisfies a
     dependency, download it from the `platform` artifact of the selected build
     (decision 2) by ranged ZIP reads, only the entries needed:
     `Applications/*/Source/*.Source.zip` for the apps in the manifest and their
     dependencies, `System.app` and the TestFramework libraries. About 40 MB for
     the current corpus out of a 1.3 GB artifact; the `w1` artifact is not
     needed. `--no-download` makes a missing app an error naming the app, the
     build and the URL.
   - *Integrity.* Every extracted entry is checked against the CRC-32 in the ZIP
     central directory; the cache keeps the entry bytes as downloaded and
     records build, entry name, size and CRC.

2. **Version selection is driven by the project's `app.json` and mirrors AL
   Runner.** The runtime is not corpus-centric. It works the same for any
   project. Two layers, as in AL Runner (item 5 of the AL Runner section above).

   *Layer A: which BC build supplies the Microsoft apps.*
   1. An explicit `--bc-version PREFIX` (an exact build, or a dot-segment prefix
      such as `27.5`) wins. The newest matching build in the artifact cache is
      used; if there is none, it is downloaded.
   2. Otherwise the target is the major of the project's floor. The floor is the
      higher of `application` and `platform` in `app.json`
      (`ReadMinimumBcVersion`). Within that major, the newest cached build is
      used, and if none is cached, the newest published build is downloaded.
      A stray build of another major never becomes the default.
   3. The default never selects a build below the floor. An explicit
      `--bc-version` below the floor runs, with a printed warning.
   3a. **Amended by ADR-015 (accepted 2026-10-13).** If the major of the floor is
      below 27, the target is BC 27 and the run prints a note. The runtime
      supports BC 27 and later only, and an app that declares an older floor
      still runs on a BC 27 or newer server. An app `runtime` below 16.0 is
      treated as 16.0, with a note. `--bc-version` below 27 exits 2.
   4. No match is an error. It names the request, the builds available and the
      cache directory. It never falls back silently.

   AL Runner's default is "the build the engine was compiled for", which has no
   equivalent here because we have no .NET engine. Step 2 is therefore our
   adaptation of AL Runner's "stay within the engine's major" rule. Everything
   else in Layer A is AL Runner's behaviour.

   *Layer B: which package satisfies each dependency.* This is
   `DependencyResolver` unchanged in rules:
   1. The pool is every app package under the search directories: the project's
      `.alpackages`, then `--deps` directories, then the selected build's
      packages in the cache. Index by app id and by (name, publisher). Keep every
      version.
   2. Roots are the `dependencies` of `app.json`. The platform floors add the
      implicit roots Microsoft/System and Microsoft/Application. Each chosen
      package's own declared dependencies are followed.
   3. Lookup by app id is authoritative when the id is known. If candidates exist
      for that id but none meets the minimum, do not fall through to name and
      publisher. Fall back to name and publisher only when the id is empty or
      unknown.
   4. A declared `version` is a minimum. Among candidates at or above it, the
      highest version wins.
   5. Output is a post-order walk: dependencies before dependents. A cycle is an
      error naming the path.
   6. Candidates all below the minimum: an error naming the app, the required
      minimum and the versions found. No candidate at all: an error naming the
      app, the version, the directories searched, and the way to supply it.
      An optional dependency that is missing is skipped with a printed line.
   7. The five Microsoft platform apps (Base Application, System Application,
      Business Foundation, Application, System) ship one package per build. For
      them the minimum is advisory when the selected build's package exists but
      is below it. The run uses that package and prints a note. AL Runner does
      the same (`SelectBestVersion`).

   Differences from AL Runner, on purpose. AL Runner skips a missing Microsoft
   platform app with a warning, because it has the DLLs. We cannot run without
   the source, so a missing one is an error. AL Runner ranks R2R packages above
   source above symbol-only ones. We have one kind of executable package, source
   with bodies. A package with no AL source ranks below one that has it (this
   keeps a symbol-only copy from shadowing the real one). Source-built workspace
   packages do not exist here yet.

   *Keeping corpus runs comparable to the oracle.* The oracle junit comes from
   BC 27.5 (`oracle/oracle.toml`, `bc_version = "27.5"`), but the corpus
   `app.json` only says `27.0.0.0` for platform, application and the Microsoft
   dependencies. Run through the generic rules, the floor alone would select the
   newest 27.x build. That might not be 27.5, and any other build can change
   results. The corpus is therefore handled by the harness, not by the runtime:
   - In corpus mode (`al-test-cli --root`), `al-test-cli` reads `bc_version`
     from `oracle/oracle.toml` (read only; `oracle/` is never edited) and passes
     it as the explicit `--bc-version`. The runtime crates never read
     `oracle.toml`. Any other project gets the generic rules.
   - The corpus checkout already carries a `.alpackages` for the app
     (`corpus/tests/al-language/.alpackages`: System Application, Base
     Application, Business Foundation, Application at 27.5.46862.48827, with full
     source; checked 2026-10-13, 7866 `.al` files in Base Application, 5638 with
     a `begin` block). It is in the pool as the project's own `.alpackages`. With
     `--bc-version 27.5` the newest cached 27.5 build wins over it. With
     `--no-download` and an empty cache it is the fallback, so an offline corpus
     run is possible with no download.
   - Every run prints the full build it selected and writes it into the junit
     and the pass-rate report (`bc_build`). A build change that flips a case then
     shows in the ratchet as a regression with its cause visible.
   - Optional, not required: ask the corpus CI to record its full build number in
     its junit. Once it does, the harness passes the exact build. That is a
     corpus-repo issue under hard rule 2; this repository does not edit
     `oracle/`.

   The residual risk is builds inside one minor, such as 27.5.46862.48827 against
   27.5.46862.53931. AL Runner measured differences between BC minors as dozens of tests in one
   large project and as none on the corpus (`docs/provisioning-tiers.md`). It did
   not measure builds inside one minor. We treat same-minor skew as probably small,
   always visible, and not a blocker.

3. **Never in the repository.** Microsoft source and the downloaded zips live in
   a cache directory outside the repo (default
   `~/.cache/al-runtime/artifacts/<build>/`; `ALRUNNER_CACHE` or `--cache-dir` moves it, see "Layer A as built") and in
   a CI cache keyed by build. A test (`no_microsoft_source_in_repo`) fails if any
   tracked file carries the Microsoft copyright header. The corpus submodule's
   own `.alpackages` is the corpus repository's content, not ours; the test
   skips `corpus/`.

4. **Platform apps are apps.** The platform `System.app` is loaded as an app with
   its real id and is an implicit dependency of every app (the corpus manifest
   does not list it; BC always provides it). Dependency apps are marked in
   `AppSource` (`kind`, `version`) so that no test is discovered or run in them
   and diagnostics name the app.

5. **The install baseline mirrors AL Runner.** Item 6 of the AL Runner section
   above describes what it does. For this runtime:
   1. It runs once per run, before the first test, for the loaded dependency
      apps and then for the project's own app.
   2. Order: seed the platform rows install code reads (Published Application
      rows for the dependency apps, the session User, the Company, the Access
      Control row that makes the user SUPER). Then each dependency app's
      `Subtype = Install` codeunits in dependency order; within a codeunit
      `OnInstallAppPerCompany`, then `OnInstallAppPerDatabase`. Then the Base
      Application codeunit 2 `Company-Initialize`, once, if Base Application is
      loaded. Snapshot (ADR-004). Then the project's own rows (its Published
      Application row, the Active Session row) and its own install codeunits.
      Snapshot again.
   3. Only install triggers run. Upgrade triggers do not, as in AL Runner.
   4. Each test codeunit (and each test, under `TestIsolation = Test`) starts
      from that snapshot. Self-populating virtual tables (AllObj, Field and the
      metadata tables) are not part of it; they are computed on read.
   5. An install trigger that fails is reported with the codeunit and trigger
      name, and the failure is not swallowed. A failed `Company-Initialize` keeps
      its partial rows and is recorded as a run-level result, as in AL Runner
      (`docs/partial-company-initialization.md` there).
   6. An install trigger that cannot run yet, because a platform table it
      touches has no provider, refuses the tests that depend on that app. It is
      never skipped silently (hard rule 4).
   7. The baseline is extra roots for decision 6. Install code and
      `Company-Initialize` reach a large part of Base Application. The compile
      cost of that is measured in the Base Application increment, not assumed.
      A persistent copy of the snapshot is an optimisation (AL Runner keeps one,
      keyed by dependency set, runner build and BC version).

6. **Compilation is demand-driven.** This is the decision, as the owner asked
   ("anything is allowed as long as the test results speak for themselves").
   The workspace indexes every dependency object by id and name with a cheap
   declaration scan, parses a file only when one of its objects is needed, builds
   member tables as today, and checks and lowers only routines reachable from the
   roots. The roots are the test codeunits being run and the install baseline
   (decision 5), including event subscribers and the triggers of tables touched.
   Unreachable code is never type-checked, so unimplemented builtins in it cost
   nothing. Reachable ones stay explicit refusals on the test that reaches them.
   The official AL compiler is all-or-nothing, so this is a deliberate
   difference. The owner allows it: "anything is allowed as long as the test
   results speak for themselves". The corpus pass list is that judge. A persistent on-disk cache of the parsed and indexed form of each
   dependency, keyed by the digest of its source entries plus the digest of our
   binary, is an optimisation. Add it only if the measured warm start needs it. It
   is not a prerequisite. Lowered IR is not cached in the first increment: other
   apps' extensions merge into a table's members, so the unit of caching would
   have to be redesigned first. Budgets: the corpus-only cold compile stays at
   1 s. With dependencies, the warm start (cache present, nothing downloaded)
   targets 1 s plus the cost of what the run reaches, reported as a metric. A
   cold start with download is reported, not budgeted.

7. **Platform-native layer: only where there is no AL.** The owner's rule: "if AL
   objects in the system app exist, we are not allowed to shim them. So anything
   that exists as AL will need to be taken as AL. Anything that is a service
   internal structure, like metadata tables for example, we have to create
   ourselves."

   | What | How |
   |---|---|
   | An object that has AL source with a body (any codeunit, table, page, report in System Application, Business Foundation, Base Application, the test libraries) | Compiled and run as AL. Never replaced natively. |
   | A platform table declared in the platform `System.app` (Date, Integer, AllObj, Session, Company, User, permissions, the metadata tables, ...) | The schema is the real declaration, taken as AL. The rows come from a native table provider, registered by table id. |
   | A platform codeunit or event publisher with no body | The runtime raises it natively. |
   | A `DotNet` type | Undecided. Refused until the owner decides. See the open question and `docs/dotnet-examples.md`. |

   The registry (the ADR-008 mechanism, a second id namespace) is the only way a
   platform table becomes usable. A platform table (any table from the platform
   `System.app`, or with an id of 2000000000 or above) with no registry entry is
   refused at compile time on the test that touches it, with a diagnostic naming
   table and id: `platform table 'Date' (2000000007) has no provider`. The refusal
   is reported through `Lowered::unsupported`, as unimplemented builtins are.
   Provider kinds, mirroring AL Runner's choices: generated rows (Date, Integer,
   Time Zone), rows derived from the compiled apps (AllObj, AllObjWithCaption and
   the metadata tables), single seeded rows (Company, User, Session, Active
   Session, Published Application, the SUPER Access Control row, with the values
   in decision 9) and stored platform tables (Record Link, the permission tables)
   that use the ordinary Store with their real declaration and need an explicit
   registry entry because their platform triggers are native.

   A test enforces the boundary: for every registry entry, no loaded app supplies
   an AL object with a body under the same id or name.

8. **`DotNet`: refused until decided.** Any use of a `DotNet` variable raises
   `Signal::Unsupported` naming the .NET type when it is first called. This is the
   safe default under hard rule 4. `docs/dotnet-examples.md` has the examples
   and the three options.

9. **Identity answers mirror AL Runner exactly.** They are host configuration
   (ADR-005, `al-host`) with these defaults:

   | Answer | Value | AL Runner source |
   |---|---|---|
   | `UserId()` | `TESTUSER` | `BcRuntime.cs:1433` |
   | `UserSecurityId()` | `{C0A1BDFA-0000-0000-0000-545553545553}` | `BcRuntime.cs:1441` |
   | `CompanyName()` | `My Company` | `BcRuntime.cs:1309` |
   | Company id | `{C0A1BDFA-0000-0000-0000-43524F4E5553}` | `BcRuntime.cs` (company seed) |
   | `SessionId()`, Session `Connection ID` | `1` (positive) | `RecordPatches.SessionVirtualTable.cs` |
   | Session / Active Session | one row, the running session | same file, `ActiveSessionSystemTable.cs` |
   | Culture | en-US (1033) | `BcRuntime.cs` (cultureSettings seed) |
   | Environment type | Production: `IsProduction()` true, `IsSandbox()` false, `IsSaaS()` false | `MetadataPatches.cs:781-835`, `CliText.cs:311-322` |
   | Service topology | on-premises, also after a test switches to SaaS | `docs/limitations.md#environment-type` |
   | Permissions | all granted; enforcement not modelled; another user refused | `docs/scope.md` section 2 |

   The `Environment Information` codeunit stays the real System Application
   source. Its `DotNet NavTenantSettingsHelper` is covered by the open question.

## Seams: what changes and what does not

Changes:

- **`al-host` (ADR-006).** The trait side only: a `DependencySource` capability
  (list the packages and entries of a build, read an entry with its CRC) and the
  identity configuration above. No OS access in `al-host`.
- **A new non-core crate `al-deps`** (std::fs, an HTTP client, a ZIP reader, the
  cache, and the version selection of decision 2) implements it; `al-test-cli`
  depends on it. Downloads and file access live here. The core crates
  (`al-syntax`, `al-sema`, `al-ir`, `al-builtins`, `al-vm`, `al-data`) keep doing
  none. The selection rules are pure functions over a list of candidate
  packages, so they are tested without the network.
- **`al-sema::workspace`:** `AppSource` gains `kind` (workspace or dependency)
  and `version`; loading takes an object index so files are parsed lazily; the
  platform `System.app` is an implicit dependency. Name resolution, visibility
  and extension merging are unchanged.
- **`al-sema::lower` / `check`:** an entry-point-driven compile (check and lower
  only what is reachable from given routines) beside the whole-workspace one.
  The corpus-only path keeps its behaviour.
- **`al-builtins` (ADR-008):** the registry gains a second id namespace for
  platform table providers.
- **`al-data` (ADR-003):** table providers (rows computed on read) need a new
  cursor source behind the Store trait. That is an amendment to ADR-003, not made
  here; it gets its own ADR before the first provider is written.
- **CLI:** `--deps DIR` (repeatable), `--bc-version PREFIX`, `--no-download`,
  `--cache-dir DIR`, `AL_RUNTIME_CACHE`. In corpus mode `al-test-cli` derives
  `--bc-version` from `oracle/oracle.toml`. `oracle/` is not edited.

Does not change: the IR and its serialisation (ADR-011), the value model
(ADR-010), `Ctx` (natives receive it as builtins do), the Session/Engine split
(ADR-001), snapshots (ADR-004), hard rules 1 to 6, the oracle and the ratchet
(ADR-007).

## First increment

Goal: prove the pipeline on the apps that cost least, with no row provider and
no `DotNet` shim yet.

1. `al-deps`: version selection (decision 2) as pure functions with tests; local
   path and download resolution of the `platform` artifact; `Source.zip` and
   `System.app` reading; cache and CRC check; `AppSource` `kind`/`version`; the
   CLI flags; corpus mode passes `bc_version` from `oracle.toml`; the selected
   build is printed and written to the junit. Tests use a tiny fixture zip written
   by the test (not Microsoft's) and a loopback range server.
2. Load System Application, Business Foundation, Permissions Mock, the System
   Application Test Library and the platform `System.app` for the corpus run,
   with the platform-table refusal (decision 7) in place so the run reports 0
   fail. Expected from the measurements: pass 1308 to about 1312, the tests that
   move past "outside the workspace" appear under their next blocker, and the 160
   silent failures are refusals instead.
3. Demand-driven compile and the warm-start metric, then Base Application.
   Expected: Base App business tables resolve for their 75 tests; no new passes
   are promised, because their next blockers are builtins.
4. The ADR-003 amendment for table providers, then providers in the order the
   survey gives: Date and Integer (33 tests), Company, User, Session and Active
   Session (31, with the identity values), AllObj and AllObjWithCaption (33), the
   metadata tables (about 170). The Published Application and Access Control
   providers come in this step too, because install code reads them.
5. The install baseline (decision 5), once the providers in step 4 that install
   code reads exist. Until then an app whose install code needs a missing
   provider refuses its dependent tests.

## First increment as built (2026-10-13)

Built, in the order of the list above:

- **`al-deps`** (a crate that may touch the OS). `version` and `select` are pure
  functions: Layer B as decision 2 describes it, with the owner's lookup order
  added. A candidate carries a *tier*: 0 is the project's `.alpackages`, then each
  `--deps` directory, then (later) the artifact cache. A package with source beats
  one without, then the nearest tier wins, then the highest version. An app built
  from source in this run (the internals fixture) beats every package. The platform
  and application floors are optional implicit roots. A Microsoft platform app below
  its minimum is used with a note. `zip` and `package` read the NAVX `.app`
  container: central directory, CRC-32 and size of every entry checked, nested
  wrapper `.app` unwrapped, `NavxManifest.xml` parsed. `load` scans the pool,
  resolves, and reads the chosen packages. There is no `DependencySource` trait in
  `al-host` yet: `al-test-cli` calls `al-deps` directly. The trait comes with the
  download.
- **Not built in the first increment** (Layer A and the download came after, see "Layer A as
  built"): loading the platform `System.app` (found and selected, then left out: its tables
  have no row providers, and a declared table without behaviour answers wrong), the install
  baseline.
- **Workspace.** `AppKind` (per app, in `Workspace::kinds`; `AppSource` is
  unchanged so existing constructors still compile). The files of a dependency
  app are *scanned* at load (`al_syntax::scan`: lexer plus preprocessor, objects,
  namespace, `using`s, `extends`, `implements`, event subscribers; a test compares
  it with the parser on every corpus file and every Microsoft file) and parsed
  the first time something needs them (`Workspace::green`). An object whose file
  does not parse cleanly is not resolvable (`usable`). `MemberTable` computes the
  members of an object, extensions merged, when first asked.
- **Demand-driven compile** (`al-sema/src/check/demand.rs`, decision 6). The checker
  checks all of the workspace's own code and follows what it uses in the dependency
  apps: calls, record variables and their writes, event subscribers, interface
  implementers, the pages a TestPage drives. A table is *read* (variable, `Get`,
  fields: lowered with the tables it relates to, nothing runs), or *written*
  (`Insert`, `Modify`, `Delete`, `Rename`, `Validate`): then the trigger and the
  subscribers of that operation are reached. An object only named (`Page::"X"`,
  `Database::"X"`) is lowered with its routines stubbed. After the check,
  `Workspace::restrict_to` makes everything not reached stop resolving, so what
  needs it anyway is refused as outside the workspace. A routine nothing reached
  is a stub whose reason says "not compiled". An object nothing reached is a shell
  of kind `uncompiled table` and so on, so lookups by id do not find it and run it
  as empty; the two lookups that would answer BC's "no such object" refuse
  instead. `AL_EAGER_DEPENDENCIES=1` compiles every dependency routine, for
  comparison: on the corpus it gives the same outcome for all 4836 tests.
- **Parser gaps found in Microsoft's source** (540 of 9572 files did not parse):
  attributes on variable declarations (`[SecurityFiltering(...)] Rec: Record X;`)
  now parse. 71 files remain: control add-ins, `trigger Name::Event()`, `event`.
- **Behaviour the new tests exposed**, each pinned by a corpus test and fixed or
  refused: `Format` of an option field answers its `OptionCaption`; a page's
  `SourceTableView` applies in filter group 2; the error text for a control of a
  page from a dependency app names an internal ID that is not understood, so it
  is refused (`docs/probes-needed.md`).

## Layer A as built (2026-10-14)

Built in `al-deps` (a crate that may touch the OS) and the `al-test-cli` flags. Core crates are
unchanged (ADR-006).

- **Build selection** (`build.rs`, pure). `BuildQuery` carries the prefix (`--bc-version`, else the
  major of the target that `al_version::choose_target` chose), whether it was explicit, and the
  app.json floor. `select_build` follows decision 2, Layer A: the newest cached match, else the newest
  published match; the default never selects below the floor; an explicit request below it
  warns; no match is an error naming the request, the builds available and the cache
  directory. `target::build_query` in `al-test-cli` is the one place that joins `choose_target`
  and the query, so the target and the build always agree on the major.
- **Ranged ZIP** (`remote.rs`). `RemoteZip` reads the last 64 KiB of the artifact (the end record),
  then the rest of the central directory if it is larger (1.1 MB for the platform artifact), then
  one request per wanted entry. Every entry is checked against the CRC-32 and size in the
  directory. The source is a `RangeSource`: `HttpSource` (ureq, retries on network errors and
  5xx; a server that answers 200 to a range request is refused, never downloaded whole) or
  `FileSource` (the unit tests' fixture file).
- **Cache** (`cache.rs`). `<root>/artifacts/<build>/platform.tail` (the saved central directory, so
  a warm run asks for nothing), `apps/<entry file name>` and `apps/<entry file name>.json` (build,
  entry path, URL, size, CRC-32). A file whose record is missing or no longer matches is removed and
  fetched again. Writes are rename-atomic, so parallel runs share the cache. Root, first match:
  `--cache-dir`, `ALRUNNER_CACHE` (ADR-016), `AL_RUNTIME_CACHE` (this ADR's first draft),
  `AL_RUNNER_CACHE_ROOT` plus `/v3`, `~/.cache/al-runtime`. This is the download cache that
  decision 1 and 3 require. It is not one of the persistent compile caches that ADR-016 defers.
- **Fallback** (`artifact.rs`, `load_with_artifacts`). Resolve with `.alpackages` and `--deps`
  (Layer B, unchanged). For each dependency nobody supplies and that Microsoft could supply:
  pick the build (cache first), read the central directory, find `<App name>.Source.zip` (or
  `Microsoft_<App name>.app`) under any folder, fetch it, accept it only if the `app.json` inside
  has the dependency's app id, save it, and resolve again. A second round picks up what the
  fetched apps need (System Application Test Library needs Any). With nothing missing no build
  is chosen and nothing is requested. A third-party dependency never reaches the feed.
- **Source archives are packages** (`package.rs`). A `*.Source.zip` has no `NavxManifest.xml`; its
  root `app.json` describes the whole app and every `.al` under any folder belongs to it. The
  module folders inside System Application Test Library carry their own `app.json`
  (`Azure AD Plan` and 55 others); they are not separate apps. So "Azure AD Plan Test Library"
  is a module of System Application Test Library, not a fourth app. `scan_dir` reads `*.zip`
  next to `*.app`.
- **Build variables.** Base Application's and Application's `app.json` hold
  `$(app_currentVersion)`, `$(app_minimumVersion)` and `$(app_platformVersion)`. They are filled from
  the build the archive came from: the build, `major.minor.0.0`, `major.0.0.0`. Evidence: the
  compiled Base Application 27.5.46862.48827 in the corpus `.alpackages` has exactly those
  values in `NavxManifest.xml`. The build comes from the record beside the cached file. A
  variable with no build is an error naming it, never a guess.
- **Where the test libraries are.** All three are in the **platform** artifact, not the country
  one: `Applications/System Application/Test/System Application Test Library.Source.zip`
  (278 KB, 169 `.al` files), `Applications/TestFramework/TestLibraries/permissions mock/Permissions
  Mock.Source.zip` (10 KB, 3 files) and, as a dependency of the first,
  `.../TestLibraries/Any/Any.Source.zip` (10 KB). About 300 KB of a 1.3 GB artifact.
- **CLI.** `--no-download`, `--cache-dir DIR`, `--bc-version` (existing: in corpus mode it picks
  the oracle leg, and now also the Microsoft build), `--fetch-deps` (resolve, download, stop),
  `--print-builds` (newest published build per version, from the index alone) and `--project DIR`
  (with `--fetch-deps`: any AL project; the target comes from `choose_target`). The feed is
  `ALRUNNER_ARTIFACT_URL` or the CDN of this ADR. Every run prints `bc build: ...`; the build is
  in `pass-rate.json` (`bc_build`), the Markdown summary and as a JUnit `<property>`.
- **CI.** The release job keys an `actions/cache` entry by the output of `--print-builds`,
  then runs `--fetch-deps` to warm it, and ends with the two `#[ignore]`d network tests.
- **Measured** (this machine, 2026-10-14, build 27.5.46862.56249): cold fetch of the three test
  libraries for the corpus 0.5 s (index 0.16 s, central directory 0.1 s, three entries 20 to
  30 ms each), warm 0.14 s with no request. A cold fetch of System Application, Business
  Foundation and Base Application for a project that has no `.alpackages` (40 MB) is 1.0 s,
  warm 0.08 s.
- **Effect on the corpus.** Eight tests move past "outside the workspace" (six on Permissions Mock,
  two on the Azure AD Plan test library). None passes yet: they stop at the platform table
  `Permission Set`, at `DotNet` `PermissionTestHelper` (refused by policy) and at a query over
  Plans. Pass counts are unchanged: 3903 for each of BC 27, 28 and 29.

## Rejected options

- **(b) Bundled hand-written AL shims for selected platform objects.** Rejected
  by the project owner: AL objects of dependent apps are not to be faked or
  hand-written. (A throwaway `Temp Blob` shim was used earlier to measure the
  idea; it made 3 tests pass and showed that the cost was in the next blockers,
  not the missing object. Dropped, together with the attribute design that went
  with it.)
- **(a) Native Rust implementations of dependent-app objects** such as `Temp
  Blob`. Rejected by the owner's answer 2 for any object that has AL source. Kept
  only for platform tables and codeunits that have none (decision 7).
- **(c) Symbols only (`SymbolReference.json`).** Gives the surface but no
  behaviour, and Base Application's is 59 MB of JSON; the source gives both. May
  still serve as a verifier of the parsed schema in a test.
- **(d) Keep refusing.** Leaves 516 tests dark, including core language tests
  that only use `Temp Blob` as a vehicle (`types/TestInterfaceVarCodeunitOut.al`).
- **(e) Pin the Microsoft build in the runtime to the oracle's build.** Rejected
  by the owner's answer 3: the runtime must work for any project.
- **(f) Whole-workspace compile of dependency apps with a persistent cache first.**
  Not chosen: the owner accepts demand-driven compilation and calls caching an
  optimisation (answer 4).

## Consequences

- The runtime executes Microsoft's AL, not an imitation. Every gap in our front
  end and VM that Base Application reaches becomes visible as an explicit
  refusal on the test that reaches it; the whole app produces 129 853 check
  diagnostics today, of which only the reachable fraction matters.
- A first run needs the network or a populated cache; CI needs an artifact cache
  step keyed by build.
- Corpus results depend on the BC build selected. The harness pins it to the
  oracle's minor, prints the full build, and records it in the junit. Same-minor
  build drift is possible until the corpus CI records its exact build.
- The install baseline and demand-driven compile interact. Install code and
  `Company-Initialize` are compile roots, so the cost of the baseline is a
  measured budget item.
- Platform behaviour is written once per table in Rust, with the real
  declaration as its schema, each with a corpus pin or a probe (hard rule 3).
- `DotNet` uses are refused until the owner decides. Tests that never run a
  DotNet line are unaffected.
- The warm-start budget for dependencies is new and must be measured, not
  assumed.

## Conformance

- `al-deps` tests: a fixture zip reads the same by range and whole; a CRC
  mismatch is an error; `--no-download` with a missing app fails naming it; the
  cache layout is stable.
- Version selection tests, named after AL Runner's resolver cases: the highest
  version at or above the minimum wins; an id match is authoritative and does not
  fall through to name; candidates all below the minimum give an error listing
  them; a missing app gives an error naming the directories searched; a cycle is
  an error; output is dependencies first; a Microsoft platform app below its
  minimum uses the selected build's package with a note; the default build is
  never below the app.json floor; an explicit `--bc-version` below it warns;
  a package with no AL source never shadows one that has it.
- Install baseline tests: dependency apps before the project app; per-company
  before per-database; upgrade triggers do not run; a failing trigger is reported
  with its name and not swallowed; a restored snapshot is independent of the live
  store; self-populating virtual tables are not snapshotted.
- `al-sema` tests: a platform table without a registry entry is refused with its
  name and id; a dependency app's tests are not discovered; an unreachable
  unsupported routine in a dependency does not block a test that does not reach it;
  a `DotNet` call raises `Signal::Unsupported` naming the type.
- Registry boundary test: no registry entry has an AL object with a body under the
  same id or name in any loaded app.
- `no_microsoft_source_in_repo`: no tracked file carries the Microsoft copyright
  header (the corpus submodule is excluded).
- `corpus_resolve.rs`: `expected-unresolved.txt` is regenerated with the
  dependencies present and then holds only what no artifact supplies.
- The corpus run (`metrics/passing.txt`) is the integration test.

## Side finding: namespace-qualified in-workspace names are mis-reported

Unrelated to this decision, and worth a separate fix. 5 of the 516 are not
external: a name qualified by a namespace the corpus itself declares
(`ALLanguage.Coverage.TxnLocal.Shipment Method` 3 tests,
`...RelationNameCollision.Shipping Agent` 1, `...PxqLocal.PXQ Table` 1) reaches
`lower.rs::resolve_obj` as one string and does not resolve. Separately,
`Workspace::resolve_ref` has a last tier that accepts a candidate in any
namespace, which the corpus relies on (no corpus file writes `using
System.Utilities;` yet uses `Codeunit "Temp Blob"`); dependency apps resolve
through the same path.

## Answered by the project owner (2026-10-13)

1. **Licensing.** "yes." Download Microsoft's app source into a local cache and a
   CI cache, compile it, commit nothing. (Decision 1, 3.)
2. **Native layer.** "if AL objects in the system app exist, we are not allowed to
   shim them. So anything that exists as AL will need to be taken as AL. Anything
   that is a service internal structure, like metadata tables for example, we have
   to create ourselves." Virtual and system tables with no AL counterpart get
   native row providers. `DotNet`: the owner needs more information and examples.
   (Decision 7, 8.)
3. **Pinning.** The runtime must not be corpus-centric. It should work for any
   project. The project's `app.json` dependencies (minimum versions) drive
   selection, using the same mechanism AL Runner uses today. (Decision 2.)
4. **Budget.** The official AL compiler is all-or-nothing. Lazy or demand-driven
   compilation is welcome, and "anything is allowed as long as the test results
   speak for themselves". (Decision 6.)
5. **Install triggers.** Mirror AL Runner. (Decision 5.)

## Acceptance and further answers (2026-10-13)

The project owner accepted this ADR and gave three further answers.

1. **Status.** The ADR is Accepted. Building on it is allowed.
2. **Where dependency apps are looked up first.** "The project's own
   `.alpackages` folder is the first place dependency apps are looked up, the way
   the AL compiler does it." The corpus ships System Application, Base
   Application, Business Foundation and Application 27.5.46862.48827, with full
   source, in `corpus/tests/al-language/.alpackages`. Downloading from the
   artifact feed (decision 1) comes later, as a fallback behind `.alpackages`
   and `--deps`. In the pool order of decision 2, Layer B, the project's
   `.alpackages` stays first.
3. **`DotNet`.** Still open. The owner asked for a generic approach, not
   per-class Rust stand-ins, because on-prem apps can declare any .NET class.
   A separate investigation is researching it. Until it is decided, decision 8
   applies: every `DotNet` use is refused, with `Signal::Unsupported` or a static
   reason that names the type.
4. **Hard rule 7 stands.** Microsoft source is never committed to this
   repository.

## Open question for the project owner

1. **`DotNet` types in Microsoft's AL.** The owner asked for a generic approach
   instead of per-class stand-ins (see above). The list below is the earlier
   framing. May the runtime contain Rust code that
   stands in for a .NET class declared in Microsoft's AL, and for which classes?
   The options, with real cases, are in `docs/dotnet-examples.md`:
   - No. Refuse every `DotNet` use. At most 20 to 28 corpus tests stay not run.
   - Yes, for standard-library classes only (`Convert`, `Encoding`,
     `MemoryStream`, `Array`, ...).
   - Yes, for both standard-library classes and Microsoft server classes
     (`NavTenantSettingsHelper`, `NavUserAccountHelper`, `ALConfigSettings`, ...),
     each backed by the identity configuration or a probe on real BC.

   Until the owner decides, decision 8 applies: refuse.

## Decided without owner review (2026-10-13/14)

The host-values work (branch `host`) needed these choices. The owner was away; each follows
AL Runner v2. Decision 9 above is unchanged except for the `SessionId()` row, which codeunit
60023 (`session/TestSessionIdentityValues.al`) corrected after the table was written.

1. **`SessionId()` and `ServiceInstanceId()` are 1, not 0.** Options: keep 0 as the table said;
   answer a positive id. The corpus asserts `> 0` for both and for the Active Session row, and
   AL Runner moved to positive ids for the same reason (`EnvironmentPatches.cs`,
   `SessionPatches.cs`, its changelog entry for 5144). The Session table row (`Connection ID`)
   follows `SessionId()` when the table exists.
2. **The clock is a capability, frozen by default** (amended by ADR-045: each read now moves it on by 1 ms). `al-host` has a `Clock` trait. The
   deterministic host (`Session::new`, all unit tests) answers 2024-01-15T12:00:00Z every time;
   the command line injects the system clock. AL Runner always uses the real clock. Options:
   real clock everywhere (matches AL Runner, makes library tests flaky near midnight); frozen
   everywhere (the corpus then runs on a date that is not today); frozen in tests and real on
   the command line (chosen). `Sleep` does not wait; it advances the session's view of the
   clock, which is all AL can observe. System stamps are strictly increasing within a session,
   because the corpus compares them with `>` after writes that land in one millisecond.
3. **The session time zone is UTC.** AL Runner seeds the host's local zone, which makes its
   results depend on the machine. The corpus pins no zone, and UTC was already the model of
   `DT2Date`, `CreateDateTime` and `Format`. `SessionSettings.Init` therefore answers `UTC`.
4. **`WorkDate` is session state, today until set.** A new test codeunit gets a new session and
   so starts at today again, which is what the Test Runner's reset after each codeunit gives.
5. **`Session.CurrentClientType()` is `Background` (AL Runner) and `DefaultClientType()` is
   `Web`.** AL Runner does not set the second. Nothing in the corpus reads either value.
   Superseded 2026-10-19 by the corpus pins (CLAUDE.md rule 3): the oracle's tests run in a web client session,
   so `CurrentClientType()` is `Web` and `DefaultClientType()` is `Windows` (session/W5SessionFacts.al:135-138), and
   the Session row's "Login Type" is "Username and Password" (session/W6SessionTimeZone.al:22-23).
6. **`CompanyName()` is `My Company`** (decision 9), replacing the placeholder `CRONUS`.
   There is still no Company row: the Company table is outside the workspace.
7. **Temporary records are not stamped** with `SystemCreatedAt/By` and `SystemModifiedAt/By`.
   Evidence is AL Runner's reading of the platform: the stamps come from the SQL layer, which a
   temporary record never reaches. No corpus test reads one. Probe wanted (docs/probes-needed.md).
8. **The dependency golden test is `#[ignore]`d in a debug build and CI runs it in the release
   step.** `al-test-cli/tests/corpus_resolve_deps.rs` loads Base Application and checks it, which
   takes about 45 s unoptimised. Options: keep it in `cargo test` (slow gate for every agent);
   ignore it and run it with the perf budgets in release (chosen).
9. **The test-library apps come from the artifact feed through the download fallback (Layer A).**
   The corpus names System Application Test Library, Permissions Mock and (through them)
   Azure AD Plan Test Library; no `.alpackages` holds them. Until Layer A exists a developer can
   put them in a local `--deps` folder; CI keeps refusing the tests that need them. None of them
   is committed (hard rule 7).

## Install baseline and Global Triggers as built (branch `baseline`, 2026-10-14)

**Global Triggers.** The platform codeunit `Global Triggers` (2000000002, in the platform `System.app`) has only
body-less business events. The runtime raises them itself (`al-vm/src/globaltriggers.rs`), whether or not
`System.app` is loaded; its subscribers are registered under the codeunit when a loaded app declares it, else under
a pseudo object key. The order and the cases are in `docs/probes-needed.md`. In the demand-driven check
(`check/demand.rs`) a write to any table, of any app, reaches the subscribers in dependency apps: the setup events
always, the database events of the operations used (Base Application's codeunit 49 `GlobalTriggerManagement` among
them). Base Application's subscribers reach platform tables that have no provider (`API Webhook Subscription`,
`User`), so five of them cannot run yet. `al-test-cli` tells the engine which (`global_triggers_find_unavailable`) and
`dispatch.rs` refuses, with `Signal::Unsupported` naming the subscriber and what it lacks, to raise an event one of them
would receive. The two setup events are the one exception: while no table the subscriber's decision reads (the record
variables of the routines it calls within two levels: change log setup, Dataverse and CDS connection setup, API webhook
subscriptions) holds a row, nothing is configured and the subscriber would have answered "no" for every table, so it is
skipped; with a row in one of them the setup event is refused too. Without that exception every write in a run with
Base Application would be refused (measured: 1724 of 4836 pass). With it, a test is refused only when a database event is
enabled for a table (by a workspace subscriber, as `ALT Triggered` does) and so reaches `GlobalTriggerManagement`
`OnDatabaseInsert` and its siblings: all 73 tests that wait on Global Triggers events, because every one of them uses
`ALT Triggered`. What unblocks them: the platform table `API Webhook Subscription` (and `Database::User`, which
`Change Log Management.IsAlwaysLoggedTable` names) from the `platform-tables` work.

**Install baseline** (`al-test-cli/src/baseline.rs`, `al-vm/src/baseline.rs`). Roots of the check: the install
codeunits of every dependency app (`Subtype = Install`, found by the scan) and Base Application's `Company-Initialize`.
`AL_NO_BASELINE=1` leaves them out (to measure). The run, once per process, on a fresh session: per dependency app, in
dependency order, every `OnInstallAppPerCompany`, then every `OnInstallAppPerDatabase` with `CompanyName()` empty (a
per-company table is refused there, a `DataPerCompany = false` table works); then `Company-Initialize`; then
`Session::capture_baseline` (rows, system id, row version and AutoIncrement counters, isolated storage, number
sequences). Every test codeunit's session starts with `restore_baseline` (the persistent store makes both O(1));
the project's own install codeunits still run per test codeunit, as before. A root that stops (Unsupported or an AL
error) is reported on stderr: `Company-Initialize` keeps the rows it wrote before, an install trigger is rolled back.
The tables such a root writes rows to (a record variable's `Insert`, `Modify`, `Rename` or `ModifyAll` in its static
closure; rows that event subscribers add and `RecordRef` writes are not found), minus the tables that hold a row in the
baseline, are `Engine::unseeded`. For them every read whose answer depends on the rows is refused, from dependency code
and from the corpus's own: `Find*`, `Next`, `Count`, `IsEmpty`, `CalcSums`, a FlowField over them, the miss of a
`TableRelation` check, a `Get` that finds nothing. A `Get` that finds the row is answered, and so is a temporary record.
Rows the session wrote itself are not an exception (BC may hold more). For every other table a read is BC's real answer.
The project's own install codeunits follow the measured order too (all per-company, then per-database with no company). Not built: seeding the platform rows (Published Application, User, Company, Access Control:
they wait for the providers), the second snapshot after the project's own installs, a persistent copy.

What stops it today on the corpus (Base Application 27.5): `Company-Initialize` stops in its `OnBeforeOnRun`
subscriber (the retention policy install, which evaluates an empty `DateFormula`: "date formula ``: this form is not
pinned by the corpus"), so no setup row is created yet; 3 of the 16 roots run to their end. The others wait for
platform tables (`Company`, `Field`, `Table Metadata`, `AllObjWithCaption`, ...), the `Copilot Capability` enum,
`Session.ApplicationIdentifier`, `System.GetUrl`, `TaskScheduler.CanCreateTask`. The mechanism is tested with
synthetic dependency apps (`al-test-cli/src/runtime.rs` tests).

### The baseline runs only when a selected test can observe it (2026-10-14, branch `fast-pure`, issue 20)

Owner request (issue 20): a test that touches nothing from Base Application should cost what it costs in an app
with no dependencies. The install baseline was the largest part of that cost: 3.5 of 4.5 s without the compile
cache (it adds 421 Base Application routines to the demand-driven compile and runs 17 roots), 0.25 s with it.

**The rule** (`crates/al-testrun/src/observe.rs`, `may_observe_baseline`). A static whitelist over the lowered IR. The
reach starts at every routine of every selected test codeunit (handlers are routines of the codeunit) and follows
direct calls and the targets of interface calls. The baseline is skipped only when everything reached is plain
computation:

- Objects: a codeunit only. Every slot (global, local, parameter, return value) has a type that holds no database
  state. A `Record`, `RecordRef`, `Query`, page, report, `DotNet` or other handle type makes the answer "may observe".
- Statements and expressions: assignment, control flow, arithmetic, conversions, calls into the reach, interface
  calls, `asserterror`, and builtins from a whitelist: the receivers `Text`, `Label`, `TextConst`, `List`,
  `Dictionary`, the JSON and XML types, `TextBuilder`, `BigText`, `Date`, `Time`, `DateTime`, `Enum`, `Version`,
  `ErrorInfo`, `Variant`, `Dialog`, and from `System` only the functions that use their arguments, the host clock or
  the session's own settings (`Format`, `Evaluate`, `Round`, `Today`, `GetLastErrorText`, ...). A record field place,
  the base object of an extension, an enum's interface implementation, a test page control, a query column and any
  `DotNet` expression are not in the list.
- Events: a call to an event publisher (`IntegrationEvent`, `BusinessEvent`, `InternalEvent`, `ExternalBusinessEvent`)
  says "may observe", because its subscribers could do anything. A custom test runner codeunit does too.
- Anything not named says "may observe". An unknown builtin, a new IR node or a new type runs the baseline.

Why this is enough: `al_vm::Baseline` holds the rows, the counters for system ids, row versions, AutoIncrement and
write stamps, the GUID counter, isolated storage and number sequences, and nothing else (execution state starts fresh
in every session). The only other place the baseline shows is `Engine::unseeded`, which a `Get` or a read of a table
consults. Each of those is reached through a record or table, `CreateGuid` (`System` and `Guid`, both left out of the
whitelist), `IsolatedStorage` or `NumberSequence`, or through an event whose subscribers do one of those.

**Where it runs.** First a text check of the selected test codeunits (`text_looks_pure`: their source holds none of
the words `record`, `query`, `page`, `report`, `xmlport`, `isolatedstorage`, `numbersequence`, `createguid`,
`database`, `dotnet`, `session`, `codeunit.run`). If it passes, the apps are compiled demand-driven, without the
baseline's roots and without the compile cache (ADR-022): that reaches only what the tests reach, a few routines.
The rule then runs on the lowered program. If it says "no test can observe the baseline", that compile is the run: no
baseline, no cache entry read or built (a cold cache costs nothing), and no .NET helper is started ahead of time. If
it says "may observe", the loaded apps go on to the compile they would have had (cache or demand-driven, with the
baseline's roots); nothing is read from disk again. So the cost of a wrong guess is one extra compile of what the
tests reach, never an outcome. A selection that fails the text check (all of the corpus, Microsoft's test buckets)
compiles as before, and the rule runs on that compile when it is finished (with the cache every dependency routine is
compiled, so the walk can enter any of them), which costs nothing when the first reached routine already says "may
observe". `--require-compile-cache` never takes the first path (CI wants its measured run to be a hit).
`--timings` says which way it went (`install baseline: not run, no selected test can observe it`, or
`install baseline: ran, a selected test may observe it: <routine and construct>`).

**Switches.** `AL_FORCE_BASELINE=1` always runs the baseline (the old behavior). `AL_NO_BASELINE=1` never does (a
measurement switch, as before). `AL_CHECK_BASELINE_RULE=1` runs the baseline as usual but starts each test codeunit
that the rule calls free of it on an empty store (`=list` also prints, for the other codeunits, the first construct
that stops the rule). `tools/baseline_rule_check.sh run` makes a normal corpus run and a checked run for BC 27, 28,
29 and the suites and compares the JUnit files with no allow list. On 2026-10-14 the rule covered 25 of 702 test
codeunits of the corpus (the others declare records, pages and queries) and the outcomes of all 5279 tests per
version, and the OnPrem and isolation suites, were identical. The check must stay at zero differences.

### Decided without owner review (2026-10-13/14), baseline

1. **Install order.** Options: AL Runner's and this ADR's text (per codeunit: per-company, then per-database, with a
   company); the order the probe agent measured on real BC 27.5 (a fresh install with two companies: every
   `OnInstallAppPerCompany` for each company first, `OnInstallAppPerDatabase` last, with `CompanyName()` empty).
   Chosen: the measurement, per app. It differs from AL Runner, which runs per codeunit with a company set. One
   company here, so "for each company" is one run.
2. **A root that stops.** `Company-Initialize` keeps its earlier rows (AL Runner's partial company initialization);
   an install trigger is rolled back (the existing handling of the project's own installers). Options: keep both,
   roll back both. Chosen as stated; the rows before a stop are exactly a prefix of BC's own.
3. **Which reads of install-seeded tables are refused.** Options: only a missing `Get` in dependency code (the old
   rule); none; every read whose answer depends on the rows, anywhere, for tables a failed root writes and that hold
   no row. Chosen: the last (reviewer's instruction). The 3 baseline cases it turned into "not run" read an empty
   Cost Accounting Setup and passed by accident; they were removed from the baselines in a
   dedicated commit (hard rule 2).
4. **Base Application's `Global Triggers` subscribers that cannot run.** Options: refuse every write in a run with
   Base Application (loses about 2300 passes); skip them silently (main's behaviour; hard rule 4); refuse the tests
   that touch the tables they read (hundreds, it removed 43 passing tests); refuse when an event reaches one,
   with the setup events skipped while the configuration tables they read are empty (chosen, after the reviewer's
   instruction to refuse dynamically). Cost: the 73 tests behind `ALT Triggered` stay not run until the platform
   tables exist.
5. **`Session.LogMessage` is a no-op** and `TelemetryScope` has the members `ExtensionPublisher` and `All`
   (Microsoft docs; ordinals not pinned). Telemetry is not observable from AL.
6. **`DeleteAll(false)` and the database events.** Per row, with the table events and the database event, following
   the probe 68212 on real BC 27.5 (corpus citation pending). This also lifts the old refusal of `DeleteAll(false)` on a
   table with delete event subscribers.

   The corpus names System Application Test Library and Permissions Mock; no `.alpackages` holds
   them. Layer A is built (see "Layer A as built"); a developer can still put them in a local
   `--deps` folder. None of them is committed (hard rule 7).

### Layer A (2026-10-14)

10. **`--no-download` skips and prints; a feed that does not answer is an error.** Decision 1 says
    `--no-download` makes a missing app an error. In corpus mode that would fail every offline
    run of the harness, while the corpus can still be measured without the test libraries (their
    tests are then reported not run, hard rule 4). Options: error as written; skip with a
    printed line (chosen for the corpus harness; the message names the app, the build or prefix and the
    feed URL, and the library keeps `MissingPolicy::Error` for a project mode); skip
    silently (rejected). When downloading is on and the feed does not answer, the run exits 2
    with the URL, so a CI run that expected the libraries can never lose them quietly. The
    ratchet would also catch it.
11. **A cached build wins over a newer published one; the project's `.alpackages` still wins over
    both.** The first is AL Runner's rule and decision 2, Layer A. The second is the owner's
    answer 2 of 2026-10-13. A consequence: the corpus's `.alpackages` has System Application
    27.5.46862.48827 but the test libraries come from the newest 27.5 build (56249), because
    48827 is no longer published (HTTP 404; the index still lists it). Same-minor skew, as the ADR
    already accepts. The build is printed and recorded.
12. **An exact build the feed has dropped is an error, not a walk down to an older build.** The
    index lists builds whose artifacts were removed. `--bc-version 27.5.46862.48827` therefore
    fails with the URL. A prefix always picks the newest, which exists. Options: walk down to the
    newest build that answers (hides the skew); error (chosen).
13. **The cache keeps each entry as Microsoft ships it (inflated out of the artifact's ZIP), with a
    record beside it, not the deflate stream.** Decision 1 says "bytes as downloaded". The
    downloaded bytes are a deflate stream of a ZIP that is itself a ZIP; keeping the inner ZIP
    lets the cached file be read like any `--deps` file. Its CRC-32 and size are in the record
    and are checked again on every run.
14. **The artifact is chosen only when something is missing.** With every dependency on disk no
    build is chosen, no request is made and the run prints `bc build: none needed`. AL Runner always
    selects a build because it needs the DLLs. We need only the apps. Option: always resolve a
    build (a network call per run, for nothing).
15. **Names: `ALRUNNER_CACHE`, `ALRUNNER_ARTIFACT_URL`, `--cache-dir`.** ADR-016 (Proposed) renames the
    environment variables. `AL_RUNTIME_CACHE` and AL Runner v2's `AL_RUNNER_CACHE_ROOT` (plus a `v3`
    subfolder) are read as fallbacks. If ADR-016 is rejected the names are two lines in `cache.rs`.
16. **`--project DIR --fetch-deps` is the only project mode so far.** The harness is corpus-only; running a
    project's tests belongs to ADR-016. The flag exercises `choose_target`, Layer A and the
    download for any app.json, and it is how the floor-derived default is checked end to end.
17. **Two network tests, not one.** `al-deps/tests/network.rs` checks the CDN protocol and the real
    archives; `al-test-cli/tests/network_deps.rs` checks the corpus flow, cold and warm. Both
    are `#[ignore]`d and take under a second; CI runs them.
7. **Table extension field triggers.** `modify(Field) { trigger OnBeforeValidate/OnAfterValidate }` run around the field's
   own OnValidate (corpus order); they are needed by the 31 Global Triggers tests that were next in line, and the old refusal
   is gone. Dependency extensions' such triggers are reached by the demand-driven check when the field is validated.

## Demand-driven compile: audit of what BC runs implicitly (2026-10-14)

The owner's concern: a partial compile must never skip business logic silently. Whatever BC
runs without being called by name is either compiled and run, or the test is refused (hard
rule 4). Two changes make that hold, and 77 small probe tests pin it
(`crates/al-test-cli/tests/demand_audit.rs`; each runs a dependency app and a workspace test, in
the demand-driven compile and in the eager one, and fails on a silent skip).

**The guard (fail closed at dispatch).** `ir::Program::uncompiled_subscribers` lists every
event subscriber of a dependency object that was not compiled, for events of objects that are
(and for `Global Triggers` and `Caption Class`, which no app declares). Manual subscribers are
left out: they fire only on a bound instance, and that is a compiled codeunit. The engine's
event registry keeps them as `missing`. Raising an event, and asking whether anyone listens
(`has_listeners`, page events, `Truncate`), refuses when a missing subscriber would receive it
(by `ElementName` for field events). The list comes from the cheap scan index, so it covers
every object, compiled or not. Page events the client does not raise (`OnAfterGetRecordEvent`,
page field validate, action events) refuse when the page opens, whether the subscriber is
compiled or not.

**Reaching more, so fewer tests are refused.** The checker now reaches: the subscribers of the
page events the test client raises (`ir::RAISED_PAGE_EVENTS`) and the `OnResolveCaptionClass`
subscribers when a page or report that declares a `CaptionClass` is used; the table of a
workspace page (all write operations); `RunObject` codeunits and pages of workspace pages; a
write without `Rec.` in table, page or report code; `RecordRef` write methods (their
`Global Triggers` events); the subscribers of events a table extension publishes; a table
extension's `OnBefore*` and `OnAfter*` triggers; a table field's `OnLookup` for a page over the
table; a report or XML port that a variable runs or `Report::"X"` names in `Report.Run`.

**Fixed on the way.** A subscriber that names its publisher with a namespace
(`Codeunit::Microsoft.Foundation.Company."Company-Initialize"`, 7 in Base Application and System
Application) attached to nothing, in both compiles. The caption class resolvers attached to the
compiled `Caption Class` codeunit and not to the key the page client raises (a difference between
the demand-driven and the eager compile, `autoformat/TestCaptionClassResolverFailure.al`).
`Page.RunModal(0, Rec)` answered "no page has id 0" for a lookup page that was not compiled;
the report and background task lookups by number name the real reason.

**What stays refused, by design.** A write through a `RecordRef` or `FieldRef` on a table the code
names only by number or by `Database::` runs the trigger and subscribers of that table as stubs,
and refuses when one fires. So do `Codeunit.Run`, `Page.Run`, `Report.Run` and
`RecordRef.Open` with a number nothing names, a background task with a bare codeunit id, and
the `RunObject` actions of dependency pages. Reaching them all would compile most of Base
Application. `TaskScheduler.CreateTask` and `StartSession` are refused by the builtins.
Not modelled, and not a demand-driven matter: the events the platform raises at session start
and company open; their effects are the state the install baseline stands for.

**The permanent check.** `tools/compare_junit.py` compares two JUnit files test by test. CI runs
the corpus a second time with `AL_EAGER_DEPENDENCIES=1` for every BC version and fails on any
difference in outcome (about twice the cost of the first run, 8 s against 15 s of CPU for one
version, so it runs on every push). `Roots::eager` does the same inside a unit test.

**Amendment (owner, 2026-10-31): runs on main and nightly only.** The eager comparison and the install baseline rule check (`tools/baseline_rule_check.sh`, `AL_CHECK_BASELINE_RULE=1`) run in CI on a push to main and on the nightly schedule, not on a pull request. Neither had failed on a pull request since 2026-10-14 apart from one catch of the eager comparison (2026-10-23), and together they cost about 3 runner minutes per run. The normal corpus run with its ratchet and work ratchet stays on every pull request, so a case already in `passing.txt` is still caught there. A merge train runs as a pull request, so its comparisons run on main after the train merges. A difference fails the CI run of main, and the regen bot and the nightly edge advance only from green main commits.

## "Outside the workspace" on Microsoft's test buckets: root causes (branch `bucket-demand`, 2026-10-14)

Over the 32 BaseApp test buckets (40300 tests), 2939 tests were refused as "outside the workspace": a global
table 1968, a global codeunit 500, a table 330, a page 71, a report 32. The demand-driven compile was not the
cause. Every one of those objects exists in the apps and was reached or reachable. They were unusable because of
four defects, none of them in the reach analysis:

1. **A dependency file did not parse** (about 95 % of the refusals). `Workspace::usable` is false for every
   object of a file that does not parse cleanly, and name lookup then skips it, so the message says "outside the
   workspace". Found by running the buckets with `AL_EAGER_DEPENDENCIES=1`, which parses every file, and listing
   `Workspace::lazy_parse_problems`. Fixed in `al-syntax`:
   - `procedure P(): Option` with no members, followed by `var` or `begin`: the `var` was taken as an option
     member. Base Application `FALedgerEntry.Table.al`, `FAJournalLine.Table.al`, `CRMFullSynchReviewLine.Table.al`,
     `CalcConsumption.Report.al`, `AzureAIUsageImpl.Codeunit.al` and others. `FA Ledger Entry` alone was 1968.
   - `trigger Variable::Event(...)` (a handler of a `WithEvents` variable) and `ControlAddIn Name` as a parameter
     type (`Business Chart Buffer`, 330 tests, and every codeunit that has an `UpdateChart(...)`-like procedure).
   - `event Name(...)` in a `controladdin` object, with or without the closing `;` (Microsoft leaves it off).
2. **A namespace-qualified table in a section argument** (`dataitem(Integer; System.Utilities.Integer)`).
   `Section::args()` returns source text and `qualified_parts` splits on a separator that text does not carry,
   so the name never matched. `Section::arg_name_parts` and `types::section_object_name` read the argument as a name;
   report, query and XML port data items, `part` pages and `TestPart`/`TestFilter` types use them.
3. **`outside_workspace` looked a name up after cutting it at its last dot.** A table named
   `Metadata Permission Set Rel.` (a platform table with no provider) was reported as outside the workspace and
   not as "has no provider". Only the message was wrong; the refusal was right.
4. Pages generated by `pages.rs` for `Tests-TestLibraries` (`procedure __source_N(): Variant begin exit(No.); end;`)
   do not parse when a source expression names a field with a space or a dot. Not fixed here: it is the quote
   bug in `pages.rs` that belongs to the lowering work. Until then `alrunner` prints a `parse:` line for it.

**Not a cause, checked.** The Tests-TestLibraries app and the test framework apps load completely (11 apps, all
files). Object lookup by name and namespace finds Fixed Assets objects in Base Application. Declared types of a
reached object are touched by `object_ctx`. A SCHEMA object's routines are stubs and carry the object's global
reasons first, so a stub of an object with an unparsable global type showed the global reason and not "not
compiled"; that stays correct (both refuse).

**Parse problems are now visible.** `alrunner` and `al-test-cli` print `parse: N dependency file(s) did not
parse ...` and the first ten messages after the compile. The list covers the files the demand reached; with
`AL_EAGER_DEPENDENCIES=1` it covers every file.

**Compile panics.** A panic while checking or lowering one routine used to end the run with exit 2. Checking a
routine (or an activation task of an object) and lowering a routine are now wrapped in `catch_unwind`.
`CheckResult::panicked` records (object, routine offset) and the message; lowering gives that routine, or every
routine of the object for a panic outside a routine, a body-less stub with the reason
"internal error while checking: ..." or "internal error while lowering: ...". The tests that reach it are not run;
the rest are. Panics outside routines (object layouts, page expansion, the loader) still end the run.

## Zero differences between the compiles (owner decision 2026-10-14, branch `zero-diff`)

The demand-driven compile (uncached), the eager compile (`AL_EAGER_DEPENDENCIES=1`) and the cached compile (ADR-022) give
exactly the same outcome for every test, on the corpus (BC 27, 28, 29 and the suites) and on the 32 Microsoft
buckets. `tools/demand-eager-known-differences.txt` is empty, and CI fails on any difference.

The rule behind it: **what a static refusal covers depends on the sources of the installed apps, never on what was
compiled and never on which other tests are selected.** Concretely:

- The call graph of a page, report or xmlport variable (`lower.rs`, `driven_edges`) is built from routine roles, not from
  the routines that happen to be compiled. A `TestPage` variable adds its page, the page's extensions and parts; a
  report or xmlport variable (also a `TestRequestPage`) adds its triggers and the routines made for its properties.
  `RunObject` targets of workspace pages are followed; those of a dependency page are compiled (the test can invoke
  the action) but add no edge. A `Page` variable of a dependency page adds no edge, and a platform method called on it
  reaches the page. `CurrPage` adds nothing. The demand-driven compile reaches exactly what these edges name.
- A platform table that lists objects (`Field`, `Key`, `Table Relations Metadata`, `Report Metadata`, `Page Metadata`,
  `Page Action`, `Event Subscription`, ...) compiles the shape of every object it lists when reached code declares it.
- A write through a `RecordRef` or `FieldRef` runs the triggers and subscribers of every table the compile holds.
- The types of a reached routine's parameters and return value are used by it (an interface it returns is compiled).
- `Report.Run(Report::"X")` in dependency code resolves quoted and namespace-qualified names.
- The reportextensions that change a run are counted from the sources of the reports that are compiled, active or not.

Cost, measured on the corpus (BC 29, 4 threads, a loaded machine): compile 3.1 s to 3.8 s (+0.6 s) for the demand-driven
compile; Tests-ERM cold compile 4.2 s to 4.7 s, cached compile unchanged (2.1 to 2.3 s).

Not decided here: one timing flake. A full run of Tests-ERM on a machine under heavy CPU load gives 752 to 756 passed,
and the extra refusals read "IsEmpty on Workflow: BC holds rows here ..." (Price List Line UT). The compile is the
same in every run (`AL_DUMP_REACH` files are identical), the thread counts do not matter, and it does not happen on
a quiet machine. Cause not found.

### A pure test beside data code, and Library Assert (2026-10-15, branch `filter-speed`, issue 39)

Measured on a demo app with a test app on Base Application and Library Assert, `--filter` on one pure test: compile 4.2
to 9 s and 2.1 GB, against 0.3 s and 684 MB for the whole suite, and the install baseline ran. Three causes, three changes
(outcomes unchanged: baseline 27/28/29, demand/eager, cache/uncached, baseline rule: 0 differences):

1. **The speculative compile reached the workspace's data code.** The pure-first compile (above) checks every workspace
   routine, so a codeunit of the app under test with a `Record "Sales Line"` procedure pulled 94,000 dependency routines in,
   and the result was thrown away when the rule said "may observe". New `Roots::reach_only` (used for that compile only):
   a procedure of a workspace *normal codeunit* (no Subtype) that no selected test names, directly or through the
   procedures it names, is not checked and compiles to a stub that refuses. Names are matched by spelling (a superset of
   what can be called). Triggers, event subscribers, publishers, objects that are not normal codeunits and codeunits with
   a Subtype stay roots. `Workspace::set_unreached`, `unreached_routines` in `check.rs`.
2. **The rule (`observe.rs`) was too coarse for a record variable.** A local record variable that is only used through
   its fields (`R.Field := x`, `Format(R.Field)`) is a buffer in memory: no row, counter or storage is touched. It no
   longer makes a test "may observe". Any other use (a method, passing it on, assigning it whole, returning it) still does.
   Library Assert's `TypeNameOf` is the case.
3. **A catalog table used as a buffer compiled every table.** Using the platform table `Field` (and `Key`, ...) as a table
   answers for every table of the installed apps, so all their shapes were compiled (93,000 routines). A record variable of
   such a table that is named in the routine only as `Name.<field>` no longer does that (`field_only_records` in `check.rs`).

Result on the demo app, one pure test, load average about 19: compile 4.2 s -> 0.17 s, total 4.6 s -> 0.22 to 0.36 s,
peak memory 2.1 GB -> 135 MB; the baseline is skipped (0.30 s saved).

### A selection beside the corpus's own objects: the lean compile (2026-10-19, branch `selection-run-cost`)

`al-test-cli --root . --codeunit 74712` with the Microsoft apps cost more than the full corpus run (177 G against 168.5 G
instructions). Two compiles each reached 98,000 of Base Application's 155,000 routines:

1. **The pure-first compile still had the whole workspace as roots.** `reach_only` leaves only procedures of normal
   codeunits out. The corpus's install codeunits, listeners, table and page extensions, pages and tables stay roots, and
   each of these groups alone reaches about 98,000 Base Application routines through the demand.
2. **The rule then said "may observe"**, because the Probe Observer calls `CompanyName()`, so the whole compile was done
   again with the baseline's roots. Company-Initialize alone reaches about 98,000 routines, too. That is the fixed part
   of every run that runs the baseline (an app with only the test and its fixtures costs 90.6 G with the baseline and
   6.8 G without).

Changes:

1. **`Roots::lean`** (`check/mod.rs`, `lean_left_out`), used only for the pure-first compile of a selection that
   `try_run_suites_with` runs: a workspace object is compiled only when a selected test codeunit names it, directly or
   through the objects it names. A name is an identifier or quoted identifier equal to a workspace object's name, or an
   integer literal equal to a workspace object's id, anywhere in a named object's text. The extensions of a named object,
   and the object a named extension extends, count as named. The routines of the other workspace objects refuse with
   `check::LEFT_OUT`; their fields, values and properties are lowered as before (`members` holds them).
2. **A lean run is checked.** The lean compile only runs tests the rule calls pure: their static reach has no record,
   page, event publisher or run by id, so only what they name runs. If a test still meets a routine the lean compile
   left out (a refusal that carries `LEFT_OUT`, before or while it runs, also in the test runner's codeunits), the apps are
   compiled completely and every selected test runs again; the second run's outcomes count (`CorpusRuntime::met_left_out`,
   `Compiled::into_apps`). A workspace codeunit with one of the ids the test runner runs (130301, ...) is such a case: no
   test names it.
3. **An install codeunit the lean compile left out does not run** in the test's session (`run_install_codeunits`). The
   tests are pure, so they cannot read what it writes; and BC runs install code in the session that publishes the app, so
   nothing else of it reaches a test's session. The complete run gets the same from the baseline, which runs it once in
   a session of its own. Before this change a pure selection ran the workspace's install code in each test's session, so a
   test that reads a single-instance codeunit the install code changed saw another value than in the complete run
   (`al-testrun/tests/lean_selection.rs`).
4. **`Database.CompanyName` is pure for the rule** (`observe.rs`, `PURE_DATABASE`): it answers the company the host gave
   the session, never a row (al-vm `sessionops.rs`). `tools/baseline_rule_check.sh run`: 0 differences; 440 of 2545
   test codeunits (BC 29) run without the baseline in that check.
5. **The lean compile is tried only when what the tests name looks free of data** (`observe::lean_closure_looks_pure`):
   a name such as `Record`, `Page` or `Session` in the code of a named workspace object means the demand reaches the
   dependency apps and the rule says "may observe" after it, so the apps go straight to the compile with the baseline
   (from the same load when there is no compile cache). `Phases::compiles` counts the compiles; `--timings` says when
   there were two.

Results (BC 29, Microsoft apps, `perf stat` instructions, load about 5):

| selection | before | after |
|---|---|---|
| `--codeunit 74712` (pure) | 177.5 G, 10.2 s | 17.7 G, 1.3 s |
| `--codeunit 73032` (pure) | 177.3 G | 17.7 G |
| `--codeunit 60079` (names a helper that deletes rows) | 177.4 G, 10.7 s, two compiles | 91.8 G, 5.6 s, one compile |
| `--codeunit 60198` (records, needs the baseline) | 91.5 G | 91.6 G |
| full run | 163.3 G | 163.3 G |

The 385 corpus codeunits the rule calls pure and whose own text looks pure, each run alone: 372 lean, none ran again,
1,692 cases, 0 outcomes that differ from the full run. Full runs BC 29 and 27 (with the suites): 0 outcome differences.

Not changed: a selection that needs the baseline still compiles about 98,000 Base Application routines, because
Company-Initialize reaches them by activation (for example the table Price List Header raises the enum Price Source
Type, whose implementations run `Page.RunModal` on lookup pages, whose actions run the posting codeunits). That is
about 90 G, half of a full run.

## Amendment 2026-10-15 (owner decision, branch `source-lookup`): a package without source is not found

**Decision (owner).** "A package that is found but can't serve the source is not a package that is found by definition.
We can only count a package as found if it actually contains the source. Otherwise the runner has to continue looking for
the source code."

The case: a project's `.alpackages` holds the symbol packages VS Code downloaded (the manifest and `SymbolReference.json`,
no AL source). They used to satisfy the dependency: the search stopped, the artifact feed was not asked (when nothing else
was missing), and every Microsoft object was "outside the workspace", the install baseline stopped at Upgrade Tag and no
test data loaded.

1. **A candidate without AL source does not satisfy a dependency** (`select.rs`, `Resolver::satisfy`). The search goes on:
   the next search location, then the artifact feed for a Microsoft app, for the same app and a version that satisfies the
   dependency. The `Application` aggregate holds no objects by design and still counts. A package of another publisher
   without source, and not on the feed, leaves the dependency missing: `dependency skipped: dependency `X` by P (id)
   version V or higher was not found with AL source (found only packages without AL source, which cannot serve it:
   <files>); searched: <every location>; put its .app with AL source ...`, and what needs it is refused one test at a time
   as before. A non-Microsoft package with source (a licensing app in `.alpackages`) works as it did.
2. **The build of the Microsoft apps and of the test data follows what the project really targets.** `apps::choose_build`,
   in this order: `--bc-version`; the build of the tests when they are Microsoft's; **the build of the Microsoft packages
   the project carries in its package folders** (`al_deps::load::microsoft_packages_build`: the highest build among
   Business Foundation, System Application, Base Application and Application, with or without source; `System` is left
   out, its number is the platform's), used when it is of the target's major and not below the app.json floor; else the
   newest published build of the target's major at or above the floor. The reason: a project's `.alpackages` holds what its
   developer compiled against, while `application` in app.json is only a minimum (Pageworks: packages 28.4, app.json 28.0).
   The BC minor of the target follows. The run header prints the rule (`build chosen by: ...`), a package build that is not
   cached, on disk or published is said (`that build is not ..., so BC X was used`), and packages outside the major or below
   the floor give a `note:`.
3. **Tests:** `select.rs` (a symbol package alone is missing and says what was seen, a later place with source satisfies, the
   aggregate counts), `load.rs` (the symbol package in the first place does not hide the source archive in the second;
   the message names the places searched; the build of the packages), `artifact.rs` (the feed supplies the source when
   `.alpackages` has only a symbol package; offline it stays missing with the reason), `apps.rs` (`choose_build`).

**Measured on a scratch copy of Pageworks** (the owner's repository is read only; a copy with the Microsoft symbol packages of
build 28.0.46665.48632 from another project's `.alpackages`, the test libraries that need the feed removed so that every
declared dependency is in `.alpackages`; BC 28.5.54151.56376 from the feed was the nearest published build). `alrunner test
Pageworks.Test/ Pageworks/`, load 16 to 21:

| | Before | After |
|---|---|---|
| Dependencies loaded | 1 (the licensing app) | 9 (Microsoft's from the feed, with source) |
| Test data | none | CRONUS 2 107 rows from 62 tables |
| Tests | 1 013: 61 passed, 0 failed, 952 not run | 1 013: 82 passed, 9 failed, 922 not run |
| Top refusal | 651 `codeunit <name> is outside the workspace` | 372 the same (the removed test libraries), 286 `[Native] ... no AL body` |
| Time | 0.24 s | 2.8 s warm; 25 to 35 s on a cold compile cache |

## The test runner's steps around each test method (2026-10-19, branch `per-test-reset`)

BC runs every test method through Microsoft's Test Runner app (`TestFramework/TestRunner/Test Runner.Source.zip`, BC 27.5).
Around each test method (not the test codeunit's OnRun) it runs code that no test names:

1. `Test Runner - Mgt`.PlatformBeforeTestRun runs codeunit 131006 "Permissions Mock" when it exists: the mock starts, or
   stops if it is started (`TestRunnerMgt.Codeunit.al:104-105`, `:198-208`).
2. Its event OnBeforeTestMethodRun reaches "ALTestRunner Reset Environment".BeforeTestMethod
   (`ALTestRunnerResetEnvironment.Codeunit.al:23-33`): ClearLastError, ApplicationArea(''), then codeunit 130301 "Reset
   State Before Test Run" when it exists (`:48-52`), codeunit 130302 "Set State Test Run" when it exists and the method's
   TestPermissions is not Disabled (`:54-62`), and codeunit 132553 "Block Changes to System Tables" when it exists
   (`:65-75`).
3. The test method runs.
4. PlatformAfterTestRun runs codeunit 131006 again (`TestRunnerMgt.Codeunit.al:127`).

130301 and 130302 are in Tests-TestLibraries, 131006 in Permissions Mock, 132553 in Prevent Metadata Updates. 130301 runs
`LibraryRandom.SetSeed(1)` (so every test method starts from `Randomize(1)`), starts the permissions mock with no set and
clears the temporary notification context. 130302 narrows the session to "D365 Full Access"; permissions are not enforced
(ADR-027), so a test whose TestPermissions is not Disabled runs on an assumption when 130302 is loaded (decided
2026-10-20, below).

**As built.** `al-testrun/src/runtime/runner_reset.rs` does steps 1, 2 and 4 in this order: our runner replaces the Test
Runner app, as it already did for ClearLastError and ApplicationArea. Each codeunit is Microsoft's own code from the
compiled apps, run by id like the runner's `Codeunit.Run` ("exists" is the runner's `AllObj` check: a codeunit with that id
in the loaded apps). The demand-driven check reaches their OnRun as roots (`check/demand.rs`, `TEST_RUNNER_CODEUNITS`), and
the baseline rule (`observe.rs`) counts what they reach as the test's. A step that is refused or fails refuses the test,
and a later failure of the codeunit is then not trusted (the step may have changed shared state). The TestPermissions
passed is the method's attribute unless it is InheritFromTestCodeunit, else the codeunit's property, else Restrictive (AL
Runner v2, `AlRunner/TestRunnerMgtEvents.cs:164-169`, read from the platform; not pinned by the corpus).

`PermissionTestHelper` (the platform object behind the mock) is a registry type (ADR-014): its constructor, `Clear` and
`Dispose` do nothing, because with no set assigned the user's own permissions apply (corpus
`permissions/TestPermissionsMockEnforcement.al:74-97`) and every access is granted here. `AddEffectivePermissionSet`
is refused when it runs.

**Not done.** The Test Runner app's events are not raised, so its other subscribers do not run. In BC 27.5 they are the
Test Runner app's own: per-test code coverage (off unless the suite asks for it) and the data-driven test input and output
codeunits ("Test Input", "Test Output", used by the AI Test Toolkit); the Performance Toolkit subscribes to
OnAfterTestMethodRun. No Base Application test app depends on the Test Runner app, so none of its tests can use them. The reset after each test codeunit
(130453.AfterTestMethod: WorkDate back to its value at the start of the run) needs nothing here: each codeunit runs on a
session of its own.

### Decided without owner review (2026-10-19)

- The runner does the steps natively and runs only the four codeunits as Microsoft's code. Running the Test Runner app itself
  would need its suite tables and the event with a `Test Method Line`; the steps are short and cited line by line.
- The order is the source's: 131006 before ClearLastError and ApplicationArea, then 130301, 130302, 132553; 131006 again
  after the method's transaction ended.

### Decided without owner review (2026-10-20, branch `lane-ms-regression`)

- 130302 no longer refuses the test. Its `PermissionTestHelper.AddEffectivePermissionSet` calls (All Objects, D365 Full
  Access, TEST TABLES, D365 BASIC, LOCAL through Permissions Mock) run on the registered assumption
  `runner-permission-sets-grant-all`: the sets grant all the test does, as the unrestricted session does here (ADR-027
  owner decision 2, "all permissions granted in 3.0"). Every test that passes after the step is a flagged pass. The
  refusal stopped about 6500 Microsoft tests (5 % of the plain 27.5 run on 2026-10-20). A test that assigns a permission
  set itself (`Library - Lower Permissions`.SetO365Basic and the like) is still refused when the call runs: its sets are
  narrower and the test may expect a denial. Reviewer: if ADR-027 decision 2 is meant to cover the runner's sets, drop
  the mark and the passes become clean.

## Amendment 2026-10-20 (owner decision, branch `dep-resolve`): older Microsoft packages in `.alpackages` give way to the feed

**Decision (owner).** A project's `.alpackages` holds compile symbols, often of several older builds. With `--bc-version`
set, a Microsoft app there that is older than the requested BC version must not win over the artifact feed. The case: the
project's `.alpackages` holds `Any` 27.1, the run asks for BC 29.0, the test library the feed supplies needs `Any` 29.0 or
higher, and the resolver failed with a version conflict.

1. **Held back for the feed** (`artifact.rs`, `load_with_artifacts`). Before a build is chosen, a package published by
   Microsoft in a folder named `.alpackages` whose major and minor are below `--bc-version` is held back, so the feed is asked
   for the app. Once a build is chosen, the same holds against the build's major and minor.
   The platform's `System` is compared on its major only: it carries the platform version (27.0.x in every 27 build).
2. **Fallback.** A held-back package whose app the feed does not supply is put back after the last round and used as before.
3. **Unchanged.** `--dep` and `--package-cache` packages, other publishers' packages and runs with `--no-download`.
4. **Conflict message.** A version conflict names the dependent app and where it came from, where the chosen version came
   from (a path, or "the artifact feed, cached at ..."), the packages that would fit, and the fix. With `--no-download` it
   says the `.alpackages` package was not replaced.

### Decided without owner review (2026-10-20)
- The rule also applies against the chosen build when `--bc-version` is not given, but a build is still taken from the
  feed: the run loads that build's Microsoft apps, so an older test library would conflict the same way.

## The test environment's apps: Microsoft's Test Runner (2026-10-20, branch `lane-test-runner-source`)

**What BC has.** BC runs tests through Microsoft's Test Runner app, so a BC test environment has it installed although no
test app declares it. The corpus pins its codeunits 130450-130455 in AllObj on BC 27, 28 and 29, and pins that
Tests-TestLibraries (130301, 130302, 130440), Permission Test Catalog (132218) and Prevent Metadata Updates (132553) are not
installed there (`permissions/lt-ba-testpermissions/BaTpEnv.al:28`). Microsoft's own recipe builds and installs the Test
Runner with the test apps (BCApps `build/projects/Apps W1/.AL-Go/settings.json:120`).

**As built.** `al-testrun/src/deps.rs` (`with_test_environment`) adds the Test Runner app (id
`23de40a6-dfe8-4f80-80db-d70f83ce8caf`) to the project that the dependency resolution works for, as an optional root with
no minimum version, unless the project or a workspace app already declares it or is it. It is found like a declared
dependency: the project's `.alpackages`, `--deps`, then the artifact feed when a build is chosen anyway
(`TestFramework/TestRunner/Test Runner.Source.zip`). It is compiled from its source like every dependency app (hard rule 7),
demand-driven. Both the corpus job and `alrunner test` go through this function.

- **Not visible by name.** An app sees only the apps it declares (`al_sema::workspace`, `visible_from`), so a test app that
  names `Codeunit::"Test Runner - Mgt"` without declaring the Test Runner still does not compile, as on BC.
- **What becomes observable.** Its objects are AllObj and AllObjWithCaption rows, the app is a row of the app tables (NAV App
  Installed App, Published Application, Installed Application), and its profile `TestRoleCenter` is an All Profile row. BC's
  environment has all of these, because the app is installed there.
- **Unchanged.** Our runner still does the Test Runner's steps around each test natively ("The test runner's steps around
  each test method" above); the app's own codeunits are not what runs the tests. The six runner frames of
  `SessionInformation.Callstack` now carry the loaded app's version (they did when an app declared it already); no corpus
  test reads that version.
- **No package for it.** A run that finds none (offline, `--no-download` with nothing on disk) goes on without it and says
  so in the verbose log; AllObj reads that may land on 130450-130455 then refuse, as before.

### Decided without owner review (2026-10-20)

- Only the Test Runner is added. The other test framework apps are installed in some environments and not in others
  (Prevent Metadata Updates: in Microsoft's recipe, settings.json:121, not in the corpus environment, BaTpEnv.al:28), and
  whether the corpus environment has Library Assert and Library Variable Storage is not pinned. Adding them would be a
  guess; they come only as declared dependencies.
- This is not a seam change: no interface changes (`Dependencies`, `AppSource`, `Request` stay as they are). The optional
  root is the mechanism the platform's `System` and the `Application` floor already use. Reviewer: if the owner sees an app
  that no project declares as a change of decision 2, this section becomes an amendment to accept.
