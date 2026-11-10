# Registering an assumption (rule 3 exceptions)

Read this when a behavior has no corpus pin but may be built under a hard rule 3 exception (`CLAUDE.md`).

Two exceptions exist (owner, 2026-10-15 and 2026-11-10): a Microsoft test needs the behavior, or an outside reporter's own repro needs it. Partner test suites do not qualify. Error texts come only from the corpus.

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
