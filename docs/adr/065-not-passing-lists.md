# ADR-065: A list of the corpus cases that do not pass replaces the passing baseline; no bot pushes to main

- Status: Proposed
- Date: 2026-11-04
- Amends: ADR-007 (the ratchet), ADR-015 (one ratchet per major)

## Context

Today each BC major and suite has a baseline `metrics/<major>/passing*.txt`: every corpus case the
runtime passes (about 65,500 lines over majors 27, 28 and 29). CI fails when a listed case stops
passing (`report::ratchet` and `report::decide` in `crates/al-test-cli/src/report.rs`, called from
`crates/al-test-cli/src/main.rs`). A case that the runtime newly passes is not added by the pull
request. After each merge, the regen bot (`.github/workflows/regen-bot.yml`, `tools/regen_bot.sh`)
takes the files that main's CI run wrote and pushes them straight to main with `[skip ci]`: the
baselines, the two coverage maps (`docs/corpus-coverage.*`, `docs/corpus-behavior-coverage.*`) and the
table of `docs/probes-needed.md`. `metrics/<major>/work.json` is not written by the bot; a pull request
that adds work on purpose commits it (`--update-work`).

Two problems follow.

1. **Every merge moves main twice.** The ruleset `main` requires pull requests to be up to date with
   main (strict status checks). Each merge is followed by a regen push, so a pull request that was
   just updated falls behind again and needs another CI run.
2. **A bot needs write access to main.** The bot pushes with a deploy key that bypasses the ruleset.
   It is the only writer to main that is not a reviewed pull request.

The owner's direction (2026-11-04): replace the passing baseline with a list of the cases that do
not pass yet. Simplicity is the goal, and no bot pushes to main.

### Measured (2026-11-04, from `gh run list` and `gh api`)

- The last 40 regen bot runs (2026-11-02 15:13 to 2026-11-04 18:17 UTC): 29 succeeded, 4 failed,
  7 were skipped. 27 regen commits reached main in that time (two successful runs pushed nothing).
- Last two days (from 2026-11-02 18:56 UTC): 36 merges to main and 25 regen pushes. A regen push
  followed its merge after 24 minutes (median; 15 to 47 minutes). Main moved 61 times instead of 36.
- In the same two days, CI ran 234 times on pull requests: 140 were skipped (drafts run no job), 94
  ran. 35 of the 94 ran on a commit that merged main into the branch. In 19 of these 35 (17
  branches), the main commit that was merged in was a regen commit. Those 19 runs took 242 minutes
  of wall time and 470 runner job minutes.
- Method: for each pull-request run, the parents of its head commit (`gh api .../commits/SHA`). A run
  counts when one parent is a regen commit on main's first-parent history. This is an upper bound:
  without the regen push, some of those branches would still have been behind because of the merge
  just before it. It is also a lower bound in one way: an author who saw the branch fall behind and
  waited for the regen push before updating is not counted.

## Decision

### 1. One list per BC major and suite

`metrics/<major>/not-passing.txt` for the main corpus app and `metrics/<major>/not-passing-<suite>.txt`
for each suite of `oracle/oracle.toml` (onprem, onprem-dotnet, testlibraries, isolation-function,
isolation-codeunit, and isolation-disabled once a corpus bump adds it). Same format as today: one
`<codeunit> <procedure>` per line, sorted, `#` comments. A line names a case of that version's oracle
that real BC passes and the runtime fails, refuses or does not run. The `baseline` key in
`oracle.toml` points at the new file.

Cases that real BC fails (`bc_failed` in the oracle, excluded from the pass rate today) are in no list
and are not checked, as today.

The lists are short: about 2,250 lines for the main app per major, about 220 for onprem, 80 for
onprem-dotnet and 20 to 40 for testlibraries; about 7,700 lines in all, against 65,500 today. Each
list shrinks as the runtime catches up.

### 2. The CI rule on every corpus run

For each version and suite, over the cases the run selected:

1. **A case that is not listed must pass.** If it fails, is refused or is not run, CI fails. This is a
   regression. Adding its line to the list needs the owner's OK, the same as removing a baseline line
   under hard rule 2 today. CI prints the case, its outcome and the reason.
2. **A listed case that now passes is a stale line.** The pull request that made it pass deletes the
   line. CI prints the lines to delete and the command that deletes them (section 6). Whether CI
   fails or only warns is open question 1; the recommendation is that it fails.
3. **A listed case that is no longer in the oracle is a stale line too.** CI asks for the line to be
   deleted, with the same rule as point 2.
4. **Every oracle case has an outcome.** Each case of the oracle must come out of the run as passed,
   failed or refused with a reason. A case with no recorded outcome fails CI, listed or not. It means
   the runner did not run a case and said nothing, which is a runner bug. Point 1 already catches this
   for unlisted cases; point 4 adds the listed ones, which would otherwise stay hidden.

