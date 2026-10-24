# ADR-053: The run seed (`--seed N`, `AL_RUNNER_SEED`)

- Status: Proposed
- Date: 2026-10-22

## Context

The v2 parity page lists `--seed N` as row SEL-9. The owner decided on 2026-10-22
that it is the one Open row needed to replace v2. ADR-016 gate R1 needs every
parity row built or deferred. Rows OUT-4 (the `Seed:` line in the summary), OUT-6
(the JUnit `seed` property) and ENV-2 (`AL_RUNNER_SEED`) wait on it too.

### What v2 does

From v2's `docs/run-seed.md` (branch `v2`):

- Every run has a run seed: `--seed N`, else `AL_RUNNER_SEED`, else a fresh one.
- Before each `[Test]` method, the session's `Random` is reset to
  `new Random(Derive(runSeed, codeunitId, methodName))`. `Derive` is 32-bit FNV-1a
  over `"{runSeed}|{codeunitId}|{methodName}"`.
- `Randomize(seed)` is honored as in BC. `Randomize()` with no seed is reseeded
  from the test's derived seed, with a warning on stderr.
- `CreateGuid()` is not covered and stays random. So text from `Any` and
  `Library - Random` (built from Guids) differs on every run.
- The seed is printed as `Seed:  N` in the summary, followed by a replay command
  for the first failure. It is the `seed` field of `--output-json` and a
  `<property name="seed">` on each JUnit `<testsuite>`.
- Replay: `al-runner <bundle> --seed N --test Name` gives the failing test the same
  numbers, alone or in the full suite.

### What AL Runner does today

AL Runner is already repeatable by default. There is no seed to set or print.

| Source | BC | AL Runner today |
|---|---|---|
| `Random` with no `Randomize` | .NET `System.Random`, seeded from the clock, one per session | .NET's legacy generator (`al-host/src/random.rs`), seeded with the host seed `DEFAULT_RANDOM_SEED = 1`. Each test codeunit has its own session, so each codeunit starts from seed 1. Tests of one codeunit share the sequence. |
| `Randomize(seed)` | `new Random(seed)` | The same, with .NET's numbers (corpus `types/TestRandomSeededContracts.al`). |
| `Randomize()` | `new Random()`, from the clock | Host seed plus a per-session count (`al-vm/src/hostvalues.rs`). |
| Microsoft's test libraries | Test Runner's codeunit 130301 calls `LibraryRandom.SetSeed(1)` (so `Randomize(1)`) before each test. `Any.SetDefaultSeed` and `LibraryRandom.Init` seed from `Time`. | 130301 runs before each test when it is loaded (owner-review, 2026-10-19). `Time` starts at 12:00:00 UTC (ADR-045), so seeds taken from `Time` repeat. |
| `CreateGuid`, `SystemId`, media and task ids | Random | A counter hashed with a fixed seed (`al-vm/src/ctx.rs`, `guid_for`). The counter is part of the install baseline. |
| `Today`, `WorkDate` | Wall clock | The wall clock's date. The time of day is fixed at noon and moves 1 ms per read (ADR-045). `ALRUNNER_REAL_CLOCK=1` turns this off. |
| Test order | Declaration order inside a codeunit | The same. Not random. |
| Threads | One session for the whole run | Each test codeunit runs in its own session, forked from the install baseline. A codeunit's outcome does not depend on which thread runs it or when. |

The runner also detects a test that draws numbers from a sequence an earlier
test of its codeunit moved (`inherited_random_draws` in
`al-testrun/src/runtime/run.rs`). It uses this when it decides whether a failure
can be trusted.

So the only things a seed can usefully change are the starting point of the
unseeded `Random` sequence, the `Randomize()` seeds, and maybe the Guid stream.
These are exactly what BC itself leaves to chance. Today's wall-clock date is a
separate input and is not part of the seed.

### The rule that limits the design (ADR-009)

The seed may change only values that BC leaves to chance: numbers from an
unseeded `Random`, the seed of `Randomize()`, and (if included) new Guids. It must
never change anything BC fixes:

- the numbers after `Randomize(seed)` (they must stay .NET's);
- test order, trigger and event order;
- the clock, the work date, identity values;
- anything AL code can see that does not come from one of the random sources above.

One case is not pinned: in BC, does a `Randomize(seed)` in one test still drive
`Random` in the next test of the same codeunit? BC keeps one session for the run,
so it probably does. v2 resets the generator before every test, so it says no.
A probe is needed before the runner may reset a generator that AL code seeded.

## Options

### Option 1: the run seed replaces the host seed

`--seed N` sets the host seed (today the constant 1). Each test codeunit session
starts its `Random` from `Derive(N, codeunitId)`, and `Randomize()` uses
`Derive(N, codeunitId) + count`. Nothing is reset between tests.

- Small change: one field of `al_host::Host` and a derivation.
- With the default seed equal to today's behavior, no current result changes.
- Replay works for a whole codeunit. Running one test alone with `--test` can give
  it other numbers when an earlier test of its codeunit drew numbers. v2 promised
  "alone or in the full suite", so this is weaker than v2.

### Option 2: v2's per-test derivation, limited to unseeded state (recommended)

As option 1, plus: before each test method, if the session's `Random` is still on
a runner-chosen seed (AL code has not called `Randomize(seed)` in this codeunit),
it is reset to `Derive(N, codeunitId, methodName)`. `Randomize()` with no seed is
reseeded from the test's derived seed plus a per-test count. A generator that AL
code seeded is left alone until the probe above says what BC does; if the probe
shows BC also resets it, the rule can follow.

- Replay of one failing test with `--seed N --test Name` gives the same numbers as
  in the full run, as in v2, for every test that does not depend on a seed AL code
  set in an earlier test.
- A test no longer depends on how many numbers earlier tests drew. The
  `inherited_random_draws` check then fires only after an AL-chosen seed.
- `Derive` is fixed and covered by golden-value tests, because a change makes
  every recorded seed useless. It may be v2's FNV-1a so a v2 seed means the same
  thing; `codeunitId` and `methodName` are the codeunit number and the method name
  as `--test` matches it.
- Unseeded numbers change once, when this is built, for tests that draw without
  Microsoft's 130301 reseed. The corpus does not pin those numbers. Microsoft's
  test buckets may move a little (a measurement run shows how much).

### Option 3: option 2, plus Guids, plus a fresh seed by default (v2's default)

As option 2, and the Guid stream is seeded from the run seed too, and a run
without `--seed` picks a new seed each time.

- Closest to v2's default. Finds tests that pass only for one set of numbers.
- Breaks today's repeatable default. `--affected` (ADR-050) and impact test
  selection (ADR-043) compare runs and need repeatable outcomes.
- Seeding Guids also changes the install baseline (its rows carry `SystemId`s), so
  the seed would join the compile-cache key, or Guids made before the first test
  would have to stay on the fixed stream.

## Decision (proposed)

Option 2, with these parts:

1. **Flag and variable.** `al-runner test --seed N` and `AL_RUNNER_SEED=N`. The flag
   wins. `N` is a 32-bit signed integer. `serve` takes `seed` in `runTests`.
2. **Default.** With neither set, the seed is a fixed documented value, so a run is
   repeatable as today. `--seed random` picks a fresh seed for runs that want v2's
   behavior (see open question 1).
3. **What it controls.** The starting point of `Random` in each test (as in option 2)
   and the seeds `Randomize()` uses. Nothing else. `Randomize(seed)` keeps .NET's
   numbers. Guids stay on today's fixed stream (see open question 2). Clock, date,
   test order and threads are not touched.
4. **Output.** The summary prints `Seed: N` (OUT-4), and after a failure one replay
   line: `al-runner test ... --seed N --test Name`. `--output-json` gets a top-level
   `seed` field (as v2, for LethAL and ALchemist). Each JUnit `<testsuite>` gets
   `<property name="seed" value="N"/>` (OUT-6). Under `--quiet` the seed line still
   prints when a test failed.
5. **Replay.** Same seed, same apps and same AL Runner version give each test the same
   numbers, in a full run or alone. The replay line says so. A test that reads a
   seed set by AL code in an earlier test replays only in the full codeunit. A
   different AL Runner version may give other numbers only if `Derive` changes,
   which the golden-value tests prevent.
6. **Child processes.** Matrix cells (ADR-052) and other child processes get the
   resolved seed through `AL_RUNNER_SEED`, so every cell runs with the same seed.
7. **Cache.** The seed is not part of the compile-cache key. The install baseline
   runs on the fixed default seed, because install code is not a test and the
   baseline is shared between runs.
8. **Affected and impact runs.** `--affected` stores the seed with the result. A run
   with another seed is a full run with the reason "the seed changed".

## Consequences

- SEL-9, OUT-4, ENV-2 and the seed part of OUT-6 become Done when this is built.
  The migration guides change their `--seed` rows.
- Unseeded random numbers change once, for tests that do not reseed. The corpus
  ratchet is not affected (it pins only seeded numbers). Microsoft test results
  need one comparison run.
- Text from `Any` and `Library - Random` is already repeatable in AL Runner (Guids
  are repeatable), which is better than v2. A fresh seed does not change it.
- `docs/probes-needed.md` keeps its "Random numbers" row, reworded for the run seed.
  One new probe row: whether a `Randomize(seed)` in one test reaches the next.

## Open questions for the owner

1. **Default seed.** Should a run without `--seed` use a fixed seed (repeatable, as
   today) or a fresh seed each run (as v2)?
2. **Guids.** Should `--seed` also change the `CreateGuid` and `SystemId` values,
   or only the random numbers (as v2)?
3. **Derivation.** Should `Derive` be v2's FNV-1a formula, so a v2 seed gives the
   same per-test seeds, or our own formula?
4. **Seeded state across tests.** Until the probe answers it, should the runner
   leave a generator that AL code seeded alone (this ADR), or reset it before each
   test as v2 does?
5. **Shuffled order.** v2 never shuffled test order and BC does not either. Should a
   seed-driven random test order stay out of scope (this ADR), or be a later
   opt-in tool?

## Conformance

- Golden-value tests for `Derive` (fixed seeds to fixed per-test seeds).
- A test that a full run and a `--test Name` run with the same seed give one test
  the same `Random` numbers.
- A test that `Randomize(seed)` still gives the numbers of
  `types/TestRandomSeededContracts.al` under every `--seed`.
- A test that two runs with the same seed give the same `--output-json` apart from
  times, and that the `Seed:` line, the JSON `seed` field and the JUnit property
  carry the seed.
- A corpus probe for a `Randomize(seed)` in one test followed by `Random` in the
  next test of the same codeunit, before item 4 of the open questions is settled.
