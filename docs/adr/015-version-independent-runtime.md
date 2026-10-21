# ADR-015: Version-independent runtime

- Status: Accepted (2026-10-13). The owner's answers to the open questions are in section 8.
- Amended (owner, 2026-10-19): the corpus side is now the corpus default branch `master`. The side
  branch `al-runtime-rs` was fast-forwarded into it, and work on the old AL runner on this corpus has stopped.
  Where this ADR says `al-runtime-rs` branch or "never touches the corpus `master`", read `master`. The flow
  (staging, bump pull request, merge commit) is in `docs/agents/corpus-writer.md`, "How a bump lands on master".
- Date: 2026-10-13
- Amends: ADR-007 (one oracle, one baseline, BC 27.5 only), ADR-008
  (catalog has no version data), ADR-013 (Layer A, floor below BC 27)

## Context

The owner's goal (2026-10-13): one codebase that supports Business Central 27
and every later major version, including future ones as they ship. Nothing
older than 27.

Decisions already made by the owner:

- **Range.** BC 27 and every later major. No BC 26 or older.
- **Oracles.** The corpus CI runs the suite against each BC version in a
  matrix and publishes one JUnit report per version. It does this on the
  corpus side branch `al-runtime-rs` (repo `StefanMaron/BusinessCentral.AL.Language.Tests`).
  This repository never touches the corpus `master`. (Superseded 2026-10-19: the corpus side is now
  `master`; see the amendment at the top.)
- **Per version.** al-runtime-rs keeps one oracle and one passing-tests
  baseline (the ratchet) per version.
- **Probes.** Local BC Linux containers are used for quick probes.

Today the runtime emulates exactly one version, BC 27.5. This ADR lists
everything that is tied to 27.5 and proposes how to remove the tie. It also
measures how much BC versions really differ, because that decides how much
machinery is worth building.

## 1. Inventory: what is tied to BC 27.5 today

All paths are relative to the repository root. Line numbers are from `main`
at commit `9d6957c`.

### 1.1 Oracle and ratchet

| Where | What is tied to 27.5 |
|---|---|
| `oracle/oracle.toml:1` | `bc_version = "27.5"`. The file holds one version, one `corpus_commit` (line 2), one `workflow_run` (line 3), one `junit` (line 4). |
| `oracle/junit-27.5.xml` | The only oracle. The file name carries the minor but not the build. The XML has no build or platform number either (its root is `<testsuites name="DEFAULT" ... timestamp="2026-10-12T20:53:55Z">`). |
| `oracle/README.md:3-4,12,19` | Documents the artifact `junit-test-results-27.5` (id 11772863083) and one bump procedure. |
| `crates/al-test-cli/src/main.rs:12` | `OracleConfig.bc_version` is a single string. It is only printed (line 135). |
| `crates/al-test-cli/src/main.rs:79` | One oracle path: `root.join("oracle/oracle.toml")`. |
| `crates/al-test-cli/src/main.rs:146` | One baseline path: `metrics/passing.txt`. |
| `metrics/passing.txt` | One ratchet file for all of the runtime. |
| `crates/al-test-cli/tests/corpus_harness.rs:73` | Copies `oracle/junit-27.5.xml` by name. |
| `CLAUDE.md:13,48,62`, `README.md:61`, `.github/workflows/ci.yml:41` | Name `metrics/passing.txt` as the one ratchet. |

### 1.2 Preprocessor symbols

BC 27.5 defines `BC27PLUS` only. The set is written out by hand in eleven
places. Eight files run the corpus with it, one is a unit-test helper, and two
only use the symbol as an arbitrary name.

| Where | Kind |
|---|---|
| `crates/al-test-cli/src/main.rs:95-96` | The corpus run itself (the one that matters). |
| `crates/al-test-cli/tests/perf_budget.rs:42` | Perf test. |
| `crates/al-test-cli/examples/compile_dump.rs:18`, `examples/phases.rs:38` | Examples. |
| `crates/al-syntax/tests/corpus_lex.rs:62-63`, `corpus_parse.rs:31,65`, `perf_parse_throughput.rs:79` | Tests. |
| `crates/al-sema/tests/corpus_resolve.rs:52`, `crates/al-sema/src/workspace.rs:512` | Test and a unit-test helper. |
| `crates/al-syntax/src/parser.rs:327-331`, `preprocessor.rs:221` | Use `BC27PLUS` only as a name to test `#if`. Harmless. Leave alone. |

The corpus CI defines the symbols per leg, cumulatively: 27.x gets
`BC27PLUS`, 28.x gets `BC27PLUS,BC28PLUS`, 29.0 gets all three
(`corpus/.github/workflows/ci.yml`, job `prepare`).

### 1.3 Builtins catalog

| Where | What is tied |
|---|---|
| `corpus/scripts/al-surface.json` | Scraped from Microsoft's docs; committed 2026-05-22 (corpus commit `3252758`). The local docs checkout used for this ADR is from the same day. It does carry data per overload: `runtime_version`, the docs line "Available or changed with runtime version X". 1739 overloads. 1334 say 1.0 (any form), 7 say 16.0, 16 say 17.0, none say 18.0. |
| `crates/al-builtins/src/signature.rs:28-37` | `DocEntry` has no `runtime_version` field, so serde drops it. The data is never read. |
| `crates/al-builtins/catalog/signatures.txt` | 1724 lines, no version column. Line 1 is the header. |
| `crates/al-builtins/src/catalog.rs:7-8,125-148` | The catalog is compiled in with `include_str!` and has no notion of a target. |
| `crates/al-builtins/src/catalog.rs:106-123` | An overload id is its position among the sorted lines of that method (`Record.SetFilter#2`). Adding an overload in the middle renumbers the ones after it. See 3.3. |
| `crates/al-sema/src/lower.rs:26` | `Catalog::load()`: one catalog for every run. (`check.rs:85` already takes the catalog as a parameter, which helps.) |
| `crates/al-sema/src/` | Nothing reads app.json `runtime`, `platform` or `application`. `crates/al-test-cli/src/runtime.rs:12-44` (`load_app`) reads `id`, `name` and `dependencies` only. |

