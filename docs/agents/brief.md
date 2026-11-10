# Brief for implementation agents

The rules every agent follows when it works on al-runtime-rs, whether it runs
on the main machine or as a remote worker (`docs/agents/remote-worker.md`).
`CLAUDE.md` holds the hard rules. This file adds the working method.

## Before you start

1. Read `CLAUDE.md`, `docs/architecture.md`, `docs/status.md`, `docs/codemap.md`,
   `docs/adr/README.md` and every ADR for your area. The hard rules in
   `CLAUDE.md` are binding.
2. Work in your own worktree and branch, never in the main checkout. Create it
   inside the main checkout, from the main checkout's root:
   `git worktree add .worktrees/<area> -b <branch> origin/main`
   (`/.worktrees/` is git-ignored). Never create a worktree next to the
   repository (`../wt-<area>`). If the worktree already exists, continue in it.
   Logs and scratch files go to your session's scratch folder or under
   `~/.cache/al-runtime/`, never into the worktree root or the folder that
   holds the repository.
3. In a new worktree, run `git submodule update --init`. The corpus pin check
   needs the corpus history, so also run `git -C corpus fetch --unshallow` if
   `git -C corpus rev-parse --is-shallow-repository` prints `true`.
4. Build once: `cargo build --release`. Then use the binary, not `cargo run`.

## How to work

- **Tests of the interpreter use `al-testkit`** (`crates/al-testkit`, a dev-dependency): `al_testkit::run(src, routine)`,
  `run_in(src, object, routine)`, `Built` for several calls on one session, and `text`/`int`/`boolean`/`error_message` to
  read a result. Do not write another compile-and-call helper. The tests of `al-vm` are five area binaries
  (`tests/{store,pages_ui,reporting,language,host}/main.rs` with one module per file) plus two that need a process of
  their own (`dotnet.rs`; `base_access_refused.rs` sets an environment variable): a new test file is a module of its area.
  The tests of `al-test-cli` and `al-testrun` are one binary each (`tests/suite/main.rs`); a new test file is a module
  there unless it must run by name (`--test NAME`), needs a feature, or needs a process of its own (see the head of
  each `main.rs`).
- **Keep files small.** `file_size_ratchet` (part of `cargo test -p al-test-cli`) fails when a Rust file under
  `crates/*/src` passes 3,000 lines, or when a file listed in `metrics/file-sizes.toml` grows past its cap.
  Put new code in a new module, not in a listed file. Never raise a cap or add a row: split the file (see
  `docs/codemap.md`) and lower the cap. Generated files and `crates/*/tests` are exempt.
- **Red/green TDD for every behavior.** Write the failing test first. Keep the
  red run output for your report. A bug fix starts with a test that reproduces
  the bug.
- **No guessing at semantics.** Cite the corpus test (`file:line`) that pins a
  behavior in a code comment. If nothing pins it, refuse explicitly and add the
  assumption to `docs/probes-needed.md`, with the probe you would want. Do not
  implement the guess. Error texts come from the corpus and are never invented.
  One exception (owner, 2026-10-15): a behavior that a Microsoft test needs in
  order to pass on real BC may be implemented before its probe. Cite that
  Microsoft test (app, codeunit, test name) in the code comment, add the
  `docs/probes-needed.md` row, and say so in your report. The probe wins when it
  lands. This is evidence, not a guess: a behavior no test requires is still refused.
  The code path must mark the assumption when it runs (its probes-needed row id),
  so a test that passes through it is reported as a flagged pass, never as a clean one.
  How: add an entry (id, assumed behavior, Microsoft tests, probe wanted) as a new file of its own,
  `crates/al-builtins/src/assumptions/<id>.rs`, holding `YourId => { id: "<id>", assumed: ..., tests: ...,
  probe: ... }` (copy any file there; the file name must be the id; `build.rs` collects the files, so two
  branches that each add one do not conflict), and call `assumptions::assume(Assumption::YourId)` on the
  path (at runtime, or `Lowerer::assume` in `al-sema/src/lower.rs` for a compile-time choice, which becomes
  an `ir::Stmt::Assume`). Mark only the part no corpus test pins. Regenerate the table in
  `docs/probes-needed.md` in the same pull request (`al-test-cli --root . --assumptions-doc`); a test fails while
  it is stale (see "Generated files" below).
  When a probe pins it, delete the entry's file: the compiler lists the marker sites that remain.
  `al-runner test --top-assumptions` ranks the entries by flagged passes; `--list-assumptions`
  prints the registry with the counts of the last run (also in `al-test-cli`).
