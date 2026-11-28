# Neighbor apps

A neighbor app shows what a change does to the shapes next to the one its evidence pins. It runs on a binary of
`main` and on the head binary of the pull request. Every test whose outcome differs needs an entry that says why.
`tools/neighbor-diff.sh` runs it, locally and in the CI job `neighbor check` (owner decision 2026-11-21).

## The rule

A pull request that changes a file the al-runner binary is built from (`tools/binary-inputs.txt`: crate sources, build
scripts, manifests, `Cargo.lock`, and the files crates include, such as `docs/language-catalog.json` and
`tools/dotnet-sidecar/`) adds or changes a folder `tests/neighbors/<topic>/`. A pure rename does not count. `<topic>` is
the branch name or the issue. The folder holds exactly one of:

- **A neighbor app**: `app.json`, `src/*.al` and `differences.tsv`, and optionally `flags.txt`: run options both
  binaries get, one per line (`#` starts a comment), for example `--strict-order`. Each run starts in a fresh copy of the app
  folder, so a relative path there (for example `--files-dir files`) names a folder of the app.
- **`none.txt`**: one line that says why no neighbor app applies, for example "comments only".
  The reviewer checks the reason. A change of comments only, or a tooling change no AL code sees, takes this form.

CI fails when the rule fails, and when a test differs and has no entry. A stale entry (an entry whose test no longer differs) fails for a folder the pull request adds or changes. For a folder given as an argument in a local run that the pull request does not change, it is printed as a note. In CI (`--changed`) every folder that runs is a changed one, so a stale entry always fails there.
A later pull request may delete old folders. Delete the folder once the fix has merged.

## The app

- One test per neighbor shape: each fact the new condition does not read (other caller, type, record state, trigger,
  extension, subscriber, sequence, object kind), each new refusal, and each reader of a new mark. Name each test for
  its shape.
- Keep it small and self-contained. Depend on Microsoft's apps only when the shape needs them.
- No Microsoft code. The legal guard checks it.
- It is not a pin. A behavior is still pinned by the corpus or runs through an assumption entry (`CLAUDE.md`, rule 3).

## Dependency apps

A neighbor app that needs another app of its own puts it in `deps/<name>/` (`app.json` and `src/*.al`).
`tools/neighbor-diff.sh` packs each one with `tools/pack_deps.py` into
`<publisher>_<name>_<version>.Source.zip` in a temporary folder and passes it as `--dep`, on the main and on the head
run, in CI as well. Never commit a `.zip`, `.app` or `.alpackages` folder (the legal guard fails the pull request),
and do not put a dependency folder in `.alpackages`: the runner compiles a plain folder there into the app itself.

## differences.tsv

One line per test that differs, `classname::name`, a tab, and the entry. Lines starting with `#` are comments.
The key is the one the table prints in its first column. An optional third column, after a second tab, names the BC
version the row applies to: a major (`27`) or a major.minor (`29.1`), matched against the build of the run. A row
without it applies on every run. Use the column when a change differs on some versions only (a version guard):
the row then counts, and is judged stale, only on runs of that version.

```
# key	entry
LT PR Tests::PartRefill_ListHost_Last	ASSUMED part-left-view-test-filter (corpus PR 1450)
LT PR Tests::PartRefill_Worksheet_First	PINNED tests/al-language/pages/PartRefill.al:120
LT PR Tests::PartRefill_SecondFilter	REFUSED First() on a part whose view no longer started with the rows it holds
LT PR Tests::PartRefill_Guarded	REFUSED First() on a part	29.1
```

An entry is one of. The check reads every fact it states; an entry that does not hold counts as missing.

- `PINNED file:line`: the corpus case that pins the head result. `file` is relative to the corpus root
  (`corpus/`): letters, digits, `_`, `.`, `-` and `/`, no leading `/`, no `..`, and it stays under `corpus/` after
  symlinks. That file has the line. The head result is a clean pass or fail (no assumption id).
- `ASSUMED id[,id]`: the assumption registry entries the head result runs through (the file name of
  `crates/al-builtins/src/assumptions/<id>.rs`), comma separated, no spaces. The head result carries those ids, and
  every id it carries that main's result did not carry is named. Text after the ids (the open probe pull request) is
  for the reader; the check does not read it.
- `REFUSED text`: the head refuses this shape on purpose, with a message that starts with `text`. The pull request
  body lists every shape that main answered and head now refuses, with the reason.

Suite-level baseline: a head whose install baseline newly runs through an assumption (JUnit property
`baseline_assumptions`) shows as the row `<baseline>`; it needs an `ASSUMED` entry like a test.

A malformed row (not 2 or 3 columns, a version that is not `27`-`29` or `29.1`) and a duplicate key are errors.

An entry that applies to a run but whose test shows no difference there is stale and fails the check. Delete it,
or give it the version column.

A difference is another status (pass, fail, not run, missing), another set of assumption ids (a clean pass that
became flagged), or another failure message. Another refusal text is not a difference.

## Run it

```
bash tools/neighbor-diff.sh --changed                # what CI runs: the folders your branch changes
bash tools/neighbor-diff.sh path/to/app [--bc-version 28] # any app folder
```

Default BC version: 29, and CI runs BC 29 only. A change that a BC version decides (a version guard, a major-gated
table) runs the app locally on 27, 28 and 29 (`--bc-version 27 --bc-version 28 --bc-version 29`), and the pull request
body has all three tables. Each run prints one table per app and version. Paste the tables into the pull request body.
Logs and JUnit files: the folder the last line names. Exit 0: every difference has an entry. Exit 1: one has none,
an entry is stale, the entries file is malformed, the rule fails, or the runs are not compared: a test stopped by a
memory or time limit on either side, no test ran on either side, or nothing passed on either side and nothing differs. Exit 2: a build or a run failed.

The main binary is the merge base of your branch and `origin/main`. Merge `origin/main` first. It comes from the
local cache, from the CI artifact of `main` (needs `gh`), or a local build, in that order.