### 1.4 Behaviour and error texts

| Where | What is tied |
|---|---|
| `crates/al-vm/src/flowfields.rs:140-146` | A FlowField Sum or Average must have the very same type as its source field (`Integer` into `BigInteger` is refused with "The following fields must have the same type"). The comment says "BC 29 relaxes Integer/BigInteger; the oracle is BC 27.5". This is the first known version difference in the code. |
| Error texts in `al-vm` and `al-builtins` | At least 20 `error(...)` call sites carry literal texts. Each text is pinned by a 27.5 oracle case. Nothing checks them on 28 or 29. |
| Platform values | ADR-013 lists identity values (user id, company, culture). A few platform values contain the build, for example `Object Metadata."Emit Version"` is `<major><3-digit build counter>` (`corpus/tests/al-language-onprem/record/TestObjectMetadataSystemTable.al:157`). These must come from the target, not a constant. |

### 1.5 CI, reports, perf, docs

| Where | What is tied |
|---|---|
| `.github/workflows/ci.yml:43-47` | One corpus run, no version. Report artifact `pass-rate-<date>-<run>` (line 67) has no version in its name. |
| `.github/workflows/ci.yml:49-57` and `perf_budget.rs`, `perf_parse_throughput.rs` | Budgets run on the 27.5 symbol set. Fine to keep on one version (see 5.6). |
| `docs/status.md:3`, `README.md:17,24,44`, `docs/corpus-inventory.md:4-5,13,46-47,56`, `docs/adr/README.md:18`, `docs/adr/007-oracle-and-ratchet.md` | Say "Oracle: BC 27.5". Documentation only. |
| `docs/adr/013-external-objects.md` (decision 2, around lines 356-440) | Layer A already selects the Microsoft build from `--bc-version` or the app.json floor. It reads the oracle's `bc_version` in corpus mode. Compatible with this ADR; one line to amend (see 3.2). |

### 1.6 Two oracle facts found while writing this ADR

1. **The other legs already exist.** The corpus run that produced the 27.5
   oracle (workflow run 38306086475, commit `cf6e546`) also produced JUnit for
   27.0, 27.3, 28.0 through 28.5 and 29.0. I downloaded them (read only) and
   compared them with the 27.5 file:
   - All ten reports have 0 failures.
   - 27.0, 27.3 and 27.5 have the same 4836 cases.
   - 28.x and 29.0 have the same 4836 cases plus **26 extra pseudo-cases named
     `OnRun`** (26 test codeunits report one). The 28 and 29 legs run
     through a different test runner (`testsuites name="hybrid"`, 27.x has
     `name="DEFAULT"`). These are a runner difference, not a BC difference.
   - Today the CLI would stop with exit 2 on these files: the 26 `OnRun` cases
     have no test procedure in the corpus (`al-test-cli/src/main.rs`, the
     `rep.unmapped` check). The oracle reader needs a rule for them.
2. **The artifacts expire on 2026-10-26** (14 days, `retention-days: 14` in
   `bc-linux/.github/workflows/bc-test-from-source.yml:1359`). Two artifacts
   share each name (`junit-test-results-28.0` is both the full report, 93 KB,
   and a 1.9 KB report of the OnPrem job). Downloading by name is ambiguous.
   Download by artifact id.

## 2. What actually differs between BC versions

Five areas matter. The measurements below are all from the repository's own
corpus and the Microsoft docs checkout
(`~/Documents/Repos/community/dynamics365smb-devitpro-pb`, commit date
2026-05-22).

### 2.0 Two version axes, not one

Business Central has two version numbers that act on a run:

- **The server version** (BC 27, 28, 29, plus a minor and a build). It decides
  platform behaviour, error texts, the system tables, and which Microsoft apps
  (Base Application and so on) are present.
- **The app's `runtime`** from app.json (16.0 for BC 27, 17.0 for BC 28, 18.0
  for BC 29). It decides which language features and builtins the compiler
  accepts. Microsoft's rule: an app can be published to a server whose runtime
  is the same or later, never earlier
  (`devenv-developing-for-multiple-platform-versions.md`). Without `runtime`,
  the compiler uses the newest it knows (`devenv-choosing-runtime.md`).

The map is arithmetic: **runtime major = BC major - 11**. Evidence: the docs
table (15.0 = BC 26, 16.0 = BC 27, 17.0 = BC 28), `bc-linux/scripts/resolve-al-tool-version.py`
("AL major = BC major - 11", BC 29 = 18.0), and the docs page for analyzer
AS0146 ("runtime version 18.0 (Fall 2026)").

The corpus shows why both axes are needed. Its app.json says `"runtime": "16.0"`
(`corpus/tests/al-language/app.json`) and the same app runs on BC 27, 28 and
29 servers. A BC 29 behaviour change still shows up there. So server behaviour
follows the server, not the app's runtime.

