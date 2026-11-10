# ADR-058: Stability testing mode: run the same tests under declared changes of the environment

- Status: Accepted (owner, 2026-10-30)
- Date: 2026-10-30
- Related: ADR-009 (match BC where observable), ADR-013 (identity values, the runner's steps around each test),
  ADR-028 (`serve`), ADR-043 (impact trace), ADR-045 (stepping test clock), ADR-046 (install baseline, `--companies`),
  ADR-050 (`--affected`), ADR-052 (matrix runs over BC versions and countries), ADR-053 (translations, deferred),
  ADR-057 (debugging API, deterministic re-execution)

## Context

Microsoft presented a planned "stability mode" for Business Central tests. It runs a suite again with:

1. other seeds for `Library - Random` and `Any`;
2. each test run alone;
3. the tests in reverse order;
4. the work date moved one year into the future.

A test that passes in the normal run and fails in one of these runs depends on something it should not depend on:
an earlier test, a fixed seed, today's date. The owner wants the same idea in AL Runner, but generic: a config file
declares which parts of the environment change and how, and the list of parts that can change is longer than
Microsoft's four.

The owner also wants (2026-10-30) the BC version and the country to be part of the same config, as matrix axes next
to the perturbations, so that one mechanism covers the existing runs per BC major and per country.

### What AL Runner fixes today

AL Runner is deterministic by design (ADR-009 lists "injected clock, identity, randomness" as allowed improvements).
Every value that differs between real BC environments or runs is fixed to a stated value:

| Value | Today | Where |
|---|---|---|
| Clock | Noon UTC of today's UTC date at the first read, then 1 ms later on every read; `Sleep(n)` adds n ms; `ALRUNNER_REAL_CLOCK=1` uses the wall clock | ADR-045, `al-testrun/src/host.rs` `run_clock_start`, `al-vm/src/hostvalues.rs` |
| Work date | Today, or what `LogInManagement.GetDefaultWorkDate()` answers in a test session with a client; reset for each test codeunit | `docs/probes-needed.md` row "WorkDate", `al-testrun` `install::login_work_date` |
| Random | .NET's `System.Random` with host seed 1 (`al_host::DEFAULT_RANDOM_SEED`) per test codeunit session; before every test method the runner runs codeunit 130301, which calls `LibraryRandom.SetSeed(1)` | `al-host/src/random.rs`, `docs/probes-needed.md` row "Random numbers", `docs/owner-review.md` 2026-10-19 per-test reset |
| Guids | `CreateGuid` and new SystemIds from a counter and a seed; the seed is the host's random seed | `al-vm/src/ctx.rs` `fresh_guid`, `fresh_system_id` |
| Identity | `UserId` `TESTUSER`, fixed `UserSecurityId`, `SessionId` and `ServiceInstanceId` 1, `TenantId` `default`, language 1033 | `docs/probes-needed.md` row "Identity", `al-host` `Identity` |
| Session settings | Language from `GlobalLanguage`, locale 1033, time zone `UTC` | `docs/probes-needed.md` row "SessionSettings" |
| Time zone | The runtime's time model is UTC; the host lists five zones | `al-host/src/lib.rs` `time_zone`, `docs/probes-needed.md` row "Time Zone" |
| Company | One company; `--companies` declares more (ADR-046) | `al-host` `Companies` |
| Test order | Test codeunits in parallel, each on a fresh session over the post-install database; methods in declaration order | `docs/manual/cli-reference.md` "Isolation and limits" |
| BC version, country | `--bc-version`, `--country`; lists or `--matrix` run one child process per cell (ADR-052); the corpus has one ratchet per major (`metrics/<major>/passing*.txt`) | ADR-052, ADR-015 |

So every fixed value above is a candidate for a perturbation. The question is which ones may change without
producing failures that real BC would never show.

## Decision

### 1. The rule for what may be perturbed

A perturbation is allowed only when both hold:

- **(a) Real BC can be in that state.** Two real BC runs, or two real environments, can differ in this value without
  any change to the apps or the test. Examples: the date the suite runs on, the seed `Any.SetDefaultSeed` draws from
  the clock, the user who runs the tests, the test order Microsoft's own stability mode uses.
- **(b) AL Runner behaves like BC in that state.** The runtime's behavior for the other value is pinned by the
  corpus, or does not depend on the value beyond what is pinned. Example: the corpus pins only that `SessionId` is
  positive, so any positive value is as correct as 1.

What BC defines deterministically is never perturbed: the sequence after `Randomize(n)` (pinned by
`types/TestRandomSeededContracts.al`), record order on a key, trigger order, event subscriber order (also across apps), error texts,
transaction behavior, AutoIncrement counters, `Format` results for a given language and region. Changing those makes
the runtime differ from BC (ADR-009), so a failure under them says nothing about the test.

A test that fails under an allowed perturbation can fail on real BC too, under the same conditions. That is the whole
value of the mode, so the rule is strict.

Each dimension below gets one of three classes:

- **safe**: (a) and (b) hold today; built in the first step.
- **pin first**: (a) holds, (b) does not yet. The dimension is accepted in the config only after the corpus pins
  the behavior (hard rule 3). Until then the config is refused with the name of the missing pin.
- **never**: (a) does not hold. The config refuses the name.

### 2. The dimensions

Perturbation axes run inside one compile (section 7). Matrix axes change the compiled apps or the database and run
as separate cells (section 3).

| Dimension (config name) | What changes | Class | Notes |
|---|---|---|---|
| `order.methods` | Test method order inside a codeunit: `declared`, `reverse`, `shuffle` (with a seed) | safe | Microsoft's mode reverses order. Codeunit globals and the database carry over between methods of one codeunit in BC too, so an order dependence found here is real. |
| `order.codeunits` | Test codeunit order: `declared`, `reverse`, `shuffle` | safe | Changes outcomes only with `--isolation disabled`, where codeunits see what earlier ones left. Under codeunit isolation every codeunit starts from the post-install database, so the axis is accepted and the report says it cannot change anything. |
| `isolation.alone` | Each test method runs as the only selected test of its codeunit (as `--run-selected-only` with one test), on a fresh session | safe | Microsoft's "one by one". The codeunit's `OnRun` and setup run as they would for one selected test in BC's test tool. |
| `random.librarySeed` | The seed of the runner's reset step: after codeunit 130301 runs, the runner calls Microsoft's own `LibraryRandom.SetSeed(n)` with the variant's n | safe | Microsoft's "different seeds for Any". The runner's steps around a test are runner behavior (ADR-013), not platform behavior, and Microsoft's runner will do the same. Only when the app that holds `Library - Random` is loaded; otherwise the axis has no effect and the report says so. |
| `random.hostSeed` | The seed of the session's `Random` before AL calls `Randomize` | safe | BC seeds from the clock (row "Random numbers"); AL sees only the range. |
| `guid.seed` | The seed of `CreateGuid` and new SystemIds | safe | Today tied to the random seed; it becomes its own host value. BC's Guids are random. The SystemIds of restored test data do not change. |
| `clock.date` | The date of the run clock: an offset (`+1Y`, `-3M`, `+45D`) or a fixed date (`2028-02-29`) | safe | A suite can run on any day. Moves `Today`, `CurrentDateTime`, the default work date (when it comes from today) and system stamps. |
| `clock.timeOfDay` | The time of day the run clock starts at, for example `23:59:59.000` | safe | ADR-045 chose noon so that the date does not change during a run; this axis tests the opposite, including `Any.SetDefaultSeed` and `LibraryRandom.Init` seeds that come from `Time`. |
| `clock.stepMs` | Milliseconds the clock moves per read: a fixed number or a range | safe | ADR-045: a test can only rely on "later than". 0 is refused (two reads must differ). |
| `workDate.offset` | The work date each test codeunit session starts with, relative to the login work date | safe | Microsoft's "work date one year later". Applied where the login work date is set today, so `WorkDate(d)` in the test still wins. |
| `identity.userId` | `UserId` and the session user's User row | safe for plain names (letters, digits, up to 50 characters); pin first for names with `\` or spaces | Any user can run tests. The User row is created with the same name, so lookups match. |
| `identity.userSecurityId`, `identity.sessionId`, `identity.serviceInstanceId`, `identity.tenantId` | The values in the row "Identity" | safe within the pinned ranges (positive ids, non-empty tenant) | The corpus pins only sign and non-empty. |
| `company.name` | The session company's name (no test data) | safe for names up to 30 characters of letters, digits and spaces; pin first for names with filter characters (`&`, `|`, `*`, `'`, `@`) | A test that writes `CompanyName` into a filter can fail on real BC with such a name; that needs a pin of how BC's own code handles it. With `--test-data` the restored company keeps its name (the data was built for it). |
| `numberSeries.offset` | Before the tests, every No. Series Line of the session company gets its last used number moved by N | safe, opt-in only | Real companies are at different numbers. A failure means the test assumed the demo company's numbers, which can be intended; the report labels this axis as "data" so a flip is read with that in mind. Not in any preset. |
| `data.setup` | A user-supplied AL codeunit that runs after the install baseline and before the tests (for example to add 10 000 items) | safe, opt-in only | The rows come from the user's AL code through the normal triggers, so no state BC could not reach. AL Runner never generates rows itself (see "never"). |
| `session.timeZone` | The session time zone (`SessionSettings`, `Format` of a DateTime) | pin first | The runtime's time model is UTC. Needs pins of `Format` and `Evaluate` of DateTime in other zones, and the 144-zone list (row "Time Zone"). |
| `session.language`, `session.region` | `GlobalLanguage`, `WindowsLanguage`, the locale | pin first | Translated reads are refused today (ADR-053, deferred); `Format` of numbers and dates in other regions needs pins. |
| `events.subscriberOrder` | Order of subscribers to one event across apps | never | The corpus pins the order across apps as fixed: ascending app id, dependencies first (runner pull request 6202, "Order cross-app event subscribers by app id"). Changing it would make the runtime differ from BC. |
| `environment` | `saas` or `onprem` (`--environment`) | safe as a matrix axis | Same compile; values of `Environment Information`. The on-prem values are an unprobed assumption today, so passes are flagged (section 6). |
| `bcVersion` | The BC major or build | matrix axis | Changes the compiled Microsoft apps and the gates (ADR-015). |
| `country` | The country version | matrix axis | Changes the compiled Microsoft apps and the test data. |
| `companies` | The companies of the database (ADR-046) | matrix axis | Changes the install baseline. |
| Synthetic rows written by the runner, `Randomize(n)` sequences, record order, trigger order, error texts, `Format` in a fixed language, rounding, AutoIncrement start values, transaction and commit behavior | | never | BC defines these. A config that names one is refused with exit 2. |

The list is open: a new dimension is added by a change to this table (class and reason) and to the registry of
dimensions in code. A dimension in class "pin first" moves to "safe" in the pull request that lands its corpus pin.

### 3. The config file

JSON, read with `--stability FILE` (also a preset name: `--stability microsoft`). One file describes the matrix axes,
the perturbation axes, the strategy and the selection.

```json
{
  "format": 1,
  "matrix": {
    "bcVersion": ["27.5", "28.5", "29.0"],
    "country": ["w1"],
    "environment": ["saas"]
  },
  "dimensions": {
    "order.methods": ["declared", "reverse"],
    "isolation.alone": [false, true],
    "random.librarySeed": { "range": [1, 1000000], "samples": 3 },
    "workDate.offset": ["+0D", "+1Y"],
    "clock.timeOfDay": ["12:00:00.000", "23:59:59.500"]
  },
  "strategy": "oneAtATime",
  "masterSeed": 4711,
  "runs": 1,
  "selection": { "codeunits": [134000, 134001], "tests": [], "excludeTests": [] }
}
```

- **Values.** A dimension takes a list of values, or a range (`range`, `samples`) for numbers and dates. Each
  dimension has a default value: today's fixed value from the table in Context. The default is always the first
  variant, the "base variant".
- **Strategy.**
  - `oneAtATime` (default): the base variant, then one variant per non-default value of each dimension, all other
    dimensions at their default. Variants = 1 + sum of (values - 1). Every flip then names one dimension.
  - `matrix`: every combination. The runner prints the count before it starts and refuses more than 1 000 variants
    without `--stability-max N`.
  - `sample`: `runs` variants, each dimension's value drawn from its values or range with a generator seeded with
    `masterSeed`. Without `masterSeed` the runner picks one from the wall clock and prints it, so the run can be
    repeated.
- **runs.** With `sample`, the number of variants. With the other strategies, how many times the whole set runs, each
  time with sampled seeds for the `range` dimensions (derived from `masterSeed` and the run number).
- **Selection.** The same fields as the command line (`--codeunit`, `--test`, `--exclude-test`); command-line
  options win over the file.
- **Matrix.** Each matrix cell runs the whole perturbation set. Cells are ADR-052's cells: one child process each,
  `--matrix-parallel` and `--matrix-memory` apply. `bcVersion` and `country` in the file are refused together with
  `--bc-version`/`--country` lists or `--matrix` on the command line.
- **Unknown names, a "never" dimension, or a "pin first" dimension** are refused with exit 2 before anything compiles,
  with the reason from the table.

**Built-in preset `microsoft`.** Microsoft's four, as `oneAtATime`:

```json
{
  "format": 1,
  "dimensions": {
    "random.librarySeed": { "range": [2, 2147483647], "samples": 1 },
    "isolation.alone": [false, true],
    "order.methods": ["declared", "reverse"],
    "workDate.offset": ["+0D", "+1Y"]
  },
  "strategy": "oneAtATime"
}
```

That is five variants: base, another library seed, each test alone, reverse order, work date plus one year.
`--stability-print-preset microsoft` writes it out as a starting point for a custom file.

**The existing runs in the same model.** The corpus runs per major and the BCApps runs per country are a config with
matrix axes only and no perturbation dimensions, so one variant per cell:

```json
{ "format": 1, "matrix": { "bcVersion": ["27.5", "28.5", "29.0"] } }
```

`--bc-version 27.5,28.5 --country w1,us` keeps working and means the same as that config. The ratchet files
`metrics/<major>/passing*.txt` stay the result of the base variant of each cell, as today; a stability run never
writes them (`--update-baseline` is refused with `--stability`). What differs between the two kinds of axis:

- A matrix axis (version, country, companies) changes the compiled apps, the install baseline or the test data. Its
  cells cannot share a compile, so each is a child process with its own compile cache entry (ADR-052).
- A perturbation axis changes only host values and runner steps. All its variants share one compile and one install
  baseline inside one process (section 7).
- `environment` is a matrix axis in the config (it changes `Environment Information`, which some install code reads),
  but it shares the compile: cells that differ only in `environment` run in one child process, each with its own
  install baseline.

### 4. Reproducibility

- Each variant gets an id: `<cell>/<n>` with n counted from 0 for the base variant, for example `29.0-w1/3`. The run
  prints, before the first variant and in the report, one line per variant with its id and its full value of every
  changed dimension:
  `29.0-w1/3  order.methods=reverse` or `29.0-w1/7  random.librarySeed=811203 workDate.offset=+1Y`.
- The report also holds the master seed, the runtime build and the config file's hash.
- `--stability-replay ID` with the same config reruns one variant alone and prints its tests as a normal run. Combined
  with `--test`, it reruns one test of one variant. `--stability-replay` also accepts the printed values directly
  (`--stability-replay "order.methods=reverse"`) so a variant can be run without the config file.
- A replay gives the same outcome as the variant in the full run: the runtime is deterministic (ADR-045, ADR-057
  section 4), and each variant starts from the same install baseline. With `isolation.alone` or a shuffled order,
  the replay of one test runs the same tests before it in the same order. A difference is a runtime bug.
- `ALRUNNER_REAL_CLOCK=1` is refused with `--stability`: the wall clock breaks replay.
- Amended by ADR-061 section 5 (2026-10-30): the variant id includes the run seed (`29.0-w1/3@4271833`), and
  `--stability-replay` with such an id sets the run seed from it. A config without `masterSeed` uses the run seed as
  its master seed, so a stability run prints one seed. The defaults of `random.hostSeed`, `guid.seed` and
  `random.librarySeed` are the values derived from the run seed; a variant's own value replaces them.

### 5. Output

- **Console.** After all variants, a table: test, base outcome, and each variant where its outcome differs
  (`pass -> fail`, `fail -> pass`, `pass -> not run`). Tests whose outcome is the same in every variant are counted,
  not listed. Then the variants with the most flips.
- **Minimal flipping perturbation.** For each flipped test from a `matrix` or `sample` variant that changed more than
  one dimension, the runner reruns that test with subsets of the changed dimensions (one dimension at a time first,
  then pairs) and reports the smallest set that still flips it, for example
  `Codeunit134000.PostSalesOrder flips with: workDate.offset=+1Y`. Capped at 20 reruns per test; past the cap the
  report says "not reduced". `oneAtATime` variants are minimal already.
- **Exit code.** 0 when no test changed outcome between variants, 1 when one did (a flip is a finding), else the
  normal exit codes of the base variant. `--stability-allow-flips` keeps 0 for reporting-only runs.
- **JUnit** (`--junit FILE`): one `testsuite` per variant, named with the variant id; the variant's dimension values
  as `properties`. A test that flipped gets a `failure` in the variant where it flipped, with the message
  `outcome changed from pass under <id>: <values>` plus the original error.
- **JSON** (`--output-json`): the version 2 shape for the base variant, plus a `stability` object:

```json
{
  "stability": {
    "format": 1,
    "config": "sha256:...",
    "masterSeed": 4711,
    "variants": [
      { "id": "29.0-w1/0", "cell": "29.0-w1", "values": {} },
      { "id": "29.0-w1/3", "cell": "29.0-w1", "values": { "order.methods": "reverse" } }
    ],
    "flips": [
      {
        "test": "Codeunit134000.PostSalesOrder",
        "base": "pass",
        "variant": "29.0-w1/3",
        "outcome": "fail",
        "message": "...",
        "minimal": { "order.methods": "reverse" },
        "dataAxis": false
      }
    ],
    "summary": { "variants": 5, "tests": 812, "stable": 806, "flipped": 6 }
  }
}
```

### 6. Interaction with other features

- **Refusals.** A test refused in the base variant is refused in every variant, and no perturbation can turn a refusal
  into a pass. A test refused in one variant only (for example a work date that reaches a refused closing date) is
  reported as `pass -> not run`, never counted as a stable pass. A "not run" in any variant is not a flip finding by
  itself; it is listed apart.
- **Flagged passes and assumptions.** A pass that ran through an unprobed assumption stays flagged in every variant. A
  flip that only happens in a variant where an assumption ran (and not in the base variant) is marked
  `relies on an unprobed assumption`, because the runtime may be wrong there, not the test.
- **`--affected` (ADR-050).** Allowed. The selection is computed once against the stored result of the base variant;
  the selected codeunits run in every variant. Only the base variant writes the stored result, so a stability run
  does not change what the next `--affected` run selects. Skipped codeunits are not perturbed and are reported as
  skipped.
- **Coverage.** `--coverage` records the base variant only. Coverage across variants is a later option (open
  question 5).
- **`serve` (ADR-028).** A `runTests` request gets a `stability` field with the config (or a preset name), restricted
  to perturbation axes: serve keeps refusing matrix axes, as it refuses `--matrix` today. Variants stream as `test`
  lines with a `variant` field; the summary holds the `stability` object. The warm slot's compile and install baseline
  are shared by all variants. Cancel stops after the current test of the current variant.
- **`--tdd` (ADR-051).** Allowed; stubbed tests stay flagged as stubbed in every variant.
- **Debugging (ADR-057).** `debug` accepts a variant id, so a flipped test can be debugged in the variant where it
  flipped. Step back works unchanged, because each variant is deterministic.
- **Impact runs (ADR-043, ADR-052 `--affected-from`).** Not combined in the first step: refused with a clear message.

### 7. Performance

The runner is in-process. A variant must not compile again.

- **Matrix cells** compile once each, as today (ADR-052). This is the only place where a compile repeats.
- **Inside a cell** the program is compiled once and the install baseline is built once. Each variant runs its test
  codeunits on new sessions over the same post-install snapshot (the store snapshot is O(1), ADR-004), with the
  variant's host values (`Host` gets the variant's clock, seeds and identity) and runner steps (order, alone,
  library seed).
- **Install baseline and host values.** The install baseline is built under the base variant's host values. A
  dimension that install code can observe (`clock.date`, `identity.userId`, `company.name`, `guid.seed`) makes the
  runner build a separate baseline for that variant, kept in memory per distinct set of install-visible values. The
  existing rule of what install code can observe (`al-testrun/src/observe.rs`) decides which dimensions count;
  `tools/baseline_rule_check.sh` covers it.
- **`isolation.alone`** runs each test on its own session; cost grows with the setup each codeunit does before its
  tests, not with compile.
- **Threads.** Variants run one after another; inside a variant the codeunits run in parallel as today. Variants
  could run in parallel too, but memory grows with each live store; this is left for measurement.
- **Budget.** A `oneAtATime` run with the `microsoft` preset should cost about 5 times the test phase of a normal run
  plus one compile. A new perf test measures it on the corpus (instructions, ignored test like the other budgets).

## Consequences

- A test author can find order, date, seed and identity dependencies locally in seconds, before Microsoft's mode
  runs in their pipeline, and can rerun one failing variant exactly.
- One config model covers the existing version and country matrix and the new perturbations. `--bc-version` lists,
  `--country` lists and `--matrix` stay as short forms.
- New host values (`guid.seed` apart from the random seed, a configurable clock step and start time, configurable
  identity) become seams of `al-host`. These are additive (defaulted `Host` fields), so no further ADR is needed.
- The table in section 2 becomes a list the project keeps current. Each "pin first" row is a corpus probe to queue
  (`docs/probes-needed.md` gets rows for time zone, language and region, filter characters in
  company and user names).
- Risk: a flip under a dimension where the runtime is wrong reads as a test defect. The rule in section 1 and the
  assumption marking in section 6 limit that; a reported flip that real BC does not show is a runtime bug with a
  corpus probe, as any other.

## Conformance

- Unit tests in `al-testrun` for the config parser: every "never" name and every "pin first" name is refused with its
  reason; unknown names are refused; the `microsoft` preset expands to five variants.
- A test that each dimension changes what AL code reads (`UserId`, `WorkDate`, `Today`, `Time`, the order of test
  method calls, the numbers of `Library - Random` after the reset step) and nothing else.
- A replay test: for each variant of a corpus run, `--stability-replay ID` gives the same outcomes.
- A test that a refused test is refused in every variant and that a pass through an assumption stays flagged.
- A differential test: the base variant's outcomes equal a normal run's outcomes, on the corpus for each major.
- A perf test on the corpus for the `microsoft` preset (section 7).

## Owner decisions (2026-10-30)

The owner accepted the ADR with the recommended answer to each open question, and one change from review.

1. **Exit code on a flip.** Exit 1 when any test changes outcome between variants. `--stability-allow-flips` keeps 0
   for report-only runs.
2. **Library seed.** After codeunit 130301 runs, the runner calls Microsoft's own `LibraryRandom.SetSeed(n)` with the
   variant's seed. When Microsoft publishes how its mode sets the seed, the runner follows that exactly.
3. **Data axes.** `numberSeries.offset` and `data.setup` are opt-in only, labeled as data axes in the report, and never
   part of a preset.
4. **Corpus CI.** Corpus CI does not run the `microsoft` preset. It runs locally, and later in a weekly job.
5. **Coverage.** `--coverage` records the base variant only.
6. **Config format.** JSON.
7. **Change from review.** `events.subscriberOrder` is "never", not "pin first": the corpus pins the order of
   subscribers across apps as fixed (ascending app id, dependencies first).

## Decided without owner review (2026-10-30, phase 3: matrix axes)

Implementation choices inside sections 3 and 4; no seam changes.

- **Cell names.** `<major>.<minor>-<country>`, then `-companies<n>` (the n-th value of the `companies` axis, from 1)
  and `-<environment>` when the config names those axes, for example `29.0-w1-companies2-onprem/3@4271833`. A config
  without those axes keeps the phase 1 ids.
- **`companies` values.** `null` (one company, the default), `"all"`, or a list of names (the session company first),
  each passed to the cell as `--companies`. The cell runs as a multi-company database does under ADR-046.
- **Command line and config.** An axis the config names cannot be combined with its own option (`bcVersion` with
  `--bc-version` or `--matrix`, `country` with `--country` or `--matrix`, `environment` with `--environment`,
  `companies` with `--companies`); a config without version or country axes takes the cells of a version or country
  list on the command line, as section 3 says.
- **Replay.** A variant id of a matrix config names its cell; the replay runs that cell alone in one process.
- **Report.** Each cell is compared with its own base variant. The matrix report adds a table of the stability cells
  and the flips within each; differences between the base variants of cells stay in ADR-052's "Differs between cells"
  (not computed between the environment cells of one process).

## Amendment 1 (Accepted, owner 2026-11-09): order dependence through SingleInstance state

- Status: Accepted (owner, 2026-11-09; decision 9 changed by the owner: bare `--stability` runs every safe dimension). Owner request (2026-11-09): stability mode should find the test codeunits that depend on the
  order they run in, because of SingleInstance state that earlier codeunits left, and report it so that teams can fix
  their tests. It adds one dimension and two parts of the report. It changes no default result.
- Why an amendment to this ADR, and not a new ADR or an ADR-066 amendment: the feature is part of stability mode (a
  variant, flips against the base variant, a report and exit code 1 on a flip). ADR-066 decides what the default run
  does, and that does not change here. This amendment only reads what ADR-066's amendments 3, 7 and 8 already track.
- Related: ADR-066 amendments 2 to 8 (SingleInstance state across test codeunits, the carry chain walk), hard rule 6
  and ADR-009 (match BC where it is observable).

### What BC does, and why the default does not change

1. **BC cannot reset SingleInstance state.** Within one test runner run, a SingleInstance codeunit's globals that
   test codeunit A left are still there in test codeunit B (P7, `al-language/codeunits/TestCuP7ASetter.al:24-58`,
   `TestCuP7BReader.al:24-37`; the same on BC 27.5, 28.5 and 29.0). Test isolation (`TestIsolation`,
   `RequiredTestIsolation`) rolls back the database, not SingleInstance globals. No setting, attribute or runner
   option of BC starts a codeunit with fresh SingleInstance state inside one session.
2. **The default stays BC-exact.** The carry chain walk (ADR-066 amendment 8) gives every chain member the
   SingleInstance state the members before it left, as BC's single session does. The other codeunits keep ADR-066
   amendments 4, 5 and 7. Changing that default would report outcomes BC does not give, against hard rule 6.
3. **Why teams still need this.** Many of these tests are broken: they pass only because another codeunit ran first.
   The same test fails on BC as soon as the codeunit starts with fresh SingleInstance state, and that happens in real
   runs: the codeunit run alone (a developer runs one codeunit from the test tool), another test selection
   (`--affected`, a pipeline that runs only changed tests), another order (a new codeunit with a lower id), or a run per
   app or per test suite (ALOps, ADR-066 amendment 6). Some of the caches sit in dependency apps the team does not
   own (Sequence No. Mgt. in Base Application, ADR-066 amendment 3). BC itself gives no way to see which codeunit
   read which SingleInstance global and who wrote it last. al-runner already tracks exactly that.

### What stability mode cannot show today (checked)

- `orderDependent` (ADR-066 amendment 4, decision 5) lists only codeunits outside the carry chain that the analysis
  found dependent. Since amendment 8 chain members are never flagged, so they are never listed (amendment 8, risk 4).
  On the stress suite below that is 789 of the 821 chain members that read a global an earlier member wrote.
- No entry says which global, of which SingleInstance codeunit in which app, or which earlier codeunit wrote it.
- `isolation.alone` resets everything for each test method (session, codeunit globals, database). `order.codeunits`
  also moves rows of the database carry chain. Neither can say that a flip comes from SingleInstance state and from
  nothing else.

### What the base variant already knows per codeunit

| Codeunit | Base result comes from | Result with fresh SingleInstance state |
|---|---|---|
| Read no global an earlier codeunit of its BC session left with another value | its run | the same: nothing it read can differ |
| Outside the chain, dependent, re-run in the lane after a failure (ADR-066 amendments 4, 7) | the lane (BC's order) | already known: its parallel run started fresh |
| Outside the chain, dependent, passed in parallel (not re-run) | the parallel run (fresh state) | already known; but the BC-order result is not, unless the lane replays it (`--strict-order`) |
| Chain member that read a global an earlier member left (amendment 8) | the walk (BC's order) | unknown: it needs one more run |

### Decision

1. **New dimension `isolation.singleInstance`**, values `carried` (default; what BC does) and `fresh`. Class: safe.
   Under `fresh`, each test codeunit starts with fresh SingleInstance state: new globals, empty temporary tables and no
   manual bindings in every SingleInstance codeunit, of every app, including Microsoft's. Nothing else changes: the
   database start of each codeunit, the carry chain's rows, the install baseline, the order, seeds, clock and identity
   are those of the base variant. So a flip under this dimension says one thing: the test depends on SingleInstance
   state an earlier codeunit left. The dimension does not model a BC setting (there is none, see above). It models
   the real situations where a codeunit starts fresh: run alone, another selection, another order, a run per app or
   suite. The reset is per test codeunit, not per test method: methods of one codeunit still share SingleInstance
   state, as on BC in every one of those situations (`isolation.alone` covers per method).
2. **The variant reruns only suspects.** A suspect is a codeunit whose first read of a watched global got a value an
   earlier codeunit of its BC session left, where that value differs from what a fresh instance holds (the
   fingerprint comparison of ADR-066 amendment 2; a value without a comparable form counts as different). Every other
   codeunit takes its base result in the `fresh` variant: it read nothing that a reset would change.
   - A chain member that is a suspect runs once more, from the very database start the walk gave it (the carried
     starts of ADR-066 amendment 5, decision 5), with fresh SingleInstance state. What that run leaves is not handed
     to later members: each suspect is compared on its own, so a flip never comes from a changed row.
   - A dependent codeunit outside the chain costs no run: its parallel run started fresh.
   Rejected: the variant as a full run with fresh state everywhere. It cost less than the base on the stress suite
   (measured below), but there later chain members also start from rows that earlier fresh runs left, so a flip could
   come from the database and not from the SingleInstance state. A full fresh run is what `--test-suites` with one
   codeunit per suite already gives.
3. **The base variant replays every dependent codeunit outside the chain** when the config names this dimension
   (`--strict-order` for the base variant only). Without it, a dependent codeunit that passed in parallel has no
   BC-order result, and the base result of a stability run would be the fresh one (ADR-066 amendment 4, "Risk: a
   false pass"). With it, both sides of the comparison are known for every suspect, and the base variant is BC's
   order. A normal run does not change.
   Note (ADR-066 amendment 12, 2026-11-09): a normal run now gives every test codeunit fresh SingleInstance state by
   default, so the default run equals this dimension's `fresh` side. A stability run runs every variant in BC's order
   (as `--strict-order`), and with this dimension the base variant still replays every dependent codeunit
   (`RunOptions::replay_every_dependent`, the meaning `--strict-order` had before amendment 12).
   Amended by ADR-070, decision 7 (2026-11-10): the two sides of this dimension become two plain runs. `fresh` is the
   default isolated run and `carried` is the sequential `--strict-order` run (one thread per BC session); the suspect
   re-runs and `replay_every_dependent` go when ADR-070 step 3 lands. Until then this decision stands as written.
4. **Each flip under this dimension carries a reason**: the test, the outcome and message in each variant, and for
   each global the codeunit read first with a carried value: the SingleInstance codeunit (id, name, and the app that
   holds it, with publisher, so a cache in a dependency app is named as such), the global's name, and the earlier test
   codeunit that last wrote it (id and name). The data exists: `Found::dependent`, `Found::changed` and
   `Found::writers` outside the chain (ADR-066 amendments 3 and 7); in the walk, `KeptSingles` already holds the
   globals the members wrote and needs, as an additive change, the member that wrote each one last. A flip is labeled
   "passes only after earlier codeunits" (fails with fresh state) or "fails only after earlier codeunits" (passes with
   fresh state). A dependency app's SingleInstance codeunit is marked "in a dependency app": the team cannot change
   that code, but it can make its test set up what it reads.
5. **"Reads carried state" without a variant.** Every stability run (with or without this dimension) lists, from the
   base variant's tracking alone, the suspects whose outcome did not flip: same fields as decision 4, the base
   outcome, and "checked" (the dimension ran and the outcome stayed) or "not checked" (no `fresh` variant in the
   config). This list costs no run. It replaces `orderDependent`, which lists a subset of it without reasons; the JSON
   keeps `orderDependent` for one release, filled from the new list, then drops it.
6. **Report.**
   - Console: after the flips table, a section "Order dependence (SingleInstance state)" with two parts: "Breaks with
     fresh SingleInstance state" (the flips of decision 4, one block per test codeunit, its tests below it) and "Reads
     carried state, outcome unchanged" (decision 5, one line per codeunit, at most 50 lines, then "and N more"; the JSON
     holds all). Entries are grouped by the app of the SingleInstance codeunit, the team's own apps first.
   - JSON (`--output-json`, `stability` object, format stays 1, fields added): `flips[].reason` for flips under this
     dimension, and `readsCarried`:

```json
{
  "flips": [
    {
      "test": "Codeunit50120.NeedsEarlierState",
      "base": "pass", "variant": "29.0-w1/1", "outcome": "fail",
      "message": "SW 50120: no earlier writer of SW Single 2",
      "minimal": { "isolation.singleInstance": "fresh" },
      "reason": {
        "kind": "passesOnlyAfterEarlierCodeunits",
        "reads": [
          { "singleInstance": { "id": 50002, "name": "SW Single 2", "app": "Stress Suite", "publisher": "Example",
                                "dependencyApp": false },
            "global": "Counter",
            "lastWriter": { "id": 50102, "name": "SW Test 50102" } }
        ]
      }
    }
  ],
  "readsCarried": [
    { "codeunit": { "id": 50121, "name": "SW Test 50121" }, "outcome": "pass", "checked": true,
      "reads": [ { "singleInstance": { "...": "..." }, "global": "Counter", "lastWriter": { "...": "..." } } ] }
  ]
}
```

   - JUnit: the `fresh` variant's testsuite as today; a flipped test's `failure` message adds the reason in one line
     ("reads Counter of SW Single 2 (app Stress Suite) that SW Test 50102 left"). Each test of a codeunit in `readsCarried`
     gets the property `singleInstanceCarried` in the base variant's testsuite with the same line.
7. **Not in the `microsoft` preset; `--stability` without a value runs every safe dimension** (owner, 2026-11-09).
   The `microsoft` preset is Microsoft's own mode, and Microsoft's mode has no such check, so it stays Microsoft's
   four. A bare `--stability` (no file, no preset name) becomes the built-in preset `all`: every dimension of class
   safe, `oneAtATime`, including `isolation.singleInstance` (`--stability-print-preset all` writes it out). No
   separate `order` preset; a config file with only `{"isolation.singleInstance": ["carried", "fresh"]}` runs the
   check alone. Decision 5's list is in every stability run, also with the `microsoft` preset, since it costs nothing.
8. **Exit code** as owner decision 1 of this ADR: a flip under this dimension is a flip (exit 1; `--stability-allow-flips`
   keeps 0). A codeunit only in `readsCarried` does not change the exit code.
9. **No `serve` field.** Stability mode is not offered in `serve` (section 6 and the CLI reference). This amendment
   keeps that; `serve` gets the dimension when stability mode comes to `serve`.

### Edge cases

- **RequiredTestIsolation Disabled** (carry chain members): decision 2, a run from the carried database start with fresh
  SingleInstance state. **Codeunit and Function**: the codeunit runs outside the chain; its parallel run is already
  fresh, so it costs nothing. Function isolation is treated as Codeunit: the reset of this dimension is per test
  codeunit either way, and test isolation does not roll back SingleInstance globals (P7). The test isolation of the codeunit is printed in its report entry, so a team sees
  whether the database or only SingleInstance state crosses codeunits.
- **`--test-suites`**: SingleInstance state crosses only within a suite (ADR-066 amendment 6). A codeunit first in its
  suite is never a suspect, and the last writer is always in the same suite. With `--test-suites` the walk already
  starts each chain member fresh (amendment 8, decision 6), so only the lane's codeunits can be suspects.
- **A writer that is itself a suspect** (a chain of dependence: B reads what A left, C reads what B left): each suspect
  reruns alone from its own start and keeps the others' base state, so the report names the direct last writer only.
  A team that fixes B's test may still see C flip; the report lists both.
- **Fails in both variants**: no flip. If the message differs, the codeunit is listed in `readsCarried` with both
  messages ("fails either way, with another message"), because another message often means the test reached another
  line. If the message is the same, it is listed as unchanged.
- **Refused or not run in either variant**: never a flip (section 6: refusals never become passes). A codeunit with a
  test refused while it ran hands nothing on (ADR-066 amendment 3, R3), so later codeunits never read its state.
- **Flaky outcomes**: runs are deterministic (run seed, ADR-061; stepping clock, ADR-045), so the same start gives the
  same outcome. The one known source of noise is the per-test CPU time limit (ADR-066 amendment 5, `FiveThousandSerialNos`).
  A test stopped by the time limit in either variant is not compared and is listed as "not compared (time limit)".
  No second run to confirm a flip.
- **Combined with other dimensions**: `isolation.alone=true` already starts every test fresh, so the pair
  (`alone`, `fresh`) is the same run as `alone`; a `matrix` strategy runs it once. With `order.codeunits` the suspects
  are worked out in the variant's own order.

### Measured (prototype)

Prototype: a local build of main (not committed; a print in the walk names each chain member that read a global an
earlier member wrote, with its run time). Release build, 12 threads, cores 0-2 and 5-11, `--seed 1`, shared machine
(load average 15 to 30, so wall times carry noise). "Fresh, all" is a full run with one codeunit per suite
(`--test-suites`), which starts every codeunit fresh: an upper bound for decision 2 and a check of which tests flip.

| | Stress suite (synthetic, 868 test codeunits, 1616 tests) | Tests-SCM-Costing (BC 29.0.54011.55816, 43 test codeunits, 1002 tests) |
|---|---|---|
| Base run (today's default), wall / peak | 22.1 s / 2.0 GB | 218.7 s / 3.9 GB (compile cache built in this run, 9.8 s) |
| Carry chain members | 821 | 43 (all) |
| Chain members that read a global an earlier member wrote (suspects by the cruder rule of the prototype) | 789 | 36 |
| Dependent codeunits outside the chain (ADR-066 amendment 3) | 47, all already re-run in the lane | 0 |
| Base with `--strict-order` (decision 3) | 21.2 s, 0 outcome changes: no extra cost here | no dependent codeunit outside the chain: no extra cost |
| Summed run time of the suspect chain members in the walk (the reruns of decision 2, before threads) | 2.3 s | 171 s; the longest member 32.4 s, so about 32 s of wall time on 12 threads (about 15 % of the base run) |
| "Fresh, all" run (rejected variant shape), wall / peak | 7.8 s / 1.1 GB | 149.0 s / 4.9 GB |
| Tests whose outcome differs, base against "fresh, all" | 36 tests in 36 codeunits: pass in BC's order, fail fresh ("no earlier writer of SW Single N"); all are dependent codeunits outside the chain; no chain member flips | 0 flips; 16 tests of one codeunit (SCM Inventory Costing II) fail either way, with another entry number in the message (Entry No.=2348 against 2349) |

- The "fresh, all" run is faster than the base because the walk can use its runs ahead again when no member keeps
  SingleInstance state (ADR-066 amendment 8, accepted cost). On SCM-Costing 40 of 43 members ran ahead.
- The 16 changed messages on SCM-Costing come from a run where earlier members also ran fresh, so the database start of
  later members differs as well (an entry-number range of Sequence No. Mgt. is left otherwise). That is why decision 2
  reruns each suspect from the walk's own database start. Whether these 16 still differ under decision 2 is not
  measured; they would be listed under "fails either way, with another message", not as flips.
- The prototype's suspect rule is cruder than decision 2's: it counts any read of a global an earlier member wrote, not
  only a value that differs from a fresh instance. So 789 and 36 are upper bounds for the number of reruns. On the
  stress suite every one of the 789 passes in both runs: they belong in "reads carried state, outcome unchanged".
- Report size: on the stress suite the console part of decision 5 would list 789 + 11 codeunits, so the cap of 50 lines
  matters; the JSON holds all.

### Decisions for the owner

Each can be accepted or rejected on its own.

1. **Amend ADR-058**, not a new ADR or an ADR-066 amendment: the feature is part of stability mode; ADR-066's default
   run does not change.
2. **The default stays BC-exact.** The carry chain and ADR-066 amendments 4 to 8 are unchanged; this amendment only adds
   a stability dimension and report parts.
3. **Dimension `isolation.singleInstance` = `carried` | `fresh`**, class safe, reset per test codeunit (not per method),
   every app's SingleInstance codeunits, nothing else changed. Stated as modelling "run alone, another selection,
   another order, a run per app or suite", not a BC setting.
4. **The `fresh` variant reruns only suspects** (a first read of a carried value that differs from a fresh instance);
   chain members from the walk's own database start, nothing handed on; codeunits outside the chain cost nothing. A
   full fresh run is rejected.
5. **With this dimension the base variant replays every dependent codeunit outside the chain** (`--strict-order` for
   the base variant), so both sides are known.
6. **Each flip carries a reason**: SingleInstance codeunit with its app and publisher (dependency apps marked), the
   global(s), the codeunit that last wrote each, outcome and message in both variants, and the direction ("passes only
   after earlier codeunits" or "fails only after earlier codeunits"). Needs one additive field in the walk (last writer
   per global).
7. **"Reads carried state, outcome unchanged" in every stability run**, from the base variant's tracking, no extra run;
   marked "checked" or "not checked". It replaces `orderDependent` after one release.
8. **Report shape**: a console section with the two parts (at most 50 lines for the second), JSON `flips[].reason` and
   `readsCarried` in format 1, the JUnit property `singleInstanceCarried` and the reason in a flip's failure message.
9. **Not in the `microsoft` preset**; a bare `--stability` runs the preset `all` (every safe dimension, this one included).
10. **Exit code**: a flip under this dimension is exit 1, like every flip; `readsCarried` alone does not change it.
11. **No `serve` field** until stability mode comes to `serve`.
12. **Fail in both variants with another message**: listed under "reads carried state" as "fails either way, with
    another message", not a flip.
13. **No confirmation rerun for flaky outcomes**: runs are deterministic; a test stopped by the CPU time limit in either
    variant is "not compared (time limit)".

### Consequences

- A team gets, from one stability run, the tests that pass only because others ran first, with the SingleInstance
  codeunit, its app, the global and the codeunit that wrote it: what it needs to fix the test, and what BC cannot tell it.
- Default runs and every normal outcome stay BC-exact. The new list in decision 5 makes chain members visible again
  (amendment 8, risk 4) without running them again.
- Cost when the dimension is on: the base variant's lane replays every dependent codeunit (decision 3) plus one run per
  suspect chain member (decision 2).
- Risk: the suspect rule is only as good as the tracking. A link ADR-066 amendment 3 misses (a flag that guards a cache
  in another codeunit) is neither a suspect nor rerun; that codeunit keeps its base result in the `fresh` variant, so a
  real flip is missed, never invented.

### Conformance

- `al-testrun` config tests: `isolation.singleInstance` takes `carried` and `fresh` only; a bare `--stability` expands to the
  preset `all` (every safe dimension, `isolation.singleInstance` included); the `microsoft` preset is unchanged.
- `crates/al-testrun/tests/suite/single_instance_lane.rs` (or a stability suite module): the corpus P7 pair
  (`TestCuP7ASetter.al`, `TestCuP7BReader.al`) flips under `fresh` with a reason naming the SingleInstance codeunit,
  its global and P7A; a chain member reading a carried value flips and later members keep their base start; a codeunit
  that read nothing carried is not rerun; a fail-fail pair with another message is listed in `readsCarried`; a
  dependency app's SingleInstance codeunit is marked as such.
- `crates/alrunner-cli/tests/cli/onrun_stability.rs`: the console section, the JSON `reason` and `readsCarried`, the
  JUnit property, exit 1 on such a flip.
- A differential test: the base variant of a run with this dimension equals a run with `strict_order` and
  `replay_every_dependent` (before ADR-066 amendment 12: a normal `--strict-order` run).
