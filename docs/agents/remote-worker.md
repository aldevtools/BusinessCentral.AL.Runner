# Remote worker: setup and task loop

For a Claude Code session that runs on another machine and takes tasks from
GitHub issues. It works on a branch, pushes the branch, and comments a report
on the issue. The orchestrator on the main machine reviews and merges.
Read `docs/agents/brief.md` first. It holds the rules for the work itself.

## Prerequisites

- **Rust 1.98.1.** CI pins this version. Install it with
  `rustup toolchain install 1.98.1 --component rustfmt clippy`. Then run
  `rustup override set 1.98.1` in the clone, or use `cargo +1.98.1`.
- **.NET 8 SDK.** The runtime starts a .NET 8 helper for AL code that uses the
  `DotNet` type. The helper builds itself on first use when the SDK is present.
  Without .NET, tests that reach a `DotNet` call are refused, and
  `--update-baseline` is refused too (the new baseline would be incomplete).
  You cannot finish a task that changes the passing set without .NET. Install
  it.
- **git, `gh`, `python3`.** `gh auth status` must show a login with push
  rights to `aldevtools/BusinessCentral.AL.Runner`. `python3` is needed by
  `tools/demand_vs_eager.sh`.
- **Disk.** About 10 GB free. Microsoft artifacts download automatically to
  `~/.cache/al-runtime` (override with `ALRUNNER_CACHE`). Each worktree also
  has its own `target/` directory of several GB. Remove a worktree after its
  branch is pushed.
- **Memory.** Keep a corpus run under about 3 GB (see "Memory and threads" in
  the brief).
- **Operating system.** Linux, macOS and WSL work. `check.sh` is plain bash
  and cargo. `systemd-run` exists only on Linux with systemd. Elsewhere, use
  `timeout` and watch memory. Native Windows is not supported.
- **No BC container.** Probes run on the main machine. Do not attempt them.

## Existing clone from before the move (2026-10-21)

The repositories moved to the GitHub organization `aldevtools`. The old
`StefanMaron/...` URLs still redirect for git, but switch a clone made before
the move once:

1. `git remote set-url origin https://github.com/aldevtools/BusinessCentral.AL.Runner`
2. `git submodule sync` (the corpus URL in `.gitmodules` changed), then
   `git submodule update --init`.
3. In a corpus clone:
   `git remote set-url origin https://github.com/aldevtools/BusinessCentral.AL.Language.Tests`
4. In a bc-linux clone:
   `git remote set-url origin https://github.com/aldevtools/MsDyn365Bc.On.Linux`
5. Check: `git fetch` in each clone prints no redirect warning.

Worktrees share the clone's settings and need nothing else.

## First run

1. Clone with the corpus submodule:
   `git clone --recurse-submodules https://github.com/aldevtools/BusinessCentral.AL.Runner`
2. `cd BusinessCentral.AL.Runner`, then fetch the corpus history, which the pin check
   needs: `git -C corpus fetch --unshallow` (skip it if the command says the
   repository is complete).
3. `cargo build --release`
4. `./target/release/al-test-cli --root .`
   The first run downloads Microsoft artifacts and takes longer.
5. Compare the counts with the table in `docs/manual/what-works.md` ("Status") and
   `docs/status.md`. On `main` today every BC version (27.5, 28.5 and 29.0)
   shows 0 failing, and the same cases are not run on each. If your numbers
   differ, that table may lag behind `main`. The ratchet is the check: the
   exit code must be 0. Exit code 1 means a baseline case no longer passes,
   and 2 is a setup error. Fix setup problems before you take any task.
6. `./check.sh` must pass on a clean `main`. It takes minutes.
7. Create the labels if they are missing, once:
   `agent-ready`, `in-progress`, `ready-for-review`, `blocked`,
   `found-by-agent` (`gh label create NAME`).

## The task loop

Work on one issue at a time. Repeat until no issue is left.

1. **Find the oldest open issue** that is ready and not taken:
   `gh issue list --state open --limit 50 --search 'label:agent-ready -label:in-progress -label:ready-for-review -label:blocked sort:created-asc' --json number,title,labels`
   Take the first one. If the list is empty, stop and say so.
