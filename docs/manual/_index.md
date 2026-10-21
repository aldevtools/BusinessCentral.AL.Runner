---
title: "AL Runner"
weight: 1
---

Run Business Central AL tests without a service tier, without Docker, without
SQL Server and without a license.

AL Runner reads your AL source with its own front end and runs it in its own
virtual machine, against an in-memory database. Microsoft's apps (System
Application, Base Application, the test libraries) are compiled from Microsoft's
own AL source. Your tests therefore run against Microsoft's real code, not
against stubs. It is one binary. It needs no .NET and no Microsoft DLLs.

**Version 3 is an alpha.** It replaces version 2, which was written in C# and is
frozen. Many tests are still reported as "not run", and results can change
between builds. Do not gate a pipeline on it yet. [Reading results](results.md)
explains what a pass, a failure and "not run" each mean.

## Where to start

- **[Getting started](getting-started.md)**: install it and run your tests.
- **[Writing tests](writing-tests.md)**: apps, test codeunits, selecting tests, isolation, test-first work.
- **[Reading results](results.md)**: what you can trust, and how to find out why a test did not run.
- **[Command line reference](cli-reference.md)**: every command and flag.
- **[Test data and Microsoft's tests](test-data.md)**: the CRONUS demo company, other countries, Microsoft's recipe.
- **[Continuous integration](ci.md)**: GitHub Actions, caching, exit codes.
- **[Editor and tool integration](editor-tools.md)**: the `serve` protocol, coverage output.
- **[Impact analysis](impact.md)**: what an extension changes in other apps' tests.
- **[What works and what is refused](what-works.md)**: the supported surface and the deliberate limits.
- **[Migrating from version 2](migrating-from-v2.md)**: command and flag changes.
- **[Troubleshooting](troubleshooting.md)**: what common messages mean.

## What it is for

Fast feedback on your own machine and in pull request builds, before the full
Business Central pipeline runs. A run of a small app takes under a second after
the first one. Linux x64, Windows x64 and macOS arm64 are supported.

It does not replace testing against a real service tier. When AL Runner cannot
answer a question the way Business Central would, it refuses the test and says
why. It never returns a guessed default. That rule is the main design decision,
and the rest of this manual follows from it.

## Where the correctness comes from

Correctness is defined by real Business Central. The
[AL language test corpus](https://github.com/aldevtools/BusinessCentral.AL.Language.Tests)
runs against real Business Central 27.5, 28.5 and 29.0 on every push, and each
result is the reference for that version. A corpus test counts as passing only
when it passes in AL Runner and passed on real Business Central. One binary
serves all supported versions.