- **A runtime fix for a user issue from a repro** (FAST FLOW: the corpus pin is not on `master` yet)
  registers its assumption with `awaiting: Awaiting { issues: &[N], files: &["tests/al-language/<area>/<file>.al"],
  since: "YYYY-MM-DD" }`: the al-runner issue and the corpus file(s) the writer is adding. A test fails when an
  entry names an al-runner issue without it. When the fix implements what the reporter asked, the PR
  says `Fixes #N` even though its passes are flagged (owner, 2026-10-28: close implemented issues, they can be
  reopened); `Refs #N` only while cases of the issue are still refused. Once every awaited file is in the pinned corpus, `al-test-cli --root .
  --awaiting-assumptions` (run by `cargo test`, `check.sh` and `tools/corpus_bump.sh`) fails until the entry is
  removed and the pin cited, or the code is fixed where the pin disagrees. `--awaiting-assumptions --summary`
  prints one status line, the oldest entry first.
- **One build, one result.** A corpus run must give each case the same outcome whatever the
  thread counts, the run order and HOME hold (`corpus_harness.rs`
  `outcomes_do_not_depend_on_threads_or_home` checks it). Before you call two runs "the same
  build", compare their `binary:` lines: a rebuild after a golden file was regenerated (for
  example `al-builtins/src/pinned_columns.rs`) changes what is refused.
- **Unsupported is refused explicitly.** Use a lowering reason, a diagnostic in
  `al-sema`, or `Signal::Unsupported`. Never run a guessed or partial behavior
  silently.
- **Never weaken an oracle.** Never edit `corpus/` or `oracle/`. Never add a
  line to `metrics/<major>/not-passing*.txt` for a case the oracle already had
  (ADR-065): that hides a regression. The one exception is a case shown to have
  passed by accident, with the owner's OK, in its own commit that names each
  case. If real BC and the corpus disagree, say so in your report; the owner
  opens the corpus issue.
- **Seams change only through an ADR.** Additive changes (a new `Value`
  variant, an IR field, a defaulted `Ctx` method) need no ADR. Core crates do
  no OS access (ADR-006).
- **Keep functions short and core crates pure.** `fn_length_ratchet` and `os_access_ratchet` (part of
  `cargo test -p al-test-cli`) fail when a `fn` under `crates/*/src` passes 250 code lines, when a function
  listed in `metrics/fn-lengths.toml` grows past its cap, or when a core-crate file gains a place that names an
  OS API (environment, files, processes, network; ADR-006) beyond its count in `metrics/os-access.toml`.
  Extract steps into functions of their own; pass settings in as parameters or through `al-host` instead of
  reading the environment. Never raise a cap or add a row; when a listed entry shrinks, lower or delete its row.
- **Do not build features that only hide start-up or run cost** (watch mode,
  warm server state, `--jobs`, persistent caches). See ADR-016. `--affected` is the owner's exception (ADR-050). The one cache the
  owner asked for is the compile cache of the dependency apps (ADR-022); do not add another.
- **Performance.** The goal is a corpus run with Microsoft's apps in under one
  second, then lower. A cache is the last resort. Never add a cache to reach a
  number; make the work itself cheaper. Measure before and after any change on
  a compile or run path (`al-test-cli --timings`).
- **Nothing from Microsoft and nothing from other projects in the repo.**
  Never commit Microsoft source, packages or downloaded artifacts. Never
  commit code copied from a third-party project, including AL Runner v2. You
  may read such code for guidance. You may not paste it.
- **Probes need real BC.** You have no BC container. Do not try to run one.
  Record what a probe must show in `docs/probes-needed.md` and refuse the
  behavior until the owner's probe result arrives.
- **A probe that passed on real BC is enough to implement and merge.** When a probe
  sits on the corpus branch `al-runtime-rs-staging` (or a writer branch) and passed
  on real BC (the local BC 29 container or the corpus CI), implement against it and
  cite the staging file. The branch may merge into `main` before the corpus bump;
  the bump then turns the cited behavior into a ratcheted corpus case. Do not wait.
- **Corpus branches.** The `corpus` submodule pins a commit of the corpus default
  branch `master` (since 2026-10-19; before that the side branch `al-runtime-rs`).
  Probes land on `master` only through a bump pull request from `al-runtime-rs-staging`
  (`docs/agents/corpus-writer.md`, "How a bump lands on master"). Never push to either
  branch yourself.
