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
  A user's report gets a regression test in `crates/al-test-cli/tests/user_reports.rs`.
- **Cite the pin.** Put the corpus `file:line` in a comment at the code that implements it.
- **Do not edit `corpus/` or `oracle/`**, and do not remove lines from
  `metrics/<major>/passing.txt`. Those files are the reference the project is measured against.
- **Match BC where AL code can see it**: values, errors, database state, trigger and event
  order, and whether code compiles. Internals, speed and diagnostics are free to be better.
- **Interfaces between crates change only through an ADR** in `docs/adr/`. If your change needs
  one, open an issue first so we can agree on the design.

### Checking your change

A full corpus run with `al-test-cli` takes about 20 seconds:

```
./target/release/al-test-cli --root . --baseline-diff
```

It lists what newly passes and which baseline cases stopped passing. No baseline case may stop
passing. CI fails a pull request that loses one.

Do not update the baselines (`metrics/<major>/passing*.txt`), the coverage maps
(`docs/corpus-coverage.*`, `docs/corpus-behavior-coverage.*`) or the assumptions table in
`docs/probes-needed.md`. A bot commits them to `main` after your pull request is merged, so new
passes are added to the baseline then. CI does not fail when these files are out of date in a pull
request. One generated file still belongs in your pull request: if you add a builtin, run
`cargo run -p al-builtins --example gen_builtin_ids` and commit `crates/al-builtins/src/builtin_ids.rs`.

To work on one test: `--explain TestName` shows why a test ends as it does, and `--codeunit ID`
or `--test NAME` runs only part of the corpus. `./target/release/al-test-cli --help` lists every
flag.

Before you push, run `./check.sh` (format, clippy with warnings as errors, all tests and the
corpus ratchet). It needs bash; on Windows, use Git Bash or WSL. CI runs the same checks, so if
you cannot run it locally, say so in the pull request.

## Opening a pull request

- One topic per pull request.
- If it fixes an issue, start the body with `Fixes #N`.
- Say which corpus tests pin the behavior, and list anything that is not pinned yet.
- Do not include machine names, URLs, user names, credentials, tenant ids or customer code, in
  the code, the docs or the pull request text.
- Code from Microsoft (decompiled or copied) or from other closed-source apps cannot be accepted.

CI for a first-time contributor's pull request starts after a maintainer approves the run.

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
