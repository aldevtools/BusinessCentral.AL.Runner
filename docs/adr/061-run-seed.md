# ADR-061: Run seed and `--seed`

- Status: Accepted (owner, 2026-10-30)
- Date: 2026-10-30
- Related: ADR-006 (no OS access in core crates), ADR-009 (match BC where observable), ADR-013 (the runner's steps
  around each test), ADR-028 (`serve`), ADR-045 (stepping test clock), ADR-050 (`--affected`), ADR-057 (debugging
  API), ADR-058 (stability mode)

## Context

Today every run draws the same "random" values:

- The session's `Random` starts from the host seed `al_host::DEFAULT_RANDOM_SEED = 1` (`crates/al-host/src/lib.rs`).
  The generator is .NET's `System.Random` (`crates/al-host/src/random.rs`).
- `Randomize()` without a seed reseeds from the host seed plus a count (`crates/al-vm/src/hostvalues.rs`).
- `CreateGuid` and new SystemIds come from a counter and the host seed (`crates/al-vm/src/ctx.rs`, `fresh_guid`).
- Before each test method, when the app that holds `Library - Random` is loaded, the runner runs codeunit 130301,
  which calls `LibraryRandom.SetSeed(1)`.

Nothing about the seed is printed. `docs/manual/migrating-from-v2.md` and `docs/manual/roadmap.md` say that version 3
has no `--seed`.

AL Runner v2 had a run seed (`docs/run-seed.md` in v2, parity row SEL-9 in `docs/al-runner-v2-parity.md`):

- Without `--seed`, the run seed was `AL_RUNNER_SEED` or a new random number.
- Before each test method, the session's `Random` was reset to `new Random(Derive(runSeed, codeunitId, methodName))`.
  `Derive` is FNV-1a (32 bit) over `"{runSeed}|{codeunitId}|{methodName}"`.
- `Randomize(n)` was honored as in BC. `Randomize()` reseeded from the test's derived seed.
- `CreateGuid` stayed random, so text from `Any` and `Library - Random` built from Guids differed on every run.
- The seed was printed in the summary (`Seed:  N`, with a replay command for the first failure), as `seed` in
  `--output-json`, and as a `seed` property on every JUnit `testsuite`.

Owner decisions:

- 2026-10-22 (v2 parity scope): of the open v2 parity rows, only `--seed` matters.
- 2026-10-30: "If a run is not specified with an explicit seed, the seed that was used ... gets printed by default
  ... important for CI and local runs ... not related to the stability mode ... like v2: a standard run prints a seed
  that can be used to rerun the exact same random values."

BC seeds `Random` from the clock and creates random Guids. AL code sees only the range of the numbers and the
format of the Guids. So any seed is as correct as 1 (ADR-009), and a fresh seed per run is closer to BC than a fixed
one. What BC defines exactly stays fixed: the sequence after `Randomize(n)` (`types/TestRandomSeededContracts.al`),
including that the sequence goes on into the next test of the codeunit (same file, lines 111-131).

## Decision

### 1. Every run has a run seed

