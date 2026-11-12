# Brief for agents

You work on AL Runner. `CLAUDE.md` has the hard rules. This file is the working method.
Your task prompt holds only the task. Read both files from YOUR worktree: the main checkout can be old.

The orchestrator starts agents from the definitions in `.claude/agents/` (`al-implementer`, `al-reviewer`,
`al-probe-writer`) plus a task-specific part: the area table, the issues, the branch and the session
attribution line. It does not hand-write the general rules into each prompt. A rule that review keeps
finding goes into `docs/agents/recurring-findings.md`, not into one prompt.

## Start
1. From the main checkout: `git fetch origin`, then `git worktree add .worktrees/<branch> -b <branch> origin/main`.
   In the worktree: `git submodule update --init`.
2. Run nothing in the main checkout. Every build, test and script runs in your worktree.
   Logs and scratch files go to your session's scratch folder, never into the repository.
3. Read `docs/codemap.md` for your crates and the ADR your task names. Do not read all docs.
   In a worktree, search with `grep -rn` on a real folder (`crates/`) and read with line ranges: the LSP tool's
   rust-analyzer indexes only the main checkout, so `findReferences` and call queries return nothing or wrong results
   in a worktree (measured 2026-11-11). Read-only work in the main checkout may use the LSP tool (`documentSymbol`,
   `findReferences`): it is exact and reads far less than grep, but the line and column must land exactly on the name.
4. Build once: `cargo build --release`. Use the binary. `al-test-cli --help` lists every flag.
5. Once per machine: `./target/release/al-test-cli --root . --all-versions --fetch-deps` fills the artifact cache with
   Microsoft's apps of the pinned builds (`~/.cache/al-runtime/artifacts`, shared by every worktree). Corpus runs and
   the Rust tests that read a Microsoft file take them only from there; a test never downloads and fails with this
   command when a file is missing. Run it again after `bc_build` in `oracle/oracle.toml` moves.

## Rules
1. **Cite, assume, or refuse** (CLAUDE.md rule 3, owner 2026-11-12). Every behavior you add or change
   has a corpus `file:line` in a code comment. A behavior the pinned corpus does not cover yet runs only
   through an assumption registry entry that cites its open probe PR (a flagged pass); your PR may land
   before that probe merges. No probe written means you guessed: refuse that path (lowering reason,
   `al-sema` diagnostic, `Signal::Unsupported`) and list it under "needs probe". A rule that also decides
   cases its evidence does not show (a wider range, "every type", "any count") is a guess for those cases.
   How to register an entry: `docs/agents/assumptions.md`.
2. **Whole area.** Your prompt has an area table: rows are every form, columns are types,
   compile/run and BC versions. Fill every cell with one of: PINNED file:line, ASSUMED id (open probe
   PR), REFUSED reason, PROBE QUEUED spec, OUT OF SCOPE. Fix the shared code path, not one caller.
   A fix that only makes the reported shape pass is not done.
3. **Refuse, do not half-run.** Never ship a partial behavior that runs and then fails.
   Refuse at the start of the shape, never after the test has already observed values.
   A case that was not run before and fails now is a regression.
4. **Red first.** Write the failing test, run it, keep the output for your report.
   A corpus case that pins the behaviour is the test: show it red, then green, and prune its not-passing line (CLAUDE.md rule 1).
   For an outside issue the first test is the reporter's repro, unchanged.
   Reproduce the reporter's own repro before you write `Fixes`.
5. **Leave existing tests alone.** Do not change the input or the expected value of an existing test
   to make it pass. If your change proves an old expectation wrong, the PR body states the reason
   and the corpus `file:line` that proves it.
6. **Numbers in harness tests are oracles.** If the runtime's count differs from BC's count, the runtime
   is wrong. Report it. Do not edit the number. Never edit `corpus/` or `oracle/`.
7. **You never start a workflow run.** No `gh workflow run`, no `gh run rerun`, no API dispatch, on any
   repo. Observe BC behavior locally with the orchestrator's observe tool. CI is only the PR's own run.
   A cell that depends on the handler routing of Microsoft's test runner (page 130455) needs the
   PR CI run: list it in your report under "needs CI observation".
8. **Never wait more than 20 minutes** on CI, a container or another agent. Write `RESUME.md` in your
   scratch folder (what you wait for, the command that continues), stop your monitors, hand back WAITING.