A `.NET`-only case that is skipped because .NET is unavailable is handled as today: a local run
prints it and does not fail, and CI runs with `--require-dotnet`, where it fails.

A selection (`--codeunit`, `--test`, `--area`, `--file`) checks only the selected cases, as the
ratchet does today.

New cases from a corpus bump are not regressions. A corpus bump pull request adds a line for each new
oracle case that the runtime does not pass. CI tells new cases apart from regressions by comparing
with the oracle on the base branch: a line added for a case the base oracle did not have is fine; a
line added for a case the base oracle had is a regression (open question 2 covers how the owner's OK
is shown).

### 3. Tests the corpus deletes

A test that the corpus removes leaves `oracle/` with the bump. There is no extra rule for it at the
bump. If it was listed, point 3 of section 2 asks for its line to be deleted. If it passed, it simply
stops being checked. The owner accepts the risk that a corpus change drops a passing case without a
signal (owner decision, 2026-11-04).

### 4. The regen bot stops writing to main

- The corpus job no longer passes `--update-baseline` on a push to main and no longer uploads the
  `regen-<run id>` artifact.
- `.github/workflows/regen-bot.yml` and `tools/regen_bot.sh` are deleted, with their test
  (`crates/al-test-cli/tests/suite/regen_bot.rs`).
- The deploy key `regen-bot` and its bypass entry in the ruleset `main` are removed (owner, in the
  repository settings).
- After this, main changes only through merged pull requests. Each merge moves main once.

### 5. Migration