- The run seed is an integer from 0 to 2147483647 (v2's range).
- Precedence: `--seed N`, then `ALRUNNER_SEED`, then `AL_RUNNER_SEED` (v2's name), then a fresh seed.
- The fresh seed is chosen by `alrunner-cli` from OS randomness (`std::collections::hash_map::RandomState`, no new
  dependency). The core crates never choose it. They get it as a `Host` field, next to the clock and identity
  (ADR-006). This is an additive `Host` field, not a seam change.
- `--seed N` replays a run: the same seed, the same apps and the same selection give the same random values and
  Guids in every test.

### 2. What the run seed drives

One run seed derives each of these values. Each has its own derivation key, so they do not depend on each other.

| Value | Derivation | When it applies |
|---|---|---|
| The session's `Random` at the start of each test method | `Derive(runSeed, codeunitId, methodName)`, v2's FNV-1a formula | Before each test method, unless AL code in this session called `Randomize(n)` with a seed. Then the sequence goes on, as BC does (`types/TestRandomSeededContracts.al:111-131`). |
| `Randomize()` without a seed | the test's derived seed plus the count of such calls | Inside a test method |
| `CreateGuid` and new SystemIds | a 64-bit key of `(runSeed, codeunitId, methodName, "guid")` and a counter | Inside a test method. The 64-bit key keeps two methods from getting the same Guids. |
| `Library - Random` seed | `Derive(runSeed, codeunitId, methodName, "library")`, applied through ADR-058's `random.librarySeed` hook (the runner calls `LibraryRandom.SetSeed(n)` after codeunit 130301) | Only with `--seed-library` or a stability variant (owner decision 2). By default the runner keeps `SetSeed(1)`. |

Why per test method, as in v2: `--seed N --test X` must give test X the same values as in the full run, even when
the tests before it in the codeunit do not run.

The install code (the install baseline, ADR-046) does not use the run seed. It keeps the fixed seed 1, so the
install baseline is the same in every run and its cache stays valid (owner decision 4).

What the run seed does **not** change:

- The clock (ADR-045): noon UTC of today, 1 ms per read. `ALRUNNER_REAL_CLOCK` works as before.
- The work date.
- Identity, company, session settings.
- Test order and isolation.
- Anything BC defines: the sequence after `Randomize(n)`, record order, trigger and event order.

Changing these is ADR-058's job, not the run seed's.

### 3. Printing and output

- **Start of the run.** One line before the first test: `Seed: 4271833`.
- **Summary.** A `Seed:` line next to `Tests:` and `Result:`, as in v2. When a test failed, one more line with the
  command that replays the first failure: `Replay: al-runner test <same paths> --seed 4271833 --test <name>`.
- **`--output-json`.** A top-level `seed` (number) and `seedSource` (`option`, `env`, `fresh` or `fixed`). The
  start and summary lines go to stderr, so stdout holds only the JSON.
- **`--quiet`.** The summary `Seed:` line is still printed. A seed that is not printed cannot be used.
- **JUnit (`--junit`).** `<property name="seed" value="N"/>` on every `testsuite`, as in v2.
- **`serve` (ADR-028).** `runTests` accepts an optional `seed`. Each request without one gets a fresh seed. The
  summary holds `seed` and `seedSource`. The warm slot's compile and install baseline do not depend on the seed.
- **`debug` (ADR-057).** Accepts `--seed N`, so a failure can be debugged with the same values.

### 4. Exit codes

The seed adds no exit code. A test that fails because of its random values exits 1, as any failure. A `--seed`
value that is not an integer in range, or an `ALRUNNER_SEED` value that is not, is a bad invocation: exit 2.

### 5. Interaction with other features

- **ADR-058 (stability mode).** The run seed is the base variant's seed. When the config has no `masterSeed`, the
  master seed is the run seed, so a stability run prints one seed, not two. The defaults of `random.hostSeed`,
  `guid.seed` and `random.librarySeed` are the values derived from the run seed. A variant's own value replaces the
  derived one for that variant only. The variant id includes the run seed: `29.0-w1/3@4271833`.
  `--stability-replay` with such an id sets the run seed from it. This amends ADR-058 section 4 (the id format and
  the `masterSeed` default).
- **`--affected` (ADR-050).** The seed is not part of the compile cache key, of the install baseline key, or of
  the change rule. The stored result records the seed it ran with. A test that failed under another seed is rerun
  by `--include-failing`, as any failed test.
- **Corpus CI and the ratchet.** `al-test-cli` runs with the fixed run seed 1 (`seedSource: fixed`) by default, so
  `metrics/<major>/passing.txt`, `check.sh`, the Microsoft bucket runs and the perf budgets stay deterministic.
  `al-test-cli --seed N` is accepted for local checks; `--update-baseline` is refused with any seed other than 1.
- **Our own CI jobs that run `al-runner`** and compare results against an expected file pass `--seed 1`.
- **Impact runs (ADR-043, ADR-052).** The with and without runs of one impact run use the same run seed, so a
  difference between them is caused by the extension, not by the values.
- **Matrix runs (ADR-052).** Every cell gets the same run seed. The child processes get it as `--seed`.

### 6. Change in values from today

The derivation per test method changes the numbers that tests draw before any `Randomize(n)`, also with seed 1.
The corpus pins only the range of those numbers, so no corpus case should change. The implementation pull request
shows `--baseline-diff` with no lost case, for every major.

## Consequences

- A user can rerun a failing test with the exact random values and Guids of the failing run, locally or from CI
  logs.
- A test that depends on its random values now fails in some runs and passes in others. That is the point: the
  seed in the log makes the failure repeatable.
- With `Library - Random` loaded, the reset step's `SetSeed(1)` still fixes the numbers of Microsoft's libraries, so
  most Microsoft tests see different Guids per run but the same `Library - Random` numbers (owner decision 2).
- Text built from Guids (`Any.AlphanumericText`, `LibraryRandom.RandText`) is now repeatable with the seed. In v2
  it was not.
- Parity row SEL-9 and the `Seed:` part of OUT-4, OUT-6 and ENV-2 become Done when built.
  `docs/manual/migrating-from-v2.md` and `docs/manual/roadmap.md` change in the same pull request as the code.

## Conformance

- Unit tests in `al-host`/`al-vm`: golden values of `Derive` taken from v2's `RunSeedTests`; the same run seed
  gives the same `Random` numbers and Guids per test method; a different seed gives different ones.
- A test that `Randomize(n)` in one test method still carries over to the next method of the codeunit
  (`types/TestRandomSeededContracts.al:111-131`) for every run seed.
- A test that `--seed N --test X` gives X the same values as the full run with `--seed N`.
- A test that the install baseline is identical for two different run seeds.
- CLI tests: the `Seed:` lines, the JSON `seed`/`seedSource`, the JUnit property, the `serve` summary field, and
  exit 2 for a bad `--seed`.
- The corpus ratchet with `al-test-cli`'s fixed seed shows no lost case.

## Owner decisions (2026-10-30)

The owner accepted the ADR with the recommended answer to each question.

1. **Default: a fresh seed per run, or a fixed seed that is printed?** A fixed printed seed (for example 1) makes
   every run repeatable without any flag, but it never finds a test that depends on its values, and printing it adds
   little. A fresh seed is v2's behavior and what the owner described. Decided: a fresh seed per run in
   `al-runner` and `serve`; a fixed seed in `al-test-cli` (section 5).
2. **Does a normal run change the `Library - Random` seed?** Real BC's test runner sets `SetSeed(1)` before every
   test, and v2 did not change it. Changing it by default would make Microsoft's own tests fail in some normal runs
   where real BC never fails. Decided: no. The derived library seed is used only with `--seed-library` and in
   ADR-058 variants.
3. **Use v2's exact `Derive` formula?** Our `Random` is the same .NET generator as v2's, so the same formula lets a
   seed from a v2 run reproduce the same `Random` numbers after migration. Decided: yes, for `Random`. Guids use
   a 64-bit key of our own, because v2's Guids were not seeded.
4. **Does install code use the run seed?** Using it would make the install baseline differ per run, so the cached
   baseline (ADR-022) and the warm `serve` slot could not be reused. Recommended: no, install code keeps seed 1. A
   test of install code that depends on random values is covered by ADR-058's `guid.seed`, which builds its own
   baseline.
5. **Is the seed part of the `--affected` stored-result check?** Decided: no (section 5). A changed seed alone
   is no reason to rerun unchanged tests.