9. **Never `pkill` or `killall`.** Other agents run the same binaries. Stop only a PID you started.
10. **Never `git stash`.** All worktrees share one stash list. Commit work in progress on your branch.
11. **Never `git commit -a` or `git add -A`.** Stage paths by name. Check `git submodule status` first.
12. **Nothing local in the repo.** No host names, URLs, user names, home paths, partner names or
    Microsoft code in commits, pull requests or issues.
13. **Hand off at 250k tokens.** Commit, write your hand-off file (done, next steps in order, traps),
    report HANDOFF. Do not start sub-agents.
14. **Error texts come from the corpus.** Never invent one.
15. **Seams change only through an ADR. Caps and budgets never go up.** Stop that part and report it.
    Any other open decision: take the sensible default and list it in your report.
16. **Commit messages** say what changed and why, in plain American English. End them with the
    session attribution line the orchestrator gives in your prompt.

## Refusals (ADR-071)
A refusal from free text: `unsupported(text)` or `Signal::unsupported(text)`.
`Signal::Unsupported(format!(..))` does not compile. Do not add a private `fn unsupported`.
A new refusal may take a key: one file in `src/refusals/` of the crate that refuses (owner decision: one list per
crate; the file name is the key), then `unsupported(refuse!(keys::NAME, "text {name}", name = value))`, and
`al-test-cli --root . --refusals-doc`. Only `al-builtins` has a list today: a crate that starts one copies the
`refusal_keys` part of `al-builtins/build.rs`, calls `al_builtins::refusal_keys!`, and adds its row to `LISTS` in
`al-test-cli/src/refusal_keys.rs` (a test fails without it). A key that two crates use sits in the lowest crate both
depend on.

## Before you hand back, in this order
1. `git fetch && git merge origin/main`.
2. `./target/release/al-test-cli --root . --baseline-diff --bc-version N` for N = 27, 28 and 29
   (about 20 s each): 0 regressions, 0 cases without an outcome.
3. No new failure: no case may fail on your branch that passed or was not run on `origin/main`
   (how to list them: `docs/agents/checks.md`).
4. `--prune-not-passing` for every major. Commit the deleted lines.
5. After a corpus bump: regenerate the goldens
   (`UPDATE_GOLDEN=1 cargo test --release -p al-test-cli --test corpus_resolve_deps -- --ignored`)
   and check the `cargo test -p al-version` gate evidence for new BC28PLUS guards.
6. `cargo test -p <crate>` and `cargo clippy -p <crate> -- -D warnings` for each crate you touched.
   If you touched dependency loading, lowering, the checker, `observe.rs` or lazy package reading:
   run the check that `docs/agents/checks.md` names for it.
7. `cargo fmt --all` LAST, after every edit. Then `tools/prepush.sh`.
8. Push your branch. `gh pr create --draft`. No auto-merge. CI runs only the `test` job on a draft (owner, 2026-11-10). Do not wait for CI. Do not mark it ready.
   The orchestrator reviews it. A single reviewed PR is marked ready with auto-merge and lands on
   its own. Merge trains are only for two or more ready PRs.
9. Stop every monitor and background process you started.

## Pull request body and final report (the same content)
- Line 1: `[al-runtime-rs / <task> / DONE | WAITING | HANDOFF | BLOCKED]`
- `Fixes #N`, one per line, only when the reporter's repro passes with no flagged assumption on its
  path. Otherwise `Refs #N` and what is still refused.
- The area table.
- Red run and green run of the repro or the new tests.
- The numbers of step 2 for 27, 28 and 29, and the lines you pruned.
- Needs probe. Needs CI observation.
- Decisions you took without the owner.
Under 200 words plus the table.

## Moved code
- **Page state** (ADR-072): `PageState`, `PageMode`, `WindowRow`, `Cursor`, `ReadBackwards`, `ControlRef` and `ControlTarget`
  live in `crates/al-vm/src/pages/state/mod.rs`, not in `crates/al-builtins/src/value.rs`. A branch that added a field in
  `value.rs`: after you merge `main`, put the field into `state/mod.rs`. It starts at its type's default; `PageState::closed`
  gets a line only for another starting value. `Value::TestPage` and
  `Value::TestControl` hold a handle: read it with `st_of` / `control_of`, build a value with `page_value` / `test_control`.

## Other files
- `docs/agents/assumptions.md`: register an unprobed behavior.
- `docs/agents/checks.md`: checks by area, listing new failures.
- `docs/agents/bump.md`: corpus bumps, generated files, bump bot.
