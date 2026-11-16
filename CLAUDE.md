# Agent instructions

al-runtime-rs is a Rust-native AL runtime (see `README.md`). Read
`docs/architecture.md`, `docs/status.md` and `docs/adr/README.md` first, then
`docs/codemap.md` (which module owns what, and the call chains of the main flows)
before you open a source file.

## Priorities: correct first, then easy to change

**1. Correct.** The runner gives Business Central's result, or it refuses (the hard rules below).

**2. Easy to change, second only to correctness, and ahead of new features and speed.** The code
must stay cheap and safe for the next person or agent to change. In practice that means:

- **One behavior, one place.** Each rule and each piece of state has one owner module. A fix
  changes that owner; it does not add a second copy of the rule somewhere else. If a fix would
  need a second copy, refactor first.
- **Small change, small rebuild.** A change in one area rebuilds and retests only the crates that
  area belongs to. Keep the crate and module boundaries that make this true (ADR-008, ADR-010,
  ADR-072).
- **Readable without the history.** A new reader finds the owner of a behavior from
  `docs/codemap.md` and understands it from its names and its corpus citation, without reading
  old pull requests.
- **The numbers only go down.** File size, function length, OS access and refusal counts are
  ratchets (see Workflow). Never raise a cap; split, extract or delete instead.
- **Leave it easier than you found it.** Every pull request may fix one nearby thing that makes
  the code harder to change: a stale comment, a duplicate helper, a too-long function.
- **Delete before adding.** Prefer removing code, special cases and workarounds over adding new
  ones.

When correctness and ease of change pull in different directions, correctness wins, but the pull
request says what it made harder and opens the follow-up.

**Take the extra time (owner, 2026-11-10).** A slower, less frequent release with a stronger
solution is better than a fast fix that needs rework or regresses later. Spend more time and more
tokens up front, on design, on reading the code and the corpus, and on review, when that gives a
cleaner, more readable solution with fewer regressions and less token waste later. On every open
question, choose the better long-term solution over the quicker one.

**3. Then features, then speed** (within the perf budgets).

## Hard rules

1. **Red/green TDD, no exceptions.** Every behaviour starts as a failing test;
   every bug fix starts with a test that reproduces it. Show the red run before
   the green one. When a corpus case pins the behaviour, that case is the test:
   run it with `--file` or `--codeunit`, show it red before and green after, and
   prune its line from the not-passing list. Write a Rust test only for logic
   the corpus cannot reach (internal tooling, CLI and report formats,
   memory and threads, refusal plumbing, regressions that need a minimal
   reproduction). Owner decision, 2026-11-10. Nothing merges with `cargo test` red. Corpus cases that are
   not yet implemented are tracked by the pass-rate report, not by tests.
2. **Never weaken an oracle.** Never edit the corpus submodule or `oracle/`,
   and never add a line to `metrics/<major>/not-passing*.txt` for a case the oracle
   already had to make CI green (ADR-065). If real BC and the corpus disagree, open an
   issue on the corpus repo. The only legitimate addition is a case shown to have passed
   by accident (for example against an unseeded store), with the owner's OK. Say so
   explicitly in the commit message.
3. **No guessing at semantics.** Every behavior cites the corpus test (file:line) that pins it, in a
   code comment. When the pinned corpus does not cover a behavior yet (owner, 2026-11-12):
   - Write a probe on a branch of the corpus repo and open its pull request (ADR-007).
   - The runtime PR may land before that probe is merged or pinned. Until it is pinned, the path runs
     only through an entry of the assumption registry (one registry in the runtime code, ids checked
     by the compiler) that cites the open probe PR. A test that passes through it is a flagged pass,
     counted apart from clean passes. Add the `docs/probes-needed.md` row in the same PR.
   - Evidence for an assumption: the probe's result on a local BC, a Microsoft test that passes on
     real BC, or an outside reporter's own repro (cite it; the probe is still written). Partner test
     suites never count.
   - The assumption covers only the shapes its evidence measures. Every other shape refuses.
   - When a corpus bump pins the probe, delete the assumption and cite the pin. If the probe
     disagrees, fix the code. The probe always wins.
   - No probe written: refuse. Never run a behavior that has neither a pin nor an assumption id.
   - Error texts come only from the corpus. Never invent one.
4. **Unsupported is refused explicitly, never run silently.** Refuse
   statically wherever possible: a lowering reason, or a diagnostic in
   `al-sema`, so the runner reports the test as not run before it starts.
   What can only be discovered mid-run raises `Signal::Unsupported`. A
   guessed or partial behaviour is never an acceptable stand-in.