How much behaviour depends on the app's runtime (as opposed to the server) is
**not measured**. Probe P2 (same tests, same server, app runtime 16.0 then 17.0 then 18.0; section 4, C8) measures it.

### 2.1 The builtin surface

Docs data (`al-surface.json`, 1739 overloads) for the versions that matter
(floor is runtime 16.0):

| Runtime | BC | Overloads tagged | Examples |
|---|---|---|---|
| 16.0 | 27 | 7 | `Record.Truncate([ResetAutoIncrement])`, `RecordRef.Truncate`, `RecordRef.Field(Text)`, `RecordRef.FieldExist(Text)`, `Database.LockTimeoutDuration`, `Guid.CreateGuid` |
| 17.0 | 28 | 16 | `Codeunit.Run(FullyQualifiedName ...)`, `Page.Run` and `Page.RunModal` by name (5 overloads), `JsonObject/JsonArray/JsonToken.SelectTokens`, `DataTransfer.AddDestinationFilter`, `Record.FullyQualifiedName`, `RecordRef.FullyQualifiedName`, `RecordRef.Open(FullyQualifiedName)`, `Report.Run/RunModal/Execute` by name |
| 18.0 | 29 | 0 (docs have none yet) | The docs snapshot is from May. BC 29 shipped in October. |

What the data can and cannot say:

- **Good.** For the 16 overloads at 17.0, the docs git history shows each file
  was first added with 17.0 (commit `0c4beb0fa9`, 2026-03-11). So they are new
  overloads, not changed ones. Because our floor is 16.0, everything tagged
  16.0 or lower is simply "always there". Only tags of 17.0 and later matter.
  That is a small data set.
- **Weak: "changed" looks like "new".** The docs line is "Available **or
  changed**". `Guid.CreateGuid` says 16.0 although the method is old. A
  changed signature loses its old form, because the docs keep one page for the
  current form. The docs git history (first commit that added the file)
  separates new from changed. The scrape does not use it yet.
- **Weak: no removals.** 56 method pages say "Available ... until version N
  where it was deprecated". The scrape's regex (`scripts/scrape-al-surface.py`,
  `runtime version\s+([\d.]+)`) keeps only the first number. Deprecated does
  not mean removed; the compiler still accepts obsolete members, with a
  warning. Real removals are rare and the docs do not mark them.
- **Weak: docs lag.** On 2026-10-13 BC 29 is out and the docs checkout has no
  18.0 builtin. A method added in BC 29 would be unknown to the catalog. Today
  that gives `AR1002 no member ...` and the test is reported not run. It is a
  safe refusal, but the text is misleading.
- **Other language surface also carries versions.** 335 of 349 property pages
  and 40 of 41 attribute pages have a runtime marker, for example
  `RequiredTestIsolation`, `TestType`, `AllowInCustomizations`, `AsReadOnly`
  (16.0), `[Native]` (17.0). The scrape does not cover properties or
  attributes. The compiler rejects use of a feature above the app's runtime
  with a diagnostic such as AL0666 "'{0}' is not available in runtime version
  '{1}'. The supported runtime versions are: {2}".

### 2.2 The app.json runtime gating language rules

Documented examples: AL0856 (statements cannot start with a parenthesis in
older runtimes), AL0813 (a property value allowed only from a runtime), AL0666
(feature not available in the app's runtime), AL1043 (runtime not supported by
the compiler). The corpus app uses runtime 16.0, the nearest app.json to the
BC 27 floor. Today nothing in al-sema reads `runtime`. That is safe for the
corpus (it only has 16.0 code) and wrong for a user project that sets 15.0 or
17.0.

### 2.3 Behaviour and error-text changes

The corpus is the best evidence. It runs identical code on ten legs and its
authors write a `#if BCnnPLUS` guard wherever a leg disagrees.

- **Guards in the whole corpus:** 7 uses of `BC29PLUS` in 3 files. **Zero**
  uses of `BC27PLUS` or `BC28PLUS` in test code
  (`grep -rn "BC2[789]PLUS" corpus/tests --include=*.al`).
- **Many comments record "identical on 27.0 through 28.4"** (for example
  `handlers/TestPageActionRunObjectNoHandler_Tests.al:16`,
  `filterpage/TestFilterPageBuilder.al:679`, `autoformat/TestAutoFormat.al:52`).
- **The three known BC 29 differences:**

| Change in BC 29 | Corpus guard | Runtime code today |
|---|---|---|
| A FlowField Sum over an `Integer` field into a `BigInteger` FlowField is accepted (27 and 28 refuse it with "The following fields must have the same type") | `record/TestCalcFormulaValidationTests.Codeunit.al:205,227` | `al-vm/src/flowfields.rs:140-146` |
| `max()` and `min()` FlowFields over `SystemRowVersion` are refused (27 and 28 calculate them) | `record/TestCalcFormulaSystemFieldsTests.Codeunit.al:298,306,353` | not implemented |
| Looking up the `SUPER` or `SECURITY` Metadata Permission Set under any App ID finds the null-App-ID row (27 and 28 key on the App ID) | `permissions/TestMetadataPermissionSet.al:98,106` | not implemented |

So the measured rate is: three behaviour changes between BC 27.0 and BC 29.0,
all at the major boundary 28 to 29, none inside a major. This is a small
sample (the corpus has 4836 cases), but it is the best evidence we have. It
supports one decision: **key behaviour by BC major**, and allow a minor key
only when a measurement demands it.

