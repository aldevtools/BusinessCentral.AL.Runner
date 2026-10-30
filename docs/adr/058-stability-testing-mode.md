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