2. **Claim it.**
   `gh issue edit N --add-label in-progress`
   `gh issue comment N --body "Taking this."`
   Add one line if useful, such as where you start.
3. **Read the issue and the docs** named in `docs/agents/brief.md`. If the
   issue is unclear, comment one specific question, label it `blocked`
   (see "If you are blocked") and move on.
4. **Make a worktree and branch** from current `origin/main`, inside the clone
   (`.worktrees/` is git-ignored; never next to the clone). Keep logs and
   scratch files out of the clone and its parent folder. The branch name is
   `remote/<issue-number>-<slug>`, where the slug is two to five lowercase
   words from the title:
   ```
   git fetch origin
   git worktree add .worktrees/N-slug -b remote/N-slug origin/main
   cd .worktrees/N-slug
   git submodule update --init
   ```
   If a branch `remote/N-...` already exists on origin (for example after a
   review round), continue on it instead: merge `origin/main` into it first.
5. **Do the work** under `docs/agents/brief.md`: red/green TDD, no guessing,
   refuse what you cannot pin, never weaken an oracle.
6. **Run the finishing gates** from the brief, in order: baselines for 27, 28
   and 29, `./check.sh`, the release-only tests, the demand/eager differential
   when it applies, then merge `origin/main`, regenerate the baselines and run
   `./check.sh` again. All must be green.
7. **Push the branch only:** `git push -u origin remote/N-slug`.
   Never push `main`. Never merge. Never open the merge yourself.
8. **Report on the issue** (format below).
   `gh issue edit N --add-label ready-for-review --remove-label in-progress`
9. **Clean up:** `cd` back to the clone and `git worktree remove .worktrees/N-slug`.
   Then go to step 1.

### Report format

Post one comment per finished issue. Keep it factual and short. Use plain
American English, first person, no marketing words.

```
Branch: remote/N-slug, last commit <hash>.

Changed
- <what changed, one line each, with the module or file>

Counts (passed / failed / not run), before -> after
- BC 27.5: 4913 / 0 / 234 -> ...
- BC 28.5: ...
- BC 29.0: ...
No baseline case regressed: yes/no.

Gates
- check.sh: green
- release-only tests: green
- demand/eager differential: green / not needed (reason)

Decisions taken without the owner
- <decision, and where it is recorded (ADR section, docs/owner-review.md)>

Probes needed
- <assumption added to docs/probes-needed.md, and the probe wanted>, or "none"

Review first
- <the file or change a reviewer should read first>
```

The counts come from `al-test-cli --root . --all-versions` on `main` before
you start and on your branch at the end.

### If you are blocked

Examples: the issue needs a probe, an ADR decision about a seam, a change to an
oracle, or something you cannot reproduce.

1. Comment why, in a few lines: what you tried, what is missing, what you
   need from the owner.
2. `gh issue edit N --add-label blocked --remove-label in-progress`
3. If you pushed partial work, say the branch name in the comment.
4. Go to the next issue.

## What not to do

- No probes that need a BC container.
- No pushes to `main`. No pushes to the corpus repo. No edits in `corpus/` or
  `oracle/`.
- No merges, and no closing of issues. The orchestrator closes an issue after
  it merges the branch.
- No new issues, except for a bug you found while working. Label those
  `found-by-agent`. Say in the issue how to reproduce it.
- No force pushes. No rewriting of history on a branch that is already pushed.
- No attribution or session lines in commit messages or comments.
- No Microsoft code and no third-party project code in the repository.
- Public comments stay factual and short. No promises, no praise, no
  speculation about the owner's plans.
- If a command asks for a secret or a login you do not have, stop and comment
  on the issue. Do not look for credentials.

## Starting prompt

Paste this into the session on the laptop, from the repository clone. It uses
`/loop` without an interval, so the session paces itself.