### 2.4 System tables and their schemas

ADR-013 compiles the real platform `System.app` from the chosen build, so the
**schema** of each system table follows the build automatically: add a field
in BC 30 and the declaration in the BC 30 artifact has it. The rows still come
from native providers. A provider that does not fill a new column is the risk.
ADR-013 already says "a shape nobody provided is refused, not emptied", so a
changed schema becomes an explicit refusal, not a silent gap. Providers must
take build-dependent values (the `Emit Version` example above) from the
target.

### 2.5 Microsoft apps

ADR-013 decision 2 (Layer A and B) already selects the Base Application and
the other Microsoft apps from the build that matches the target. Nothing more
is needed here, except that Layer A must accept targets above 27 without code
change (it does, by design) and handle a floor below 27 (3.2).

## 3. Decision

### 3.1 One `Target` value, built once per run

A new leaf crate `al-version` (no dependencies, no OS access, ADR-006) holds:

```text
BcVersion { major: u16, minor: u16, build: Option<String> }   // 29, 0, "29.0.54011.55816"
Runtime   { major: u16, minor: u16 }                          // 18.0
Target    { bc: BcVersion, runtime: Runtime }
```

- `Runtime::for_bc(major) = major - 11`, minor 0.
- Versions compare as `(major, minor)`. The build string is for reports only.
- `Target` is created once by the entry point (the CLI) and travels with the
  compiled program: an additive field on the IR `Program` (hard rule 5 says
  additive fields need no ADR). Natives read it through a defaulted `Ctx`
  method.

Core crates never read `oracle.toml`, `--bc-version` or the environment. They
receive a `Target`.

### 3.2 How the target is chosen for a run

Consistent with ADR-013 Layer A:

1. `--bc-version PREFIX` wins. It selects the BC version (`29`, `29.0`,
   `27.5`, or a full build). The same flag already selects the Microsoft
   build.
2. Otherwise the BC major is the major of the app.json floor (the higher of
   `application` and `platform`), as in ADR-013 Layer A step 2.
3. **New, amends Layer A step 2 (owner decision Q7).** If that major is below 27, the target is
   BC 27 and the run prints a note. Reason: an app that declares a floor of 24
   still runs on a BC 27 or newer server. We do not emulate older servers.
4. The app's `runtime` comes from app.json. If it is missing, use
   `Runtime::for_bc(major)`. If the app's runtime is above the target's, exit
   2 with BC's own rule (the text is in the Microsoft docs; confirm with probe
   P3, C8). If it is below 16.0, treat it as 16.0 and print a warning (we never
   reject code BC accepts, ADR-009; the catalog cannot show older signatures).
5. A project can also have a build floor above the flag (`--bc-version 27`
   with `application: 28.0.0.0`). That already runs with a warning (ADR-013
   Layer A step 3).

In corpus mode the harness reads the oracle entry for the chosen version and
passes the same values. The corpus app.json says `runtime 16.0`, so on BC 29
the target is `bc 29.0, runtime 16.0`. This is exactly what the corpus CI
does.

### 3.3 Version as data: the builtin catalog

1. **Tags in the catalog.** Each line in `signatures.txt` may end with
   `@N.M` ("available from runtime N.M"). A line may also end with `^N.M`
   ("not available from runtime N.M on"). Lines at or below the floor
   (runtime 16.0) get no tag. Today that means 16 tagged lines (all `@17.0`).
2. **Generation.** `DocEntry` reads `runtime_version`. The generator writes
   the tag. The existing golden-file test
   (`catalog_generated.rs`) fails when the docs scrape and the file disagree,
   so a corpus bump shows new tags as a diff. `overrides.txt` can change a
   tag, with evidence, like any other change.
3. **Filtering.** `Catalog::load()` becomes `Catalog::for_target(&Target)`.
   `overloads()` and `global()` return only overloads available at the app's
   runtime. A second call, `Catalog::hidden(receiver, method)`, returns what
   the filter removed.
4. **Refusing the way the BC compiler refuses.** When a call matches no
   visible overload but matches a hidden one, the checker emits a compile
   diagnostic `AR1005 'X' is not available in runtime 16.0 (introduced in
   17.0)`. In BC this is a compile error, so the app does not build. The
   harness reports the test as not run with that reason. Which AL code and
   text BC uses (AL0666 or AL0132) is probe P1 (a call above the app's runtime; section 4, C8). Diagnostics are ours to word
   (ADR-009). Only "does not compile" must match.
5. **Unknown members.** `AR1002 no member` for a name that is in no version
   of the catalog now says: "not known to the catalog (docs for runtime up to
   17.0)". Same refusal, honest text.
6. **Overload ids must not change when a version adds an overload.** Today
   the id is the position in sorted order (`catalog.rs:106-123`). New rule:
   number overloads first by tag (untagged first, then `@17.0`, then `@18.0`),
   and by sorted line inside one tag. An overload added in a later version
   then always gets a number after all existing ones, and registered
   implementations (`Record.SetFilter#2`) keep their ids. A test pins the
   existing ids so a regenerate cannot renumber them.
7. **Sources.** Phase 1 (this ADR): the docs scrape, using the docs git
   history to tell new from changed (a new file at tag N is `@N`; a changed
   one is not tagged). Phase 2 (open question 4): a per-version dump of the
   compiler's own member tables, produced in each corpus CI leg, so the
   catalog does not depend on when Microsoft publishes docs.