1. Download the `pass-rate-*` artifact of the newest green CI run on main (its
   `runtime-junit-<major>-<suite>.xml` files: the runtime's outcome for every case).
2. For each major (27, 28, 29) and suite (normal, onprem, onprem-dotnet, testlibraries, the 28
   isolation suites, and isolation-disabled where its oracle exists), write the oracle cases that BC
   passes minus the cases the runtime passed. A new flag, `--write-not-passing`, does this from a
   local full run with `--require-dotnet`. The download is the cross-check: both must give the same
   lists.
3. Check that each list is the complement of today's `passing*.txt` within the oracle. A case in
   `passing*.txt` that is not in the oracle is reported and dropped; it is not checked today either.
4. In one pull request: add the lists, change the `baseline` keys in `oracle.toml`, change the ratchet
   code (section 6), delete the `passing*.txt` files, the regen bot and its test, and change CI and the
   documents. Land it while no other pull request is about to merge, so no case moves between the
   download and the merge.

Files the regen bot also writes today:

- **`metrics/<major>/work.json`:** not written by the regen bot. It stays as it is: a pull request
  that adds work on purpose runs `--update-work` and commits it.
- **The coverage maps** (`docs/corpus-coverage.*`, `docs/corpus-behavior-coverage.*`): open question 3.
  The recommendation is to stop committing them. CI on a push to main uploads them as an artifact, and
  `al-test-cli --coverage-map` and `--behavior-coverage-map` make them locally in seconds. The test in
  `corpus_resolve_deps.rs` that compares them with the committed files changes to check that they can
  be built.
- **The table of `docs/probes-needed.md`:** it comes from the assumption registry in the code, so the
  pull request that changes the registry regenerates it, and a test fails while it is stale (as
  `builtin_ids_current` does for `builtin_ids.rs`).

### 6. What changes in code, tools and documents

- **`al-test-cli`:** the ratchet reads `not-passing*.txt` and applies section 2.
  - `report::ratchet` returns regressions (unlisted, not passing), stale lines (listed and passing, or
    not in the oracle) and cases without an outcome.
  - Exit 1 for regressions and cases without an outcome, and for stale lines if open question 1 goes
    that way. Exit 2 for setup errors, as today.
  - `--update-baseline` is replaced by `--prune-not-passing`: it deletes stale lines and nothing else.
    It never adds a line, so it cannot hide a regression. It refuses while a selection is active, as
    today.
  - Adding lines has its own flag, `--list-new-cases`, for corpus bumps: it adds lines only for cases
    that the base branch's oracle does not have. A regression line is added by hand, with the owner's
    OK.
  - `--baseline-diff` keeps its name and shows, by area: regressions, stale lines to delete, cases
    still listed, and cases without an outcome.
- **CI (`ci.yml`):** the corpus step drops `--update-baseline`; the step "Generated files for the regen
  bot" and the `regen-*` upload go away. The job summary lists stale lines and new-case lines.
- **Corpus bump (`tools/corpus_bump.sh`, `tools/corpus_bump_bot.sh`):** the bump writes the lines for
  new cases that do not pass (`--list-new-cases`) and deletes the lines of cases that left the oracle
  (`--prune-not-passing`). The bump bot may write `metrics/*/not-passing*.txt`, but only these two
  kinds of change; it still never adds a line for a case the base oracle has.
- **`check.sh`:** no change. It runs `corpus_harness`, which runs the ratchet and so the new rule.
- **CLAUDE.md:**
  - Hard rule 2: "never remove lines from `metrics/<major>/passing.txt`" becomes "never add a line
    to `metrics/<major>/not-passing*.txt` for a case the oracle already had". The exception for a case
    that passed by accident stays: such a case is added to the list, with the reason in the commit
    message.
  - Workflow: the paragraph about the regen bot is replaced. A pull request deletes the lines of the
    cases it makes pass, in the same pull request (`--prune-not-passing`). The coverage maps and the
    probes table are handled as in section 5.
  - "Regenerate the baseline after merging; never hand-merge `metrics/<major>/passing.txt`" becomes:
    after merging main, resolve a conflict in a list by taking both deletions, then run
    `--prune-not-passing`.
- **`docs/agents/brief.md`:** the "Never weaken an oracle" bullet and "Generated files" section change
  as in CLAUDE.md. Step "Before you commit" adds `--prune-not-passing` after the `--baseline-diff` run.
- **`.github/pull_request_template.md`:** the checkbox becomes "`--baseline-diff`: no regression; stale
  lines deleted".
- **ADR-007:** amended to name the not-passing lists as the ratchet. Its decision 2 (failures gate only
  on regressions) stays true.

## Consequences

### What stays the same

- The oracle, the corpus pin and `pin::check` do not change. Hard rule 2 keeps its intent: the
  runtime may not lose a case without the owner's OK.
- The set of cases CI protects is the same: every case BC passes that the runtime passed is still
  checked, now as "not listed".
- `--require-dotnet`, selection runs, the work ratchet, `work.json`, the step time guard, golden files
  and the harness counts work as today.
- Main still requires up-to-date pull requests. Merges still move main and still make open pull
  requests fall behind; only the regen pushes go away.

### Benefits

- No bot writes to main; one deploy key and one ruleset bypass go away.
- Main moves once per merge. In the measured two days that is 36 moves instead of 61, and up to 19 pull
  request CI runs (470 job minutes) fewer.
- The list in a pull request shows its effect directly: deleted lines are the cases it fixed.
- The files shrink from about 65,500 lines to about 7,700, and the list doubles as a work list.

### Risks

- **Merge conflicts in the lists.** Two pull requests that delete adjacent lines of one list conflict
  in git. Fix: take both deletions, or merge main and run `--prune-not-passing`. Today the bot avoids
  this; after the change each author handles it.
- **A stale line hides a later regression for a short time.** If CI only warns (open question 1), a
  case that passes but stays listed can break again later and nobody notices. A failing check closes
  this gap.
- **A pull request that adds a regression line looks like a normal edit.** The owner's OK has to be
  visible in review (open question 2). The same is true today for a removed baseline line.
- **Corpus deletions drop cases without a signal** (section 3, accepted by the owner).
- **A runner that stops recording outcomes** would hide listed cases; section 2 point 4 catches it.
- **Migration timing.** A case that changes between the download and the merge makes the migration
  pull request red. It is fixed in that pull request by running `--prune-not-passing` or by
  investigating the regression.

## Open questions for the owner

1. **Stale line: fail or warn?** Recommendation: **fail**. CI prints the exact lines and
   `--prune-not-passing` deletes them, so the cost is one command. With a warning, a passing case stays
   listed, and if it breaks again later CI says nothing: the same gap the passing baseline closes
   today. The runtime runs with a fixed seed, so a pass is not random, and a case cannot become stale
   on main by itself: the only ways are a pull request that fixes it or a corpus bump, and both run CI.
   The one cost is that a pull request can fail CI for a case it fixed by accident; the message says
   which lines to delete.
2. **How is the owner's OK for a regression line shown?** Recommendation: CI lists each added line for
   a case the base oracle already had in the job summary under "regressions accepted by this pull
   request", and the reviewer checks it against the owner's OK, as for a removed baseline line today.
   A required label would be stricter but adds a step the owner asked to avoid.
3. **Coverage maps: keep them committed?** Recommendation: stop committing them and upload them as a
   CI artifact on main (section 5). Committing them in each pull request would make every feature pull
   request conflict with the others on large generated files.

## Conformance

- `crates/al-test-cli/tests/corpus_harness.rs`: an unlisted case that fails exits 1; a listed case
  that passes is reported as stale (and exits 1 if open question 1 is accepted as recommended); a
  listed case missing from the oracle is reported as stale; an oracle case with no outcome exits 1;
  `--prune-not-passing` only deletes lines.
- `crates/al-test-cli/tests/suite/corpus_bump_script.rs`: the bump adds lines only for cases the base
  oracle lacks.
- No workflow in `.github/workflows/` pushes to main.
