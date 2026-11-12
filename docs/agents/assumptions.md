# Registering an assumption (hard rule 3)

Read this when a behavior has no pin in the pinned corpus yet (`CLAUDE.md`, hard rule 3, owner 2026-11-12).
Such a behavior runs only through an assumption entry, and only after its probe is written. The entry cites
the open probe PR; your PR may land before the probe merges. The evidence is the probe's local result, a
Microsoft test that passes on real BC, or an outside reporter's own repro. Partner test suites do not qualify.
Error texts come only from the corpus.

- **The entry.** Cite the evidence in the code comment: the open probe PR and its corpus file, the Microsoft
  test (app, codeunit, test name), or the outside issue and its repro. Mark only the shapes the evidence
  measures; every other shape refuses.
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
- **An entry that waits for a probe** (the probe PR is open, its file is not in the pinned corpus yet)
  registers `awaiting: Awaiting { issues: &[N], files: &["tests/al-language/<area>/<file>.al"],
  since: "YYYY-MM-DD" }`: the al-runner issue (empty when none) and the corpus file(s) the probe adds. A test fails when an
  entry names an al-runner issue without it. When the fix implements what the reporter asked, the PR
  says `Fixes #N` even though its passes are flagged (owner, 2026-10-28: close implemented issues, they can be
  reopened); `Refs #N` only while cases of the issue are still refused. Once every awaited file is in the pinned corpus, `al-test-cli --root .
  --awaiting-assumptions` (run by `cargo test`, `check.sh` and `tools/corpus_bump.sh`) fails until the entry is
  removed and the pin cited, or the code is fixed where the pin disagrees. `--awaiting-assumptions --summary`
  prints one status line, the oldest entry first.