```
/loop Work through the GitHub issues of aldevtools/BusinessCentral.AL.Runner, one at a time, following docs/agents/remote-worker.md. Before the first issue, read docs/agents/remote-worker.md, docs/agents/brief.md, CLAUDE.md, docs/architecture.md, docs/status.md, docs/codemap.md and docs/adr/README.md. Each round: find the oldest open issue with label agent-ready and without the labels in-progress, ready-for-review and blocked; claim it (label in-progress, comment "Taking this."); make a worktree and branch remote/<number>-<slug> from origin/main; do the work with red/green TDD under the rules in docs/agents/brief.md; run all finishing gates (baselines for BC 27, 28 and 29, ./check.sh, the release-only tests, tools/demand_vs_eager.sh run when it applies, merge origin/main, regenerate baselines, ./check.sh again); push the branch only; comment the report in the format of docs/agents/remote-worker.md on the issue; add label ready-for-review and remove in-progress; remove the worktree; then take the next issue. If an issue is blocked (needs a probe, an ADR decision, an oracle change, or is unclear), comment why, label it blocked, remove in-progress, and move on. Never push main or the corpus repo, never merge, never close issues, never edit corpus/ or oracle/, never weaken a baseline, run corpus runs one at a time under a memory cap or timeout, and keep public comments factual and short. Open a new issue only for a bug you found or for self-picked work when the queue is empty, with label found-by-agent. Never stop the loop. If an issue you labelled in-progress is still open from an earlier round, resume it first. When no matching issue is left: (1) fetch origin, run `al-runner test` on Tests-ERM of the BC 27.5 artifact with `--top-refusals 40` (the plain command loads test data), (2) pick the highest-ranked refusal that no open issue and no recent commit on main mentions, (3) open an issue for it with labels agent-ready and found-by-agent (goal, facts with counts, scope, done-when, as in the other issues), and take it. If even that finds nothing, schedule the next round in 15 minutes and look again. Each round ends with the next round scheduled, never with a stop.
```

## Corpus writer mode (from 2026-10-15)

Owner decision: the remote worker writes corpus probes full time, one whole topic at
a time (every method and overload of a type, with edge cases and error texts), so
later runtime fixes find their pin already there. The working method is
`docs/agents/corpus-writer.md`: a local BC 29 container, observe then assert, your own
corpus branch `al-runtime-rs-writer-laptop`, ids 70000-74999, names prefixed `LT `.
GitHub CI of the corpus repo is only the final check for BC 27 and 28.
The runtime pins the corpus default branch `master` (since 2026-10-19). Only the
publisher moves it, through a pull request from staging ("How a bump lands on
master" in `docs/agents/corpus-writer.md`).

Topic issues carry the label `corpus-topic`. Each round: take the oldest open
`corpus-topic` issue without `in-progress`, `ready-for-review` or `blocked`; claim it;
write and measure the probes locally until green on BC 29; merge
`al-runtime-rs-staging` into your branch; push your branch; dispatch the corpus
matrix on it; comment the report (topic, members pinned with file names, members
left and why, findings that surprised you, the CI run) and label `ready-for-review`.
The publisher (a local agent) merges your branch into staging. When no topic issue is
left, take the next type from `docs/corpus-coverage.md` (pinned X of Y per type),
open a `corpus-topic` issue for it labelled `found-by-agent`, and take it.

Starting prompt for this mode:

```
/loop Write corpus probes for aldevtools/BusinessCentral.AL.Runner, one topic at a time, following the "Corpus writer mode" section of docs/agents/remote-worker.md and docs/agents/corpus-writer.md. Before the first topic, read both files, CLAUDE.md and docs/probes-needed.md, and set up the local BC 29 container as the guide says. Each round: take the oldest open GitHub issue with label corpus-topic and without in-progress, ready-for-review and blocked; claim it (label in-progress, comment "Taking this."); pin every member of the topic on the local BC 29 container with observe-then-assert, asserting only the measured facts; merge al-runtime-rs-staging into your branch al-runtime-rs-writer-laptop; push that branch only; dispatch the corpus matrix on it; comment the report; label ready-for-review and remove in-progress; take the next topic. If an issue you labelled in-progress is still open, resume it first. When no topic issue is left, open one for the type with the fewest pinned members in docs/corpus-coverage.md (labels corpus-topic and found-by-agent) and take it. Never push master or al-runtime-rs-staging of the corpus repo (master is the branch the runtime pins; it takes pull requests only), never edit or delete an existing corpus test, never change al-runtime-rs code, and keep public comments factual and short. Never stop the loop; if blocked, comment why, label blocked, and move to the next topic.
```