- **A corpus bump shrinks `docs/probes-needed.md`.** Whoever bumps the corpus
  deletes every row the new cases pin, and reports the table row count before
  and after.

## When the owner is not available

Work does not stop for an owner decision. Pick the sensible default, using AL
Runner v2 behavior as guidance. Then:

1. Record the decision in the relevant ADR under
   "Decided without owner review (<date>)".
2. Add one line to `docs/owner-review.md`.
3. Never weaken an oracle or a hard rule to make progress. A decision that
   needs a seam change is not yours: write the ADR as Proposed, stop that part
   of the task, and say so in your report.

## Code navigation: read less text

Large files cost many tokens (`lower.rs` about 4700 lines, `pages.rs` about
3700, `check.rs` and `runtime.rs` about 2300 each).

1. Read `docs/codemap.md` first. It says which module owns what.
2. Use the LSP tool (rust-analyzer). Load it once with ToolSearch
   `select:LSP`. Then:
   - `documentSymbol` for the outline of a file, before you read it.
   - `workspaceSymbol` to find a definition anywhere.
   - `goToDefinition`, `findReferences`, `incomingCalls`, `outgoingCalls` and
     `hover` instead of grep plus reading.
   The first query in a new worktree waits for indexing (about a minute).
3. Read with `offset` and `limit`. Never read a file over 500 lines whole.
4. Keep tool output small: pipe through `head`, use `grep -c`, and use the
   `al-test-cli` selection and report flags instead of dumping JUnit XML.

## Fast loop

A full corpus run takes about 20 s with Microsoft's apps, and `check.sh` takes
minutes. Do not iterate on either. `al-test-cli --help` lists every flag.

1. Find work: `./target/release/al-test-cli --root . --top-refusals 15` ranks
   the refusal reasons of the not-run cases, with one example test each.
   `--top-refusals-area FOLDER` does it for one corpus folder.
2. Understand one test: `--explain TestName` prints its outcome, the refusal
   reason with routine and `file:line`, and for a failure the error, code and
   stack. Add `--codeunit ID` if the name is not unique.
3. Iterate: select with `--codeunit ID`, `--test SUBSTR`, `--area FOLDER` or
   `--file SUFFIX` (each repeats; the union is used). Only the selected test
   codeunits compile and run, and only they are checked against the lists. Add
   `--no-deps` (about 0.5 s) when the test needs no Microsoft app. A selected
   codeunit runs whole, because its tests share state.
4. Before you commit: one run with `--baseline-diff` on the newest major only
   (about 20 s; no `--all-versions` run locally). It shows, by area,
   the regressions (not listed, not passing), the stale lines (listed cases that
   pass, or left the oracle), the cases still listed and the cases without an
   outcome. Then `--prune-not-passing` deletes the stale lines.
5. `--timings` shows where a run spends its time.

Do not write ad-hoc scripts that parse JUnit XML. If a view of a run is
missing, add a flag to `al-test-cli`.

## Memory and threads

A corpus run must stay under about 3 GB. One uncapped run once reached 14 GB.
A run that hits a cap is a bug in your change (unbounded recursion, a loop
that never ends, a snapshot per iteration). Find it with a unit test before
you run the corpus again.

- **Linux with systemd:** run under a cap:
  `systemd-run --user --scope -q -p MemoryMax=3G ./target/release/al-test-cli ...`
  Use 4 to 6 GB for eager runs or runs with test data.
- **On the owner's main machine, several agents share 31 GB.** On 2026-10-14
  their builds and runs together pushed the machine out of memory, and
  systemd-oomd killed the owner's terminal with every session in it. There, put
  ALL heavy work in the shared slice `al-work.slice` (capped at 20 GB in total),
  so the kernel kills one of your processes instead of the desktop: the corpus
  and bucket runs, and also `cargo build`, `cargo test` and `./check.sh`:
  `systemd-run --user --scope -q --slice=al-work.slice -p MemoryMax=6G cargo build --release ...`,
  `systemd-run --user --scope -q --slice=al-work.slice -p MemoryMax=8G ./check.sh`.
  Keep `CARGO_BUILD_JOBS=4`. If the slice does not exist (`systemctl --user
  show al-work.slice -p MemoryMax` shows `infinity`), say so in your report
  and run as before. A process killed inside the slice is not a bug in your
  change by itself: rerun it once before you look for a leak.
- **Everywhere else (macOS, WSL without systemd):** keep a run under about
  3 GB, watch it, and always use `timeout`, for example
  `timeout 600 ./target/release/al-test-cli ...`.
