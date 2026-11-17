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
  ratchets (see `docs/agents/workflow.md`, "Ratchets"). Never raise a cap; split, extract or delete instead.
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

Commands, flags and reasons: `docs/agents/workflow.md`. The quoted names below are its sections.

- Build once: `cargo build --release`, then use `./target/release/al-test-cli --root .` ("Build and run").
- Iterate on one codeunit, area or file. Before committing: `--baseline-diff` and `--prune-not-passing` on BC 27, 28, 29 ("Fast loop").
- PR CI is the gate. Locally run only `cargo test -p`, `cargo clippy -p ... -- -D warnings` and `cargo fmt --all` ("PR CI is the gate").
- Pull requests are drafts, without auto-merge. Reviewed PRs land through a merge train ("Draft pull requests and merge trains").
- Never run `pkill`, `git stash`, `git commit -a` / `git add -A`. Never start a workflow run ("Commands agents never run").
- File size, function length and OS access caps only go down. Never raise a cap or add a row ("Ratchets").
- Core crates do no OS access (ADR-006).
- One worktree and branch per area, under `.worktrees/` of the main checkout, never next to it ("Worktrees").
- Logs and scratch files go to your scratch folder or `~/.cache/al-runtime/`, never the repo root ("Logs and shell hygiene").
- Working method: `docs/agents/brief.md`. What review keeps finding: `docs/agents/recurring-findings.md`.