### 3.4 Behaviour differences are version-gated implementations

1. **A gate is a named fact, in one table.** `al-version/src/gates.rs`:

   ```text
   Gate::FlowFieldSumIntegerIntoBigInteger   since BC 29   pinned by record/TestCalcFormulaValidationTests.Codeunit.al:205,227
   Gate::FlowFieldMinMaxOverRowVersionRefused since BC 29   pinned by record/TestCalcFormulaSystemFieldsTests.Codeunit.al:298,353
   Gate::SuperPermissionSetIgnoresAppId       since BC 29   pinned by permissions/TestMetadataPermissionSet.al:98,106
   ```

   Code asks `target.has(Gate::X)`. It is `true` when `target.bc.major >= since`.
2. **The default is the newest behaviour.** A target newer than every gate
   (BC 30 today) gets all behaviours. A new major therefore inherits the
   newest known behaviour and only a failing oracle case adds a gate.
3. **Both sides are pinned.** A gate is added only when the per-version
   oracles contain a case for each side (corpus `#if BCnPLUS` guards give
   exactly that). A difference seen in an oracle but not understood is
   refused on the versions that need it (`Signal::Unsupported` or a lowering
   reason), never guessed (hard rules 3 and 4).
4. **Completeness test.** A test scans the corpus for `BC<n>PLUS` guards
   with n above 27. Each guarded file must be cited by a gate. A new guard in
   the corpus without a gate fails the test, so the corpus tells us when a
   gate is missing, even before the case fails.
5. **Error texts** that differ by version are looked up through a gate
   (`target.has(Gate::...)` picks the text). A text is changed per version
   only where two oracles disagree.
6. **No version numbers in code.** A test greps `crates/*/src` for the
   literals `BC2`, `27.5`, `major ==`/`major >=` outside `al-version` and
   fails. All version logic is data or a gate.

### 3.5 Per-version oracle and baseline files

Layout:

```text
oracle/oracle.toml            one file, one table per version
oracle/27/junit.xml           oracle for BC 27
oracle/28/junit.xml
oracle/29/junit.xml
metrics/27/passing.txt        ratchet for BC 27
metrics/28/passing.txt
metrics/29/passing.txt
```

`oracle.toml`:

```toml
corpus_repo   = "corpus"
corpus_app    = "corpus/tests/al-language"
corpus_commit = "cf6e546793a92c482499ce86eadf0a0b13d2a742"   # shared: one CI run, all legs

[[version]]
major        = 27
bc_version   = "27.5"                  # the leg the file came from
bc_build     = "27.5.46862.53931"      # optional until the corpus records it (section 4)
workflow_run = 38306086475
artifact_id  = 11772863083
junit        = "27/junit.xml"
baseline     = "metrics/27/passing.txt"
```

Rules:

- `corpus_commit` must equal the submodule pin, as today (exit 2 otherwise).
- Every version's JUnit must come from the CI run of that same commit.
- The JUnit reader ignores the runner's `OnRun` pseudo-cases, with a unit
  test (1.6). Anything else without a test procedure is still exit 2.
- Each version has its own ratchet. `--update-baseline` writes only the
  baseline of the chosen version. It refuses while any case in that baseline
  regresses (unchanged rule).
- A case can pass on 27 and fail on 29. That is what the gate is for. The
  ratchet is per version, so one version never hides a regression in another.
- Which leg is the oracle for a major (27.5 or 27.0) is open question 1.

### 3.6 CLI and CI

- `al-test-cli --bc-version 29` runs corpus mode for that version. Without the
  flag it runs the newest version that has an oracle. A prefix that matches no
  `[[version]]` exits 2 and lists the known ones.
- `al-test-cli --all-versions` runs every version in turn and exits 1 if any
  regresses. Outputs are named per version (`pass-rate-29.json`,
  `pass-rate-29.md`, `runtime-junit-29.xml`).
- The summary adds a **drift table**: cases that pass in one version's
  oracle and are absent or failed in another.
- CI (`.github/workflows/ci.yml`): the corpus step runs `--all-versions`.
  Artifact `pass-rate-<date>-<run>` holds all per-version files. The build
  takes about one second per version, so a matrix job is not needed yet.
  Perf budgets stay on the newest version (they measure the compiler, not the
  version).
- `tools/inventory.py` already takes the JUnit path as an argument. No change.

### 3.7 Adding a new BC major, zero code in the common case

For BC 30:

1. Corpus side: add 30 to the matrix list. The symbols come from the number
   (`BC27PLUS` ... `BC30PLUS`). One CI run produces its JUnit.
2. In this repo, run `tools/add-bc-version 30 <run-id>` (new script). It
   downloads the artifact by id, writes `oracle/30/junit.xml`, adds the
   `[[version]]` table, and creates an empty baseline.
3. Bump the corpus submodule if needed. Regenerate the catalog:
   `UPDATE_GOLDEN=1 cargo test -p al-builtins`. New builtins appear as `@19.0`
   tags in the diff.
4. Run `al-test-cli --bc-version 30`. The target gets runtime 19.0 and all
   known gates.
5. Read the result:
   - Everything that passed on 29 passes: run `--update-baseline`. Done, with
     no Rust change.
   - A case fails: it is a real difference. Add a gate or a refusal
     (red/green, citing the new guard in the corpus). Only then is there code.

### 3.8 A target with no oracle

