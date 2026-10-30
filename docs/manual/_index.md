---
title: "AL Runner"
weight: 1
---

Run Business Central AL tests without a BC service tier, Docker or SQL Server.

AL Runner reads your AL source with its own front end and runs it in its own
virtual machine, against an in-memory database. Microsoft's apps (System
Application, Base Application, the test libraries) are compiled from Microsoft's
own AL source. Your tests therefore run against Microsoft's real code, not
against stubs. It is one binary. It needs no .NET and no Microsoft DLLs.

**For testing only. Not for production use.** AL Runner is a developer tool: use
it on your own machine while you write code, and in your build pipelines for fast
feedback. Do not use it to run AL code in production or against production data.
Do not rely on it alone to decide that an app is ready for production: before you
deploy, run your tests at least once in a way Microsoft supports, for example in a
Business Central container on Windows (BcContainerHelper) or with AL-Go for GitHub.

**AL Runner is an alpha.** This version replaces AL Runner version 2, which was
written in C# and is frozen ([Migrating from version 2](migrating-from-v2.md)). Many tests are still reported as "not run", and results can change
between builds. [Reading results](results.md) explains what a pass, a failure and
"not run" each mean.

## Where to start

- **[Getting started](getting-started.md)**: install it and run your tests.
- **[Supported versions](versions.md)**: Business Central versions, platforms and release channels.
- **[Writing tests](writing-tests.md)**: apps, test codeunits, selecting tests, isolation, test-first work.
- **[Stability mode](stability.md)**: run tests again under changed seeds, order and dates to find hidden dependencies.
- **[Reading results](results.md)**: what you can trust, and how to find out why a test did not run.
- **[How results are checked](how-it-works.md)**: why results can be trusted, and the known risks.
- **[Command line reference](cli-reference.md)**: every command and flag.
- **[Test data](test-data.md)**: the CRONUS demo company, other countries, the RapidStart package company.
- **[BC license](license.md)**: which license a run uses, `--license FILE`, and what License Permission answers.
- **[Running Microsoft's tests](microsoft-tests.md)**: `test-microsoft`, the recipe, and the Microsoft tests your extension can affect.
- **[Continuous integration](ci.md)**: GitHub Actions, caching, exit codes.
- **[Editor and tool integration](editor-tools.md)**: the `serve` protocol, coverage output.
- **[Tool protocol](tool-protocol.md)**: the command line, JSON and `serve` contract for tool authors.
- **[Debugging](debugging.md)**: `al-runner debug`, breakpoints and stepping from an editor.
- **[Impact analysis](impact.md)**: what an extension changes in other apps' tests.
- **[What works and what is refused](what-works.md)**: the supported surface and the deliberate limits.
- **[Roadmap](roadmap.md)**: what is deferred, experimental, not built yet or decided against.
- **[Migrating from version 2](migrating-from-v2.md)**: command and flag changes.
- **[Troubleshooting](troubleshooting.md)**: what common messages mean.
- **[Questions and answers](faq.md)**: short answers to common questions.
- **[Reporting a problem](feedback.md)**: a wrong result, a refused feature you need, or a crash.

## What it is for

Fast feedback on your own machine and in pull request builds, before the full
Business Central pipeline runs. A run of a small app takes under a second after
the first one. Linux x64, Linux arm64, Windows x64 and macOS arm64 are supported.

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