5. **Seams change only through an ADR** in `docs/adr/`. Draft it as Proposed
   and ask the project owner (AskUserQuestion) to accept it before building on
   it. Additive changes (a new `Value` variant, an IR field, a defaulted `Ctx`
   method) do not need an ADR; changing or removing a seam does.
6. **Match BC where it is observable; improve on it everywhere else**
   (ADR-009). Values, errors, database state, trigger and event order, host
   interaction and whether code compiles must match real BC. Internals,
   performance, determinism, diagnostics and tooling are free to be better.
   Never reject code BC accepts, and never "fix" BC behaviour AL code can see.
7. **Do not fake objects from dependent apps.** Microsoft's apps are compiled
   from their real source (ADR-013, Proposed). Native code exists only for
   platform objects that have no AL body. Nothing from Microsoft is ever
   committed to this repository.

## Workflow

- Build once in release and use the binary: `cargo build --release`, then
  `./target/release/al-test-cli --root .` (add `--junit FILE` for per-test
  outcomes and refusal reasons).
- **Not-passing lists** (ADR-065): `metrics/<major>/not-passing*.txt` list, per BC major and test app, the
  oracle cases real BC passes and the runtime does not pass yet. CI fails when a case that is not listed does
  not pass (a regression), when a listed case passes or left the oracle (a stale line), and when an oracle
  case gets no outcome. A PR deletes the lines of the cases it makes pass, in the same PR:
  `./target/release/al-test-cli --root . --prune-not-passing` (it only deletes). A corpus bump adds lines
  for new cases with `--list-new-cases HEAD`. Adding a line for a case the oracle already had needs the
  owner's OK; CI shows it in the job summary. No bot pushes to `main`.
