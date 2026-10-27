# BusinessCentral.AL.Runner

Run Business Central AL tests without a BC service tier, without Docker, without SQL Server
and without a BC license.

**The manual is at <https://aldev.tools/al-runner/>.** Its source is in [`docs/manual/`](docs/manual/).

> **AL Runner v3 replaces v2.** This repository now holds AL Runner v3, written in Rust.
> AL Runner v2 (C#) is frozen on the [`v2` branch](https://github.com/aldevtools/BusinessCentral.AL.Runner/tree/v2) (MIT License).
> Its dotnet tool [`MSDyn365BC.AL.Runner`](https://www.nuget.org/packages/MSDyn365BC.AL.Runner)
> stays on NuGet. v2 gets no further releases; the versions published on NuGet keep working.
> See [Migrating from version 2](docs/manual/migrating-from-v2.md). If you maintain a tool that
> calls AL Runner, see [Tool protocol](docs/manual/tool-protocol.md).
>
> v3 was developed in the repository `StefanMaron/al-runtime-rs`, now archived. Issue and pull
> request numbers below about 5570 in the commit history and in older documents refer to that
> repository.

## What it is

AL Runner is a standalone test runner for Business Central AL code. You point it at your app
and its test app, and it runs the test codeunits and reports each test as passed, failed or
not run.

AL Runner parses and checks AL source with its own front end and runs it in its own VM against an
in-memory database. It needs no BC service tier, no Microsoft DLLs and no .NET. Microsoft's
apps (System Application, Base Application, the test libraries) are compiled from their own
source, so your tests run against Microsoft's real AL code, not against stubs.

A feature AL Runner does not support is refused: the test is reported as "not run", with the
reason. It never returns a guessed default instead. Every behavior it implements is checked
against real Business Central 27.5, 28.5 and 29.0 by the
[AL language test corpus](https://github.com/aldevtools/BusinessCentral.AL.Language.Tests).
See [How results are checked](docs/manual/how-it-works.md).

> [!WARNING]
> **For testing only. Not for production use.**
>
> AL Runner is a developer tool. Use it to run your tests on your own machine while you write
> code, and in your build pipelines for fast feedback. It is not Business Central and does not
> replace it.
>
> - Do not use it to run AL code in production, or against production data.
> - Do not rely on it alone to decide that an app is ready for production. Before you deploy to
>   production, run your tests at least once in a way Microsoft supports, for example in a
>   Business Central container on Windows (BcContainerHelper) or with AL-Go for GitHub.
> - It is an alpha. Many tests are still refused, and results may change between builds.

## Install

`al-runner` is built for Linux x64, Linux arm64, Windows x64 and macOS arm64. With [mise](https://mise.jdx.dev):

```
mise use -g github:aldevtools/BusinessCentral.AL.Runner    # the newest numbered alpha
mise up                                                     # update
```

Then run the tests of your app:

```
al-runner test ./MyApp ./MyApp.Test
```

[Getting started](docs/manual/getting-started.md) covers the nightly `edge` build, the install
scripts, signing, dependencies and the cache.

## Documentation

- [The manual](https://aldev.tools/al-runner/): install, writing tests, reading results, CI,
  Microsoft's tests, editor tools, the command line reference.
- [What works and what is refused](docs/manual/what-works.md), and the [Roadmap](docs/manual/roadmap.md).
- [Reporting a problem](docs/manual/feedback.md): open an issue with the "Alpha feedback" template.

## Working on the runtime

Clone with `--recurse-submodules`; the corpus is a pinned submodule.

```
cargo build --release
./target/release/al-test-cli --root .                  # pass rate for the newest BC version, ratchet check
./target/release/al-test-cli --root . --all-versions   # 27.5, 28.5 and 29.0, then compare
./target/release/al-test-cli --root . --explain TEST   # why one test passed, failed or was not run
./check.sh                                             # fmt, clippy, all tests, corpus ratchet
```

Documents for contributors:

- `CLAUDE.md`: the rules every contributor and agent follows.
- `docs/architecture.md`: pipeline, crates, runtime model, budgets.
- `docs/codemap.md`: which module owns what, and where the entry points are.
- `docs/status.md`: current coverage, open work, known issues.
- `docs/adr/`: architecture decisions. Seams change only through an ADR.
- `docs/releasing.md`: how an edge build, a numbered alpha and a hotfix are released.
- `docs/alrunner-buckets.md`: how to run Microsoft's test buckets, and the measurements.
- `docs/demo-data-differences.md`: where the demo backup differs from Microsoft's test database, with proof.
- `docs/bcdb.md`: the `.bak` and `.bacpac` reader.
- `docs/al-runner-v2-parity.md`: which features of AL Runner v2 exist in the current version.
- `docs/dotnet-examples.md`: how Microsoft's AL uses `DotNet`.
- `docs/probes-needed.md`: behavior taken from documentation only, waiting for a probe on real BC.
- `docs/corpus-coverage.md`: which parts of the AL language the corpus covers.
- `docs/agents/brief.md`: the working method for implementation agents.
- `oracle/`: the real-BC results the pass rate is measured against.
- `metrics/<major>/passing.txt`: the cases that must keep passing, per BC major.
- `docs/history/`: the original spike brief and report, and retired research notes.

## License

AL Runner is licensed under the Business Source License 1.1 (`LICENSE`). You may use it for free,
including commercial work: developing, testing and building Business Central extensions for yourself or
for your customers, on your own machines and in build pipelines. You may not sell it, or offer it or a
product or service built substantially on it to others for a fee. Each version becomes available under
the Apache License 2.0 four years after it is published. Versions published before this change remain
under the MIT License. For other licensing arrangements, contact the author.