- **Corpus mode, no `[[version]]` for the target: exit 2.** There is no truth
  to measure against, so a pass rate would be meaningless.
- **Normal project mode (ADR-013): run, with a warning.** Text: "No oracle for
  BC 31. Using the behaviour of BC 29, the newest verified. Results for this
  version are not checked against real BC." The behaviour is the newest known
  one (3.4 point 2). This is open question 3.
- **BC 26 and older: never run.** A `--bc-version 26` exits 2: "BC 26 and
  older are not supported" (see 3.2 point 3 for an app whose floor is older).

### 3.9 Preprocessor symbols

The corpus convention `BC<n>PLUS` is built by a pure function
`al_version::corpus_symbols(major)`: one symbol per `n` from 27 through
`major`. The test runner (`al-test-cli`) calls it. It lives in `al-version`
so the call sites in 1.2 share it. For other projects the symbols come
from app.json `preprocessorSymbols` and nothing else (the runtime must not
invent `BC29PLUS` for a third-party app).

## 4. The corpus side (proposal for the `al-runtime-rs` branch only)

Nothing here touches the corpus `master`. Today the corpus CI already has the
matrix (27.0, 27.3, 27.5, 28.0 to 28.5, 29.0), the full-build pin for 29.0
(`bc_artifact_url`), and the cumulative symbols. What it lacks for this ADR:

**C1. Version list as data.** In job `prepare`, build the matrix from a list
of majors and minors, and compute `preprocessor_symbols` from the major. Today
the symbols are typed per entry, and the override branch has `for v in 27 28`
(missing 29). Adding BC 30 should be one list entry.

**C2. One leg per major is the oracle leg.** Mark the oracle leg of each major
(newest minor, default). The other legs stay as the drift check.

**C3. Artifact selection per version, with the full build.** The reusable
workflow (`StefanMaron/MsDyn365Bc.On.Linux`, `bc-test-from-source.yml`)
resolves `bc_version: "27.5"` to the newest 27.5 build at run time, so two runs
can use different builds, and nothing records which one. Proposal:
1. `prepare` resolves each leg to a full build once (from the public index
   `.../sandbox/indexes/w1.json`, the same source the container uses) and puts
   `bc_build` into the matrix.
2. Each leg passes the full build as `bc_version` (the download script skips
   index resolution when the version has three or more parts,
   `download-artifacts.sh:308`) or as `bc_artifact_url` (already done for 29.0).
   Probe P4 (one leg with a full build as `bc_version`; C8) checks that the AL-tool lookup accepts it.
3. The corpus never relies on a floating build for an oracle. A new hotfix
   enters the oracle only by changing the list.

**C4. JUnit artifact name with the full build.** The reusable workflow names
the artifact `junit-test-results-<bc_version>` (line 1356). If C3 passes the
full build as `bc_version`, the name becomes
`junit-test-results-27.5.46862.53931` with no upstream change. In addition, a
final `collect` job (`needs: [test, test-onprem]`) does the following:
- Downloads every `junit-test-results-*` of the run by artifact id.
  (Two artifacts share each name today: the main job and the OnPrem job.)
- Writes `oracle-manifest.json` with, per leg: requested version, full build,
  runtime, symbols, file, test count, failures, corpus commit, run id.
- Uploads one artifact `oracles-<corpus-sha>` with **90 days** retention
  (the upstream default is 14).
- Fails if any leg has no JUnit, or if a leg's case set differs from the
  oracle leg of its major (the drift check).
- The JUnit XML is kept byte for byte. The build lives in the manifest and in
  the file name, not inside the XML.

**C5. Runner difference.** The 27.x legs report `name="DEFAULT"` and 28.x/29.0
report `name="hybrid"` with 26 extra `OnRun` pseudo-cases (1.6). Best fix is
upstream (do not report `OnRun` as a case). Until then, al-runtime-rs ignores
those pseudo-cases (3.5).

**C6. Surface data per version.** Extend `scripts/scrape-al-surface.py` to also
write `introduced` (from the docs history) and `until` (the deprecated-until
number). Optional phase 2: each leg runs a step that dumps the AL compiler's
member tables for the built-in types to `al-surface-<build>.json`
(the corpus already does something close: `TestPageActionReachability_Tests.al:43`
lists the TestPage member table from `Microsoft.Dynamics.Nav.CodeAnalysis.dll`).
The dump is the truth for "exists in this version"; the docs stay the source
of parameter names and descriptions.

**C7. Local probes.** A BC Linux container per version is already how the
corpus authors probe (`BC_VERSION=27.5.46862.48612 docker compose up -d
--wait`, `download-artifacts.sh:367`). Document the recipe in the branch's
`CONTRIBUTING.md`, with the exact full-build form, so a probe on BC 28.5 and
on 29.0 is one command each.

