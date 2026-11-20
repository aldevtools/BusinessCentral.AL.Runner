# Workflow

The details behind the "Workflow" section of `CLAUDE.md`. Read the section your task touches.
`CLAUDE.md` keeps the short form of each rule; this file has the commands, flags and reasons.

## Build and run

Build once in release and use the binary: `cargo build --release`, then
`./target/release/al-test-cli --root .` (add `--junit FILE` for per-test
outcomes and refusal reasons).

## Fast loop

**Fast loop** (a full run takes about 20 s with Microsoft's apps and `check.sh` minutes; do not
iterate on either). `./target/release/al-test-cli --help` lists every flag.
1. Rank what is refused: `al-test-cli --root . --top-refusals 15` (add `--top-refusals-area record`
   for one folder). Reasons are folded: quoted names and numbers become `<name>` and `N`.
2. Pick a test and see why it ends as it does: `al-test-cli --root . --explain TestName` prints the
   outcome, the refusal reason with routine and `file:line`, or for a failure the error, code and
   stack. Add `--codeunit ID` when the name is not unique.
3. Iterate on one codeunit, area or file: `--codeunit ID`, `--test SUBSTR`, `--area FOLDER`,
   `--file SUFFIX` (repeatable, union). Only the selected test codeunits are compiled and run,
   and only the selected cases are checked against the lists. Add `--no-deps` for a run of about half a
   second when the test needs no Microsoft app.
4. Before committing, one run with `--baseline-diff` on each of BC 27, 28 and 29 (`--bc-version N`, about 20 s each; owner, 2026-11-10): it lists,
   by area, the regressions, the stale lines, the cases still listed and the cases without an
   outcome. Then `--prune-not-passing --bc-version N`, one run per major, deletes the stale lines of that major (without
   `--bc-version` it prunes only the newest; it refuses while a selection is active).
5. `--timings` says where a run spent its time (load, compile, install baseline, tests).
6. `AL_REACH_REPORT=1` prints, after the compile, how many routines each kind of root pulls in, which are refused,
   and how many only matter through a refused routine (slow on a bucket: it walks every test).

## Not-passing lists

**Not-passing lists** (ADR-065): `metrics/<major>/not-passing*.txt` list, per BC major and test app, the
oracle cases real BC passes and the runtime does not pass yet. CI fails when a case that is not listed does
not pass (a regression), when a listed case passes or left the oracle (a stale line), and when an oracle
case gets no outcome. A PR deletes the lines of the cases it makes pass, in the same PR:
`./target/release/al-test-cli --root . --prune-not-passing` (it only deletes). A corpus bump adds lines
for new cases with `--list-new-cases HEAD`. Adding a line for a case the oracle already had needs the
owner's OK; CI shows it in the job summary. No bot pushes to `main`.

## Order probes

