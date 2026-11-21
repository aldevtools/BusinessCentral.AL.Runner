# Neighbor apps

A neighbor app shows what a change does to the shapes next to the one its evidence pins. It runs on a binary of
`main` and on the head binary of the pull request. Every test whose outcome differs needs an entry that says why.
`tools/neighbor-diff.sh` runs it, locally and in the CI job `neighbor check` (owner decision 2026-11-21).

## The rule

A pull request that changes a file the al-runner binary is built from (`tools/binary-inputs.txt`: crate sources, build
scripts, manifests, `Cargo.lock`, and the files crates include, such as `docs/language-catalog.json` and
`tools/dotnet-sidecar/`) adds or changes a folder `tests/neighbors/<topic>/`. A pure rename does not count. `<topic>` is
the branch name or the issue. The folder holds exactly one of:

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
The key is the one the table prints in its first column. A key ending in ` @27` (any BC major) applies only on that
major; a key without it applies on every major the run covers.

```
# key	entry
LT PR Tests::PartRefill_ListHost_Last	ASSUMED part-left-view-test-filter (corpus PR 1450)
LT PR Tests::PartRefill_Worksheet_First	PINNED tests/al-language/pages/PartRefill.al:120
LT PR Tests::PartRefill_SecondFilter	REFUSED First() on a part whose view no longer started with the rows it holds
LT PR Tests::PartRefill_Guarded @27	REFUSED First() on a part
```

An entry is one of. The check reads every fact it states; an entry that does not hold counts as missing.

- `PINNED file:line`: the corpus case that pins the head result. `file` is relative to the corpus root
  (`corpus/`), and that file has the line. The head result is a clean pass or fail (no assumption id).
- `ASSUMED id`: the assumption registry entry the head result runs through (the file name of
  `crates/al-builtins/src/assumptions/<id>.rs`), then its open probe pull request. The head result carries that id.
- `REFUSED text`: the head refuses this shape on purpose, with a message that starts with `text`. The pull request
  body lists every shape that main answered and head now refuses, with the reason.

An entry whose test shows no difference on the run's major is stale and fails the check. Delete it.

A difference is another status (pass, fail, not run, missing), another set of assumption ids (a clean pass that
became flagged), or another failure message. Another refusal text is not a difference.

## Run it

```
tools/neighbor-diff.sh --changed                     # what CI runs: the folders your branch changes
tools/neighbor-diff.sh path/to/app [--bc-version 28]  # any app folder
```

Default BC version: 29, and CI runs BC 29 only. A change that a BC version decides (a version guard, a major-gated
table) runs the app locally on 27, 28 and 29 (`--bc-version 27 --bc-version 28 --bc-version 29`), and the pull request
body has all three tables. Each run prints one table per app and version. Paste the tables into the pull request body.
Logs and JUnit files: the folder the last line names. Exit 0: every difference has an entry. Exit 1: one has none,
an entry is stale, or the rule fails. Exit 2: a build or a run failed.

The main binary is the merge base of your branch and `origin/main`. Merge `origin/main` first. It comes from the
local cache, from the CI artifact of `main` (needs `gh`), or a local build, in that order.