**C8. Probes to run (each is a corpus test or a CI leg on the branch):**
- **P1.** App `runtime 15.0` (or any value below the call's tag) calling a
  builtin tagged higher, for example `Record.Truncate(true)` (16.0) against a
  15.0 app on BC 27. Record the diagnostic code and text.
- **P2.** The same test codeunit on the same BC 29 server with app.json
  `runtime` 16.0, then 17.0, then 18.0. Any difference in outcome is
  behaviour gated by the app's runtime, which this ADR does not model yet.
- **P3.** Publish an app whose runtime is above the server's. Record the text.
- **P4.** One leg with a full build as `bc_version`; confirm the compile step
  and the artifact name.
- **P5.** `Record.Truncate()` with no argument on a 15.0 app: does the old
  form still compile? (Shows whether a "changed" entry hides a valid older
  form.)

## 5. Migration plan

Small steps. Steps 1 to 4 change no behaviour. Each step keeps `check.sh`
green. Where a step changes a hard-rule text (`CLAUDE.md`), the owner approves
that edit first.

1. **`al-version` crate.** `BcVersion`, `Runtime`, `Target`, parsing,
   `Runtime::for_bc`, `corpus_symbols`. Pure functions with tests. No consumer.
2. **One symbol function.** Replace the ten `BC27PLUS` sites in 1.2 that run the corpus or a
   helper with `corpus_symbols(27)`. Check that `compile_dump` output is byte identical
   before and after.
3. **Oracle schema.** New `oracle.toml` with one `[[version]]` (27, 27.5).
   Move `oracle/junit-27.5.xml` to `oracle/27/junit.xml` and
   `metrics/passing.txt` to `metrics/27/passing.txt`. The CLI reads the new
   schema, `--bc-version` selects, default is the only version. Update
   `oracle/README.md`, `corpus_harness.rs:73`, `ci.yml`, `README.md`, and the
   `CLAUDE.md` paths. Pass counts do not change.
4. **Read app.json `runtime`, `platform`, `application`** into `AppSource`
   (additive) and build the `Target`. Nothing uses the runtime yet. Report the
   target in the run's first line: `BC 27.5, runtime 16.0`.
5. **Catalog tags.** `DocEntry` reads `runtime_version`; tags in
   `signatures.txt`; `Catalog::for_target`; append-only ids; `AR1005`. Red
   test first: a 15.0 app calling a 16.0 method is refused; a 17.0 call at
   target 27 is refused with `AR1005`. The corpus app runs as before (it has
   no 17.0 calls). Pass counts do not change.
6. **Oracles for 28 and 29.** Done by hand from the existing run
   (artifact ids in 1.6, before they expire on 2026-10-26): add the two
   `[[version]]` tables and the two JUnit files, skip `OnRun` pseudo-cases
   (red test first: the 28 file now gives exit 2). Create baselines with
   `--update-baseline`. This is the first run that can show real
   version differences.
7. **First gates.** Run `--bc-version 29`. The three known differences (2.3)
   each become a red test, then a gate. The completeness test (3.4 point 4)
   lands here.
8. **CI over all versions.** `--all-versions`, per-version reports, the drift
   table.
9. **No-oracle and old-version rules** (3.8) and the floor-below-27 rule
   (3.2 point 3), with tests. Amend ADR-013 Layer A in the same commit.
10. **No-version-literal test** (3.4 point 6) and docs: `status.md` gets one
    table row per version.
11. **Corpus side** (section 4), in parallel with steps 6 to 10 and on the
    branch only: C1 to C5 first, C6 to C8 after.
12. **Later, only if needed:** compiler dump as catalog source (phase 2), the
    minor-level oracle (question 1), the app-runtime probe result (P2).

Time-sensitive: the CI artifacts for the current oracle run expire on
2026-10-26. Step 6 can be done first if the owner wants the data kept.

## 6. Consequences

- A new BC major usually costs a matrix entry, one script run, one baseline
  update. Code changes only for behaviour that really changed.
- The 28 and 29 oracles exist today. Step 6 can start without waiting for
  any corpus change.
- Every version difference in the runtime is one named gate with its
  evidence. `grep Gate::` lists them. The corpus guards and the gates are
  forced to agree by a test.
- Docs lag is visible: a builtin new in BC 29 is "not known to the catalog",
  not silently missing.
- Costs: `Target` is threaded through compile and run (one field); the
  catalog format gains a suffix; the oracle directory grows by about 0.6 MB per
  version; CI runs the suite once per version (about one second each).
- The ratchet is stricter: a regression on any version fails CI.
- Risks: (1) docs data may be wrong about "introduced" (mitigated by the
  override mechanism and phase 2 dump). (2) Behaviour that depends on the
  app's runtime is not modelled until probe P2 (C8) has been run. (3) Differences inside one
  major (between minors) are checked only by the drift report.

Does not change: the IR and its serialisation (ADR-011), the value model
(ADR-010), the Store trait (ADR-003), the Engine/Session split (ADR-001),
hard rules 1 to 6, and the rule that `oracle/` and `corpus/` are never edited
by hand to turn CI green.

## 7. Conformance

- `al-version` unit tests: runtime arithmetic, version ordering, prefix
  matching (`27.5` matches `27.5.x`, not `27.50`), symbols.
- `al-builtins`: the generated catalog is up to date with tags; existing
  overload ids are pinned and do not change when a later-tagged overload is
  added; `for_target` hides `@17.0` lines at runtime 16.0.
- `al-sema`: `AR1005` for a hidden overload; `AR1002` text for an unknown one.
- `al-test-cli` harness tests: unknown `--bc-version` exits 2; pin mismatch
  exits 2; `OnRun` pseudo-cases are skipped; a baseline case that does not
  pass exits 1, per version.
- Gate completeness test (3.4 point 4) and no-version-literal test (3.4
  point 6).
- CI: `--all-versions` over every `[[version]]`.

## 8. Open questions for the owner

Each question has one decision and a recommendation. The owner answered all
seven on 2026-10-13; the answer follows each question.

**Q1. What is the key of an oracle and a baseline?**
- *Per major, taken from the newest minor of that major (27.5, 28.5, 29.0).*
  Three oracles and three baselines. The other minors are only compared by
  the corpus `collect` job (drift check, C4). Small and easy to review.
- *Per major plus an automatic drift check* (same as above, but the drift
  check also fails the corpus job when a minor's case set differs). Same
  files, stricter corpus CI.
- *Per minor.* One oracle and one baseline for each of the ten legs. The
  most precise, about ten times the files, and ten ratchets to keep green.

Recommendation: per major plus the drift check. Evidence: 27.0, 27.3 and 27.5

**Owner's answer (Q1):** Per major, taken from the newest minor of that major, plus an automatic drift check over the other minors (the second option). Accepted 2026-10-13.
have identical case sets, 28.0 through 28.5 and 29.0 likewise, and the corpus
records no behaviour difference inside a major.

**Q2. What does `al-test-cli --root .` do with no `--bc-version`?**
- *Run the newest version that has an oracle.* Matches "default is the
  newest behaviour". Changes what a bare command means today (BC 27.5).
- *Run every version and print one table.* Most honest. About one second each.
- *Refuse without the flag.* Explicit, but annoying.

Recommendation: run the newest; CI uses `--all-versions`.

**Owner's answer (Q2):** Run the newest version that has an oracle. CI uses `--all-versions`. Accepted 2026-10-13.

**Q3. A normal project targets a BC version we have no oracle for. What happens?**
- *Run with a warning* ("using the behaviour of BC 29, the newest verified").
  A project on BC 31 works the day BC 31 ships.
- *Refuse (exit 2).* Safe, but defeats "every later version automatically".

Recommendation: run with a warning. Corpus mode still refuses (3.8).

**Owner's answer (Q3):** Run with a warning. The project runs on the newest verified behaviour. Corpus mode still refuses. Accepted 2026-10-13.

**Q4. Where does the builtin surface per version come from?**
- *Docs only*, with the docs git history for new versus changed. Cheap, works
  now, lags Microsoft's docs (no BC 29 data today).
- *Docs plus a dump of the AL compiler's member tables from each corpus leg.*
  Exact "exists in this version". Needs a corpus-side script and a reflection
  step over `Microsoft.Dynamics.Nav.CodeAnalysis.dll`.
- *Probes only* (compile a call per overload on each version). Exact but
  needs a generated, typed call for about 1740 overloads.

Recommendation: docs now (step 5), the compiler dump as a follow-up when a

**Owner's answer (Q4):** Docs now (step 5). A compiler member-table dump comes later, as phase 2. Accepted 2026-10-13.
BC 29 or BC 30 builtin is missing.

**Q5. How do we get the full build into the oracle?**
- *Pin the full build in the corpus matrix* (C3). Reproducible; hotfixes enter
  by editing the list. No upstream change.
- *Ask the BC Linux workflow repo to publish the build as an output and in
  the artifact name.* Cleaner long term; changes another repo.
- *Both.*

Recommendation: pin in the matrix now; ask upstream later.

**Owner's answer (Q5):** Pin the full build in the corpus CI matrix now. Asking the BC Linux repo upstream to publish the build comes later. Accepted 2026-10-13.

**Q6. An app declares `runtime` below 16.0 (older than the catalog can show).**
- *Treat as 16.0 and warn.* Never rejects code BC accepts (ADR-009).
- *Refuse.* Strict. Rejects some code BC accepts.

Recommendation: treat as 16.0 and warn.

**Owner's answer (Q6):** Treat as 16.0 and warn: the app runs as BC 27 / runtime 16.0 with a note. Accepted 2026-10-13.

**Q7. May this ADR amend ADR-013 Layer A so that a floor below BC 27 runs as BC 27?**
- *Yes: target BC 27 with a printed note.* An app with `application: 24.0`
  still works.
- *No: exit 2 for any floor below 27.* Follows "nothing older" literally.

Recommendation: yes, with the note. The app is not asking us to emulate BC 24,

**Owner's answer (Q7):** Yes. A floor below BC 27 runs as BC 27 / runtime 16.0 with a printed note. This amends ADR-013 Layer A step 2 (the same wording is added to ADR-013). Accepted 2026-10-13.
it only declares the lowest server it can run on.

## Decided without owner review (2026-10-21): the runtime each app declares (al-runner issue 5573)

- `Target::app_runtimes` carries the `runtime` each app under test declares in its app.json when it is below the
  target's runtime. The checker gates the builtins of that app's code by its own runtime, not by the target's
  (dependency apps are never gated: the AL compiler vetted them). Before, only the first app's runtime set the target,
  and a runtime below 16.0 silently ran as 16.0.
- A call of a catalog overload tagged newer than the app's runtime is the AL compiler's AL0666, with its text
  (must-not-compile 0576, 0620, 0364, ...); it replaces the runtime's own AR1005. Lowering refuses the routine.
- Below the runtime floor (16.0) the catalog's tags decide nothing. Microsoft's docs give each overload the runtime
  it was "available or changed" with (`catalog/docs-runtimes.txt`, generated from `corpus/scripts/al-surface.json`).
  A docs runtime at or below the app's runtime clears the call (the overload existed then). A later one, or none
  (a line only `overrides.txt` has), is undecidable ("or changed"): the call is AR1007 and the routine is refused.
  The compiler's AL0666 text is not reported there: no corpus case pins it below 16.0.
- Not covered: language features, properties and object types newer than the app's runtime. They are as ungated as
  before, at every runtime.