**Order probes** (ADR-066 amendment 12): the corpus runs in the default order (fresh SingleInstance state and the
install baseline's NumberSequence values per test codeunit). Corpus files that pin what carries from one test
codeunit to the next are listed in `metrics/strict-order-files.txt`; each pass runs their codeunits once more,
alone, with `--strict-order`, and their cases take that outcome. They must pass there and get no not-passing line.

## Generated docs and ids

The table in `docs/probes-needed.md` is regenerated in the PR that changes the assumption registry
(`al-test-cli --root . --assumptions-doc`); a test fails while it is stale. The coverage maps
(`docs/corpus-coverage.*`, `docs/corpus-behavior-coverage.*`) are not committed: `--coverage-map` and
`--behavior-coverage-map` write them locally, and CI on `main` uploads them as an artifact.

After adding a builtin to the catalog or a registry, or an invented id: `cargo run -p al-builtins --example gen_builtin_ids`
(the test `builtin_ids_current` fails while `builtin_ids.rs` is stale).

## PR CI is the gate

**PR CI is the gate** (owner, 2026-11-05). Do not run `./check.sh`, `cargo test --workspace` or an
`--all-versions` corpus run locally before a pull request: on this shared machine they cost up to an
hour, and PR CI runs the full corpus on every major in about 6 to 12 minutes. Locally run
`cargo test -p` and `cargo clippy -p ... -- -D warnings` for the crates you touched, and `cargo fmt --all`. `./check.sh` (fmt, clippy, all tests, the lists) stays for a person who
wants the full check on their own machine.

## Draft pull requests and merge trains

Open the pull request as a draft and without auto-merge (owner, 2026-11-09): CI runs only the `test` job on a draft (owner, 2026-11-10),
so it takes little of the CI queue. The orchestrator reviews it. Every reviewed PR lands through
a merge train, never on its own (owner, 2026-11-13): the train's pull request runs CI once for all of them. An outside
PR with one small thing left joins the train as it is, and the orchestrator fixes that thing in the train. The orchestrator fixes or hands back
what that run reports.

## Commands agents never run

Agents never run `pkill`, `git stash`, `git commit -a` / `git add -A`, or start a
workflow run; observation runs are local, and CI is only the PR's own run.

- **Never `pkill` or `killall`.** Other agents run the same binaries. Stop only a PID you started.
- **Never `git stash`.** All worktrees share one stash list. Commit work in progress on your branch.
- **Never `git commit -a` or `git add -A`.** Stage paths by name. Check `git submodule status` first.
- **Never start a workflow run.** What that covers and how to observe instead: `docs/agents/brief.md`, rule 7.

## Corpus bumps

**Corpus bumps**: `tools/corpus_bump.sh RUN_ID` takes the oracle from a green corpus master run.
`tools/corpus_bump.sh --from-pr N[,N...]` (owner, 2026-11-05) does not wait for that run (about 55
minutes): the pin moves to the merge commit of the corpus pull requests, and only the codeunits their own
CI runs ran get those runs' results; every other case keeps the current oracle. It refuses when a run is
not green, a BC version is missing, a shared fixture changed, or it cannot show the run tested the merged
files. `--dry-run` shows what it would take, `--body FILE` writes the pull request body (which runs the
oracle came from). The next bump from a master run corrects anything only master shows. After the oracle, a
bump also regenerates the golden files (`pinned_columns.rs`, `expected-unresolved.txt`; commit them with the bump),
prunes the lists again, runs `cargo test -p al-version`, and prints what a person must fix (a regression, a case
without an outcome, a gate to extend) as a to-do list before it exits 1.

## Ratchets

Core crates do no OS access (ADR-006).

**File size ratchet** (`crates/al-test-cli/tests/file_size_ratchet.rs`): a Rust file under `crates/*/src` may
not pass 3,000 lines. The files already over are listed in `metrics/file-sizes.toml`, each with its line
count as its own cap. Generated files (`@generated` or `Generated by` in the first five lines) and
`crates/*/tests` are exempt. Never raise a cap or add a row (the owner decides); split the file (see
`docs/codemap.md`) and lower the cap. New code goes into a new module, not into a listed file.

**Code ratchets** (`cargo test -p al-test-cli --test fn_length_ratchet --test os_access_ratchet`, about 2 s each):
a `fn` body under `crates/*/src` may not pass 250 code lines (blank and comment lines not counted); the longer
ones are listed in `metrics/fn-lengths.toml` as `"path::name" = cap`. A file of a core crate may not name an OS
API (`std::env`, `std::fs`, `File::open`, `std::process`, ...; ADR-006); the current places are counted in
`metrics/os-access.toml`. Caps and counts only go down: extract functions, move OS access to a front end or
`al-host`, then lower or delete the row. Never raise a cap or add a row (the owner decides).

Work ratchet: the full corpus run compares what each pass compiled and ran (routines lowered, objects loaded,
dependency apps from source or cache, install baseline rows, tests run) with `metrics/<major>/work.json` and fails
on a count more than 10 % above it; CI's corpus job also fails a step more than 1.5x main's median time
(`docs/architecture.md`, "Work ratchet and step time guard"). A PR that adds work on purpose runs
`./target/release/al-test-cli --root . --all-versions --update-work` and commits the file in the same PR.

## Perf budgets

Perf budgets (cold compile of the corpus ≤ 695 000 instructions per KiB of corpus source; the full corpus run with Microsoft's apps ≤ 82.8 G instructions for the Microsoft apps plus 4 230 000 per KiB of the corpus source it compiles; wall time printed for information only; parser ≥ 50 MB/s) are
`#[ignore]`d tests that need release:
`cargo test --release --no-fail-fast -p al-syntax --test perf_parse_throughput -p al-test-cli --test perf_budget -p al-test-cli --test perf_vm -p al-data --lib -- --ignored --nocapture`.
CI runs them nightly (and on manual dispatch and before a numbered release), not on a pull request or a push to main (owner, 2026-11-04; from 2026-10-31 they also ran on every push to main). CI has no instruction counter, so there the CPU-time fallback only warns; with a counter (locally) the instruction budgets fail as before. `cargo run --release -p al-test-cli --example phases` prints
timings per phase. `--example compile_dump` dumps every compile output, to
diff before and after a performance change.

## Install baseline and eager compile

The install baseline is skipped when no selected test can observe it (ADR-013 note): after a change to `al-testrun/src/observe.rs`
or the baseline, `tools/baseline_rule_check.sh run` must show 0 differences (`AL_FORCE_BASELINE=1` turns the skip off).

Demand-driven against eager compile of the Microsoft apps (ADR-013; CI runs it nightly, not on a pull request or a push to main, owner 2026-11-04): `tools/demand_vs_eager.sh run`
(about 3 min, release binary needed) compares the main app and the OnPrem and isolation suites for BC 27, 28, 29.

## Worktrees

Parallel work: one `git worktree` and branch per area, inside the main
checkout (`/.worktrees/` is git-ignored): from the main checkout run
`git worktree add .worktrees/<area> -b <area> origin/main`. Never create a
worktree next to the repository (`../wt-<area>`). Merge `main` into the branch
before handing it back. A conflict in a not-passing list: take both sides'
deletions, then run `--prune-not-passing`.

## Logs and shell hygiene

Logs and scratch files go to your session's scratch folder or under
`~/.cache/al-runtime/`, never into the worktree root or
the folder that holds the repository.

Shell hygiene: never build a file list with `$(grep -l …)` that can come back
empty. `grep PATTERN` with no files waits on stdin forever; use `grep -rn
PATTERN dir/` with a real folder (`crates/`, `docs/`): `grep -rn` on `.` in
the main checkout also walks `.worktrees/` and `target/`.

## Other agent docs

Working method for implementation agents (gates, memory caps, report):
`docs/agents/brief.md`. Agent definitions: `.claude/agents/` (implementer, reviewer, probe writer);
the findings review keeps raising: `docs/agents/recurring-findings.md`.

A session on another machine that takes GitHub issues (setup and task loop):
`docs/agents/remote-worker.md`.
