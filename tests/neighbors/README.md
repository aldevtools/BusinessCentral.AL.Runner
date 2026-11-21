# Neighbor apps

A neighbor app shows what a change does to the shapes next to the one its evidence pins. It runs on a binary of
`main` and on the head binary of the pull request. Every test whose outcome differs needs an entry that says why.
`tools/neighbor-diff.sh` runs it, locally and in the CI job `neighbor check` (owner decision 2026-11-21).

## The rule

A pull request that changes a `.rs` file under `crates/*/src/` (a pure rename does not count) adds or changes a folder
`tests/neighbors/<topic>/`. `<topic>` is the branch name or the issue. The folder holds one of:

- **A neighbor app**: `app.json`, `src/*.al` and `differences.tsv`.
- **`none.txt`**: one line that says why no neighbor app applies, for example "comments only".
  The reviewer checks the reason. A change of comments only, or a tooling change no AL code sees, takes this form.

CI fails when the rule fails, and when a test differs and has no entry. Only folders the pull request changes run.
A later pull request may delete old folders.

## The app

- One test per neighbor shape: each fact the new condition does not read (other caller, type, record state, trigger,
  extension, subscriber, sequence, object kind), each new refusal, and each reader of a new mark. Name each test for
  its shape.
- Keep it small and self-contained. Depend on Microsoft's apps only when the shape needs them.
- No Microsoft code. The legal guard checks it.
- It is not a pin. A behavior is still pinned by the corpus or runs through an assumption entry (`CLAUDE.md`, rule 3).

## differences.tsv

One line per test that differs, `classname::name`, a tab, and the entry. Lines starting with `#` are comments.
The key is the one the table prints in its first column.

```
# key	entry
LT PR Tests::PartRefill_ListHost_Last	ASSUMED PartRefillLeftView (corpus PR 1450)
LT PR Tests::PartRefill_Worksheet_First	PINNED tests/al-language/pages/PartRefill.al:120
LT PR Tests::PartRefill_SecondFilter	REFUSED First() on a part whose view no longer started with the rows it holds
```

An entry is one of:

- `PINNED file:line`: the corpus case that pins the head result.
- `ASSUMED id`: the assumption entry the head result runs through, with its open probe pull request.
- `REFUSED text`: the head refuses this shape on purpose. Name the refusal. The pull request body lists every shape
  that main answered and head now refuses, with the reason.

A difference is another status (pass, fail, not run, missing), another set of assumption ids (a clean pass that
became flagged), or another failure message. Another refusal text is not a difference.

## Run it

```
tools/neighbor-diff.sh --changed                     # what CI runs: the folders your branch changes
tools/neighbor-diff.sh path/to/app [--bc-version 28]  # any app folder
```

Default BC version: 29. Each run prints one table per app and version. Paste the tables into the pull request body.
Logs and JUnit files: the folder the last line names. Exit 0: every difference has an entry. Exit 1: one has none,
or the rule fails. Exit 2: a build or a run failed.

The main binary is the merge base of your branch and `origin/main`. Merge `origin/main` first. It comes from the
local cache, from the CI artifact of `main` (needs `gh`), or a local build, in that order.
