---
title: "Stability mode"
weight: 55
---

Stability mode runs your tests again under changes that real Business Central can
also show. A test that passes in the normal run and fails in a changed run depends
on something it should not depend on: an earlier test, a fixed seed, today's date.

It is built on pull requests that were not merged when this page was written. If
`al-runner test --help` does not list `--stability`, your build does not have it yet.

## Run it

```bash
al-runner test ./MyApp ./MyApp.Test --stability microsoft
```

The preset `microsoft` has five variants:

1. The base variant: a normal run.
2. Another seed for `Library - Random`.
3. Each test alone, on a fresh session.
4. The test methods of each codeunit in reverse order.
5. The work date one year later.

All variants share one compile. A run costs about five times the test phase of a
normal run, plus one compile.

`al-runner test --stability-print-preset microsoft` prints the preset as JSON. Save
it as a starting point for your own file, then run `--stability my-config.json`.

## The config file

```json
{
  "format": 1,
  "dimensions": {
    "order.methods": ["declared", "reverse", "shuffle:7"],
    "isolation.alone": [false, true],
    "random.librarySeed": { "range": [2, 1000000], "samples": 3 },
    "workDate.offset": ["+0D", "+1Y"],
    "clock.date": ["+0D", "2028-02-29"],
    "clock.timeOfDay": ["12:00:00.000", "23:59:59.500"]
  },
  "strategy": "oneAtATime",
  "masterSeed": 4711,
  "runs": 1,
  "selection": { "codeunits": [50100], "tests": [], "excludeTests": [] }
}
```

- **dimensions.** Each name is one thing that changes. Its value is a list, or a
  range of numbers (`range` and `samples`). The first value should be the normal
  one. It is the base variant.
- **strategy.**
  - `oneAtATime` (default): the base variant, then one variant for each other value
    of each dimension, with all other dimensions at their normal value. A flip then
    names one dimension.
  - `matrix`: every combination. The runner prints how many before it starts and
    stops above 1000 unless you pass `--stability-max N`.
  - `sample`: `runs` variants, with values drawn at random.
- **masterSeed.** Seeds the random draws. Without it, the runner picks one from the
  clock and prints it, so you can repeat the run.
- **selection.** The same choices as `--codeunit`, `--filter` and `--exclude-test`.
  The command line wins over the file.

### Dimensions

| Dimension | What changes |
|---|---|
| `order.methods` | Order of the test methods in a codeunit: `declared`, `reverse`, `shuffle` or `shuffle:SEED` |
| `order.codeunits` | Order of the test codeunits. It changes results only with `--isolation disabled`. Otherwise every codeunit starts from the same database and the report says so. |
| `isolation.alone` | `true` runs each test alone, on a fresh session |
| `random.librarySeed` | The seed `Library - Random` gets before each test |
| `random.hostSeed` | The seed of the session's `Random` before AL calls `Randomize` |
| `guid.seed` | The seed of `CreateGuid` and new SystemIds |
| `workDate.offset` | The work date each test codeunit starts with, such as `+1Y`, `-3M`, `+45D` |
| `clock.date` | The date of the run clock: an offset or a fixed date such as `2028-02-29` |
| `clock.timeOfDay` | The time of day the clock starts at. A normal run starts at noon. |
| `clock.stepMs` | Milliseconds the clock moves per read. It must be above 0. |
| `identity.userId` | `UserId`. Letters and digits, up to 50 characters. |
| `identity.userSecurityId`, `identity.sessionId`, `identity.serviceInstanceId`, `identity.tenantId` | The other identity values |
| `company.name` | The session company's name: letters, digits and spaces, up to 30 characters. With test data, the restored company keeps its name. |

### Which changes are allowed

Every dimension has a class. The class answers one question: could real Business
Central be in this state?

- **safe.** Yes, and AL Runner behaves like Business Central there. The dimensions
  above are safe.
- **pin first.** Yes, but AL Runner's behavior there is not measured yet. Time zone,
  language and region, and user or company names with special characters are in
  this class. The config is refused with the name of the missing measurement.
- **never.** No. Business Central defines the value: the sequence after
  `Randomize(n)`, record order, trigger order, the order of event subscribers,
  error texts, rounding. A failure under such a change would say nothing about your
  test. The config is refused with the reason.

A config with an unknown name, a "never" or "pin first" dimension, or a dimension
this build has not built yet stops with exit 2 before anything compiles.

## Read the report

After the last variant the runner prints a report like this one (the exact wording of the base line can differ):

```text
Stability (microsoft, 5 variants):
  29.0-w1/0  base
  29.0-w1/1  random.librarySeed=811203
  29.0-w1/2  isolation.alone=true
  29.0-w1/3  order.methods=reverse
  29.0-w1/4  workDate.offset=+1Y

Tests whose outcome changed:
  Codeunit50100.PostInvoice  (base: pass)
    pass -> fail under 29.0-w1/2  isolation.alone=true
      Item 'A' does not exist.

Variants with the most flips:
     1  29.0-w1/2  isolation.alone=true

Stability: 120 tests, 119 stable, 1 flipped, 0 changed to or from not run; master seed 4711, config sha256:...
```

How to read it:

- **Variant ids** have the shape `<BC version>-<country>/<number>`. Number 0 is the
  base variant. The line next to the id lists every value that changed.
- **A flip** is a test that passes in the base variant and fails in another one. The
  line names the variant and the dimension. With `oneAtATime`, that dimension is the
  cause.
  - `isolation.alone`: the test needs data an earlier test created.
  - `order.methods`: a test depends on the order of the others.
  - `workDate.offset` or `clock.date`: the test assumes today's date.
  - `random.librarySeed`: the test assumes a particular random value.
- **Changed to or from not run** is listed apart. It is not a finding by itself. A
  test refused in the base variant is refused in every variant.
- A flip that only happens where an unprobed assumption ran is marked "relies on an
  unprobed assumption". The runtime may be wrong there, not the test.

## Repeat one variant

Run one variant as a normal run, with its id and the same config:

```bash
al-runner test ./MyApp ./MyApp.Test --stability my-config.json --stability-replay 29.0-w1/2
```

Or give the printed values, without a config:

```bash
al-runner test ./MyApp ./MyApp.Test --stability-replay "order.methods=reverse workDate.offset=+1Y"
```

Add `--filter` to repeat one test. The result is the same as in the full run, because
the runtime is deterministic.

## Exit codes and output

- Exit 1 when a test changes between pass and fail. `--stability-allow-flips` keeps
  the exit code of the base variant, for reports only.
- `--junit FILE` writes one test suite per variant. The variant's values are its
  properties. A flipped test fails with `outcome changed from pass under <id>`.
- `--output-json FILE` adds a `stability` object with the variants, the flips and
  the summary.

## Limits

- Not with `--affected`, `--matrix`, version or country lists, `--recipe`,
  `--explain`, `serve` or `ALRUNNER_REAL_CLOCK=1`.
- Version, country and environment as matrix axes in the same file are planned.
  Use `--bc-version` and `--country` lists for those today.