- **Order probes** (ADR-066 amendment 12): the corpus runs in the default order (fresh SingleInstance state and the
  install baseline's NumberSequence values per test codeunit). Corpus files that pin what carries from one test
  codeunit to the next are listed in `metrics/strict-order-files.txt`; each pass runs their codeunits once more,
  alone, with `--strict-order`, and their cases take that outcome. They must pass there and get no not-passing line.
- The table in `docs/probes-needed.md` is regenerated in the PR that changes the assumption registry
  (`al-test-cli --root . --assumptions-doc`); a test fails while it is stale. The coverage maps
  (`docs/corpus-coverage.*`, `docs/corpus-behavior-coverage.*`) are not committed: `--coverage-map` and
  `--behavior-coverage-map` write them locally, and CI on `main` uploads them as an artifact.
- **Fast loop** (a full run takes about 20 s with Microsoft's apps and `check.sh` minutes; do not
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
- After adding a builtin to the catalog or a registry, or an invented id: `cargo run -p al-builtins --example gen_builtin_ids`
  (the test `builtin_ids_current` fails while `builtin_ids.rs` is stale).
- **PR CI is the gate** (owner, 2026-11-05). Do not run `./check.sh`, `cargo test --workspace` or an
  `--all-versions` corpus run locally before a pull request: on this shared machine they cost up to an
  hour, and PR CI runs the full corpus on every major in about 6 to 12 minutes. Locally run
  `cargo test -p` and `cargo clippy -p ... -- -D warnings` for the crates you touched, and `cargo fmt --all`.
  Open the pull request as a draft and without auto-merge (owner, 2026-11-09): CI runs only the `test` job on a draft (owner, 2026-11-10),
  so it takes little of the CI queue. The orchestrator reviews it. Every reviewed PR lands through
  a merge train, never on its own (owner, 2026-11-13): the train's pull request runs CI once for all of them. An outside
  PR with one small thing left joins the train as it is, and the orchestrator fixes that thing in the train. The orchestrator fixes or hands back
  what that run reports. Agents never run `pkill`, `git stash`, `git commit -a` / `git add -A`, or start a
  workflow run; observation runs are local, and CI is only the PR's own run. `./check.sh` (fmt, clippy, all tests, the lists) stays for a person who
  wants the full check on their own machine.
- **Corpus bumps**: `tools/corpus_bump.sh RUN_ID` takes the oracle from a green corpus master run.
  `tools/corpus_bump.sh --from-pr N[,N...]` (owner, 2026-11-05) does not wait for that run (about 55
  minutes): the pin moves to the merge commit of the corpus pull requests, and only the codeunits their own
  CI runs ran get those runs' results; every other case keeps the current oracle. It refuses when a run is
  not green, a BC version is missing, a shared fixture changed, or it cannot show the run tested the merged
  files. `--dry-run` shows what it would take, `--body FILE` writes the pull request body (which runs the
  oracle came from). The next bump from a master run corrects anything only master shows. After the oracle, a
  bump also regenerates the golden files (`pinned_columns.rs`, `expected-unresolved.txt`; commit them with the bump),
  prunes the lists again, runs `cargo test -p al-version`, and prints what a person must fix (a regression, a case
  without an outcome, a gate to extend) as a to-do list before it exits 1.
- Perf budgets (cold compile of the corpus ≤ 695 000 instructions per KiB of corpus source; the full corpus run with Microsoft's apps ≤ 82.8 G instructions for the Microsoft apps plus 4 230 000 per KiB of the corpus source it compiles; wall time printed for information only; parser ≥ 50 MB/s) are
  `#[ignore]`d tests that need release:
  `cargo test --release --no-fail-fast -p al-syntax --test perf_parse_throughput -p al-test-cli --test perf_budget -p al-test-cli --test perf_vm -p al-data --lib -- --ignored --nocapture`.
  CI runs them nightly (and on manual dispatch and before a numbered release), not on a pull request or a push to main (owner, 2026-11-04; from 2026-10-31 they also ran on every push to main). CI has no instruction counter, so there the CPU-time fallback only warns; with a counter (locally) the instruction budgets fail as before. `cargo run --release -p al-test-cli --example phases` prints
  timings per phase. `--example compile_dump` dumps every compile output, to
  diff before and after a performance change.
- Work ratchet: the full corpus run compares what each pass compiled and ran (routines lowered, objects loaded,
  dependency apps from source or cache, install baseline rows, tests run) with `metrics/<major>/work.json` and fails
  on a count more than 10 % above it; CI's corpus job also fails a step more than 1.5x main's median time
  (`docs/architecture.md`, "Work ratchet and step time guard"). A PR that adds work on purpose runs
  `./target/release/al-test-cli --root . --all-versions --update-work` and commits the file in the same PR.
- The install baseline is skipped when no selected test can observe it (ADR-013 note): after a change to `al-testrun/src/observe.rs`
  or the baseline, `tools/baseline_rule_check.sh run` must show 0 differences (`AL_FORCE_BASELINE=1` turns the skip off).
- Demand-driven against eager compile of the Microsoft apps (ADR-013; CI runs it nightly, not on a pull request or a push to main, owner 2026-11-04): `tools/demand_vs_eager.sh run`
  (about 3 min, release binary needed) compares the main app and the OnPrem and isolation suites for BC 27, 28, 29.
- Core crates do no OS access (ADR-006).
- **File size ratchet** (`crates/al-test-cli/tests/file_size_ratchet.rs`): a Rust file under `crates/*/src` may
  not pass 3,000 lines. The files already over are listed in `metrics/file-sizes.toml`, each with its line
  count as its own cap. Generated files (`@generated` or `Generated by` in the first five lines) and
  `crates/*/tests` are exempt. Never raise a cap or add a row (the owner decides); split the file (see
  `docs/codemap.md`) and lower the cap. New code goes into a new module, not into a listed file.
- **Code ratchets** (`cargo test -p al-test-cli --test fn_length_ratchet --test os_access_ratchet`, about 2 s each):
  a `fn` body under `crates/*/src` may not pass 250 code lines (blank and comment lines not counted); the longer
  ones are listed in `metrics/fn-lengths.toml` as `"path::name" = cap`. A file of a core crate may not name an OS
  API (`std::env`, `std::fs`, `File::open`, `std::process`, ...; ADR-006); the current places are counted in
  `metrics/os-access.toml`. Caps and counts only go down: extract functions, move OS access to a front end or
  `al-host`, then lower or delete the row. Never raise a cap or add a row (the owner decides).
- Parallel work: one `git worktree` and branch per area, inside the main
  checkout (`/.worktrees/` is git-ignored): from the main checkout run
  `git worktree add .worktrees/<area> -b <area> origin/main`. Never create a
  worktree next to the repository (`../wt-<area>`). Merge `main` into the branch
  before handing it back. A conflict in a not-passing list: take both sides'
  deletions, then run `--prune-not-passing`.
- Logs and scratch files go to your session's scratch folder or under
  `~/.cache/al-runtime/`, never into the worktree root or
  `/home/stefan/Documents/Repos/community`.
- Shell hygiene: never build a file list with `$(grep -l …)` that can come back
  empty. `grep PATTERN` with no files waits on stdin forever; use `grep -rn
  PATTERN dir/` with a real folder (`crates/`, `docs/`): `grep -rn` on `.` in
  the main checkout also walks `.worktrees/` and `target/`.
- Working method for implementation agents (gates, memory caps, report):
  `docs/agents/brief.md`. Agent definitions: `.claude/agents/` (implementer, reviewer, probe writer);
  the findings review keeps raising: `docs/agents/recurring-findings.md`.
- A session on another machine that takes GitHub issues (setup and task loop):
  `docs/agents/remote-worker.md`.
