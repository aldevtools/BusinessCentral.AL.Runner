# Contributing to AL Runner

Thank you for helping. Bug reports, real-BC measurements and pull requests are all welcome.

This page explains how a change gets into AL Runner. The most important rule comes first:
**AL Runner never guesses how Business Central behaves.** Every behavior it implements is
checked against a test that ran on real BC. If you know the rule below, the rest of this page
is mostly mechanics.

## Where behavior comes from: the corpus

The corpus is the repository
[aldevtools/BusinessCentral.AL.Language.Tests](https://github.com/aldevtools/BusinessCentral.AL.Language.Tests).
Its tests run on real BC 27.5, 28.5 and 29.0 in CI. It is checked out in `corpus/` as a git
submodule, and the expected results are in `oracle/`.

Each behavior in AL Runner cites the corpus test that pins it, as `file:line` in a code comment.
So a change usually goes like this:

1. **The behavior is already pinned.** Find the corpus test (search `corpus/tests/` for the
   method or property name), cite it in a code comment, implement it, and open a pull request.
2. **The behavior is not pinned yet.** It needs a corpus test first. You can:
   - open a pull request on the corpus repository (see its
     [CONTRIBUTING.md](https://github.com/aldevtools/BusinessCentral.AL.Language.Tests/blob/master/CONTRIBUTING.md)), or
   - describe the case in your issue or pull request (the AL code and the results you saw on real
     BC). A maintainer then writes the corpus test.

   Until the corpus test is merged, the code path can be added as a registered assumption in
   `crates/al-builtins/src/assumptions.rs`. Tests that pass through it are reported as
   "flagged passes", apart from clean passes. When the corpus test lands, it decides. If it
   disagrees with the assumption, the code changes.
3. **The behavior cannot be supported.** Then AL Runner refuses it with a clear reason, and the
   test is reported as "not run". It never runs a guessed or partial behavior instead.

Your own measurements on a BC container are very useful as evidence and as the start of a
corpus test. They do not replace one, because the corpus runs on every supported BC version and
keeps running after every change.

Error texts are never invented. They come from the corpus.

## What makes a good issue

The [alpha feedback template](.github/ISSUE_TEMPLATE/alpha-feedback.md) asks for what we need.
The most useful report is a test that behaves differently in AL Runner than on real BC, with a
small app that shows it. Replace customer names and code with neutral ones.

## Setting up

1. Install Rust. CI uses Rust 1.98.1 (`.github/workflows/ci.yml`); a newer stable release works
   too.
2. Clone with the corpus:
   `git clone --recurse-submodules https://github.com/aldevtools/BusinessCentral.AL.Runner`.
   For an existing clone: `git submodule update --init`.
3. Build once in release: `cargo build --release`.

Without the corpus submodule, many tests fail. That is expected and is not caused by your
change.

## Making a change

Before you open a source file, read `docs/architecture.md` and `docs/codemap.md` (which module
owns what). `CLAUDE.md` lists the project rules in full. The short version:

- **Test first.** Every behavior starts as a failing test, and every bug fix starts with a test
  that reproduces the bug. Unit tests live next to the code or in the crate's `tests/` folder.
  A user's report gets a regression test in `crates/al-test-cli/tests/suite/user_reports.rs`.
- **Cite the pin.** Put the corpus `file:line` in a comment at the code that implements it.
- **Do not edit `corpus/` or `oracle/`**, and do not add a line to `metrics/<major>/not-passing*.txt`
  for a case the oracle already had. Those files are the reference the project is measured against.
- **Match BC where AL code can see it**: values, errors, database state, trigger and event
  order, and whether code compiles. Internals, speed and diagnostics are free to be better.
- **Interfaces between crates change only through an ADR** in `docs/adr/`. If your change needs
  one, open an issue first so we can agree on the design.

### Checking your change

A full corpus run with `al-test-cli` takes about 20 seconds:

```
./target/release/al-test-cli --root . --baseline-diff
```

Each BC major and test app has a list of the corpus cases the runtime does not pass yet,
`metrics/<major>/not-passing*.txt` (ADR-065). The diff shows, by area, the regressions (cases not
listed that do not pass), the stale lines (listed cases that now pass, or that left the oracle), the
cases still listed, and cases that got no outcome. CI fails a pull request on any of them.

When your change makes listed cases pass, delete their lines in the same pull request:

```
./target/release/al-test-cli --root . --prune-not-passing
```

It only deletes lines. Never add a line to hide a regression: a line for a case the oracle already
had needs the owner's OK, and CI shows it in the job summary for review.

If you change the assumption registry, regenerate the table in `docs/probes-needed.md` in the same
pull request (`./target/release/al-test-cli --root . --assumptions-doc`); a test fails while it is
stale. If you add a builtin, run `cargo run -p al-builtins --example gen_builtin_ids` and commit
`crates/al-builtins/src/builtin_ids.rs`. The coverage maps (`docs/corpus-coverage.*`,
`docs/corpus-behavior-coverage.*`) are not committed: `--coverage-map` and `--behavior-coverage-map`
write them locally, and CI on `main` uploads them as an artifact.

To work on one test: `--explain TestName` shows why a test ends as it does, and `--codeunit ID`
or `--test NAME` runs only part of the corpus. `./target/release/al-test-cli --help` lists every
flag.

Before you push, run `bash ./check.sh` (format, clippy with warnings as errors, all tests and the
corpus ratchet). It needs bash; on Windows, use Git Bash or WSL, and start every `.sh` script with `bash` as written here (in PowerShell a bare `.sh` path can open in an editor instead of running). CI runs the same checks, so if
you cannot run it locally, say so in the pull request.

If your change touches a file the binary is built from (`tools/binary-inputs.txt`), add a neighbor app: a small
test app under `tests/neighbors/<topic>/` with one test per shape next to the one you fixed (another caller, type, trigger,
extension, subscriber, record state or call order), and a `differences.tsv` that says, for each test whose
outcome changed against `main`, which corpus test pins it, which assumption entry it runs through, or that
it is refused on purpose. `bash tools/neighbor-diff.sh --changed` runs it on your build and on a build of
`main` and prints the table. CI runs the same check first and fails when a difference has no entry. When
no neighbor app applies, add `tests/neighbors/<topic>/none.txt` with the reason. Details:
[tests/neighbors/README.md](tests/neighbors/README.md).

## Opening a pull request

- One topic per pull request.
- If it fixes an issue, start the body with `Fixes #N`.
- Say which corpus tests pin the behavior, and list anything that is not pinned yet.
- Do not include machine names, URLs, user names, credentials, tenant ids or customer code, in
  the code, the docs or the pull request text.
- Code from Microsoft (decompiled or copied) or from other closed-source apps cannot be accepted.

CI for a first-time contributor's pull request starts after a maintainer approves the run.

## How a pull request gets reviewed

A review starts only after the pull request reports ready. Before that, CI runs the job `test` and the
job "neighbor check" on every draft. The neighbor check runs small test apps for the cases next to your
change on a build of `main` and on a build of your branch, and compares the results. Most review findings
so far were cases next to a change that ran without a corpus pin. The check finds them before a reviewer
reads the code, so reviews take fewer rounds.

For each pull request:

1. Merge the current `main` into your branch.
2. If your change touches a file the binary is built from (`tools/binary-inputs.txt`, which includes
   `crates/*/src/**/*.rs`), add a folder `tests/neighbors/<topic>/`. `<topic>` is your branch name or the
   issue number. The folder holds one of:
   - A neighbor app: `app.json`, `src/*.al` with one test for each case next to your change, and
     `differences.tsv`. The file explains every test whose result differs from `main`: the corpus test
     that pins it (`PINNED file:line`), the assumption entry it runs through (`ASSUMED id`), or the
     refusal you added on purpose (`REFUSED text`).
   - `none.txt`: one line that says why no neighbor app applies, for example a speed change, a
     refactor or comments only. The reviewer checks the reason.

   The format is in [tests/neighbors/README.md](tests/neighbors/README.md).
3. Run `bash tools/neighbor-diff.sh --changed` locally. See [tools/neighbor-diff.sh](tools/neighbor-diff.sh).
   It runs the check CI runs and prints a table. Paste the table into the pull request description.
4. Push once when it passes. Draft CI (`test` and "neighbor check") runs on that push.
5. When draft CI is green, post a comment with the words "ready for review". Keep the pull request a
   draft. Marking a pull request ready for review starts the full CI set, and a maintainer does that.

Reviews follow the order in which pull requests report ready. If a review already left findings on a
pull request, the next review checks only those findings and the results of the neighbor check.

If the neighbor check does not fit your change, say so on the pull request.

### When a pull request goes quiet

If a review asks you for changes and there is no comment or commit from you for 7 days, a maintainer
finishes the pull request on a branch of their own. Your commits keep your name. When the linked issue
has the label `priority: high` or `priority: critical`, the wait is 3 days, because the issue affects many
users. If you need more time, say so in a comment and the wait starts again.

Issues that an open pull request fixes (`Fixes #N`) carry the label `author: implementing`. Maintainers
leave those issues to you while the pull request is active.

## License and contributor agreement

AL Runner is released under the license in [LICENSE](LICENSE).

Before a contribution can be merged, its author signs the
[AL Runner Contributor Assignment Agreement](docs/contributor-agreement/INDIVIDUAL.md). It
transfers the rights in the contribution to the project owner, Stefan Maroń, and gives the
contributor back a broad license to their own contribution.

- **As an individual:** on your first pull request, a bot asks you to sign. You sign by
  commenting the sentence it shows. You only sign once.
- **As part of your job:** your employer most likely owns your code. Then your employer signs
  the [Entity version](docs/contributor-agreement/ENTITY.md) by email first, as described at its
  end.

A pull request is not merged until every author has signed.
