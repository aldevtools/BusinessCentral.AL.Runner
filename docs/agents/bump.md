# Generated files, corpus bumps and the bump bot

Moved out of `brief.md`. Read it when you bump the corpus or change a generated file.

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
  `pinned_columns.rs` does not name the corpus commit, so it changes only when a pinned column changes.
- `crates/al-sema/tests/expected-unresolved.txt`: a reviewed list that never
  grows without a decision.

## The isolation oracle (Windows nightly)

The suites `isolation-function` and `isolation-codeunit` (BC 28, corpus `tests/al-language-isolation-probe`, runners
61300 and 61301) take their oracle from the corpus workflow "Nightly Windows (official MS BC container)", which
`tools/corpus_bump.sh` does not read. When a pin changes the probe app, the CLI refuses ("the oracle is recorded for
corpus commit X, but the pin Y changes or removes existing test files") and the bump points here. Record the oracle
again with `tools/record_windows_oracle.sh [--dry-run] [--pin SHA] RUN_ID`: a green run of that workflow whose commit
is an ancestor of the pin with the same probe app. It copies both XUnit files unchanged and rewrites the two suite
tables of `oracle/oracle.toml` and their `oracle/README.md` rows. Update the comment above the tables by hand.