- Do not use `prlimit --as` or `ulimit -v` on runs that use Microsoft's apps.
  They also cap the .NET helper process. CoreCLR then fails to start ("Failed
  to create CoreCLR, 0x8007000E") and every DotNet test is refused. That looks
  like many regressions and is not.
- Never run several corpus runs at once. Never run `cargo-mutants` with more
  than `-j 1`.
- Set thread counts to fit the machine. The runtime uses one worker thread per
  core by default. On a shared or small machine use, for example:
  `CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4 AL_CHECK_THREADS=4 AL_LOWER_THREADS=4 AL_TEST_THREADS=4`
  Use fewer than 4 on a machine with fewer than 8 cores.
- Timing measurements need a quiet machine. Note the load average with every
  number you report.

## Generated files

No bot writes to `main` (ADR-065). A pull request keeps these files current
itself, and CI fails while they are stale:

- `metrics/<major>/not-passing*.txt`, the lists of the cases the runtime does
  not pass yet. Delete the lines of the cases your change makes pass
  (`al-test-cli --root . --prune-not-passing`; it never adds a line). A line
  for a case the oracle already had is a regression and needs the owner's OK
  (CLAUDE.md hard rule 2); CI shows it in the job summary.
- The assumptions table in `docs/probes-needed.md`
  (`al-test-cli --assumptions-doc`), when you change the registry. The text
  outside the table is still yours to edit.

The coverage maps (`docs/corpus-coverage.md` and `.json`, `--coverage-map`;
`docs/corpus-behavior-coverage.md` and `.json`, `--behavior-coverage-map`) are
not committed. Make them locally in seconds, or take the artifact
`coverage-maps-<run id>` of a CI run on `main`.

The corpus bump bot (`.github/workflows/corpus-bump-bot.yml`,
`tools/corpus_bump_bot.sh`) follows the corpus: every 20 minutes it looks for
a green corpus run newer than the pin and opens a pull request on a branch
`corpus-bump/<commit>` with the pin, `oracle/`, the harness counts and the
not-passing lists (lines for new cases that do not pass, `--list-new-cases
HEAD`; stale lines deleted). It never adds a line for a case the old oracle had
and does not merge. Its pull request body lists what is left for a person
(regressions, new failures, landed awaiting
assumptions, guards without a gate, harness counts, stale golden files). Fix
those on its branch; the bot never pushes to a branch that exists.

A bump need not wait for the corpus master run (about 55 minutes after a corpus
pull request merges; owner, 2026-11-05): `tools/corpus_bump.sh --from-pr
N[,N...]` moves the pin to the merge commit of those corpus pull requests and
replaces, in the current oracle, only the codeunits their own CI runs ran (a
subset run per BC version) with those runs' results. It refuses when a run is
not green, a BC version is missing, a pull request changed a shared fixture
(`_fixtures`, the xapp apps, an `app.json`) or deleted a test file, master has
a commit since the pin that is not one of the listed pull requests, or it
cannot show the run tested the merged files. `--dry-run` prints what it would take; `--body FILE` writes the pull
request body, which names the corpus runs the oracle came from and says it is
a PR-results bump. `oracle/README.md` keeps a list of these bumps until the
next bump from a master run, which replaces every oracle file.

These are still regenerated in the pull request, because the build or a
reviewed list depends on them, and CI fails while they are stale:

- `crates/al-builtins/src/builtin_ids.rs`
  (`cargo run -p al-builtins --example gen_builtin_ids`): the VM dispatches on
  these ids, so a new builtin does not work without them.
- `crates/al-builtins/src/pinned_columns.rs` and `catalog/signatures.txt`
  (`UPDATE_GOLDEN=1`): compiled into the runtime; a corpus bump regenerates them.
- `crates/al-sema/tests/expected-unresolved.txt`: a reviewed list that never
  grows without a decision.

## Finishing

Run these in order. Do not skip any. PR CI is the full gate (owner,
2026-11-05): it runs the full corpus on every major in about 6 to 12 minutes.
Do not run `./check.sh`, `cargo test --workspace` or an `--all-versions` corpus
run locally; on the shared machine they cost up to an hour.

1. Run `./target/release/al-test-cli --root . --baseline-diff` once on the
   newest major: no regression and no case without an outcome. Then
   `./target/release/al-test-cli --root . --prune-not-passing` deletes the lines
   of the cases your change makes pass; commit them (see "Generated files"
   above).
2. Run `cargo test -p <crate>` and `cargo clippy -p <crate> -- -D warnings`
   for each crate you touched, and `cargo fmt --all`. PR CI runs the rest
   (`check.sh`: all tests and the not-passing lists of every major).
3. Run the release-only tests that cover your change. CI runs them nightly:
   `cargo test --release --no-fail-fast -p al-syntax --test perf_parse_throughput -p al-test-cli --test perf_budget --test perf_vm --test corpus_resolve_deps -p al-data --lib -- --ignored`
   If the golden list of unresolved items changes, review the change and
   justify it in your report before you regenerate it with `UPDATE_GOLDEN=1`.
4. Run the demand/eager differential if you touched dependency loading,
   demand-driven compile, lowering, the checker or anything a Microsoft app
   reaches. When unsure, run it:
   `tools/demand_vs_eager.sh run`
   It takes about 3 minutes and needs `python3`. It compares a normal run with
   an eager run (`AL_EAGER_DEPENDENCIES=1`). Any difference fails, and
   `tools/cache_vs_uncached.sh run` does the same for the compile cache. Owner
   decision of 2026-10-14: there are zero differences.
   `tools/demand-eager-known-differences.txt` is empty and stays empty; never
   add a line to make a run pass. Find the cause: `AL_BLOCK_PATH=1` prints the
   call chain from a test to the routine that refuses it, and
   `AL_DUMP_REACH=FILE` writes what the demand-driven compile reached, `AL_STACK_OF=TEXT` prints the routines a not-run test was refused in (for a
   diff between two runs). A static refusal may depend on the sources only,
   never on what was compiled or on which other tests are selected.
   If you touched `al-testrun/src/observe.rs` or the baseline, also run
   `tools/baseline_rule_check.sh run`: the rule that skips the install baseline
   must give zero outcome differences. If you touched the lazy reading of dependency packages
   (`al-deps/src/lazy.rs`, `Workspace::load_apps_lazy`), run `al-runner test` on Tests-ERM twice, once with
   `ALRUNNER_EAGER_SOURCES=1`, and compare the two `--junit` files with `tools/compare_junit.py`: 0 differences.
5. No corpus case may newly fail. A case that was not run and now fails counts
   as wrong behavior (hard rule 4); the lists do not catch it (a listed case may fail or be refused). Run
   `al-test-cli --root . --bc-version 27 --junit FILE` on `origin/main` and on
   your branch, and list every case that fails on the branch but not on main.
   Fix each one or refuse its path. Before each commit, check
   `git submodule status` (the corpus must stay at the oracle's
   `corpus_commit`), and stage explicit paths: never `git add -A` or
   `git commit -a`.
6. Commit on your branch. Do not add attribution or session lines to commit
   messages. The message says what changed and why, in plain American English.
7. Merge current `origin/main` into your branch (`git fetch`, then
   `git merge origin/main`). Resolve conflicts. In a not-passing list take the
   deletions of both sides, then run `--prune-not-passing`. Commit.
8. Do not merge into `main`. Whether you push is set by your task: a local
   agent does not push, a remote worker pushes its own branch and nothing else.
9. Before every push, also a push of a fix after review, run `tools/prepush.sh`.
   It runs the fast checks that most often failed in CI after a green local run:
   `cargo fmt --check`, the version-literal and gate-completeness tests of
   `al-version`, the assumption registry tests, the `builtin_ids_current` test, and
   clippy with `-D warnings` on the crates your branch changed. With a warm target
   folder it takes under a minute. Push only when it ends with "all checks passed".
   It does not replace PR CI, and it installs no git hook; `tools/prepush.sh -h`
   shows how to add one yourself.
10. Open the pull request as a draft and without auto-merge (owner,
    2026-11-09): CI runs only the `test` job on a draft (2026-11-10), so it takes little of the CI
    queue. Do not wait for CI and do not mark it ready. The orchestrator reviews
    it and lands it in a merge train; the train's pull request runs CI once for
    several drafts and is the gate.

## Report

Your final message or issue comment has these parts:

- Branch and final commit.
- Passed, failed and not run, before and after, for each BC version (27, 28,
  29). Confirm no regression against the not-passing lists, and name the lines you deleted.
- What you implemented, with the corpus files that now pass.
- What is still refused or failing in your area, with the reason and counts.
- Gates run (crate tests, release-only tests, demand/eager, PR CI) and their result.
- Assumptions you added to `docs/probes-needed.md`, and the probes you want.
- Decisions taken without the owner, and what a reviewer should look at first.
