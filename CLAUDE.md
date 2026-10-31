# Agent instructions

al-runtime-rs is a Rust-native AL runtime (see `README.md`). Read
`docs/architecture.md`, `docs/status.md` and `docs/adr/README.md` first, then
`docs/codemap.md` (which module owns what, and the call chains of the main flows)
before you open a source file.

## Hard rules

1. **Red/green TDD, no exceptions.** Every behaviour starts as a failing test;
   every bug fix starts with a test that reproduces it. Show the red run before
   the green one. Nothing merges with `cargo test` red. Corpus cases that are
   not yet implemented are tracked by the pass-rate report, not by tests.
2. **Never weaken an oracle.** Never edit the corpus submodule or `oracle/`,
   and never remove lines from `metrics/<major>/passing.txt` to make CI green. If real
   BC and the corpus disagree, open an issue on the corpus repo. The only
   legitimate removal is a case shown to have passed by accident (for example
   against an unseeded store). Say so explicitly in the commit message.
3. **No guessing at semantics.** Cite the corpus test (file:line) that pins a
   behaviour in a code comment. If nothing pins it, write a probe test on a
   branch of the corpus repo and read the result from its CI (ADR-007), get it
   merged there, then bump the submodule and oracle and implement. Until then,
   record a documentation-only assumption in `docs/probes-needed.md`, or
   refuse. Error texts come from the corpus and are never invented.
   Exception (owner, 2026-10-15): a behavior that Microsoft's own tests require
   to pass on real BC (the test passes in Microsoft's CI and needs exactly this
   behavior) may be implemented before a probe exists. Cite that test in the
   code comment, add a `docs/probes-needed.md` row, and queue the probe. When
   the probe lands it wins; fix the code if it disagrees. Error texts still come
   only from the corpus. Every such code path marks its assumption when it runs,
   and a test that passes through one is reported as a flagged pass ("passed,
   relies on an unprobed assumption"), counted apart from clean passes. The
   assumptions are registered in the runtime code itself (one registry, ids
   checked by the compiler and a test), not only in a document.
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
- Do not regenerate `metrics/<major>/passing*.txt`, the coverage maps (`docs/corpus-coverage.*`,
  `docs/corpus-behavior-coverage.*`) or the table in `docs/probes-needed.md` in a feature PR: after
  each merge to `main`, the regen bot (`.github/workflows/regen-bot.yml`) adds the new passes from
  main's CI run and pushes them. PR CI still fails a PR that loses a baseline case. Baseline lines
  are only ever added by the bot; removing one stays a human decision (hard rule 2).
- **Fast loop** (a full run takes about 20 s with Microsoft's apps and `check.sh` minutes; do not
  iterate on either). `./target/release/al-test-cli --help` lists every flag.
  1. Rank what is refused: `al-test-cli --root . --top-refusals 15` (add `--top-refusals-area record`
     for one folder). Reasons are folded: quoted names and numbers become `<name>` and `N`.
  2. Pick a test and see why it ends as it does: `al-test-cli --root . --explain TestName` prints the
     outcome, the refusal reason with routine and `file:line`, or for a failure the error, code and
     stack. Add `--codeunit ID` when the name is not unique.
  3. Iterate on one codeunit, area or file: `--codeunit ID`, `--test SUBSTR`, `--area FOLDER`,
     `--file SUFFIX` (repeatable, union). Only the selected test codeunits are compiled and run,
     and only the selected baseline cases are ratcheted. Add `--no-deps` for a run of about half a
     second when the test needs no Microsoft app.
  4. Before committing, one full run: `--baseline-diff` lists, by area, what newly passes and which
     baseline cases stopped passing. `--update-baseline` refuses while a selection is active.
  5. `--timings` says where a run spent its time (load, compile, install baseline, tests).
  6. `AL_REACH_REPORT=1` prints, after the compile, how many routines each kind of root pulls in, which are refused,
     and how many only matter through a refused routine (slow on a bucket: it walks every test).
- After adding a builtin to the catalog or a registry, or an invented id: `cargo run -p al-builtins --example gen_builtin_ids`
  (the test `builtin_ids_current` fails while `builtin_ids.rs` is stale).
- `./check.sh` before every commit: fmt, clippy -D warnings, all tests, and the
  corpus ratchet. It fails on the first error.
- Perf budgets (cold compile of the corpus ≤ 695 000 instructions per KiB of corpus source; the full corpus run with Microsoft's apps ≤ 83.2 G instructions for the Microsoft apps plus 4 210 000 per KiB of the corpus source it compiles; wall time printed for information only; parser ≥ 50 MB/s) are
  `#[ignore]`d tests that need release:
  `cargo test --release --no-fail-fast -p al-syntax --test perf_parse_throughput -p al-test-cli --test perf_budget -p al-test-cli --test perf_vm -p al-data --lib -- --ignored --nocapture`.
  CI runs them on a push to main and nightly, not on a pull request (owner, 2026-10-31). CI has no instruction counter, so there the CPU-time fallback only warns; with a counter (locally) the instruction budgets fail as before. `cargo run --release -p al-test-cli --example phases` prints
  timings per phase. `--example compile_dump` dumps every compile output, to
  diff before and after a performance change.
- Work ratchet: the full corpus run compares what each pass compiled and ran (routines lowered, objects loaded,
  dependency apps from source or cache, install baseline rows, tests run) with `metrics/<major>/work.json` and fails
  on a count more than 10 % above it; CI's corpus job also fails a step more than 1.5x main's median time
  (`docs/architecture.md`, "Work ratchet and step time guard"). A PR that adds work on purpose runs
  `./target/release/al-test-cli --root . --all-versions --update-work` and commits the file in the same PR.
- The install baseline is skipped when no selected test can observe it (ADR-013 note): after a change to `al-testrun/src/observe.rs`
  or the baseline, `tools/baseline_rule_check.sh run` must show 0 differences (`AL_FORCE_BASELINE=1` turns the skip off).
- Demand-driven against eager compile of the Microsoft apps (ADR-013; CI runs it on a push to main and nightly, not on a pull request): `tools/demand_vs_eager.sh run`
  (about 3 min, release binary needed) compares the main app and the OnPrem and isolation suites for BC 27, 28, 29.
- Core crates do no OS access (ADR-006).
- Parallel work: one `git worktree` and branch per area, inside the main
  checkout (`/.worktrees/` is git-ignored): from the main checkout run
  `git worktree add .worktrees/<area> -b <area> origin/main`. Never create a
  worktree next to the repository (`../wt-<area>`). Merge `main` into the branch
  before handing it back, and regenerate the baseline after merging; never
  hand-merge `metrics/<major>/passing.txt`.
- Logs and scratch files go to your session's scratch folder or under
  `~/.cache/al-runtime/`, never into the worktree root or
  `/home/stefan/Documents/Repos/community`.
- Shell hygiene: never build a file list with `$(grep -l …)` that can come back
  empty. `grep PATTERN` with no files waits on stdin forever; use `grep -rn
  PATTERN dir/` with a real folder (`crates/`, `docs/`): `grep -rn` on `.` in
  the main checkout also walks `.worktrees/` and `target/`.
- Working method for implementation agents (gates, memory caps, report):
  `docs/agents/brief.md`.
- A session on another machine that takes GitHub issues (setup and task loop):
  `docs/agents/remote-worker.md`.
